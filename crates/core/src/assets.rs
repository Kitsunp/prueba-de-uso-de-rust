use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AssetId(pub u32);

impl AssetId {
    pub fn from_path(path: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        // Intentional truncation to u32.
        // TODO: Consider upgrading AssetId to u64 for better collision resistance if needed.
        AssetId((hasher.finish() & 0xFFFFFFFF) as u32)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub entries: BTreeMap<String, String>,
}
