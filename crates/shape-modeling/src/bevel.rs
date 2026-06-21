use std::collections::{BTreeMap, BTreeSet, VecDeque};

use shape_asset::{GeometrySource, ModelingOperationSpec, OperationId, PartDefinition};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, PolygonFace, PolygonMesh, SplitNormal,
    TriangulatedPolygonMesh, build_adjacency, compute_face_normals, triangulate_polygon_mesh,
};

use crate::ModelingError;

const EPSILON: f32 = 1.0e-6;

/// Generator families with explicit, bounded bevel semantics.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BevelSource {
    /// Rounded-box edge and corner bands.
    RoundedBox,
    /// Plate front/back perimeter bevels.
    Plate,
    /// Cylinder cap rims.
    Cylinder,
    /// Frustum cap rims.
    Frustum,
    /// Closed sweep profile corners before sweep generation.
    SweepProfileCorners,
}

impl BevelSource {
    #[must_use]
    fn name(self) -> &'static str {
        match self {
            Self::RoundedBox => "RoundedBox",
            Self::Plate => "Plate",
            Self::Cylinder => "Cylinder",
            Self::Frustum => "Frustum",
            Self::SweepProfileCorners => "Sweep profile corners",
        }
    }

    #[must_use]
    fn rejects_touching_bands(self) -> bool {
        matches!(self, Self::SweepProfileCorners)
    }
}

/// Semantic edge classes addressable by the bevel capability layer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticEdgeClass {
    /// All edge classes supported by the source.
    AllSupported,
    /// Rounded-box feature edges.
    RoundedBoxEdges,
    /// Plate front/back perimeter edges.
    PlatePerimeter,
    /// Cylinder cap rim edges.
    CylinderCapRims,
    /// Frustum cap rim edges.
    FrustumCapRims,
    /// Sweep profile polygon corners.
    SweepProfileCorners,
}

/// Explicit bevel profile controls used by supported generators.
#[derive(Debug, Clone, PartialEq)]
pub struct BevelProfile {
    /// Bevel width or radius in source-local units.
    pub radius: f32,
    /// Segment count across the bevel band.
    pub segments: u32,
    /// Profile shaping exponent. The schema-2 operation does not serialize this yet.
    pub profile_exponent: f32,
    /// Whether preview normals should stay hard across adjacent non-bevel surfaces.
    pub harden_adjacent_normals: bool,
    /// Semantic edge classes affected by this profile.
    pub affected_edge_classes: Vec<SemanticEdgeClass>,
}

impl BevelProfile {
    /// Create a profile from the schema-2 operation fields.
    #[must_use]
    pub fn from_schema_fields(radius: f32, segments: u32) -> Self {
        Self {
            radius,
            segments,
            profile_exponent: 1.0,
            harden_adjacent_normals: true,
            affected_edge_classes: vec![SemanticEdgeClass::AllSupported],
        }
    }

    /// Return true when this profile requests no generated bevel topology.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.radius.abs() <= EPSILON
    }
}

/// Bounded bevel support for one source instance.
#[derive(Debug, Clone, PartialEq)]
pub struct BevelSupport {
    /// Supported source family.
    pub source: BevelSource,
    /// Maximum safe radius before bands overlap or faces collapse.
    pub max_radius: f32,
    /// Source-local semantic edge classes supported by this capability.
    pub semantic_edge_classes: Vec<SemanticEdgeClass>,
}

/// Capability contract for explicit bevelable sources.
pub trait BevelCapability {
    /// Supported source family.
    fn source(&self) -> BevelSource;

    /// Maximum safe radius before bands overlap or faces collapse.
    fn max_radius(&self) -> f32;

    /// Source-local semantic edge classes supported by this capability.
    fn semantic_edge_classes(&self) -> &[SemanticEdgeClass];

