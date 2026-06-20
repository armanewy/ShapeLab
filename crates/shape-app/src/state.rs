//! UI-independent application state contract.

#![allow(dead_code)]

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use shape_core::{
    CandidateId, CoreError, EditProgram, NodeId, NodeKind, ParamGroup, ParamPath, PrimitiveKind,
    RevisionId, SetScalarEdit, ShapeDocument, ShapeNode, Transform3, get_scalar, validate_document,
};
use shape_mesh::{MeshSettings, TriangleMesh};
use shape_presets::{PresetError, PresetId, build_preset};
use shape_project::{Project, ProjectError};
use shape_render::{OrbitCamera, RenderSettings, RenderedImage, fit_camera_to_bounds};
use shape_search::{ExplorationMode, SearchRequest, TargetScope};

use crate::commands::{AppCommand, AppEffect};
use crate::jobs::{CandidatePreview, GenerationId, JobEvent, JobId, JobPhase, JobRequest};
use crate::viewport::ViewportAction;

const DEFAULT_PRESET_ID: &str = "desk-lamp";
const DEFAULT_SEED: u64 = 100;
const DEFAULT_PROPOSAL_COUNT: usize = 64;
const DEFAULT_RESULT_COUNT: usize = 6;
const DEFAULT_DESCRIPTOR_RESOLUTION: usize = 8;
const MIN_PROPOSAL_COUNT: usize = 1;
const MAX_PROPOSAL_COUNT: usize = 512;
const MIN_RESULT_COUNT: usize = 1;
const MAX_RESULT_COUNT: usize = 12;
const MAX_RECOVERABLE_ERRORS: usize = 12;

/// Lightweight status exposed to panels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatusMessage {
    pub text: String,
    pub phase: AppPhase,
    pub progress: Option<f32>,
}

/// Coarse application phase.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AppPhase {
    Idle,
    Loading,
    BuildingPreview,
    GeneratingCandidates,
    Rendering,
    Saving,
    Exporting,
    Error,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            text: "Ready".to_owned(),
            phase: AppPhase::Idle,
            progress: None,
        }
    }
}

/// Current preview cache.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentPreview {
    pub mesh: TriangleMesh,
    pub image: RenderedImage,
}

/// UI-independent state transition error.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppStateError {
    InvalidSelection(NodeId),
    MissingCurrentRevision(RevisionId),
    LockedParameter(ParamPath),
    MissingCandidate(CandidateId),
    MissingSavePath,
    MissingPreviewForExport,
    MissingActivePreset,
    JobIdOverflow,
    GenerationIdOverflow,
    InvalidSearchBudget(&'static str),
    Core(String),
    Project(String),
    Preset(String),
}

impl fmt::Display for AppStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelection(node) => {
                write!(formatter, "node {node:?} is not in the project")
            }
            Self::MissingCurrentRevision(revision) => {
                write!(formatter, "current revision {revision:?} is missing")
            }
            Self::LockedParameter(path) => write!(formatter, "parameter {path:?} is locked"),
            Self::MissingCandidate(candidate) => {
                write!(formatter, "candidate {candidate:?} is not available")
            }
            Self::MissingSavePath => write!(formatter, "save requires a file path"),
            Self::MissingPreviewForExport => {
                write!(formatter, "export requires a current mesh preview")
            }
            Self::MissingActivePreset => write!(formatter, "there is no active preset to reset"),
            Self::JobIdOverflow => write!(formatter, "job id overflow"),
            Self::GenerationIdOverflow => write!(formatter, "generation id overflow"),
            Self::InvalidSearchBudget(message) => {
                write!(formatter, "invalid search budget: {message}")
            }
            Self::Core(message) | Self::Project(message) | Self::Preset(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl Error for AppStateError {}

impl From<CoreError> for AppStateError {
    fn from(error: CoreError) -> Self {
        Self::Core(error.to_string())
    }
}

impl From<ProjectError> for AppStateError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error.to_string())
    }
}

impl From<PresetError> for AppStateError {
    fn from(error: PresetError) -> Self {
        Self::Preset(error.to_string())
    }
}

