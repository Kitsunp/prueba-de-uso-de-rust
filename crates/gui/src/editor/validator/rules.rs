use super::*;
use std::collections::HashSet;
use std::path::Path;

pub(super) fn validate_with_asset_probe_impl<F>(
    graph: &NodeGraph,
    asset_exists: F,
) -> Vec<LintIssue>
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
            StoryNode::Scene {
                background,
                music,
                characters,
                ..
            } => {
                if let Some(background) = background {
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

                if let Some(music) = music {
                    if music.trim().is_empty() {
                        issues.push(LintIssue::warning(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AudioAssetEmpty,
                            "Scene music path is empty",
                        ));
                    } else if is_unsafe_asset_ref(music) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::UnsafeAssetPath,
                            format!("Unsafe music path: '{}'", music),
                        ));
                    } else if should_probe_asset_exists(music) && !asset_exists(music) {
                        issues.push(LintIssue::error(
                            Some(*id),
                            ValidationPhase::Graph,
                            LintCode::AssetReferenceMissing,
                            format!("Music asset does not exist: '{}'", music),
                        ));
                    }
                }

                if characters.iter().any(|c| c.name.trim().is_empty()) {
                    issues.push(LintIssue::error(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::EmptyCharacterName,
                        "Scene has character entry with empty name",
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

pub(super) fn default_asset_exists(path: &str) -> bool {
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
