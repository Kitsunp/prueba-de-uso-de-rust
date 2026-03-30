use super::import_renpy_project;
use super::syntax::{
    parse_cond_expr, parse_dialogue_line, parse_menu_option_decl, parse_show_decl,
};
use super::{ImportProfile, ImportRenpyOptions};
use crate::{CmpOp, CondRaw, EventRaw, ScriptRaw};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub(crate) fn temp_renpy_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");
    let output_root = dir.path().join("out_project");
    (dir, project_root, game_dir, output_root)
}

pub(crate) fn write_renpy_file(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write script");
}
#[test]
fn parse_condition_supports_var_cmp() {
    let cond = parse_cond_expr("score >= 10").expect("cond");
    match cond {
        CondRaw::VarCmp { key, op, value } => {
            assert_eq!(key, "score");
            assert_eq!(op, CmpOp::Ge);
            assert_eq!(value, 10);
        }
        _ => panic!("expected var cmp"),
    }
}
#[test]
fn parse_menu_option_without_cond() {
    let (text, cond) = parse_menu_option_decl("\"Go\":").expect("menu option");
    assert_eq!(text, "Go");
    assert!(cond.is_none());
}

#[test]
fn parse_dialogue_alias_resolution() {
    let mut aliases = std::collections::HashMap::new();
    aliases.insert("e".to_string(), "Eileen".to_string());
    let dialogue = parse_dialogue_line("e \"Hello\"", &aliases).expect("dialogue");
    assert_eq!(dialogue.speaker, "Eileen");
    assert_eq!(dialogue.text, "Hello");
}

#[test]
fn parse_dialogue_single_quotes() {
    let aliases = std::collections::HashMap::new();
    let dialogue = parse_dialogue_line("e 'Hola'", &aliases).expect("dialogue");
    assert_eq!(dialogue.speaker, "e");
    assert_eq!(dialogue.text, "Hola");
}

#[test]
fn parse_show_bg_alias_maps_to_background_patch() {
    let mut aliases = std::collections::HashMap::new();
    aliases.insert("bg street".to_string(), "images/bg/street.png".to_string());
    let parsed = parse_show_decl("show bg street", &aliases).expect("show parse");
    assert_eq!(
        parsed.patch.background.as_deref(),
        Some("images/bg/street.png")
    );
    assert!(parsed.patch.add.is_empty());
}

#[test]
fn import_writes_project_files_and_valid_script() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");

    let script = r#"
label start:
    "Hello"
    menu:
        "Go":
            jump end_route
        "Stay":
            jump start

label end_route:
    "End"
"#;
    fs::write(game_dir.join("script.rpy"), script).expect("write script");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root: project_root.clone(),
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    assert!(output_root.join("main.json").exists());
    assert!(output_root.join("project.vnm").exists());
    assert!(output_root.join("import_report.json").exists());
    assert!(report.events_generated >= 3);

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse imported script");
    assert!(script.labels.contains_key("start"));
    assert!(script.compile().is_ok(), "imported script must compile");
}

#[test]
fn import_degrades_unsupported_statements_to_ext_call() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");

    let script = r#"
label start:
    call route_a
    queue music "audio/theme.ogg"
    return

label route_a:
    "X"
"#;
    fs::write(game_dir.join("script.rpy"), script).expect("write script");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    assert!(report.degraded_events >= 2);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "unsupported_call"));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "unsupported_audio_queue"));

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    assert!(script
        .events
        .iter()
        .any(|event| matches!(event, EventRaw::ExtCall { .. })));
}

#[test]
fn import_patches_missing_targets_and_keeps_script_compileable() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");

    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    jump nowhere
"#,
    )
    .expect("write");

    let output_root = dir.path().join("out_project");
    let _ = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    assert!(script.labels.contains_key("nowhere"));
    assert!(script.compile().is_ok());
}

#[test]
fn import_rewrites_and_copies_assets() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(game_dir.join("bg")).expect("mkdir bg");
    fs::write(game_dir.join("bg").join("room.png"), b"img").expect("write asset");
    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    scene "bg/room.png"
"#,
    )
    .expect("write script");

    let output_root = dir.path().join("out_project");
    import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    assert!(output_root
        .join("assets")
        .join("bg")
        .join("room.png")
        .exists());
    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    let scene_bg = script.events.iter().find_map(|event| match event {
        EventRaw::Scene(scene) => scene.background.clone(),
        _ => None,
    });
    assert_eq!(scene_bg.as_deref(), Some("assets/bg/room.png"));
}

#[test]
fn import_collapses_unsupported_blocks_into_single_event() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");

    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    python:
        score = 1
        points = score + 1
    play music 'audio/theme.ogg'
    "After"
"#,
    )
    .expect("write script");
    fs::create_dir_all(game_dir.join("audio")).expect("mkdir audio");
    fs::write(game_dir.join("audio").join("theme.ogg"), b"snd").expect("write audio");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    let unexpected_indent = report
        .issues_by_code
        .get("unexpected_indent")
        .copied()
        .unwrap_or(0);
    assert_eq!(unexpected_indent, 0);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unsupported_block_statement"),
        "unsupported blocks must be reported once"
    );

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    assert!(script.compile().is_ok());
    let audio_asset = script.events.iter().find_map(|event| match event {
        EventRaw::AudioAction(action) => action.asset.clone(),
        _ => None,
    });
    assert_eq!(audio_asset.as_deref(), Some("assets/audio/theme.ogg"));
}

