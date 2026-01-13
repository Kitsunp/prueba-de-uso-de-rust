use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crc32fast::Hasher;
use directories::ProjectDirs;
use eframe::egui;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use visual_novel_engine::{
    Engine, EngineState, EventCompiled, ResourceLimiter, ScriptRaw, SecurityPolicy, UiState,
    UiView, VnError,
};

#[derive(Clone, Debug, Default)]
pub struct DisplayInfo {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct VnConfig {
    pub title: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub fullscreen: bool,
    pub scale_factor: Option<f32>,
}

impl Default for VnConfig {
    fn default() -> Self {
        Self {
            title: "Visual Novel".to_string(),
            width: None,
            height: None,
            fullscreen: false,
            scale_factor: None,
        }
    }
}

impl VnConfig {
    pub fn resolve(&self, display: Option<DisplayInfo>) -> ResolvedConfig {
        let mut width = self.width.unwrap_or(1280.0);
        let mut height = self.height.unwrap_or(720.0);
        let mut fullscreen = self.fullscreen;
        let mut ui_scale = 1.0;
        let mut scale_factor = self.scale_factor.unwrap_or(1.0);

        if let Some(display) = display {
            scale_factor = self.scale_factor.unwrap_or(display.scale_factor.max(1.0));
            if self.width.is_none() || self.height.is_none() {
                if display.height < 720.0 {
                    fullscreen = true;
                    width = display.width;
                    height = display.height;
                    ui_scale = 1.1;
                }
            }
        }

        ResolvedConfig {
            title: self.title.clone(),
            width,
            height,
            fullscreen,
            scale_factor,
            ui_scale,
        }
    }

    pub fn preferences_path(&self) -> PathBuf {
        ProjectDirs::from("com", "vnengine", "visual_novel")
            .map(|dirs| dirs.config_dir().join("prefs.json"))
            .unwrap_or_else(|| PathBuf::from("prefs.json"))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub fullscreen: bool,
    pub scale_factor: f32,
    pub ui_scale: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UserPreferences {
    pub fullscreen: bool,
    pub ui_scale: f32,
    pub vsync: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveData {
    pub script_hash: u32,
    pub state: EngineState,
}

pub fn save_state_to(path: &Path, data: &SaveData) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(data)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    fs::write(path, payload)
}

pub fn load_state_from(path: &Path) -> std::io::Result<SaveData> {
    let raw = fs::read_to_string(path)?;
    let parsed = serde_json::from_str(&raw)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
    Ok(parsed)
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            fullscreen: false,
            ui_scale: 1.0,
            vsync: true,
        }
    }
}

impl UserPreferences {
    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let parsed = serde_json::from_str(&raw)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        Ok(parsed)
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        fs::write(path, payload)
    }
}

#[derive(Debug, Error)]
pub enum GuiError {
    #[error("script error: {0}")]
    Script(#[from] VnError),
    #[error("gui error: {0}")]
    Gui(#[from] eframe::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn run_app(script_json: String, config: Option<VnConfig>) -> Result<(), GuiError> {
    let script_hash = script_checksum(&script_json);
    let script = ScriptRaw::from_json(&script_json)?;
    let engine = Engine::new(
        script,
        SecurityPolicy::default(),
        ResourceLimiter::default(),
    )?;
    let config = config.unwrap_or_default();
    let preferences_path = config.preferences_path();
    let preferences = UserPreferences::load_from(&preferences_path).unwrap_or_default();
    let resolved = config.resolve(None);
    let title = resolved.title.clone();
    let options = native_options(&resolved, &preferences);

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            Box::new(VnApp::new(
                engine,
                resolved,
                preferences,
                preferences_path,
                script_hash,
                cc,
            ))
        }),
    )?;
    Ok(())
}

fn native_options(resolved: &ResolvedConfig, prefs: &UserPreferences) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([resolved.width.max(1.0), resolved.height.max(1.0)]);
    if resolved.fullscreen || prefs.fullscreen {
        viewport = viewport.with_fullscreen(true);
    }

    eframe::NativeOptions {
        viewport,
        vsync: prefs.vsync,
        ..Default::default()
    }
}

struct VnApp {
    engine: Engine,
    config: ResolvedConfig,
    prefs: UserPreferences,
    prefs_path: PathBuf,
    show_settings: bool,
    show_history: bool,
    show_inspector: bool,
    last_error: Option<String>,
    assets: AssetManager,
    applied_scale: f32,
    label_jump_input: String,
    script_hash: u32,
}

