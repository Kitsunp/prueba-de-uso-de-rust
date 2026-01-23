//! Script synchronization for NodeGraph.
//!
//! This module provides bidirectional conversion between NodeGraph and ScriptRaw.
//! Extracted from node_graph.rs to comply with Criterio J (<500 lines).

use std::collections::BTreeMap;

use eframe::egui;
use visual_novel_engine::{
    ChoiceOptionRaw, ChoiceRaw, DialogueRaw, EventRaw, SceneUpdateRaw, ScriptRaw,
};

use super::node_graph::NodeGraph;
use super::node_types::{StoryNode, NODE_VERTICAL_SPACING};

/// Creates a NodeGraph from a raw script.
///
/// # Contract
/// - Maps each `EventRaw` to a `StoryNode`
/// - Creates connections based on sequential flow and jumps
/// - Adds Start/End markers
///
/// # Postconditions
/// - Graph contains Start node (unless script is empty)
/// - Graph contains End node (unless script is empty)
/// - Graph is marked as NOT modified
pub fn from_script(script: &ScriptRaw) -> NodeGraph {
    let mut graph = NodeGraph::new();

    if script.events.is_empty() {
        return graph;
    }

    // Add Start node
    let start_id = graph.add_node(StoryNode::Start, egui::pos2(50.0, 30.0));

    // Map script indices to node IDs
    let mut index_to_id: BTreeMap<usize, u32> = BTreeMap::new();

    // Create nodes for each event
    for (idx, event) in script.events.iter().enumerate() {
        let y = 100.0 + (idx as f32) * NODE_VERTICAL_SPACING;
        let node = match event {
            EventRaw::Dialogue(d) => StoryNode::Dialogue {
                speaker: d.speaker.clone(),
                text: d.text.clone(),
            },
            EventRaw::Choice(c) => StoryNode::Choice {
                prompt: c.prompt.clone(),
                options: c.options.iter().map(|o| o.text.clone()).collect(),
            },
            EventRaw::Scene(s) => StoryNode::Scene {
                background: s.background.clone().unwrap_or_default(),
            },
            EventRaw::Jump { target } => StoryNode::Jump {
                target: target.clone(),
            },
            _ => continue, // Skip SetFlag, SetVar, JumpIf, etc.
        };

        let id = graph.add_node(node, egui::pos2(100.0, y));
        index_to_id.insert(idx, id);
    }

    // Connect Start to first event
    if let Some(&first_id) = index_to_id.get(&0) {
        graph.connect(start_id, first_id);
    }

    // Create sequential connections and handle jumps
    let label_to_index: BTreeMap<&str, usize> = script
        .labels
        .iter()
        .map(|(name, idx)| (name.as_str(), *idx))
        .collect();

    for (idx, event) in script.events.iter().enumerate() {
        let Some(&from_id) = index_to_id.get(&idx) else {
            continue;
        };

        match event {
            EventRaw::Jump { target } => {
                if let Some(&target_idx) = label_to_index.get(target.as_str()) {
                    if let Some(&target_id) = index_to_id.get(&target_idx) {
                        graph.connect(from_id, target_id);
                    }
                }
            }
            EventRaw::Choice(c) => {
                for option in &c.options {
                    if let Some(&target_idx) = label_to_index.get(option.target.as_str()) {
                        if let Some(&target_id) = index_to_id.get(&target_idx) {
                            graph.connect(from_id, target_id);
                        }
                    }
                }
            }
            _ => {
                if let Some(&next_id) = index_to_id.get(&(idx + 1)) {
                    graph.connect(from_id, next_id);
                }
            }
        }
    }

    // Add End node
    let last_y = 100.0 + (script.events.len() as f32) * NODE_VERTICAL_SPACING;
    let end_id = graph.add_node(StoryNode::End, egui::pos2(100.0, last_y));

    // Connect nodes with no outgoing connections to End
    let nodes_with_outgoing: Vec<u32> = graph.connections_slice().iter().map(|(f, _)| *f).collect();
    let nodes_to_connect: Vec<u32> = graph
        .nodes_slice()
        .iter()
        .filter(|(id, node, _)| {
            !nodes_with_outgoing.contains(id) && !matches!(node, StoryNode::End)
        })
        .map(|(id, _, _)| *id)
        .collect();

    for id in nodes_to_connect {
        graph.connect(id, end_id);
    }

    graph.clear_modified();
    graph
}

