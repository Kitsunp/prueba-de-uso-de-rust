use std::collections::BTreeMap;

use ::visual_novel_engine::{
    AssetId, AudioCommand, CharacterPatchCompiled, CharacterPatchRaw, CharacterPlacementCompiled,
    CharacterPlacementRaw, ChoiceOptionRaw, ChoiceRaw, CmpOp, CondRaw, DialogueRaw,
    Engine as CoreEngine, EventCompiled, EventRaw, ResourceLimiter, ScenePatchRaw,
    SceneUpdateRaw, ScriptRaw, SecurityPolicy, SharedStr, UiState, UiView, VnError,
    SCRIPT_SCHEMA_VERSION,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyDictMethods, PyList, PyListMethods};
use serde::Serialize;
use visual_novel_gui::{run_app as run_gui, GuiError, SecurityMode, VnConfig as GuiConfig};
use std::time::Duration;

fn vn_error_to_py(err: VnError) -> PyErr {
    let report = miette::Report::new(err);
    pyo3::exceptions::PyValueError::new_err(report.to_string())
}

#[pymodule]
fn visual_novel_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<PyAudio>()?;
    m.add_class::<PyResourceConfig>()?;
    m.add_class::<PyScriptBuilder>()?;
    m.add_class::<PyVnConfig>()?;
    m.add_function(wrap_pyfunction!(run_visual_novel, m)?)?;
    m.add("PyEngine", m.getattr("Engine")?)?;
    Ok(())
}

#[pyclass(name = "ResourceConfig")]
#[derive(Clone, Debug)]
pub struct PyResourceConfig {
    #[pyo3(get, set)]
    pub max_texture_memory: usize,
    #[pyo3(get, set)]
    pub max_script_bytes: usize,
}

#[pymethods]
impl PyResourceConfig {
    #[new]
    #[pyo3(signature = (max_texture_memory=None, max_script_bytes=None))]
    fn new(max_texture_memory: Option<usize>, max_script_bytes: Option<usize>) -> Self {
        Self {
            max_texture_memory: max_texture_memory.unwrap_or(512 * 1024 * 1024),
            max_script_bytes: max_script_bytes.unwrap_or(ResourceLimiter::default().max_script_bytes),
        }
    }
}

#[pyclass(name = "Engine")]
#[derive(Debug)]
pub struct PyEngine {
    inner: CoreEngine,
    resource_limits: ResourceLimiter,
    max_texture_memory: usize,
    prefetch_depth: usize,
    handler: Option<Py<PyAny>>,
}

#[pymethods]
impl PyEngine {
    #[new]
    pub fn new(script_json: &str) -> PyResult<Self> {
        let resource_limits = ResourceLimiter::default();
        let script =
            ScriptRaw::from_json_with_limits(script_json, resource_limits).map_err(vn_error_to_py)?;
        let inner = CoreEngine::new(
            script,
            SecurityPolicy::default(),
            resource_limits,
        )
        .map_err(vn_error_to_py)?;
        Ok(Self {
            inner,
            resource_limits,
            max_texture_memory: 512 * 1024 * 1024,
            prefetch_depth: 0,
            handler: None,
        })
    }

    fn current_event<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let event = self.inner.current_event().map_err(vn_error_to_py)?;
        event_to_python(&event, py)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<PyObject> {
        let (_audio, change) = self.inner.step().map_err(vn_error_to_py)?;
        let event = change.event;
        if let EventCompiled::ExtCall { command, args } = &event {
            if let Some(handler) = &self.handler {
                let handler = handler.clone_ref(py);
                handler.call1(py, (command.as_str(), args.clone()))?;
            }
        }
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
        Py::new(py, PyAudio::new(py, slf)?)
    }
}

#[pyclass(name = "AudioController")]
pub struct PyAudio {
    engine: Py<PyAny>,
}

impl PyAudio {
    fn new(py: Python<'_>, engine: PyRef<'_, PyEngine>) -> PyResult<Self> {
        Ok(Self {
            engine: engine.into_py(py),
        })
    }
}

