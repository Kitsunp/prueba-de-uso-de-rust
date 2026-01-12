from visual_novel_engine import PyEngine

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
print(engine.current_event())
print(engine.visual_state())
engine.step()
print(engine.visual_state())
