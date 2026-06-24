//! Native desktop host for the Foundry workflow state.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, TextureOptions};
use shape_foundry::{
    CatalogContentRef, FoundryBuildStamp, FoundryCatalogError, FoundryCatalogResolver,
    FoundryCommand,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels, headless_fixture_catalogs,
};
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
    texture_cache: FoundryTextureCache,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FoundryTab {
    Home,
    Directions,
    Customize,
    Pack,
    History,
    Export,
}

impl Default for FoundryDesktopApp {
    fn default() -> Self {
        Self {
            state: FoundryAppState::default(),
            tab: FoundryTab::Home,
            jobs: FoundryJobCoordinator::default(),
            texture_cache: FoundryTextureCache::default(),
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
                ui.selectable_value(&mut self.tab, FoundryTab::Home, "Choose");
                ui.selectable_value(&mut self.tab, FoundryTab::Directions, "Directions");
                ui.selectable_value(&mut self.tab, FoundryTab::Customize, "Customize");
                ui.selectable_value(&mut self.tab, FoundryTab::Pack, "Pack");
                ui.selectable_value(&mut self.tab, FoundryTab::Export, "Export");
                ui.separator();
                ui.selectable_value(&mut self.tab, FoundryTab::History, "History");
            });
        egui::CentralPanel::default().show_inside(ui, |ui| match self.tab {
            FoundryTab::Home => self.show_home(ui),
            FoundryTab::Directions => commands.extend(self.show_directions(ui)),
            FoundryTab::Customize => commands.extend(self.show_customize(ui)),
            FoundryTab::Pack => commands.extend(self.show_pack(ui)),
            FoundryTab::History => commands.extend(self.show_history(ui)),
            FoundryTab::Export => commands.extend(self.show_export(ui)),
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
            ui.heading("Shape Lab");
            if ui.button("Choose").clicked() {
                self.tab = FoundryTab::Home;
            }
            if ui.button("Open project").clicked()
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
                .add_enabled(has_document, egui::Button::new("Build model"))
                .clicked()
            {
                commands.push(FoundryAppCommand::RequestBuild);
            }
            if ui
                .add_enabled(has_output, egui::Button::new("Preview model"))
                .clicked()
            {
                commands.push(FoundryAppCommand::RequestPreview);
            }
            if ui
                .add_enabled(has_output, egui::Button::new("Export"))
                .clicked()
            {
                self.tab = FoundryTab::Export;
            }
            if ui
                .add_enabled(
                    self.state
                        .project_file
                        .as_ref()
                        .is_some_and(|project| project.project.can_undo()),
                    egui::Button::new("Undo"),
                )
                .clicked()
            {
                commands.push(history::undo_command());
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

    fn show_home(&mut self, ui: &mut egui::Ui) {
        ui.heading("Choose what to make");
        ui.label(
            "Start with a semantic asset family, then generate directions and customize the result.",
        );
        ui.add_space(12.0);
        if ui.button("Open project").clicked()
            && let Some(path) = open_foundry_project_file()
        {
            self.apply_commands(vec![FoundryAppCommand::Load(path)], ui.ctx());
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (label, fixture) in built_in_fixture_catalogs_with_labels() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.strong(label);
                            ui.weak(profile_description(&fixture.slug));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Start").clicked() {
                                self.load_fixture(fixture, ui.ctx());
                            }
                        });
                    });
                });
            }
        });
    }

    fn show_directions(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
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
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        for candidate in &self.state.candidates {
            ui.horizontal(|ui| {
                let preview_id = candidate_preview_texture_id(candidate);
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build,
                        rgba8: &candidate.rgba8,
                        width: candidate.width,
                        height: candidate.height,
                        max_edge: 96.0,
                    },
                );
                ui.vertical(|ui| {
                    ui.label(&candidate.title);
                    ui.weak(&candidate.subtitle);
                    if candidate.width > 0 && candidate.height > 0 {
                        ui.weak(format!("{}x{}", candidate.width, candidate.height));
                    }
                    if let Some(reason) = &candidate.preview_failure {
                        ui.weak(reason);
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

    fn show_customize(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Customize");
        self.show_current_preview(ui);
        if self.state.controls.is_empty() {
            ui.weak("No compiled customizer controls yet.");
            return commands;
        }
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        for control in default_customize_controls(&self.state.controls) {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&control.label);
                    ui.weak(&control.kind);
                    ui.weak(format!("{} option(s)", control.options.len()));
                    if let Some(reason) = &control.locked_reason {
                        ui.weak(reason);
                    }
                    if ui.button("Select").clicked() {
                        commands.push(customize::select_control_command(Some(control.id.clone())));
                    }
                    let lock_label = if control.locked { "Unlock" } else { "Lock" };
                    if ui.button(lock_label).clicked()
                        && let Some(command) =
                            customize::control_lock_command(control, !control.locked)
                    {
                        commands.push(command);
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
                        let preview_id = option_preview_texture_id(option);
                        show_rgba_preview(
                            ui,
                            texture_cache,
                            FoundryPreviewDraw {
                                preview_id: &preview_id,
                                build: current_build,
                                rgba8: &option.rgba8,
                                width: option.width,
                                height: option.height,
                                max_edge: 64.0,
                            },
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
                        let disabled_reason =
                            customize::option_action_disabled_reason(control, option);
                        let preview_response =
                            ui.add_enabled(disabled_reason.is_none(), egui::Button::new("Preview"));
                        let preview_clicked = if let Some(reason) = &disabled_reason {
                            preview_response.on_disabled_hover_text(reason).clicked()
                        } else {
                            preview_response.clicked()
                        };
                        if preview_clicked {
                            commands.extend(customize::preview_control_value_intents(
                                control,
                                option.value.clone(),
                            ));
                        }
                        let apply_response =
                            ui.add_enabled(disabled_reason.is_none(), egui::Button::new("Apply"));
                        let apply_clicked = if let Some(reason) = &disabled_reason {
                            apply_response.on_disabled_hover_text(reason).clicked()
                        } else {
                            apply_response.clicked()
                        };
                        if apply_clicked {
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

    fn show_export(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Export");
        self.show_current_preview(ui);
        let can_export = self.state.current_output.is_some();
        if ui
            .add_enabled(can_export, egui::Button::new("Export Current Asset"))
            .clicked()
            && let Some(out_dir) = select_asset_export_dir()
        {
            commands.push(FoundryAppCommand::run(FoundryCommand::Export {
                profile: "default".to_owned(),
                out_dir: Some(out_dir.to_string_lossy().to_string()),
            }));
        }
        if self.state.pack.can_export {
            ui.weak("Pack export is available from the Pack tab.");
        }
        if !can_export {
            ui.weak("Build the current asset before exporting.");
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

    fn show_current_preview(&mut self, ui: &mut egui::Ui) {
        let preview = self.state.current_preview.clone();
        let has_output = self.state.current_output.is_some();
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        ui.horizontal(|ui| {
            if let Some(preview) = &preview {
                let preview_id = format!("current-{}", preview.preview_id);
                show_rgba_preview(
                    ui,
                    &mut self.texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: preview.build.as_ref(),
                        rgba8: &preview.rgba8,
                        width: preview.width,
                        height: preview.height,
                        max_edge: 180.0,
                    },
                );
                ui.weak(format!("Preview {}x{}", preview.width, preview.height));
            } else if has_output {
                if rendering_preview {
                    ui.weak("Rendering preview...");
                } else {
                    ui.weak("Preview is available from the toolbar.");
                }
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
        let member_id = unique_pack_member_id(&self.state.pack, &document.document_id.0);
        Some(pack::add_current_asset_to_pack_command(pack_id, member_id))
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
                    self.tab = FoundryTab::Directions;
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
                    self.tab = FoundryTab::Directions;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }
}

impl eframe::App for FoundryDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        FoundryDesktopApp::ui(self, ui, frame);
    }
}

#[derive(Default)]
struct FoundryJobCoordinator {
    tx: Option<Sender<FoundryJobEvent>>,
    rx: Option<Receiver<FoundryJobEvent>>,
    preview_cache: Arc<Mutex<FoundryPreviewCache>>,
}

impl FoundryJobCoordinator {
    fn submit(&mut self, request: FoundryJobRequest) {
        let job_id = request.job_id();
        let tx = self.tx().clone();
        let preview_cache = Arc::clone(&self.preview_cache);
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let event = if request_uses_preview_cache(&request) {
                match preview_cache.lock() {
                    Ok(mut preview_cache) => {
                        run_foundry_job(request, &resolver, &mut preview_cache)
                    }
                    Err(_) => FoundryJobEvent::Failed {
                        job_id,
                        message: "Foundry preview cache lock was poisoned.".to_owned(),
                    },
                }
            } else {
                let mut preview_cache = FoundryPreviewCache::default();
                run_foundry_job(request, &resolver, &mut preview_cache)
            };
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
        self.preview_cache = Arc::new(Mutex::new(FoundryPreviewCache::default()));
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

fn request_uses_preview_cache(request: &FoundryJobRequest) -> bool {
    matches!(
        request,
        FoundryJobRequest::RenderPreview { .. }
            | FoundryJobRequest::PreviewControlValue { .. }
            | FoundryJobRequest::GenerateCandidates { .. }
    )
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

fn select_asset_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Current Foundry Asset")
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

fn profile_description(slug: &str) -> &'static str {
    match slug {
        "roman-bridge" => {
            "A reinforced timber bridge with supports, bracing, railings, and span controls."
        }
        "sci-fi-crate" => {
            "A compact hard-surface prop with vents, handles, trims, and panel detail."
        }
        "stylized-lamp" => {
            "A furniture-scale lamp with height, stem, base, shade, and softness controls."
        }
        "market-stall" => {
            "A modular market stall with canopy, counter, display, and signage choices."
        }
        "sci-fi-door" => {
            "A panelized industrial door with frame, vents, locks, and surface detail."
        }
        "storage-barrel" => {
            "A coopered storage barrel with bands, proportions, lid, and wearable silhouette."
        }
        "signpost" => "A wayfinding post with arrows, boards, base, and directional variation.",
        "workshop-chair" => {
            "A practical workshop chair with seat, legs, back, braces, and style controls."
        }
        "handcart" => {
            "A market handcart with bed, wheels, handles, rails, and cargo-ready proportions."
        }
        "stylized-tree" => {
            "A storybook tree with trunk, canopy, branch, root, and stylized silhouette controls."
        }
        _ => "A built-in semantic asset family ready for visual direction generation.",
    }
}

fn product_visible_strings_for_default_shell() -> Vec<&'static str> {
    let mut strings = vec![
        "Shape Lab",
        "Choose",
        "Directions",
        "Customize",
        "Pack",
        "Export",
        "Choose what to make",
        "Open project",
        "Start",
        "Build model",
        "Preview model",
        "Save",
        "Save As",
        "Undo",
    ];
    strings.extend(
        built_in_fixture_catalogs_with_labels()
            .into_iter()
            .map(|(label, _)| label),
    );
    strings
}

fn default_customize_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> impl Iterator<Item = &crate::foundry::view_model::FoundryControlView> {
    controls
        .iter()
        .filter(|control| control.primary && control.visible)
}

fn candidate_preview_texture_id(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> String {
    candidate
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("candidate-{}", candidate.id.0))
}

fn option_preview_texture_id(option: &crate::foundry::view_model::FoundryOptionCard) -> String {
    option
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("option-{}-{}", option.control_id, option.label))
}

fn unique_pack_member_id(pack: &crate::foundry::view_model::FoundryPackView, base: &str) -> String {
    let base = if base.trim().is_empty() {
        "member"
    } else {
        base.trim()
    };
    if !pack.members.contains_key(base) {
        return base.to_owned();
    }

    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !pack.members.contains_key(&candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded member suffix search should always find an unused id")
}

#[derive(Default)]
struct FoundryTextureCache {
    textures: BTreeMap<String, CachedFoundryTexture>,
}

struct CachedFoundryTexture {
    identity: FoundryTextureIdentity,
    texture: egui::TextureHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FoundryTextureIdentity {
    preview_id: String,
    build_fingerprint: Option<String>,
    width: u32,
    height: u32,
    rgba_fingerprint: u64,
}

impl FoundryTextureIdentity {
    fn new(
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Self {
        Self {
            preview_id: preview_id.to_owned(),
            build_fingerprint: build.map(|build| build.build_fingerprint.0.to_hex()),
            width,
            height,
            rgba_fingerprint: preview_image_fingerprint(rgba8),
        }
    }

    fn texture_name(&self) -> String {
        format!(
            "foundry-preview-{}-{}x{}-{}-{:016x}",
            self.preview_id,
            self.width,
            self.height,
            self.build_fingerprint.as_deref().unwrap_or("no-build"),
            self.rgba_fingerprint
        )
    }
}

impl FoundryTextureCache {
    fn texture(
        &mut self,
        ctx: &egui::Context,
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        rgba8: &[u8],
        width: u32,
        height: u32,
    ) -> egui::TextureHandle {
        let identity = FoundryTextureIdentity::new(preview_id, build, width, height, rgba8);
        if let Some(cached) = self
            .textures
            .get(preview_id)
            .filter(|cached| cached.identity == identity)
        {
            return cached.texture.clone();
        }

        let color_image =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba8);
        let texture =
            ctx.load_texture(identity.texture_name(), color_image, TextureOptions::LINEAR);
        self.textures.insert(
            preview_id.to_owned(),
            CachedFoundryTexture {
                identity,
                texture: texture.clone(),
            },
        );
        texture
    }
}

fn preview_image_fingerprint(rgba8: &[u8]) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    rgba8.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

struct FoundryPreviewDraw<'a> {
    preview_id: &'a str,
    build: Option<&'a FoundryBuildStamp>,
    rgba8: &'a [u8],
    width: u32,
    height: u32,
    max_edge: f32,
}

fn show_rgba_preview(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    preview: FoundryPreviewDraw<'_>,
) {
    let width_usize = preview.width as usize;
    let height_usize = preview.height as usize;
    let expected_len = width_usize.saturating_mul(height_usize).saturating_mul(4);
    if preview.width == 0 || preview.height == 0 || preview.rgba8.len() != expected_len {
        ui.allocate_space(egui::vec2(preview.max_edge, preview.max_edge));
        return;
    }

    let texture = texture_cache.texture(
        ui.ctx(),
        preview.preview_id,
        preview.build,
        preview.rgba8,
        preview.width,
        preview.height,
    );
    let scale =
        (preview.max_edge / preview.width as f32).min(preview.max_edge / preview.height as f32);
    let size = egui::vec2(preview.width as f32 * scale, preview.height as f32 * scale);
    ui.image((texture.id(), size));
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_foundry::compile_foundry_document;

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

    #[test]
    fn pack_member_ids_increment_for_repeated_ui_adds() {
        let mut pack = crate::foundry::view_model::FoundryPackView::default();
        assert_eq!(
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc"
        );

        pack.members.insert(
            "roman-bridge-doc".to_owned(),
            shape_foundry::FoundryDocumentId("roman-bridge-doc".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc-2"
        );

        pack.members.insert(
            "roman-bridge-doc-2".to_owned(),
            shape_foundry::FoundryDocumentId("roman-bridge-doc-2".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc-3"
        );
    }

    #[test]
    fn product_app_launches_on_choose_home() {
        let app = FoundryDesktopApp::default();

        assert_eq!(app.tab, FoundryTab::Home);
        assert!(app.state.document.is_none());
    }

    #[test]
    fn product_home_lists_ten_profiles() {
        let labels = built_in_fixture_catalogs_with_labels()
            .into_iter()
            .map(|(label, _)| label)
            .collect::<Vec<_>>();

        assert_eq!(labels.len(), 10);
        assert!(labels.contains(&"Roman Timber Bridge"));
        assert!(labels.contains(&"Sci-Fi Industrial Crate"));
        assert!(labels.contains(&"Stylized Furniture Lamp"));
        assert!(labels.contains(&"Market Stall Kit"));
        assert!(labels.contains(&"Sci-Fi Door Panel"));
        assert!(labels.contains(&"Coopered Storage Barrel"));
        assert!(labels.contains(&"Wayfinding Signpost"));
        assert!(labels.contains(&"Workshop Chair"));
        assert!(labels.contains(&"Market Handcart"));
        assert!(labels.contains(&"Storybook Tree"));
    }

    #[test]
    fn default_product_strings_hide_legacy_and_technical_surfaces() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n");

        for forbidden in [
            "Legacy",
            "Implicit",
            "Asset Modeling Lab",
            "Modeling Workspace",
            "Advanced Recipe",
            "From Existing Recipe",
        ] {
            assert!(
                !joined.contains(forbidden),
                "default product strings unexpectedly contain {forbidden}: {joined}"
            );
        }
    }

    #[test]
    fn product_shell_steps_are_novice_facing() {
        let strings = product_visible_strings_for_default_shell();

        for required in [
            "Choose what to make",
            "Choose",
            "Directions",
            "Customize",
            "Pack",
            "Export",
            "Open project",
            "Start",
        ] {
            assert!(
                strings.contains(&required),
                "missing product string {required}"
            );
        }
    }

    #[test]
    fn preview_texture_identity_tracks_preview_id_build_and_pixels() {
        let bridge = shape_foundry_catalog::roman_bridge::fixture_catalog();
        let crate_fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
        let build_a = compile_foundry_document(&bridge.document, &bridge)
            .expect("bridge fixture compiles")
            .build_stamp;
        let build_b = compile_foundry_document(&crate_fixture.document, &crate_fixture)
            .expect("crate fixture compiles")
            .build_stamp;

        let identity = FoundryTextureIdentity::new(
            "option-a",
            Some(&build_a),
            2,
            1,
            &[0, 0, 0, 255, 255, 255, 255, 255],
        );

        assert_eq!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-b",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_b),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 254, 255],
            )
        );
    }

    #[test]
    fn desktop_foundry_exposes_product_steps_and_lamp_profile() {
        let tabs = [
            FoundryTab::Home,
            FoundryTab::Directions,
            FoundryTab::Customize,
            FoundryTab::Pack,
            FoundryTab::Export,
            FoundryTab::History,
        ];
        assert_eq!(tabs.len(), 6);

        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::stylized_lamp::fixture_catalog(),
            &ctx,
        );

        for _ in 0..200 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(app.tab, FoundryTab::Directions);
        assert_eq!(
            app.state
                .document
                .as_ref()
                .map(|document| document.family_content_ref.stable_id.as_str()),
            Some("stylized-lamp-family")
        );
        assert!(app.state.current_output.is_some());
    }

    #[test]
    fn loading_project_enters_workflow_step() {
        let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
        let state = FoundryAppState::new(fixture.document).expect("fixture state");
        let mut project_file = state.project_file.expect("project file");
        let path = temp_foundry_project_path("load-enters-workflow");
        project_file.save_as(&path).expect("project saves");
        let mut app = FoundryDesktopApp::default();

        app.load_project(path.clone(), &egui::Context::default());

        assert_eq!(app.tab, FoundryTab::Directions);
        assert!(app.state.document.is_some());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn default_customize_surface_hides_non_primary_and_hidden_controls() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(shape_foundry_catalog::scifi_crate::fixture_catalog(), &ctx);

        for _ in 0..200 {
            app.poll_jobs(&ctx);
            if !app.state.controls.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let default_ids = default_customize_controls(&app.state.controls)
            .map(|control| control.id.as_str())
            .collect::<Vec<_>>();
        assert!(default_ids.contains(&"body_proportions"));
        assert!(!default_ids.contains(&"has_trim"));
        assert!(!default_ids.contains(&"runtime_wear"));
        assert!(!default_ids.contains(&"advisory_weathering"));
        assert!(
            default_customize_controls(&app.state.controls)
                .all(|control| control.primary && control.visible)
        );
    }

    fn temp_foundry_project_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "shape-lab-{name}-{}-{nanos}{FOUNDRY_PROJECT_FILE_SUFFIX}",
            std::process::id()
        ))
    }
}
