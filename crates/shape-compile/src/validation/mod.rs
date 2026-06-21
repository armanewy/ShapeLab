//! Modeling validation and geometric quality metrics for compiled static assets.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{OperationId, PartInstanceId, RegionId, SocketId};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, MeshBounds, PolygonFace, PolygonMesh,
    TriangulatedPolygonMesh, validate_polygon_mesh,
};

use crate::{AssetArtifact, CompiledPart};

const EPSILON: f32 = 1.0e-6;

/// Validation severity for a discovered modeling issue.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// The asset violates a required static-asset contract.
    Error,
    /// The asset is usable but has geometric quality risk.
    Warning,
    /// Informational diagnostic.
    Info,
}

/// One geometric or assembly validation issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue severity.
    pub severity: ValidationSeverity,
    /// Stable issue code.
    pub code: String,
    /// Part instances involved in the issue.
    pub part_instances: Vec<PartInstanceId>,
    /// Modeling or assembly operation involved in the issue, when known.
    pub operation: Option<OperationId>,
    /// Human-readable diagnostic.
    pub message: String,
    /// World-space location when a meaningful point is available.
    pub location: Option<[f32; 3]>,
}

/// Aggregate quality metrics for a compiled model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Number of compiled part instances.
    pub part_count: u64,
    /// Total polygon face count.
    pub polygon_count: u64,
    /// Total triangulated face count.
    pub triangle_count: u64,
    /// Fraction of polygon faces that are quads.
    pub quad_fraction: f32,
    /// Fraction of parts that are closed two-manifold meshes.
    pub manifold_closed_part_fraction: f32,
    /// Shortest authored edge discovered across all parts.
    pub minimum_edge: Option<f32>,
    /// Largest per-face edge aspect ratio discovered across all parts.
    pub maximum_aspect_ratio: Option<f32>,
    /// Count of authored hard, feature, seam, attachment, or open edges.
    pub hard_edge_count: u64,
    /// Number of distinct part/region pairs with face provenance.
    pub region_count: u64,
    /// Fraction of polygon faces with part definition and instance provenance.
    pub provenance_coverage: f32,
    /// Count of disallowed part pairs with narrow-phase triangle intersections.
    pub accidental_intersection_count: u64,
}

/// Complete validation report for a compiled model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelValidationReport {
    /// Deterministically ordered issues.
    pub issues: Vec<ValidationIssue>,
    /// Aggregate quality metrics.
    pub metrics: QualityMetrics,
}

impl ModelValidationReport {
    /// Return true when no error-severity issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != ValidationSeverity::Error)
    }
}

/// Validation thresholds and budgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationLimits {
    /// Minimum acceptable world-space edge length.
    pub minimum_edge_length: f32,
    /// Minimum acceptable world-space polygon face area.
    pub minimum_face_area: f32,
    /// Maximum acceptable per-face longest-edge/shortest-edge ratio.
    pub maximum_aspect_ratio: f32,
    /// Maximum total triangle count.
    pub maximum_triangle_count: u64,
    /// Maximum compiled part count.
    pub maximum_part_count: u64,
    /// Epsilon used to classify coincident duplicate vertices.
    pub duplicate_vertex_epsilon: f32,
    /// Positive AABB overlap tolerance before narrow-phase checks run.
    pub narrow_phase_aabb_margin: f32,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            minimum_edge_length: 1.0e-5,
            minimum_face_area: 1.0e-8,
            maximum_aspect_ratio: 1.0e4,
            maximum_triangle_count: u64::MAX,
            maximum_part_count: u64::MAX,
            duplicate_vertex_epsilon: 1.0e-6,
            narrow_phase_aabb_margin: 1.0e-5,
        }
    }
}

/// Expected socket attachment metadata supplied by an export or packaging layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedAttachment {
    /// Parent part instance.
    pub parent: PartInstanceId,
    /// Child part instance.
    pub child: PartInstanceId,
    /// Socket on the parent part.
    pub parent_socket: SocketId,
    /// Socket on the child part.
    pub child_socket: SocketId,
    /// Maximum allowed socket-origin distance in world units.
    pub max_origin_distance: f32,
    /// Maximum allowed angle between corresponding socket axes.
    pub max_axis_angle_degrees: f32,
    /// Optional maximum allowed mesh clearance before the child is considered detached.
    pub max_clearance: Option<f32>,
}

impl ExpectedAttachment {
    /// Build a rigid attachment expectation with conservative default tolerances.
    #[must_use]
    pub fn rigid(parent: PartInstanceId, child: PartInstanceId, socket: SocketId) -> Self {
        Self {
            parent,
            child,
            parent_socket: socket,
            child_socket: socket,
            max_origin_distance: 1.0e-4,
            max_axis_angle_degrees: 1.0,
            max_clearance: Some(1.0e-4),
        }
    }
}

/// Explicit relationship metadata used to distinguish authored relationships
/// from accidental intersections or clearances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PartRelationship {
    /// This pair is intentionally allowed to overlap.
    IntentionalOverlap {
        /// First part instance.
        first: PartInstanceId,
        /// Second part instance.
        second: PartInstanceId,
        /// Human-facing reason for the relationship.
        reason: String,
    },
    /// This pair must maintain at least the authored clearance.
    MinimumClearance {
        /// First part instance.
        first: PartInstanceId,
        /// Second part instance.
        second: PartInstanceId,
        /// Minimum clearance in world units.
        clearance: f32,
    },
    /// The contained part must remain inside the container's authored bounds.
    Containment {
        /// Containing part instance.
        container: PartInstanceId,
        /// Contained part instance.
        contained: PartInstanceId,
    },
}

