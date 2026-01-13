use serde::{Deserialize, Serialize};

use super::SharedStr;

/// Dialogue line with speaker and text in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct DialogueRaw {
    pub speaker: String,
    pub text: String,
}

/// Dialogue line with interned speaker and text.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueCompiled {
    pub speaker: SharedStr,
    pub text: SharedStr,
}