/// State owned by the application coordinator.
#[derive(Debug)]
pub(crate) struct AppState {
    pub project: Project,
    pub selected_node: Option<NodeId>,
    pub selected_target_scope: TargetScope,
    pub enabled_param_groups: BTreeSet<ParamGroup>,
    pub exploration_mode: ExplorationMode,
    pub seed: u64,
    pub generation_counter: u64,
    pub current_preview: Option<CurrentPreview>,
    pub candidate_slots: Vec<CandidatePreview>,
    pub camera: OrbitCamera,
    pub mesh_settings: MeshSettings,
    pub render_settings: RenderSettings,
    pub proposal_count: usize,
    pub result_count: usize,
    pub descriptor_resolution: usize,
    pub active_preset: Option<PresetId>,
    pub current_file_path: Option<PathBuf>,
    pub dirty: bool,
    pub status: StatusMessage,
    pub recoverable_errors: VecDeque<String>,
    pub active_jobs: BTreeSet<JobId>,
    pub active_generation: Option<GenerationId>,
    pub active_preview_job: Option<JobId>,
    pub active_render_job: Option<JobId>,
    pub active_generation_job: Option<JobId>,
    pub next_job_id: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_preset(PresetId(DEFAULT_PRESET_ID.to_owned())).unwrap_or_else(|error| {
            let mut state = Self::from_document(fallback_document(), None);
            state.record_error(error.to_string());
            state
        })
    }
}

impl AppState {
    /// Create state from a built-in preset.
    pub(crate) fn from_preset(preset_id: PresetId) -> Result<Self, AppStateError> {
        let document = build_preset(&preset_id)?;
        Ok(Self::from_document(document, Some(preset_id)))
    }

    fn from_document(document: ShapeDocument, active_preset: Option<PresetId>) -> Self {
        let title = document.title.clone();
        let root = document.root;
        let project = Project::new(title, document);
        Self {
            project,
            selected_node: Some(root),
            selected_target_scope: TargetScope::Selected,
            enabled_param_groups: all_param_groups(),
            exploration_mode: ExplorationMode::Explore,
            seed: DEFAULT_SEED,
            generation_counter: 0,
            current_preview: None,
            candidate_slots: Vec::new(),
            camera: OrbitCamera::default(),
            mesh_settings: MeshSettings::default(),
            render_settings: RenderSettings::default(),
            proposal_count: DEFAULT_PROPOSAL_COUNT,
            result_count: DEFAULT_RESULT_COUNT,
            descriptor_resolution: DEFAULT_DESCRIPTOR_RESOLUTION,
            active_preset,
            current_file_path: None,
            dirty: false,
            status: StatusMessage::default(),
            recoverable_errors: VecDeque::new(),
            active_jobs: BTreeSet::new(),
            active_generation: None,
            active_preview_job: None,
            active_render_job: None,
            active_generation_job: None,
            next_job_id: 1,
        }
    }

    /// Apply one UI command and return deferred side effects.
    pub(crate) fn handle_command(
        &mut self,
        command: AppCommand,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        let result = self.apply_command(command);
        if let Err(error) = &result {
            self.record_error(error.to_string());
        }
        result
    }

    /// Apply a background job event. Returns true when the event affected state.
    pub(crate) fn handle_job_event(&mut self, event: JobEvent) -> bool {
        match event {
            JobEvent::Started { job_id, phase } => {
                if !self.active_jobs.contains(&job_id) {
                    return false;
                }
                self.set_status(
                    status_for_job_phase(phase),
                    app_phase_for_job_phase(phase),
                    None,
                );
                true
            }
            JobEvent::Progress {
                job_id,
                phase,
                progress,
            } => {
                if !self.active_jobs.contains(&job_id) {
                    return false;
                }
                let progress = progress.is_finite().then_some(progress.clamp(0.0, 1.0));
                self.set_status(
                    status_for_job_phase(phase),
                    app_phase_for_job_phase(phase),
                    progress,
                );
                true
            }
            JobEvent::CurrentPreviewReady {
                job_id,
                mesh,
                image,
                camera,
            } => self.apply_current_preview_event(job_id, mesh, image, camera),
            JobEvent::CandidatePreviewReady {
                job_id,
                generation_id,
                preview,
            } => self.apply_candidate_preview_event(job_id, generation_id, preview),
            JobEvent::GenerationComplete {
                job_id,
                generation_id,
            } => self.apply_generation_complete(job_id, generation_id),
            JobEvent::ExportComplete { job_id, path } => {
                if !self.finish_known_job(job_id) {
                    return false;
                }
                self.set_status(format!("Exported {}", path.display()), AppPhase::Idle, None);
                true
            }
            JobEvent::Failed { job_id, message } => {
                if !self.finish_known_job(job_id) {
                    return false;
                }
                self.record_error(message);
                true
            }
            JobEvent::Cancelled { job_id } => {
                if !self.finish_known_job(job_id) {
                    return false;
                }
                self.set_status("Job cancelled", AppPhase::Idle, None);
                true
            }
        }
    }

