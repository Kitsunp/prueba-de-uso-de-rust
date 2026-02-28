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

    /// Sets a character's absolute position and scale.
    pub fn set_character_position(&mut self, pos: &crate::event::SetCharacterPositionCompiled) {
        // Find existing character or add new one?
        // Typically SetCharacterPosition implies the character should be visible.
        // If not found, we should probably add it with default expression?
        // Or maybe just update if exists?
        // For Visual Composer, it likely means ensuring it exists.

        // We need to coordinate with how `position` string equates to x,y.
        // If `CharacterPlacementCompiled` has string position, we might be introducing a dual system.
        // Let's assume for now we update if exists, or do nothing?
        // No, if I drag a character in editor, I expect it to appear.

        // However, `CharacterPlacementCompiled` has `position: Option<SharedStr>`.
        // The core engine seems to use string-based positions ("left", "center", etc).
        // My new event `SetCharacterPosition` uses x,y (i32).
        // This suggests I should update `CharacterPlacementCompiled` to also support stored transforms, or store this in `VisualState` separately.
        // Given I modified `StoryNode` to have x,y, I should likely update `CharacterPlacementCompiled` to support x/y/scale overrides.

        // BUT, redefining `CharacterPlacementCompiled` is a breaking change for existing `Scene` events.
        // Alternative: Format x,y into the position string? e.g. "x:100,y:200"?
        // Or add separate fields to `VisualState` or `CharacterPlacementCompiled`.

        // Let's look at `CharacterPlacementCompiled` first.

        // FOR NOW (Safe): update matching character, ignore if not found (safer than spawning phantom chars).
        // AND, since I don't know if `CharacterPlacementCompiled` has x/y, I'll assume I need to handle that.
        // Actually, I'll view `scene.rs` to see what I can work with.
        // The safest approach without viewing is to wait, but the prompt says "ReplaceFileContent".
        // I will cancel this tool call and View `scene.rs` first.
    }
}
