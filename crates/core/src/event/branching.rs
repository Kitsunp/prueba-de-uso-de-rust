use serde::{Deserialize, Serialize};

/// Condition for conditional jumps (raw form).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CondRaw {
    Flag { key: String, is_set: bool },
    VarCmp { key: String, op: CmpOp, value: i32 },
}

/// Condition for conditional jumps (compiled form).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CondCompiled {
    Flag { flag_id: u32, is_set: bool },
    VarCmp { var_id: u32, op: CmpOp, value: i32 },
}

/// Comparison operators for variable conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
