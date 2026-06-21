//! Native Asset Modeling Lab coordinator.

use std::collections::{BTreeMap, BTreeSet, VecDeque, btree_map::Entry};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, TextureHandle, TextureOptions};
use shape_asset::{AssetRecipe, PartInstanceId};
use shape_modeling_assets::{BenchmarkAsset, benchmark_assets};
use shape_render::{RenderSettings, RenderedImage};

use crate::asset::io::{ASSET_PROJECT_DIALOG_LABEL, suggested_asset_project_file_name};
use crate::asset::panels::{candidate_gallery, history, inspector, part_tree};
use crate::asset::view_model::build_asset_ui_state;
use crate::asset::viewport::{AssetViewportOverlay, NormalizedRect, show_asset_viewport};
use crate::asset::{
    AssetAppCommand, AssetAppEffect, AssetAppState, AssetCandidateId, AssetJobEvent,
    AssetJobRequest, AssetTemplate, run_asset_job,
};
use crate::viewport::{ViewportAction, ViewportInteractionState, ViewportRenderRequest};

const MAX_STATUS_MESSAGES: usize = 8;

/// Asset Modeling Lab desktop surface.
pub(crate) struct AssetModelingLabApp {
    state: Option<AssetAppState>,
    jobs: AssetJobCoordinator,
    current_texture: Option<TextureHandle>,
    candidate_textures: BTreeMap<AssetCandidateId, TextureHandle>,
    viewport_state: ViewportInteractionState,
    render_settings: RenderSettings,
    wireframe: bool,
    show_template_picker: bool,
    status: VecDeque<String>,
    left_tab: AssetLeftTab,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum AssetLeftTab {
    Parts,
    History,
}

impl Default for AssetModelingLabApp {
    fn default() -> Self {
        Self {
            state: None,
            jobs: AssetJobCoordinator::default(),
            current_texture: None,
            candidate_textures: BTreeMap::new(),
            viewport_state: ViewportInteractionState::default(),
            render_settings: asset_render_settings(),
            wireframe: false,
            show_template_picker: true,
            status: VecDeque::from(["Choose a template to begin Asset Modeling Lab.".to_owned()]),
            left_tab: AssetLeftTab::Parts,
        }
    }
}

impl AssetModelingLabApp {
    /// Draw the asset modeling lab surface.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs(&ctx);

