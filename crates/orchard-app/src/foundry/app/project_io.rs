use super::*;

pub(super) fn project_file_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            stem.trim_end_matches(".object-orchard-foundry")
                .replace(['-', '_'], " ")
        })
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Object Orchard Project".to_owned())
}

pub(super) fn asset_title_from_id(document_id: &str) -> &'static str {
    match document_id {
        id if id.contains("box-primitive") => "Box Primitive",
        id if id.contains("lidded-box") => "Lidded Box",
        id if id.contains("panel-with-knob") => "Panel with Knob",
        id if id.contains("handled-panel") => "Handled Panel",
        id if id.contains("hinged-panel") => "Hinged Panel",
        id if id.contains("flat-panel-primitive") => "Flat Panel Primitive",
        id if id.contains("sphere-primitive") => "Sphere Primitive",
        _ => "Object Orchard Project",
    }
}

pub(super) fn product_safe_status(status: &str) -> String {
    if status.starts_with("Saved ") {
        "Project saved".to_owned()
    } else if status.starts_with("Loaded ") {
        "Project loaded".to_owned()
    } else if status.starts_with("Exported ") && status.contains(" pack member") {
        "Pack export complete".to_owned()
    } else if status.starts_with("Exported ") {
        "Export complete".to_owned()
    } else if status.contains('\\') || status.contains('/') {
        "Project path needs attention".to_owned()
    } else if crate::foundry::ui::copy::first_forbidden_product_term(status).is_some() {
        "Project needs attention".to_owned()
    } else {
        status.to_owned()
    }
}

pub(super) fn open_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Object Orchard Foundry", &["json"])
        .pick_file()
}

pub(super) fn save_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Object Orchard Foundry", &["json"])
        .set_file_name("foundry-project.object-orchard-foundry.json")
        .save_file()
        .map(normalize_foundry_project_path)
}

pub(super) fn select_pack_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Foundry Pack")
        .pick_folder()
}

pub(super) fn select_asset_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Current Foundry Asset")
        .pick_folder()
}

pub(super) fn normalize_foundry_project_path(path: PathBuf) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path;
    };
    if file_name.ends_with(FOUNDRY_PROJECT_FILE_SUFFIX) {
        return path;
    }

    let base_name = file_name.strip_suffix(".json").unwrap_or(file_name);
    path.with_file_name(format!("{base_name}{FOUNDRY_PROJECT_FILE_SUFFIX}"))
}

impl FoundryDesktopApp {
    pub(super) fn save_project(&mut self, path: PathBuf, project: FoundryProject) {
        if let Err(error) = ensure_foundry_project_path(&path) {
            self.state.status = Some(error.to_string());
            return;
        }
        match project.save_json(&path) {
            Ok(()) => {
                self.state.mark_saved(path.clone());
                self.remember_recent_project(path.clone());
                self.state.status = Some(format!("Saved {}", path.display()));
            }
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    pub(super) fn load_project(&mut self, path: PathBuf, ctx: &egui::Context) {
        match FoundryProjectFile::load(&path) {
            Ok(project_file) => match self.state.replace_loaded_project(project_file) {
                Ok(effects) => {
                    self.make_trace_started_at = Instant::now();
                    self.state.set_make_trace_elapsed_ms(0);
                    self.jobs.reset();
                    self.texture_cache.clear();
                    self.material_looks.clear_for_asset();
                    self.state.status = Some(format!("Loaded {}", path.display()));
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
                    self.remember_recent_project(path.clone());
                    self.make_preparation_started_at = Some(Instant::now());
                    self.make_generation_started_at = None;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    pub(super) fn load_fixture(&mut self, fixture: FoundryFixtureCatalog, ctx: &egui::Context) {
        self.make_trace_started_at = Instant::now();
        self.make_generation_started_at = None;
        self.jobs.reset();
        self.texture_cache.clear();
        self.material_looks.clear_for_asset();
        match FoundryAppState::new(fixture.document) {
            Ok(mut state) => match state.request_build() {
                Ok(effects) => {
                    state.status = Some(format!("Loaded {} fixture.", fixture.slug));
                    self.state = state;
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
                    self.make_preparation_started_at = Some(Instant::now());
                    self.make_generation_started_at = None;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    pub(super) fn remember_recent_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|recent| recent != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(5);
    }
}
