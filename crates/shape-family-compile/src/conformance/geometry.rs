//! Geometric conformance report contracts.

use serde::{Deserialize, Serialize};
use shape_family::{ConstraintKind, FamilyRuleExecutionPolicy};

use super::ConformanceStatus;

/// Explicit executable geometric binding kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConstraintBindingKind {
    /// Compare the bounds of one or more roles.
    RoleBounds,
    /// Enforce clearance between roles.
    RoleClearance,
    /// Require roles to touch.
    RoleMustTouch,
    /// Require one role to contain another.
    RoleMustContain,
    /// Require compatible socket connection.
    SocketConnection,
    /// Require support through a valid attachment.
    SupportViaAttachment,
    /// Enforce a triangle budget on the compiled artifact.
    ArtifactTriangleBudget,
    /// Adapter/runtime metadata is acknowledged but not evaluated by the compiler.
    AdapterDeferredMetadata,
}

/// Numeric measurement captured while evaluating one geometric row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintMeasurement {
    /// Stable measurement key.
    pub key: String,
    /// Numeric value in family/export units.
    pub value: f32,
    /// Optional accepted minimum.
    pub minimum: Option<f32>,
    /// Optional accepted maximum.
    pub maximum: Option<f32>,
}

/// Conformance row for one family geometric constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintConformance {
    /// Family constraint ID.
    pub constraint_id: String,
    /// Roles governed by this row.
    pub roles: Vec<String>,
    /// Theme-neutral constraint class.
    pub kind: ConstraintKind,
    /// Concrete executable binding used for evaluation, if any.
    pub binding: Option<ConstraintBindingKind>,
    /// Rule policy.
    pub policy: FamilyRuleExecutionPolicy,
    /// Measurements captured by the evaluator.
    pub measurements: Vec<ConstraintMeasurement>,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this constraint.
    pub issue_codes: Vec<String>,
}
