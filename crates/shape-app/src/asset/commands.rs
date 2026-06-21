//! UI-independent commands and effects for explicit asset authoring.

#![allow(dead_code)]

use std::path::PathBuf;

use shape_asset::{
    AssetRecipe, OperationId, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, RevisionId, Transform3,
};

use crate::viewport::ViewportAction;

use super::jobs::{AssetCandidateId, AssetJobRequest};
use super::state::AssetModelingProject;

/// Template metadata plus a full recipe snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetTemplate {
    pub id: String,
    pub title: String,
    pub recipe: AssetRecipe,
}

/// Lock target exposed at the app command boundary.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AssetLockTarget {
    Parameter(ParameterId),
    Instance(PartInstanceId),
    Subtree(PartInstanceId),
    Topology(PartDefinitionId),
}

/// User intent boundary for explicit asset state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetAppCommand {
    SelectPart(Option<PartInstanceId>),
    SelectParameter(Option<ParameterId>),
    SelectCutOperation(Option<OperationId>),
    SetParameter {
        parameter: ParameterId,
        value: f32,
    },
    SetCutOperationScalar {
        definition: PartDefinitionId,
        operation: OperationId,
        field: String,
        value: f32,
    },
    RemoveCutOperation {
        definition: PartDefinitionId,
        operation: OperationId,
    },
    SetTransform {
        instance: PartInstanceId,
        transform: Transform3,
    },
    SetLock {
        target: AssetLockTarget,
        locked: bool,
    },
    AddOptionalPart {
        instance: PartInstance,
    },
    RemoveOptionalPart(PartInstanceId),
    ReplaceCompatiblePart {
        instance: PartInstanceId,
        definition: PartDefinitionId,
    },
    ReplaceDefinition(PartDefinition),
    ToggleOptionalPart {
        instance: PartInstanceId,
        enabled: bool,
    },
    GenerateRefine,
    GenerateExplore,
    AcceptCandidate(AssetCandidateId),
    RejectCandidate(AssetCandidateId),
    Undo,
    SwitchBranch(RevisionId),
    LoadTemplate(Box<AssetTemplate>),
    Save,
    SaveAs(PathBuf),
    Load(PathBuf),
    ExportObj(PathBuf),
    ExportPackage(PathBuf),
    FitCamera,
    SetWireframe(bool),
    Viewport(ViewportAction),
}

/// Side effects requested by the state reducer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetAppEffect {
    StartJob(Box<AssetJobRequest>),
    SaveProject {
        path: PathBuf,
        project: Box<AssetModelingProject>,
    },
    LoadProject(PathBuf),
}
