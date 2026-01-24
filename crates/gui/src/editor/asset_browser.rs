use eframe::egui;
use visual_novel_engine::manifest::ProjectManifest;

pub struct AssetBrowserPanel<'a> {
    pub manifest: &'a ProjectManifest,
}

impl<'a> AssetBrowserPanel<'a> {
    pub fn new(manifest: &'a ProjectManifest) -> Self {
        Self { manifest }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("📂 Asset Browser");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Backgrounds", |ui| {
                if self.manifest.assets.backgrounds.is_empty() {
                    ui.label("No backgrounds in manifest");
                } else {
                    self.render_grid(ui, "bg");
                }
            });

            ui.collapsing("Characters", |ui| {
                if self.manifest.assets.characters.is_empty() {
                    ui.label("No characters in manifest");
                } else {
                    self.render_grid(ui, "char");
                }
            });

            ui.collapsing("Audio", |ui| {
                if self.manifest.assets.audio.is_empty() {
                    ui.label("No audio in manifest");
                } else {
                    // List view for audio
                    for (name, _) in &self.manifest.assets.audio {
                        ui.label(format!("🎵 {}", name));
                    }
                }
            });
        });
    }

    fn render_grid(&self, ui: &mut egui::Ui, type_id: &str) {
        egui::ScrollArea::vertical()
            .id_source(type_id)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let assets: Vec<(&String, std::path::PathBuf)> = match type_id {
                        "bg" => self
                            .manifest
                            .assets
                            .backgrounds
                            .iter()
                            .map(|(k, v)| (k, v.clone()))
                            .collect(),
                        "char" => self
                            .manifest
                            .assets
                            .characters
                            .iter()
                            .map(|(k, v)| (k, v.path.clone()))
                            .collect(),
                        _ => vec![],
                    };

                    for (name, path) in assets {
                        let btn = ui.add(
                            egui::Button::new(format!("📄\n{}", name))
                                .min_size(egui::vec2(80.0, 80.0)),
                        );

                        // Simple Drag Payload: "type:name"
                        if btn.drag_started() {
                            let payload = format!("asset://{}/{}", type_id, name);
                            ui.memory_mut(|mem| {
                                mem.data
                                    .insert_temp(egui::Id::new("dragged_asset"), payload)
                            });
                        }

                        // Drag Source visual (tooltip) - consumes btn
                        btn.on_hover_text(format!("Drag to scene\nPath: {:?}", path));
                    }
                });
            });
    }
}
