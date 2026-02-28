use std::collections::HashSet;

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
    InvalidCharacterScale,
    EmptyJumpTarget,
    CompileError,
    RuntimeInitError,
    DryRunUnreachableCompiled,
    DryRunStepLimit,
    DryRunRuntimeError,
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
            LintCode::InvalidCharacterScale => "VAL_SCALE_INVALID",
            LintCode::EmptyJumpTarget => "VAL_JUMP_EMPTY",
            LintCode::CompileError => "CMP_SCRIPT_ERROR",
            LintCode::RuntimeInitError => "CMP_RUNTIME_INIT",
            LintCode::DryRunUnreachableCompiled => "DRY_UNREACHABLE",
            LintCode::DryRunStepLimit => "DRY_STEP_LIMIT",
            LintCode::DryRunRuntimeError => "DRY_RUNTIME_ERROR",
            LintCode::DryRunFinished => "DRY_FINISHED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub node_id: Option<u32>,
    pub severity: LintSeverity,
    pub phase: ValidationPhase,
    pub code: LintCode,
    pub message: String,
}

impl LintIssue {
    pub fn new(
        node_id: Option<u32>,
        severity: LintSeverity,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            severity,
            phase,
            code,
            message: message.into(),
        }
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
        match node {
            StoryNode::Dialogue { .. }
            | StoryNode::Scene { .. }
            | StoryNode::SetVariable { .. }
            | StoryNode::Transition { .. }
            | StoryNode::ScenePatch(_) => {
                if !has_outgoing(graph, *id) && !matches!(node, StoryNode::End) {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        ValidationPhase::Graph,
                        LintCode::DeadEnd,
                        "Node has no outgoing transition",
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
            StoryNode::AudioAction { asset, .. } => match asset {
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
                Some(_) => {}
            },
            StoryNode::CharacterPlacement { scale, .. } => {
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
            StoryNode::Generic(_) => {}
            StoryNode::Start | StoryNode::End => {}
        }
    }

    issues
}

fn has_outgoing(graph: &NodeGraph, node_id: u32) -> bool {
    graph.connections.iter().any(|c| c.from == node_id)
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