impl PartRelationship {
    /// Create an intentional overlap relationship.
    #[must_use]
    pub fn intentional_overlap(
        first: PartInstanceId,
        second: PartInstanceId,
        reason: impl Into<String>,
    ) -> Self {
        Self::IntentionalOverlap {
            first,
            second,
            reason: reason.into(),
        }
    }

    fn pair(&self) -> PartPair {
        match self {
            Self::IntentionalOverlap { first, second, .. }
            | Self::MinimumClearance { first, second, .. } => PartPair::new(*first, *second),
            Self::Containment {
                container,
                contained,
            } => PartPair::new(*container, *contained),
        }
    }

    fn permits_overlap(&self) -> bool {
        matches!(
            self,
            Self::IntentionalOverlap { .. } | Self::Containment { .. }
        )
    }
}

/// Model validation input metadata and thresholds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelValidationConfig {
    /// Numeric thresholds and budgets.
    pub limits: ValidationLimits,
    /// Parts that must be present in the compiled artifact.
    pub required_parts: BTreeSet<PartInstanceId>,
    /// Expected socket attachments.
    pub expected_attachments: Vec<ExpectedAttachment>,
    /// Explicit pair relationships.
    pub relationships: Vec<PartRelationship>,
}

/// Validate a compiled artifact using default static-asset thresholds.
#[must_use]
pub fn validate_artifact(artifact: &AssetArtifact) -> ModelValidationReport {
    validate_model(artifact, &ModelValidationConfig::default())
}

/// Validate a compiled artifact with explicit validation metadata.
#[must_use]
pub fn validate_model(
    artifact: &AssetArtifact,
    config: &ModelValidationConfig,
) -> ModelValidationReport {
    let mut issues = Vec::new();
    let mut metrics = MetricsBuilder::default();
    let mut part_lookup = BTreeMap::new();

    for part in &artifact.compiled_parts {
        if part_lookup.insert(part.instance_id, part).is_some() {
            push_issue(
                &mut issues,
                ValidationSeverity::Error,
                "duplicate_part_instance",
                [part.instance_id],
                part.generated_by,
                format!(
                    "Part instance {} appears more than once.",
                    part.instance_id.0
                ),
                None,
            );
        }
    }

    for part in &artifact.compiled_parts {
        validate_part(part, config, &mut issues, &mut metrics);
    }

    validate_required_parts(config, &part_lookup, &mut issues);
    validate_expected_attachments(config, &part_lookup, &mut issues);
    let accidental_intersection_count =
        validate_part_pairs(artifact, config, &part_lookup, &mut issues);

    let mut report = ModelValidationReport {
        issues,
        metrics: metrics.finish(
            artifact.compiled_parts.len() as u64,
            accidental_intersection_count,
        ),
    };
    if report.metrics.triangle_count > config.limits.maximum_triangle_count {
        push_issue(
            &mut report.issues,
            ValidationSeverity::Error,
            "excessive_total_triangle_count",
            [],
            None,
            format!(
                "Triangle count {} exceeds budget {}.",
                report.metrics.triangle_count, config.limits.maximum_triangle_count
            ),
            None,
        );
    }
    if report.metrics.part_count > config.limits.maximum_part_count {
        push_issue(
            &mut report.issues,
            ValidationSeverity::Error,
            "excessive_part_count",
            [],
            None,
            format!(
                "Part count {} exceeds budget {}.",
                report.metrics.part_count, config.limits.maximum_part_count
            ),
            None,
        );
    }
    sort_issues(&mut report.issues);
    report
}

fn validate_part(
    part: &CompiledPart,
    config: &ModelValidationConfig,
    issues: &mut Vec<ValidationIssue>,
    metrics: &mut MetricsBuilder,
) {
    let mesh = &part.world_mesh;
    let edge_uses = collect_edge_uses(mesh);
    let closed_manifold = part_is_closed_manifold(&edge_uses);
    metrics.observe_part(part, closed_manifold);

    append_polygon_issues(part, mesh, issues);
    validate_non_finite_values(part, mesh, issues);
    validate_duplicate_vertices(part, mesh, config.limits.duplicate_vertex_epsilon, issues);
    validate_duplicate_faces(part, mesh, issues);
    validate_edges(part, mesh, config, &edge_uses, issues, metrics);
    validate_faces(part, mesh, config, closed_manifold, issues, metrics);
    validate_components(part, mesh, &edge_uses, issues);
}

fn append_polygon_issues(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    issues: &mut Vec<ValidationIssue>,
) {
    for issue in validate_polygon_mesh(mesh).issues {
        let message = match issue.subject {
            Some(subject) => format!("{subject}: {}", issue.message),
            None => issue.message,
        };
        push_issue(
            issues,
            ValidationSeverity::Error,
            format!("polygon_{}", issue.code),
            [part.instance_id],
            part.generated_by,
            message,
            None,
        );
    }
}

fn validate_non_finite_values(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    issues: &mut Vec<ValidationIssue>,
) {
    for (index, position) in mesh.positions.iter().enumerate() {
        if !point_is_finite(*position) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "non_finite_position",
                [part.instance_id],
                part.generated_by,
                format!("Position {index} is not finite."),
                None,
            );
        }
    }
    if !mesh.bounds.is_empty()
        && (!point_is_finite(mesh.bounds.min) || !point_is_finite(mesh.bounds.max))
    {
        push_issue(
            issues,
            ValidationSeverity::Error,
            "non_finite_bounds",
            [part.instance_id],
            part.generated_by,
            "Mesh bounds are not finite.",
            None,
        );
    }
    for (index, normal) in part.triangulated_world.mesh.normals.iter().enumerate() {
        if !point_is_finite(*normal) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "non_finite_normal",
                [part.instance_id],
                part.generated_by,
                format!("Split normal {index} is not finite."),
                None,
            );
        }
    }
}

