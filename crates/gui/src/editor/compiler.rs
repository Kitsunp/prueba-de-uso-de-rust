use crate::editor::{
    node_graph::NodeGraph,
    script_sync,
    validator::{self, LintCode, LintIssue, LintSeverity, ValidationPhase},
};
use visual_novel_engine::{Engine, EventCompiled, ScriptRaw, StoryGraph};

const DRY_RUN_MAX_STEPS: usize = 2048;

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

pub struct CompilationResult {
    pub script: ScriptRaw,
    pub engine_result: Result<Engine, String>,
    pub issues: Vec<LintIssue>,
    pub phase_trace: Vec<PhaseTrace>,
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

                    issues.extend(run_dry_run(engine.clone()));
                    let dry_run_errors = issues
                        .iter()
                        .filter(|i| {
                            i.phase == ValidationPhase::DryRun && i.severity == LintSeverity::Error
                        })
                        .count();
                    phase_trace.push(PhaseTrace {
                        phase: CompilationPhase::DryRun,
                        ok: dry_run_errors == 0,
                        detail: format!(
                            "Dry run complete ({} dry-run error(s))",
                            dry_run_errors
                        ),
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
    }
}

fn run_dry_run(mut engine: Engine) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let mut steps = 0usize;

    loop {
        if steps >= DRY_RUN_MAX_STEPS {
            issues.push(LintIssue::warning(
                Some(engine.state().position),
                ValidationPhase::DryRun,
                LintCode::DryRunStepLimit,
                format!(
                    "Dry Run reached {} steps; possible loop or blocking flow",
                    DRY_RUN_MAX_STEPS
                ),
            ));
            break;
        }

        let ip = engine.state().position;
        let event = match engine.current_event() {
            Ok(event) => event,
            Err(_) => {
                issues.push(LintIssue::info(
                    None,
                    ValidationPhase::DryRun,
                    LintCode::DryRunFinished,
                    format!("Dry Run finished in {} step(s)", steps),
                ));
                break;
            }
        };

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
            issues.push(LintIssue::error(
                Some(ip),
                ValidationPhase::DryRun,
                LintCode::DryRunRuntimeError,
                format!("Dry Run runtime error at ip {}: {}", ip, err),
            ));
            break;
        }

        steps += 1;
    }

    issues
}
