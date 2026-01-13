use serde::{Deserialize, Serialize};

use super::SharedStr;

/// Choice prompt and options in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ChoiceRaw {
    pub prompt: String,
    pub options: Vec<ChoiceOptionRaw>,
}

/// Choice prompt and options with pre-resolved targets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceCompiled {
    pub prompt: SharedStr,
    pub options: Vec<ChoiceOptionCompiled>,
}

/// Choice option with label target in raw form.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ChoiceOptionRaw {
    pub text: String,
    pub target: String,
}

/// Choice option with pre-resolved target instruction pointer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOptionCompiled {
    pub text: SharedStr,
    pub target_ip: u32,
}