    /// Validate one profile against this concrete source instance.
    fn validate_profile(
        &self,
        operation: OperationId,
        profile: &BevelProfile,
    ) -> Result<(), ModelingError> {
        validate_profile_scalars(profile)?;
        if profile.is_disabled() {
            return Ok(());
        }
        if profile.segments == 0 {
            return Err(ModelingError::InvalidInput(format!(
                "operation {:?} bevel segments must be positive when radius is positive",
                operation
            )));
        }
        let max_radius = self.max_radius();
        if !max_radius.is_finite() || max_radius <= EPSILON {
            return Err(ModelingError::InvalidInput(format!(
                "operation {:?} cannot bevel {} because no positive bevel band remains",
                operation,
                self.source().name()
            )));
        }
        let overlaps = if self.source().rejects_touching_bands() {
            profile.radius >= max_radius - EPSILON
        } else {
            profile.radius > max_radius + EPSILON
        };
        if overlaps {
            return Err(ModelingError::InvalidInput(format!(
                "operation {:?} bevel radius {:.6} exceeds safe {} limit {:.6}",
                operation,
                profile.radius,
                self.source().name(),
                max_radius
            )));
        }
        Ok(())
    }
}

impl BevelCapability for BevelSupport {
    fn source(&self) -> BevelSource {
        self.source
    }

    fn max_radius(&self) -> f32 {
        self.max_radius
    }

    fn semantic_edge_classes(&self) -> &[SemanticEdgeClass] {
        &self.semantic_edge_classes
    }
}

/// Return the explicit bevel capability for a source, when one exists.
pub fn capability_for_source(
    source: &GeometrySource,
) -> Result<Option<BevelSupport>, ModelingError> {
    let support = match source {
        GeometrySource::RoundedBox { half_extents, .. } => {
            let half = positive_triplet(*half_extents, "rounded_box.half_extents")?;
            BevelSupport {
                source: BevelSource::RoundedBox,
                max_radius: half[0].min(half[1]).min(half[2]),
                semantic_edge_classes: vec![SemanticEdgeClass::RoundedBoxEdges],
            }
        }
        GeometrySource::Cylinder { radius, height, .. } => BevelSupport {
            source: BevelSource::Cylinder,
            max_radius: finite_positive(*radius, "cylinder.radius")?
                .min(finite_positive(*height, "cylinder.height")? * 0.5),
            semantic_edge_classes: vec![SemanticEdgeClass::CylinderCapRims],
        },
        GeometrySource::Frustum {
            bottom_radius,
            top_radius,
            height,
            ..
        } => BevelSupport {
            source: BevelSource::Frustum,
            max_radius: finite_positive(*bottom_radius, "frustum.bottom_radius")?
                .min(finite_positive(*top_radius, "frustum.top_radius")?)
                .min(finite_positive(*height, "frustum.height")? * 0.5),
            semantic_edge_classes: vec![SemanticEdgeClass::FrustumCapRims],
        },
        GeometrySource::Plate { size, thickness } => {
            let width = finite_positive(size[0], "plate.size.x")?;
            let height = finite_positive(size[1], "plate.size.z")?;
            BevelSupport {
                source: BevelSource::Plate,
                max_radius: (finite_positive(*thickness, "plate.thickness")? * 0.5)
                    .min(width * 0.25)
                    .min(height * 0.25),
                semantic_edge_classes: vec![SemanticEdgeClass::PlatePerimeter],
            }
        }
        GeometrySource::Sweep { profile, .. } => BevelSupport {
            source: BevelSource::SweepProfileCorners,
            max_radius: max_sweep_profile_radius(profile)?,
            semantic_edge_classes: vec![SemanticEdgeClass::SweepProfileCorners],
        },
        GeometrySource::Lathe { .. }
        | GeometrySource::LiteralMesh { .. }
        | GeometrySource::ReservedBooleanResult { .. } => return Ok(None),
    };
    Ok(Some(support))
}

