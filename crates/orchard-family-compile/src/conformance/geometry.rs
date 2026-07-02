//! Geometric conformance report contracts and explicit executable bindings.
//!
//! Family [`ConstraintKind`] values are semantic labels. They are not executable
//! by themselves. Evaluation only happens for constraints that have a caller
//! supplied [`ExplicitConstraintBinding`] keyed by constraint ID.

use std::collections::BTreeMap;

use orchard_asset::{AssetRecipe, PartInstanceId, SocketId};
use orchard_compile::{AssetArtifact, CompiledPart};
use orchard_family::{ConstraintKind, FamilyRuleExecutionPolicy, GeometricConstraint};
use orchard_poly::MeshBounds;
use serde::{Deserialize, Serialize};

use super::ConformanceStatus;

const EPSILON: f32 = 1.0e-5;

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

/// Inputs available to geometric constraint bindings.
#[derive(Debug, Copy, Clone)]
pub struct ConstraintEvaluationContext<'a> {
    /// Instantiated asset recipe.
    pub recipe: &'a AssetRecipe,
    /// Compiled artifact, when the caller has one.
    pub artifact: Option<&'a AssetArtifact>,
}

/// Explicit constraint binding map keyed by family constraint ID.
pub type ConstraintBindingMap = BTreeMap<String, ExplicitConstraintBinding>;

/// Executable conformance binding.
pub trait ConstraintBinding {
    /// Return the concrete binding kind reported in conformance rows.
    fn binding_kind(&self) -> ConstraintBindingKind;

    /// Evaluate one family constraint against an asset recipe/artifact.
    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance;
}

/// Union of supported explicit constraint bindings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExplicitConstraintBinding {
    /// Compare role aggregate extents against optional min/max limits.
    RoleBounds(RoleBounds),
    /// Enforce minimum clearance between role occurrences.
    RoleClearance(RoleClearance),
    /// Require occurrences of one role to touch another role.
    RoleMustTouch(RoleMustTouch),
    /// Require one role to contain another role.
    RoleMustContain(RoleMustContain),
    /// Require socket attachment between two roles.
    SocketConnection(SocketConnection),
    /// Require a supported role to be carried by an attachment/contact role.
    SupportViaAttachment(SupportViaAttachment),
    /// Enforce artifact triangle budget.
    ArtifactTriangleBudget(ArtifactTriangleBudget),
}

impl ConstraintBinding for ExplicitConstraintBinding {
    fn binding_kind(&self) -> ConstraintBindingKind {
        match self {
            Self::RoleBounds(binding) => binding.binding_kind(),
            Self::RoleClearance(binding) => binding.binding_kind(),
            Self::RoleMustTouch(binding) => binding.binding_kind(),
            Self::RoleMustContain(binding) => binding.binding_kind(),
            Self::SocketConnection(binding) => binding.binding_kind(),
            Self::SupportViaAttachment(binding) => binding.binding_kind(),
            Self::ArtifactTriangleBudget(binding) => binding.binding_kind(),
        }
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        if constraint.execution_policy == FamilyRuleExecutionPolicy::RuntimeOnly {
            return deferred_row(constraint, Some(self.binding_kind()));
        }
        match self {
            Self::RoleBounds(binding) => binding.evaluate(constraint, context),
            Self::RoleClearance(binding) => binding.evaluate(constraint, context),
            Self::RoleMustTouch(binding) => binding.evaluate(constraint, context),
            Self::RoleMustContain(binding) => binding.evaluate(constraint, context),
            Self::SocketConnection(binding) => binding.evaluate(constraint, context),
            Self::SupportViaAttachment(binding) => binding.evaluate(constraint, context),
            Self::ArtifactTriangleBudget(binding) => binding.evaluate(constraint, context),
        }
    }
}

/// Compare aggregate role bounds against optional extent limits.
#[derive(Debug, Copy, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RoleBounds {
    /// Optional minimum accepted role extent per axis.
    pub minimum_extent: Option<[f32; 3]>,
    /// Optional maximum accepted role extent per axis.
    pub maximum_extent: Option<[f32; 3]>,
}

