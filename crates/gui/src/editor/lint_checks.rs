//! Lint checks for the node graph.
//!
//! Validates the graph structure and reports potential issues.
//! Run after context menu actions or on user request.

use super::node_graph::NodeGraph;
use super::node_types::StoryNode;

/// Severity level for lint warnings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LintSeverity {
    /// Critical issue that will prevent script execution
    Error,
    /// Potential problem that should be addressed
    Warning,
    /// Informational note
    Info,
}

/// A single lint issue found in the graph.
#[derive(Clone, Debug)]
pub struct LintIssue {
    pub severity: LintSeverity,
    pub message: String,
    pub node_id: Option<u32>,
}

impl LintIssue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: LintSeverity::Error,
            message: message.into(),
            node_id: None,
        }
    }

    fn warning(message: impl Into<String>, node_id: Option<u32>) -> Self {
        Self {
            severity: LintSeverity::Warning,
            message: message.into(),
            node_id,
        }
    }

    #[allow(dead_code)]
    fn info(message: impl Into<String>) -> Self {
        Self {
            severity: LintSeverity::Info,
            message: message.into(),
            node_id: None,
        }
    }
}

/// Runs all lint checks on the graph.
///
/// # Returns
/// A vector of issues found, may be empty if graph is valid.
pub fn validate(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    issues.extend(check_missing_start(graph));
    issues.extend(check_missing_end(graph));
    issues.extend(check_unreachable_nodes(graph));
    issues.extend(check_dead_ends(graph));
    issues.extend(check_empty_dialogue(graph));
    issues.extend(check_orphan_choices(graph));

    issues
}

/// Checks if there's a Start node.
fn check_missing_start(graph: &NodeGraph) -> Vec<LintIssue> {
    let has_start = graph
        .nodes()
        .any(|(_, node, _)| matches!(node, StoryNode::Start));
    if !has_start && !graph.is_empty() {
        vec![LintIssue::error("No Start node found. Story cannot begin.")]
    } else {
        vec![]
    }
}

/// Checks if there's an End node.
fn check_missing_end(graph: &NodeGraph) -> Vec<LintIssue> {
    let has_end = graph
        .nodes()
        .any(|(_, node, _)| matches!(node, StoryNode::End));
    if !has_end && !graph.is_empty() {
        vec![LintIssue::warning(
            "No End node found. Story may not have a proper ending.",
            None,
        )]
    } else {
        vec![]
    }
}

/// Finds nodes with no incoming connections (except Start).
fn check_unreachable_nodes(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // Collect all target node IDs
    let reachable: Vec<u32> = graph.connections().map(|(_, to)| *to).collect();

    for (id, node, _) in graph.nodes() {
        // Start nodes don't need incoming connections
        if matches!(node, StoryNode::Start) {
            continue;
        }

        if !reachable.contains(id) {
            issues.push(LintIssue::warning(
                format!(
                    "{} node is unreachable (no incoming connections)",
                    node.type_name()
                ),
                Some(*id),
            ));
        }
    }

    issues
}

/// Finds nodes with no outgoing connections (except End).
fn check_dead_ends(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    // Collect all source node IDs
    let has_outgoing: Vec<u32> = graph.connections().map(|(from, _)| *from).collect();

    for (id, node, _) in graph.nodes() {
        // End nodes don't need outgoing connections
        if matches!(node, StoryNode::End) {
            continue;
        }

        if !has_outgoing.contains(id) {
            issues.push(LintIssue::warning(
                format!(
                    "{} node is a dead end (no outgoing connections)",
                    node.type_name()
                ),
                Some(*id),
            ));
        }
    }

    issues
}

/// Checks for empty dialogue text.
fn check_empty_dialogue(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    for (id, node, _) in graph.nodes() {
        if let StoryNode::Dialogue { text, .. } = node {
            if text.trim().is_empty() {
                issues.push(LintIssue::warning("Dialogue has empty text", Some(*id)));
            }
        }
    }

    issues
}

/// Checks for Choice nodes with options that have no destination.
fn check_orphan_choices(graph: &NodeGraph) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    for (id, node, _) in graph.nodes() {
        if let StoryNode::Choice { options, .. } = node {
            let outgoing_count = graph.connections().filter(|(from, _)| from == id).count();

            if outgoing_count < options.len() {
                issues.push(LintIssue::warning(
                    format!(
                        "Choice has {} options but only {} connections",
                        options.len(),
                        outgoing_count
                    ),
                    Some(*id),
                ));
            }
        }
    }

    issues
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    fn pos(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(x, y)
    }

    #[test]
    fn test_valid_graph_no_issues() {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let dialogue = graph.add_node(StoryNode::default(), pos(0.0, 100.0));
        let end = graph.add_node(StoryNode::End, pos(0.0, 200.0));

        graph.connect(start, dialogue);
        graph.connect(dialogue, end);

        let issues = validate(&graph);
        assert!(issues.is_empty(), "Valid graph should have no issues");
    }

    #[test]
    fn test_missing_start() {
        let mut graph = NodeGraph::new();
        graph.add_node(StoryNode::default(), pos(0.0, 0.0));

        let issues = validate(&graph);
        assert!(issues.iter().any(|i| i.severity == LintSeverity::Error));
    }

    #[test]
    fn test_missing_end() {
        let mut graph = NodeGraph::new();
        graph.add_node(StoryNode::Start, pos(0.0, 0.0));

        let issues = validate(&graph);
        assert!(issues.iter().any(|i| i.message.contains("End")));
    }

    #[test]
    fn test_unreachable_node() {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let end = graph.add_node(StoryNode::End, pos(0.0, 100.0));
        let orphan = graph.add_node(StoryNode::default(), pos(100.0, 0.0));

        graph.connect(start, end);
        // orphan is not connected

        let issues = validate(&graph);
        assert!(issues.iter().any(|i| i.node_id == Some(orphan)));
    }

    #[test]
    fn test_dead_end() {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let dialogue = graph.add_node(StoryNode::default(), pos(0.0, 100.0));
        graph.add_node(StoryNode::End, pos(0.0, 200.0));

        graph.connect(start, dialogue);
        // dialogue doesn't connect to end

        let issues = validate(&graph);
        assert!(issues.iter().any(|i| i.message.contains("dead end")));
    }

    #[test]
    fn test_empty_dialogue() {
        let mut graph = NodeGraph::new();
        graph.add_node(
            StoryNode::Dialogue {
                speaker: "Test".to_string(),
                text: "   ".to_string(), // whitespace only
            },
            pos(0.0, 0.0),
        );

        let issues = validate(&graph);
        assert!(issues.iter().any(|i| i.message.contains("empty text")));
    }

    #[test]
    fn test_orphan_choice() {
        let mut graph = NodeGraph::new();
        let choice = graph.add_node(
            StoryNode::Choice {
                prompt: "Pick".to_string(),
                options: vec!["A".to_string(), "B".to_string()],
            },
            pos(0.0, 0.0),
        );
        let target = graph.add_node(StoryNode::End, pos(0.0, 100.0));

        // Only connect one option
        graph.connect(choice, target);

        let issues = validate(&graph);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("2 options but only 1")));
    }
}
