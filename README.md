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
- **Branching y variables**: Condiciones (`jump_if`) y variables enteras (`set_var`) con comparadores.
- **Estado Visual**: Mantiene fondo, música y personajes acumulados.
- **Interfaz Gráfica Nativa**: Visualizador completo con `eframe` (egui).
- **Persistencia**: Guardados binarios con `script_id` (SHA-256) y verificación de integridad.
- **Historial de Diálogo**: Backlog navegable de los últimos 200 mensajes.
- **Inspector de Depuración**: Herramienta en tiempo real para modificar banderas y saltar etiquetas.
- **Bindings Python**: Usa el motor desde Python con `pyo3`.
- **AssetStore**: Carga de assets con saneamiento de rutas, límites y manifest opcional.

## Instalación

### Solo el núcleo (sin GUI)

```toml
[dependencies]
visual_novel_engine = { path = "crates/core" }
```

### Instalación Automática (Windows)

Ejecuta el script incluido para compilar Rust e instalar los bindings de Python:

```powershell
.\install.ps1
```

### Compilación Manual

1. **Rust (Core + GUI)**: `cargo build --release`
2. **Python Bindings**:
   ```bash
   pip install maturin
   maturin build --manifest-path crates/py/Cargo.toml --release
   pip install target/wheels/*.whl --force-reinstall
   ```

### Con interfaz gráfica

```toml
[dependencies]
visual_novel_gui = { path = "crates/gui" }
```

## Prerrequisitos y Configuración Local

Para compilar y ejecutar el proyecto correctamente en tu entorno local, asegúrate de instalar:

### Windows

1.  **Rust**: Usando `rustup`.
2.  **C++ Build Tools**: A través de Visual Studio Installer (necesario para el enlazado).
3.  **Drivers de GPU**: Asegúrate de tener drivers compatibles con **Vulkan**, **DirectX 12** o **DirectX 11**.
    - Si no tienes GPU dedicada, el motor usará el fallback de Software automáticamente.
4.  **Python 3.10+**: Necesario si planeas compilar o probar los bindings de Python (`crates/py`).

### Testing

El proyecto incluye una suite de pruebas completa:

```bash
# Ejecutar verificación de compilación (Rápido)
cargo check --workspace --tests

# Ejecutar todos los tests (Unitarios + Integración + Snapshots)
# Nota: Puede fallar en entornos sin librerías gráficas o de python enlazadas.
cargo test --workspace
```

## Uso rápido (Rust)

### Solo lógica (sin ventana)

```rust
use visual_novel_engine::{Engine, Script, SecurityPolicy, ResourceLimiter};

let script_json = r#"
{
  "script_schema_version": "1.0",
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

- `script_schema_version`: versión del esquema JSON del guion.
- `events`: lista de eventos.
- `labels`: mapa de etiquetas a índices (`start` es obligatorio).

```json
{"type": "dialogue", "speaker": "Ava", "text": "Hola"}
{"type": "choice", "prompt": "¿Ir?", "options": [{"text": "Sí", "target": "end"}]}
{"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [{"name": "Ava", "expression": "smile", "position": "center"}]}
{"type": "jump", "target": "intro"}
{"type": "set_flag", "key": "visited", "value": true}
{"type": "set_var", "key": "counter", "value": 3}
{"type": "jump_if", "cond": {"kind": "var_cmp", "key": "counter", "op": "gt", "value": 1}, "target": "high"}
{"type": "patch", "background": "bg/night.png", "add": [{"name": "Ava", "expression": "smile", "position": "left"}], "update": [], "remove": []}
```

## Sistema de Guardado

El motor incluye persistencia segura:

- **Identidad de Script**: Cada save guarda el `script_id` (SHA-256 del binario compilado).
- **Validación al Cargar**: Si el guion cambió, el save se rechaza para evitar corrupción.
- **Formato binario**: Los saves usan un formato binario canónico con versión y checksum.

## Herramientas de Desarrollo

### Inspector (`F12`)

Ventana de depuración para desarrolladores:

- Ver y modificar **banderas** en tiempo real.
- Saltar a cualquier **etiqueta** del guion.
- Monitorear **FPS** y uso de memoria del historial.

### CLI (`vnengine`)

Comandos principales para QA y herramientas internas:

```bash
vnengine validate script.json
vnengine compile script.json -o script.vnsc
vnengine trace script.json --steps 50 -o trace.yaml
vnengine verify-save save.vns --script script.vnsc
vnengine manifest assets/ -o manifest.json
```

## Bindings de Python

### Instalación

```bash
pip install visual_novel_engine --find-links=target/wheels
```

> Nota: Asegúrate de haber ejecutado `install.ps1` o construido con `maturin` primero.

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

## Seguridad y modos de ejecución

El motor soporta dos modos:

- **Trusted** (default): scripts/assets confiables.
- **Untrusted**: valida rutas, tamaños y hashes de assets (manifest opcional).
