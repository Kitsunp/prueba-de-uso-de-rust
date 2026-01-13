# Guía de Testing

## CI/CD Automático

Los tests se ejecutan automáticamente en GitHub Actions.

Ver `.github/workflows/tests.yml` para detalles.

## Comandos de Test

### Core (Lógica del Motor)

```bash
# Tests unitarios
cargo test -p visual_novel_engine --verbose

# Benchmarks (Criterion)
cargo bench -p visual_novel_engine --bench core_benches
```

### GUI (Interfaz Gráfica)

```bash
# Tests unitarios de configuración
cargo test -p visual_novel_gui --verbose
```

### Python Bindings

```bash
# Requiere maturin instalado
maturin develop --features python

# Ejecutar tests de Python
python -m pytest tests/python/ -v
```

## Tests Manuales Recomendados

1. **GUI Básica**: Ejecutar `cargo run --example gui_demo` y verificar que la ventana abre correctamente.
2. **Guardado/Carga**: Usar el menú (`ESC`) para guardar, cerrar, reabrir y cargar la partida.
3. **Inspector**: Presionar `F12` y modificar una bandera; verificar que el cambio persiste.
4. **Historial**: Avanzar varios diálogos y abrir el historial para verificar que se registran.

## Estructura de Tests

```
tests/
├── python/
│   └── test_vnengine.py    # Tests de integración Python
crates/
├── core/
│   └── src/lib.rs          # Tests unitarios inline (#[cfg(test)])
└── gui/
    └── src/lib.rs          # Tests de configuración
```