fn validate_duplicate_vertices(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    epsilon: f32,
    issues: &mut Vec<ValidationIssue>,
) {
    let threshold = epsilon.max(0.0);
    let threshold_squared = threshold * threshold;
    for (left_index, left) in mesh.positions.iter().enumerate() {
        if !point_is_finite(*left) {
            continue;
        }
        for (right_offset, right) in mesh.positions[left_index + 1..].iter().enumerate() {
            if !point_is_finite(*right) {
                continue;
            }
            let right_index = left_index + 1 + right_offset;
            if distance_squared(*left, *right) <= threshold_squared {
                push_issue(
                    issues,
                    ValidationSeverity::Error,
                    "coincident_duplicate_vertices",
                    [part.instance_id],
                    part.generated_by,
                    format!("Vertices {left_index} and {right_index} are coincident."),
                    Some(midpoint(*left, *right)),
                );
            }
        }
    }
}

fn validate_duplicate_faces(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    issues: &mut Vec<ValidationIssue>,
) {
    let mut seen = BTreeMap::<Vec<u32>, usize>::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        let mut key = face.vertices.clone();
        key.sort_unstable();
        if let Some(first) = seen.insert(key, index) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "duplicate_face",
                [part.instance_id],
                face_operation(part, mesh, index),
                format!("Face {index} duplicates face {first}."),
                face_center(mesh, face),
            );
        }
    }
}

fn validate_edges(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    config: &ModelValidationConfig,
    edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>,
    issues: &mut Vec<ValidationIssue>,
    metrics: &mut MetricsBuilder,
) {
    let mut measured_edges = BTreeSet::new();
    for (edge, uses) in edge_uses {
        if let (Some(left), Some(right)) = (
            mesh.positions.get(edge.a as usize),
            mesh.positions.get(edge.b as usize),
        ) && point_is_finite(*left)
            && point_is_finite(*right)
        {
            let length = distance(*left, *right);
            metrics.observe_edge_length(length);
            if measured_edges.insert(*edge) && length < config.limits.minimum_edge_length {
                push_issue(
                    issues,
                    ValidationSeverity::Warning,
                    "minimum_edge_length",
                    [part.instance_id],
                    edge_operation(part, mesh, edge),
                    format!(
                        "Edge {}.{} length {length:.6} is below minimum {:.6}.",
                        edge.a, edge.b, config.limits.minimum_edge_length
                    ),
                    Some(midpoint(*left, *right)),
                );
            }
        }

        if uses.len() > 2 {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "nonmanifold_edge",
                [part.instance_id],
                edge_operation(part, mesh, edge),
                format!(
                    "Edge {}.{} is incident to {} faces.",
                    edge.a,
                    edge.b,
                    uses.len()
                ),
                edge_location(mesh, edge),
            );
        } else if uses.len() == 2 {
            if uses[0].from == uses[1].from && uses[0].to == uses[1].to {
                push_issue(
                    issues,
                    ValidationSeverity::Error,
                    "inconsistent_winding",
                    [part.instance_id],
                    face_operation(part, mesh, uses[0].face),
                    format!(
                        "Faces {} and {} traverse edge {}.{} in the same direction.",
                        uses[0].face, uses[1].face, edge.a, edge.b
                    ),
                    edge_location(mesh, edge),
                );
            }
        } else if uses.len() == 1 && !edge_is_declared_open(mesh, edge) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "unexpected_open_boundary",
                [part.instance_id],
                edge_operation(part, mesh, edge),
                format!("Boundary edge {}.{} is not declared open.", edge.a, edge.b),
                edge_location(mesh, edge),
            );
        }
    }

    for edge in mesh.edge_metadata.iter().filter_map(|(edge, metadata)| {
        edge_metadata_declares_open(metadata.boundary_role).then_some(edge)
    }) {
        if edge_uses.get(edge).map(Vec::len) != Some(1) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "expected_open_boundary_missing",
                [part.instance_id],
                edge_operation(part, mesh, edge),
                format!(
                    "Edge {}.{} is declared open but is not a boundary edge.",
                    edge.a, edge.b
                ),
                edge_location(mesh, edge),
            );
        }
    }
}

fn validate_faces(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    config: &ModelValidationConfig,
    closed_manifold: bool,
    issues: &mut Vec<ValidationIssue>,
    metrics: &mut MetricsBuilder,
) {
    let mesh_center = finite_positions_center(&mesh.positions);
    for (index, face) in mesh.faces.iter().enumerate() {
        let Some(points) = face_points(mesh, face) else {
            continue;
        };
        let area = polygon_area(&points);
        if area < config.limits.minimum_face_area {
            push_issue(
                issues,
                ValidationSeverity::Warning,
                "minimum_face_area",
                [part.instance_id],
                face_operation(part, mesh, index),
                format!(
                    "Face {index} area {area:.6} is below minimum {:.6}.",
                    config.limits.minimum_face_area
                ),
                face_center(mesh, face),
            );
        }

        if let Some(aspect_ratio) = face_aspect_ratio(&points) {
            metrics.observe_aspect_ratio(aspect_ratio);
            if aspect_ratio > config.limits.maximum_aspect_ratio {
                push_issue(
                    issues,
                    ValidationSeverity::Warning,
                    "extreme_aspect_ratio",
                    [part.instance_id],
                    face_operation(part, mesh, index),
                    format!(
                        "Face {index} aspect ratio {aspect_ratio:.3} exceeds {:.3}.",
                        config.limits.maximum_aspect_ratio
                    ),
                    face_center(mesh, face),
                );
            }
        }

        if closed_manifold
            && let (Some(center), Some(normal), Some(mesh_center)) =
                (face_center(mesh, face), face_normal(&points), mesh_center)
            && dot(normal, sub(center, mesh_center)) < -EPSILON
        {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "inverted_normal",
                [part.instance_id],
                face_operation(part, mesh, index),
                format!("Face {index} normal points inward."),
                Some(center),
            );
        }
    }
}

