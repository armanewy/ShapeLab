//! Novice-facing explicit asset UI contracts.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod panels;
pub(crate) mod viewport;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(pub u64);
    };
}

id_type!(PartDefinitionId);
id_type!(PartInstanceId);
id_type!(ParameterId);
id_type!(RegionId);
id_type!(SocketId);
id_type!(AssetCandidateId);
id_type!(AssetJobId);
id_type!(AssetRevisionId);

/// Asset-space transform DTO aligned with `docs/asset-app-contracts.md`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetTransform3 {
    pub translation: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for AssetTransform3 {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// User-intent boundary emitted by asset panels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetAppCommand {
    SelectPart(PartInstanceId),
    SelectRegion(PartInstanceId, RegionId),
    SelectSocket(PartInstanceId, SocketId),
    SetParameter(ParameterId, f32),
    SetTransform(PartInstanceId, AssetTransform3),
    ToggleOptionalPart(PartInstanceId, bool),
    RequestPreview(AssetJobRequest),
    RequestCompile(AssetJobRequest),
    CancelJob(AssetJobId),
    AcceptCandidate(AssetCandidateId),
    RejectCandidate(AssetCandidateId),
    SwitchRevision(AssetRevisionId),
    Undo,
    SetParameterLock(ParameterId, bool),
    SetPartLock(PartInstanceId, bool),
    SetSubtreeLock(PartInstanceId, bool),
    SetTopologyLock(PartDefinitionId, bool),
    SetWireframe(bool),
}

/// Deterministic background-work request summary for panel commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetJobRequest {
    pub job_id: AssetJobId,
    pub recipe_revision: AssetRevisionId,
    pub kind: AssetJobKind,
}

/// Background job kind names from the asset app contract.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AssetJobKind {
    Preview,
    Compile,
    Inspect,
    CandidateSearch,
}

/// Fixed beginner-facing parameter groups.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum AssetParameterGroup {
    Size,
    Proportions,
    Placement,
    Curvature,
    EdgeSoftness,
    Repetition,
    PartPresence,
    DetailDensity,
}

impl AssetParameterGroup {
    #[must_use]
    pub(crate) fn all() -> [Self; 8] {
        [
            Self::Size,
            Self::Proportions,
            Self::Placement,
            Self::Curvature,
            Self::EdgeSoftness,
            Self::Repetition,
            Self::PartPresence,
            Self::DetailDensity,
        ]
    }

    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Proportions => "Proportions",
            Self::Placement => "Placement",
            Self::Curvature => "Curvature",
            Self::EdgeSoftness => "Edge Softness",
            Self::Repetition => "Repetition",
            Self::PartPresence => "Part Presence",
            Self::DetailDensity => "Detail Density",
        }
    }

    #[must_use]
    pub(crate) fn help(self) -> &'static str {
        match self {
            Self::Size => "Overall dimensions and thickness controls.",
            Self::Proportions => "Relative width, height, and depth controls.",
            Self::Placement => "Where a part sits and how it is turned.",
            Self::Curvature => "Profile and bend-like shape controls.",
            Self::EdgeSoftness => "Bevels, roundness, and crisp edge controls.",
            Self::Repetition => "Array and repeated-part controls.",
            Self::PartPresence => "Optional parts that can be included or omitted.",
            Self::DetailDensity => "Segment and authored detail-count controls.",
        }
    }
}

/// Snapshot consumed by the contract-level asset panels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetUiState {
    pub title: String,
    pub selected_part: Option<PartInstanceId>,
    pub parts: Vec<AssetPart>,
    pub parameters: Vec<AssetParameter>,
    pub candidates: Vec<AssetCandidate>,
    pub history: Vec<AssetHistoryRevision>,
    pub active_job: Option<AssetJobProgress>,
    pub validation: Vec<AssetValidationMessage>,
    pub parameter_locks: BTreeSet<ParameterId>,
    pub part_locks: BTreeSet<PartInstanceId>,
    pub subtree_locks: BTreeSet<PartInstanceId>,
    pub topology_locks: BTreeSet<PartDefinitionId>,
    pub wireframe: bool,
}

