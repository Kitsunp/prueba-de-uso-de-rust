use crate::editor::errors::EditorError;
use crate::editor::{node_graph::NodeGraph, script_sync};
use std::path::PathBuf;
use visual_novel_engine::{
    manifest::{ManifestMigrationReport, ProjectManifest},
    ScriptRaw,
};

pub struct LoadedProject {
    pub manifest: ProjectManifest,
    pub manifest_migration_report: Option<ManifestMigrationReport>,
    pub entry_point_script: Option<(PathBuf, LoadedScript)>,
}

pub struct LoadedScript {
    pub graph: NodeGraph,
    pub was_imported: bool,
}

pub fn load_project(path: PathBuf) -> Result<LoadedProject, EditorError> {
    // 1. Load Manifest (TOML)
    let manifest_content = std::fs::read_to_string(&path).map_err(EditorError::IoError)?;

    let (manifest, migration_report) = ProjectManifest::from_toml_with_migration(&manifest_content)
        .map_err(|e| {
            EditorError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
    let manifest_migration_report = migration_report.changed().then_some(migration_report);

    // 2. Load Entry Point Script if exists
    let entry_point_script = {
        let entry = std::path::PathBuf::from(&manifest.settings.entry_point);
        let parent = path.parent().unwrap_or(&path);
        let script_path = if entry.is_absolute() {
            entry
        } else {
            parent.join(entry)
        };

        if script_path.exists() {
            Some((script_path.clone(), load_script(script_path)?))
        } else {
            None
        }
    };

    Ok(LoadedProject {
        manifest,
        manifest_migration_report,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use visual_novel_engine::manifest::MANIFEST_SCHEMA_VERSION;

    #[test]
    fn load_project_applies_manifest_migration_for_legacy_schema() {
        let dir = tempdir().expect("tempdir");
        let manifest_path = dir.path().join("project.vnm");
        let script_path = dir.path().join("main.json");

        let legacy_manifest = r#"
schema_version = "0.9"

[metadata]
name = "Legacy Project"
author = "QA"
version = "0.1.0"

[settings]
resolution = [1280, 720]
default_language = "es"
supported_languages = ["es", "en"]
entry_point = "main.json"

[assets]
"#;
        fs::write(&manifest_path, legacy_manifest).expect("write manifest");
        fs::write(
            &script_path,
            r#"{
  "script_schema_version": "1.0",
  "events": [
    { "type": "dialogue", "speaker": "Narrador", "text": "Hola" }
  ],
  "labels": { "start": 0 }
}"#,
        )
        .expect("write script");

        let loaded = load_project(manifest_path).expect("legacy manifest should load");
        assert_eq!(
            loaded.manifest.manifest_schema_version,
            MANIFEST_SCHEMA_VERSION
        );
        assert!(loaded.manifest_migration_report.is_some());
        assert!(loaded.entry_point_script.is_some());
    }
}