fn validate_components(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>,
    issues: &mut Vec<ValidationIssue>,
) {
    if mesh.faces.len() <= 1 {
        return;
    }
    let components = connected_components(mesh.faces.len(), edge_uses);
    if components > 1 {
        push_issue(
            issues,
            ValidationSeverity::Error,
            "isolated_component",
            [part.instance_id],
            part.generated_by,
            format!("Part contains {components} disconnected face components."),
            None,
        );
    }
}

fn validate_required_parts(
    config: &ModelValidationConfig,
    part_lookup: &BTreeMap<PartInstanceId, &CompiledPart>,
    issues: &mut Vec<ValidationIssue>,
) {
    for part in &config.required_parts {
        if !part_lookup.contains_key(part) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "detached_required_part",
                [*part],
                None,
                format!(
                    "Required part {} is absent from the compiled artifact.",
                    part.0
                ),
                None,
            );
        }
    }
}

fn validate_expected_attachments(
    config: &ModelValidationConfig,
    part_lookup: &BTreeMap<PartInstanceId, &CompiledPart>,
    issues: &mut Vec<ValidationIssue>,
) {
    for expected in &config.expected_attachments {
        let Some(parent) = part_lookup.get(&expected.parent) else {
            push_missing_attachment(expected, "parent part is missing", issues);
            continue;
        };
        let Some(child) = part_lookup.get(&expected.child) else {
            push_missing_attachment(expected, "child part is missing", issues);
            continue;
        };
        let Some(parent_socket) = parent.sockets_world.get(&expected.parent_socket) else {
            push_missing_attachment(expected, "parent socket is missing", issues);
            continue;
        };
        let Some(child_socket) = child.sockets_world.get(&expected.child_socket) else {
            push_missing_attachment(expected, "child socket is missing", issues);
            continue;
        };

        let parent_frame = &parent_socket.local_frame;
        let child_frame = &child_socket.local_frame;
        let origin_distance = distance(parent_frame.origin, child_frame.origin);
        if !origin_distance.is_finite() || origin_distance > expected.max_origin_distance {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "invalid_socket_alignment",
                [expected.parent, expected.child],
                None,
                format!(
                    "Socket origins are {origin_distance:.6} apart; maximum is {:.6}.",
                    expected.max_origin_distance
                ),
                Some(midpoint(parent_frame.origin, child_frame.origin)),
            );
        }
        if !frames_axes_align(parent_frame, child_frame, expected.max_axis_angle_degrees) {
            push_issue(
                issues,
                ValidationSeverity::Error,
                "invalid_socket_alignment",
                [expected.parent, expected.child],
                None,
                format!(
                    "Socket axes exceed {:.3} degrees of angular mismatch.",
                    expected.max_axis_angle_degrees
                ),
                Some(midpoint(parent_frame.origin, child_frame.origin)),
            );
        }

        if let Some(max_clearance) = expected.max_clearance {
            let clearance = part_clearance(parent, child, max_clearance);
            if clearance > max_clearance {
                push_issue(
                    issues,
                    ValidationSeverity::Error,
                    "detached_required_part",
                    [expected.parent, expected.child],
                    None,
                    format!(
                        "Attachment clearance {clearance:.6} exceeds maximum {max_clearance:.6}."
                    ),
                    None,
                );
            }
        }
    }
}

fn push_missing_attachment(
    expected: &ExpectedAttachment,
    detail: &'static str,
    issues: &mut Vec<ValidationIssue>,
) {
    push_issue(
        issues,
        ValidationSeverity::Error,
        "missing_attachment",
        [expected.parent, expected.child],
        None,
        format!(
            "Expected attachment {}.{} -> {}.{} is missing: {detail}.",
            expected.parent.0, expected.parent_socket.0, expected.child.0, expected.child_socket.0
        ),
        None,
    );
}

fn validate_part_pairs(
    artifact: &AssetArtifact,
    config: &ModelValidationConfig,
    part_lookup: &BTreeMap<PartInstanceId, &CompiledPart>,
    issues: &mut Vec<ValidationIssue>,
) -> u64 {
    let mut permitted_overlaps = BTreeSet::new();
    for relationship in &config.relationships {
        if relationship.permits_overlap() {
            permitted_overlaps.insert(relationship.pair());
        }
    }

    let mut accidental_intersections = 0_u64;
    for (left_index, left) in artifact.compiled_parts.iter().enumerate() {
        for right in &artifact.compiled_parts[left_index + 1..] {
            let pair = PartPair::new(left.instance_id, right.instance_id);
            if permitted_overlaps.contains(&pair) {
                continue;
            }
            if bounds_strictly_overlap(
                &left.world_mesh.bounds,
                &right.world_mesh.bounds,
                config.limits.narrow_phase_aabb_margin,
            ) {
                push_issue(
                    issues,
                    ValidationSeverity::Warning,
                    "accidental_aabb_overlap",
                    pair.instances(),
                    pair_operation(left, right),
                    "Part AABBs overlap without explicit relationship metadata.",
                    bounds_overlap_center(&left.world_mesh.bounds, &right.world_mesh.bounds),
                );
                if meshes_intersect(left, right) {
                    accidental_intersections = accidental_intersections.saturating_add(1);
                    push_issue(
                        issues,
                        ValidationSeverity::Error,
                        "triangle_intersection",
                        pair.instances(),
                        pair_operation(left, right),
                        "Part triangles intersect without explicit relationship metadata.",
                        bounds_overlap_center(&left.world_mesh.bounds, &right.world_mesh.bounds),
                    );
                }
            }
        }
    }

    for relationship in &config.relationships {
        match relationship {
            PartRelationship::MinimumClearance {
                first,
                second,
                clearance,
            } => {
                let Some(first_part) = part_lookup.get(first) else {
                    continue;
                };
                let Some(second_part) = part_lookup.get(second) else {
                    continue;
                };
                let actual = part_clearance(first_part, second_part, *clearance);
                if actual < *clearance {
                    push_issue(
                        issues,
                        ValidationSeverity::Error,
                        "minimum_authored_clearance",
                        [*first, *second],
                        pair_operation(first_part, second_part),
                        format!(
                            "Part clearance {actual:.6} is below authored minimum {clearance:.6}."
                        ),
                        bounds_overlap_center(
                            &first_part.world_mesh.bounds,
                            &second_part.world_mesh.bounds,
                        ),
                    );
                }
            }
            PartRelationship::Containment {
                container,
                contained,
            } => {
                let Some(container_part) = part_lookup.get(container) else {
                    continue;
                };
                let Some(contained_part) = part_lookup.get(contained) else {
                    continue;
                };
                if !bounds_contains(
                    &container_part.world_mesh.bounds,
                    &contained_part.world_mesh.bounds,
                    EPSILON,
                ) {
                    push_issue(
                        issues,
                        ValidationSeverity::Error,
                        "declared_containment_missing",
                        [*container, *contained],
                        pair_operation(container_part, contained_part),
                        "Declared contained part is outside the container bounds.",
                        Some(bounds_center(&contained_part.world_mesh.bounds)),
                    );
                }
            }
            PartRelationship::IntentionalOverlap { .. } => {}
        }
    }

    accidental_intersections
}