/// Return the active explicit bevel profile. Later operations override earlier ones.
pub fn active_bevel_profile(definition: &PartDefinition) -> Option<(OperationId, BevelProfile)> {
    definition
        .geometry
        .operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            ModelingOperationSpec::SetBevelProfile {
                operation,
                radius,
                segments,
            } => Some((
                *operation,
                BevelProfile::from_schema_fields(*radius, *segments),
            )),
            _ => None,
        })
}

/// Validate an explicit bevel operation against the source family it targets.
pub fn validate_explicit_bevel(definition: &PartDefinition) -> Result<(), ModelingError> {
    let Some((operation, profile)) = active_bevel_profile(definition) else {
        return Ok(());
    };
    validate_profile_scalars(&profile)?;
    if profile.is_disabled() {
        return Ok(());
    }
    let Some(capability) = capability_for_source(&definition.geometry.source)? else {
        return Err(ModelingError::UnsupportedOperation {
            operation,
            reason:
                "SetBevelProfile is supported only for RoundedBox, Plate, Cylinder, Frustum, and Sweep profile corners"
                    .to_owned(),
        });
    };
    capability.validate_profile(operation, &profile)
}

/// Return a sweep profile with explicit corner bevels applied, if requested.
pub fn sweep_profile_for_definition(
    definition: &PartDefinition,
) -> Result<Vec<[f32; 2]>, ModelingError> {
    let GeometrySource::Sweep { profile, .. } = &definition.geometry.source else {
        return Err(ModelingError::InvalidInput(
            "sweep profile bevel requested for a non-sweep source".to_owned(),
        ));
    };
    let Some((operation, bevel_profile)) = active_bevel_profile(definition) else {
        return Ok(profile.clone());
    };
    if bevel_profile.is_disabled() {
        return Ok(profile.clone());
    }
    let capability = capability_for_source(&definition.geometry.source)?.ok_or_else(|| {
        ModelingError::UnsupportedOperation {
            operation,
            reason: "sweep profile bevel capability was not available".to_owned(),
        }
    })?;
    capability.validate_profile(operation, &bevel_profile)?;
    bevel_sweep_profile(profile, &bevel_profile)
}

/// Apply a deterministic corner bevel to a closed 2D sweep profile.
pub fn bevel_sweep_profile(
    profile: &[[f32; 2]],
    bevel_profile: &BevelProfile,
) -> Result<Vec<[f32; 2]>, ModelingError> {
    let profile = normalize_closed_profile(profile)?;
    if bevel_profile.is_disabled() {
        return Ok(profile);
    }
    validate_profile_scalars(bevel_profile)?;
    if bevel_profile.segments == 0 {
        return Err(ModelingError::InvalidInput(
            "sweep profile bevel segments must be positive when radius is positive".to_owned(),
        ));
    }
    let max_radius = max_sweep_profile_radius(&profile)?;
    if bevel_profile.radius >= max_radius - EPSILON {
        return Err(ModelingError::InvalidInput(format!(
            "sweep profile bevel radius {:.6} leaves no remaining edge between corner bands",
            bevel_profile.radius
        )));
    }

    let orientation = signed_area(&profile).signum();
    let segments = bevel_profile.segments as usize;
    let radius = bevel_profile.radius;
    let mut beveled = Vec::with_capacity(profile.len() * (segments + 1));
    for index in 0..profile.len() {
        let previous = profile[(index + profile.len() - 1) % profile.len()];
        let current = profile[index];
        let next = profile[(index + 1) % profile.len()];
        let incoming = normalize2(sub2(previous, current))?;
        let outgoing = normalize2(sub2(next, current))?;
        let turn = cross2(sub2(current, previous), sub2(next, current));
        if turn * orientation <= EPSILON {
            return Err(ModelingError::InvalidInput(
                "sweep profile bevel supports convex profile corners only".to_owned(),
            ));
        }
        let start = add2(current, scale2(incoming, radius));
        let end = add2(current, scale2(outgoing, radius));
        for step in 0..=segments {
            let t = shaped_parameter(
                step as f32 / segments as f32,
                bevel_profile.profile_exponent,
            );
            beveled.push(quadratic2(start, current, end, t));
        }
    }
    Ok(beveled)
}

