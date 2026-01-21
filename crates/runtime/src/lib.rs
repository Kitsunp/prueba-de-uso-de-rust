//! Runtime layer for driving the engine with a winit + pixels loop.

mod loader;
pub mod render;

pub use loader::{AsyncLoader, LoadRequest, LoadResult};

use std::collections::HashMap;

// use pixels::{Pixels, SurfaceTexture}; // Removed unused imports
// Logic moved to software.rs
use visual_novel_engine::{
    AudioCommand, Engine, EventCompiled, RenderOutput, TextRenderer, UiState, UiView, VisualState,
};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use self::render::{BuiltinSoftwareDrawer, RenderBackend, SoftwareBackend, WgpuBackend};

/// Input trait that maps window events into engine actions.
pub trait Input {
    fn handle_window_event(&mut self, event: &WindowEvent) -> InputAction;
}

/// Audio trait stub for runtime audio playback.
pub trait Audio {
    fn play_music(&mut self, id: &str);
    fn stop_music(&mut self);
    fn play_sfx(&mut self, id: &str);
}

/// Asset store trait for runtime resource loading.
pub trait AssetStore {
    fn load_bytes(&self, id: &str) -> Result<Vec<u8>, String>;
}

/// Input actions produced by the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction {
    None,
    Advance,
    Choose(usize),
    Quit,
}

// UiState and UiView are re-exported from the core crate for shared use.

/// Simple input mapper that uses keyboard shortcuts.
#[derive(Default)]
pub struct BasicInput;

impl Input for BasicInput {
    fn handle_window_event(&mut self, event: &WindowEvent) -> InputAction {
        let WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
            ..
        } = event
        else {
            return InputAction::None;
        };

        match key {
            VirtualKeyCode::Escape => InputAction::Quit,
            VirtualKeyCode::Space | VirtualKeyCode::Return => InputAction::Advance,
            VirtualKeyCode::Key1 => InputAction::Choose(0),
            VirtualKeyCode::Key2 => InputAction::Choose(1),
            VirtualKeyCode::Key3 => InputAction::Choose(2),
            VirtualKeyCode::Key4 => InputAction::Choose(3),
            VirtualKeyCode::Key5 => InputAction::Choose(4),
            VirtualKeyCode::Key6 => InputAction::Choose(5),
            VirtualKeyCode::Key7 => InputAction::Choose(6),
            VirtualKeyCode::Key8 => InputAction::Choose(7),
            VirtualKeyCode::Key9 => InputAction::Choose(8),
            _ => InputAction::None,
        }
    }
}

/// Minimal audio backend placeholder.
#[derive(Default)]
pub struct SilentAudio;

impl Audio for SilentAudio {
    fn play_music(&mut self, _id: &str) {}

    fn stop_music(&mut self) {}

    fn play_sfx(&mut self, _id: &str) {}
}

/// In-memory asset store placeholder.
#[derive(Default)]
pub struct MemoryAssetStore {
    assets: HashMap<String, Vec<u8>>,
}

impl MemoryAssetStore {
    pub fn insert(&mut self, id: impl Into<String>, data: Vec<u8>) {
        self.assets.insert(id.into(), data);
    }
}

impl AssetStore for MemoryAssetStore {
    fn load_bytes(&self, id: &str) -> Result<Vec<u8>, String> {
        self.assets
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Asset not found: {}", id))
    }
}

/// Runtime application wrapper. Logic controller.
pub struct RuntimeApp<I, A, S> {
    engine: Engine,
    visual: VisualState,
    input: I,
    audio: A,
    assets: S,
    ui: UiState,
}

