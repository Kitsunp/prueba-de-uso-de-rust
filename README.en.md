# Visual Novel Engine (Rust)

Complete visual novel engine based on events. It loads a JSON script, advances through events, handles choices, saves/loads game state, and displays the story with a native graphical interface.

## Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start (Rust)](#quick-start-rust)
- [Graphical Interface (GUI)](#graphical-interface-gui)
- [Script Format](#script-format)
- [Save System](#save-system)
- [Development Tools](#development-tools)
- [Python Bindings](#python-bindings)
- [Code Layout](#code-layout)

## Features

- **Logic Engine**: Dialogue, scene, choice, jump, and flag events.
- **Visual State**: Maintains accumulated background, music, and characters.
- **Native GUI**: Full viewer built with `eframe` (egui).
- **Persistence**: Save/load system with integrity verification (checksum).
- **Dialogue History**: Scrollable backlog of the last 200 messages.
- **Debug Inspector**: Real-time tool to modify flags and jump to labels.
- **Python Bindings**: Use the engine from Python via `pyo3`.

## Installation

### Core only (no GUI)

```toml
[dependencies]
visual_novel_engine = { path = "crates/core" }
```

### Automatic Install (Windows)

Run the included script to build Rust and install Python bindings:

```powershell
.\install.ps1
```

### Manual Build

1. **Rust (Core + GUI)**: `cargo build --release`
2. **Python Bindings**:
   ```bash
   pip install maturin
   maturin build --manifest-path crates/py/Cargo.toml --release
   pip install target/wheels/*.whl --force-reinstall
   ```

### With graphical interface

```toml
[dependencies]
visual_novel_gui = { path = "crates/gui" }
```

## Quick Start (Rust)

### Logic only (no window)

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

println!("Current event: {:?}", engine.current_event()?);
engine.step()?;
engine.choose(0)?; // Pick the first option
```

### With graphical interface

```rust
use visual_novel_gui::{run_app, VnConfig};

let script_json = include_str!("my_story.json");

let config = VnConfig {
    title: "My Visual Novel".to_string(),
    width: Some(1280.0),
    height: Some(720.0),
    ..Default::default()
};

run_app(script_json.to_string(), Some(config))?;
```

## Graphical Interface (GUI)

The GUI provides a complete visual novel experience:

- **Scene Rendering**: Displays backgrounds, characters, and music info.
- **Dialogue Box**: Presents text and choices interactively.
- **Settings Menu** (`ESC`): Adjust UI scale, fullscreen, and VSync.
- **History** (UI button): Review past dialogue lines.
- **Save/Load**: Native file dialogs from the settings menu.

## Script Format

A script is JSON with:

- `events`: list of events.
- `labels`: map of labels to indices (`start` is required).

```json
{"type": "dialogue", "speaker": "Ava", "text": "Hello"}
{"type": "choice", "prompt": "Go?", "options": [{"text": "Yes", "target": "end"}]}
{"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [{"name": "Ava", "expression": "smile", "position": "center"}]}
{"type": "jump", "target": "intro"}
{"type": "set_flag", "key": "visited", "value": true}
```

## Save System

The engine includes secure persistence:

- **Script Checksum**: Each save stores a hash of the original script.
- **Validation on Load**: If the script changed, the save is rejected to prevent corruption.
- **JSON Format**: Saves are human-readable and debuggable.

## Development Tools

### Inspector (`F12`)

Debug window for developers:

- View and modify **flags** in real time.
- Jump to any **label** in the script.
- Monitor **FPS** and history memory usage.

## Python Bindings

### Installation

```bash
pip install visual_novel_engine --find-links=target/wheels
```

> Note: Ensure you ran `install.ps1` or built manually with `maturin` first.

### Basic usage (logic only)

```python
from visual_novel_engine import PyEngine

engine = PyEngine(script_json)
print(engine.current_event())
engine.step()
engine.choose(0)
```

### With graphical interface

```python
import visual_novel_engine as vn

config = vn.VnConfig(width=1280.0, height=720.0)
vn.run_visual_novel(script_json, config)
```

## Code Layout

- `crates/core/`: Engine core (logic, compilation, state).
- `crates/gui/`: Graphical interface with eframe.
- `crates/py/`: Python bindings.
- `examples/`: Usage examples in Rust and Python.
