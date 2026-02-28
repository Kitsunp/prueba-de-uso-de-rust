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
                background: s.background.clone(),
                music: s.music.clone(),
                characters: s.characters.clone(),
            },
            EventRaw::Jump { target } => StoryNode::Jump {
                target: target.clone(),
            },
            EventRaw::SetVar { key, value } => StoryNode::SetVariable {
                key: key.clone(),
                value: *value,
            },
            EventRaw::JumpIf { cond, target } => StoryNode::JumpIf {
                target: target.clone(),
                cond: cond.clone(),
            },
            EventRaw::Patch(patch) => StoryNode::ScenePatch(patch.clone()),
            EventRaw::ExtCall {
                command: _command, ..
            } => StoryNode::Generic(event.clone()), // Use Generic for ExtCall for now
            EventRaw::AudioAction(action) => StoryNode::AudioAction {
                channel: action.channel.clone(),
                action: action.action.clone(),
                asset: action.asset.clone(),
                volume: action.volume,
                fade_duration_ms: action.fade_duration_ms,
                loop_playback: action.loop_playback,
            },
            EventRaw::Transition(trans) => StoryNode::Transition {
                kind: trans.kind.clone(),
                duration_ms: trans.duration_ms,
                color: trans.color.clone(),
            },
            EventRaw::SetCharacterPosition(pos) => StoryNode::CharacterPlacement {
                name: pos.name.clone(),
                x: pos.x,
                y: pos.y,
                scale: pos.scale,
            },
            // CRITICAL: Capture any unhandled event as a GenericNode to prevent data loss.
            other => StoryNode::Generic(other.clone()),
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
                for (opt_idx, option) in c.options.iter().enumerate() {
                    if let Some(&target_idx) = label_to_index.get(option.target.as_str()) {
                        if let Some(&target_id) = index_to_id.get(&target_idx) {
                            // Smart Branching: Connect from specific option port
                            graph.connect_port(from_id, opt_idx, target_id);
                        }
                    }
                }
            }
            EventRaw::JumpIf { target, .. } => {
                // Handle JumpIf logic flow: it can go to target OR next
                if let Some(&target_idx) = label_to_index.get(target.as_str()) {
                    if let Some(&target_id) = index_to_id.get(&target_idx) {
                        graph.connect(from_id, target_id);
                    }
                }
                // Also flow to next (fallthrough)
                if let Some(&next_id) = index_to_id.get(&(idx + 1)) {
                    graph.connect(from_id, next_id);
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
    // Use GraphConnection struct access
    let nodes_with_outgoing: Vec<u32> = graph.connections().map(|c| c.from).collect();
    let nodes_to_connect: Vec<u32> = graph
        .nodes()
        .map(|(id, _, _)| *id)
        .filter(|id| {
            !nodes_with_outgoing.contains(id)
                && !matches!(graph.get_node(*id), Some(StoryNode::End))
        })
        .collect();

    for id in nodes_to_connect {
        graph.connect(id, end_id);
    }

    graph.clear_modified();
    graph
}

/// Converts a NodeGraph to a raw script.
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

        for conn in graph.connections() {
            if conn.from == id && !visited.contains(&conn.to) {
                queue.push(conn.to);
            }
        }
    }

    // Convert visited nodes to events
    let mut node_to_label: BTreeMap<u32, String> = BTreeMap::new();

    for &id in &visited {
        let Some((_, node, _)) = graph.nodes().find(|(nid, _, _)| *nid == id) else {
            continue;
        };

        // We only generate labels/events for non-marker nodes
        if node.is_marker() {
            continue;
        }

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
                // Collect outgoing connections per port
                // We map options indices to targets
                let choice_options: Vec<ChoiceOptionRaw> = options
                    .iter()
                    .enumerate()
                    .map(|(i, text)| {
                        // Find connection from this port
                        let target_node = graph
                            .connections()
                            .find(|c| c.from == id && c.from_port == i)
                            .map(|c| c.to);

                        let target = target_node
                            .and_then(|tid| {
                                let node = graph.get_node(tid)?;
                                let label = match node {
                                    StoryNode::Start => "start".to_string(),
                                    StoryNode::End => "__end".to_string(),
                                    _ => format!("node_{}", tid),
                                };
                                Some(label)
                            })
                            .unwrap_or_else(|| format!("__unlinked_node_{}_option_{}", id, i));

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
            StoryNode::SetVariable { key, value } => {
                events.push(EventRaw::SetVar {
                    key: key.clone(),
                    value: *value,
                });
            }
            StoryNode::JumpIf { cond, target } => {
                events.push(EventRaw::JumpIf {
                    cond: cond.clone(),
                    target: target.clone(),
                });
            }
            StoryNode::ScenePatch(patch) => {
                events.push(EventRaw::Patch(patch.clone()));
            }
            StoryNode::Scene {
                background,
                music,
                characters,
            } => {
                events.push(EventRaw::Scene(SceneUpdateRaw {
                    background: background.clone(),
                    music: music.clone(),
                    characters: characters.clone(),
                }));
            }
            StoryNode::AudioAction {
                channel,
                action,
                asset,
                volume,
                fade_duration_ms,
                loop_playback,
            } => {
                events.push(EventRaw::AudioAction(visual_novel_engine::AudioActionRaw {
                    channel: channel.clone(),
                    action: action.clone(),
                    asset: asset.clone(),
                    volume: *volume,
                    fade_duration_ms: *fade_duration_ms,
                    loop_playback: *loop_playback,
                }));
            }
            StoryNode::Transition {
                kind,
                duration_ms,
                color,
            } => {
                events.push(EventRaw::Transition(
                    visual_novel_engine::SceneTransitionRaw {
                        kind: kind.clone(),
                        duration_ms: *duration_ms,
                        color: color.clone(),
                    },
                ));
            }
            StoryNode::CharacterPlacement { name, x, y, scale } => {
                events.push(EventRaw::SetCharacterPosition(
                    visual_novel_engine::SetCharacterPositionRaw {
                        name: name.clone(),
                        x: *x,
                        y: *y,
                        scale: *scale,
                    },
                ));
            }
            StoryNode::Generic(event) => {
                // Pass through the generic event
                events.push(event.clone());
            }
            StoryNode::Start | StoryNode::End => {
                // Skip start/end markers
            }
        }
    }

    // Add synthetic end label when at least one edge explicitly targets End marker.
    if events.iter().any(|event| match event {
        EventRaw::Jump { target } => target == "__end",
        EventRaw::JumpIf { target, .. } => target == "__end",
        EventRaw::Choice(choice) => choice.options.iter().any(|option| option.target == "__end"),
        _ => false,
    }) {
        labels.insert("__end".to_string(), events.len());
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
    use visual_novel_engine::CharacterPlacementRaw;

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
    fn test_roundtrip_scene_preserves_music_and_characters() {
        let mut labels = BTreeMap::new();
        labels.insert("start".to_string(), 0);

        let events = vec![EventRaw::Scene(SceneUpdateRaw {
            background: Some("bg/room.png".to_string()),
            music: Some("bgm/theme.ogg".to_string()),
            characters: vec![CharacterPlacementRaw {
                name: "Ava".to_string(),
                expression: Some("smile".to_string()),
                position: Some("left".to_string()),
                x: Some(10),
                y: Some(20),
                scale: Some(1.2),
            }],
        })];

        let original = ScriptRaw::new(events, labels);
        let graph = from_script(&original);
        let roundtrip = to_script(&graph);

        let Some(EventRaw::Scene(scene)) = roundtrip.events.first() else {
            panic!("Expected first event to be scene");
        };
        assert_eq!(scene.background.as_deref(), Some("bg/room.png"));
        assert_eq!(scene.music.as_deref(), Some("bgm/theme.ogg"));
        assert_eq!(scene.characters.len(), 1);
        assert_eq!(scene.characters[0].name, "Ava");
    }

    #[test]
    fn test_unlinked_choice_option_is_explicitly_marked() {
        let mut graph = NodeGraph::new();
        let start_id = graph.add_node(StoryNode::Start, egui::pos2(50.0, 30.0));
        let choice_id = graph.add_node(
            StoryNode::Choice {
                prompt: "Elige".to_string(),
                options: vec!["A".to_string(), "B".to_string()],
            },
            egui::pos2(100.0, 120.0),
        );
        graph.connect(start_id, choice_id);

        let script = to_script(&graph);
        let Some(EventRaw::Choice(choice)) = script.events.first() else {
            panic!("Expected first event to be choice");
        };
        assert_eq!(choice.options.len(), 2);
        assert!(choice.options[0].target.starts_with("__unlinked_node_"));
        assert!(choice.options[1].target.starts_with("__unlinked_node_"));
    }
}
