#!/bin/bash
# Script para ejecutar todos los tests del proyecto

set -e

echo "=== Ejecutando tests de Rust ==="
cargo test --verbose

echo ""
echo "=== Ejecutando tests de Rust con feature Python (embed) ==="
# python-embed habilita PyO3 auto-initialize para tests Rust que embeben CPython.
export LD_LIBRARY_PATH="$(python -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
cargo test --features python-embed --verbose

echo ""
echo "=== Verificando formato del código ==="
cargo fmt -- --check || {
    echo "Error: El código no está formateado correctamente."
    echo "Ejecuta 'cargo fmt' para arreglarlo."
    exit 1
}

echo ""
echo "=== Ejecutando Clippy ==="
cargo clippy --all-features -- -D warnings

echo ""
echo "=== Construyendo extensión de Python ==="
# python habilita pyo3/extension-module para construir el módulo Python vía maturin.
if [ ! -d ".venv" ]; then
    python -m venv .venv
fi

source .venv/bin/activate

if ! command -v maturin &> /dev/null; then
    echo "maturin no instalado, instalando..."
    python -m pip install --upgrade pip
    python -m pip install maturin
fi

maturin develop --features python

echo ""
echo "=== Ejecutando tests de Python ==="
python -m unittest tests.python.test_examples -v

echo ""
echo "✅ Todos los tests pasaron exitosamente!"
