#[cfg(any(feature = "python", feature = "python-embed"))]
mod python_bindings {
    use pyo3::prelude::*;
    use pyo3::types::PyDict;

    use visual_novel_engine::{PyEngine, VnError};

    fn sample_script() -> String {
        r#"{
            "script_schema_version": "1.0",
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

    fn event_to_dict(event: &Bound<'_, PyAny>) -> Py<PyDict> {
        if let Ok(dict) = event.downcast::<PyDict>() {
            return dict.to_owned().into();
        }
        let dict = event.call_method0("to_dict").unwrap();
        dict.downcast::<PyDict>().unwrap().to_owned().into()
    }

    #[test]
    fn python_engine_exposes_methods() {
        Python::with_gil(|py| {
            let engine = Py::new(py, PyEngine::new_from_json(&sample_script()).unwrap()).unwrap();
            let engine_ref = engine.bind(py);
            let event = engine_ref.call_method0("current_event").unwrap();
            let dict = event_to_dict(&event);
            let dict = dict.bind(py);
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

    #[test]
    fn python_events_include_legacy_keys() {
        Python::with_gil(|py| {
            let choice_script = r#"{
                "script_schema_version": "1.0",
                "events": [
                    {"type": "choice", "prompt": "Go?", "options": [
                        {"text": "Yes", "target": "end"}
                    ]},
                    {"type": "dialogue", "speaker": "Ava", "text": "Done"}
                ],
                "labels": {"start": 0, "end": 1}
            }"#;
            let engine = Py::new(py, PyEngine::new_from_json(choice_script).unwrap()).unwrap();
            let event = engine.bind(py).call_method0("current_event").unwrap();
            let dict = event_to_dict(&event);
            let dict = dict.bind(py);
            let options = dict.get_item("options").unwrap().unwrap();
            let option_list = options.downcast::<pyo3::types::PyList>().unwrap();
            let option_item = option_list.get_item(0).unwrap();
            let first_option = option_item.downcast::<PyDict>().unwrap();
            assert!(first_option.get_item("target").unwrap().is_some());
            assert!(first_option.get_item("target_ip").unwrap().is_some());

            let jump_script = r#"{
                "script_schema_version": "1.0",
                "events": [
                    {"type": "jump", "target": "next"},
                    {"type": "dialogue", "speaker": "Ava", "text": "Next"}
                ],
                "labels": {"start": 0, "next": 1}
            }"#;
            let engine = Py::new(py, PyEngine::new_from_json(jump_script).unwrap()).unwrap();
            let event = engine.bind(py).call_method0("current_event").unwrap();
            let dict = event_to_dict(&event);
            let dict = dict.bind(py);
            assert!(dict.get_item("target").unwrap().is_some());
            assert!(dict.get_item("target_ip").unwrap().is_some());

            let flag_script = r#"{
                "script_schema_version": "1.0",
                "events": [
                    {"type": "set_flag", "key": "seen", "value": true}
                ],
                "labels": {"start": 0}
            }"#;
            let engine = Py::new(py, PyEngine::new_from_json(flag_script).unwrap()).unwrap();
            let event = engine.bind(py).call_method0("current_event").unwrap();
            let dict = event_to_dict(&event);
            let dict = dict.bind(py);
            assert!(dict.get_item("key").unwrap().is_some());
            assert!(dict.get_item("flag_id").unwrap().is_some());
        });
    }
}
