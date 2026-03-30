use std::time::Duration;

use eframe::egui;
use tracing::instrument;
use visual_novel_engine::{
    localization_key, AssetId, AudioCommand, Engine, EventCompiled, LocalizationCatalog,
};

use super::super::node_types::ToastState;
use super::state::PlayerSessionState;

#[path = "content.rs"]
mod content;
#[path = "controls.rs"]
mod controls;

pub fn render_player_ui(
    engine: &mut Option<Engine>,
    toast: &mut Option<ToastState>,
    player: &mut PlayerSessionState,
    player_locale: &mut String,
    localization_catalog: &LocalizationCatalog,
    ctx: &egui::Context,
) -> Vec<AudioCommand> {
    let mut audio_commands = Vec::new();
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(ref mut eng) = engine {
            audio_commands.extend(render_event_ui(
                ui,
                ctx,
                eng,
                toast,
                player,
                player_locale,
                localization_catalog,
            ));
        } else {
            render_no_script_ui(ui);
        }
    });
    audio_commands
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
) -> Vec<AudioCommand> {
    let mut audio_commands = Vec::new();
    let now_sec = ctx.input(|i| i.time);
    let current_ip = engine.state().position;
    let ip_changed = player.on_position_changed(current_ip, now_sec);
    if ip_changed {
        queue_scene_audio_if_current(engine, &mut audio_commands);
    }

    controls::render_header_bar(ui, engine, toast, player, now_sec);
    ui.separator();
    controls::render_player_controls(ui, player, player_locale, localization_catalog);
    controls::render_backlog_window(ctx, engine, player);
    controls::render_choice_history_window(ctx, engine, player);
    ui.separator();

    match engine.current_event() {
        Ok(event) => {
            if player.should_skip_current(&event, engine) {
                if matches!(event, EventCompiled::ExtCall { .. }) {
                    let _ = engine.resume();
                    audio_commands.extend(engine.take_audio_commands());
                } else if let Ok((cmd, _)) = engine.step() {
                    audio_commands.extend(cmd);
                }
                ctx.request_repaint_after(Duration::from_millis(16));
                return audio_commands;
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
                    if content::render_dialogue(
                        ui,
                        ctx,
                        player,
                        &localized_speaker,
                        &localized_text,
                        now_sec,
                    ) {
                        if let Ok((cmd, _)) = engine.step() {
                            audio_commands.extend(cmd);
                        }
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
                    content::render_choice(
                        ui,
                        engine,
                        toast,
                        &localized_prompt,
                        &localized_options,
                        &c.options,
                        &mut audio_commands,
                    );
                }
                EventCompiled::Scene(s) => {
                    if content::render_scene(
                        ui,
                        player,
                        s.background.as_ref().map(|s| s.as_ref()),
                        now_sec,
                    ) {
                        if let Ok((cmd, _)) = engine.step() {
                            audio_commands.extend(cmd);
                        }
                    }
                }
                EventCompiled::Transition(t) => {
                    content::render_transition(
                        ui,
                        ctx,
                        engine,
                        t.kind,
                        t.duration_ms,
                        &mut audio_commands,
                    );
                }
                EventCompiled::ExtCall { .. } => {
                    ui.label("Processing custom action...");
                    let _ = engine.resume();
                    audio_commands.extend(engine.take_audio_commands());
                }
                EventCompiled::Jump { .. }
                | EventCompiled::SetFlag { .. }
                | EventCompiled::SetVar { .. }
                | EventCompiled::JumpIf { .. }
                | EventCompiled::Patch(_)
                | EventCompiled::AudioAction(_)
                | EventCompiled::SetCharacterPosition(_) => {
                    ui.label("Processing...");
                    if let Ok((cmd, _)) = engine.step() {
                        audio_commands.extend(cmd);
                    }
                }
            }
        }
        Err(e) => {
            let error_str = format!("{e}");
            if error_str.contains("End") || error_str.contains("position") {
                content::render_end(ui, engine, toast, player, now_sec, &mut audio_commands);
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    format!(
                        "Player runtime error at ip {}: {}",
                        engine.state().position,
                        e
                    ),
                );
            }
        }
    }
    audio_commands
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

fn queue_scene_audio_if_current(engine: &Engine, audio_commands: &mut Vec<AudioCommand>) {
    let Ok(EventCompiled::Scene(scene)) = engine.current_event() else {
        return;
    };
    let Some(music) = &scene.music else {
        return;
    };
    audio_commands.push(AudioCommand::PlayBgm {
        resource: AssetId::from_path(music.as_ref()),
        path: music.clone(),
        r#loop: true,
        volume: None,
        fade_in: Duration::from_millis(500),
    });
}

pub(crate) fn byte_index_for_char(text: &str, char_count: usize) -> usize {
    text.char_indices()
        .nth(char_count)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}
