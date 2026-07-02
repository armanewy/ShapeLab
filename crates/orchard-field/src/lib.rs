#![forbid(unsafe_code)]

//! Implicit scalar field compiler and deterministic grid sampling.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Quat, Vec2, Vec3};
use orchard_core_legacy::{Aabb, NodeId, NodeKind, PrimitiveKind, ShapeDocument, Transform3};
use rayon::prelude::*;
use thiserror::Error;

const MIN_SCALE_ABS: f32 = 1.0e-6;
const MAX_GRID_SAMPLES: usize = 16_777_216;

/// Thread-safe scalar field sampled in world space.
pub trait ScalarField: Send + Sync {
    /// Sample signed distance at a point. Negative values are inside.
    fn sample(&self, point: Vec3) -> f32;

    /// Conservative world-space bounds.
    fn bounds(&self) -> Aabb;
}

/// Immutable compiled field arena.
#[derive(Debug, Clone)]
pub struct CompiledField {
    document: ShapeDocument,
    arena: Vec<CompiledNode>,
    root: usize,
    bounds: Aabb,
}

impl CompiledField {
    /// Return the source document used to compile this field.
    #[must_use]
    pub fn document(&self) -> &ShapeDocument {
        &self.document
    }

    fn sample_node(&self, index: usize, point: Vec3) -> f32 {
        let node = &self.arena[index];
        let local_point = node.transform.to_local(point);
        let local_distance = match &node.op {
            FieldOp::Empty => f32::INFINITY,
            FieldOp::Sphere { radius } => signed_distance_sphere(local_point, *radius),
            FieldOp::RoundedBox {
                half_extents,
                roundness,
            } => signed_distance_rounded_box(local_point, *half_extents, *roundness),
            FieldOp::Capsule {
                half_length,
                radius,
            } => signed_distance_capsule(local_point, *half_length, *radius),
            FieldOp::Cylinder {
                half_height,
                radius,
                roundness,
            } => signed_distance_cylinder(local_point, *half_height, *radius, *roundness),
            FieldOp::Torus {
                major_radius,
                minor_radius,
            } => signed_distance_torus(local_point, *major_radius, *minor_radius),
            FieldOp::Union { children } => children
                .iter()
                .map(|child| self.sample_node(*child, local_point))
                .fold(f32::INFINITY, f32::min),
            FieldOp::SmoothUnion {
                children,
                smoothness,
            } => children
                .iter()
                .map(|child| self.sample_node(*child, local_point))
                .fold(f32::INFINITY, |a, b| smooth_min(a, b, *smoothness)),
            FieldOp::Difference { base, subtractors } => {
                let base_distance = self.sample_node(*base, local_point);
                let subtractor_distance = subtractors
                    .iter()
                    .map(|child| self.sample_node(*child, local_point))
                    .fold(f32::INFINITY, f32::min);
                if subtractors.is_empty() {
                    base_distance
                } else {
                    base_distance.max(-subtractor_distance)
                }
            }
            FieldOp::Intersection { children } => {
                if children.is_empty() {
                    f32::INFINITY
                } else {
                    children
                        .iter()
                        .map(|child| self.sample_node(*child, local_point))
                        .fold(f32::NEG_INFINITY, f32::max)
                }
            }
        };
        local_distance * node.transform.distance_scale
    }
}

impl ScalarField for CompiledField {
    fn sample(&self, point: Vec3) -> f32 {
        self.sample_node(self.root, point)
    }

    fn bounds(&self) -> Aabb {
        self.bounds
    }
}

#[derive(Debug, Clone)]
struct CompiledNode {
    transform: CompiledTransform,
    op: FieldOp,
    bounds: Aabb,
}

