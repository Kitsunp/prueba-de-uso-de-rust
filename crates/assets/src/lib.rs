#![allow(unused_assignments)]
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    Trusted,
    Untrusted,
}

#[derive(Clone, Debug)]
pub struct AssetLimits {
    pub max_bytes: u64,
    pub max_width: u32,
    pub max_height: u32,
}

impl Default for AssetLimits {
    fn default() -> Self {
        Self {
            max_bytes: 15 * 1024 * 1024,
            max_width: 4096,
            max_height: 4096,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssetManifest {
    pub manifest_version: u16,
    pub assets: BTreeMap<String, AssetEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssetEntry {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset path traversal blocked")]
    Traversal,
    #[error("unsupported asset extension: {0}")]
    UnsupportedExtension(String),
    #[error("asset too large: {size} bytes (max {max})")]
    TooLarge { size: u64, max: u64 },
    #[error("asset dimensions {width}x{height} exceed limit {max_width}x{max_height}")]
    InvalidDimensions {
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },
    #[error("manifest required for untrusted assets")]
    ManifestMissing,
    #[error("unsupported manifest version {0}")]
    ManifestVersion(u16),
    #[error("manifest entry missing for asset '{0}'")]
    ManifestEntryMissing(String),
    #[error("manifest hash mismatch for asset '{0}'")]
    ManifestHashMismatch(String),
    #[error("manifest size mismatch for asset '{0}'")]
    ManifestSizeMismatch(String),
    #[error("image decode error: {0}")]
    Decode(String),
    #[error("asset exceeds cache budget: {bytes} bytes (budget {budget})")]
    BudgetExceeded { bytes: usize, budget: usize },
}

#[derive(Clone, Debug)]
pub struct AssetStore {
    root: PathBuf,
    mode: SecurityMode,
    allowed_image_extensions: HashSet<String>,
    limits: AssetLimits,
    manifest: Option<AssetManifest>,
    require_manifest: bool,
}

impl AssetStore {
    pub fn new(
        root: PathBuf,
        mode: SecurityMode,
        manifest_path: Option<PathBuf>,
        require_manifest: bool,
    ) -> Result<Self, AssetError> {
        let manifest = match manifest_path {
            Some(path) => {
                let raw = fs::read_to_string(path)?;
                let manifest: AssetManifest = serde_json::from_str(&raw)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
                if manifest.manifest_version != 1 {
                    return Err(AssetError::ManifestVersion(manifest.manifest_version));
                }
                Some(manifest)
            }
            None => None,
        };
        let allowed_image_extensions = ["png", "jpg", "jpeg"]
            .into_iter()
            .map(|ext| ext.to_string())
            .collect();
        Ok(Self {
            root,
            mode,
            allowed_image_extensions,
            limits: AssetLimits::default(),
            manifest,
            require_manifest,
        })
    }

    pub fn with_limits(mut self, limits: AssetLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Loads raw bytes for an asset (e.g. for audio)
    pub fn load_bytes(&self, asset_path: &str) -> Result<Vec<u8>, AssetError> {
        let rel = sanitize_rel_path(Path::new(asset_path))?;
        let full_path = self.root.join(&rel); // sanitize_rel_path prevents traversal

        let bytes = fs::read(&full_path)?;
        let size = bytes.len() as u64;
        if size > self.limits.max_bytes {
            return Err(AssetError::TooLarge {
                size,
                max: self.limits.max_bytes,
            });
        }
        self.verify_manifest(asset_path, size, &bytes)?;
        Ok(bytes)
    }

    pub fn load_image(&self, asset_path: &str) -> Result<LoadedImage, AssetError> {
        let rel = sanitize_rel_path(Path::new(asset_path))?;
        let extension = rel
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|value| value.to_lowercase())
            .ok_or_else(|| AssetError::UnsupportedExtension(asset_path.to_string()))?;
        if !self.allowed_image_extensions.contains(&extension) {
            return Err(AssetError::UnsupportedExtension(asset_path.to_string()));
        }

        let bytes = self.load_bytes(asset_path)?;

        let image =
            image::load_from_memory(&bytes).map_err(|err| AssetError::Decode(err.to_string()))?;
        let rgba = image.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        if width > self.limits.max_width || height > self.limits.max_height {
            return Err(AssetError::InvalidDimensions {
                width,
                height,
                max_width: self.limits.max_width,
                max_height: self.limits.max_height,
            });
        }
        Ok(LoadedImage {
            name: asset_path.to_string(),
            size: [width as usize, height as usize],
            pixels: rgba.into_raw(),
        })
    }

    fn verify_manifest(&self, asset_path: &str, size: u64, bytes: &[u8]) -> Result<(), AssetError> {
        if self.mode == SecurityMode::Untrusted && self.require_manifest && self.manifest.is_none()
        {
            return Err(AssetError::ManifestMissing);
        }
        let Some(manifest) = &self.manifest else {
            return Ok(());
        };
        let entry = manifest
            .assets
            .get(asset_path)
            .ok_or_else(|| AssetError::ManifestEntryMissing(asset_path.to_string()))?;
        if entry.size != size {
            return Err(AssetError::ManifestSizeMismatch(asset_path.to_string()));
        }
        let expected = entry.sha256.to_lowercase();
        let actual = sha256_hex(bytes);
        if expected != actual {
            return Err(AssetError::ManifestHashMismatch(asset_path.to_string()));
        }
        Ok(())
    }
}

pub struct LoadedImage {
    pub name: String,
    pub size: [usize; 2],
    pub pixels: Vec<u8>,
}

pub fn sanitize_rel_path(rel: &Path) -> Result<PathBuf, AssetError> {
    use std::path::Component::*;
    let mut out = PathBuf::new();
    for component in rel.components() {
        match component {
            CurDir => {}
            Normal(part) => out.push(part),
            ParentDir | RootDir | Prefix(_) => return Err(AssetError::Traversal),
        }
    }
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_image_rejects_unsupported_extension_before_io() {
        let store = AssetStore::new(PathBuf::from("."), SecurityMode::Trusted, None, false)
            .expect("asset store should initialize");

        let err = match store.load_image("assets/theme.ogg") {
            Ok(_) => panic!("non-image extension must be rejected"),
            Err(err) => err,
        };

        assert!(matches!(err, AssetError::UnsupportedExtension(_)));
    }
}
