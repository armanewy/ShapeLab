#![forbid(unsafe_code)]

//! Low-level legacy document and modeling conventions for Shape Lab.
//!
//! Boundary contract:
//! - `shape-core` owns useful low-level geometry conventions and legacy
//!   implicit/SDF `ShapeDocument` compatibility.
//! - `shape-core::ShapeDocument` is not the canonical A-J product IR for
//!   Object Orchard.
//! - New product semantics for `AssetRecipe`, ObjectPlan approval, authoring
//!   operation logs, relationship contracts, pattern contracts, surface
//!   workflow, terrain readiness, collision, motion, export readiness, public
//!   catalog publishing, Godot-ready status, or game-ready status belong in
//!   `shape-asset` / future `shape-orchard-ir` contracts, not in this crate.
//! - Product-visible controls should eventually route through typed authoring
//!   operations over the semantic asset lane rather than raw `ShapeDocument`
//!   mutation.
//!
//! Coordinate contract:
//! - Geometry uses `f32`.
//! - Coordinates are right handed.
//! - Positive Y is up.
//! - Primitive local forward is negative Z.
//! - Capsule and cylinder primary axes are local Y.
//! - SDF values are negative inside and positive outside.
//! - Mesh triangles are counterclockwise when viewed from outside.
//! - Document rotations are stored as XYZ Euler degrees.
//! - Nonuniform transformed SDF distances are scaled by the smallest absolute
//!   scale component. This preserves the zero set but is not an exact distance.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Scalar type used by MVP geometry code.
pub type Scalar = f32;

const SCHEMA_VERSION: u32 = 1;
const MIN_PARAMETER_DIMENSION: Scalar = 0.01;
const MAX_PARAMETER_DIMENSION: Scalar = 5.0;
const MIN_SCALE_COMPONENT: Scalar = 1.0e-5;
const EDIT_PRECONDITION_TOLERANCE: Scalar = 1.0e-4;

/// Stable identifier for a shape node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// Stable identifier for a project revision.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RevisionId(pub u64);

/// Stable identifier for a generated candidate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CandidateId(pub u64);

/// Translation, Euler rotation, and scale stored in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3 {
    /// World-space translation.
    pub translation: Vec3,
    /// XYZ Euler rotation in degrees.
    pub rotation_degrees: Vec3,
    /// Per-axis scale.
    pub scale: Vec3,
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_degrees: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform3 {
    /// Build a transform matrix from the document convention.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation_degrees.x.to_radians(),
            self.rotation_degrees.y.to_radians(),
            self.rotation_degrees.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(self.scale, rotation, self.translation)
    }
}

/// Axis-aligned bounding box.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb {
    /// Return an empty box.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(Scalar::INFINITY),
            max: Vec3::splat(Scalar::NEG_INFINITY),
        }
    }

    /// Return true when no finite volume is represented.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Return the union of two boxes.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Return the intersection of two boxes.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let result = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        if result.is_empty() {
            Self::empty()
        } else {
            result
        }
    }

    /// Return the box expanded equally in every direction.
    #[must_use]
    pub fn expanded(&self, amount: Scalar) -> Self {
        if self.is_empty() {
            return *self;
        }
        let delta = Vec3::splat(amount);
        Self {
            min: self.min - delta,
            max: self.max + delta,
        }
    }

    /// Return the center of the box, or zero for an empty box.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        if self.is_empty() {
            Vec3::ZERO
        } else {
            (self.min + self.max) * 0.5
        }
    }

    /// Return the size of the box, or zero for an empty box.
    #[must_use]
    pub fn extent(&self) -> Vec3 {
        if self.is_empty() {
            Vec3::ZERO
        } else {
            self.max - self.min
        }
    }

    /// Return true when this box fully contains `other`.
    #[must_use]
    pub fn contains_aabb(&self, other: &Self) -> bool {
        other.is_empty()
            || (!self.is_empty()
                && self.min.cmple(other.min).all()
                && self.max.cmpge(other.max).all())
    }

    /// Conservatively transform the eight corners and return their AABB.
    #[must_use]
    pub fn transformed(&self, transform: &Transform3) -> Self {
        if self.is_empty() {
            return *self;
        }
        let matrix = transform.matrix();
        let mut result = Self::empty();
        for x in [self.min.x, self.max.x] {
            for y in [self.min.y, self.max.y] {
                for z in [self.min.z, self.max.z] {
                    let p = matrix.transform_point3(Vec3::new(x, y, z));
                    result = result.union(&Self { min: p, max: p });
                }
            }
        }
        result
    }
}

/// Primitive node kinds supported by the implicit MVP backend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveKind {
    /// Sphere centered at the local origin.
    Sphere { radius: Scalar },
    /// Rounded box centered at the local origin.
    RoundedBox {
        half_extents: Vec3,
        roundness: Scalar,
    },
    /// Capsule along local Y.
    Capsule { half_length: Scalar, radius: Scalar },
    /// Capped cylinder along local Y.
    Cylinder {
        half_height: Scalar,
        radius: Scalar,
        roundness: Scalar,
    },
    /// Torus around local Y.
    Torus {
        major_radius: Scalar,
        minor_radius: Scalar,
    },
}

/// Shape graph operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Analytic primitive.
    Primitive(PrimitiveKind),
    /// Boolean union.
    Union { children: Vec<NodeId> },
    /// Smooth Boolean union.
    SmoothUnion {
        children: Vec<NodeId>,
        smoothness: Scalar,
    },
    /// Boolean difference.
    Difference {
        base: NodeId,
        subtractors: Vec<NodeId>,
    },
    /// Boolean intersection.
    Intersection { children: Vec<NodeId> },
}

