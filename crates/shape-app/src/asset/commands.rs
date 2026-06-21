//! UI-independent commands and effects for explicit asset authoring.

#![allow(dead_code)]

use std::path::PathBuf;

use shape_asset::{
    AssetRecipe, ParameterId, PartDefinition, PartDefinitionId, PartInstance, PartInstanceId,
    RevisionId, Transform3,
};

use super::jobs::{AssetCandidateId, AssetJobRequest};

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
    SetParameter {
        parameter: ParameterId,
        value: f32,
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
    GenerateRefine,
    GenerateExplore,
    AcceptCandidate(AssetCandidateId),
    Undo,
    SwitchBranch(RevisionId),
    LoadTemplate(AssetTemplate),
    Save,
    SaveAs(PathBuf),
    Load(PathBuf),
    ExportPackage(PathBuf),
    FitCamera,
}

/// Side effects requested by the state reducer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetAppEffect {
    StartJob(Box<AssetJobRequest>),
    SaveRecipe {
        path: PathBuf,
        recipe: Box<AssetRecipe>,
    },
    LoadRecipe(PathBuf),
}
