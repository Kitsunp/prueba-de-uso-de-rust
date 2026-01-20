//! Runtime layer for driving the engine with a winit + pixels loop.

mod loader;

pub use loader::{AsyncLoader, LoadRequest, LoadResult};

use std::collections::HashMap;

use pixels::{Pixels, SurfaceTexture};
use visual_novel_engine::{
    AudioCommand, Engine, EventCompiled, RenderOutput, TextRenderer, UiState, UiView, VisualState,
};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

/// Renderer trait for drawing UI state to a frame buffer.
pub trait Renderer {
    fn render(&mut self, frame: &mut [u8], size: (u32, u32), ui: &UiState);
}

/// Input trait that maps window events into engine actions.
pub trait Input {
    fn handle_window_event(&mut self, event: &WindowEvent) -> InputAction;
}

/// Audio trait stub for runtime audio playback.
pub trait Audio {
    fn play_music(&mut self, id: &str);
    fn stop_music(&mut self);
}

/// Asset store trait for runtime resource loading.
pub trait AssetStore {
    fn load_bytes(&self, id: &str) -> Option<Vec<u8>>;
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
    fn load_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.assets.get(id).cloned()
    }
}

/// Runtime application wrapper.
pub struct RuntimeApp<R, I, A, S> {
    engine: Engine,
    visual: VisualState,
    renderer: R,
    input: I,
    audio: A,
    assets: S,
    ui: UiState,
}

impl<R, I, A, S> RuntimeApp<R, I, A, S>
where
    R: Renderer,
    I: Input,
    A: Audio,
    S: AssetStore,
{
    pub fn new(
        engine: Engine,
        renderer: R,
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
            renderer,
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

    pub fn render_frame(&mut self, frame: &mut [u8], size: (u32, u32)) {
        self.renderer.render(frame, size, &self.ui);
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
                AudioCommand::PlayBgm { resource, .. } => {
                    self.audio.play_music(&resource.0.to_string());
                }
                AudioCommand::StopBgm { .. } => {
                    self.audio.stop_music();
                }
                AudioCommand::PlaySfx { .. } => {}
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

/// Simple pixels-based renderer.
#[derive(Default)]
pub struct PixelsRenderer;

impl Renderer for PixelsRenderer {
    fn render(&mut self, frame: &mut [u8], size: (u32, u32), ui: &UiState) {
        let (width, height) = size;
        let background = match &ui.view {
            UiView::Dialogue { .. } => [32, 32, 64, 255],
            UiView::Choice { .. } => [24, 48, 48, 255],
            UiView::Scene { .. } => [48, 24, 48, 255],
            UiView::System { .. } => [48, 48, 48, 255],
        };
        clear(frame, background);

        let dialog_height = height / 3;
        let dialog_y = height.saturating_sub(dialog_height + 16);
        match &ui.view {
            UiView::Dialogue { .. } | UiView::Choice { .. } => {
                draw_rect(
                    frame,
                    (width, height),
                    RectSpec {
                        x: 16,
                        y: dialog_y,
                        width: width.saturating_sub(32),
                        height: dialog_height,
                        color: [12, 12, 12, 220],
                    },
                );
            }
            UiView::Scene { .. } => {
                draw_rect(
                    frame,
                    (width, height),
                    RectSpec {
                        x: 16,
                        y: 16,
                        width: width.saturating_sub(32),
                        height: height.saturating_sub(32),
                        color: [20, 20, 20, 180],
                    },
                );
            }
            UiView::System { .. } => {
                draw_rect(
                    frame,
                    (width, height),
                    RectSpec {
                        x: 16,
                        y: 16,
                        width: width.saturating_sub(32),
                        height: 48,
                        color: [96, 16, 16, 200],
                    },
                );
            }
        }

        if let UiView::Choice { options, .. } = &ui.view {
            let option_height = 24;
            let mut y = dialog_y + 16;
            for _ in options {
                draw_rect(
                    frame,
                    (width, height),
                    RectSpec {
                        x: 32,
                        y,
                        width: width.saturating_sub(64),
                        height: option_height,
                        color: [40, 120, 120, 220],
                    },
                );
                y = y.saturating_add(option_height + 8);
            }
        }
    }
}

fn clear(frame: &mut [u8], color: [u8; 4]) {
    for chunk in frame.chunks_exact_mut(4) {
        chunk.copy_from_slice(&color);
    }
}

struct RectSpec {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
}

fn draw_rect(frame: &mut [u8], size: (u32, u32), rect: RectSpec) {
    let (width, height) = size;
    let max_x = (rect.x + rect.width).min(width);
    let max_y = (rect.y + rect.height).min(height);
    for row in rect.y..max_y {
        for col in rect.x..max_x {
            let idx = ((row * width + col) * 4) as usize;
            if idx + 4 <= frame.len() {
                frame[idx..idx + 4].copy_from_slice(&rect.color);
            }
        }
    }
}

/// Run the runtime loop using winit and pixels.
pub fn run_winit<R, I, A, S>(mut app: RuntimeApp<R, I, A, S>) -> !
where
    R: Renderer + 'static,
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
    let surface = SurfaceTexture::new(size.width, size.height, &window);
    let mut pixels =
        Pixels::new(size.width, size.height, surface).expect("failed to create pixel surface");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    let _ = pixels.resize_surface(size.width, size.height);
                    let _ = pixels.resize_buffer(size.width, size.height);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    let _ = pixels.resize_surface(new_inner_size.width, new_inner_size.height);
                    let _ = pixels.resize_buffer(new_inner_size.width, new_inner_size.height);
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
                let extent = pixels.context().texture_extent;
                let frame = pixels.frame_mut();
                app.render_frame(frame, (extent.width, extent.height));
                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