#[derive(Debug, Default)]
struct MetricsBuilder {
    polygon_count: u64,
    triangle_count: u64,
    quad_count: u64,
    closed_manifold_count: u64,
    minimum_edge: Option<f32>,
    maximum_aspect_ratio: Option<f32>,
    hard_edge_count: u64,
    regions: BTreeSet<(PartInstanceId, RegionId)>,
    provenance_faces: u64,
}

impl MetricsBuilder {
    fn observe_part(&mut self, part: &CompiledPart, closed_manifold: bool) {
        let mesh = &part.world_mesh;
        self.polygon_count = self.polygon_count.saturating_add(mesh.faces.len() as u64);
        self.triangle_count = self
            .triangle_count
            .saturating_add((part.triangulated_world.mesh.indices.len() / 3) as u64);
        self.quad_count = self.quad_count.saturating_add(
            mesh.faces
                .iter()
                .filter(|face| face.vertices.len() == 4)
                .count() as u64,
        );
        if closed_manifold {
            self.closed_manifold_count = self.closed_manifold_count.saturating_add(1);
        }
        self.hard_edge_count = self.hard_edge_count.saturating_add(
            mesh.edge_metadata
                .values()
                .filter(|metadata| {
                    metadata.classification == EdgeClassification::Hard
                        || metadata.boundary_role != BoundaryRole::Smooth
                })
                .count() as u64,
        );

        for (index, _face) in mesh.faces.iter().enumerate() {
            if let Some(metadata) = mesh.face_metadata.get(index) {
                if metadata.part_definition.is_some() && metadata.part_instance.is_some() {
                    self.provenance_faces = self.provenance_faces.saturating_add(1);
                }
                if let Some(region) = metadata.region {
                    let instance = metadata.part_instance.unwrap_or(part.instance_id);
                    self.regions.insert((instance, region));
                }
            }
        }
    }

    fn observe_edge_length(&mut self, length: f32) {
        if !length.is_finite() {
            return;
        }
        self.minimum_edge = Some(
            self.minimum_edge
                .map_or(length, |current| current.min(length)),
        );
    }

    fn observe_aspect_ratio(&mut self, aspect_ratio: f32) {
        if !aspect_ratio.is_finite() {
            return;
        }
        self.maximum_aspect_ratio = Some(
            self.maximum_aspect_ratio
                .map_or(aspect_ratio, |current| current.max(aspect_ratio)),
        );
    }

