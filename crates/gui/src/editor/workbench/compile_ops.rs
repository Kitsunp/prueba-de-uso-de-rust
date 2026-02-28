use super::*;

impl EditorWorkbench {
    pub fn run_dry_validation(&mut self) -> bool {
        let result = crate::editor::compiler::compile_project(&self.node_graph);
        self.current_script = Some(result.script);
        self.last_dry_run_report = result.dry_run_report.clone();
        self.validation_issues = result.issues;
        Self::append_phase_trace_issues(&mut self.validation_issues, &result.phase_trace);
        self.show_validation = !self.validation_issues.is_empty();

        let has_errors = self
            .validation_issues
            .iter()
            .any(|issue| issue.severity == LintSeverity::Error);
        if has_errors {
            self.toast = Some(ToastState::error("Validation found blocking errors"));
            return false;
        }

        match result.engine_result {
            Ok(engine) => {
                self.engine = Some(engine);
                self.toast = Some(ToastState::success("Dry Run completed"));
                true
            }
            Err(e) => {
                self.validation_issues.push(LintIssue::error(
                    None,
                    ValidationPhase::Runtime,
                    LintCode::RuntimeInitError,
                    format!("Runtime initialization failed: {}", e),
                ));
                self.show_validation = true;
                self.toast = Some(ToastState::error("Validation failed at runtime init"));
                false
            }
        }
    }

    pub fn compile_preview(&mut self) -> bool {
        let ok = self.run_dry_validation();
        if ok {
            self.toast = Some(ToastState::success("Compilation preview successful"));
        }
        ok
    }

    pub fn export_compiled_project(&mut self) {
        if !self.run_dry_validation() {
            return;
        }

        let Some(script) = self.current_script.as_ref() else {
            self.toast = Some(ToastState::error("No script to export"));
            return;
        };

        let compiled = match script.compile() {
            Ok(compiled) => compiled,
            Err(e) => {
                self.toast = Some(ToastState::error(format!("Compile failed: {}", e)));
                return;
            }
        };

        let bytes = match compiled.to_binary() {
            Ok(bytes) => bytes,
            Err(e) => {
                self.toast = Some(ToastState::error(format!("Binary export failed: {}", e)));
                return;
            }
        };

        let path = rfd::FileDialog::new()
            .add_filter("VN Project", &["vnproject"])
            .set_file_name("game.vnproject")
            .save_file();

        if let Some(path) = path {
            match std::fs::write(&path, bytes) {
                Ok(_) => {
                    self.toast = Some(ToastState::success("Exported .vnproject successfully"));
                }
                Err(e) => {
                    self.toast = Some(ToastState::error(format!("Export failed: {}", e)));
                }
            }
        } else {
            self.toast = Some(ToastState::warning("Export cancelled"));
        }
    }

    pub fn export_dry_run_repro(&mut self) {
        let result = crate::editor::compiler::compile_project(&self.node_graph);
        let repro = result.minimal_repro_script();
        let script = result.script.clone();
        self.current_script = Some(script);
        self.last_dry_run_report = result.dry_run_report.clone();
        self.validation_issues = result.issues;
        Self::append_phase_trace_issues(&mut self.validation_issues, &result.phase_trace);
        self.show_validation = !self.validation_issues.is_empty();

        let Some(repro) = repro else {
            self.toast = Some(ToastState::warning(
                "No se pudo generar un repro fiel para el Dry Run actual",
            ));
            return;
        };

        let Ok(payload) = repro.to_json() else {
            self.toast = Some(ToastState::error("Failed to serialize dry-run repro"));
            return;
        };

        let path = rfd::FileDialog::new()
            .add_filter("Script JSON", &["json"])
            .set_file_name("dry_run_repro.json")
            .save_file();

        if let Some(path) = path {
            match std::fs::write(&path, payload) {
                Ok(_) => {
                    self.toast = Some(ToastState::success("Dry-run repro exported"));
                }
                Err(e) => {
                    self.toast = Some(ToastState::error(format!("Repro export failed: {}", e)));
                }
            }
        } else {
            self.toast = Some(ToastState::warning("Repro export cancelled"));
        }
    }

    pub fn sync_graph_to_script(&mut self) -> Result<(), String> {
        let result = crate::editor::compiler::compile_project(&self.node_graph);

        // Update State
        self.current_script = Some(result.script);
        self.last_dry_run_report = result.dry_run_report.clone();
        self.validation_issues = result.issues;
        Self::append_phase_trace_issues(&mut self.validation_issues, &result.phase_trace);
        self.show_validation = !self.validation_issues.is_empty();

        match result.engine_result {
            Ok(engine) => {
                self.engine = Some(engine);
                Ok(())
            }
            Err(e) => {
                self.validation_issues.push(LintIssue::error(
                    None,
                    ValidationPhase::Runtime,
                    LintCode::RuntimeInitError,
                    format!("Engine Error: {}", e),
                ));
                self.show_validation = true;
                Err(e)
            }
        }
    }
}
