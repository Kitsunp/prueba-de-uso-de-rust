use std::collections::HashSet;
use std::path::Path;

use crate::editor::execution_contract;
use crate::editor::node_graph::NodeGraph;
use crate::editor::node_types::StoryNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPhase {
    Graph,
    Compile,
    Runtime,
    DryRun,
}

impl ValidationPhase {
    pub fn label(self) -> &'static str {
        match self {
            ValidationPhase::Graph => "GRAPH",
            ValidationPhase::Compile => "COMPILE",
            ValidationPhase::Runtime => "RUNTIME",
            ValidationPhase::DryRun => "DRYRUN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCode {
    MissingStart,
    MultipleStart,
    UnreachableNode,
    DeadEnd,
    ChoiceNoOptions,
    ChoiceOptionUnlinked,
    ChoicePortOutOfRange,
    AudioAssetMissing,
    AudioAssetEmpty,
    AssetReferenceMissing,
    SceneBackgroundEmpty,
    UnsafeAssetPath,
    InvalidAudioChannel,
    InvalidAudioAction,
    InvalidAudioVolume,
    InvalidAudioFade,
    InvalidCharacterScale,
    InvalidTransitionDuration,
    InvalidTransitionKind,
    EmptyCharacterName,
    EmptySpeakerName,
    EmptyJumpTarget,
    ContractUnsupportedExport,
    GenericEventUnchecked,
    CompileError,
    RuntimeInitError,
    DryRunUnreachableCompiled,
    DryRunStepLimit,
    DryRunRuntimeError,
    DryRunParityMismatch,
    DryRunFinished,
}

impl LintCode {
    pub fn label(self) -> &'static str {
        match self {
            LintCode::MissingStart => "VAL_START_MISSING",
            LintCode::MultipleStart => "VAL_START_MULTIPLE",
            LintCode::UnreachableNode => "VAL_UNREACHABLE",
            LintCode::DeadEnd => "VAL_DEAD_END",
            LintCode::ChoiceNoOptions => "VAL_CHOICE_EMPTY",
            LintCode::ChoiceOptionUnlinked => "VAL_CHOICE_UNLINKED",
            LintCode::ChoicePortOutOfRange => "VAL_CHOICE_PORT_OOB",
            LintCode::AudioAssetMissing => "VAL_AUDIO_MISSING",
            LintCode::AudioAssetEmpty => "VAL_AUDIO_EMPTY",
            LintCode::AssetReferenceMissing => "VAL_ASSET_NOT_FOUND",
            LintCode::SceneBackgroundEmpty => "VAL_SCENE_BG_EMPTY",
            LintCode::UnsafeAssetPath => "VAL_ASSET_UNSAFE_PATH",
            LintCode::InvalidAudioChannel => "VAL_AUDIO_CHANNEL_INVALID",
            LintCode::InvalidAudioAction => "VAL_AUDIO_ACTION_INVALID",
            LintCode::InvalidAudioVolume => "VAL_AUDIO_VOLUME_INVALID",
            LintCode::InvalidAudioFade => "VAL_AUDIO_FADE_INVALID",
            LintCode::InvalidCharacterScale => "VAL_SCALE_INVALID",
            LintCode::InvalidTransitionDuration => "VAL_TRANSITION_DURATION",
            LintCode::InvalidTransitionKind => "VAL_TRANSITION_KIND_INVALID",
            LintCode::EmptyCharacterName => "VAL_CHARACTER_NAME_EMPTY",
            LintCode::EmptySpeakerName => "VAL_SPEAKER_EMPTY",
            LintCode::EmptyJumpTarget => "VAL_JUMP_EMPTY",
            LintCode::ContractUnsupportedExport => "VAL_CONTRACT_EXPORT_UNSUPPORTED",
            LintCode::GenericEventUnchecked => "VAL_GENERIC_UNCHECKED",
            LintCode::CompileError => "CMP_SCRIPT_ERROR",
            LintCode::RuntimeInitError => "CMP_RUNTIME_INIT",
            LintCode::DryRunUnreachableCompiled => "DRY_UNREACHABLE",
            LintCode::DryRunStepLimit => "DRY_STEP_LIMIT",
            LintCode::DryRunRuntimeError => "DRY_RUNTIME_ERROR",
            LintCode::DryRunParityMismatch => "DRY_PARITY_MISMATCH",
            LintCode::DryRunFinished => "DRY_FINISHED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub node_id: Option<u32>,
    pub event_ip: Option<u32>,
    pub severity: LintSeverity,
    pub phase: ValidationPhase,
    pub code: LintCode,
    pub message: String,
}

impl LintIssue {
    pub fn diagnostic_id(&self) -> String {
        let node = self
            .node_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "global".to_string());
        let event_ip = self
            .event_ip
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "na".to_string());
        format!(
            "{}:{}:{}:{}",
            self.phase.label(),
            self.code.label(),
            node,
            event_ip
        )
    }