impl AssetUiState {
    #[must_use]
    pub(crate) fn empty(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            selected_part: None,
            parts: Vec::new(),
            parameters: Vec::new(),
            candidates: Vec::new(),
            history: Vec::new(),
            active_job: None,
            validation: Vec::new(),
            parameter_locks: BTreeSet::new(),
            part_locks: BTreeSet::new(),
            subtree_locks: BTreeSet::new(),
            topology_locks: BTreeSet::new(),
            wireframe: false,
        }
    }

    #[must_use]
    pub(crate) fn selected_part(&self) -> Option<&AssetPart> {
        self.selected_part
            .and_then(|selected| self.parts.iter().find(|part| part.id == selected))
    }

    #[must_use]
    pub(crate) fn definition_use_counts(&self) -> BTreeMap<PartDefinitionId, usize> {
        let mut counts = BTreeMap::new();
        for part in &self.parts {
            *counts.entry(part.definition).or_insert(0) += 1;
        }
        counts
    }
}

/// One visible part instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetPart {
    pub id: PartInstanceId,
    pub parent: Option<PartInstanceId>,
    pub definition: PartDefinitionId,
    pub name: String,
    pub definition_name: String,
    pub enabled: bool,
    pub optional: bool,
    pub generated: GeneratedPartKind,
    pub socket_count: usize,
    pub region_count: usize,
    pub warning_count: usize,
}

/// How a part instance was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GeneratedPartKind {
    Authored,
    Mirrored,
    LinearArray { index: u32, count: u32 },
    RadialArray { index: u32, count: u32 },
}

impl GeneratedPartKind {
    #[must_use]
    pub(crate) fn label(&self) -> Option<String> {
        match self {
            Self::Authored => None,
            Self::Mirrored => Some("Mirrored".to_owned()),
            Self::LinearArray { index, count } => Some(format!("Array {index}/{count}")),
            Self::RadialArray { index, count } => Some(format!("Radial {index}/{count}")),
        }
    }
}

/// Reflected scalar parameter.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetParameter {
    pub id: ParameterId,
    pub part: Option<PartInstanceId>,
    pub definition: Option<PartDefinitionId>,
    pub label: String,
    pub technical_name: String,
    pub group: AssetParameterGroup,
    pub value: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub step: f32,
    pub locked: bool,
    pub topology_changing: bool,
    pub beginner_description: String,
}

/// Candidate card DTO.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCandidate {
    pub id: AssetCandidateId,
    pub title: String,
    pub structural_changes: usize,
    pub numeric_changes: usize,
    pub edits: Vec<AssetCandidateEdit>,
    pub validation: AssetValidationState,
}

/// One explanatory candidate change line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCandidateEdit {
    pub subject: String,
    pub label: String,
    pub before: Option<f32>,
    pub after: Option<f32>,
    pub structural: bool,
}

/// Validation state shown on parts, candidates, and viewport overlays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssetValidationState {
    Valid,
    Warning(String),
    Error(String),
    Pending,
}

impl AssetValidationState {
    #[must_use]
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Valid => "Valid",
            Self::Warning(_) => "Warning",
            Self::Error(_) => "Blocked",
            Self::Pending => "Checking",
        }
    }

    #[must_use]
    pub(crate) fn detail(&self) -> Option<&str> {
        match self {
            Self::Valid | Self::Pending => None,
            Self::Warning(message) | Self::Error(message) => Some(message.as_str()),
        }
    }
}

/// One validation message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetValidationMessage {
    pub part: Option<PartInstanceId>,
    pub state: AssetValidationState,
    pub message: String,
}

/// Branchable asset revision row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetHistoryRevision {
    pub id: AssetRevisionId,
    pub parent: Option<AssetRevisionId>,
    pub label: String,
    pub operation_summary: String,
    pub child_count: usize,
    pub selected: bool,
}

/// Current generation or compile job progress.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetJobProgress {
    pub job_id: AssetJobId,
    pub kind: AssetJobKind,
    pub phase: String,
    pub completed: usize,
    pub total: usize,
}

impl AssetJobProgress {
    #[must_use]
    pub(crate) fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }
}
