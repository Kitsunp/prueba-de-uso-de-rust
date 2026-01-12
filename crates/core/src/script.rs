//! Raw and compiled script representations.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{VnError, VnResult};
use crate::event::{
    CharacterPlacementCompiled, ChoiceCompiled, ChoiceOptionCompiled, DialogueCompiled,
    EventCompiled, EventRaw, SceneUpdateCompiled, SharedStr,
};

/// JSON-facing script format with label names and raw string data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptRaw {
    pub events: Vec<EventRaw>,
    pub labels: HashMap<String, usize>,
}

/// Runtime-ready script that resolves labels and interns strings.
#[derive(Clone, Debug)]
pub struct ScriptCompiled {
    pub events: Vec<EventCompiled>,
    pub labels: HashMap<String, u32>,
    pub start_ip: u32,
    pub flag_count: u32,
}

impl ScriptRaw {
    /// Creates a raw script from events and labels.
    pub fn new(events: Vec<EventRaw>, labels: HashMap<String, usize>) -> Self {
        Self { events, labels }
    }

    /// Parses a JSON script into a raw script structure.
    pub fn from_json(input: &str) -> VnResult<Self> {
        serde_json::from_str(input).map_err(|err| {
            let (offset, length) = json_error_span(input, &err);
            VnError::Serialization {
                message: err.to_string(),
                src: input.to_string(),
                span: (offset, length).into(),
            }
        })
    }

    /// Returns the index of the `start` label.
    pub fn start_index(&self) -> VnResult<usize> {
        self.labels
            .get("start")
            .copied()
            .ok_or_else(|| VnError::InvalidScript("missing 'start' label".to_string()))
    }

    /// Compiles a raw script into its runtime representation.
    ///
    /// Resolves label targets, assigns flag ids, and interns repeated strings.
    pub fn compile(&self) -> VnResult<ScriptCompiled> {
        let _event_len = u32::try_from(self.events.len())
            .map_err(|_| VnError::InvalidScript("event count exceeds u32::MAX".to_string()))?;
        let mut pool = StringPool::default();
        let mut compiled_events = Vec::with_capacity(self.events.len());
        let mut compiled_labels = HashMap::with_capacity(self.labels.len());
        let mut flag_map: HashMap<String, u32> = HashMap::new();

        for (label, index) in &self.labels {
            if *index >= self.events.len() {
                return Err(VnError::InvalidScript(format!(
                    "label '{label}' points outside events"
                )));
            }
            let ip = u32::try_from(*index)
                .map_err(|_| VnError::InvalidScript(format!("label '{label}' out of range")))?;
            compiled_labels.insert(label.clone(), ip);
        }

        let start_ip = compiled_labels
            .get("start")
            .copied()
            .ok_or_else(|| VnError::InvalidScript("missing 'start' label".to_string()))?;

        for event in &self.events {
            let compiled = match event {
                EventRaw::Dialogue(dialogue) => EventCompiled::Dialogue(DialogueCompiled {
                    speaker: pool.intern(&dialogue.speaker),
                    text: pool.intern(&dialogue.text),
                }),
                EventRaw::Choice(choice) => EventCompiled::Choice(ChoiceCompiled {
                    prompt: pool.intern(&choice.prompt),
                    options: choice
                        .options
                        .iter()
                        .map(|option| {
                            let target_ip = compiled_labels
                                .get(&option.target)
                                .copied()
                                .ok_or_else(|| {
                                    VnError::InvalidScript(format!(
                                        "choice target '{}' not found",
                                        option.target
                                    ))
                                })?;
                            Ok(ChoiceOptionCompiled {
                                text: pool.intern(&option.text),
                                target_ip,
                            })
                        })
                        .collect::<VnResult<Vec<_>>>()?,
                }),
                EventRaw::Scene(scene) => EventCompiled::Scene(SceneUpdateCompiled {
                    background: scene.background.as_deref().map(|value| pool.intern(value)),
                    music: scene.music.as_deref().map(|value| pool.intern(value)),
                    characters: scene
                        .characters
                        .iter()
                        .map(|character| CharacterPlacementCompiled {
                            name: pool.intern(&character.name),
                            expression: character
                                .expression
                                .as_deref()
                                .map(|value| pool.intern(value)),
                            position: character
                                .position
                                .as_deref()
                                .map(|value| pool.intern(value)),
                        })
                        .collect(),
                }),
                EventRaw::Jump { target } => {
                    let target_ip = compiled_labels.get(target).copied().ok_or_else(|| {
                        VnError::InvalidScript(format!("jump target '{target}' not found"))
                    })?;
                    EventCompiled::Jump { target_ip }
                }
                EventRaw::SetFlag { key, value } => {
                    let flag_id = match flag_map.get(key) {
                        Some(id) => *id,
                        None => {
                            let next_id = u32::try_from(flag_map.len()).map_err(|_| {
                                VnError::InvalidScript("too many flags".to_string())
                            })?;
                            flag_map.insert(key.clone(), next_id);
                            next_id
                        }
                    };
                    EventCompiled::SetFlag {
                        flag_id,
                        value: *value,
                    }
                }
            };
            compiled_events.push(compiled);
        }

        Ok(ScriptCompiled {
            events: compiled_events,
            labels: compiled_labels,
            start_ip,
            flag_count: flag_map.len() as u32,
        })
    }
}

fn json_error_span(input: &str, error: &serde_json::Error) -> (usize, usize) {
    let line = error.line();
    let column = error.column();
    if line == 0 || column == 0 {
        return (0, 1);
    }
    let mut current_line = 1usize;
    let mut offset = 0usize;
    for chunk in input.split_inclusive('\n') {
        if current_line == line {
            let column_index = column.saturating_sub(1);
            let byte_index = chunk
                .char_indices()
                .nth(column_index)
                .map(|(idx, _)| idx)
                .unwrap_or(chunk.len().saturating_sub(1));
            offset += byte_index;
            return (offset, 1);
        }
        offset += chunk.len();
        current_line += 1;
    }
    (input.len().saturating_sub(1), 1)
}

#[derive(Default)]
struct StringPool {
    cache: HashMap<String, SharedStr>,
}

impl StringPool {
    fn intern(&mut self, value: &str) -> SharedStr {
        if let Some(existing) = self.cache.get(value) {
            return existing.clone();
        }
        let shared: SharedStr = Arc::from(value);
        self.cache.insert(value.to_string(), shared.clone());
        shared
    }
}
