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
        ui.heading("Asset Browser");
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
                    for (name, path) in &self.manifest.assets.audio {
                        let button = ui.add(egui::Button::new(format!("Audio {name}")));
                        if button.drag_started() {
                            let payload = format!(
                                "asset://audio/{}",
                                path.to_string_lossy().replace('\\', "/")
                            );
                            ui.memory_mut(|mem| {
                                mem.data
                                    .insert_temp(egui::Id::new("dragged_asset"), payload)
                            });
                        }
                        button.on_hover_text(format!("Drag to scene\nPath: {:?}", path));
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
                            .map(|(name, path)| (name, path.clone()))
                            .collect(),
                        "char" => self
                            .manifest
                            .assets
                            .characters
                            .iter()
                            .map(|(name, asset)| (name, asset.path.clone()))
                            .collect(),
                        _ => Vec::new(),
                    };

                    for (name, path) in assets {
                        let button = ui.add(
                            egui::Button::new(format!("Asset\n{name}"))
                                .min_size(egui::vec2(80.0, 80.0)),
                        );

                        let value = match type_id {
                            "bg" => path.to_string_lossy().replace('\\', "/"),
                            "char" => name.to_string(),
                            _ => name.to_string(),
                        };
                        if button.drag_started() {
                            let payload = format!("asset://{type_id}/{value}");
                            ui.memory_mut(|mem| {
                                mem.data
                                    .insert_temp(egui::Id::new("dragged_asset"), payload)
                            });
                        }

                        button.on_hover_text(format!("Drag to scene\nPath: {:?}", path));
                    }
                });
            });
    }
}
