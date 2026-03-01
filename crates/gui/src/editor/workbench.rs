use directories::ProjectDirs;
use eframe::egui;
use serde::{Deserialize, Serialize};
use visual_novel_engine::{Engine, LocalizationCatalog, ScriptRaw};

use crate::editor::{
    asset_browser::AssetBrowserPanel,
    diagnostics::DiagnosticLanguage,
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

#[derive(Clone, Debug)]
pub struct QuickFixAuditEntry {
    pub diagnostic_id: String,
    pub fix_id: String,
    pub node_id: Option<u32>,
    pub event_ip: Option<u32>,
    pub before_crc32: u32,
    pub after_crc32: u32,
}

#[derive(Clone, Debug)]
pub struct PendingStructuralFix {
    pub issue_index: usize,
    pub fix_id: String,
}

#[derive(Clone, Debug)]
pub struct PendingAutoFixOperation {
    pub issue: LintIssue,
    pub fix_id: String,
}

#[derive(Clone, Debug)]
pub struct PendingAutoFixBatch {
    pub include_review: bool,
    pub operations: Vec<PendingAutoFixOperation>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoFixBatchResult {
    pub applied: usize,
    pub skipped: usize,
}

/// Main editor workbench state and UI.
pub struct EditorWorkbench {
    pub config: VnConfig,
    pub node_graph: NodeGraph,
    pub undo_stack: UndoStack,
    pub manifest: Option<visual_novel_engine::manifest::ProjectManifest>,
    pub project_root: Option<std::path::PathBuf>,
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
    pub validation_collapsed: bool,
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
    pub player_state: crate::editor::player_ui::PlayerSessionState,

    // Engine Instance (for Player Mode)
    pub engine: Option<Engine>,

    // Validation
    pub validation_issues: Vec<LintIssue>,
    pub last_dry_run_report: Option<crate::editor::compiler::DryRunReport>,
    pub diagnostic_language: DiagnosticLanguage,
    pub player_locale: String,
    pub localization_catalog: LocalizationCatalog,
    pub selected_issue: Option<usize>,
    pub last_fix_snapshot: Option<NodeGraph>,
    pub quick_fix_audit: Vec<QuickFixAuditEntry>,
    pub show_fix_confirm: bool,
    pub fix_diff_dialog: Option<DiffDialog>,
    pub pending_structural_fix: Option<PendingStructuralFix>,
    pub pending_auto_fix_batch: Option<PendingAutoFixBatch>,

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
            project_root: None,
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
            validation_collapsed: false,
            show_save_confirm: false,
            selected_node: None,
            selected_entity: None,
            scene: visual_novel_engine::SceneState::default(),
            timeline: visual_novel_engine::Timeline::new(60), // 60 ticks per second
            current_time: 0.0,
            is_playing: false,
            player_state: crate::editor::player_ui::PlayerSessionState::default(),
            engine: None,
            validation_issues: Vec::new(),
            last_dry_run_report: None,
            diagnostic_language: DiagnosticLanguage::Es,
            player_locale: "en".to_string(),
            localization_catalog: LocalizationCatalog::default(),
            selected_issue: None,
            last_fix_snapshot: None,
            quick_fix_audit: Vec::new(),
            show_fix_confirm: false,
            fix_diff_dialog: None,
            pending_structural_fix: None,
            pending_auto_fix_batch: None,
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
                let migrated_manifest = loaded_project
                    .manifest_migration_report
                    .as_ref()
                    .map(|report| report.entries.len());
                let project_root = path
                    .parent()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or(path.clone());
                self.project_root = Some(project_root.clone());
                self.localization_catalog =
                    Self::load_localization_catalog(&project_root, &loaded_project.manifest);
                self.player_locale = loaded_project.manifest.settings.default_language.clone();
                self.manifest = Some(loaded_project.manifest);
                if let Some((script_path, loaded_script)) = loaded_project.entry_point_script {
                    self.apply_loaded_script(loaded_script, script_path);
                    if let Some(steps) = migrated_manifest {
                        self.toast = Some(crate::editor::node_types::ToastState::warning(format!(
                            "Project loaded with manifest migration ({steps} step(s))"
                        )));
                    }
                } else {
                    self.toast = Some(if let Some(steps) = migrated_manifest {
                        crate::editor::node_types::ToastState::warning(format!(
                            "Project loaded without entry script (manifest migrated in {steps} step(s))"
                        ))
                    } else {
                        crate::editor::node_types::ToastState::success(
                            "Project loaded (No entry script)",
                        )
                    });
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
                if self.project_root.is_none() {
                    self.project_root = path.parent().map(std::path::Path::to_path_buf);
                }
                if let Some(root) = &self.project_root {
                    self.localization_catalog = Self::discover_locales_without_manifest(root);
                    if self.player_locale.trim().is_empty() {
                        self.player_locale = self.localization_catalog.default_locale.clone();
                    }
                }
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

    fn load_localization_catalog(
        project_root: &std::path::Path,
        manifest: &visual_novel_engine::manifest::ProjectManifest,
    ) -> LocalizationCatalog {
        let mut catalog = LocalizationCatalog::new(manifest.settings.default_language.clone());
        for locale in &manifest.settings.supported_languages {
            let path = project_root.join("locales").join(format!("{locale}.json"));
            if !path.exists() {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_str::<std::collections::BTreeMap<String, String>>(&raw)
            else {
                continue;
            };
            catalog.insert_locale_table(locale.clone(), parsed);
        }
        catalog
    }

    fn discover_locales_without_manifest(project_root: &std::path::Path) -> LocalizationCatalog {
        let mut catalog = LocalizationCatalog::default();
        let locale_dir = project_root.join("locales");
        if !locale_dir.exists() {
            return catalog;
        }

        let Ok(entries) = std::fs::read_dir(locale_dir) else {
            return catalog;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_str::<std::collections::BTreeMap<String, String>>(&raw)
            else {
                continue;
            };
            catalog.insert_locale_table(stem.to_string(), parsed);
        }

        if let Some(first) = catalog.locale_codes().first() {
            catalog.default_locale = first.clone();
        }
        catalog
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
                if ui.button("Exportar Repro Dry Run").clicked() {
                    self.export_dry_run_repro();
                }
                if ui.button("Exportar Reporte Diagnostico").clicked() {
                    self.export_diagnostic_report();
                }
                if ui.button("Importar Reporte Diagnostico").clicked() {
                    self.import_diagnostic_report();
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

        let mut should_apply_structural_fix = false;
        if self.show_fix_confirm {
            if let Some(dialog) = &self.fix_diff_dialog {
                if dialog.show(ctx, &mut self.show_fix_confirm) {
                    should_apply_structural_fix = true;
                }
            }
        }

        if should_apply_structural_fix {
            if self.pending_auto_fix_batch.is_some() {
                match self.apply_pending_autofix_batch() {
                    Ok(result) => {
                        self.toast = Some(ToastState::success(format!(
                            "Auto-fix batch applied: {} applied, {} skipped",
                            result.applied, result.skipped
                        )));
                    }
                    Err(err) => {
                        self.toast =
                            Some(ToastState::error(format!("Auto-fix batch failed: {err}")));
                    }
                }
            } else {
                match self.apply_pending_structural_fix() {
                    Ok(fix_id) => {
                        self.toast = Some(ToastState::success(format!(
                            "Applied structural fix '{fix_id}'"
                        )));
                    }
                    Err(err) => {
                        self.toast =
                            Some(ToastState::error(format!("Structural fix failed: {err}")));
                    }
                }
            }
            self.fix_diff_dialog = None;
            self.show_fix_confirm = false;
        } else if !self.show_fix_confirm {
            self.pending_structural_fix = None;
            self.pending_auto_fix_batch = None;
            self.fix_diff_dialog = None;
        }

        self.persist_layout_prefs_if_changed();
    }
}

mod compile_ops;
mod quick_fix_ops;
mod report_ops;
#[cfg(test)]
#[path = "tests/workbench_tests.rs"]
mod tests;
mod ui;
