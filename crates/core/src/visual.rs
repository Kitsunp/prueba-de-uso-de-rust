use crate::event::{CharacterPlacement, SceneUpdate};

#[derive(Clone, Debug, Default)]
pub struct VisualState {
    pub background: Option<String>,
    pub music: Option<String>,
    pub characters: Vec<CharacterPlacement>,
}

impl VisualState {
    pub fn apply_scene(&mut self, update: &SceneUpdate) {
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
