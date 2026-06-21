//! Reducer-style explicit asset app state.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetEdit, AssetEditProgram, AssetRecipe, AssetValidationIssue, ParameterId, PartDefinitionId,
    PartInstance, PartInstanceId, RevisionId, apply_edit_program_with_report,
    validate_asset_recipe,
};
use shape_compile::{AssetArtifact, ConstructionTimelineReport};
use shape_render::{OrbitCamera, RenderSettings, fit_camera_to_bounds};

use super::commands::{AssetAppCommand, AssetAppEffect, AssetLockTarget, AssetTemplate};
use super::jobs::{
    AssetCandidate, AssetCandidateId, AssetCandidatePreview, AssetCompileOutput, AssetGenerationId,
    AssetGenerationMode, AssetJobEvent, AssetJobId, AssetJobKind, AssetJobRequest, AssetJobSlot,
    AssetOutputPolicy, AssetPreview, preview_mesh_from_artifact,
};

const FIRST_JOB_ID: u64 = 1;
const FIRST_GENERATION_ID: u64 = 1;
const FIRST_REVISION_ID: u64 = 1;
const FIRST_CHILD_REVISION_ID: u64 = 2;
const DEFAULT_CANDIDATE_COUNT: usize = 6;
const ASSET_APP_PROJECT_KIND: &str = "shape-lab.asset-modeling-lab";
const ASSET_APP_PROJECT_SCHEMA_VERSION: u32 = 1;

/// UI-facing validation issue independent of compile and recipe issue sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetAppIssue {
    pub subject: Option<String>,
    pub code: String,
    pub message: String,
}

impl From<AssetValidationIssue> for AssetAppIssue {
    fn from(issue: AssetValidationIssue) -> Self {
        Self {
            subject: issue.subject,
            code: issue.code,
            message: issue.message,
        }
    }
}

/// Snapshot of recipe lock sets reflected into app state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AssetLockSnapshot {
    pub parameters: BTreeSet<ParameterId>,
    pub instances: BTreeSet<PartInstanceId>,
    pub subtrees: BTreeSet<PartInstanceId>,
    pub topology: BTreeSet<PartDefinitionId>,
}

impl AssetLockSnapshot {
    fn from_recipe(recipe: &AssetRecipe) -> Self {
        Self {
            parameters: recipe.locks.clone(),
            instances: recipe.instance_locks.clone(),
            subtrees: recipe.subtree_locks.clone(),
            topology: recipe.topology_locks.clone(),
        }
    }
}

/// One visible candidate slot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCandidateSlot {
    pub slot: usize,
    pub candidate: AssetCandidate,
    pub preview: Option<AssetCandidatePreview>,
    pub preview_failure: Option<String>,
}

/// Current or latest generation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetGenerationState {
    pub generation_id: AssetGenerationId,
    pub mode: AssetGenerationMode,
    pub job_id: AssetJobId,
}

/// One app-level recipe revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AssetRevision {
    pub id: RevisionId,
    pub parent: Option<RevisionId>,
    pub label: String,
    pub recipe: AssetRecipe,
}

/// Branchable app-level revision history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AssetRevisionHistory {
    pub current: RevisionId,
    pub revisions: BTreeMap<RevisionId, AssetRevision>,
    next_revision: u64,
}

/// Branch-preserving project file used by Asset Modeling Lab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AssetModelingProject {
    pub project_kind: String,
    pub schema_version: u32,
    pub title: String,
    pub current_file_path_hint: Option<PathBuf>,
    pub revision_history: AssetRevisionHistory,
}

impl AssetRevisionHistory {
    fn new(recipe: AssetRecipe) -> Self {
        let current = RevisionId(FIRST_REVISION_ID);
        let mut revisions = BTreeMap::new();
        revisions.insert(
            current,
            AssetRevision {
                id: current,
                parent: None,
                label: "Initial asset".to_owned(),
                recipe,
            },
        );
        Self {
            current,
            revisions,
            next_revision: FIRST_CHILD_REVISION_ID,
        }
    }

    fn push_child(&mut self, label: impl Into<String>, recipe: AssetRecipe) -> RevisionId {
        let id = RevisionId(self.next_revision);
        self.next_revision = self.next_revision.saturating_add(1);
        self.revisions.insert(
            id,
            AssetRevision {
                id,
                parent: Some(self.current),
                label: label.into(),
                recipe,
            },
        );
        self.current = id;
        id
    }

