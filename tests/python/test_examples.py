import unittest


def load_engine():
    try:
        from visual_novel_engine import PyEngine  # type: ignore
    except Exception as exc:  # pragma: no cover - environment dependent
        return None, exc
    return PyEngine, None


class ExampleUsageTests(unittest.TestCase):
    def test_basic_engine_example(self):
        PyEngine, err = load_engine()
        if PyEngine is None:
            self.skipTest(f"PyEngine not available: {err}")
        script_json = """
        {
          "events": [
            {"type": "dialogue", "speaker": "Ava", "text": "Hola"},
            {"type": "choice", "prompt": "Ir?", "options": [
              {"text": "Si", "target": "end"},
              {"text": "No", "target": "start"}
            ]},
            {"type": "dialogue", "speaker": "Ava", "text": "Fin"}
          ],
          "labels": {"start": 0, "end": 2}
        }
        """
        engine = PyEngine(script_json)
        event = engine.current_event()
        self.assertEqual(event["type"], "dialogue")
        engine.step()
        choice = engine.current_event()
        self.assertEqual(choice["type"], "choice")

    def test_scene_visuals_example(self):
        PyEngine, err = load_engine()
        if PyEngine is None:
            self.skipTest(f"PyEngine not available: {err}")
        script_json = """
        {
          "events": [
            {"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [
              {"name": "Ava", "expression": "smile", "position": "center"}
            ]},
            {"type": "dialogue", "speaker": "Ava", "text": "Bienvenido"}
          ],
          "labels": {"start": 0}
        }
        """
        engine = PyEngine(script_json)
        state = engine.visual_state()
        self.assertEqual(state["background"], "bg/room.png")
        self.assertEqual(state["music"], "music/theme.ogg")
        self.assertEqual(len(state["characters"]), 1)


if __name__ == "__main__":
    unittest.main()
