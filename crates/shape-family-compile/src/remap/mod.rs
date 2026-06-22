//! Typed semantic ID remapping boundaries for recipe-fragment instantiation.
//!
//! Fragment remapping must remain structural: every semantic ID kind has an
//! explicit map, and future implementation modules extend these maps rather
//! than rewriting serialized text.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::{
    BoundaryLoopId, OperationId, ParameterId, PartDefinitionId, PartInstanceId, RegionId, SocketId,
};
use thiserror::Error;

pub mod assembly;
pub mod ids;
pub mod operations;
pub mod ports;
pub mod relationships;
pub mod variation;

/// Full typed remap from one source fragment into an instantiated recipe.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentRemap {
    /// Part-definition remaps.
    pub definitions: BTreeMap<PartDefinitionId, PartDefinitionId>,
    /// Part-instance remaps.
    pub instances: BTreeMap<PartInstanceId, PartInstanceId>,
    /// Parameter remaps.
    pub parameters: BTreeMap<ParameterId, ParameterId>,
    /// Modeling-operation remaps.
    pub operations: BTreeMap<OperationId, OperationId>,
    /// Surface-region remaps.
    pub regions: BTreeMap<RegionId, RegionId>,
    /// Boundary-loop remaps.
    pub boundary_loops: BTreeMap<BoundaryLoopId, BoundaryLoopId>,
    /// Socket remaps.
    pub sockets: BTreeMap<SocketId, SocketId>,
}

/// IDs allocated while preparing a fragment remap.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocatedSemanticIds {
    /// Allocated definition IDs.
    pub definitions: Vec<PartDefinitionId>,
    /// Allocated instance IDs.
    pub instances: Vec<PartInstanceId>,
    /// Allocated parameter IDs.
    pub parameters: Vec<ParameterId>,
    /// Allocated operation IDs.
    pub operations: Vec<OperationId>,
    /// Allocated region IDs.
    pub regions: Vec<RegionId>,
    /// Allocated boundary-loop IDs.
    pub boundary_loops: Vec<BoundaryLoopId>,
    /// Allocated socket IDs.
    pub sockets: Vec<SocketId>,
}

/// Deterministic audit report for one fragment remap.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentRemapReport {
    /// Source fragment ID.
    pub fragment_id: String,
    /// Typed remap that was applied.
    pub remap: FragmentRemap,
    /// Allocated IDs that back the remap.
    pub allocated: AllocatedSemanticIds,
    /// Non-fatal warnings emitted by a remap stage.
    pub warnings: Vec<String>,
}

/// Fragment remap failure.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FragmentRemapError {
    /// The source fragment references an ID outside the fragment contract.
    #[error("fragment `{fragment}` references external {id_kind} `{id}`")]
    ExternalReference {
        /// Source fragment ID.
        fragment: String,
        /// Semantic ID kind.
        id_kind: String,
        /// Display form of the offending ID.
        id: String,
    },
    /// A remap stage found a missing typed mapping.
    #[error("fragment `{fragment}` has no {id_kind} remap for `{id}`")]
    MissingMapping {
        /// Source fragment ID.
        fragment: String,
        /// Semantic ID kind.
        id_kind: String,
        /// Display form of the offending ID.
        id: String,
    },
    /// A remap stage produced two targets for one semantic source ID.
    #[error("fragment `{fragment}` duplicated {id_kind} remap for `{id}`")]
    DuplicateMapping {
        /// Source fragment ID.
        fragment: String,
        /// Semantic ID kind.
        id_kind: String,
        /// Display form of the offending ID.
        id: String,
    },
    /// The requested remap feature is intentionally unsupported in this stage.
    #[error("fragment `{fragment}` remap stage `{stage}` is unsupported: {reason}")]
    Unsupported {
        /// Source fragment ID.
        fragment: String,
        /// Remap stage.
        stage: String,
        /// Reason.
        reason: String,
    },
}
