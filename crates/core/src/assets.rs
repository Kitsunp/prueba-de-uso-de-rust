use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Opaque asset identifier. Implementation uses u64 hash for collision resistance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AssetId(u64);

impl AssetId {
    /// Creates an AssetId from a path string.
    pub fn from_path(path: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    /// Returns the raw u64 value for serialization purposes only.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub entries: BTreeMap<String, String>,
}
