#[derive(Clone, Copy, Debug)]
pub struct ResourceLimiter {
    pub max_events: usize,
    pub max_text_length: usize,
    pub max_label_length: usize,
    pub max_asset_length: usize,
    pub max_characters: usize,
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self {
            max_events: 10_000,
            max_text_length: 4_096,
            max_label_length: 64,
            max_asset_length: 128,
            max_characters: 32,
        }
    }
}