    fn parent_of_current(&self) -> Option<RevisionId> {
        self.revisions
            .get(&self.current)
            .and_then(|revision| revision.parent)
    }

    fn recipe(&self, revision: RevisionId) -> Option<AssetRecipe> {
        self.revisions
            .get(&revision)
            .map(|revision| revision.recipe.clone())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ActiveAssetJob {
    pub job_id: AssetJobId,
    pub recipe_revision: RevisionId,
    pub generation_id: Option<AssetGenerationId>,
}

/// UI-independent explicit asset app state.
#[derive(Debug, Clone)]
pub(crate) struct AssetAppState {
    pub recipe: AssetRecipe,
    pub selected_part_instance: Option<PartInstanceId>,
    pub selected_parameter: Option<ParameterId>,
    pub locks: AssetLockSnapshot,
    pub current_artifact: Option<AssetArtifact>,
    pub current_timeline: Option<ConstructionTimelineReport>,
    pub current_preview: Option<AssetPreview>,
    pub current_camera: OrbitCamera,
    pub candidate_slots: Vec<AssetCandidateSlot>,
    pub active_generation: Option<AssetGenerationState>,
    pub revision_history: AssetRevisionHistory,
    pub current_file_path: Option<PathBuf>,
    pub dirty: bool,
    pub validation_issues: Vec<AssetAppIssue>,
    pub current_template: Option<AssetTemplate>,
    pub active_jobs: BTreeMap<AssetJobSlot, ActiveAssetJob>,
    pub stale_jobs: BTreeSet<AssetJobId>,
    next_job_id: u64,
    next_generation_id: u64,
}

impl AssetAppState {
    /// Create state from a full recipe snapshot.
    pub(crate) fn new(recipe: AssetRecipe) -> Result<Self, AssetAppStateError> {
        ensure_valid_recipe(&recipe)?;
        let selected_part_instance = first_part(&recipe);
        let selected_parameter = first_parameter(&recipe);
        let locks = AssetLockSnapshot::from_recipe(&recipe);
        let validation_issues = recipe_issues(&recipe);
        Ok(Self {
            revision_history: AssetRevisionHistory::new(recipe.clone()),
            recipe,
            selected_part_instance,
            selected_parameter,
            locks,
            current_artifact: None,
            current_timeline: None,
            current_preview: None,
            current_camera: OrbitCamera::default(),
            candidate_slots: Vec::new(),
            active_generation: None,
            current_file_path: None,
            dirty: false,
            validation_issues,
            current_template: None,
            active_jobs: BTreeMap::new(),
            stale_jobs: BTreeSet::new(),
            next_job_id: FIRST_JOB_ID,
            next_generation_id: FIRST_GENERATION_ID,
        })
    }

    /// Create state from a template and keep the template as reset metadata.
    pub(crate) fn from_template(template: AssetTemplate) -> Result<Self, AssetAppStateError> {
        let mut state = Self::new(template.recipe.clone())?;
        state.current_template = Some(template);
        Ok(state)
    }

    /// Apply one command and return any deferred side effects.
    pub(crate) fn handle_command(
        &mut self,
        command: AssetAppCommand,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        match command {
            AssetAppCommand::SelectPart(instance) => {
                self.select_part(instance)?;
                Ok(Vec::new())
            }
            AssetAppCommand::SelectParameter(parameter) => {
                self.select_parameter(parameter)?;
                Ok(Vec::new())
            }
            AssetAppCommand::SetParameter { parameter, value } => {
                self.set_parameter(parameter, value)
            }
            AssetAppCommand::SetTransform {
                instance,
                transform,
            } => self.apply_edit(
                "Set transform",
                vec![AssetEdit::SetTransform {
                    instance,
                    transform,
                }],
                true,
            ),
            AssetAppCommand::SetLock { target, locked } => self.set_lock(target, locked),
            AssetAppCommand::AddOptionalPart { instance } => self.add_optional_part(instance),
            AssetAppCommand::RemoveOptionalPart(instance) => self.apply_edit(
                "Remove optional part",
                vec![AssetEdit::SetOptionalPartEnabled {
                    instance,
                    enabled: false,
                }],
                true,
            ),
            AssetAppCommand::ReplaceCompatiblePart {
                instance,
                definition,
            } => self.apply_edit(
                "Replace compatible part",
                vec![AssetEdit::ReplaceInstanceDefinition {
                    instance,
                    definition,
                }],
                true,
            ),
            AssetAppCommand::ReplaceDefinition(definition) => self.apply_edit(
                "Replace definition",
                vec![AssetEdit::ReplaceDefinition { definition }],
                true,
            ),
            AssetAppCommand::ToggleOptionalPart { instance, enabled } => self.apply_edit(
                "Toggle optional part",
                vec![AssetEdit::SetOptionalPartEnabled { instance, enabled }],
                true,
            ),
            AssetAppCommand::GenerateRefine => self.start_generation(AssetGenerationMode::Refine),
            AssetAppCommand::GenerateExplore => self.start_generation(AssetGenerationMode::Explore),
            AssetAppCommand::AcceptCandidate(candidate) => self.accept_candidate(candidate),
            AssetAppCommand::RejectCandidate(_) => Ok(Vec::new()),
            AssetAppCommand::Undo => self.undo(),
            AssetAppCommand::SwitchBranch(revision) => self.switch_revision(revision, true),
            AssetAppCommand::LoadTemplate(template) => self.load_template(template),
            AssetAppCommand::Save => self.save(),
            AssetAppCommand::SaveAs(path) => Ok(vec![AssetAppEffect::SaveProject {
                path,
                project: Box::new(self.project_snapshot()),
            }]),
            AssetAppCommand::Load(path) => Ok(vec![AssetAppEffect::LoadProject(path)]),
            AssetAppCommand::ExportObj(path) => self.export_obj(path),
            AssetAppCommand::ExportPackage(path) => self.export_package(path),
            AssetAppCommand::FitCamera => {
                self.fit_camera();
                Ok(Vec::new())
            }
            AssetAppCommand::SetWireframe(_) => Ok(Vec::new()),
        }
    }

    /// Apply a background job event. Returns true when the event affected state.
    pub(crate) fn handle_job_event(&mut self, event: AssetJobEvent) -> bool {
        match event {
            AssetJobEvent::Queued { job_id, slot }
            | AssetJobEvent::Started { job_id, slot }
            | AssetJobEvent::Progress { job_id, slot, .. } => {
                self.is_current_job(slot, job_id, None, None)
            }
            AssetJobEvent::CompileReady {
                job_id,
                recipe_revision,
                output,
            } => self.apply_compile_ready(job_id, recipe_revision, *output),
            AssetJobEvent::PreviewReady {
                job_id,
                recipe_revision,
                preview,
            } => self.apply_preview_ready(job_id, recipe_revision, preview),
            AssetJobEvent::CandidatesReady {
                job_id,
                generation_id,
                recipe_revision,
                candidates,
            } => self.apply_candidates_ready(job_id, generation_id, recipe_revision, candidates),
            AssetJobEvent::CandidatePreviewsReady {
                job_id,
                generation_id,
                recipe_revision,
                previews,
                failures,
            } => self.apply_candidate_previews_ready(
                job_id,
                generation_id,
                recipe_revision,
                previews,
                failures,
            ),
            AssetJobEvent::ExportPackageReady {
                job_id,
                recipe_revision,
                ..
            } => {
                if !self.is_current_job(
                    AssetJobSlot::ExportPackage,
                    job_id,
                    Some(recipe_revision),
                    None,
                ) {
                    return false;
                }
                self.finish_job(AssetJobSlot::ExportPackage, job_id);
                true
            }
            AssetJobEvent::ExportObjReady {
                job_id,
                recipe_revision,
                ..
            } => {
                if !self.is_current_job(
                    AssetJobSlot::ExportObj,
                    job_id,
                    Some(recipe_revision),
                    None,
                ) {
                    return false;
                }
                self.finish_job(AssetJobSlot::ExportObj, job_id);
                true
            }
            AssetJobEvent::Failed { job_id, message } => self.apply_job_failure(job_id, message),
            AssetJobEvent::Cancelled { job_id } => self.apply_job_cancelled(job_id),
        }
    }

    /// Replace state after a UI-side load effect completes.
    pub(crate) fn replace_loaded_recipe(
        &mut self,
        recipe: AssetRecipe,
        path: PathBuf,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        ensure_valid_recipe(&recipe)?;
        self.recipe = recipe.clone();
        self.revision_history = AssetRevisionHistory::new(recipe);
        self.current_file_path = Some(path);
        self.current_template = None;
        self.dirty = false;
        self.refresh_after_recipe_replacement();
        self.schedule_compile_current()
    }

    /// Replace state with a branch-preserving project snapshot.
    pub(crate) fn replace_loaded_project(
        &mut self,
        project: AssetModelingProject,
        path: PathBuf,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        ensure_valid_project_snapshot(&project)?;
        let recipe = project
            .revision_history
            .recipe(project.revision_history.current)
            .ok_or(AssetAppStateError::InvalidProject(
                "current revision is missing".to_owned(),
            ))?;
        self.recipe = recipe;
        self.revision_history = project.revision_history;
        self.current_file_path = Some(path);
        self.current_template = None;
        self.dirty = false;
        self.refresh_after_recipe_replacement();
        self.schedule_compile_current()
    }

    /// Mark a save effect as successfully completed.
    pub(crate) fn mark_saved(&mut self, path: PathBuf) {
        self.current_file_path = Some(path);
        self.dirty = false;
    }

    /// Request a render of the current compiled preview.
    pub(crate) fn request_render_current_preview(
        &mut self,
        render_settings: RenderSettings,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        self.schedule_job(AssetJobKind::RenderCurrentPreview {
            camera: self
                .current_preview
                .as_ref()
                .map(|_| self.current_camera.clone()),
            render_settings,
        })
    }

    /// Request compilation of the current recipe.
    pub(crate) fn request_compile_current(
        &mut self,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        self.schedule_compile_current()
    }

    /// Request compilation of candidate previews for the latest generation.
    pub(crate) fn request_candidate_previews(
        &mut self,
        render_settings: RenderSettings,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let Some(generation_id) = self
            .active_generation
            .as_ref()
            .map(|generation| generation.generation_id)
        else {
            return Ok(Vec::new());
        };
        let candidates = self
            .candidate_slots
            .iter()
            .map(|slot| slot.candidate.clone())
            .collect::<Vec<_>>();
        let effects = self.schedule_job(AssetJobKind::CompileCandidatePreviews {
            generation_id,
            candidates,
            camera: self.current_camera.clone(),
            render_settings,
        })?;
        if let Some(active) = self
            .active_jobs
            .get(&AssetJobSlot::CompileCandidatePreviews)
            .copied()
            && let Some(generation) = &mut self.active_generation
        {
            generation.job_id = active.job_id;
        }
        Ok(effects)
    }

    fn select_part(&mut self, instance: Option<PartInstanceId>) -> Result<(), AssetAppStateError> {
        if let Some(instance) = instance
            && !self.recipe.instances.contains_key(&instance)
        {
            return Err(AssetAppStateError::UnknownPartInstance(instance));
        }
        self.selected_part_instance = instance;
        Ok(())
    }

    fn select_parameter(
        &mut self,
        parameter: Option<ParameterId>,
    ) -> Result<(), AssetAppStateError> {
        if let Some(parameter) = parameter
            && !self.recipe.parameters.contains_key(&parameter)
        {
            return Err(AssetAppStateError::UnknownParameter(parameter));
        }
        self.selected_parameter = parameter;
        Ok(())
    }

    fn set_parameter(
        &mut self,
        parameter: ParameterId,
        value: f32,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        if !value.is_finite() {
            return Err(AssetAppStateError::NonFiniteParameter(parameter));
        }
        if !self.recipe.parameters.contains_key(&parameter) {
            return Err(AssetAppStateError::UnknownParameter(parameter));
        }
        if self.recipe.locks.contains(&parameter) {
            return Err(AssetAppStateError::LockedParameter(parameter));
        }
        self.selected_parameter = Some(parameter);
        self.apply_edit(
            "Set parameter",
            vec![AssetEdit::SetScalar { parameter, value }],
            true,
        )
    }

    fn set_lock(
        &mut self,
        target: AssetLockTarget,
        locked: bool,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let edit = match target {
            AssetLockTarget::Parameter(parameter) => AssetEdit::SetLock { parameter, locked },
            AssetLockTarget::Instance(instance) => AssetEdit::SetInstanceLock { instance, locked },
            AssetLockTarget::Subtree(instance) => AssetEdit::SetSubtreeLock { instance, locked },
            AssetLockTarget::Topology(definition) => {
                AssetEdit::SetTopologyLock { definition, locked }
            }
        };
        self.apply_edit("Set lock", vec![edit], true)
    }

    fn add_optional_part(
        &mut self,
        instance: PartInstance,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let instance_id = instance.id;
        let outcome = self.edited_recipe(
            "Add optional part",
            vec![AssetEdit::AddInstance { instance }],
        )?;
        let mut recipe = outcome;
        recipe.variation.optional_instances.insert(instance_id);
        ensure_valid_recipe(&recipe)?;
        self.commit_recipe("Add optional part", recipe, true)?;
        self.selected_part_instance = Some(instance_id);
        self.schedule_compile_current()
    }

    fn accept_candidate(
        &mut self,
        candidate: AssetCandidateId,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let candidate = self
            .candidate_slots
            .iter()
            .find(|slot| slot.candidate.id == candidate)
            .map(|slot| slot.candidate.clone())
            .ok_or(AssetAppStateError::UnknownCandidate(candidate))?;
        self.commit_recipe("Accept candidate", candidate.recipe, true)?;
        self.candidate_slots.clear();
        self.active_generation = None;
        self.schedule_compile_current()
    }

    fn undo(&mut self) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let parent = self
            .revision_history
            .parent_of_current()
            .ok_or(AssetAppStateError::NoParentRevision)?;
        self.switch_revision(parent, true)
    }

    fn switch_revision(
        &mut self,
        revision: RevisionId,
        dirty: bool,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let recipe = self
            .revision_history
            .recipe(revision)
            .ok_or(AssetAppStateError::UnknownRevision(revision))?;
        self.revision_history.current = revision;
        self.recipe = recipe;
        self.dirty = dirty;
        self.refresh_after_recipe_replacement();
        self.schedule_compile_current()
    }

    fn load_template(
        &mut self,
        template: AssetTemplate,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        ensure_valid_recipe(&template.recipe)?;
        self.recipe = template.recipe.clone();
        self.revision_history = AssetRevisionHistory::new(template.recipe.clone());
        self.current_template = Some(template);
        self.current_file_path = None;
        self.dirty = false;
        self.current_camera = OrbitCamera::default();
        self.refresh_after_recipe_replacement();
        self.schedule_compile_current()
    }

    fn save(&self) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let path = self
            .current_file_path
            .clone()
            .ok_or(AssetAppStateError::MissingSavePath)?;
        Ok(vec![AssetAppEffect::SaveProject {
            path,
            project: Box::new(self.project_snapshot()),
        }])
    }

