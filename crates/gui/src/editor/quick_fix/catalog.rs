use eframe::egui;

use crate::editor::{LintCode, LintIssue, NodeGraph, StoryNode};

use super::{QuickFixCandidate, QuickFixRisk};

fn fix_choice_add_default_option() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "choice_add_default_option",
        title_es: "Agregar opcion por defecto",
        title_en: "Add default option",
        preconditions_es: "Nodo Choice sin opciones.",
        preconditions_en: "Choice node has no options.",
        postconditions_es: "Choice queda con al menos una opcion.",
        postconditions_en: "Choice has at least one option.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_choice_link_unlinked_to_end() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "choice_link_unlinked_to_end",
        title_es: "Conectar opciones sueltas a End",
        title_en: "Connect dangling options to End",
        preconditions_es: "Choice con opciones sin conexion saliente.",
        preconditions_en: "Choice has options without outgoing links.",
        postconditions_es: "Cada opcion sin destino queda conectada a End.",
        postconditions_en: "Each unlinked option is connected to End.",
        risk: QuickFixRisk::Review,
        structural: true,
    }
}

fn fix_choice_expand_options_to_ports() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "choice_expand_options_to_ports",
        title_es: "Sincronizar opciones con puertos",
        title_en: "Sync options with connected ports",
        preconditions_es: "Hay conexiones de puertos fuera del rango de opciones.",
        preconditions_en: "There are connected ports beyond current option count.",
        postconditions_es: "Cantidad de opciones cubre todos los puertos conectados.",
        postconditions_en: "Option count covers all connected ports.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_add_missing_start() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "graph_add_start",
        title_es: "Agregar nodo Start",
        title_en: "Add Start node",
        preconditions_es: "No existe Start en el grafo.",
        preconditions_en: "No Start node exists in graph.",
        postconditions_es: "Grafo contiene Start y punto de entrada.",
        postconditions_en: "Graph contains Start entry point.",
        risk: QuickFixRisk::Review,
        structural: true,
    }
}

fn fix_dead_end_to_end() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "node_connect_dead_end_to_end",
        title_es: "Conectar nodo sin salida a End",
        title_en: "Connect dead-end node to End",
        preconditions_es: "Nodo con dead-end y sin salida.",
        preconditions_en: "Node has dead-end and no outgoing edge.",
        postconditions_es: "Nodo queda conectado a End.",
        postconditions_en: "Node gets an outgoing edge to End.",
        risk: QuickFixRisk::Review,
        structural: true,
    }
}

