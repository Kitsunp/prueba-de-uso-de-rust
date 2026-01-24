//! Editor module for the Visual Novel Engine.
//!
//! This module provides a visual editor workbench with:
//! - Timeline panel for keyframe editing
//! - Graph panel for story flow visualization
//! - Viewport for scene preview
//! - Inspector for entity properties

mod asset_browser;
mod diff_dialog;
mod errors;
mod graph_panel;
mod inspector_panel;
mod lint_checks;
mod lint_panel;
mod node_editor;
mod node_graph;
mod node_rendering;
mod node_types;
mod player_ui;
mod script_sync;
mod timeline_panel;
mod undo;
mod viewport_panel;
mod visual_composer;

pub use asset_browser::AssetBrowserPanel;
pub use diff_dialog::DiffDialog;
pub use errors::EditorError;
pub use graph_panel::GraphPanel;
pub use inspector_panel::InspectorPanel;
pub use lint_checks::{validate as validate_graph, LintIssue, LintSeverity};
pub use lint_panel::LintPanel;
pub use node_editor::NodeEditorPanel;
pub use node_graph::NodeGraph;
pub use node_types::{ContextMenu, StoryNode, ToastKind, ToastState};
pub use timeline_panel::TimelinePanel;
pub use undo::UndoStack;
pub use viewport_panel::ViewportPanel;
pub use visual_composer::VisualComposerPanel;

use eframe::egui;
use std::path::PathBuf;
use tracing::{info, instrument};

use visual_novel_engine::{
    manifest::ProjectManifest, Engine, ResourceLimiter, SceneState, ScriptRaw, SecurityPolicy,
    StoryGraph, Timeline,
};

/// Runs the editor workbench as a standalone application.
pub fn run_editor() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Visual Novel Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "Visual Novel Editor",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(EditorApp::default())
        }),
    )
}

/// The editor application wrapper for eframe.
struct EditorApp {
    workbench: EditorWorkbench,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            workbench: EditorWorkbench::new(),
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update timeline if playing (approximately 60 fps)
        if self.workbench.is_playing {
            self.workbench.update(1);
            ctx.request_repaint();
        }

        self.workbench.ui(ctx);
    }
}

/// Editor mode for the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorMode {
    /// Normal playback mode.
    #[default]
    Player,
    /// Editor mode with panels.
    Editor,
}

/// The visual editor workbench.
pub struct EditorWorkbench {
    /// Current mode.
    pub mode: EditorMode,
    /// The game engine.
    pub engine: Option<Engine>,
    /// The story graph (generated from script).
    pub graph: Option<StoryGraph>,
    /// The current raw script (for editing).
    pub current_script: Option<ScriptRaw>,
    /// Path to the current script file.
    pub current_file_path: Option<PathBuf>,
    /// The animation timeline (for the editor).
    pub timeline: Timeline,
    /// The scene state (entity system).
    pub scene: SceneState,
    /// Currently selected node in the graph.
    pub selected_node: Option<u32>,
    /// Currently selected entity.
    pub selected_entity: Option<u32>,
    /// Timeline playback state.
    pub is_playing: bool,
    /// Current playback time.
    pub current_time: u32,
    /// Panel states.
    pub show_timeline: bool,
    pub show_graph: bool,
    pub show_inspector: bool,
    pub show_node_editor: bool,
    /// The node editor graph.
    pub node_graph: NodeGraph,
    /// Undo/redo stack for the node graph.
    pub undo_stack: UndoStack,
    /// Current toast notification.
    pub toast: Option<ToastState>,
    /// Error message (if any).
    pub error: Option<String>,
    /// The project manifest (Single Source of Truth).
    pub manifest: Option<ProjectManifest>,
    /// Show asset browser panel.
    pub show_asset_browser: bool,
    /// Is the Node Editor popped out in a separate window?
    pub node_editor_window_open: bool,

    // Validation
    /// Issues found during validation.
    pub validation_issues: Vec<LintIssue>,
    /// Show validation panel.
    pub show_validation: bool,

    // Save Confirmation
    pub show_save_confirm: bool,
    pub diff_dialog: Option<DiffDialog>,
    pub pending_save_path: Option<PathBuf>,
}

