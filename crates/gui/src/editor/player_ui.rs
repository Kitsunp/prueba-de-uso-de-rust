//! Player UI for testing stories in the editor.

use std::time::Duration;

use eframe::egui;
use tracing::{info, instrument};
use visual_novel_engine::{
    localization_key, ChoiceHistoryEntry, ChoiceOptionCompiled, Engine, EventCompiled,
    LocalizationCatalog,
};

use super::node_types::ToastState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipMode {
    Off,
    ReadOnly,
    All,
}

#[derive(Clone, Debug)]
pub struct PlayerSessionState {
    pub show_backlog: bool,
    pub show_choice_history: bool,
    pub autoplay_enabled: bool,
    pub autoplay_delay_ms: u64,
    pub text_chars_per_second: f32,
    pub skip_mode: SkipMode,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub bgm_muted: bool,
    pub sfx_muted: bool,
    pub voice_muted: bool,
    current_ip: Option<u32>,
    line_started_at_sec: f64,
    last_auto_step_at_sec: Option<f64>,
}

impl Default for PlayerSessionState {
    fn default() -> Self {
        Self {
            show_backlog: false,
            show_choice_history: false,
            autoplay_enabled: false,
            autoplay_delay_ms: 1200,
            text_chars_per_second: 45.0,
            skip_mode: SkipMode::Off,
            bgm_volume: 1.0,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            bgm_muted: false,
            sfx_muted: false,
            voice_muted: false,
            current_ip: None,
            line_started_at_sec: 0.0,
            last_auto_step_at_sec: None,
        }
    }
}

impl PlayerSessionState {
    fn on_position_changed(&mut self, position: u32, now_sec: f64) {
        if self.current_ip != Some(position) {
            self.current_ip = Some(position);
            self.line_started_at_sec = now_sec;
        }
    }

    fn reset_runtime_progress(&mut self, now_sec: f64) {
        self.current_ip = None;
        self.line_started_at_sec = now_sec;
        self.last_auto_step_at_sec = None;
    }

    fn reveal_current_line(&mut self, text: &str, now_sec: f64) {
        let cps = self.text_chars_per_second.max(1.0) as f64;
        let needed = (text.chars().count() as f64) / cps;
        self.line_started_at_sec = now_sec - needed;
    }

    fn visible_text<'a>(&self, text: &'a str, now_sec: f64) -> &'a str {
        if text.is_empty() {
            return text;
        }
        let cps = self.text_chars_per_second.max(1.0) as f64;
        let elapsed = (now_sec - self.line_started_at_sec).max(0.0);
        let visible_chars = (elapsed * cps).floor() as usize;
        let total_chars = text.chars().count();
        if visible_chars >= total_chars {
            return text;
        }
        let byte_end = byte_index_for_char(text, visible_chars);
        &text[..byte_end]
    }

    fn is_text_fully_revealed(&self, text: &str, now_sec: f64) -> bool {
        self.visible_text(text, now_sec).len() == text.len()
    }

    fn should_skip_current(&self, event: &EventCompiled, engine: &Engine) -> bool {
        match self.skip_mode {
            SkipMode::Off => false,
            SkipMode::ReadOnly => {
                matches!(event, EventCompiled::Dialogue(_)) && engine.is_current_dialogue_read()
            }
            SkipMode::All => !matches!(event, EventCompiled::Choice(_)),
        }
    }

    fn autoplay_ready(&self, now_sec: f64) -> bool {
        if !self.autoplay_enabled {
            return false;
        }
        match self.last_auto_step_at_sec {
            Some(last) => (now_sec - last).max(0.0) >= (self.autoplay_delay_ms as f64) / 1000.0,
            None => true,
        }
    }

    fn mark_auto_step(&mut self, now_sec: f64) {
        self.last_auto_step_at_sec = Some(now_sec);
    }
}

fn byte_index_for_char(text: &str, char_count: usize) -> usize {
    text.char_indices()
        .nth(char_count)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

/// Renders the player mode UI.
#[instrument(skip_all)]
pub fn render_player_ui(
    engine: &mut Option<Engine>,
    toast: &mut Option<ToastState>,
    player: &mut PlayerSessionState,
    player_locale: &mut String,
    localization_catalog: &LocalizationCatalog,
    ctx: &egui::Context,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(ref mut eng) = engine {
            render_event_ui(
                ui,
                ctx,
                eng,
                toast,
                player,
                player_locale,
                localization_catalog,
            );
        } else {
            render_no_script_ui(ui);
        }
    });
}