    /// Replace state after a project was loaded by an I/O effect.
    pub(crate) fn replace_loaded_project(
        &mut self,
        project: Project,
        path: PathBuf,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        project.validate()?;
        let selected_node = project.current_document()?.root;
        self.project = project;
        self.selected_node = Some(selected_node);
        self.active_preset = None;
        self.current_file_path = Some(path);
        self.dirty = false;
        self.stop_all_jobs();
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.schedule_preview_rebuild(None)
    }

    /// Mark a save effect as completed successfully.
    pub(crate) fn mark_saved(&mut self, path: PathBuf) {
        self.current_file_path = Some(path);
        self.dirty = false;
        self.set_status("Project saved", AppPhase::Idle, None);
    }

    /// Mark an export effect as completed successfully.
    pub(crate) fn mark_exported(&mut self, path: PathBuf) {
        self.set_status(format!("Exported {}", path.display()), AppPhase::Idle, None);
    }

    /// Schedule a rebuild of the current preview from the current document.
    pub(crate) fn request_preview_rebuild(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        self.invalidate_preview();
        self.set_status("Building preview", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(None)
    }

    /// Record a recoverable error from UI-side I/O or coordination code.
    pub(crate) fn record_recoverable_error(&mut self, message: impl Into<String>) {
        self.record_error(message.into());
    }

    fn apply_command(&mut self, command: AppCommand) -> Result<Vec<AppEffect>, AppStateError> {
        match command {
            AppCommand::SelectNode(node) => {
                self.select_node(node)?;
                Ok(Vec::new())
            }
            AppCommand::SetScalar { path, value } => self.set_scalar(path, value),
            AppCommand::ToggleLock { path, locked } => {
                self.toggle_lock(path, locked)?;
                Ok(Vec::new())
            }
            AppCommand::SetTargetScope(scope) => {
                self.selected_target_scope = scope;
                self.clear_candidates_and_generation();
                self.set_status("Target updated", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::SetParameterGroup { group, enabled } => {
                if enabled {
                    self.enabled_param_groups.insert(group);
                } else {
                    self.enabled_param_groups.remove(&group);
                }
                self.clear_candidates_and_generation();
                self.set_status("Search controls updated", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::SetExplorationMode(mode) => {
                self.exploration_mode = mode;
                self.clear_candidates_and_generation();
                self.set_status("Exploration mode updated", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::SetSearchBudget {
                proposal_count,
                result_count,
            } => {
                self.set_search_budget(proposal_count, result_count)?;
                Ok(Vec::new())
            }
            AppCommand::SetSeed(seed) => {
                self.seed = seed;
                self.generation_counter = 0;
                self.clear_candidates_and_generation();
                self.set_status("Seed updated", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::GenerateDirections => self.generate_directions(),
            AppCommand::CancelActiveGeneration => {
                self.cancel_active_generation();
                Ok(Vec::new())
            }
            AppCommand::AcceptCandidate(candidate_id) => self.accept_candidate(candidate_id),
            AppCommand::DismissCandidate(candidate_id) => {
                self.candidate_slots
                    .retain(|preview| preview.candidate.id != candidate_id);
                self.set_status("Candidate dismissed", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::ClearCandidates => {
                self.clear_candidates_and_generation();
                self.set_status("Candidates cleared", AppPhase::Idle, None);
                Ok(Vec::new())
            }
            AppCommand::Undo => self.undo(),
            AppCommand::SwitchRevision(revision) => self.switch_revision(revision),
            AppCommand::LoadPreset(preset) => self.load_preset(preset, true),
            AppCommand::ResetCurrentPreset => {
                let preset = self
                    .active_preset
                    .clone()
                    .ok_or(AppStateError::MissingActivePreset)?;
                self.load_preset(preset, true)
            }
            AppCommand::Viewport(action) => self.apply_viewport_action(action),
            AppCommand::Save => self.save(),
            AppCommand::SaveAs(path) => {
                self.set_status("Saving project", AppPhase::Saving, None);
                Ok(vec![AppEffect::SaveProject(path)])
            }
            AppCommand::OpenProject(path) => {
                self.set_status("Loading project", AppPhase::Loading, None);
                Ok(vec![AppEffect::LoadProject(path)])
            }
            AppCommand::ExportCurrentObj(path) => self.export_current_obj(path),
            AppCommand::Exit => Ok(vec![AppEffect::RequestExit]),
            AppCommand::FitView => self.fit_view(),
        }
    }

    fn select_node(&mut self, node: Option<NodeId>) -> Result<(), AppStateError> {
        if let Some(node) = node {
            let document = self.project.current_document()?;
            if !document.nodes.contains_key(&node) {
                return Err(AppStateError::InvalidSelection(node));
            }
        }
        self.selected_node = node;
        self.set_status("Selection updated", AppPhase::Idle, None);
        Ok(())
    }

    fn set_scalar(&mut self, path: ParamPath, value: f32) -> Result<Vec<AppEffect>, AppStateError> {
        let current = self.project.current()?;
        if current.document.locks.contains(&path) {
            return Err(AppStateError::LockedParameter(path));
        }
        let before = get_scalar(&current.document, &path)?;
        if before == value {
            self.set_status("Parameter unchanged", AppPhase::Idle, None);
            return Ok(Vec::new());
        }

        let edit = EditProgram {
            label: "Direct parameter edit".to_owned(),
            seed: self.seed,
            operations: vec![SetScalarEdit {
                path: path.clone(),
                before,
                after: value,
            }],
        };
        let edited = shape_core::apply_edit(&current.document, &edit)?;
        self.replace_current_document(edited)?;
        self.selected_node = Some(path.node);
        self.dirty = true;
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.set_status("Parameter updated", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(Some(self.camera.clone()))
    }

    fn toggle_lock(&mut self, path: ParamPath, locked: bool) -> Result<(), AppStateError> {
        let current = self.project.current()?;
        get_scalar(&current.document, &path)?;
        let mut document = current.document.clone();
        let changed = if locked {
            document.locks.insert(path)
        } else {
            document.locks.remove(&path)
        };
        if !changed {
            self.set_status("Lock unchanged", AppPhase::Idle, None);
            return Ok(());
        }
        ensure_valid_document(&document)?;
        self.replace_current_document(document)?;
        self.dirty = true;
        self.clear_candidates_and_generation();
        self.set_status("Parameter lock updated", AppPhase::Idle, None);
        Ok(())
    }

    fn set_search_budget(
        &mut self,
        proposal_count: usize,
        result_count: usize,
    ) -> Result<(), AppStateError> {
        if !(MIN_PROPOSAL_COUNT..=MAX_PROPOSAL_COUNT).contains(&proposal_count) {
            return Err(AppStateError::InvalidSearchBudget(
                "proposal count is outside the supported range",
            ));
        }
        if !(MIN_RESULT_COUNT..=MAX_RESULT_COUNT).contains(&result_count) {
            return Err(AppStateError::InvalidSearchBudget(
                "result count is outside the supported range",
            ));
        }
        if result_count > proposal_count {
            return Err(AppStateError::InvalidSearchBudget(
                "result count cannot exceed proposal count",
            ));
        }
        self.proposal_count = proposal_count;
        self.result_count = result_count;
        self.clear_candidates_and_generation();
        self.set_status("Search budget updated", AppPhase::Idle, None);
        Ok(())
    }

    fn generate_directions(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        if self.enabled_param_groups.is_empty() {
            return Err(AppStateError::InvalidSearchBudget(
                "at least one parameter group must be enabled",
            ));
        }
        let document = self.project.current_document()?.clone();
        let generation_number = self
            .generation_counter
            .checked_add(1)
            .ok_or(AppStateError::GenerationIdOverflow)?;
        self.generation_counter = generation_number;
        let generation_id = GenerationId(generation_number);
        let request_seed = self.seed;
        self.seed = self.seed.wrapping_add(1);
        self.clear_candidates_and_generation();
        self.active_generation = Some(generation_id);
        let job_id = self.allocate_job_id()?;
        self.active_generation_job = Some(job_id);
        self.active_jobs.insert(job_id);

        let request = SearchRequest {
            seed: request_seed,
            proposal_count: self.proposal_count,
            result_count: self.result_count,
            descriptor_resolution: self.descriptor_resolution,
            selected_node: self.selected_node,
            target_scope: self.selected_target_scope,
            enabled_groups: self.enabled_param_groups.clone(),
            mode: self.exploration_mode,
        };
        self.set_status(
            "Generating directions",
            AppPhase::GeneratingCandidates,
            Some(0.0),
        );
        Ok(vec![AppEffect::StartJob(Box::new(
            JobRequest::GenerateCandidates {
                job_id,
                generation_id,
                document,
                request,
                candidate_mesh_settings: candidate_mesh_settings(self.mesh_settings),
                thumbnail_settings: thumbnail_render_settings(&self.render_settings),
                camera: self.camera.clone(),
            },
        ))])
    }

    fn cancel_active_generation(&mut self) {
        if let Some(job_id) = self.active_generation_job.take() {
            self.active_jobs.remove(&job_id);
        }
        self.active_generation = None;
        self.set_status("Generation cancelled", AppPhase::Idle, None);
    }

    fn accept_candidate(
        &mut self,
        candidate_id: CandidateId,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        let candidate = self
            .candidate_slots
            .iter()
            .find(|preview| preview.candidate.id == candidate_id)
            .map(|preview| preview.candidate.clone())
            .ok_or(AppStateError::MissingCandidate(candidate_id))?;
        self.project.accept_candidate(candidate)?;
        self.dirty = true;
        self.reconcile_selection_to_document()?;
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.set_status("Candidate accepted", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(Some(self.camera.clone()))
    }

    fn undo(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        self.project.undo()?;
        self.dirty = true;
        self.reconcile_selection_to_document()?;
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.set_status("Moved to parent revision", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(Some(self.camera.clone()))
    }

    fn switch_revision(&mut self, revision: RevisionId) -> Result<Vec<AppEffect>, AppStateError> {
        self.project.switch_to(revision)?;
        self.dirty = true;
        self.reconcile_selection_to_document()?;
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.set_status("Revision selected", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(Some(self.camera.clone()))
    }

    fn load_preset(
        &mut self,
        preset: PresetId,
        dirty: bool,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        let document = build_preset(&preset)?;
        let title = document.title.clone();
        let root = document.root;
        self.project = Project::try_new(title, document)?;
        self.selected_node = Some(root);
        self.active_preset = Some(preset);
        self.current_file_path = None;
        self.dirty = dirty;
        self.stop_all_jobs();
        self.invalidate_preview();
        self.clear_candidates_and_generation();
        self.camera = OrbitCamera::default();
        self.set_status("Preset loaded", AppPhase::BuildingPreview, None);
        self.schedule_preview_rebuild(None)
    }

    fn apply_viewport_action(
        &mut self,
        action: ViewportAction,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        match action {
            ViewportAction::Orbit {
                delta_yaw,
                delta_pitch,
                camera: _,
            } => {
                self.camera.orbit(delta_yaw, delta_pitch);
                self.schedule_current_render()
            }
            ViewportAction::Pan {
                delta_right,
                delta_up,
                camera: _,
            } => {
                self.camera.pan(delta_right, delta_up);
                self.schedule_current_render()
            }
            ViewportAction::Zoom { factor, camera: _ } => {
                self.camera.zoom(factor);
                self.schedule_current_render()
            }
            ViewportAction::SetCamera(camera) => {
                self.camera = camera.clamped();
                self.schedule_current_render()
            }
            ViewportAction::FitToObject => self.fit_view(),
            ViewportAction::ResetCamera => {
                self.camera = OrbitCamera::default();
                self.schedule_current_render()
            }
            ViewportAction::RequestInteractiveRender(request)
            | ViewportAction::RequestFinalRender(request) => {
                self.camera = request.camera.clamped();
                self.schedule_current_render()
            }
        }
    }

    fn save(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        let path = self
            .current_file_path
            .clone()
            .ok_or(AppStateError::MissingSavePath)?;
        self.set_status("Saving project", AppPhase::Saving, None);
        Ok(vec![AppEffect::SaveProject(path)])
    }

    fn export_current_obj(&mut self, path: PathBuf) -> Result<Vec<AppEffect>, AppStateError> {
        let mesh = self
            .current_preview
            .as_ref()
            .map(|preview| preview.mesh.clone())
            .ok_or(AppStateError::MissingPreviewForExport)?;
        let job_id = self.allocate_job_id()?;
        self.active_jobs.insert(job_id);
        self.set_status("Exporting OBJ", AppPhase::Exporting, None);
        Ok(vec![AppEffect::StartJob(Box::new(
            JobRequest::ExportCurrent { job_id, mesh, path },
        ))])
    }

    fn fit_view(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        if let Some(preview) = &self.current_preview {
            self.camera = fit_camera_to_bounds(preview.mesh.bounds);
            self.schedule_current_render()
        } else {
            self.camera = OrbitCamera::default();
            self.set_status("No preview to fit", AppPhase::Idle, None);
            Ok(Vec::new())
        }
    }

    fn schedule_preview_rebuild(
        &mut self,
        camera: Option<OrbitCamera>,
    ) -> Result<Vec<AppEffect>, AppStateError> {
        let job_id = self.allocate_job_id()?;
        self.active_preview_job = Some(job_id);
        self.active_jobs.insert(job_id);
        Ok(vec![AppEffect::StartJob(Box::new(
            JobRequest::BuildCurrentPreview {
                job_id,
                document: self.project.current_document()?.clone(),
                mesh_settings: self.mesh_settings,
                render_settings: self.render_settings.clone(),
                camera,
            },
        ))])
    }

    fn schedule_current_render(&mut self) -> Result<Vec<AppEffect>, AppStateError> {
        let Some(preview) = &self.current_preview else {
            self.set_status("No preview to render", AppPhase::Idle, None);
            return Ok(Vec::new());
        };
        let mesh = preview.mesh.clone();
        let job_id = self.allocate_job_id()?;
        self.active_render_job = Some(job_id);
        self.active_jobs.insert(job_id);
        self.set_status("Rendering view", AppPhase::Rendering, None);
        Ok(vec![AppEffect::StartJob(Box::new(
            JobRequest::RenderCurrentCamera {
                job_id,
                mesh,
                camera: self.camera.clone(),
                render_settings: self.render_settings.clone(),
            },
        ))])
    }

    fn allocate_job_id(&mut self) -> Result<JobId, AppStateError> {
        let id = JobId(self.next_job_id);
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(AppStateError::JobIdOverflow)?;
        Ok(id)
    }

    fn replace_current_document(&mut self, document: ShapeDocument) -> Result<(), AppStateError> {
        ensure_valid_document(&document)?;
        let current_revision = self.project.current_revision;
        let revision = self
            .project
            .revisions
            .get_mut(&current_revision)
            .ok_or(AppStateError::MissingCurrentRevision(current_revision))?;
        revision.document = document;
        self.project.validate()?;
        Ok(())
    }

    fn reconcile_selection_to_document(&mut self) -> Result<(), AppStateError> {
        let document = self.project.current_document()?;
        if let Some(selected) = self.selected_node
            && document.nodes.contains_key(&selected)
        {
            return Ok(());
        }
        self.selected_node = Some(document.root);
        Ok(())
    }

    fn invalidate_preview(&mut self) {
        self.current_preview = None;
        if let Some(job_id) = self.active_preview_job.take() {
            self.active_jobs.remove(&job_id);
        }
        if let Some(job_id) = self.active_render_job.take() {
            self.active_jobs.remove(&job_id);
        }
    }

    fn clear_candidates_and_generation(&mut self) {
        self.candidate_slots.clear();
        if let Some(job_id) = self.active_generation_job.take() {
            self.active_jobs.remove(&job_id);
        }
        self.active_generation = None;
    }

    fn stop_all_jobs(&mut self) {
        self.active_jobs.clear();
        self.active_preview_job = None;
        self.active_render_job = None;
        self.active_generation_job = None;
        self.active_generation = None;
    }

    fn record_error(&mut self, message: String) {
        self.recoverable_errors.push_back(message.clone());
        while self.recoverable_errors.len() > MAX_RECOVERABLE_ERRORS {
            self.recoverable_errors.pop_front();
        }
        self.set_status(message, AppPhase::Error, None);
    }

    fn set_status(&mut self, text: impl Into<String>, phase: AppPhase, progress: Option<f32>) {
        self.status = StatusMessage {
            text: text.into(),
            phase,
            progress,
        };
    }

    fn apply_current_preview_event(
        &mut self,
        job_id: JobId,
        mesh: TriangleMesh,
        image: RenderedImage,
        camera: OrbitCamera,
    ) -> bool {
        let is_preview = self.active_preview_job == Some(job_id);
        let is_render = self.active_render_job == Some(job_id);
        if !is_preview && !is_render {
            self.active_jobs.remove(&job_id);
            return false;
        }
        self.active_jobs.remove(&job_id);
        if is_preview {
            self.active_preview_job = None;
        }
        if is_render {
            self.active_render_job = None;
        }
        self.camera = camera.clamped();
        self.current_preview = Some(CurrentPreview { mesh, image });
        self.set_status("Preview ready", AppPhase::Idle, None);
        true
    }

    fn apply_candidate_preview_event(
        &mut self,
        job_id: JobId,
        generation_id: GenerationId,
        preview: CandidatePreview,
    ) -> bool {
        if self.active_generation != Some(generation_id)
            || self.active_generation_job != Some(job_id)
            || !self.active_jobs.contains(&job_id)
        {
            return false;
        }
        if let Some(existing) = self
            .candidate_slots
            .iter_mut()
            .find(|slot| slot.slot == preview.slot)
        {
            *existing = preview;
        } else {
            self.candidate_slots.push(preview);
        }
        self.candidate_slots.sort_by_key(|preview| preview.slot);
        self.set_status("Candidate ready", AppPhase::GeneratingCandidates, None);
        true
    }

    fn apply_generation_complete(&mut self, job_id: JobId, generation_id: GenerationId) -> bool {
        let was_known = self.active_jobs.remove(&job_id);
        if self.active_generation != Some(generation_id)
            || self.active_generation_job != Some(job_id)
            || !was_known
        {
            return false;
        }
        self.active_generation = None;
        self.active_generation_job = None;
        self.set_status("Generation complete", AppPhase::Idle, Some(1.0));
        true
    }

    fn finish_known_job(&mut self, job_id: JobId) -> bool {
        let known = self.active_jobs.remove(&job_id);
        if self.active_preview_job == Some(job_id) {
            self.active_preview_job = None;
        }
        if self.active_render_job == Some(job_id) {
            self.active_render_job = None;
        }
        if self.active_generation_job == Some(job_id) {
            self.active_generation_job = None;
            self.active_generation = None;
        }
        known
    }
}

fn ensure_valid_document(document: &ShapeDocument) -> Result<(), AppStateError> {
    let report = validate_document(document);
    if report.is_valid() {
        Ok(())
    } else {
        Err(AppStateError::Core(format!(
            "document validation failed with {} issue(s)",
            report.issues.len()
        )))
    }
}

fn all_param_groups() -> BTreeSet<ParamGroup> {
    [
        ParamGroup::Form,
        ParamGroup::Placement,
        ParamGroup::Rotation,
        ParamGroup::Scale,
        ParamGroup::Blend,
    ]
    .into_iter()
    .collect()
}

fn candidate_mesh_settings(settings: MeshSettings) -> MeshSettings {
    MeshSettings {
        resolution: settings.resolution.clamp(12, 28),
        padding_fraction: settings.padding_fraction,
        iso_value: settings.iso_value,
    }
}

fn thumbnail_render_settings(settings: &RenderSettings) -> RenderSettings {
    let mut thumbnail = settings.clone();
    thumbnail.width = thumbnail.width.clamp(160, 320);
    thumbnail.height = thumbnail.height.clamp(120, 240);
    thumbnail
}

fn status_for_job_phase(phase: JobPhase) -> &'static str {
    match phase {
        JobPhase::CompileField => "Preparing shape",
        JobPhase::Mesh => "Building mesh",
        JobPhase::Render => "Rendering view",
        JobPhase::Search => "Generating directions",
        JobPhase::Export => "Exporting OBJ",
    }
}

fn app_phase_for_job_phase(phase: JobPhase) -> AppPhase {
    match phase {
        JobPhase::CompileField | JobPhase::Mesh => AppPhase::BuildingPreview,
        JobPhase::Render => AppPhase::Rendering,
        JobPhase::Search => AppPhase::GeneratingCandidates,
        JobPhase::Export => AppPhase::Exporting,
    }
}

fn fallback_document() -> ShapeDocument {
    let root = ShapeNode {
        id: NodeId(1),
        name: "Fallback Sphere".to_owned(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Transform3::default(),
        kind: NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.5 }),
    };
    ShapeDocument::new("Fallback", root)
}