impl VnApp {
    fn new(
        engine: Engine,
        config: ResolvedConfig,
        mut prefs: UserPreferences,
        prefs_path: PathBuf,
        script_hash: u32,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        if config.fullscreen {
            prefs.fullscreen = true;
        }
        let mut app = Self {
            engine,
            config,
            prefs,
            prefs_path,
            show_settings: false,
            show_history: false,
            show_inspector: false,
            last_error: None,
            assets: AssetManager::default(),
            applied_scale: 0.0,
            label_jump_input: String::new(),
            script_hash,
        };
        let scale = app.config.scale_factor * app.prefs.ui_scale;
        cc.egui_ctx.set_pixels_per_point(scale.max(0.5));
        app.applied_scale = scale;
        app
    }

    fn render_scene(&mut self, ui: &mut egui::Ui) {
        let visual = self.engine.visual_state();
        ui.group(|ui| {
            ui.heading("Scene");
            if let Some(background) = visual.background.as_deref() {
                if let Some(texture) = self.assets.texture_for_path(ui.ctx(), background) {
                    let size = ui.available_width();
                    ui.add(egui::Image::new(texture).max_width(size));
                } else {
                    ui.label(format!("Background: {background}"));
                }
            }
            if let Some(music) = visual.music.as_deref() {
                ui.label(format!("Music: {music}"));
            }
            if !visual.characters.is_empty() {
                for character in &visual.characters {
                    let mut line = character.name.as_ref().to_string();
                    if let Some(expression) = character.expression.as_deref() {
                        line.push_str(&format!(" ({expression})"));
                    }
                    if let Some(position) = character.position.as_deref() {
                        line.push_str(&format!(" @ {position}"));
                    }
                    ui.label(line);
                }
            }
        });
    }

    fn render_ui(&mut self, ui: &mut egui::Ui) {
        match self.engine.current_event() {
            Ok(event) => self.render_event(ui, event),
            Err(VnError::EndOfScript) => {
                ui.label("End of script.");
            }
            Err(err) => {
                ui.colored_label(egui::Color32::RED, err.to_string());
            }
        }
    }

    fn render_event(&mut self, ui: &mut egui::Ui, event: EventCompiled) {
        let ui_state = UiState::from_event(&event, self.engine.visual_state());
        ui.group(|ui| match ui_state.view {
            UiView::Dialogue { speaker, text } => {
                ui.heading(speaker);
                ui.label(text);
                if ui.button("History").clicked() {
                    self.show_history = !self.show_history;
                }
                if ui.button("Continue").clicked() {
                    self.advance();
                }
            }
            UiView::Choice { prompt, options } => {
                ui.heading(prompt);
                for (idx, option) in options.into_iter().enumerate() {
                    if ui.button(option).clicked() {
                        self.choose(idx);
                    }
                }
                if ui.button("History").clicked() {
                    self.show_history = !self.show_history;
                }
            }
            UiView::Scene { description } => {
                ui.label(description);
                if ui.button("History").clicked() {
                    self.show_history = !self.show_history;
                }
                if ui.button("Continue").clicked() {
                    self.advance();
                }
            }
            UiView::System { message } => {
                ui.label(message);
                if ui.button("History").clicked() {
                    self.show_history = !self.show_history;
                }
                if ui.button("Continue").clicked() {
                    self.advance();
                }
            }
        });
    }

    fn render_history(&self, ctx: &egui::Context) {
        if !self.show_history {
            return;
        }
        egui::Window::new("History").show(ctx, |ui| {
            for entry in &self.engine.state().history {
                ui.label(format!("{}: {}", entry.speaker, entry.text));
                ui.separator();
            }
        });
    }

    fn render_inspector(&mut self, ctx: &egui::Context) {
        if !self.show_inspector {
            return;
        }
        let event_summary = match self.engine.current_event() {
            Ok(event) => event_kind(&event),
            Err(err) => format!("Error: {err}"),
        };
        let history_bytes = history_bytes(&self.engine.state().history);
        let dt = ctx.input(|i| i.unstable_dt);
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        egui::Window::new("Inspector").show(ctx, |ui| {
            ui.label(format!("IP: {}", self.engine.state().position));
            ui.label(format!("Event: {event_summary}"));
            ui.label(format!("FPS: {:.1}", fps));
            ui.label(format!("History bytes (approx): {}", history_bytes));
            ui.label(format!("Textures cached: {}", self.assets.textures.len()));
            ui.separator();
            ui.label("Flags:");
            let flag_count = self.engine.flag_count();
            for flag_id in 0..flag_count {
                let mut value = self.engine.state().get_flag(flag_id);
                if ui.checkbox(&mut value, format!("flag {flag_id}")).changed() {
                    self.engine.set_flag(flag_id, value);
                }
            }
            ui.separator();
            ui.label("Jump to label:");
            ui.text_edit_singleline(&mut self.label_jump_input);
            if ui.button("Jump").clicked() {
                if let Err(err) = self.engine.jump_to_label(&self.label_jump_input) {
                    self.last_error = Some(err.to_string());
                }
            }
            ui.separator();
            ui.label("Available labels:");
            for label in self.engine.labels().keys() {
                ui.label(label);
            }
        });
    }

