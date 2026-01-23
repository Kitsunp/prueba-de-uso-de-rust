//! Node editor panel for the visual editor workbench.
//!
//! This module provides the UI widget for the visual graph editor.
//! The data structures (`NodeGraph`, `StoryNode`) are in separate modules.
//! Rendering utilities are in `node_rendering`.
//!
//! # Design Principles
//! - **Modularity**: UI separated from data (Criterio J ≤500 lines)
//! - **Single Responsibility**: Only rendering and input handling

use eframe::egui;

use super::node_graph::NodeGraph;
use super::node_rendering;
use super::node_types::{ContextMenu, StoryNode, NODE_HEIGHT, NODE_WIDTH};

// =============================================================================
// NodeEditorPanel - UI Widget
// =============================================================================

/// Node editor panel widget with pan/zoom and context menu.
pub struct NodeEditorPanel<'a> {
    graph: &'a mut NodeGraph,
}

impl<'a> NodeEditorPanel<'a> {
    pub fn new(graph: &'a mut NodeGraph) -> Self {
        Self { graph }
    }

    #[inline]
    fn graph_to_screen(&self, rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
        rect.min + (pos.to_vec2() + self.graph.pan()) * self.graph.zoom()
    }

    #[inline]
    /// Transforms a screen-space position to graph-space.
    #[allow(dead_code)]
    fn screen_to_graph(&self, rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
        ((pos - rect.min) / self.graph.zoom() - self.graph.pan()).to_pos2()
    }

    /// Main UI entry point.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Node Editor");
        ui.separator();

        self.render_toolbar(ui);
        ui.separator();

        let available_size = ui.available_size();
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        painter.rect_filled(rect, 5.0, egui::Color32::from_rgb(25, 25, 35));

