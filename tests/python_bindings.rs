#[cfg(any(feature = "python", feature = "python-embed"))]
mod python_bindings {
    use pyo3::prelude::*;
    use pyo3::types::PyDict;

    use visual_novel_engine::{PyEngine, VnError};

    fn sample_script() -> String {
        r#"{
            "events": [
                {"type": "scene", "background": "bg/room.png", "music": "music/theme.ogg", "characters": [
                    {"name": "Ava", "expression": "smile", "position": "center"}
                ]},
                {"type": "dialogue", "speaker": "Ava", "text": "Hola"}
            ],
            "labels": {"start": 0}
        }"#
            .to_string()
    }

    #[test]
    fn python_engine_exposes_methods() {
        Python::with_gil(|py| {
            let engine = Py::new(py, PyEngine::new_from_json(&sample_script()).unwrap()).unwrap();
            let engine_ref = engine.bind(py);
            let event = engine_ref.call_method0("current_event").unwrap();
            let dict = event.downcast::<PyDict>().unwrap();
            let event_type: String = dict.get_item("type").unwrap().unwrap().extract().unwrap();
            assert_eq!(event_type, "scene");

            let state = engine_ref.call_method0("visual_state").unwrap();
            let state_dict = state.downcast::<PyDict>().unwrap();
            let background: Option<String> = state_dict
                .get_item("background")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(background.as_deref(), Some("bg/room.png"));
        });
    }

    #[test]
    fn python_engine_reports_diagnostics() {
        let broken_json = "{\n  \"events\": [\n    {\"type\": \"dialogue\"}\n  ],\n  \"labels\": {\"start\": 0}\n";
        let err = PyEngine::new_from_json(broken_json)
            .err()
            .expect("should fail");
        match err {
            VnError::Serialization { message, .. } => {
                let lowered = message.to_lowercase();
                assert!(
                    lowered.contains("eof")
                        || lowered.contains("expected")
                        || lowered.contains("at line")
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }

        Python::with_gil(|_py| {
            let result = PyEngine::new(broken_json);
            let py_err = result.err().expect("should be error");
            let message = py_err.to_string();
            assert!(message.contains("serialization"));
        });
    }
}