#[test]
fn import_issues_include_traceability_fields() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");
    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    return
"#,
    )
    .expect("write script");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root,
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    assert!(
        !report.issues.is_empty(),
        "import should produce diagnostics"
    );
    let traces: BTreeSet<String> = report
        .issues
        .iter()
        .map(|issue| issue.trace_id.clone())
        .collect();
    assert_eq!(traces.len(), report.issues.len(), "trace_id must be unique");
    assert!(report
        .issues
        .iter()
        .all(|issue| issue.trace_id.starts_with("imp-")));
    assert!(report
        .issues
        .iter()
        .all(|issue| !issue.root_cause.trim().is_empty()));
    assert!(report
        .issues
        .iter()
        .all(|issue| issue.root_cause.contains("area=") && issue.root_cause.contains("phase=")));
    assert!(report
        .issues
        .iter()
        .all(|issue| !issue.how_to_fix.trim().is_empty()));
    assert!(report
        .issues
        .iter()
        .all(|issue| !issue.docs_ref.trim().is_empty()));
    assert!(report
        .issues
        .iter()
        .all(|issue| issue.docs_ref.ends_with(&issue.code)));
}

#[test]
fn ext_call_events_are_decorated_with_trace_envelope_v2() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");
    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    call route_a
    queue music "audio/theme.ogg"
    return
"#,
    )
    .expect("write script");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    let fallback_issue_traces: BTreeSet<String> = report
        .issues
        .iter()
        .filter(|issue| issue.fallback_applied.as_deref() == Some("event_raw.ext_call"))
        .map(|issue| issue.trace_id.clone())
        .collect();
    assert!(
        !fallback_issue_traces.is_empty(),
        "expected ext_call fallback issues"
    );

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    let mut decorated_count = 0usize;
    for (event_ip, event) in script.events.iter().enumerate() {
        let EventRaw::ExtCall { command, args } = event else {
            continue;
        };
        decorated_count += 1;
        assert_eq!(command, "vn.import.renpy.ext_v2");
        assert!(
            !args.is_empty(),
            "decorated ext_call must include envelope as first arg"
        );
        let envelope: serde_json::Value =
            serde_json::from_str(&args[0]).expect("valid extcall envelope json");
        assert_eq!(
            envelope.get("schema").and_then(serde_json::Value::as_str),
            Some("vn.import.trace.extcall.v2")
        );
        let trace_id = envelope
            .get("trace_id")
            .and_then(serde_json::Value::as_str)
            .expect("trace_id");
        assert!(
            fallback_issue_traces.contains(trace_id),
            "every ext_call envelope trace_id must map to an issue"
        );
        assert_eq!(
            envelope
                .get("event_ip")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            Some(event_ip)
        );
        assert!(
            envelope
                .get("action_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.starts_with("renpy.")),
            "action_id should be canonicalized for imported behavior"
        );
        let payload_len = envelope
            .get("payload")
            .and_then(serde_json::Value::as_array)
            .map(|payload| payload.len())
            .unwrap_or(0);
        assert_eq!(
            args.len().saturating_sub(1),
            payload_len,
            "raw payload should be preserved after envelope arg"
        );
    }

    assert!(
        decorated_count >= 3,
        "expected decorated call/queue/return extcalls"
    );
}

#[test]
fn degraded_events_keep_one_to_one_issue_traceability() {
    let dir = tempdir().expect("tempdir");
    let project_root = dir.path().join("renpy_project");
    let game_dir = project_root.join("game");
    fs::create_dir_all(&game_dir).expect("mkdir game");
    fs::write(
        game_dir.join("script.rpy"),
        r#"
label start:
    call route_a
    queue music "audio/theme.ogg"
    return
    $ score += 1
    python:
        x = 1
"#,
    )
    .expect("write script");

    let output_root = dir.path().join("out_project");
    let report = import_renpy_project(ImportRenpyOptions {
        project_root,
        output_root: output_root.clone(),
        entry_label: "start".to_string(),
        report_path: None,
        profile: ImportProfile::StoryFirst,
        include_tl: None,
        include_ui: None,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        strict_mode: false,
        fallback_policy: super::ImportFallbackPolicy::DegradeWithTrace,
    })
    .expect("import");

    let json = fs::read_to_string(output_root.join("main.json")).expect("read main");
    let script = ScriptRaw::from_json(&json).expect("parse script");
    let ext_calls = script
        .events
        .iter()
        .filter(|event| matches!(event, EventRaw::ExtCall { .. }))
        .count();
    let fallback_issues = report
        .issues
        .iter()
        .filter(|issue| issue.fallback_applied.as_deref() == Some("event_raw.ext_call"))
        .count();

    assert_eq!(report.degraded_events, ext_calls);
    assert_eq!(fallback_issues, ext_calls);
    assert!(
        report
            .issues_by_code
            .get("unsupported_block_statement")
            .copied()
            .unwrap_or(0)
            >= 1
    );
}
