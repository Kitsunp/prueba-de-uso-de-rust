import json
import sys
import types
import unittest
from concurrent.futures import ThreadPoolExecutor

from vnengine.app import EngineApp
from vnengine.builder import ScriptBuilder
from vnengine.engine import Engine, _load_native_engine
from vnengine.types import (
    CharacterPlacement,
    Choice,
    ChoiceOption,
    Dialogue,
    Jump,
    JumpIf,
    Scene,
    Script,
    SCRIPT_SCHEMA_VERSION,
    SetFlag,
    SetVar,
    event_from_dict,
)


class TypesTests(unittest.TestCase):
    def test_script_serialization_is_stable(self):
        script = Script(
            events=[Dialogue(speaker="Ava", text="Hola")],
            labels={"b": 1, "a": 0},
        )
        expected = (
            f'{{"events":[{{"speaker":"Ava","text":"Hola","type":"dialogue"}}],'
            f'"labels":{{"a":0,"b":1}},"script_schema_version":"{SCRIPT_SCHEMA_VERSION}"}}'
        )
        self.assertEqual(script.to_json(), expected)

    def test_script_requires_schema_version(self):
        with self.assertRaises(ValueError):
            Script.from_json('{"events": [], "labels": {"start": 0}}')

    def test_event_from_dict_rejects_unknown_type(self):
        with self.assertRaises(ValueError):
            event_from_dict({"type": "unknown"})

    def test_character_from_dict_coerces_optional_fields(self):
        placement = CharacterPlacement.from_dict(
            {"name": "Ava", "expression": 1, "position": True}
        )
        self.assertEqual(placement.expression, "1")
        self.assertEqual(placement.position, "True")

    def test_set_flag_from_dict_requires_bool(self):
        with self.assertRaises(ValueError):
            SetFlag.from_dict({"key": "flag", "value": "false"})

    def test_set_var_from_dict_requires_int(self):
        with self.assertRaises(ValueError):
            SetVar.from_dict({"key": "counter", "value": "5"})

    def test_jump_if_requires_cond(self):
        with self.assertRaises(ValueError):
            JumpIf.from_dict({"type": "jump_if", "cond": {"kind": "unknown"}, "target": "end"})


class BuilderTests(unittest.TestCase):
    def test_builder_json_is_stable_across_threads(self):
        builder = ScriptBuilder()
        builder.label("start")
        builder.dialogue("Ava", "Hola")
        builder.choice("Go?", [("Yes", "end"), ("No", "start")])
        builder.label("end")
        builder.set_flag("done", True)
        builder.set_var("counter", 3)
        builder.jump_if_var("counter", "gt", 1, target="end")
        builder.patch(add=[("Ava", "happy", "left")], update=[("Ava", None, "center")], remove=[])

        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(lambda _: builder.to_json(), range(8)))

        self.assertTrue(all(result == results[0] for result in results))
        payload = json.loads(results[0])
        self.assertEqual(payload["labels"], {"end": 2, "start": 0})
        self.assertEqual(payload["script_schema_version"], SCRIPT_SCHEMA_VERSION)
        patch_events = [event for event in payload["events"] if event["type"] == "patch"]
        self.assertEqual(len(patch_events), 1)
        self.assertEqual(patch_events[0]["add"][0]["name"], "Ava")


class EngineWrapperTests(unittest.TestCase):
    def setUp(self):
        self._original_module = sys.modules.get("visual_novel_engine")

    def tearDown(self):
        if self._original_module is None:
            sys.modules.pop("visual_novel_engine", None)
        else:
            sys.modules["visual_novel_engine"] = self._original_module

    def test_engine_wrapper_prefers_engine_binding(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine_cls = _load_native_engine()
        self.assertIs(engine_cls, FakeEngine)

    def test_engine_from_script_accepts_mapping(self):
        captured = {}
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                captured["payload"] = script_json

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {"script_schema_version": SCRIPT_SCHEMA_VERSION, "events": [], "labels": {"start": 0}}
        )
        self.assertIsInstance(engine.raw, FakeEngine)
        self.assertEqual(
            captured["payload"],
            f'{{"events":[],"labels":{{"start":0}},"script_schema_version":"{SCRIPT_SCHEMA_VERSION}"}}',
        )

    def test_engine_ui_state_calls_native(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json

            def ui_state(self):
                return {"type": "choice", "prompt": "Go?", "options": ["Yes", "No"]}

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {"script_schema_version": SCRIPT_SCHEMA_VERSION, "events": [], "labels": {"start": 0}}
        )
        self.assertEqual(
            engine.ui_state(),
            {"type": "choice", "prompt": "Go?", "options": ["Yes", "No"]},
        )

    def test_engine_ui_state_requires_binding(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {"script_schema_version": SCRIPT_SCHEMA_VERSION, "events": [], "labels": {"start": 0}}
        )
        with self.assertRaises(RuntimeError):
            engine.ui_state()


class EngineAppTests(unittest.TestCase):
    def test_engine_app_runs_choices(self):
        events = [
            {"type": "choice", "prompt": "Go?", "options": []},
            {"type": "dialogue", "speaker": "Ava", "text": "Done"},
        ]

        class FakeEngine:
            def __init__(self):
                self.index = 0

            def current_event(self):
                if self.index >= len(events):
                    raise ValueError("script exhausted")
                return events[self.index]

            def choose(self, option_index):
                self.index += 1
                return events[self.index - 1]

            def step(self):
                self.index += 1
                return events[self.index - 1]

        app = EngineApp(FakeEngine())
        collected = app.run(lambda _event: 0)
        self.assertEqual(len(collected), 2)
        self.assertEqual(collected[0]["type"], "choice")
        self.assertEqual(collected[1]["type"], "dialogue")

    def test_engine_app_propagates_unexpected_errors(self):
        class BrokenEngine:
            def current_event(self):
                raise RuntimeError("boom")

        app = EngineApp(BrokenEngine())
        with self.assertRaises(RuntimeError):
            app.run()


class GuiBindingTests(unittest.TestCase):
    def test_run_visual_novel_rejects_invalid_json(self):
        import visual_novel_engine as vn

        with self.assertRaises(ValueError):
            vn.run_visual_novel("{invalid", None)

    def test_gui_bindings_exist(self):
        import visual_novel_engine as vn

        config = vn.VnConfig(width=800.0, height=600.0, fullscreen=False)
        self.assertIsNotNone(config)
        self.assertTrue(callable(vn.run_visual_novel))


if __name__ == "__main__":
    unittest.main()
