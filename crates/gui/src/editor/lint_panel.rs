//! Validation panel for displaying lint issues.

use super::lint_checks::{LintIssue, LintSeverity};
use eframe::egui;

/// Panel for displaying validation results.
pub struct LintPanel<'a> {
    issues: &'a [LintIssue],
    selected_node: &'a mut Option<u32>,
}

impl<'a> LintPanel<'a> {
    pub fn new(issues: &'a [LintIssue], selected_node: &'a mut Option<u32>) -> Self {
        Self {
            issues,
            selected_node,
        }
    }

    pub fn ui(self, ui: &mut egui::Ui) {
        ui.heading("📋 Validation Report");
        ui.separator();

        if self.issues.is_empty() {
            ui.label(egui::RichText::new("✅ No issues found.").color(egui::Color32::GREEN));
            return;
        }

        let error_count = self
            .issues
            .iter()
            .filter(|i| i.severity == LintSeverity::Error)
            .count();
        let warning_count = self
            .issues
            .iter()
            .filter(|i| i.severity == LintSeverity::Warning)
            .count();

        ui.label(format!(
            "Found {} errors, {} warnings.",
            error_count, warning_count
        ));
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for issue in self.issues {
                let icon = match issue.severity {
                    LintSeverity::Error => "❌",
                    LintSeverity::Warning => "⚠️",
                    LintSeverity::Info => "ℹ️",
                };

                let color = match issue.severity {
                    LintSeverity::Error => egui::Color32::RED,
                    LintSeverity::Warning => egui::Color32::YELLOW,
                    LintSeverity::Info => egui::Color32::LIGHT_BLUE,
                };

                let text = egui::RichText::new(format!("{} {}", icon, issue.message)).color(color);

                let resp = ui.selectable_label(false, text);

                if resp.clicked() {
                    if let Some(node_id) = issue.node_id {
                        *self.selected_node = Some(node_id);
                    }
                }

                if resp.hovered() {
                    ui.close_menu(); // Hack to close tooltips if any? No.
                }

                ui.separator();
            }
        });
    }
}
