//! Graph panel for the editor workbench.
//!
//! Displays the story flow as a visual graph with nodes and edges.

use eframe::egui;
use visual_novel_engine::{NodeType, StoryGraph};

/// Graph panel widget.
pub struct GraphPanel<'a> {
    graph: &'a Option<StoryGraph>,
    selected_node: &'a mut Option<u32>,
}

impl<'a> GraphPanel<'a> {
    pub fn new(graph: &'a Option<StoryGraph>, selected_node: &'a mut Option<u32>) -> Self {
        Self {
            graph,
            selected_node,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Story Graph");
        ui.separator();

        if let Some(graph) = self.graph {
            let stats = graph.stats();

            // Statistics
            ui.horizontal(|ui| {
                ui.label(format!("Nodes: {}", stats.total_nodes));
                ui.separator();
                ui.label(format!("Edges: {}", stats.edge_count));
            });

            // Unreachable warning
            if stats.unreachable_nodes > 0 {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⚠️").color(egui::Color32::YELLOW));
                    ui.label(
                        egui::RichText::new(format!("{} unreachable", stats.unreachable_nodes))
                            .color(egui::Color32::YELLOW),
                    );
                });
            }

            ui.separator();

            // Node list
            egui::ScrollArea::vertical().show(ui, |ui| {
                for node in &graph.nodes {
                    let is_selected = *self.selected_node == Some(node.id);
                    let reachable_color = if node.reachable {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::RED
                    };

                    let icon = match &node.node_type {
                        NodeType::Dialogue { .. } => "💬",
                        NodeType::Choice { .. } => "🔀",
                        NodeType::Scene { .. } => "🎬",
                        NodeType::Jump => "↪",
                        NodeType::ConditionalJump { .. } => "❓",
                        NodeType::StateChange { .. } => "🔧",
                        NodeType::Patch => "📝",
                        NodeType::ExtCall { .. } => "📞",
                    };

                    let label_text = match &node.node_type {
                        NodeType::Dialogue {
                            speaker,
                            text_preview,
                        } => {
                            format!("{} {}: {}", icon, speaker, truncate(text_preview, 20))
                        }
                        NodeType::Choice {
                            prompt,
                            option_count,
                        } => {
                            format!("{} {} ({} opts)", icon, truncate(prompt, 15), option_count)
                        }
                        NodeType::Scene { background } => {
                            format!(
                                "{} Scene: {:?}",
                                icon,
                                background.as_ref().map(|s| truncate(s, 15))
                            )
                        }
                        NodeType::Jump => format!("{} Jump", icon),
                        NodeType::ConditionalJump { condition } => {
                            format!("{} If: {}", icon, truncate(condition, 15))
                        }
                        NodeType::StateChange { description } => {
                            format!("{} {}", icon, truncate(description, 20))
                        }
                        NodeType::Patch => format!("{} Patch", icon),
                        NodeType::ExtCall { command } => {
                            format!("{} Call: {}", icon, truncate(command, 15))
                        }
                    };

                    let labels_text = if node.labels.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", node.labels.join(", "))
                    };

                    let full_text = format!("{}: {}{}", node.id, label_text, labels_text);

                    let response = ui.selectable_label(
                        is_selected,
                        egui::RichText::new(full_text).color(reachable_color),
                    );

                    if response.clicked() {
                        *self.selected_node = Some(node.id);
                    }
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No script loaded");
            });
        }
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() > max_len {
        &s[..max_len.min(s.len())]
    } else {
        s
    }
}
