#![allow(unused_assignments)]
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

#[derive(Debug)]
pub struct AssetStore {
    root: PathBuf,
    mode: SecurityMode,
    allowed_image_extensions: HashSet<String>,
    limits: AssetLimits,
    manifest: Option<AssetManifest>,
    require_manifest: bool,
    byte_cache: Mutex<ByteCache>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetFingerprintEntry {
    pub rel_path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetFingerprintCatalog {
    pub entries: BTreeMap<String, AssetFingerprintEntry>,
    pub dedup_groups: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformBudget {
    pub max_total_bytes: u64,
    pub max_assets: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetReport {
    pub total_bytes: u64,
    pub asset_count: usize,
    pub duplicate_blob_count: usize,
    pub unique_blob_count: usize,
    pub within_budget: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTarget {
    Desktop,
    Mobile,
    Web,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Audio,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscodePreset {
    pub target: PlatformTarget,
    pub image_extension: &'static str,
    pub audio_extension: &'static str,
    pub image_quality: u8,
    pub audio_bitrate_kbps: u16,
    pub max_texture_side: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscodeRecommendation {
    pub rel_path: String,
    pub kind: AssetKind,
    pub source_extension: String,
    pub target_extension: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenePreloadPlan {
    pub by_scene: BTreeMap<String, Vec<String>>,
    pub unique_assets: Vec<String>,
    pub total_references: usize,
    pub deduped_references: usize,
    pub cache_hit_rate: f32,
}

#[derive(Debug)]
struct CachedBytes {
    data: Vec<u8>,
    bytes: usize,
    last_used: u64,
}

#[derive(Debug)]
struct ByteCache {
    entries: HashMap<String, CachedBytes>,
    usage_counter: u64,
    current_bytes: usize,
    max_bytes: usize,
}

impl ByteCache {
    fn new(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            usage_counter: 0,
            current_bytes: 0,
            max_bytes,
        }
    }

    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.usage_counter = self.usage_counter.wrapping_add(1);
        self.entries.get_mut(key).map(|entry| {
            entry.last_used = self.usage_counter;
            entry.data.clone()
        })
    }

    fn insert(&mut self, key: String, data: Vec<u8>) {
        let bytes = data.len();
        if bytes > self.max_bytes {
            return;
        }

        self.usage_counter = self.usage_counter.wrapping_add(1);

        if let Some(old) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.bytes);
        }

        while self.current_bytes + bytes > self.max_bytes {
            let Some((evict_key, evict_bytes)) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(k, entry)| (k.clone(), entry.bytes))
            else {
                break;
            };
            self.entries.remove(&evict_key);
            self.current_bytes = self.current_bytes.saturating_sub(evict_bytes);
        }

        self.entries.insert(
            key,
            CachedBytes {
                data,
                bytes,
                last_used: self.usage_counter,
            },
        );
        self.current_bytes = self.current_bytes.saturating_add(bytes);
    }
}

impl PlatformTarget {
    pub fn default_budget(self) -> PlatformBudget {
        match self {
            PlatformTarget::Desktop => PlatformBudget {
                max_total_bytes: 2 * 1024 * 1024 * 1024,
                max_assets: 20_000,
            },
            PlatformTarget::Mobile => PlatformBudget {
                max_total_bytes: 512 * 1024 * 1024,
                max_assets: 10_000,
            },
            PlatformTarget::Web => PlatformBudget {
                max_total_bytes: 256 * 1024 * 1024,
                max_assets: 8_000,
            },
        }
    }

    pub fn default_transcode_preset(self) -> TranscodePreset {
        match self {
            PlatformTarget::Desktop => TranscodePreset {
                target: self,
                image_extension: "png",
                audio_extension: "ogg",
                image_quality: 95,
                audio_bitrate_kbps: 192,
                max_texture_side: 4096,
            },
            PlatformTarget::Mobile => TranscodePreset {
                target: self,
                image_extension: "webp",
                audio_extension: "ogg",
                image_quality: 85,
                audio_bitrate_kbps: 128,
                max_texture_side: 2048,
            },
            PlatformTarget::Web => TranscodePreset {
                target: self,
                image_extension: "webp",
                audio_extension: "mp3",
                image_quality: 80,
                audio_bitrate_kbps: 128,
                max_texture_side: 2048,
            },
        }
    }
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
            byte_cache: Mutex::new(ByteCache::new(64 * 1024 * 1024)),
        })
    }