/// One named node in the shape graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeNode {
    /// Stable node ID.
    pub id: NodeId,
    /// Human-facing label.
    pub name: String,
    /// Free-form tags.
    pub tags: BTreeSet<String>,
    /// Disabled nodes remain in the document but are ignored by evaluators.
    pub enabled: bool,
    /// Node transform.
    pub transform: Transform3,
    /// Node kind.
    pub kind: NodeKind,
}

/// Canonical scalar parameter path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ParamPath {
    /// Node owning the parameter.
    pub node: NodeId,
    /// Canonical key, such as `primitive.radius`.
    pub key: String,
}

/// Human-facing parameter grouping.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ParamGroup {
    /// Shape dimensions.
    Form,
    /// Translation.
    Placement,
    /// Rotation.
    Rotation,
    /// Scale.
    Scale,
    /// CSG blend controls.
    Blend,
}

/// Description of one editable scalar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDescriptor {
    /// Parameter path.
    pub path: ParamPath,
    /// Human-facing label.
    pub label: String,
    /// UI grouping.
    pub group: ParamGroup,
    /// Minimum permitted value.
    pub minimum: Scalar,
    /// Maximum permitted value.
    pub maximum: Scalar,
    /// Suggested UI step.
    pub step: Scalar,
    /// Standard deviation for mutation.
    pub mutation_sigma: Scalar,
}

/// Authoritative shape document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeDocument {
    /// Schema version for project compatibility.
    pub schema_version: u32,
    /// Human-facing title.
    pub title: String,
    /// Root node.
    pub root: NodeId,
    /// Shape graph nodes.
    pub nodes: BTreeMap<NodeId, ShapeNode>,
    /// Next available node ID.
    pub next_node_id: u64,
    /// Locked scalar parameter paths.
    pub locks: BTreeSet<ParamPath>,
}

impl ShapeDocument {
    /// Create a document around an existing root node.
    #[must_use]
    pub fn new(title: impl Into<String>, root: ShapeNode) -> Self {
        let root_id = root.id;
        let next_node_id = root_id.0.saturating_add(1);
        let mut nodes = BTreeMap::new();
        nodes.insert(root_id, root);
        Self {
            schema_version: SCHEMA_VERSION,
            title: title.into(),
            root: root_id,
            nodes,
            next_node_id,
            locks: BTreeSet::new(),
        }
    }

    /// Allocate the next stable node ID from this document.
    pub fn allocate_node_id(&mut self) -> NodeId {
        allocate_node_id(self)
    }

    /// Insert an already-built node, rejecting duplicate IDs.
    pub fn insert_node(&mut self, node: ShapeNode) -> Result<(), CoreError> {
        insert_node(self, node)
    }

    /// Allocate and insert a default-enabled node with an identity transform.
    pub fn insert_new_node(&mut self, name: impl Into<String>, kind: NodeKind) -> NodeId {
        let id = self.allocate_node_id();
        let node = ShapeNode {
            id,
            name: name.into(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind,
        };
        self.nodes.insert(id, node);
        id
    }
}

/// Scalar assignment recorded in an edit program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetScalarEdit {
    /// Edited parameter.
    pub path: ParamPath,
    /// Value expected before applying the edit.
    pub before: Scalar,
    /// New value.
    pub after: Scalar,
}

/// Replayable semantic edit program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditProgram {
    /// Human-facing label.
    pub label: String,
    /// Deterministic seed associated with the edit.
    pub seed: u64,
    /// Scalar operations.
    pub operations: Vec<SetScalarEdit>,
}

/// Validation issue emitted by core checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Optional node related to the issue.
    pub node: Option<NodeId>,
    /// Optional parameter related to the issue.
    pub path: Option<ParamPath>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Collection of validation issues.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Discovered issues.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Return true when no issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Error type for core document APIs.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The requested node does not exist.
    #[error("unknown node {0:?}")]
    UnknownNode(NodeId),
    /// A node with that ID is already present.
    #[error("duplicate node {0:?}")]
    DuplicateNode(NodeId),
    /// The requested scalar path does not exist for the node kind.
    #[error("unknown scalar path {0:?}")]
    UnknownScalar(ParamPath),
    /// A non-finite scalar was supplied.
    #[error("non-finite scalar for {0:?}: {1}")]
    NonFiniteScalar(ParamPath, Scalar),
    /// Recorded edit precondition did not match the current document.
    #[error("edit precondition failed for {path:?}: expected {expected}, found {actual}")]
    EditPreconditionFailed {
        /// Edited path.
        path: ParamPath,
        /// Expected current value.
        expected: Scalar,
        /// Actual current value.
        actual: Scalar,
    },
    /// The edited document failed validation.
    #[error("document validation failed")]
    ValidationFailed(ValidationReport),
}

/// Validate document invariants and collect every discoverable issue.
#[must_use]
pub fn validate_document(document: &ShapeDocument) -> ValidationReport {
    let mut report = ValidationReport::default();

    if !document.nodes.contains_key(&document.root) {
        push_issue(
            &mut report,
            Some(document.root),
            None,
            "missing_root",
            "Document root does not exist.",
        );
    }

    validate_next_node_id(document, &mut report);

    for (id, node) in &document.nodes {
        if node.id != *id {
            push_issue(
                &mut report,
                Some(*id),
                None,
                "node_id_mismatch",
                "Node map key and node ID differ.",
            );
        }
        validate_transform(*id, &node.transform, &mut report);
        validate_kind_values(*id, &node.kind, &mut report);
        validate_references(document, *id, &node.kind, &mut report);
        validate_parameter_descriptors(document, node, &mut report);
    }

    validate_locks(document, &mut report);
    detect_cycles(document, &mut report);

    report
}

