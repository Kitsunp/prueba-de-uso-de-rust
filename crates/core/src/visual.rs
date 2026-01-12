//! Visual state handling for scenes.

use crate::event::{CharacterPlacementCompiled, SceneUpdateCompiled, SharedStr};

/// Current visual state for rendering.
#[derive(Clone, Debug, Default)]
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
            self.characters = update.characters.clone();
        }
    }
}
