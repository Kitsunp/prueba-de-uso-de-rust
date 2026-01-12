//! Event definitions for raw and compiled scripts.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Shared string storage used by compiled events.
pub type SharedStr = Arc<str>;

/// JSON-facing events used in `ScriptRaw`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventRaw {
    Dialogue(DialogueRaw),
    Choice(ChoiceRaw),
    Scene(SceneUpdateRaw),
    Jump { target: String },
    SetFlag { key: String, value: bool },
}

/// Runtime events with pre-resolved targets and interned strings.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventCompiled {
    Dialogue(DialogueCompiled),
    Choice(ChoiceCompiled),
    Scene(SceneUpdateCompiled),
    Jump { target_ip: u32 },
    SetFlag { flag_id: u32, value: bool },
}

/// Dialogue line with speaker and text in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueRaw {
    pub speaker: String,
    pub text: String,
}

/// Dialogue line with interned speaker and text.
#[derive(Clone, Debug, Serialize)]
pub struct DialogueCompiled {
    pub speaker: SharedStr,
    pub text: SharedStr,
}

/// Choice prompt and options in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceRaw {
    pub prompt: String,
    pub options: Vec<ChoiceOptionRaw>,
}

/// Choice prompt and options with pre-resolved targets.
#[derive(Clone, Debug, Serialize)]
pub struct ChoiceCompiled {
    pub prompt: SharedStr,
    pub options: Vec<ChoiceOptionCompiled>,
}

/// Choice option with label target in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOptionRaw {
    pub text: String,
    pub target: String,
}

/// Choice option with pre-resolved target instruction pointer.
#[derive(Clone, Debug, Serialize)]
pub struct ChoiceOptionCompiled {
    pub text: SharedStr,
    pub target_ip: u32,
}

/// Scene update payload in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneUpdateRaw {
    pub background: Option<String>,
    pub music: Option<String>,
    pub characters: Vec<CharacterPlacementRaw>,
}

/// Scene update payload with interned strings.
#[derive(Clone, Debug, Serialize)]
pub struct SceneUpdateCompiled {
    pub background: Option<SharedStr>,
    pub music: Option<SharedStr>,
    pub characters: Vec<CharacterPlacementCompiled>,
}

/// Character placement in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterPlacementRaw {
    pub name: String,
    pub expression: Option<String>,
    pub position: Option<String>,
}

/// Character placement with interned strings.
#[derive(Clone, Debug, Serialize)]
pub struct CharacterPlacementCompiled {
    pub name: SharedStr,
    pub expression: Option<SharedStr>,
    pub position: Option<SharedStr>,
}

impl EventRaw {
    /// Serializes the raw event to JSON.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Serializes the raw event to a JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
}

impl EventCompiled {
    /// Serializes the compiled event to JSON.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Serializes the compiled event to a JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl EventRaw {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
        let dict = PyDict::new_bound(py);
        match self {
            EventRaw::Dialogue(dialogue) => {
                dict.set_item("type", "dialogue")?;
                dict.set_item("speaker", dialogue.speaker.as_str())?;
                dict.set_item("text", dialogue.text.as_str())?;
            }
            EventRaw::Choice(choice) => {
                dict.set_item("type", "choice")?;
                dict.set_item("prompt", choice.prompt.as_str())?;
                let options = PyList::empty_bound(py);
                for option in &choice.options {
                    let option_dict = PyDict::new_bound(py);
                    option_dict.set_item("text", option.text.as_str())?;
                    option_dict.set_item("target", option.target.as_str())?;
                    options.append(option_dict)?;
                }
                dict.set_item("options", options)?;
            }
            EventRaw::Scene(scene) => {
                dict.set_item("type", "scene")?;
                dict.set_item("background", scene.background.as_deref())?;
                dict.set_item("music", scene.music.as_deref())?;
                let characters = PyList::empty_bound(py);
                for character in &scene.characters {
                    let character_dict = PyDict::new_bound(py);
                    character_dict.set_item("name", character.name.as_str())?;
                    character_dict.set_item("expression", character.expression.as_deref())?;
                    character_dict.set_item("position", character.position.as_deref())?;
                    characters.append(character_dict)?;
                }
                dict.set_item("characters", characters)?;
            }
            EventRaw::Jump { target } => {
                dict.set_item("type", "jump")?;
                dict.set_item("target", target.as_str())?;
            }
            EventRaw::SetFlag { key, value } => {
                dict.set_item("type", "set_flag")?;
                dict.set_item("key", key.as_str())?;
                dict.set_item("value", *value)?;
            }
        }
        Ok(dict.into())
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl EventCompiled {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
        let dict = PyDict::new_bound(py);
        match self {
            EventCompiled::Dialogue(dialogue) => {
                dict.set_item("type", "dialogue")?;
                dict.set_item("speaker", dialogue.speaker.as_ref())?;
                dict.set_item("text", dialogue.text.as_ref())?;
            }
            EventCompiled::Choice(choice) => {
                dict.set_item("type", "choice")?;
                dict.set_item("prompt", choice.prompt.as_ref())?;
                let options = PyList::empty_bound(py);
                for option in &choice.options {
                    let option_dict = PyDict::new_bound(py);
                    option_dict.set_item("text", option.text.as_ref())?;
                    option_dict.set_item("target_ip", option.target_ip)?;
                    options.append(option_dict)?;
                }
                dict.set_item("options", options)?;
            }
            EventCompiled::Scene(scene) => {
                dict.set_item("type", "scene")?;
                dict.set_item("background", scene.background.as_deref())?;
                dict.set_item("music", scene.music.as_deref())?;
                let characters = PyList::empty_bound(py);
                for character in &scene.characters {
                    let character_dict = PyDict::new_bound(py);
                    character_dict.set_item("name", character.name.as_ref())?;
                    character_dict.set_item("expression", character.expression.as_deref())?;
                    character_dict.set_item("position", character.position.as_deref())?;
                    characters.append(character_dict)?;
                }
                dict.set_item("characters", characters)?;
            }
            EventCompiled::Jump { target_ip } => {
                dict.set_item("type", "jump")?;
                dict.set_item("target_ip", *target_ip)?;
            }
            EventCompiled::SetFlag { flag_id, value } => {
                dict.set_item("type", "set_flag")?;
                dict.set_item("flag_id", *flag_id)?;
                dict.set_item("value", *value)?;
            }
        }
        Ok(dict.into())
    }
}