impl ConstraintBinding for RoleBounds {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::RoleBounds
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let mut row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "role_bounds_missing_artifact");
        };
        let Some(bounds) = constraint
            .roles
            .iter()
            .map(|role| role_bounds(context.recipe, artifact, role))
            .collect::<Option<Vec<_>>>()
            .and_then(union_bounds)
        else {
            return missing(row, "role_bounds_missing_role");
        };
        let extent = bounds_extent(&bounds);
        for axis in 0..3 {
            row.measurements.push(ConstraintMeasurement {
                key: format!("extent.{axis}"),
                value: extent[axis],
                minimum: self.minimum_extent.map(|minimum| minimum[axis]),
                maximum: self.maximum_extent.map(|maximum| maximum[axis]),
            });
        }
        if let Some(minimum) = self.minimum_extent
            && (0..3).any(|axis| extent[axis] + EPSILON < minimum[axis])
        {
            return failed(row, "role_bounds_below_minimum");
        }
        if let Some(maximum) = self.maximum_extent
            && (0..3).any(|axis| extent[axis] > maximum[axis] + EPSILON)
        {
            return failed(row, "role_bounds_exceeds_maximum");
        }
        passed(row)
    }
}

/// Enforce minimum clearance between occurrences.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleClearance {
    /// Minimum accepted clearance in world units.
    pub minimum: f32,
}

impl ConstraintBinding for RoleClearance {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::RoleClearance
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let mut row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "role_clearance_missing_artifact");
        };
        let Some(pairs) = role_pairs(context.recipe, artifact, &constraint.roles) else {
            return missing(row, "role_clearance_missing_role");
        };
        if pairs.is_empty() {
            return missing(row, "role_clearance_missing_pair");
        }
        let mut minimum_actual = f32::INFINITY;
        for (left, right) in pairs {
            minimum_actual = minimum_actual.min(bounds_distance(
                &left.world_mesh.bounds,
                &right.world_mesh.bounds,
            ));
        }
        row.measurements.push(ConstraintMeasurement {
            key: "minimum_clearance".to_owned(),
            value: minimum_actual,
            minimum: Some(self.minimum),
            maximum: None,
        });
        if minimum_actual + EPSILON < self.minimum {
            failed(row, "role_clearance_below_minimum")
        } else {
            passed(row)
        }
    }
}

/// Require the first role to touch the second role.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleMustTouch {
    /// Maximum accepted clearance in world units.
    pub max_clearance: f32,
}

impl ConstraintBinding for RoleMustTouch {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::RoleMustTouch
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let mut row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "role_touch_missing_artifact");
        };
        let Some((left_role, right_role)) = first_two_roles(&constraint.roles) else {
            return missing(row, "role_touch_missing_role");
        };
        let left_parts = parts_for_role(context.recipe, artifact, left_role);
        let right_parts = parts_for_role(context.recipe, artifact, right_role);
        if left_parts.is_empty() || right_parts.is_empty() {
            return missing(row, "role_touch_missing_role");
        }
        let mut worst_clearance = 0.0_f32;
        for left in &left_parts {
            let nearest = right_parts
                .iter()
                .filter(|right| left.instance_id != right.instance_id)
                .map(|right| bounds_distance(&left.world_mesh.bounds, &right.world_mesh.bounds))
                .fold(f32::INFINITY, f32::min);
            worst_clearance = worst_clearance.max(nearest);
            if nearest > self.max_clearance + EPSILON {
                row.measurements.push(ConstraintMeasurement {
                    key: format!("nearest_clearance.{}", left.instance_id.0),
                    value: nearest,
                    minimum: None,
                    maximum: Some(self.max_clearance),
                });
                return failed(row, "role_touch_missing");
            }
        }
        row.measurements.push(ConstraintMeasurement {
            key: "maximum_nearest_clearance".to_owned(),
            value: worst_clearance,
            minimum: None,
            maximum: Some(self.max_clearance),
        });
        passed(row)
    }
}

/// Require one role's bounds to contain another role's bounds.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleMustContain {
    /// Numeric tolerance in world units.
    pub epsilon: f32,
}

