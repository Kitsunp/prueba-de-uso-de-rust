use super::*;

impl EditorWorkbench {
    pub(super) fn render_player_mode(&mut self, ctx: &egui::Context) {
        // Use the player_ui module which manages the central panel itself
        crate::editor::player_ui::render_player_ui(&mut self.engine, &mut self.toast, ctx);
    }

    pub(super) fn render_editor_mode(&mut self, ctx: &egui::Context) {
        // 1. Bottom Panels (Validation & Timeline)
        if self.show_validation {
            egui::TopBottomPanel::bottom("validation_panel")
                .resizable(true)
                .min_height(100.0)
                .show(ctx, |ui| {
                    LintPanel::new(&self.validation_issues, &mut self.selected_node).ui(ui);
                });
        }

        if self.show_timeline && !self.show_validation {
            egui::TopBottomPanel::bottom("timeline_panel")
                .default_height(200.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let mut current_time_u32 = self.current_time as u32;
                    let mut is_playing = self.is_playing;

                    TimelinePanel::new(&mut self.timeline, &mut current_time_u32, &mut is_playing)
                        .ui(ui);

                    self.current_time = current_time_u32 as f32;
                    self.is_playing = is_playing;
                });
        }

        // 2. Left Panel (Asset Browser)
        if self.show_asset_browser {
            egui::SidePanel::left("asset_browser_panel")
                .resizable(true)
                .default_width(200.0)
                .show(ctx, |ui| {
                    if let Some(manifest) = &self.manifest {
                        AssetBrowserPanel::new(manifest).ui(ui);
                    } else {
                        ui.label("No project loaded.");
                    }
                });
        }

        // 3. Central Area (Docking Logic)

