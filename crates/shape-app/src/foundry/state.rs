//! State reducer for the native Foundry surface.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use shape_asset::RevisionId;
use shape_core::Aabb;
use shape_foundry::{
    CatalogContentRef, ControlEvaluationContext, ControlKind, ControlValue, FeasibleControlDomain,
    FoundryAssetDocument, FoundryBuildStamp, FoundryCandidateId, FoundryCatalogError,
    FoundryCatalogLock, FoundryCatalogResolver, FoundryCommand, FoundryCompilationOutput,
    FoundryConformanceSummary, FoundryDocumentId, FoundryEdit, FoundryLock, FoundryLockMode,
    FoundryLockTarget, FoundryPackDocument, FoundryPackExportProfile, FoundryPreferenceEvent,
    FoundryPreferenceLog, FoundryPreferenceProfile, FoundryPreferenceScope,
    FoundryProjectRevisionProgram, FoundryValidationReport, GenerateCandidatesRequest,
    GeneratedRecipeSnapshot, ProviderOverride, SharedProviderPolicy, apply_foundry_command,
    compile_foundry_document, control_divergence, default_control_value, effective_control_domain,
    validate_foundry_document,
};
use shape_mesh::TriangleMesh;
use shape_project::foundry::{FoundryProjectFile, FoundryProjectLoadReport};
use shape_render::{clay_readability_render_settings, fit_camera_to_bounds, render_mesh};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRequest,
};

use super::commands::{FoundryAppCommand, FoundryAppEffect};
use super::jobs::{
    FoundryJobEvent, FoundryJobRequest, FoundryJobSlot, candidate_cards_from_output,
};
use super::trace::{
    MakeJobTrace, MakeJobTraceEventInput, MakeJobTraceEventKind, build_trace_stamp,
    document_asset_name, document_trace_stamp, queued_kind_for_request, request_asset_name,
    request_trace_stamp, started_kind_for_request, trace_event, trace_event_for_request,
    trigger_action_for_request,
};
use super::view_model::{
    FoundryCandidateCard, FoundryControlPresentation, FoundryControlView, FoundryNumericRange,
    FoundryOptionCard, FoundryPackView,
};

const FIRST_JOB_ID: u64 = 1;
pub(crate) const DEFAULT_PREVIEW_PIXELS: u32 = 512;
const CANCELED_IDEA_SEARCH_STATUS: &str = "Canceled earlier idea search.";

/// Last rendered current-document preview.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryPreviewImage {
    /// Stable preview slot ID.
    pub preview_id: String,
    /// RGBA8 image bytes.
    pub rgba8: Vec<u8>,
    /// Image width.
    pub width: u32,
    /// Image height.
    pub height: u32,
    /// Camera used for the preview.
    pub camera: shape_render::OrbitCamera,
    /// Build represented by the preview.
    pub build: Option<FoundryBuildStamp>,
}

/// Complete UI-independent state for a Foundry editing session.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryAppState {
    /// Open semantic source document.
    pub document: Option<FoundryAssetDocument>,
    /// Project persistence wrapper when the session is backed by a project file.
    pub project_file: Option<FoundryProjectFile>,
    /// Current project path, when saved or loaded.
    pub project_path: Option<PathBuf>,
    /// Last project-load report.
    pub load_report: Option<FoundryProjectLoadReport>,
    /// Current revision ID.
    pub current_revision: Option<RevisionId>,
    /// Last exact compilation output for the current document.
    pub current_output: Option<Box<FoundryCompilationOutput>>,
    /// Build stamp from the current exact compilation.
    pub current_build: Option<FoundryBuildStamp>,
    /// Exact recipe snapshot paired with the current semantic document.
    pub recipe_snapshot: Option<GeneratedRecipeSnapshot>,
    /// Validation report for the current semantic document.
    pub validation: Option<FoundryValidationReport>,
    /// Active customizer control.
    pub active_control: Option<String>,
    /// Selected candidate card.
    pub selected_candidate: Option<FoundryCandidateId>,
    /// Selected pack member.
    pub selected_pack_member: Option<String>,
    /// Current Foundry locks reflected from the semantic document/project.
    pub locks: Vec<FoundryLock>,
    /// Visible controls for the customizer deck.
    pub controls: Vec<FoundryControlView>,
    /// Whole-model direction cards.
    pub candidates: Vec<FoundryCandidateCard>,
    /// Latest candidate generation output and diagnostics.
    pub candidate_output: Option<Box<FoundryCandidateOutput>>,
    /// Replayable edits keyed by generated candidate ID.
    pub candidate_edits: BTreeMap<FoundryCandidateId, FoundryEdit>,
    /// Local-only preference signals from explicit candidate choices.
    pub local_preferences: FoundryPreferenceLog,
    /// Current rendered whole-model preview.
    pub current_preview: Option<FoundryPreviewImage>,
    /// Family-pack workspace view.
    pub pack: FoundryPackView,
    /// Active background jobs keyed by app-local job ID.
    pub active_jobs: BTreeMap<u64, FoundryJobRequest>,
    /// Job IDs whose results have already been superseded.
    pub stale_jobs: BTreeSet<u64>,
    /// Whether the project has unsaved changes.
    pub dirty: bool,
    /// Whether the document is opened in read-only recovery mode.
    pub read_only: bool,
    /// Whether Advanced Recipe is open.
    pub advanced_recipe_open: bool,
    /// Last recoverable status or error message.
    pub status: Option<String>,
    /// Local-only Make job trace for diagnostics and dogfood runs.
    pub make_job_trace: MakeJobTrace,
    next_job_id: u64,
}

impl Default for FoundryAppState {
    fn default() -> Self {
        Self {
            document: None,
            project_file: None,
            project_path: None,
            load_report: None,
            current_revision: None,
            current_output: None,
            current_build: None,
            recipe_snapshot: None,
            validation: None,
            active_control: None,
            selected_candidate: None,
            selected_pack_member: None,
            locks: Vec::new(),
            controls: Vec::new(),
            candidates: Vec::new(),
            candidate_output: None,
            candidate_edits: BTreeMap::new(),
            local_preferences: FoundryPreferenceLog::new(),
            current_preview: None,
            pack: FoundryPackView::default(),
            active_jobs: BTreeMap::new(),
            stale_jobs: BTreeSet::new(),
            dirty: false,
            read_only: false,
            advanced_recipe_open: false,
            status: None,
            make_job_trace: MakeJobTrace::default(),
            next_job_id: FIRST_JOB_ID,
        }
    }
}

impl FoundryAppState {
    /// Create app state from an initial semantic document.
    pub(crate) fn new(document: FoundryAssetDocument) -> Result<Self, FoundryAppStateError> {
        let validation = validate_foundry_document(&document);
        if !validation.is_valid() {
            return Err(FoundryAppStateError::InvalidDocument(
                validation.issues.len(),
            ));
        }

        let catalog_lock = document
            .catalog_lock
            .clone()
            .unwrap_or_else(|| FoundryCatalogLock::from_document_refs(&document));
        let project_file = FoundryProjectFile::new(
            document.document_id.0.clone(),
            document.clone(),
            catalog_lock,
            document.build_stamp.clone(),
            None,
            FoundryConformanceSummary::default(),
        )
        .map_err(project_error)?;

        let mut state = Self {
            document: Some(document),
            validation: Some(validation),
            project_file: Some(project_file),
            ..Self::default()
        };
        state.refresh_from_document_snapshot();
        state.refresh_project_flags();
        state.record_template_started();
        Ok(state)
    }

    /// Create app state from a loaded project wrapper.
    pub(crate) fn from_project_file(
        project_file: FoundryProjectFile,
    ) -> Result<Self, FoundryAppStateError> {
        let revision = project_file.project.current().map_err(project_error)?;
        let mut state = Self {
            document: Some(revision.document.clone()),
            current_build: revision.build_stamp.clone(),
            recipe_snapshot: revision.recipe_snapshot.clone(),
            validation: Some(validate_foundry_document(&revision.document)),
            load_report: Some(project_file.load_report.clone()),
            read_only: project_file.load_report.read_only_recovery,
            project_file: Some(project_file),
            ..Self::default()
        };
        state.refresh_from_document_snapshot();
        state.refresh_project_flags();
        state.record_state_transition("Loaded project into Make.");
        Ok(state)
    }

