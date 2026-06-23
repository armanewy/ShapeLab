//! Native desktop host for the Foundry workflow state.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, TextureOptions};
use shape_foundry::{CatalogContentRef, FoundryCatalogError, FoundryCatalogResolver};
use shape_foundry_catalog::{FoundryFixtureCatalog, headless_fixture_catalogs};
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryProject, FoundryProjectFile, ensure_foundry_project_path,
};
use shape_render::foundry::FoundryPreviewCache;

use crate::foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
    panels::{customize, directions, history, pack},
    run_foundry_job,
};

/// Native Foundry workflow surface.
pub(crate) struct FoundryDesktopApp {
    state: FoundryAppState,
    tab: FoundryTab,
    jobs: FoundryJobCoordinator,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FoundryTab {
    Directions,
    Customize,
    History,
    Pack,
}

impl Default for FoundryDesktopApp {
    fn default() -> Self {
        Self {
            state: FoundryAppState::default(),
            tab: FoundryTab::Directions,
            jobs: FoundryJobCoordinator::default(),
        }
    }
}

impl FoundryDesktopApp {
    /// Draw the Foundry workflow surface.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs(&ctx);

        let mut commands = Vec::new();
        egui::Panel::top("foundry_toolbar").show_inside(ui, |ui| {
            commands.extend(self.show_toolbar(ui));
        });
        egui::Panel::bottom("foundry_status")
            .default_size(28.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Foundry");
                    if self.state.read_only {
                        ui.weak("Read-only recovery");
                    }
                    if self.state.dirty {
                        ui.weak("Unsaved");
                    }
                    if let Some(status) = &self.state.status {
                        ui.weak(status);
                    }
                });
            });
        egui::Panel::left("foundry_sections")
            .resizable(true)
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Foundry");
                ui.selectable_value(&mut self.tab, FoundryTab::Directions, "Directions");
                ui.selectable_value(&mut self.tab, FoundryTab::Customize, "Customize");
                ui.selectable_value(&mut self.tab, FoundryTab::History, "History");
                ui.selectable_value(&mut self.tab, FoundryTab::Pack, "Pack");
            });
        egui::CentralPanel::default().show_inside(ui, |ui| match self.tab {
            FoundryTab::Directions => commands.extend(self.show_directions(ui)),
            FoundryTab::Customize => commands.extend(self.show_customize(ui)),
            FoundryTab::History => commands.extend(self.show_history(ui)),
            FoundryTab::Pack => commands.extend(self.show_pack(ui)),
        });

        self.apply_commands(commands, &ctx);
        if !self.state.active_jobs.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        let _ = frame;
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            let has_document = self.state.document.is_some();
            let has_output = self.state.current_output.is_some();
            if ui.button("New Bridge").clicked() {
                self.load_fixture(
                    shape_foundry_catalog::roman_bridge::fixture_catalog(),
                    ui.ctx(),
                );
            }
            if ui.button("New Crate").clicked() {
                self.load_fixture(
                    shape_foundry_catalog::scifi_crate::fixture_catalog(),
                    ui.ctx(),
                );
            }
            if ui.button("Open").clicked()
                && let Some(path) = open_foundry_project_file()
            {
                commands.push(FoundryAppCommand::Load(path));
            }
            if ui
                .add_enabled(
                    has_document && self.state.project_path.is_some(),
                    egui::Button::new("Save"),
                )
                .clicked()
            {
                commands.push(FoundryAppCommand::Save);
            }
            if ui
                .add_enabled(has_document, egui::Button::new("Save As"))
                .clicked()
                && let Some(path) = save_foundry_project_file()
            {
                commands.push(FoundryAppCommand::SaveAs(path));
            }
            if ui
                .add_enabled(has_document, egui::Button::new("Build"))
                .clicked()
            {
                commands.push(FoundryAppCommand::RequestBuild);
            }
            if ui
                .add_enabled(has_output, egui::Button::new("Preview"))
                .clicked()
            {
                commands.push(FoundryAppCommand::RequestPreview);
            }
            for action in directions::direction_mode_actions(None, 0, None) {
                if ui
                    .add_enabled(has_document, egui::Button::new(action.label))
                    .clicked()
                {
                    commands.push(action.app_command());
                }
            }
        });
        commands
    }

    fn show_directions(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Directions");
        if self.state.document.is_none() {
            ui.weak("Open a Foundry project to generate whole-model directions.");
            return commands;
        }
        self.show_current_preview(ui);
        ui.label(format!(
            "{} candidate direction(s)",
            self.state.candidates.len()
        ));
        for candidate in &self.state.candidates {
            ui.horizontal(|ui| {
                show_rgba_preview(
                    ui,
                    &format!("foundry-candidate-{}", candidate.id.0),
                    &candidate.rgba8,
                    candidate.width,
                    candidate.height,
                    96.0,
                );
                ui.vertical(|ui| {
                    ui.label(&candidate.title);
                    ui.weak(&candidate.subtitle);
                    if candidate.width > 0 && candidate.height > 0 {
                        ui.weak(format!("{}x{}", candidate.width, candidate.height));
                    }
                    if candidate.selected {
                        ui.weak("selected");
                    }
                });
                if ui.button("Select").clicked() {
                    commands.push(FoundryAppCommand::SelectCandidate(Some(
                        candidate.id.clone(),
                    )));
                }
                if ui
                    .add_enabled(candidate.selectable, egui::Button::new("Choose"))
                    .clicked()
                {
                    commands.push(directions::accept_candidate_command(candidate.id.clone()));
                }
                if ui.button("Reject").clicked() {
                    commands.push(directions::reject_candidate_command(candidate.id.clone()));
                }
            });
        }
        commands
    }

    fn show_customize(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Customize");
        self.show_current_preview(ui);
        if self.state.controls.is_empty() {
            ui.weak("No compiled customizer controls yet.");
            return commands;
        }
        for control in &self.state.controls {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&control.label);
                    ui.weak(&control.kind);
                    ui.weak(format!("{} option(s)", control.options.len()));
                    if ui.button("Select").clicked() {
                        commands.push(customize::select_control_command(Some(control.id.clone())));
                    }
                    if ui
                        .add_enabled(
                            customize::control_can_reset(control),
                            egui::Button::new("Reset"),
                        )
                        .clicked()
                    {
                        commands.extend(customize::reset_control_intents(control));
                    }
                });
                for option in &control.options {
                    ui.horizontal(|ui| {
                        show_rgba_preview(
                            ui,
                            &format!("foundry-option-{}-{}", option.control_id, option.label),
                            &option.rgba8,
                            option.width,
                            option.height,
                            64.0,
                        );
                        ui.vertical(|ui| {
                            ui.weak(&option.label);
                            if option.selected {
                                ui.weak("current");
                            }
                            if let Some(reason) = &option.unavailable_reason {
                                ui.weak(reason);
                            }
                        });
                        if ui
                            .add_enabled(
                                option.unavailable_reason.is_none() && !control.locked,
                                egui::Button::new("Preview"),
                            )
                            .clicked()
                        {
                            commands.extend(customize::preview_control_value_intents(
                                control,
                                option.value.clone(),
                            ));
                        }
                        if ui
                            .add_enabled(
                                option.unavailable_reason.is_none() && !control.locked,
                                egui::Button::new("Apply"),
                            )
                            .clicked()
                        {
                            commands.extend(customize::choose_option_intents(control, option));
                        }
                    });
                }
            });
        }
        commands
    }

    fn show_history(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = history::build_history_view(&self.state);
        ui.heading("History");
        ui.horizontal(|ui| {
            for action in &view.actions {
                if ui
                    .add_enabled(action.enabled, egui::Button::new(&action.label))
                    .clicked()
                    && let Some(command) = self.history_dispatch_command(action.dispatch.as_ref())
                {
                    commands.push(command);
                }
            }
        });
        ui.label(format!("{} revision(s)", view.rows.len()));
        for row in view.rows {
            ui.horizontal(|ui| {
                ui.label(row.label);
                if row.selected {
                    ui.weak("current");
                }
                if let Some(intent) = &row.switch_intent
                    && ui.button("Switch").clicked()
                    && let Some(command) = self.history_dispatch_command(intent.dispatch.as_ref())
                {
                    commands.push(command);
                }
                if let Some(intent) = &row.branch_intent
                    && ui.button("Branch").clicked()
                    && let Some(command) = self.history_dispatch_command(intent.dispatch.as_ref())
                {
                    commands.push(command);
                }
            });
        }
        commands
    }

    fn show_pack(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = pack::pack_panel_view(&self.state.pack);
        ui.heading("Pack");
        ui.horizontal(|ui| {
            let add_enabled = self.state.document.is_some();
            if ui
                .add_enabled(add_enabled, egui::Button::new("Add Current Asset"))
                .clicked()
                && let Some(command) = self.add_current_to_pack_command()
            {
                commands.push(command);
            }
            if ui
                .add_enabled(view.export.enabled, egui::Button::new("Batch Export"))
                .clicked()
                && let Some(out_dir) = select_pack_export_dir()
                && let Some(command) = pack::batch_export_command(&self.state.pack, out_dir)
            {
                commands.push(command);
            }
        });
        if !view.active {
            ui.weak("No family pack workspace is open.");
            return commands;
        }
        ui.label(format!("{} member(s)", view.members.len()));
        if view.export.enabled {
            ui.weak("Batch export ready");
        } else if let Some(reason) = view.export.disabled_reason {
            ui.weak(reason);
        }
        commands
    }

    fn show_current_preview(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(preview) = &self.state.current_preview {
                show_rgba_preview(
                    ui,
                    &format!("foundry-current-{}", preview.preview_id),
                    &preview.rgba8,
                    preview.width,
                    preview.height,
                    180.0,
                );
                ui.weak(format!("Preview {}x{}", preview.width, preview.height));
            } else if self.state.current_output.is_some() {
                ui.weak("Preview queued or available from the toolbar.");
            } else {
                ui.weak("Build the current asset to render a preview.");
            }
        });
    }

    fn history_dispatch_command(
        &self,
        dispatch: Option<&history::FoundryHistoryActionDispatch>,
    ) -> Option<FoundryAppCommand> {
        match dispatch? {
            history::FoundryHistoryActionDispatch::Command(command) => Some(command.clone()),
            history::FoundryHistoryActionDispatch::RequestSaveAsPath => {
                save_foundry_project_file().map(FoundryAppCommand::SaveAs)
            }
            history::FoundryHistoryActionDispatch::RequestLoadPath => {
                open_foundry_project_file().map(FoundryAppCommand::Load)
            }
        }
    }

    fn add_current_to_pack_command(&self) -> Option<FoundryAppCommand> {
        let document = self.state.document.as_ref()?;
        let pack_id = self
            .state
            .pack
            .pack_id
            .clone()
            .unwrap_or_else(|| "foundry_pack".to_owned());
        Some(pack::add_current_asset_to_pack_command(
            pack_id,
            document.document_id.0.clone(),
        ))
    }

    fn apply_commands(&mut self, commands: Vec<FoundryAppCommand>, ctx: &egui::Context) {
        for command in commands {
            match self.state.handle_command(command) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
    }

    fn apply_effects(&mut self, effects: Vec<FoundryAppEffect>, ctx: &egui::Context) {
        for effect in effects {
            match effect {
                FoundryAppEffect::StartJob(request) => self.submit_job(*request),
                FoundryAppEffect::SaveProject { path, project } => {
                    self.save_project(path, *project);
                }
                FoundryAppEffect::LoadProject(path) => self.load_project(path, ctx),
            }
        }
    }

    fn submit_job(&mut self, request: FoundryJobRequest) {
        self.jobs.submit(request);
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let mut affected = false;
        let mut schedule_preview = false;

        loop {
            let event = match self.jobs.try_recv() {
                Ok(event) => event,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.state.status = Some("Foundry background worker disconnected.".to_owned());
                    break;
                }
            };
            let should_preview = matches!(
                event,
                FoundryJobEvent::CompileFinished { .. } | FoundryJobEvent::EditApplied { .. }
            );
            if self.state.handle_job_event(event) {
                affected = true;
                schedule_preview |= should_preview;
            }
        }

        if schedule_preview {
            match self.state.request_preview(128, 128) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if affected {
            ctx.request_repaint();
        }
    }

    fn save_project(&mut self, path: PathBuf, project: FoundryProject) {
        if let Err(error) = ensure_foundry_project_path(&path) {
            self.state.status = Some(error.to_string());
            return;
        }
        match project.save_json(&path) {
            Ok(()) => {
                self.state.mark_saved(path.clone());
                self.state.status = Some(format!("Saved {}", path.display()));
            }
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn load_project(&mut self, path: PathBuf, ctx: &egui::Context) {
        match FoundryProjectFile::load(&path) {
            Ok(project_file) => match self.state.replace_loaded_project(project_file) {
                Ok(effects) => {
                    self.jobs.reset();
                    self.state.status = Some(format!("Loaded {}", path.display()));
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn load_fixture(&mut self, fixture: FoundryFixtureCatalog, ctx: &egui::Context) {
        self.jobs.reset();
        match FoundryAppState::new(fixture.document) {
            Ok(mut state) => match state.request_build() {
                Ok(effects) => {
                    state.status = Some(format!("Loaded {} fixture.", fixture.slug));
                    self.state = state;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }
}

#[derive(Default)]
struct FoundryJobCoordinator {
    tx: Option<Sender<FoundryJobEvent>>,
    rx: Option<Receiver<FoundryJobEvent>>,
}

impl FoundryJobCoordinator {
    fn submit(&mut self, request: FoundryJobRequest) {
        let tx = self.tx().clone();
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let mut preview_cache = FoundryPreviewCache::default();
            let event = run_foundry_job(request, &resolver, &mut preview_cache);
            let _ = tx.send(event);
        });
    }

    fn try_recv(&mut self) -> Result<FoundryJobEvent, crossbeam_channel::TryRecvError> {
        self.rx().try_recv()
    }

    fn reset(&mut self) {
        let (tx, rx) = unbounded();
        self.tx = Some(tx);
        self.rx = Some(rx);
    }

    fn tx(&mut self) -> &Sender<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.tx.as_ref().expect("tx initialized")
    }

    fn rx(&mut self) -> &Receiver<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.rx.as_ref().expect("rx initialized")
    }
}

struct BuiltInFoundryCatalogResolver {
    fixtures: Vec<FoundryFixtureCatalog>,
}

impl Default for BuiltInFoundryCatalogResolver {
    fn default() -> Self {
        Self {
            fixtures: headless_fixture_catalogs(),
        }
    }
}

impl FoundryCatalogResolver for BuiltInFoundryCatalogResolver {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.fixtures
            .iter()
            .find_map(|fixture| {
                fixture
                    .entries
                    .get(&content_ref.stable_id)
                    .map(|entry| entry.canonical_json.clone())
            })
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

fn open_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab Foundry", &["json"])
        .pick_file()
}

fn save_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab Foundry", &["json"])
        .set_file_name("foundry-project.shapelab-foundry.json")
        .save_file()
        .map(normalize_foundry_project_path)
}

fn select_pack_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Foundry Pack")
        .pick_folder()
}

fn normalize_foundry_project_path(path: PathBuf) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path;
    };
    if file_name.ends_with(FOUNDRY_PROJECT_FILE_SUFFIX) {
        return path;
    }

    let base_name = file_name.strip_suffix(".json").unwrap_or(file_name);
    path.with_file_name(format!("{base_name}{FOUNDRY_PROJECT_FILE_SUFFIX}"))
}

fn show_rgba_preview(
    ui: &mut egui::Ui,
    texture_name: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    max_edge: f32,
) {
    let width_usize = width as usize;
    let height_usize = height as usize;
    let expected_len = width_usize.saturating_mul(height_usize).saturating_mul(4);
    if width == 0 || height == 0 || rgba8.len() != expected_len {
        ui.allocate_space(egui::vec2(max_edge, max_edge));
        return;
    }

    let color_image = ColorImage::from_rgba_unmultiplied([width_usize, height_usize], rgba8);
    let texture =
        ui.ctx()
            .load_texture(texture_name.to_owned(), color_image, TextureOptions::LINEAR);
    let scale = (max_edge / width as f32).min(max_edge / height as f32);
    let size = egui::vec2(width as f32 * scale, height as f32 * scale);
    ui.image((texture.id(), size));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_foundry_effects_execute_background_jobs() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(shape_foundry_catalog::roman_bridge::fixture_catalog(), &ctx);

        for _ in 0..200 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(app.state.current_output.is_some());
        assert!(app.state.current_preview.is_some() || !app.state.active_jobs.is_empty());
    }

    #[test]
    fn save_as_paths_use_loadable_foundry_suffix() {
        let normalized = normalize_foundry_project_path(PathBuf::from("bridge.json"));
        assert_eq!(normalized, PathBuf::from("bridge.shapelab-foundry.json"));
        ensure_foundry_project_path(&normalized).expect("normalized path is loadable");
    }

    #[test]
    fn desktop_foundry_pack_action_dispatches_through_reducer() {
        let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
        let app = FoundryDesktopApp {
            state: FoundryAppState::new(fixture.document).expect("fixture state"),
            ..FoundryDesktopApp::default()
        };

        assert!(matches!(
            app.add_current_to_pack_command()
                .and_then(|command| command.single_foundry_command().cloned()),
            Some(shape_foundry::FoundryCommand::AddCurrentToPack { .. })
        ));
    }
}
