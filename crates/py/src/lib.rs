use std::collections::{BTreeMap, HashMap};

use ::visual_novel_engine::{
    CharacterPlacementRaw, ChoiceOptionRaw, ChoiceRaw, DialogueRaw, Engine as CoreEngine,
    EventCompiled, EventRaw, ResourceLimiter, SceneUpdateRaw, ScriptRaw, SecurityPolicy, VnError,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
use serde::Serialize;

fn vn_error_to_py(err: VnError) -> PyErr {
    let report = miette::Report::new(err);
    pyo3::exceptions::PyValueError::new_err(report.to_string())
}

#[pymodule]
fn visual_novel_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<PyScriptBuilder>()?;
    m.add("PyEngine", m.getattr("Engine")?)?;
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
        let script = ScriptRaw::from_json(script_json).map_err(vn_error_to_py)?;
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
        let event = self.inner.step_event().map_err(vn_error_to_py)?;
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
            character_dict.set_item("name", character.name.as_ref())?;
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
    events: Vec<EventRaw>,
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
        self.events.push(EventRaw::Dialogue(DialogueRaw {
            speaker: speaker.to_string(),
            text: text.to_string(),
        }));
    }

    fn choice(&mut self, prompt: &str, options: Vec<(String, String)>) {
        let options = options
            .into_iter()
            .map(|(text, target)| ChoiceOptionRaw { text, target })
            .collect();
        self.events.push(EventRaw::Choice(ChoiceRaw {
            prompt: prompt.to_string(),
            options,
        }));
    }

    #[pyo3(signature = (background=None, music=None, characters=Vec::new()))]
    fn scene(
        &mut self,
        background: Option<String>,
        music: Option<String>,
        characters: Vec<(String, Option<String>, Option<String>)>,
    ) {
        let characters = characters
            .into_iter()
            .map(|(name, expression, position)| CharacterPlacementRaw {
                name,
                expression,
                position,
            })
            .collect();
        self.events.push(EventRaw::Scene(SceneUpdateRaw {
            background,
            music,
            characters,
        }));
    }

    fn jump(&mut self, target: &str) {
        self.events.push(EventRaw::Jump {
            target: target.to_string(),
        });
    }

    fn set_flag(&mut self, key: &str, value: bool) {
        self.events.push(EventRaw::SetFlag {
            key: key.to_string(),
            value,
        });
    }

    fn build_json(&self) -> PyResult<String> {
        let script = StableScript::from_parts(&self.events, &self.labels);
        serde_json::to_string(&script).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to serialize script: {err}"))
        })
    }
}

#[derive(Serialize)]
struct StableScript {
    events: Vec<EventRaw>,
    labels: BTreeMap<String, usize>,
}

impl StableScript {
    fn from_parts(events: &[EventRaw], labels: &HashMap<String, usize>) -> Self {
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

fn event_to_python(event: &EventCompiled, py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    match event {
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
