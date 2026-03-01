//! Python bindings for the visual editor components.
//!
//! Exposes NodeGraph, StoryNode, and validation to Python for scripting.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use visual_novel_engine::{
    CharacterPatchRaw, CharacterPlacementRaw, CmpOp, CondRaw, EventRaw, ScenePatchRaw,
};
use visual_novel_gui::editor::quick_fix::{
    apply_fix, suggest_fixes, QuickFixCandidate, QuickFixRisk,
};
use visual_novel_gui::editor::{
    validate_graph, DiagnosticLanguage, LintIssue, LintSeverity, NodeGraph, StoryNode,
};

fn parse_cmp_op(op: &str) -> PyResult<CmpOp> {
    match op {
        "eq" => Ok(CmpOp::Eq),
        "ne" => Ok(CmpOp::Ne),
        "lt" => Ok(CmpOp::Lt),
        "le" => Ok(CmpOp::Le),
        "gt" => Ok(CmpOp::Gt),
        "ge" => Ok(CmpOp::Ge),
        _ => Err(PyValueError::new_err(format!(
            "Unknown comparison op '{op}'"
        ))),
    }
}

fn select_fix_candidate(
    issue: &LintIssue,
    graph: &NodeGraph,
    include_review: bool,
) -> Option<QuickFixCandidate> {
    let candidates = suggest_fixes(issue, graph);
    if include_review {
        candidates
            .iter()
            .find(|candidate| candidate.risk == QuickFixRisk::Safe)
            .cloned()
            .or_else(|| candidates.into_iter().next())
    } else {
        candidates
            .into_iter()
            .find(|candidate| candidate.risk == QuickFixRisk::Safe)
    }
}

fn apply_autofix_pass(graph: &mut NodeGraph, include_review: bool) -> Result<usize, String> {
    let mut applied = 0usize;
    let mut guard = 0usize;

    while guard < 128 {
        guard += 1;
        let issues = validate_graph(graph);
        let mut applied_this_round = false;

        for issue in issues {
            let Some(candidate) = select_fix_candidate(&issue, graph, include_review) else {
                continue;
            };
            if apply_fix(graph, &issue, candidate.fix_id)? {
                applied += 1;
                applied_this_round = true;
                break;
            }
        }

        if !applied_this_round {
            break;
        }
    }

    Ok(applied)
}

// =============================================================================
// PyStoryNode - Node types for the story graph
// =============================================================================

/// A node in the story graph.
#[pyclass(name = "StoryNode")]
#[derive(Clone)]
pub struct PyStoryNode {
    inner: StoryNode,
}

#[pymethods]
impl PyStoryNode {
    #[staticmethod]
    fn dialogue(speaker: String, text: String) -> Self {
        Self {
            inner: StoryNode::Dialogue { speaker, text },
        }
    }

    #[staticmethod]
    fn choice(prompt: String, options: Vec<String>) -> Self {
        Self {
            inner: StoryNode::Choice { prompt, options },
        }
    }

    #[staticmethod]
    #[pyo3(signature = (background=None, music=None, characters=Vec::new()))]
    fn scene(
        background: Option<String>,
        music: Option<String>,
        characters: Vec<(String, Option<String>, Option<String>)>,
    ) -> Self {
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

        Self {
            inner: StoryNode::Scene {
                profile: None,
                background,
                music,
                characters,
            },
        }
    }

    #[staticmethod]
    fn jump(target: String) -> Self {
        Self {
            inner: StoryNode::Jump { target },
        }
    }

    #[staticmethod]
    fn set_variable(key: String, value: i32) -> Self {
        Self {
            inner: StoryNode::SetVariable { key, value },
        }
    }

    #[staticmethod]
    fn jump_if_flag(key: String, is_set: bool, target: String) -> Self {
        Self {
            inner: StoryNode::JumpIf {
                target,
                cond: CondRaw::Flag { key, is_set },
            },
        }
    }

    #[staticmethod]
    fn jump_if_var(key: String, op: String, value: i32, target: String) -> PyResult<Self> {
        Ok(Self {
            inner: StoryNode::JumpIf {
                target,
                cond: CondRaw::VarCmp {
                    key,
                    op: parse_cmp_op(&op)?,
                    value,
                },
            },
        })
    }

