use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
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
    allowed_extensions: HashSet<String>,
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
        let allowed_extensions = ["png", "jpg", "jpeg"]
            .into_iter()
            .map(|ext| ext.to_string())
            .collect();
        Ok(Self {
            root,
            mode,
            allowed_extensions,
            limits: AssetLimits::default(),
            manifest,
            require_manifest,
        })
    }

    pub fn with_limits(mut self, limits: AssetLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn load_image(&self, asset_path: &str) -> Result<LoadedImage, AssetError> {
        let rel = sanitize_rel_path(Path::new(asset_path))?;
        let extension = rel
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|value| value.to_lowercase())
            .ok_or_else(|| AssetError::UnsupportedExtension(asset_path.to_string()))?;
        if !self.allowed_extensions.contains(&extension) {
            return Err(AssetError::UnsupportedExtension(extension));
        }
        let full_path = self.root.join(&rel);
        let bytes = fs::read(&full_path)?;
        let size = bytes.len() as u64;
        if size > self.limits.max_bytes {
            return Err(AssetError::TooLarge {
                size,
                max: self.limits.max_bytes,
            });
        }
        self.verify_manifest(asset_path, size, &bytes)?;
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

#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub bytes: usize,
    pub budget_bytes: usize,
}

struct CachedTexture {
    texture: egui::TextureHandle,
    bytes: usize,
    last_used: u64,
}

pub struct AssetManager {
    store: AssetStore,
    cache: HashMap<String, CachedTexture>,
    budget_bytes: usize,
    current_bytes: usize,
    usage_counter: u64,
    stats: CacheStats,
}

impl AssetManager {
    pub fn new(store: AssetStore, budget_bytes: usize) -> Self {
        let stats = CacheStats {
            budget_bytes,
            ..CacheStats::default()
        };
        Self {
            store,
            cache: HashMap::new(),
            budget_bytes,
            current_bytes: 0,
            usage_counter: 0,
            stats,
        }
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            bytes: self.current_bytes,
            ..self.stats.clone()
        }
    }

    pub fn texture_for_asset(
        &mut self,
        ctx: &egui::Context,
        asset_path: &str,
    ) -> Result<Option<&egui::TextureHandle>, AssetError> {
        self.usage_counter = self.usage_counter.wrapping_add(1);
        if let Some(entry) = self.cache.get_mut(asset_path) {
            entry.last_used = self.usage_counter;
            self.stats.hits += 1;
            return Ok(Some(&entry.texture));
        }
        let loaded = self.store.load_image(asset_path)?;
        let bytes = loaded.pixels.len();
        if bytes > self.budget_bytes {
            return Err(AssetError::BudgetExceeded {
                bytes,
                budget: self.budget_bytes,
            });
        }
        self.stats.misses += 1;
        while self.current_bytes + bytes > self.budget_bytes {
            if !self.evict_lru() {
                break;
            }
        }
        let texture = ctx.load_texture(
            loaded.name.clone(),
            egui::ColorImage::from_rgba_unmultiplied(loaded.size, &loaded.pixels),
            egui::TextureOptions::default(),
        );
        self.current_bytes += bytes;
        self.cache.insert(
            loaded.name.clone(),
            CachedTexture {
                texture,
                bytes,
                last_used: self.usage_counter,
            },
        );
        Ok(self.cache.get(asset_path).map(|entry| &entry.texture))
    }

    fn evict_lru(&mut self) -> bool {
        let Some((key, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, entry)| (key.clone(), entry.bytes))
        else {
            return false;
        };
        if let Some(entry) = self.cache.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(entry.bytes);
            self.stats.evictions += 1;
            return true;
        }
        false
    }
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