#[pymethods]
impl PyAudio {
    #[pyo3(signature = (resource, r#loop=true, fade_in=0.0))]
    fn play_bgm(&self, py: Python<'_>, resource: &str, r#loop: bool, fade_in: f64) -> PyResult<()> {
        let mut engine: PyRefMut<'_, PyEngine> = self.engine.bind(py).extract()?;
        engine.inner.queue_audio_command(AudioCommand::PlayBgm {
            resource: AssetId::from_path(resource),
            r#loop,
            fade_in: Duration::from_secs_f64(fade_in.max(0.0)),
        });
        Ok(())
    }

    #[pyo3(signature = (fade_out=0.0))]
    fn stop_all(&self, py: Python<'_>, fade_out: f64) -> PyResult<()> {
        let mut engine: PyRefMut<'_, PyEngine> = self.engine.bind(py).extract()?;
        engine.inner.queue_audio_command(AudioCommand::StopBgm {
            fade_out: Duration::from_secs_f64(fade_out.max(0.0)),
        });
        Ok(())
    }

    fn play_sfx(&self, py: Python<'_>, resource: &str) -> PyResult<()> {
        let mut engine: PyRefMut<'_, PyEngine> = self.engine.bind(py).extract()?;
        engine.inner.queue_audio_command(AudioCommand::PlaySfx {
            resource: AssetId::from_path(resource),
        });
        Ok(())
    }
}

#[pyclass(name = "ScriptBuilder")]
pub struct PyScriptBuilder {
    events: Vec<EventRaw>,
    labels: BTreeMap<String, usize>,
}

#[pyclass(name = "VnConfig")]
#[derive(Clone, Debug)]
pub struct PyVnConfig {
    #[pyo3(get, set)]
    pub title: Option<String>,
    #[pyo3(get, set)]
    pub width: Option<f32>,
    #[pyo3(get, set)]
    pub height: Option<f32>,
    #[pyo3(get, set)]
    pub fullscreen: Option<bool>,
    #[pyo3(get, set)]
    pub scale_factor: Option<f32>,
    #[pyo3(get, set)]
    pub assets_root: Option<String>,
    #[pyo3(get, set)]
    pub asset_cache_budget_mb: Option<u64>,
    #[pyo3(get, set)]
    pub security_mode: Option<String>,
    #[pyo3(get, set)]
    pub manifest_path: Option<String>,
    #[pyo3(get, set)]
    pub require_manifest: Option<bool>,
}

#[pymethods]
impl PyVnConfig {
    #[new]
    #[pyo3(signature = (title=None, width=None, height=None, fullscreen=None, scale_factor=None, assets_root=None, asset_cache_budget_mb=None, security_mode=None, manifest_path=None, require_manifest=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        title: Option<String>,
        width: Option<f32>,
        height: Option<f32>,
        fullscreen: Option<bool>,
        scale_factor: Option<f32>,
        assets_root: Option<String>,
        asset_cache_budget_mb: Option<u64>,
        security_mode: Option<String>,
        manifest_path: Option<String>,
        require_manifest: Option<bool>,
    ) -> Self {
        Self {
            title,
            width,
            height,
            fullscreen,
            scale_factor,
            assets_root,
            asset_cache_budget_mb,
            security_mode,
            manifest_path,
            require_manifest,
        }
    }
}

impl From<PyVnConfig> for GuiConfig {
    fn from(config: PyVnConfig) -> Self {
        let mut base = GuiConfig::default();
        if let Some(title) = config.title {
            base.title = title;
        }
        if let Some(width) = config.width {
            base.width = Some(width);
        }
        if let Some(height) = config.height {
            base.height = Some(height);
        }
        if let Some(fullscreen) = config.fullscreen {
            base.fullscreen = fullscreen;
        }
        if let Some(scale_factor) = config.scale_factor {
            base.scale_factor = Some(scale_factor);
        }
        if let Some(assets_root) = config.assets_root {
            base.assets_root = Some(assets_root.into());
        }
        if let Some(budget) = config.asset_cache_budget_mb {
            base.asset_cache_budget_mb = Some(budget);
        }
        if let Some(security_mode) = config.security_mode {
            base.security_mode = parse_security_mode(&security_mode);
        }
        if let Some(manifest_path) = config.manifest_path {
            base.manifest_path = Some(manifest_path.into());
        }
        if let Some(require_manifest) = config.require_manifest {
            base.require_manifest = Some(require_manifest);
        }
        base
    }
}