    pub fn with_limits(mut self, limits: AssetLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_cache_budget(mut self, budget_bytes: usize) -> Self {
        self.byte_cache = Mutex::new(ByteCache::new(budget_bytes));
        self
    }

    /// Loads raw bytes for an asset (e.g. for audio)
    pub fn load_bytes(&self, asset_path: &str) -> Result<Vec<u8>, AssetError> {
        let rel = sanitize_rel_path(Path::new(asset_path))?;
        let cache_key = rel.to_string_lossy().replace('\\', "/");

        if let Some(bytes) = self
            .byte_cache
            .lock()
            .map_err(|_| std::io::Error::other("asset cache lock poisoned"))?
            .get(&cache_key)
        {
            return Ok(bytes);
        }

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
        self.byte_cache
            .lock()
            .map_err(|_| std::io::Error::other("asset cache lock poisoned"))?
            .insert(cache_key, bytes.clone());
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

impl AssetFingerprintCatalog {
    pub fn build(root: &Path, allowed_extensions: &[&str]) -> Result<Self, AssetError> {
        let mut entries = BTreeMap::new();
        let mut dedup_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let allowed: HashSet<String> = allowed_extensions
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }

                if !is_allowed_by_extension(&path, &allowed) {
                    continue;
                }

                let rel = path
                    .strip_prefix(root)
                    .map_err(|_| AssetError::Traversal)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = fs::read(&path)?;
                let size = bytes.len() as u64;
                let sha256 = sha256_hex(&bytes);
                entries.insert(
                    rel.clone(),
                    AssetFingerprintEntry {
                        rel_path: rel.clone(),
                        sha256: sha256.clone(),
                        size,
                    },
                );
                dedup_groups.entry(sha256).or_default().push(rel);
            }
        }

        Ok(Self {
            entries,
            dedup_groups,
        })
    }

    pub fn unique_blob_count(&self) -> usize {
        self.dedup_groups.len()
    }

    pub fn duplicate_blob_count(&self) -> usize {
        self.dedup_groups
            .values()
            .map(Vec::len)
            .filter(|count| *count > 1)
            .map(|count| count - 1)
            .sum()
    }

    pub fn budget_report(&self, budget: PlatformBudget) -> BudgetReport {
        let total_bytes = self.entries.values().map(|entry| entry.size).sum();
        let asset_count = self.entries.len();
        let within_budget =
            total_bytes <= budget.max_total_bytes && asset_count <= budget.max_assets;
        BudgetReport {
            total_bytes,
            asset_count,
            duplicate_blob_count: self.duplicate_blob_count(),
            unique_blob_count: self.unique_blob_count(),
            within_budget,
        }
    }

    pub fn transcode_recommendations(
        &self,
        target: PlatformTarget,
    ) -> Vec<TranscodeRecommendation> {
        let preset = target.default_transcode_preset();
        let mut output = Vec::new();

        for entry in self.entries.values() {
            let source_extension = Path::new(&entry.rel_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase())
                .unwrap_or_default();
            let kind = infer_asset_kind(&entry.rel_path);
            let target_extension = match kind {
                AssetKind::Image => Some(preset.image_extension),
                AssetKind::Audio => Some(preset.audio_extension),
                AssetKind::Other => None,
            };

            let Some(target_extension) = target_extension else {
                continue;
            };
            if source_extension == target_extension {
                continue;
            }

            output.push(TranscodeRecommendation {
                rel_path: entry.rel_path.clone(),
                kind,
                source_extension,
                target_extension: target_extension.to_string(),
                reason: format!(
                    "target={:?} prefers .{} for {:?} assets",
                    target, target_extension, kind
                ),
            });
        }

        output
    }