    pub fn new(
        node_id: Option<u32>,
        severity: LintSeverity,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            event_ip: None,
            severity,
            phase,
            code,
            message: message.into(),
        }
    }

    pub fn with_event_ip(mut self, event_ip: Option<u32>) -> Self {
        self.event_ip = event_ip;
        self
    }

    pub fn error(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Error, phase, code, message)
    }

    pub fn warning(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Warning, phase, code, message)
    }

    pub fn info(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Info, phase, code, message)
    }
}

pub fn validate(graph: &NodeGraph) -> Vec<LintIssue> {
    validate_with_asset_probe(graph, default_asset_exists)
}

pub fn validate_with_asset_probe<F>(graph: &NodeGraph, asset_exists: F) -> Vec<LintIssue>
where
    F: Fn(&str) -> bool,
{
    let mut issues = Vec::new();

    let start_nodes: Vec<u32> = graph
        .nodes
        .iter()
        .filter_map(|(id, node, _)| {
            if matches!(node, StoryNode::Start) {
                Some(*id)
            } else {
                None
            }
        })
        .collect();

    if start_nodes.is_empty() {
        issues.push(LintIssue::error(
            None,
            ValidationPhase::Graph,
            LintCode::MissingStart,
            "Missing Start node",
        ));
    } else if start_nodes.len() > 1 {
        issues.push(LintIssue::warning(
            None,
            ValidationPhase::Graph,
            LintCode::MultipleStart,
            format!("Multiple Start nodes found ({})", start_nodes.len()),
        ));
    }

    let mut visited: HashSet<u32> = HashSet::new();
    for id in &start_nodes {
        visit_node(graph, *id, &mut visited);
    }

    for (id, _, _) in &graph.nodes {
        if !visited.contains(id) {
            issues.push(LintIssue::warning(
                Some(*id),
                ValidationPhase::Graph,
                LintCode::UnreachableNode,
                "Unreachable node (dead code)",
            ));
        }
    }

    for (id, node, _) in &graph.nodes {
        let contract = execution_contract::contract_for_node(node);
        if !node.is_marker() && !contract.export_supported {
            issues.push(LintIssue::error(
                Some(*id),
                ValidationPhase::Graph,
                LintCode::ContractUnsupportedExport,
                format!(
                    "Event '{}' is not export-compatible (contract mismatch)",
                    contract.event_name
                ),
            ));
        }

        match node {
            StoryNode::Dialogue { speaker, .. } => {
                if speaker.trim().is_empty() {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptySpeakerName,
                        "Dialogue speaker is empty",
                    ));
                }
            }
            StoryNode::Scene { background } => {
                if background.trim().is_empty() {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::SceneBackgroundEmpty,
                        "Scene background path is empty",
                    ));
                } else if is_unsafe_asset_ref(background) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::UnsafeAssetPath,
                        format!("Unsafe background path: '{}'", background),
                    ));
                } else if should_probe_asset_exists(background) && !asset_exists(background) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::AssetReferenceMissing,
                        format!("Background asset does not exist: '{}'", background),
                    ));
                }
            }
            StoryNode::SetVariable { .. } => {}
            StoryNode::ScenePatch(patch) => {
                if let Some(bg) = &patch.background {
                    if is_unsafe_asset_ref(bg) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::UnsafeAssetPath,
                            format!("Unsafe scene patch background path: '{}'", bg),
                        ));
                    } else if should_probe_asset_exists(bg) && !asset_exists(bg) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AssetReferenceMissing,
                            format!("Scene patch background does not exist: '{}'", bg),
                        ));
                    }
                }

                if let Some(music) = &patch.music {
                    if is_unsafe_asset_ref(music) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::UnsafeAssetPath,
                            format!("Unsafe scene patch music path: '{}'", music),
                        ));
                    } else if should_probe_asset_exists(music) && !asset_exists(music) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AssetReferenceMissing,
                            format!("Scene patch music does not exist: '{}'", music),
                        ));
                    }
                }

                if patch.add.iter().any(|c| c.name.trim().is_empty()) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyCharacterName,
                        "Scene patch has add-entry with empty character name",
                    ));
                }
                if patch.update.iter().any(|c| c.name.trim().is_empty()) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyCharacterName,
                        "Scene patch has update-entry with empty character name",
                    ));
                }
                if patch.remove.iter().any(|name| name.trim().is_empty()) {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyCharacterName,
                        "Scene patch has empty character name in remove-list",
                    ));
                }
            }
            StoryNode::Generic(_) => {
                issues.push(LintIssue::warning(
                    Some(*id),
                    ValidationPhase::Graph,
                    LintCode::GenericEventUnchecked,
                    "Generic event has limited semantic validation",
                ));
            }
            StoryNode::Transition {
                kind, duration_ms, ..
            } => {
                if *duration_ms == 0 {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::InvalidTransitionDuration,
                        "Transition duration should be > 0 ms",
                    ));
                }
                if !is_valid_transition_kind(kind) {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::InvalidTransitionKind,
                        format!("Transition kind '{}' is not recognized", kind),
                    ));
                }
            }
            StoryNode::Jump { target } => {
                if target.trim().is_empty() {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyJumpTarget,
                        "Jump target is empty",
                    ));
                }
            }
            StoryNode::Choice { options, .. } => {
                if options.is_empty() {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::ChoiceNoOptions,
                        "Choice node has no options",
                    ));
                }

                for (idx, _) in options.iter().enumerate() {
                    if !graph
                        .connections
                        .iter()
                        .any(|c| c.from == *id && c.from_port == idx)
                    {
                        issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::ChoiceOptionUnlinked,
                            format!("Choice option {} has no outgoing connection", idx + 1),
                        ));
                    }
                }

                for conn in graph.connections.iter().filter(|c| c.from == *id) {
                    if conn.from_port >= options.len() {
                        issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::ChoicePortOutOfRange,
                            format!(
                                "Connection from invalid option port {} (options: {})",
                                conn.from_port,
                                options.len()
                            ),
                        ));
                    }
                }
            }
            StoryNode::AudioAction {
                channel,
                action,
                asset,
                volume,
                fade_duration_ms,
                ..
            } => {
                let normalized_channel = channel.trim().to_ascii_lowercase();
                let normalized_action = action.trim().to_ascii_lowercase();

                if !is_valid_audio_channel(&normalized_channel) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::InvalidAudioChannel,
                        format!("Invalid audio channel '{}'", channel),
                    ));
                }
                if !is_valid_audio_action(&normalized_action) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::InvalidAudioAction,
                        format!("Invalid audio action '{}'", action),
                    ));
                }
                if let Some(value) = volume {
                    if !value.is_finite() || !(0.0..=1.0).contains(value) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::InvalidAudioVolume,
                            "Audio volume must be finite and in range [0.0, 1.0]",
                        ));
                    }
                }
                if let Some(duration) = fade_duration_ms {
                    if *duration == 0 && matches!(normalized_action.as_str(), "stop" | "fade_out") {
                        issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::InvalidAudioFade,
                            "Fade duration should be > 0 ms for stop/fade_out",
                        ));
                    }
                }

                if normalized_action == "play" {
                    match asset {
                        None => issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AudioAssetMissing,
                            "Audio asset path is missing",
                        )),
                        Some(path) if path.trim().is_empty() => issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AudioAssetEmpty,
                            "Audio asset path is empty",
                        )),
                        Some(path) if is_unsafe_asset_ref(path) => issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::UnsafeAssetPath,
                            format!("Unsafe audio asset path: '{}'", path),
                        )),
                        Some(path) if should_probe_asset_exists(path) && !asset_exists(path) => {
                            issues.push(LintIssue::error(
                                Some(*id),
                                ValidationPhase::Graph,
                                LintCode::AssetReferenceMissing,
                                format!("Audio asset does not exist: '{}'", path),
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }
            StoryNode::CharacterPlacement { name, scale, .. } => {
                if name.trim().is_empty() {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyCharacterName,
                        "Character name cannot be empty",
                    ));
                }
                if let Some(scale) = scale {
                    if !scale.is_finite() || *scale <= 0.0 {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::InvalidCharacterScale,
                            "Character scale must be finite and > 0",
                        ));
                    }
                }
            }
            StoryNode::JumpIf { target, .. } => {
                if target.trim().is_empty() {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyJumpTarget,
                        "JumpIf target is empty",
                    ));
                }
            }
            StoryNode::Start | StoryNode::End => {}
        }

        if !matches!(node, StoryNode::End) && !has_outgoing(graph, *id) {
            issues.push(LintIssue::warning(
                Some(*id),
                ValidationPhase::Graph,
                LintCode::DeadEnd,
                "Node has no outgoing transition",
            ));
        }
    }

    issues
}

