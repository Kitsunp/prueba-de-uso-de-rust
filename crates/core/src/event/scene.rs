use serde::{Deserialize, Serialize};

use super::SharedStr;

/// Scene update payload in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SceneUpdateRaw {
    pub background: Option<String>,
    pub music: Option<String>,
    pub characters: Vec<CharacterPlacementRaw>,
}

/// Scene update payload with interned strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneUpdateCompiled {
    pub background: Option<SharedStr>,
    pub music: Option<SharedStr>,
    pub characters: Vec<CharacterPlacementCompiled>,
}

/// Character placement in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct CharacterPlacementRaw {
    pub name: String,
    pub expression: Option<String>,
    pub position: Option<String>,
}

/// Character placement with interned strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterPlacementCompiled {
    pub name: SharedStr,
    pub expression: Option<SharedStr>,
    pub position: Option<SharedStr>,
}

/// Character patch for partial updates.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct CharacterPatchRaw {
    pub name: String,
    pub expression: Option<String>,
    pub position: Option<String>,
}

/// Character patch with interned strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterPatchCompiled {
    pub name: SharedStr,
    pub expression: Option<SharedStr>,
    pub position: Option<SharedStr>,
}

/// Scene patch in raw form (handling partial updates).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ScenePatchRaw {
    pub background: Option<String>,
    pub music: Option<String>,
    #[serde(default)]
    pub add: Vec<CharacterPlacementRaw>,
    #[serde(default)]
    pub update: Vec<CharacterPatchRaw>,
    #[serde(default)]
    pub remove: Vec<String>,
}

/// Scene patch with interned strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenePatchCompiled {
    pub background: Option<SharedStr>,
    pub music: Option<SharedStr>,
    pub add: Vec<CharacterPlacementCompiled>,
    pub update: Vec<CharacterPatchCompiled>,
    pub remove: Vec<SharedStr>,
}
