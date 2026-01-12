use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Dialogue(Dialogue),
    Choice(Choice),
    Scene(SceneUpdate),
    Jump { target: String },
    SetFlag { key: String, value: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dialogue {
    pub speaker: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub prompt: String,
    pub options: Vec<ChoiceOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub text: String,
    pub target: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneUpdate {
    pub background: Option<String>,
    pub music: Option<String>,
    pub characters: Vec<CharacterPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterPlacement {
    pub name: String,
    pub expression: Option<String>,
    pub position: Option<String>,
}

impl Event {
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::Value::Null)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
}

#[cfg(feature = "python")]
impl Event {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::types::{PyDict, PyList};
        let dict = PyDict::new(py);
        match self {
            Event::Dialogue(dialogue) => {
                dict.set_item("type", "dialogue")?;
                dict.set_item("speaker", dialogue.speaker.as_str())?;
                dict.set_item("text", dialogue.text.as_str())?;
            }
            Event::Choice(choice) => {
                dict.set_item("type", "choice")?;
                dict.set_item("prompt", choice.prompt.as_str())?;
                let options = PyList::empty(py);
                for option in &choice.options {
                    let option_dict = PyDict::new(py);
                    option_dict.set_item("text", option.text.as_str())?;
                    option_dict.set_item("target", option.target.as_str())?;
                    options.append(option_dict)?;
                }
                dict.set_item("options", options)?;
            }
            Event::Scene(scene) => {
                dict.set_item("type", "scene")?;
                dict.set_item("background", scene.background.as_deref())?;
                dict.set_item("music", scene.music.as_deref())?;
                let characters = PyList::empty(py);
                for character in &scene.characters {
                    let character_dict = PyDict::new(py);
                    character_dict.set_item("name", character.name.as_str())?;
                    character_dict.set_item("expression", character.expression.as_deref())?;
                    character_dict.set_item("position", character.position.as_deref())?;
                    characters.append(character_dict)?;
                }
                dict.set_item("characters", characters)?;
            }
            Event::Jump { target } => {
                dict.set_item("type", "jump")?;
                dict.set_item("target", target.as_str())?;
            }
            Event::SetFlag { key, value } => {
                dict.set_item("type", "set_flag")?;
                dict.set_item("key", key.as_str())?;
                dict.set_item("value", *value)?;
            }
        }
        Ok(dict.into())
    }
}
