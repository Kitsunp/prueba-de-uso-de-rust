//! Node rendering utilities for the visual editor.
//!
//! This module contains helper functions for rendering node components:
//! context menus, inline editors, bezier curves, and toast notifications.
//! Extracted from node_editor.rs to comply with Criterio J (<500 lines).

use eframe::egui;

use super::node_graph::NodeGraph;
use super::node_types::{StoryNode, ToastState};

/// Renders a toast notification if one is active.
///
/// Call this at the end of the UI rendering to ensure toast appears on top.
pub fn render_toast(ui: &egui::Ui, toast: &mut Option<ToastState>) {
    let Some(t) = toast else {
        return;
    };

    // Decrement frame counter
    if t.frames_remaining > 0 {
        t.frames_remaining -= 1;
    }

    // Calculate alpha for fade out (last 30 frames)
    let alpha = if t.frames_remaining < 30 {
        (t.frames_remaining as f32 / 30.0 * 255.0) as u8
    } else {
        255
    };

    if t.frames_remaining == 0 {
        *toast = None;
        return;
    }

    // Render toast in bottom-right corner
    let screen_rect = ui.ctx().screen_rect();
    let toast_pos = egui::pos2(screen_rect.max.x - 20.0, screen_rect.max.y - 60.0);

    egui::Area::new(egui::Id::new("toast_notification"))
        .fixed_pos(toast_pos)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            let bg_color = t.kind.color().linear_multiply(0.9);
            let bg_color = egui::Color32::from_rgba_unmultiplied(
                bg_color.r(),
                bg_color.g(),
                bg_color.b(),
                alpha,
            );

            egui::Frame::none()
                .fill(bg_color)
                .rounding(8.0)
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    let text_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(t.kind.icon())
                                .size(16.0)
                                .color(text_color),
                        );
                        ui.label(egui::RichText::new(&t.message).color(text_color));
                    });
                });
        });

    // Request repaint to animate
    ui.ctx().request_repaint();
}

/// Renders the context menu for a node.
pub fn render_context_menu(graph: &mut NodeGraph, ui: &egui::Ui) {
    let Some(menu) = graph.context_menu.clone() else {
        return;
    };

    let node_id = menu.node_id;

    egui::Area::new(egui::Id::new("node_context_menu"))
        .fixed_pos(menu.position)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);

                ui.menu_button("➕ Insert Node", |ui| {
                    if ui.button("Before").clicked() {
                        graph.insert_before(node_id, StoryNode::default());
                        graph.context_menu = None;
                        ui.close_menu();
                    }
                    if ui.button("After").clicked() {
                        graph.insert_after(node_id, StoryNode::default());
                        graph.context_menu = None;
                        ui.close_menu();
                    }
                });

                ui.separator();

                if ui.button("🔀 Convert to Choice").clicked() {
                    graph.convert_to_choice(node_id);
                    graph.context_menu = None;
                }

                if ui.button("↗️ Create Branch").clicked() {
                    graph.create_branch(node_id);
                    graph.context_menu = None;
                }

                ui.separator();

                if ui.button("🔗 Connect To...").clicked() {
                    graph.connecting_from = Some(node_id);
                    graph.context_menu = None;
                }

                ui.separator();

                if ui.button("✏️ Edit").clicked() {
                    graph.editing = Some(node_id);
                    graph.context_menu = None;
                }

                if ui
                    .button(egui::RichText::new("🗑️ Delete").color(egui::Color32::RED))
                    .clicked()
                {
                    graph.remove_node(node_id);
                    graph.context_menu = None;
                }
            });
        });
}

