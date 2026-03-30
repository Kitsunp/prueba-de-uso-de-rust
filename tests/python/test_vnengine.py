import json
import os
import subprocess
import sys
import shutil
import types
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from uuid import uuid4

from vnengine.app import EngineApp
from vnengine.builder import ScriptBuilder
from vnengine.engine import Engine, _load_native_engine
from vnengine.localization import LocalizationCatalog, collect_script_localization_keys
from vnengine.types import (
    AudioAction,
    CharacterPlacement,
    Dialogue,
    JumpIf,
    Script,
    SCRIPT_SCHEMA_VERSION,
    SetCharacterPosition,
    SetFlag,
    SetVar,
    Transition,
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

    def test_script_accepts_missing_schema_version_for_legacy(self):
        parsed = Script.from_json('{"events": [], "labels": {"start": 0}}')
        self.assertEqual(parsed.labels["start"], 0)

    def test_script_accepts_legacy_major_schema_version(self):
        parsed = Script.from_json(
            '{"script_schema_version":"0.9","events":[],"labels":{"start":0}}'
        )
        self.assertEqual(parsed.labels["start"], 0)

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
            JumpIf.from_dict(
                {"type": "jump_if", "cond": {"kind": "unknown"}, "target": "end"}
            )

    def test_audio_transition_and_position_roundtrip(self):
        events = [
            AudioAction(channel="bgm", action="play", asset="music.ogg"),
            Transition(kind="fade", duration_ms=300),
            SetCharacterPosition(name="Ava", x=10, y=20, scale=1.0),
        ]
        script = Script(events=events, labels={"start": 0})
        parsed = Script.from_json(script.to_json())

        self.assertEqual(len(parsed.events), 3)
        self.assertEqual(parsed.events[0].to_dict()["type"], "audio_action")
        self.assertEqual(parsed.events[1].to_dict()["type"], "transition")
        self.assertEqual(parsed.events[2].to_dict()["type"], "set_character_position")

    def test_audio_action_loop_playback_requires_bool(self):
        with self.assertRaises(ValueError):
            AudioAction.from_dict(
                {
                    "channel": "bgm",
                    "action": "play",
                    "asset": None,
                    "volume": None,
                    "fade_duration_ms": None,
                    "loop_playback": "false",
                }
            )

    def test_script_labels_require_int_indices(self):
        with self.assertRaises(ValueError):
            Script.from_json(
                '{"script_schema_version":"1.0","events":[],"labels":{"start":true}}'
            )

    def test_set_var_rejects_bool_payload(self):
        with self.assertRaises(ValueError):
            SetVar.from_dict({"key": "counter", "value": True})

    def test_character_and_transition_numeric_fields_reject_bool(self):
        with self.assertRaises(ValueError):
            CharacterPlacement.from_dict({"name": "Ava", "x": True})
        with self.assertRaises(ValueError):
            Transition.from_dict({"kind": "fade", "duration_ms": False})
        with self.assertRaises(ValueError):
            SetCharacterPosition.from_dict(
                {"name": "Ava", "x": 1, "y": 2, "scale": True}
            )
        with self.assertRaises(ValueError):
            AudioAction.from_dict(
                {
                    "channel": "bgm",
                    "action": "play",
                    "asset": None,
                    "volume": True,
                    "fade_duration_ms": None,
                    "loop_playback": None,
                }
            )


class LocalizationTests(unittest.TestCase):
    def test_collect_and_validate_localization_keys(self):
        script = Script(
            events=[
                Dialogue(speaker="loc:speaker.narrator", text="loc:dialogue.intro"),
                AudioAction(channel="bgm", action="play", asset="theme.ogg"),
            ],
            labels={"start": 0},
        )
        keys = collect_script_localization_keys(script)
        self.assertEqual(keys, {"speaker.narrator", "dialogue.intro"})

        catalog = LocalizationCatalog(
            default_locale="en",
            locales={
                "en": {"speaker.narrator": "Narrator"},
                "es": {"speaker.narrator": "Narrador", "unused": "x"},
            },
        )
        missing, orphan = catalog.validate_keys(keys)
        self.assertIn("en:dialogue.intro", missing)
        self.assertIn("es:dialogue.intro", missing)
        self.assertIn("es:unused", orphan)


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
        builder.patch(
            add=[("Ava", "happy", "left")], update=[("Ava", None, "center")], remove=[]
        )
        builder.audio_action("bgm", "play", asset="music/theme.ogg", loop_playback=True)
        builder.transition("fade", 250)
        builder.set_character_position("Ava", 32, 48, 1.1)
        builder.ext_call("open_minigame", ["cards"])

        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(lambda _: builder.to_json(), range(8)))

        self.assertTrue(all(result == results[0] for result in results))
        payload = json.loads(results[0])
        self.assertEqual(payload["labels"], {"end": 2, "start": 0})
        self.assertEqual(payload["script_schema_version"], SCRIPT_SCHEMA_VERSION)
        patch_events = [
            event for event in payload["events"] if event["type"] == "patch"
        ]
        self.assertEqual(len(patch_events), 1)
        self.assertEqual(patch_events[0]["add"][0]["name"], "Ava")
        self.assertTrue(
            any(event["type"] == "audio_action" for event in payload["events"])
        )
        self.assertTrue(
            any(event["type"] == "transition" for event in payload["events"])
        )
        self.assertTrue(
            any(
                event["type"] == "set_character_position" for event in payload["events"]
            )
        )
        self.assertTrue(any(event["type"] == "ext_call" for event in payload["events"]))

    def test_builder_ext_call_rejects_non_string_args(self):
        builder = ScriptBuilder()
        with self.assertRaises(ValueError):
            builder.ext_call("open_minigame", ["cards", 7])


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
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        self.assertIsInstance(engine.raw, FakeEngine)
        self.assertEqual(
            captured["payload"],
            f'{{"events":[],"labels":{{"start":0}},"script_schema_version":"{SCRIPT_SCHEMA_VERSION}"}}',
        )

    def test_engine_wrapper_extcall_policy_methods_delegate(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json
                self.allowed = []
                self.handler = None
                self.error = None

            def allow_ext_call_command(self, command):
                self.allowed.append(command)

            def clear_ext_call_capabilities(self):
                self.allowed.clear()

            def register_handler(self, callback):
                self.handler = callback

            def last_ext_call_error(self):
                return self.error

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        sentinel = object()
        engine.allow_ext_call_command("minigame_start")
        engine.register_handler(sentinel)
        engine.clear_ext_call_capabilities()
        self.assertEqual(engine.last_ext_call_error(), None)
        self.assertEqual(engine.raw.allowed, [])
        self.assertIs(engine.raw.handler, sentinel)

    def test_engine_step_normalizes_native_step_result_and_tracks_audio(self):
        module = types.ModuleType("visual_novel_engine")

        class StepResult:
            def __init__(self):
                self.event = {"type": "dialogue", "speaker": "Ava", "text": "Hola"}
                self.audio = [{"type": "play_bgm", "path": "theme.ogg"}]

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json

            def step(self):
                return StepResult()

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        event = engine.step()
        self.assertEqual(event["type"], "dialogue")
        self.assertEqual(
            engine.last_audio_commands(), [{"type": "play_bgm", "path": "theme.ogg"}]
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
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
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
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        with self.assertRaises(RuntimeError):
            engine.ui_state()

    def test_engine_read_tracking_wrapper_methods(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json

            def is_current_dialogue_read(self):
                return True

            def choice_history(self):
                return [{"option_index": 0}]

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        self.assertTrue(engine.is_current_dialogue_read())
        self.assertEqual(engine.choice_history(), [{"option_index": 0}])

    def test_engine_prefetch_wrapper_methods(self):
        module = types.ModuleType("visual_novel_engine")

        class FakeEngine:
            def __init__(self, script_json):
                self.script_json = script_json
                self.depth = 0

            def set_prefetch_depth(self, depth):
                self.depth = depth

            def prefetch_assets_hint(self):
                return ["bg/room.png"] if self.depth > 0 else []

        module.Engine = FakeEngine
        sys.modules["visual_novel_engine"] = module

        engine = Engine.from_script(
            {
                "script_schema_version": SCRIPT_SCHEMA_VERSION,
                "events": [],
                "labels": {"start": 0},
            }
        )
        engine.set_prefetch_depth(2)
        self.assertEqual(engine.prefetch_assets_hint(), ["bg/room.png"])

    def test_engine_can_be_created_from_any_cwd(self):
        repo_python = Path(__file__).resolve().parents[2] / "python"
        temp_root = Path(__file__).resolve().parents[2] / "target"
        module_dir = temp_root / f"vnengine_python_module_{uuid4().hex}"
        cwd_dir = temp_root / f"vnengine_python_cwd_{uuid4().hex}"
        module_dir.mkdir(parents=True, exist_ok=False)
        cwd_dir.mkdir(parents=True, exist_ok=False)

        try:
            module_path = module_dir / "visual_novel_engine.py"
            module_path.write_text(
                """
class Engine:
    def __init__(self, script_json):
        self.script_json = script_json

    def current_event(self):
        return {"type": "dialogue", "speaker": "Ava", "text": "Hola"}
""".strip()
            )

            code = """
from vnengine.engine import Engine

engine = Engine.from_script({
    "script_schema_version": "1.0",
    "events": [{"type": "dialogue", "speaker": "Ava", "text": "Hola"}],
    "labels": {"start": 0},
})
print(engine.current_event()["type"])
print(engine.raw.script_json)
""".strip()

            env = os.environ.copy()
            pythonpath_parts = [str(module_dir), str(repo_python)]
            if env.get("PYTHONPATH"):
                pythonpath_parts.append(env["PYTHONPATH"])
            env["PYTHONPATH"] = os.pathsep.join(pythonpath_parts)

            result = subprocess.run(
                [sys.executable, "-c", code],
                cwd=cwd_dir,
                env=env,
                capture_output=True,
                text=True,
                check=True,
            )
        finally:
            shutil.rmtree(module_dir, ignore_errors=True)
            shutil.rmtree(cwd_dir, ignore_errors=True)

        self.assertEqual(
            result.stdout.splitlines(),
            [
                "dialogue",
                '{"events":[{"speaker":"Ava","text":"Hola","type":"dialogue"}],"labels":{"start":0},"script_schema_version":"1.0"}',
            ],
        )


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
