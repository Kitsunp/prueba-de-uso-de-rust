use crate::editor::node_graph::NodeGraph;
use crate::editor::node_types::StoryNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub node_id: Option<u32>,
    pub severity: LintSeverity,
    pub message: String,
}

impl LintIssue {
    pub fn error(node_id: Option<u32>, message: impl Into<String>) -> Self {
        Self {
            node_id,
            severity: LintSeverity::Error,
            message: message.into(),
        }
    }

    pub fn warning(node_id: Option<u32>, message: impl Into<String>) -> Self {
        Self {
            node_id,
            severity: LintSeverity::Warning,
            message: message.into(),
        }
    }
}

pub fn validate(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // Rule 1: Check for Start Node
    let has_start = graph
        .nodes
        .iter()
        .any(|(_, n, _)| matches!(n, StoryNode::Start));
    if !has_start {
        issues.push(LintIssue::error(None, "Missing Start Node"));
    }

    // Rule 2: Check for Unreachable Nodes (basic flood fill)
    let mut visited = std::collections::HashSet::new();
    // Start DFS from Start nodes
    for (id, node, _) in &graph.nodes {
        if matches!(node, StoryNode::Start) {
            visit_node(graph, *id, &mut visited);
        }
    }

    // Check for nodes not visited (excluding comments or notes if we had them)
    for (id, _, _) in &graph.nodes {
        if !visited.contains(id) {
            issues.push(LintIssue::warning(
                Some(*id),
                "Unreachable node (Dead code)",
            ));
        }
    }

    // Rule 3: Check for Dead Ends (Dialogues/Choices without connections)
    // Rule 4: Check for Invalid Audio/Image paths (basic empty check)
    for (id, node, _) in &graph.nodes {
        match node {
            StoryNode::Dialogue { .. } => {
                if !graph.connections.iter().any(|c| c.from == *id) {
                    issues.push(LintIssue::warning(
                        Some(*id),
                        "Dialogue has no following event (Game ends?)",
                    ));
                }
            }
            StoryNode::Choice { options, .. } => {
                if options.is_empty() {
                    issues.push(LintIssue::error(Some(*id), "Choice node has no options"));
                }
                // Check if all options are connected
                // This requires iterating options logic/ports
            }
            StoryNode::AudioAction { asset, .. } => {
                if let Some(path) = asset {
                    if path.trim().is_empty() {
                        issues.push(LintIssue::warning(Some(*id), "Audio asset path is empty"));
                    }
                } else {
                    issues.push(LintIssue::warning(Some(*id), "Audio asset path is missing"));
                }
            }
            _ => {}
        }
    }

    issues
}

fn visit_node(graph: &NodeGraph, node_id: u32, visited: &mut std::collections::HashSet<u32>) {
    if !visited.insert(node_id) {
        return;
    }

    // Find all outgoing connections from this node
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