fn validate_next_node_id(document: &ShapeDocument, report: &mut ValidationReport) {
    let Some(max_id) = document.nodes.keys().map(|id| id.0).max() else {
        return;
    };
    if document.next_node_id <= max_id {
        push_issue(
            report,
            None,
            None,
            "next_node_id_not_fresh",
            "Document next_node_id would reallocate an existing node ID.",
        );
    }
}

fn validate_transform(node: NodeId, transform: &Transform3, report: &mut ValidationReport) {
    validate_finite_component(
        node,
        "transform.translation.x",
        transform.translation.x,
        report,
    );
    validate_finite_component(
        node,
        "transform.translation.y",
        transform.translation.y,
        report,
    );
    validate_finite_component(
        node,
        "transform.translation.z",
        transform.translation.z,
        report,
    );
    validate_finite_component(
        node,
        "transform.rotation_degrees.x",
        transform.rotation_degrees.x,
        report,
    );
    validate_finite_component(
        node,
        "transform.rotation_degrees.y",
        transform.rotation_degrees.y,
        report,
    );
    validate_finite_component(
        node,
        "transform.rotation_degrees.z",
        transform.rotation_degrees.z,
        report,
    );
    validate_finite_component(node, "transform.scale.x", transform.scale.x, report);
    validate_finite_component(node, "transform.scale.y", transform.scale.y, report);
    validate_finite_component(node, "transform.scale.z", transform.scale.z, report);

    for (key, value) in [
        ("transform.scale.x", transform.scale.x),
        ("transform.scale.y", transform.scale.y),
        ("transform.scale.z", transform.scale.z),
    ] {
        if value.is_finite() && value.abs() <= MIN_SCALE_COMPONENT {
            push_issue(
                report,
                Some(node),
                Some(path(node, key)),
                "near_zero_scale",
                "Transform scale component is zero or too close to zero.",
            );
        }
    }
}

fn validate_kind_values(node: NodeId, kind: &NodeKind, report: &mut ValidationReport) {
    match kind {
        NodeKind::Primitive(primitive) => validate_primitive_values(node, primitive, report),
        NodeKind::SmoothUnion { smoothness, .. } => {
            validate_finite_component(node, "csg.smoothness", *smoothness, report);
            if smoothness.is_finite() && *smoothness < 0.0 {
                push_issue(
                    report,
                    Some(node),
                    Some(path(node, "csg.smoothness")),
                    "negative_smoothness",
                    "Smooth-union smoothness cannot be negative.",
                );
            }
        }
        NodeKind::Union { .. } | NodeKind::Difference { .. } | NodeKind::Intersection { .. } => {}
    }
}

fn validate_primitive_values(
    node: NodeId,
    primitive: &PrimitiveKind,
    report: &mut ValidationReport,
) {
    match primitive {
        PrimitiveKind::Sphere { radius } => {
            validate_positive_dimension(node, "primitive.radius", *radius, report);
        }
        PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        } => {
            validate_positive_dimension(node, "primitive.half_extents.x", half_extents.x, report);
            validate_positive_dimension(node, "primitive.half_extents.y", half_extents.y, report);
            validate_positive_dimension(node, "primitive.half_extents.z", half_extents.z, report);
            validate_finite_component(node, "primitive.roundness", *roundness, report);
            if roundness.is_finite() && *roundness < 0.0 {
                push_issue(
                    report,
                    Some(node),
                    Some(path(node, "primitive.roundness")),
                    "negative_roundness",
                    "Rounded-box roundness cannot be negative.",
                );
            }
            let smallest_extent = half_extents.x.min(half_extents.y).min(half_extents.z);
            if smallest_extent.is_finite() && roundness.is_finite() && *roundness > smallest_extent
            {
                push_issue(
                    report,
                    Some(node),
                    Some(path(node, "primitive.roundness")),
                    "roundness_too_large",
                    "Rounded-box roundness cannot exceed the smallest half extent.",
                );
            }
        }
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => {
            validate_positive_dimension(node, "primitive.half_length", *half_length, report);
            validate_positive_dimension(node, "primitive.radius", *radius, report);
        }
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            roundness,
        } => {
            validate_positive_dimension(node, "primitive.half_height", *half_height, report);
            validate_positive_dimension(node, "primitive.radius", *radius, report);
            validate_finite_component(node, "primitive.roundness", *roundness, report);
            if roundness.is_finite() && *roundness < 0.0 {
                push_issue(
                    report,
                    Some(node),
                    Some(path(node, "primitive.roundness")),
                    "negative_roundness",
                    "Cylinder roundness cannot be negative.",
                );
            }
        }
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => {
            validate_positive_dimension(node, "primitive.major_radius", *major_radius, report);
            validate_positive_dimension(node, "primitive.minor_radius", *minor_radius, report);
            if major_radius.is_finite() && minor_radius.is_finite() && minor_radius >= major_radius
            {
                push_issue(
                    report,
                    Some(node),
                    Some(path(node, "primitive.minor_radius")),
                    "torus_minor_radius_too_large",
                    "Torus minor radius must be smaller than the major radius.",
                );
            }
        }
    }
}

fn validate_positive_dimension(
    node: NodeId,
    key: &'static str,
    value: Scalar,
    report: &mut ValidationReport,
) {
    validate_finite_component(node, key, value, report);
    if value.is_finite() && value <= 0.0 {
        push_issue(
            report,
            Some(node),
            Some(path(node, key)),
            "invalid_dimension",
            "Primitive dimensions must be greater than zero.",
        );
    }
}

fn validate_finite_component(
    node: NodeId,
    key: &'static str,
    value: Scalar,
    report: &mut ValidationReport,
) {
    if !value.is_finite() {
        push_issue(
            report,
            Some(node),
            Some(path(node, key)),
            "non_finite",
            "Scalar value must be finite.",
        );
    }
}