fn fix_fill_speaker() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "dialogue_fill_speaker",
        title_es: "Asignar speaker Narrator",
        title_en: "Set speaker to Narrator",
        preconditions_es: "Dialogue con speaker vacio.",
        preconditions_en: "Dialogue has empty speaker.",
        postconditions_es: "Speaker no vacio.",
        postconditions_en: "Speaker is non-empty.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_fill_jump_target() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "jump_set_start_target",
        title_es: "Asignar target start",
        title_en: "Set target to start",
        preconditions_es: "Jump/JumpIf con target vacio.",
        preconditions_en: "Jump/JumpIf has empty target.",
        postconditions_es: "Target apuntando a start.",
        postconditions_en: "Target points to start.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_transition_kind() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "transition_set_fade",
        title_es: "Normalizar tipo de transicion a fade",
        title_en: "Normalize transition kind to fade",
        preconditions_es: "Tipo de transicion fuera de contrato.",
        preconditions_en: "Transition kind outside contract.",
        postconditions_es: "Tipo valido (fade).",
        postconditions_en: "Valid kind (fade).",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_transition_duration() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "transition_set_default_duration",
        title_es: "Asignar duracion por defecto (300ms)",
        title_en: "Set default duration (300ms)",
        preconditions_es: "Duracion <= 0.",
        preconditions_en: "Duration <= 0.",
        postconditions_es: "Duracion valida > 0.",
        postconditions_en: "Valid duration > 0.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_channel() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_normalize_channel",
        title_es: "Normalizar canal de audio",
        title_en: "Normalize audio channel",
        preconditions_es: "Canal fuera de contrato.",
        preconditions_en: "Channel outside contract.",
        postconditions_es: "Canal valido (bgm/sfx/voice).",
        postconditions_en: "Valid channel (bgm/sfx/voice).",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_action() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_normalize_action",
        title_es: "Normalizar accion de audio",
        title_en: "Normalize audio action",
        preconditions_es: "Accion fuera de contrato.",
        preconditions_en: "Action outside contract.",
        postconditions_es: "Accion valida (play/stop/fade_out).",
        postconditions_en: "Valid action (play/stop/fade_out).",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_volume() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_clamp_volume",
        title_es: "Ajustar volumen al rango [0,1]",
        title_en: "Clamp volume to [0,1]",
        preconditions_es: "Volumen invalido o no finito.",
        preconditions_en: "Volume invalid or non-finite.",
        postconditions_es: "Volumen valido en [0,1].",
        postconditions_en: "Volume valid in [0,1].",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_fade() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_set_default_fade",
        title_es: "Asignar fade por defecto (250ms)",
        title_en: "Set default fade (250ms)",
        preconditions_es: "Accion stop/fade_out con fade invalido.",
        preconditions_en: "Stop/fade_out action has invalid fade.",
        postconditions_es: "Fade valido para stop/fade_out.",
        postconditions_en: "Valid fade for stop/fade_out.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_scene_bg_empty() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "scene_clear_empty_background",
        title_es: "Limpiar background vacio",
        title_en: "Clear empty background",
        preconditions_es: "Background declarado pero vacio.",
        preconditions_en: "Background declared but empty.",
        postconditions_es: "Background en None o valor valido.",
        postconditions_en: "Background is None or valid.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_asset_empty() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_clear_empty_asset",
        title_es: "Limpiar asset de audio vacio",
        title_en: "Clear empty audio asset",
        preconditions_es: "Asset de audio es cadena vacia.",
        preconditions_en: "Audio asset is an empty string.",
        postconditions_es: "Asset queda None para evitar ruta invalida.",
        postconditions_en: "Asset becomes None to avoid invalid path.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_scene_music_empty() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "scene_clear_empty_music",
        title_es: "Limpiar musica vacia en Scene",
        title_en: "Clear empty music in Scene",
        preconditions_es: "Scene con musica declarada pero vacia.",
        preconditions_en: "Scene has declared music path but it is empty.",
        postconditions_es: "Scene.music queda en None.",
        postconditions_en: "Scene.music becomes None.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

fn fix_audio_missing_asset() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "audio_missing_asset_to_stop",
        title_es: "Normalizar play sin asset a stop",
        title_en: "Normalize play without asset to stop",
        preconditions_es: "AudioAction en play sin asset valido.",
        preconditions_en: "AudioAction is play without a valid asset.",
        postconditions_es: "Accion queda en stop con asset None.",
        postconditions_en: "Action is set to stop with asset None.",
        risk: QuickFixRisk::Review,
        structural: false,
    }
}

fn fix_clear_missing_asset_reference() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "clear_missing_asset_reference",
        title_es: "Limpiar referencia de asset inexistente",
        title_en: "Clear missing asset reference",
        preconditions_es: "Referencia de asset no existe en disco y campo es opcional.",
        preconditions_en: "Asset reference is missing on disk and field is optional.",
        postconditions_es: "Referencia se limpia para evitar fallo de carga.",
        postconditions_en: "Reference is cleared to avoid loading failure.",
        risk: QuickFixRisk::Review,
        structural: false,
    }
}

fn fix_clear_unsafe_asset_reference() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "clear_unsafe_asset_reference",
        title_es: "Limpiar referencia de asset insegura",
        title_en: "Clear unsafe asset reference",
        preconditions_es: "Referencia de asset viola politicas de ruta segura.",
        preconditions_en: "Asset reference violates safe-path policy.",
        postconditions_es: "Referencia insegura eliminada del nodo.",
        postconditions_en: "Unsafe reference is removed from node.",
        risk: QuickFixRisk::Review,
        structural: false,
    }
}

fn fix_character_entries() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "character_prune_or_fill_invalid_names",
        title_es: "Corregir nombres de personajes invalidos",
        title_en: "Fix invalid character names",
        preconditions_es: "Hay entradas de personaje con nombre vacio.",
        preconditions_en: "Character entries with empty names exist.",
        postconditions_es: "Entradas invalidas eliminadas o nombre por defecto aplicado.",
        postconditions_en: "Invalid entries pruned or default name applied.",
        risk: QuickFixRisk::Review,
        structural: false,
    }
}

