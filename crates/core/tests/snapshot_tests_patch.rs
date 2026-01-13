mod common;
use common::run_headless;

#[test]
fn golden_script_incremental_patch() {
    let script_json = r#"
    {
        "events": [
            {
                "type": "scene",
                "background": "bg_city",
                "music": "music_day",
                "characters": [
                    {"name": "Alice", "expression": "smile", "position": "left"}
                ]
            },
            {"type": "dialogue", "speaker": "Alice", "text": "Hello!"},
            {
                "type": "patch",
                "characters": [
                    {"name": "Alice", "expression": "surprised", "position": "left"},
                    {"name": "Bob", "expression": "neutral", "position": "right"}
                ]
            },
            {"type": "dialogue", "speaker": "Bob", "text": "Hi Alice."},
            {
                "type": "patch",
                "background": "bg_night"
            },
            {"type": "dialogue", "speaker": "Alice", "text": "It got dark."},
            {
                "type": "patch",
                "music": "music_night",
                "characters": []
            },
            {"type": "dialogue", "speaker": "Narrator", "text": "They left."}
        ],
        "labels": {
            "start": 0
        }
    }
    "#;

    let trace = run_headless(script_json, 15);
    insta::assert_yaml_snapshot!(trace);
}
