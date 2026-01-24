//! Inspector panel for the editor workbench.
//!
//! Displays properties of selected nodes and entities.

use crate::editor::{NodeGraph, StoryNode};
use eframe::egui;
use visual_novel_engine::SceneState;

/// Inspector panel widget.
pub struct InspectorPanel<'a> {
    scene: &'a SceneState,
    graph: &'a mut NodeGraph, // Now mutable NodeGraph
    selected_node: Option<u32>,
    selected_entity: Option<u32>,
}

impl<'a> InspectorPanel<'a> {
    pub fn new(
        scene: &'a SceneState,
        graph: &'a mut NodeGraph,
        selected_node: Option<u32>,
        selected_entity: Option<u32>,
    ) -> Self {
        Self {
            scene,
            graph,
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
                self.render_node_editor(ui);
            });

            ui.separator();

            // Selected Entity section
            ui.collapsing("Selected Entity", |ui| {
                self.render_entity_info(ui);
            });

            ui.separator();

            // Stats
            ui.label(format!("Graph Nodes: {}", self.graph.len()));
        });
    }

    fn render_node_editor(&mut self, ui: &mut egui::Ui) {
        let mut delete_option_idx = None;
        let mut add_option_req = false;
        let mut standard_changed = false;

        if let Some(node_id) = self.selected_node {
            if let Some(node) = self.graph.get_node_mut(node_id) {
                ui.label(format!("Node ID: {}", node_id));
                ui.separator();

                match node {
                    StoryNode::Dialogue { speaker, text } => {
                        ui.label("Speaker:");
                        standard_changed |= ui.text_edit_singleline(speaker).changed();
                        ui.label("Text:");
                        standard_changed |= ui.text_edit_multiline(text).changed();
                    }
                    StoryNode::Choice { prompt, options } => {
                        ui.label("Prompt:");
                        standard_changed |= ui.text_edit_multiline(prompt).changed();

                        ui.separator();
                        ui.label("Options:");

                        // Option List
                        for (i, opt) in options.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                standard_changed |= ui.text_edit_singleline(opt).changed();
                                if ui.button("🗑").clicked() {
                                    delete_option_idx = Some(i);
                                }
                            });
                        }

                        if ui.button("➕ Add Option").clicked() {
                            add_option_req = true;
                        }
                    }
                    StoryNode::Scene { background } => {
                        ui.label("Background Image:");
                        standard_changed |= ui.text_edit_singleline(background).changed();
                    }
                    StoryNode::Jump { target } => {
                        ui.label("Jump Target (Label):");
                        standard_changed |= ui.text_edit_singleline(target).changed();
                    }
                    StoryNode::Start => {
                        ui.label("Start Node (Entry Point)");
                    }
                    StoryNode::End => {
                        ui.label("End Node (Termination)");
                    }
                }

                if standard_changed {
                    self.graph.mark_modified();
                }
            } else {
                ui.label("Node not found in editor graph.");
                return; // Avoid borrow conflicts if we continued
            }

            // Apply Structural Changes (After dropping node borrow)
            if let Some(idx) = delete_option_idx {
                self.graph.remove_choice_option(node_id, idx);
            }

            if add_option_req {
                if let Some(StoryNode::Choice { options, .. }) = self.graph.get_node_mut(node_id) {
                    options.push("New Option".to_string());
                    self.graph.mark_modified();
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
}