        let mut commands = Vec::new();
        egui::Panel::top("asset_modeling_toolbar").show_inside(ui, |ui| {
            commands.extend(self.show_toolbar(ui));
        });
        egui::Panel::bottom("asset_modeling_status")
            .default_size(28.0)
            .show_inside(ui, |ui| self.show_status(ui));
        egui::Panel::bottom("asset_modeling_candidates")
            .resizable(true)
            .default_size(220.0)
            .size_range(160.0..=340.0)
            .show_inside(ui, |ui| {
                if let Some(state) = &self.state {
                    let ui_state = build_asset_ui_state(state, self.wireframe);
                    commands.extend(self.show_candidate_directions(ui, state, &ui_state));
                } else {
                    ui.heading("Directions");
                    ui.weak("Template required");
                }
            });
        egui::Panel::left("asset_modeling_left")
            .resizable(true)
            .default_size(280.0)
            .size_range(210.0..=420.0)
            .show_inside(ui, |ui| {
                if let Some(state) = &self.state {
                    let ui_state = build_asset_ui_state(state, self.wireframe);
                    commands.extend(self.show_left_panel(ui, &ui_state));
                } else {
                    ui.heading("Parts");
                    ui.weak("No template selected");
                }
            });
        egui::Panel::right("asset_modeling_right")
            .resizable(true)
            .default_size(360.0)
            .size_range(260.0..=500.0)
            .show_inside(ui, |ui| {
                if let Some(state) = &self.state {
                    let ui_state = build_asset_ui_state(state, self.wireframe);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        commands.extend(inspector::show(ui, &ui_state));
                        ui.separator();
                        self.show_validation(ui, &ui_state);
                    });
                } else {
                    ui.heading("Inspector");
                    ui.weak("No template selected");
                }
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.show_template_picker || self.state.is_none() {
                commands.extend(self.show_template_choices(ui));
            } else if let Some(state) = &self.state {
                let ui_state = build_asset_ui_state(state, self.wireframe);
                let overlay = self.viewport_overlay(state, &ui_state);
                let response = show_asset_viewport(
                    ui,
                    &mut self.viewport_state,
                    &state.current_camera,
                    self.current_texture.as_ref(),
                    &overlay,
                );
                commands.extend(response.commands);
            }
        });

        self.apply_commands(commands, &ctx);
        if self
            .state
            .as_ref()
            .is_some_and(|state| !state.active_jobs.is_empty())
        {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        let _ = frame;
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) -> Vec<AssetAppCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            if ui.button("New Template").clicked() {
                self.show_template_picker = true;
            }
            if ui.button("Open").clicked()
                && let Some(path) = open_asset_project_file()
            {
                commands.push(AssetAppCommand::Load(path));
            }
            let can_save = self
                .state
                .as_ref()
                .is_some_and(|state| state.current_file_path.is_some());
            if ui
                .add_enabled(can_save, egui::Button::new("Save"))
                .clicked()
            {
                commands.push(AssetAppCommand::Save);
            }
            if ui.button("Save As").clicked()
                && let Some(state) = &self.state
                && let Some(path) = save_asset_project_file(&state.recipe.title)
            {
                commands.push(AssetAppCommand::SaveAs(path));
            }
            ui.menu_button("Export", |ui| {
                if ui.button("OBJ").clicked()
                    && let Some(path) = export_obj_file()
                {
                    commands.push(AssetAppCommand::ExportObj(path));
                    ui.close();
                }
                if ui.button("Canonical Package").clicked()
                    && let Some(path) = export_package_directory()
                {
                    commands.push(AssetAppCommand::ExportPackage(path));
                    ui.close();
                }
            });
            if ui.button("Undo").clicked() {
                commands.push(AssetAppCommand::Undo);
            }
            if ui.button("Refine").clicked() {
                commands.push(AssetAppCommand::GenerateRefine);
            }
            if ui.button("Explore").clicked() {
                commands.push(AssetAppCommand::GenerateExplore);
            }
            if let Some(state) = &self.state {
                let marker = if state.dirty { "Unsaved" } else { "Saved" };
                ui.separator();
                ui.label(format!("Asset Modeling Lab | {marker}"));
            }
        });
        commands
    }

    fn show_template_choices(&mut self, ui: &mut egui::Ui) -> Vec<AssetAppCommand> {
        let mut commands = Vec::new();
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading("Asset Modeling Lab");
            ui.label("Choose a template");
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                for template in asset_templates() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(egui::vec2(230.0, 120.0));
                        ui.label(egui::RichText::new(&template.title).strong());
                        ui.small(template_description(&template.id));
                        if ui.button("Start").clicked() {
                            commands.push(AssetAppCommand::LoadTemplate(template));
                        }
                    });
                }
            });
        });
        commands
    }

    fn show_left_panel(
        &mut self,
        ui: &mut egui::Ui,
        ui_state: &crate::asset::AssetUiState,
    ) -> Vec<AssetAppCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.left_tab, AssetLeftTab::Parts, "Parts");
            ui.selectable_value(&mut self.left_tab, AssetLeftTab::History, "History");
        });
        ui.separator();
        match self.left_tab {
            AssetLeftTab::Parts => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    commands.extend(part_tree::show(ui, ui_state));
                });
            }
            AssetLeftTab::History => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    commands.extend(history::show(ui, ui_state));
                });
            }
        }
        commands
    }

    fn show_validation(&self, ui: &mut egui::Ui, state: &crate::asset::AssetUiState) {
        ui.heading("Validation");
        if state.validation.is_empty() {
            ui.label("Valid");
        } else {
            for message in &state.validation {
                ui.label(format!("{}: {}", message.state.label(), message.message));
            }
        }
        ui.separator();
        ui.heading("Locks");
        ui.small(format!("{} parameter lock(s)", state.parameter_locks.len()));
        ui.small(format!("{} part lock(s)", state.part_locks.len()));
        ui.small(format!("{} subtree lock(s)", state.subtree_locks.len()));
        ui.small(format!("{} topology lock(s)", state.topology_locks.len()));
    }

    fn show_candidate_directions(
        &self,
        ui: &mut egui::Ui,
        state: &AssetAppState,
        ui_state: &crate::asset::AssetUiState,
    ) -> Vec<AssetAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Directions");
        if let Some(progress) = candidate_gallery::generation_progress_label(ui_state) {
            ui.label(progress);
        }
        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.render_current_direction_card(ui, ui_state);
                    for slot in &state.candidate_slots {
                        commands.extend(self.render_candidate_direction_card(ui, slot));
                    }
                    for index in
                        state.candidate_slots.len()..candidate_gallery::VISIBLE_CANDIDATE_SLOTS
                    {
                        Self::render_empty_direction_card(ui, index, ui_state.active_job.is_some());
                    }
                });
            });
        commands
    }

    fn render_current_direction_card(&self, ui: &mut egui::Ui, state: &crate::asset::AssetUiState) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(250.0);
            ui.label(egui::RichText::new("Current asset").strong());
            if let Some(texture) = &self.current_texture {
                ui.image((texture.id(), egui::vec2(230.0, 150.0)));
            }
            ui.small(format!("{} part(s)", state.parts.len()));
            if state.validation.is_empty() {
                ui.small("No validation warnings");
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(226, 164, 72),
                    format!("{} validation note(s)", state.validation.len()),
                );
            }
        });
    }

    fn render_candidate_direction_card(
        &self,
        ui: &mut egui::Ui,
        slot: &crate::asset::state::AssetCandidateSlot,
    ) -> Vec<AssetAppCommand> {
        let mut commands = Vec::new();
        let candidate = crate::asset::view_model::candidate_for_slot(slot);
        let summary = candidate_gallery::candidate_summary(&candidate);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(270.0);
            ui.label(egui::RichText::new(format!("Direction {}", slot.slot + 1)).strong());
            if let Some(texture) = self.candidate_textures.get(&slot.candidate.id) {
                ui.image((texture.id(), egui::vec2(250.0, 160.0)));
            } else {
                ui.allocate_ui(egui::vec2(250.0, 160.0), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.weak("Rendering preview");
                    });
                });
            }
            ui.horizontal_wrapped(|ui| {
                ui.small(summary.structural_summary);
                ui.small("·");
                ui.small(summary.numeric_summary);
                ui.small("·");
                ui.small(format!("Validation: {}", summary.validation));
            });
            for line in candidate_gallery::candidate_edit_lines(&candidate, 3) {
                ui.small(line);
            }
            ui.horizontal(|ui| {
                if ui.button("Choose").clicked() {
                    commands.push(candidate_gallery::accept_candidate_command(
                        slot.candidate.id,
                    ));
                }
                if ui.small_button("Dismiss").clicked() {
                    commands.push(candidate_gallery::reject_candidate_command(
                        slot.candidate.id,
                    ));
                }
            });
        });
        commands
    }

    fn render_empty_direction_card(ui: &mut egui::Ui, index: usize, active: bool) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(220.0);
            ui.label(egui::RichText::new(format!("Direction {}", index + 1)).strong());
            ui.allocate_ui(egui::vec2(200.0, 120.0), |ui| {
                ui.centered_and_justified(|ui| {
                    if active {
                        ui.weak("Generating");
                    } else {
                        ui.weak("No direction yet");
                    }
                });
            });
        });
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if let Some(message) = self.status.back() {
                ui.label(message);
            }
            if let Some(state) = &self.state
                && let Some(job) = build_asset_ui_state(state, self.wireframe).active_job
            {
                ui.separator();
                ui.label(format!("{}: {}/{}", job.phase, job.completed, job.total));
            }
        });
    }

    fn apply_commands(&mut self, commands: Vec<AssetAppCommand>, ctx: &egui::Context) {
        for command in commands {
            if let AssetAppCommand::SetWireframe(wireframe) = command {
                self.wireframe = wireframe;
                continue;
            }
            let render_after_accept = matches!(command, AssetAppCommand::AcceptCandidate(_));
            let render_after_viewport = self.prepare_viewport_command(&command);
            let Some(state) = &mut self.state else {
                if let AssetAppCommand::LoadTemplate(template) = command {
                    self.load_template(template, ctx);
                } else if let AssetAppCommand::Load(path) = command {
                    self.load_project(path, ctx);
                }
                continue;
            };
            let result = state.handle_command(command);
            match result {
                Ok(effects) => {
                    self.apply_effects(effects, ctx);
                    if render_after_accept && let Some(state) = &mut self.state {
                        match state.request_render_current_preview(self.render_settings.clone()) {
                            Ok(effects) => self.apply_effects(effects, ctx),
                            Err(error) => self.push_status(error.to_string()),
                        }
                    }
                    if render_after_viewport && let Some(state) = &mut self.state {
                        match state.request_render_current_preview(self.render_settings.clone()) {
                            Ok(effects) => self.apply_effects(effects, ctx),
                            Err(error) => self.push_status(error.to_string()),
                        }
                    }
                }
                Err(error) => self.push_status(error.to_string()),
            }
        }
    }

    fn prepare_viewport_command(&mut self, command: &AssetAppCommand) -> bool {
        match command {
            AssetAppCommand::Viewport(
                ViewportAction::RequestInteractiveRender(request)
                | ViewportAction::RequestFinalRender(request),
            ) => {
                apply_viewport_render_size(&mut self.render_settings, request);
                true
            }
            AssetAppCommand::Viewport(
                ViewportAction::FitToObject
                | ViewportAction::ResetCamera
                | ViewportAction::SetCamera(_),
            ) => true,
            _ => false,
        }
    }

    fn apply_effects(&mut self, effects: Vec<AssetAppEffect>, ctx: &egui::Context) {
        for effect in effects {
            match effect {
                AssetAppEffect::StartJob(request) => self.submit_job(*request),
                AssetAppEffect::SaveProject { path, project } => {
                    self.save_project(path, *project);
                }
                AssetAppEffect::LoadProject(path) => self.load_project(path, ctx),
            }
        }
    }

    fn submit_job(&mut self, request: AssetJobRequest) {
        self.jobs.submit(request);
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let mut affected = false;
        let mut schedule_render = false;
        let mut schedule_candidate_previews = false;

        while let Ok(event) = self.jobs.try_recv() {
            let compile_ready = matches!(event, AssetJobEvent::CompileReady { .. });
            let candidates_ready = matches!(event, AssetJobEvent::CandidatesReady { .. });
            let export_message = export_status_message(&event);
            let Some(state) = &mut self.state else {
                continue;
            };
            let accepted = state.handle_job_event(event);
            if accepted {
                affected = true;
                schedule_render |= compile_ready;
                schedule_candidate_previews |= candidates_ready;
                if let Some(message) = export_message {
                    self.push_status(message);
                }
            }
        }

        if schedule_render && let Some(state) = &mut self.state {
            match state.request_render_current_preview(self.render_settings.clone()) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.push_status(error.to_string()),
            }
        }
        if schedule_candidate_previews && let Some(state) = &mut self.state {
            match state.request_candidate_previews(candidate_render_settings(&self.render_settings))
            {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.push_status(error.to_string()),
            }
        }
        if affected {
            self.refresh_textures(ctx);
            ctx.request_repaint();
        }
    }

    fn refresh_textures(&mut self, ctx: &egui::Context) {
        if let Some(preview) = self
            .state
            .as_ref()
            .and_then(|state| state.current_preview.as_ref())
        {
            self.current_texture =
                load_rendered_texture(ctx, "shape-lab-asset-current", &preview.image);
        }

        let live_ids: BTreeSet<AssetCandidateId> = self
            .state
            .as_ref()
            .map(|state| {
                state
                    .candidate_slots
                    .iter()
                    .map(|slot| slot.candidate.id)
                    .collect()
            })
            .unwrap_or_default();
        self.candidate_textures
            .retain(|candidate_id, _| live_ids.contains(candidate_id));
        if let Some(state) = &self.state {
            for slot in &state.candidate_slots {
                let Some(preview) = &slot.preview else {
                    continue;
                };
                if let Entry::Vacant(entry) = self.candidate_textures.entry(slot.candidate.id)
                    && let Some(texture) = load_asset_candidate_texture(ctx, preview)
                {
                    entry.insert(texture);
                }
            }
        }
    }

    fn load_template(&mut self, template: AssetTemplate, ctx: &egui::Context) {
        match AssetAppState::from_template(template) {
            Ok(mut state) => match state.request_compile_current() {
                Ok(effects) => {
                    self.state = Some(state);
                    self.show_template_picker = false;
                    self.current_texture = None;
                    self.candidate_textures.clear();
                    self.push_status("Template loaded");
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.push_status(error.to_string()),
            },
            Err(error) => self.push_status(error.to_string()),
        }
    }

    fn save_project(&mut self, path: PathBuf, project: crate::asset::state::AssetModelingProject) {
        match save_project_snapshot(&path, &project) {
            Ok(()) => {
                if let Some(state) = &mut self.state {
                    state.mark_saved(path.clone());
                }
                self.push_status(format!("Saved {}", path.display()));
            }
            Err(error) => self.push_status(error),
        }
    }

    fn load_project(&mut self, path: PathBuf, ctx: &egui::Context) {
        let result = load_project_snapshot(&path);
        match result {
            Ok(LoadedAssetProject::Project(project)) => {
                let mut state = if let Some(state) = self.state.take() {
                    state
                } else {
                    let Some(seed_recipe) = project
                        .revision_history
                        .revisions
                        .values()
                        .next()
                        .map(|revision| revision.recipe.clone())
                    else {
                        self.push_status("asset project has no revisions");
                        return;
                    };
                    match AssetAppState::new(seed_recipe) {
                        Ok(state) => state,
                        Err(error) => {
                            self.push_status(error.to_string());
                            return;
                        }
                    }
                };
                match state.replace_loaded_project(project, path.clone()) {
                    Ok(effects) => {
                        self.state = Some(state);
                        self.show_template_picker = false;
                        self.candidate_textures.clear();
                        self.push_status(format!("Loaded {}", path.display()));
                        self.apply_effects(effects, ctx);
                    }
                    Err(error) => self.push_status(error.to_string()),
                }
            }
            Ok(LoadedAssetProject::Recipe(recipe)) => {
                let recipe = *recipe;
                let mut state = self
                    .state
                    .take()
                    .or_else(|| AssetAppState::new(recipe.clone()).ok())
                    .expect("recipe was already validated");
                match state.replace_loaded_recipe(recipe, path.clone()) {
                    Ok(effects) => {
                        self.state = Some(state);
                        self.show_template_picker = false;
                        self.candidate_textures.clear();
                        self.push_status(format!("Loaded {}", path.display()));
                        self.apply_effects(effects, ctx);
                    }
                    Err(error) => self.push_status(error.to_string()),
                }
            }
            Err(error) => self.push_status(error),
        }
    }

    fn viewport_overlay(
        &self,
        state: &AssetAppState,
        ui_state: &crate::asset::AssetUiState,
    ) -> AssetViewportOverlay {
        let selected_part_name = state
            .selected_part_instance
            .and_then(|part| ui_state.parts.iter().find(|candidate| candidate.id == part))
            .map(|part| part.name.clone());
        AssetViewportOverlay {
            title: state.recipe.title.clone(),
            selected_part_name,
            selected_part_bounds: selected_part_bounds_overlay(state),
            validation_marker: if ui_state.validation.is_empty() {
                Some(crate::asset::AssetValidationState::Valid)
            } else {
                Some(crate::asset::AssetValidationState::Warning(format!(
                    "{} validation issue(s)",
                    ui_state.validation.len()
                )))
            },
            wireframe: self.wireframe,
            active_job_label: ui_state.active_job.as_ref().map(|job| job.phase.clone()),
            progress: ui_state.active_job.as_ref().map(|job| job.fraction()),
            ..AssetViewportOverlay::default()
        }
    }

    fn push_status(&mut self, message: impl Into<String>) {
        self.status.push_back(message.into());
        while self.status.len() > MAX_STATUS_MESSAGES {
            self.status.pop_front();
        }
    }
}