    /// Apply one command and return deferred side effects.
    pub(crate) fn handle_command(
        &mut self,
        command: FoundryAppCommand,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        match command {
            FoundryAppCommand::SelectControl(control) => {
                self.active_control = control;
                Ok(Vec::new())
            }
            FoundryAppCommand::SelectCandidate(candidate) => {
                self.select_candidate(candidate);
                Ok(Vec::new())
            }
            FoundryAppCommand::SelectPackMember(member) => {
                self.selected_pack_member = member.clone();
                self.pack.selected_member = member;
                Ok(Vec::new())
            }
            FoundryAppCommand::RunFoundryCommand(command) => self.handle_foundry_command(*command),
            FoundryAppCommand::RunFoundryEdit(edit) => self.schedule_apply_edit(*edit),
            FoundryAppCommand::RunFoundryCommandProgram { label, commands } => {
                self.schedule_apply_edit(FoundryEdit { label, commands })
            }
            FoundryAppCommand::RequestCandidates(request) => self.request_candidates(request),
            FoundryAppCommand::CancelIdeaGeneration => self.cancel_idea_generation(),
            FoundryAppCommand::PreviewControlValue { control_id, value } => {
                self.preview_control_value(control_id, value)
            }
            FoundryAppCommand::RequestBuild => self.request_build(),
            FoundryAppCommand::RetryPreparation => self.retry_preparation(),
            FoundryAppCommand::RequestPreview { width, height } => {
                self.request_preview(width, height)
            }
            FoundryAppCommand::Save => self.save(),
            FoundryAppCommand::SaveAs(path) => self.save_as(path),
            FoundryAppCommand::Load(path) => Ok(vec![FoundryAppEffect::LoadProject(path)]),
            FoundryAppCommand::RequestPackBatchExport { out_dir } => {
                self.batch_export_current_pack(out_dir)
            }
            FoundryAppCommand::SetAdvancedRecipeOpen(open) => {
                self.advanced_recipe_open = open;
                Ok(Vec::new())
            }
        }
    }

    /// Apply a background job event. Returns true when the event affected state.
    pub(crate) fn handle_job_event(&mut self, event: FoundryJobEvent) -> bool {
        let job_id = event.job_id();
        let Some(active_request) = self.active_jobs.get(&job_id).cloned() else {
            self.stale_jobs.insert(job_id);
            self.status =
                Some("Ignored a background result because newer work is active.".to_owned());
            self.record_stale_result(
                job_id,
                None,
                None,
                None,
                "Ignored a background result because newer work is active.",
            );
            return false;
        };
        if !self.request_source_is_current(&active_request) {
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
            self.status = Some("Ignored a background result because the model changed.".to_owned());
            self.record_stale_result(
                job_id,
                Some(active_request.slot()),
                request_trace_stamp(&active_request),
                request_asset_name(&active_request),
                "Ignored a background result because the model changed.",
            );
            return false;
        }
        if !job_event_matches_request(&event, &active_request) {
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
            self.status =
                Some("Ignored a background result that no longer matches the request.".to_owned());
            self.record_stale_result(
                job_id,
                Some(active_request.slot()),
                request_trace_stamp(&active_request),
                request_asset_name(&active_request),
                "Ignored a background result that no longer matches the request.",
            );
            return false;
        }

        let accepted = match event {
            FoundryJobEvent::CompileFinished { output, .. } => {
                self.record_job_finished(
                    MakeJobTraceEventKind::BuildFinished,
                    &active_request,
                    Some(build_trace_stamp(&output.build_stamp)),
                    Some(document_asset_name(&output.document)),
                    "Build finished.",
                );
                self.apply_compilation_output(*output);
                true
            }
            FoundryJobEvent::PreviewRendered {
                preview_id,
                rgba8,
                width,
                height,
                camera,
                build,
                ..
            } => {
                self.record_job_finished(
                    MakeJobTraceEventKind::PreviewFinished,
                    &active_request,
                    build.as_ref().map(build_trace_stamp),
                    request_asset_name(&active_request),
                    "Preview finished.",
                );
                self.current_preview = Some(FoundryPreviewImage {
                    preview_id,
                    rgba8,
                    width,
                    height,
                    camera,
                    build,
                });
                true
            }
            FoundryJobEvent::CandidatesGenerated {
                request: candidate_request,
                output,
                cards,
                ..
            } => {
                self.record_job_finished(
                    MakeJobTraceEventKind::CandidateCompiled,
                    &active_request,
                    request_trace_stamp(&active_request),
                    self.document.as_ref().map(document_asset_name),
                    "Candidate plans compiled.",
                );
                self.apply_candidates_generated(candidate_request, *output, cards);
                true
            }
            FoundryJobEvent::CandidatePreviewsRendered {
                cards,
                rejected_count,
                ..
            } => {
                self.record_job_finished(
                    MakeJobTraceEventKind::CandidateRendered,
                    &active_request,
                    request_trace_stamp(&active_request),
                    request_asset_name(&active_request),
                    "Candidate previews rendered.",
                );
                self.record_job_finished(
                    MakeJobTraceEventKind::CandidateFinished,
                    &active_request,
                    request_trace_stamp(&active_request),
                    request_asset_name(&active_request),
                    "Candidate search finished.",
                );
                self.apply_candidate_previews(cards, rejected_count);
                true
            }
            FoundryJobEvent::EditApplied { edit, output, .. } => {
                self.record_job_finished(
                    MakeJobTraceEventKind::BuildFinished,
                    &active_request,
                    Some(build_trace_stamp(&output.build_stamp)),
                    Some(document_asset_name(&output.document)),
                    "Edit build finished.",
                );
                self.apply_edit_output(*edit, *output);
                self.stale_obsolete_active_jobs_except(job_id);
                true
            }
            FoundryJobEvent::PackCompiled { pack, .. } => {
                let mut pack = *pack;
                if let Some(selected) = self
                    .selected_pack_member
                    .clone()
                    .filter(|selected| pack.members.contains_key(selected))
                {
                    pack.selected_member = Some(selected.clone());
                    self.selected_pack_member = Some(selected);
                } else {
                    self.selected_pack_member = pack.selected_member.clone();
                }
                self.pack = pack;
                self.record_current_pack_member_added();
                true
            }
            FoundryJobEvent::PackExportFinished {
                profile,
                out_dir,
                member_count,
                ..
            } => {
                self.status = Some(format!(
                    "Exported {member_count} pack member(s) with {profile} to {}",
                    out_dir.display()
                ));
                true
            }
            FoundryJobEvent::ExportFinished {
                profile, out_dir, ..
            } => {
                self.record_current_variant_exported();
                self.status = Some(format!("Exported {profile} to {}", out_dir.display()));
                true
            }
            FoundryJobEvent::Failed { message, .. } => {
                let event_kind = failed_kind_for_request(&active_request);
                self.record_job_finished(
                    event_kind,
                    &active_request,
                    request_trace_stamp(&active_request),
                    request_asset_name(&active_request),
                    message.clone(),
                );
                self.status = Some(message);
                true
            }
        };
        self.active_jobs.remove(&job_id);
        accepted
    }