impl<I, A, S> RuntimeApp<I, A, S>
where
    I: Input,
    A: Audio,
    S: AssetStore,
{
    pub fn new(
        engine: Engine,
        input: I,
        audio: A,
        assets: S,
    ) -> visual_novel_engine::VnResult<Self> {
        let event = engine.current_event()?;
        let visual = Self::derive_visual(&engine, &event);
        let ui = UiState::from_event(&event, &visual);
        let mut app = Self {
            engine,
            visual,
            input,
            audio,
            assets,
            ui,
        };
        let audio_commands = app.engine.take_audio_commands();
        app.apply_audio_commands(&audio_commands);
        Ok(app)
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn ui(&self) -> &UiState {
        &self.ui
    }

    pub fn handle_action(&mut self, action: InputAction) -> visual_novel_engine::VnResult<bool> {
        match action {
            InputAction::None => {}
            InputAction::Quit => return Ok(false),
            InputAction::Advance => {
                let (audio_commands, _) = self.engine.step()?;
                self.refresh_state()?;
                self.apply_audio_commands(&audio_commands);
            }
            InputAction::Choose(index) => {
                let _ = self.engine.choose(index)?;
                self.refresh_state()?;
                // After jumping, check if target is a Scene and apply its audio
                self.apply_audio_for_current_scene();
            }
        }
        Ok(true)
    }

    fn refresh_state(&mut self) -> visual_novel_engine::VnResult<()> {
        let event = self.engine.current_event()?;
        self.visual = Self::derive_visual(&self.engine, &event);
        self.ui = UiState::from_event(&event, &self.visual);
        Ok(())
    }

    /// Applies audio when the current event is a Scene (used after jump without step)
    fn apply_audio_for_current_scene(&mut self) {
        if let Ok(EventCompiled::Scene(scene)) = self.engine.current_event() {
            if let Some(music) = &scene.music {
                self.audio.play_music(music);
            }
        }
    }

    fn derive_visual(engine: &Engine, event: &EventCompiled) -> VisualState {
        let mut visual = engine.visual_state().clone();
        if let EventCompiled::Scene(scene) = event {
            visual.apply_scene(scene);
        }
        visual
    }

    fn apply_audio_commands(&mut self, commands: &[AudioCommand]) {
        for command in commands {
            match command {
                AudioCommand::PlayBgm { path, .. } => {
                    // Use path directly from the command (no workaround needed)
                    self.audio.play_music(path.as_ref());
                }
                AudioCommand::StopBgm { .. } => {
                    self.audio.stop_music();
                }
                AudioCommand::PlaySfx { path, .. } => {
                    self.audio.play_sfx(path.as_ref());
                }
            }
        }
    }

    pub fn render_text(&self) -> visual_novel_engine::VnResult<RenderOutput> {
        let renderer = TextRenderer;
        self.engine.render_current(&renderer)
    }

    pub fn assets(&self) -> &S {
        &self.assets
    }
}

/// Run the runtime loop using winit and a rendering backend (hybrid: wgpu or software).
pub fn run_winit<I, A, S>(mut app: RuntimeApp<I, A, S>) -> !
where
    I: Input + 'static,
    A: Audio + 'static,
    S: AssetStore + 'static,
{
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("VN Runtime")
        .with_inner_size(LogicalSize::new(960.0, 540.0))
        .with_min_inner_size(LogicalSize::new(640.0, 360.0))
        .build(&event_loop)
        .expect("failed to build runtime window");

    let size = window.inner_size();

    // Initialize Backend with Fallback
    let mut backend: Box<dyn RenderBackend> =
        match WgpuBackend::new(&window, size.width, size.height) {
            Ok(backend) => {
                eprintln!("Using WGPU Hardware Backend");
                Box::new(backend)
            }
            Err(err) => {
                eprintln!(
                    "WGPU Backend initialization failed: {}. Falling back to Software Backend.",
                    err
                );
                Box::new(SoftwareBackend::new(
                    &window,
                    size.width,
                    size.height,
                    Box::new(BuiltinSoftwareDrawer),
                ))
            }
        };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    backend.resize(size.width, size.height);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    backend.resize(new_inner_size.width, new_inner_size.height);
                }
                _ => {
                    let action = app.input.handle_window_event(&event);
                    match app.handle_action(action) {
                        Ok(true) => {
                            window.request_redraw();
                        }
                        Ok(false) => {
                            *control_flow = ControlFlow::Exit;
                        }
                        Err(_) => {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
            },
            Event::RedrawRequested(_) => {
                if let Err(e) = backend.render(app.ui()) {
                    eprintln!("Render error: {}", e);
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                // window.request_redraw();
            }
            _ => {}
        }
    });
}