    fn export_obj(&mut self, path: PathBuf) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        self.schedule_job(AssetJobKind::ExportObj { path })
    }

    fn export_package(&mut self, path: PathBuf) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        self.schedule_job(AssetJobKind::ExportPackage { path })
    }

    /// Build a serializable, branch-preserving project snapshot.
    pub(crate) fn project_snapshot(&self) -> AssetModelingProject {
        AssetModelingProject {
            project_kind: ASSET_APP_PROJECT_KIND.to_owned(),
            schema_version: ASSET_APP_PROJECT_SCHEMA_VERSION,
            title: self.recipe.title.clone(),
            current_file_path_hint: self.current_file_path.clone(),
            revision_history: self.revision_history.clone(),
        }
    }

    fn fit_camera(&mut self) {
        if let Some(artifact) = &self.current_artifact {
            let mesh = preview_mesh_from_artifact(artifact);
            self.current_camera = fit_camera_to_bounds(mesh.bounds);
        } else {
            self.current_camera = OrbitCamera::default();
        }
    }

    fn apply_edit(
        &mut self,
        label: &'static str,
        operations: Vec<AssetEdit>,
        dirty: bool,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let recipe = self.edited_recipe(label, operations)?;
        self.commit_recipe(label, recipe, dirty)?;
        self.schedule_compile_current()
    }

    fn edited_recipe(
        &self,
        label: &'static str,
        operations: Vec<AssetEdit>,
    ) -> Result<AssetRecipe, AssetAppStateError> {
        let program = AssetEditProgram {
            label: label.to_owned(),
            seed: self.revision_history.current.0,
            operations,
        };
        apply_edit_program_with_report(&self.recipe, &program)
            .map(|outcome| outcome.recipe)
            .map_err(|rejection| {
                let message = rejection
                    .report
                    .entries
                    .last()
                    .map(|entry| entry.message.clone())
                    .unwrap_or_else(|| "asset edit was rejected".to_owned());
                AssetAppStateError::EditRejected(message)
            })
    }

    fn commit_recipe(
        &mut self,
        label: impl Into<String>,
        recipe: AssetRecipe,
        dirty: bool,
    ) -> Result<RevisionId, AssetAppStateError> {
        ensure_valid_recipe(&recipe)?;
        let revision = self.revision_history.push_child(label, recipe.clone());
        self.recipe = recipe;
        self.dirty = dirty;
        self.refresh_after_recipe_replacement();
        Ok(revision)
    }

    fn refresh_after_recipe_replacement(&mut self) {
        self.mark_active_jobs_stale();
        self.locks = AssetLockSnapshot::from_recipe(&self.recipe);
        self.validation_issues = recipe_issues(&self.recipe);
        self.current_artifact = None;
        self.current_timeline = None;
        self.candidate_slots.clear();
        self.active_generation = None;
        self.reconcile_selection();
    }

    fn reconcile_selection(&mut self) {
        if self
            .selected_part_instance
            .is_some_and(|selected| !self.recipe.instances.contains_key(&selected))
        {
            self.selected_part_instance = first_part(&self.recipe);
        }
        if self
            .selected_parameter
            .is_some_and(|selected| !self.recipe.parameters.contains_key(&selected))
        {
            self.selected_parameter = first_parameter(&self.recipe);
        }
    }

    fn schedule_compile_current(&mut self) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        self.schedule_job(AssetJobKind::CompileCurrentAsset)
    }

    fn start_generation(
        &mut self,
        mode: AssetGenerationMode,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let generation_id = self.allocate_generation_id()?;
        self.candidate_slots.clear();
        self.active_generation = Some(AssetGenerationState {
            generation_id,
            mode,
            job_id: AssetJobId(0),
        });
        let effects = self.schedule_job(AssetJobKind::GenerateCandidates {
            generation_id,
            mode,
            max_candidates: DEFAULT_CANDIDATE_COUNT,
        })?;
        if let Some(active) = self
            .active_jobs
            .get(&AssetJobSlot::GenerateCandidates)
            .copied()
            && let Some(generation) = &mut self.active_generation
        {
            generation.job_id = active.job_id;
        }
        Ok(effects)
    }

    fn schedule_job(
        &mut self,
        kind: AssetJobKind,
    ) -> Result<Vec<AssetAppEffect>, AssetAppStateError> {
        let job_id = self.allocate_job_id()?;
        let slot = kind.slot();
        if let Some(old) = self.active_jobs.insert(
            slot,
            ActiveAssetJob {
                job_id,
                recipe_revision: self.revision_history.current,
                generation_id: kind.generation_id(),
            },
        ) {
            self.stale_jobs.insert(old.job_id);
        }
        Ok(vec![AssetAppEffect::StartJob(Box::new(AssetJobRequest {
            job_id,
            recipe_revision: self.revision_history.current,
            kind,
            recipe: self.recipe.clone(),
            validation_limits: None,
            output_policy: AssetOutputPolicy::PreviewAndPackage,
        }))])
    }

    fn allocate_job_id(&mut self) -> Result<AssetJobId, AssetAppStateError> {
        let id = AssetJobId(self.next_job_id);
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(AssetAppStateError::JobIdOverflow)?;
        Ok(id)
    }

    fn allocate_generation_id(&mut self) -> Result<AssetGenerationId, AssetAppStateError> {
        let id = AssetGenerationId(self.next_generation_id);
        self.next_generation_id = self
            .next_generation_id
            .checked_add(1)
            .ok_or(AssetAppStateError::GenerationIdOverflow)?;
        Ok(id)
    }

    fn mark_active_jobs_stale(&mut self) {
        self.stale_jobs
            .extend(self.active_jobs.values().map(|job| job.job_id));
        self.active_jobs.clear();
    }

    fn is_current_job(
        &mut self,
        slot: AssetJobSlot,
        job_id: AssetJobId,
        recipe_revision: Option<RevisionId>,
        generation_id: Option<AssetGenerationId>,
    ) -> bool {
        let Some(active) = self.active_jobs.get(&slot).copied() else {
            self.stale_jobs.insert(job_id);
            return false;
        };
        if active.job_id != job_id {
            self.stale_jobs.insert(job_id);
            return false;
        }
        if recipe_revision.is_some_and(|revision| {
            revision != active.recipe_revision || revision != self.revision_history.current
        }) {
            self.stale_jobs.insert(job_id);
            return false;
        }
        if generation_id.is_some_and(|generation| active.generation_id != Some(generation)) {
            self.stale_jobs.insert(job_id);
            return false;
        }
        true
    }

    fn finish_job(&mut self, slot: AssetJobSlot, job_id: AssetJobId) {
        if self
            .active_jobs
            .get(&slot)
            .is_some_and(|active| active.job_id == job_id)
        {
            self.active_jobs.remove(&slot);
        }
    }

    fn apply_compile_ready(
        &mut self,
        job_id: AssetJobId,
        recipe_revision: RevisionId,
        output: AssetCompileOutput,
    ) -> bool {
        if !self.is_current_job(
            AssetJobSlot::CompileCurrentAsset,
            job_id,
            Some(recipe_revision),
            None,
        ) {
            return false;
        }
        self.current_artifact = Some(output.artifact);
        self.current_timeline = Some(output.timeline);
        self.validation_issues.clear();
        self.finish_job(AssetJobSlot::CompileCurrentAsset, job_id);
        true
    }

    fn apply_preview_ready(
        &mut self,
        job_id: AssetJobId,
        recipe_revision: RevisionId,
        preview: AssetPreview,
    ) -> bool {
        if !self.is_current_job(
            AssetJobSlot::RenderCurrentPreview,
            job_id,
            Some(recipe_revision),
            None,
        ) {
            return false;
        }
        self.current_camera = preview.camera.clone();
        self.current_preview = Some(preview);
        self.finish_job(AssetJobSlot::RenderCurrentPreview, job_id);
        true
    }

    fn apply_candidates_ready(
        &mut self,
        job_id: AssetJobId,
        generation_id: AssetGenerationId,
        recipe_revision: RevisionId,
        candidates: Vec<AssetCandidate>,
    ) -> bool {
        if !self.is_current_job(
            AssetJobSlot::GenerateCandidates,
            job_id,
            Some(recipe_revision),
            Some(generation_id),
        ) {
            return false;
        }
        self.candidate_slots = candidates
            .into_iter()
            .map(|candidate| AssetCandidateSlot {
                slot: candidate.slot,
                candidate,
                preview: None,
                preview_failure: None,
            })
            .collect();
        self.candidate_slots.sort_by_key(|slot| slot.slot);
        self.finish_job(AssetJobSlot::GenerateCandidates, job_id);
        true
    }

    fn apply_candidate_previews_ready(
        &mut self,
        job_id: AssetJobId,
        generation_id: AssetGenerationId,
        recipe_revision: RevisionId,
        previews: Vec<AssetCandidatePreview>,
        failures: Vec<super::jobs::AssetCandidatePreviewFailure>,
    ) -> bool {
        if !self.is_current_job(
            AssetJobSlot::CompileCandidatePreviews,
            job_id,
            Some(recipe_revision),
            Some(generation_id),
        ) {
            return false;
        }
        for preview in previews {
            if let Some(slot) = self
                .candidate_slots
                .iter_mut()
                .find(|slot| slot.candidate.id == preview.candidate_id)
            {
                slot.preview = Some(preview);
                slot.preview_failure = None;
            }
        }
        for failure in failures {
            if let Some(slot) = self
                .candidate_slots
                .iter_mut()
                .find(|slot| slot.candidate.id == failure.candidate_id)
            {
                slot.preview = None;
                slot.preview_failure = Some(failure.message);
            }
        }
        self.finish_job(AssetJobSlot::CompileCandidatePreviews, job_id);
        true
    }

    fn apply_job_failure(&mut self, job_id: AssetJobId, message: String) -> bool {
        let Some(slot) = self.active_slot_for_job(job_id) else {
            self.stale_jobs.insert(job_id);
            return false;
        };
        self.finish_job(slot, job_id);
        if matches!(slot, AssetJobSlot::CompileCurrentAsset) {
            self.current_artifact = None;
            self.current_timeline = None;
            self.validation_issues = vec![AssetAppIssue {
                subject: None,
                code: "compile_failed".to_owned(),
                message,
            }];
        }
        if matches!(
            slot,
            AssetJobSlot::GenerateCandidates | AssetJobSlot::CompileCandidatePreviews
        ) {
            self.active_generation = None;
        }
        true
    }

    fn apply_job_cancelled(&mut self, job_id: AssetJobId) -> bool {
        let Some(slot) = self.active_slot_for_job(job_id) else {
            self.stale_jobs.insert(job_id);
            return false;
        };
        self.finish_job(slot, job_id);
        if matches!(
            slot,
            AssetJobSlot::GenerateCandidates | AssetJobSlot::CompileCandidatePreviews
        ) {
            self.active_generation = None;
        }
        true
    }

    fn active_slot_for_job(&self, job_id: AssetJobId) -> Option<AssetJobSlot> {
        self.active_jobs
            .iter()
            .find_map(|(slot, active)| (active.job_id == job_id).then_some(*slot))
    }
}