#[derive(Debug, Clone)]
enum FieldOp {
    Empty,
    Sphere {
        radius: f32,
    },
    RoundedBox {
        half_extents: Vec3,
        roundness: f32,
    },
    Capsule {
        half_length: f32,
        radius: f32,
    },
    Cylinder {
        half_height: f32,
        radius: f32,
        roundness: f32,
    },
    Torus {
        major_radius: f32,
        minor_radius: f32,
    },
    Union {
        children: Vec<usize>,
    },
    SmoothUnion {
        children: Vec<usize>,
        smoothness: f32,
    },
    Difference {
        base: usize,
        subtractors: Vec<usize>,
    },
    Intersection {
        children: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
struct CompiledTransform {
    translation: Vec3,
    inverse_rotation: Quat,
    inverse_scale: Vec3,
    distance_scale: f32,
}

impl CompiledTransform {
    fn from_transform(transform: &Transform3, node: NodeId) -> Result<Self, FieldCompileError> {
        if !vec3_is_finite(transform.translation)
            || !vec3_is_finite(transform.rotation_degrees)
            || !vec3_is_finite(transform.scale)
        {
            return Err(FieldCompileError::InvalidNode {
                node,
                reason: "transform contains non-finite values",
            });
        }

        let scale_abs = transform.scale.abs();
        let distance_scale = scale_abs.min_element();
        if distance_scale <= MIN_SCALE_ABS {
            return Err(FieldCompileError::InvalidNode {
                node,
                reason: "transform scale is zero or too close to zero",
            });
        }

        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            transform.rotation_degrees.x.to_radians(),
            transform.rotation_degrees.y.to_radians(),
            transform.rotation_degrees.z.to_radians(),
        );
        if !quat_is_finite(rotation) {
            return Err(FieldCompileError::InvalidNode {
                node,
                reason: "rotation compiled to a non-finite quaternion",
            });
        }

        Ok(Self {
            translation: transform.translation,
            inverse_rotation: rotation.inverse(),
            inverse_scale: Vec3::new(
                transform.scale.x.recip(),
                transform.scale.y.recip(),
                transform.scale.z.recip(),
            ),
            distance_scale,
        })
    }

    fn to_local(&self, point: Vec3) -> Vec3 {
        (self.inverse_rotation * (point - self.translation)) * self.inverse_scale
    }
}

/// Field compilation and sampling errors.
#[derive(Debug, Error)]
pub enum FieldCompileError {
    /// The source document is invalid.
    #[error("invalid shape document")]
    InvalidDocument,
    /// A referenced node does not exist.
    #[error("missing node {0:?}")]
    MissingNode(NodeId),
    /// The shape graph contains a cycle.
    #[error("shape graph cycle involving node {0:?}")]
    Cycle(NodeId),
    /// A node cannot be compiled into a finite field.
    #[error("invalid node {node:?}: {reason}")]
    InvalidNode {
        /// Node that failed compilation.
        node: NodeId,
        /// Stable human-readable reason.
        reason: &'static str,
    },
    /// Compiled field data was non-finite.
    #[error("compiled field contains non-finite data")]
    NonFiniteCompiledData,
    /// Grid settings are invalid.
    #[error("invalid grid: {0}")]
    InvalidGrid(String),
}

/// Uniform grid sampling specification.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GridSpec {
    /// Sampling bounds.
    pub bounds: Aabb,
    /// Samples along X.
    pub resolution_x: usize,
    /// Samples along Y.
    pub resolution_y: usize,
    /// Samples along Z.
    pub resolution_z: usize,
}

/// Sampled grid values in deterministic X-fastest, then Y, then Z order.
///
/// The value index is `x + resolution_x * (y + resolution_y * z)`.
#[derive(Debug, Clone, PartialEq)]
pub struct GridSamples {
    /// Grid specification.
    pub spec: GridSpec,
    /// Scalar values.
    pub values: Vec<f32>,
}

/// Compile a shape document into a scalar field.
pub fn compile_document(document: &ShapeDocument) -> Result<CompiledField, FieldCompileError> {
    validate_field_document(document)?;
    let report = orchard_core_legacy::validate_document(document);
    if !report.is_valid() {
        return Err(FieldCompileError::InvalidDocument);
    }

    let mut compiler = FieldCompiler::new(document);
    let root = compiler.compile_node(document.root)?;
    let bounds = compiler
        .arena
        .get(root)
        .map(|node| node.bounds)
        .ok_or(FieldCompileError::InvalidDocument)?;

    let field = CompiledField {
        document: document.clone(),
        arena: compiler.arena,
        root,
        bounds,
    };
    validate_compiled_field(&field)?;
    Ok(field)
}

struct FieldCompiler<'a> {
    document: &'a ShapeDocument,
    indices: BTreeMap<NodeId, usize>,
    visiting: BTreeSet<NodeId>,
    arena: Vec<CompiledNode>,
}

impl<'a> FieldCompiler<'a> {
    fn new(document: &'a ShapeDocument) -> Self {
        Self {
            document,
            indices: BTreeMap::new(),
            visiting: BTreeSet::new(),
            arena: Vec::new(),
        }
    }