fn fix_character_scale() -> QuickFixCandidate {
    QuickFixCandidate {
        fix_id: "character_set_default_scale",
        title_es: "Asignar escala por defecto (1.0)",
        title_en: "Set default scale (1.0)",
        preconditions_es: "Escala invalida o no finita.",
        preconditions_en: "Scale invalid or non-finite.",
        postconditions_es: "Escala valida > 0.",
        postconditions_en: "Scale valid > 0.",
        risk: QuickFixRisk::Safe,
        structural: false,
    }
}

struct QuickFixRule {
    build: fn() -> QuickFixCandidate,
    matches: fn(&LintIssue, &NodeGraph) -> bool,
    apply: fn(&mut NodeGraph, &LintIssue) -> Result<bool, String>,
}

pub(super) fn suggest_fixes(issue: &LintIssue, graph: &NodeGraph) -> Vec<QuickFixCandidate> {
    quick_fix_rules()
        .iter()
        .filter(|rule| (rule.matches)(issue, graph))
        .map(|rule| (rule.build)())
        .collect()
}

pub(super) fn apply_fix(
    graph: &mut NodeGraph,
    issue: &LintIssue,
    fix_id: &str,
) -> Result<bool, String> {
    let rule = quick_fix_rules()
        .iter()
        .find(|rule| (rule.build)().fix_id == fix_id)
        .ok_or_else(|| format!("unsupported fix_id '{fix_id}'"))?;
    if !(rule.matches)(issue, graph) {
        return Err(format!(
            "fix '{fix_id}' preconditions failed for issue {}",
            issue.diagnostic_id()
        ));
    }
    (rule.apply)(graph, issue)
}

fn quick_fix_rules() -> &'static [QuickFixRule] {
    const RULES: &[QuickFixRule] = &[
        QuickFixRule {
            build: fix_add_missing_start,
            matches: matches_missing_start,
            apply: apply_missing_start,
        },
        QuickFixRule {
            build: fix_dead_end_to_end,
            matches: matches_dead_end,
            apply: apply_dead_end,
        },
        QuickFixRule {
            build: fix_choice_add_default_option,
            matches: matches_choice_no_options,
            apply: apply_choice_no_options,
        },
        QuickFixRule {
            build: fix_choice_link_unlinked_to_end,
            matches: matches_choice_option_unlinked,
            apply: apply_choice_option_unlinked,
        },
        QuickFixRule {
            build: fix_choice_expand_options_to_ports,
            matches: matches_choice_port_out_of_range,
            apply: apply_choice_port_out_of_range,
        },
        QuickFixRule {
            build: fix_fill_speaker,
            matches: matches_empty_speaker,
            apply: apply_empty_speaker,
        },
        QuickFixRule {
            build: fix_fill_jump_target,
            matches: matches_empty_jump_target,
            apply: apply_empty_jump_target,
        },
        QuickFixRule {
            build: fix_transition_kind,
            matches: matches_invalid_transition_kind,
            apply: apply_invalid_transition_kind,
        },
        QuickFixRule {
            build: fix_transition_duration,
            matches: matches_invalid_transition_duration,
            apply: apply_invalid_transition_duration,
        },
        QuickFixRule {
            build: fix_audio_channel,
            matches: matches_invalid_audio_channel,
            apply: apply_invalid_audio_channel,
        },
        QuickFixRule {
            build: fix_audio_action,
            matches: matches_invalid_audio_action,
            apply: apply_invalid_audio_action,
        },
        QuickFixRule {
            build: fix_audio_volume,
            matches: matches_invalid_audio_volume,
            apply: apply_invalid_audio_volume,
        },
        QuickFixRule {
            build: fix_audio_fade,
            matches: matches_invalid_audio_fade,
            apply: apply_invalid_audio_fade,
        },
        QuickFixRule {
            build: fix_scene_bg_empty,
            matches: matches_empty_scene_background,
            apply: apply_empty_scene_background,
        },
        QuickFixRule {
            build: fix_scene_music_empty,
            matches: matches_empty_scene_music,
            apply: apply_empty_scene_music,
        },
        QuickFixRule {
            build: fix_audio_asset_empty,
            matches: matches_empty_audio_asset,
            apply: apply_empty_audio_asset,
        },
        QuickFixRule {
            build: fix_audio_missing_asset,
            matches: matches_audio_missing_asset,
            apply: apply_audio_missing_asset,
        },
        QuickFixRule {
            build: fix_clear_missing_asset_reference,
            matches: matches_missing_asset_reference,
            apply: apply_clear_asset_reference,
        },
        QuickFixRule {
            build: fix_clear_unsafe_asset_reference,
            matches: matches_unsafe_asset_reference,
            apply: apply_clear_asset_reference,
        },
        QuickFixRule {
            build: fix_character_entries,
            matches: matches_empty_character_name,
            apply: apply_empty_character_name,
        },
        QuickFixRule {
            build: fix_character_scale,
            matches: matches_invalid_character_scale,
            apply: apply_invalid_character_scale,
        },
    ];
    RULES
}