fn validate_references(
    document: &ShapeDocument,
    node: NodeId,
    kind: &NodeKind,
    report: &mut ValidationReport,
) {
    match kind {
        NodeKind::Union { children } | NodeKind::Intersection { children } => {
            if children.is_empty() {
                push_issue(
                    report,
                    Some(node),
                    None,
                    "empty_combiner",
                    "Combiner nodes need at least one child.",
                );
            }
        }
        NodeKind::SmoothUnion { children, .. } => {
            if children.is_empty() {
                push_issue(
                    report,
                    Some(node),
                    None,
                    "empty_combiner",
                    "Smooth-union nodes need at least one child.",
                );
            }
        }
        NodeKind::Primitive(_) | NodeKind::Difference { .. } => {}
    }

    for referenced in referenced_nodes(kind) {
        if !document.nodes.contains_key(&referenced) {
            push_issue(
                report,
                Some(node),
                None,
                "dangling_reference",
                format!("Referenced node {referenced:?} does not exist."),
            );
        }
    }
}

fn validate_parameter_descriptors(
    document: &ShapeDocument,
    node: &ShapeNode,
    report: &mut ValidationReport,
) {
    for descriptor in enumerate_node_parameters(node) {
        if !descriptor.minimum.is_finite()
            || !descriptor.maximum.is_finite()
            || !descriptor.step.is_finite()
            || !descriptor.mutation_sigma.is_finite()
            || descriptor.minimum >= descriptor.maximum
            || descriptor.step <= 0.0
            || descriptor.mutation_sigma < 0.0
        {
            push_issue(
                report,
                Some(node.id),
                Some(descriptor.path.clone()),
                "impossible_parameter_range",
                "Parameter descriptor has an impossible range.",
            );
        }

        match get_scalar(document, &descriptor.path) {
            Ok(value) if value.is_finite() => {}
            Ok(_) => push_issue(
                report,
                Some(node.id),
                Some(descriptor.path.clone()),
                "non_finite",
                "Scalar value must be finite.",
            ),
            Err(_) => push_issue(
                report,
                Some(node.id),
                Some(descriptor.path.clone()),
                "impossible_parameter_range",
                "Parameter descriptor points at an unreadable scalar.",
            ),
        }
    }
}

