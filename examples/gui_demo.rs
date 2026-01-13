use visual_novel_gui::{run_app, VnConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let script_json = r#"
    {
      "events": [
        {"type": "dialogue", "speaker": "System", "text": "Bienvenido a la demo gráfica."}
      ],
      "labels": {"start": 0}
    }
    "#;

    let config = VnConfig {
        title: "Demo Visual Novel".to_string(),
        width: Some(1280.0),
        height: Some(720.0),
        ..Default::default()
    };

    run_app(script_json.to_string(), Some(config))?;
    Ok(())
}