    /// Request a current-document compilation job.
    pub(crate) fn request_build(&mut self) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self.current_document()?.clone();
        if let Some(active_request) = self
            .active_jobs
            .values()
            .find(|request| {
                matches!(
                    request.slot(),
                    FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
                )
            })
            .cloned()
        {
            if equivalent_build_request(&active_request, &document) {
                self.record_job_reused(&active_request, "Equivalent build job already active.");
            } else {
                self.record_user_action_blocked(
                    &active_request,
                    "Build request blocked while another build is active.",
                );
            }
            return Ok(Vec::new());
        }

        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::CompileCurrent {
            job_id,
            document: Box::new(document),
        }))
    }

    /// Cancel visible preparation work and request a fresh current-document build.
    pub(crate) fn retry_preparation(
        &mut self,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        self.stale_all_active_jobs();
        self.clear_candidates();
        self.status = Some("Retrying preparation.".to_owned());
        self.request_build()
    }

    /// Request a current-preview render from the latest compiled output.
    pub(crate) fn request_preview(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let output = self
            .current_output
            .as_ref()
            .ok_or(FoundryAppStateError::MissingCurrentOutput)?
            .clone();
        if let Some(active_request) = self
            .active_jobs
            .values()
            .find(|request| request.slot() == FoundryJobSlot::RenderPreview)
            .cloned()
        {
            if equivalent_preview_request(&active_request, &output.build_stamp, width, height) {
                self.record_job_reused(&active_request, "Equivalent preview job already active.");
            } else {
                self.record_user_action_blocked(
                    &active_request,
                    "Preview request blocked while another preview is active.",
                );
            }
            return Ok(Vec::new());
        }
        if self.current_preview.as_ref().is_some_and(|preview| {
            preview.width == width
                && preview.height == height
                && preview.build == self.current_build
        }) {
            return Ok(Vec::new());
        }

        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::RenderPreview {
            job_id,
            output,
            width,
            height,
        }))
    }

    /// Request candidate generation with an explicit candidate request.
    pub(crate) fn request_candidates(
        &mut self,
        mut request: FoundryCandidateRequest,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self.current_document()?.clone();
        if request.preference_profile.is_none() {
            request.preference_profile = self.preference_profile_for_current_scope();
        }
        if let Some(active_request) = self
            .active_jobs
            .values()
            .find(|request| request.slot() == FoundryJobSlot::GenerateCandidates)
            .cloned()
        {
            if equivalent_candidate_request(&active_request, &document, &request) {
                self.record_job_reused(&active_request, "Equivalent candidate job already active.");
            } else {
                self.record_user_action_blocked(
                    &active_request,
                    "Candidate request blocked while another idea search is active.",
                );
            }
            return Ok(Vec::new());
        }
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::GenerateCandidates {
            job_id,
            document: Box::new(document),
            request,
        }))
    }

    /// Request candidate preview rendering and visual-legibility filtering.
    pub(crate) fn request_candidate_previews(
        &mut self,
        request: FoundryCandidateRequest,
        output: FoundryCandidateOutput,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self.current_document()?.clone();
        if let Some(active_request) = self
            .active_jobs
            .values()
            .find(|request| request.slot() == FoundryJobSlot::GenerateCandidates)
            .cloned()
        {
            if equivalent_candidate_request(&active_request, &document, &request) {
                self.record_job_reused(
                    &active_request,
                    "Equivalent candidate preview job already active.",
                );
            } else {
                self.record_user_action_blocked(
                    &active_request,
                    "Candidate preview request blocked while another idea job is active.",
                );
            }
            return Ok(Vec::new());
        }
        let job_id = self.allocate_job_id()?;
        Ok(
            self.schedule_job(FoundryJobRequest::RenderCandidatePreviews {
                job_id,
                document: Box::new(document),
                request,
                output: Box::new(output),
            }),
        )
    }

    /// Request a non-persistent preview for a sampled control value.
    pub(crate) fn preview_control_value(
        &mut self,
        control_id: String,
        value: ControlValue,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self.current_document()?.clone();
        if let Some(active_request) = self
            .active_jobs
            .values()
            .find(|request| request.slot() == FoundryJobSlot::RenderPreview)
            .cloned()
        {
            if equivalent_control_preview_request(&active_request, &document, &control_id, &value) {
                self.record_job_reused(
                    &active_request,
                    "Equivalent control preview job already active.",
                );
            } else {
                self.record_user_action_blocked(
                    &active_request,
                    "Control preview blocked while another preview is active.",
                );
            }
            return Ok(Vec::new());
        }
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::PreviewControlValue {
            job_id,
            document: Box::new(document),
            control_id,
            value,
            width: DEFAULT_PREVIEW_PIXELS,
            height: DEFAULT_PREVIEW_PIXELS,
        }))
    }

    /// Cancel active idea generation and return to the last stable Make state.
    pub(crate) fn cancel_idea_generation(
        &mut self,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let canceled_count = self.stale_active_jobs_in_slot(FoundryJobSlot::GenerateCandidates);
        self.clear_candidates();
        self.status = Some(if canceled_count > 0 {
            CANCELED_IDEA_SEARCH_STATUS.to_owned()
        } else {
            "No idea search is running.".to_owned()
        });
        Ok(Vec::new())
    }

    /// Replace state after a project load completes.
    pub(crate) fn replace_loaded_project(
        &mut self,
        project_file: FoundryProjectFile,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let next_job_id = self.next_job_id;
        let mut stale_jobs = self.stale_jobs.clone();
        stale_jobs.extend(self.active_jobs.keys().copied());
        let mut loaded = Self::from_project_file(project_file)?;
        loaded.next_job_id = next_job_id;
        loaded.stale_jobs = stale_jobs;
        *self = loaded;
        self.request_build()
    }

    /// Mark a save effect as completed by the host.
    pub(crate) fn mark_saved(&mut self, path: PathBuf) {
        if let Some(project_file) = &self.project_file {
            let mut clean = FoundryProjectFile::clean(project_file.project.clone(), Some(path));
            clean.load_report = project_file.load_report.clone();
            self.project_file = Some(clean);
        }
        self.refresh_project_flags();
    }

    fn handle_foundry_command(
        &mut self,
        command: FoundryCommand,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        match command {
            FoundryCommand::GenerateCandidates(request) => {
                self.request_candidates(candidate_request_from_command(request))
            }
            FoundryCommand::GenerateFocusedPartCandidates {
                group_id,
                channels,
                mode,
            } => self.request_candidates(focused_candidate_request_from_command(
                group_id, channels, mode,
            )),
            FoundryCommand::AcceptCandidate { candidate_id } => self.accept_candidate(candidate_id),
            FoundryCommand::RejectCandidate { candidate_id } => {
                self.reject_candidate(&candidate_id);
                Ok(Vec::new())
            }
            FoundryCommand::SetFocusPartGroup { .. }
            | FoundryCommand::ClearFocusPartGroup
            | FoundryCommand::ClearVariationFocus => self.apply_focus_scope_command(command),
            FoundryCommand::Undo => self.undo(),
            FoundryCommand::SwitchRevision { revision_id } => self.switch_revision(revision_id),
            FoundryCommand::Export { profile, out_dir } => self.export(profile, out_dir),
            FoundryCommand::AddCurrentToPack { pack_id, member_id } => {
                self.add_current_to_pack(pack_id, member_id)
            }
            command => {
                if let FoundryCommand::SetControl { control_id, .. } = &command {
                    self.active_control = Some(control_id.clone());
                }
                let preference_event = self.preference_event_for_command(&command);
                let effects = self.schedule_apply_edit(FoundryEdit {
                    label: foundry_command_label(&command).to_owned(),
                    commands: vec![command],
                })?;
                if let Some(event) = preference_event {
                    self.local_preferences.record(event);
                }
                Ok(effects)
            }
        }
    }

    fn apply_focus_scope_command(
        &mut self,
        command: FoundryCommand,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self
            .document
            .as_mut()
            .ok_or(FoundryAppStateError::MissingDocument)?;
        apply_foundry_command(document, &command)
            .map_err(|error| FoundryAppStateError::Project(format!("{error:?}")))?;
        self.clear_candidates();
        self.refresh_from_document_snapshot();
        self.refresh_project_flags();
        Ok(Vec::new())
    }

    fn schedule_apply_edit(
        &mut self,
        edit: FoundryEdit,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        if self.read_only {
            return Err(FoundryAppStateError::ReadOnly);
        }
        let document = self.current_document()?.clone();
        let job_id = self.allocate_job_id()?;
        self.stale_all_active_jobs();
        self.clear_candidates();
        Ok(self.schedule_job(FoundryJobRequest::ApplyEdit {
            job_id,
            document: Box::new(document),
            edit: Box::new(edit),
        }))
    }

    fn accept_candidate(
        &mut self,
        candidate_id: FoundryCandidateId,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let edit = self
            .candidate_edits
            .get(&candidate_id)
            .cloned()
            .ok_or_else(|| FoundryAppStateError::UnknownCandidate(candidate_id.clone()))?;
        let preference_event = self.candidate_acceptance_event(&candidate_id);
        let effects = self.schedule_apply_edit(edit)?;
        if let Some(event) = preference_event {
            self.local_preferences.record(event);
        }
        Ok(effects)
    }

    fn reject_candidate(&mut self, candidate_id: &FoundryCandidateId) {
        self.record_candidate_rejection(candidate_id);
        self.candidates
            .retain(|candidate| &candidate.id != candidate_id);
        self.candidate_edits.remove(candidate_id);
        if self.selected_candidate.as_ref() == Some(candidate_id) {
            self.selected_candidate = None;
        }
    }

    fn undo(&mut self) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let project_file = self
            .project_file
            .as_mut()
            .ok_or(FoundryAppStateError::NoParentRevision)?;
        project_file.undo().map_err(project_error)?;
        self.load_current_project_revision()?;
        self.stale_all_active_jobs();
        self.request_build()
    }

    fn switch_revision(
        &mut self,
        revision_id: RevisionId,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let project_file = self
            .project_file
            .as_mut()
            .ok_or(FoundryAppStateError::UnknownRevision(revision_id))?;
        project_file.switch_to(revision_id).map_err(project_error)?;
        self.load_current_project_revision()?;
        self.stale_all_active_jobs();
        self.request_build()
    }

    fn export(
        &mut self,
        profile: String,
        out_dir: Option<String>,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let out_dir = out_dir
            .map(PathBuf::from)
            .ok_or(FoundryAppStateError::MissingExportOutput)?;
        let output = self
            .current_output
            .as_ref()
            .ok_or(FoundryAppStateError::MissingCurrentOutput)?
            .clone();
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::Export {
            job_id,
            output,
            profile,
            out_dir,
        }))
    }

    fn add_current_to_pack(
        &mut self,
        pack_id: String,
        member_id: String,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let mut document = self.current_document()?.clone();
        document.document_id = FoundryDocumentId(member_id.clone());
        let mut pack = self
            .pack
            .pack
            .clone()
            .filter(|pack| pack.pack_id == pack_id)
            .unwrap_or_else(|| {
                FoundryPackDocument::new(
                    pack_id.clone(),
                    document.family_content_ref.clone(),
                    document.style_content_ref.clone(),
                    FoundryPackExportProfile {
                        profile: "default".to_owned(),
                        require_all_members: true,
                    },
                )
            });
        pack.members.insert(member_id.clone(), document);
        self.selected_pack_member = Some(member_id.clone());
        self.pack = pack_view_from_document(pack.clone(), Some(member_id));

        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::CompilePack {
            job_id,
            pack: Box::new(pack),
        }))
    }

    fn batch_export_current_pack(
        &mut self,
        out_dir: PathBuf,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let pack = self
            .pack
            .pack
            .clone()
            .ok_or(FoundryAppStateError::MissingPack)?;
        if !self.pack.can_export {
            return Err(FoundryAppStateError::PackNotExportable);
        }
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::ExportPack {
            job_id,
            pack: Box::new(pack),
            out_dir,
        }))
    }

    fn save(&self) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let project_file = self
            .project_file
            .as_ref()
            .ok_or(FoundryAppStateError::MissingDocument)?;
        let path = project_file
            .path
            .clone()
            .ok_or(FoundryAppStateError::MissingSavePath)?;
        Ok(vec![FoundryAppEffect::SaveProject {
            path,
            project: Box::new(project_file.project.clone()),
        }])
    }

    fn save_as(&self, path: PathBuf) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let project = self
            .project_file
            .as_ref()
            .map(|project_file| project_file.project.clone())
            .ok_or(FoundryAppStateError::MissingDocument)?;
        Ok(vec![FoundryAppEffect::SaveProject {
            path,
            project: Box::new(project),
        }])
    }

    fn load_current_project_revision(&mut self) -> Result<(), FoundryAppStateError> {
        let project_file = self
            .project_file
            .as_ref()
            .ok_or(FoundryAppStateError::MissingDocument)?;
        let revision = project_file.project.current().map_err(project_error)?;
        self.document = Some(revision.document.clone());
        self.current_build = revision.build_stamp.clone();
        self.recipe_snapshot = revision.recipe_snapshot.clone();
        self.current_output = None;
        self.current_preview = None;
        self.clear_candidates();
        self.refresh_from_document_snapshot();
        self.refresh_project_flags();
        Ok(())
    }

    fn apply_compilation_output(&mut self, output: FoundryCompilationOutput) {
        self.document = Some(output.document.clone());
        self.current_build = Some(output.build_stamp.clone());
        self.recipe_snapshot = Some(output.recipe_snapshot.clone());
        self.validation = Some(validate_foundry_document(&output.document));
        self.locks = output.document.foundry_locks.clone();
        self.controls = control_views_from_output(&output.document, &output);
        self.ensure_active_control_is_visible();
        self.current_output = Some(Box::new(output));
        self.refresh_project_flags();
    }

    fn apply_edit_output(&mut self, edit: FoundryEdit, output: FoundryCompilationOutput) {
        if let Err(error) = self.accept_project_edit(edit, &output) {
            self.status = Some(error.to_string());
        }
        self.apply_compilation_output(output);
        self.current_preview = None;
        self.clear_candidates();
    }

    fn accept_project_edit(
        &mut self,
        edit: FoundryEdit,
        output: &FoundryCompilationOutput,
    ) -> Result<(), FoundryAppStateError> {
        let catalog_lock = output.catalog.catalog_lock.clone();
        if let Some(project_file) = &mut self.project_file {
            project_file
                .accept_program(
                    FoundryProjectRevisionProgram::from_edit(edit),
                    output.document.clone(),
                    catalog_lock,
                    Some(output.build_stamp.clone()),
                    Some(output.recipe_snapshot.clone()),
                    output.conformance_summary.clone(),
                )
                .map_err(project_error)?;
        } else {
            self.project_file = Some(
                FoundryProjectFile::new(
                    output.document.document_id.0.clone(),
                    output.document.clone(),
                    catalog_lock,
                    Some(output.build_stamp.clone()),
                    Some(output.recipe_snapshot.clone()),
                    output.conformance_summary.clone(),
                )
                .map_err(project_error)?,
            );
        }
        self.refresh_project_flags();
        Ok(())
    }

    fn apply_candidates_generated(
        &mut self,
        request: FoundryCandidateRequest,
        output: FoundryCandidateOutput,
        cards: Vec<FoundryCandidateCard>,
    ) {
        self.candidate_edits = output
            .candidates
            .iter()
            .map(|candidate| (candidate.id.clone(), candidate.edit.clone()))
            .collect();
        let selected = self.selected_candidate.as_ref();
        self.candidates = if cards.is_empty() {
            candidate_cards_from_output(&output, Some(request.mode), selected)
        } else {
            cards
                .into_iter()
                .map(|mut card| {
                    card.selected = selected.is_some_and(|selected| selected == &card.id);
                    card
                })
                .collect()
        };
        self.selected_candidate =
            preferred_candidate_selection(&self.candidates, self.selected_candidate.as_ref());
        let selected = self.selected_candidate.clone();
        for card in &mut self.candidates {
            card.selected = selected
                .as_ref()
                .is_some_and(|selected| selected == &card.id);
        }
        self.candidate_output = Some(Box::new(output));
    }

    fn apply_candidate_previews(
        &mut self,
        cards: Vec<FoundryCandidateCard>,
        rejected_count: usize,
    ) {
        let selected = self.selected_candidate.clone();
        let visible_candidate_ids = cards
            .iter()
            .map(|card| card.id.clone())
            .collect::<BTreeSet<_>>();
        self.candidates = cards
            .into_iter()
            .map(|mut card| {
                card.selected = selected
                    .as_ref()
                    .is_some_and(|selected| selected == &card.id);
                card
            })
            .collect();
        self.candidate_edits
            .retain(|candidate_id, _| visible_candidate_ids.contains(candidate_id));
        self.selected_candidate =
            preferred_candidate_selection(&self.candidates, self.selected_candidate.as_ref());
        let selected = self.selected_candidate.clone();
        for card in &mut self.candidates {
            card.selected = selected
                .as_ref()
                .is_some_and(|selected| selected == &card.id);
        }
        if rejected_count > 0 {
            self.status = Some(format!(
                "Found {} clear ideas. Rejected {rejected_count} that looked too similar.",
                self.candidates.len()
            ));
        } else if !self.candidates.is_empty() {
            self.status = Some(format!("Found {} clear ideas.", self.candidates.len()));
        }
    }

    /// Set the app-relative trace clock used for subsequently recorded events.
    pub(crate) fn set_make_trace_elapsed_ms(&mut self, elapsed_ms: u64) {
        self.make_job_trace.set_elapsed_ms(elapsed_ms);
    }

    /// Write the local Make trace and latency summary JSON files.
    pub(crate) fn write_make_job_trace_outputs(&self, dir: &Path) -> std::io::Result<()> {
        self.make_job_trace.write_outputs(dir)
    }

    fn schedule_job(&mut self, request: FoundryJobRequest) -> Vec<FoundryAppEffect> {
        let canceled_count = self.stale_active_jobs_in_slot(request.slot());
        let job_id = request.job_id();
        self.active_jobs.insert(job_id, request.clone());
        self.record_job_queued_and_started(&request, canceled_count > 0);
        vec![FoundryAppEffect::StartJob(Box::new(request))]
    }

    fn stale_active_jobs_in_slot(&mut self, slot: FoundryJobSlot) -> usize {
        let stale = self
            .active_jobs
            .values()
            .filter_map(|request| (request.slot() == slot).then_some(request.clone()))
            .collect::<Vec<_>>();
        for request in &stale {
            let job_id = request.job_id();
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
            self.record_job_canceled(request, "Canceled previous job in the same slot.");
        }
        stale.len()
    }

    fn stale_all_active_jobs(&mut self) {
        let stale = self.active_jobs.values().cloned().collect::<Vec<_>>();
        self.stale_jobs
            .extend(stale.iter().map(FoundryJobRequest::job_id));
        self.active_jobs.clear();
        for request in stale {
            self.record_job_canceled(
                &request,
                "Canceled active job because user action invalidated it.",
            );
        }
    }

    fn stale_obsolete_active_jobs_except(&mut self, retained_job_id: u64) {
        let stale = self
            .active_jobs
            .iter()
            .filter_map(|(job_id, request)| {
                (*job_id != retained_job_id && !self.request_source_is_current(request))
                    .then_some(request.clone())
            })
            .collect::<Vec<_>>();
        for request in stale {
            let job_id = request.job_id();
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
            self.record_job_canceled(
                &request,
                "Canceled obsolete job after newer build finished.",
            );
        }
    }

    fn record_template_started(&mut self) {
        if let Some(document) = &self.document {
            self.make_job_trace
                .record(trace_event(MakeJobTraceEventInput {
                    elapsed_ms: self.make_job_trace.elapsed_ms(),
                    event_kind: MakeJobTraceEventKind::TemplateStarted,
                    job_id: None,
                    slot: None,
                    stamp: Some(document_trace_stamp(document)),
                    asset_name: Some(document_asset_name(document)),
                    trigger_action: Some("StartTemplate".to_owned()),
                    queue_depth: self.active_jobs.len(),
                    canceled_previous_job: false,
                    ignored_as_stale: false,
                    message: "Template started.".to_owned(),
                }));
        }
    }

    fn record_state_transition(&mut self, message: &str) {
        let (stamp, asset_name) = self
            .document
            .as_ref()
            .map(|document| {
                (
                    Some(document_trace_stamp(document)),
                    Some(document_asset_name(document)),
                )
            })
            .unwrap_or((None, None));
        self.make_job_trace
            .record(trace_event(MakeJobTraceEventInput {
                elapsed_ms: self.make_job_trace.elapsed_ms(),
                event_kind: MakeJobTraceEventKind::StateTransition,
                job_id: None,
                slot: None,
                stamp,
                asset_name,
                trigger_action: None,
                queue_depth: self.active_jobs.len(),
                canceled_previous_job: false,
                ignored_as_stale: false,
                message: message.to_owned(),
            }));
    }

    fn record_job_queued_and_started(
        &mut self,
        request: &FoundryJobRequest,
        canceled_previous_job: bool,
    ) {
        let queue_depth = self.active_jobs.len();
        let queued_kind = queued_kind_for_request(request);
        let started_kind = started_kind_for_request(request);
        self.make_job_trace.record(trace_event_for_request(
            self.make_job_trace.elapsed_ms(),
            queued_kind,
            request,
            queue_depth,
            canceled_previous_job,
            format!("{} queued.", trigger_action_for_request(request)),
        ));
        self.make_job_trace.record(trace_event_for_request(
            self.make_job_trace.elapsed_ms(),
            started_kind,
            request,
            queue_depth,
            canceled_previous_job,
            format!("{} started.", trigger_action_for_request(request)),
        ));
    }

    fn record_job_finished(
        &mut self,
        event_kind: MakeJobTraceEventKind,
        request: &FoundryJobRequest,
        stamp: Option<String>,
        asset_name: Option<String>,
        message: impl Into<String>,
    ) {
        self.make_job_trace
            .record(trace_event(MakeJobTraceEventInput {
                elapsed_ms: self.make_job_trace.elapsed_ms(),
                event_kind,
                job_id: Some(request.job_id()),
                slot: Some(request.slot()),
                stamp,
                asset_name,
                trigger_action: Some(trigger_action_for_request(request).to_owned()),
                queue_depth: self.active_jobs.len(),
                canceled_previous_job: false,
                ignored_as_stale: false,
                message: message.into(),
            }));
    }

    fn record_job_canceled(&mut self, request: &FoundryJobRequest, message: &str) {
        self.make_job_trace.record(trace_event_for_request(
            self.make_job_trace.elapsed_ms(),
            MakeJobTraceEventKind::JobCanceled,
            request,
            self.active_jobs.len(),
            true,
            message,
        ));
    }

    fn record_job_reused(&mut self, request: &FoundryJobRequest, message: &str) {
        self.make_job_trace.record(trace_event_for_request(
            self.make_job_trace.elapsed_ms(),
            MakeJobTraceEventKind::JobReused,
            request,
            self.active_jobs.len(),
            false,
            message,
        ));
    }

    fn record_user_action_blocked(&mut self, request: &FoundryJobRequest, message: &str) {
        self.make_job_trace.record(trace_event_for_request(
            self.make_job_trace.elapsed_ms(),
            MakeJobTraceEventKind::UserActionBlocked,
            request,
            self.active_jobs.len(),
            false,
            message,
        ));
    }

    fn record_stale_result(
        &mut self,
        job_id: u64,
        slot: Option<FoundryJobSlot>,
        stamp: Option<String>,
        asset_name: Option<String>,
        message: &str,
    ) {
        self.make_job_trace
            .record(trace_event(MakeJobTraceEventInput {
                elapsed_ms: self.make_job_trace.elapsed_ms(),
                event_kind: MakeJobTraceEventKind::JobIgnoredAsStale,
                job_id: Some(job_id),
                slot,
                stamp,
                asset_name,
                trigger_action: None,
                queue_depth: self.active_jobs.len(),
                canceled_previous_job: false,
                ignored_as_stale: true,
                message: message.to_owned(),
            }));
    }

    fn allocate_job_id(&mut self) -> Result<u64, FoundryAppStateError> {
        let job_id = self.next_job_id;
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(FoundryAppStateError::JobIdOverflow)?;
        Ok(job_id)
    }

    fn current_document(&self) -> Result<&FoundryAssetDocument, FoundryAppStateError> {
        self.document
            .as_ref()
            .ok_or(FoundryAppStateError::MissingDocument)
    }

    fn refresh_from_document_snapshot(&mut self) {
        if let Some(document) = &self.document {
            self.validation = Some(validate_foundry_document(document));
            self.locks = document.foundry_locks.clone();
            if self.current_build.is_none() {
                self.current_build = document.build_stamp.clone();
            }
        }
        self.ensure_active_control_is_visible();
    }

    fn refresh_project_flags(&mut self) {
        if let Some(project_file) = &self.project_file {
            self.current_revision = Some(project_file.project.current_revision);
            self.project_path = project_file.path.clone();
            self.load_report = Some(project_file.load_report.clone());
            self.read_only = project_file.load_report.read_only_recovery;
            self.dirty = project_file.is_dirty();
        } else {
            self.current_revision = None;
            self.project_path = None;
            self.dirty = false;
        }
    }

    fn ensure_active_control_is_visible(&mut self) {
        if self
            .active_control
            .as_ref()
            .is_some_and(|active| self.controls.iter().any(|control| &control.id == active))
        {
            return;
        }
        self.active_control = self
            .controls
            .iter()
            .find(|control| control.visible)
            .or_else(|| self.controls.first())
            .map(|control| control.id.clone());
    }

    fn select_candidate(&mut self, candidate: Option<FoundryCandidateId>) {
        self.selected_candidate = candidate;
        for card in &mut self.candidates {
            card.selected = self
                .selected_candidate
                .as_ref()
                .is_some_and(|selected| selected == &card.id);
        }
    }

    fn clear_candidates(&mut self) {
        self.selected_candidate = None;
        self.candidates.clear();
        self.candidate_output = None;
        self.candidate_edits.clear();
    }

    fn preference_scope(&self) -> Option<FoundryPreferenceScope> {
        let output = self.current_output.as_ref()?;
        let document = self.document.as_ref()?;
        Some(FoundryPreferenceScope::new(
            output.catalog.family.id.clone(),
            document.customizer_profile_ref.stable_id.clone(),
        ))
    }

    fn preference_profile_for_current_scope(&self) -> Option<FoundryPreferenceProfile> {
        let scope = self.preference_scope()?;
        let profile = self.local_preferences.profile_for_scope(scope);
        (!profile.is_empty()).then_some(profile)
    }

    fn candidate_acceptance_event(
        &self,
        candidate_id: &FoundryCandidateId,
    ) -> Option<FoundryPreferenceEvent> {
        let scope = self.preference_scope()?;
        let output = self.candidate_output.as_ref()?;
        let accepted = output
            .candidates
            .iter()
            .find(|candidate| candidate.id == *candidate_id)?;
        let rejected_candidate_ids = output
            .candidates
            .iter()
            .filter(|candidate| candidate.id != *candidate_id)
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>();
        let rejected_controls = output
            .candidates
            .iter()
            .filter(|candidate| candidate.id != *candidate_id)
            .flat_map(|candidate| candidate.changed_controls.iter().cloned())
            .collect::<Vec<_>>();
        Some(FoundryPreferenceEvent::CandidateComparison {
            scope,
            mode: None,
            accepted_candidate_id: candidate_id.clone(),
            accepted_controls: accepted.changed_controls.clone(),
            rejected_candidate_ids,
            rejected_controls,
            weight: 1.0,
        })
    }

    fn record_candidate_rejection(&mut self, candidate_id: &FoundryCandidateId) {
        let Some(scope) = self.preference_scope() else {
            return;
        };
        let Some(output) = &self.candidate_output else {
            return;
        };
        let Some(rejected) = output
            .candidates
            .iter()
            .find(|candidate| candidate.id == *candidate_id)
        else {
            return;
        };
        self.local_preferences
            .record(FoundryPreferenceEvent::CandidateRejected {
                scope,
                candidate_id: candidate_id.clone(),
                changed_controls: rejected.changed_controls.clone(),
                weight: 1.0,
            });
    }

    fn preference_event_for_command(
        &self,
        command: &FoundryCommand,
    ) -> Option<FoundryPreferenceEvent> {
        let scope = self.preference_scope()?;
        match command {
            FoundryCommand::ResetControl { control_id } => {
                let control_id = self.visible_control_id(control_id)?;
                Some(FoundryPreferenceEvent::ControlReset {
                    scope,
                    control_id,
                    weight: 1.0,
                })
            }
            FoundryCommand::SetLock { lock }
                if matches!(
                    lock.mode,
                    FoundryLockMode::Locked | FoundryLockMode::SearchProtected
                ) =>
            {
                let control_id = self.visible_control_id_for_lock_target(&lock.target)?;
                Some(FoundryPreferenceEvent::ControlLocked {
                    scope,
                    control_id,
                    weight: 1.0,
                })
            }
            _ => None,
        }
    }

    fn record_current_variant_exported(&mut self) {
        let Some(scope) = self.preference_scope() else {
            return;
        };
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let changed_controls = self.visible_changed_controls_from_document(document);
        self.local_preferences
            .record(FoundryPreferenceEvent::VariantExported {
                scope,
                changed_controls,
                weight: 1.0,
            });
    }

    fn record_current_pack_member_added(&mut self) {
        let Some(scope) = self.preference_scope() else {
            return;
        };
        if self.selected_pack_member.is_none() {
            return;
        }
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let changed_controls = self.visible_changed_controls_from_document(document);
        self.local_preferences
            .record(FoundryPreferenceEvent::PackMemberAdded {
                scope,
                changed_controls,
                weight: 1.0,
            });
    }

    fn visible_changed_controls_from_document(
        &self,
        document: &FoundryAssetDocument,
    ) -> Vec<String> {
        let mut controls = document
            .control_state
            .keys()
            .filter_map(|control_id| self.visible_control_id(control_id))
            .collect::<Vec<_>>();
        controls.sort();
        controls.dedup();
        controls
    }

    fn visible_control_id_for_lock_target(&self, target: &FoundryLockTarget) -> Option<String> {
        match target {
            FoundryLockTarget::Control(control_id) => self.visible_control_id(control_id),
            FoundryLockTarget::Provider(role) => self.visible_provider_control_id_for_role(role),
            _ => None,
        }
    }

    fn visible_control_id(&self, control_id: &str) -> Option<String> {
        self.controls
            .iter()
            .find(|control| control.visible && control.id == control_id)
            .map(|control| control.id.clone())
    }

    fn visible_provider_control_id_for_role(&self, role: &str) -> Option<String> {
        self.controls
            .iter()
            .find(|control| {
                control.visible
                    && control.presentation == FoundryControlPresentation::ProviderGallery
                    && control
                        .options
                        .iter()
                        .any(|option| option.provider_role.as_deref() == Some(role))
            })
            .map(|control| control.id.clone())
    }

    fn request_source_is_current(&self, request: &FoundryJobRequest) -> bool {
        match request {
            FoundryJobRequest::CompileCurrent { document, .. }
            | FoundryJobRequest::GenerateCandidates { document, .. }
            | FoundryJobRequest::RenderCandidatePreviews { document, .. }
            | FoundryJobRequest::PreviewControlValue { document, .. }
            | FoundryJobRequest::ApplyEdit { document, .. } => self
                .document
                .as_ref()
                .is_some_and(|current| document_sources_match(document, current)),
            FoundryJobRequest::RenderPreview { output, .. }
            | FoundryJobRequest::Export { output, .. } => self
                .current_output
                .as_ref()
                .is_some_and(|current| current.build_stamp == output.build_stamp),
            FoundryJobRequest::CompilePack { pack, .. }
            | FoundryJobRequest::ExportPack { pack, .. } => self
                .pack
                .pack
                .as_ref()
                .is_some_and(|current| current == pack.as_ref()),
        }
    }
}