/// Converts a NodeGraph to a raw script.
///
/// # Contract
/// - Generates `EventRaw` for each non-marker node
/// - Creates labels for targets
/// - Maintains execution order via BFS traversal from Start
///
/// # Returns
/// A ScriptRaw that can be serialized to JSON or compiled.
pub fn to_script(graph: &NodeGraph) -> ScriptRaw {
    let mut events = Vec::new();
    let mut labels = BTreeMap::new();

    // Find start node
    let start_id = graph
        .nodes()
        .find(|(_, node, _)| matches!(node, StoryNode::Start))
        .map(|(id, _, _)| *id);

    // BFS traversal from start
    let mut visited = Vec::new();
    let mut queue = Vec::new();

    if let Some(start) = start_id {
        queue.push(start);
    }

    while let Some(id) = queue.pop() {
        if visited.contains(&id) {
            continue;
        }
        visited.push(id);

        for (from, to) in graph.connections() {
            if *from == id && !visited.contains(to) {
                queue.push(*to);
            }
        }
    }

    // Convert visited nodes to events
    let mut node_to_label: BTreeMap<u32, String> = BTreeMap::new();

    for &id in &visited {
        let Some((_, node, _)) = graph.nodes().find(|(nid, _, _)| *nid == id) else {
            continue;
        };

        let event_idx = events.len();
        let label = format!("node_{}", id);
        node_to_label.insert(id, label.clone());
        labels.insert(label, event_idx);

        match node {
            StoryNode::Dialogue { speaker, text } => {
                events.push(EventRaw::Dialogue(DialogueRaw {
                    speaker: speaker.clone(),
                    text: text.clone(),
                }));
            }
            StoryNode::Choice { prompt, options } => {
                let outgoing: Vec<u32> = graph
                    .connections()
                    .filter(|(f, _)| *f == id)
                    .map(|(_, t)| *t)
                    .collect();

                let choice_options: Vec<ChoiceOptionRaw> = options
                    .iter()
                    .enumerate()
                    .map(|(i, text)| {
                        let target = outgoing
                            .get(i)
                            .map(|tid| format!("node_{}", tid))
                            .unwrap_or_else(|| "start".to_string());
                        ChoiceOptionRaw {
                            text: text.clone(),
                            target,
                        }
                    })
                    .collect();

                events.push(EventRaw::Choice(ChoiceRaw {
                    prompt: prompt.clone(),
                    options: choice_options,
                }));
            }
            StoryNode::Jump { target } => {
                events.push(EventRaw::Jump {
                    target: target.clone(),
                });
            }
            StoryNode::Scene { background } => {
                events.push(EventRaw::Scene(SceneUpdateRaw {
                    background: Some(background.clone()),
                    music: None,
                    characters: vec![],
                }));
            }
            StoryNode::Start | StoryNode::End => {
                // Skip start/end markers - they don't generate script events
            }
        }
    }

    // Add start label
    if !labels.contains_key("start") && !events.is_empty() {
        labels.insert("start".to_string(), 0);
    }

    ScriptRaw::new(events, labels)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(x, y)
    }

    #[test]
    fn test_roundtrip_empty_script() {
        let script = ScriptRaw::new(vec![], BTreeMap::new());
        let graph = from_script(&script);
        let roundtrip = to_script(&graph);

        // Empty script should remain empty
        assert!(roundtrip.events.is_empty());
    }

    #[test]
    fn test_roundtrip_single_dialogue() {
        let mut labels = BTreeMap::new();
        labels.insert("start".to_string(), 0);

        let events = vec![EventRaw::Dialogue(DialogueRaw {
            speaker: "Alice".to_string(),
            text: "Hello, world!".to_string(),
        })];

        let original = ScriptRaw::new(events, labels);
        let graph = from_script(&original);
        let roundtrip = to_script(&graph);

        // Should have at least one dialogue event
        assert!(!roundtrip.events.is_empty());
        assert!(roundtrip.labels.contains_key("start"));
    }

    #[test]
    fn test_roundtrip_preserves_dialogue_content() {
        let mut labels = BTreeMap::new();
        labels.insert("start".to_string(), 0);

        let events = vec![
            EventRaw::Dialogue(DialogueRaw {
                speaker: "Bob".to_string(),
                text: "First line".to_string(),
            }),
            EventRaw::Dialogue(DialogueRaw {
                speaker: "Alice".to_string(),
                text: "Second line".to_string(),
            }),
        ];

        let original = ScriptRaw::new(events, labels);
        let graph = from_script(&original);
        let roundtrip = to_script(&graph);

        // Count dialogue events
        let dialogue_count = roundtrip
            .events
            .iter()
            .filter(|e| matches!(e, EventRaw::Dialogue(_)))
            .count();

        // Should preserve both dialogues
        assert_eq!(dialogue_count, 2);
    }

    #[test]
    fn test_roundtrip_choice_structure() {
        let mut labels = BTreeMap::new();
        labels.insert("start".to_string(), 0);
        labels.insert("option_a".to_string(), 1);
        labels.insert("option_b".to_string(), 2);

        let events = vec![
            EventRaw::Choice(ChoiceRaw {
                prompt: "Choose wisely".to_string(),
                options: vec![
                    ChoiceOptionRaw {
                        text: "Option A".to_string(),
                        target: "option_a".to_string(),
                    },
                    ChoiceOptionRaw {
                        text: "Option B".to_string(),
                        target: "option_b".to_string(),
                    },
                ],
            }),
            EventRaw::Dialogue(DialogueRaw {
                speaker: "A".to_string(),
                text: "Path A".to_string(),
            }),
            EventRaw::Dialogue(DialogueRaw {
                speaker: "B".to_string(),
                text: "Path B".to_string(),
            }),
        ];

        let original = ScriptRaw::new(events, labels);
        let graph = from_script(&original);

        // Graph should have nodes
        assert!(graph.len() >= 4);

        let roundtrip = to_script(&graph);

        // Should have choice event
        let has_choice = roundtrip
            .events
            .iter()
            .any(|e| matches!(e, EventRaw::Choice(_)));
        assert!(has_choice);
    }

    #[test]
    fn test_manual_graph_to_script() {
        // Test creating a graph manually and converting to script
        let mut graph = NodeGraph::new();

        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let dialogue = graph.add_node(
            StoryNode::Dialogue {
                speaker: "Narrator".to_string(),
                text: "Welcome to the story".to_string(),
            },
            pos(0.0, 100.0),
        );
        let end = graph.add_node(StoryNode::End, pos(0.0, 200.0));

        graph.connect(start, dialogue);
        graph.connect(dialogue, end);

        let script = to_script(&graph);

        // Should have at least one dialogue
        assert!(!script.events.is_empty());
        assert!(script.labels.contains_key("start"));
    }

    #[test]
    fn test_roundtrip_scene_nodes() {
        // Test that Scene nodes are properly synced
        let mut graph = NodeGraph::new();

        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let scene = graph.add_node(
            StoryNode::Scene {
                background: "forest_bg.png".to_string(),
            },
            pos(0.0, 100.0),
        );
        let dialogue = graph.add_node(
            StoryNode::Dialogue {
                speaker: "Guide".to_string(),
                text: "Welcome to the forest".to_string(),
            },
            pos(0.0, 200.0),
        );
        let end = graph.add_node(StoryNode::End, pos(0.0, 300.0));

        graph.connect(start, scene);
        graph.connect(scene, dialogue);
        graph.connect(dialogue, end);

        let script = to_script(&graph);

        // Should have Scene event
        let has_scene = script
            .events
            .iter()
            .any(|e| matches!(e, EventRaw::Scene(_)));
        assert!(has_scene, "Script should contain a Scene event");

        // Should have dialogue
        let has_dialogue = script
            .events
            .iter()
            .any(|e| matches!(e, EventRaw::Dialogue(_)));
        assert!(has_dialogue, "Script should contain a Dialogue event");
    }

    #[test]
    fn test_roundtrip_with_jump() {
        // Test Jump nodes are properly synced
        let mut graph = NodeGraph::new();

        let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
        let jump = graph.add_node(
            StoryNode::Jump {
                target: "some_label".to_string(),
            },
            pos(0.0, 100.0),
        );
        let end = graph.add_node(StoryNode::End, pos(0.0, 200.0));

        graph.connect(start, jump);
        graph.connect(jump, end);

        let script = to_script(&graph);

        // Should have Jump event
        let has_jump = script
            .events
            .iter()
            .any(|e| matches!(e, EventRaw::Jump { .. }));
        assert!(has_jump, "Script should contain a Jump event");
    }
}
