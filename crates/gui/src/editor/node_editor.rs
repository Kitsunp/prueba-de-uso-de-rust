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
        let mut is_panning = response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && ui.input(|i| i.modifiers.ctrl));

        if response.dragged_by(egui::PointerButton::Primary)
            && !is_panning
            && self.graph.dragging_node.is_none()
        {
            // Check if we started dragging on a node
            if let Some(_pos) = response.interact_pointer_pos() {
                // We need the START position of the drag, not current.
                if let Some(start_pos) = ui.input(|i| i.pointer.press_origin()) {
                    let mut started_on_node = false;
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
        if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            self.graph.zoom_by(0.1);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            self.graph.zoom_by(-0.1);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Num0)) {
            self.graph.reset_view();
        }
        if ui.input(|i| i.key_pressed(egui::Key::H)) {
            self.graph.zoom_to_fit();
        }

        // === Node Action Shortcuts ===
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if let Some(id) = self.graph.selected {
                self.graph.remove_node(id);
                self.graph.selected = None;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            if let Some(id) = self.graph.selected {
                self.graph.editing = Some(id);
            }
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            if let Some(id) = self.graph.selected {
                self.graph.duplicate_node(id);
            }
        }
    }

    fn render_connections(&self, painter: &egui::Painter, rect: egui::Rect) {
        for conn in self.graph.connections() {
            let from_pos = self
                .graph
                .nodes()
                .find(|(id, _, _)| *id == conn.from)
                .map(|(_, node, p)| (*p, node));
            let to_pos = self
                .graph
                .nodes()
                .find(|(id, _, _)| *id == conn.to)
                .map(|(_, _, p)| *p);

            if let (Some((from_base, from_node)), Some(to_base)) = (from_pos, to_pos) {
                // Determine source port position
                let from_screen = self.graph_to_screen(
                    rect,
                    self.calculate_port_pos(from_base, from_node, conn.from_port),
                );

                // Target is always Top-Center (Input)
                let offset_x = (NODE_WIDTH / 2.0) * self.graph.zoom();
                let to_screen = self.graph_to_screen(rect, to_base) + egui::vec2(offset_x, 0.0);

                node_rendering::draw_bezier_connection(painter, from_screen, to_screen);
            }
        }
    }

    /// Calculates local graph position of an output port
    fn calculate_port_pos(
        &self,
        node_pos: egui::Pos2,
        node: &StoryNode,
        port: usize,
    ) -> egui::Pos2 {
        match node {
            StoryNode::Choice { .. } => {
                let header_height = 40.0;
                let option_height = 30.0;
                let option_offset =
                    header_height + (port as f32 * option_height) + (option_height / 2.0);

                node_pos + egui::vec2(NODE_WIDTH / 2.0, option_offset + 15.0)
            }
            _ => {
                // Standard single output (Bottom Center)
                node_pos + egui::vec2(NODE_WIDTH / 2.0, NODE_HEIGHT)
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

        // 1. Handle Drag Start (Nodes)
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                // Check ports first (priority over node move)
                for (id, node, n_pos) in nodes.iter().rev() {
                    if let StoryNode::Choice { options, .. } = node {
                        // Check option ports
                        for (i, _) in options.iter().enumerate() {
                            let port_pos = self.calculate_port_pos(*n_pos, node, i);
                            let screen_pos = self.graph_to_screen(rect, port_pos);
                            if screen_pos.distance(pos) < 10.0 * self.graph.zoom() {
                                self.graph.connecting_from = Some((*id, i));
                                return; // Consumed by port drag
                            }
                        }
                    } else if node.can_connect_from() {
                        // Standard port
                        let port_pos = self.calculate_port_pos(*n_pos, node, 0);
                        let screen_pos = self.graph_to_screen(rect, port_pos);
                        if screen_pos.distance(pos) < 10.0 * self.graph.zoom() {
                            self.graph.connecting_from = Some((*id, 0));
                            return;
                        }
                    }
                }

                // Then Node Drag
                for (id, _, n_pos) in nodes.iter().rev() {
                    let screen_pos = self.graph_to_screen(rect, *n_pos);
                    let height = self.get_node_height(self.graph.get_node(*id).unwrap());
                    let size = egui::vec2(NODE_WIDTH, height) * self.graph.zoom();
                    let node_rect = egui::Rect::from_min_size(screen_pos, size);
                    if node_rect.contains(pos) {
                        self.graph.dragging_node = Some(*id);
                        break;
                    }
                }
            }
        }

        // 2. Handle Dragging
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
            if let Some((from, _)) = self.graph.connecting_from {
                if let Some(pos) = response.interact_pointer_pos() {
                    for (to_id, _, to_pos) in nodes.iter().rev() {
                        let screen_pos = self.graph_to_screen(rect, *to_pos);
                        let height = self.get_node_height(self.graph.get_node(*to_id).unwrap());
                        let size = egui::vec2(NODE_WIDTH, height) * self.graph.zoom();
                        let node_rect = egui::Rect::from_min_size(screen_pos, size);

                        if node_rect.contains(pos) {
                            if from != *to_id {
                                // FINALIZE CONNECTION
                                let port = self.graph.connecting_from.unwrap().1;
                                self.graph.connect_port(from, port, *to_id);
                            }
                            break;
                        }
                    }
                }
                self.graph.connecting_from = None;
            }
        }

        // Rendering Loop
        for (id, node, pos) in &nodes {
            let screen_pos = self.graph_to_screen(rect, *pos);
            let height = self.get_node_height(node);
            let size = egui::vec2(NODE_WIDTH, height) * self.graph.zoom();
            let node_rect = egui::Rect::from_min_size(screen_pos, size);

            if !rect.intersects(node_rect) {
                continue;
            }

            let is_selected = self.graph.selected == Some(*id);
            let is_connecting = self.graph.connecting_from.map(|(nid, _)| nid) == Some(*id);
            let is_dragging = self.graph.dragging_node == Some(*id);

            // Shape
            let bg_color = if is_selected || is_dragging {
                node.color().linear_multiply(1.3)
            } else if is_connecting {
                egui::Color32::YELLOW.linear_multiply(0.3)
            } else {
                node.color()
            };

            painter.rect_filled(node_rect, 6.0 * self.graph.zoom(), bg_color);
            let border_color = if is_selected {
                egui::Color32::from_rgb(100, 150, 255)
            } else {
                egui::Color32::from_rgb(80, 80, 90)
            };
            painter.rect_stroke(
                node_rect,
                2.0 * self.graph.zoom(),
                egui::Stroke::new(2.0, border_color),
            );

            // Content
            let font_size = 13.0 * self.graph.zoom();
            let text_pos = node_rect.min + egui::vec2(8.0, 8.0) * self.graph.zoom();
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                format!("{} {}", node.icon(), node.type_name()),
                egui::FontId::proportional(font_size),
                egui::Color32::WHITE,
            );

            // Body / Options
            match node {
                StoryNode::Choice { options, .. } => {
                    let header_height = 40.0 * self.graph.zoom();
                    let option_h = 30.0 * self.graph.zoom();

                    for (i, opt) in options.iter().enumerate() {
                        let y_off = header_height + (i as f32 * option_h);
                        let opt_rect = egui::Rect::from_min_size(
                            node_rect.min + egui::vec2(0.0, y_off),
                            egui::vec2(node_rect.width(), option_h),
                        );

                        // Double-click on option to edit
                        if ui.input(|inp| {
                            inp.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        }) {
                            if let Some(p) = response.interact_pointer_pos() {
                                if opt_rect.contains(p) {
                                    double_clicked_node = Some(*id);
                                }
                            }
                        }

                        painter.line_segment(
                            [opt_rect.left_top(), opt_rect.right_top()],
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );

                        painter.text(
                            opt_rect.left_center() + egui::vec2(5.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            crate::editor::graph_panel::truncate(opt, 15),
                            egui::FontId::proportional(11.0 * self.graph.zoom()),
                            egui::Color32::LIGHT_GRAY,
                        );

                        // Socket visual & Interaction
                        let socket_center =
                            self.graph_to_screen(rect, self.calculate_port_pos(*pos, node, i));
                        let hover_radius = 8.0 * self.graph.zoom();
                        let is_hovered = response
                            .hover_pos()
                            .is_some_and(|p| p.distance(socket_center) < hover_radius);

                        let mut color = egui::Color32::WHITE;
                        let mut radius = 4.0 * self.graph.zoom();

                        if is_hovered {
                            color = egui::Color32::YELLOW;
                            radius = 6.0 * self.graph.zoom();
                            // Tooltip
                            painter.text(
                                socket_center + egui::vec2(10.0, -10.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("Connect '{}'", opt),
                                egui::FontId::proportional(12.0),
                                egui::Color32::YELLOW,
                            );
                        }

                        painter.circle_filled(socket_center, radius, color);
                    }
                }
                _ => {
                    painter.text(
                        node_rect.min + egui::vec2(8.0, 28.0) * self.graph.zoom(),
                        egui::Align2::LEFT_TOP,
                        self.get_node_preview(node),
                        egui::FontId::proportional(11.0 * self.graph.zoom()),
                        egui::Color32::from_gray(200),
                    );

                    if node.can_connect_from() {
                        let socket_center =
                            self.graph_to_screen(rect, self.calculate_port_pos(*pos, node, 0));
                        let hover_radius = 8.0 * self.graph.zoom();
                        let is_hovered = response
                            .hover_pos()
                            .is_some_and(|p| p.distance(socket_center) < hover_radius);

                        let mut color = egui::Color32::WHITE;
                        let mut radius = 4.0 * self.graph.zoom();

                        if is_hovered {
                            color = egui::Color32::YELLOW;
                            radius = 6.0 * self.graph.zoom();
                            painter.text(
                                socket_center + egui::vec2(10.0, -10.0),
                                egui::Align2::LEFT_BOTTOM,
                                "Standard Output",
                                egui::FontId::proportional(12.0),
                                egui::Color32::YELLOW,
                            );
                        }

                        painter.circle_filled(socket_center, radius, color);
                    }
                }
            }

            if response.clicked() && !is_dragging && self.graph.connecting_from.is_none() {
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
            self.graph.selected = Some(id);
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

    fn get_node_height(&self, node: &StoryNode) -> f32 {
        match node {
            StoryNode::Choice { options, .. } => {
                let header = 40.0;
                let option_h = 30.0;
                header + (options.len().max(1) as f32 * option_h) + 10.0
            }
            _ => NODE_HEIGHT,
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
            StoryNode::SetVariable { key, value } => format!("{} = {}", key, value),
            StoryNode::ScenePatch(patch) => {
                let mut summary = String::new();
                if patch.music.is_some() {
                    summary.push_str("♫ ");
                }
                if !patch.add.is_empty() {
                    summary.push_str("C+ ");
                }
                if !patch.update.is_empty() {
                    summary.push_str("C~ ");
                }
                if !patch.remove.is_empty() {
                    summary.push_str("C- ");
                }
                if summary.is_empty() {
                    "Empty Patch".to_string()
                } else {
                    summary
                }
            }
            StoryNode::JumpIf { .. } => "Conditional".to_string(),
            StoryNode::Start => "Entry Point".to_string(),
            StoryNode::End => "Exit Point".to_string(),
            StoryNode::Generic(event) => {
                let json = event.to_json_value();
                let type_name = json
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                format!("Generic: {}", type_name)
            }
            StoryNode::AudioAction {
                channel, action, ..
            } => {
                format!("{} {}", action, channel)
            }
            StoryNode::Transition { kind, .. } => {
                format!("Transition: {}", kind)
            }
            StoryNode::CharacterPlacement { name, x, y, .. } => {
                format!("{}: ({}, {})", name, x, y)
            }
        }
    }

    fn render_connecting_line(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        if let Some((from_id, from_port)) = self.graph.connecting_from {
            if let Some((_, node, pos)) = self.graph.nodes().find(|(id, _, _)| *id == from_id) {
                if let Some(cursor) = response.hover_pos() {
                    let from =
                        self.graph_to_screen(rect, self.calculate_port_pos(*pos, node, from_port));
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
            "🔗 Drag to target node to connect • Esc to cancel"
        } else {
            "Drag from socket to connect • Double-click to edit"
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
        let mut undo = UndoStack::new();
        let _panel = NodeEditorPanel::new(&mut graph, &mut undo);
    }
}