fn preferred_candidate_selection(
    candidates: &[FoundryCandidateCard],
    current: Option<&FoundryCandidateId>,
) -> Option<FoundryCandidateId> {
    if let Some(current) = current
        && candidates
            .iter()
            .any(|card| &card.id == current && card.selectable)
    {
        return Some(current.clone());
    }
    candidates
        .iter()
        .find(|card| card.selectable)
        .or_else(|| current.and_then(|current| candidates.iter().find(|card| &card.id == current)))
        .or_else(|| candidates.first())
        .map(|card| card.id.clone())
}

/// State transition errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FoundryAppStateError {
    /// No current document is open.
    MissingDocument,
    /// No compiled output is available for preview/export.
    MissingCurrentOutput,
    /// Saving requires a path.
    MissingSavePath,
    /// Export requires a destination directory.
    MissingExportOutput,
    /// No family pack is open.
    MissingPack,
    /// Current family pack is not ready for batch export.
    PackNotExportable,
    /// Candidate ID was not generated in the current state.
    UnknownCandidate(FoundryCandidateId),
    /// Revision ID is not present in the current history.
    UnknownRevision(RevisionId),
    /// Current revision has no parent.
    NoParentRevision,
    /// Read-only recovery state cannot be edited.
    ReadOnly,
    /// Job ID allocation overflowed.
    JobIdOverflow,
    /// Initial document failed validation.
    InvalidDocument(usize),
    /// Project-level operation failed.
    Project(String),
}

