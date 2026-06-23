//! State snapshot contract for the native Foundry surface.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use shape_asset::RevisionId;
use shape_foundry::{
    FoundryAssetDocument, FoundryBuildStamp, FoundryCandidateId, FoundryCompilationOutput,
    FoundryLock, FoundryValidationReport, GeneratedRecipeSnapshot,
};
use shape_project::foundry::{FoundryProjectFile, FoundryProjectLoadReport};

use super::jobs::FoundryJobRequest;
use super::view_model::{FoundryCandidateCard, FoundryControlView, FoundryPackView};

/// Complete UI-independent state for a Foundry editing session.
#[derive(Debug, Clone, PartialEq, Default)]
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
}
