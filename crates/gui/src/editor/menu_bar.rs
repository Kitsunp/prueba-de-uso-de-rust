use crate::editor::EditorWorkbench;
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
                workbench.prepare_save_confirmation();
                ui.close_menu();
            }
            if ui.button("Export Game (.vnproject)").clicked() {
                workbench.export_compiled_project();
                ui.close_menu();
            }
        });
        ui.menu_button("Tools", |ui| {
            if ui.button("Validate / Dry Run").clicked() {
                workbench.run_dry_validation();
                ui.close_menu();
            }
            if ui.button("Compile Preview").clicked() {
                workbench.compile_preview();
                ui.close_menu();
            }
        });
        ui.menu_button("View", |ui| {
            ui.checkbox(&mut workbench.show_graph, "Graph Panel");
            ui.checkbox(&mut workbench.show_inspector, "Inspector");
            ui.checkbox(&mut workbench.show_timeline, "Timeline");
            ui.separator();
            ui.checkbox(
                &mut workbench.node_editor_window_open,
                "Floating Node Editor",
            );
        });
    });
}