fn render_no_script_ui(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.heading("No script loaded");
            ui.add_space(10.0);
            ui.label("Use File -> Open Script to load a story");
        });
    });
}

#[instrument(skip_all)]
fn render_event_ui(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    player: &mut PlayerSessionState,
    player_locale: &mut String,
    localization_catalog: &LocalizationCatalog,
) {
    let now_sec = ctx.input(|i| i.time);
    let current_ip = engine.state().position;
    player.on_position_changed(current_ip, now_sec);

    render_header_bar(ui, engine, toast, player, now_sec);
    ui.separator();
    render_player_controls(ui, player, player_locale, localization_catalog);
    render_backlog_window(ctx, engine, player);
    render_choice_history_window(ctx, engine, player);
    ui.separator();

    match engine.current_event() {
        Ok(event) => {
            if player.should_skip_current(&event, engine) {
                let _ = engine.step();
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }

            ui.add_space(14.0);
            match event {
                EventCompiled::Dialogue(d) => {
                    let localized_speaker = localize_inline_value(
                        d.speaker.as_ref(),
                        player_locale,
                        localization_catalog,
                    );
                    let localized_text =
                        localize_inline_value(d.text.as_ref(), player_locale, localization_catalog);
                    if render_dialogue(
                        ui,
                        ctx,
                        player,
                        &localized_speaker,
                        &localized_text,
                        now_sec,
                    ) {
                        let _ = engine.step();
                    }
                }
                EventCompiled::Choice(c) => {
                    let localized_prompt = localize_inline_value(
                        c.prompt.as_ref(),
                        player_locale,
                        localization_catalog,
                    );
                    let localized_options = c
                        .options
                        .iter()
                        .map(|option| {
                            localize_inline_value(
                                option.text.as_ref(),
                                player_locale,
                                localization_catalog,
                            )
                        })
                        .collect::<Vec<_>>();
                    render_choice(
                        ui,
                        engine,
                        toast,
                        &localized_prompt,
                        &localized_options,
                        &c.options,
                    );
                }
                EventCompiled::Scene(s) => {
                    if render_scene(
                        ui,
                        player,
                        s.background.as_ref().map(|s| s.as_ref()),
                        now_sec,
                    ) {
                        let _ = engine.step();
                    }
                }
                EventCompiled::Transition(t) => {
                    render_transition(ui, ctx, engine, t.kind, t.duration_ms);
                }
                EventCompiled::Jump { .. }
                | EventCompiled::SetFlag { .. }
                | EventCompiled::SetVar { .. }
                | EventCompiled::JumpIf { .. }
                | EventCompiled::Patch(_)
                | EventCompiled::ExtCall { .. }
                | EventCompiled::AudioAction(_)
                | EventCompiled::SetCharacterPosition(_) => {
                    ui.label("Processing...");
                    let _ = engine.step();
                }
            }
        }
        Err(e) => {
            let error_str = format!("{}", e);
            if error_str.contains("End") || error_str.contains("position") {
                render_end(ui, engine, toast, player, now_sec);
            } else {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
            }
        }
    }
}

fn render_header_bar(
    ui: &mut egui::Ui,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    player: &mut PlayerSessionState,
    now_sec: f64,
) {
    ui.horizontal(|ui| {
        ui.heading("Player Mode");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Restart").clicked() {
                info!("Restarting story");
                if engine.jump_to_label("start").is_ok() {
                    engine.clear_session_history();
                    player.reset_runtime_progress(now_sec);
                    *toast = Some(ToastState::success("Story restarted"));
                }
            }
        });
    });
}

