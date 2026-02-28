//! Story graph generation and analysis for the Visual Novel Engine.
//!
//! This module generates a directed graph representation of the narrative flow
//! from compiled scripts. It enables:
//! - Visualization of story structure
//! - Detection of unreachable nodes (dead code)
//! - Navigation in the editor
//!
//! # Contracts
//! - **Precondition**: Graph is generated from a valid `ScriptCompiled`.
//! - **Postcondition**: All reachable nodes are marked, unreachable nodes are flagged.

use std::collections::{BTreeMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::event::{CondCompiled, EventCompiled};
use crate::script::ScriptCompiled;

// =============================================================================
// Node Types
// =============================================================================

/// Unique identifier for a graph node (corresponds to event index/IP).
pub type NodeId = u32;

/// Type of node in the story graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// A dialogue event.
    Dialogue {
        speaker: String,
        text_preview: String,
    },
    /// A choice point with multiple options.
    Choice { prompt: String, option_count: usize },
    /// A scene change.
    Scene { background: Option<String> },
    /// An unconditional jump.
    Jump,
    /// A conditional jump.
    ConditionalJump { condition: String },
    /// A flag or variable modification.
    StateChange { description: String },
    /// A scene patch (partial update).
    Patch,
    /// An external command call.
    ExtCall { command: String },
    /// An audio action.
    AudioAction {
        channel: u8,
        action: u8,
        asset: Option<String>,
    },
    /// A scene transition.
    Transition { kind: String, duration: u64 },
    /// Explicit character placement with coordinates.
    CharacterPlacement {
        name: String,
        x: i32,
        y: i32,
        scale: Option<f32>,
    },
}

/// A node in the story graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    /// The instruction pointer / event index.
    pub id: NodeId,
    /// The type of node.
    pub node_type: NodeType,
    /// Label(s) pointing to this node (if any).
    pub labels: Vec<String>,
    /// Whether this node is reachable from the start.
    pub reachable: bool,
}

// =============================================================================
// Edge Types
// =============================================================================

/// Type of transition between nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Normal sequential flow (next instruction).
    Sequential,
    /// Unconditional jump.
    Jump,
    /// Conditional jump (when condition is true).
    ConditionalTrue,
    /// Conditional jump fallthrough (when condition is false).
    ConditionalFalse,
    /// Choice option selected.
    Choice { option_index: usize },
}

/// A directed edge in the story graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID.
    pub from: NodeId,
    /// Target node ID.
    pub to: NodeId,
    /// Type of edge.
    pub edge_type: EdgeType,
    /// Optional label (e.g., choice text).
    pub label: Option<String>,
}

// =============================================================================
// Story Graph
// =============================================================================

/// The complete story graph generated from a compiled script.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoryGraph {
    /// All nodes in the graph.
    pub nodes: Vec<GraphNode>,
    /// All edges in the graph.
    pub edges: Vec<GraphEdge>,
    /// The starting node ID.
    pub start_id: NodeId,
    /// Labels mapped to node IDs.
    pub label_map: BTreeMap<String, NodeId>,
}

/// Statistics about the story graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes.
    pub total_nodes: usize,
    /// Number of reachable nodes.
    pub reachable_nodes: usize,
    /// Number of unreachable nodes.
    pub unreachable_nodes: usize,
    /// Number of dialogue nodes.
    pub dialogue_count: usize,
    /// Number of choice nodes.
    pub choice_count: usize,
    /// Number of branch points (choices + conditionals).
    pub branch_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
}

impl StoryGraph {
    /// Generates a story graph from a compiled script.
    pub fn from_script(script: &ScriptCompiled) -> Self {
        let mut nodes = Vec::with_capacity(script.events.len());
        let mut edges = Vec::new();

        // Create reverse label map
        let mut label_map: BTreeMap<String, NodeId> = BTreeMap::new();
        for (label, &ip) in &script.labels {
            label_map.insert(label.clone(), ip);
        }

        // Create IP to labels mapping
        let mut ip_labels: BTreeMap<NodeId, Vec<String>> = BTreeMap::new();
        for (label, &ip) in &script.labels {
            ip_labels.entry(ip).or_default().push(label.clone());
        }

        // Generate nodes and edges
        for (ip, event) in script.events.iter().enumerate() {
            let ip = ip as NodeId;
            let labels = ip_labels.get(&ip).cloned().unwrap_or_default();

            let (node_type, event_edges) = Self::process_event(ip, event, script.events.len());

            nodes.push(GraphNode {
                id: ip,
                node_type,
                labels,
                reachable: false, // Will be computed later
            });

            edges.extend(event_edges);
        }

        let mut graph = Self {
            nodes,
            edges,
            start_id: script.start_ip,
            label_map,
        };

        // Compute reachability
        graph.compute_reachability();

        graph
    }

