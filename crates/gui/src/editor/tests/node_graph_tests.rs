use super::*;

fn pos(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(x, y)
}

#[test]
fn test_node_graph_new_is_empty() {
    let graph = NodeGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert_eq!(graph.connection_count(), 0);
    assert!(!graph.is_modified());
}

#[test]
fn test_node_graph_add_node() {
    let mut graph = NodeGraph::new();
    let id1 = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let id2 = graph.add_node(StoryNode::End, pos(100.0, 100.0));
    assert_eq!(graph.len(), 2);
    assert_ne!(id1, id2);
    assert!(graph.is_modified());
}

#[test]
fn test_node_graph_remove_node() {
    let mut graph = NodeGraph::new();
    let id1 = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let id2 = graph.add_node(StoryNode::End, pos(100.0, 100.0));
    graph.connect(id1, id2);
    graph.remove_node(id1);
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.connection_count(), 0);
}

#[test]
fn test_node_graph_connect() {
    let mut graph = NodeGraph::new();
    let a = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let b = graph.add_node(StoryNode::End, pos(100.0, 100.0));
    graph.connect(a, b);
    assert_eq!(graph.connection_count(), 1);
    graph.connect(a, b); // Duplicate
    assert_eq!(graph.connection_count(), 1);
}

#[test]
fn test_node_graph_self_loop_prevented() {
    let mut graph = NodeGraph::new();
    let a = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    graph.connect(a, a);
    assert_eq!(graph.connection_count(), 0);
}

#[test]
fn test_zoom_clamp() {
    let mut graph = NodeGraph::new();
    graph.set_zoom(0.0);
    assert_eq!(graph.zoom(), ZOOM_MIN);
    graph.set_zoom(10.0);
    assert_eq!(graph.zoom(), ZOOM_MAX);
}

#[test]
fn test_insert_before() {
    let mut graph = NodeGraph::new();
    let a = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let c = graph.add_node(StoryNode::End, pos(0.0, 100.0));
    graph.connect(a, c);
    graph.insert_before(c, StoryNode::default());
    assert_eq!(graph.len(), 3);
    assert_eq!(graph.connection_count(), 2);
}

#[test]
fn test_insert_after() {
    let mut graph = NodeGraph::new();
    let a = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let c = graph.add_node(StoryNode::End, pos(0.0, 100.0));
    graph.connect(a, c);
    graph.insert_after(a, StoryNode::default());
    assert_eq!(graph.len(), 3);
    assert_eq!(graph.connection_count(), 2);
}

#[test]
fn test_create_branch() {
    let mut graph = NodeGraph::new();
    let a = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    graph.create_branch(a);
    assert_eq!(graph.len(), 4);
    assert_eq!(graph.connection_count(), 3);
}

#[test]
fn test_create_branch_from_end_does_nothing() {
    let mut graph = NodeGraph::new();
    let end = graph.add_node(StoryNode::End, pos(0.0, 0.0));
    graph.create_branch(end);
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.connection_count(), 0);
}

#[test]
fn test_connecting_choice_port_auto_creates_option() {
    let mut graph = NodeGraph::new();
    let start = graph.add_node(StoryNode::Start, pos(0.0, 0.0));
    let choice = graph.add_node(
        StoryNode::Choice {
            prompt: "Select".to_string(),
            options: vec!["A".to_string()],
        },
        pos(0.0, 100.0),
    );
    let target = graph.add_node(
        StoryNode::Dialogue {
            speaker: "N".to_string(),
            text: "B".to_string(),
        },
        pos(200.0, 100.0),
    );

    graph.connect(start, choice);
    graph.connect_port(choice, 1, target);

    let Some(StoryNode::Choice { options, .. }) = graph.get_node(choice) else {
        panic!("choice node should exist");
    };
    assert_eq!(options.len(), 2);
    assert_eq!(graph.connection_count(), 2);
}

#[test]
fn test_disconnect_port_removes_only_selected_output_port() {
    let mut graph = NodeGraph::new();
    let choice = graph.add_node(
        StoryNode::Choice {
            prompt: "Select".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
        },
        pos(0.0, 0.0),
    );
    let a = graph.add_node(StoryNode::End, pos(-100.0, 100.0));
    let b = graph.add_node(StoryNode::End, pos(100.0, 100.0));

    graph.connect_port(choice, 0, a);
    graph.connect_port(choice, 1, b);
    assert_eq!(graph.connection_count(), 2);

    graph.disconnect_port(choice, 1);
    assert_eq!(graph.connection_count(), 1);
    assert!(graph
        .connections()
        .any(|c| c.from == choice && c.from_port == 0));
}

#[test]
fn test_scene_profile_save_and_apply() {
    let mut graph = NodeGraph::new();
    let scene = graph.add_node(
        StoryNode::Scene {
            profile: None,
            background: Some("bg/room.png".to_string()),
            music: Some("bgm/theme.ogg".to_string()),
            characters: vec![visual_novel_engine::CharacterPlacementRaw {
                name: "Ava".to_string(),
                ..Default::default()
            }],
        },
        pos(0.0, 0.0),
    );
    let other_scene = graph.add_node(
        StoryNode::Scene {
            profile: None,
            background: None,
            music: None,
            characters: Vec::new(),
        },
        pos(0.0, 120.0),
    );

    assert!(graph.save_scene_profile("intro", scene));
    assert!(graph.apply_scene_profile("intro", other_scene));

    let Some(StoryNode::Scene {
        profile,
        background,
        music,
        characters,
    }) = graph.get_node(other_scene)
    else {
        panic!("expected scene node");
    };

    assert_eq!(profile.as_deref(), Some("intro"));
    assert_eq!(background.as_deref(), Some("bg/room.png"));
    assert_eq!(music.as_deref(), Some("bgm/theme.ogg"));
    assert_eq!(characters.len(), 1);
}
