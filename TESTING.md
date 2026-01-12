# Guia de Testing

## CI/CD Automatico

Los tests se ejecutan automaticamente en GitHub Actions.

Ver .github/workflows/tests.yml para detalles.

## Comandos recomendados

- Ejecutar tests del core:
  `cargo test -p visual_novel_engine --verbose`
- Smoke de benches del core (Criterion):
  `cargo bench -p visual_novel_engine --bench core_benches -- --warm-up-time 0.1 --measurement-time 0.1 --sample-size 10`