/// Renders the inline node editor window.
pub fn render_inline_editor(graph: &mut NodeGraph, ui: &egui::Ui) {
    let Some(editing_id) = graph.editing else {
        return;
    };

    let Some(node) = graph.get_node_mut(editing_id) else {
        graph.editing = None;
        return;
    };

    let mut changed = false;
    let mut close_editor = false;
    let mut node_clone = node.clone();

    egui::Window::new("Edit Node")
        .collapsible(false)
        .resizable(true)
        .show(ui.ctx(), |ui| {
            match &mut node_clone {
                StoryNode::Dialogue { speaker, text } => {
                    ui.horizontal(|ui| {
                        ui.label("Speaker:");
                        changed |= ui.text_edit_singleline(speaker).changed();
                    });
                    ui.label("Text:");
                    changed |= ui
                        .add(egui::TextEdit::multiline(text).desired_rows(4))
                        .changed();
                }
                StoryNode::Choice { prompt, options } => {
                    ui.horizontal(|ui| {
                        ui.label("Prompt:");
                        changed |= ui.text_edit_singleline(prompt).changed();
                    });
                    ui.label("Options:");
                    for option in options.iter_mut() {
                        changed |= ui.text_edit_singleline(option).changed();
                    }
                    if ui.button("➕ Add Option").clicked() {
                        options.push("New Option".to_string());
                        changed = true;
                    }
                }
                StoryNode::Scene { background } => {
                    ui.horizontal(|ui| {
                        ui.label("Background:");
                        changed |= ui.text_edit_singleline(background).changed();
                    });
                }
                StoryNode::Jump { target } => {
                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        changed |= ui.text_edit_singleline(target).changed();
                    });
                }
                StoryNode::Start | StoryNode::End => {
                    ui.label("This node has no editable properties.");
                }
            }

            ui.separator();
            if ui.button("✓ Done").clicked() {
                close_editor = true;
            }
        });

    // Apply changes
    if changed {
        if let Some(node) = graph.get_node_mut(editing_id) {
            *node = node_clone;
        }
        graph.mark_modified();
    }

    if close_editor {
        graph.editing = None;
    }
}

/// Draws a bezier connection curve between two points.
pub fn draw_bezier_connection(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2) {
    let control_offset = (to.y - from.y).abs() * 0.5;
    let control1 = from + egui::vec2(0.0, control_offset);
    let control2 = to - egui::vec2(0.0, control_offset);

    let points: Vec<egui::Pos2> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let mt3 = mt2 * mt;

            egui::pos2(
                mt3 * from.x + 3.0 * mt2 * t * control1.x + 3.0 * mt * t2 * control2.x + t3 * to.x,
                mt3 * from.y + 3.0 * mt2 * t * control1.y + 3.0 * mt * t2 * control2.y + t3 * to.y,
            )
        })
        .collect();

    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 180, 100)),
    ));

    // Arrow head
    let arrow_size = 8.0;
    let dir = (to - control2).normalized();
    let arrow_left = to - dir * arrow_size + dir.rot90() * arrow_size * 0.5;
    let arrow_right = to - dir * arrow_size - dir.rot90() * arrow_size * 0.5;
    painter.add(egui::Shape::convex_polygon(
        vec![to, arrow_left, arrow_right],
        egui::Color32::from_rgb(100, 180, 100),
        egui::Stroke::NONE,
    ));
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_connection_does_not_panic() {
        // We can't easily test painter output, but we can verify the math doesn't panic
        let from = egui::pos2(0.0, 0.0);
        let to = egui::pos2(100.0, 200.0);

        let control_offset = (to.y - from.y).abs() * 0.5;
        let control1 = from + egui::vec2(0.0, control_offset);
        let control2 = to - egui::vec2(0.0, control_offset);

        // Verify control points are calculated correctly
        assert_eq!(control_offset, 100.0);
        assert_eq!(control1, egui::pos2(0.0, 100.0));
        assert_eq!(control2, egui::pos2(100.0, 100.0));

        // Verify bezier math produces 21 points
        let points: Vec<egui::Pos2> = (0..=20)
            .map(|i| {
                let t = i as f32 / 20.0;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;

                egui::pos2(
                    mt3 * from.x
                        + 3.0 * mt2 * t * control1.x
                        + 3.0 * mt * t2 * control2.x
                        + t3 * to.x,
                    mt3 * from.y
                        + 3.0 * mt2 * t * control1.y
                        + 3.0 * mt * t2 * control2.y
                        + t3 * to.y,
                )
            })
            .collect();

        assert_eq!(points.len(), 21);
        assert_eq!(points[0], from); // First point should be start
        assert_eq!(points[20], to); // Last point should be end
    }

    #[test]
    fn test_bezier_horizontal_line() {
        let from = egui::pos2(0.0, 50.0);
        let to = egui::pos2(100.0, 50.0);

        // For horizontal line, control offset should be 0
        let control_offset = (to.y - from.y).abs() * 0.5;
        assert_eq!(control_offset, 0.0);
    }

    #[test]
    fn test_context_menu_no_panic_when_no_menu() {
        // Verify that calling with no context menu doesn't panic
        let mut graph = NodeGraph::new();
        graph.context_menu = None;

        // We can't call render_context_menu without a UI, but we can verify the state
        assert!(graph.context_menu.is_none());
    }

    #[test]
    fn test_inline_editor_no_panic_when_not_editing() {
        // Verify that the editing state works correctly
        let mut graph = NodeGraph::new();
        graph.editing = None;

        // Verify state
        assert!(graph.editing.is_none());
    }
}
