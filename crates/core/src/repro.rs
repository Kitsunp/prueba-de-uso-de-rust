use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{VnError, VnResult};
use crate::event::{CondCompiled, EventCompiled};
use crate::{Engine, ResourceLimiter, ScriptRaw, SecurityPolicy};

pub const REPRO_CASE_SCHEMA: &str = "vnengine.repro_case.v1";
const DEFAULT_REPRO_MAX_STEPS: usize = 2048;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReproCase {
    pub schema: String,
    pub title: String,
    pub created_unix_ms: u64,
    pub script: ScriptRaw,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    #[serde(default)]
    pub choice_route: Vec<usize>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub oracle: ReproOracle,
    #[serde(default)]
    pub notes: Option<String>,
}

impl ReproCase {
    pub fn new(title: impl Into<String>, script: ScriptRaw) -> Self {
        Self {
            schema: REPRO_CASE_SCHEMA.to_string(),
            title: title.into(),
            created_unix_ms: now_unix_ms(),
            script,
            max_steps: DEFAULT_REPRO_MAX_STEPS,
            choice_route: Vec::new(),
            environment: default_environment_snapshot(),
            oracle: ReproOracle::default(),
            notes: None,
        }
    }

    pub fn from_json(payload: &str) -> VnResult<Self> {
        let case: Self = serde_json::from_str(payload).map_err(|err| VnError::Serialization {
            message: format!("invalid repro JSON: {err}"),
            src: payload.to_string(),
            span: (0, 0).into(),
        })?;
        if case.schema != REPRO_CASE_SCHEMA {
            return Err(VnError::InvalidScript(format!(
                "unsupported repro schema '{}'",
                case.schema
            )));
        }
        Ok(case)
    }