impl Default for EditorWorkbench {
    fn default() -> Self {
        Self {
            mode: EditorMode::Editor,
            engine: None,
            graph: None,
            current_script: None,
            current_file_path: None,
            timeline: Timeline::new(60),
            scene: SceneState::new(),
            selected_node: None,
            selected_entity: None,
            is_playing: false,
            current_time: 0,
            show_timeline: true,
            show_graph: true,
            show_inspector: true,
            show_node_editor: false,
            node_graph: NodeGraph::new(),
            undo_stack: UndoStack::new(),
            toast: None,
            error: None,
            manifest: Some(ProjectManifest::new("New Project", "Unknown")),
            show_asset_browser: true,
            node_editor_window_open: false,
            validation_issues: Vec::new(),
            show_validation: false,
            show_save_confirm: false,
            diff_dialog: None,
            pending_save_path: None,
        }
    }
}

impl EditorWorkbench {
    /// Creates a new editor workbench.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a script into the editor.
    /// Loads a script into the editor.
    ///
    /// # Contract
    /// - Parses the script (must be valid JSON)
    /// - Syncs NodeGraph (always)
    /// - Attempts to compile and create Engine
    /// - If compilation fails, loads graph but sets error state
    pub fn load_script(&mut self, json: &str) -> Result<(), String> {
        let script = ScriptRaw::from_json(json).map_err(|e| e.to_string())?;

        // 1. Always load the graph so the user can see the structure
        self.node_graph = NodeGraph::from_script(&script);
        self.node_graph.clear_modified();
        self.current_script = Some(script.clone());
        self.selected_node = Some(0);

        // 2. Try to compile for Engine/Playback
        match script.compile() {
            Ok(compiled) => {
                // Success: Full Load
                self.graph = Some(StoryGraph::from_script(&compiled));

                let engine = Engine::new(
                    script,
                    SecurityPolicy::default(),
                    ResourceLimiter::default(),
                )
                .map_err(|e| e.to_string())?;

                self.engine = Some(engine);
                self.error = None;
                self.toast = Some(ToastState::success("Script loaded successfully"));
            }
            Err(e) => {
                // Failure: Partial Load (Graph only)
                self.graph = None;
                self.engine = None;
                self.error = Some(format!("Loaded with errors: {}", e));
                self.toast = Some(ToastState::warning("Loaded with errors"));

                // Add to validation panel too
                self.validation_issues.push(LintIssue {
                    node_id: None,
                    severity: LintSeverity::Error,
                    message: e.to_string(),
                });
                self.show_validation = true;
            }
        }

        Ok(())
    }

