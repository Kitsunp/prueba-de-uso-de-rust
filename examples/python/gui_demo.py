import visual_novel_engine as vn

script_json = """
{
  "events": [
    {"type": "dialogue", "speaker": "System", "text": "Bienvenido a la demo gráfica."}
  ],
  "labels": {"start": 0}
}
"""

config = vn.VnConfig(width=1024.0, height=768.0, fullscreen=False)

print("Iniciando novela visual...")
try:
    vn.run_visual_novel(script_json, config)
    print("Ejecución finalizada.")
except Exception as exc:
    print(f"Error: {exc}")