impl fmt::Display for FoundryAppStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDocument => formatter.write_str("foundry document is required"),
            Self::MissingCurrentOutput => formatter.write_str("current foundry output is required"),
            Self::MissingSavePath => formatter.write_str("foundry save requires a file path"),
            Self::MissingExportOutput => {
                formatter.write_str("foundry export requires an output directory")
            }
            Self::MissingPack => formatter.write_str("foundry pack is required"),
            Self::PackNotExportable => {
                formatter.write_str("foundry pack is not ready for batch export")
            }
            Self::UnknownCandidate(candidate) => {
                write!(formatter, "unknown foundry candidate {:?}", candidate)
            }
            Self::UnknownRevision(revision) => {
                write!(formatter, "unknown foundry revision {:?}", revision)
            }
            Self::NoParentRevision => formatter.write_str("current foundry revision has no parent"),
            Self::ReadOnly => formatter.write_str("foundry project is read-only"),
            Self::JobIdOverflow => formatter.write_str("foundry job id overflow"),
            Self::InvalidDocument(issue_count) => {
                write!(
                    formatter,
                    "foundry document is invalid with {issue_count} issue(s)"
                )
            }
            Self::Project(message) => write!(formatter, "foundry project error: {message}"),
        }
    }
}

impl Error for FoundryAppStateError {}

