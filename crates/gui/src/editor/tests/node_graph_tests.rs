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
