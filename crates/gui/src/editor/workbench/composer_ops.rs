use super::*;
use crate::editor::StoryNode;

impl EditorWorkbench {
    pub(super) fn build_entity_node_map(&self) -> std::collections::HashMap<u32, u32> {
        let mut map = std::collections::HashMap::new();
        use crate::editor::node_types::StoryNode;

        let mut bind_owner = |entity_id: u32, node_id: u32, prefer_existing: bool| {
            if prefer_existing {
                map.entry(entity_id).or_insert(node_id);
            } else {
                map.insert(entity_id, node_id);
            }
        };

        // Fallback ownership map when preview trace ownership is unavailable.
        for (nid, node, _) in self.node_graph.nodes() {
            match node {
                StoryNode::Dialogue { speaker, .. } => {
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Character(c) = &entity.kind {
                            if c.name.as_ref() == speaker.as_str() {
                                bind_owner(entity.id.raw(), *nid, false);
                            }
                        }
                    }
                }
                StoryNode::Scene {
                    background,
                    music,
                    characters,
                    ..
                } => {
                    if let Some(background) = background {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Image(img) = &entity.kind {
                                if img.path.as_ref() == background.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                    if let Some(music) = music {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Audio(audio) = &entity.kind {
                                if audio.path.as_ref() == music.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                    for character in characters {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Character(c) = &entity.kind {
                                if c.name.as_ref() == character.name.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                        if let Some(expression) = &character.expression {
                            for entity in self.scene.iter() {
                                if let visual_novel_engine::EntityKind::Image(img) = &entity.kind {
                                    if img.path.as_ref() == expression.as_str() {
                                        bind_owner(entity.id.raw(), *nid, false);
                                    }
                                }
                            }
                        }
                    }
                }
                StoryNode::ScenePatch(patch) => {
                    if let Some(background) = &patch.background {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Image(img) = &entity.kind {
                                if img.path.as_ref() == background.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                    if let Some(music) = &patch.music {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Audio(audio) = &entity.kind {
                                if audio.path.as_ref() == music.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                    for character in &patch.add {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Character(c) = &entity.kind {
                                if c.name.as_ref() == character.name.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                    for character in &patch.update {
                        for entity in self.scene.iter() {
                            if let visual_novel_engine::EntityKind::Character(c) = &entity.kind {
                                if c.name.as_ref() == character.name.as_str() {
                                    bind_owner(entity.id.raw(), *nid, false);
                                }
                            }
                        }
                    }
                }
                StoryNode::CharacterPlacement { name, .. } => {
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Character(character) = &entity.kind
                        {
                            if character.name.as_ref() == name.as_str() {
                                bind_owner(entity.id.raw(), *nid, false);
                            }
                        }
                    }
                }
                StoryNode::AudioAction {
                    asset: Some(asset), ..
                } => {
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Audio(audio) = &entity.kind {
                            if audio.path.as_ref() == asset.as_str() {
                                // Keep scene/patch ownership when already resolved.
                                bind_owner(entity.id.raw(), *nid, true);
                            }
                        }
                    }
                }
                StoryNode::Generic(visual_novel_engine::EventRaw::SetCharacterPosition(pos)) => {
                    for entity in self.scene.iter() {
                        if let visual_novel_engine::EntityKind::Character(character) = &entity.kind
                        {
                            if character.name.as_ref() == pos.name.as_str() {
                                bind_owner(entity.id.raw(), *nid, false);
                            }
                        }
                    }
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
