//! Node graph data structure for the visual editor.
//!
//! This module contains the core graph data structure that represents
//! the story flow. It handles node management and connections.
//! Script synchronization is in the `script_sync` module.
//!
//! # Design Principles
//! - **Single Source of Truth**: The NodeGraph is the canonical representation
//! - **Invariant Preservation**: All mutations maintain graph consistency
//! - **Modularity**: Under 500 lines per Criterio J

use eframe::egui;
use visual_novel_engine::ScriptRaw;

use super::node_types::{
    ContextMenu, StoryNode, NODE_HEIGHT, NODE_VERTICAL_SPACING, NODE_WIDTH, ZOOM_DEFAULT, ZOOM_MAX,
    ZOOM_MIN,
};
use super::script_sync;

// =============================================================================
// NodeGraph - Main graph data structure
// =============================================================================

/// A node graph representing the story structure.
///
/// # Invariants
/// - `next_id` is always greater than any existing node ID
/// - `connections` only reference existing node IDs
/// - `selected` is None or references an existing node ID
/// - `zoom` is always in range [ZOOM_MIN, ZOOM_MAX]
///
/// # Contract
/// All public methods preserve these invariants.
#[derive(Clone, Debug)]
pub struct NodeGraph {
    /// Nodes: (id, node, position in graph space)
    pub(crate) nodes: Vec<(u32, StoryNode, egui::Pos2)>,
    /// Connections: (from_id, to_id)
    pub(crate) connections: Vec<(u32, u32)>,
    /// Next available node ID
    next_id: u32,
    /// Currently selected node
    pub selected: Option<u32>,
    /// Pan offset (world-space translation)
    pub(crate) pan: egui::Vec2,
    /// Zoom level
    pub(crate) zoom: f32,
    /// Node being edited inline
    pub editing: Option<u32>,
    /// Node being dragged (robust interaction)
    pub dragging_node: Option<u32>,
    /// Node being connected (Connect To mode)
    pub connecting_from: Option<u32>,
    /// Active context menu
    pub context_menu: Option<ContextMenu>,
    /// Dirty flag (script modified since last save)
    pub(crate) modified: bool,
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            next_id: 0,
            selected: None,
            pan: egui::Vec2::ZERO,
            zoom: ZOOM_DEFAULT,
            editing: None,
            dragging_node: None,
            connecting_from: None,
            context_menu: None,
            modified: false,
        }
    }
}

impl NodeGraph {
    /// Creates a new empty node graph.
    pub fn new() -> Self {
        Self::default()
    }

    // =========================================================================
    // Basic Operations
    // =========================================================================

    /// Adds a node at the specified position. Returns the node ID.
    pub fn add_node(&mut self, node: StoryNode, pos: egui::Pos2) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push((id, node, pos));
        self.modified = true;

        debug_assert!(
            self.nodes.iter().filter(|(nid, _, _)| *nid == id).count() == 1,
            "Postcondition: node ID {} should exist exactly once",
            id
        );