/// State transition errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssetAppStateError {
    UnknownPartInstance(PartInstanceId),
    UnknownParameter(ParameterId),
    UnknownCandidate(AssetCandidateId),
    UnknownRevision(RevisionId),
    LockedParameter(ParameterId),
    NonFiniteParameter(ParameterId),
    MissingSavePath,
    NoParentRevision,
    JobIdOverflow,
    GenerationIdOverflow,
    InvalidRecipe(Vec<AssetAppIssue>),
    InvalidProject(String),
    EditRejected(String),
}

impl fmt::Display for AssetAppStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPartInstance(instance) => {
                write!(formatter, "unknown part instance {instance:?}")
            }
            Self::UnknownParameter(parameter) => {
                write!(formatter, "unknown parameter {parameter:?}")
            }
            Self::UnknownCandidate(candidate) => {
                write!(formatter, "unknown candidate {candidate:?}")
            }
            Self::UnknownRevision(revision) => write!(formatter, "unknown revision {revision:?}"),
            Self::LockedParameter(parameter) => write!(formatter, "locked parameter {parameter:?}"),
            Self::NonFiniteParameter(parameter) => {
                write!(formatter, "non-finite value for parameter {parameter:?}")
            }
            Self::MissingSavePath => formatter.write_str("save requires a file path"),
            Self::NoParentRevision => formatter.write_str("current revision has no parent"),
            Self::JobIdOverflow => formatter.write_str("asset job id overflow"),
            Self::GenerationIdOverflow => formatter.write_str("asset generation id overflow"),
            Self::InvalidRecipe(issues) => {
                write!(
                    formatter,
                    "asset recipe is invalid with {} issue(s)",
                    issues.len()
                )
            }
            Self::InvalidProject(message) => write!(formatter, "invalid asset project: {message}"),
            Self::EditRejected(message) => write!(formatter, "asset edit rejected: {message}"),
        }
    }
}

