//! Editor module for the Visual Novel Engine.
//!
//! This module provides a visual editor workbench with:
//! - Timeline panel for keyframe editing
//! - Graph panel for story flow visualization
//! - Viewport for scene preview
//! - Inspector for entity properties

mod graph_panel;
mod inspector_panel;
mod lint_checks;
mod node_editor;
mod node_graph;
mod node_rendering;
mod node_types;
mod player_ui;
mod script_sync;
mod timeline_panel;
mod undo;
mod viewport_panel;

pub use graph_panel::GraphPanel;
pub use inspector_panel::InspectorPanel;
pub use lint_checks::{validate as validate_graph, LintIssue, LintSeverity};
pub use node_editor::NodeEditorPanel;
pub use node_graph::NodeGraph;
pub use node_types::{ContextMenu, StoryNode, ToastKind, ToastState};
pub use timeline_panel::TimelinePanel;
pub use undo::UndoStack;
pub use viewport_panel::ViewportPanel;

use eframe::egui;
use std::path::PathBuf;
use tracing::{info, instrument, warn};

use visual_novel_engine::{
    Engine, ResourceLimiter, SceneState, ScriptRaw, SecurityPolicy, StoryGraph, Timeline,
};

mod errors;
pub use errors::EditorError;

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
        Box::new(|_cc| Box::new(EditorApp::default())),
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
        }
    }
}

impl EditorWorkbench {
    /// Creates a new editor workbench.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a script into the editor.
    ///
    /// # Contract
    /// - Parses and compiles the script
    /// - Syncs the NodeGraph from the raw script
    /// - Creates the engine for playback
    pub fn load_script(&mut self, json: &str) -> Result<(), String> {
        let script = ScriptRaw::from_json(json).map_err(|e| e.to_string())?;
        let compiled = script.compile().map_err(|e| e.to_string())?;

        // Generate the story graph (for visualization)
        self.graph = Some(StoryGraph::from_script(&compiled));

        // Sync the node editor graph from raw script
        self.node_graph = NodeGraph::from_script(&script);
        self.node_graph.clear_modified();

        // Store the current script
        self.current_script = Some(script.clone());

        // Create the engine for playback
        let engine = Engine::new(
            script,
            SecurityPolicy::default(),
            ResourceLimiter::default(),
        )
        .map_err(|e| e.to_string())?;

        self.engine = Some(engine);
        self.selected_node = Some(0);
        self.error = None;

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
    ///
    /// # Contract
    /// - Converts NodeGraph to ScriptRaw
    /// - Recompiles and updates the engine
    /// - Clears the modified flag
    pub fn sync_graph_to_script(&mut self) -> Result<(), String> {
        let script = self.node_graph.to_script();
        let compiled = script.compile().map_err(|e| e.to_string())?;

        // Update story graph
        self.graph = Some(StoryGraph::from_script(&compiled));

        // Update engine
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

        debug_assert!(
            self.node_graph.is_empty(),
            "Postcondition: new script should have empty graph"
        );
    }

    /// Saves the script to the current file path.
    ///
    /// # Errors
    /// Returns EditorError if no file path is set or write fails.
    #[instrument(skip(self))]
    pub fn save_script(&mut self) -> Result<(), String> {
        let path = self
            .current_file_path
            .clone()
            .ok_or_else(|| EditorError::NoFilePath.to_string())?;

        info!("Saving script to {:?}", path);
        self.save_script_as(&path)
    }

    /// Saves the script to a specific path.
    ///
    /// # Contract
    /// - Converts graph to script
    /// - Writes JSON to file
    /// - Updates current_file_path
    #[instrument(skip(self))]
    pub fn save_script_as(&mut self, path: &std::path::Path) -> Result<(), String> {
        debug_assert!(
            path.extension().is_some(),
            "Precondition: path should have an extension"
        );

        info!("Saving script as {:?}", path);

        // Sync graph to script first
        let script = self.node_graph.to_script();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&script)
            .map_err(|e| EditorError::JsonError(e).to_string())?;

        // Write to file
        std::fs::write(path, &json).map_err(|e| EditorError::IoError(e).to_string())?;

        // Update state
        self.current_file_path = Some(path.to_path_buf());
        self.current_script = Some(script);
        self.node_graph.clear_modified();

        info!("Script saved successfully ({} bytes)", json.len());

        debug_assert!(
            !self.node_graph.is_modified(),
            "Postcondition: modified flag should be cleared"
        );

        Ok(())
    }

