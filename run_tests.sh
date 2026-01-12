#!/bin/bash
# Script para ejecutar todos los tests del proyecto

set -e

echo "=== Ejecutando tests de Rust ==="
cargo test --verbose

echo ""
echo "=== Ejecutando tests de Rust con feature Python ==="
cargo test --features python --verbose

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
if command -v maturin &> /dev/null; then
    maturin develop --features python
    
    echo ""
    echo "=== Ejecutando tests de Python ==="
    python -m unittest tests.python.test_examples -v
else
    echo "Advertencia: maturin no instalado, saltando tests de Python"
    echo "Instala con: pip install maturin"
fi

echo ""
echo "✅ Todos los tests pasaron exitosamente!"