    #[staticmethod]
    #[pyo3(signature = (background=None, music=None, add=Vec::new(), update=Vec::new(), remove=Vec::new()))]
    fn scene_patch(
        background: Option<String>,
        music: Option<String>,
        add: Vec<(String, Option<String>, Option<String>)>,
        update: Vec<(String, Option<String>, Option<String>)>,
        remove: Vec<String>,
    ) -> Self {
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

        Self {
            inner: StoryNode::ScenePatch(ScenePatchRaw {
                background,
                music,
                add,
                update,
                remove,
            }),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (channel, action, asset=None, volume=None, fade_duration_ms=None, loop_playback=None))]
    fn audio_action(
        channel: String,
        action: String,
        asset: Option<String>,
        volume: Option<f32>,
        fade_duration_ms: Option<u64>,
        loop_playback: Option<bool>,
    ) -> Self {
        Self {
            inner: StoryNode::AudioAction {
                channel,
                action,
                asset,
                volume,
                fade_duration_ms,
                loop_playback,
            },
        }
    }

    #[staticmethod]
    #[pyo3(signature = (kind, duration_ms, color=None))]
    fn transition(kind: String, duration_ms: u32, color: Option<String>) -> Self {
        Self {
            inner: StoryNode::Transition {
                kind,
                duration_ms,
                color,
            },
        }
    }

    #[staticmethod]
    #[pyo3(signature = (name, x, y, scale=None))]
    fn character_placement(name: String, x: i32, y: i32, scale: Option<f32>) -> Self {
        Self {
            inner: StoryNode::CharacterPlacement { name, x, y, scale },
        }
    }

    #[staticmethod]
    fn generic(event_json: String) -> PyResult<Self> {
        let event: EventRaw = serde_json::from_str(&event_json)
            .map_err(|err| PyValueError::new_err(format!("Invalid event JSON: {err}")))?;
        Ok(Self {
            inner: StoryNode::Generic(event),
        })
    }

    #[staticmethod]
    fn start() -> Self {
        Self {
            inner: StoryNode::Start,
        }
    }

    #[staticmethod]
    fn end() -> Self {
        Self {
            inner: StoryNode::End,
        }
    }

    #[getter]
    fn node_type(&self) -> String {
        self.inner.type_name().to_string()
    }

    fn __repr__(&self) -> String {
        format!("StoryNode({})", self.inner.type_name())
    }
}

impl From<StoryNode> for PyStoryNode {
    fn from(inner: StoryNode) -> Self {
        Self { inner }
    }
}

// =============================================================================
// PyNodeGraph - The visual story graph
// =============================================================================

/// A graph of story nodes with connections.
#[pyclass(name = "NodeGraph")]
pub struct PyNodeGraph {
    inner: NodeGraph,
}

#[pymethods]
impl PyNodeGraph {
    #[new]
    fn new() -> Self {
        Self {
            inner: NodeGraph::new(),
        }
    }

    fn add_node(&mut self, node: PyStoryNode, x: f32, y: f32) -> u32 {
        let pos = eframe::egui::pos2(x, y);
        self.inner.add_node(node.inner, pos)
    }

    fn connect(&mut self, from_id: u32, to_id: u32) {
        self.inner.connect(from_id, to_id);
    }

    fn remove_node(&mut self, node_id: u32) {
        self.inner.remove_node(node_id);
    }

    fn node_count(&self) -> usize {
        self.inner.len()
    }

