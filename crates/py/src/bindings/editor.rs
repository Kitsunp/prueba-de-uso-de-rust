//! Python bindings for the visual editor components.
//!
//! Exposes NodeGraph, StoryNode, and validation to Python for scripting.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use visual_novel_gui::editor::{validate_graph, LintIssue, LintSeverity, NodeGraph, StoryNode};

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
    /// Creates a dialogue node.
    #[staticmethod]
    fn dialogue(speaker: String, text: String) -> Self {
        Self {
            inner: StoryNode::Dialogue { speaker, text },
        }
    }

    /// Creates a choice node.
    #[staticmethod]
    fn choice(prompt: String, options: Vec<String>) -> Self {
        Self {
            inner: StoryNode::Choice { prompt, options },
        }
    }

    /// Creates a scene node.
    #[staticmethod]
    fn scene(background: String) -> Self {
        Self {
            inner: StoryNode::Scene { background },
        }
    }

    /// Creates a jump node.
    #[staticmethod]
    fn jump(target: String) -> Self {
        Self {
            inner: StoryNode::Jump { target },
        }
    }

    /// Creates a start node.
    #[staticmethod]
    fn start() -> Self {
        Self {
            inner: StoryNode::Start,
        }
    }

    /// Creates an end node.
    #[staticmethod]
    fn end() -> Self {
        Self {
            inner: StoryNode::End,
        }
    }

    /// Returns the node type as a string.
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
    /// Creates a new empty graph.
    #[new]
    fn new() -> Self {
        Self {
            inner: NodeGraph::new(),
        }
    }

    /// Adds a node to the graph.
    ///
    /// Returns the node ID.
    fn add_node(&mut self, node: PyStoryNode, x: f32, y: f32) -> u32 {
        let pos = eframe::egui::pos2(x, y);
        self.inner.add_node(node.inner, pos)
    }

    /// Connects two nodes.
    fn connect(&mut self, from_id: u32, to_id: u32) {
        self.inner.connect(from_id, to_id);
    }

    /// Removes a node by ID.
    fn remove_node(&mut self, node_id: u32) {
        self.inner.remove_node(node_id);
    }

    /// Returns the number of nodes.
    fn node_count(&self) -> usize {
        self.inner.len()
    }

    /// Returns the number of connections.
    fn connection_count(&self) -> usize {
        self.inner.connection_count()
    }

    /// Returns whether the graph is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Converts the graph to a script JSON string.
    fn to_script_json(&self) -> PyResult<String> {
        let script = self.inner.to_script();
        serde_json::to_string_pretty(&script).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Saves the graph to a JSON file.
    fn save(&self, path: &str) -> PyResult<()> {
        let script = self.inner.to_script();
        let json = serde_json::to_string_pretty(&script)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Loads a graph from a JSON file.
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
    /// Error severity (critical issues).
    #[classattr]
    fn Error() -> Self {
        Self {
            inner: LintSeverity::Error,
        }
    }

    /// Warning severity (potential issues).
    #[classattr]
    fn Warning() -> Self {
        Self {
            inner: LintSeverity::Warning,
        }
    }

    /// Info severity (informational).
    #[classattr]
    fn Info() -> Self {
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
}

#[pymethods]
impl PyLintIssue {
    fn __repr__(&self) -> String {
        format!(
            "LintIssue({:?}, {:?}, node={:?})",
            self.severity.__repr__(),
            self.message,
            self.node_id
        )
    }
}

impl From<LintIssue> for PyLintIssue {
    fn from(issue: LintIssue) -> Self {
        Self {
            severity: PyLintSeverity {
                inner: issue.severity,
            },
            message: issue.message,
            node_id: issue.node_id,
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
    m.add_class::<PyLintSeverity>()?;
    m.add_class::<PyLintIssue>()?;
    m.add_function(wrap_pyfunction!(py_validate_graph, m)?)?;
    Ok(())
}
