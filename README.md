# Visual Novel Engine (Rust)

Motor completo para novelas visuales basado en eventos. Permite interpretar un guion en JSON, avanzar por los eventos, tomar decisiones, guardar/cargar partidas y visualizar la historia con una interfaz gráfica nativa.

## Contenido

- [Características](#características)
- [Instalación](#instalación)
- [Uso rápido (Rust)](#uso-rápido-rust)
- [Interfaz Gráfica (GUI)](#interfaz-gráfica-gui)
- [Formato del guion](#formato-del-guion)
- [Sistema de Guardado](#sistema-de-guardado)
- [Herramientas de Desarrollo](#herramientas-de-desarrollo)
- [Bindings de Python](#bindings-de-python)
- [Estructura del código](#estructura-del-código)

## Características

- **Motor Lógico**: Eventos de diálogo, escena, elecciones, saltos y banderas.
- **Estado Visual**: Mantiene fondo, música y personajes acumulados.
- **Interfaz Gráfica Nativa**: Visualizador completo con `eframe` (egui).
- **Persistencia**: Sistema de guardado/carga con verificación de integridad (checksum).
- **Historial de Diálogo**: Backlog navegable de los últimos 200 mensajes.
- **Inspector de Depuración**: Herramienta en tiempo real para modificar banderas y saltar etiquetas.
- **Bindings Python**: Usa el motor desde Python con `pyo3`.

## Instalación

### Solo el núcleo (sin GUI)

```toml
[dependencies]
visual_novel_engine = { path = "crates/core" }
```

### Con interfaz gráfica

```toml
[dependencies]
visual_novel_gui = { path = "crates/gui" }
```

## Uso rápido (Rust)

### Solo lógica (sin ventana)

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

println!("Evento actual: {:?}", engine.current_event()?);
engine.step()?;
engine.choose(0)?; // Elige la primera opción
```

### Con interfaz gráfica

```rust
use visual_novel_gui::{run_app, VnConfig};

let script_json = include_str!("mi_historia.json");

let config = VnConfig {
    title: "Mi Novela Visual".to_string(),
    width: Some(1280.0),
    height: Some(720.0),
    ..Default::default()
};

run_app(script_json.to_string(), Some(config))?;
```

## Interfaz Gráfica (GUI)

La GUI proporciona una experiencia completa de novela visual:

- **Renderizado de Escenas**: Muestra fondos, personajes y música.
- **Caja de Diálogo**: Presenta el texto y opciones de forma interactiva.
- **Menú de Configuración** (`ESC`): Ajusta escala de UI, pantalla completa y VSync.
- **Historial** (botón en UI): Revisa los últimos diálogos leídos.
- **Guardar/Cargar**: Desde el menú de configuración, usa diálogos de archivo nativos.

## Formato del guion

Un guion es un JSON con:

- `events`: lista de eventos.
- `labels`: mapa de etiquetas a índices (`start` es obligatorio).

```json
{"type": "dialogue", "speaker": "Ava", "text": "Hola"}
{"type": "choice", "prompt": "¿Ir?", "options": [{"text": "Sí", "target": "end"}]}
{"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [{"name": "Ava", "expression": "smile", "position": "center"}]}
{"type": "jump", "target": "intro"}
{"type": "set_flag", "key": "visited", "value": true}
```

## Sistema de Guardado

El motor incluye persistencia segura:

- **Checksum de Script**: Cada save guarda un hash del guion original.
- **Validación al Cargar**: Si el guion cambió, el save se rechaza para evitar corrupción.
- **Formato JSON**: Los saves son legibles y depurables.

## Herramientas de Desarrollo

### Inspector (`F12`)

Ventana de depuración para desarrolladores:

- Ver y modificar **banderas** en tiempo real.
- Saltar a cualquier **etiqueta** del guion.
- Monitorear **FPS** y uso de memoria del historial.

## Bindings de Python

### Instalación

```bash
maturin develop --features python
```

### Uso básico (solo lógica)

```python
from visual_novel_engine import PyEngine

engine = PyEngine(script_json)
print(engine.current_event())
engine.step()
engine.choose(0)
```

### Con interfaz gráfica

```python
import visual_novel_engine as vn

config = vn.VnConfig(width=1280.0, height=720.0)
vn.run_visual_novel(script_json, config)
```

## Estructura del código

- `crates/core/`: Núcleo del motor (lógica, compilación, estado).
- `crates/gui/`: Interfaz gráfica con eframe.
- `crates/py/`: Bindings de Python.
- `examples/`: Ejemplos de uso en Rust y Python.