    fn finish(self, part_count: u64, accidental_intersection_count: u64) -> QualityMetrics {
        QualityMetrics {
            part_count,
            polygon_count: self.polygon_count,
            triangle_count: self.triangle_count,
            quad_fraction: if self.polygon_count == 0 {
                0.0
            } else {
                self.quad_count as f32 / self.polygon_count as f32
            },
            manifold_closed_part_fraction: if part_count == 0 {
                1.0
            } else {
                self.closed_manifold_count as f32 / part_count as f32
            },
            minimum_edge: self.minimum_edge,
            maximum_aspect_ratio: self.maximum_aspect_ratio,
            hard_edge_count: self.hard_edge_count,
            region_count: self.regions.len() as u64,
            provenance_coverage: if self.polygon_count == 0 {
                1.0
            } else {
                self.provenance_faces as f32 / self.polygon_count as f32
            },
            accidental_intersection_count,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PartPair {
    first: PartInstanceId,
    second: PartInstanceId,
}

impl PartPair {
    fn new(first: PartInstanceId, second: PartInstanceId) -> Self {
        if first <= second {
            Self { first, second }
        } else {
            Self {
                first: second,
                second: first,
            }
        }
    }

    fn instances(self) -> [PartInstanceId; 2] {
        [self.first, self.second]
    }
}

#[derive(Debug, Copy, Clone)]
struct EdgeUse {
    face: usize,
    from: u32,
    to: u32,
}

#[derive(Debug, Copy, Clone)]
struct Triangle {
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
}

fn collect_edge_uses(mesh: &PolygonMesh) -> BTreeMap<EdgeKey, Vec<EdgeUse>> {
    let mut edge_uses = BTreeMap::<EdgeKey, Vec<EdgeUse>>::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        if face.vertices.len() < 2 {
            continue;
        }
        for edge_index in 0..face.vertices.len() {
            let from = face.vertices[edge_index];
            let to = face.vertices[(edge_index + 1) % face.vertices.len()];
            if from as usize >= mesh.positions.len() || to as usize >= mesh.positions.len() {
                continue;
            }
            edge_uses
                .entry(EdgeKey::new(from, to))
                .or_default()
                .push(EdgeUse {
                    face: face_index,
                    from,
                    to,
                });
        }
    }
    edge_uses
}

fn part_is_closed_manifold(edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>) -> bool {
    !edge_uses.is_empty()
        && edge_uses.values().all(|uses| {
            uses.len() == 2 && !(uses[0].from == uses[1].from && uses[0].to == uses[1].to)
        })
}

fn connected_components(face_count: usize, edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>) -> usize {
    let mut neighbors = vec![BTreeSet::<usize>::new(); face_count];
    for uses in edge_uses.values() {
        for left in uses {
            for right in uses {
                if left.face != right.face && left.face < face_count && right.face < face_count {
                    neighbors[left.face].insert(right.face);
                }
            }
        }
    }

    let mut seen = vec![false; face_count];
    let mut components = 0_usize;
    for start in 0..face_count {
        if seen[start] {
            continue;
        }
        components += 1;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(face) = stack.pop() {
            for neighbor in &neighbors[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    stack.push(*neighbor);
                }
            }
        }
    }
    components
}

fn edge_is_declared_open(mesh: &PolygonMesh, edge: &EdgeKey) -> bool {
    mesh.edge_metadata
        .get(edge)
        .is_some_and(|metadata| edge_metadata_declares_open(metadata.boundary_role))
}

fn edge_metadata_declares_open(role: BoundaryRole) -> bool {
    matches!(
        role,
        BoundaryRole::OpenBoundary | BoundaryRole::SeamCandidate
    )
}

fn face_operation(
    part: &CompiledPart,
    mesh: &PolygonMesh,
    face_index: usize,
) -> Option<OperationId> {
    mesh.face_metadata
        .get(face_index)
        .and_then(|metadata| metadata.operation)
        .or(part.generated_by)
}

fn edge_operation(part: &CompiledPart, mesh: &PolygonMesh, edge: &EdgeKey) -> Option<OperationId> {
    mesh.edge_metadata
        .get(edge)
        .and_then(|metadata| metadata.operation)
        .or(part.generated_by)
}

fn face_points(mesh: &PolygonMesh, face: &PolygonFace) -> Option<Vec<[f32; 3]>> {
    if face.vertices.len() < 3 {
        return None;
    }
    let mut points = Vec::with_capacity(face.vertices.len());
    for vertex in &face.vertices {
        let point = *mesh.positions.get(*vertex as usize)?;
        if !point_is_finite(point) {
            return None;
        }
        points.push(point);
    }
    Some(points)
}

fn face_center(mesh: &PolygonMesh, face: &PolygonFace) -> Option<[f32; 3]> {
    let points = face_points(mesh, face)?;
    Some(points_center(&points))
}

fn edge_location(mesh: &PolygonMesh, edge: &EdgeKey) -> Option<[f32; 3]> {
    let left = *mesh.positions.get(edge.a as usize)?;
    let right = *mesh.positions.get(edge.b as usize)?;
    (point_is_finite(left) && point_is_finite(right)).then_some(midpoint(left, right))
}

fn polygon_area(points: &[[f32; 3]]) -> f32 {
    normal_sum(points).map_or(0.0, |normal| 0.5 * length(normal))
}

fn face_normal(points: &[[f32; 3]]) -> Option<[f32; 3]> {
    normalize(normal_sum(points)?)
}

fn normal_sum(points: &[[f32; 3]]) -> Option<[f32; 3]> {
    if points.len() < 3 || !points.iter().copied().all(point_is_finite) {
        return None;
    }
    let mut normal = [0.0, 0.0, 0.0];
    for (index, current) in points.iter().enumerate() {
        let next = points[(index + 1) % points.len()];
        normal[0] += (current[1] - next[1]) * (current[2] + next[2]);
        normal[1] += (current[2] - next[2]) * (current[0] + next[0]);
        normal[2] += (current[0] - next[0]) * (current[1] + next[1]);
    }
    Some(normal)
}

fn face_aspect_ratio(points: &[[f32; 3]]) -> Option<f32> {
    if points.len() < 3 {
        return None;
    }
    let mut shortest = f32::INFINITY;
    let mut longest = 0.0_f32;
    for index in 0..points.len() {
        let edge = distance(points[index], points[(index + 1) % points.len()]);
        if !edge.is_finite() || edge <= EPSILON {
            return None;
        }
        shortest = shortest.min(edge);
        longest = longest.max(edge);
    }
    Some(longest / shortest)
}

fn finite_positions_center(points: &[[f32; 3]]) -> Option<[f32; 3]> {
    let finite = points
        .iter()
        .copied()
        .filter(|point| point_is_finite(*point))
        .collect::<Vec<_>>();
    (!finite.is_empty()).then(|| points_center(&finite))
}

fn points_center(points: &[[f32; 3]]) -> [f32; 3] {
    let mut sum = [0.0, 0.0, 0.0];
    for point in points {
        sum = add(sum, *point);
    }
    scale(sum, 1.0 / points.len() as f32)
}

fn frames_axes_align(
    parent: &shape_asset::Frame3,
    child: &shape_asset::Frame3,
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
        let Some(left) = normalize(left) else {
            return false;
        };
        let Some(right) = normalize(right) else {
            return false;
        };
        dot(left, right) >= min_dot
    })
}