#[derive(Default)]
struct AssetJobCoordinator {
    tx: Option<Sender<AssetJobEvent>>,
    rx: Option<Receiver<AssetJobEvent>>,
}

impl AssetJobCoordinator {
    fn submit(&mut self, request: AssetJobRequest) {
        let tx = self.tx().clone();
        thread::spawn(move || {
            for event in run_asset_job(request) {
                let _ = tx.send(event);
            }
        });
    }

    fn try_recv(&mut self) -> Result<AssetJobEvent, crossbeam_channel::TryRecvError> {
        self.rx().try_recv()
    }

    fn tx(&mut self) -> &Sender<AssetJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            let (tx, rx) = unbounded();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }
        self.tx.as_ref().expect("tx initialized")
    }

    fn rx(&mut self) -> &Receiver<AssetJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            let (tx, rx) = unbounded();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }
        self.rx.as_ref().expect("rx initialized")
    }
}

enum LoadedAssetProject {
    Project(crate::asset::state::AssetModelingProject),
    Recipe(Box<AssetRecipe>),
}

fn asset_templates() -> Vec<AssetTemplate> {
    benchmark_assets()
        .into_iter()
        .map(|asset| AssetTemplate {
            id: asset.slug().to_owned(),
            title: asset.recipe().title.clone(),
            recipe: asset.recipe(),
        })
        .collect()
}