    fn connection_count(&self) -> usize {
        self.inner.connection_count()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn to_script_json(&self) -> PyResult<String> {
        let script = self.inner.to_script();
        serde_json::to_string_pretty(&script).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn search_nodes(&self, query: &str) -> Vec<u32> {
        self.inner.search_nodes(query)
    }

    fn validate(&self) -> Vec<PyLintIssue> {
        validate_graph(&self.inner)
            .into_iter()
            .map(PyLintIssue::from)
            .collect()
    }

    fn fix_candidates(&self, issue_index: usize) -> PyResult<Vec<PyQuickFixCandidate>> {
        let issues = validate_graph(&self.inner);
        let issue = issues
            .get(issue_index)
            .ok_or_else(|| PyValueError::new_err(format!("invalid issue index {issue_index}")))?;
        Ok(suggest_fixes(issue, &self.inner)
            .into_iter()
            .map(PyQuickFixCandidate::from)
            .collect())
    }

    #[pyo3(signature = (issue_index, include_review=false))]
    fn autofix_issue(
        &mut self,
        issue_index: usize,
        include_review: bool,
    ) -> PyResult<Option<String>> {
        let issues = validate_graph(&self.inner);
        let issue = issues
            .get(issue_index)
            .ok_or_else(|| PyValueError::new_err(format!("invalid issue index {issue_index}")))?;
        let candidate = select_fix_candidate(issue, &self.inner, include_review)
            .ok_or_else(|| PyValueError::new_err("no fix candidate available for issue"))?;
        let changed =
            apply_fix(&mut self.inner, issue, candidate.fix_id).map_err(PyValueError::new_err)?;
        if changed {
            Ok(Some(candidate.fix_id.to_string()))
        } else {
            Ok(None)
        }
    }

    fn autofix_safe(&mut self) -> PyResult<usize> {
        apply_autofix_pass(&mut self.inner, false).map_err(PyValueError::new_err)
    }

    fn autofix_full(&mut self) -> PyResult<usize> {
        apply_autofix_pass(&mut self.inner, true).map_err(PyValueError::new_err)
    }

    fn set_bookmark(&mut self, name: String, node_id: u32) -> bool {
        self.inner.set_bookmark(name, node_id)
    }

    fn remove_bookmark(&mut self, name: &str) -> bool {
        self.inner.remove_bookmark(name)
    }

    fn bookmark_target(&self, name: &str) -> Option<u32> {
        self.inner.bookmarked_node(name)
    }

    fn list_bookmarks(&self) -> Vec<(String, u32)> {
        self.inner
            .bookmarks()
            .map(|(name, node_id)| (name.clone(), *node_id))
            .collect()
    }

    fn save(&self, path: &str) -> PyResult<()> {
        let script = self.inner.to_script();
        let json = serde_json::to_string_pretty(&script)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let content =
            std::fs::read_to_string(path).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let script: visual_novel_engine::ScriptRaw =
            serde_json::from_str(&content).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let graph = NodeGraph::from_script(&script);
        Ok(Self { inner: graph })
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeGraph(nodes={}, connections={})",
            self.inner.len(),
            self.inner.connection_count()
        )
    }
}

/// Quick-fix candidate metadata exposed to Python.
#[pyclass(name = "QuickFixCandidate")]
#[derive(Clone)]
pub struct PyQuickFixCandidate {
    #[pyo3(get)]
    fix_id: String,
    #[pyo3(get)]
    risk: String,
    #[pyo3(get)]
    structural: bool,
    #[pyo3(get)]
    title_es: String,
    #[pyo3(get)]
    title_en: String,
    #[pyo3(get)]
    preconditions_es: String,
    #[pyo3(get)]
    preconditions_en: String,
    #[pyo3(get)]
    postconditions_es: String,
    #[pyo3(get)]
    postconditions_en: String,
}

#[pymethods]
impl PyQuickFixCandidate {
    fn __repr__(&self) -> String {
        format!(
            "QuickFixCandidate(fix_id='{}', risk='{}', structural={})",
            self.fix_id, self.risk, self.structural
        )
    }
}

impl From<QuickFixCandidate> for PyQuickFixCandidate {
    fn from(candidate: QuickFixCandidate) -> Self {
        Self {
            fix_id: candidate.fix_id.to_string(),
            risk: candidate.risk.label().to_string(),
            structural: candidate.structural,
            title_es: candidate.title_es.to_string(),
            title_en: candidate.title_en.to_string(),
            preconditions_es: candidate.preconditions_es.to_string(),
            preconditions_en: candidate.preconditions_en.to_string(),
            postconditions_es: candidate.postconditions_es.to_string(),
            postconditions_en: candidate.postconditions_en.to_string(),
        }
    }
}

// =============================================================================
// PyLintSeverity - Validation severity levels
// =============================================================================

/// Severity level for lint issues.
#[pyclass(name = "LintSeverity")]
#[derive(Clone)]
pub struct PyLintSeverity {
    inner: LintSeverity,
}

#[pymethods]
impl PyLintSeverity {
    #[classattr]
    #[pyo3(name = "Error")]
    fn error() -> Self {
        Self {
            inner: LintSeverity::Error,
        }
    }

