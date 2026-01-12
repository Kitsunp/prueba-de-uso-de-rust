# Visual Novel Engine (Rust)

Event-driven visual novel engine. It loads a JSON script, advances through events, handles choices, and keeps visual state (background, music, characters).

## Contents

- [Features](#features)
- [Installation](#installation)
- [Quick start (Rust)](#quick-start-rust)
- [Script format](#script-format)
- [State and rendering](#state-and-rendering)
- [Security policy and limits](#security-policy-and-limits)
- [Python bindings](#python-bindings)
- [Code layout](#code-layout)

## Features

- Dialogue, scene, choice, jump, and flag events.
- Accumulated visual state (background, music, characters).
- Script validation with security policy and resource limits.
- Reference text renderer for quick debugging.
- Optional Python bindings via `pyo3`.

## Installation

```toml
[dependencies]
visual_novel_engine = { path = "crates/core" }
```

> Adjust the path for your project. This repository works as a local crate.

## Quick start (Rust)

```rust
use visual_novel_engine::{Engine, Script, SecurityPolicy, ResourceLimiter};

let script_json = r#"
{
  "events": [
    {"type": "dialogue", "speaker": "Ava", "text": "Hello"},
    {"type": "choice", "prompt": "Go?", "options": [
      {"text": "Yes", "target": "end"},
      {"text": "No", "target": "start"}
    ]},
    {"type": "dialogue", "speaker": "Ava", "text": "The end"}
  ],
  "labels": {"start": 0, "end": 2}
}
"#;

let script = Script::from_json(script_json)?;
let mut engine = Engine::new(script, SecurityPolicy::default(), ResourceLimiter::default())?;

let event = engine.current_event()?;
println!("Current event: {event:?}");

engine.step()?; // advance in the flow
engine.choose(0)?; // pick the first option
```

## Script format

A script is JSON with:

- `events`: list of events.
- `labels`: map of labels to `events` indices (`start` is required).

Supported event types:

```json
{"type": "dialogue", "speaker": "Ava", "text": "Hello"}
{"type": "choice", "prompt": "Go?", "options": [{"text": "Yes", "target": "end"}]}
{"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [{"name": "Ava", "expression": "smile", "position": "center"}]}
{"type": "jump", "target": "intro"}
{"type": "set_flag", "key": "visited", "value": true}
```

## State and rendering

- The engine maintains `EngineState` with position, flags, and visual state.
- `visual_state()` exposes the current background, music, and characters.
- `TextRenderer` renders events into text for quick checks.

Rendering example:

```rust
use visual_novel_engine::{Engine, Script, SecurityPolicy, ResourceLimiter, TextRenderer};

let script_json = r#"
{
  "events": [
    {"type": "dialogue", "speaker": "Ava", "text": "Hello world"}
  ],
  "labels": {"start": 0}
}
"#;

let script = Script::from_json(script_json)?;
let engine = Engine::new(script, SecurityPolicy::default(), ResourceLimiter::default())?;
let output = engine.render_current(&TextRenderer)?;
println!("{}", output.text);
```

## Security policy and limits

Validation enforces:

- Presence of the `start` label.
- Length limits for text, labels, and asset references.
- Valid label indices.
- Non-empty choice options with valid targets.

Tune limits via `ResourceLimiter` or relax rules with `SecurityPolicy`.

## Python bindings

Enable the `python` feature to expose `PyEngine`:

```bash
maturin develop --features python
```

Example:

```python
from visual_novel_engine import PyEngine

engine = PyEngine(script_json)
print(engine.current_event())
print(engine.step())
print(engine.choose(0))
```

More examples in `examples/python`.

## Code layout

- `crates/core/src/engine.rs`: engine core and event navigation.
- `crates/core/src/script.rs`: JSON loading and script validation helpers.
- `crates/core/src/event.rs`: event definitions and serialization.
- `crates/core/src/visual.rs`: visual state (background, music, characters).
- `crates/core/src/render.rs`: render interface and `TextRenderer`.
- `crates/core/src/security.rs`: validation policy and rules.
- `crates/core/src/resource.rs`: resource limits.
- `crates/core/src/error.rs`: error types with diagnostics.
- `crates/core/src/state.rs`: engine internal state.
