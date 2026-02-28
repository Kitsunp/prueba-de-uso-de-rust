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
        .map(|step| step.event_signature.clone())
        .collect();
    let first = ChoicePolicy::Strategy(ChoiceStrategy::First);
    let raw_seq: Vec<String> = simulate_raw_sequence(&result.script, 32, &first)
        .into_iter()
        .map(|step| step.event_signature)
        .collect();
    assert_eq!(runtime_seq, raw_seq);
    assert!(!result
        .issues
        .iter()
        .any(|issue| issue.code == LintCode::DryRunParityMismatch));
}

#[test]
fn raw_simulation_supports_multiple_choice_routes() {
    let graph = build_branching_graph();
    let script = crate::editor::script_sync::to_script(&graph);

    let first_policy = ChoicePolicy::Strategy(ChoiceStrategy::First);
    let last_policy = ChoicePolicy::Strategy(ChoiceStrategy::Last);
    let alternating_policy = ChoicePolicy::Strategy(ChoiceStrategy::Alternating);
    let first = simulate_raw_sequence(&script, 32, &first_policy);
    let last = simulate_raw_sequence(&script, 32, &last_policy);
    let alternating = simulate_raw_sequence(&script, 32, &alternating_policy);

    assert!(!first.is_empty());
    assert!(!last.is_empty());
    assert!(!alternating.is_empty());
    assert_ne!(
        first.iter().map(|s| &s.event_signature).collect::<Vec<_>>(),
        last.iter().map(|s| &s.event_signature).collect::<Vec<_>>()
    );
}

#[test]
fn route_enumerator_covers_choice_branches() {
    let graph = build_branching_graph();
    let script = crate::editor::script_sync::to_script(&graph);
    let routes = enumerate_choice_routes(&script, 64, 16, 8);

    assert!(routes.iter().any(|route| route.as_slice() == [0]));
    assert!(routes.iter().any(|route| route.as_slice() == [1]));
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