fn validate_locks(document: &ShapeDocument, report: &mut ValidationReport) {
    for lock in &document.locks {
        if get_scalar(document, lock).is_err() {
            push_issue(
                report,
                Some(lock.node),
                Some(lock.clone()),
                "unknown_lock_path",
                "Lock points at an unknown scalar parameter.",
            );
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

fn detect_cycles(document: &ShapeDocument, report: &mut ValidationReport) {
    let mut states = BTreeMap::<NodeId, VisitState>::new();
    let mut stack = Vec::<NodeId>::new();
    let mut reported_edges = BTreeSet::<(NodeId, NodeId)>::new();

    for node in document.nodes.keys().copied() {
        detect_cycles_from(
            document,
            node,
            &mut states,
            &mut stack,
            &mut reported_edges,
            report,
        );
    }
}

fn detect_cycles_from(
    document: &ShapeDocument,
    node: NodeId,
    states: &mut BTreeMap<NodeId, VisitState>,
    stack: &mut Vec<NodeId>,
    reported_edges: &mut BTreeSet<(NodeId, NodeId)>,
    report: &mut ValidationReport,
) {
    match states.get(&node) {
        Some(VisitState::Visited) => return,
        Some(VisitState::Visiting) => {
            report_cycle(node, node, stack, reported_edges, report);
            return;
        }
        None => {}
    }

    states.insert(node, VisitState::Visiting);
    stack.push(node);

    let Some(shape_node) = document.nodes.get(&node) else {
        stack.pop();
        states.insert(node, VisitState::Visited);
        return;
    };

    for child in referenced_nodes(&shape_node.kind) {
        if !document.nodes.contains_key(&child) {
            continue;
        }
        match states.get(&child) {
            Some(VisitState::Visiting) => {
                report_cycle(node, child, stack, reported_edges, report);
            }
            Some(VisitState::Visited) => {}
            None => detect_cycles_from(document, child, states, stack, reported_edges, report),
        }
    }

    stack.pop();
    states.insert(node, VisitState::Visited);
}

fn report_cycle(
    parent: NodeId,
    child: NodeId,
    stack: &[NodeId],
    reported_edges: &mut BTreeSet<(NodeId, NodeId)>,
    report: &mut ValidationReport,
) {
    if !reported_edges.insert((parent, child)) {
        return;
    }

    let cycle = stack.iter().position(|id| *id == child).map_or_else(
        || vec![child],
        |start| {
            let mut ids = stack[start..].to_vec();
            ids.push(child);
            ids
        },
    );
    let cycle_text = cycle
        .iter()
        .map(|id| id.0.to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    push_issue(
        report,
        Some(parent),
        None,
        "cycle",
        format!("Graph cycle detected: {cycle_text}."),
    );
}

/// Enumerate scalar parameters in stable node/key order.
#[must_use]
pub fn enumerate_parameters(document: &ShapeDocument) -> Vec<ParamDescriptor> {
    let mut result = Vec::new();
    for node in document.nodes.values() {
        result.extend(enumerate_node_parameters(node));
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

fn enumerate_node_parameters(node: &ShapeNode) -> Vec<ParamDescriptor> {
    let placement = ParamLimits::new(-5.0, 5.0, 0.01, 0.15);
    let rotation = ParamLimits::new(-360.0, 360.0, 1.0, 8.0);
    let scale = ParamLimits::new(0.05, 10.0, 0.01, 0.1);
    let form = ParamLimits::new(MIN_PARAMETER_DIMENSION, MAX_PARAMETER_DIMENSION, 0.01, 0.12);
    let mut result = vec![
        descriptor(
            node.id,
            "transform.translation.x",
            "Position X",
            ParamGroup::Placement,
            placement,
        ),
        descriptor(
            node.id,
            "transform.translation.y",
            "Position Y",
            ParamGroup::Placement,
            placement,
        ),
        descriptor(
            node.id,
            "transform.translation.z",
            "Position Z",
            ParamGroup::Placement,
            placement,
        ),
        descriptor(
            node.id,
            "transform.rotation_degrees.x",
            "Rotation X",
            ParamGroup::Rotation,
            rotation,
        ),
        descriptor(
            node.id,
            "transform.rotation_degrees.y",
            "Rotation Y",
            ParamGroup::Rotation,
            rotation,
        ),
        descriptor(
            node.id,
            "transform.rotation_degrees.z",
            "Rotation Z",
            ParamGroup::Rotation,
            rotation,
        ),
        descriptor(
            node.id,
            "transform.scale.x",
            "Scale X",
            ParamGroup::Scale,
            scale,
        ),
        descriptor(
            node.id,
            "transform.scale.y",
            "Scale Y",
            ParamGroup::Scale,
            scale,
        ),
        descriptor(
            node.id,
            "transform.scale.z",
            "Scale Z",
            ParamGroup::Scale,
            scale,
        ),
    ];

    match &node.kind {
        NodeKind::Primitive(PrimitiveKind::Sphere { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.radius",
                "Radius",
                ParamGroup::Form,
                form,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        }) => {
            let half_extent_minimum = (*roundness).max(MIN_PARAMETER_DIMENSION);
            let half_extent_limits =
                ParamLimits::new(half_extent_minimum, MAX_PARAMETER_DIMENSION, 0.01, 0.12);
            result.push(descriptor(
                node.id,
                "primitive.half_extents.x",
                "Half Width",
                ParamGroup::Form,
                half_extent_limits,
            ));
            result.push(descriptor(
                node.id,
                "primitive.half_extents.y",
                "Half Height",
                ParamGroup::Form,
                half_extent_limits,
            ));
            result.push(descriptor(
                node.id,
                "primitive.half_extents.z",
                "Half Depth",
                ParamGroup::Form,
                half_extent_limits,
            ));
            let max_roundness = half_extents
                .x
                .min(half_extents.y)
                .min(half_extents.z)
                .min(2.0);
            result.push(descriptor(
                node.id,
                "primitive.roundness",
                "Roundness",
                ParamGroup::Form,
                ParamLimits::new(0.0, max_roundness, 0.01, 0.05),
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Capsule { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.half_length",
                "Half Length",
                ParamGroup::Form,
                form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.radius",
                "Radius",
                ParamGroup::Form,
                form,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height,
            radius,
            ..
        }) => {
            result.push(descriptor(
                node.id,
                "primitive.half_height",
                "Half Height",
                ParamGroup::Form,
                form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.radius",
                "Radius",
                ParamGroup::Form,
                form,
            ));
            let max_roundness = (*half_height).min(*radius).min(2.0);
            result.push(descriptor(
                node.id,
                "primitive.roundness",
                "Roundness",
                ParamGroup::Form,
                ParamLimits::new(0.0, max_roundness, 0.01, 0.05),
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        }) => {
            let major_minimum = (*minor_radius + MIN_PARAMETER_DIMENSION)
                .clamp(MIN_PARAMETER_DIMENSION, MAX_PARAMETER_DIMENSION);
            result.push(descriptor(
                node.id,
                "primitive.major_radius",
                "Major Radius",
                ParamGroup::Form,
                ParamLimits::new(major_minimum, MAX_PARAMETER_DIMENSION, 0.01, 0.12),
            ));
            let minor_maximum =
                (*major_radius - MIN_PARAMETER_DIMENSION).min(MAX_PARAMETER_DIMENSION);
            result.push(descriptor(
                node.id,
                "primitive.minor_radius",
                "Minor Radius",
                ParamGroup::Form,
                ParamLimits::new(MIN_PARAMETER_DIMENSION, minor_maximum, 0.01, 0.08),
            ));
        }
        NodeKind::SmoothUnion { .. } => {
            result.push(descriptor(
                node.id,
                "csg.smoothness",
                "Blend Smoothness",
                ParamGroup::Blend,
                ParamLimits::new(0.0, 2.0, 0.01, 0.05),
            ));
        }
        NodeKind::Union { .. } | NodeKind::Difference { .. } | NodeKind::Intersection { .. } => {}
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

#[derive(Debug, Copy, Clone)]
struct ParamLimits {
    minimum: Scalar,
    maximum: Scalar,
    step: Scalar,
    mutation_sigma: Scalar,
}

impl ParamLimits {
    const fn new(minimum: Scalar, maximum: Scalar, step: Scalar, mutation_sigma: Scalar) -> Self {
        Self {
            minimum,
            maximum,
            step,
            mutation_sigma,
        }
    }
}

fn descriptor(
    node: NodeId,
    key: &str,
    label: &str,
    group: ParamGroup,
    limits: ParamLimits,
) -> ParamDescriptor {
    ParamDescriptor {
        path: ParamPath {
            node,
            key: key.to_owned(),
        },
        label: label.to_owned(),
        group,
        minimum: limits.minimum,
        maximum: limits.maximum,
        step: limits.step,
        mutation_sigma: limits.mutation_sigma,
    }
}

/// Read a scalar parameter.
pub fn get_scalar(document: &ShapeDocument, path: &ParamPath) -> Result<Scalar, CoreError> {
    let node = document
        .nodes
        .get(&path.node)
        .ok_or(CoreError::UnknownNode(path.node))?;
    scalar_from_node(node, &path.key).ok_or_else(|| CoreError::UnknownScalar(path.clone()))
}

fn scalar_from_node(node: &ShapeNode, key: &str) -> Option<Scalar> {
    match key {
        "transform.translation.x" => Some(node.transform.translation.x),
        "transform.translation.y" => Some(node.transform.translation.y),
        "transform.translation.z" => Some(node.transform.translation.z),
        "transform.rotation_degrees.x" => Some(node.transform.rotation_degrees.x),
        "transform.rotation_degrees.y" => Some(node.transform.rotation_degrees.y),
        "transform.rotation_degrees.z" => Some(node.transform.rotation_degrees.z),
        "transform.scale.x" => Some(node.transform.scale.x),
        "transform.scale.y" => Some(node.transform.scale.y),
        "transform.scale.z" => Some(node.transform.scale.z),
        "primitive.radius" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::Sphere { radius })
            | NodeKind::Primitive(PrimitiveKind::Capsule { radius, .. })
            | NodeKind::Primitive(PrimitiveKind::Cylinder { radius, .. }) => Some(*radius),
            _ => None,
        },
        "primitive.half_extents.x" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                Some(half_extents.x)
            }
            _ => None,
        },
        "primitive.half_extents.y" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                Some(half_extents.y)
            }
            _ => None,
        },
        "primitive.half_extents.z" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                Some(half_extents.z)
            }
            _ => None,
        },
        "primitive.roundness" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { roundness, .. })
            | NodeKind::Primitive(PrimitiveKind::Cylinder { roundness, .. }) => Some(*roundness),
            _ => None,
        },
        "primitive.half_length" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::Capsule { half_length, .. }) => Some(*half_length),
            _ => None,
        },
        "primitive.half_height" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::Cylinder { half_height, .. }) => Some(*half_height),
            _ => None,
        },
        "primitive.major_radius" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::Torus { major_radius, .. }) => Some(*major_radius),
            _ => None,
        },
        "primitive.minor_radius" => match &node.kind {
            NodeKind::Primitive(PrimitiveKind::Torus { minor_radius, .. }) => Some(*minor_radius),
            _ => None,
        },
        "csg.smoothness" => match &node.kind {
            NodeKind::SmoothUnion { smoothness, .. } => Some(*smoothness),
            _ => None,
        },
        _ => None,
    }
}