    /// Processes an event and returns its node type and outgoing edges.
    fn process_event(
        ip: NodeId,
        event: &EventCompiled,
        event_count: usize,
    ) -> (NodeType, Vec<GraphEdge>) {
        let next_ip = ip + 1;
        let has_next = (next_ip as usize) < event_count;

        match event {
            EventCompiled::Dialogue(dialogue) => {
                let text_preview = if dialogue.text.len() > 50 {
                    format!("{}...", &dialogue.text[..47])
                } else {
                    dialogue.text.to_string()
                };
                let node_type = NodeType::Dialogue {
                    speaker: dialogue.speaker.to_string(),
                    text_preview,
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::Choice(choice) => {
                let node_type = NodeType::Choice {
                    prompt: choice.prompt.to_string(),
                    option_count: choice.options.len(),
                };
                let edges = choice
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, opt)| GraphEdge {
                        from: ip,
                        to: opt.target_ip,
                        edge_type: EdgeType::Choice { option_index: idx },
                        label: Some(opt.text.to_string()),
                    })
                    .collect();
                (node_type, edges)
            }

            EventCompiled::Scene(scene) => {
                let node_type = NodeType::Scene {
                    background: scene.background.as_ref().map(|s| s.to_string()),
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::Jump { target_ip } => {
                let edges = vec![GraphEdge {
                    from: ip,
                    to: *target_ip,
                    edge_type: EdgeType::Jump,
                    label: None,
                }];
                (NodeType::Jump, edges)
            }

            EventCompiled::JumpIf { cond, target_ip } => {
                let condition = Self::format_condition(cond);
                let node_type = NodeType::ConditionalJump { condition };
                let mut edges = vec![GraphEdge {
                    from: ip,
                    to: *target_ip,
                    edge_type: EdgeType::ConditionalTrue,
                    label: Some("true".to_string()),
                }];
                if has_next {
                    edges.push(GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::ConditionalFalse,
                        label: Some("false".to_string()),
                    });
                }
                (node_type, edges)
            }

            EventCompiled::SetFlag { flag_id, value } => {
                let desc = format!("flag[{}] = {}", flag_id, value);
                let node_type = NodeType::StateChange { description: desc };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::SetVar { var_id, value } => {
                let desc = format!("var[{}] = {}", var_id, value);
                let node_type = NodeType::StateChange { description: desc };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::Patch(_) => {
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (NodeType::Patch, edges)
            }

            EventCompiled::ExtCall { command, args: _ } => {
                let node_type = NodeType::ExtCall {
                    command: command.clone(),
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::AudioAction(action) => {
                let node_type = NodeType::AudioAction {
                    channel: action.channel,
                    action: action.action,
                    asset: action.asset.as_ref().map(|s| s.to_string()),
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }

            EventCompiled::Transition(transition) => {
                let node_type = NodeType::Transition {
                    kind: if transition.kind == 0 {
                        "fade".to_string()
                    } else {
                        "dissolve".to_string()
                    }, // simplistic mapping for now
                    duration: transition.duration_ms.into(),
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }
            EventCompiled::SetCharacterPosition(pos) => {
                let node_type = NodeType::CharacterPlacement {
                    name: pos.name.to_string(),
                    x: pos.x,
                    y: pos.y,
                    scale: pos.scale,
                };
                let edges = if has_next {
                    vec![GraphEdge {
                        from: ip,
                        to: next_ip,
                        edge_type: EdgeType::Sequential,
                        label: None,
                    }]
                } else {
                    vec![]
                };
                (node_type, edges)
            }
        }
    }

    /// Formats a condition for display.
    fn format_condition(cond: &CondCompiled) -> String {
        match cond {
            CondCompiled::Flag { flag_id, is_set } => {
                if *is_set {
                    format!("flag[{}]", flag_id)
                } else {
                    format!("!flag[{}]", flag_id)
                }
            }
            CondCompiled::VarCmp { var_id, op, value } => {
                format!("var[{}] {:?} {}", var_id, op, value)
            }
        }
    }

    /// Computes reachability using BFS from the start node.
    fn compute_reachability(&mut self) {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();

        queue.push_back(self.start_id);
        visited.insert(self.start_id);

        while let Some(node_id) = queue.pop_front() {
            // Find all outgoing edges
            for edge in &self.edges {
                if edge.from == node_id && !visited.contains(&edge.to) {
                    visited.insert(edge.to);
                    queue.push_back(edge.to);
                }
            }
        }

        // Mark nodes as reachable
        for node in &mut self.nodes {
            node.reachable = visited.contains(&node.id);
        }
    }

    /// Returns statistics about the graph.
    pub fn stats(&self) -> GraphStats {
        let reachable_nodes = self.nodes.iter().filter(|n| n.reachable).count();
        let dialogue_count = self
            .nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Dialogue { .. }))
            .count();
        let choice_count = self
            .nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Choice { .. }))
            .count();
        let conditional_count = self
            .nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::ConditionalJump { .. }))
            .count();

        GraphStats {
            total_nodes: self.nodes.len(),
            reachable_nodes,
            unreachable_nodes: self.nodes.len() - reachable_nodes,
            dialogue_count,
            choice_count,
            branch_count: choice_count + conditional_count,
            edge_count: self.edges.len(),
        }
    }

    /// Returns all unreachable node IDs.
    pub fn unreachable_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|n| !n.reachable)
            .map(|n| n.id)
            .collect()
    }

    /// Gets a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.get(id as usize)
    }

    /// Gets all outgoing edges from a node.
    pub fn outgoing_edges(&self, id: NodeId) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == id).collect()
    }

    /// Gets all incoming edges to a node.
    pub fn incoming_edges(&self, id: NodeId) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.to == id).collect()
    }

    /// Finds a node by label.
    pub fn find_by_label(&self, label: &str) -> Option<NodeId> {
        self.label_map.get(label).copied()
    }

    /// Exports the graph to DOT format for visualization with Graphviz.
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph StoryGraph {\n");
        dot.push_str("    rankdir=TB;\n");
        dot.push_str("    node [shape=box];\n\n");

        // Nodes
        for node in &self.nodes {
            let color = if !node.reachable {
                "red"
            } else if node.id == self.start_id {
                "green"
            } else {
                "black"
            };

            let label = match &node.node_type {
                NodeType::Dialogue {
                    speaker,
                    text_preview,
                } => {
                    format!(
                        "[{}] {}: {}",
                        node.id,
                        speaker,
                        text_preview.replace('"', "'")
                    )
                }
                NodeType::Choice {
                    prompt,
                    option_count,
                } => {
                    format!(
                        "[{}] Choice: {} ({} options)",
                        node.id,
                        prompt.replace('"', "'"),
                        option_count
                    )
                }
                NodeType::Scene { background } => {
                    format!("[{}] Scene: {:?}", node.id, background)
                }
                NodeType::Jump => format!("[{}] Jump", node.id),
                NodeType::ConditionalJump { condition } => {
                    format!("[{}] If: {}", node.id, condition)
                }
                NodeType::StateChange { description } => {
                    format!("[{}] {}", node.id, description)
                }
                NodeType::Patch => format!("[{}] Patch", node.id),
                NodeType::ExtCall { command } => format!("[{}] Call: {}", node.id, command),
                NodeType::AudioAction {
                    channel, action, ..
                } => {
                    format!("[{}] Audio: {}/{}", node.id, channel, action)
                }
                NodeType::Transition { kind, .. } => {
                    format!("[{}] Transition: {}", node.id, kind)
                }
                NodeType::CharacterPlacement { name, x, y, scale } => {
                    format!(
                        "[{}] Placement: {} ({}, {}) s={:?}",
                        node.id, name, x, y, scale
                    )
                }
            };

            let shape = match &node.node_type {
                NodeType::Choice { .. } => "diamond",
                NodeType::ConditionalJump { .. } => "diamond",
                NodeType::Jump => "ellipse",
                _ => "box",
            };

            dot.push_str(&format!(
                "    n{} [label=\"{}\" shape={} color={}];\n",
                node.id, label, shape, color
            ));
        }

        dot.push('\n');

        // Edges
        for edge in &self.edges {
            let style = match edge.edge_type {
                EdgeType::Sequential => "solid",
                EdgeType::Jump => "dashed",
                EdgeType::ConditionalTrue => "bold",
                EdgeType::ConditionalFalse => "dotted",
                EdgeType::Choice { .. } => "solid",
            };

            let label = edge
                .label
                .as_ref()
                .map(|l| format!(" [label=\"{}\"]", l.replace('"', "'")))
                .unwrap_or_default();

            dot.push_str(&format!(
                "    n{} -> n{} [style={}{}];\n",
                edge.from, edge.to, style, label
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ChoiceCompiled, ChoiceOptionCompiled, DialogueCompiled, SharedStr};

    fn make_dialogue(speaker: &str, text: &str) -> EventCompiled {
        EventCompiled::Dialogue(DialogueCompiled {
            speaker: SharedStr::from(speaker),
            text: SharedStr::from(text),
        })
    }

    fn make_choice(prompt: &str, options: Vec<(&str, u32)>) -> EventCompiled {
        EventCompiled::Choice(ChoiceCompiled {
            prompt: SharedStr::from(prompt),
            options: options
                .into_iter()
                .map(|(text, target)| ChoiceOptionCompiled {
                    text: SharedStr::from(text),
                    target_ip: target,
                })
                .collect(),
        })
    }

    #[test]
    fn test_linear_script_graph() {
        let script = ScriptCompiled {
            events: vec![
                make_dialogue("Alice", "Hello!"),
                make_dialogue("Bob", "Hi there!"),
                make_dialogue("Alice", "Nice to meet you."),
            ],
            labels: [("start".to_string(), 0)].into_iter().collect(),
            start_ip: 0,
            flag_count: 0,
        };

        let graph = StoryGraph::from_script(&script);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.nodes.iter().all(|n| n.reachable));

        let stats = graph.stats();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.reachable_nodes, 3);
        assert_eq!(stats.unreachable_nodes, 0);
        assert_eq!(stats.dialogue_count, 3);
    }

    #[test]
    fn test_branching_script_graph() {
        let script = ScriptCompiled {
            events: vec![
                make_dialogue("Narrator", "What do you choose?"),
                make_choice("Choose wisely", vec![("Option A", 2), ("Option B", 3)]),
                make_dialogue("Narrator", "You chose A!"),
                make_dialogue("Narrator", "You chose B!"),
            ],
            labels: [("start".to_string(), 0)].into_iter().collect(),
            start_ip: 0,
            flag_count: 0,
        };

        let graph = StoryGraph::from_script(&script);

        assert_eq!(graph.nodes.len(), 4);
        let stats = graph.stats();
        assert_eq!(stats.choice_count, 1);
        assert_eq!(stats.branch_count, 1);
        assert!(graph.nodes.iter().all(|n| n.reachable));
    }

    #[test]
    fn test_unreachable_detection() {
        let script = ScriptCompiled {
            events: vec![
                make_dialogue("Alice", "Start"),
                EventCompiled::Jump { target_ip: 3 },
                make_dialogue("Hidden", "This is unreachable!"),
                make_dialogue("Alice", "End"),
            ],
            labels: [("start".to_string(), 0)].into_iter().collect(),
            start_ip: 0,
            flag_count: 0,
        };

        let graph = StoryGraph::from_script(&script);
        let unreachable = graph.unreachable_nodes();

        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0], 2);

        let stats = graph.stats();
        assert_eq!(stats.unreachable_nodes, 1);
    }

    #[test]
    fn test_dot_export() {
        let script = ScriptCompiled {
            events: vec![
                make_dialogue("Test", "Hello"),
                EventCompiled::Jump { target_ip: 0 },
            ],
            labels: [("start".to_string(), 0)].into_iter().collect(),
            start_ip: 0,
            flag_count: 0,
        };

        let graph = StoryGraph::from_script(&script);
        let dot = graph.to_dot();

        assert!(dot.contains("digraph StoryGraph"));
        assert!(dot.contains("n0 ->"));
        assert!(dot.contains("n1 ->"));
    }

    #[test]
    fn test_find_by_label() {
        let script = ScriptCompiled {
            events: vec![
                make_dialogue("A", "Start"),
                make_dialogue("B", "Middle"),
                make_dialogue("C", "End"),
            ],
            labels: [
                ("start".to_string(), 0),
                ("middle".to_string(), 1),
                ("end".to_string(), 2),
            ]
            .into_iter()
            .collect(),
            start_ip: 0,
            flag_count: 0,
        };

        let graph = StoryGraph::from_script(&script);

        assert_eq!(graph.find_by_label("start"), Some(0));
        assert_eq!(graph.find_by_label("middle"), Some(1));
        assert_eq!(graph.find_by_label("end"), Some(2));
        assert_eq!(graph.find_by_label("nonexistent"), None);
    }
}
