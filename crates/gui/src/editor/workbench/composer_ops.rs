use super::*;
use crate::editor::StoryNode;

impl EditorWorkbench {
    pub(super) fn build_entity_node_map(&self) -> std::collections::HashMap<u32, u32> {
        let mut map = std::collections::HashMap::new();
        use crate::editor::node_types::StoryNode;
        use std::collections::HashMap;

        let mut characters_by_name: HashMap<String, Vec<u32>> = HashMap::new();
        let mut images_by_path: HashMap<String, Vec<u32>> = HashMap::new();
        let mut audio_by_path: HashMap<String, Vec<u32>> = HashMap::new();

        for entity in self.scene.iter() {
            match &entity.kind {
                visual_novel_engine::EntityKind::Character(character) => {
                    characters_by_name
                        .entry(character.name.to_string())
                        .or_default()
                        .push(entity.id.raw());
                }
                visual_novel_engine::EntityKind::Image(image) => {
                    images_by_path
                        .entry(image.path.to_string())
                        .or_default()
                        .push(entity.id.raw());
                }
                visual_novel_engine::EntityKind::Audio(audio) => {
                    audio_by_path
                        .entry(audio.path.to_string())
                        .or_default()
                        .push(entity.id.raw());
                }
                _ => {}
            }
        }

        let mut bind_owner = |entity_id: u32, node_id: u32, prefer_existing: bool| {
            if prefer_existing {
                map.entry(entity_id).or_insert(node_id);
            } else {
                map.insert(entity_id, node_id);
            }
        };
        let mut bind_matches = |matches: Option<&Vec<u32>>, node_id: u32, prefer_existing: bool| {
            if let Some(entity_ids) = matches {
                for &entity_id in entity_ids {
                    bind_owner(entity_id, node_id, prefer_existing);
                }
            }
        };

        // Fallback ownership map when preview trace ownership is unavailable.
        for (nid, node, _) in self.node_graph.nodes() {
            match node {
                StoryNode::Dialogue { speaker, .. } => {
                    bind_matches(characters_by_name.get(speaker.as_str()), *nid, false);
                }
                StoryNode::Scene {
                    background,
                    music,
                    characters,
                    ..
                } => {
                    if let Some(background) = background {
                        bind_matches(images_by_path.get(background.as_str()), *nid, false);
                    }
                    if let Some(music) = music {
                        bind_matches(audio_by_path.get(music.as_str()), *nid, false);
                    }
                    for character in characters {
                        bind_matches(characters_by_name.get(character.name.as_str()), *nid, false);
                        if let Some(expression) = &character.expression {
                            bind_matches(images_by_path.get(expression.as_str()), *nid, false);
                        }
                    }
                }
                StoryNode::ScenePatch(patch) => {
                    if let Some(background) = &patch.background {
                        bind_matches(images_by_path.get(background.as_str()), *nid, false);
                    }
                    if let Some(music) = &patch.music {
                        bind_matches(audio_by_path.get(music.as_str()), *nid, false);
                    }
                    for character in &patch.add {
                        bind_matches(characters_by_name.get(character.name.as_str()), *nid, false);
                    }
                    for character in &patch.update {
                        bind_matches(characters_by_name.get(character.name.as_str()), *nid, false);
                    }
                }
                StoryNode::CharacterPlacement { name, .. } => {
                    bind_matches(characters_by_name.get(name.as_str()), *nid, false);
                }
                StoryNode::AudioAction {
                    asset: Some(asset), ..
                } => {
                    // Keep scene/patch ownership when already resolved.
                    bind_matches(audio_by_path.get(asset.as_str()), *nid, true);
                }
                StoryNode::Generic(visual_novel_engine::EventRaw::SetCharacterPosition(pos)) => {
                    bind_matches(characters_by_name.get(pos.name.as_str()), *nid, false);
                }
                _ => {}
            }
        }
        map
    }

    pub(crate) fn apply_composer_node_mutation(
        &mut self,
        node_id: u32,
        mutation: crate::editor::visual_composer::ComposerNodeMutation,
    ) -> bool {
        match mutation {
            crate::editor::visual_composer::ComposerNodeMutation::CharacterPosition {
                name,
                x,
                y,
                scale,
            } => {
                let Some(node) = self.node_graph.get_node_mut(node_id) else {
                    return false;
                };
                match node {
                    StoryNode::CharacterPlacement {
                        name: node_name,
                        x: node_x,
                        y: node_y,
                        scale: node_scale,
                    } => {
                        let changed = *node_name != name
                            || *node_x != x
                            || *node_y != y
                            || *node_scale != scale;
                        if changed {
                            *node_name = name;
                            *node_x = x;
                            *node_y = y;
                            *node_scale = scale;
                        }
                        changed
                    }
                    StoryNode::Scene { characters, .. } => {
                        if let Some(character) =
                            characters.iter_mut().find(|entry| entry.name == name)
                        {
                            let changed = character.x != Some(x)
                                || character.y != Some(y)
                                || character.scale != scale;
                            if changed {
                                character.x = Some(x);
                                character.y = Some(y);
                                character.scale = scale;
                            }
                            changed
                        } else {
                            characters.push(visual_novel_engine::CharacterPlacementRaw {
                                name,
                                expression: None,
                                position: None,
                                x: Some(x),
                                y: Some(y),
                                scale,
                            });
                            true
                        }
                    }
                    StoryNode::ScenePatch(patch) => {
                        if let Some(character) =
                            patch.add.iter_mut().find(|entry| entry.name == name)
                        {
                            let changed = character.x != Some(x)
                                || character.y != Some(y)
                                || character.scale != scale;
                            if changed {
                                character.x = Some(x);
                                character.y = Some(y);
                                character.scale = scale;
                            }
                            changed
                        } else {
                            patch.add.push(visual_novel_engine::CharacterPlacementRaw {
                                name,
                                expression: None,
                                position: None,
                                x: Some(x),
                                y: Some(y),
                                scale,
                            });
                            true
                        }
                    }
                    StoryNode::Generic(visual_novel_engine::EventRaw::SetCharacterPosition(
                        pos,
                    )) => {
                        let changed =
                            pos.name != name || pos.x != x || pos.y != y || pos.scale != scale;
                        if changed {
                            pos.name = name;
                            pos.x = x;
                            pos.y = y;
                            pos.scale = scale;
                        }
                        changed
                    }
                    _ => false,
                }
            }
        }
    }
}
