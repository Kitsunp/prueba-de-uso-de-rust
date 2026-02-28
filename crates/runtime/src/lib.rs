//! Runtime layer for driving the engine with a winit + pixels loop.

pub mod assets;
pub mod audio;
pub mod input;
mod loader;
pub mod render;

pub use loader::{AsyncLoader, LoadRequest, LoadResult};

use std::sync::Arc;

// use pixels::{Pixels, SurfaceTexture}; // Removed unused imports
// Logic moved to software.rs
use visual_novel_engine::{
    AudioCommand, Engine, EventCompiled, RenderOutput, TextRenderer, UiState, VisualState,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

pub use self::assets::{AssetStore, MemoryAssetStore};
pub use self::audio::{Audio, RodioBackend, SilentAudio};
pub use self::input::{ConfigurableInput, Input, InputAction};
use self::render::{BuiltinSoftwareDrawer, RenderBackend, SoftwareBackend, WgpuBackend};

// AssetStore and MemoryAssetStore moved to assets.rs

/// Runtime application wrapper. Logic controller.
pub struct RuntimeApp<I, A, S> {
    engine: Engine,
    visual: VisualState,
    input: I,
    audio: A,
    assets: S,
    ui: UiState,
    last_bgm_path: Option<String>,
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
            last_bgm_path: None,
        };
        let audio_commands = app.engine.take_audio_commands();
        app.apply_audio_commands(&audio_commands);
        Ok(app)
    }

    /// Creates a new RuntimeApp trying to use RodioBackend (if available), falling back to SilentAudio.
    pub fn new_auto(
        engine: Engine,
        input: I,
        assets: Arc<S>,
    ) -> visual_novel_engine::VnResult<RuntimeApp<I, Box<dyn Audio>, Arc<S>>>
    where
        S: AssetStore + Send + Sync + 'static,
    {
        let audio: Box<dyn Audio> = match RodioBackend::new(assets.clone()) {
            Ok(backend) => {
                eprintln!("Audio: Using Rodio Backend");
                Box::new(backend)
            }
            Err(e) => {
                eprintln!(
                    "Audio: Rodio initialization failed ({}), using SilentAudio",
                    e
                );
                Box::new(SilentAudio)
            }
        };

        RuntimeApp::new(engine, input, audio, assets)
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
            InputAction::Back | InputAction::Menu => {
                // Action recognized but currently non-mutating in runtime mode.
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
                if self.last_bgm_path.as_deref() != Some(music.as_ref()) {
                    self.audio.play_music(music);
                    self.last_bgm_path = Some(music.to_string());
                }
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
                    self.last_bgm_path = Some(path.as_ref().to_string());
                }
                AudioCommand::StopBgm { .. } => {
                    self.audio.stop_music();
                    self.last_bgm_path = None;
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
    let event_loop = EventLoop::new().expect("failed to create event loop");
    #[allow(deprecated)]
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("VN Runtime")
            .with_inner_size(LogicalSize::new(960.0, 540.0))
            .with_min_inner_size(LogicalSize::new(640.0, 360.0))
            .build(&event_loop)
            .expect("failed to build runtime window"),
    );

    let size = window.inner_size();

    // Initialize Backend with Fallback
    let mut backend: Box<dyn RenderBackend> =
        match WgpuBackend::new(window.clone(), size.width, size.height) {
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
                    window.clone(),
                    size.width,
                    size.height,
                    Box::new(BuiltinSoftwareDrawer),
                ))
            }
        };

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(size) => {
                        backend.resize(size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        if let Err(e) = backend.render(app.ui()) {
                            eprintln!("Render error: {}", e);
                            elwt.exit();
                        }
                    }
                    _ => {
                        let action = app.input.handle_window_event(&event);
                        match app.handle_action(action) {
                            Ok(true) => {
                                window.request_redraw();
                            }
                            Ok(false) => {
                                elwt.exit();
                            }
                            Err(_) => {
                                elwt.exit();
                            }
                        }
                    }
                },
                Event::AboutToWait => {
                    // window.request_redraw();
                }
                _ => {}
            }
        })
        .expect("event loop error");

    // The run function in 0.29 may return, but we treat this as a divergent function
    std::process::exit(0);
}