/// Set a scalar parameter, leaving the document unchanged on failure.
pub fn set_scalar(
    document: &mut ShapeDocument,
    path: &ParamPath,
    value: Scalar,
) -> Result<(), CoreError> {
    if !value.is_finite() {
        return Err(CoreError::NonFiniteScalar(path.clone(), value));
    }
    let mut clone = document.clone();
    let node = clone
        .nodes
        .get_mut(&path.node)
        .ok_or(CoreError::UnknownNode(path.node))?;
    if !set_scalar_on_node(node, &path.key, value) {
        return Err(CoreError::UnknownScalar(path.clone()));
    }
    *document = clone;
    Ok(())
}

fn set_scalar_on_node(node: &mut ShapeNode, key: &str, value: Scalar) -> bool {
    match key {
        "transform.translation.x" => node.transform.translation.x = value,
        "transform.translation.y" => node.transform.translation.y = value,
        "transform.translation.z" => node.transform.translation.z = value,
        "transform.rotation_degrees.x" => node.transform.rotation_degrees.x = value,
        "transform.rotation_degrees.y" => node.transform.rotation_degrees.y = value,
        "transform.rotation_degrees.z" => node.transform.rotation_degrees.z = value,
        "transform.scale.x" => node.transform.scale.x = value,
        "transform.scale.y" => node.transform.scale.y = value,
        "transform.scale.z" => node.transform.scale.z = value,
        "primitive.radius" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::Sphere { radius })
            | NodeKind::Primitive(PrimitiveKind::Capsule { radius, .. })
            | NodeKind::Primitive(PrimitiveKind::Cylinder { radius, .. }) => *radius = value,
            _ => return false,
        },
        "primitive.half_extents.x" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                half_extents.x = value;
            }
            _ => return false,
        },
        "primitive.half_extents.y" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                half_extents.y = value;
            }
            _ => return false,
        },
        "primitive.half_extents.z" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { half_extents, .. }) => {
                half_extents.z = value;
            }
            _ => return false,
        },
        "primitive.roundness" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::RoundedBox { roundness, .. })
            | NodeKind::Primitive(PrimitiveKind::Cylinder { roundness, .. }) => *roundness = value,
            _ => return false,
        },
        "primitive.half_length" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::Capsule { half_length, .. }) => *half_length = value,
            _ => return false,
        },
        "primitive.half_height" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::Cylinder { half_height, .. }) => {
                *half_height = value;
            }
            _ => return false,
        },
        "primitive.major_radius" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::Torus { major_radius, .. }) => *major_radius = value,
            _ => return false,
        },
        "primitive.minor_radius" => match &mut node.kind {
            NodeKind::Primitive(PrimitiveKind::Torus { minor_radius, .. }) => *minor_radius = value,
            _ => return false,
        },
        "csg.smoothness" => match &mut node.kind {
            NodeKind::SmoothUnion { smoothness, .. } => *smoothness = value,
            _ => return false,
        },
        _ => return false,
    }
    true
}

/// Apply a replayable edit atomically to a document clone.
pub fn apply_edit(
    document: &ShapeDocument,
    edit: &EditProgram,
) -> Result<ShapeDocument, CoreError> {
    let mut clone = document.clone();
    for operation in &edit.operations {
        let actual = get_scalar(&clone, &operation.path)?;
        if (actual - operation.before).abs() > EDIT_PRECONDITION_TOLERANCE {
            return Err(CoreError::EditPreconditionFailed {
                path: operation.path.clone(),
                expected: operation.before,
                actual,
            });
        }
        set_scalar(&mut clone, &operation.path, operation.after)?;
    }
    let report = validate_document(&clone);
    if report.is_valid() {
        Ok(clone)
    } else {
        Err(CoreError::ValidationFailed(report))
    }
}

