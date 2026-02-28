use super::*;

#[test]
fn test_node_editor_panel_creation() {
    let mut graph = NodeGraph::new();
    let mut undo = UndoStack::new();
    let _panel = NodeEditorPanel::new(&mut graph, &mut undo);
}
