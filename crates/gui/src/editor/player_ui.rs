//! Player UI for testing stories in the editor.
//!
//! This module provides the interactive player mode UI,
//! rendering each event type appropriately and handling user input.

use std::time::Duration;

use eframe::egui;
use tracing::{info, instrument};
use visual_novel_engine::{ChoiceOptionCompiled, Engine, EventCompiled};

use super::node_types::ToastState;

/// Renders the player mode UI.
#[instrument(skip_all)]
pub fn render_player_ui(
    engine: &mut Option<Engine>,
    toast: &mut Option<ToastState>,
    ctx: &egui::Context,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(ref mut eng) = engine {
            render_event_ui(ui, ctx, eng, toast);
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
) {
    ui.horizontal(|ui| {
        ui.heading("Player Mode");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Restart").clicked() {
                info!("Restarting story");
                if engine.jump_to_label("start").is_ok() {
                    *toast = Some(ToastState::success("Story restarted"));
                }
            }
        });
    });
    ui.separator();

    match engine.current_event() {
        Ok(event) => {
            ui.add_space(20.0);
            match event {
                EventCompiled::Dialogue(d) => {
                    render_dialogue(ui, engine, d.speaker.as_ref(), d.text.as_ref());
                }
                EventCompiled::Choice(c) => {
                    render_choice(ui, engine, toast, c.prompt.as_ref(), &c.options);
                }
                EventCompiled::Scene(s) => {
                    render_scene(ui, engine, s.background.as_ref().map(|s| s.as_ref()));
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
                render_end(ui, engine, toast);
            } else {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
            }
        }
    }
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

fn render_dialogue(ui: &mut egui::Ui, engine: &mut Engine, speaker: &str, text: &str) {
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
            ui.label(egui::RichText::new(text).size(16.0));
        });

    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Continue").clicked() {
                let _ = engine.step();
            }
        });
    });
}

fn render_choice(
    ui: &mut egui::Ui,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    prompt: &str,
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
        if ui
            .add(egui::Button::new(option.text.as_ref()).min_size(egui::vec2(200.0, 40.0)))
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

fn render_scene(ui: &mut egui::Ui, engine: &mut Engine, background: Option<&str>) {
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
        let _ = engine.step();
    }
}

fn render_end(ui: &mut egui::Ui, engine: &mut Engine, toast: &mut Option<ToastState>) {
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
                *toast = Some(ToastState::success("Story restarted"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_player_ui_module_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