        self.render_grid(&painter, rect);
        self.handle_input(ui, &response);
        self.render_connections(&painter, rect);
        self.render_nodes(ui, &painter, rect, &response);
        self.render_connecting_line(&painter, rect, &response);
        node_rendering::render_context_menu(self.graph, ui);
        node_rendering::render_inline_editor(self.graph, ui);
        self.render_status_bar(&painter, rect);
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("➕ Add Node", |ui| {
                let pos = egui::pos2(100.0, 100.0) - self.graph.pan().to_pos2().to_vec2();
                if ui.button("💬 Dialogue").clicked() {
                    self.graph.add_node(StoryNode::default(), pos);
                    ui.close_menu();
                }
                if ui.button("🔀 Choice").clicked() {
                    self.graph.add_node(
                        StoryNode::Choice {
                            prompt: "Choose:".to_string(),
                            options: vec!["A".to_string(), "B".to_string()],
                        },
                        pos,
                    );
                    ui.close_menu();
                }
                if ui.button("🎬 Scene").clicked() {
                    self.graph.add_node(
                        StoryNode::Scene {
                            background: "bg.png".to_string(),
                        },
                        pos,
                    );
                    ui.close_menu();
                }
                if ui.button("↪ Jump").clicked() {
                    self.graph.add_node(
                        StoryNode::Jump {
                            target: "label".to_string(),
                        },
                        pos,
                    );
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("▶ Start").clicked() {
                    self.graph
                        .add_node(StoryNode::Start, egui::pos2(50.0, 30.0));
                    ui.close_menu();
                }
                if ui.button("⏹ End").clicked() {
                    self.graph
                        .add_node(StoryNode::End, egui::pos2(200.0, 300.0));
                    ui.close_menu();
                }
            });

            ui.separator();
            if ui.button("🔍 Reset View").clicked() {
                self.graph.reset_view();
            }
            ui.label(format!("Zoom: {:.0}%", self.graph.zoom() * 100.0));
            ui.separator();
            ui.label(format!(
                "Nodes: {} | Connections: {}",
                self.graph.len(),
                self.graph.connection_count()
            ));
            if self.graph.is_modified() {
                ui.label("⚠ Modified");
            }
        });
    }

    fn render_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_spacing = 50.0 * self.graph.zoom();
        let grid_color = egui::Color32::from_rgba_unmultiplied(80, 80, 100, 40);
        let offset_x = (self.graph.pan().x * self.graph.zoom()) % grid_spacing;
        let offset_y = (self.graph.pan().y * self.graph.zoom()) % grid_spacing;

        let mut x = rect.min.x + offset_x;
        while x < rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, grid_color),
            );
            x += grid_spacing;
        }
        let mut y = rect.min.y + offset_y;
        while y < rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += grid_spacing;
        }
    }

    fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        // Pan with middle mouse or Ctrl+drag
        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && ui.input(|i| i.modifiers.ctrl))
        {
            self.graph.pan_by(response.drag_delta() / self.graph.zoom());
        }

        // Zoom with scroll wheel
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > 0.0 {
            self.graph.zoom_by(scroll_delta * 0.002);
        }

        // Double-click to reset view
        if response.double_clicked() {
            self.graph.reset_view();
        }

        // Escape to cancel modes
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.graph.connecting_from = None;
            self.graph.context_menu = None;
        }

        // Click outside to close menu
        if response.clicked() && self.graph.context_menu.is_some() {
            self.graph.context_menu = None;
        }

        // === Zoom Keyboard Shortcuts ===
        // + or = to zoom in
        if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            self.graph.zoom_by(0.1);
        }
        // - to zoom out
        if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            self.graph.zoom_by(-0.1);
        }
        // 0 to reset view
        if ui.input(|i| i.key_pressed(egui::Key::Num0)) {
            self.graph.reset_view();
        }
        // H to zoom-to-fit all nodes
        if ui.input(|i| i.key_pressed(egui::Key::H)) {
            self.graph.zoom_to_fit();
        }

        // === Node Action Shortcuts ===
        // Delete or Backspace to remove selected node
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if let Some(id) = self.graph.selected {
                self.graph.remove_node(id);
                self.graph.selected = None;
            }
        }
        // E to edit selected node
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            if let Some(id) = self.graph.selected {
                self.graph.editing = Some(id);
            }
        }
        // Ctrl+D to duplicate selected node
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            if let Some(id) = self.graph.selected {
                self.graph.duplicate_node(id);
            }
        }
    }

    fn render_connections(&self, painter: &egui::Painter, rect: egui::Rect) {
        for (from, to) in self.graph.connections() {
            let from_pos = self
                .graph
                .nodes()
                .find(|(id, _, _)| *id == *from)
                .map(|(_, _, p)| *p);
            let to_pos = self
                .graph
                .nodes()
                .find(|(id, _, _)| *id == *to)
                .map(|(_, _, p)| *p);
            if let (Some(from_pos), Some(to_pos)) = (from_pos, to_pos) {
                let from_screen = self.graph_to_screen(rect, from_pos)
                    + egui::vec2(NODE_WIDTH / 2.0, NODE_HEIGHT);
                let to_screen =
                    self.graph_to_screen(rect, to_pos) + egui::vec2(NODE_WIDTH / 2.0, 0.0);
                node_rendering::draw_bezier_connection(painter, from_screen, to_screen);
            }
        }
    }

    fn render_nodes(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        let mut clicked_node = None;
        let mut right_clicked_node = None;
        let mut double_clicked_node = None;
        let nodes: Vec<_> = self.graph.nodes().cloned().collect();

        for (id, node, pos) in &nodes {
            let screen_pos = self.graph_to_screen(rect, *pos);
            let size = egui::vec2(NODE_WIDTH, NODE_HEIGHT) * self.graph.zoom();
            let node_rect = egui::Rect::from_min_size(screen_pos, size);
            if !rect.intersects(node_rect) {
                continue;
            }

            let is_selected = self.graph.selected == Some(*id);
            let is_connecting = self.graph.connecting_from == Some(*id);
            let bg_color = if is_selected {
                node.color().linear_multiply(1.3)
            } else if is_connecting {
                egui::Color32::YELLOW.linear_multiply(0.3)
            } else {
                node.color()
            };

            painter.rect_filled(node_rect, 6.0 * self.graph.zoom(), bg_color);
            let border = if is_selected {
                egui::Color32::from_rgb(100, 150, 255)
            } else {
                egui::Color32::from_rgb(80, 80, 90)
            };
            painter.rect_stroke(
                node_rect,
                6.0 * self.graph.zoom(),
                egui::Stroke::new(2.0, border),
            );

            let font = 13.0 * self.graph.zoom();
            painter.text(
                node_rect.min + egui::vec2(8.0, 8.0) * self.graph.zoom(),
                egui::Align2::LEFT_TOP,
                format!("{} {}", node.icon(), node.type_name()),
                egui::FontId::proportional(font),
                egui::Color32::WHITE,
            );
            painter.text(
                node_rect.min + egui::vec2(8.0, 28.0) * self.graph.zoom(),
                egui::Align2::LEFT_TOP,
                self.get_node_preview(node),
                egui::FontId::proportional(11.0 * self.graph.zoom()),
                egui::Color32::LIGHT_GRAY,
            );

            if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if node_rect.contains(p) {
                        clicked_node = Some(*id);
                    }
                }
            }
            if response.secondary_clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if node_rect.contains(p) {
                        right_clicked_node = Some((*id, p));
                    }
                }
            }
            if ui.input(|i| {
                i.pointer
                    .button_double_clicked(egui::PointerButton::Primary)
            }) {
                if let Some(p) = response.interact_pointer_pos() {
                    if node_rect.contains(p) {
                        double_clicked_node = Some(*id);
                    }
                }
            }
        }

        if let Some(id) = clicked_node {
            if let Some(from) = self.graph.connecting_from.take() {
                if from != id {
                    self.graph.connect(from, id);
                }
            } else {
                self.graph.selected = Some(id);
            }
        }
        if let Some((id, pos)) = right_clicked_node {
            self.graph.context_menu = Some(ContextMenu {
                node_id: id,
                position: pos,
            });
        }
        if let Some(id) = double_clicked_node {
            self.graph.editing = Some(id);
        }
    }

    fn get_node_preview(&self, node: &StoryNode) -> String {
        match node {
            StoryNode::Dialogue { speaker, .. } => speaker.chars().take(15).collect(),
            StoryNode::Choice { prompt, .. } => prompt.chars().take(15).collect(),
            StoryNode::Scene { background } => background.chars().take(15).collect(),
            StoryNode::Jump { target } => {
                format!("→ {}", target.chars().take(10).collect::<String>())
            }
            StoryNode::Start => "Entry Point".to_string(),
            StoryNode::End => "Exit Point".to_string(),
        }
    }

    fn render_connecting_line(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        if let Some(from_id) = self.graph.connecting_from {
            if let Some((_, _, pos)) = self.graph.nodes().find(|(id, _, _)| *id == from_id) {
                if let Some(cursor) = response.hover_pos() {
                    let from = self.graph_to_screen(rect, *pos)
                        + egui::vec2(NODE_WIDTH / 2.0, NODE_HEIGHT);
                    painter.line_segment(
                        [from, cursor],
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    );
                }
            }
        }
    }

    fn render_status_bar(&self, painter: &egui::Painter, rect: egui::Rect) {
        let hint = if self.graph.connecting_from.is_some() {
            "🔗 Click a node to connect • Esc to cancel"
        } else {
            "Click to select • Right-click for menu • Double-click to edit"
        };
        painter.text(
            rect.max - egui::vec2(10.0, 10.0),
            egui::Align2::RIGHT_BOTTOM,
            hint,
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(120, 120, 130),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_editor_panel_creation() {
        let mut graph = NodeGraph::new();
        let _panel = NodeEditorPanel::new(&mut graph);
    }

    #[test]
    fn test_get_node_preview_dialogue() {
        let node = StoryNode::Dialogue {
            speaker: "Alice".to_string(),
            text: "Hello!".to_string(),
        };
        let mut graph = NodeGraph::new();
        let panel = NodeEditorPanel::new(&mut graph);
        assert_eq!(panel.get_node_preview(&node), "Alice");
    }
}
