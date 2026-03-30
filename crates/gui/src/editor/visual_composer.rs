use crate::editor::StoryNode;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use visual_novel_engine::{Engine, EntityId, EntityKind, SceneState};

pub enum ComposerNodeMutation {
    CharacterPosition {
        name: String,
        x: i32,
        y: i32,
        scale: Option<f32>,
    },
}

pub enum VisualComposerAction {
    SelectNode(u32),
    CreateNode {
        node: StoryNode,
        pos: egui::Pos2,
    },
    MutateNode {
        node_id: u32,
        mutation: ComposerNodeMutation,
    },
}

/// The WYSIWYG Scene Composer.
pub struct VisualComposerPanel<'a> {
    scene: &'a mut SceneState,
    engine: &'a Option<Engine>,
    project_root: Option<&'a Path>,
    image_cache: &'a mut HashMap<String, egui::TextureHandle>,
    image_failures: &'a mut HashMap<String, String>,
    selected_entity_id: &'a mut Option<u32>,
}

impl<'a> VisualComposerPanel<'a> {
    pub fn new(
        scene: &'a mut SceneState,
        engine: &'a Option<Engine>,
        project_root: Option<&'a Path>,
        image_cache: &'a mut HashMap<String, egui::TextureHandle>,
        image_failures: &'a mut HashMap<String, String>,
        selected_entity_id: &'a mut Option<u32>,
    ) -> Self {
        Self {
            scene,
            engine,
            project_root,
            image_cache,
            image_failures,
            selected_entity_id,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        entity_owners: &HashMap<u32, u32>,
    ) -> Option<VisualComposerAction> {
        let mut action = None;
        ui.heading("Visual Composer");
        ui.separator();

        let available_size = ui.available_size();
        let viewport_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(available_size.x, available_size.y - 30.0),
        );

        let response = ui.allocate_rect(viewport_rect, egui::Sense::click());

        if response.hovered() && ui.input(|input| input.pointer.any_released()) {
            if let Some(payload) =
                ui.memory(|mem| mem.data.get_temp::<String>(egui::Id::new("dragged_asset")))
            {
                if payload.starts_with("asset://") {
                    let parts: Vec<&str> = payload
                        .trim_start_matches("asset://")
                        .splitn(2, '/')
                        .collect();
                    if parts.len() == 2 {
                        let asset_type = parts[0];
                        let asset_name = parts[1];
                        let drop_pos = response.hover_pos().unwrap_or(viewport_rect.center());
                        let local = drop_pos - viewport_rect.min;
                        let pos = egui::pos2(local.x.max(0.0), local.y.max(0.0));
                        let node = match asset_type {
                            "char" => Some(StoryNode::Dialogue {
                                speaker: asset_name.to_string(),
                                text: "...".to_string(),
                            }),
                            "bg" => Some(StoryNode::Scene {
                                profile: None,
                                background: Some(asset_name.to_string()),
                                music: None,
                                characters: Vec::new(),
                            }),
                            "audio" => Some(StoryNode::AudioAction {
                                channel: "bgm".to_string(),
                                action: "play".to_string(),
                                asset: Some(asset_name.to_string()),
                                volume: None,
                                fade_duration_ms: None,
                                loop_playback: Some(true),
                            }),
                            _ => None,
                        };

                        if let Some(node) = node {
                            action = Some(VisualComposerAction::CreateNode { node, pos });
                        }

                        ui.memory_mut(|mem| {
                            mem.data.remove::<String>(egui::Id::new("dragged_asset"))
                        });
                    }
                }
            }
        }

        if response.clicked() {
            *self.selected_entity_id = None;
        }

        ui.painter()
            .rect_filled(viewport_rect, 0.0, egui::Color32::from_rgb(20, 20, 20));

        let mut moved_entity = None;
        let ids: Vec<u32> = self.scene.iter().map(|entity| entity.id.raw()).collect();

        for raw_id in ids {
            let Some(entity) = self.scene.get(EntityId::new(raw_id)).cloned() else {
                continue;
            };

            let position = viewport_rect.min
                + egui::vec2(entity.transform.x as f32, entity.transform.y as f32);
            let size = match &entity.kind {
                EntityKind::Character(_) => egui::vec2(80.0, 120.0),
                EntityKind::Image(_) => egui::vec2(120.0, 120.0),
                EntityKind::Video(_) => egui::vec2(160.0, 90.0),
                EntityKind::Audio(_) => egui::vec2(220.0, 34.0),
                EntityKind::Text(_) => egui::vec2(180.0, 42.0),
            };
            let rect = egui::Rect::from_min_size(position, size);
            let interact = ui.interact(rect, egui::Id::new(raw_id), egui::Sense::click_and_drag());

            if interact.clicked() || interact.double_clicked() {
                *self.selected_entity_id = Some(raw_id);
                if let Some(node_id) = entity_owners.get(&raw_id) {
                    action = Some(VisualComposerAction::SelectNode(*node_id));
                }
            }
            if interact.dragged() {
                *self.selected_entity_id = Some(raw_id);
                moved_entity = Some((raw_id, ui.input(|input| input.pointer.delta())));
            }

            let is_selected = *self.selected_entity_id == Some(raw_id);
            if is_selected {
                ui.painter().rect_stroke(
                    rect.expand(2.0),
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                );
            }

            match &entity.kind {
                EntityKind::Image(image) => {
                    if let Some(texture_id) =
                        self.resolve_image_texture(ui.ctx(), image.path.as_ref())
                    {
                        ui.painter().image(
                            texture_id,
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        ui.painter().rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(1.0, egui::Color32::from_gray(220)),
                        );
                    } else {
                        self.paint_placeholder(
                            ui,
                            rect,
                            is_selected,
                            format!("Image {}", image.path.as_ref()),
                        );
                    }
                }
                EntityKind::Character(character) => {
                    if let Some(expression) = &character.expression {
                        if let Some(texture_id) =
                            self.resolve_image_texture(ui.ctx(), expression.as_ref())
                        {
                            ui.painter().image(
                                texture_id,
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, egui::Color32::from_gray(215)),
                            );
                            ui.painter().text(
                                rect.center_bottom() - egui::vec2(0.0, 8.0),
                                egui::Align2::CENTER_BOTTOM,
                                character.name.as_ref(),
                                egui::FontId::default(),
                                egui::Color32::WHITE,
                            );
                            continue;
                        }
                    }
                    self.paint_placeholder(
                        ui,
                        rect,
                        is_selected,
                        format!("Char {}", character.name.as_ref()),
                    );
                }
                EntityKind::Audio(audio) => {
                    let color = if is_selected {
                        egui::Color32::from_rgb(110, 165, 120)
                    } else {
                        egui::Color32::from_rgb(90, 140, 100)
                    };
                    ui.painter().rect_filled(rect, 4.0, color);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Audio {}", audio.path.as_ref()),
                        egui::FontId::default(),
                        egui::Color32::WHITE,
                    );
                }
                EntityKind::Video(video) => {
                    self.paint_placeholder(
                        ui,
                        rect,
                        is_selected,
                        format!("Video {}", video.path.as_ref()),
                    );
                }
                EntityKind::Text(text) => {
                    self.paint_placeholder(ui, rect, is_selected, text.content.clone());
                }
            }
        }

        if let Some((raw_id, delta)) = moved_entity {
            let id = EntityId::new(raw_id);
            if let Some(entity) = self.scene.get_mut(id) {
                entity.transform.x += delta.x as i32;
                entity.transform.y += delta.y as i32;

                if let Some(node_id) = entity_owners.get(&raw_id) {
                    if let EntityKind::Character(character) = &entity.kind {
                        let scale = if entity.transform.scale == 1000 {
                            None
                        } else {
                            Some(entity.transform.scale as f32 / 1000.0)
                        };
                        action = Some(VisualComposerAction::MutateNode {
                            node_id: *node_id,
                            mutation: ComposerNodeMutation::CharacterPosition {
                                name: character.name.to_string(),
                                x: entity.transform.x,
                                y: entity.transform.y,
                                scale,
                            },
                        });
                    }
                }
            }
        }

        ui.allocate_ui_at_rect(
            viewport_rect.translate(egui::vec2(0.0, viewport_rect.height() + 5.0)),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Entities: {}", self.scene.len()));
                    if let Some(sel) = *self.selected_entity_id {
                        ui.label(format!("Selected: #{}", sel));
                        if let Some(entity) = self.scene.get(EntityId::new(sel)) {
                            ui.label(format!(
                                "Pos: ({}, {})",
                                entity.transform.x, entity.transform.y
                            ));
                        }
                    }

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

    fn paint_placeholder(&self, ui: &egui::Ui, rect: egui::Rect, is_selected: bool, label: String) {
        let color = if is_selected {
            egui::Color32::from_rgb(120, 170, 220)
        } else {
            egui::Color32::from_rgb(100, 150, 200)
        };
        ui.painter().rect_filled(rect, 4.0, color);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }

    fn resolve_image_texture(
        &mut self,
        ctx: &egui::Context,
        asset_path: &str,
    ) -> Option<egui::TextureId> {
        if let Some(texture) = self.image_cache.get(asset_path) {
            return Some(texture.id());
        }
        if self.image_failures.contains_key(asset_path) {
            return None;
        }
        let Some(project_root) = self.project_root else {
            self.image_failures.insert(
                asset_path.to_string(),
                "project_root not available".to_string(),
            );
            return None;
        };

        let image = self.load_image(project_root, asset_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(image.size, &image.pixels);
        let texture = ctx.load_texture(
            format!("composer_image::{asset_path}"),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        self.image_cache.insert(asset_path.to_string(), texture);
        Some(id)
    }

    fn load_image(
        &mut self,
        project_root: &Path,
        asset_path: &str,
    ) -> Result<vnengine_assets::LoadedImage, ()> {
        let store = match vnengine_assets::AssetStore::new(
            project_root.to_path_buf(),
            vnengine_assets::SecurityMode::Trusted,
            None,
            false,
        ) {
            Ok(store) => store,
            Err(err) => {
                self.image_failures.insert(
                    asset_path.to_string(),
                    format!("asset store initialization failed: {err}"),
                );
                return Err(());
            }
        };

        let candidates = candidate_image_paths(asset_path);
        let mut failures = Vec::new();
        for candidate in &candidates {
            match store.load_image(candidate) {
                Ok(image) => return Ok(image),
                Err(err) => failures.push(format!("'{}': {}", candidate, err)),
            }
        }

        self.image_failures.insert(
            asset_path.to_string(),
            format!(
                "image load failed after {} candidate(s): {}",
                candidates.len(),
                failures.join(" | ")
            ),
        );
        Err(())
    }
}

fn candidate_image_paths(asset_path: &str) -> Vec<String> {
    const IMAGE_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];
    crate::editor::asset_candidates::candidate_asset_paths(asset_path, &IMAGE_EXTS)
}
