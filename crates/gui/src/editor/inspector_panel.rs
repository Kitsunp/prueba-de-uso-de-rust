//! Inspector panel for the editor workbench.
//!
//! Displays properties of selected nodes and entities.

use crate::editor::NodeGraph;
use eframe::egui;
use visual_novel_engine::SceneState;

/// Inspector panel widget.
pub struct InspectorPanel<'a> {
    scene: &'a SceneState,
    graph: &'a mut NodeGraph,
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
        ui.heading("Inspector");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Selected Node", |ui| {
                self.render_node_editor(ui);
            });

            ui.separator();

            ui.collapsing("Selected Entity", |ui| {
                self.render_entity_info(ui);
            });

            ui.separator();
            ui.label(format!("Graph Nodes: {}", self.graph.len()));
        });
    }
}

mod entity_info;
#[path = "inspector_panel_node_editor.rs"]
mod node_editor;