/// Compute area-weighted split normals using existing hard-edge metadata.
pub fn compute_weighted_split_normal_groups(
    mesh: &PolygonMesh,
) -> Result<Vec<SplitNormal>, ModelingError> {
    let adjacency = build_adjacency(mesh)?;
    let face_normals = compute_face_normals(mesh)?;
    let face_areas = mesh
        .faces
        .iter()
        .map(|face| polygon_area(mesh, &face.vertices))
        .collect::<Result<Vec<_>, _>>()?;
    let mut groups = Vec::new();

    for (vertex_index, incident_faces) in adjacency.vertex_faces.iter().enumerate() {
        let mut remaining = incident_faces.iter().copied().collect::<BTreeSet<_>>();
        while let Some(start) = remaining.iter().next().copied() {
            let mut faces = Vec::new();
            let mut queue = VecDeque::from([start]);
            remaining.remove(&start);
            while let Some(face_index) = queue.pop_front() {
                faces.push(face_index);
                for neighbor in &adjacency.face_neighbors[face_index] {
                    if remaining.contains(neighbor)
                        && faces_are_smooth_neighbors(
                            mesh,
                            face_index,
                            *neighbor,
                            vertex_index as u32,
                        )
                    {
                        remaining.remove(neighbor);
                        queue.push_back(*neighbor);
                    }
                }
            }
            faces.sort_unstable();
            let mut normal = [0.0, 0.0, 0.0];
            for face in &faces {
                normal = add3(normal, scale3(face_normals[*face], face_areas[*face]));
            }
            groups.push(SplitNormal {
                vertex: vertex_index as u32,
                faces,
                normal: normalize3_or_default(normal),
            });
        }
    }

    groups.sort_by(|left, right| {
        left.vertex
            .cmp(&right.vertex)
            .then_with(|| left.faces.cmp(&right.faces))
    });
    Ok(groups)
}

/// Compute one representative area-weighted normal per source vertex.
pub fn compute_weighted_vertex_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, ModelingError> {
    let groups = compute_weighted_split_normal_groups(mesh)?;
    let mut normals = vec![[0.0, 0.0, 0.0]; mesh.positions.len()];
    for group in groups {
        normals[group.vertex as usize] = add3(normals[group.vertex as usize], group.normal);
    }
    Ok(normals.into_iter().map(normalize3_or_default).collect())
}

/// Triangulate a polygon mesh and replace preview normals with area-weighted split normals.
pub fn triangulate_with_weighted_normals(
    mesh: &PolygonMesh,
) -> Result<TriangulatedPolygonMesh, ModelingError> {
    let groups = compute_weighted_split_normal_groups(mesh)?;
    let mut normal_lookup = BTreeMap::new();
    for group in groups {
        for face in &group.faces {
            normal_lookup.insert((group.vertex, *face), group.normal);
        }
    }

    let mut vertex_id_to_source = BTreeMap::new();
    for (index, id) in mesh.vertex_ids.iter().enumerate() {
        vertex_id_to_source.insert(*id, index as u32);
    }
    let mut face_id_to_index = BTreeMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        face_id_to_index.insert(face.id, index);
    }

    let mut triangulated = triangulate_polygon_mesh(mesh)?;
    for (triangle_index, triangle) in triangulated.mesh.indices.chunks_exact(3).enumerate() {
        let source_face = face_id_to_index
            .get(&triangulated.triangle_to_polygon_face[triangle_index])
            .copied()
            .ok_or_else(|| {
                ModelingError::InvalidInput(
                    "triangulation returned an unknown polygon face id".to_owned(),
                )
            })?;
        for output_vertex in triangle {
            let vertex_id = triangulated.vertex_ids[*output_vertex as usize];
            let source_vertex = vertex_id_to_source
                .get(&vertex_id)
                .copied()
                .ok_or_else(|| {
                    ModelingError::InvalidInput(
                        "triangulation returned an unknown source vertex id".to_owned(),
                    )
                })?;
            let normal = normal_lookup
                .get(&(source_vertex, source_face))
                .copied()
                .ok_or_else(|| {
                    ModelingError::InvalidInput(
                        "missing weighted split normal for triangulated vertex".to_owned(),
                    )
                })?;
            triangulated.mesh.normals[*output_vertex as usize] = normal;
        }
    }
    Ok(triangulated)
}