fn template_description(id: &str) -> &'static str {
    match BenchmarkAsset::parse(id) {
        Some(BenchmarkAsset::IndustrialCrate) => {
            "Crate body, handles, bolts, panels, vents, feet, and optional trim."
        }
        Some(BenchmarkAsset::MultiCutPanel) => {
            "One plate with a recessed panel, circular mounting holes, rectangular vents, and semantic cut metadata."
        }
        Some(BenchmarkAsset::ExplicitDeskLamp) => {
            "Lathed base and shade with swept supports, collars, switch details, and rim trim."
        }
        Some(BenchmarkAsset::StylizedStool) => {
            "Rounded seat, four tapered legs, support rails, bevels, and optional edge trim."
        }
        None => "Explicit asset template.",
    }
}

fn save_project_snapshot(
    path: &Path,
    project: &crate::asset::state::AssetModelingProject,
) -> Result<(), String> {
    let mut json = serde_json::to_vec_pretty(project).map_err(|error| error.to_string())?;
    json.push(b'\n');
    fs::write(path, json).map_err(|error| error.to_string())
}

fn load_project_snapshot(path: &Path) -> Result<LoadedAssetProject, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    match serde_json::from_slice::<crate::asset::state::AssetModelingProject>(&bytes) {
        Ok(project) => Ok(LoadedAssetProject::Project(project)),
        Err(project_error) => serde_json::from_slice::<AssetRecipe>(&bytes)
            .map(Box::new)
            .map(LoadedAssetProject::Recipe)
            .map_err(|recipe_error| {
                format!(
                    "project parse failed: {project_error}; recipe parse failed: {recipe_error}"
                )
            }),
    }
}