impl ConstraintBinding for RoleMustContain {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::RoleMustContain
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "role_containment_missing_artifact");
        };
        let Some((container_role, contained_role)) = first_two_roles(&constraint.roles) else {
            return missing(row, "role_containment_missing_role");
        };
        let containers = parts_for_role(context.recipe, artifact, container_role);
        let contained = parts_for_role(context.recipe, artifact, contained_role);
        if containers.is_empty() || contained.is_empty() {
            return missing(row, "role_containment_missing_role");
        }
        for part in contained {
            let contained_by_any = containers.iter().any(|container| {
                bounds_contains(
                    &container.world_mesh.bounds,
                    &part.world_mesh.bounds,
                    self.epsilon.max(EPSILON),
                )
            });
            if !contained_by_any {
                return failed(row, "role_containment_missing");
            }
        }
        passed(row)
    }
}

/// Require socket-based connection between two roles.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocketConnection {
    /// Required parent socket. When absent, any parent socket may satisfy the binding.
    pub parent_socket: Option<SocketId>,
    /// Required child socket. When absent, any child socket may satisfy the binding.
    pub child_socket: Option<SocketId>,
    /// Maximum accepted socket-origin distance in world units.
    pub max_origin_distance: f32,
    /// Maximum accepted angle between corresponding socket axes in degrees.
    pub max_axis_angle_degrees: f32,
    /// Optional maximum mesh clearance in world units.
    pub max_clearance: Option<f32>,
}

impl ConstraintBinding for SocketConnection {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::SocketConnection
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let row = row_for(constraint, Some(self.binding_kind()));
        let Some((parent_role, child_role)) = first_two_roles(&constraint.roles) else {
            return missing(row, "socket_connection_missing_role");
        };
        let Some(artifact) = context.artifact else {
            if constraint.execution_policy == FamilyRuleExecutionPolicy::RuntimeOnly {
                return deferred_row(constraint, Some(self.binding_kind()));
            }
            return missing(row, "socket_connection_missing_artifact");
        };
        let parents = parts_for_role(context.recipe, artifact, parent_role);
        let children = parts_for_role(context.recipe, artifact, child_role);
        if parents.is_empty() || children.is_empty() {
            return missing(row, "socket_connection_missing_role");
        }
        for child in children {
            let connected = parents.iter().any(|parent| {
                let Some((parent_socket, child_socket)) = self.recipe_attachment_socket_pair(
                    context.recipe,
                    parent.instance_id,
                    child.instance_id,
                ) else {
                    return false;
                };
                self.socket_frames_match(parent, child, parent_socket, child_socket)
                    && self.max_clearance.is_none_or(|maximum| {
                        bounds_distance(&parent.world_mesh.bounds, &child.world_mesh.bounds)
                            <= maximum + EPSILON
                    })
            });
            if !connected {
                return failed(row, "socket_connection_missing");
            }
        }
        passed(row)
    }
}

impl SocketConnection {
    fn recipe_attachment_socket_pair(
        &self,
        recipe: &AssetRecipe,
        parent: PartInstanceId,
        child: PartInstanceId,
    ) -> Option<(SocketId, SocketId)> {
        let child_instance = recipe.instances.get(&child)?;
        let Some(attachment) = &child_instance.attachment else {
            return None;
        };
        if attachment.parent_instance == parent
            && self
                .parent_socket
                .is_none_or(|socket| attachment.parent_socket == socket)
            && self
                .child_socket
                .is_none_or(|socket| attachment.child_socket == socket)
        {
            Some((attachment.parent_socket, attachment.child_socket))
        } else {
            None
        }
    }

    fn socket_frames_match(
        &self,
        parent: &CompiledPart,
        child: &CompiledPart,
        parent_socket: SocketId,
        child_socket: SocketId,
    ) -> bool {
        let Some(parent_socket) = parent.sockets_world.get(&parent_socket) else {
            return false;
        };
        let Some(child_socket) = child.sockets_world.get(&child_socket) else {
            return false;
        };
        distance(
            parent_socket.local_frame.origin,
            child_socket.local_frame.origin,
        ) <= self.max_origin_distance + EPSILON
            && frames_axes_align(
                &parent_socket.local_frame,
                &child_socket.local_frame,
                self.max_axis_angle_degrees,
            )
    }
}

