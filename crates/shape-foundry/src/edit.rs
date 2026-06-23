//! Local override and semantic edit contracts.

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetEditProgram, BoundaryLoopId, OperationId, ParameterId, PartDefinitionId, PartInstanceId,
    RegionId, SocketId,
};
use shape_family_compile::identity::GeometryInputFingerprint;

/// Stable local override ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalRecipeOverrideId(pub String);

/// What happens to a local override when style/provider inputs change.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideSurvivalPolicy {
    /// Keep only when the base geometry fingerprint is unchanged.
    Pinned,
    /// Replay and validate against the changed recipe.
    Revalidate,
    /// Drop when the style/provider changes.
    DropOnStyleChange,
}

/// Semantic target touched by a local override.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TouchedSemanticTarget {
    /// Family parameter slot.
    FamilySlot(String),
    /// Asset parameter.
    Parameter(ParameterId),
    /// Part definition.
    PartDefinition(PartDefinitionId),
    /// Part occurrence.
    PartInstance(PartInstanceId),
    /// Modeling operation.
    Operation(OperationId),
    /// Surface region.
    Region(RegionId),
    /// Boundary loop.
    BoundaryLoop(BoundaryLoopId),
    /// Socket.
    Socket(SocketId),
    /// Pack-authored semantic target.
    Custom(String),
}

/// Local recipe override applied after base family instantiation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalRecipeOverride {
    /// Stable override ID.
    pub id: LocalRecipeOverrideId,
    /// Geometry fingerprint of the recipe this override was authored against.
    pub base_geometry_fingerprint: GeometryInputFingerprint,
    /// Ordered semantic edit program.
    pub edit_program: AssetEditProgram,
    /// Semantic targets the edit touches.
    pub touched_targets: Vec<TouchedSemanticTarget>,
    /// Survival policy when upstream inputs change.
    pub survival_policy: OverrideSurvivalPolicy,
}

/// Replayable foundry edit program row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryEdit {
    /// Human-facing edit label.
    pub label: String,
    /// Commands applied by this edit.
    pub commands: Vec<crate::command::FoundryCommand>,
}
