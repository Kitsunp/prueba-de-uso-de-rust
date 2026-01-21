use super::audio::PyAudio;
use super::conversion::{event_to_python, ui_state_to_python};
use super::types::{vn_error_to_py, PyResourceConfig};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
use visual_novel_engine::{
    AudioCommand, Engine as CoreEngine, EventCompiled, ResourceLimiter, ScriptRaw, SecurityPolicy,
    UiState,
};

#[pyclass(name = "Engine")]
#[derive(Debug)]
pub struct PyEngine {
    pub(crate) inner: CoreEngine,
    resource_limits: ResourceLimiter,
    max_texture_memory: usize,
    prefetch_depth: usize,
    handler: Option<Py<PyAny>>,
    last_audio_commands: Vec<AudioCommand>,
}

#[pyclass]
pub struct StepResult {
    #[pyo3(get)]
    pub event: PyObject,
    #[pyo3(get)]
    pub audio: PyObject,
}

#[pymethods]
impl PyEngine {
    #[new]
    pub fn new(script_json: &str) -> PyResult<Self> {
        let resource_limits = ResourceLimiter::default();
        let script = ScriptRaw::from_json_with_limits(script_json, resource_limits)
            .map_err(vn_error_to_py)?;
        let inner = CoreEngine::new(script, SecurityPolicy::default(), resource_limits)
            .map_err(vn_error_to_py)?;
        Ok(Self {
            inner,
            resource_limits,
            max_texture_memory: 512 * 1024 * 1024,
            prefetch_depth: 0,
            handler: None,
            last_audio_commands: Vec::new(),
        })
    }

    fn current_event<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.current_event().map_err(vn_error_to_py)?;
        event_to_python(&event, py)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<StepResult> {
        let (audio, change) = self.inner.step().map_err(vn_error_to_py)?;
        self.last_audio_commands = audio;
        let event = change.event;
        if let EventCompiled::ExtCall { command, args } = &event {
            if let Some(handler) = &self.handler {
                let handler = handler.clone_ref(py);
                // Catch exceptions from handler
                if let Err(e) = handler.call1(py, (command.as_str(), args.clone())) {
                    // Log or store the error, but don't fail the step
                    eprintln!("ExtCall handler error: {:?}", e);
                    // Or store in PyEngine for later retrieval
                }
            }
        }
        let event_obj = event_to_python(&event, py)?;
        let audio_obj = self.get_last_audio_commands(py)?;
        Ok(StepResult {
            event: event_obj,
            audio: audio_obj,
        })
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
        let dict = PyDict::new(py);
        dict.set_item("background", state.background.as_deref())?;
        dict.set_item("music", state.music.as_deref())?;
        let characters = PyList::empty(py);
        for character in &state.characters {
            let character_dict = PyDict::new(py);
            character_dict.set_item("name", character.name.as_ref())?;
            character_dict.set_item("expression", character.expression.as_deref())?;
            character_dict.set_item("position", character.position.as_deref())?;
            characters.append(character_dict)?;
        }
        dict.set_item("characters", characters)?;
        Ok(dict.into())
    }

    fn ui_state<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.current_event().map_err(vn_error_to_py)?;
        let ui = UiState::from_event(&event, self.inner.visual_state());
        ui_state_to_python(&ui, py)
    }

    fn get_last_audio_commands<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for cmd in &self.last_audio_commands {
            let dict = PyDict::new(py);
            match cmd {
                AudioCommand::PlayBgm {
                    resource,
                    path,
                    r#loop,
                    fade_in,
                } => {
                    dict.set_item("type", "play_bgm")?;
                    dict.set_item("resource", resource.as_u64().to_string())?;
                    dict.set_item("path", path.as_ref())?;
                    dict.set_item("loop", r#loop)?;
                    dict.set_item("fade_in", fade_in.as_secs_f64())?;
                }
                AudioCommand::StopBgm { fade_out } => {
                    dict.set_item("type", "stop_bgm")?;
                    dict.set_item("fade_out", fade_out.as_secs_f64())?;
                }
                AudioCommand::PlaySfx { resource, path } => {
                    dict.set_item("type", "play_sfx")?;
                    dict.set_item("resource", resource.as_u64().to_string())?;
                    dict.set_item("path", path.as_ref())?;
                }
            }
            list.append(dict)?;
        }
        Ok(list.into())
    }

    fn set_resources(&mut self, config: PyResourceConfig) {
        self.max_texture_memory = config.max_texture_memory;
        self.resource_limits.max_script_bytes = config.max_script_bytes;
    }

    fn get_memory_usage<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("current_texture_bytes", 0usize)?;
        dict.set_item("max_texture_memory", self.max_texture_memory)?;
        dict.set_item("max_script_bytes", self.resource_limits.max_script_bytes)?;
        Ok(dict.into())
    }

    fn set_prefetch_depth(&mut self, depth: usize) {
        self.prefetch_depth = depth;
    }

    fn is_loading(&self) -> bool {
        false
    }

    fn register_handler(&mut self, callback: Py<PyAny>) {
        self.handler = Some(callback);
    }

    fn resume(&mut self) -> PyResult<()> {
        self.inner.resume().map_err(vn_error_to_py)?;
        Ok(())
    }

    fn audio(slf: PyRef<'_, Self>) -> PyResult<Py<PyAudio>> {
        let py = slf.py();
        let engine: Py<PyEngine> = slf.into();
        Py::new(py, PyAudio::new(py, engine)?)
    }
}