/// Require support through attachment/contact between two roles.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportViaAttachment {
    /// Maximum accepted vertical clearance between support top and supported bottom.
    pub max_clearance: f32,
    /// Vertical axis index. Shape Lab conventions use Y (`1`).
    pub vertical_axis: usize,
}

impl ConstraintBinding for SupportViaAttachment {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::SupportViaAttachment
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "support_attachment_missing_artifact");
        };
        let Some((supported_role, support_role)) = first_two_roles(&constraint.roles) else {
            return missing(row, "support_attachment_missing_role");
        };
        let supported_parts = parts_for_role(context.recipe, artifact, supported_role);
        let support_parts = parts_for_role(context.recipe, artifact, support_role);
        if supported_parts.is_empty() || support_parts.is_empty() {
            return missing(row, "support_attachment_missing_role");
        }
        let axis = self.vertical_axis.min(2);
        for supported in supported_parts {
            let has_support = support_parts.iter().any(|support| {
                supported.instance_id != support.instance_id
                    && (recipe_instances_attached(
                        context.recipe,
                        supported.instance_id,
                        support.instance_id,
                    ) || bounds_supports(
                        &supported.world_mesh.bounds,
                        &support.world_mesh.bounds,
                        axis,
                        self.max_clearance,
                    ))
            });
            if !has_support {
                return failed(row, "support_attachment_missing");
            }
        }
        passed(row)
    }
}

/// Enforce maximum compiled triangle count.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTriangleBudget {
    /// Maximum accepted artifact triangles.
    pub maximum_triangles: u32,
}

impl ConstraintBinding for ArtifactTriangleBudget {
    fn binding_kind(&self) -> ConstraintBindingKind {
        ConstraintBindingKind::ArtifactTriangleBudget
    }

    fn evaluate(
        &self,
        constraint: &GeometricConstraint,
        context: ConstraintEvaluationContext<'_>,
    ) -> ConstraintConformance {
        let mut row = row_for(constraint, Some(self.binding_kind()));
        let Some(artifact) = context.artifact else {
            return missing(row, "triangle_budget_missing_artifact");
        };
        let actual = artifact.statistics.triangle_count.min(u32::MAX as u64) as u32;
        row.measurements.push(ConstraintMeasurement {
            key: "triangle_count".to_owned(),
            value: actual as f32,
            minimum: None,
            maximum: Some(self.maximum_triangles as f32),
        });
        if actual > self.maximum_triangles {
            failed(row, "triangle_budget_exceeded")
        } else {
            passed(row)
        }
    }
}

/// Evaluate family constraints with explicit caller-supplied bindings.
#[must_use]
pub fn evaluate_geometric_constraints(
    constraints: &[GeometricConstraint],
    bindings: &ConstraintBindingMap,
    recipe: &AssetRecipe,
    artifact: Option<&AssetArtifact>,
) -> Vec<ConstraintConformance> {
    let context = ConstraintEvaluationContext { recipe, artifact };
    constraints
        .iter()
        .map(|constraint| match bindings.get(&constraint.id) {
            Some(binding) => binding.evaluate(constraint, context),
            None => unbound_constraint_row(constraint),
        })
        .collect()
}

/// Return rows that make implementation-binding coverage explicit.
///
/// A required non-runtime constraint without an explicit binding is unsupported.
/// Advisory constraints are reported but non-rejecting, and runtime-only
/// constraints are explicitly deferred.
#[must_use]
pub fn validate_constraint_binding_coverage(
    constraints: &[GeometricConstraint],
    bindings: &ConstraintBindingMap,
) -> Vec<ConstraintConformance> {
    constraints
        .iter()
        .filter(|constraint| {
            constraint.execution_policy == FamilyRuleExecutionPolicy::RuntimeOnly
                || !bindings.contains_key(&constraint.id)
        })
        .map(|constraint| match bindings.get(&constraint.id) {
            Some(binding)
                if constraint.execution_policy == FamilyRuleExecutionPolicy::RuntimeOnly =>
            {
                deferred_row(constraint, Some(binding.binding_kind()))
            }
            None => unbound_constraint_row(constraint),
            Some(_) => row_for(constraint, None),
        })
        .collect()
}

