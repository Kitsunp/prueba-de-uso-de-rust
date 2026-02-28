use directories::ProjectDirs;
use eframe::egui;
use serde::{Deserialize, Serialize};
use visual_novel_engine::{Engine, ScriptRaw};

use crate::editor::{
    asset_browser::AssetBrowserPanel,
    diff_dialog::DiffDialog,
    inspector_panel::InspectorPanel,
    lint_panel::LintPanel,
    node_editor::NodeEditorPanel,
    node_graph::NodeGraph,
    node_types::ToastState,
    timeline_panel::TimelinePanel,
    undo::UndoStack,
    EditorMode,
    LintCode,
    LintIssue,
    LintSeverity, // Imported from mod.rs export
    ValidationPhase,
};
use crate::VnConfig;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct LayoutPreferences {
    show_graph: bool,
    show_inspector: bool,
    show_timeline: bool,
    show_asset_browser: bool,
    node_editor_window_open: bool,
}

/// Main editor workbench state and UI.
pub struct EditorWorkbench {
    pub config: VnConfig,
    pub node_graph: NodeGraph,
    pub undo_stack: UndoStack,
    pub manifest: Option<visual_novel_engine::manifest::ProjectManifest>,
    pub current_script: Option<ScriptRaw>,
    pub saved_script_snapshot: Option<ScriptRaw>,
    pub pending_save_path: Option<std::path::PathBuf>,

    // UI State
    pub mode: EditorMode,
    pub show_graph: bool,
    pub show_inspector: bool,
    pub show_timeline: bool,
    pub show_node_editor: bool,
    pub show_asset_browser: bool,
    pub show_validation: bool,
    pub show_save_confirm: bool,

    // Selection
    pub selected_node: Option<u32>,
    pub selected_entity: Option<u32>,

    // Scene Data
    pub scene: visual_novel_engine::SceneState,

    // Timeline/Playback
    pub timeline: visual_novel_engine::Timeline,
    pub current_time: f32,
    pub is_playing: bool,

    // Engine Instance (for Player Mode)
    pub engine: Option<Engine>,

    // Validation
    pub validation_issues: Vec<LintIssue>,
    pub last_dry_run_report: Option<crate::editor::compiler::DryRunReport>,

    // Feedback
    pub toast: Option<ToastState>,
    pub diff_dialog: Option<DiffDialog>,

    // New layout flags
    pub node_editor_window_open: bool,
    layout_prefs_path: std::path::PathBuf,
    last_layout_prefs: LayoutPreferences,
}

impl EditorWorkbench {
    fn append_phase_trace_issues(
        issues: &mut Vec<LintIssue>,
        traces: &[crate::editor::compiler::PhaseTrace],
    ) {
        for trace in traces {
            let phase = match trace.phase {
                crate::editor::compiler::CompilationPhase::GraphSync => ValidationPhase::Graph,
                crate::editor::compiler::CompilationPhase::GraphValidation => {
                    ValidationPhase::Graph
                }
                crate::editor::compiler::CompilationPhase::ScriptCompile => {
                    ValidationPhase::Compile
                }
                crate::editor::compiler::CompilationPhase::RuntimeInit => ValidationPhase::Runtime,
                crate::editor::compiler::CompilationPhase::DryRun => ValidationPhase::DryRun,
            };

            let entry = if trace.ok {
                LintIssue::info(
                    None,
                    phase,
                    LintCode::DryRunFinished,
                    format!("Phase {} OK: {}", trace.phase.label(), trace.detail),
                )
            } else {
                LintIssue::warning(
                    None,
                    phase,
                    LintCode::RuntimeInitError,
                    format!("Phase {} FAILED: {}", trace.phase.label(), trace.detail),
                )
            };
            issues.push(entry);
        }
    }

