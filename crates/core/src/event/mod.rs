//! Event definitions for raw and compiled scripts.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub mod branching;
pub mod choice;
pub mod dialogue;
pub mod scene;

#[cfg(any(feature = "python", feature = "python-embed"))]
mod python_bridge;
#[cfg(any(feature = "python", feature = "python-embed"))]
mod python_bridge_helpers;

pub use branching::{CmpOp, CondCompiled, CondRaw};
pub use choice::{ChoiceCompiled, ChoiceOptionCompiled, ChoiceOptionRaw, ChoiceRaw};
pub use dialogue::{DialogueCompiled, DialogueRaw};
pub use scene::{
    CharacterPatchCompiled, CharacterPatchRaw, CharacterPlacementCompiled, CharacterPlacementRaw,
    ScenePatchCompiled, ScenePatchRaw, SceneUpdateCompiled, SceneUpdateRaw,
};

#[cfg(any(feature = "python", feature = "python-embed"))]
pub use python_bridge::PyEvent;

/// Shared string storage used by compiled events.
pub type SharedStr = Arc<str>;

/// JSON-facing events used in `ScriptRaw`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventRaw {
    Dialogue(DialogueRaw),
    Choice(ChoiceRaw),
    Scene(SceneUpdateRaw),
    Jump { target: String },
    SetFlag { key: String, value: bool },
    SetVar { key: String, value: i32 },
    JumpIf { cond: CondRaw, target: String },
    Patch(ScenePatchRaw),
    ExtCall { command: String, args: Vec<String> },
}

/// Runtime events with pre-resolved targets and interned strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventCompiled {
    Dialogue(DialogueCompiled),
    Choice(ChoiceCompiled),
    Scene(SceneUpdateCompiled),
    Jump { target_ip: u32 },
    SetFlag { flag_id: u32, value: bool },
    SetVar { var_id: u32, value: i32 },
    JumpIf { cond: CondCompiled, target_ip: u32 },
    Patch(ScenePatchCompiled),
    ExtCall { command: String, args: Vec<String> },
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
        match self {
            EventCompiled::Dialogue(dialogue) => serde_json::json!({
                "type": "dialogue",
                "speaker": dialogue.speaker.as_ref(),
                "text": dialogue.text.as_ref(),
            }),
            EventCompiled::Choice(choice) => serde_json::json!({
                "type": "choice",
                "prompt": choice.prompt.as_ref(),
                "options": choice.options.iter().map(|option| serde_json::json!({
                    "text": option.text.as_ref(),
                    "target": option.target_ip,
                    "target_ip": option.target_ip,
                })).collect::<Vec<_>>(),
            }),
            EventCompiled::Scene(scene) => serde_json::json!({
                "type": "scene",
                "background": scene.background.as_deref(),
                "music": scene.music.as_deref(),
                "characters": scene.characters.iter().map(|character| serde_json::json!({
                    "name": character.name.as_ref(),
                    "expression": character.expression.as_deref(),
                    "position": character.position.as_deref(),
                })).collect::<Vec<_>>(),
            }),
            EventCompiled::Jump { target_ip } => serde_json::json!({
                "type": "jump",
                "target": target_ip,
                "target_ip": target_ip,
            }),
            EventCompiled::SetFlag { flag_id, value } => serde_json::json!({
                "type": "set_flag",
                "key": flag_id,
                "flag_id": flag_id,
                "value": value,
            }),
            EventCompiled::SetVar { var_id, value } => serde_json::json!({
                "type": "set_var",
                "var_id": var_id,
                "value": value,
            }),
            EventCompiled::JumpIf { cond, target_ip } => serde_json::json!({
                "type": "jump_if",
                "cond": cond_to_json(cond),
                "target_ip": target_ip,
            }),
            EventCompiled::Patch(patch) => serde_json::json!({
                "type": "patch",
                "background": patch.background.as_deref(),
                "music": patch.music.as_deref(),
                "add": patch.add.iter().map(|character| serde_json::json!({
                    "name": character.name.as_ref(),
                    "expression": character.expression.as_deref(),
                    "position": character.position.as_deref(),
                })).collect::<Vec<_>>(),
                "update": patch.update.iter().map(|character| serde_json::json!({
                    "name": character.name.as_ref(),
                    "expression": character.expression.as_deref(),
                    "position": character.position.as_deref(),
                })).collect::<Vec<_>>(),
                "remove": patch.remove.iter().map(|name| name.as_ref()).collect::<Vec<_>>(),
            }),
            EventCompiled::ExtCall { command, args } => serde_json::json!({
                "type": "ext_call",
                "command": command,
                "args": args,
            }),
        }
    }

    /// Serializes the compiled event to a JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
}

fn cond_to_json(cond: &CondCompiled) -> serde_json::Value {
    match cond {
        CondCompiled::Flag { flag_id, is_set } => serde_json::json!({
            "kind": "flag",
            "flag_id": flag_id,
            "is_set": is_set,
        }),
        CondCompiled::VarCmp { var_id, op, value } => serde_json::json!({
            "kind": "var_cmp",
            "var_id": var_id,
            "op": op,
            "value": value,
        }),
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl EventRaw {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::IntoPyObject;
        let event = pyo3::Py::new(py, PyEvent::from_raw(self.clone()))?;
        Ok(event.into_pyobject(py)?.into_any().unbind())
    }
}

#[cfg(any(feature = "python", feature = "python-embed"))]
impl EventCompiled {
    pub fn to_python(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::IntoPyObject;
        let event = pyo3::Py::new(py, PyEvent::from_compiled(self.clone()))?;
        Ok(event.into_pyobject(py)?.into_any().unbind())
    }
}
