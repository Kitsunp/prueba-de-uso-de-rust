//! Visual state handling for scenes.

use serde::{Deserialize, Serialize};

use crate::event::{CharacterPlacementCompiled, SceneUpdateCompiled, SharedStr};

/// Current visual state for rendering.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VisualState {
    pub background: Option<SharedStr>,
    pub music: Option<SharedStr>,
    pub characters: Vec<CharacterPlacementCompiled>,
}

impl VisualState {
    /// Applies a scene update to the visual state.
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
    pub fn apply_patch(&mut self, patch: &crate::event::ScenePatchCompiled) {
        if let Some(background) = &patch.background {
            self.background = Some(background.clone());
        }
        if let Some(music) = &patch.music {
            self.music = Some(music.clone());
        }
        if let Some(chars) = &patch.characters {
            self.characters.clear();
            self.characters.extend_from_slice(chars);
        }
    }
}