    pub fn new(config: VnConfig) -> Self {
        // Initialize with default/empty state
        let graph = NodeGraph::default();
        if graph.nodes.is_empty() {
            // Optional: graph.add_node(...)
        }

        let mut undo_stack = UndoStack::new();
        undo_stack.push(graph.clone());

        let layout_prefs_path = Self::layout_prefs_path();
        let loaded_prefs = Self::load_layout_prefs(&layout_prefs_path);

        let mut workbench = Self {
            config,
            node_graph: graph,
            undo_stack,
            manifest: None,
            current_script: None,
            saved_script_snapshot: None,
            pending_save_path: None,
            mode: EditorMode::Editor,
            show_graph: true,
            show_inspector: true,
            show_timeline: true,
            show_node_editor: false,
            show_asset_browser: true,
            show_validation: false,
            show_save_confirm: false,
            selected_node: None,
            selected_entity: None,
            scene: visual_novel_engine::SceneState::default(),
            timeline: visual_novel_engine::Timeline::new(60), // 60 ticks per second
            current_time: 0.0,
            is_playing: false,
            engine: None,
            validation_issues: Vec::new(),
            last_dry_run_report: None,
            toast: None,
            diff_dialog: None,
            node_editor_window_open: false,
            layout_prefs_path,
            last_layout_prefs: LayoutPreferences {
                show_graph: true,
                show_inspector: true,
                show_timeline: true,
                show_asset_browser: true,
                node_editor_window_open: false,
            },
        };

        if let Some(prefs) = loaded_prefs {
            workbench.apply_layout_prefs(&prefs);
        }
        workbench.last_layout_prefs = workbench.collect_layout_prefs();

        workbench
    }

    pub fn update(&mut self, _dt: usize) {
        if self.is_playing {
            // Simple tick approx 60fps or whatever dt implies
            self.current_time += 1.0;
            if self.current_time > self.timeline.duration() as f32 {
                self.current_time = 0.0;
                self.is_playing = false;
            }
        }
    }

    fn layout_prefs_path() -> std::path::PathBuf {
        if let Some(project_dirs) = ProjectDirs::from("com", "vnengine", "editor") {
            project_dirs.config_dir().join("layout.json")
        } else {
            std::path::PathBuf::from("editor_layout.json")
        }
    }

    fn load_layout_prefs(path: &std::path::Path) -> Option<LayoutPreferences> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    fn apply_layout_prefs(&mut self, prefs: &LayoutPreferences) {
        self.show_graph = prefs.show_graph;
        self.show_inspector = prefs.show_inspector;
        self.show_timeline = prefs.show_timeline;
        self.show_asset_browser = prefs.show_asset_browser;
        self.node_editor_window_open = prefs.node_editor_window_open;
    }

    fn collect_layout_prefs(&self) -> LayoutPreferences {
        LayoutPreferences {
            show_graph: self.show_graph,
            show_inspector: self.show_inspector,
            show_timeline: self.show_timeline,
            show_asset_browser: self.show_asset_browser,
            node_editor_window_open: self.node_editor_window_open,
        }
    }

    fn persist_layout_prefs_if_changed(&mut self) {
        let now = self.collect_layout_prefs();
        if now == self.last_layout_prefs {
            return;
        }
        self.last_layout_prefs = now.clone();

        if let Some(parent) = self.layout_prefs_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(payload) = serde_json::to_string_pretty(&now) {
            let _ = std::fs::write(&self.layout_prefs_path, payload);
        }
    }

    pub fn load_project(&mut self, path: std::path::PathBuf) {
        match crate::editor::project_io::load_project(path.clone()) {
            Ok(loaded_project) => {
                self.manifest = Some(loaded_project.manifest);
                if let Some((script_path, loaded_script)) = loaded_project.entry_point_script {
                    self.apply_loaded_script(loaded_script, script_path);
                } else {
                    self.toast = Some(crate::editor::node_types::ToastState::success(
                        "Project loaded (No entry script)",
                    ));
                }
            }
            Err(e) => {
                self.toast = Some(crate::editor::node_types::ToastState::error(format!(
                    "Failed to load project: {}",
                    e
                )));
                tracing::error!("Failed to load project: {}", e);
            }
        }
    }

    pub fn load_script(&mut self, path: std::path::PathBuf) {
        match crate::editor::project_io::load_script(path.clone()) {
            Ok(loaded_script) => {
                self.apply_loaded_script(loaded_script, path);
            }
            Err(e) => {
                self.toast = Some(crate::editor::node_types::ToastState::error(format!(
                    "Failed to load script: {}",
                    e
                )));
                tracing::error!("Failed to load script: {}", e);
            }
        }
    }