    /// Loads a script from a file path.
    pub fn load_script_from_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.load_script(&content)?;
        self.current_file_path = Some(path.to_path_buf());
        Ok(())
    }

    /// Syncs the node graph back to the current script.
    pub fn sync_graph_to_script(&mut self) -> Result<(), String> {
        let script = self.node_graph.to_script();
        let compiled = script.compile().map_err(|e| e.to_string())?;

        self.graph = Some(StoryGraph::from_script(&compiled));
        let engine = Engine::new(
            script.clone(),
            SecurityPolicy::default(),
            ResourceLimiter::default(),
        )
        .map_err(|e| e.to_string())?;

        self.engine = Some(engine);
        self.current_script = Some(script);
        self.node_graph.clear_modified();

        Ok(())
    }

    /// Creates a new empty script.
    #[instrument(skip(self))]
    pub fn new_script(&mut self) {
        info!("Creating new script");
        self.node_graph = NodeGraph::new();
        self.current_script = None;
        self.current_file_path = None;
        self.engine = None;
        self.graph = None;
        self.undo_stack.clear();
        self.error = None;
    }

    /// Request to save the script ( triggers confirmation ).
    pub fn request_save(&mut self) {
        if let Some(path) = self.current_file_path.clone() {
            self.request_save_as(&path);
        }
    }

    /// Request to save as specific path ( triggers confirmation ).
    pub fn request_save_as(&mut self, path: &std::path::Path) {
        self.pending_save_path = Some(path.to_path_buf());
        self.diff_dialog = Some(DiffDialog::new(
            &self.node_graph,
            self.current_script.as_ref(),
        ));
        self.show_save_confirm = true;
    }

    /// Actual save execution (called after confirmation).
    pub fn execute_save(&mut self) -> Result<(), String> {
        let Some(path) = self.pending_save_path.clone() else {
            return Err("No save path specified".to_string());
        };

        info!("Saving script as {:?}", path);

        let script = self.node_graph.to_script();

        // Serialize to JSON with version envelope
        let json = script
            .to_json()
            .map_err(|e| EditorError::EngineError(e.to_string()).to_string())?;

        // Write to file
        std::fs::write(&path, &json).map_err(|e| EditorError::IoError(e).to_string())?;

        self.current_file_path = Some(path);
        self.current_script = Some(script);
        self.node_graph.clear_modified();

        info!("Script saved successfully ({} bytes)", json.len());
        Ok(())
    }

    /// Validates the current project structure.
    pub fn validate_project(&mut self) -> bool {
        self.validation_issues = validate_graph(&self.node_graph);

        if !self.validation_issues.is_empty() {
            self.show_validation = true;
            self.show_timeline = false;
            self.show_asset_browser = false;
            return false;
        }

        true
    }

    /// Exports the compiled script to a file.
    #[instrument(skip(self))]
    pub fn export_compiled(&self, path: &std::path::Path) -> Result<(), String> {
        info!("Exporting compiled script to {:?}", path);

        let script = self.node_graph.to_script();
        let compiled = script
            .compile()
            .map_err(|e| EditorError::CompileError(e.to_string()).to_string())?;

        let json =
            serde_json::to_string(&compiled).map_err(|e| EditorError::JsonError(e).to_string())?;

        std::fs::write(path, &json).map_err(|e| EditorError::IoError(e).to_string())?;

        info!("Compiled script exported ({} bytes)", json.len());
        Ok(())
    }

    /// Updates the editor state (called each frame).
    pub fn update(&mut self, delta_ticks: u32) {
        if self.is_playing {
            self.current_time = self.current_time.saturating_add(delta_ticks);
            self.timeline.seek(self.current_time);
        }
    }

    /// Toggles between Player and Editor modes.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditorMode::Player => EditorMode::Editor,
            EditorMode::Editor => EditorMode::Player,
        };
    }

    /// Renders the editor UI.
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📄 New Script").clicked() {
                        self.new_script();
                        self.toast = Some(ToastState::success("New script created"));
                        ui.close_menu();
                    }

                    if ui.button("📂 Open Script...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Script", &["json"])
                            .pick_file()
                        {
                            if let Err(e) = self.load_script_from_path(&path) {
                                self.error = Some(e);
                            }
                            // Toast is handled inside load_script/load_script_from_path
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    let can_save = self.current_file_path.is_some();
                    if ui
                        .add_enabled(can_save, egui::Button::new("💾 Save"))
                        .clicked()
                    {
                        self.request_save();
                        ui.close_menu();
                    }

                    if ui.button("💾 Save As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Script", &["json"])
                            .set_file_name("script.json")
                            .save_file()
                        {
                            self.request_save_as(&path);
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🛡️ Check Issues").clicked() {
                        if self.validate_project() {
                            self.toast = Some(ToastState::success("Project is valid!"));
                        } else {
                            self.toast = Some(ToastState::warning("Issues found"));
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("📦 Export Compiled...").clicked() {
                        self.validate_project();
                        let has_errors = self
                            .validation_issues
                            .iter()
                            .any(|i| i.severity == LintSeverity::Error);

                        if has_errors {
                            self.toast =
                                Some(ToastState::error("Fix critical errors before exporting"));
                        } else if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Compiled Script", &["vnc"])
                            .set_file_name("script.vnc")
                            .save_file()
                        {
                            if let Err(e) = self.export_compiled(&path) {
                                self.error = Some(e);
                            } else {
                                self.toast = Some(ToastState::success("Compiled script exported"));
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🚪 Exit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_timeline, "Timeline Panel");
                    ui.checkbox(&mut self.show_graph, "Graph Panel");
                    ui.checkbox(&mut self.show_inspector, "Inspector Panel");
                    ui.separator();
                    ui.checkbox(&mut self.show_node_editor, "📊 Node Editor");
                    ui.checkbox(&mut self.node_editor_window_open, "🗖 Separate Window");
                });

                ui.menu_button("Mode", |ui| {
                    if ui
                        .radio_value(&mut self.mode, EditorMode::Player, "Player")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.mode, EditorMode::Editor, "Editor")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mode_text = match self.mode {
                        EditorMode::Player => "🎮 Player",
                        EditorMode::Editor => "✏️ Editor",
                    };
                    ui.label(egui::RichText::new(mode_text).strong());
                });
            });
        });

        let mut clear_error = false;
        if let Some(ref error) = self.error {
            let error_clone = error.clone();
            egui::TopBottomPanel::top("error_banner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⚠️").color(egui::Color32::YELLOW));
                    ui.label(egui::RichText::new(&error_clone).color(egui::Color32::RED));
                    if ui.button("✕").clicked() {
                        clear_error = true;
                    }
                });
            });
        }
        if clear_error {
            self.error = None;
        }

        if self.show_save_confirm {
            if let Some(dialog) = &self.diff_dialog {
                if dialog.show(ctx, &mut self.show_save_confirm) {
                    if let Err(e) = self.execute_save() {
                        self.error = Some(e);
                    } else {
                        self.toast = Some(ToastState::success("Script saved safely"));
                    }
                }
            }
        }

        match self.mode {
            EditorMode::Player => self.render_player_mode(ctx),
            EditorMode::Editor => self.render_editor_mode(ctx),
        }
    }

    fn render_player_mode(&mut self, ctx: &egui::Context) {
        player_ui::render_player_ui(&mut self.engine, &mut self.toast, ctx);
    }

    fn render_editor_mode(&mut self, ctx: &egui::Context) {
        if self.show_graph {
            egui::SidePanel::left("graph_panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    GraphPanel::new(&mut self.node_graph).ui(ui);
                });
        }

        if self.show_inspector {
            egui::SidePanel::right("inspector_panel")
                .default_width(250.0)
                .resizable(true)
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
        }

        if self.show_timeline || self.show_asset_browser || self.show_validation {
            egui::TopBottomPanel::bottom("timeline_panel")
                .default_height(200.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.show_timeline
                                    && !self.show_asset_browser
                                    && !self.show_validation,
                                "Timeline",
                            )
                            .clicked()
                        {
                            self.show_timeline = true;
                            self.show_asset_browser = false;
                            self.show_validation = false;
                        }
                        if ui
                            .selectable_label(self.show_asset_browser, "Asset Browser")
                            .clicked()
                        {
                            self.show_asset_browser = true;
                            self.show_timeline = false;
                            self.show_validation = false;
                        }
                        if ui
                            .selectable_label(self.show_validation, "Validation")
                            .clicked()
                        {
                            self.show_validation = true;
                            self.show_timeline = false;
                            self.show_asset_browser = false;
                        }
                    });
                    ui.separator();

                    if self.show_validation {
                        LintPanel::new(&self.validation_issues, &mut self.selected_node).ui(ui);
                    } else if self.show_asset_browser {
                        if let Some(manifest) = &self.manifest {
                            AssetBrowserPanel::new(manifest).ui(ui);
                        } else {
                            ui.label("No Manifest Loaded");
                        }
                    } else {
                        TimelinePanel::new(
                            &mut self.timeline,
                            &mut self.current_time,
                            &mut self.is_playing,
                        )
                        .ui(ui);
                    }
                });
        }

        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                if let Some(previous) = self.undo_stack.undo(self.node_graph.clone()) {
                    self.node_graph = previous;
                    self.toast = Some(ToastState::success("Undo"));
                }
            }
            if (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                if let Some(next) = self.undo_stack.redo(self.node_graph.clone()) {
                    self.node_graph = next;
                    self.toast = Some(ToastState::success("Redo"));
                }
            }
        });

        if self.node_editor_window_open {
            let mut was_modified = false;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("node_editor_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Visual Novel Graph")
                    .with_inner_size([900.0, 600.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let graph_before = self.node_graph.clone();
                        NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack).ui(ui);
                        if self.node_graph.is_modified() {
                            self.undo_stack.push(graph_before);
                            self.node_graph.clear_modified();
                            was_modified = true;
                        }
                    });
                },
            );

            if was_modified {
                if let Err(e) = self.sync_graph_to_script() {
                    tracing::warn!("Live sync failed: {}", e);
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.node_editor_window_open || !self.show_node_editor {
                VisualComposerPanel::new(
                    &mut self.scene,
                    &self.engine,
                    &mut self.node_graph,
                    &mut self.selected_entity,
                )
                .ui(ui);

                if self.node_graph.is_modified() {
                    if let Err(e) = self.sync_graph_to_script() {
                        tracing::warn!("Drop sync failed: {}", e);
                    }
                    self.node_graph.clear_modified();
                }
            } else {
                let graph_before = self.node_graph.clone();
                NodeEditorPanel::new(&mut self.node_graph, &mut self.undo_stack).ui(ui);

                if self.node_graph.is_modified() {
                    self.undo_stack.push(graph_before);
                    if let Err(e) = self.sync_graph_to_script() {
                        tracing::warn!("Live sync failed: {}", e);
                    }
                    self.node_graph.clear_modified();
                }
            }
            node_rendering::render_toast(ui, &mut self.toast);
        });
    }
}
