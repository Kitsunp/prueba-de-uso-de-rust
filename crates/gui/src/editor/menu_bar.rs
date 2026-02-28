use crate::editor::{DiffDialog, EditorWorkbench};
use eframe::egui;

pub fn render_menu_bar(ui: &mut egui::Ui, workbench: &mut EditorWorkbench) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Open Project...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Project", &["toml"])
                    .pick_file()
                {
                    workbench.load_project(path);
                    ui.close_menu();
                }
            }
            if ui.button("Save").clicked() {
                workbench.show_save_confirm = true;
                // Generate diff
                let new_script = workbench.node_graph.to_script();
                workbench.diff_dialog =
                    Some(DiffDialog::new(&workbench.node_graph, Some(&new_script)));
                ui.close_menu();
            }
        });
        ui.menu_button("View", |ui| {
            ui.checkbox(&mut workbench.show_graph, "Graph Panel");
            ui.checkbox(&mut workbench.show_inspector, "Inspector");
            ui.checkbox(&mut workbench.show_timeline, "Timeline");
            ui.separator();
            ui.checkbox(&mut workbench.show_node_editor, "Floating Node Editor");
        });
    });
}