fn open_asset_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(ASSET_PROJECT_DIALOG_LABEL, &["json"])
        .pick_file()
}

fn save_asset_project_file(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(ASSET_PROJECT_DIALOG_LABEL, &["json"])
        .set_file_name(suggested_asset_project_file_name(title))
        .save_file()
}

fn export_obj_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Wavefront OBJ", &["obj"])
        .set_file_name("asset-model.obj")
        .save_file()
}

fn export_package_directory() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_directory(".")
        .pick_folder()
        .map(|path| path.join("shape-lab-model-package"))
}

fn export_status_message(event: &AssetJobEvent) -> Option<String> {
    match event {
        AssetJobEvent::ExportPackageReady {
            path,
            package_paths,
            ..
        } => Some(format!(
            "Exported package {} ({} part file(s))",
            path.display(),
            package_paths.parts.len()
        )),
        AssetJobEvent::ExportObjReady { path, report, .. } => Some(format!(
            "Exported OBJ {} ({} object(s), {} face(s))",
            path.display(),
            report.object_count,
            report.face_count
        )),
        _ => None,
    }
}

fn candidate_render_settings(settings: &RenderSettings) -> RenderSettings {
    let mut candidate = settings.clone();
    candidate.width = candidate.width.clamp(260, 420);
    candidate.height = candidate.height.clamp(180, 300);
    candidate
}