    fn advance(&mut self) {
        match self.engine.step() {
            Ok(()) => {}
            Err(VnError::EndOfScript) => {}
            Err(err) => self.last_error = Some(err.to_string()),
        }
    }

    fn choose(&mut self, index: usize) {
        match self.engine.choose(index) {
            Ok(_) => {}
            Err(VnError::EndOfScript) => {}
            Err(err) => self.last_error = Some(err.to_string()),
        }
    }

    fn apply_preferences(&mut self, ctx: &egui::Context) {
        let scale = (self.config.scale_factor * self.prefs.ui_scale).max(0.5);
        if (scale - self.applied_scale).abs() > f32::EPSILON {
            ctx.set_pixels_per_point(scale);
            self.applied_scale = scale;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.prefs.fullscreen));
    }

    fn save_state(&mut self, path: &Path) {
        let data = SaveData {
            script_hash: self.script_hash,
            state: self.engine.state().clone(),
        };
        if let Err(err) = save_state_to(path, &data) {
            self.last_error = Some(format!("Failed to save state: {err}"));
        }
    }

    fn load_state(&mut self, path: &Path) {
        match load_state_from(path) {
            Ok(data) => {
                if data.script_hash != self.script_hash {
                    self.last_error =
                        Some("Save data does not match the current script".to_string());
                    return;
                }
                if let Err(err) = self.engine.set_state(data.state) {
                    self.last_error = Some(format!("Failed to load state: {err}"));
                }
            }
            Err(err) => self.last_error = Some(format!("Failed to load state: {err}")),
        }
    }

    fn persist_preferences(&self) {
        if let Err(err) = self.prefs.save_to(&self.prefs_path) {
            eprintln!("Failed to save GUI preferences: {err}");
        }
    }
}

impl eframe::App for VnApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings = !self.show_settings;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.show_inspector = !self.show_inspector;
        }

        self.apply_preferences(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&self.config.title);
            ui.separator();
            self.render_scene(ui);
            ui.separator();
            self.render_ui(ui);
            if let Some(message) = &self.last_error {
                ui.separator();
                ui.colored_label(egui::Color32::RED, message);
            }
        });

        if self.show_settings {
            let mut dirty = false;
            egui::Window::new("Settings").show(ctx, |ui| {
                dirty |= ui
                    .checkbox(&mut self.prefs.fullscreen, "Fullscreen")
                    .changed();
                dirty |= ui
                    .checkbox(&mut self.prefs.vsync, "VSync (restart required)")
                    .changed();
                dirty |= ui
                    .add(egui::Slider::new(&mut self.prefs.ui_scale, 0.75..=2.0).text("UI Scale"))
                    .changed();
                if ui.button("Save State").clicked() {
                    if let Some(path) = FileDialog::new().set_title("Save State").save_file() {
                        self.save_state(&path);
                    }
                }
                if ui.button("Load State").clicked() {
                    if let Some(path) = FileDialog::new().set_title("Load State").pick_file() {
                        self.load_state(&path);
                    }
                }
            });

            if dirty {
                self.persist_preferences();
            }
        }

        self.render_history(ctx);
        self.render_inspector(ctx);
    }
}

fn script_checksum(script_json: &str) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(script_json.as_bytes());
    hasher.finalize()
}

fn history_bytes(
    history: &std::collections::VecDeque<visual_novel_engine::DialogueCompiled>,
) -> usize {
    history
        .iter()
        .map(|entry| entry.speaker.len() + entry.text.len())
        .sum()
}

fn event_kind(event: &EventCompiled) -> String {
    match event {
        EventCompiled::Dialogue(_) => "Dialogue".to_string(),
        EventCompiled::Choice(_) => "Choice".to_string(),
        EventCompiled::Scene(_) => "Scene".to_string(),
        EventCompiled::Jump { .. } => "Jump".to_string(),
        EventCompiled::SetFlag { .. } => "SetFlag".to_string(),
        EventCompiled::SetVar { .. } => "SetVar".to_string(),
        EventCompiled::JumpIf { .. } => "JumpIf".to_string(),
        EventCompiled::Patch(_) => "Patch".to_string(),
    }
}

#[derive(Default)]
struct AssetManager {
    textures: HashMap<String, egui::TextureHandle>,
}

impl AssetManager {
    fn texture_for_path(
        &mut self,
        ctx: &egui::Context,
        path: &str,
    ) -> Option<&egui::TextureHandle> {
        if self.textures.contains_key(path) {
            return self.textures.get(path);
        }
        let image = image::open(path).ok()?;
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();
        let texture = ctx.load_texture(
            path.to_string(),
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::default(),
        );
        self.textures.insert(path.to_string(), texture);
        self.textures.get(path)
    }
}
