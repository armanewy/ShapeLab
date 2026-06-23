//! State reducer for the native Foundry surface.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use shape_asset::RevisionId;
use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlValue, FoundryAssetDocument, FoundryBuildStamp,
    FoundryCandidateId, FoundryCatalogLock, FoundryCommand, FoundryCompilationOutput,
    FoundryConformanceSummary, FoundryEdit, FoundryLock, FoundryLockMode, FoundryLockTarget,
    FoundryPackDocument, FoundryPackExportProfile, FoundryProjectRevisionProgram,
    FoundryValidationReport, GenerateCandidatesRequest, GeneratedRecipeSnapshot, ProviderOverride,
    SharedProviderPolicy, control_divergence, default_control_value, effective_control_domain,
    validate_foundry_document,
};
use shape_project::foundry::{FoundryProjectFile, FoundryProjectLoadReport};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRequest,
};

use super::commands::{FoundryAppCommand, FoundryAppEffect};
use super::jobs::{
    FoundryJobEvent, FoundryJobRequest, FoundryJobSlot, candidate_cards_from_output,
};
use super::view_model::{
    FoundryCandidateCard, FoundryControlView, FoundryOptionCard, FoundryPackView,
};

const FIRST_JOB_ID: u64 = 1;
const DEFAULT_PREVIEW_PIXELS: u32 = 128;

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
            current_preview: None,
            pack: FoundryPackView::default(),
            active_jobs: BTreeMap::new(),
            stale_jobs: BTreeSet::new(),
            dirty: false,
            read_only: false,
            advanced_recipe_open: false,
            status: None,
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
            FoundryAppCommand::RequestBuild => self.request_build(),
            FoundryAppCommand::RequestPreview => {
                self.request_preview(DEFAULT_PREVIEW_PIXELS, DEFAULT_PREVIEW_PIXELS)
            }
            FoundryAppCommand::Save => self.save(),
            FoundryAppCommand::SaveAs(path) => self.save_as(path),
            FoundryAppCommand::Load(path) => Ok(vec![FoundryAppEffect::LoadProject(path)]),
            FoundryAppCommand::SetAdvancedRecipeOpen(open) => {
                self.advanced_recipe_open = open;
                Ok(Vec::new())
            }
        }
    }

    /// Apply a background job event. Returns true when the event affected state.
    pub(crate) fn handle_job_event(&mut self, event: FoundryJobEvent) -> bool {
        let job_id = event.job_id();
        let Some(request) = self.active_jobs.get(&job_id).cloned() else {
            self.stale_jobs.insert(job_id);
            return false;
        };
        if !self.request_source_is_current(&request) {
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
            return false;
        }
        if !job_event_matches_request(&event, &request) {
            self.stale_jobs.insert(job_id);
            return false;
        }

        let accepted = match event {
            FoundryJobEvent::CompileFinished { output, .. } => {
                self.apply_compilation_output(*output);
                true
            }
            FoundryJobEvent::PreviewRendered {
                preview_id,
                rgba8,
                width,
                height,
                camera,
                ..
            } => {
                self.current_preview = Some(FoundryPreviewImage {
                    preview_id,
                    rgba8,
                    width,
                    height,
                    camera,
                    build: self.current_build.clone(),
                });
                true
            }
            FoundryJobEvent::CandidatesGenerated {
                request,
                output,
                cards,
                ..
            } => {
                self.apply_candidates_generated(request, *output, cards);
                true
            }
            FoundryJobEvent::EditApplied { edit, output, .. } => {
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
                true
            }
            FoundryJobEvent::ExportFinished {
                profile, out_dir, ..
            } => {
                self.status = Some(format!("Exported {profile} to {}", out_dir.display()));
                true
            }
            FoundryJobEvent::Failed { message, .. } => {
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
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::CompileCurrent {
            job_id,
            document: Box::new(document),
        }))
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
        request: FoundryCandidateRequest,
    ) -> Result<Vec<FoundryAppEffect>, FoundryAppStateError> {
        let document = self.current_document()?.clone();
        let job_id = self.allocate_job_id()?;
        Ok(self.schedule_job(FoundryJobRequest::GenerateCandidates {
            job_id,
            document: Box::new(document),
            request,
        }))
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
            FoundryCommand::AcceptCandidate { candidate_id } => self.accept_candidate(candidate_id),
            FoundryCommand::RejectCandidate { candidate_id } => {
                self.reject_candidate(&candidate_id);
                Ok(Vec::new())
            }
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
                self.schedule_apply_edit(FoundryEdit {
                    label: foundry_command_label(&command).to_owned(),
                    commands: vec![command],
                })
            }
        }
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
        self.selected_candidate = Some(candidate_id);
        self.schedule_apply_edit(edit)
    }

    fn reject_candidate(&mut self, candidate_id: &FoundryCandidateId) {
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
        let document = self.current_document()?.clone();
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
        self.candidate_output = Some(Box::new(output));
    }

    fn schedule_job(&mut self, request: FoundryJobRequest) -> Vec<FoundryAppEffect> {
        self.stale_active_jobs_in_slot(request.slot());
        let job_id = request.job_id();
        self.active_jobs.insert(job_id, request.clone());
        vec![FoundryAppEffect::StartJob(Box::new(request))]
    }

    fn stale_active_jobs_in_slot(&mut self, slot: FoundryJobSlot) {
        let stale = self
            .active_jobs
            .iter()
            .filter_map(|(job_id, request)| (request.slot() == slot).then_some(*job_id))
            .collect::<Vec<_>>();
        for job_id in stale {
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
        }
    }

    fn stale_all_active_jobs(&mut self) {
        self.stale_jobs.extend(self.active_jobs.keys().copied());
        self.active_jobs.clear();
    }

    fn stale_obsolete_active_jobs_except(&mut self, retained_job_id: u64) {
        let stale = self
            .active_jobs
            .iter()
            .filter_map(|(job_id, request)| {
                (*job_id != retained_job_id && !self.request_source_is_current(request))
                    .then_some(*job_id)
            })
            .collect::<Vec<_>>();
        for job_id in stale {
            self.active_jobs.remove(&job_id);
            self.stale_jobs.insert(job_id);
        }
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

    fn request_source_is_current(&self, request: &FoundryJobRequest) -> bool {
        match request {
            FoundryJobRequest::CompileCurrent { document, .. }
            | FoundryJobRequest::GenerateCandidates { document, .. }
            | FoundryJobRequest::ApplyEdit { document, .. } => self
                .document
                .as_ref()
                .is_some_and(|current| document_sources_match(document, current)),
            FoundryJobRequest::RenderPreview { output, .. }
            | FoundryJobRequest::Export { output, .. } => self
                .current_output
                .as_ref()
                .is_some_and(|current| current.build_stamp == output.build_stamp),
            FoundryJobRequest::CompilePack { pack, .. } => self
                .pack
                .pack
                .as_ref()
                .is_some_and(|current| current == pack.as_ref()),
        }
    }
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
            FoundryJobEvent::CandidatesGenerated { .. },
            FoundryJobRequest::GenerateCandidates { .. },
        ) | (
            FoundryJobEvent::EditApplied { .. },
            FoundryJobRequest::ApplyEdit { .. }
        ) | (
            FoundryJobEvent::PackCompiled { .. },
            FoundryJobRequest::CompilePack { .. }
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

fn candidate_request_from_command(request: GenerateCandidatesRequest) -> FoundryCandidateRequest {
    let result_count = request.count.max(1) as usize;
    FoundryCandidateRequest {
        seed: request.seed,
        proposal_count: (result_count * 8).clamp(24, 72),
        result_count,
        mode: FoundryCandidateMode::Refine,
        strategy_id: request.strategy_id,
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
    profile
        .controls
        .iter()
        .map(|control| {
            let default_value = default_control_value(control, context).ok();
            let value = current_control_value(document, control, default_value.clone());
            let domain = effective_control_domain(control, context).unwrap_or_default();
            FoundryControlView {
                id: control.id.clone(),
                label: control.label.clone(),
                section: control
                    .section
                    .as_deref()
                    .and_then(|section| sections.get(section).copied())
                    .map(str::to_owned),
                kind: control_kind_label(&control.kind).to_owned(),
                value: value.clone(),
                default_value,
                primary: control.primary,
                visible: control.visible,
                locked: control_is_locked(document, &control.id, &control.kind),
                topology_behavior: control.topology_behavior,
                divergence: control_divergence(control, document),
                options: option_cards_for_control(control, &domain, value.as_ref()),
                advanced_path: Some(format!("controls.{}", control.id)),
                help: None,
            }
        })
        .collect()
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
) -> Vec<FoundryOptionCard> {
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
                    domain,
                    current,
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
                    domain,
                    current,
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
                    domain,
                    current,
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
    domain: &shape_foundry::FeasibleControlDomain,
    current: Option<&ControlValue>,
) -> FoundryOptionCard {
    FoundryOptionCard {
        control_id: control_id.to_owned(),
        selected: current == Some(&value),
        unavailable_reason: domain.unavailable_reason(&value).map(str::to_owned),
        value,
        label,
        provider_role,
        preview_id,
        rgba8: Vec::new(),
        width: 0,
        height: 0,
        camera: None,
    }
}

fn control_is_locked(
    document: &FoundryAssetDocument,
    control_id: &str,
    kind: &ControlKind,
) -> bool {
    document.foundry_locks.iter().any(|lock| {
        lock.mode == FoundryLockMode::Locked
            && match (&lock.target, kind) {
                (FoundryLockTarget::Control(locked), _) => locked == control_id,
                (
                    FoundryLockTarget::Provider(locked),
                    ControlKind::ProviderGallery { role, .. },
                )
                | (FoundryLockTarget::Role(locked), ControlKind::ProviderGallery { role, .. }) => {
                    locked == role
                }
                _ => false,
            }
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
        FoundryCommand::GenerateCandidates(_) => "Generate candidates",
        FoundryCommand::AcceptCandidate { .. } => "Accept candidate",
        FoundryCommand::RejectCandidate { .. } => "Reject candidate",
        FoundryCommand::Undo => "Undo",
        FoundryCommand::SwitchRevision { .. } => "Switch revision",
        FoundryCommand::Export { .. } => "Export",
        FoundryCommand::AddCurrentToPack { .. } => "Add current to pack",
    }
}