    pub fn scene_preload_plan(scene_assets: &BTreeMap<String, Vec<String>>) -> ScenePreloadPlan {
        let mut by_scene = BTreeMap::new();
        let mut unique = std::collections::BTreeSet::new();
        let mut total_references = 0usize;

        for (scene_id, raw_assets) in scene_assets {
            let mut local_seen = HashSet::new();
            let mut local_assets = Vec::new();
            for asset in raw_assets {
                let trimmed = asset.trim();
                if trimmed.is_empty() {
                    continue;
                }
                total_references = total_references.saturating_add(1);
                if local_seen.insert(trimmed.to_string()) {
                    local_assets.push(trimmed.to_string());
                }
                unique.insert(trimmed.to_string());
            }
            by_scene.insert(scene_id.clone(), local_assets);
        }

        let unique_assets: Vec<String> = unique.into_iter().collect();
        let deduped_references = unique_assets.len();
        let cache_hit_rate = if total_references == 0 {
            1.0
        } else {
            ((total_references.saturating_sub(deduped_references)) as f32)
                / (total_references as f32)
        };

        ScenePreloadPlan {
            by_scene,
            unique_assets,
            total_references,
            deduped_references,
            cache_hit_rate,
        }
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

fn is_allowed_by_extension(path: &Path, allowed: &HashSet<String>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| allowed.contains(&ext.to_ascii_lowercase()))
        .unwrap_or(false)
}

fn infer_asset_kind(path: &str) -> AssetKind {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp") => AssetKind::Image,
        Some("ogg" | "wav" | "flac" | "mp3" | "m4a") => AssetKind::Audio,
        _ => AssetKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn load_bytes_uses_cache_for_repeated_reads() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vn_assets_cache_{unique}"));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let asset_rel = PathBuf::from("audio").join("theme.ogg");
        let asset_path = root.join(&asset_rel);
        std::fs::create_dir_all(asset_path.parent().expect("parent path should exist"))
            .expect("asset parent directory should be created");
        std::fs::write(&asset_path, [1u8, 2, 3, 4]).expect("asset file should be written");

        let store = AssetStore::new(root.clone(), SecurityMode::Trusted, None, false)
            .expect("asset store should initialize")
            .with_cache_budget(1024);

        let first = store
            .load_bytes("audio/theme.ogg")
            .expect("first read should succeed");
        assert_eq!(first, vec![1, 2, 3, 4]);

        std::fs::remove_file(&asset_path).expect("asset file should be removed");

        let second = store
            .load_bytes("audio/theme.ogg")
            .expect("second read should be served from cache");
        assert_eq!(second, vec![1, 2, 3, 4]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn fingerprint_catalog_detects_duplicate_blobs_and_budget() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vn_assets_fingerprint_{unique}"));
        std::fs::create_dir_all(root.join("audio")).expect("audio dir");
        std::fs::create_dir_all(root.join("bg")).expect("bg dir");

        std::fs::write(root.join("audio/a.ogg"), [1u8, 2, 3]).expect("write a");
        std::fs::write(root.join("audio/b.ogg"), [1u8, 2, 3]).expect("write b duplicate");
        std::fs::write(root.join("bg/c.png"), [9u8, 8, 7, 6]).expect("write c");

        let catalog = AssetFingerprintCatalog::build(&root, &["ogg", "png"]).expect("catalog");
        assert_eq!(catalog.entries.len(), 3);
        assert_eq!(catalog.unique_blob_count(), 2);
        assert_eq!(catalog.duplicate_blob_count(), 1);

        let ok_budget = PlatformBudget {
            max_total_bytes: 32,
            max_assets: 8,
        };
        let report = catalog.budget_report(ok_budget);
        assert!(report.within_budget);
        assert_eq!(report.asset_count, 3);

        let tight_budget = PlatformBudget {
            max_total_bytes: 4,
            max_assets: 2,
        };
        let report = catalog.budget_report(tight_budget);
        assert!(!report.within_budget);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn asset_fingerprint_stability() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vn_assets_stability_{unique}"));
        std::fs::create_dir_all(root.join("audio")).expect("audio dir");
        std::fs::create_dir_all(root.join("bg")).expect("bg dir");
        std::fs::write(root.join("audio/theme.ogg"), [1u8, 3, 5, 7]).expect("write audio");
        std::fs::write(root.join("bg/room.png"), [9u8, 8, 7, 6]).expect("write image");

        let first = AssetFingerprintCatalog::build(&root, &["ogg", "png"]).expect("catalog 1");
        let second = AssetFingerprintCatalog::build(&root, &["ogg", "png"]).expect("catalog 2");

        assert_eq!(first.entries, second.entries);
        assert_eq!(first.dedup_groups, second.dedup_groups);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dedup_reduces_duplicate_load() {
        let scenes = std::collections::BTreeMap::from([
            (
                "intro".to_string(),
                vec![
                    "bg/room.png".to_string(),
                    "music/theme.ogg".to_string(),
                    "bg/room.png".to_string(),
                ],
            ),
            (
                "choice_a".to_string(),
                vec![
                    "bg/room.png".to_string(),
                    "music/theme.ogg".to_string(),
                    "sfx/click.ogg".to_string(),
                ],
            ),
        ]);

        let plan = AssetFingerprintCatalog::scene_preload_plan(&scenes);
        assert_eq!(plan.total_references, 6);
        assert_eq!(plan.deduped_references, 3);
        assert!(plan.cache_hit_rate > 0.4);
    }

    #[test]
    fn platform_budget_enforcement() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vn_assets_budget_platform_{unique}"));
        std::fs::create_dir_all(root.join("audio")).expect("audio dir");
        std::fs::write(root.join("audio/theme.ogg"), [1u8, 2, 3, 4, 5]).expect("write audio");
        let catalog = AssetFingerprintCatalog::build(&root, &["ogg"]).expect("catalog");

        let mobile_budget = PlatformTarget::Mobile.default_budget();
        assert!(catalog.budget_report(mobile_budget).within_budget);

        let tight = PlatformBudget {
            max_total_bytes: 2,
            max_assets: 1,
        };
        assert!(!catalog.budget_report(tight).within_budget);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scene_preload_hit_rate() {
        let scenes = std::collections::BTreeMap::from([
            (
                "s1".to_string(),
                vec!["bg/a.png".to_string(), "music/a.ogg".to_string()],
            ),
            (
                "s2".to_string(),
                vec!["bg/a.png".to_string(), "music/b.ogg".to_string()],
            ),
            (
                "s3".to_string(),
                vec!["bg/a.png".to_string(), "music/a.ogg".to_string()],
            ),
        ]);

        let plan = AssetFingerprintCatalog::scene_preload_plan(&scenes);
        assert_eq!(plan.total_references, 6);
        assert_eq!(plan.deduped_references, 3);
        assert!((plan.cache_hit_rate - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn transcode_recommendations_follow_platform_presets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vn_assets_transcode_{unique}"));
        std::fs::create_dir_all(root.join("audio")).expect("audio dir");
        std::fs::create_dir_all(root.join("bg")).expect("bg dir");
        std::fs::write(root.join("audio/theme.wav"), [1u8, 2, 3]).expect("write audio");
        std::fs::write(root.join("bg/room.png"), [7u8, 8, 9]).expect("write image");
        std::fs::write(root.join("bg/skip.webp"), [0u8, 1, 2]).expect("write webp");

        let catalog =
            AssetFingerprintCatalog::build(&root, &["wav", "png", "webp"]).expect("catalog");
        let mobile = catalog.transcode_recommendations(PlatformTarget::Mobile);
        assert!(mobile
            .iter()
            .any(|item| item.rel_path == "audio/theme.wav" && item.target_extension == "ogg"));
        assert!(mobile
            .iter()
            .any(|item| item.rel_path == "bg/room.png" && item.target_extension == "webp"));
        assert!(!mobile.iter().any(|item| item.rel_path == "bg/skip.webp"));

        let _ = std::fs::remove_dir_all(root);
    }
}
