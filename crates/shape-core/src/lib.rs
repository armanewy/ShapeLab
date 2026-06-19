#![forbid(unsafe_code)]

//! Core semantic document types for Shape Lab.
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
            schema_version: 1,
            title: title.into(),
            root: root_id,
            nodes,
            next_node_id,
            locks: BTreeSet::new(),
        }
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
    /// Algorithm is intentionally left for a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

/// Validate basic document invariants.
#[must_use]
pub fn validate_document(document: &ShapeDocument) -> ValidationReport {
    let mut report = ValidationReport::default();
    if !document.nodes.contains_key(&document.root) {
        report.issues.push(ValidationIssue {
            node: Some(document.root),
            path: None,
            code: "missing_root".to_owned(),
            message: "Document root does not exist.".to_owned(),
        });
    }
    for (id, node) in &document.nodes {
        if node.id != *id {
            report.issues.push(ValidationIssue {
                node: Some(*id),
                path: None,
                code: "node_id_mismatch".to_owned(),
                message: "Node map key and node ID differ.".to_owned(),
            });
        }
        for descriptor in enumerate_node_parameters(node) {
            match get_scalar(document, &descriptor.path) {
                Ok(value) if value.is_finite() => {}
                Ok(value) => report.issues.push(ValidationIssue {
                    node: Some(*id),
                    path: Some(descriptor.path),
                    code: "non_finite".to_owned(),
                    message: format!("Parameter value is not finite: {value}."),
                }),
                Err(_) => {}
            }
        }
    }
    report
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
    let rotation = ParamLimits::new(-180.0, 180.0, 1.0, 8.0);
    let scale = ParamLimits::new(0.05, 10.0, 0.01, 0.1);
    let medium_form = ParamLimits::new(0.01, 5.0, 0.01, 0.12);
    let roundness = ParamLimits::new(0.0, 2.0, 0.01, 0.05);
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
                medium_form,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::RoundedBox { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.half_extents.x",
                "Half Width",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.half_extents.y",
                "Half Height",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.half_extents.z",
                "Half Depth",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.roundness",
                "Roundness",
                ParamGroup::Form,
                roundness,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Capsule { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.half_length",
                "Half Length",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.radius",
                "Radius",
                ParamGroup::Form,
                medium_form,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Cylinder { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.half_height",
                "Half Height",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.radius",
                "Radius",
                ParamGroup::Form,
                medium_form,
            ));
            result.push(descriptor(
                node.id,
                "primitive.roundness",
                "Roundness",
                ParamGroup::Form,
                roundness,
            ));
        }
        NodeKind::Primitive(PrimitiveKind::Torus { .. }) => {
            result.push(descriptor(
                node.id,
                "primitive.major_radius",
                "Major Radius",
                ParamGroup::Form,
                ParamLimits::new(0.02, 5.0, 0.01, 0.12),
            ));
            result.push(descriptor(
                node.id,
                "primitive.minor_radius",
                "Minor Radius",
                ParamGroup::Form,
                ParamLimits::new(0.01, 2.5, 0.01, 0.08),
            ));
        }
        NodeKind::SmoothUnion { .. } => {
            result.push(descriptor(
                node.id,
                "csg.smoothness",
                "Blend Smoothness",
                ParamGroup::Blend,
                roundness,
            ));
        }
        NodeKind::Union { .. } | NodeKind::Difference { .. } | NodeKind::Intersection { .. } => {}
    }
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
                *half_height = value
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
        if (actual - operation.before).abs() > 1.0e-4 {
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

/// Return descendants reachable from a node in stable order.
pub fn descendants_of(document: &ShapeDocument, node: NodeId) -> Result<Vec<NodeId>, CoreError> {
    if !document.nodes.contains_key(&node) {
        return Err(CoreError::UnknownNode(node));
    }
    let mut result = BTreeSet::new();
    collect_descendants(document, node, &mut result);
    result.remove(&node);
    Ok(result.into_iter().collect())
}

fn collect_descendants(document: &ShapeDocument, node: NodeId, result: &mut BTreeSet<NodeId>) {
    if !result.insert(node) {
        return;
    }
    let Some(shape_node) = document.nodes.get(&node) else {
        return;
    };
    for child in referenced_nodes(&shape_node.kind) {
        collect_descendants(document, child, result);
    }
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