    /// Exports the compiled script to a file.
    ///
    /// # Contract
    /// - Compiles the script
    /// - Writes compiled JSON to file
    #[instrument(skip(self))]
    pub fn export_compiled(&self, path: &std::path::Path) -> Result<(), String> {
        debug_assert!(
            path.extension().is_some(),
            "Precondition: path should have an extension"
        );

        info!("Exporting compiled script to {:?}", path);

        // Compile the script
        let script = self.node_graph.to_script();
        let compiled = script
            .compile()
            .map_err(|e| EditorError::CompileError(e.to_string()).to_string())?;

        // Serialize compiled script to JSON
        let json =
            serde_json::to_string(&compiled).map_err(|e| EditorError::JsonError(e).to_string())?;

        // Write to file
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
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    // New Script
                    if ui.button("📄 New Script").clicked() {
                        self.new_script();
                        self.toast = Some(ToastState::success("New script created"));
                        ui.close_menu();
                    }

                    // Open Script
                    if ui.button("📂 Open Script...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Script", &["json"])
                            .pick_file()
                        {
                            if let Err(e) = self.load_script_from_path(&path) {
                                self.error = Some(e);
                            } else {
                                self.toast = Some(ToastState::success("Script loaded"));
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    // Save
                    let can_save = self.current_file_path.is_some();
                    if ui
                        .add_enabled(can_save, egui::Button::new("💾 Save"))
                        .clicked()
                    {
                        if let Err(e) = self.save_script() {
                            self.error = Some(e);
                        } else {
                            self.toast = Some(ToastState::success("Script saved"));
                        }
                        ui.close_menu();
                    }

                    // Save As
                    if ui.button("💾 Save As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Script", &["json"])
                            .set_file_name("script.json")
                            .save_file()
                        {
                            if let Err(e) = self.save_script_as(&path) {
                                self.error = Some(e);
                            } else {
                                self.toast = Some(ToastState::success("Script saved"));
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    // Export Compiled
                    if ui.button("📦 Export Compiled...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
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

        // Error banner
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

        match self.mode {
            EditorMode::Player => self.render_player_mode(ctx),
            EditorMode::Editor => self.render_editor_mode(ctx),
        }
    }

    /// Renders player mode (game view only).
    fn render_player_mode(&mut self, ctx: &egui::Context) {
        // Delegate to player_ui module
        player_ui::render_player_ui(&mut self.engine, &mut self.toast, ctx);
    }

    /// Renders editor mode with panels.
    fn render_editor_mode(&mut self, ctx: &egui::Context) {
        // Left panel: Graph
        if self.show_graph {
            egui::SidePanel::left("graph_panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    GraphPanel::new(&self.graph, &mut self.selected_node).ui(ui);
                });
        }

        // Right panel: Inspector
        if self.show_inspector {
            egui::SidePanel::right("inspector_panel")
                .default_width(250.0)
                .resizable(true)
                .show(ctx, |ui| {
                    InspectorPanel::new(
                        &self.scene,
                        &self.graph,
                        self.selected_node,
                        self.selected_entity,
                    )
                    .ui(ui);
                });
        }

        // Bottom panel: Timeline
        if self.show_timeline {
            egui::TopBottomPanel::bottom("timeline_panel")
                .default_height(150.0)
                .resizable(true)
                .show(ctx, |ui| {
                    TimelinePanel::new(
                        &mut self.timeline,
                        &mut self.current_time,
                        &mut self.is_playing,
                    )
                    .ui(ui);
                });
        }

        // Global keyboard shortcuts for undo/redo
        ctx.input(|i| {
            // Ctrl+Z = Undo
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                if let Some(previous) = self.undo_stack.undo(self.node_graph.clone()) {
                    self.node_graph = previous;
                    self.toast = Some(ToastState::success("Undo"));
                }
            }
            // Ctrl+Y or Ctrl+Shift+Z = Redo
            if (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                if let Some(next) = self.undo_stack.redo(self.node_graph.clone()) {
                    self.node_graph = next;
                    self.toast = Some(ToastState::success("Redo"));
                }
            }
        });

        // Central panel: Viewport or Node Editor
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_node_editor {
                // Save state before potential modifications for undo
                let graph_before = self.node_graph.clone();

                NodeEditorPanel::new(&mut self.node_graph).ui(ui);

                // If graph was modified, push to undo stack
                if self.node_graph.is_modified() {
                    self.undo_stack.push(graph_before);
                    self.node_graph.clear_modified();
                }
            } else {
                ViewportPanel::new(&self.scene, &self.engine).ui(ui);
            }

            // Render toast notification on top
            node_rendering::render_toast(ui, &mut self.toast);
        });
    }
}
