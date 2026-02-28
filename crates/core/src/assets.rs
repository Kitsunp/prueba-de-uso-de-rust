use std::hash::{Hash, Hasher};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Opaque asset identifier.
///
/// Uses a deterministic non-cryptographic u64 hash for stable IDs across runs.
/// This is not collision-resistant in the cryptographic sense.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AssetId(u64);

impl AssetId {
    /// Creates an `AssetId` from a path string using deterministic FNV-1a 64-bit.
    ///
    /// Intended for stable lookup keys, not for security decisions.
    pub fn from_path(path: &str) -> Self {
        let mut hasher = FnvHasher64::default();
        path.hash(&mut hasher);
        AssetId(hasher.finish())
    }

    /// Returns the raw u64 value for serialization purposes only.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Fowler-Noll-Vo 1a 64-bit Hasher.
/// Used for deterministic AssetId generation independent of process seed.
struct FnvHasher64 {
    state: u64,
}

impl Default for FnvHasher64 {
    fn default() -> Self {
        Self {
            state: 0xcbf29ce484222325,
        }
    }
}

impl Hasher for FnvHasher64 {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AssetManifest {
    pub entries: std::collections::BTreeMap<String, String>,
}
