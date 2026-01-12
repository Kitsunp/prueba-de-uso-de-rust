use std::collections::HashMap;

use crate::visual::VisualState;

#[derive(Clone, Debug)]
pub struct EngineState {
    pub position: usize,
    pub flags: HashMap<String, bool>,
    pub visual: VisualState,
}

impl EngineState {
    pub fn new(position: usize) -> Self {
        Self {
            position,
            flags: HashMap::new(),
            visual: VisualState::default(),
        }
    }
}
