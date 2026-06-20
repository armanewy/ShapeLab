//! Application shell and desktop coordinator.

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::path::PathBuf;
use std::time::Duration;

use egui::{ColorImage, TextureHandle, TextureOptions};
use shape_core::CandidateId;
use shape_mesh::write_obj_to_path;
use shape_project::Project;
use shape_render::RenderedImage;

use crate::commands::{AppCommand, AppEffect};
use crate::jobs::{CandidatePreview, JobCoordinator, JobEvent, JobRequest};
use crate::panels::{gallery, history, inspector, menus, outliner, status};
use crate::state::{AppPhase, AppState};
use crate::viewport::{ViewportInteractionState, ViewportOverlayInfo, ViewportRenderRequest};

/// Native desktop application.
pub(crate) struct ShapeLabApp {
    state: AppState,
    jobs: Option<JobCoordinator>,
    current_texture: Option<TextureHandle>,
    candidate_textures: BTreeMap<CandidateId, TextureHandle>,
    viewport_state: ViewportInteractionState,
    inspector_state: inspector::InspectorPanelState,
    left_tab: LeftTab,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum LeftTab {
    Parts,
    History,
}

impl Default for ShapeLabApp {
    fn default() -> Self {
        let mut state = AppState::default();
        let jobs = match JobCoordinator::with_default_workers() {
            Ok(jobs) => Some(jobs),
            Err(error) => {
                state.record_recoverable_error(error.to_string());
                None
            }
        };

        let mut app = Self {
            state,
            jobs,
            current_texture: None,
            candidate_textures: BTreeMap::new(),
            viewport_state: ViewportInteractionState::default(),
            inspector_state: inspector::InspectorPanelState::default(),
            left_tab: LeftTab::Parts,
        };
        match app.state.request_preview_rebuild() {
            Ok(effects) => app.apply_effects(effects, None),
            Err(error) => app.state.record_recoverable_error(error.to_string()),
        }
        app
    }
}

impl eframe::App for ShapeLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs(&ctx);

        let mut commands = Vec::new();
        egui::Panel::top("shape_lab_menu").show_inside(ui, |ui| {
            commands.extend(menus::show(ui, &self.state));
        });
        egui::Panel::bottom("shape_lab_status").show_inside(ui, |ui| {
            status::show(ui, &self.state);
        });
        egui::Panel::bottom("shape_lab_gallery")
            .resizable(true)
            .default_size(190.0)
            .size_range(150.0..=320.0)
            .show_inside(ui, |ui| {
                commands.extend(gallery::show(
                    ui,
                    &self.state,
                    self.current_texture.as_ref(),
                    &self.candidate_textures,
                ));
            });
        egui::Panel::left("shape_lab_left")
            .resizable(true)
            .default_size(260.0)
            .size_range(190.0..=420.0)
            .show_inside(ui, |ui| {
                commands.extend(self.show_left_panel(ui));
            });
        egui::Panel::right("shape_lab_right")
            .resizable(true)
            .default_size(330.0)
            .size_range(240.0..=480.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    commands.extend(inspector::show(ui, &self.state, &mut self.inspector_state));
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let overlay = self.viewport_overlay();
            let response = crate::viewport::show_viewport(
                ui,
                &mut self.viewport_state,
                &self.state.camera,
                self.current_texture.as_ref(),
                &overlay,
            );
            commands.extend(response.actions.into_iter().map(AppCommand::Viewport));
        });

        self.apply_commands(commands, &ctx);
        if !self.state.active_jobs.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        let _ = frame;
    }
}

