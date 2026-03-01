use super::*;

impl EditorWorkbench {
    pub(super) fn render_player_mode(&mut self, ctx: &egui::Context) {
        // Use the player_ui module which manages the central panel itself
        crate::editor::player_ui::render_player_ui(
            &mut self.engine,
            &mut self.toast,
            &mut self.player_state,
            &mut self.player_locale,
            &self.localization_catalog,
            ctx,
        );
    }

    pub(super) fn render_editor_mode(&mut self, ctx: &egui::Context) {
        // 1. Bottom Panels (Validation & Timeline)
        if self.show_validation {
            let mut close_validation = false;
            let mut toggle_validation_collapse = false;
            egui::TopBottomPanel::bottom("validation_panel")
                .resizable(!self.validation_collapsed)
                .default_height(if self.validation_collapsed { 44.0 } else { 240.0 })
                .min_height(if self.validation_collapsed { 36.0 } else { 56.0 })
                .show(ctx, |ui| {
                    let error_count = self
                        .validation_issues
                        .iter()
                        .filter(|issue| issue.severity == LintSeverity::Error)
                        .count();
                    let warning_count = self
                        .validation_issues
                        .iter()
                        .filter(|issue| issue.severity == LintSeverity::Warning)
                        .count();
                    let info_count = self
                        .validation_issues
                        .iter()
                        .filter(|issue| issue.severity == LintSeverity::Info)
                        .count();

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Validation Report  |  E:{} W:{} I:{}",
                                error_count, warning_count, info_count
                            ))
                            .strong(),
                        );
                        ui.separator();
                        let collapse_label = if self.validation_collapsed {
                            "Expandir"
                        } else {
                            "Minimizar"
                        };
                        if ui.button(collapse_label).clicked() {
                            toggle_validation_collapse = true;
                        }
                        if ui.button("Cerrar").clicked() {
                            close_validation = true;
                        }
                    });

                    if self.validation_collapsed {
                        return;
                    }

                    let lint_response = LintPanel::new(
                        &self.validation_issues,
                        &mut self.selected_node,
                        &mut self.selected_issue,
                        &mut self.diagnostic_language,
                        &self.node_graph,
                        self.last_fix_snapshot.is_some(),
                    )
                    .ui(ui);

                    for action in lint_response.actions {
                        match action {
                            crate::editor::lint_panel::LintPanelAction::ApplyFix {
                                issue_index,
                                fix_id,
                                structural,
                            } => {
                                if structural {
                                    match self
                                        .prepare_structural_fix_confirmation(issue_index, &fix_id)
                                    {
                                        Ok(()) => {
                                            self.toast = Some(ToastState::warning(format!(
                                                "Review diff and confirm structural fix '{fix_id}'"
                                            )));
                                        }
                                        Err(err) => {
                                            self.toast = Some(ToastState::error(format!(
                                                "Fix '{fix_id}' preview failed: {err}"
                                            )));
                                        }
                                    }
                                } else {
                                    match self.apply_issue_fix(issue_index, &fix_id) {
                                        Ok(()) => {
                                            self.toast = Some(ToastState::success(format!(
                                                "Applied fix '{fix_id}'"
                                            )));
                                        }
                                        Err(err) => {
                                            self.toast = Some(ToastState::error(format!(
                                                "Fix '{fix_id}' failed: {err}"
                                            )));
                                        }
                                    }
                                }
                            }
                            crate::editor::lint_panel::LintPanelAction::ApplyAllSafeFixes => {
                                let applied = self.apply_all_safe_fixes();
                                if applied > 0 {
                                    self.toast = Some(ToastState::success(format!(
                                        "Applied {applied} safe fix(es)"
                                    )));
                                } else {
                                    self.toast = Some(ToastState::warning(
                                        "No safe fixes available for current diagnostics",
                                    ));
                                }
                            }
                            crate::editor::lint_panel::LintPanelAction::PrepareAutoFixBatch {
                                include_review,
                            } => match self.prepare_autofix_batch_confirmation(include_review) {
                                Ok(planned) => {
                                    self.toast = Some(ToastState::warning(format!(
                                        "Review horizontal diff and confirm auto-fix batch ({planned} planned)"
                                    )));
                                }
                                Err(err) => {
                                    self.toast = Some(ToastState::warning(format!(
                                        "Auto-fix batch not prepared: {err}"
                                    )));
                                }
                            },
                            crate::editor::lint_panel::LintPanelAction::AutoFixIssue {
                                issue_index,
                                include_review,
                            } => match self.apply_best_fix_for_issue(issue_index, include_review) {
                                Ok(outcome) => {
                                    self.toast = Some(ToastState::success(outcome));
                                }
                                Err(err) => {
                                    self.toast = Some(ToastState::error(format!(
                                        "Issue auto-fix failed: {err}"
                                    )));
                                }
                            },
                            crate::editor::lint_panel::LintPanelAction::RevertLastFix => {
                                if self.revert_last_fix() {
                                    self.toast =
                                        Some(ToastState::success("Last fix reverted successfully"));
                                } else {
                                    self.toast = Some(ToastState::warning("No fix to revert"));
                                }
                            }
                        }
                    }
                });
            if toggle_validation_collapse {
                self.validation_collapsed = !self.validation_collapsed;
            }
            if close_validation {
                self.show_validation = false;
                self.validation_collapsed = false;
            }
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

        // 3. Docked Graph/Inspector Panels (context-level to avoid nested layout clipping)
        if !self.node_editor_window_open && self.show_graph {
            let total_width = ctx.available_rect().width();
            let graph_default = (total_width * 0.34).clamp(260.0, 680.0);
            let inspector_default = (total_width * 0.22).clamp(220.0, 420.0);

            if self.show_inspector {
                egui::SidePanel::right("inspector_docked_panel")
                    .resizable(true)
                    .min_width(220.0)
                    .max_width(520.0)
                    .default_width(inspector_default)
                    .show(ctx, |ui| {
                        let selected = self.node_graph.selected;
                        InspectorPanel::new(
                            &self.scene,
                            &mut self.node_graph,
                            selected,
                            self.selected_entity,
                        )
                        .ui(ui);
                    });

                egui::SidePanel::left("logic_graph_docked_panel")
                    .resizable(true)
                    .min_width(260.0)
                    .max_width(960.0)
                    .default_width(graph_default)
                    .show(ctx, |ui| {
                        ui.heading("Logic Graph");
                        let mut panel =
                            NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack);
                        panel.ui(ui);
                    });
            } else {
                egui::SidePanel::left("logic_graph_docked_panel_no_inspector")
                    .resizable(true)
                    .min_width(260.0)
                    .max_width(1080.0)
                    .default_width(graph_default)
                    .show(ctx, |ui| {
                        ui.heading("Logic Graph");
                        let mut panel =
                            NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack);
                        panel.ui(ui);
                    });
            }
        }

        // 4. Central Area (Composer + detached inspector logic)

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
            } else {
                // Docked graph mode: graph/inspector are drawn at ctx-level, central keeps composer.
                let mut composer = crate::editor::visual_composer::VisualComposerPanel::new(
                    &mut self.scene,
                    &self.engine,
                    &mut self.selected_entity,
                );
                if let Some(act) = composer.ui(ui, &entity_owners) {
                    composer_actions.push(act);
                }
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
