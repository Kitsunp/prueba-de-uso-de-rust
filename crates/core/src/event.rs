//! Event definitions for raw and compiled scripts.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(any(feature = "python", feature = "python-embed"))]
use pyo3::{prelude::PyAnyMethods, IntoPy};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
        pyo3::Py::new(py, PyEvent::from_raw(self.clone())).map(|event| event.into_py(py))
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl EventCompiled {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        pyo3::Py::new(py, PyEvent::from_compiled(self.clone()))
            .map(|event| event.into_py(py))
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[derive(Clone, Debug)]
enum PyEventData {
    Raw(EventRaw),
    Compiled(EventCompiled),
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[pyo3::pyclass]
#[derive(Debug)]
pub struct PyEvent {
    data: PyEventData,
    cached_dict: std::cell::RefCell<Option<pyo3::PyObject>>,
    cached_options: std::cell::RefCell<Option<pyo3::PyObject>>,
    cached_characters: std::cell::RefCell<Option<pyo3::PyObject>>,
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl PyEvent {
    fn from_raw(event: EventRaw) -> Self {
        Self {
            data: PyEventData::Raw(event),
            cached_dict: std::cell::RefCell::new(None),
            cached_options: std::cell::RefCell::new(None),
            cached_characters: std::cell::RefCell::new(None),
        }
    }

    fn from_compiled(event: EventCompiled) -> Self {
        Self {
            data: PyEventData::Compiled(event),
            cached_dict: std::cell::RefCell::new(None),
            cached_options: std::cell::RefCell::new(None),
            cached_characters: std::cell::RefCell::new(None),
        }
    }

    fn event_type(&self) -> &'static str {
        match &self.data {
            PyEventData::Raw(event) => match event {
                EventRaw::Dialogue(_) => "dialogue",
                EventRaw::Choice(_) => "choice",
                EventRaw::Scene(_) => "scene",
                EventRaw::Jump { .. } => "jump",
                EventRaw::SetFlag { .. } => "set_flag",
            },
            PyEventData::Compiled(event) => match event {
                EventCompiled::Dialogue(_) => "dialogue",
                EventCompiled::Choice(_) => "choice",
                EventCompiled::Scene(_) => "scene",
                EventCompiled::Jump { .. } => "jump",
                EventCompiled::SetFlag { .. } => "set_flag",
            },
        }
    }

    fn build_dict(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::types::{PyDict, PyDictMethods};
        let dict = PyDict::new_bound(py);
        dict.set_item("type", self.event_type())?;
        if let Some(value) = self.speaker_value() {
            dict.set_item("speaker", value)?;
        }
        if let Some(value) = self.text_value() {
            dict.set_item("text", value)?;
        }
        if let Some(value) = self.prompt_value() {
            dict.set_item("prompt", value)?;
        }
        if let Some(options) = self.options_value(py)? {
            dict.set_item("options", options)?;
        }
        if let Some(value) = self.background_value() {
            dict.set_item("background", value)?;
        }
        if let Some(value) = self.music_value() {
            dict.set_item("music", value)?;
        }
        if let Some(characters) = self.characters_value(py)? {
            dict.set_item("characters", characters)?;
        }
        if let Some(value) = self.target_value(py)? {
            dict.set_item("target", value)?;
        }
        if let Some(value) = self.target_ip_value() {
            dict.set_item("target_ip", value)?;
        }
        if let Some(value) = self.key_value(py)? {
            dict.set_item("key", value)?;
        }
        if let Some(value) = self.flag_id_value() {
            dict.set_item("flag_id", value)?;
        }
        if let Some(value) = self.value_flag() {
            dict.set_item("value", value)?;
        }
        Ok(dict.into())
    }

    fn speaker_value(&self) -> Option<&str> {
        match &self.data {
            PyEventData::Raw(EventRaw::Dialogue(dialogue)) => Some(dialogue.speaker.as_str()),
            PyEventData::Compiled(EventCompiled::Dialogue(dialogue)) => {
                Some(dialogue.speaker.as_ref())
            }
            _ => None,
        }
    }

    fn text_value(&self) -> Option<&str> {
        match &self.data {
            PyEventData::Raw(EventRaw::Dialogue(dialogue)) => Some(dialogue.text.as_str()),
            PyEventData::Compiled(EventCompiled::Dialogue(dialogue)) => Some(dialogue.text.as_ref()),
            _ => None,
        }
    }

    fn prompt_value(&self) -> Option<&str> {
        match &self.data {
            PyEventData::Raw(EventRaw::Choice(choice)) => Some(choice.prompt.as_str()),
            PyEventData::Compiled(EventCompiled::Choice(choice)) => Some(choice.prompt.as_ref()),
            _ => None,
        }
    }

    fn background_value(&self) -> Option<&str> {
        match &self.data {
            PyEventData::Raw(EventRaw::Scene(scene)) => scene.background.as_deref(),
            PyEventData::Compiled(EventCompiled::Scene(scene)) => scene.background.as_deref(),
            _ => None,
        }
    }

    fn music_value(&self) -> Option<&str> {
        match &self.data {
            PyEventData::Raw(EventRaw::Scene(scene)) => scene.music.as_deref(),
            PyEventData::Compiled(EventCompiled::Scene(scene)) => scene.music.as_deref(),
            _ => None,
        }
    }

    fn target_ip_value(&self) -> Option<u32> {
        match &self.data {
            PyEventData::Compiled(EventCompiled::Jump { target_ip }) => Some(*target_ip),
            _ => None,
        }
    }

    fn flag_id_value(&self) -> Option<u32> {
        match &self.data {
            PyEventData::Compiled(EventCompiled::SetFlag { flag_id, .. }) => Some(*flag_id),
            _ => None,
        }
    }

    fn value_flag(&self) -> Option<bool> {
        match &self.data {
            PyEventData::Raw(EventRaw::SetFlag { value, .. }) => Some(*value),
            PyEventData::Compiled(EventCompiled::SetFlag { value, .. }) => Some(*value),
            _ => None,
        }
    }

    fn target_value(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        use pyo3::IntoPy;
        match &self.data {
            PyEventData::Raw(EventRaw::Jump { target }) => Ok(Some(target.as_str().into_py(py))),
            PyEventData::Compiled(EventCompiled::Jump { target_ip }) => {
                Ok(Some(target_ip.into_py(py)))
            }
            _ => Ok(None),
        }
    }

    fn key_value(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        use pyo3::IntoPy;
        match &self.data {
            PyEventData::Raw(EventRaw::SetFlag { key, .. }) => Ok(Some(key.as_str().into_py(py))),
            PyEventData::Compiled(EventCompiled::SetFlag { flag_id, .. }) => {
                Ok(Some(flag_id.into_py(py)))
            }
            _ => Ok(None),
        }
    }

    fn options_value(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        if self.cached_options.borrow().is_some() {
            return Ok(self.cached_options.borrow().clone());
        }
        let list = match &self.data {
            PyEventData::Raw(EventRaw::Choice(choice)) => {
                use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
                let options = PyList::empty_bound(py);
                for option in &choice.options {
                    let option_dict = PyDict::new_bound(py);
                    option_dict.set_item("text", option.text.as_str())?;
                    option_dict.set_item("target", option.target.as_str())?;
                    options.append(option_dict)?;
                }
                Some(options.into())
            }
            PyEventData::Compiled(EventCompiled::Choice(choice)) => {
                use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
                let options = PyList::empty_bound(py);
                for option in &choice.options {
                    let option_dict = PyDict::new_bound(py);
                    option_dict.set_item("text", option.text.as_ref())?;
                    option_dict.set_item("target", option.target_ip)?;
                    option_dict.set_item("target_ip", option.target_ip)?;
                    options.append(option_dict)?;
                }
                Some(options.into())
            }
            _ => None,
        };
        if list.is_some() {
            *self.cached_options.borrow_mut() = list.clone();
        }
        Ok(list)
    }

    fn characters_value(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        if self.cached_characters.borrow().is_some() {
            return Ok(self.cached_characters.borrow().clone());
        }
        let list = match &self.data {
            PyEventData::Raw(EventRaw::Scene(scene)) => {
                use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
                let characters = PyList::empty_bound(py);
                for character in &scene.characters {
                    let character_dict = PyDict::new_bound(py);
                    character_dict.set_item("name", character.name.as_str())?;
                    character_dict.set_item("expression", character.expression.as_deref())?;
                    character_dict.set_item("position", character.position.as_deref())?;
                    characters.append(character_dict)?;
                }
                Some(characters.into())
            }
            PyEventData::Compiled(EventCompiled::Scene(scene)) => {
                use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
                let characters = PyList::empty_bound(py);
                for character in &scene.characters {
                    let character_dict = PyDict::new_bound(py);
                    character_dict.set_item("name", character.name.as_ref())?;
                    character_dict.set_item("expression", character.expression.as_deref())?;
                    character_dict.set_item("position", character.position.as_deref())?;
                    characters.append(character_dict)?;
                }
                Some(characters.into())
            }
            _ => None,
        };
        if list.is_some() {
            *self.cached_characters.borrow_mut() = list.clone();
        }
        Ok(list)
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[pyo3::pymethods]
impl PyEvent {
    #[getter]
    fn r#type(&self) -> &str {
        self.event_type()
    }

    #[getter]
    fn speaker(&self) -> Option<&str> {
        self.speaker_value()
    }

    #[getter]
    fn text(&self) -> Option<&str> {
        self.text_value()
    }

    #[getter]
    fn prompt(&self) -> Option<&str> {
        self.prompt_value()
    }

    #[getter]
    fn background(&self) -> Option<&str> {
        self.background_value()
    }

    #[getter]
    fn music(&self) -> Option<&str> {
        self.music_value()
    }

    #[getter]
    fn target(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        self.target_value(py)
    }

    #[getter]
    fn target_ip(&self) -> Option<u32> {
        self.target_ip_value()
    }

    #[getter]
    fn key(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        self.key_value(py)
    }

    #[getter]
    fn flag_id(&self) -> Option<u32> {
        self.flag_id_value()
    }

    #[getter]
    fn value(&self) -> Option<bool> {
        self.value_flag()
    }

    #[getter]
    fn options(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        self.options_value(py)
    }

    #[getter]
    fn characters(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        self.characters_value(py)
    }

    #[getter]
    fn as_dict(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        self.to_dict(py)
    }

    fn __getitem__(
        &self,
        py: pyo3::Python<'_>,
        key: &pyo3::Bound<'_, pyo3::PyAny>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::types::PyDictMethods;
        let dict = self.to_dict(py)?;
        let dict = dict.bind(py).downcast::<pyo3::types::PyDict>()?;
        match dict.get_item(key)? {
            Some(value) => Ok(value.into_py(py)),
            None => Err(pyo3::exceptions::PyKeyError::new_err("missing key")),
        }
    }

    fn to_dict(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        if let Some(cached) = self.cached_dict.borrow().clone() {
            return Ok(cached);
        }
        let dict = self.build_dict(py)?;
        *self.cached_dict.borrow_mut() = Some(dict.clone_ref(py));
        Ok(dict)
    }
}
