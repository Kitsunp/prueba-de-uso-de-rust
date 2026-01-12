# Visual Novel Engine (Rust)

Motor básico para novelas visuales basado en eventos. Permite interpretar un guion en JSON, avanzar por los eventos, tomar decisiones y mantener estado visual (fondo, música y personajes).

## Contenido

- [Características](#características)
- [Instalación](#instalación)
- [Uso rápido (Rust)](#uso-rápido-rust)
- [Formato del guion](#formato-del-guion)
- [Estado y renderizado](#estado-y-renderizado)
- [Políticas de seguridad y límites](#políticas-de-seguridad-y-límites)
- [Bindings de Python](#bindings-de-python)
- [Estructura del código](#estructura-del-código)
- [Reporte automático de líneas](#reporte-automático-de-líneas)

## Características

- Eventos de diálogo, escena, elecciones, saltos y banderas.
- Estado visual acumulado (fondo, música, personajes).
- Validación de guiones con política de seguridad y límites de recursos.
- Renderizador de texto de referencia para pruebas rápidas.
- Bindings opcionales para Python mediante `pyo3`.

## Instalación

```toml
[dependencies]
visual_novel_engine = { path = "." }
```

> Ajusta la ruta según tu proyecto. Este repositorio funciona como crate local.

## Uso rápido (Rust)

```rust
use visual_novel_engine::{Engine, Script, SecurityPolicy, ResourceLimiter};

let script_json = r#"
{
  "events": [
    {"type": "dialogue", "speaker": "Ava", "text": "Hola"},
    {"type": "choice", "prompt": "¿Ir?", "options": [
      {"text": "Sí", "target": "end"},
      {"text": "No", "target": "start"}
    ]},
    {"type": "dialogue", "speaker": "Ava", "text": "Fin"}
  ],
  "labels": {"start": 0, "end": 2}
}
"#;

let script = Script::from_json(script_json)?;
let mut engine = Engine::new(script, SecurityPolicy::default(), ResourceLimiter::default())?;

let event = engine.current_event()?;
println!("Evento actual: {event:?}");

engine.step()?; // avanza en el flujo
engine.choose(0)?; // elige la primera opción
```

## Formato del guion

Un guion es un JSON con:

- `events`: lista de eventos.
- `labels`: mapa de etiquetas a índices de `events` (obligatorio `start`).

Tipos de evento disponibles:

```json
{"type": "dialogue", "speaker": "Ava", "text": "Hola"}
{"type": "choice", "prompt": "¿Ir?", "options": [{"text": "Sí", "target": "end"}]}
{"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [{"name": "Ava", "expression": "smile", "position": "center"}]}
{"type": "jump", "target": "intro"}
{"type": "set_flag", "key": "visited", "value": true}
```

## Estado y renderizado

- El motor mantiene `EngineState` con posición, banderas y estado visual.
- `visual_state()` expone el fondo, música y personajes vigentes.
- `TextRenderer` permite renderizar eventos a texto para depuración rápida.

Ejemplo de renderizado:

```rust
use visual_novel_engine::{Engine, Script, SecurityPolicy, ResourceLimiter, TextRenderer};

let script_json = r#"
{
  "events": [
    {"type": "dialogue", "speaker": "Ava", "text": "Hola mundo"}
  ],
  "labels": {"start": 0}
}
"#;

let script = Script::from_json(script_json)?;
let engine = Engine::new(script, SecurityPolicy::default(), ResourceLimiter::default())?;
let output = engine.render_current(&TextRenderer)?;
println!("{}", output.text);
```

## Políticas de seguridad y límites

La validación comprueba:

- Existencia de la etiqueta `start`.
- Longitud de textos, etiquetas y assets.
- Índices válidos de etiquetas.
- Opciones de elección no vacías y con targets válidos.

Puedes ajustar límites creando tu propio `ResourceLimiter` o relajar ciertas reglas con `SecurityPolicy`.

## Bindings de Python

Con la feature `python` se expone `PyEngine`:

```bash
maturin develop --features python
```

Ejemplo:

```python
from visual_novel_engine import PyEngine

engine = PyEngine(script_json)
print(engine.current_event())
print(engine.step())
print(engine.choose(0))
```

Más ejemplos en `examples/python`.

## Estructura del código

- `src/engine.rs`: núcleo del motor y navegación de eventos.
- `src/script.rs`: carga y validación de guiones JSON.
- `src/event.rs`: definición de eventos y serialización.
- `src/visual.rs`: estado visual (fondo, música, personajes).
- `src/render.rs`: interfaz de renderizado y `TextRenderer`.
- `src/security.rs`: política de validación y reglas.
- `src/resource.rs`: límites de recursos.
- `src/error.rs`: errores con diagnósticos.
- `src/state.rs`: estado interno del motor.

## Reporte automático de líneas

El repositorio incluye un comando para generar un reporte de líneas por lenguaje usando la librería `tokei` y formateado con `tabled`.

```bash
cargo run --bin repo_report -- --output docs/line_report.md
```

El reporte queda en `docs/line_report.md` y puede ejecutarse nuevamente cada vez que se necesite actualizar.
