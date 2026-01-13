use visual_novel_engine::{ScriptRaw, VnError, SCRIPT_SCHEMA_VERSION};

#[test]
fn script_json_requires_schema_version() {
    let script_json = r#"{
        "events": [],
        "labels": {"start": 0}
    }"#;

    let err = ScriptRaw::from_json(script_json).expect_err("should reject missing schema");
    assert!(matches!(err, VnError::InvalidScript(_)));
}

#[test]
fn script_json_rejects_incompatible_schema_version() {
    let script_json = r#"{
        "script_schema_version": "9.9",
        "events": [],
        "labels": {"start": 0}
    }"#;

    let err = ScriptRaw::from_json(script_json).expect_err("should reject bad schema");
    match err {
        VnError::InvalidScript(message) => {
            assert!(message.contains("schema incompatible"));
            assert!(message.contains(SCRIPT_SCHEMA_VERSION));
        }
        _ => panic!("expected schema error"),
    }
}