fn unbound_constraint_row(constraint: &GeometricConstraint) -> ConstraintConformance {
    match constraint.execution_policy {
        FamilyRuleExecutionPolicy::RuntimeOnly => deferred_row(
            constraint,
            Some(ConstraintBindingKind::AdapterDeferredMetadata),
        ),
        FamilyRuleExecutionPolicy::Required => {
            let mut row = row_for(constraint, None);
            row.status = ConformanceStatus::Unsupported;
            row.issue_codes
                .push("required_constraint_binding_missing".to_owned());
            row
        }
        FamilyRuleExecutionPolicy::Advisory => {
            let mut row = row_for(constraint, None);
            row.status = ConformanceStatus::NotEvaluated;
            row.issue_codes
                .push("advisory_constraint_binding_missing".to_owned());
            row
        }
    }
}

fn deferred_row(
    constraint: &GeometricConstraint,
    binding: Option<ConstraintBindingKind>,
) -> ConstraintConformance {
    let mut row = row_for(constraint, binding);
    row.status = ConformanceStatus::Deferred;
    row.issue_codes
        .push("constraint_runtime_deferred".to_owned());
    row
}

fn row_for(
    constraint: &GeometricConstraint,
    binding: Option<ConstraintBindingKind>,
) -> ConstraintConformance {
    ConstraintConformance {
        constraint_id: constraint.id.clone(),
        roles: constraint.roles.clone(),
        kind: constraint.kind.clone(),
        binding,
        policy: constraint.execution_policy,
        measurements: Vec::new(),
        status: ConformanceStatus::NotEvaluated,
        issue_codes: Vec::new(),
    }
}

fn passed(mut row: ConstraintConformance) -> ConstraintConformance {
    row.status = ConformanceStatus::Passed;
    row
}

fn failed(mut row: ConstraintConformance, issue_code: &str) -> ConstraintConformance {
    row.status = ConformanceStatus::Failed;
    row.issue_codes.push(issue_code.to_owned());
    row
}

fn missing(mut row: ConstraintConformance, issue_code: &str) -> ConstraintConformance {
    row.status = ConformanceStatus::Missing;
    row.issue_codes.push(issue_code.to_owned());
    row
}

fn first_two_roles(roles: &[String]) -> Option<(&str, &str)> {
    roles
        .first()
        .zip(roles.get(1))
        .map(|(first, second)| (first.as_str(), second.as_str()))
}

fn role_bounds(recipe: &AssetRecipe, artifact: &AssetArtifact, role: &str) -> Option<MeshBounds> {
    union_bounds(
        parts_for_role(recipe, artifact, role)
            .into_iter()
            .map(|part| part.world_mesh.bounds),
    )
}

fn union_bounds(bounds: impl IntoIterator<Item = MeshBounds>) -> Option<MeshBounds> {
    let mut bounds = bounds.into_iter();
    let first = bounds.next()?;
    Some(bounds.fold(first, |combined, bounds| combined.union(&bounds)))
}

fn parts_for_role<'a>(
    recipe: &AssetRecipe,
    artifact: &'a AssetArtifact,
    role: &str,
) -> Vec<&'a CompiledPart> {
    artifact
        .compiled_parts
        .iter()
        .filter(|part| compiled_part_matches_role(recipe, part, role))
        .collect()
}

fn compiled_part_matches_role(recipe: &AssetRecipe, part: &CompiledPart, role: &str) -> bool {
    let role_tag = role_tag(role);
    let definition_match = recipe
        .definitions
        .get(&part.definition_id)
        .is_some_and(|definition| {
            definition.tags.contains(role) || definition.tags.contains(&role_tag)
        });
    let instance_match = recipe
        .instances
        .get(&part.instance_id)
        .or_else(|| {
            part.prototype_instance_id
                .and_then(|prototype| recipe.instances.get(&prototype))
        })
        .is_some_and(|instance| {
            instance.enabled && (instance.tags.contains(role) || instance.tags.contains(&role_tag))
        });
    definition_match || instance_match
}

fn role_tag(role: &str) -> String {
    format!("role:{role}")
}

