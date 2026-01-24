//! Inspector panel for the editor workbench.
//!
//! Displays properties of selected nodes and entities.

use eframe::egui;
use visual_novel_engine::{NodeType, SceneState, StoryGraph};

/// Inspector panel widget.
pub struct InspectorPanel<'a> {
    scene: &'a SceneState,
    graph: &'a Option<StoryGraph>,
    current_script: &'a Option<visual_novel_engine::ScriptRaw>,
    selected_node: Option<u32>,
    selected_entity: Option<u32>,
}

impl<'a> InspectorPanel<'a> {
    pub fn new(
        scene: &'a SceneState,
        graph: &'a Option<StoryGraph>,
        current_script: &'a Option<visual_novel_engine::ScriptRaw>,
        selected_node: Option<u32>,
        selected_entity: Option<u32>,
    ) -> Self {
        Self {
            scene,
            graph,
            current_script,
            selected_node,
            selected_entity,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Inspector");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Selected Node section
            ui.collapsing("Selected Node", |ui| {
                self.render_node_info(ui);
            });

            ui.separator();

            // Selected Entity section
            ui.collapsing("Selected Entity", |ui| {
                self.render_entity_info(ui);
            });

            ui.separator();

            // Scene Overview
            ui.collapsing("Scene Overview", |ui| {
                self.render_scene_overview(ui);
            });

            ui.separator();

            // JSON Source View
            ui.collapsing("JSON Source", |ui| {
                self.render_json_source(ui);
            });
        });
    }

    fn render_node_info(&self, ui: &mut egui::Ui) {
        if let (Some(node_id), Some(graph)) = (self.selected_node, self.graph) {
            if let Some(node) = graph.get_node(node_id) {
                ui.label(format!("ID: {}", node.id));
                ui.label(format!(
                    "Reachable: {}",
                    if node.reachable { "✓" } else { "✗" }
                ));

                if !node.labels.is_empty() {
                    ui.label(format!("Labels: {}", node.labels.join(", ")));
                }

                ui.separator();

                match &node.node_type {
                    NodeType::Dialogue {
                        speaker,
                        text_preview,
                    } => {
                        ui.label("Type: Dialogue");
                        ui.label(format!("Speaker: {}", speaker));
                        ui.label(format!("Text: {}", text_preview));
                    }
                    NodeType::Choice {
                        prompt,
                        option_count,
                    } => {
                        ui.label("Type: Choice");
                        ui.label(format!("Prompt: {}", prompt));
                        ui.label(format!("Options: {}", option_count));
                    }
                    NodeType::Scene { background } => {
                        ui.label("Type: Scene");
                        ui.label(format!("Background: {:?}", background));
                    }
                    NodeType::Jump => {
                        ui.label("Type: Jump");
                    }
                    NodeType::ConditionalJump { condition } => {
                        ui.label("Type: Conditional Jump");
                        ui.label(format!("Condition: {}", condition));
                    }
                    NodeType::StateChange { description } => {
                        ui.label("Type: State Change");
                        ui.label(format!("Action: {}", description));
                    }
                    NodeType::Patch => {
                        ui.label("Type: Scene Patch");
                    }
                    NodeType::ExtCall { command } => {
                        ui.label("Type: External Call");
                        ui.label(format!("Command: {}", command));
                    }
                }

                // Outgoing edges
                ui.separator();
                ui.label("Outgoing Edges:");
                let edges = graph.outgoing_edges(node_id);
                for edge in edges {
                    let label = edge
                        .label
                        .as_ref()
                        .map(|l| format!(" \"{}\"", l))
                        .unwrap_or_default();
                    ui.label(format!("  → {} ({:?}){}", edge.to, edge.edge_type, label));
                }
            }
        } else {
            ui.label("No node selected");
        }
    }

    fn render_entity_info(&self, ui: &mut egui::Ui) {
        if let Some(entity_id) = self.selected_entity {
            if let Some(entity) = self
                .scene
                .get(visual_novel_engine::EntityId::new(entity_id))
            {
                ui.label(format!("ID: {}", entity.id));

                ui.separator();
                ui.label("Transform:");
                ui.label(format!(
                    "  Position: ({}, {})",
                    entity.transform.x, entity.transform.y
                ));
                ui.label(format!("  Z-Order: {}", entity.transform.z_order));
                ui.label(format!(
                    "  Scale: {}",
                    entity.transform.scale as f32 / 1000.0
                ));
                ui.label(format!(
                    "  Opacity: {}",
                    entity.transform.opacity as f32 / 1000.0
                ));

                ui.separator();
                ui.label(format!("Kind: {:?}", entity.kind));
            }
        } else {
            ui.label("No entity selected");
        }
    }

    fn render_scene_overview(&self, ui: &mut egui::Ui) {
        ui.label(format!("Entities: {}", self.scene.len()));

        if !self.scene.is_empty() {
            ui.separator();
            for entity in self.scene.iter() {
                ui.label(format!("  {} - {:?}", entity.id, entity.kind));
            }
        }
    }

    fn render_json_source(&self, ui: &mut egui::Ui) {
        if let Some(script) = self.current_script {
            if let Ok(json) = serde_json::to_string_pretty(script) {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut json.as_str())
                                .code_editor()
                                .lock_focus(true) // Prevent editing
                                .desired_width(f32::INFINITY),
                        );
                    });
            } else {
                ui.label("Error serializing script");
            }
        } else {
            ui.label("No script loaded");
        }
    }
}
