//! Visual state handling for scenes.

use serde::{Deserialize, Serialize};

use crate::event::{
    CharacterPlacementCompiled, ScenePatchCompiled, SceneUpdateCompiled, SharedStr,
};

/// Current visual state for rendering.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VisualState {
    pub background: Option<SharedStr>,
    pub music: Option<SharedStr>,
    pub characters: Vec<CharacterPlacementCompiled>,
}

impl VisualState {
    /// Applies a scene update to the visual state.
    ///
    /// Note: Scene events preserve existing values when fields are None.
    /// To fully replace/clear values, use Patch events with explicit null.
    pub fn apply_scene(&mut self, update: &SceneUpdateCompiled) {
        if let Some(background) = &update.background {
            self.background = Some(background.clone());
        }
        if let Some(music) = &update.music {
            self.music = Some(music.clone());
        }
        if !update.characters.is_empty() {
            self.characters.clear();
            self.characters.extend_from_slice(&update.characters);
        }
    }

    /// Applies a partial scene patch to the visual state.
    pub fn apply_patch(&mut self, patch: &ScenePatchCompiled) {
        if let Some(background) = &patch.background {
            self.background = Some(background.clone());
        }
        if let Some(music) = &patch.music {
            self.music = Some(music.clone());
        }
        if !patch.remove.is_empty() {
            let remove = patch
                .remove
                .iter()
                .map(|name| name.as_ref())
                .collect::<Vec<_>>();
            self.characters
                .retain(|character| !remove.contains(&character.name.as_ref()));
        }
        for patch_update in &patch.update {
            if let Some(existing) = self
                .characters
                .iter_mut()
                .find(|entry| entry.name.as_ref() == patch_update.name.as_ref())
            {
                if let Some(expression) = &patch_update.expression {
                    existing.expression = Some(expression.clone());
                }
                if let Some(position) = &patch_update.position {
                    existing.position = Some(position.clone());
                }
            }
        }
        if !patch.add.is_empty() {
            for new_character in &patch.add {
                match self
                    .characters
                    .iter_mut()
                    .find(|entry| entry.name.as_ref() == new_character.name.as_ref())
                {
                    Some(existing) => {
                        existing.expression = new_character.expression.clone();
                        existing.position = new_character.position.clone();
                    }
                    None => self.characters.push(new_character.clone()),
                }
            }
        }
    }
}