    fn apply_loaded_script(
        &mut self,
        loaded_script: crate::editor::project_io::LoadedScript,
        path: std::path::PathBuf,
    ) {
        self.node_graph = loaded_script.graph;
        let mut stack = UndoStack::new();
        stack.push(self.node_graph.clone());
        self.undo_stack = stack;
        self.pending_save_path = Some(path);
        self.saved_script_snapshot = Some(self.node_graph.to_script());

        let msg = if loaded_script.was_imported {
            "Imported script"
        } else {
            "Script loaded"
        };
        self.toast = Some(ToastState::success(msg));

        // CRITICAL: Sync to engine
        let _ = self.sync_graph_to_script();
    }

    pub fn execute_save(&mut self, path: &std::path::Path, _content_unused: &str) {
        if let Err(e) = crate::editor::project_io::save_script(path, &self.node_graph) {
            tracing::error!("Failed to save: {}", e);
            self.toast = Some(ToastState::error(format!("Save failed: {}", e)));
        } else {
            self.saved_script_snapshot = Some(self.node_graph.to_script());
            self.node_graph.clear_modified();
        }
    }

    pub fn prepare_save_confirmation(&mut self) {
        let maybe_path = self.pending_save_path.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Script JSON", &["json"])
                .set_file_name("script.json")
                .save_file()
        });

        if let Some(path) = maybe_path {
            self.pending_save_path = Some(path);
            let new_script = self.node_graph.to_script();
            self.show_save_confirm = true;
            self.diff_dialog = Some(DiffDialog::new(
                self.saved_script_snapshot.as_ref(),
                &new_script,
            ));
        } else {
            self.toast = Some(ToastState::warning("Save cancelled"));
        }
    }

    pub fn run_dry_validation(&mut self) -> bool {
        let result = crate::editor::compiler::compile_project(&self.node_graph);
        self.current_script = Some(result.script);
        self.last_dry_run_report = result.dry_run_report.clone();
        self.validation_issues = result.issues;
        Self::append_phase_trace_issues(&mut self.validation_issues, &result.phase_trace);
        self.show_validation = !self.validation_issues.is_empty();

        let has_errors = self
            .validation_issues
            .iter()
            .any(|issue| issue.severity == LintSeverity::Error);
        if has_errors {
            self.toast = Some(ToastState::error("Validation found blocking errors"));
            return false;
        }

        match result.engine_result {
            Ok(engine) => {
                self.engine = Some(engine);
                self.toast = Some(ToastState::success("Dry Run completed"));
                true
            }
            Err(e) => {
                self.validation_issues.push(LintIssue::error(
                    None,
                    ValidationPhase::Runtime,
                    LintCode::RuntimeInitError,
                    format!("Runtime initialization failed: {}", e),
                ));
                self.show_validation = true;
                self.toast = Some(ToastState::error("Validation failed at runtime init"));
                false
            }
        }
    }

    pub fn compile_preview(&mut self) -> bool {
        let ok = self.run_dry_validation();
        if ok {
            self.toast = Some(ToastState::success("Compilation preview successful"));
        }
        ok
    }

    pub fn export_compiled_project(&mut self) {
        if !self.run_dry_validation() {
            return;
        }

        let Some(script) = self.current_script.as_ref() else {
            self.toast = Some(ToastState::error("No script to export"));
            return;
        };

        let compiled = match script.compile() {
            Ok(compiled) => compiled,
            Err(e) => {
                self.toast = Some(ToastState::error(format!("Compile failed: {}", e)));
                return;
            }
        };

        let bytes = match compiled.to_binary() {
            Ok(bytes) => bytes,
            Err(e) => {
                self.toast = Some(ToastState::error(format!("Binary export failed: {}", e)));
                return;
            }
        };

        let path = rfd::FileDialog::new()
            .add_filter("VN Project", &["vnproject"])
            .set_file_name("game.vnproject")
            .save_file();

        if let Some(path) = path {
            match std::fs::write(&path, bytes) {
                Ok(_) => {
                    self.toast = Some(ToastState::success("Exported .vnproject successfully"));
                }
                Err(e) => {
                    self.toast = Some(ToastState::error(format!("Export failed: {}", e)));
                }
            }
        } else {
            self.toast = Some(ToastState::warning("Export cancelled"));
        }
    }

    pub fn sync_graph_to_script(&mut self) -> Result<(), String> {
        let result = crate::editor::compiler::compile_project(&self.node_graph);

        // Update State
        self.current_script = Some(result.script);
        self.last_dry_run_report = result.dry_run_report.clone();
        self.validation_issues = result.issues;
        Self::append_phase_trace_issues(&mut self.validation_issues, &result.phase_trace);
        self.show_validation = !self.validation_issues.is_empty();

        match result.engine_result {
            Ok(engine) => {
                self.engine = Some(engine);
                Ok(())
            }
            Err(e) => {
                self.validation_issues.push(LintIssue::error(
                    None,
                    ValidationPhase::Runtime,
                    LintCode::RuntimeInitError,
                    format!("Engine Error: {}", e),
                ));
                self.show_validation = true;
                Err(e)
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu_bar").show(ctx, |ui| {
            crate::editor::menu_bar::render_menu_bar(ui, self);
        });

        // Mode Switching
        egui::TopBottomPanel::top("mode_switcher").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (label, color) = match self.mode {
                    EditorMode::Editor => ("EDITOR", egui::Color32::from_rgb(70, 130, 220)),
                    EditorMode::Player => ("PLAYER", egui::Color32::from_rgb(230, 140, 50)),
                };
                ui.label(
                    egui::RichText::new(format!("Modo: {}", label))
                        .strong()
                        .color(color),
                );
                ui.separator();

                if ui
                    .selectable_label(self.mode == EditorMode::Editor, "Edit")
                    .clicked()
                {
                    self.mode = EditorMode::Editor;
                }
                if ui
                    .selectable_label(self.mode == EditorMode::Player, "Play")
                    .clicked()
                {
                    self.mode = EditorMode::Player;
                }

                ui.separator();
                if ui.button("Validar (Dry Run)").clicked() {
                    self.run_dry_validation();
                }
                if ui.button("Compilar").clicked() {
                    self.compile_preview();
                }
                if ui.button("Guardar").clicked() {
                    self.prepare_save_confirmation();
                }
                if ui.button("Exportar .vnproject").clicked() {
                    self.export_compiled_project();
                }
                if ui.button("Reset Layout").clicked() {
                    self.show_graph = true;
                    self.show_inspector = true;
                    self.show_timeline = true;
                    self.show_asset_browser = true;
                    self.node_editor_window_open = false;
                    self.toast = Some(ToastState::success("Layout restablecido"));
                }
            });
        });

        match self.mode {
            EditorMode::Player => self.render_player_mode(ctx),
            EditorMode::Editor => self.render_editor_mode(ctx),
        }

        // Render Diff Dialog (Modal-ish)
        let mut should_save = false;
        if self.show_save_confirm {
            if let Some(dialog) = &self.diff_dialog {
                // Return true if confirmed
                if dialog.show(ctx, &mut self.show_save_confirm) {
                    should_save = true;
                }
            }
        }

        if should_save {
            if self.run_dry_validation() {
                if let Some(path) = self.pending_save_path.clone() {
                    self.execute_save(&path, "");
                    self.toast = Some(ToastState::success("Saved successfully"));
                }
            } else {
                self.toast = Some(ToastState::error(
                    "Save blocked: fix validation errors first",
                ));
            }
            self.diff_dialog = None;
            self.show_save_confirm = false;
        }

        self.persist_layout_prefs_if_changed();
    }

    fn render_player_mode(&mut self, ctx: &egui::Context) {
        // Use the player_ui module which manages the central panel itself
        crate::editor::player_ui::render_player_ui(&mut self.engine, &mut self.toast, ctx);
    }

    fn render_editor_mode(&mut self, ctx: &egui::Context) {
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
                StoryNode::Scene { background } => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbench_initialization() {
        let config = VnConfig::default();
        let mut workbench = EditorWorkbench::new(config);

        // Assert default state
        assert_eq!(workbench.mode, EditorMode::Editor);
        assert!(workbench.node_graph.nodes.is_empty());
        assert!(!workbench.is_playing);

        // Add dummy track
        let mut track = visual_novel_engine::Track::new(
            visual_novel_engine::EntityId::new(1),
            visual_novel_engine::PropertyType::PositionX,
        );
        track
            .add_keyframe(visual_novel_engine::Keyframe::new(
                100,
                0,
                visual_novel_engine::Easing::Linear,
            ))
            .unwrap();
        workbench.timeline.add_track(track).unwrap();

        // Test simple update
        workbench.is_playing = true;
        workbench.update(1);
        assert!(
            workbench.current_time > 0.0,
            "Time should advance when playing"
        );
    }
}
