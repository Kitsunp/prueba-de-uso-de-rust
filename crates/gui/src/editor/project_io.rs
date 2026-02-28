use crate::editor::errors::EditorError;
use crate::editor::{node_graph::NodeGraph, script_sync};
use std::path::PathBuf;
use toml;
use visual_novel_engine::{manifest::ProjectManifest, ScriptRaw}; // Added toml import

pub struct LoadedProject {
    pub manifest: ProjectManifest,
    pub entry_point_script: Option<(PathBuf, LoadedScript)>,
}

pub struct LoadedScript {
    pub graph: NodeGraph,
    pub was_imported: bool,
}

pub fn load_project(path: PathBuf) -> Result<LoadedProject, EditorError> {
    // 1. Load Manifest (TOML)
    let manifest_content = std::fs::read_to_string(&path).map_err(EditorError::IoError)?;

    // Use toml deserializer
    let manifest: ProjectManifest = toml::from_str(&manifest_content).map_err(|e| {
        EditorError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;

    // 2. Load Entry Point Script if exists
    let entry_point_script = if let Ok(entry_rel) =
        std::path::Path::new(&manifest.settings.entry_point).strip_prefix("")
    {
        // Simple path join, assuming relative to manifest
        let parent = path.parent().unwrap_or(&path);
        let script_path = parent.join(entry_rel);
        if script_path.exists() {
            Some((script_path.clone(), load_script(script_path)?))
        } else {
            None
        }
    } else {
        None
    };

    Ok(LoadedProject {
        manifest,
        entry_point_script,
    })
}

pub fn load_script(path: PathBuf) -> Result<LoadedScript, EditorError> {
    let content = std::fs::read_to_string(&path).map_err(EditorError::IoError)?;

    // Try parsing as ScriptRaw (JSON)
    let script = ScriptRaw::from_json(&content)
        .map_err(|e| EditorError::CompileError(format!("Parse error: {}", e)))?;

    let graph = script_sync::from_script(&script);
    Ok(LoadedScript {
        graph,
        was_imported: false,
    })
}

pub fn save_script(path: &std::path::Path, graph: &NodeGraph) -> Result<(), EditorError> {
    let script = script_sync::to_script(graph);
    let json = script
        .to_json()
        .map_err(|e| EditorError::CompileError(format!("Serialization error: {}", e)))?;

    std::fs::write(path, json).map_err(EditorError::IoError)?;

    Ok(())
}