fn render_player_controls(
    ui: &mut egui::Ui,
    player: &mut PlayerSessionState,
    player_locale: &mut String,
    localization_catalog: &LocalizationCatalog,
) {
    ui.horizontal_wrapped(|ui| {
        if !localization_catalog.locale_codes().is_empty() {
            egui::ComboBox::from_id_source("player_locale_selector")
                .selected_text(format!("Locale: {}", player_locale))
                .show_ui(ui, |ui| {
                    for locale in localization_catalog.locale_codes() {
                        ui.selectable_value(player_locale, locale.clone(), locale);
                    }
                });
        }

        ui.checkbox(&mut player.autoplay_enabled, "Auto");
        ui.add(egui::Slider::new(&mut player.autoplay_delay_ms, 200..=5000).text("Auto delay ms"));
        ui.add(
            egui::Slider::new(&mut player.text_chars_per_second, 10.0..=240.0).text("Text chars/s"),
        );

        egui::ComboBox::from_id_source("player_skip_mode")
            .selected_text(match player.skip_mode {
                SkipMode::Off => "Skip: Off",
                SkipMode::ReadOnly => "Skip: Read",
                SkipMode::All => "Skip: All",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut player.skip_mode, SkipMode::Off, "Skip: Off");
                ui.selectable_value(&mut player.skip_mode, SkipMode::ReadOnly, "Skip: Read");
                ui.selectable_value(&mut player.skip_mode, SkipMode::All, "Skip: All");
            });

        ui.separator();
        ui.checkbox(&mut player.show_backlog, "Backlog");
        ui.checkbox(&mut player.show_choice_history, "Choice history");
    });

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("Audio mix (preview):");
        ui.checkbox(&mut player.bgm_muted, "Mute BGM");
        ui.add(egui::Slider::new(&mut player.bgm_volume, 0.0..=1.0).text("BGM"));
        ui.checkbox(&mut player.sfx_muted, "Mute SFX");
        ui.add(egui::Slider::new(&mut player.sfx_volume, 0.0..=1.0).text("SFX"));
        ui.checkbox(&mut player.voice_muted, "Mute Voice");
        ui.add(egui::Slider::new(&mut player.voice_volume, 0.0..=1.0).text("Voice"));
    });
}

fn render_backlog_window(ctx: &egui::Context, engine: &Engine, player: &mut PlayerSessionState) {
    if !player.show_backlog {
        return;
    }
    let mut open = player.show_backlog;
    egui::Window::new("Backlog")
        .open(&mut open)
        .default_width(420.0)
        .show(ctx, |ui| {
            if engine.state().history.is_empty() {
                ui.label("No dialogue history yet.");
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in engine.state().history.iter().rev() {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(line.speaker.as_ref()).strong());
                        ui.label(line.text.as_ref());
                    });
                    ui.add_space(6.0);
                }
            });
        });
    player.show_backlog = open;
}

fn render_choice_history_window(
    ctx: &egui::Context,
    engine: &Engine,
    player: &mut PlayerSessionState,
) {
    if !player.show_choice_history {
        return;
    }
    let mut open = player.show_choice_history;
    egui::Window::new("Choice History")
        .open(&mut open)
        .default_width(420.0)
        .show(ctx, |ui| {
            if engine.choice_history().is_empty() {
                ui.label("No choices selected yet.");
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in engine.choice_history().iter().rev() {
                    render_choice_history_entry(ui, entry);
                    ui.add_space(6.0);
                }
            });
        });
    player.show_choice_history = open;
}

fn render_choice_history_entry(ui: &mut egui::Ui, entry: &ChoiceHistoryEntry) {
    ui.group(|ui| {
        ui.label(format!(
            "ip {} -> option {}",
            entry.event_ip,
            entry.option_index + 1
        ));
        ui.label(format!("\"{}\"", entry.option_text));
        ui.label(format!("target ip {}", entry.target_ip));
    });
}

fn transition_kind_label(kind: u8) -> &'static str {
    match kind {
        0 => "fade",
        1 => "dissolve",
        2 => "cut",
        _ => "unknown",
    }
}