fn require_node_id(issue: &LintIssue, fix_id: &str) -> Result<u32, String> {
    issue
        .node_id
        .ok_or_else(|| format!("fix '{fix_id}' requires node_id"))
}

fn node_is_choice(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(graph.get_node(node_id), Some(StoryNode::Choice { .. }))
}

fn node_is_dialogue(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(graph.get_node(node_id), Some(StoryNode::Dialogue { .. }))
}

fn node_is_jump_like(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(
        graph.get_node(node_id),
        Some(StoryNode::Jump { .. } | StoryNode::JumpIf { .. })
    )
}

fn node_is_transition(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(graph.get_node(node_id), Some(StoryNode::Transition { .. }))
}

fn node_is_audio_action(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(graph.get_node(node_id), Some(StoryNode::AudioAction { .. }))
}

fn node_is_scene(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(graph.get_node(node_id), Some(StoryNode::Scene { .. }))
}

fn node_is_character_container(graph: &NodeGraph, node_id: u32) -> bool {
    matches!(
        graph.get_node(node_id),
        Some(
            StoryNode::Scene { .. }
                | StoryNode::ScenePatch(_)
                | StoryNode::CharacterPlacement { .. }
        )
    )
}

fn matches_missing_start(issue: &LintIssue, _graph: &NodeGraph) -> bool {
    issue.code == LintCode::MissingStart
}

fn matches_dead_end(issue: &LintIssue, graph: &NodeGraph) -> bool {
    if issue.code != LintCode::DeadEnd {
        return false;
    }
    let Some(node_id) = issue.node_id else {
        return false;
    };
    !matches!(graph.get_node(node_id), Some(StoryNode::End))
}

fn matches_choice_no_options(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::ChoiceNoOptions
        && issue
            .node_id
            .is_some_and(|node_id| node_is_choice(graph, node_id))
}

fn matches_choice_option_unlinked(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::ChoiceOptionUnlinked
        && issue
            .node_id
            .is_some_and(|node_id| node_is_choice(graph, node_id))
}

fn matches_choice_port_out_of_range(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::ChoicePortOutOfRange
        && issue
            .node_id
            .is_some_and(|node_id| node_is_choice(graph, node_id))
}

fn matches_empty_speaker(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::EmptySpeakerName
        && issue
            .node_id
            .is_some_and(|node_id| node_is_dialogue(graph, node_id))
}

fn matches_empty_jump_target(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::EmptyJumpTarget
        && issue
            .node_id
            .is_some_and(|node_id| node_is_jump_like(graph, node_id))
}

fn matches_invalid_transition_kind(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidTransitionKind
        && issue
            .node_id
            .is_some_and(|node_id| node_is_transition(graph, node_id))
}

fn matches_invalid_transition_duration(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidTransitionDuration
        && issue
            .node_id
            .is_some_and(|node_id| node_is_transition(graph, node_id))
}

fn matches_invalid_audio_channel(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidAudioChannel
        && issue
            .node_id
            .is_some_and(|node_id| node_is_audio_action(graph, node_id))
}

fn matches_invalid_audio_action(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidAudioAction
        && issue
            .node_id
            .is_some_and(|node_id| node_is_audio_action(graph, node_id))
}

fn matches_invalid_audio_volume(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidAudioVolume
        && issue
            .node_id
            .is_some_and(|node_id| node_is_audio_action(graph, node_id))
}

fn matches_invalid_audio_fade(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidAudioFade
        && issue
            .node_id
            .is_some_and(|node_id| node_is_audio_action(graph, node_id))
}

fn matches_empty_scene_background(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::SceneBackgroundEmpty
        && issue
            .node_id
            .is_some_and(|node_id| node_is_scene(graph, node_id))
}

fn matches_empty_scene_music(issue: &LintIssue, graph: &NodeGraph) -> bool {
    if issue.code != LintCode::AudioAssetEmpty {
        return false;
    }
    let Some(node_id) = issue.node_id else {
        return false;
    };
    let Some(StoryNode::Scene { music, .. }) = graph.get_node(node_id) else {
        return false;
    };
    music
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
}