fn project_error(error: impl fmt::Display) -> FoundryAppStateError {
    FoundryAppStateError::Project(error.to_string())
}

fn job_event_matches_request(event: &FoundryJobEvent, request: &FoundryJobRequest) -> bool {
    matches!(
        (event, request),
        (
            FoundryJobEvent::CompileFinished { .. },
            FoundryJobRequest::CompileCurrent { .. }
        ) | (
            FoundryJobEvent::PreviewRendered { .. },
            FoundryJobRequest::RenderPreview { .. }
        ) | (
            FoundryJobEvent::PreviewRendered { .. },
            FoundryJobRequest::PreviewControlValue { .. }
        ) | (
            FoundryJobEvent::CandidatesGenerated { .. },
            FoundryJobRequest::GenerateCandidates { .. },
        ) | (
            FoundryJobEvent::CandidatePreviewsRendered { .. },
            FoundryJobRequest::RenderCandidatePreviews { .. },
        ) | (
            FoundryJobEvent::EditApplied { .. },
            FoundryJobRequest::ApplyEdit { .. }
        ) | (
            FoundryJobEvent::PackCompiled { .. },
            FoundryJobRequest::CompilePack { .. }
        ) | (
            FoundryJobEvent::PackExportFinished { .. },
            FoundryJobRequest::ExportPack { .. }
        ) | (
            FoundryJobEvent::ExportFinished { .. },
            FoundryJobRequest::Export { .. }
        ) | (FoundryJobEvent::Failed { .. }, _)
    )
}

fn document_sources_match(left: &FoundryAssetDocument, right: &FoundryAssetDocument) -> bool {
    left.schema_version == right.schema_version
        && left.document_id == right.document_id
        && left.family_content_ref == right.family_content_ref
        && left.style_content_ref == right.style_content_ref
        && left.family_implementation_ref == right.family_implementation_ref
        && left.style_implementation_ref == right.style_implementation_ref
        && left.customizer_profile_ref == right.customizer_profile_ref
        && left.control_state == right.control_state
        && left.provider_overrides == right.provider_overrides
        && left.foundry_locks == right.foundry_locks
        && left.local_recipe_overrides == right.local_recipe_overrides
        && left.seed == right.seed
}

fn equivalent_build_request(
    active_request: &FoundryJobRequest,
    document: &FoundryAssetDocument,
) -> bool {
    match active_request {
        FoundryJobRequest::CompileCurrent {
            document: active_document,
            ..
        } => document_sources_match(active_document, document),
        _ => false,
    }
}

fn equivalent_preview_request(
    active_request: &FoundryJobRequest,
    build: &FoundryBuildStamp,
    width: u32,
    height: u32,
) -> bool {
    match active_request {
        FoundryJobRequest::RenderPreview {
            output,
            width: active_width,
            height: active_height,
            ..
        } => output.build_stamp == *build && *active_width == width && *active_height == height,
        _ => false,
    }
}

fn equivalent_control_preview_request(
    active_request: &FoundryJobRequest,
    document: &FoundryAssetDocument,
    control_id: &str,
    value: &ControlValue,
) -> bool {
    match active_request {
        FoundryJobRequest::PreviewControlValue {
            document: active_document,
            control_id: active_control_id,
            value: active_value,
            ..
        } => {
            document_sources_match(active_document, document)
                && active_control_id == control_id
                && active_value == value
        }
        _ => false,
    }
}

fn equivalent_candidate_request(
    active_request: &FoundryJobRequest,
    document: &FoundryAssetDocument,
    request: &FoundryCandidateRequest,
) -> bool {
    match active_request {
        FoundryJobRequest::GenerateCandidates {
            document: active_document,
            request: active_request,
            ..
        }
        | FoundryJobRequest::RenderCandidatePreviews {
            document: active_document,
            request: active_request,
            ..
        } => document_sources_match(active_document, document) && active_request == request,
        _ => false,
    }
}