fn render_transition(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &mut Engine,
    kind: u8,
    duration_ms: u32,
) {
    let ip = engine.state().position;
    let now = ctx.input(|i| i.time);
    let transition_id = egui::Id::new("player_transition_state");

    let mut state = ctx.data_mut(|data| data.get_temp::<(u32, f64)>(transition_id));
    if !matches!(state, Some((prev_ip, _)) if prev_ip == ip) {
        ctx.data_mut(|data| data.insert_temp(transition_id, (ip, now)));
        state = Some((ip, now));
    }

    let start_time = state.map(|(_, t)| t).unwrap_or(now);
    let duration_secs = (duration_ms.max(1) as f64) / 1000.0;
    let elapsed = (now - start_time).max(0.0);
    let progress = (elapsed / duration_secs).clamp(0.0, 1.0) as f32;

    ui.label(format!(
        "Transition {} ({} ms)",
        transition_kind_label(kind),
        duration_ms
    ));
    ui.add(
        egui::ProgressBar::new(progress)
            .desired_width(280.0)
            .show_percentage(),
    );

    if progress >= 1.0 || ui.button("Skip Transition").clicked() {
        let _ = engine.step();
        ctx.data_mut(|data| data.remove::<(u32, f64)>(transition_id));
    } else {
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn render_dialogue(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    player: &mut PlayerSessionState,
    speaker: &str,
    text: &str,
    now_sec: f64,
) -> bool {
    let rendered_text = player.visible_text(text, now_sec);
    let text_complete = player.is_text_fully_revealed(text, now_sec);

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(60, 60, 80))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(speaker).size(18.0).strong());
        });

    ui.add_space(10.0);

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(40, 40, 50))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(rendered_text).size(16.0));
        });

    ui.add_space(20.0);
    let mut should_advance = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if text_complete {
                "Continue"
            } else {
                "Show full"
            };
            if ui.button(label).clicked() {
                if text_complete {
                    should_advance = true;
                } else {
                    player.reveal_current_line(text, now_sec);
                }
            }
        });
    });

    if !text_complete {
        ctx.request_repaint_after(Duration::from_millis(16));
    } else if player.autoplay_ready(now_sec) {
        player.mark_auto_step(now_sec);
        should_advance = true;
    }

    should_advance
}

fn render_choice(
    ui: &mut egui::Ui,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    prompt: &str,
    localized_options: &[String],
    options: &[ChoiceOptionCompiled],
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(80, 60, 60))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(prompt).size(18.0).strong());
        });

    ui.add_space(15.0);
    for (i, option) in options.iter().enumerate() {
        let label = localized_options
            .get(i)
            .map(String::as_str)
            .unwrap_or(option.text.as_ref());
        if ui
            .add(egui::Button::new(label).min_size(egui::vec2(200.0, 40.0)))
            .clicked()
        {
            info!("Choice selected: {} ({})", option.text.as_ref(), i);
            let _ = engine.choose(i);
            *toast = Some(ToastState::success(format!(
                "Selected: {}",
                option.text.as_ref()
            )));
        }
        ui.add_space(5.0);
    }
}

fn localize_inline_value(
    raw: &str,
    locale: &str,
    localization_catalog: &LocalizationCatalog,
) -> String {
    if let Some(key) = localization_key(raw) {
        localization_catalog.resolve_or_key(locale, key)
    } else {
        raw.to_string()
    }
}

fn render_scene(
    ui: &mut egui::Ui,
    player: &mut PlayerSessionState,
    background: Option<&str>,
    now_sec: f64,
) -> bool {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(40, 60, 40))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Scene Change").size(18.0).strong());
            if let Some(bg) = background {
                ui.label(format!("Background: {}", bg));
            }
        });

    ui.add_space(10.0);
    if ui.button("Continue").clicked() {
        return true;
    }
    if player.autoplay_ready(now_sec) {
        player.mark_auto_step(now_sec);
        return true;
    }
    false
}

fn render_end(
    ui: &mut egui::Ui,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    player: &mut PlayerSessionState,
    now_sec: f64,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(50.0);
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(60, 40, 60))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("The End").size(32.0).strong());
            });

        ui.add_space(30.0);
        if ui.button("Play Again").clicked() {
            info!("Restarting from end");
            if engine.jump_to_label("start").is_ok() {
                engine.clear_session_history();
                player.reset_runtime_progress(now_sec);
                *toast = Some(ToastState::success("Story restarted"));
            }
        }
    });
}

#[cfg(test)]
#[path = "tests/player_ui_tests.rs"]
mod tests;
