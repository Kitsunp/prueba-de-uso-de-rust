use std::collections::BTreeMap;

use super::*;
use visual_novel_engine::{DialogueRaw, EventRaw, ResourceLimiter, ScriptRaw, SecurityPolicy};

fn one_dialogue_engine() -> Engine {
    let script = ScriptRaw::new(
        vec![EventRaw::Dialogue(DialogueRaw {
            speaker: "Narrator".to_string(),
            text: "Hola mundo".to_string(),
        })],
        BTreeMap::from([("start".to_string(), 0usize)]),
    );
    Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .expect("engine must build")
}

#[test]
fn text_reveal_is_utf8_safe() {
    let mut state = PlayerSessionState {
        text_chars_per_second: 1.0,
        ..PlayerSessionState::default()
    };
    state.on_position_changed(0, 0.0);
    let line = "Hola \u{3053}\u{3093}\u{306B}\u{3061}\u{306F}";
    let first = state.visible_text(line, 1.0);
    let second = state.visible_text(line, 5.0);

    assert!(line.starts_with(first));
    assert!(line.starts_with(second));
}

#[test]
fn skip_read_only_only_skips_seen_dialogue() {
    let state = PlayerSessionState {
        skip_mode: SkipMode::ReadOnly,
        ..PlayerSessionState::default()
    };
    let mut engine = one_dialogue_engine();
    let event = engine.current_event().expect("event at start");

    assert!(!state.should_skip_current(&event, &engine));

    let _ = engine.step().expect("step dialogue");
    engine.jump_to_label("start").expect("restart to start");
    let event = engine.current_event().expect("event at start again");

    assert!(state.should_skip_current(&event, &engine));
}

#[test]
fn autoplay_delay_is_respected() {
    let mut state = PlayerSessionState {
        autoplay_enabled: true,
        autoplay_delay_ms: 1000,
        ..PlayerSessionState::default()
    };

    assert!(state.autoplay_ready(0.0));
    state.mark_auto_step(0.2);
    assert!(!state.autoplay_ready(0.9));
    assert!(state.autoplay_ready(1.3));
}