fn validate_profile_scalars(profile: &BevelProfile) -> Result<(), ModelingError> {
    if !profile.radius.is_finite() || profile.radius < 0.0 {
        return Err(ModelingError::InvalidInput(
            "bevel radius must be finite and non-negative".to_owned(),
        ));
    }
    if !profile.profile_exponent.is_finite() || profile.profile_exponent <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "bevel profile exponent must be finite and positive".to_owned(),
        ));
    }
    Ok(())
}

fn max_sweep_profile_radius(profile: &[[f32; 2]]) -> Result<f32, ModelingError> {
    let profile = normalize_closed_profile(profile)?;
    let mut min_edge = f32::INFINITY;
    for index in 0..profile.len() {
        let next = (index + 1) % profile.len();
        min_edge = min_edge.min(length2(sub2(profile[next], profile[index])));
    }
    Ok(min_edge * 0.5)
}

fn normalize_closed_profile(profile: &[[f32; 2]]) -> Result<Vec<[f32; 2]>, ModelingError> {
    if profile.len() < 3 {
        return Err(ModelingError::InvalidInput(
            "sweep profile requires at least three points".to_owned(),
        ));
    }
    let mut normalized = profile.to_vec();
    if points2_close(
        normalized[0],
        *normalized.last().expect("profile is not empty"),
    ) {
        normalized.pop();
    }
    if normalized.len() < 3 {
        return Err(ModelingError::InvalidInput(
            "sweep profile requires at least three unique points".to_owned(),
        ));
    }
    for point in &normalized {
        if !point[0].is_finite() || !point[1].is_finite() {
            return Err(ModelingError::InvalidInput(
                "sweep profile points must be finite".to_owned(),
            ));
        }
    }
    for index in 0..normalized.len() {
        if points2_close(
            normalized[index],
            normalized[(index + 1) % normalized.len()],
        ) {
            return Err(ModelingError::InvalidInput(
                "sweep profile cannot contain collapsed edges".to_owned(),
            ));
        }
    }
    if signed_area(&normalized).abs() <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "sweep profile area must be non-zero".to_owned(),
        ));
    }
    Ok(normalized)
}

fn positive_triplet(values: [f32; 3], label: &'static str) -> Result<[f32; 3], ModelingError> {
    for value in values {
        finite_positive(value, label)?;
    }
    Ok(values)
}

fn finite_positive(value: f32, label: &'static str) -> Result<f32, ModelingError> {
    if value.is_finite() && value > EPSILON {
        Ok(value)
    } else {
        Err(ModelingError::InvalidInput(format!(
            "{label} must be finite and positive"
        )))
    }
}

fn faces_are_smooth_neighbors(
    mesh: &PolygonMesh,
    left_face: usize,
    right_face: usize,
    vertex: u32,
) -> bool {
    let Some(edge) = shared_edge_at_vertex(&mesh.faces[left_face], &mesh.faces[right_face], vertex)
    else {
        return false;
    };
    let edge_metadata = mesh
        .edge_metadata
        .get(&edge)
        .cloned()
        .unwrap_or_else(EdgeMetadata::smooth);
    if edge_metadata.classification == EdgeClassification::Hard
        || matches!(
            edge_metadata.boundary_role,
            BoundaryRole::Hard
                | BoundaryRole::Feature
                | BoundaryRole::Attachment
                | BoundaryRole::SeamCandidate
                | BoundaryRole::OpenBoundary
        )
    {
        return false;
    }
    mesh.face_metadata[left_face].smoothing_group == mesh.face_metadata[right_face].smoothing_group
}