impl ShapeLabApp {
    fn show_left_panel(&mut self, ui: &mut egui::Ui) -> Vec<AppCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.left_tab, LeftTab::Parts, "Parts");
            ui.selectable_value(&mut self.left_tab, LeftTab::History, "History");
        });
        ui.separator();
        match self.left_tab {
            LeftTab::Parts => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    commands.extend(outliner::show(ui, &self.state));
                });
            }
            LeftTab::History => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    commands.extend(history::show(ui, &self.state));
                });
            }
        }
        commands
    }

    fn apply_commands(&mut self, commands: Vec<AppCommand>, ctx: &egui::Context) {
        for command in commands {
            self.prepare_command(&command);
            let cancel_all = matches!(command, AppCommand::CancelActiveGeneration);
            let result = self.state.handle_command(command);
            if cancel_all && let Some(jobs) = &self.jobs {
                jobs.cancel_all();
            }
            match result {
                Ok(effects) => self.apply_effects(effects, Some(ctx)),
                Err(error) => self.state.record_recoverable_error(error.to_string()),
            }
        }
    }

    fn prepare_command(&mut self, command: &AppCommand) {
        if let AppCommand::Viewport(
            crate::viewport::ViewportAction::RequestInteractiveRender(request)
            | crate::viewport::ViewportAction::RequestFinalRender(request),
        ) = command
        {
            apply_viewport_render_size(&mut self.state, request);
        }
        if matches!(command, AppCommand::GenerateDirections) {
            let proposal_count = self.inspector_state.proposal_count;
            let result_count = self.inspector_state.result_count;
            if let Err(error) = self.state.handle_command(AppCommand::SetSearchBudget {
                proposal_count,
                result_count,
            }) {
                self.state.record_recoverable_error(error.to_string());
            }
        }
    }

    fn apply_effects(&mut self, effects: Vec<AppEffect>, ctx: Option<&egui::Context>) {
        for effect in effects {
            match effect {
                AppEffect::StartJob(request) => self.submit_job(*request),
                AppEffect::SaveProject(path) => self.save_project(path),
                AppEffect::LoadProject(path) => self.load_project(path, ctx),
                AppEffect::ExportCurrentObj(path) => self.export_current_obj(path),
                AppEffect::RequestExit => {
                    if let Some(ctx) = ctx {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
    }

    fn submit_job(&mut self, request: JobRequest) {
        let job_id = request.job_id();
        let Some(jobs) = &self.jobs else {
            self.state
                .record_recoverable_error("background workers are unavailable");
            return;
        };
        match jobs.submit(request) {
            Ok(_) => {}
            Err(error) => {
                self.state.handle_job_event(JobEvent::Failed {
                    job_id,
                    message: error.to_string(),
                });
            }
        }
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let mut affected = false;
        while let Some(jobs) = &self.jobs {
            let event = match jobs.try_recv() {
                Ok(event) => event,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.state
                        .record_recoverable_error("background workers disconnected");
                    break;
                }
            };
            affected |= self.state.handle_job_event(event);
        }
        if affected {
            self.refresh_textures(ctx);
            ctx.request_repaint();
        }
    }

    fn refresh_textures(&mut self, ctx: &egui::Context) {
        if let Some(preview) = &self.state.current_preview {
            self.current_texture = load_rendered_texture(ctx, "shape-lab-current", &preview.image);
        } else {
            self.current_texture = None;
        }

        let live_ids: BTreeSet<CandidateId> = self
            .state
            .candidate_slots
            .iter()
            .map(|preview| preview.candidate.id)
            .collect();
        self.candidate_textures
            .retain(|candidate_id, _| live_ids.contains(candidate_id));
        for preview in &self.state.candidate_slots {
            if let Entry::Vacant(entry) = self.candidate_textures.entry(preview.candidate.id)
                && let Some(texture) = load_candidate_texture(ctx, preview)
            {
                entry.insert(texture);
            }
        }
    }

    fn save_project(&mut self, path: PathBuf) {
        match self.state.project.save_json(&path) {
            Ok(()) => self.state.mark_saved(path),
            Err(error) => self.state.record_recoverable_error(error.to_string()),
        }
    }

    fn load_project(&mut self, path: PathBuf, ctx: Option<&egui::Context>) {
        match Project::load_json(&path)
            .map_err(|error| error.to_string())
            .and_then(|project| {
                self.state
                    .replace_loaded_project(project, path.clone())
                    .map_err(|error| error.to_string())
            }) {
            Ok(effects) => {
                self.current_texture = None;
                self.candidate_textures.clear();
                self.apply_effects(effects, ctx);
            }
            Err(error) => self.state.record_recoverable_error(error),
        }
    }

    fn export_current_obj(&mut self, path: PathBuf) {
        let Some(preview) = &self.state.current_preview else {
            self.state
                .record_recoverable_error("export requires a ready preview");
            return;
        };
        match write_obj_to_path(&preview.mesh, &path) {
            Ok(()) => {
                self.state.mark_exported(path);
                true
            }
            Err(error) => {
                self.state
                    .record_recoverable_error(format!("export failed: {error}"));
                false
            }
        };
    }

    fn viewport_overlay(&self) -> ViewportOverlayInfo {
        let document = self.state.project.current_document().ok();
        let selected_node_name = self
            .state
            .selected_node
            .and_then(|node| document.and_then(|document| document.nodes.get(&node)))
            .map(|node| node.name.clone());

        ViewportOverlayInfo {
            title: self.state.project.title.clone(),
            revision_id: Some(format!(
                "Revision {}",
                self.state.project.current_revision.0
            )),
            selected_node_name,
            active_job_label: (!self.state.active_jobs.is_empty())
                .then(|| self.state.status.text.clone()),
            progress: self.state.status.progress,
            recoverable_error: self.state.recoverable_errors.back().cloned(),
            rendering: matches!(
                self.state.status.phase,
                AppPhase::BuildingPreview | AppPhase::Rendering | AppPhase::GeneratingCandidates
            ),
        }
    }
}

fn apply_viewport_render_size(state: &mut AppState, request: &ViewportRenderRequest) {
    state.render_settings.width = request.size.width;
    state.render_settings.height = request.size.height;
}

fn load_candidate_texture(
    ctx: &egui::Context,
    preview: &CandidatePreview,
) -> Option<TextureHandle> {
    load_rendered_texture(
        ctx,
        &format!("shape-lab-candidate-{}", preview.candidate.id.0),
        &preview.image,
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