        // Prepare Data for decoupled rendering to avoid simultaneous mutable borrows
        let entity_owners = self.build_entity_node_map();
        let mut composer_actions = Vec::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.node_editor_window_open || !self.show_graph {
                // Detached graph mode: Composer + optional Inspector
                if self.show_inspector {
                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            let mut composer =
                                crate::editor::visual_composer::VisualComposerPanel::new(
                                    &mut self.scene,
                                    &self.engine,
                                    &mut self.selected_entity,
                                );
                            if let Some(act) = composer.ui(ui, &entity_owners) {
                                composer_actions.push(act);
                            }
                        });
                        columns[1].vertical(|ui| {
                            ui.heading("Inspector");
                            let selected = self.node_graph.selected;
                            InspectorPanel::new(
                                &self.scene,
                                &mut self.node_graph,
                                selected,
                                self.selected_entity,
                            )
                            .ui(ui);
                        });
                    });
                } else {
                    let mut composer = crate::editor::visual_composer::VisualComposerPanel::new(
                        &mut self.scene,
                        &self.engine,
                        &mut self.selected_entity,
                    );
                    if let Some(act) = composer.ui(ui, &entity_owners) {
                        composer_actions.push(act);
                    }
                }
            } else if self.show_inspector {
                // Docked graph mode: Graph | Composer | Inspector
                ui.columns(3, |columns| {
                    columns[0].vertical(|ui| {
                        ui.heading("Logic Graph");
                        let mut panel =
                            NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack);
                        panel.ui(ui);
                    });

                    columns[1].vertical(|ui| {
                        let mut composer = crate::editor::visual_composer::VisualComposerPanel::new(
                            &mut self.scene,
                            &self.engine,
                            &mut self.selected_entity,
                        );
                        if let Some(act) = composer.ui(ui, &entity_owners) {
                            composer_actions.push(act);
                        }
                    });

                    columns[2].vertical(|ui| {
                        ui.heading("Inspector");
                        let selected = self.node_graph.selected;
                        InspectorPanel::new(
                            &self.scene,
                            &mut self.node_graph,
                            selected,
                            self.selected_entity,
                        )
                        .ui(ui);
                    });
                });
            } else {
                // Docked graph mode without inspector: Graph | Composer
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.heading("Logic Graph");
                        let mut panel =
                            NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack);
                        panel.ui(ui);
                    });

                    columns[1].vertical(|ui| {
                        let mut composer = crate::editor::visual_composer::VisualComposerPanel::new(
                            &mut self.scene,
                            &self.engine,
                            &mut self.selected_entity,
                        );
                        if let Some(act) = composer.ui(ui, &entity_owners) {
                            composer_actions.push(act);
                        }
                    });
                });
            }

            crate::editor::node_rendering::render_toast(ui, &mut self.toast);
        });

        // 5. Apply Deferred Actions
        for action in composer_actions {
            match action {
                crate::editor::visual_composer::VisualComposerAction::SelectNode(nid) => {
                    self.node_graph.selected = Some(nid);
                    self.selected_node = Some(nid);
                    self.selected_entity = None;
                }
                crate::editor::visual_composer::VisualComposerAction::CreateNode { node, pos } => {
                    // Since visual composer has no graph offset knowledge well,
                    // users might need to drag it later.
                    let new_id = self.node_graph.add_node(node, pos);
                    self.node_graph.selected = Some(new_id);
                    self.node_graph.mark_modified();
                }
            }
        }

        // Common Sync
        if let Some(node_id) = self.node_graph.selected {
            self.selected_node = Some(node_id);
            self.selected_entity = None;
        }

        if self.node_graph.modified {
            self.undo_stack.push(self.node_graph.clone());
            self.node_graph.clear_modified();
            let _ = self.sync_graph_to_script();
        }

        // 6. Floating/Detached Node Editor
        if self.node_editor_window_open && self.show_graph {
            let mut embedded_open = self.node_editor_window_open;
            let mut detached_closed = false;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("node_editor_detached"),
                egui::ViewportBuilder::default()
                    .with_title("Node Editor")
                    .with_inner_size([1000.0, 700.0]),
                |viewport_ctx, class| match class {
                    egui::ViewportClass::Embedded => {
                        egui::Window::new("Node Editor")
                            .open(&mut embedded_open)
                            .resizable(true)
                            .show(viewport_ctx, |ui| {
                                let mut panel = NodeEditorPanel::new(
                                    &mut self.node_graph,
                                    &mut self.undo_stack,
                                );
                                panel.ui(ui);
                            });
                    }
                    egui::ViewportClass::Immediate | egui::ViewportClass::Root => {
                        egui::CentralPanel::default().show(viewport_ctx, |ui| {
                            let mut panel =
                                NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack);
                            panel.ui(ui);
                        });
                        if viewport_ctx.input(|i| i.viewport().close_requested()) {
                            detached_closed = true;
                        }
                    }
                    egui::ViewportClass::Deferred => {}
                },
            );
            self.node_editor_window_open = embedded_open && !detached_closed;

            if self.node_graph.is_modified() {
                let _ = self.sync_graph_to_script();
            }
        }
    }

    fn build_entity_node_map(&self) -> std::collections::HashMap<u32, u32> {
        let mut map = std::collections::HashMap::new();
        use crate::editor::node_types::StoryNode;
        // Simple heuristic: Map entity to the node that DEFINES it (Character Dialog or Scene Background)
        // This requires traversing the graph.
        for (nid, node, _) in self.node_graph.nodes() {
            match node {
                StoryNode::Dialogue { speaker, .. } => {
                    // Find entity with this name in scene...
                    // But entity IDs are dynamic from engine state.
                    // We match by name.
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Character(c) = &entity.kind {
                            if c.name.as_ref() == speaker.as_str() {
                                map.insert(entity.id.raw(), *nid);
                            }
                        }
                    }
                }
                StoryNode::Scene {
                    background: Some(background),
                    ..
                } => {
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Image(img) = &entity.kind {
                            if img.path.as_ref() == background.as_str() {
                                map.insert(entity.id.raw(), *nid);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        map
    }
}
