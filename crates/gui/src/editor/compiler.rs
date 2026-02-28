use crate::editor::{node_graph::NodeGraph, script_sync, validator, LintIssue, LintSeverity};
use visual_novel_engine::{Engine, ScriptRaw};

pub struct CompilationResult {
    pub script: ScriptRaw,
    pub engine_result: Result<Engine, String>,
    pub issues: Vec<LintIssue>,
}

pub fn compile_project(graph: &NodeGraph) -> CompilationResult {
    // 1. Convert Graph to Script (Raw)
    let script = script_sync::to_script(graph);

    // 2. Validate Graph (Lints)
    let mut issues = validator::validate(graph);

    // 3. Compile for Engine
    let engine_result = match script.compile() {
        Ok(compiled) => {
            // Create engine instance with default security and limits for editor preview
            match Engine::from_compiled(
                compiled,
                visual_novel_engine::SecurityPolicy::default(),
                visual_novel_engine::ResourceLimiter::default(),
            ) {
                Ok(engine) => Ok(engine),
                Err(e) => Err(format!("Runtime Init Error: {}", e)),
            }
        }
        Err(e) => {
            issues.push(LintIssue {
                node_id: None,
                severity: LintSeverity::Error,
                message: format!("Compilation Error: {}", e),
            });
            Err(format!("Compilation Failed: {}", e))
        }
    };

    CompilationResult {
        script,
        engine_result,
        issues,
    }
}
