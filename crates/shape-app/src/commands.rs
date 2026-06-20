//! UI-independent application commands and effects.

#![allow(dead_code)]

use std::path::PathBuf;

use shape_core::{CandidateId, NodeId, ParamGroup, ParamPath, RevisionId};
use shape_presets::PresetId;
use shape_search::{ExplorationMode, TargetScope};

use crate::jobs::JobRequest;
use crate::viewport::ViewportAction;

/// Command emitted by UI panels or keyboard shortcuts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppCommand {
    SelectNode(Option<NodeId>),
    SetScalar {
        path: ParamPath,
        value: f32,
    },
    ToggleLock {
        path: ParamPath,
        locked: bool,
    },
    SetTargetScope(TargetScope),
    SetParameterGroup {
        group: ParamGroup,
        enabled: bool,
    },
    SetExplorationMode(ExplorationMode),
    SetSearchBudget {
        proposal_count: usize,
        result_count: usize,
    },
    SetSeed(u64),
    GenerateDirections,
    CancelActiveGeneration,
    AcceptCandidate(CandidateId),
    DismissCandidate(CandidateId),
    ClearCandidates,
    Undo,
    SwitchRevision(RevisionId),
    LoadPreset(PresetId),
    ResetCurrentPreset,
    Viewport(ViewportAction),
    Save,
    SaveAs(PathBuf),
    OpenProject(PathBuf),
    ExportCurrentObj(PathBuf),
    Exit,
    FitView,
}

/// Side effect requested by the state layer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppEffect {
    StartJob(Box<JobRequest>),
    SaveProject(PathBuf),
    LoadProject(PathBuf),
    RequestExit,
}
