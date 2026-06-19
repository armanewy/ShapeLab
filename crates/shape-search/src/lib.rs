#![forbid(unsafe_code)]

//! Deterministic candidate search contracts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shape_core::{CandidateId, EditProgram, NodeId, ParamGroup, ShapeDocument};
use thiserror::Error;

/// Exploration distance.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorationMode {
    /// Local changes around the current model.
    Refine,
    /// Broader changes for directional discovery.
    Explore,
}

/// Target affected by mutation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetScope {
    /// Only the selected node.
    Selected,
    /// Selected node and descendants.
    Subtree,
    /// Entire document.
    WholeModel,
}

/// Candidate generation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Deterministic seed.
    pub seed: u64,
    /// Number of raw proposals.
    pub proposal_count: usize,
    /// Number of final candidates.
    pub result_count: usize,
    /// Descriptor grid resolution.
    pub descriptor_resolution: usize,
    /// Selected node, if any.
    pub selected_node: Option<NodeId>,
    /// Target scope.
    pub target_scope: TargetScope,
    /// Enabled parameter groups.
    pub enabled_groups: BTreeSet<ParamGroup>,
    /// Exploration mode.
    pub mode: ExplorationMode,
}

/// Coarse geometric descriptor for diversity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeDescriptor {
    /// Occupancy or feature values.
    pub values: Vec<f32>,
}

/// Generated candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Stable ID within a generation.
    pub id: CandidateId,
    /// Candidate document.
    pub document: ShapeDocument,
    /// Edit that produced the candidate.
    pub edit: EditProgram,
    /// Coarse descriptor.
    pub descriptor: ShapeDescriptor,
    /// Distance from parent descriptor.
    pub distance_from_parent: f32,
}

/// Search errors.
#[derive(Debug, Error)]
pub enum SearchError {
    /// The requested operation belongs to a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

/// Generate diverse candidate documents.
pub fn generate_candidates(
    _document: &ShapeDocument,
    _request: &SearchRequest,
) -> Result<Vec<Candidate>, SearchError> {
    Err(SearchError::NotImplemented("candidate generation"))
}
