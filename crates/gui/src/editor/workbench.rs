use eframe::egui;
use visual_novel_engine::{Engine, ScriptRaw};

use crate::editor::{
    asset_browser::AssetBrowserPanel,
    diff_dialog::DiffDialog,
    graph_panel::GraphPanel,
    inspector_panel::InspectorPanel,
    lint_panel::LintPanel,
    node_graph::NodeGraph,
    node_types::ToastState,
    timeline_panel::TimelinePanel,
    undo::UndoStack,
    EditorMode,
    LintIssue,
    LintSeverity, // Imported from mod.rs export
};
use crate::VnConfig;

/// Main editor workbench state and UI.
pub struct EditorWorkbench {
    pub config: VnConfig,
    pub node_graph: NodeGraph,
    pub undo_stack: UndoStack,
    pub manifest: Option<visual_novel_engine::manifest::ProjectManifest>,
    pub current_script: Option<ScriptRaw>,
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

    // Feedback
    pub toast: Option<ToastState>,
    pub diff_dialog: Option<DiffDialog>,

    // New layout flags
    pub node_editor_window_open: bool,
}

impl EditorWorkbench {
    pub fn new(config: VnConfig) -> Self {
        // Initialize with default/empty state
        let graph = NodeGraph::default();
        if graph.nodes.is_empty() {
            // Optional: graph.add_node(...)
        }

        let mut undo_stack = UndoStack::new();
        undo_stack.push(graph.clone());

        Self {
            config,
            node_graph: graph,
            undo_stack,
            manifest: None,
            current_script: None,
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
            toast: None,
            diff_dialog: None,
            node_editor_window_open: false,
        }
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
                self.toast = Some(crate::editor::node_types::ToastState::error(&format!(
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
                self.toast = Some(crate::editor::node_types::ToastState::error(&format!(
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
            self.toast = Some(ToastState::error(&format!("Save failed: {}", e)));
        } else {
            // Success
        }
    }

    pub fn sync_graph_to_script(&mut self) -> Result<(), String> {
        let result = crate::editor::compiler::compile_project(&self.node_graph);

        // Update State
        self.current_script = Some(result.script);
        self.validation_issues = result.issues;
        self.show_validation = !self.validation_issues.is_empty();

        match result.engine_result {
            Ok(engine) => {
                self.engine = Some(engine);
                Ok(())
            }
            Err(e) => {
                self.validation_issues.push(LintIssue {
                    node_id: None,
                    severity: LintSeverity::Error,
                    message: format!("Engine Error: {}", e),
                });
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
                ui.selectable_value(&mut self.mode, EditorMode::Editor, "🛠 Editor");
                ui.selectable_value(&mut self.mode, EditorMode::Player, "▶ Player");
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
            if let Some(path) = self.pending_save_path.clone() {
                self.execute_save(&path, "");
                self.toast = Some(ToastState::success("Saved successfully"));
            }
            self.diff_dialog = None;
            self.show_save_confirm = false;
        }
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

        // 2. Right Panel (Inspector)
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

        // 3. Left Panel (Asset Browser)
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

        // 4. Central Area (Docking Logic)

        // Prepare Data for decoupled rendering to avoid simultaneous mutable borrows
        let entity_owners = self.build_entity_node_map();
        let mut composer_actions = Vec::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut composer = crate::editor::visual_composer::VisualComposerPanel::new(
                &mut self.scene,
                &self.engine,
                &mut self.selected_entity,
            );

            if self.node_editor_window_open {
                // Detached Mode: Composer fills background
                if let Some(act) = composer.ui(ui, &entity_owners) {
                    composer_actions.push(act);
                }
            } else {
                // Docked Mode: Split View (Graph | Composer)
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.heading("Logic Graph");
                        let mut panel = GraphPanel::new(&mut self.node_graph);
                        panel.ui(ui);
                    });

                    columns[1].vertical(|ui| {
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

        // 6. Floating Window (Detached Node Editor)
        if self.node_editor_window_open {
            let mut open = self.node_editor_window_open;
            egui::Window::new("Node Editor")
                .open(&mut open)
                .resizable(true)
                .show(ctx, |ui| {
                    let mut panel = GraphPanel::new(&mut self.node_graph);
                    panel.ui(ui);
                });
            self.node_editor_window_open = open;

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
