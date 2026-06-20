//! UI-independent application state contract.

#![allow(dead_code)]

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;

use shape_core::{NodeId, ParamGroup};
use shape_mesh::{MeshSettings, TriangleMesh};
use shape_presets::PresetId;
use shape_project::Project;
use shape_render::{OrbitCamera, RenderSettings, RenderedImage};
use shape_search::{ExplorationMode, TargetScope};

use crate::jobs::{CandidatePreview, GenerationId, JobId};

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

/// Current preview cache.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentPreview {
    pub mesh: TriangleMesh,
    pub image: RenderedImage,
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
    pub active_preset: Option<PresetId>,
    pub current_file_path: Option<PathBuf>,
    pub dirty: bool,
    pub status: StatusMessage,
    pub recoverable_errors: VecDeque<String>,
    pub active_jobs: BTreeSet<JobId>,
    pub active_generation: Option<GenerationId>,
}
