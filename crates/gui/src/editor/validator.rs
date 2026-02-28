use crate::editor::execution_contract;
use crate::editor::node_graph::NodeGraph;
use crate::editor::node_types::StoryNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPhase {
    Graph,
    Compile,
    Runtime,
    DryRun,
}

impl ValidationPhase {
    pub fn label(self) -> &'static str {
        match self {
            ValidationPhase::Graph => "GRAPH",
            ValidationPhase::Compile => "COMPILE",
            ValidationPhase::Runtime => "RUNTIME",
            ValidationPhase::DryRun => "DRYRUN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCode {
    MissingStart,
    MultipleStart,
    UnreachableNode,
    DeadEnd,
    ChoiceNoOptions,
    ChoiceOptionUnlinked,
    ChoicePortOutOfRange,
    AudioAssetMissing,
    AudioAssetEmpty,
    AssetReferenceMissing,
    SceneBackgroundEmpty,
    UnsafeAssetPath,
    InvalidAudioChannel,
    InvalidAudioAction,
    InvalidAudioVolume,
    InvalidAudioFade,
    InvalidCharacterScale,
    InvalidTransitionDuration,
    InvalidTransitionKind,
    EmptyCharacterName,
    EmptySpeakerName,
    EmptyJumpTarget,
    ContractUnsupportedExport,
    GenericEventUnchecked,
    CompileError,
    RuntimeInitError,
    DryRunUnreachableCompiled,
    DryRunStepLimit,
    DryRunRuntimeError,
    DryRunParityMismatch,
    DryRunFinished,
}

impl LintCode {
    pub fn label(self) -> &'static str {
        match self {
            LintCode::MissingStart => "VAL_START_MISSING",
            LintCode::MultipleStart => "VAL_START_MULTIPLE",
            LintCode::UnreachableNode => "VAL_UNREACHABLE",
            LintCode::DeadEnd => "VAL_DEAD_END",
            LintCode::ChoiceNoOptions => "VAL_CHOICE_EMPTY",
            LintCode::ChoiceOptionUnlinked => "VAL_CHOICE_UNLINKED",
            LintCode::ChoicePortOutOfRange => "VAL_CHOICE_PORT_OOB",
            LintCode::AudioAssetMissing => "VAL_AUDIO_MISSING",
            LintCode::AudioAssetEmpty => "VAL_AUDIO_EMPTY",
            LintCode::AssetReferenceMissing => "VAL_ASSET_NOT_FOUND",
            LintCode::SceneBackgroundEmpty => "VAL_SCENE_BG_EMPTY",
            LintCode::UnsafeAssetPath => "VAL_ASSET_UNSAFE_PATH",
            LintCode::InvalidAudioChannel => "VAL_AUDIO_CHANNEL_INVALID",
            LintCode::InvalidAudioAction => "VAL_AUDIO_ACTION_INVALID",
            LintCode::InvalidAudioVolume => "VAL_AUDIO_VOLUME_INVALID",
            LintCode::InvalidAudioFade => "VAL_AUDIO_FADE_INVALID",
            LintCode::InvalidCharacterScale => "VAL_SCALE_INVALID",
            LintCode::InvalidTransitionDuration => "VAL_TRANSITION_DURATION",
            LintCode::InvalidTransitionKind => "VAL_TRANSITION_KIND_INVALID",
            LintCode::EmptyCharacterName => "VAL_CHARACTER_NAME_EMPTY",
            LintCode::EmptySpeakerName => "VAL_SPEAKER_EMPTY",
            LintCode::EmptyJumpTarget => "VAL_JUMP_EMPTY",
            LintCode::ContractUnsupportedExport => "VAL_CONTRACT_EXPORT_UNSUPPORTED",
            LintCode::GenericEventUnchecked => "VAL_GENERIC_UNCHECKED",
            LintCode::CompileError => "CMP_SCRIPT_ERROR",
            LintCode::RuntimeInitError => "CMP_RUNTIME_INIT",
            LintCode::DryRunUnreachableCompiled => "DRY_UNREACHABLE",
            LintCode::DryRunStepLimit => "DRY_STEP_LIMIT",
            LintCode::DryRunRuntimeError => "DRY_RUNTIME_ERROR",
            LintCode::DryRunParityMismatch => "DRY_PARITY_MISMATCH",
            LintCode::DryRunFinished => "DRY_FINISHED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub node_id: Option<u32>,
    pub event_ip: Option<u32>,
    pub severity: LintSeverity,
    pub phase: ValidationPhase,
    pub code: LintCode,
    pub message: String,
}

impl LintIssue {
    pub fn diagnostic_id(&self) -> String {
        let node = self
            .node_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "global".to_string());
        let event_ip = self
            .event_ip
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "na".to_string());
        format!(
            "{}:{}:{}:{}",
            self.phase.label(),
            self.code.label(),
            node,
            event_ip
        )
    }

    pub fn new(
        node_id: Option<u32>,
        severity: LintSeverity,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            event_ip: None,
            severity,
            phase,
            code,
            message: message.into(),
        }
    }

    pub fn with_event_ip(mut self, event_ip: Option<u32>) -> Self {
        self.event_ip = event_ip;
        self
    }

    pub fn error(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Error, phase, code, message)
    }

    pub fn warning(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Warning, phase, code, message)
    }

    pub fn info(
        node_id: Option<u32>,
        phase: ValidationPhase,
        code: LintCode,
        message: impl Into<String>,
    ) -> Self {
        Self::new(node_id, LintSeverity::Info, phase, code, message)
    }
}

pub fn validate(graph: &NodeGraph) -> Vec<LintIssue> {
    validate_with_asset_probe(graph, rules::default_asset_exists)
}

pub fn validate_with_asset_probe<F>(graph: &NodeGraph, asset_exists: F) -> Vec<LintIssue>
where
    F: Fn(&str) -> bool,
{
    rules::validate_with_asset_probe_impl(graph, asset_exists)
}

mod rules;

#[cfg(test)]
#[path = "tests/validator_tests.rs"]
mod tests;
