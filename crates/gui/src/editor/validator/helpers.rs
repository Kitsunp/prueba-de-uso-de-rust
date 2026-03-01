use std::collections::HashSet;
use std::path::Path;

use crate::editor::NodeGraph;

pub(super) fn has_outgoing(graph: &NodeGraph, node_id: u32) -> bool {
    graph.connections.iter().any(|c| c.from == node_id)
}

pub(super) fn default_asset_exists(path: &str) -> bool {
    let candidate = Path::new(path.trim());
    if candidate.is_absolute() {
        return candidate.is_file();
    }

    match std::env::current_dir() {
        Ok(cwd) => cwd.join(candidate).is_file(),
        Err(_) => candidate.is_file(),
    }
}

pub(super) fn should_probe_asset_exists(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() {
        return false;
    }

    p.contains('/')
        || p.contains('\\')
        || Path::new(p).extension().is_some()
        || p.starts_with("assets/")
        || p.starts_with("assets\\")
}

pub(super) fn is_valid_audio_channel(channel: &str) -> bool {
    matches!(channel, "bgm" | "sfx" | "voice")
}

pub(super) fn is_valid_audio_action(action: &str) -> bool {
    matches!(action, "play" | "stop" | "fade_out")
}

pub(super) fn is_valid_transition_kind(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "fade" | "fade_black" | "dissolve" | "cut"
    )
}

pub(super) fn is_unsafe_asset_ref(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() {
        return false;
    }

    p.contains("..")
        || p.starts_with('/')
        || p.starts_with('\\')
        || p.starts_with("http://")
        || p.starts_with("https://")
        || p.chars().nth(1).is_some_and(|second| {
            second == ':' && p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        })
}

pub(super) fn visit_node(graph: &NodeGraph, node_id: u32, visited: &mut HashSet<u32>) {
    if !visited.insert(node_id) {
        return;
    }

    let outgoing: Vec<u32> = graph
        .connections
        .iter()
        .filter(|c| c.from == node_id)
        .map(|c| c.to)
        .collect();

    for target in outgoing {
        visit_node(graph, target, visited);
    }
}

pub(super) fn detect_reachable_cycle_nodes(graph: &NodeGraph, start_nodes: &[u32]) -> Vec<u32> {
    let mut visited = HashSet::new();
    let mut active = HashSet::new();
    let mut cycle_nodes = HashSet::new();

    for start in start_nodes {
        detect_cycles_from(graph, *start, &mut visited, &mut active, &mut cycle_nodes);
    }

    let mut out: Vec<u32> = cycle_nodes.into_iter().collect();
    out.sort_unstable();
    out
}

fn detect_cycles_from(
    graph: &NodeGraph,
    node_id: u32,
    visited: &mut HashSet<u32>,
    active: &mut HashSet<u32>,
    cycle_nodes: &mut HashSet<u32>,
) {
    if active.contains(&node_id) {
        cycle_nodes.insert(node_id);
        return;
    }
    if !visited.insert(node_id) {
        return;
    }

    active.insert(node_id);
    for target in graph
        .connections
        .iter()
        .filter(|connection| connection.from == node_id)
        .map(|connection| connection.to)
    {
        if active.contains(&target) {
            cycle_nodes.insert(node_id);
            cycle_nodes.insert(target);
            continue;
        }
        detect_cycles_from(graph, target, visited, active, cycle_nodes);
        if cycle_nodes.contains(&target) {
            cycle_nodes.insert(node_id);
        }
    }
    active.remove(&node_id);
}
