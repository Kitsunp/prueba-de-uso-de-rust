//! Inspector panel for the editor workbench.
//!
//! Displays properties of selected nodes and entities.

use crate::editor::{NodeGraph, StoryNode};
use eframe::egui;
use visual_novel_engine::SceneState;

/// Inspector panel widget.
pub struct InspectorPanel<'a> {
    scene: &'a SceneState,
    graph: &'a mut NodeGraph, // Now mutable NodeGraph
    selected_node: Option<u32>,
    selected_entity: Option<u32>,
}

impl<'a> InspectorPanel<'a> {
    pub fn new(
        scene: &'a SceneState,
        graph: &'a mut NodeGraph,
        selected_node: Option<u32>,
        selected_entity: Option<u32>,
    ) -> Self {
        Self {
            scene,
            graph,
            selected_node,
            selected_entity,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Inspector");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Selected Node section
            ui.collapsing("Selected Node", |ui| {
                self.render_node_editor(ui);
            });

            ui.separator();

            // Selected Entity section
            ui.collapsing("Selected Entity", |ui| {
                self.render_entity_info(ui);
            });

            ui.separator();

            // Stats
            ui.label(format!("Graph Nodes: {}", self.graph.len()));
        });
    }

    fn render_node_editor(&mut self, ui: &mut egui::Ui) {
        let mut delete_option_idx = None;
        let mut add_option_req = false;
        let mut save_scene_profile_req: Option<String> = None;
        let mut apply_scene_profile_req: Option<String> = None;
        let mut standard_changed = false;
        let scene_profile_names = self.graph.scene_profile_names();

        if let Some(node_id) = self.selected_node {
            if let Some(node) = self.graph.get_node_mut(node_id) {
                ui.label(format!("Node ID: {}", node_id));
                ui.separator();

                match node {
                    StoryNode::Dialogue { speaker, text } => {
                        ui.label("Speaker:");
                        standard_changed |= ui.text_edit_singleline(speaker).changed();
                        ui.label("Text:");
                        standard_changed |= ui.text_edit_multiline(text).changed();
                    }
                    StoryNode::Choice { prompt, options } => {
                        ui.label("Prompt:");
                        standard_changed |= ui.text_edit_multiline(prompt).changed();

                        ui.separator();
                        ui.label("Options:");

                        // Option List
                        for (i, opt) in options.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                standard_changed |= ui.text_edit_singleline(opt).changed();
                                if ui.button("🗑").clicked() {
                                    delete_option_idx = Some(i);
                                }
                            });
                        }

                        if ui.button("➕ Add Option").clicked() {
                            add_option_req = true;
                        }
                    }
                    StoryNode::Scene {
                        profile,
                        background,
                        music,
                        characters,
                    } => {
                        let mut profile_id = profile.clone().unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Scene Profile:");
                            if ui.text_edit_singleline(&mut profile_id).changed() {
                                *profile = if profile_id.trim().is_empty() {
                                    None
                                } else {
                                    Some(profile_id.clone())
                                };
                                standard_changed = true;
                            }
                        });

                        if !scene_profile_names.is_empty() {
                            let selected_text = profile
                                .clone()
                                .unwrap_or_else(|| "<select profile>".to_string());
                            egui::ComboBox::from_label("Available Profiles")
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    for name in &scene_profile_names {
                                        if ui.selectable_label(false, name).clicked() {
                                            *profile = Some(name.clone());
                                            standard_changed = true;
                                        }
                                    }
                                });
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Save Profile").clicked() {
                                save_scene_profile_req = profile.clone();
                            }
                            if ui.button("Apply Profile").clicked() {
                                apply_scene_profile_req = profile.clone();
                            }
                        });

                        ui.separator();
                        let mut bg = background.clone().unwrap_or_default();
                        ui.label("Background Image:");
                        if ui.text_edit_singleline(&mut bg).changed() {
                            *background = if bg.trim().is_empty() { None } else { Some(bg) };
                            standard_changed = true;
                        }

                        let mut bgm = music.clone().unwrap_or_default();
                        ui.label("Background Music:");
                        if ui.text_edit_singleline(&mut bgm).changed() {
                            *music = if bgm.trim().is_empty() {
                                None
                            } else {
                                Some(bgm)
                            };
                            standard_changed = true;
                        }

                        ui.separator();
                        ui.label(format!("Characters in Scene: {}", characters.len()));
                        for character in characters.iter_mut() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Name:");
                                    standard_changed |=
                                        ui.text_edit_singleline(&mut character.name).changed();
                                });
                                let mut expr = character.expression.clone().unwrap_or_default();
                                ui.horizontal(|ui| {
                                    ui.label("Expr:");
                                    if ui.text_edit_singleline(&mut expr).changed() {
                                        character.expression = if expr.trim().is_empty() {
                                            None
                                        } else {
                                            Some(expr)
                                        };
                                        standard_changed = true;
                                    }
                                });
                                let mut pos = character.position.clone().unwrap_or_default();
                                ui.horizontal(|ui| {
                                    ui.label("Pos:");
                                    if ui.text_edit_singleline(&mut pos).changed() {
                                        character.position = if pos.trim().is_empty() {
                                            None
                                        } else {
                                            Some(pos)
                                        };
                                        standard_changed = true;
                                    }
                                });
                            });
                        }
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
                        ui.label("Target Label:");
                        standard_changed |= ui.text_edit_singleline(target).changed();

                        ui.separator();
                        ui.label("Condition:");

                        // Condition Type Selector
                        let is_flag = matches!(cond, visual_novel_engine::CondRaw::Flag { .. });
                        let mut type_changed = false;

                        egui::ComboBox::from_label("Type")
                            .selected_text(if is_flag {
                                "Flag"
                            } else {
                                "Variable Comparison"
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(is_flag, "Flag").clicked() && !is_flag {
                                    *cond = visual_novel_engine::CondRaw::Flag {
                                        key: "flag_name".to_string(),
                                        is_set: true,
                                    };
                                    type_changed = true;
                                }
                                if ui
                                    .selectable_label(!is_flag, "Variable Comparison")
                                    .clicked()
                                    && is_flag
                                {
                                    *cond = visual_novel_engine::CondRaw::VarCmp {
                                        key: "var_name".to_string(),
                                        op: visual_novel_engine::CmpOp::Eq,
                                        value: 0,
                                    };
                                    type_changed = true;
                                }
                            });

                        standard_changed |= type_changed;

                        match cond {
                            visual_novel_engine::CondRaw::Flag { key, is_set } => {
                                ui.label("Flag Key:");
                                standard_changed |= ui.text_edit_singleline(key).changed();
                                ui.horizontal(|ui| {
                                    ui.label("Is Set:");
                                    standard_changed |= ui.checkbox(is_set, "").changed();
                                });
                            }
                            visual_novel_engine::CondRaw::VarCmp { key, op, value } => {
                                ui.label("Var Key:");
                                standard_changed |= ui.text_edit_singleline(key).changed();

                                ui.horizontal(|ui| {
                                    ui.label("Op:");
                                    // Simple ComboBox for CmpOp
                                    // Converting CmpOp to string/debug for display
                                    let current_op = format!("{:?}", op);
                                    egui::ComboBox::from_id_source("cmp_op")
                                        .selected_text(current_op)
                                        .show_ui(ui, |ui| {
                                            let ops = [
                                                visual_novel_engine::CmpOp::Eq,
                                                visual_novel_engine::CmpOp::Ne,
                                                visual_novel_engine::CmpOp::Lt,
                                                visual_novel_engine::CmpOp::Le,
                                                visual_novel_engine::CmpOp::Gt,
                                                visual_novel_engine::CmpOp::Ge,
                                            ];
                                            for o in ops {
                                                if ui
                                                    .selectable_label(*op == o, format!("{:?}", o))
                                                    .clicked()
                                                {
                                                    *op = o;
                                                    standard_changed = true;
                                                }
                                            }
                                        });

                                    ui.label("Val:");
                                    standard_changed |=
                                        ui.add(egui::DragValue::new(value)).changed();
                                });
                            }
                        }
                    }
                    StoryNode::ScenePatch(patch) => {
                        ui.label("🎭 Scene Patch");
                        ui.separator();

                        // Music
                        let mut music = patch.music.clone().unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Music:");
                            if ui.text_edit_singleline(&mut music).changed() {
                                patch.music = if music.is_empty() {
                                    None
                                } else {
                                    Some(music.clone())
                                };
                                standard_changed = true;
                            }
                        });

                        // Background (Optional override)
                        let mut bg = patch.background.clone().unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Background (Override):");
                            if ui.text_edit_singleline(&mut bg).changed() {
                                patch.background = if bg.is_empty() {
                                    None
                                } else {
                                    Some(bg.clone())
                                };
                                standard_changed = true;
                            }
                        });

                        ui.separator();

                        // Add Character (Collapsing because complex)
                        ui.collapsing(format!("Add Characters ({})", patch.add.len()), |ui| {
                            let mut delete_add_idx = None;
                            for (i, char) in patch.add.iter_mut().enumerate() {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Name:");
                                        standard_changed |=
                                            ui.text_edit_singleline(&mut char.name).changed();
                                        if ui.button("🗑").clicked() {
                                            delete_add_idx = Some(i);
                                        }
                                    });
                                    let mut expr = char.expression.clone().unwrap_or_default();
                                    ui.horizontal(|ui| {
                                        ui.label("Expr:");
                                        if ui.text_edit_singleline(&mut expr).changed() {
                                            char.expression = if expr.is_empty() {
                                                None
                                            } else {
                                                Some(expr.clone())
                                            };
                                            standard_changed = true;
                                        }
                                    });
                                    let mut pos = char.position.clone().unwrap_or_default();
                                    ui.horizontal(|ui| {
                                        ui.label("Pos:");
                                        if ui.text_edit_singleline(&mut pos).changed() {
                                            char.position = if pos.is_empty() {
                                                None
                                            } else {
                                                Some(pos.clone())
                                            };
                                            standard_changed = true;
                                        }
                                    });
                                });
                            }
                            if let Some(idx) = delete_add_idx {
                                patch.add.remove(idx);
                                standard_changed = true;
                            }
                            if ui.button("➕ Add Character").clicked() {
                                patch
                                    .add
                                    .push(visual_novel_engine::CharacterPlacementRaw::default());
                                standard_changed = true;
                            }
                        });

                        // Remove Character
                        ui.separator();
                        ui.collapsing(
                            format!("Remove Characters ({})", patch.remove.len()),
                            |ui| {
                                let mut delete_rem_idx = None;
                                for (i, name) in patch.remove.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("Name:");
                                        standard_changed |= ui.text_edit_singleline(name).changed();
                                        if ui.button("🗑").clicked() {
                                            delete_rem_idx = Some(i);
                                        }
                                    });
                                }
                                if let Some(idx) = delete_rem_idx {
                                    patch.remove.remove(idx);
                                    standard_changed = true;
                                }
                                if ui.button("➕ REMOVE Character").clicked() {
                                    patch.remove.push("StartTypingName".to_string());
                                    standard_changed = true;
                                }
                            },
                        );
                    }
                    StoryNode::Generic(event) => {
                        ui.label("📦 Generic Event (Read-Only)");
                        ui.label("This event type is not yet fully supported in the editor UI, but its data is preserved.");

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
                        ui.label("🔊 Audio Action");
                        ui.separator();

                        ui.label("Channel (bgm/sfx/voice):");
                        standard_changed |= ui.text_edit_singleline(channel).changed();

                        ui.label("Action (play/stop/fade_out):");
                        standard_changed |= ui.text_edit_singleline(action).changed();

                        let mut asset_str = asset.clone().unwrap_or_default();
                        ui.label("Asset Path:");
                        let asset_resp = ui.text_edit_singleline(&mut asset_str);
                        if asset_resp.changed() {
                            *asset = if asset_str.is_empty() {
                                None
                            } else {
                                Some(asset_str)
                            };
                            standard_changed = true;
                        }

                        // Drag & Drop Logic
                        if asset_resp.hovered() && ui.ctx().dragged_id().is_some() {
                            // Check payload
                            let payload = ui.memory(|mem| {
                                mem.data.get_temp::<String>(egui::Id::new("dragged_asset"))
                            });
                            if let Some(payload) = payload {
                                /* We can't access payload content here unless we clone it or it's Copy.
                                   Mem.data returns Option<T>.
                                   Wait, get_temp returns Option<T> where T: Clone. String is Clone.
                                */
                                // Check if it's audio
                                if payload.starts_with("asset://audio/") {
                                    // Visual feedback
                                    ui.painter().rect_stroke(
                                        asset_resp.rect,
                                        0.0,
                                        (2.0, egui::Color32::GREEN),
                                    );

                                    if ui.input(|i| i.pointer.any_released()) {
                                        if let Some(filename) =
                                            payload.strip_prefix("asset://audio/")
                                        {
                                            *asset = Some(filename.to_string());
                                            standard_changed = true;
                                        }
                                    }
                                }
                            }
                        }

                        ui.separator();
                        ui.label("Options:");

                        let mut vol = volume.unwrap_or(1.0);
                        ui.horizontal(|ui| {
                            ui.label("Volume:");
                            if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0)).changed() {
                                *volume = Some(vol);
                                standard_changed = true;
                            }
                        });

                        let mut fade = fade_duration_ms.unwrap_or(0);
                        ui.horizontal(|ui| {
                            ui.label("Fade (ms):");
                            if ui.add(egui::DragValue::new(&mut fade)).changed() {
                                *fade_duration_ms = if fade > 0 { Some(fade) } else { None };
                                standard_changed = true;
                            }
                        });

                        let mut looping = loop_playback.unwrap_or(false);
                        ui.horizontal(|ui| {
                            ui.label("Loop:");
                            if ui.checkbox(&mut looping, "").changed() {
                                *loop_playback = Some(looping);
                                standard_changed = true;
                            }
                        });
                    }
                    StoryNode::Transition {
                        kind,
                        duration_ms,
                        color,
                    } => {
                        ui.label("⏳ Transition");
                        ui.separator();

                        ui.label("Kind (fade/dissolve/cut):");
                        standard_changed |= ui.text_edit_singleline(kind).changed();

                        ui.label("Duration (ms):");
                        standard_changed |= ui.add(egui::DragValue::new(duration_ms)).changed();

                        let mut color_str = color.clone().unwrap_or_default();
                        ui.label("Color (Hex/Name):");
                        if ui.text_edit_singleline(&mut color_str).changed() {
                            *color = if color_str.is_empty() {
                                None
                            } else {
                                Some(color_str)
                            };
                            standard_changed = true;
                        }
                    }
                    StoryNode::CharacterPlacement { name, x, y, scale } => {
                        ui.label("🧍 Character Placement");
                        ui.separator();

                        ui.label("Name:");
                        standard_changed |= ui.text_edit_singleline(name).changed();

                        ui.horizontal(|ui| {
                            ui.label("Position:");
                            ui.label("X");
                            standard_changed |= ui.add(egui::DragValue::new(x)).changed();
                            ui.label("Y");
                            standard_changed |= ui.add(egui::DragValue::new(y)).changed();
                        });

                        ui.horizontal(|ui| {
                            ui.label("Scale:");
                            let mut s = scale.unwrap_or(1.0);
                            if ui.add(egui::DragValue::new(&mut s).speed(0.1)).changed() {
                                *scale = Some(s);
                                standard_changed = true;
                            }
                        });
                    }
                }

                if standard_changed {
                    self.graph.mark_modified();
                }
            } else {
                ui.label("Node not found in editor graph.");
                return; // Avoid borrow conflicts if we continued
            }

            // Apply Structural Changes (After dropping node borrow)
            if let Some(idx) = delete_option_idx {
                self.graph.remove_choice_option(node_id, idx);
            }

            if add_option_req {
                if let Some(StoryNode::Choice { options, .. }) = self.graph.get_node_mut(node_id) {
                    options.push("New Option".to_string());
                    self.graph.mark_modified();
                }
            }

            if let Some(profile_id) = save_scene_profile_req {
                let _ = self.graph.save_scene_profile(profile_id, node_id);
            }
            if let Some(profile_id) = apply_scene_profile_req {
                let _ = self.graph.apply_scene_profile(&profile_id, node_id);
            }
        } else {
            ui.label("No node selected");
        }
    }
}

mod entity_info;