    fn compile_node(&mut self, id: NodeId) -> Result<usize, FieldCompileError> {
        if let Some(index) = self.indices.get(&id) {
            return Ok(*index);
        }
        if !self.visiting.insert(id) {
            return Err(FieldCompileError::Cycle(id));
        }

        let node = self
            .document
            .nodes
            .get(&id)
            .ok_or(FieldCompileError::MissingNode(id))?;
        let transform = CompiledTransform::from_transform(&node.transform, id)?;

        let (op, local_bounds) = if node.enabled {
            self.compile_enabled_node(id, &node.kind)?
        } else {
            (FieldOp::Empty, Aabb::empty())
        };

        let bounds = local_bounds.transformed(&node.transform);
        if !aabb_is_finite_or_empty(&bounds) {
            return Err(FieldCompileError::NonFiniteCompiledData);
        }

        let index = self.arena.len();
        self.arena.push(CompiledNode {
            transform,
            op,
            bounds,
        });
        self.indices.insert(id, index);
        self.visiting.remove(&id);
        Ok(index)
    }

    fn compile_enabled_node(
        &mut self,
        id: NodeId,
        kind: &NodeKind,
    ) -> Result<(FieldOp, Aabb), FieldCompileError> {
        let compiled = match kind {
            NodeKind::Primitive(primitive) => (
                compile_primitive_op(primitive),
                primitive_local_bounds(primitive),
            ),
            NodeKind::Union { children } => {
                let compiled_children = self.compile_children(children)?;
                let bounds = union_bounds(&self.arena, &compiled_children);
                (
                    FieldOp::Union {
                        children: compiled_children,
                    },
                    bounds,
                )
            }
            NodeKind::SmoothUnion {
                children,
                smoothness,
            } => {
                if !smoothness.is_finite() || *smoothness < 0.0 {
                    return Err(FieldCompileError::InvalidNode {
                        node: id,
                        reason: "smoothness must be finite and non-negative",
                    });
                }
                let compiled_children = self.compile_children(children)?;
                let bounds = union_bounds(&self.arena, &compiled_children).expanded(*smoothness);
                (
                    FieldOp::SmoothUnion {
                        children: compiled_children,
                        smoothness: *smoothness,
                    },
                    bounds,
                )
            }
            NodeKind::Difference { base, subtractors } => {
                let compiled_base = self.compile_node(*base)?;
                let compiled_subtractors = self.compile_children(subtractors)?;
                (
                    FieldOp::Difference {
                        base: compiled_base,
                        subtractors: compiled_subtractors,
                    },
                    self.arena[compiled_base].bounds,
                )
            }
            NodeKind::Intersection { children } => {
                let compiled_children = self.compile_children(children)?;
                let bounds = intersection_bounds(&self.arena, &compiled_children);
                (
                    FieldOp::Intersection {
                        children: compiled_children,
                    },
                    bounds,
                )
            }
        };
        Ok(compiled)
    }

    fn compile_children(&mut self, children: &[NodeId]) -> Result<Vec<usize>, FieldCompileError> {
        children
            .iter()
            .map(|child| self.compile_node(*child))
            .collect()
    }
}

fn compile_primitive_op(primitive: &PrimitiveKind) -> FieldOp {
    match primitive {
        PrimitiveKind::Sphere { radius } => FieldOp::Sphere { radius: *radius },
        PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        } => FieldOp::RoundedBox {
            half_extents: *half_extents,
            roundness: *roundness,
        },
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => FieldOp::Capsule {
            half_length: *half_length,
            radius: *radius,
        },
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            roundness,
        } => FieldOp::Cylinder {
            half_height: *half_height,
            radius: *radius,
            roundness: *roundness,
        },
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => FieldOp::Torus {
            major_radius: *major_radius,
            minor_radius: *minor_radius,
        },
    }
}

fn primitive_local_bounds(primitive: &PrimitiveKind) -> Aabb {
    match primitive {
        PrimitiveKind::Sphere { radius } => aabb_from_half_extent(Vec3::splat(*radius)),
        PrimitiveKind::RoundedBox { half_extents, .. } => aabb_from_half_extent(*half_extents),
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => aabb_from_half_extent(Vec3::new(*radius, *half_length + *radius, *radius)),
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            ..
        } => aabb_from_half_extent(Vec3::new(*radius, *half_height, *radius)),
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => aabb_from_half_extent(Vec3::new(
            *major_radius + *minor_radius,
            *minor_radius,
            *major_radius + *minor_radius,
        )),
    }
}

fn aabb_from_half_extent(half_extent: Vec3) -> Aabb {
    Aabb {
        min: -half_extent,
        max: half_extent,
    }
}