fn shared_edge_at_vertex(left: &PolygonFace, right: &PolygonFace, vertex: u32) -> Option<EdgeKey> {
    let left_edges = directed_face_edges(left)
        .into_iter()
        .filter(|edge| edge.from == vertex || edge.to == vertex)
        .map(DirectedEdge::key)
        .collect::<BTreeSet<_>>();
    directed_face_edges(right)
        .into_iter()
        .filter(|edge| edge.from == vertex || edge.to == vertex)
        .map(DirectedEdge::key)
        .find(|edge| left_edges.contains(edge))
}

fn directed_face_edges(face: &PolygonFace) -> Vec<DirectedEdge> {
    let mut edges = Vec::with_capacity(face.vertices.len());
    for index in 0..face.vertices.len() {
        let next = (index + 1) % face.vertices.len();
        edges.push(DirectedEdge {
            from: face.vertices[index],
            to: face.vertices[next],
        });
    }
    edges
}

fn polygon_area(mesh: &PolygonMesh, vertices: &[u32]) -> Result<f32, ModelingError> {
    if vertices.len() < 3 {
        return Err(ModelingError::InvalidInput(
            "polygon face must contain at least three vertices".to_owned(),
        ));
    }
    let origin = position(mesh, vertices[0])?;
    let mut area = 0.0;
    for index in 1..vertices.len() - 1 {
        let a = position(mesh, vertices[index])?;
        let b = position(mesh, vertices[index + 1])?;
        area += length3(cross3(sub3(a, origin), sub3(b, origin))) * 0.5;
    }
    Ok(area)
}

fn position(mesh: &PolygonMesh, vertex: u32) -> Result<[f32; 3], ModelingError> {
    mesh.positions.get(vertex as usize).copied().ok_or_else(|| {
        ModelingError::InvalidInput("polygon vertex index is out of range".to_owned())
    })
}

fn shaped_parameter(t: f32, exponent: f32) -> f32 {
    if t <= 0.0 || t >= 1.0 || (exponent - 1.0).abs() <= EPSILON {
        return t;
    }
    let left = t.powf(exponent);
    let right = (1.0 - t).powf(exponent);
    left / (left + right)
}

fn quadratic2(start: [f32; 2], control: [f32; 2], end: [f32; 2], t: f32) -> [f32; 2] {
    let one_minus = 1.0 - t;
    add2(
        add2(
            scale2(start, one_minus * one_minus),
            scale2(control, 2.0 * one_minus * t),
        ),
        scale2(end, t * t),
    )
}

fn points2_close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() <= EPSILON && (a[1] - b[1]).abs() <= EPSILON
}

fn normalize2(vector: [f32; 2]) -> Result<[f32; 2], ModelingError> {
    let length = length2(vector);
    if !length.is_finite() || length <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "cannot normalize collapsed 2D vector".to_owned(),
        ));
    }
    Ok(scale2(vector, 1.0 / length))
}

fn normalize3_or_default(vector: [f32; 3]) -> [f32; 3] {
    let length = length3(vector);
    if !length.is_finite() || length <= EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        scale3(vector, 1.0 / length)
    }
}

fn signed_area(profile: &[[f32; 2]]) -> f32 {
    let mut area = 0.0;
    for index in 0..profile.len() {
        let current = profile[index];
        let next = profile[(index + 1) % profile.len()];
        area += current[0] * next[1] - next[0] * current[1];
    }
    area * 0.5
}

fn add2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn scale2(vector: [f32; 2], scalar: f32) -> [f32; 2] {
    [vector[0] * scalar, vector[1] * scalar]
}

fn length2(vector: [f32; 2]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1]).sqrt()
}

fn cross2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length3(vector: [f32; 3]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

#[derive(Debug, Copy, Clone)]
struct DirectedEdge {
    from: u32,
    to: u32,
}

impl DirectedEdge {
    fn key(self) -> EdgeKey {
        EdgeKey::new(self.from, self.to)
    }
}
