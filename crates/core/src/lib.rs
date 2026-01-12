mod engine;
mod error;
mod event;
mod render;
mod resource;
mod script;
mod security;
mod state;
mod visual;

pub use engine::Engine;
pub use error::{VnError, VnResult};
pub use event::{
    CharacterPlacementCompiled, CharacterPlacementRaw, ChoiceCompiled, ChoiceOptionCompiled,
    ChoiceOptionRaw, ChoiceRaw, DialogueCompiled, DialogueRaw, EventCompiled, EventRaw,
    SceneUpdateCompiled, SceneUpdateRaw, SharedStr,
};
pub use render::{RenderBackend, RenderOutput, TextRenderer};
pub use resource::ResourceLimiter;
pub use script::{ScriptCompiled, ScriptRaw};
pub use security::SecurityPolicy;
pub use state::EngineState;
pub use visual::VisualState;

pub type Event = EventCompiled;
pub type Script = ScriptRaw;

#[cfg(any(feature = "python", feature = "python-embed"))]
use pyo3::prelude::*;

#[cfg(any(feature = "python", feature = "python-embed"))]
fn vn_error_to_py(err: VnError) -> pyo3::PyErr {
    let report = miette::Report::new(err);
    pyo3::exceptions::PyValueError::new_err(report.to_string())
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[pymodule]
fn visual_novel_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<crate::event::PyEvent>()?;
    Ok(())
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[pyclass]
#[derive(Debug)]
pub struct PyEngine {
    inner: Engine,
}

#[cfg(any(feature = "python", feature = "python-embed"))]
#[pymethods]
impl PyEngine {
    #[new]
    pub fn new(script_json: &str) -> PyResult<Self> {
        let script = Script::from_json(script_json).map_err(vn_error_to_py)?;
        let inner = Engine::new(
            script,
            SecurityPolicy::default(),
            ResourceLimiter::default(),
        )
        .map_err(vn_error_to_py)?;
        Ok(Self { inner })
    }

    fn current_event<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.current_event().map_err(vn_error_to_py)?;
        event.to_python(py)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.step_event().map_err(vn_error_to_py)?;
        event.to_python(py)
    }

    fn choose<'py>(&mut self, py: Python<'py>, option_index: usize) -> PyResult<PyObject> {
        let event = self.inner.choose(option_index).map_err(vn_error_to_py)?;
        event.to_python(py)
    }

    fn current_event_json(&self) -> PyResult<String> {
        self.inner.current_event_json().map_err(vn_error_to_py)
    }

    fn visual_state<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
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

#[cfg(any(feature = "python", feature = "python-embed"))]
impl PyEngine {
    pub fn new_from_json(script_json: &str) -> VnResult<Self> {
        let script = Script::from_json(script_json)?;
        let inner = Engine::new(
            script,
            SecurityPolicy::default(),
            ResourceLimiter::default(),
        )?;
        Ok(Self { inner })
    }
}