fn matches_empty_audio_asset(issue: &LintIssue, graph: &NodeGraph) -> bool {
    if issue.code != LintCode::AudioAssetEmpty {
        return false;
    }
    let Some(node_id) = issue.node_id else {
        return false;
    };
    let Some(StoryNode::AudioAction { asset, .. }) = graph.get_node(node_id) else {
        return false;
    };
    asset
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
}

fn matches_audio_missing_asset(issue: &LintIssue, graph: &NodeGraph) -> bool {
    if issue.code != LintCode::AudioAssetMissing {
        return false;
    }
    let Some(node_id) = issue.node_id else {
        return false;
    };
    let Some(StoryNode::AudioAction { action, asset, .. }) = graph.get_node(node_id) else {
        return false;
    };
    action.trim().eq_ignore_ascii_case("play")
        && asset.as_deref().is_none_or(|value| value.trim().is_empty())
}

fn matches_missing_asset_reference(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::AssetReferenceMissing
        && clearable_asset_field(graph, issue, false).is_some()
}

fn matches_unsafe_asset_reference(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::UnsafeAssetPath && clearable_asset_field(graph, issue, true).is_some()
}

fn matches_empty_character_name(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::EmptyCharacterName
        && issue
            .node_id
            .is_some_and(|node_id| node_is_character_container(graph, node_id))
}

fn matches_invalid_character_scale(issue: &LintIssue, graph: &NodeGraph) -> bool {
    issue.code == LintCode::InvalidCharacterScale
        && issue.node_id.is_some_and(|node_id| {
            matches!(
                graph.get_node(node_id),
                Some(StoryNode::CharacterPlacement { .. })
            )
        })
}

fn apply_missing_start(graph: &mut NodeGraph, _issue: &LintIssue) -> Result<bool, String> {
    Ok(apply_add_missing_start(graph))
}

fn apply_dead_end(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_connect_dead_end_to_end(
        graph,
        require_node_id(issue, "node_connect_dead_end_to_end")?,
    )
}

fn apply_choice_no_options(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_choice_add_default_option(graph, require_node_id(issue, "choice_add_default_option")?)
}

fn apply_choice_option_unlinked(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_choice_link_unlinked_to_end(
        graph,
        require_node_id(issue, "choice_link_unlinked_to_end")?,
    )
}

fn apply_choice_port_out_of_range(
    graph: &mut NodeGraph,
    issue: &LintIssue,
) -> Result<bool, String> {
    apply_choice_expand_options_to_ports(
        graph,
        require_node_id(issue, "choice_expand_options_to_ports")?,
    )
}

fn apply_empty_speaker(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_fill_speaker(graph, require_node_id(issue, "dialogue_fill_speaker")?)
}

fn apply_empty_jump_target(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_set_jump_target_start(graph, require_node_id(issue, "jump_set_start_target")?)
}

fn apply_invalid_transition_kind(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_set_transition_kind_fade(graph, require_node_id(issue, "transition_set_fade")?)
}

fn apply_invalid_transition_duration(
    graph: &mut NodeGraph,
    issue: &LintIssue,
) -> Result<bool, String> {
    apply_set_transition_duration(
        graph,
        require_node_id(issue, "transition_set_default_duration")?,
    )
}

fn apply_invalid_audio_channel(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_channel_fix(graph, require_node_id(issue, "audio_normalize_channel")?)
}

fn apply_invalid_audio_action(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_action_fix(graph, require_node_id(issue, "audio_normalize_action")?)
}

fn apply_invalid_audio_volume(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_volume_fix(graph, require_node_id(issue, "audio_clamp_volume")?)
}

fn apply_invalid_audio_fade(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_fade_fix(graph, require_node_id(issue, "audio_set_default_fade")?)
}

fn apply_empty_scene_background(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_scene_background_clear(
        graph,
        require_node_id(issue, "scene_clear_empty_background")?,
    )
}

fn apply_empty_scene_music(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_scene_music_clear(graph, require_node_id(issue, "scene_clear_empty_music")?)
}

fn apply_empty_audio_asset(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_asset_clear(graph, require_node_id(issue, "audio_clear_empty_asset")?)
}

fn apply_audio_missing_asset(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_audio_missing_asset_fix(
        graph,
        require_node_id(issue, "audio_missing_asset_to_stop")?,
    )
}

fn apply_empty_character_name(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_character_name_fix(
        graph,
        require_node_id(issue, "character_prune_or_fill_invalid_names")?,
    )
}

