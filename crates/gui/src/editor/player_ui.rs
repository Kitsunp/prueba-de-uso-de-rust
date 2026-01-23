//! Player UI for testing stories in the editor.
//!
//! This module provides the interactive player mode UI,
//! rendering each event type appropriately and handling user input.

use eframe::egui;
use tracing::{info, instrument};
use visual_novel_engine::{Engine, EventCompiled};

use super::node_types::ToastState;

/// Renders the player mode UI.
///
/// # Arguments
/// * `engine` - Mutable reference to the game engine
/// * `toast` - Optional toast state for feedback
/// * `ctx` - egui context
#[instrument(skip_all)]
pub fn render_player_ui(
    engine: &mut Option<Engine>,
    toast: &mut Option<ToastState>,
    ctx: &egui::Context,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(ref mut eng) = engine {
            render_event_ui(ui, eng, toast);
        } else {
            render_no_script_ui(ui);
        }
    });
}

/// Renders UI when no script is loaded.
fn render_no_script_ui(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.heading("📖 No script loaded");
            ui.add_space(10.0);
            ui.label("Use File → Open Script to load a story");
        });
    });
}

/// Renders the appropriate UI for the current event.
#[instrument(skip_all)]
fn render_event_ui(ui: &mut egui::Ui, engine: &mut Engine, toast: &mut Option<ToastState>) {
    // Header
    ui.horizontal(|ui| {
        ui.heading("🎮 Player Mode");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Restart").clicked() {
                info!("Restarting story");
                if engine.jump_to_label("start").is_ok() {
                    *toast = Some(ToastState::success("Story restarted"));
                }
            }
        });
    });
    ui.separator();

    // Get current event
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
                EventCompiled::Jump { .. }
                | EventCompiled::SetFlag { .. }
                | EventCompiled::SetVar { .. }
                | EventCompiled::JumpIf { .. }
                | EventCompiled::Patch(_)
                | EventCompiled::ExtCall { .. } => {
                    // Auto-advance non-visual events
                    ui.label("Processing...");
                    let _ = engine.step();
                }
            }
        }
        Err(e) => {
            // End of story or error
            let error_str = format!("{}", e);
            if error_str.contains("End") || error_str.contains("position") {
                render_end(ui, engine, toast);
            } else {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
            }
        }
    }
}

/// Renders a dialogue event with Continue button.
fn render_dialogue(ui: &mut egui::Ui, engine: &mut Engine, speaker: &str, text: &str) {
    // Speaker box
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(60, 60, 80))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("💬")
                        .size(24.0)
                        .color(egui::Color32::LIGHT_BLUE),
                );
                ui.label(
                    egui::RichText::new(speaker)
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });
        });

    ui.add_space(10.0);

    // Text box
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(40, 40, 50))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(16.0)
                    .color(egui::Color32::LIGHT_GRAY),
            );
        });

    ui.add_space(20.0);

    // Continue button
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("Continue ▶").size(14.0))
                .clicked()
            {
                let _ = engine.step();
            }
        });
    });
}

/// Renders a choice event with option buttons.
fn render_choice(
    ui: &mut egui::Ui,
    engine: &mut Engine,
    toast: &mut Option<ToastState>,
    prompt: &str,
    options: &[visual_novel_engine::ChoiceOptionCompiled],
) {
    // Prompt
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(80, 60, 60))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🔀")
                        .size(24.0)
                        .color(egui::Color32::YELLOW),
                );
                ui.label(
                    egui::RichText::new(prompt)
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });
        });

    ui.add_space(15.0);

    // Options as buttons
    for (i, option) in options.iter().enumerate() {
        if ui
            .add(
                egui::Button::new(egui::RichText::new(option.text.as_ref()).size(14.0))
                    .min_size(egui::vec2(200.0, 40.0)),
            )
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

/// Renders a scene change event (auto-advances).
fn render_scene(ui: &mut egui::Ui, engine: &mut Engine, background: Option<&str>) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(40, 60, 40))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🎬")
                        .size(24.0)
                        .color(egui::Color32::GREEN),
                );
                ui.label(
                    egui::RichText::new("Scene Change")
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });
            if let Some(bg) = background {
                ui.label(format!("Background: {}", bg));
            }
        });

    ui.add_space(10.0);

    // Continue button
    if ui.button("Continue ▶").clicked() {
        let _ = engine.step();
    }
}

/// Renders the end of story with restart option.
fn render_end(ui: &mut egui::Ui, engine: &mut Engine, toast: &mut Option<ToastState>) {
    ui.vertical_centered(|ui| {
        ui.add_space(50.0);

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(60, 40, 60))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("🏁 The End")
                        .size(32.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });

        ui.add_space(30.0);

        if ui
            .button(egui::RichText::new("🔄 Play Again").size(16.0))
            .clicked()
        {
            info!("Restarting from end");
            if engine.jump_to_label("start").is_ok() {
                *toast = Some(ToastState::success("Story restarted"));
            }
        }
    });
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_player_ui_module_compiles() {
        // Smoke test - module compiles
        assert!(true);
    }
}
