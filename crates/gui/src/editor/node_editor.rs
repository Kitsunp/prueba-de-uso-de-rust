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
use super::undo::UndoStack;

// =============================================================================
// NodeEditorPanel - UI Widget
// =============================================================================

/// Node editor panel widget with pan/zoom and context menu.
pub struct NodeEditorPanel<'a> {
    graph: &'a mut NodeGraph,
    undo_stack: &'a mut UndoStack,
}

impl<'a> NodeEditorPanel<'a> {
    pub fn new(graph: &'a mut NodeGraph, undo_stack: &'a mut UndoStack) -> Self {
        Self { graph, undo_stack }
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
                    let id = self.graph.add_node(StoryNode::default(), pos);
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
                if ui.button("🔀 Choice").clicked() {
                    let id = self.graph.add_node(
                        StoryNode::Choice {
                            prompt: "Choose:".to_string(),
                            options: vec!["A".to_string(), "B".to_string()],
                        },
                        pos,
                    );
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
                if ui.button("🎬 Scene").clicked() {
                    let id = self.graph.add_node(
                        StoryNode::Scene {
                            background: "bg.png".to_string(),
                        },
                        pos,
                    );
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
                if ui.button("↪ Jump").clicked() {
                    let id = self.graph.add_node(
                        StoryNode::Jump {
                            target: "label".to_string(),
                        },
                        pos,
                    );
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("▶ Start").clicked() {
                    let id = self
                        .graph
                        .add_node(StoryNode::Start, egui::pos2(50.0, 30.0));
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
                if ui.button("⏹ End").clicked() {
                    let id = self
                        .graph
                        .add_node(StoryNode::End, egui::pos2(200.0, 300.0));
                    self.graph.selected = Some(id);
                    ui.close_menu();
                }
            });

            ui.separator();
            if ui.button("🔍 Reset View").clicked() {
                self.graph.reset_view();
            }
            ui.label(format!("Zoom: {:.0}%", self.graph.zoom() * 100.0));

            ui.separator();

            // Undo/Redo
            if ui
                .add_enabled(self.undo_stack.can_undo(), egui::Button::new("↩"))
                .clicked()
            {
                if let Some(previous) = self.undo_stack.undo(self.graph.clone()) {
                    *self.graph = previous;
                }
            }
            if ui
                .add_enabled(self.undo_stack.can_redo(), egui::Button::new("↪"))
                .clicked()
            {
                if let Some(next) = self.undo_stack.redo(self.graph.clone()) {
                    *self.graph = next;
                }
            }

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
        // Pan with middle mouse, Ctrl+drag, or Left-drag on background
        // We check if we are NOT dragging a specific node context later, but here
        // response.dragged() means the painter (background) captured the drag.
        // If the click started on a node, egui usually consumes it if we used separate widgets,
        // but here we paint manually.
        // However, in render_nodes, we check press_origin.
        // If we drag the background, we want to pan.
        // WE MUST ensure we don't pan if we are actually moving a node.
        // The node drag logic updates node position. We should prevent panning if a node is being moved.
        // But handle_input runs BEFORE render_nodes.
        // Actually, if we use `response.dragged_by(PointerButton::Primary)`, it fires for both.
        // Strategy: Only pan if standard pan keys OR (Left Drag AND no node selected/hovered?)
        // Simpler: If the user presses Ctrl or Middle, explicit pan.
        // For implicit left-pan: likely need to know if a node was hit.
        // Efficient way: Check if hover_pos is contained in any node rect? No, expensive.
        // Better: Let's trust that users who want to pan simply drag empty space.
        // But if I drag a node, I don't want to move the camera.
        // The node drag logic is in `render_nodes`.
        // Let's implement a flag or logic:
        // For now, let's just ADD Left Drag to the existing condition, assuming conflict resolution happens elsewhere or isn't fatal.
        // Wait, if I drag a node, `response.dragged()` is true. If I pan, I move the node AND the camera. That's bad.
        // Correct fix: We need to know if we clicked a node.
        // We can check `self.graph.get_node_at(screen_pos)`? We don't have that easily without iterating.
        // Let's stick to the requested "Input: Habilitar Pan con Clic Izquierdo en el fondo".
        // Use `response.drag_start_pos()` if available or interact pointer.
        // If we can cheap-check if we started on a node.

        let mut is_panning = response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && ui.input(|i| i.modifiers.ctrl));

        // Attempt to allow comfortable left-drag pan (if no node under cursor)
        // This is tricky in immediate mode without hit-testing first.
        // Let's defer strict hit-testing and just allow it for now, relying on the user to grab nodes accurately?
        // No, that's buggy.
        // Alternative: Input handling usually happens *after* rendering in some architectures, or we do a hit test pass.
        // Let's iterate nodes inversely to check hit?
        // Since we have < 100 nodes typically, it's fast.

        if response.dragged_by(egui::PointerButton::Primary)
            && !is_panning
            && self.graph.dragging_node.is_none()
        {
            // Check if we started dragging on a node
            if let Some(_pos) = response.interact_pointer_pos() {
                // We need the START position of the drag, not current.
                if let Some(start_pos) = ui.input(|i| i.pointer.press_origin()) {
                    let mut started_on_node = false;
                    // Simple hit test
                    for (_, _, n_pos) in self.graph.nodes() {
                        let screen_pos = self.graph_to_screen(response.rect, *n_pos);
                        let size = egui::vec2(NODE_WIDTH, NODE_HEIGHT) * self.graph.zoom();
                        let rect = egui::Rect::from_min_size(screen_pos, size);
                        if rect.contains(start_pos) {
                            started_on_node = true;
                            break;
                        }
                    }

                    if !started_on_node {
                        is_panning = true;
                    }
                }
            }
        }

        if is_panning {
            self.graph.pan_by(response.drag_delta() / self.graph.zoom());
        }

        // Pan with Arrow Keys
        let pan_speed = 5.0; // Pixels per frame approx
        if ui.input(|i| i.key_down(egui::Key::ArrowUp)) {
            self.graph
                .pan_by(egui::vec2(0.0, pan_speed) / self.graph.zoom());
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowDown)) {
            self.graph
                .pan_by(egui::vec2(0.0, -pan_speed) / self.graph.zoom());
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowLeft)) {
            self.graph
                .pan_by(egui::vec2(pan_speed, 0.0) / self.graph.zoom());
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowRight)) {
            self.graph
                .pan_by(egui::vec2(-pan_speed, 0.0) / self.graph.zoom());
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
                // Fix: Scale the offsets by graph.zoom() so lines stay attached when zoomed out
                let offset_x = (NODE_WIDTH / 2.0) * self.graph.zoom();
                let offset_y = NODE_HEIGHT * self.graph.zoom();

                let from_screen =
                    self.graph_to_screen(rect, from_pos) + egui::vec2(offset_x, offset_y);

                let to_screen = self.graph_to_screen(rect, to_pos) + egui::vec2(offset_x, 0.0);

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

        // DRAG LOGIC: State Machine
        // 1. Handle Drag Start
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                // Check if any node was hit
                // Iterate in reverse to respect Z-order (last rendered is top)
                for (id, _, n_pos) in self.graph.nodes().collect::<Vec<_>>().iter().rev() {
                    let screen_pos = self.graph_to_screen(rect, *n_pos);
                    let size = egui::vec2(NODE_WIDTH, NODE_HEIGHT) * self.graph.zoom();
                    let node_rect = egui::Rect::from_min_size(screen_pos, size);
                    if node_rect.contains(pos) {
                        self.graph.dragging_node = Some(*id);
                        break;
                    }
                }
            }
        }

        // 2. Handle Dragging (continuous)
        if response.dragged_by(egui::PointerButton::Primary) && self.graph.context_menu.is_none() {
            if let Some(id) = self.graph.dragging_node {
                let delta = response.drag_delta() / self.graph.zoom();
                if let Some(node_pos) = self.graph.get_node_pos_mut(id) {
                    *node_pos += delta;
                    self.graph.mark_modified();
                }
            }
        }

        // 3. Handle Drag End
        if response.drag_stopped() {
            self.graph.dragging_node = None;
        }

        for (id, node, pos) in &nodes {
            let screen_pos = self.graph_to_screen(rect, *pos);
            let size = egui::vec2(NODE_WIDTH, NODE_HEIGHT) * self.graph.zoom();
            let node_rect = egui::Rect::from_min_size(screen_pos, size);
            if !rect.intersects(node_rect) {
                continue;
            }

            let is_selected = self.graph.selected == Some(*id);
            let is_connecting = self.graph.connecting_from == Some(*id);
            // Highlight if dragging THIS node
            let is_dragging = self.graph.dragging_node == Some(*id);

            let bg_color = if is_selected || is_dragging {
                node.color().linear_multiply(1.3)
            } else if is_connecting {
                egui::Color32::YELLOW.linear_multiply(0.3)
            } else {
                node.color()
            };

            painter.rect_filled(node_rect, 6.0 * self.graph.zoom(), bg_color);
            let border = if is_selected || is_dragging {
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
                if is_dragging {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_gray(240)
                }, // Explicit white for readability on dark nodes
            );

            painter.text(
                node_rect.min + egui::vec2(8.0, 28.0) * self.graph.zoom(),
                egui::Align2::LEFT_TOP,
                self.get_node_preview(node),
                egui::FontId::proportional(11.0 * self.graph.zoom()),
                egui::Color32::from_gray(200),
            );

            // Standard Clicks
            if response.clicked() && !is_dragging {
                // Only check clicks if we didn't just drag
                if let Some(p) = response.interact_pointer_pos() {
                    if node_rect.contains(p) {
                        clicked_node = Some(*id);
                    }
                }
            }
            // Right Click
            if response.secondary_clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    if node_rect.contains(p) {
                        right_clicked_node = Some((*id, p));
                    }
                }
            }
            // Double Click
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
                    let offset_x = (NODE_WIDTH / 2.0) * self.graph.zoom();
                    let offset_y = NODE_HEIGHT * self.graph.zoom();

                    let from = self.graph_to_screen(rect, *pos) + egui::vec2(offset_x, offset_y);
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