fn apply_invalid_character_scale(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    apply_character_scale_fix(
        graph,
        require_node_id(issue, "character_set_default_scale")?,
    )
}

fn apply_add_missing_start(graph: &mut NodeGraph) -> bool {
    if graph
        .nodes()
        .any(|(_, node, _)| matches!(node, StoryNode::Start))
    {
        return false;
    }
    graph.add_node(StoryNode::Start, egui::pos2(50.0, 30.0));
    true
}

fn apply_connect_dead_end_to_end(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(node) = graph.get_node(node_id).cloned() else {
        return Err(format!("node_id {node_id} not found"));
    };
    if matches!(node, StoryNode::End) {
        return Ok(false);
    }
    if graph.connections().any(|c| c.from == node_id) {
        return Ok(false);
    }
    let end_id = ensure_end_node(graph, node_id)?;
    graph.connect(node_id, end_id);
    Ok(true)
}

fn apply_choice_add_default_option(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Choice { options, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Choice"));
    };
    if !options.is_empty() {
        return Ok(false);
    }
    options.push("Option 1".to_string());
    graph.mark_modified();
    Ok(true)
}

fn apply_choice_link_unlinked_to_end(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let options_len = match graph.get_node(node_id) {
        Some(StoryNode::Choice { options, .. }) => options.len(),
        _ => return Err(format!("node_id {node_id} is not Choice")),
    };
    if options_len == 0 {
        return Ok(false);
    }
    let unlinked: Vec<usize> = (0..options_len)
        .filter(|idx| {
            !graph
                .connections()
                .any(|conn| conn.from == node_id && conn.from_port == *idx)
        })
        .collect();
    if unlinked.is_empty() {
        return Ok(false);
    }
    let end_id = ensure_end_node(graph, node_id)?;
    for port in unlinked {
        graph.connect_port(node_id, port, end_id);
    }
    Ok(true)
}

fn apply_choice_expand_options_to_ports(
    graph: &mut NodeGraph,
    node_id: u32,
) -> Result<bool, String> {
    let max_port = graph
        .connections()
        .filter(|conn| conn.from == node_id)
        .map(|conn| conn.from_port)
        .max()
        .unwrap_or(0);

    let Some(StoryNode::Choice { options, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Choice"));
    };
    let before = options.len();
    while options.len() <= max_port {
        let next = options.len() + 1;
        options.push(format!("Option {next}"));
    }
    if options.len() != before {
        graph.mark_modified();
        return Ok(true);
    }
    Ok(false)
}

fn apply_fill_speaker(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Dialogue { speaker, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Dialogue"));
    };
    if !speaker.trim().is_empty() {
        return Ok(false);
    }
    *speaker = "Narrator".to_string();
    graph.mark_modified();
    Ok(true)
}

fn apply_set_jump_target_start(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    if !graph
        .nodes()
        .any(|(_, node, _)| matches!(node, StoryNode::Start))
    {
        return Err("cannot set jump target to start: no Start node exists".to_string());
    }
    let Some(node) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} not found"));
    };
    match node {
        StoryNode::Jump { target } | StoryNode::JumpIf { target, .. } => {
            if target.trim().is_empty() {
                *target = "start".to_string();
                graph.mark_modified();
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Err(format!("node_id {node_id} is not Jump/JumpIf")),
    }
}

fn apply_set_transition_kind_fade(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Transition { kind, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Transition"));
    };
    let normalized = kind.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "fade" | "fade_black" | "dissolve" | "cut"
    ) {
        return Ok(false);
    }
    *kind = "fade".to_string();
    graph.mark_modified();
    Ok(true)
}

fn apply_set_transition_duration(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Transition { duration_ms, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Transition"));
    };
    if *duration_ms > 0 {
        return Ok(false);
    }
    *duration_ms = 300;
    graph.mark_modified();
    Ok(true)
}

fn canonical_token(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .map(|char| char.to_ascii_lowercase())
        .collect()
}

fn normalize_audio_channel(value: &str) -> &'static str {
    match canonical_token(value).as_str() {
        "bgm" | "music" | "backgroundmusic" | "bgmusic" => "bgm",
        "sfx" | "fx" | "soundeffect" | "soundeffects" => "sfx",
        "voice" | "vo" | "voiceover" => "voice",
        _ => "bgm",
    }
}