fn part_clearance(left: &CompiledPart, right: &CompiledPart, exact_below: f32) -> f32 {
    let aabb_distance = bounds_distance(&left.world_mesh.bounds, &right.world_mesh.bounds);
    if aabb_distance > exact_below {
        return aabb_distance;
    }
    triangle_mesh_distance(left, right).unwrap_or(aabb_distance)
}

fn meshes_intersect(left: &CompiledPart, right: &CompiledPart) -> bool {
    let left_triangles = triangles(&left.triangulated_world);
    let right_triangles = triangles(&right.triangulated_world);
    left_triangles.iter().any(|left_triangle| {
        right_triangles.iter().any(|right_triangle| {
            triangle_bounds_overlap(left_triangle, right_triangle, EPSILON)
                && triangles_intersect(*left_triangle, *right_triangle)
        })
    })
}

fn triangle_mesh_distance(left: &CompiledPart, right: &CompiledPart) -> Option<f32> {
    let left_triangles = triangles(&left.triangulated_world);
    let right_triangles = triangles(&right.triangulated_world);
    if left_triangles.is_empty() || right_triangles.is_empty() {
        return None;
    }
    let mut minimum = f32::INFINITY;
    for left_triangle in &left_triangles {
        for right_triangle in &right_triangles {
            let distance = triangle_distance(*left_triangle, *right_triangle);
            if distance <= EPSILON {
                return Some(0.0);
            }
            minimum = minimum.min(distance);
        }
    }
    minimum.is_finite().then_some(minimum)
}

fn triangles(mesh: &TriangulatedPolygonMesh) -> Vec<Triangle> {
    mesh.mesh
        .indices
        .chunks_exact(3)
        .filter_map(|indices| {
            let a = *mesh.mesh.positions.get(indices[0] as usize)?;
            let b = *mesh.mesh.positions.get(indices[1] as usize)?;
            let c = *mesh.mesh.positions.get(indices[2] as usize)?;
            (point_is_finite(a) && point_is_finite(b) && point_is_finite(c)).then_some(Triangle {
                a,
                b,
                c,
            })
        })
        .collect()
}

fn triangle_bounds_overlap(left: &Triangle, right: &Triangle, epsilon: f32) -> bool {
    let left_bounds = triangle_bounds(left);
    let right_bounds = triangle_bounds(right);
    bounds_overlap(&left_bounds, &right_bounds, epsilon)
}

fn triangle_bounds(triangle: &Triangle) -> MeshBounds {
    MeshBounds {
        min: [
            triangle.a[0].min(triangle.b[0]).min(triangle.c[0]),
            triangle.a[1].min(triangle.b[1]).min(triangle.c[1]),
            triangle.a[2].min(triangle.b[2]).min(triangle.c[2]),
        ],
        max: [
            triangle.a[0].max(triangle.b[0]).max(triangle.c[0]),
            triangle.a[1].max(triangle.b[1]).max(triangle.c[1]),
            triangle.a[2].max(triangle.b[2]).max(triangle.c[2]),
        ],
    }
}

fn triangles_intersect(left: Triangle, right: Triangle) -> bool {
    triangle_edges(left)
        .into_iter()
        .any(|(start, end)| segment_triangle_intersects(start, end, right))
        || triangle_edges(right)
            .into_iter()
            .any(|(start, end)| segment_triangle_intersects(start, end, left))
        || point_on_triangle(left.a, right)
        || point_on_triangle(right.a, left)
}

fn triangle_distance(left: Triangle, right: Triangle) -> f32 {
    if triangles_intersect(left, right) {
        return 0.0;
    }
    [
        point_triangle_distance(left.a, right),
        point_triangle_distance(left.b, right),
        point_triangle_distance(left.c, right),
        point_triangle_distance(right.a, left),
        point_triangle_distance(right.b, left),
        point_triangle_distance(right.c, left),
    ]
    .into_iter()
    .fold(f32::INFINITY, f32::min)
}

fn triangle_edges(triangle: Triangle) -> [([f32; 3], [f32; 3]); 3] {
    [
        (triangle.a, triangle.b),
        (triangle.b, triangle.c),
        (triangle.c, triangle.a),
    ]
}

fn segment_triangle_intersects(start: [f32; 3], end: [f32; 3], triangle: Triangle) -> bool {
    let direction = sub(end, start);
    let edge1 = sub(triangle.b, triangle.a);
    let edge2 = sub(triangle.c, triangle.a);
    let h = cross(direction, edge2);
    let determinant = dot(edge1, h);
    if determinant.abs() <= EPSILON {
        return false;
    }
    let inverse_determinant = 1.0 / determinant;
    let s = sub(start, triangle.a);
    let u = inverse_determinant * dot(s, h);
    if !(-EPSILON..=1.0 + EPSILON).contains(&u) {
        return false;
    }
    let q = cross(s, edge1);
    let v = inverse_determinant * dot(direction, q);
    if v < -EPSILON || u + v > 1.0 + EPSILON {
        return false;
    }
    let t = inverse_determinant * dot(edge2, q);
    (-EPSILON..=1.0 + EPSILON).contains(&t)
}

fn point_on_triangle(point: [f32; 3], triangle: Triangle) -> bool {
    let normal = cross(sub(triangle.b, triangle.a), sub(triangle.c, triangle.a));
    let Some(unit_normal) = normalize(normal) else {
        return false;
    };
    if dot(sub(point, triangle.a), unit_normal).abs() > EPSILON {
        return false;
    }
    point_in_triangle(point, triangle)
}

