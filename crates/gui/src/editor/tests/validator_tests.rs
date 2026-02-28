use super::*;
use crate::editor::node_graph::NodeGraph;
use crate::editor::node_types::StoryNode;
use eframe::egui;

fn p(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(x, y)
}

#[test]
fn diagnostic_id_is_stable_and_includes_phase_code_node_and_ip() {
    let issue = LintIssue::warning(
        Some(7),
        ValidationPhase::Graph,
        LintCode::UnreachableNode,
        "dead code",
    );
    assert_eq!(issue.diagnostic_id(), "GRAPH:VAL_UNREACHABLE:7:na");

    let issue = issue.with_event_ip(Some(3));
    assert_eq!(issue.diagnostic_id(), "GRAPH:VAL_UNREACHABLE:7:3");
}

#[test]
fn validate_reports_choice_unlinked_with_explicit_code() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let choice = graph.add_node(
        StoryNode::Choice {
            prompt: "Choose".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
        },
        p(0.0, 100.0),
    );
    graph.connect(start, choice);

    let issues = validate(&graph);
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::ChoiceOptionUnlinked));
    assert!(issues.iter().any(|i| i.phase == ValidationPhase::Graph));
}

#[test]
fn validate_reports_unsafe_asset_paths_and_transition_duration() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let scene = graph.add_node(
        StoryNode::Scene {
            profile: None,
            background: Some("../secrets/bg.png".to_string()),
            music: None,
            characters: Vec::new(),
        },
        p(0.0, 80.0),
    );
    let transition = graph.add_node(
        StoryNode::Transition {
            kind: "unknown".to_string(),
            duration_ms: 0,
            color: None,
        },
        p(0.0, 160.0),
    );
    let end = graph.add_node(StoryNode::End, p(0.0, 240.0));
    graph.connect(start, scene);
    graph.connect(scene, transition);
    graph.connect(transition, end);

    let issues = validate(&graph);
    assert!(issues.iter().any(|i| i.code == LintCode::UnsafeAssetPath));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::InvalidTransitionDuration));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::InvalidTransitionKind));
}

#[test]
fn validate_reports_empty_character_name() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let placement = graph.add_node(
        StoryNode::CharacterPlacement {
            name: "".to_string(),
            x: 10,
            y: 10,
            scale: Some(1.0),
        },
        p(0.0, 100.0),
    );
    let end = graph.add_node(StoryNode::End, p(0.0, 200.0));
    graph.connect(start, placement);
    graph.connect(placement, end);

    let issues = validate(&graph);
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::EmptyCharacterName));
}

#[test]
fn validate_reports_missing_assets_when_probe_fails() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let scene = graph.add_node(
        StoryNode::Scene {
            profile: None,
            background: Some("assets/bg_forest.png".to_string()),
            music: None,
            characters: Vec::new(),
        },
        p(0.0, 100.0),
    );
    let end = graph.add_node(StoryNode::End, p(0.0, 200.0));
    graph.connect(start, scene);
    graph.connect(scene, end);

    let issues = validate_with_asset_probe(&graph, |_asset| false);
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::AssetReferenceMissing));
}

#[test]
fn validate_reports_invalid_audio_params() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let audio = graph.add_node(
        StoryNode::AudioAction {
            channel: "music".to_string(),
            action: "boom".to_string(),
            asset: Some("assets/sfx/beep.wav".to_string()),
            volume: Some(1.5),
            fade_duration_ms: Some(0),
            loop_playback: Some(true),
        },
        p(0.0, 100.0),
    );
    let end = graph.add_node(StoryNode::End, p(0.0, 200.0));
    graph.connect(start, audio);
    graph.connect(audio, end);

    let issues = validate_with_asset_probe(&graph, |_asset| true);
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::InvalidAudioChannel));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::InvalidAudioAction));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::InvalidAudioVolume));
}

#[test]
fn validate_reports_scene_patch_and_generic_limits() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
    let patch = graph.add_node(
        StoryNode::ScenePatch(visual_novel_engine::ScenePatchRaw {
            background: Some("../unsafe/bg.png".to_string()),
            music: None,
            add: vec![visual_novel_engine::CharacterPlacementRaw {
                name: "".to_string(),
                ..Default::default()
            }],
            update: Vec::new(),
            remove: Vec::new(),
        }),
        p(0.0, 100.0),
    );
    let generic = graph.add_node(
        StoryNode::Generic(visual_novel_engine::EventRaw::ExtCall {
            command: "mod_hook".to_string(),
            args: vec!["x".to_string()],
        }),
        p(0.0, 200.0),
    );
    let end = graph.add_node(StoryNode::End, p(0.0, 300.0));

    graph.connect(start, patch);
    graph.connect(patch, generic);
    graph.connect(generic, end);

    let issues = validate_with_asset_probe(&graph, |_asset| true);
    assert!(issues.iter().any(|i| i.code == LintCode::UnsafeAssetPath));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::GenericEventUnchecked));
    assert!(issues
        .iter()
        .any(|i| i.code == LintCode::ContractUnsupportedExport));
}