fn failed_kind_for_request(request: &FoundryJobRequest) -> MakeJobTraceEventKind {
    match request {
        FoundryJobRequest::CompileCurrent { .. } | FoundryJobRequest::ApplyEdit { .. } => {
            MakeJobTraceEventKind::BuildFailed
        }
        FoundryJobRequest::RenderPreview { .. } | FoundryJobRequest::PreviewControlValue { .. } => {
            MakeJobTraceEventKind::PreviewFailed
        }
        FoundryJobRequest::GenerateCandidates { .. }
        | FoundryJobRequest::RenderCandidatePreviews { .. } => {
            MakeJobTraceEventKind::CandidateFailed
        }
        FoundryJobRequest::CompilePack { .. }
        | FoundryJobRequest::ExportPack { .. }
        | FoundryJobRequest::Export { .. } => MakeJobTraceEventKind::StateTransition,
    }
}

fn candidate_request_from_command(request: GenerateCandidatesRequest) -> FoundryCandidateRequest {
    let result_count = request.count.max(1) as usize;
    FoundryCandidateRequest {
        seed: request.seed,
        proposal_count: (result_count * 2).clamp(8, 72),
        result_count,
        mode: FoundryCandidateMode::Refine,
        strategy_id: request.strategy_id,
        preference_profile: None,
        variation_intent: request.variation_intent.normalized(),
    }
}

fn focused_candidate_request_from_command(
    group_id: String,
    channels: Vec<shape_foundry::VariationChannel>,
    mode: String,
) -> FoundryCandidateRequest {
    let display_name = group_id
        .split(['_', '-', '.'])
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = first.to_uppercase().collect::<String>();
                    label.push_str(&chars.as_str().to_ascii_lowercase());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let channels = if channels.is_empty() {
        vec![shape_foundry::VariationChannel::Shape]
    } else {
        channels
    };
    FoundryCandidateRequest {
        seed: 0,
        proposal_count: 24,
        result_count: 6,
        mode: focused_candidate_mode(&mode),
        strategy_id: None,
        preference_profile: None,
        variation_intent: shape_foundry::VariationIntent {
            scope: shape_foundry::VariationScope::SemanticPartGroup {
                group_id,
                display_name: display_name.clone(),
            },
            channels,
            human_label: "Focused part".to_owned(),
            human_summary: format!("Vary the {display_name} part group."),
        }
        .normalized(),
    }
}

fn focused_candidate_mode(mode: &str) -> FoundryCandidateMode {
    match mode.trim().to_ascii_lowercase().as_str() {
        "explore" => FoundryCandidateMode::Explore,
        "silhouette" => FoundryCandidateMode::Silhouette,
        "structure" => FoundryCandidateMode::Structure,
        "detail" => FoundryCandidateMode::Detail,
        _ => FoundryCandidateMode::Refine,
    }
}

fn control_views_from_output(
    document: &FoundryAssetDocument,
    output: &FoundryCompilationOutput,
) -> Vec<FoundryControlView> {
    let profile = &output.catalog.customizer_profile;
    let sections = profile
        .sections
        .iter()
        .map(|section| (section.id.as_str(), section.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    let option_previews = option_previews_from_output(document, output, context);
    profile
        .controls
        .iter()
        .map(|control| {
            let default_value = default_control_value(control, context).ok();
            let value = current_control_value(document, control, default_value.clone());
            let domain = effective_control_domain(control, context).unwrap_or_default();
            let locked_reason = control_locked_reason(document, &control.id, &control.kind);
            FoundryControlView {
                id: control.id.clone(),
                label: control.label.clone(),
                section: control
                    .section
                    .as_deref()
                    .and_then(|section| sections.get(section).copied())
                    .map(str::to_owned),
                kind: control_kind_label(&control.kind).to_owned(),
                presentation: control_presentation(&control.kind),
                value: value.clone(),
                default_value,
                primary: control.primary,
                visible: control.visible,
                locked: locked_reason.is_some(),
                locked_reason,
                topology_behavior: control.topology_behavior,
                divergence: control_divergence(control, document),
                options: option_cards_for_control(
                    control,
                    &domain,
                    value.as_ref(),
                    &option_previews,
                ),
                numeric_range: numeric_range_for_domain(&domain),
                advanced_path: Some(format!("controls.{}", control.id)),
                help: None,
            }
        })
        .collect()
}

fn numeric_range_for_domain(domain: &FeasibleControlDomain) -> Option<FoundryNumericRange> {
    domain
        .continuous_intervals
        .first()
        .map(|interval| FoundryNumericRange {
            minimum: interval.minimum,
            maximum: interval.maximum,
            step: 0.01_f32.max((interval.maximum - interval.minimum) / 100.0),
        })
}

fn control_presentation(kind: &ControlKind) -> FoundryControlPresentation {
    match kind {
        ControlKind::ContinuousAxis { .. } => FoundryControlPresentation::ContinuousMacroAxis,
        ControlKind::IntegerStepper { .. } => FoundryControlPresentation::Stepper,
        ControlKind::Toggle { .. } => FoundryControlPresentation::Toggle,
        ControlKind::ChoiceGallery { .. } => FoundryControlPresentation::ChoiceGallery,
        ControlKind::ProviderGallery { .. } => FoundryControlPresentation::ProviderGallery,
    }
}

fn current_control_value(
    document: &FoundryAssetDocument,
    control: &shape_foundry::CustomizerControl,
    default_value: Option<ControlValue>,
) -> Option<ControlValue> {
    if let ControlKind::ProviderGallery { role, .. } = &control.kind
        && let Some(ProviderOverride { provider_ref, .. }) = document.provider_overrides.get(role)
    {
        return Some(ControlValue::Provider(provider_ref.stable_id.clone()));
    }
    document
        .control_state
        .get(&control.id)
        .cloned()
        .or(default_value)
}

fn option_cards_for_control(
    control: &shape_foundry::CustomizerControl,
    domain: &shape_foundry::FeasibleControlDomain,
    current: Option<&ControlValue>,
    previews: &BTreeMap<String, OptionPreviewImage>,
) -> Vec<FoundryOptionCard> {
    let context = OptionCardContext {
        domain,
        current,
        previews,
    };
    match &control.kind {
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| {
                let value = ControlValue::Choice(option.value.clone());
                option_card(
                    &control.id,
                    value,
                    option.label.clone(),
                    None,
                    Some(option.preview.preview_id.clone()),
                    context,
                )
            })
            .collect(),
        ControlKind::ProviderGallery { role, options } => options
            .iter()
            .map(|option| {
                let value = ControlValue::Provider(option.provider_id.clone());
                option_card(
                    &control.id,
                    value,
                    option.label.clone(),
                    Some(role.clone()),
                    Some(option.preview.preview_id.clone()),
                    context,
                )
            })
            .collect(),
        _ => domain
            .discrete_values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                option_card(
                    &control.id,
                    value.clone(),
                    control_value_label(value),
                    None,
                    Some(format!("{}-option-{index}", control.id)),
                    context,
                )
            })
            .collect(),
    }
}

fn option_card(
    control_id: &str,
    value: ControlValue,
    label: String,
    provider_role: Option<String>,
    preview_id: Option<String>,
    context: OptionCardContext<'_>,
) -> FoundryOptionCard {
    let preview = preview_id
        .as_ref()
        .and_then(|preview_id| context.previews.get(preview_id));
    FoundryOptionCard {
        control_id: control_id.to_owned(),
        selected: context.current == Some(&value),
        unavailable_reason: context.domain.unavailable_reason(&value).map(str::to_owned),
        value,
        label,
        provider_role,
        preview_id,
        rgba8: preview.map_or_else(Vec::new, |preview| preview.rgba8.clone()),
        width: preview.map_or(0, |preview| preview.width),
        height: preview.map_or(0, |preview| preview.height),
        camera: preview.map(|preview| preview.camera.clone()),
    }
}

#[derive(Debug, Copy, Clone)]
struct OptionCardContext<'a> {
    domain: &'a shape_foundry::FeasibleControlDomain,
    current: Option<&'a ControlValue>,
    previews: &'a BTreeMap<String, OptionPreviewImage>,
}

fn option_previews_from_output(
    document: &FoundryAssetDocument,
    output: &FoundryCompilationOutput,
    context: ControlEvaluationContext<'_>,
) -> BTreeMap<String, OptionPreviewImage> {
    let resolver = OutputCatalogResolver::from_output(output);
    let mut previews = BTreeMap::new();
    for control in &output.catalog.customizer_profile.controls {
        if !control.primary || !control.visible {
            continue;
        }
        let domain = effective_control_domain(control, context).unwrap_or_default();
        for (value, preview_id) in option_preview_values(control, &domain) {
            if previews.contains_key(&preview_id) {
                continue;
            }
            if let Some(preview) =
                option_preview_for_control_value(document, &resolver, &control.id, value)
            {
                previews.insert(preview_id, preview);
            }
        }
    }
    previews
}

