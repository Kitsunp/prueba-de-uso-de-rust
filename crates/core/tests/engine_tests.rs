use std::collections::HashMap;

use visual_novel_engine::{
    CharacterPlacement, Engine, Event, RenderBackend, ResourceLimiter, SceneUpdate, Script,
    SecurityPolicy, TextRenderer,
};

fn sample_script() -> Script {
    let events = vec![
        Event::Scene(SceneUpdate {
            background: Some("bg/room.png".to_string()),
            music: Some("music/theme.ogg".to_string()),
            characters: vec![CharacterPlacement {
                name: "Ava".to_string(),
                expression: Some("smile".to_string()),
                position: Some("center".to_string()),
            }],
        }),
        Event::Dialogue(visual_novel_engine::Dialogue {
            speaker: "Ava".to_string(),
            text: "Hola".to_string(),
        }),
        Event::Choice(visual_novel_engine::Choice {
            prompt: "Ir?".to_string(),
            options: vec![
                visual_novel_engine::ChoiceOption {
                    text: "Si".to_string(),
                    target: "end".to_string(),
                },
                visual_novel_engine::ChoiceOption {
                    text: "No".to_string(),
                    target: "start".to_string(),
                },
            ],
        }),
        Event::Dialogue(visual_novel_engine::Dialogue {
            speaker: "Ava".to_string(),
            text: "Fin".to_string(),
        }),
    ];
    let mut labels = HashMap::new();
    labels.insert("start".to_string(), 0);
    labels.insert("end".to_string(), 3);
    Script::new(events, labels)
}

fn script_without_start_label() -> Script {
    let events = vec![Event::Dialogue(visual_novel_engine::Dialogue {
        speaker: "Ava".to_string(),
        text: "Hola".to_string(),
    })];
    let labels = HashMap::new();
    Script::new(events, labels)
}

fn script_with_invalid_choice_target() -> Script {
    let events = vec![Event::Choice(visual_novel_engine::Choice {
        prompt: "Ir?".to_string(),
        options: vec![visual_novel_engine::ChoiceOption {
            text: "Si".to_string(),
            target: "missing".to_string(),
        }],
    })];
    let mut labels = HashMap::new();
    labels.insert("start".to_string(), 0);
    Script::new(events, labels)
}

#[test]
fn engine_steps_through_dialogue() {
    let script = sample_script();
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap();
    let scene = engine.step().unwrap();
    assert!(matches!(scene, Event::Scene(_)));
    let dialogue = engine.step().unwrap();
    assert!(matches!(dialogue, Event::Dialogue(_)));
}

#[test]
fn engine_choice_jumps() {
    let script = sample_script();
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap();
    engine.step().unwrap();
    engine.step().unwrap();
    let choice = engine.choose(0).unwrap();
    assert!(matches!(choice, Event::Choice(_)));
    let next = engine.step().unwrap();
    if let Event::Dialogue(dialogue) = next {
        assert_eq!(dialogue.text, "Fin");
    } else {
        panic!("expected dialogue");
    }
}

#[test]
fn json_round_trip() {
    let script = sample_script();
    let serialized = serde_json::to_string(&script).unwrap();
    let parsed = Script::from_json(&serialized).unwrap();
    assert_eq!(parsed.events.len(), 4);
}

#[test]
fn engine_rejects_missing_start_label() {
    let script = script_without_start_label();
    let error = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .expect_err("should reject missing start label");
    assert!(matches!(
        error,
        visual_novel_engine::VnError::InvalidScript(_)
    ));
}

#[test]
fn engine_rejects_invalid_choice_target() {
    let script = script_with_invalid_choice_target();
    let error = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .expect_err("should reject missing choice target");
    assert!(matches!(
        error,
        visual_novel_engine::VnError::InvalidScript(_)
    ));
}

#[test]
fn engine_signals_end_of_script() {
    let script = sample_script();
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap();
    engine.step().unwrap();
    engine.step().unwrap();
    engine.choose(0).unwrap();
    engine.step().unwrap();
    let result = engine.step();
    assert!(matches!(
        result,
        Err(visual_novel_engine::VnError::EndOfScript)
    ));
}

#[test]
fn scene_updates_visual_state_and_renderer_output() {
    let script = sample_script();
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap();
    let scene = engine.step().unwrap();
    assert!(matches!(scene, Event::Scene(_)));
    let visual = engine.visual_state();
    assert_eq!(visual.background.as_deref(), Some("bg/room.png"));
    assert_eq!(visual.music.as_deref(), Some("music/theme.ogg"));
    assert_eq!(visual.characters.len(), 1);

    let renderer = TextRenderer::default();
    let output = renderer.render(&scene, visual);
    assert!(output.text.contains("Background: bg/room.png"));
    assert!(output.text.contains("Characters: Ava (smile) @ center"));
}

#[test]
fn renderer_formats_choice_and_dialogue() {
    let script = sample_script();
    let mut engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap();
    engine.step().unwrap();
    let dialogue = engine.step().unwrap();
    let renderer = TextRenderer::default();
    let output = renderer.render(&dialogue, engine.visual_state());
    assert!(output.text.contains("Ava: Hola"));

    let choice = engine.step().unwrap();
    let output = renderer.render(&choice, engine.visual_state());
    assert!(output.text.contains("1. Si"));
    assert!(output.text.contains("2. No"));
}