fn role_pairs<'a>(
    recipe: &AssetRecipe,
    artifact: &'a AssetArtifact,
    roles: &[String],
) -> Option<Vec<(&'a CompiledPart, &'a CompiledPart)>> {
    match roles {
        [role] => {
            let parts = parts_for_role(recipe, artifact, role);
            if parts.is_empty() {
                return None;
            }
            let mut pairs = Vec::new();
            for left_index in 0..parts.len() {
                for right in &parts[left_index + 1..] {
                    pairs.push((parts[left_index], *right));
                }
            }
            Some(pairs)
        }
        [first, second, ..] => {
            let left_parts = parts_for_role(recipe, artifact, first);
            let right_parts = parts_for_role(recipe, artifact, second);
            if left_parts.is_empty() || right_parts.is_empty() {
                return None;
            }
            Some(
                left_parts
                    .iter()
                    .flat_map(|left| {
                        right_parts
                            .iter()
                            .filter(move |right| left.instance_id != right.instance_id)
                            .map(move |right| (*left, *right))
                    })
                    .collect(),
            )
        }
        [] => None,
    }
}

fn bounds_extent(bounds: &MeshBounds) -> [f32; 3] {
    [
        bounds.max[0] - bounds.min[0],
        bounds.max[1] - bounds.min[1],
        bounds.max[2] - bounds.min[2],
    ]
}

fn bounds_distance(left: &MeshBounds, right: &MeshBounds) -> f32 {
    let mut sum = 0.0_f32;
    for axis in 0..3 {
        let gap = if left.max[axis] < right.min[axis] {
            right.min[axis] - left.max[axis]
        } else if right.max[axis] < left.min[axis] {
            left.min[axis] - right.max[axis]
        } else {
            0.0
        };
        sum += gap * gap;
    }
    sum.sqrt()
}

fn bounds_contains(container: &MeshBounds, contained: &MeshBounds, epsilon: f32) -> bool {
    (0..3).all(|axis| {
        contained.min[axis] + epsilon >= container.min[axis]
            && contained.max[axis] <= container.max[axis] + epsilon
    })
}

fn bounds_supports(
    supported: &MeshBounds,
    support: &MeshBounds,
    vertical_axis: usize,
    max_clearance: f32,
) -> bool {
    if (0..3).filter(|axis| *axis != vertical_axis).any(|axis| {
        supported.max[axis] < support.min[axis] || support.max[axis] < supported.min[axis]
    }) {
        return false;
    }
    let vertical_gap = supported.min[vertical_axis] - support.max[vertical_axis];
    (-EPSILON..=max_clearance + EPSILON).contains(&vertical_gap)
}

fn recipe_instances_attached(
    recipe: &AssetRecipe,
    first: PartInstanceId,
    second: PartInstanceId,
) -> bool {
    child_attached_to_parent(recipe, first, second)
        || child_attached_to_parent(recipe, second, first)
}

fn child_attached_to_parent(
    recipe: &AssetRecipe,
    parent: PartInstanceId,
    child: PartInstanceId,
) -> bool {
    recipe.instances.get(&child).is_some_and(|instance| {
        instance.parent == Some(parent)
            || instance
                .attachment
                .as_ref()
                .is_some_and(|attachment| attachment.parent_instance == parent)
    })
}

fn frames_axes_align(
    parent: &orchard_asset::Frame3,
    child: &orchard_asset::Frame3,
    max_axis_angle_degrees: f32,
) -> bool {
    if !max_axis_angle_degrees.is_finite() {
        return false;
    }
    let min_dot = max_axis_angle_degrees.to_radians().cos();
    [
        (parent.x_axis, child.x_axis),
        (parent.y_axis, child.y_axis),
        (parent.z_axis, child.z_axis),
    ]
    .into_iter()
    .all(|(left, right)| {
        normalize(left)
            .zip(normalize(right))
            .is_some_and(|(left, right)| dot(left, right) >= min_dot)
    })
}

fn distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    let delta = [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
    dot(delta, delta).sqrt()
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(vector, vector).sqrt();
    (length.is_finite() && length > EPSILON).then_some([
        vector[0] / length,
        vector[1] / length,
        vector[2] / length,
    ])
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