fn option_preview_values(
    control: &shape_foundry::CustomizerControl,
    domain: &shape_foundry::FeasibleControlDomain,
) -> Vec<(ControlValue, String)> {
    match &control.kind {
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| {
                (
                    ControlValue::Choice(option.value.clone()),
                    option.preview.preview_id.clone(),
                )
            })
            .collect(),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| {
                (
                    ControlValue::Provider(option.provider_id.clone()),
                    option.preview.preview_id.clone(),
                )
            })
            .collect(),
        ControlKind::ContinuousAxis { .. }
        | ControlKind::IntegerStepper { .. }
        | ControlKind::Toggle { .. } => domain
            .discrete_values
            .iter()
            .enumerate()
            .map(|(index, value)| (value.clone(), format!("{}-option-{index}", control.id)))
            .collect(),
    }
}

fn option_preview_for_control_value(
    document: &FoundryAssetDocument,
    resolver: &OutputCatalogResolver,
    control_id: &str,
    value: ControlValue,
) -> Option<OptionPreviewImage> {
    let mut preview_document = document.clone();
    apply_foundry_command(
        &mut preview_document,
        &FoundryCommand::SetControl {
            control_id: control_id.to_owned(),
            value,
        },
    )
    .ok()?;
    let output = compile_foundry_document(&preview_document, resolver).ok()?;
    render_option_preview_from_output(&output)
}

#[derive(Debug, Clone)]
struct OptionPreviewImage {
    rgba8: Vec<u8>,
    width: u32,
    height: u32,
    camera: shape_render::OrbitCamera,
}

fn render_option_preview_from_output(
    output: &FoundryCompilationOutput,
) -> Option<OptionPreviewImage> {
    let mesh = &output.artifact.combined_preview.mesh;
    let mesh = TriangleMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: mesh.bounds.min.into(),
            max: mesh.bounds.max.into(),
        },
    };
    let camera = fit_camera_to_bounds(mesh.bounds);
    let settings = clay_readability_render_settings(64, 64);
    let image = render_mesh(&mesh, &camera, &settings).ok()?;
    Some(OptionPreviewImage {
        rgba8: image.rgba8,
        width: image.width,
        height: image.height,
        camera,
    })
}

#[derive(Debug, Clone)]
struct OutputCatalogResolver {
    entries: BTreeMap<String, String>,
}

impl OutputCatalogResolver {
    fn from_output(output: &FoundryCompilationOutput) -> Self {
        Self {
            entries: output
                .catalog
                .resolved_content
                .values()
                .map(|content| {
                    (
                        content.content_ref.stable_id.clone(),
                        content.canonical_json.clone(),
                    )
                })
                .collect(),
        }
    }
}

impl FoundryCatalogResolver for OutputCatalogResolver {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.entries
            .get(&content_ref.stable_id)
            .cloned()
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

fn control_locked_reason(
    document: &FoundryAssetDocument,
    control_id: &str,
    kind: &ControlKind,
) -> Option<String> {
    document
        .foundry_locks
        .iter()
        .find(|lock| {
            lock.mode == FoundryLockMode::Locked
                && match (&lock.target, kind) {
                    (FoundryLockTarget::Control(locked), _) => locked == control_id,
                    (
                        FoundryLockTarget::Provider(locked),
                        ControlKind::ProviderGallery { role, .. },
                    )
                    | (
                        FoundryLockTarget::Role(locked),
                        ControlKind::ProviderGallery { role, .. },
                    ) => locked == role,
                    _ => false,
                }
        })
        .map(|lock| {
            lock.reason
                .as_deref()
                .filter(|reason| !reason.trim().is_empty())
                .unwrap_or("Control is locked.")
                .to_owned()
        })
}

fn control_kind_label(kind: &ControlKind) -> &'static str {
    match kind {
        ControlKind::ContinuousAxis { .. } => "Scalar",
        ControlKind::IntegerStepper { .. } => "Integer",
        ControlKind::Toggle { .. } => "Toggle",
        ControlKind::ChoiceGallery { .. } => "Choice",
        ControlKind::ProviderGallery { .. } => "Provider",
    }
}

fn control_value_label(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => format!("{value:.2}"),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => {
            if *value {
                "On".to_owned()
            } else {
                "Off".to_owned()
            }
        }
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn pack_view_from_document(
    pack: FoundryPackDocument,
    selected_member: Option<String>,
) -> FoundryPackView {
    let shared_provider_choices = match &pack.shared_provider_policy {
        SharedProviderPolicy::Independent => BTreeMap::new(),
        SharedProviderPolicy::SharedExact(providers) => providers
            .iter()
            .map(|(role, provider_ref)| (role.clone(), provider_ref.stable_id.clone()))
            .collect(),
    };
    FoundryPackView {
        pack_id: Some(pack.pack_id.clone()),
        members: pack
            .members
            .iter()
            .map(|(member_id, document)| (member_id.clone(), document.document_id.clone()))
            .collect(),
        selected_member,
        shared_locks: pack.shared_locks.clone(),
        shared_provider_choices,
        member_override_counts: BTreeMap::new(),
        coherence_warnings: Vec::new(),
        coherent: false,
        can_export: false,
        pack: Some(pack),
    }
}

fn foundry_command_label(command: &FoundryCommand) -> &'static str {
    match command {
        FoundryCommand::SetControl { .. } => "Set control",
        FoundryCommand::ResetControl { .. } => "Reset control",
        FoundryCommand::SelectProvider { .. } => "Select provider",
        FoundryCommand::SetRolePresence { .. } => "Set role presence",
        FoundryCommand::SetStyle { .. } => "Set style",
        FoundryCommand::SetLock { .. } => "Set lock",
        FoundryCommand::ClearLock { .. } => "Clear lock",
        FoundryCommand::SetVariationIntent { .. } => "Set variation intent",
        FoundryCommand::SetVariationScope { .. } => "Set variation scope",
        FoundryCommand::SetVariationChannels { .. } => "Set variation channels",
        FoundryCommand::ClearVariationFocus => "Clear variation focus",
        FoundryCommand::ClearFocusPartGroup => "Clear focus part",
        FoundryCommand::SetFocusPartGroup { .. } => "Set focus part",
        FoundryCommand::GenerateFocusedPartCandidates { .. } => "Generate focused candidates",
        FoundryCommand::GenerateCandidates(_) => "Generate candidates",
        FoundryCommand::AcceptCandidate { .. } => "Accept candidate",
        FoundryCommand::RejectCandidate { .. } => "Reject candidate",
        FoundryCommand::Undo => "Undo",
        FoundryCommand::SwitchRevision { .. } => "Switch revision",
        FoundryCommand::Export { .. } => "Export",
        FoundryCommand::AddCurrentToPack { .. } => "Add current to pack",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_candidate_selection_keeps_current_selectable_direction() {
        let cards = vec![
            test_candidate_card("candidate-a", true),
            test_candidate_card("candidate-b", true),
        ];

        assert_eq!(
            preferred_candidate_selection(&cards, Some(&cards[1].id)),
            Some(cards[1].id.clone())
        );
    }

    #[test]
    fn preferred_candidate_selection_skips_unselectable_current_direction() {
        let cards = vec![
            test_candidate_card("candidate-a", false),
            test_candidate_card("candidate-b", true),
            test_candidate_card("candidate-c", true),
        ];

        assert_eq!(
            preferred_candidate_selection(&cards, Some(&cards[0].id)),
            Some(cards[1].id.clone())
        );
    }

    #[test]
    fn preferred_candidate_selection_keeps_pending_direction_when_none_are_selectable() {
        let cards = vec![
            test_candidate_card("candidate-a", false),
            test_candidate_card("candidate-b", false),
        ];

        assert_eq!(
            preferred_candidate_selection(&cards, Some(&cards[1].id)),
            Some(cards[1].id.clone())
        );
    }

    fn test_candidate_card(id: &str, selectable: bool) -> FoundryCandidateCard {
        FoundryCandidateCard {
            id: FoundryCandidateId(id.to_owned()),
            slot: 0,
            mode: Some(FoundryCandidateMode::Explore),
            parent: false,
            title: "Direction".to_owned(),
            subtitle: "Whole asset".to_owned(),
            preview_id: Some(format!("{id}-preview")),
            rgba8: Vec::new(),
            width: 0,
            height: 0,
            camera: Some(shape_render::OrbitCamera::default()),
            preview_failure: None,
            changed_controls: Vec::new(),
            changed_roles: Vec::new(),
            explanations: Vec::new(),
            rejections: BTreeMap::new(),
            validation_label: "Ready".to_owned(),
            validation_detail: None,
            selectable,
            selected: false,
            variation_intent_label: "Direction".to_owned(),
            variation_scope_label: "Whole asset".to_owned(),
            variation_channel_labels: Vec::new(),
            visible_delta_label: "Clear".to_owned(),
            what_changed_summary: "Visible whole-asset change.".to_owned(),
            legibility_class: shape_foundry::CandidateLegibilityClass::Clear,
            focus_part_label: None,
            surface_unavailable_reason: None,
        }
    }
}