fn normalize_audio_action(value: &str, has_asset: bool) -> &'static str {
    match canonical_token(value).as_str() {
        "play" | "start" | "resume" => "play",
        "stop" | "halt" => "stop",
        "fadeout" | "fade" => "fade_out",
        _ => {
            if has_asset {
                "play"
            } else {
                "stop"
            }
        }
    }
}

fn apply_audio_channel_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction { channel, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    let normalized = normalize_audio_channel(channel).to_string();
    if channel.trim().eq_ignore_ascii_case(&normalized) {
        return Ok(false);
    }
    *channel = normalized;
    graph.mark_modified();
    Ok(true)
}

fn apply_audio_action_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction { action, asset, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    let has_asset = asset
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let normalized = normalize_audio_action(action, has_asset).to_string();
    if action.trim().eq_ignore_ascii_case(&normalized) {
        return Ok(false);
    }
    *action = normalized;
    graph.mark_modified();
    Ok(true)
}

fn apply_audio_volume_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction { volume, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    let Some(current) = *volume else {
        return Ok(false);
    };
    let normalized = if current.is_finite() {
        current.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if (normalized - current).abs() < f32::EPSILON {
        return Ok(false);
    }
    *volume = Some(normalized);
    graph.mark_modified();
    Ok(true)
}

fn apply_audio_fade_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction {
        action,
        fade_duration_ms,
        ..
    }) = graph.get_node_mut(node_id)
    else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    let normalized_action = action.trim().to_ascii_lowercase();
    if !matches!(normalized_action.as_str(), "stop" | "fade_out") {
        return Ok(false);
    }
    if fade_duration_ms.unwrap_or(0) > 0 {
        return Ok(false);
    }
    *fade_duration_ms = Some(250);
    graph.mark_modified();
    Ok(true)
}

fn apply_scene_background_clear(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Scene { background, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Scene"));
    };
    if !background.as_deref().is_some_and(|v| v.trim().is_empty()) {
        return Ok(false);
    }
    *background = None;
    graph.mark_modified();
    Ok(true)
}

fn apply_scene_music_clear(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::Scene { music, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not Scene"));
    };
    if !music
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Ok(false);
    }
    *music = None;
    graph.mark_modified();
    Ok(true)
}

fn apply_audio_asset_clear(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction { asset, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    if !asset
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Ok(false);
    }
    *asset = None;
    graph.mark_modified();
    Ok(true)
}

fn apply_audio_missing_asset_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::AudioAction { action, asset, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not AudioAction"));
    };
    let is_play = action.trim().eq_ignore_ascii_case("play");
    let missing_asset = asset.as_deref().is_none_or(|value| value.trim().is_empty());
    if !is_play || !missing_asset {
        return Ok(false);
    }
    *action = "stop".to_string();
    *asset = None;
    graph.mark_modified();
    Ok(true)
}

fn apply_character_name_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(node) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} not found"));
    };
    let mut changed = false;
    match node {
        StoryNode::Scene { characters, .. } => {
            let before = characters.len();
            characters.retain(|character| !character.name.trim().is_empty());
            changed = characters.len() != before;
        }
        StoryNode::ScenePatch(patch) => {
            let before_add = patch.add.len();
            let before_upd = patch.update.len();
            let before_rem = patch.remove.len();
            patch
                .add
                .retain(|character| !character.name.trim().is_empty());
            patch
                .update
                .retain(|character| !character.name.trim().is_empty());
            patch.remove.retain(|name| !name.trim().is_empty());
            changed = before_add != patch.add.len()
                || before_upd != patch.update.len()
                || before_rem != patch.remove.len();
        }
        StoryNode::CharacterPlacement { name, .. } => {
            if name.trim().is_empty() {
                *name = "Character".to_string();
                changed = true;
            }
        }
        _ => return Err(format!("node_id {node_id} is not a character container")),
    }
    if changed {
        graph.mark_modified();
    }
    Ok(changed)
}

fn apply_character_scale_fix(graph: &mut NodeGraph, node_id: u32) -> Result<bool, String> {
    let Some(StoryNode::CharacterPlacement { scale, .. }) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} is not CharacterPlacement"));
    };
    let invalid = scale.is_some_and(|value| !value.is_finite() || value <= 0.0);
    if !invalid {
        return Ok(false);
    }
    *scale = Some(1.0);
    graph.mark_modified();
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetField {
    SceneBackground,
    SceneMusic,
    ScenePatchBackground,
    ScenePatchMusic,
    AudioAsset,
}