fn asset_render_settings() -> RenderSettings {
    RenderSettings {
        background: [0, 0, 0, 0],
        ..RenderSettings::default()
    }
}

fn selected_part_bounds_overlay(state: &AssetAppState) -> Option<NormalizedRect> {
    let selected = state.selected_part_instance?;
    let artifact = state.current_artifact.as_ref()?;
    let part = artifact
        .compiled_parts
        .iter()
        .find(|part| part.instance_id == selected)
        .or_else(|| {
            artifact
                .compiled_parts
                .iter()
                .find(|part| part.prototype_instance_id == Some(selected))
        })?;
    if part.triangulated_world.mesh.positions.is_empty() {
        return None;
    }

    let aspect_ratio = state
        .current_preview
        .as_ref()
        .and_then(|preview| {
            (preview.image.height > 0)
                .then(|| preview.image.width as f32 / preview.image.height as f32)
        })
        .filter(|aspect| aspect.is_finite() && *aspect > 0.0)
        .unwrap_or(1.0);
    let camera = state.current_camera.clamped();
    let yaw = camera.yaw_degrees.to_radians();
    let pitch = camera.pitch_degrees.to_radians();
    let pitch_cos = pitch.cos();
    let target = [camera.target.x, camera.target.y, camera.target.z];
    let eye = [
        target[0] + pitch_cos * yaw.sin() * camera.distance,
        target[1] + pitch.sin() * camera.distance,
        target[2] + pitch_cos * yaw.cos() * camera.distance,
    ];
    let forward = normalize3(sub3(target, eye), [0.0, 0.0, -1.0]);
    let right = normalize3(cross3(forward, [0.0, 1.0, 0.0]), [1.0, 0.0, 0.0]);
    let up = normalize3(cross3(right, forward), [0.0, 1.0, 0.0]);
    let half_fov_tan = (camera.vertical_fov_degrees.to_radians() * 0.5).tan();

    let mut min = [f32::INFINITY, f32::INFINITY];
    let mut max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut projected_any = false;
    for position in &part.triangulated_world.mesh.positions {
        let point = [position[0], position[1], position[2]];
        let delta = sub3(point, eye);
        let depth = dot3(delta, forward);
        if !depth.is_finite() || depth <= 0.01 {
            continue;
        }
        let half_height = depth * half_fov_tan;
        let half_width = half_height * aspect_ratio;
        if half_height <= f32::EPSILON || half_width <= f32::EPSILON {
            continue;
        }
        let x = 0.5 + dot3(delta, right) / (half_width * 2.0);
        let y = 0.5 - dot3(delta, up) / (half_height * 2.0);
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
        projected_any = true;
    }
    if !projected_any {
        return None;
    }

    let padding = 0.018;
    Some(NormalizedRect {
        min: [
            (min[0] - padding).clamp(0.0, 1.0),
            (min[1] - padding).clamp(0.0, 1.0),
        ],
        max: [
            (max[0] + padding).clamp(0.0, 1.0),
            (max[1] + padding).clamp(0.0, 1.0),
        ],
    })
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize3(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length_squared = dot3(value, value);
    if !length_squared.is_finite() || length_squared <= f32::EPSILON {
        return fallback;
    }
    let scale = length_squared.sqrt().recip();
    [value[0] * scale, value[1] * scale, value[2] * scale]
}

fn apply_viewport_render_size(settings: &mut RenderSettings, request: &ViewportRenderRequest) {
    settings.width = request.size.width;
    settings.height = request.size.height;
}

fn load_asset_candidate_texture(
    ctx: &egui::Context,
    preview: &crate::asset::jobs::AssetCandidatePreview,
) -> Option<TextureHandle> {
    let image = RenderedImage {
        width: preview.thumbnail_width,
        height: preview.thumbnail_height,
        rgba8: preview.thumbnail_rgba.clone(),
    };
    load_rendered_texture(
        ctx,
        &format!("shape-lab-asset-candidate-{}", preview.candidate_id.0),
        &image,
    )
}

fn load_rendered_texture(
    ctx: &egui::Context,
    name: &str,
    image: &RenderedImage,
) -> Option<TextureHandle> {
    let size = [
        usize::try_from(image.width).ok()?,
        usize::try_from(image.height).ok()?,
    ];
    let expected_len = size[0].checked_mul(size[1])?.checked_mul(4)?;
    if image.rgba8.len() != expected_len {
        return None;
    }
    let color_image = ColorImage::from_rgba_unmultiplied(size, &image.rgba8);
    Some(ctx.load_texture(name.to_owned(), color_image, TextureOptions::LINEAR))
}

#[allow(dead_code)]
fn _part_instance_for_docs(_: PartInstanceId) {}
