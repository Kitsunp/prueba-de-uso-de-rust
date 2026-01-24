//! Graph panel for the editor workbench.
//!
//! Displays the story flow as a visual graph with nodes and edges.

use crate::editor::{NodeGraph, StoryNode};
use eframe::egui;

/// Graph panel widget.
pub struct GraphPanel<'a> {
    graph: &'a mut NodeGraph,
}

impl<'a> GraphPanel<'a> {
    pub fn new(graph: &'a mut NodeGraph) -> Self {
        Self { graph }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Story Graph");
        ui.separator();

        let node_count = self.graph.len();
        let connection_count = self.graph.connection_count();

        // Statistics
        ui.horizontal(|ui| {
            ui.label(format!("Nodes: {}", node_count));
            ui.separator();
            ui.label(format!("Edges: {}", connection_count));
        });

        ui.separator();

        // Node list
        // snapshot IDs to avoid borrow conflict while iterating?
        // We need to iterate and potentially mutate selection.
        // We can't iterate `self.graph.nodes` (borrow) and mutate `self.graph.selected` (borrow mut).
        // So we collect a list of (id, type_info) first.
        let nodes: Vec<(u32, String, egui::Color32)> = self
            .graph
            .nodes()
            .map(|(id, node, _)| {
                let info = match node {
                    StoryNode::Dialogue { speaker, text } => {
                        format!("💬 {}: {}", speaker, truncate(text, 20))
                    }
                    StoryNode::Choice { prompt, .. } => format!("🔀 {}", truncate(prompt, 20)),
                    StoryNode::Scene { background } => format!("🎬 {}", truncate(background, 20)),
                    StoryNode::Jump { target } => format!("↪ Jump to {}", target),
                    StoryNode::Start => "▶ Start".to_string(),
                    StoryNode::End => "⏹ End".to_string(),
                };
                (*id, info, node.color())
            })
            .collect();

        let mut new_selection = None;
        let current_selection = self.graph.selected;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, text, color) in nodes {
                let is_selected = current_selection == Some(id);
                let response = ui.selectable_label(
                    is_selected,
                    egui::RichText::new(format!("{}: {}", id, text)).color(color),
                );

                if response.clicked() {
                    new_selection = Some(id);
                }
            }
        });

        if let Some(id) = new_selection {
            self.graph.selected = Some(id);
        }
    }
}

/// Truncates a string to a certain length with ellipsis.
pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max_chars).collect();
        result.push('…');
        result
    }
}