fn point_in_triangle(point: [f32; 3], triangle: Triangle) -> bool {
    let v0 = sub(triangle.c, triangle.a);
    let v1 = sub(triangle.b, triangle.a);
    let v2 = sub(point, triangle.a);
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    let denominator = dot00 * dot11 - dot01 * dot01;
    if denominator.abs() <= EPSILON {
        return false;
    }
    let inverse_denominator = 1.0 / denominator;
    let u = (dot11 * dot02 - dot01 * dot12) * inverse_denominator;
    let v = (dot00 * dot12 - dot01 * dot02) * inverse_denominator;
    u >= -EPSILON && v >= -EPSILON && u + v <= 1.0 + EPSILON
}

fn point_triangle_distance(point: [f32; 3], triangle: Triangle) -> f32 {
    let normal = cross(sub(triangle.b, triangle.a), sub(triangle.c, triangle.a));
    let Some(unit_normal) = normalize(normal) else {
        return f32::INFINITY;
    };
    let signed_distance = dot(sub(point, triangle.a), unit_normal);
    let projected = sub(point, scale(unit_normal, signed_distance));
    if point_in_triangle(projected, triangle) {
        signed_distance.abs()
    } else {
        triangle_edges(triangle)
            .into_iter()
            .map(|(start, end)| point_segment_distance(point, start, end))
            .fold(f32::INFINITY, f32::min)
    }
}

fn point_segment_distance(point: [f32; 3], start: [f32; 3], end: [f32; 3]) -> f32 {
    let segment = sub(end, start);
    let length_squared = dot(segment, segment);
    if length_squared <= EPSILON {
        return distance(point, start);
    }
    let t = (dot(sub(point, start), segment) / length_squared).clamp(0.0, 1.0);
    distance(point, add(start, scale(segment, t)))
}

fn bounds_strictly_overlap(left: &MeshBounds, right: &MeshBounds, epsilon: f32) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    (0..3).all(|axis| {
        left.max[axis].min(right.max[axis]) - left.min[axis].max(right.min[axis]) > epsilon
    })
}

fn bounds_overlap(left: &MeshBounds, right: &MeshBounds, epsilon: f32) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    (0..3).all(|axis| {
        left.min[axis] <= right.max[axis] + epsilon && right.min[axis] <= left.max[axis] + epsilon
    })
}

fn bounds_distance(left: &MeshBounds, right: &MeshBounds) -> f32 {
    if left.is_empty() || right.is_empty() {
        return f32::INFINITY;
    }
    let mut squared = 0.0_f32;
    for axis in 0..3 {
        let gap = if left.max[axis] < right.min[axis] {
            right.min[axis] - left.max[axis]
        } else if right.max[axis] < left.min[axis] {
            left.min[axis] - right.max[axis]
        } else {
            0.0
        };
        squared += gap * gap;
    }
    squared.sqrt()
}

fn bounds_overlap_center(left: &MeshBounds, right: &MeshBounds) -> Option<[f32; 3]> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let min = [
        left.min[0].max(right.min[0]),
        left.min[1].max(right.min[1]),
        left.min[2].max(right.min[2]),
    ];
    let max = [
        left.max[0].min(right.max[0]),
        left.max[1].min(right.max[1]),
        left.max[2].min(right.max[2]),
    ];
    Some(midpoint(min, max))
}

fn bounds_contains(container: &MeshBounds, contained: &MeshBounds, epsilon: f32) -> bool {
    if container.is_empty() || contained.is_empty() {
        return false;
    }
    (0..3).all(|axis| {
        container.min[axis] <= contained.min[axis] + epsilon
            && container.max[axis] + epsilon >= contained.max[axis]
    })
}

fn bounds_center(bounds: &MeshBounds) -> [f32; 3] {
    midpoint(bounds.min, bounds.max)
}

fn pair_operation(left: &CompiledPart, right: &CompiledPart) -> Option<OperationId> {
    (left.generated_by == right.generated_by)
        .then_some(left.generated_by)
        .flatten()
}

fn push_issue(
    issues: &mut Vec<ValidationIssue>,
    severity: ValidationSeverity,
    code: impl Into<String>,
    part_instances: impl IntoIterator<Item = PartInstanceId>,
    operation: Option<OperationId>,
    message: impl Into<String>,
    location: Option<[f32; 3]>,
) {
    let mut part_instances = part_instances.into_iter().collect::<Vec<_>>();
    part_instances.sort_unstable();
    part_instances.dedup();
    issues.push(ValidationIssue {
        severity,
        code: code.into(),
        part_instances,
        operation,
        message: message.into(),
        location: location.filter(|point| point_is_finite(*point)),
    });
}

fn sort_issues(issues: &mut [ValidationIssue]) {
    issues.sort_by(|left, right| {
        severity_rank(left.severity)
            .cmp(&severity_rank(right.severity))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.part_instances.cmp(&right.part_instances))
            .then_with(|| left.operation.cmp(&right.operation))
            .then_with(|| location_bits(left.location).cmp(&location_bits(right.location)))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn severity_rank(severity: ValidationSeverity) -> u8 {
    match severity {
        ValidationSeverity::Error => 0,
        ValidationSeverity::Warning => 1,
        ValidationSeverity::Info => 2,
    }
}

fn location_bits(location: Option<[f32; 3]>) -> Option<[u32; 3]> {
    location.map(|point| point.map(f32::to_bits))
}

fn point_is_finite(point: [f32; 3]) -> bool {
    point.iter().copied().all(f32::is_finite)
}

fn midpoint(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    scale(add(left, right), 0.5)
}

fn distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    distance_squared(left, right).sqrt()
}

fn distance_squared(left: [f32; 3], right: [f32; 3]) -> f32 {
    dot(sub(left, right), sub(left, right))
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    if !point_is_finite(vector) {
        return None;
    }
    let length = length(vector);
    (length.is_finite() && length > EPSILON).then_some(scale(vector, 1.0 / length))
}

fn length(vector: [f32; 3]) -> f32 {
    dot(vector, vector).sqrt()
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}