fn union_bounds(arena: &[CompiledNode], children: &[usize]) -> Aabb {
    children
        .iter()
        .map(|child| arena[*child].bounds)
        .fold(Aabb::empty(), |bounds, child| bounds.union(&child))
}

fn intersection_bounds(arena: &[CompiledNode], children: &[usize]) -> Aabb {
    let Some((first, rest)) = children.split_first() else {
        return Aabb::empty();
    };
    rest.iter()
        .map(|child| arena[*child].bounds)
        .fold(arena[*first].bounds, |bounds, child| {
            bounds.intersection(&child)
        })
}

fn validate_field_document(document: &ShapeDocument) -> Result<(), FieldCompileError> {
    if !document.nodes.contains_key(&document.root) {
        return Err(FieldCompileError::InvalidDocument);
    }
    for (id, node) in &document.nodes {
        if node.id != *id {
            return Err(FieldCompileError::InvalidNode {
                node: *id,
                reason: "node ID does not match its map key",
            });
        }
        validate_transform(*id, &node.transform)?;
        validate_kind(*id, &node.kind)?;
        for reference in referenced_nodes(&node.kind) {
            if !document.nodes.contains_key(&reference) {
                return Err(FieldCompileError::MissingNode(reference));
            }
        }
    }
    validate_acyclic(document)
}

fn validate_transform(id: NodeId, transform: &Transform3) -> Result<(), FieldCompileError> {
    CompiledTransform::from_transform(transform, id).map(|_| ())
}

fn validate_kind(id: NodeId, kind: &NodeKind) -> Result<(), FieldCompileError> {
    match kind {
        NodeKind::Primitive(primitive) => validate_primitive(id, primitive),
        NodeKind::Union { children } | NodeKind::Intersection { children } => {
            validate_nonempty_children(id, children)
        }
        NodeKind::SmoothUnion {
            children,
            smoothness,
        } => {
            validate_nonempty_children(id, children)?;
            if !smoothness.is_finite() || *smoothness < 0.0 {
                return Err(FieldCompileError::InvalidNode {
                    node: id,
                    reason: "smoothness must be finite and non-negative",
                });
            }
            Ok(())
        }
        NodeKind::Difference { .. } => Ok(()),
    }
}

fn validate_nonempty_children(id: NodeId, children: &[NodeId]) -> Result<(), FieldCompileError> {
    if children.is_empty() {
        return Err(FieldCompileError::InvalidNode {
            node: id,
            reason: "combiner must reference at least one child",
        });
    }
    Ok(())
}

fn validate_primitive(id: NodeId, primitive: &PrimitiveKind) -> Result<(), FieldCompileError> {
    match primitive {
        PrimitiveKind::Sphere { radius } => validate_positive(id, *radius, "radius"),
        PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        } => {
            validate_positive_vec3(id, *half_extents, "half extents")?;
            validate_nonnegative(id, *roundness, "roundness")?;
            if *roundness > half_extents.min_element() {
                return Err(FieldCompileError::InvalidNode {
                    node: id,
                    reason: "rounded-box roundness exceeds the smallest half extent",
                });
            }
            Ok(())
        }
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => {
            validate_positive(id, *half_length, "half length")?;
            validate_positive(id, *radius, "radius")
        }
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            roundness,
        } => {
            validate_positive(id, *half_height, "half height")?;
            validate_positive(id, *radius, "radius")?;
            validate_nonnegative(id, *roundness, "roundness")?;
            if *roundness > (*half_height).min(*radius) {
                return Err(FieldCompileError::InvalidNode {
                    node: id,
                    reason: "cylinder roundness exceeds the smallest cylinder dimension",
                });
            }
            Ok(())
        }
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => {
            validate_positive(id, *major_radius, "major radius")?;
            validate_positive(id, *minor_radius, "minor radius")?;
            if minor_radius >= major_radius {
                return Err(FieldCompileError::InvalidNode {
                    node: id,
                    reason: "torus minor radius must be smaller than major radius",
                });
            }
            Ok(())
        }
    }
}

fn validate_positive(
    id: NodeId,
    value: f32,
    reason: &'static str,
) -> Result<(), FieldCompileError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(FieldCompileError::InvalidNode { node: id, reason });
    }
    Ok(())
}

fn validate_nonnegative(
    id: NodeId,
    value: f32,
    reason: &'static str,
) -> Result<(), FieldCompileError> {
    if !value.is_finite() || value < 0.0 {
        return Err(FieldCompileError::InvalidNode { node: id, reason });
    }
    Ok(())
}

