use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use visual_novel_engine::{
    ChoiceOptionRaw, ChoiceRaw, Engine, EventRaw, ResourceLimiter, SceneUpdateRaw, ScriptRaw,
    SecurityPolicy,
};
use vnengine_runtime::{AssetStore, Audio, Input, InputAction, Renderer, RuntimeApp};

#[derive(Default)]
struct NullRenderer;

impl Renderer for NullRenderer {
    fn render(&mut self, _frame: &mut [u8], _size: (u32, u32), _ui: &visual_novel_engine::UiState) {
    }
}

#[derive(Default)]
struct NullInput;

impl Input for NullInput {
    fn handle_window_event(&mut self, _event: &winit::event::WindowEvent) -> InputAction {
        InputAction::None
    }
}

#[derive(Default)]
struct NullAssets;

impl AssetStore for NullAssets {
    fn load_bytes(&self, _id: &str) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Default, Debug)]
struct AudioState {
    last_music: Option<String>,
    play_calls: Vec<String>,
    stop_calls: usize,
}

#[derive(Clone, Default)]
struct SharedAudio {
    state: Rc<RefCell<AudioState>>,
}

impl Audio for SharedAudio {
    fn play_music(&mut self, id: &str) {
        let mut state = self.state.borrow_mut();
        state.last_music = Some(id.to_string());
        state.play_calls.push(id.to_string());
    }

    fn stop_music(&mut self) {
        let mut state = self.state.borrow_mut();
        state.last_music = None;
        state.stop_calls += 1;
    }
}

fn build_engine(events: Vec<EventRaw>, labels: BTreeMap<String, usize>) -> Engine {
    let script = ScriptRaw::new(events, labels);
    Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )
    .unwrap()
}

#[test]
fn audio_updates_when_choice_jumps_to_scene() {
    let events = vec![
        EventRaw::Choice(ChoiceRaw {
            prompt: "Pick".to_string(),
            options: vec![ChoiceOptionRaw {
                text: "Go".to_string(),
                target: "scene".to_string(),
            }],
        }),
        EventRaw::Scene(SceneUpdateRaw {
            background: None,
            music: Some("music/theme.ogg".to_string()),
            characters: Vec::new(),
        }),
    ];
    let labels = BTreeMap::from([("start".to_string(), 0), ("scene".to_string(), 1)]);
    let engine = build_engine(events, labels);
    let audio_state = Rc::new(RefCell::new(AudioState::default()));

    let mut app = RuntimeApp::new(
        engine,
        NullRenderer,
        NullInput,
        SharedAudio {
            state: audio_state.clone(),
        },
        NullAssets,
    )
    .unwrap();

    app.handle_action(InputAction::Choose(0)).unwrap();

    let state = audio_state.borrow();
    assert_eq!(state.last_music.as_deref(), Some("music/theme.ogg"));
    assert_eq!(state.play_calls, vec!["music/theme.ogg".to_string()]);
}

#[test]
fn audio_switches_music_for_scene_jump() {
    let events = vec![
        EventRaw::Scene(SceneUpdateRaw {
            background: None,
            music: Some("music/old.ogg".to_string()),
            characters: Vec::new(),
        }),
        EventRaw::Choice(ChoiceRaw {
            prompt: "Pick".to_string(),
            options: vec![ChoiceOptionRaw {
                text: "Go".to_string(),
                target: "next_scene".to_string(),
            }],
        }),
        EventRaw::Scene(SceneUpdateRaw {
            background: None,
            music: Some("music/new.ogg".to_string()),
            characters: Vec::new(),
        }),
    ];
    let labels = BTreeMap::from([("start".to_string(), 0), ("next_scene".to_string(), 2)]);
    let engine = build_engine(events, labels);
    let audio_state = Rc::new(RefCell::new(AudioState::default()));

    let mut app = RuntimeApp::new(
        engine,
        NullRenderer,
        NullInput,
        SharedAudio {
            state: audio_state.clone(),
        },
        NullAssets,
    )
    .unwrap();

    app.handle_action(InputAction::Advance).unwrap();
    app.handle_action(InputAction::Choose(0)).unwrap();

    let state = audio_state.borrow();
    assert_eq!(
        state.play_calls,
        vec![
            "music/old.ogg".to_string(),
            "music/old.ogg".to_string(),
            "music/new.ogg".to_string(),
        ]
    );
    assert_eq!(state.last_music.as_deref(), Some("music/new.ogg"));
}
