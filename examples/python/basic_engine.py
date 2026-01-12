"""Minimal example that advances a script and resolves a choice."""

from __future__ import annotations

from visual_novel_engine import PyEngine

SCRIPT_JSON = """
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


def main() -> None:
    engine = PyEngine(SCRIPT_JSON)
    print("current:", engine.current_event())
    print("step:", engine.step())
    print("choice:", engine.current_event())
    print("choose:", engine.choose(0))
    print("step:", engine.step())


if __name__ == "__main__":
    main()
