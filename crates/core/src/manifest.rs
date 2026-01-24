use std::collections::HashMap;
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The single source of truth for a visual novel project.
///
/// The manifest acts as a "compass", guiding the loading of assets and configuration.
/// Anything not strictly declared here is considered non-existent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectManifest {
    pub metadata: ProjectMetadata,
    pub settings: ProjectSettings,
    pub assets: AssetManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSettings {
    pub resolution: (u32, u32),
    pub default_language: String,
    pub supported_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AssetManifest {
    pub backgrounds: HashMap<String, PathBuf>,
    pub characters: HashMap<String, CharacterAsset>,
    pub audio: HashMap<String, PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterAsset {
    pub path: PathBuf,
    /// Default scale for this character (1.0 = normal)
    pub scale: Option<f32>,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ManifestError {
    #[error("manifest file not found at {0}")]
    #[diagnostic(
        code(manifest::not_found),
        help("Create a 'project.vnm' file in the root directory")
    )]
    NotFound(PathBuf),

    #[error("failed to parse manifest: {0}")]
    #[diagnostic(code(manifest::parse_error))]
    ParseError(#[from] toml::de::Error),

    #[error("io error: {0}")]
    #[diagnostic(code(manifest::io_error))]
    IoError(#[from] std::io::Error),
}

impl ProjectManifest {
    /// load a manifest from a file path.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        let manifest: ProjectManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// save the manifest to a file path.
    pub fn save(&self, path: &Path) -> Result<(), ManifestError> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            // Mapping toml ser error separately as it's different check?
            // For simplicity wrapping via io or just implementing from
            // But toml::ser::Error is different.
            // Let's just use invalid data or similar wrapper.
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// creates a default new project manifest.
    pub fn new(name: &str, author: &str) -> Self {
        Self {
            metadata: ProjectMetadata {
                name: name.to_string(),
                author: author.to_string(),
                version: "0.1.0".to_string(),
                description: None,
            },
            settings: ProjectSettings {
                resolution: (1280, 720),
                default_language: "en".to_string(),
                supported_languages: vec!["en".to_string()],
            },
            assets: AssetManifest::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let manifest = ProjectManifest::new("Test Project", "Tester");
        let toml_str = toml::to_string(&manifest).expect("Failed to serialize");
        let loaded: ProjectManifest = toml::from_str(&toml_str).expect("Failed to deserialize");

        assert_eq!(manifest, loaded);
        assert_eq!(loaded.metadata.name, "Test Project");
    }

    #[test]
    fn test_settings_defaults() {
        let manifest = ProjectManifest::new("P", "A");
        assert_eq!(manifest.settings.resolution, (1280, 720));
        assert_eq!(manifest.settings.default_language, "en");
    }
}
