use pyo3::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use visual_novel_engine::{
    CharacterPatchRaw, CharacterPlacementRaw, ChoiceOptionRaw, ChoiceRaw, CmpOp, CondRaw,
    DialogueRaw, EventRaw, ScenePatchRaw, SceneUpdateRaw, SCRIPT_SCHEMA_VERSION,
};

#[pyclass(name = "ScriptBuilder")]
pub struct PyScriptBuilder {
    events: Vec<EventRaw>,
    labels: BTreeMap<String, usize>,
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
                x: None,
                y: None,
                scale: None,
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
                x: None,
                y: None,
                scale: None,
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