fn has_outgoing(graph: &NodeGraph, node_id: u32) -> bool {
    graph.connections.iter().any(|c| c.from == node_id)
}

fn default_asset_exists(path: &str) -> bool {
    let candidate = Path::new(path.trim());
    if candidate.is_absolute() {
        return candidate.is_file();
    }

    match std::env::current_dir() {
        Ok(cwd) => cwd.join(candidate).is_file(),
        Err(_) => candidate.is_file(),
    }
}

fn should_probe_asset_exists(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() {
        return false;
    }

    p.contains('/')
        || p.contains('\\')
        || Path::new(p).extension().is_some()
        || p.starts_with("assets/")
        || p.starts_with("assets\\")
}

fn is_valid_audio_channel(channel: &str) -> bool {
    matches!(channel, "bgm" | "sfx" | "voice")
}

fn is_valid_audio_action(action: &str) -> bool {
    matches!(action, "play" | "stop" | "fade_out")
}

fn is_valid_transition_kind(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "fade" | "fade_black" | "dissolve" | "cut"
    )
}

fn is_unsafe_asset_ref(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() {
        return false;
    }

    p.contains("..")
        || p.starts_with('/')
        || p.starts_with('\\')
        || p.starts_with("http://")
        || p.starts_with("https://")
        || p.chars().nth(1).is_some_and(|second| {
            second == ':' && p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        })
}

fn visit_node(graph: &NodeGraph, node_id: u32, visited: &mut HashSet<u32>) {
    if !visited.insert(node_id) {
        return;
    }

    let outgoing: Vec<u32> = graph
        .connections
        .iter()
        .filter(|c| c.from == node_id)
        .map(|c| c.to)
        .collect();

    for target in outgoing {
        visit_node(graph, target, visited);
    }
}

#[cfg(test)]
mod tests {
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
                background: "../secrets/bg.png".to_string(),
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
                background: "assets/bg_forest.png".to_string(),
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
    }
}