    pub fn to_json(&self) -> VnResult<String> {
        serde_json::to_string_pretty(self).map_err(|err| VnError::Serialization {
            message: err.to_string(),
            src: "".to_string(),
            span: (0, 0).into(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ReproOracle {
    #[serde(default)]
    pub expected_stop_reason: Option<ReproStopReason>,
    #[serde(default)]
    pub expected_event_ip: Option<u32>,
    #[serde(default)]
    pub expected_event_kind: Option<String>,
    #[serde(default)]
    pub monitors: Vec<ReproMonitor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReproStopReason {
    Finished,
    StepLimit,
    RuntimeError,
    CompileError,
    InitError,
}

impl ReproStopReason {
    pub fn label(&self) -> &'static str {
        match self {
            ReproStopReason::Finished => "finished",
            ReproStopReason::StepLimit => "step_limit",
            ReproStopReason::RuntimeError => "runtime_error",
            ReproStopReason::CompileError => "compile_error",
            ReproStopReason::InitError => "init_error",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReproMonitor {
    EventKindAtStep {
        monitor_id: String,
        step: usize,
        expected: String,
    },
    EventSignatureContains {
        monitor_id: String,
        step: usize,
        needle: String,
    },
    VisualBackgroundAtStep {
        monitor_id: String,
        step: usize,
        expected: Option<String>,
    },
    VisualMusicAtStep {
        monitor_id: String,
        step: usize,
        expected: Option<String>,
    },
    CharacterCountAtLeast {
        monitor_id: String,
        step: usize,
        min: usize,
    },
    StopMessageContains {
        monitor_id: String,
        needle: String,
    },
    StalledSignatureWindow {
        monitor_id: String,
        window: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReproStepTrace {
    pub step: usize,
    pub event_ip: u32,
    pub event_kind: String,
    pub event_signature: String,
    pub visual_background: Option<String>,
    pub visual_music: Option<String>,
    pub character_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReproMonitorResult {
    pub monitor_id: String,
    pub matched: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReproRunReport {
    pub schema: String,
    pub stop_reason: ReproStopReason,
    pub stop_message: String,
    pub failing_event_ip: Option<u32>,
    pub executed_steps: usize,
    pub max_steps: usize,
    pub steps: Vec<ReproStepTrace>,
    pub monitor_results: Vec<ReproMonitorResult>,
    pub matched_monitors: Vec<String>,
    pub signature_match: bool,
    pub oracle_triggered: bool,
}

impl ReproRunReport {
    pub fn to_json(&self) -> VnResult<String> {
        serde_json::to_string_pretty(self).map_err(|err| VnError::Serialization {
            message: err.to_string(),
            src: "".to_string(),
            span: (0, 0).into(),
        })
    }
}

pub fn run_repro_case(case: &ReproCase) -> ReproRunReport {
    run_repro_case_with_limits(case, SecurityPolicy::default(), ResourceLimiter::default())
}

pub fn run_repro_case_with_limits(
    case: &ReproCase,
    policy: SecurityPolicy,
    limits: ResourceLimiter,
) -> ReproRunReport {
    let mut traces = Vec::new();
    let mut failing_event_ip = None;
    let stop: (ReproStopReason, String) = match case.script.compile() {
        Ok(compiled) => match Engine::from_compiled(compiled, policy, limits) {
            Ok(mut engine) => {
                let mut steps = 0usize;
                let mut choice_cursor = 0usize;
                loop {
                    if steps >= case.max_steps {
                        break (
                            ReproStopReason::StepLimit,
                            format!("step limit reached ({})", case.max_steps),
                        );
                    }
                    let event = match engine.current_event() {
                        Ok(event) => event,
                        Err(VnError::EndOfScript) => {
                            break (ReproStopReason::Finished, "end of script".to_string())
                        }
                        Err(err) => {
                            break (
                                ReproStopReason::RuntimeError,
                                format!("current_event: {err}"),
                            );
                        }
                    };

                    let event_ip = engine.state().position;
                    traces.push(ReproStepTrace {
                        step: steps,
                        event_ip,
                        event_kind: event_kind_compiled(&event).to_string(),
                        event_signature: compiled_event_signature(&event),
                        visual_background: engine
                            .visual_state()
                            .background
                            .as_ref()
                            .map(|value| value.as_ref().to_string()),
                        visual_music: engine
                            .visual_state()
                            .music
                            .as_ref()
                            .map(|value| value.as_ref().to_string()),
                        character_count: engine.visual_state().characters.len(),
                    });

                    let step_result = match &event {
                        EventCompiled::Choice(choice) => {
                            let selected = case
                                .choice_route
                                .get(choice_cursor)
                                .copied()
                                .unwrap_or(0)
                                .min(choice.options.len().saturating_sub(1));
                            choice_cursor = choice_cursor.saturating_add(1);
                            engine.choose(selected).map(|_| ())
                        }
                        _ => engine.step().map(|_| ()),
                    };
                    if let Err(err) = step_result {
                        failing_event_ip = Some(event_ip);
                        break (ReproStopReason::RuntimeError, format!("step failed: {err}"));
                    }
                    steps = steps.saturating_add(1);
                }
            }
            Err(err) => (
                ReproStopReason::InitError,
                format!("engine init failed: {err}"),
            ),
        },
        Err(err) => (
            ReproStopReason::CompileError,
            format!("compile failed: {err}"),
        ),
    };

    let (stop_reason, stop_message) = stop;
    let signature_match = matches_expected_signature(
        &case.oracle,
        &stop_reason,
        failing_event_ip,
        traces.as_slice(),
    );
    let monitor_results =
        evaluate_monitors(&case.oracle.monitors, &stop_message, traces.as_slice());
    let matched_monitors = monitor_results
        .iter()
        .filter(|result| result.matched)
        .map(|result| result.monitor_id.clone())
        .collect::<Vec<_>>();
    let oracle_triggered = signature_match || !matched_monitors.is_empty();

    ReproRunReport {
        schema: "vnengine.repro_run_report.v1".to_string(),
        stop_reason,
        stop_message,
        failing_event_ip,
        executed_steps: traces.len(),
        max_steps: case.max_steps,
        steps: traces,
        monitor_results,
        matched_monitors,
        signature_match,
        oracle_triggered,
    }
}

fn matches_expected_signature(
    oracle: &ReproOracle,
    stop_reason: &ReproStopReason,
    failing_event_ip: Option<u32>,
    steps: &[ReproStepTrace],
) -> bool {
    if oracle.expected_stop_reason.is_none()
        && oracle.expected_event_ip.is_none()
        && oracle.expected_event_kind.is_none()
    {
        return false;
    }

    if let Some(expected) = &oracle.expected_stop_reason {
        if expected != stop_reason {
            return false;
        }
    }

    if let Some(expected_ip) = oracle.expected_event_ip {
        if failing_event_ip == Some(expected_ip) {
            // ok
        } else if !steps.iter().any(|step| step.event_ip == expected_ip) {
            return false;
        }
    }

    if let Some(expected_kind) = oracle
        .expected_event_kind
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    {
        let kind_match = if let Some(expected_ip) = oracle.expected_event_ip {
            steps
                .iter()
                .find(|step| step.event_ip == expected_ip)
                .map(|step| step.event_kind.eq_ignore_ascii_case(expected_kind.as_str()))
                .unwrap_or(false)
        } else {
            steps
                .iter()
                .any(|step| step.event_kind.eq_ignore_ascii_case(expected_kind.as_str()))
        };
        if !kind_match {
            return false;
        }
    }

    true
}

fn evaluate_monitors(
    monitors: &[ReproMonitor],
    stop_message: &str,
    steps: &[ReproStepTrace],
) -> Vec<ReproMonitorResult> {
    monitors
        .iter()
        .map(|monitor| match monitor {
            ReproMonitor::EventKindAtStep {
                monitor_id,
                step,
                expected,
            } => {
                let expected_norm = expected.trim().to_ascii_lowercase();
                let matched = steps
                    .get(*step)
                    .map(|trace| {
                        trace
                            .event_kind
                            .eq_ignore_ascii_case(expected_norm.as_str())
                    })
                    .unwrap_or(false);
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("step={} expected_kind='{}'", step, expected_norm),
                }
            }
            ReproMonitor::EventSignatureContains {
                monitor_id,
                step,
                needle,
            } => {
                let matched = steps
                    .get(*step)
                    .map(|trace| trace.event_signature.contains(needle))
                    .unwrap_or(false);
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("step={} needle='{}'", step, needle),
                }
            }
            ReproMonitor::VisualBackgroundAtStep {
                monitor_id,
                step,
                expected,
            } => {
                let got = steps
                    .get(*step)
                    .and_then(|trace| trace.visual_background.clone());
                let matched = got == *expected;
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("step={} expected_bg={:?} got={:?}", step, expected, got),
                }
            }
            ReproMonitor::VisualMusicAtStep {
                monitor_id,
                step,
                expected,
            } => {
                let got = steps
                    .get(*step)
                    .and_then(|trace| trace.visual_music.clone());
                let matched = got == *expected;
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("step={} expected_music={:?} got={:?}", step, expected, got),
                }
            }
            ReproMonitor::CharacterCountAtLeast {
                monitor_id,
                step,
                min,
            } => {
                let got = steps
                    .get(*step)
                    .map(|trace| trace.character_count)
                    .unwrap_or(0);
                let matched = got >= *min;
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("step={} min_chars={} got={}", step, min, got),
                }
            }
            ReproMonitor::StopMessageContains { monitor_id, needle } => {
                let matched = stop_message.contains(needle);
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("stop_message contains '{}'", needle),
                }
            }
            ReproMonitor::StalledSignatureWindow { monitor_id, window } => {
                let window_size = (*window).max(2);
                let mut matched = false;
                let mut streak = 1usize;
                for idx in 1..steps.len() {
                    if steps[idx].event_signature == steps[idx - 1].event_signature {
                        streak = streak.saturating_add(1);
                        if streak >= window_size {
                            matched = true;
                            break;
                        }
                    } else {
                        streak = 1;
                    }
                }
                ReproMonitorResult {
                    monitor_id: monitor_id.clone(),
                    matched,
                    detail: format!("window={window_size}"),
                }
            }
        })
        .collect()
}

fn event_kind_compiled(event: &EventCompiled) -> &'static str {
    match event {
        EventCompiled::Dialogue(_) => "dialogue",
        EventCompiled::Choice(_) => "choice",
        EventCompiled::Scene(_) => "scene",
        EventCompiled::Jump { .. } => "jump",
        EventCompiled::SetFlag { .. } => "set_flag",
        EventCompiled::SetVar { .. } => "set_var",
        EventCompiled::JumpIf { .. } => "jump_if",
        EventCompiled::Patch(_) => "patch",
        EventCompiled::ExtCall { .. } => "ext_call",
        EventCompiled::AudioAction(_) => "audio_action",
        EventCompiled::Transition(_) => "transition",
        EventCompiled::SetCharacterPosition(_) => "set_character_position",
    }
}

fn compiled_event_signature(event: &EventCompiled) -> String {
    match event {
        EventCompiled::Dialogue(dialogue) => {
            format!(
                "dialogue|{}|{}",
                dialogue.speaker.as_ref(),
                dialogue.text.as_ref()
            )
        }
        EventCompiled::Choice(choice) => {
            format!("choice|{}|{}", choice.prompt.as_ref(), choice.options.len())
        }
        EventCompiled::Scene(scene) => format!(
            "scene|bg={:?}|music={:?}|chars={}",
            scene.background.as_deref(),
            scene.music.as_deref(),
            scene.characters.len()
        ),
        EventCompiled::Jump { .. } => "jump".to_string(),
        EventCompiled::SetFlag { value, .. } => format!("set_flag|{}", value),
        EventCompiled::SetVar { value, .. } => format!("set_var|{}", value),
        EventCompiled::JumpIf { cond, .. } => format!("jump_if|{}", cond_signature(cond)),
        EventCompiled::Patch(patch) => format!(
            "patch|bg={:?}|music={:?}|add={}|upd={}|rm={}",
            patch.background.as_deref(),
            patch.music.as_deref(),
            patch.add.len(),
            patch.update.len(),
            patch.remove.len()
        ),
        EventCompiled::ExtCall { command, args } => {
            format!("ext_call|{}|{}", command, args.len())
        }
        EventCompiled::AudioAction(action) => format!(
            "audio|{}|{}|asset={:?}|vol={}|fade={:?}|loop={:?}",
            compiled_audio_channel(action.channel),
            compiled_audio_action(action.action),
            action.asset.as_deref(),
            fmt_opt_f32(action.volume),
            action.fade_duration_ms,
            action.loop_playback
        ),
        EventCompiled::Transition(trans) => format!(
            "transition|{}|{}|{:?}",
            compiled_transition_kind(trans.kind),
            trans.duration_ms,
            trans.color.as_deref()
        ),
        EventCompiled::SetCharacterPosition(pos) => format!(
            "set_character_position|{}|{}|{}|{}",
            pos.name.as_ref(),
            pos.x,
            pos.y,
            fmt_opt_f32(pos.scale)
        ),
    }
}

fn cond_signature(cond: &CondCompiled) -> String {
    match cond {
        CondCompiled::Flag { is_set, .. } => format!("flag|{}", is_set),
        CondCompiled::VarCmp { op, value, .. } => format!("var|{:?}|{}", op, value),
    }
}

fn compiled_audio_channel(channel: u8) -> &'static str {
    match channel {
        0 => "bgm",
        1 => "sfx",
        2 => "voice",
        _ => "unknown",
    }
}

fn compiled_audio_action(action: u8) -> &'static str {
    match action {
        0 => "play",
        1 => "stop",
        2 => "fade_out",
        _ => "unknown",
    }
}

fn compiled_transition_kind(kind: u8) -> &'static str {
    match kind {
        0 => "fade",
        1 => "dissolve",
        2 => "cut",
        _ => "unknown",
    }
}

fn fmt_opt_f32(value: Option<f32>) -> String {
    match value {
        Some(v) => format!("{v:.3}"),
        None => "none".to_string(),
    }
}

fn default_max_steps() -> usize {
    DEFAULT_REPRO_MAX_STEPS
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn default_environment_snapshot() -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("os".to_string(), std::env::consts::OS.to_string());
    env.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    env.insert("family".to_string(), std::env::consts::FAMILY.to_string());
    env
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{ChoiceOptionRaw, ChoiceRaw, DialogueRaw, EventRaw};

    use super::*;

    fn linear_script() -> ScriptRaw {
        ScriptRaw::new(
            vec![EventRaw::Dialogue(DialogueRaw {
                speaker: "Narrator".to_string(),
                text: "Hola".to_string(),
            })],
            BTreeMap::from([("start".to_string(), 0usize)]),
        )
    }

    #[test]
    fn repro_case_json_roundtrip() {
        let mut case = ReproCase::new("repro", linear_script());
        case.oracle.expected_stop_reason = Some(ReproStopReason::Finished);
        case.oracle.monitors.push(ReproMonitor::EventKindAtStep {
            monitor_id: "m_event".to_string(),
            step: 0,
            expected: "dialogue".to_string(),
        });
        let payload = case.to_json().expect("serialize repro");
        let loaded = ReproCase::from_json(&payload).expect("deserialize repro");
        assert_eq!(loaded.schema, REPRO_CASE_SCHEMA);
        assert_eq!(loaded.oracle.monitors.len(), 1);
    }

    #[test]
    fn run_repro_case_matches_monitor() {
        let mut case = ReproCase::new("monitor", linear_script());
        case.oracle.monitors.push(ReproMonitor::EventKindAtStep {
            monitor_id: "kind0".to_string(),
            step: 0,
            expected: "dialogue".to_string(),
        });
        let report = run_repro_case(&case);
        assert_eq!(report.stop_reason, ReproStopReason::Finished);
        assert!(report.oracle_triggered);
        assert!(report.matched_monitors.iter().any(|id| id == "kind0"));
    }

    #[test]
    fn run_repro_case_honors_choice_route() {
        let script = ScriptRaw::new(
            vec![
                EventRaw::Choice(ChoiceRaw {
                    prompt: "Pick".to_string(),
                    options: vec![
                        ChoiceOptionRaw {
                            text: "A".to_string(),
                            target: "left".to_string(),
                        },
                        ChoiceOptionRaw {
                            text: "B".to_string(),
                            target: "right".to_string(),
                        },
                    ],
                }),
                EventRaw::Dialogue(DialogueRaw {
                    speaker: "L".to_string(),
                    text: "Left".to_string(),
                }),
                EventRaw::Dialogue(DialogueRaw {
                    speaker: "R".to_string(),
                    text: "Right".to_string(),
                }),
            ],
            BTreeMap::from([
                ("start".to_string(), 0usize),
                ("left".to_string(), 1usize),
                ("right".to_string(), 2usize),
            ]),
        );
        let mut case = ReproCase::new("choice", script);
        case.choice_route = vec![1];
        case.oracle
            .monitors
            .push(ReproMonitor::EventSignatureContains {
                monitor_id: "right_dialogue".to_string(),
                step: 1,
                needle: "Right".to_string(),
            });
        let report = run_repro_case(&case);
        assert!(report.oracle_triggered);
        assert!(report
            .matched_monitors
            .iter()
            .any(|id| id == "right_dialogue"));
    }
}
