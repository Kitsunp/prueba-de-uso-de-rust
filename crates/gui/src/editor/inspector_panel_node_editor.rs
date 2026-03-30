use crate::editor::StoryNode;
use eframe::egui;

use super::InspectorPanel;

#[path = "inspector_panel_node_sections.rs"]
mod node_sections;

#[derive(Default)]
struct NodeEditActions {
    delete_option_idx: Option<usize>,
    add_option_req: bool,
    save_scene_profile_req: Option<String>,
    apply_scene_profile_req: Option<String>,
}

impl<'a> InspectorPanel<'a> {
    pub(super) fn render_node_editor(&mut self, ui: &mut egui::Ui) {
        let mut actions = NodeEditActions::default();
        let mut standard_changed = false;
        let scene_profile_names = self.graph.scene_profile_names();

        let Some(node_id) = self.selected_node else {
            ui.label("No node selected");
            return;
        };

        if let Some(node) = self.graph.get_node_mut(node_id) {
            ui.label(format!("Node ID: {}", node_id));
            ui.separator();

            match node {
                StoryNode::Dialogue { speaker, text } => {
                    node_sections::render_dialogue_node(ui, speaker, text, &mut standard_changed);
                }
                StoryNode::Choice { prompt, options } => {
                    node_sections::render_choice_node(
                        ui,
                        prompt,
                        options,
                        &mut standard_changed,
                        &mut actions,
                    );
                }
                StoryNode::Scene {
                    profile,
                    background,
                    music,
                    characters,
                } => {
                    node_sections::render_scene_node(
                        ui,
                        node_sections::SceneNodeRefs {
                            profile,
                            background,
                            music,
                            characters,
                        },
                        &scene_profile_names,
                        &mut standard_changed,
                        &mut actions,
                    );
                }
                StoryNode::Jump { target } => {
                    ui.label("Jump Target (Label):");
                    standard_changed |= ui.text_edit_singleline(target).changed();
                }
                StoryNode::Start => {
                    ui.label("Start Node (Entry Point)");
                }
                StoryNode::End => {
                    ui.label("End Node (Termination)");
                }
                StoryNode::SetVariable { key, value } => {
                    ui.label("Variable Name:");
                    standard_changed |= ui.text_edit_singleline(key).changed();
                    ui.label("Value (i32):");
                    standard_changed |= ui.add(egui::DragValue::new(value)).changed();
                }
                StoryNode::JumpIf { target, cond } => {
                    node_sections::render_jump_if_node(ui, target, cond, &mut standard_changed);
                }
                StoryNode::ScenePatch(patch) => {
                    node_sections::render_scene_patch_node(ui, patch, &mut standard_changed);
                }
                StoryNode::Generic(event) => {
                    ui.label("Generic Event (Read-Only)");
                    ui.label(
                        "This event type is not yet fully supported in the editor UI, but its data is preserved.",
                    );

                    let json = event.to_json_string();
                    ui.add(
                        egui::TextEdit::multiline(&mut json.as_str())
                            .code_editor()
                            .interactive(false),
                    );
                }
                StoryNode::AudioAction {
                    channel,
                    action,
                    asset,
                    volume,
                    fade_duration_ms,
                    loop_playback,
                } => {
                    node_sections::render_audio_action_node(
                        ui,
                        node_sections::AudioActionRefs {
                            channel,
                            action,
                            asset,
                            volume,
                            fade_duration_ms,
                            loop_playback,
                        },
                        &mut standard_changed,
                    );
                }
                StoryNode::Transition {
                    kind,
                    duration_ms,
                    color,
                } => {
                    node_sections::render_transition_node(
                        ui,
                        kind,
                        duration_ms,
                        color,
                        &mut standard_changed,
                    );
                }
                StoryNode::CharacterPlacement { name, x, y, scale } => {
                    node_sections::render_character_placement_node(
                        ui,
                        name,
                        x,
                        y,
                        scale,
                        &mut standard_changed,
                    );
                }
            }

            if standard_changed {
                self.graph.mark_modified();
            }
        } else {
            ui.label("Node not found in editor graph.");
            return;
        }

        if let Some(idx) = actions.delete_option_idx {
            self.graph.remove_choice_option(node_id, idx);
        }

        if actions.add_option_req {
            if let Some(StoryNode::Choice { options, .. }) = self.graph.get_node_mut(node_id) {
                options.push("New Option".to_string());
                self.graph.mark_modified();
            }
        }

        if let Some(profile_id) = actions.save_scene_profile_req {
            let _ = self.graph.save_scene_profile(profile_id, node_id);
        }
        if let Some(profile_id) = actions.apply_scene_profile_req {
            let _ = self.graph.apply_scene_profile(&profile_id, node_id);
        }
    }
}
