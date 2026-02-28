use crate::editor::StoryNode; // Removed NodeGraph
use eframe::egui;
use std::collections::HashMap;
use visual_novel_engine::{Engine, EntityId, EntityKind, SceneState};

pub enum VisualComposerAction {
    SelectNode(u32),
    CreateNode { node: StoryNode, pos: egui::Pos2 },
}

/// The WYSIWYG Scene Composer.
pub struct VisualComposerPanel<'a> {
    scene: &'a mut SceneState,
    engine: &'a Option<Engine>,
    // graph: Removed to avoid borrow conflicts
    selected_entity_id: &'a mut Option<u32>,
}

impl<'a> VisualComposerPanel<'a> {
    pub fn new(
        scene: &'a mut SceneState,
        engine: &'a Option<Engine>,
        selected_entity_id: &'a mut Option<u32>,
    ) -> Self {
        Self {
            scene,
            engine,
            selected_entity_id,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        entity_owners: &HashMap<u32, u32>,
    ) -> Option<VisualComposerAction> {
        let mut action = None;
        ui.heading("🎨 Visual Composer");
        ui.separator();

        let available_size = ui.available_size();
        let viewport_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(available_size.x, available_size.y - 30.0),
        );

        // 1. Draw Background (Canvas)
        let response = ui.allocate_rect(viewport_rect, egui::Sense::click());

        // Handle Drop
        if response.hovered() && ui.input(|i| i.pointer.any_released()) {
            if let Some(payload) =
                ui.memory(|mem| mem.data.get_temp::<String>(egui::Id::new("dragged_asset")))
            {
                if payload.starts_with("asset://") {
                    // Parse asset://type/name
                    let parts: Vec<&str> = payload
                        .trim_start_matches("asset://")
                        .splitn(2, '/')
                        .collect();
                    if parts.len() == 2 {
                        let asset_type = parts[0];
                        let asset_name = parts[1];

                        // Heuristic Position: Center of viewport or cursor?
                        // If we use cursor, it might be relative to screen.
                        // visual_composer doesn't know graph scroll/pan.
                        // We will return screen pos, workbench converts it if possible,
                        // or just creates it at a default location.
                        // Let's assume (0,0) for now or use a fixed offset in workbench.
                        let pos = egui::pos2(100.0, 100.0);

                        let node = match asset_type {
                            "char" => Some(StoryNode::Dialogue {
                                speaker: asset_name.to_string(),
                                text: "...".to_string(),
                            }),
                            "bg" => Some(StoryNode::Scene {
                                background: asset_name.to_string(),
                            }),
                            _ => None,
                        };

                        if let Some(n) = node {
                            action = Some(VisualComposerAction::CreateNode { node: n, pos });
                        }

                        // Clear payload
                        ui.memory_mut(|mem| {
                            mem.data.remove::<String>(egui::Id::new("dragged_asset"))
                        });
                    }
                }
            }
        }

        // Handle canvas clicks (deselect)
        if response.clicked() {
            *self.selected_entity_id = None;
        }

        ui.painter()
            .rect_filled(viewport_rect, 0.0, egui::Color32::from_rgb(20, 20, 20));

        // 2. Render Entities (Back to Front)
        let mut moved_entity = None; // (id, delta)

        // Iterate keys/ids first using public API
        let ids: Vec<u32> = self.scene.iter().map(|e| e.id.raw()).collect();

        // Render loop
        for raw_id in ids {
            let id = EntityId::new(raw_id);
            if let Some(entity) = self.scene.get(id) {
                // Calculate Screen Rect
                let pos = viewport_rect.min
                    + egui::vec2(entity.transform.x as f32, entity.transform.y as f32);
                let size = match &entity.kind {
                    EntityKind::Character(_) => egui::vec2(80.0, 120.0),
                    EntityKind::Image(_) => egui::vec2(100.0, 100.0),
                    EntityKind::Video(_) => egui::vec2(160.0, 90.0),
                    _ => egui::vec2(100.0, 30.0),
                };

                let rect = egui::Rect::from_min_size(pos, size);

                // Interaction
                let id_salt = egui::Id::new(raw_id);
                let interact = ui.interact(rect, id_salt, egui::Sense::click_and_drag());

                // Selection
                if interact.clicked() {
                    *self.selected_entity_id = Some(raw_id);

                    // Trigger Node Selection if mapping exists
                    if let Some(node_id) = entity_owners.get(&raw_id) {
                        action = Some(VisualComposerAction::SelectNode(*node_id));
                    }
                }

                // Dragging
                if interact.dragged() {
                    *self.selected_entity_id = Some(raw_id);
                    moved_entity = Some((raw_id, interact.drag_delta()));
                }

                let is_selected = *self.selected_entity_id == Some(raw_id);

                // Rendering - Highlight
                if is_selected {
                    ui.painter().rect_stroke(
                        rect.expand(2.0),
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    );
                }

                // Entity Visual
                let color = if is_selected {
                    egui::Color32::from_rgb(120, 170, 220)
                } else {
                    egui::Color32::from_rgb(100, 150, 200)
                };

                ui.painter().rect_filled(rect, 4.0, color);

                // Text Label
                let label = match &entity.kind {
                    EntityKind::Character(c) => format!("👤 {}", c.name),
                    EntityKind::Image(i) => format!("🖼 {}", i.path.as_ref()),
                    _ => format!("Entity {}", raw_id),
                };

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            }
        }

        // Apply Movement (Mutation)
        if let Some((raw_id, delta)) = moved_entity {
            let id = EntityId::new(raw_id);
            if let Some(entity) = self.scene.get_mut(id) {
                entity.transform.x += delta.x as i32;
                entity.transform.y += delta.y as i32;
            }
        }

        // Info Bar
        ui.allocate_ui_at_rect(
            viewport_rect.translate(egui::vec2(0.0, viewport_rect.height() + 5.0)),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Entities: {}", self.scene.len()));
                    if let Some(sel) = self.selected_entity_id {
                        ui.label(format!("Selected: #{}", sel));
                        if let Some(ent) = self.scene.get(EntityId::new(*sel)) {
                            ui.label(format!("Pos: ({}, {})", ent.transform.x, ent.transform.y));
                        }
                    }

                    // Show Engine Event (Fixing unused warning)
                    if let Some(engine) = self.engine {
                        ui.separator();
                        if let Ok(event) = engine.current_event() {
                            ui.label(format!("Event: {:?}", event));
                        }
                    }
                });
            },
        );

        action
    }
}
