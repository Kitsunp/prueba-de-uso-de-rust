use crate::error::{VnError, VnResult};
use crate::event::Event;
use crate::resource::ResourceLimiter;
use crate::script::Script;

#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    pub allow_empty_speaker: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_empty_speaker: false,
        }
    }
}

impl SecurityPolicy {
    pub fn validate(&self, script: &Script, limits: ResourceLimiter) -> VnResult<()> {
        if script.events.len() > limits.max_events {
            return Err(VnError::ResourceLimit("event count".to_string()));
        }

        if !script.labels.contains_key("start") {
            return Err(VnError::InvalidScript(
                "missing 'start' label".to_string(),
            ));
        }

        for (label, index) in &script.labels {
            if label.len() > limits.max_label_length {
                return Err(VnError::ResourceLimit(format!(
                    "label '{label}' too long"
                )));
            }
            if *index >= script.events.len() {
                return Err(VnError::InvalidScript(format!(
                    "label '{label}' points outside events"
                )));
            }
        }

        for event in &script.events {
            match event {
                Event::Dialogue(dialogue) => {
                    if !self.allow_empty_speaker && dialogue.speaker.trim().is_empty() {
                        return Err(VnError::SecurityPolicy(
                            "speaker cannot be empty".to_string(),
                        ));
                    }
                    if dialogue.text.len() > limits.max_text_length {
                        return Err(VnError::ResourceLimit("dialogue text".to_string()));
                    }
                }
                Event::Choice(choice) => {
                    if choice.prompt.len() > limits.max_text_length {
                        return Err(VnError::ResourceLimit("choice prompt".to_string()));
                    }
                    if choice.options.is_empty() {
                        return Err(VnError::InvalidScript(
                            "choice must have options".to_string(),
                        ));
                    }
                    for option in &choice.options {
                        if option.text.len() > limits.max_text_length {
                            return Err(VnError::ResourceLimit("choice option".to_string()));
                        }
                        if option.target.len() > limits.max_label_length {
                            return Err(VnError::ResourceLimit("choice target".to_string()));
                        }
                        if !script.labels.contains_key(&option.target) {
                            return Err(VnError::InvalidScript(format!(
                                "choice target '{}' not found",
                                option.target
                            )));
                        }
                    }
                }
                Event::Scene(scene) => {
                    if scene.characters.len() > limits.max_characters {
                        return Err(VnError::ResourceLimit("character count".to_string()));
                    }
                    if let Some(background) = &scene.background {
                        if background.len() > limits.max_asset_length {
                            return Err(VnError::ResourceLimit("background asset".to_string()));
                        }
                    }
                    if let Some(music) = &scene.music {
                        if music.len() > limits.max_asset_length {
                            return Err(VnError::ResourceLimit("music asset".to_string()));
                        }
                    }
                    for character in &scene.characters {
                        if character.name.len() > limits.max_asset_length {
                            return Err(VnError::ResourceLimit("character name".to_string()));
                        }
                        if let Some(expression) = &character.expression {
                            if expression.len() > limits.max_asset_length {
                                return Err(VnError::ResourceLimit(
                                    "character expression".to_string(),
                                ));
                            }
                        }
                        if let Some(position) = &character.position {
                            if position.len() > limits.max_asset_length {
                                return Err(VnError::ResourceLimit(
                                    "character position".to_string(),
                                ));
                            }
                        }
                    }
                }
                Event::Jump { target } => {
                    if target.len() > limits.max_label_length {
                        return Err(VnError::ResourceLimit("jump target".to_string()));
                    }
                    if !script.labels.contains_key(target) {
                        return Err(VnError::InvalidScript(format!(
                            "jump target '{target}' not found"
                        )));
                    }
                }
                Event::SetFlag { key, .. } => {
                    if key.len() > limits.max_label_length {
                        return Err(VnError::ResourceLimit("flag key".to_string()));
                    }
                }
            }
        }
        Ok(())
    }
}
