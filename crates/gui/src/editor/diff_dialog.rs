//! Save Preview / Diff Dialog
//!
//! Visualizes changes before saving script.

use super::node_graph::NodeGraph;
use eframe::egui;
use visual_novel_engine::ScriptRaw;

pub struct DiffDialog {
    node_graph_snapshot: NodeGraph,
    saved_script_snapshot: Option<ScriptRaw>,
}

impl DiffDialog {
    pub fn new(graph: &NodeGraph, script: Option<&ScriptRaw>) -> Self {
        Self {
            node_graph_snapshot: graph.clone(),
            saved_script_snapshot: script.cloned(),
        }
    }

    /// Renders the diff dialog. Returns true if "Confirm" is clicked.
    pub fn show(&self, ctx: &egui::Context, open: &mut bool) -> bool {
        let mut confirmed = false;
        if *open {
            egui::Window::new("💾 Confirm Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label("You are about to save changes to the script.");
                    ui.separator();

                    let current_nodes = self.node_graph_snapshot.len();
                    // Basic stats
                    ui.heading("Statistics");
                    ui.label(format!("Current Nodes: {}", current_nodes));

                    if let Some(original) = &self.saved_script_snapshot {
                        let original_count = original.events.len(); // Rough approximation as events mapped to nodes
                        ui.label(format!("Last Saved Events: {}", original_count));

                        let diff = current_nodes as i32 - original_count as i32;
                        let diff_text = if diff > 0 {
                            format!("+{} (Approx.)", diff)
                        } else {
                            format!("{} (Approx.)", diff)
                        };

                        ui.label(
                            egui::RichText::new(format!("Change Delta: {}", diff_text)).strong(),
                        );
                    } else {
                        ui.label("New File (No previous save)");
                    }

                    ui.separator();
                    ui.label(
                        egui::RichText::new("⚠️ This will overwrite the file on disk.")
                            .color(egui::Color32::YELLOW),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            *open = false;
                        }
                        if ui.button("✅ Confirm Save").clicked() {
                            confirmed = true;
                            *open = false;
                        }
                    });
                });
        }
        confirmed
    }
}
