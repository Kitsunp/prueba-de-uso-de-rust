//! Save Preview / Diff Dialog
//!
//! Visualizes changes before saving script.

use eframe::egui;
use visual_novel_engine::ScriptRaw;

pub struct DiffDialog {
    previous_script: Option<ScriptRaw>,
    current_script: ScriptRaw,
    stats: DiffStats,
    lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Default)]
struct DiffStats {
    added_events: usize,
    removed_events: usize,
    modified_events: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffKind {
    Added,
    Removed,
    Context,
}

#[derive(Clone, Debug)]
struct DiffLine {
    kind: DiffKind,
    text: String,
}

impl DiffDialog {
    pub fn new(previous_script: Option<&ScriptRaw>, current_script: &ScriptRaw) -> Self {
        let previous_script = previous_script.cloned();
        let current_script = current_script.clone();
        let stats = compute_stats(previous_script.as_ref(), &current_script);
        let lines = build_diff_lines(previous_script.as_ref(), &current_script);

        Self {
            previous_script,
            current_script,
            stats,
            lines,
        }
    }

    /// Renders the diff dialog. Returns true if "Confirm" is clicked.
    pub fn show(&self, ctx: &egui::Context, open: &mut bool) -> bool {
        let mut confirmed = false;
        if *open {
            egui::Window::new("Confirmar Cambios")
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(780.0, 520.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label("Estas por guardar cambios en el script.");
                    ui.separator();

                    ui.heading("Resumen de Cambios");
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("+{}", self.stats.added_events))
                                .color(egui::Color32::GREEN)
                                .strong(),
                        );
                        ui.label("agregados");
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("~{}", self.stats.modified_events))
                                .color(egui::Color32::YELLOW)
                                .strong(),
                        );
                        ui.label("modificados");
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("-{}", self.stats.removed_events))
                                .color(egui::Color32::RED)
                                .strong(),
                        );
                        ui.label("eliminados");
                    });

                    if self.previous_script.is_none() {
                        ui.label("Archivo nuevo (sin snapshot previo).");
                    } else {
                        ui.label(format!("Eventos actuales: {}", self.current_script.events.len()));
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("Diff (estilo Git):").strong());
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for line in &self.lines {
                                let (prefix, color) = match line.kind {
                                    DiffKind::Added => ("+", egui::Color32::GREEN),
                                    DiffKind::Removed => ("-", egui::Color32::RED),
                                    DiffKind::Context => (" ", egui::Color32::GRAY),
                                };
                                ui.label(
                                    egui::RichText::new(format!("{} {}", prefix, line.text))
                                        .monospace()
                                        .color(color),
                                );
                            }
                        });

                    ui.separator();
                    ui.label(
                        egui::RichText::new("Esto sobrescribira el archivo en disco.")
                            .color(egui::Color32::YELLOW),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            *open = false;
                        }
                        if ui.button("Confirmar Guardado").clicked() {
                            confirmed = true;
                            *open = false;
                        }
                    });
                });
        }
        confirmed
    }
}

fn compute_stats(previous: Option<&ScriptRaw>, current: &ScriptRaw) -> DiffStats {
    let Some(previous) = previous else {
        return DiffStats {
            added_events: current.events.len(),
            removed_events: 0,
            modified_events: 0,
        };
    };

    let mut stats = DiffStats::default();
    let old_len = previous.events.len();
    let new_len = current.events.len();
    let common = old_len.min(new_len);

    for idx in 0..common {
        let old_value = serde_json::to_value(&previous.events[idx]).ok();
        let new_value = serde_json::to_value(&current.events[idx]).ok();
        if old_value != new_value {
            stats.modified_events += 1;
        }
    }

    if new_len > old_len {
        stats.added_events = new_len - old_len;
    } else if old_len > new_len {
        stats.removed_events = old_len - new_len;
    }

    stats
}

fn build_diff_lines(previous: Option<&ScriptRaw>, current: &ScriptRaw) -> Vec<DiffLine> {
    let new_json = current
        .to_json()
        .unwrap_or_else(|_| "<error serializando script actual>".to_string());
    let new_lines: Vec<&str> = new_json.lines().collect();

    let Some(previous) = previous else {
        return new_lines
            .into_iter()
            .map(|line| DiffLine {
                kind: DiffKind::Added,
                text: line.to_string(),
            })
            .collect();
    };

    let old_json = previous
        .to_json()
        .unwrap_or_else(|_| "<error serializando script previo>".to_string());
    let old_lines: Vec<&str> = old_json.lines().collect();

    if old_lines == new_lines {
        return vec![DiffLine {
            kind: DiffKind::Context,
            text: "Sin cambios de contenido".to_string(),
        }];
    }

    let mut prefix = 0usize;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < old_lines.len().saturating_sub(prefix)
        && suffix < new_lines.len().saturating_sub(prefix)
        && old_lines[old_lines.len() - 1 - suffix] == new_lines[new_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let mut out = Vec::new();
    push_context_window(&mut out, &old_lines[..prefix], true);

    for line in &old_lines[prefix..old_lines.len().saturating_sub(suffix)] {
        out.push(DiffLine {
            kind: DiffKind::Removed,
            text: (*line).to_string(),
        });
    }

    for line in &new_lines[prefix..new_lines.len().saturating_sub(suffix)] {
        out.push(DiffLine {
            kind: DiffKind::Added,
            text: (*line).to_string(),
        });
    }

    push_context_window(
        &mut out,
        &new_lines[new_lines.len().saturating_sub(suffix)..],
        false,
    );

    out
}

fn push_context_window(out: &mut Vec<DiffLine>, lines: &[&str], is_prefix: bool) {
    const CONTEXT_LINES: usize = 4;
    if lines.is_empty() {
        return;
    }

    if lines.len() <= CONTEXT_LINES {
        for line in lines {
            out.push(DiffLine {
                kind: DiffKind::Context,
                text: (*line).to_string(),
            });
        }
        return;
    }

    if is_prefix {
        for line in &lines[..CONTEXT_LINES] {
            out.push(DiffLine {
                kind: DiffKind::Context,
                text: (*line).to_string(),
            });
        }
        out.push(DiffLine {
            kind: DiffKind::Context,
            text: format!("... {} lineas sin cambios ...", lines.len() - CONTEXT_LINES),
        });
    } else {
        out.push(DiffLine {
            kind: DiffKind::Context,
            text: format!("... {} lineas sin cambios ...", lines.len() - CONTEXT_LINES),
        });
        for line in &lines[lines.len() - CONTEXT_LINES..] {
            out.push(DiffLine {
                kind: DiffKind::Context,
                text: (*line).to_string(),
            });
        }
    }
}