/// Return descendants reachable from a node in stable order with no duplicates.
pub fn descendants_of(document: &ShapeDocument, node: NodeId) -> Result<Vec<NodeId>, CoreError> {
    if !document.nodes.contains_key(&node) {
        return Err(CoreError::UnknownNode(node));
    }
    let mut result = BTreeSet::new();
    collect_descendants(document, node, &mut result)?;
    result.remove(&node);
    Ok(result.into_iter().collect())
}

fn collect_descendants(
    document: &ShapeDocument,
    node: NodeId,
    result: &mut BTreeSet<NodeId>,
) -> Result<(), CoreError> {
    let shape_node = document
        .nodes
        .get(&node)
        .ok_or(CoreError::UnknownNode(node))?;
    for child in referenced_nodes(&shape_node.kind) {
        if !document.nodes.contains_key(&child) {
            return Err(CoreError::UnknownNode(child));
        }
        if result.insert(child) {
            collect_descendants(document, child, result)?;
        }
    }
    Ok(())
}

fn referenced_nodes(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Primitive(_) => Vec::new(),
        NodeKind::Union { children }
        | NodeKind::SmoothUnion { children, .. }
        | NodeKind::Intersection { children } => children.clone(),
        NodeKind::Difference { base, subtractors } => {
            let mut result = vec![*base];
            result.extend(subtractors.iter().copied());
            result
        }
    }
}

/// Allocate the next stable node ID.
pub fn allocate_node_id(document: &mut ShapeDocument) -> NodeId {
    let id = NodeId(document.next_node_id);
    document.next_node_id = document.next_node_id.saturating_add(1);
    id
}

/// Insert an already-built node, rejecting duplicate IDs.
pub fn insert_node(document: &mut ShapeDocument, node: ShapeNode) -> Result<(), CoreError> {
    if document.nodes.contains_key(&node.id) {
        return Err(CoreError::DuplicateNode(node.id));
    }
    let node_id = node.id;
    document.nodes.insert(node_id, node);
    document.next_node_id = document.next_node_id.max(node_id.0.saturating_add(1));
    Ok(())
}

fn path(node: NodeId, key: &'static str) -> ParamPath {
    ParamPath {
        node,
        key: key.to_owned(),
    }
}