#[pyfunction]
fn run_visual_novel(script_json: String, config: Option<PyVnConfig>) -> PyResult<()> {
    let gui_config = config.map(Into::into);
    run_gui(script_json, gui_config).map_err(|err| match err {
        GuiError::Script(script) => pyo3::exceptions::PyValueError::new_err(script.to_string()),
        _ => pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to run GUI: {err}")),
    })
}

fn parse_security_mode(mode: &str) -> SecurityMode {
    match mode {
        "untrusted" => SecurityMode::Untrusted,
        _ => SecurityMode::Trusted,
    }
}

#[pymethods]
impl PyScriptBuilder {
    #[new]
    fn new() -> Self {
        Self {
            events: Vec::new(),
            labels: BTreeMap::new(),
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

    fn set_var(&mut self, key: &str, value: i32) {
        self.events.push(EventRaw::SetVar {
            key: key.to_string(),
            value,
        });
    }

    fn jump_if_flag(&mut self, key: &str, is_set: bool, target: &str) {
        self.events.push(EventRaw::JumpIf {
            cond: CondRaw::Flag {
                key: key.to_string(),
                is_set,
            },
            target: target.to_string(),
        });
    }

    fn jump_if_var(&mut self, key: &str, op: &str, value: i32, target: &str) -> PyResult<()> {
        let op = parse_cmp_op(op)?;
        self.events.push(EventRaw::JumpIf {
            cond: CondRaw::VarCmp {
                key: key.to_string(),
                op,
                value,
            },
            target: target.to_string(),
        });
        Ok(())
    }

    #[pyo3(signature = (background=None, music=None, add=Vec::new(), update=Vec::new(), remove=Vec::new()))]
    fn patch(
        &mut self,
        background: Option<String>,
        music: Option<String>,
        add: Vec<(String, Option<String>, Option<String>)>,
        update: Vec<(String, Option<String>, Option<String>)>,
        remove: Vec<String>,
    ) {
        let add = add
            .into_iter()
            .map(|(name, expression, position)| CharacterPlacementRaw {
                name,
                expression,
                position,
            })
            .collect();
        let update = update
            .into_iter()
            .map(|(name, expression, position)| CharacterPatchRaw {
                name,
                expression,
                position,
            })
            .collect();
        self.events.push(EventRaw::Patch(ScenePatchRaw {
            background,
            music,
            add,
            update,
            remove,
        }));
    }

    fn ext_call(&mut self, command: &str, args: Vec<String>) {
        self.events.push(EventRaw::ExtCall {
            command: command.to_string(),
            args,
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
    script_schema_version: String,
    events: Vec<EventRaw>,
    labels: BTreeMap<String, usize>,
}

impl StableScript {
    fn from_parts(events: &[EventRaw], labels: &BTreeMap<String, usize>) -> Self {
        Self {
            script_schema_version: SCRIPT_SCHEMA_VERSION.to_string(),
            events: events.to_vec(),
            labels: labels.clone(),
        }
    }
}

fn event_to_python(event: &EventCompiled, py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    match event {
        EventCompiled::Dialogue(dialogue) => {
            dict.set_item("type", "dialogue")?;
            dict.set_item("speaker", dialogue.speaker.as_ref())?;
            dict.set_item("text", dialogue.text.as_ref())?;
        }
        EventCompiled::Choice(choice) => {
            dict.set_item("type", "choice")?;
            dict.set_item("prompt", choice.prompt.as_ref())?;
            let options = PyList::empty(py);
            for option in &choice.options {
                let option_dict = PyDict::new(py);
                option_dict.set_item("text", option.text.as_ref())?;
                option_dict.set_item("target", option.target_ip)?;
                option_dict.set_item("target_ip", option.target_ip)?;
                options.append(option_dict)?;
            }
            dict.set_item("options", options)?;
        }
        EventCompiled::Scene(scene) => {
            dict.set_item("type", "scene")?;
            dict.set_item("background", scene.background.as_deref())?;
            dict.set_item("music", scene.music.as_deref())?;
            let characters = PyList::empty(py);
            for character in &scene.characters {
                let character_dict = PyDict::new(py);
                character_dict.set_item("name", character.name.as_ref())?;
                character_dict.set_item("expression", character.expression.as_deref())?;
                character_dict.set_item("position", character.position.as_deref())?;
                characters.append(character_dict)?;
            }
            dict.set_item("characters", characters)?;
        }
        EventCompiled::Jump { target_ip } => {
            dict.set_item("type", "jump")?;
            dict.set_item("target", *target_ip)?;
            dict.set_item("target_ip", *target_ip)?;
        }
        EventCompiled::SetFlag { flag_id, value } => {
            dict.set_item("type", "set_flag")?;
            dict.set_item("key", *flag_id)?;
            dict.set_item("flag_id", *flag_id)?;
            dict.set_item("value", *value)?;
        }
        EventCompiled::SetVar { var_id, value } => {
            dict.set_item("type", "set_var")?;
            dict.set_item("var_id", *var_id)?;
            dict.set_item("value", *value)?;
        }
        EventCompiled::JumpIf { target_ip, .. } => {
            dict.set_item("type", "jump_if")?;
            dict.set_item("target_ip", *target_ip)?;
        }
        EventCompiled::Patch(patch) => {
            dict.set_item("type", "patch")?;
            dict.set_item("background", patch.background.as_deref())?;
            dict.set_item("music", patch.music.as_deref())?;
            dict.set_item("add", characters_to_python(py, &patch.add)?)?;
            dict.set_item("update", patch_update_to_python(py, &patch.update)?)?;
            dict.set_item("remove", string_list_to_python(py, &patch.remove)?)?;
        }
        EventCompiled::ExtCall { command, args } => {
            dict.set_item("type", "ext_call")?;
            dict.set_item("command", command)?;
            let list = PyList::empty(py);
            for arg in args {
                list.append(arg)?;
            }
            dict.set_item("args", list)?;
        }
    }
    Ok(dict.into())
}

fn parse_cmp_op(op: &str) -> PyResult<CmpOp> {
    match op {
        "eq" => Ok(CmpOp::Eq),
        "ne" => Ok(CmpOp::Ne),
        "lt" => Ok(CmpOp::Lt),
        "le" => Ok(CmpOp::Le),
        "gt" => Ok(CmpOp::Gt),
        "ge" => Ok(CmpOp::Ge),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown comparison op '{op}'"
        ))),
    }
}

fn characters_to_python(
    py: Python<'_>,
    characters: &[CharacterPlacementCompiled],
) -> PyResult<PyObject> {
    let list = PyList::empty(py);
    for character in characters {
        let character_dict = PyDict::new(py);
        character_dict.set_item("name", character.name.as_ref())?;
        character_dict.set_item("expression", character.expression.as_deref())?;
        character_dict.set_item("position", character.position.as_deref())?;
        list.append(character_dict)?;
    }
    Ok(list.into())
}

fn patch_update_to_python(py: Python<'_>, update: &[CharacterPatchCompiled]) -> PyResult<PyObject> {
    let list = PyList::empty(py);
    for character in update {
        let character_dict = PyDict::new(py);
        character_dict.set_item("name", character.name.as_ref())?;
        character_dict.set_item("expression", character.expression.as_deref())?;
        character_dict.set_item("position", character.position.as_deref())?;
        list.append(character_dict)?;
    }
    Ok(list.into())
}

fn string_list_to_python(py: Python<'_>, items: &[SharedStr]) -> PyResult<PyObject> {
    let list = PyList::empty(py);
    for item in items {
        list.append(item.as_ref())?;
    }
    Ok(list.into())
}

fn ui_state_to_python(ui: &UiState, py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    match &ui.view {
        UiView::Dialogue { speaker, text } => {
            dict.set_item("type", "dialogue")?;
            dict.set_item("speaker", speaker)?;
            dict.set_item("text", text)?;
        }
        UiView::Choice { prompt, options } => {
            dict.set_item("type", "choice")?;
            dict.set_item("prompt", prompt)?;
            let list = PyList::empty(py);
            for option in options {
                list.append(option)?;
            }
            dict.set_item("options", list)?;
        }
        UiView::Scene { description } => {
            dict.set_item("type", "scene")?;
            dict.set_item("description", description)?;
        }
        UiView::System { message } => {
            dict.set_item("type", "system")?;
            dict.set_item("message", message)?;
        }
    }
    Ok(dict.into())
}
