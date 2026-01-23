//! Viewport panel for the editor workbench.
//!
//! Displays a preview of the scene with entities.

use eframe::egui;
use visual_novel_engine::{Engine, EntityKind, SceneState};

/// Viewport panel widget.
pub struct ViewportPanel<'a> {
    scene: &'a SceneState,
    engine: &'a Option<Engine>,
}

impl<'a> ViewportPanel<'a> {
    pub fn new(scene: &'a SceneState, engine: &'a Option<Engine>) -> Self {
        Self { scene, engine }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖼 Viewport");
        ui.separator();

        // Viewport area
        let available_size = ui.available_size();
        let viewport_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(available_size.x, available_size.y - 30.0),
        );

        // Draw viewport background
        ui.painter()
            .rect_filled(viewport_rect, 5.0, egui::Color32::from_rgb(30, 30, 40));

        // Draw viewport border
        ui.painter().rect_stroke(
            viewport_rect,
            5.0,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
        );

        // Render entities
        if !self.scene.is_empty() {
            for entity in self.scene.iter_sorted() {
                let entity_x = viewport_rect.min.x + entity.transform.x as f32;
                let entity_y = viewport_rect.min.y + entity.transform.y as f32;

                let opacity = (entity.transform.opacity as f32 / 1000.0 * 255.0) as u8;
                let color = egui::Color32::from_rgba_unmultiplied(200, 200, 255, opacity);

                match &entity.kind {
                    EntityKind::Image(data) => {
                        // Draw placeholder rectangle for image
                        let size = egui::vec2(100.0, 100.0);
                        let rect = egui::Rect::from_min_size(egui::pos2(entity_x, entity_y), size);
                        ui.painter().rect_filled(rect, 3.0, color);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("🖼 {}", truncate_path(&data.path)),
                            egui::FontId::default(),
                            egui::Color32::BLACK,
                        );
                    }
                    EntityKind::Text(data) => {
                        ui.painter().text(
                            egui::pos2(entity_x, entity_y),
                            egui::Align2::LEFT_TOP,
                            &data.content,
                            egui::FontId::proportional(data.font_size as f32),
                            color,
                        );
                    }
                    EntityKind::Character(data) => {
                        // Draw placeholder for character
                        let size = egui::vec2(80.0, 120.0);
                        let rect = egui::Rect::from_min_size(egui::pos2(entity_x, entity_y), size);
                        ui.painter()
                            .rect_filled(rect, 5.0, egui::Color32::from_rgb(100, 150, 200));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("👤 {}", data.name.as_ref()),
                            egui::FontId::default(),
                            egui::Color32::WHITE,
                        );
                        if let Some(expr) = &data.expression {
                            ui.painter().text(
                                egui::pos2(rect.center().x, rect.max.y - 15.0),
                                egui::Align2::CENTER_CENTER,
                                expr.as_ref(),
                                egui::FontId::proportional(12.0),
                                egui::Color32::LIGHT_GRAY,
                            );
                        }
                    }
                    EntityKind::Video(data) => {
                        let size = egui::vec2(160.0, 90.0);
                        let rect = egui::Rect::from_min_size(egui::pos2(entity_x, entity_y), size);
                        ui.painter()
                            .rect_filled(rect, 3.0, egui::Color32::from_rgb(50, 50, 70));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("🎬 {}", truncate_path(&data.path)),
                            egui::FontId::default(),
                            egui::Color32::WHITE,
                        );
                    }
                    EntityKind::Audio(data) => {
                        ui.painter().text(
                            egui::pos2(entity_x, entity_y),
                            egui::Align2::LEFT_TOP,
                            format!("🔊 {}", truncate_path(&data.path)),
                            egui::FontId::default(),
                            color,
                        );
                    }
                }
            }
        } else {
            let center = viewport_rect.center();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "No entities in scene\nAdd entities using the Entity menu",
                egui::FontId::proportional(16.0),
                egui::Color32::GRAY,
            );
        }

        // Advance past the viewport area
        ui.allocate_rect(viewport_rect, egui::Sense::hover());

        // Viewport info bar
        ui.horizontal(|ui| {
            ui.label(format!(
                "Size: {}x{}",
                available_size.x as i32,
                (available_size.y - 30.0) as i32
            ));
            ui.separator();
            ui.label(format!("Entities: {}", self.scene.len()));

            if let Some(engine) = self.engine {
                ui.separator();
                if let Ok(event) = engine.current_event() {
                    ui.label(
                        format!("Event: {:?}", event)
                            .chars()
                            .take(50)
                            .collect::<String>(),
                    );
                }
            }
        });
    }
}

fn truncate_path(path: &visual_novel_engine::SharedStr) -> String {
    let s = path.as_ref();
    if s.len() > 15 {
        format!("...{}", &s[s.len() - 12..])
    } else {
        s.to_string()
    }
}
