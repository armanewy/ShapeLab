//! Relationship contract shells for semantic composition.

use serde::{Deserialize, Serialize};

/// Placement policy for a relationship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementPolicy {
    /// Position rule.
    pub position_rule: PositionRule,
}

impl Default for PlacementPolicy {
    fn default() -> Self {
        Self {
            position_rule: PositionRule::PreserveCurrentOnDetach,
        }
    }
}

/// Finite position rule vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PositionRule {
    /// Keep a fixed offset from a parent edge.
    FixedOffsetFromEdge {
        /// Parent edge label.
        edge: String,
        /// Authored local offset.
        offset: [f32; 3],
    },
    /// Keep a normalized parent surface coordinate.
    ProportionalUv {
        /// U coordinate in [0, 1].
        u: f32,
        /// V coordinate in [0, 1].
        v: f32,
    },
    /// Keep centered in a named zone.
    CenteredInZone {
        /// Zone label.
        zone: String,
    },
    /// Preserve current placement when detached.
    PreserveCurrentOnDetach,
}

/// Orientation policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum OrientationPolicy {
    /// Preserve authored child orientation.
    #[default]
    PreserveChild,
    /// Align to a future surface normal within a maximum angle.
    AlignToSurfaceNormal {
        /// Maximum alignment angle in degrees.
        max_angle_degrees: f32,
    },
}

/// Scale policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ScalePolicy {
    /// Preserve child scale.
    #[default]
    PreserveChild,
    /// Uniformly inherit parent scale.
    UniformWithParent,
    /// Clamp child scale to a finite range.
    ClampToRange {
        /// Minimum scale.
        minimum: f32,
        /// Maximum scale.
        maximum: f32,
    },
}

/// Contact policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ContactPolicy {
    /// No contact evidence yet.
    #[default]
    NotChecked,
    /// Future surface contact with clearance.
    SurfaceContact {
        /// Non-negative clearance.
        clearance: f32,
    },
    /// Intentional gap with clearance.
    IntentionalGap {
        /// Non-negative clearance.
        clearance: f32,
    },
    /// Intentional overlap.
    IntentionalOverlap,
}

/// Edit policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipEditPolicy {
    /// Relationship may be edited through supported controls.
    #[default]
    Editable,
    /// Relationship is locked.
    Locked,
}

/// Selection policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionPolicy {
    /// Parent and child may be selected independently.
    #[default]
    Independent,
    /// Select relationship as one unit.
    SelectTogether,
}

/// Reset policy for a relationship.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetPolicy {
    /// Reset to authored placement policy.
    #[default]
    AuthoredPlacement,
    /// Preserve current placement.
    PreserveCurrent,
}

/// Export realization policy shell.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportRealizationPolicy {
    /// Export realization has not been decided.
    #[default]
    Pending,
    /// Preserve authored relationship semantics in sidecar/report data.
    PreserveSemanticSidecar,
    /// Preserve as separate nodes if exporter supports hierarchy.
    PreserveNodes,
    /// Bake into a merged mesh only when a later export gate proves it.
    BakedUnion,
}