fn push_issue(
    report: &mut ValidationReport,
    node: Option<NodeId>,
    path: Option<ParamPath>,
    code: &'static str,
    message: impl Into<String>,
) {
    report.issues.push(ValidationIssue {
        node,
        path,
        code: code.to_owned(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primitive_node(id: u64, name: &str, primitive: PrimitiveKind) -> ShapeNode {
        ShapeNode {
            id: NodeId(id),
            name: name.to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Primitive(primitive),
        }
    }

    fn csg_node(id: u64, name: &str, kind: NodeKind) -> ShapeNode {
        ShapeNode {
            id: NodeId(id),
            name: name.to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind,
        }
    }

    fn sphere_document() -> ShapeDocument {
        ShapeDocument::new(
            "Sphere",
            primitive_node(1, "Sphere", PrimitiveKind::Sphere { radius: 1.0 }),
        )
    }

    fn issue_codes(report: &ValidationReport) -> BTreeSet<String> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect()
    }

    fn assert_valid(document: &ShapeDocument) {
        let report = validate_document(document);
        assert!(
            report.is_valid(),
            "expected valid document, got {:?}",
            report.issues
        );
    }

    #[test]
    fn valid_mixed_primitive_csg_document() {
        let root = csg_node(
            5,
            "Root difference",
            NodeKind::Difference {
                base: NodeId(4),
                subtractors: vec![NodeId(3)],
            },
        );
        let mut document = ShapeDocument::new("Mixed", root);
        document
            .insert_node(primitive_node(
                1,
                "Sphere",
                PrimitiveKind::Sphere { radius: 1.0 },
            ))
            .unwrap();
        document
            .insert_node(primitive_node(
                2,
                "Box",
                PrimitiveKind::RoundedBox {
                    half_extents: Vec3::splat(0.75),
                    roundness: 0.1,
                },
            ))
            .unwrap();
        document
            .insert_node(primitive_node(
                3,
                "Cylinder",
                PrimitiveKind::Cylinder {
                    half_height: 0.8,
                    radius: 0.25,
                    roundness: 0.05,
                },
            ))
            .unwrap();
        document
            .insert_node(csg_node(
                4,
                "Blend",
                NodeKind::SmoothUnion {
                    children: vec![NodeId(1), NodeId(2)],
                    smoothness: 0.2,
                },
            ))
            .unwrap();

        assert_valid(&document);
        assert_eq!(
            descendants_of(&document, document.root).unwrap(),
            vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4)]
        );
    }

    #[test]
    fn dangling_child_is_reported() {
        let document = ShapeDocument::new(
            "Dangling",
            csg_node(
                1,
                "Root",
                NodeKind::Union {
                    children: vec![NodeId(42)],
                },
            ),
        );

        let report = validate_document(&document);
        assert!(issue_codes(&report).contains("dangling_reference"));
    }

    #[test]
    fn cycle_is_reported() {
        let mut document = ShapeDocument::new(
            "Cycle",
            csg_node(
                1,
                "A",
                NodeKind::Union {
                    children: vec![NodeId(2)],
                },
            ),
        );
        document
            .insert_node(csg_node(
                2,
                "B",
                NodeKind::Union {
                    children: vec![NodeId(1)],
                },
            ))
            .unwrap();

        let report = validate_document(&document);
        assert!(issue_codes(&report).contains("cycle"));
    }

    #[test]
    fn shared_dag_child_is_valid_and_deduplicated() {
        let mut document = ShapeDocument::new(
            "DAG",
            csg_node(
                10,
                "Root",
                NodeKind::Union {
                    children: vec![NodeId(2), NodeId(3)],
                },
            ),
        );
        document
            .insert_node(primitive_node(
                1,
                "Shared",
                PrimitiveKind::Sphere { radius: 0.5 },
            ))
            .unwrap();
        document
            .insert_node(csg_node(
                2,
                "Left",
                NodeKind::Union {
                    children: vec![NodeId(1)],
                },
            ))
            .unwrap();
        document
            .insert_node(csg_node(
                3,
                "Right",
                NodeKind::Intersection {
                    children: vec![NodeId(1)],
                },
            ))
            .unwrap();

        assert_valid(&document);
        assert_eq!(
            descendants_of(&document, document.root).unwrap(),
            vec![NodeId(1), NodeId(2), NodeId(3)]
        );
    }

    #[test]
    fn parameter_enumeration_is_stable() {
        let mut document = ShapeDocument::new(
            "Stable",
            csg_node(
                9,
                "Root",
                NodeKind::Union {
                    children: vec![NodeId(5), NodeId(2)],
                },
            ),
        );
        document
            .insert_node(primitive_node(
                5,
                "Torus",
                PrimitiveKind::Torus {
                    major_radius: 1.0,
                    minor_radius: 0.2,
                },
            ))
            .unwrap();
        document
            .insert_node(primitive_node(
                2,
                "Capsule",
                PrimitiveKind::Capsule {
                    half_length: 0.75,
                    radius: 0.2,
                },
            ))
            .unwrap();

        let first = enumerate_parameters(&document);
        let second = enumerate_parameters(&document);
        assert_eq!(first, second);

        let mut sorted = first
            .iter()
            .map(|descriptor| &descriptor.path)
            .collect::<Vec<_>>();
        let actual = sorted.clone();
        sorted.sort();
        assert_eq!(actual, sorted);
    }

    #[test]
    fn set_get_round_trip_for_every_primitive_kind() {
        let primitives = vec![
            PrimitiveKind::Sphere { radius: 1.0 },
            PrimitiveKind::RoundedBox {
                half_extents: Vec3::splat(1.0),
                roundness: 0.1,
            },
            PrimitiveKind::Capsule {
                half_length: 1.0,
                radius: 0.25,
            },
            PrimitiveKind::Cylinder {
                half_height: 1.0,
                radius: 0.3,
                roundness: 0.05,
            },
            PrimitiveKind::Torus {
                major_radius: 1.0,
                minor_radius: 0.2,
            },
        ];

        for primitive in primitives {
            let mut document =
                ShapeDocument::new("Primitive", primitive_node(1, "Primitive", primitive));
            for descriptor in enumerate_parameters(&document) {
                let value = (descriptor.minimum + descriptor.maximum) * 0.5;
                set_scalar(&mut document, &descriptor.path, value).unwrap();
                let actual = get_scalar(&document, &descriptor.path).unwrap();
                assert!(
                    (actual - value).abs() <= Scalar::EPSILON,
                    "round trip failed for {:?}: {actual} != {value}",
                    descriptor.path
                );
            }
        }
    }

    #[test]
    fn failed_edit_is_atomic() {
        let document = sphere_document();
        let edit = EditProgram {
            label: "bad edit".to_owned(),
            seed: 1,
            operations: vec![
                SetScalarEdit {
                    path: path(NodeId(1), "primitive.radius"),
                    before: 1.0,
                    after: 1.5,
                },
                SetScalarEdit {
                    path: path(NodeId(1), "transform.scale.x"),
                    before: 2.0,
                    after: 0.5,
                },
            ],
        };

        assert!(apply_edit(&document, &edit).is_err());
        assert_eq!(
            get_scalar(&document, &path(NodeId(1), "primitive.radius")).unwrap(),
            1.0
        );
    }

    #[test]
    fn locks_are_validated() {
        let mut document = sphere_document();
        document.locks.insert(path(NodeId(1), "primitive.radius"));
        assert_valid(&document);

        document
            .locks
            .insert(path(NodeId(1), "primitive.minor_radius"));
        let report = validate_document(&document);
        assert!(issue_codes(&report).contains("unknown_lock_path"));
    }

    #[test]
    fn serde_json_round_trip() {
        let mut document = sphere_document();
        document
            .nodes
            .get_mut(&NodeId(1))
            .unwrap()
            .tags
            .insert("primary".to_owned());

        let json = serde_json::to_string(&document).unwrap();
        let round_tripped: ShapeDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(document, round_tripped);
    }

    #[test]
    fn validation_collects_several_errors() {
        let mut nodes = BTreeMap::new();
        let mut bad_node = primitive_node(1, "Bad", PrimitiveKind::Sphere { radius: -1.0 });
        bad_node.transform.translation.x = Scalar::NAN;
        bad_node.transform.scale.y = 0.0;
        nodes.insert(NodeId(1), bad_node);
        let mut document = ShapeDocument {
            schema_version: SCHEMA_VERSION,
            title: "Bad".to_owned(),
            root: NodeId(99),
            nodes,
            next_node_id: 1,
            locks: BTreeSet::new(),
        };
        document
            .locks
            .insert(path(NodeId(1), "primitive.minor_radius"));

        let codes = issue_codes(&validate_document(&document));

        assert!(codes.contains("missing_root"));
        assert!(codes.contains("non_finite"));
        assert!(codes.contains("near_zero_scale"));
        assert!(codes.contains("invalid_dimension"));
        assert!(codes.contains("unknown_lock_path"));
        assert!(codes.contains("next_node_id_not_fresh"));
    }
}
