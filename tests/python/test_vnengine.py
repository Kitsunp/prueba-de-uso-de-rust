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
    Scene,
    Script,
    SetFlag,
    event_from_dict,
)


class TypesTests(unittest.TestCase):
    def test_script_serialization_is_stable(self):
        script = Script(
            events=[Dialogue(speaker="Ava", text="Hola")],
            labels={"b": 1, "a": 0},
        )
        expected = (
            '{"events":[{"speaker":"Ava","text":"Hola","type":"dialogue"}],'
            '"labels":{"a":0,"b":1}}'
        )
        self.assertEqual(script.to_json(), expected)

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


class BuilderTests(unittest.TestCase):
    def test_builder_json_is_stable_across_threads(self):
        builder = ScriptBuilder()
        builder.label("start")
        builder.dialogue("Ava", "Hola")
        builder.choice("Go?", [("Yes", "end"), ("No", "start")])
        builder.label("end")
        builder.set_flag("done", True)

        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(lambda _: builder.to_json(), range(8)))

        self.assertTrue(all(result == results[0] for result in results))
        payload = json.loads(results[0])
        self.assertEqual(payload["labels"], {"end": 2, "start": 0})


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

        engine = Engine.from_script({"events": [], "labels": {"start": 0}})
        self.assertIsInstance(engine.raw, FakeEngine)
        self.assertEqual(captured["payload"], '{"events":[],"labels":{"start":0}}')


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


if __name__ == "__main__":
    unittest.main()
