from visual_novel_engine import PyEngine

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
print(engine.current_event())
print(engine.step())
print(engine.choose(0))
print(engine.step())