    #[classattr]
    #[pyo3(name = "Warning")]
    fn warning() -> Self {
        Self {
            inner: LintSeverity::Warning,
        }
    }

    #[classattr]
    #[pyo3(name = "Info")]
    fn info() -> Self {
        Self {
            inner: LintSeverity::Info,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            LintSeverity::Error => "LintSeverity.Error".to_string(),
            LintSeverity::Warning => "LintSeverity.Warning".to_string(),
            LintSeverity::Info => "LintSeverity.Info".to_string(),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// =============================================================================
// PyLintIssue - Validation issue
// =============================================================================

/// A validation issue found in the graph.
#[pyclass(name = "LintIssue")]
#[derive(Clone)]
pub struct PyLintIssue {
    #[pyo3(get)]
    severity: PyLintSeverity,
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    node_id: Option<u32>,
    #[pyo3(get)]
    event_ip: Option<u32>,
    #[pyo3(get)]
    edge_from: Option<u32>,
    #[pyo3(get)]
    edge_to: Option<u32>,
    #[pyo3(get)]
    asset_path: Option<String>,
    #[pyo3(get)]
    phase: String,
    #[pyo3(get)]
    code: String,
    #[pyo3(get)]
    diagnostic_id: String,
    #[pyo3(get)]
    message_es: String,
    #[pyo3(get)]
    message_en: String,
    #[pyo3(get)]
    root_cause_es: String,
    #[pyo3(get)]
    root_cause_en: String,
    #[pyo3(get)]
    why_failed_es: String,
    #[pyo3(get)]
    why_failed_en: String,
    #[pyo3(get)]
    how_to_fix_es: String,
    #[pyo3(get)]
    how_to_fix_en: String,
    #[pyo3(get)]
    docs_ref: String,
}

#[pymethods]
impl PyLintIssue {
    fn __repr__(&self) -> String {
        format!(
            "LintIssue({}, {}, node={:?}, ip={:?}, diag={})",
            self.severity.__repr__(),
            self.message,
            self.node_id,
            self.event_ip,
            self.diagnostic_id
        )
    }
}

impl From<LintIssue> for PyLintIssue {
    fn from(issue: LintIssue) -> Self {
        Self {
            severity: PyLintSeverity {
                inner: issue.severity,
            },
            message: issue.message.clone(),
            node_id: issue.node_id,
            event_ip: issue.event_ip,
            edge_from: issue.edge_from,
            edge_to: issue.edge_to,
            asset_path: issue.asset_path.clone(),
            phase: issue.phase.label().to_string(),
            code: issue.code.label().to_string(),
            diagnostic_id: issue.diagnostic_id(),
            message_es: issue.localized_message(DiagnosticLanguage::Es),
            message_en: issue.localized_message(DiagnosticLanguage::En),
            root_cause_es: issue.explanation(DiagnosticLanguage::Es).root_cause,
            root_cause_en: issue.explanation(DiagnosticLanguage::En).root_cause,
            why_failed_es: issue.explanation(DiagnosticLanguage::Es).why_failed,
            why_failed_en: issue.explanation(DiagnosticLanguage::En).why_failed,
            how_to_fix_es: issue.explanation(DiagnosticLanguage::Es).how_to_fix,
            how_to_fix_en: issue.explanation(DiagnosticLanguage::En).how_to_fix,
            docs_ref: issue.explanation(DiagnosticLanguage::En).docs_ref,
        }
    }
}

// =============================================================================
// validate_graph function
// =============================================================================

/// Validates a node graph and returns a list of issues.
#[pyfunction]
pub fn py_validate_graph(graph: &PyNodeGraph) -> Vec<PyLintIssue> {
    validate_graph(&graph.inner)
        .into_iter()
        .map(PyLintIssue::from)
        .collect()
}

// =============================================================================
// Module registration
// =============================================================================

/// Registers editor classes with the Python module.
pub fn register_editor_classes(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStoryNode>()?;
    m.add_class::<PyNodeGraph>()?;
    m.add_class::<PyQuickFixCandidate>()?;
    m.add_class::<PyLintSeverity>()?;
    m.add_class::<PyLintIssue>()?;
    m.add_function(wrap_pyfunction!(py_validate_graph, m)?)?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/editor_tests.rs"]
mod tests;