fn is_unsafe_asset_path(value: &str) -> bool {
    let path = value.trim();
    if path.is_empty() {
        return false;
    }
    path.contains("..")
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.starts_with("http://")
        || path.starts_with("https://")
        || path.chars().nth(1).is_some_and(|second| {
            second == ':' && path.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        })
}

fn clearable_asset_field(
    graph: &NodeGraph,
    issue: &LintIssue,
    unsafe_only: bool,
) -> Option<AssetField> {
    let node_id = issue.node_id?;
    let node = graph.get_node(node_id)?;
    let mut candidates: Vec<(AssetField, String)> = Vec::new();

    match node {
        StoryNode::Scene {
            background, music, ..
        } => {
            if let Some(path) = background.as_ref() {
                candidates.push((AssetField::SceneBackground, path.clone()));
            }
            if let Some(path) = music.as_ref() {
                candidates.push((AssetField::SceneMusic, path.clone()));
            }
        }
        StoryNode::ScenePatch(patch) => {
            if let Some(path) = patch.background.as_ref() {
                candidates.push((AssetField::ScenePatchBackground, path.clone()));
            }
            if let Some(path) = patch.music.as_ref() {
                candidates.push((AssetField::ScenePatchMusic, path.clone()));
            }
        }
        StoryNode::AudioAction { asset, .. } => {
            if let Some(path) = asset.as_ref() {
                candidates.push((AssetField::AudioAsset, path.clone()));
            }
        }
        _ => {}
    }

    if candidates.is_empty() {
        return None;
    }

    let filtered = if let Some(explicit_path) = issue.asset_path.as_ref() {
        let target = explicit_path.trim();
        candidates
            .into_iter()
            .filter(|(_, path)| path.trim() == target)
            .collect::<Vec<_>>()
    } else if unsafe_only {
        candidates
            .into_iter()
            .filter(|(_, path)| is_unsafe_asset_path(path))
            .collect::<Vec<_>>()
    } else {
        candidates
    };

    if filtered.len() == 1 {
        return Some(filtered[0].0);
    }
    None
}

fn apply_clear_asset_reference(graph: &mut NodeGraph, issue: &LintIssue) -> Result<bool, String> {
    let field = clearable_asset_field(graph, issue, issue.code == LintCode::UnsafeAssetPath)
        .ok_or_else(|| {
            format!(
                "unable to resolve a unique asset field for issue {}",
                issue.diagnostic_id()
            )
        })?;
    let node_id = require_node_id(issue, "clear_asset_reference")?;
    let Some(node) = graph.get_node_mut(node_id) else {
        return Err(format!("node_id {node_id} not found"));
    };

    let mut changed = false;
    match (field, node) {
        (AssetField::SceneBackground, StoryNode::Scene { background, .. }) => {
            if background.take().is_some() {
                changed = true;
            }
        }
        (AssetField::SceneMusic, StoryNode::Scene { music, .. }) => {
            if music.take().is_some() {
                changed = true;
            }
        }
        (AssetField::ScenePatchBackground, StoryNode::ScenePatch(patch)) => {
            if patch.background.take().is_some() {
                changed = true;
            }
        }
        (AssetField::ScenePatchMusic, StoryNode::ScenePatch(patch)) => {
            if patch.music.take().is_some() {
                changed = true;
            }
        }
        (AssetField::AudioAsset, StoryNode::AudioAction { action, asset, .. }) => {
            if asset.take().is_some() {
                changed = true;
            }
            if action.trim().eq_ignore_ascii_case("play") {
                *action = "stop".to_string();
                changed = true;
            }
        }
        _ => {
            return Err(format!(
                "asset field {:?} is incompatible with node {}",
                field, node_id
            ));
        }
    }

    if changed {
        graph.mark_modified();
    }
    Ok(changed)
}

fn ensure_end_node(graph: &mut NodeGraph, source_node_id: u32) -> Result<u32, String> {
    if let Some((id, _, _)) = graph
        .nodes()
        .find(|(_, node, _)| matches!(node, StoryNode::End))
        .cloned()
    {
        return Ok(id);
    }

    let source_pos = graph
        .nodes()
        .find(|(id, _, _)| *id == source_node_id)
        .map(|(_, _, pos)| *pos)
        .ok_or_else(|| format!("source node {source_node_id} not found"))?;
    Ok(graph.add_node(
        StoryNode::End,
        egui::pos2(source_pos.x + 140.0, source_pos.y + 120.0),
    ))
}
