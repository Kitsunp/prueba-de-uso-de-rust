use std::collections::{BTreeMap, HashMap};

use crate::editor::{
    node_graph::NodeGraph,
    script_sync,
    validator::{self, LintCode, LintIssue, LintSeverity, ValidationPhase},
};
use visual_novel_engine::{Engine, EventCompiled, EventRaw, ScriptRaw, StoryGraph};

const DRY_RUN_MAX_STEPS: usize = 2048;
const REPRO_DEFAULT_RADIUS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationPhase {
    GraphSync,
    GraphValidation,
    ScriptCompile,
    RuntimeInit,
    DryRun,
}

impl CompilationPhase {
    pub fn label(self) -> &'static str {
        match self {
            CompilationPhase::GraphSync => "GRAPH_SYNC",
            CompilationPhase::GraphValidation => "GRAPH_VALIDATION",
            CompilationPhase::ScriptCompile => "SCRIPT_COMPILE",
            CompilationPhase::RuntimeInit => "RUNTIME_INIT",
            CompilationPhase::DryRun => "DRY_RUN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaseTrace {
    pub phase: CompilationPhase,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRunStopReason {
    Finished,
    StepLimit,
    RuntimeError,
}

impl DryRunStopReason {
    pub fn label(self) -> &'static str {
        match self {
            DryRunStopReason::Finished => "finished",
            DryRunStopReason::StepLimit => "step_limit",
            DryRunStopReason::RuntimeError => "runtime_error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DryRunStepTrace {
    pub step: usize,
    pub event_ip: u32,
    pub event_kind: String,
    pub visual_background: Option<String>,
    pub visual_music: Option<String>,
    pub character_count: usize,
}

#[derive(Debug, Clone)]
pub struct DryRunReport {
    pub max_steps: usize,
    pub executed_steps: usize,
    pub stop_reason: DryRunStopReason,
    pub stop_message: String,
    pub failing_event_ip: Option<u32>,
    pub steps: Vec<DryRunStepTrace>,
}

impl DryRunReport {
    pub fn first_event_ip(&self) -> Option<u32> {
        self.steps.first().map(|step| step.event_ip)
    }

    pub fn minimal_repro_script(&self, script: &ScriptRaw, radius: usize) -> Option<ScriptRaw> {
        let candidate_ip = self.failing_event_ip.or_else(|| self.first_event_ip())?;
        Some(build_minimal_repro_script(script, candidate_ip, radius))
    }
}

pub struct CompilationResult {
    pub script: ScriptRaw,
    pub engine_result: Result<Engine, String>,
    pub issues: Vec<LintIssue>,
    pub phase_trace: Vec<PhaseTrace>,
    pub dry_run_report: Option<DryRunReport>,
}

impl CompilationResult {
    pub fn minimal_repro_script(&self) -> Option<ScriptRaw> {
        self.dry_run_report
            .as_ref()
            .and_then(|report| report.minimal_repro_script(&self.script, REPRO_DEFAULT_RADIUS))
    }
}

pub fn compile_project(graph: &NodeGraph) -> CompilationResult {
    let mut phase_trace = Vec::new();

    phase_trace.push(PhaseTrace {
        phase: CompilationPhase::GraphSync,
        ok: true,
        detail: "Graph converted to ScriptRaw".to_string(),
    });
    let script = script_sync::to_script(graph);

    let mut issues = validator::validate(graph);
    phase_trace.push(PhaseTrace {
        phase: CompilationPhase::GraphValidation,
        ok: !issues.iter().any(|i| i.severity == LintSeverity::Error),
        detail: format!("{} issue(s) from graph validation", issues.len()),
    });

    let mut dry_run_report = None;
    let engine_result = match script.compile() {
        Ok(compiled) => {
            phase_trace.push(PhaseTrace {
                phase: CompilationPhase::ScriptCompile,
                ok: true,
                detail: "ScriptRaw compiled successfully".to_string(),
            });

            let story_graph = StoryGraph::from_script(&compiled);
            let unreachable = story_graph.unreachable_nodes();
            if !unreachable.is_empty() {
                issues.push(LintIssue::warning(
                    None,
                    ValidationPhase::DryRun,
                    LintCode::DryRunUnreachableCompiled,
                    format!(
                        "Dry Run detected {} unreachable compiled event(s)",
                        unreachable.len()
                    ),
                ));
            }

            match Engine::from_compiled(
                compiled.clone(),
                visual_novel_engine::SecurityPolicy::default(),
                visual_novel_engine::ResourceLimiter::default(),
            ) {
                Ok(engine) => {
                    phase_trace.push(PhaseTrace {
                        phase: CompilationPhase::RuntimeInit,
                        ok: true,
                        detail: "Engine initialized".to_string(),
                    });

                    let outcome = run_dry_run(engine.clone());
                    dry_run_report = Some(outcome.report.clone());
                    issues.extend(outcome.issues);
                    issues.extend(check_preview_runtime_parity(&script, &outcome.report));

                    let dry_run_errors = issues
                        .iter()
                        .filter(|i| {
                            i.phase == ValidationPhase::DryRun && i.severity == LintSeverity::Error
                        })
                        .count();
                    phase_trace.push(PhaseTrace {
                        phase: CompilationPhase::DryRun,
                        ok: dry_run_errors == 0,
                        detail: format!("Dry run complete ({} dry-run error(s))", dry_run_errors),
                    });

                    Ok(engine)
                }
                Err(e) => {
                    issues.push(LintIssue::error(
                        None,
                        ValidationPhase::Runtime,
                        LintCode::RuntimeInitError,
                        format!("Runtime initialization failed: {}", e),
                    ));
                    phase_trace.push(PhaseTrace {
                        phase: CompilationPhase::RuntimeInit,
                        ok: false,
                        detail: e.to_string(),
                    });
                    Err(format!("Runtime Init Error: {}", e))
                }
            }
        }
        Err(e) => {
            issues.push(LintIssue::error(
                None,
                ValidationPhase::Compile,
                LintCode::CompileError,
                format!("Compilation Error: {}", e),
            ));
            phase_trace.push(PhaseTrace {
                phase: CompilationPhase::ScriptCompile,
                ok: false,
                detail: e.to_string(),
            });
            Err(format!("Compilation Failed: {}", e))
        }
    };

    CompilationResult {
        script,
        engine_result,
        issues,
        phase_trace,
        dry_run_report,
    }
}

#[derive(Debug, Clone)]
struct DryRunOutcome {
    issues: Vec<LintIssue>,
    report: DryRunReport,
}

fn run_dry_run(mut engine: Engine) -> DryRunOutcome {
    let mut issues = Vec::new();
    let mut traces = Vec::new();
    let mut steps = 0usize;
    let mut failing_event_ip = None;

    let (stop_reason, stop_message) = loop {
        if steps >= DRY_RUN_MAX_STEPS {
            let stop_message = format!(
                "Dry Run reached {} steps; possible loop or blocking flow",
                DRY_RUN_MAX_STEPS
            );
            issues.push(
                LintIssue::warning(
                    Some(engine.state().position),
                    ValidationPhase::DryRun,
                    LintCode::DryRunStepLimit,
                    stop_message.clone(),
                )
                .with_event_ip(Some(engine.state().position)),
            );
            break (DryRunStopReason::StepLimit, stop_message);
        }

        let ip = engine.state().position;
        let event = match engine.current_event() {
            Ok(event) => event,
            Err(_) => {
                let msg = format!("Dry Run finished in {} step(s)", steps);
                issues.push(LintIssue::info(
                    None,
                    ValidationPhase::DryRun,
                    LintCode::DryRunFinished,
                    msg.clone(),
                ));
                break (DryRunStopReason::Finished, msg);
            }
        };

        traces.push(DryRunStepTrace {
            step: steps,
            event_ip: ip,
            event_kind: event_kind_compiled(&event).to_string(),
            visual_background: engine
                .state()
                .visual
                .background
                .as_ref()
                .map(|value| value.as_ref().to_string()),
            visual_music: engine
                .state()
                .visual
                .music
                .as_ref()
                .map(|value| value.as_ref().to_string()),
            character_count: engine.state().visual.characters.len(),
        });

        let run_result = match event {
            EventCompiled::Choice(choice) => {
                if choice.options.is_empty() {
                    Err(visual_novel_engine::VnError::InvalidChoice)
                } else {
                    engine.choose(0).map(|_| ())
                }
            }
            EventCompiled::ExtCall { .. } => engine.resume(),
            _ => engine.step().map(|_| ()),
        };

        if let Err(err) = run_result {
            let stop_message = format!("Dry Run runtime error at ip {}: {}", ip, err);
            failing_event_ip = Some(ip);
            issues.push(
                LintIssue::error(
                    Some(ip),
                    ValidationPhase::DryRun,
                    LintCode::DryRunRuntimeError,
                    stop_message.clone(),
                )
                .with_event_ip(Some(ip)),
            );
            break (DryRunStopReason::RuntimeError, stop_message);
        }

        steps += 1;
    };

    DryRunOutcome {
        issues,
        report: DryRunReport {
            max_steps: DRY_RUN_MAX_STEPS,
            executed_steps: steps,
            stop_reason,
            stop_message,
            failing_event_ip,
            steps: traces,
        },
    }
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

fn event_kind_raw(event: &EventRaw) -> &'static str {
    match event {
        EventRaw::Dialogue(_) => "dialogue",
        EventRaw::Choice(_) => "choice",
        EventRaw::Scene(_) => "scene",
        EventRaw::Jump { .. } => "jump",
        EventRaw::SetFlag { .. } => "set_flag",
        EventRaw::SetVar { .. } => "set_var",
        EventRaw::JumpIf { .. } => "jump_if",
        EventRaw::Patch(_) => "patch",
        EventRaw::ExtCall { .. } => "ext_call",
        EventRaw::AudioAction(_) => "audio_action",
        EventRaw::Transition(_) => "transition",
        EventRaw::SetCharacterPosition(_) => "set_character_position",
    }
}

fn collect_raw_sequence(script: &ScriptRaw, max_steps: usize) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut steps = 0usize;
    let mut ip = match script.start_index() {
        Ok(idx) => idx,
        Err(_) => return out,
    };

    while ip < script.events.len() && steps < max_steps {
        let event = &script.events[ip];
        out.push((ip as u32, event_kind_raw(event).to_string()));

        match event {
            EventRaw::Jump { target } => {
                let Some(target_ip) = script.labels.get(target).copied() else {
                    break;
                };
                ip = target_ip;
            }
            EventRaw::Choice(choice) => {
                let Some(target) = choice.options.first().map(|opt| opt.target.as_str()) else {
                    break;
                };
                let Some(target_ip) = script.labels.get(target).copied() else {
                    break;
                };
                ip = target_ip;
            }
            EventRaw::JumpIf { .. } => ip += 1,
            _ => ip += 1,
        }

        steps += 1;
    }

    out
}

fn check_preview_runtime_parity(script: &ScriptRaw, report: &DryRunReport) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let runtime_sequence: Vec<(u32, String)> = report
        .steps
        .iter()
        .map(|step| (step.event_ip, step.event_kind.clone()))
        .collect();
    let raw_sequence = collect_raw_sequence(script, report.max_steps);
    let overlap = runtime_sequence.len().min(raw_sequence.len());

    for idx in 0..overlap {
        let (runtime_ip, runtime_kind) = &runtime_sequence[idx];
        let (raw_ip, raw_kind) = &raw_sequence[idx];
        if runtime_kind != raw_kind {
            issues.push(
                LintIssue::warning(
                    None,
                    ValidationPhase::DryRun,
                    LintCode::DryRunParityMismatch,
                    format!(
                        "Parity mismatch at step {}: preview {}@{} vs runtime {}@{}",
                        idx, raw_kind, raw_ip, runtime_kind, runtime_ip
                    ),
                )
                .with_event_ip(Some(*runtime_ip)),
            );
            break;
        }
    }

    if runtime_sequence.len() != raw_sequence.len() {
        let mismatch_step = overlap;
        let mismatch_ip = runtime_sequence
            .get(mismatch_step)
            .map(|entry| entry.0)
            .or_else(|| raw_sequence.get(mismatch_step).map(|entry| entry.0));
        issues.push(
            LintIssue::warning(
                None,
                ValidationPhase::DryRun,
                LintCode::DryRunParityMismatch,
                format!(
                    "Parity length mismatch: preview={} runtime={}",
                    raw_sequence.len(),
                    runtime_sequence.len()
                ),
            )
            .with_event_ip(mismatch_ip),
        );
    }

    issues
}

fn build_minimal_repro_script(script: &ScriptRaw, failure_ip: u32, radius: usize) -> ScriptRaw {
    if script.events.is_empty() {
        return ScriptRaw::new(Vec::new(), BTreeMap::new());
    }

    let failure_idx = (failure_ip as usize).min(script.events.len().saturating_sub(1));
    let start_idx = failure_idx.saturating_sub(radius);
    let end_idx = (failure_idx + radius + 1).min(script.events.len());
    let mut events = script.events[start_idx..end_idx].to_vec();

    let mut old_to_new_label: HashMap<String, String> = HashMap::new();
    let mut labels = BTreeMap::new();

    for offset in 0..events.len() {
        let local_name = format!("repro_{}", offset);
        labels.insert(local_name.clone(), offset);
    }

    for (label, old_idx) in &script.labels {
        if *old_idx >= start_idx && *old_idx < end_idx {
            old_to_new_label.insert(label.clone(), format!("repro_{}", old_idx - start_idx));
        }
    }

    labels.insert("start".to_string(), failure_idx - start_idx);

    for event in &mut events {
        rewrite_event_targets(event, &old_to_new_label);
    }

    ScriptRaw::new(events, labels)
}

fn rewrite_event_targets(event: &mut EventRaw, old_to_new_label: &HashMap<String, String>) {
    let fallback = "start".to_string();
    match event {
        EventRaw::Jump { target } => {
            *target = old_to_new_label
                .get(target)
                .cloned()
                .unwrap_or_else(|| fallback.clone());
        }
        EventRaw::JumpIf { target, .. } => {
            *target = old_to_new_label
                .get(target)
                .cloned()
                .unwrap_or_else(|| fallback.clone());
        }
        EventRaw::Choice(choice) => {
            for option in &mut choice.options {
                option.target = old_to_new_label
                    .get(&option.target)
                    .cloned()
                    .unwrap_or_else(|| fallback.clone());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::node_graph::NodeGraph;
    use crate::editor::node_types::StoryNode;
    use eframe::egui;

    fn p(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(x, y)
    }

    fn build_linear_graph() -> NodeGraph {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
        let dialogue = graph.add_node(
            StoryNode::Dialogue {
                speaker: "Ava".to_string(),
                text: "Hola".to_string(),
            },
            p(0.0, 100.0),
        );
        let end = graph.add_node(StoryNode::End, p(0.0, 200.0));
        graph.connect(start, dialogue);
        graph.connect(dialogue, end);
        graph
    }

    fn build_branching_graph() -> NodeGraph {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
        let intro = graph.add_node(
            StoryNode::Dialogue {
                speaker: "Narrador".to_string(),
                text: "Inicio".to_string(),
            },
            p(0.0, 100.0),
        );
        let choice = graph.add_node(
            StoryNode::Choice {
                prompt: "Ruta".to_string(),
                options: vec!["A".to_string(), "B".to_string()],
            },
            p(0.0, 200.0),
        );
        let branch_a = graph.add_node(
            StoryNode::Dialogue {
                speaker: "A".to_string(),
                text: "Ruta A".to_string(),
            },
            p(-120.0, 300.0),
        );
        let branch_b = graph.add_node(
            StoryNode::Dialogue {
                speaker: "B".to_string(),
                text: "Ruta B".to_string(),
            },
            p(120.0, 300.0),
        );
        let end = graph.add_node(StoryNode::End, p(0.0, 400.0));

        graph.connect(start, intro);
        graph.connect(intro, choice);
        graph.connect_port(choice, 0, branch_a);
        graph.connect_port(choice, 1, branch_b);
        graph.connect(branch_a, end);
        graph.connect(branch_b, end);

        graph
    }

    #[test]
    fn compile_project_emits_expected_phase_trace_order() {
        let graph = build_linear_graph();
        let result = compile_project(&graph);

        let phases: Vec<CompilationPhase> = result.phase_trace.iter().map(|p| p.phase).collect();
        assert_eq!(
            phases,
            vec![
                CompilationPhase::GraphSync,
                CompilationPhase::GraphValidation,
                CompilationPhase::ScriptCompile,
                CompilationPhase::RuntimeInit,
                CompilationPhase::DryRun,
            ]
        );
    }

    #[test]
    fn compile_project_reports_dry_run_completion() {
        let graph = build_linear_graph();
        let result = compile_project(&graph);

        assert!(result.engine_result.is_ok());
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == LintCode::DryRunFinished));
    }

    #[test]
    fn preview_runtime_sequence_matches_raw_sequence_for_default_route() {
        let graph = build_branching_graph();
        let result = compile_project(&graph);
        let report = result.dry_run_report.expect("dry run report");
        let runtime_seq: Vec<String> = report
            .steps
            .iter()
            .map(|step| step.event_kind.clone())
            .collect();
        let raw_seq: Vec<String> = collect_raw_sequence(&result.script, 32)
            .into_iter()
            .map(|(_, kind)| kind)
            .collect();
        assert_eq!(runtime_seq, raw_seq);
        assert!(!result
            .issues
            .iter()
            .any(|issue| issue.code == LintCode::DryRunParityMismatch));
    }

    #[test]
    fn dry_run_report_contains_step_snapshots() {
        let graph = build_linear_graph();
        let result = compile_project(&graph);
        let report = result.dry_run_report.expect("dry run report");

        assert!(!report.steps.is_empty());
        assert!(report
            .steps
            .iter()
            .enumerate()
            .all(|(idx, trace)| trace.step == idx));
    }

    #[test]
    fn minimal_repro_script_is_compileable() {
        let graph = build_branching_graph();
        let result = compile_project(&graph);
        let repro = result.minimal_repro_script().expect("repro script");
        assert!(repro.compile().is_ok());
    }

    #[test]
    fn dry_run_runtime_error_includes_event_ip() {
        let mut graph = NodeGraph::new();
        let start = graph.add_node(StoryNode::Start, p(0.0, 0.0));
        let choice = graph.add_node(
            StoryNode::Choice {
                prompt: "No options".to_string(),
                options: Vec::new(),
            },
            p(0.0, 100.0),
        );
        let end = graph.add_node(StoryNode::End, p(0.0, 200.0));
        graph.connect(start, choice);
        graph.connect(choice, end);

        let result = compile_project(&graph);
        let dry_error = result
            .issues
            .iter()
            .find(|issue| issue.code == LintCode::DryRunRuntimeError);
        assert!(dry_error.is_some());
        assert!(dry_error.and_then(|issue| issue.event_ip).is_some());
    }
}
