use std::collections::{BTreeMap, HashMap};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
use serde::Serialize;
use visual_novel_engine::{
    CharacterPlacement, Choice, ChoiceOption, Dialogue, Engine as CoreEngine, Event,
    ResourceLimiter, SceneUpdate, Script, SecurityPolicy, VnError,
};

fn vn_error_to_py(err: VnError) -> PyErr {
    let report = miette::Report::new(err);
    pyo3::exceptions::PyValueError::new_err(report.to_string())
}

#[pymodule]
fn visual_novel_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<PyScriptBuilder>()?;
    Ok(())
}

#[pyclass(name = "Engine")]
#[derive(Debug)]
pub struct PyEngine {
    inner: CoreEngine,
}

#[pymethods]
impl PyEngine {
    #[new]
    pub fn new(script_json: &str) -> PyResult<Self> {
        let script = Script::from_json(script_json).map_err(vn_error_to_py)?;
        let inner = CoreEngine::new(
            script,
            SecurityPolicy::default(),
            ResourceLimiter::default(),
        )
        .map_err(vn_error_to_py)?;
        Ok(Self { inner })
    }

    fn current_event<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.current_event().map_err(vn_error_to_py)?;
        event_to_python(&event, py)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.step().map_err(vn_error_to_py)?;
        event_to_python(&event, py)
    }

    fn choose<'py>(&mut self, py: Python<'py>, option_index: usize) -> PyResult<PyObject> {
        let event = self.inner.choose(option_index).map_err(vn_error_to_py)?;
        event_to_python(&event, py)
    }

    fn current_event_json(&self) -> PyResult<String> {
        self.inner.current_event_json().map_err(vn_error_to_py)
    }

    fn visual_state<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let state = self.inner.visual_state();
        let dict = PyDict::new_bound(py);
        dict.set_item("background", state.background.as_deref())?;
        dict.set_item("music", state.music.as_deref())?;
        let characters = PyList::empty_bound(py);
        for character in &state.characters {
            let character_dict = PyDict::new_bound(py);
            character_dict.set_item("name", character.name.as_str())?;
            character_dict.set_item("expression", character.expression.as_deref())?;
            character_dict.set_item("position", character.position.as_deref())?;
            characters.append(character_dict)?;
        }
        dict.set_item("characters", characters)?;
        Ok(dict.into())
    }
}

#[pyclass(name = "ScriptBuilder")]
pub struct PyScriptBuilder {
    events: Vec<Event>,
    labels: HashMap<String, usize>,
}

#[pymethods]
impl PyScriptBuilder {
    #[new]
    fn new() -> Self {
        Self {
            events: Vec::new(),
            labels: HashMap::new(),
        }
    }

    fn label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.events.len());
    }

    fn dialogue(&mut self, speaker: &str, text: &str) {
        self.events
            .push(Event::Dialogue(Dialogue {
                speaker: speaker.to_string(),
                text: text.to_string(),
            }));
    }

    fn choice(&mut self, prompt: &str, options: Vec<(String, String)>) {
        let options = options
            .into_iter()
            .map(|(text, target)| ChoiceOption { text, target })
            .collect();
        self.events.push(Event::Choice(Choice {
            prompt: prompt.to_string(),
            options,
        }));
    }

    fn scene(
        &mut self,
        background: Option<String>,
        music: Option<String>,
        characters: Vec<(String, Option<String>, Option<String>)>,
    ) {
        let characters = characters
            .into_iter()
            .map(|(name, expression, position)| CharacterPlacement {
                name,
                expression,
                position,
            })
            .collect();
        self.events.push(Event::Scene(SceneUpdate {
            background,
            music,
            characters,
        }));
    }

    fn jump(&mut self, target: &str) {
        self.events
            .push(Event::Jump { target: target.to_string() });
    }

    fn set_flag(&mut self, key: &str, value: bool) {
        self.events
            .push(Event::SetFlag { key: key.to_string(), value });
    }

    fn build_json(&self) -> PyResult<String> {
        let script = StableScript::from_parts(&self.events, &self.labels);
        serde_json::to_string(&script).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to serialize script: {err}"
            ))
        })
    }
}

#[derive(Serialize)]
struct StableScript {
    events: Vec<Event>,
    labels: BTreeMap<String, usize>,
}

impl StableScript {
    fn from_parts(events: &[Event], labels: &HashMap<String, usize>) -> Self {
        let mut ordered_labels = BTreeMap::new();
        for (key, value) in labels {
            ordered_labels.insert(key.clone(), *value);
        }
        Self {
            events: events.to_vec(),
            labels: ordered_labels,
        }
    }
}

fn event_to_python(event: &Event, py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    match event {
        Event::Dialogue(dialogue) => {
            dict.set_item("type", "dialogue")?;
            dict.set_item("speaker", dialogue.speaker.as_str())?;
            dict.set_item("text", dialogue.text.as_str())?;
        }
        Event::Choice(choice) => {
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
        Event::Scene(scene) => {
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