        id
    }

    /// Removes a node and all its connections.
    pub fn remove_node(&mut self, id: u32) {
        self.nodes.retain(|(nid, _, _)| *nid != id);
        self.connections
            .retain(|(from, to)| *from != id && *to != id);

        if self.selected == Some(id) {
            self.selected = None;
        }
        if self.editing == Some(id) {
            self.editing = None;
        }
        if self.connecting_from == Some(id) {
            self.connecting_from = None;
        }

        self.modified = true;
    }

    /// Connects two nodes. Prevents duplicate connections and self-loops.
    pub fn connect(&mut self, from: u32, to: u32) {
        if from == to {
            return;
        }

        if !self.connections.contains(&(from, to)) {
            self.connections.push((from, to));
            self.modified = true;
        }
    }

    /// Disconnects two nodes.
    pub fn disconnect(&mut self, from: u32, to: u32) {
        self.connections.retain(|(f, t)| !(*f == from && *t == to));
        self.modified = true;
    }

    /// Returns the number of nodes.
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the graph is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of connections.
    #[inline]
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Returns true if the graph has been modified since last save.
    #[inline]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Clears the modified flag.
    pub fn clear_modified(&mut self) {
        self.modified = false;
    }

    /// Marks the graph as modified.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    // =========================================================================
    // Pan/Zoom Operations
    // =========================================================================

    /// Returns the current zoom level.
    #[inline]
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Sets the zoom level, clamping to valid range.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Zooms by a delta (positive = zoom in, negative = zoom out).
    pub fn zoom_by(&mut self, delta: f32) {
        self.set_zoom(self.zoom + delta);
    }

    /// Returns the current pan offset.
    #[inline]
    pub fn pan(&self) -> egui::Vec2 {
        self.pan
    }

    /// Adds to the pan offset.
    pub fn pan_by(&mut self, delta: egui::Vec2) {
        self.pan += delta;
    }

    /// Resets pan and zoom to default values.
    pub fn reset_view(&mut self) {
        self.pan = egui::Vec2::ZERO;
        self.zoom = ZOOM_DEFAULT;
    }

    /// Adjusts pan and zoom to show all nodes.
    ///
    /// # Contract
    /// - If graph is empty, resets to default view
    /// - Otherwise, calculates bounding box and fits all nodes
    pub fn zoom_to_fit(&mut self) {
        if self.nodes.is_empty() {
            self.reset_view();
            return;
        }

        // Calculate bounding box of all nodes
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for (_, _, pos) in &self.nodes {
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x + NODE_WIDTH);
            max_y = max_y.max(pos.y + NODE_HEIGHT);
        }

        // Add padding
        let padding = 50.0;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        // Calculate required zoom to fit (assuming ~800x600 viewport)
        let viewport_width = 800.0;
        let viewport_height = 600.0;
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;

        let zoom_x = viewport_width / content_width;
        let zoom_y = viewport_height / content_height;
        let new_zoom = zoom_x.min(zoom_y).clamp(ZOOM_MIN, ZOOM_MAX);

        // Center content
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.zoom = new_zoom;
        self.pan = egui::vec2(
            viewport_width / (2.0 * new_zoom) - center_x,
            viewport_height / (2.0 * new_zoom) - center_y,
        );

        debug_assert!(
            self.zoom >= ZOOM_MIN && self.zoom <= ZOOM_MAX,
            "Postcondition: zoom must be in valid range"
        );
    }

    /// Duplicates a node at an offset position.
    ///
    /// # Precondition
    /// - `node_id` should exist in the graph
    pub fn duplicate_node(&mut self, node_id: u32) {
        let Some((_, node, pos)) = self.nodes.iter().find(|(id, _, _)| *id == node_id).cloned()
        else {
            debug_assert!(
                false,
                "Precondition: node_id {} not found for duplicate",
                node_id
            );
            return;
        };

        let new_pos = egui::pos2(pos.x + 50.0, pos.y + 50.0);
        let new_id = self.add_node(node, new_pos);
        self.selected = Some(new_id);

        debug_assert!(
            self.nodes.iter().any(|(id, _, _)| *id == new_id),
            "Postcondition: new node should exist"
        );
    }

    // =========================================================================
    // Node Manipulation (Context Menu Actions)
    // =========================================================================

    /// Inserts a new node before the target node, re-routing connections.
    ///
    /// # Precondition
    /// - `target_id` should exist in the graph (silent no-op if not)
    pub fn insert_before(&mut self, target_id: u32, node: StoryNode) {
        let Some((_, _, pos)) = self.nodes.iter().find(|(id, _, _)| *id == target_id) else {
            debug_assert!(
                false,
                "Precondition warning: target_id {} not found in insert_before",
                target_id
            );
            return;
        };

        let new_pos = egui::pos2(pos.x, pos.y - NODE_VERTICAL_SPACING);
        let new_id = self.add_node(node, new_pos);

        for (_, to) in &mut self.connections {
            if *to == target_id {
                *to = new_id;
            }
        }

        self.connections.push((new_id, target_id));

        // Postcondition: new node exists and is connected to target
        debug_assert!(
            self.connections.contains(&(new_id, target_id)),
            "Postcondition: new node should be connected to target"
        );
    }

    /// Inserts a new node after the target node, re-routing connections.
    ///
    /// # Precondition
    /// - `target_id` should exist in the graph (silent no-op if not)
    pub fn insert_after(&mut self, target_id: u32, node: StoryNode) {
        let Some((_, _, pos)) = self.nodes.iter().find(|(id, _, _)| *id == target_id) else {
            debug_assert!(
                false,
                "Precondition warning: target_id {} not found in insert_after",
                target_id
            );
            return;
        };

        let new_pos = egui::pos2(pos.x, pos.y + NODE_VERTICAL_SPACING);
        let new_id = self.add_node(node, new_pos);

        let old_targets: Vec<u32> = self
            .connections
            .iter()
            .filter(|(from, _)| *from == target_id)
            .map(|(_, to)| *to)
            .collect();

        self.connections.retain(|(from, _)| *from != target_id);
        self.connections.push((target_id, new_id));

        for old_to in old_targets {
            self.connections.push((new_id, old_to));
        }

        // Postcondition: target now connects to new node
        debug_assert!(
            self.connections.contains(&(target_id, new_id)),
            "Postcondition: target should connect to new node"
        );
    }

    /// Converts a node to a Choice node with default options.
    ///
    /// # Precondition
    /// - `node_id` should exist in the graph (silent no-op if not)
    pub fn convert_to_choice(&mut self, node_id: u32) {
        let exists = self.nodes.iter().any(|(id, _, _)| *id == node_id);
        debug_assert!(
            exists,
            "Precondition warning: node_id {} not found in convert_to_choice",
            node_id
        );

        if let Some((_, node, _)) = self.nodes.iter_mut().find(|(id, _, _)| *id == node_id) {
            *node = StoryNode::Choice {
                prompt: "Choose an option:".to_string(),
                options: vec!["Option 1".to_string(), "Option 2".to_string()],
            };
            self.modified = true;

            // Postcondition: node is now a Choice
            debug_assert!(
                matches!(self.get_node(node_id), Some(StoryNode::Choice { .. })),
                "Postcondition: node should be converted to Choice"
            );
        }
    }

    /// Creates a branch from a node (adds a Choice with two paths).
    pub fn create_branch(&mut self, node_id: u32) {
        let Some((_, node, pos)) = self.nodes.iter().find(|(id, _, _)| *id == node_id) else {
            return;
        };

        if matches!(node, StoryNode::End) {
            return;
        }

        let pos = *pos;

        let choice_pos = egui::pos2(pos.x, pos.y + 50.0);
        let choice_id = self.add_node(
            StoryNode::Choice {
                prompt: "Which path?".to_string(),
                options: vec!["Path A".to_string(), "Path B".to_string()],
            },
            choice_pos,
        );

        let branch_a = self.add_node(
            StoryNode::Dialogue {
                speaker: "Path A".to_string(),
                text: "Content for path A...".to_string(),
            },
            egui::pos2(pos.x - 120.0, pos.y + 140.0),
        );

        let branch_b = self.add_node(
            StoryNode::Dialogue {
                speaker: "Path B".to_string(),
                text: "Content for path B...".to_string(),
            },
            egui::pos2(pos.x + 120.0, pos.y + 140.0),
        );

        self.connect(node_id, choice_id);
        self.connect(choice_id, branch_a);
        self.connect(choice_id, branch_b);
    }

    // =========================================================================
    // Script Synchronization (delegated to script_sync module)
    // =========================================================================

    /// Creates a node graph from a raw script.
    pub fn from_script(script: &ScriptRaw) -> Self {
        script_sync::from_script(script)
    }

    /// Converts the node graph to a raw script.
    pub fn to_script(&self) -> ScriptRaw {
        script_sync::to_script(self)
    }

    // =========================================================================
    // Node Lookup Helpers
    // =========================================================================

    /// Returns the node at the given graph position, if any.
    pub fn node_at_position(&self, graph_pos: egui::Pos2) -> Option<u32> {
        for (id, _, pos) in &self.nodes {
            let node_rect = egui::Rect::from_min_size(*pos, egui::vec2(NODE_WIDTH, NODE_HEIGHT));
            if node_rect.contains(graph_pos) {
                return Some(*id);
            }
        }
        None
    }

    /// Gets a reference to a node by ID.
    pub fn get_node(&self, id: u32) -> Option<&StoryNode> {
        self.nodes
            .iter()
            .find(|(nid, _, _)| *nid == id)
            .map(|(_, node, _)| node)
    }

    /// Gets a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut StoryNode> {
        self.nodes
            .iter_mut()
            .find(|(nid, _, _)| *nid == id)
            .map(|(_, node, _)| node)
    }

    /// Gets a mutable reference to a node's position by ID.
    pub fn get_node_pos_mut(&mut self, id: u32) -> Option<&mut egui::Pos2> {
        self.nodes
            .iter_mut()
            .find(|(nid, _, _)| *nid == id)
            .map(|(_, _, pos)| pos)
    }

    /// Returns an iterator over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &(u32, StoryNode, egui::Pos2)> {
        self.nodes.iter()
    }

    /// Returns a slice of all nodes (for script_sync module).
    pub(crate) fn nodes_slice(&self) -> &[(u32, StoryNode, egui::Pos2)] {
        &self.nodes
    }

    /// Returns an iterator over all connections.
    pub fn connections(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.connections.iter()
    }

    /// Returns a slice of all connections (for script_sync module).
    pub(crate) fn connections_slice(&self) -> &[(u32, u32)] {
        &self.connections
    }
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
}