fn validate_positive_vec3(
    id: NodeId,
    value: Vec3,
    reason: &'static str,
) -> Result<(), FieldCompileError> {
    if !vec3_is_finite(value) || value.x <= 0.0 || value.y <= 0.0 || value.z <= 0.0 {
        return Err(FieldCompileError::InvalidNode { node: id, reason });
    }
    Ok(())
}

fn validate_acyclic(document: &ShapeDocument) -> Result<(), FieldCompileError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in document.nodes.keys().copied() {
        visit_node(document, id, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_node(
    document: &ShapeDocument,
    id: NodeId,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<(), FieldCompileError> {
    if visited.contains(&id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(FieldCompileError::Cycle(id));
    }
    let node = document
        .nodes
        .get(&id)
        .ok_or(FieldCompileError::MissingNode(id))?;
    for child in referenced_nodes(&node.kind) {
        visit_node(document, child, visiting, visited)?;
    }
    visiting.remove(&id);
    visited.insert(id);
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

fn validate_compiled_field(field: &CompiledField) -> Result<(), FieldCompileError> {
    if field.root >= field.arena.len() || !aabb_is_finite_or_empty(&field.bounds) {
        return Err(FieldCompileError::NonFiniteCompiledData);
    }
    for node in &field.arena {
        if !aabb_is_finite_or_empty(&node.bounds)
            || !vec3_is_finite(node.transform.translation)
            || !quat_is_finite(node.transform.inverse_rotation)
            || !vec3_is_finite(node.transform.inverse_scale)
            || !node.transform.distance_scale.is_finite()
        {
            return Err(FieldCompileError::NonFiniteCompiledData);
        }
    }
    Ok(())
}

// Sphere SDF: the signed distance is the distance from the point to the
// center, offset by the radius.
fn signed_distance_sphere(point: Vec3, radius: f32) -> f32 {
    point.length() - radius
}

// Rounded-box SDF: shrink the box by the corner radius, measure the distance
// to that inner box, then subtract the radius to restore the rounded corners.
fn signed_distance_rounded_box(point: Vec3, half_extents: Vec3, roundness: f32) -> f32 {
    let inner_half_extents = (half_extents - Vec3::splat(roundness)).max(Vec3::ZERO);
    let q = point.abs() - inner_half_extents;
    q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - roundness
}

// Capsule SDF: clamp the point to the closest location on the Y-axis segment
// and subtract the swept sphere radius.
fn signed_distance_capsule(point: Vec3, half_length: f32, radius: f32) -> f32 {
    let closest_y = point.y.clamp(-half_length, half_length);
    Vec3::new(point.x, point.y - closest_y, point.z).length() - radius
}

// Rounded capped-cylinder SDF: evaluate a 2D rounded rectangle in
// (radial distance, Y), which represents rotating that profile around Y.
fn signed_distance_cylinder(point: Vec3, half_height: f32, radius: f32, roundness: f32) -> f32 {
    let profile = Vec2::new(Vec2::new(point.x, point.z).length(), point.y.abs());
    let inner = Vec2::new(radius - roundness, half_height - roundness);
    let d = profile - inner;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0) - roundness
}

// Torus SDF around Y: reduce the problem to the distance from the point's
// (XZ radius, Y) coordinate to the major-radius circle cross section.
fn signed_distance_torus(point: Vec3, major_radius: f32, minor_radius: f32) -> f32 {
    let q = Vec2::new(Vec2::new(point.x, point.z).length() - major_radius, point.y);
    q.length() - minor_radius
}

fn smooth_min(a: f32, b: f32, smoothness: f32) -> f32 {
    if !a.is_finite() {
        return b;
    }
    if !b.is_finite() {
        return a;
    }
    if smoothness <= 0.0 {
        return a.min(b);
    }
    let h = (0.5 + 0.5 * (b - a) / smoothness).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - smoothness * h * (1.0 - h)
}

/// Sample a field on a uniform grid.
pub fn sample_grid(
    field: &impl ScalarField,
    spec: GridSpec,
) -> Result<GridSamples, FieldCompileError> {
    let count = validate_grid_spec(&spec)?;
    let plane = spec
        .resolution_x
        .checked_mul(spec.resolution_y)
        .ok_or_else(|| FieldCompileError::InvalidGrid("grid is too large".to_owned()))?;
    let min = spec.bounds.min;
    let extent = spec.bounds.extent();
    let denom = Vec3::new(
        spec.resolution_x.saturating_sub(1).max(1) as f32,
        spec.resolution_y.saturating_sub(1).max(1) as f32,
        spec.resolution_z.saturating_sub(1).max(1) as f32,
    );

    let values = (0..count)
        .into_par_iter()
        .map(|index| {
            let x = index % spec.resolution_x;
            let y = (index / spec.resolution_x) % spec.resolution_y;
            let z = index / plane;
            let t = Vec3::new(x as f32, y as f32, z as f32) / denom;
            field.sample(min + extent * t)
        })
        .collect();

    Ok(GridSamples { spec, values })
}

fn validate_grid_spec(spec: &GridSpec) -> Result<usize, FieldCompileError> {
    if spec.resolution_x == 0 || spec.resolution_y == 0 || spec.resolution_z == 0 {
        return Err(FieldCompileError::InvalidGrid(
            "all resolutions must be positive".to_owned(),
        ));
    }
    if spec.bounds.is_empty() {
        return Err(FieldCompileError::InvalidGrid(
            "bounds must not be empty".to_owned(),
        ));
    }
    if !aabb_is_finite_or_empty(&spec.bounds) {
        return Err(FieldCompileError::InvalidGrid(
            "bounds must be finite".to_owned(),
        ));
    }
    let count = spec
        .resolution_x
        .checked_mul(spec.resolution_y)
        .and_then(|value| value.checked_mul(spec.resolution_z))
        .ok_or_else(|| FieldCompileError::InvalidGrid("grid is too large".to_owned()))?;
    if count > MAX_GRID_SAMPLES {
        return Err(FieldCompileError::InvalidGrid(
            "grid is too large".to_owned(),
        ));
    }
    Ok(count)
}

fn vec3_is_finite(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn quat_is_finite(value: Quat) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite() && value.w.is_finite()
}

fn aabb_is_finite_or_empty(bounds: &Aabb) -> bool {
    bounds.is_empty() || (vec3_is_finite(bounds.min) && vec3_is_finite(bounds.max))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use glam::Vec3;
    use orchard_core_legacy::{NodeKind, PrimitiveKind, ShapeNode};

    use super::*;

    const EPSILON: f32 = 1.0e-4;

    #[test]
    fn known_sphere_distances() {
        let field = compile_document(&primitive_document(PrimitiveKind::Sphere { radius: 2.0 }))
            .expect("sphere compiles");

        assert_close(field.sample(Vec3::ZERO), -2.0);
        assert_close(field.sample(Vec3::new(2.0, 0.0, 0.0)), 0.0);
        assert_close(field.sample(Vec3::new(3.5, 0.0, 0.0)), 1.5);
    }

    #[test]
    fn capsule_axis_and_endpoint_distances() {
        let field = compile_document(&primitive_document(PrimitiveKind::Capsule {
            half_length: 1.0,
            radius: 0.5,
        }))
        .expect("capsule compiles");

        assert_close(field.sample(Vec3::ZERO), -0.5);
        assert_close(field.sample(Vec3::new(0.5, 0.0, 0.0)), 0.0);
        assert_close(field.sample(Vec3::new(0.0, 1.5, 0.0)), 0.0);
        assert_close(field.sample(Vec3::new(0.0, 2.0, 0.0)), 0.5);
    }

    #[test]
    fn rounded_box_inside_and_outside_points() {
        let field = compile_document(&primitive_document(PrimitiveKind::RoundedBox {
            half_extents: Vec3::splat(1.0),
            roundness: 0.25,
        }))
        .expect("rounded box compiles");

        assert_close(field.sample(Vec3::ZERO), -1.0);
        assert_close(field.sample(Vec3::new(1.0, 0.0, 0.0)), 0.0);
        assert_close(field.sample(Vec3::new(1.25, 0.0, 0.0)), 0.25);
    }

    #[test]
    fn cylinder_and_torus_points() {
        let cylinder = compile_document(&primitive_document(PrimitiveKind::Cylinder {
            half_height: 1.0,
            radius: 0.5,
            roundness: 0.1,
        }))
        .expect("cylinder compiles");

        assert_close(cylinder.sample(Vec3::new(0.5, 0.0, 0.0)), 0.0);
        assert_close(cylinder.sample(Vec3::new(0.0, 1.0, 0.0)), 0.0);
        assert!(cylinder.sample(Vec3::ZERO) < -0.49);

        let torus = compile_document(&primitive_document(PrimitiveKind::Torus {
            major_radius: 1.0,
            minor_radius: 0.25,
        }))
        .expect("torus compiles");

        assert_close(torus.sample(Vec3::new(1.25, 0.0, 0.0)), 0.0);
        assert_close(torus.sample(Vec3::new(1.0, 0.0, 0.0)), -0.25);
        assert_close(torus.sample(Vec3::ZERO), 0.75);
    }

    #[test]
    fn csg_union_intersection_difference_and_smooth_union() {
        let left = translated_sphere(NodeId(2), -0.75, 1.0);
        let right = translated_sphere(NodeId(3), 0.75, 1.0);
        let union_root = node(
            NodeId(1),
            NodeKind::Union {
                children: vec![NodeId(2), NodeId(3)],
            },
        );
        let union = compile_document(&document(union_root, vec![left.clone(), right.clone()]))
            .expect("union compiles");
        assert!(union.sample(Vec3::new(-0.75, 0.0, 0.0)) < -0.99);
        assert!(union.sample(Vec3::new(0.0, 0.0, 0.0)) < 0.0);

        let intersection_root = node(
            NodeId(1),
            NodeKind::Intersection {
                children: vec![NodeId(2), NodeId(3)],
            },
        );
        let intersection = compile_document(&document(
            intersection_root,
            vec![left.clone(), right.clone()],
        ))
        .expect("intersection compiles");
        assert!(intersection.sample(Vec3::ZERO) < 0.0);
        assert!(intersection.sample(Vec3::new(-0.75, 0.0, 0.0)) > 0.0);

        let base = node(
            NodeId(2),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
        );
        let cutter = translated_sphere(NodeId(3), 0.5, 0.5);
        let difference_root = node(
            NodeId(1),
            NodeKind::Difference {
                base: NodeId(2),
                subtractors: vec![NodeId(3)],
            },
        );
        let difference = compile_document(&document(difference_root, vec![base, cutter]))
            .expect("difference compiles");
        assert!(difference.sample(Vec3::new(0.5, 0.0, 0.0)) > 0.0);
        assert!(difference.sample(Vec3::new(-0.75, 0.0, 0.0)) < 0.0);

        let smooth_root = node(
            NodeId(1),
            NodeKind::SmoothUnion {
                children: vec![NodeId(2), NodeId(3)],
                smoothness: 0.5,
            },
        );
        let smooth =
            compile_document(&document(smooth_root, vec![left, right])).expect("smooth compiles");
        assert!(smooth.sample(Vec3::ZERO) < union.sample(Vec3::ZERO));
    }

    #[test]
    fn translated_rotated_uniform_and_nonuniform_scaled_nodes() {
        let translated = compile_document(&document(
            node_with_transform(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
                Transform3 {
                    translation: Vec3::new(2.0, 3.0, 4.0),
                    ..Transform3::default()
                },
            ),
            Vec::new(),
        ))
        .expect("translated sphere compiles");
        assert_close(translated.sample(Vec3::new(2.0, 3.0, 4.0)), -1.0);
        assert_close(translated.sample(Vec3::new(3.0, 3.0, 4.0)), 0.0);

        let rotated = compile_document(&document(
            node_with_transform(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::RoundedBox {
                    half_extents: Vec3::new(1.0, 2.0, 1.0),
                    roundness: 0.0,
                }),
                Transform3 {
                    rotation_degrees: Vec3::new(0.0, 0.0, 90.0),
                    ..Transform3::default()
                },
            ),
            Vec::new(),
        ))
        .expect("rotated box compiles");
        assert_close(rotated.sample(Vec3::new(-2.0, 0.0, 0.0)), 0.0);
        assert!(rotated.sample(Vec3::new(-2.25, 0.0, 0.0)) > 0.0);

        let uniform = compile_document(&document(
            node_with_transform(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
                Transform3 {
                    scale: Vec3::splat(2.0),
                    ..Transform3::default()
                },
            ),
            Vec::new(),
        ))
        .expect("uniform scale compiles");
        assert_close(uniform.sample(Vec3::ZERO), -2.0);
        assert_close(uniform.sample(Vec3::new(2.0, 0.0, 0.0)), 0.0);

        let nonuniform = compile_document(&document(
            node_with_transform(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
                Transform3 {
                    scale: Vec3::new(2.0, 3.0, 4.0),
                    ..Transform3::default()
                },
            ),
            Vec::new(),
        ))
        .expect("nonuniform scale compiles");
        assert_close(nonuniform.sample(Vec3::ZERO), -2.0);
        assert_close(nonuniform.sample(Vec3::new(0.0, 3.0, 0.0)), 0.0);
    }

    #[test]
    fn transformed_bounds_contain_sampled_surface_points() {
        let transform = Transform3 {
            translation: Vec3::new(2.0, -1.0, 0.5),
            rotation_degrees: Vec3::new(10.0, 35.0, 20.0),
            scale: Vec3::new(2.0, 1.0, 0.5),
        };
        let field = compile_document(&document(
            node_with_transform(
                NodeId(1),
                NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
                transform.clone(),
            ),
            Vec::new(),
        ))
        .expect("transformed sphere compiles");

        for local_surface in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
            let world = transform.matrix().transform_point3(local_surface);
            assert_contains_point(field.bounds(), world);
            assert_close(field.sample(world), 0.0);
        }
    }

    #[test]
    fn shared_dag_child_compiles_once() {
        let child = node(
            NodeId(2),
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
        );
        let root = node(
            NodeId(1),
            NodeKind::Union {
                children: vec![NodeId(2), NodeId(2)],
            },
        );
        let field = compile_document(&document(root, vec![child])).expect("DAG compiles");

        assert_eq!(field.arena.len(), 2);
        assert_close(field.sample(Vec3::ZERO), -1.0);
    }

    #[test]
    fn invalid_documents_are_rejected() {
        let missing_root = ShapeDocument {
            schema_version: 1,
            title: "missing root".to_owned(),
            root: NodeId(99),
            nodes: BTreeMap::new(),
            next_node_id: 100,
            locks: BTreeSet::new(),
        };
        assert!(matches!(
            compile_document(&missing_root),
            Err(FieldCompileError::InvalidDocument)
        ));

        let dangling = document(
            node(
                NodeId(1),
                NodeKind::Union {
                    children: vec![NodeId(2)],
                },
            ),
            Vec::new(),
        );
        assert!(matches!(
            compile_document(&dangling),
            Err(FieldCompileError::MissingNode(NodeId(2)))
        ));
    }

    #[test]
    fn sample_grid_uses_x_fastest_indexing() {
        #[derive(Debug)]
        struct CoordinateField;

        impl ScalarField for CoordinateField {
            fn sample(&self, point: Vec3) -> f32 {
                point.x + 10.0 * point.y + 100.0 * point.z
            }

            fn bounds(&self) -> Aabb {
                Aabb {
                    min: Vec3::ZERO,
                    max: Vec3::ONE,
                }
            }
        }

        let samples = sample_grid(
            &CoordinateField,
            GridSpec {
                bounds: Aabb {
                    min: Vec3::ZERO,
                    max: Vec3::ONE,
                },
                resolution_x: 2,
                resolution_y: 2,
                resolution_z: 2,
            },
        )
        .expect("grid samples");

        assert_eq!(
            samples.values,
            vec![0.0, 1.0, 10.0, 11.0, 100.0, 101.0, 110.0, 111.0]
        );
    }

    fn primitive_document(primitive: PrimitiveKind) -> ShapeDocument {
        document(node(NodeId(1), NodeKind::Primitive(primitive)), Vec::new())
    }

    fn translated_sphere(id: NodeId, x: f32, radius: f32) -> ShapeNode {
        node_with_transform(
            id,
            NodeKind::Primitive(PrimitiveKind::Sphere { radius }),
            Transform3 {
                translation: Vec3::new(x, 0.0, 0.0),
                ..Transform3::default()
            },
        )
    }

    fn node(id: NodeId, kind: NodeKind) -> ShapeNode {
        node_with_transform(id, kind, Transform3::default())
    }

    fn node_with_transform(id: NodeId, kind: NodeKind, transform: Transform3) -> ShapeNode {
        ShapeNode {
            id,
            name: format!("node {}", id.0),
            tags: BTreeSet::new(),
            enabled: true,
            transform,
            kind,
        }
    }

    fn document(root: ShapeNode, extra_nodes: Vec<ShapeNode>) -> ShapeDocument {
        let root_id = root.id;
        let mut nodes = BTreeMap::new();
        nodes.insert(root.id, root);
        for node in extra_nodes {
            nodes.insert(node.id, node);
        }
        let next_node_id = nodes
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
        ShapeDocument {
            schema_version: 1,
            title: "test".to_owned(),
            root: root_id,
            nodes,
            next_node_id,
            locks: BTreeSet::new(),
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to be within {EPSILON} of {expected}",
        );
    }

    fn assert_contains_point(bounds: Aabb, point: Vec3) {
        let padding = Vec3::splat(EPSILON);
        assert!(
            point.cmpge(bounds.min - padding).all() && point.cmple(bounds.max + padding).all(),
            "expected bounds {bounds:?} to contain point {point:?}",
        );
    }
}