impl Error for AssetAppStateError {}

fn first_part(recipe: &AssetRecipe) -> Option<PartInstanceId> {
    recipe
        .root_instances
        .first()
        .copied()
        .or_else(|| recipe.instances.keys().next().copied())
}

fn first_parameter(recipe: &AssetRecipe) -> Option<ParameterId> {
    recipe.parameters.keys().next().copied()
}

fn ensure_valid_recipe(recipe: &AssetRecipe) -> Result<(), AssetAppStateError> {
    let issues = recipe_issues(recipe);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(AssetAppStateError::InvalidRecipe(issues))
    }
}

fn ensure_valid_project_snapshot(project: &AssetModelingProject) -> Result<(), AssetAppStateError> {
    if project.project_kind != ASSET_APP_PROJECT_KIND {
        return Err(AssetAppStateError::InvalidProject(format!(
            "unsupported project kind '{}'",
            project.project_kind
        )));
    }
    if project.schema_version != ASSET_APP_PROJECT_SCHEMA_VERSION {
        return Err(AssetAppStateError::InvalidProject(format!(
            "unsupported schema version {}",
            project.schema_version
        )));
    }
    if project.revision_history.revisions.is_empty() {
        return Err(AssetAppStateError::InvalidProject(
            "revision history is empty".to_owned(),
        ));
    }
    if !project
        .revision_history
        .revisions
        .contains_key(&project.revision_history.current)
    {
        return Err(AssetAppStateError::InvalidProject(
            "current revision is missing".to_owned(),
        ));
    }

    let mut max_revision = 0;
    for (id, revision) in &project.revision_history.revisions {
        if revision.id != *id {
            return Err(AssetAppStateError::InvalidProject(format!(
                "revision map key {:?} does not match stored id {:?}",
                id, revision.id
            )));
        }
        max_revision = max_revision.max(id.0);
        if let Some(parent) = revision.parent {
            if !project.revision_history.revisions.contains_key(&parent) {
                return Err(AssetAppStateError::InvalidProject(format!(
                    "revision {:?} has missing parent {:?}",
                    id, parent
                )));
            }
            if parent.0 >= id.0 {
                return Err(AssetAppStateError::InvalidProject(format!(
                    "revision {:?} parent {:?} does not preserve monotonic ids",
                    id, parent
                )));
            }
        }
        ensure_valid_recipe(&revision.recipe)?;
    }
    if project.revision_history.next_revision <= max_revision {
        return Err(AssetAppStateError::InvalidProject(
            "next revision id is not greater than existing revisions".to_owned(),
        ));
    }
    Ok(())
}

fn recipe_issues(recipe: &AssetRecipe) -> Vec<AssetAppIssue> {
    validate_asset_recipe(recipe)
        .issues
        .into_iter()
        .map(AssetAppIssue::from)
        .collect()
}
