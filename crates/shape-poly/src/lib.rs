#![forbid(unsafe_code)]

//! Explicit indexed polygon topology contracts for the part-aware modeling lane.
//!
//! Vertex and face IDs in this crate are deterministic for a given topology
//! signature. They are not guaranteed to survive topology-changing parameters
//! such as segment counts. Semantic provenance is carried through metadata that
//! points back to asset part, region, and operation IDs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use glam::{EulerRot, Mat3, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use shape_asset::{
    BoundaryLoopId, OperationId, PartDefinitionId, PartInstanceId, RegionId, SurfaceRole,
    Transform3,
};
use thiserror::Error;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const MIN_NORMAL_LENGTH: f32 = 1.0e-6;
const MIN_AREA_LENGTH: f32 = 1.0e-7;
const GEOMETRY_EPSILON: f32 = 1.0e-6;

/// Deterministic identifier for generated polygon elements.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ElementId(pub u64);

/// Axis-aligned bounds for polygon and triangle meshes.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshBounds {
    /// Minimum corner.
    pub min: [f32; 3],
    /// Maximum corner.
    pub max: [f32; 3],
}

impl MeshBounds {
    /// Return an empty bounds value.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min: [1.0, 1.0, 1.0],
            max: [0.0, 0.0, 0.0],
        }
    }

    /// Return true when this bounds contains no finite volume.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.min[0] > self.max[0] || self.min[1] > self.max[1] || self.min[2] > self.max[2]
    }

    /// Return the union of two bounds values.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    /// Return bounds that include one point.
    #[must_use]
    pub fn include_point(&self, point: [f32; 3]) -> Self {
        if self.is_empty() {
            return Self {
                min: point,
                max: point,
            };
        }
        Self {
            min: [
                self.min[0].min(point[0]),
                self.min[1].min(point[1]),
                self.min[2].min(point[2]),
            ],
            max: [
                self.max[0].max(point[0]),
                self.max[1].max(point[1]),
                self.max[2].max(point[2]),
            ],
        }
    }
}

/// Polygon face with stable semantic element ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonFace {
    /// Stable face ID for this topology signature.
    pub id: ElementId,
    /// Ordered vertex indices.
    pub vertices: Vec<u32>,
}

/// Explicit indexed polygon mesh with semantic metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Stable vertex IDs.
    pub vertex_ids: Vec<ElementId>,
    /// Polygon faces.
    pub faces: Vec<PolygonFace>,
    /// Per-face semantic metadata.
    pub face_metadata: Vec<FaceMetadata>,
    /// Per-edge semantic metadata keyed by canonical vertex pair.
    #[serde(with = "edge_metadata_serde")]
    pub edge_metadata: BTreeMap<EdgeKey, EdgeMetadata>,
    /// Deterministic topology signature.
    pub topology_signature: u64,
    /// Mesh bounds.
    pub bounds: MeshBounds,
}

impl PolygonMesh {
    /// Return an empty polygon mesh. Empty meshes are useful sentinels but fail validation.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            positions: Vec::new(),
            vertex_ids: Vec::new(),
            faces: Vec::new(),
            face_metadata: Vec::new(),
            edge_metadata: BTreeMap::new(),
            topology_signature: compute_topology_signature(&[], &[]),
            bounds: MeshBounds::empty(),
        }
    }
}

/// Semantic metadata attached to one polygon face.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaceMetadata {
    /// Source part definition.
    pub part_definition: Option<PartDefinitionId>,
    /// Source part instance.
    pub part_instance: Option<PartInstanceId>,
    /// Source semantic region.
    pub region: Option<RegionId>,
    /// Source modeling operation.
    pub operation: Option<OperationId>,
    /// Optional smoothing group.
    pub smoothing_group: Option<u32>,
    /// Generic surface role.
    pub surface_role: Option<SurfaceRole>,
}

/// Canonical edge key stored as an ordered vertex pair.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeKey {
    /// Smaller vertex index.
    pub a: u32,
    /// Larger vertex index.
    pub b: u32,
}

impl EdgeKey {
    /// Build a canonical edge key from two vertex indices.
    #[must_use]
    pub fn new(first: u32, second: u32) -> Self {
        if first <= second {
            Self {
                a: first,
                b: second,
            }
        } else {
            Self {
                a: second,
                b: first,
            }
        }
    }
}

/// Semantic metadata attached to an edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeMetadata {
    /// Boundary role.
    pub boundary_role: BoundaryRole,
    /// Hard or smooth classification.
    pub classification: EdgeClassification,
    /// Whether this edge is a future UV seam candidate.
    pub seam_candidate: bool,
    /// Source modeling operation.
    pub operation: Option<OperationId>,
    /// Optional transition between two semantic regions.
    pub region_transition: Option<(RegionId, RegionId)>,
    /// Optional generated boundary loop identity.
    #[serde(default)]
    pub boundary_loop: Option<BoundaryLoopId>,
}

impl EdgeMetadata {
    /// Metadata for an explicitly declared open boundary edge.
    #[must_use]
    pub fn open_boundary() -> Self {
        Self {
            boundary_role: BoundaryRole::OpenBoundary,
            classification: EdgeClassification::Hard,
            seam_candidate: true,
            operation: None,
            region_transition: None,
            boundary_loop: None,
        }
    }

    /// Metadata for a smooth interior edge.
    #[must_use]
    pub fn smooth() -> Self {
        Self {
            boundary_role: BoundaryRole::Smooth,
            classification: EdgeClassification::Smooth,
            seam_candidate: false,
            operation: None,
            region_transition: None,
            boundary_loop: None,
        }
    }

    /// Metadata for a hard interior edge.
    #[must_use]
    pub fn hard() -> Self {
        Self {
            boundary_role: BoundaryRole::Hard,
            classification: EdgeClassification::Hard,
            seam_candidate: false,
            operation: None,
            region_transition: None,
            boundary_loop: None,
        }
    }
}

mod edge_metadata_serde {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::{EdgeKey, EdgeMetadata};

    #[derive(Serialize, Deserialize)]
    struct EdgeMetadataEntry {
        edge: EdgeKey,
        metadata: EdgeMetadata,
    }

    pub(super) fn serialize<S>(
        value: &BTreeMap<EdgeKey, EdgeMetadata>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value
            .iter()
            .map(|(edge, metadata)| EdgeMetadataEntry {
                edge: *edge,
                metadata: metadata.clone(),
            })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<EdgeKey, EdgeMetadata>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<EdgeMetadataEntry>::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.edge, entry.metadata))
            .collect())
    }
}

/// Boundary role for polygon edges.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BoundaryRole {
    /// Smooth interior edge.
    Smooth,
    /// Hard interior edge.
    Hard,
    /// Feature edge.
    Feature,
    /// Attachment boundary.
    Attachment,
    /// Future UV seam candidate.
    SeamCandidate,
    /// Open polygon boundary.
    OpenBoundary,
}

/// Edge normal classification.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EdgeClassification {
    /// Smooth shading across the edge.
    Smooth,
    /// Hard split across the edge.
    Hard,
}

/// Indexed triangle mesh derived from explicit polygons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriangleMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Mesh bounds.
    pub bounds: MeshBounds,
}

/// Triangulated form of a polygon mesh with provenance maps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriangulatedPolygonMesh {
    /// Triangle mesh payload.
    pub mesh: TriangleMesh,
    /// Source polygon face for each triangle.
    pub triangle_to_polygon_face: Vec<ElementId>,
    /// Source semantic region for each triangle.
    pub triangle_to_region: Vec<Option<RegionId>>,
    /// Source part instance for each triangle.
    pub triangle_to_part: Vec<Option<PartInstanceId>>,
    /// Source operation for each triangle.
    pub triangle_to_operation: Vec<Option<OperationId>>,
    /// Stable vertex IDs copied from the polygon mesh. Split-normal duplicates repeat IDs.
    pub vertex_ids: Vec<ElementId>,
}

/// One deterministic boundary loop discovered in adjacency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryLoop {
    /// Ordered vertices around the boundary. The first vertex is repeated at the end.
    pub vertices: Vec<u32>,
    /// Ordered canonical edges around the boundary.
    pub edges: Vec<EdgeKey>,
}

/// Polygon mesh adjacency by vertex, face, edge, boundary, and component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAdjacency {
    /// Face indices incident to each vertex.
    pub vertex_faces: Vec<Vec<usize>>,
    /// Neighboring face indices for each face.
    pub face_neighbors: Vec<BTreeSet<usize>>,
    /// Face indices incident to each edge.
    pub edge_faces: BTreeMap<EdgeKey, Vec<usize>>,
    /// Deterministic boundary loops.
    pub boundary_loops: Vec<BoundaryLoop>,
    /// Face-index connected components.
    pub connected_components: Vec<Vec<usize>>,
}

/// A normal attached to one source vertex split group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitNormal {
    /// Source vertex index.
    pub vertex: u32,
    /// Source faces that contributed to the split normal.
    pub faces: Vec<usize>,
    /// Split normal value.
    pub normal: [f32; 3],
}

/// Validation issue for polygon meshes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolyValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Polygon validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PolyValidationReport {
    /// Discovered issues.
    pub issues: Vec<PolyValidationIssue>,
}

impl PolyValidationReport {
    /// Return true when no issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Error type for polygon topology helpers.
#[derive(Debug, Error)]
pub enum PolyError {
    /// Mesh validation failed.
    #[error("polygon mesh validation failed")]
    ValidationFailed(PolyValidationReport),
    /// Mesh index conversion overflowed u32.
    #[error("mesh index overflow")]
    IndexOverflow,
    /// Input is not a supported polygon mesh.
    #[error("invalid polygon mesh: {0}")]
    InvalidMesh(String),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DirectedEdge {
    from: u32,
    to: u32,
}

impl DirectedEdge {
    fn key(self) -> EdgeKey {
        EdgeKey::new(self.from, self.to)
    }
}

#[derive(Debug, Copy, Clone)]
struct EdgeUse {
    from: u32,
    to: u32,
}

#[derive(Debug, Clone)]
struct FaceGeometry {
    normal_sum: Vec3,
    normal: Vec3,
    projected: Vec<Vec2>,
}

#[derive(Debug, Copy, Clone)]
enum ProjectionAxis {
    X,
    Y,
    Z,
}

/// Validate polygon mesh invariants and collect every issue.
#[must_use]
pub fn validate_polygon_mesh(mesh: &PolygonMesh) -> PolyValidationReport {
    let mut report = PolyValidationReport::default();

    if mesh.positions.is_empty() {
        push_issue(
            &mut report,
            None,
            "empty_positions",
            "Polygon meshes must contain at least one vertex position.",
        );
    }
    if mesh.positions.len() != mesh.vertex_ids.len() {
        push_issue(
            &mut report,
            None,
            "invalid_metadata_lengths",
            "Position and vertex ID counts must match.",
        );
        push_issue(
            &mut report,
            None,
            "vertex_id_count_mismatch",
            "Position and vertex ID counts must match.",
        );
    }
    if mesh.faces.len() != mesh.face_metadata.len() {
        push_issue(
            &mut report,
            None,
            "invalid_metadata_lengths",
            "Face and face metadata counts must match.",
        );
        push_issue(
            &mut report,
            None,
            "face_metadata_count_mismatch",
            "Face and face metadata counts must match.",
        );
    }

    validate_unique_vertex_ids(mesh, &mut report);
    validate_unique_face_ids(mesh, &mut report);
    validate_positions(mesh, &mut report);
    validate_face_metadata(mesh, &mut report);

    let mut edge_uses: BTreeMap<EdgeKey, Vec<EdgeUse>> = BTreeMap::new();
    let mut directed_uses: BTreeMap<DirectedEdge, Vec<usize>> = BTreeMap::new();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        validate_face_vertices(mesh, face_index, face, &mut report);
        if face_indices_are_in_range(mesh, face) && face.vertices.len() >= 3 {
            validate_face_geometry(mesh, face_index, face, &mut report);
            collect_edge_uses(face_index, face, &mut edge_uses, &mut directed_uses);
        }
    }

    validate_edges(mesh, &edge_uses, &directed_uses, &mut report);
    validate_edge_metadata(mesh, &edge_uses, &mut report);
    validate_bounds(mesh, &mut report);

    report
}

/// Build full deterministic adjacency for a valid polygon mesh.
pub fn build_adjacency(mesh: &PolygonMesh) -> Result<MeshAdjacency, PolyError> {
    ensure_valid(mesh)?;

    let mut vertex_faces = vec![Vec::new(); mesh.positions.len()];
    let mut edge_faces: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    let mut directed_boundary_edges = Vec::new();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        for vertex in &face.vertices {
            vertex_faces[*vertex as usize].push(face_index);
        }
        for edge in directed_face_edges(face) {
            let key = edge.key();
            edge_faces.entry(key).or_default().push(face_index);
        }
    }
    for faces in edge_faces.values_mut() {
        faces.sort_unstable();
    }

    for face in &mesh.faces {
        for edge in directed_face_edges(face) {
            if edge_faces
                .get(&edge.key())
                .is_some_and(|faces| faces.len() == 1)
            {
                directed_boundary_edges.push(edge);
            }
        }
    }
    directed_boundary_edges.sort_unstable();

    let mut face_neighbors = vec![BTreeSet::new(); mesh.faces.len()];
    for faces in edge_faces.values() {
        for face in faces {
            for neighbor in faces {
                if face != neighbor {
                    face_neighbors[*face].insert(*neighbor);
                }
            }
        }
    }

    let boundary_loops = build_boundary_loops(&directed_boundary_edges);
    let connected_components = build_connected_components(&face_neighbors);

    Ok(MeshAdjacency {
        vertex_faces,
        face_neighbors,
        edge_faces,
        boundary_loops,
        connected_components,
    })
}

/// Triangulate polygon faces with deterministic convex and ear-clipping rules.
pub fn triangulate_polygon_mesh(mesh: &PolygonMesh) -> Result<TriangulatedPolygonMesh, PolyError> {
    ensure_valid(mesh)?;

    let mut polygon_triangles = Vec::new();
    let mut triangle_to_polygon_face = Vec::new();
    let mut triangle_to_region = Vec::new();
    let mut triangle_to_part = Vec::new();
    let mut triangle_to_operation = Vec::new();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        let triangles = triangulate_face(mesh, face_index)?;
        let metadata = &mesh.face_metadata[face_index];
        for triangle in triangles {
            polygon_triangles.push((face_index, triangle));
            triangle_to_polygon_face.push(face.id);
            triangle_to_region.push(metadata.region);
            triangle_to_part.push(metadata.part_instance);
            triangle_to_operation.push(metadata.operation);
        }
    }

    let split_data = build_split_normal_lookup(mesh)?;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_ids = Vec::new();
    let mut output_vertices: BTreeMap<(u32, usize), u32> = BTreeMap::new();

    for (face_index, triangle) in &polygon_triangles {
        for vertex in triangle {
            let split = split_data
                .vertex_face_split
                .get(&(*vertex, *face_index))
                .copied()
                .ok_or_else(|| {
                    PolyError::InvalidMesh("missing split normal for face vertex".to_owned())
                })?;
            let output_key = (*vertex, split);
            let output_index = if let Some(index) = output_vertices.get(&output_key) {
                *index
            } else {
                let next = u32::try_from(positions.len()).map_err(|_| PolyError::IndexOverflow)?;
                positions.push(mesh.positions[*vertex as usize]);
                normals.push(split_data.split_normals[split].to_array());
                vertex_ids.push(mesh.vertex_ids[*vertex as usize]);
                output_vertices.insert(output_key, next);
                next
            };
            indices.push(output_index);
        }
    }

    Ok(TriangulatedPolygonMesh {
        mesh: TriangleMesh {
            positions,
            normals,
            indices,
            bounds: mesh.bounds,
        },
        triangle_to_polygon_face,
        triangle_to_region,
        triangle_to_part,
        triangle_to_operation,
        vertex_ids,
    })
}

/// Compute one normal per polygon face.
pub fn compute_face_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, PolyError> {
    ensure_valid(mesh)?;
    let mut normals = Vec::with_capacity(mesh.faces.len());
    for face in &mesh.faces {
        normals.push(face_normal(mesh, face).to_array());
    }
    Ok(normals)
}

/// Compute smooth vertex normals by averaging all incident polygon face normals.
pub fn compute_smooth_vertex_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, PolyError> {
    ensure_valid(mesh)?;
    let face_normals = compute_face_normal_vectors(mesh)?;
    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    for (face, normal) in mesh.faces.iter().zip(face_normals) {
        for vertex in &face.vertices {
            normals[*vertex as usize] += normal;
        }
    }
    Ok(normals
        .into_iter()
        .map(|normal| normalize_or_default(normal).to_array())
        .collect())
}

/// Compute one representative split normal per source vertex for compatibility.
///
/// Use [`compute_split_normal_groups`] or [`triangulate_polygon_mesh`] when each
/// hard-edge/smoothing split must be represented independently.
pub fn compute_split_vertex_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, PolyError> {
    ensure_valid(mesh)?;
    let split_data = build_split_normal_lookup(mesh)?;
    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    for split in &split_data.splits {
        normals[split.vertex as usize] += split_data.split_normals[split.normal_index];
    }
    Ok(normals
        .into_iter()
        .map(|normal| normalize_or_default(normal).to_array())
        .collect())
}

/// Compute every hard-edge/smoothing-group split normal.
pub fn compute_split_normal_groups(mesh: &PolygonMesh) -> Result<Vec<SplitNormal>, PolyError> {
    ensure_valid(mesh)?;
    let split_data = build_split_normal_lookup(mesh)?;
    let mut groups = split_data
        .splits
        .into_iter()
        .map(|split| SplitNormal {
            vertex: split.vertex,
            faces: split.faces,
            normal: split_data.split_normals[split.normal_index].to_array(),
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.vertex
            .cmp(&right.vertex)
            .then_with(|| left.faces.cmp(&right.faces))
    });
    Ok(groups)
}

/// Return a copy of a polygon mesh transformed by an asset transform.
pub fn transform_polygon_mesh(
    mesh: &PolygonMesh,
    transform: &Transform3,
) -> Result<PolygonMesh, PolyError> {
    ensure_valid(mesh)?;
    let positions = mesh
        .positions
        .iter()
        .map(|position| transform.transform_point(*position))
        .collect::<Vec<_>>();
    let mut faces = mesh.faces.clone();
    if transform_orientation_flips(transform) {
        for face in &mut faces {
            face.vertices.reverse();
        }
    }
    let mut transformed = mesh.clone();
    transformed.positions = positions;
    transformed.faces = faces;
    transformed.bounds = bounds_from_positions(&transformed.positions)?;
    transformed.topology_signature =
        compute_topology_signature(&transformed.positions, &transformed.faces);
    ensure_valid(&transformed)?;
    Ok(transformed)
}

/// Return a triangle mesh transformed with inverse-transpose normal handling.
pub fn transform_triangle_mesh(
    mesh: &TriangleMesh,
    transform: &Transform3,
) -> Result<TriangleMesh, PolyError> {
    if mesh.positions.len() != mesh.normals.len() {
        return Err(PolyError::InvalidMesh(
            "triangle positions and normals must have matching lengths".to_owned(),
        ));
    }
    let normal_matrix = normal_transform_matrix(transform)?;
    let positions = mesh
        .positions
        .iter()
        .map(|position| transform.transform_point(*position))
        .collect::<Vec<_>>();
    let normals = mesh
        .normals
        .iter()
        .map(|normal| normalize_or_default(normal_matrix * Vec3::from_array(*normal)).to_array())
        .collect::<Vec<_>>();
    Ok(TriangleMesh {
        bounds: bounds_from_positions(&positions)?,
        positions,
        normals,
        indices: mesh.indices.clone(),
    })
}

/// Combine multiple polygon meshes into one indexed polygon mesh.
pub fn combine_polygon_meshes(meshes: &[PolygonMesh]) -> Result<PolygonMesh, PolyError> {
    let mut combined = PolygonMesh::empty();
    let mut vertex_offset: u32 = 0;
    let mut vertex_ids = BTreeSet::new();
    let mut face_ids = BTreeSet::new();

    for mesh in meshes {
        ensure_valid(mesh)?;
        for vertex_id in &mesh.vertex_ids {
            if !vertex_ids.insert(*vertex_id) {
                return Err(PolyError::InvalidMesh(format!(
                    "duplicate vertex ElementId {}",
                    vertex_id.0
                )));
            }
        }
        for face in &mesh.faces {
            if !face_ids.insert(face.id) {
                return Err(PolyError::InvalidMesh(format!(
                    "duplicate face ElementId {}",
                    face.id.0
                )));
            }
        }

        combined.positions.extend(mesh.positions.iter().copied());
        combined.vertex_ids.extend(mesh.vertex_ids.iter().copied());
        for face in &mesh.faces {
            let mut vertices = Vec::with_capacity(face.vertices.len());
            for vertex in &face.vertices {
                vertices.push(
                    vertex
                        .checked_add(vertex_offset)
                        .ok_or(PolyError::IndexOverflow)?,
                );
            }
            combined.faces.push(PolygonFace {
                id: face.id,
                vertices,
            });
        }
        combined
            .face_metadata
            .extend(mesh.face_metadata.iter().cloned());
        for (edge, metadata) in &mesh.edge_metadata {
            let a = edge
                .a
                .checked_add(vertex_offset)
                .ok_or(PolyError::IndexOverflow)?;
            let b = edge
                .b
                .checked_add(vertex_offset)
                .ok_or(PolyError::IndexOverflow)?;
            combined
                .edge_metadata
                .insert(EdgeKey::new(a, b), metadata.clone());
        }
        let add = u32::try_from(mesh.positions.len()).map_err(|_| PolyError::IndexOverflow)?;
        vertex_offset = vertex_offset
            .checked_add(add)
            .ok_or(PolyError::IndexOverflow)?;
        combined.bounds = combined.bounds.union(&mesh.bounds);
    }

    combined.topology_signature = compute_topology_signature(&combined.positions, &combined.faces);
    if combined.positions.is_empty() {
        return Ok(combined);
    }
    ensure_valid(&combined)?;
    Ok(combined)
}

/// Construct a polygon mesh from positions, faces, and per-face metadata.
///
/// Open boundary edges are declared as `BoundaryRole::OpenBoundary` so the
/// constructor is convenient for plates and single polygons. Direct struct
/// construction can still be used to validate undeclared boundaries.
pub fn polygon_mesh_from_faces(
    positions: Vec<[f32; 3]>,
    faces: Vec<Vec<u32>>,
    face_metadata: Vec<FaceMetadata>,
) -> Result<PolygonMesh, PolyError> {
    let vertex_ids = (0..positions.len())
        .map(|index| ElementId(index as u64))
        .collect::<Vec<_>>();
    let faces = faces
        .into_iter()
        .enumerate()
        .map(|(index, vertices)| PolygonFace {
            id: ElementId(index as u64),
            vertices,
        })
        .collect::<Vec<_>>();
    let bounds = bounds_from_positions(&positions)?;
    let topology_signature = compute_topology_signature(&positions, &faces);
    let mut mesh = PolygonMesh {
        positions,
        vertex_ids,
        faces,
        face_metadata,
        edge_metadata: BTreeMap::new(),
        topology_signature,
        bounds,
    };
    declare_open_boundaries(&mut mesh)?;
    ensure_valid(&mesh)?;
    Ok(mesh)
}

/// Add `OpenBoundary` metadata for every currently undeclared boundary edge.
pub fn declare_open_boundaries(mesh: &mut PolygonMesh) -> Result<(), PolyError> {
    let edge_uses = collect_canonical_edges(mesh)?;
    for (edge, uses) in edge_uses {
        if uses.len() == 1 {
            mesh.edge_metadata
                .entry(edge)
                .or_insert_with(EdgeMetadata::open_boundary);
        }
    }
    Ok(())
}

/// Compute deterministic topology signature from face order and vertex indices.
#[must_use]
pub fn compute_topology_signature(_positions: &[[f32; 3]], faces: &[PolygonFace]) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = fnv_u64(hash, faces.len() as u64);
    for face in faces {
        hash = fnv_u64(hash, face.vertices.len() as u64);
        for vertex in &face.vertices {
            hash = fnv_u64(hash, u64::from(*vertex));
        }
    }
    hash
}

/// Compute finite bounds for a set of positions.
pub fn bounds_from_positions(positions: &[[f32; 3]]) -> Result<MeshBounds, PolyError> {
    let mut bounds = MeshBounds::empty();
    for position in positions {
        if !array_is_finite(*position) {
            return Err(PolyError::InvalidMesh(
                "positions must be finite".to_owned(),
            ));
        }
        bounds = bounds.include_point(*position);
    }
    Ok(bounds)
}

fn validate_unique_vertex_ids(mesh: &PolygonMesh, report: &mut PolyValidationReport) {
    let mut seen = BTreeMap::new();
    for (index, id) in mesh.vertex_ids.iter().enumerate() {
        if let Some(first) = seen.insert(*id, index) {
            push_issue(
                report,
                Some(format!("vertex_id.{index}")),
                "duplicate_vertex_id",
                format!(
                    "Vertex ElementId {} duplicates vertex ID at index {first}.",
                    id.0
                ),
            );
        }
    }
}

fn validate_unique_face_ids(mesh: &PolygonMesh, report: &mut PolyValidationReport) {
    let mut seen = BTreeMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        if let Some(first) = seen.insert(face.id, index) {
            push_issue(
                report,
                Some(format!("face.{index}")),
                "duplicate_face_id",
                format!(
                    "Face ElementId {} duplicates face ID at index {first}.",
                    face.id.0
                ),
            );
        }
    }
}

fn validate_positions(mesh: &PolygonMesh, report: &mut PolyValidationReport) {
    for (index, position) in mesh.positions.iter().enumerate() {
        if !array_is_finite(*position) {
            push_issue(
                report,
                Some(format!("position.{index}")),
                "non_finite_position",
                "All positions must be finite.",
            );
        }
    }
}

fn validate_face_metadata(mesh: &PolygonMesh, report: &mut PolyValidationReport) {
    for (index, metadata) in mesh.face_metadata.iter().enumerate() {
        if metadata.part_definition.is_some_and(|id| id.0 == 0)
            || metadata.part_instance.is_some_and(|id| id.0 == 0)
            || metadata.operation.is_some_and(|id| id.0 == 0)
        {
            push_issue(
                report,
                Some(format!("face_metadata.{index}")),
                "invalid_provenance_metadata",
                "Face provenance IDs must be non-zero when present.",
            );
        }
        if metadata.region.is_some_and(|id| id.0 == 0) {
            push_issue(
                report,
                Some(format!("face_metadata.{index}.region")),
                "invalid_region_metadata",
                "Face region IDs must be non-zero when present.",
            );
        }
        if matches!(&metadata.surface_role, Some(SurfaceRole::Custom(value)) if value.is_empty()) {
            push_issue(
                report,
                Some(format!("face_metadata.{index}.surface_role")),
                "invalid_region_metadata",
                "Custom surface roles must not be empty.",
            );
        }
    }
}

fn validate_face_vertices(
    mesh: &PolygonMesh,
    face_index: usize,
    face: &PolygonFace,
    report: &mut PolyValidationReport,
) {
    if face.vertices.len() < 3 {
        push_issue(
            report,
            Some(format!("face.{face_index}")),
            "face_too_small",
            "Polygon faces must have at least three vertices.",
        );
    }

    let mut unique_vertices = BTreeSet::new();
    for (local_index, vertex) in face.vertices.iter().enumerate() {
        if *vertex as usize >= mesh.positions.len() {
            push_issue(
                report,
                Some(format!("face.{face_index}.vertex.{local_index}")),
                "invalid_face_index",
                "Face references a missing vertex.",
            );
            push_issue(
                report,
                Some(format!("face.{face_index}.vertex.{local_index}")),
                "vertex_index_out_of_range",
                "Face references a missing vertex.",
            );
        }
        if !unique_vertices.insert(*vertex) {
            push_issue(
                report,
                Some(format!("face.{face_index}.vertex.{local_index}")),
                "repeated_face_vertex",
                "Polygon face repeats a vertex index.",
            );
            push_issue(
                report,
                Some(format!("face.{face_index}.vertex.{local_index}")),
                "duplicate_face_vertex",
                "Polygon face repeats a vertex index.",
            );
        }
        if !face.vertices.is_empty() {
            let next = face.vertices[(local_index + 1) % face.vertices.len()];
            if *vertex == next {
                push_issue(
                    report,
                    Some(format!("face.{face_index}.edge.{local_index}")),
                    "repeated_consecutive_vertex",
                    "Polygon face has repeated consecutive vertices.",
                );
            }
        }
    }
}

fn validate_face_geometry(
    mesh: &PolygonMesh,
    face_index: usize,
    face: &PolygonFace,
    report: &mut PolyValidationReport,
) {
    let Some(geometry) = face_geometry(mesh, face) else {
        if project_face_for_validation(mesh, face)
            .is_some_and(|projected| polygon_self_intersects(&projected))
        {
            push_issue(
                report,
                Some(format!("face.{face_index}")),
                "self_intersecting_face",
                "Polygon face edges self-intersect.",
            );
        }
        push_issue(
            report,
            Some(format!("face.{face_index}")),
            "zero_area_face",
            "Polygon face has zero area.",
        );
        return;
    };
    if geometry.normal_sum.length() <= MIN_AREA_LENGTH {
        push_issue(
            report,
            Some(format!("face.{face_index}")),
            "zero_area_face",
            "Polygon face has zero area.",
        );
    }
    if polygon_self_intersects(&geometry.projected) {
        push_issue(
            report,
            Some(format!("face.{face_index}")),
            "self_intersecting_face",
            "Polygon face edges self-intersect.",
        );
    }
}

fn project_face_for_validation(mesh: &PolygonMesh, face: &PolygonFace) -> Option<Vec<Vec2>> {
    if face.vertices.len() < 3 || !face_indices_are_in_range(mesh, face) {
        return None;
    }
    let points = face
        .vertices
        .iter()
        .map(|vertex| Vec3::from_array(mesh.positions[*vertex as usize]))
        .collect::<Vec<_>>();
    let mut min = points[0];
    let mut max = points[0];
    for point in &points {
        min = min.min(*point);
        max = max.max(*point);
    }
    let extent = max - min;
    let axis = if extent.x <= extent.y && extent.x <= extent.z {
        ProjectionAxis::X
    } else if extent.y <= extent.z {
        ProjectionAxis::Y
    } else {
        ProjectionAxis::Z
    };
    Some(
        points
            .iter()
            .map(|point| project_point(*point, axis))
            .collect(),
    )
}

fn validate_edges(
    mesh: &PolygonMesh,
    edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>,
    directed_uses: &BTreeMap<DirectedEdge, Vec<usize>>,
    report: &mut PolyValidationReport,
) {
    for (edge, faces) in directed_uses {
        if faces.len() > 1 {
            push_issue(
                report,
                Some(format!("edge.{}.{}", edge.from, edge.to)),
                "duplicate_directed_edge",
                "Directed edge appears more than once.",
            );
        }
    }

    for (edge, uses) in edge_uses {
        if uses.len() > 2 {
            push_issue(
                report,
                Some(format!("edge.{}.{}", edge.a, edge.b)),
                "nonmanifold_edge",
                "Canonical edge is incident to more than two faces.",
            );
        } else if uses.len() == 2 {
            if uses[0].from == uses[1].from && uses[0].to == uses[1].to {
                push_issue(
                    report,
                    Some(format!("edge.{}.{}", edge.a, edge.b)),
                    "inconsistent_winding",
                    "Manifold neighboring faces must traverse shared edges in opposite directions.",
                );
            }
            if mesh
                .edge_metadata
                .get(edge)
                .is_some_and(|metadata| metadata.boundary_role == BoundaryRole::OpenBoundary)
            {
                push_issue(
                    report,
                    Some(format!("edge.{}.{}", edge.a, edge.b)),
                    "invalid_region_metadata",
                    "Interior edges must not be tagged as open boundaries.",
                );
            }
        } else if uses.len() == 1
            && mesh
                .edge_metadata
                .get(edge)
                .is_none_or(|metadata| metadata.boundary_role != BoundaryRole::OpenBoundary)
        {
            push_issue(
                report,
                Some(format!("edge.{}.{}", edge.a, edge.b)),
                "undeclared_open_boundary",
                "Open boundary edges must be declared with OpenBoundary metadata.",
            );
        }
    }
}

fn validate_edge_metadata(
    mesh: &PolygonMesh,
    edge_uses: &BTreeMap<EdgeKey, Vec<EdgeUse>>,
    report: &mut PolyValidationReport,
) {
    for (edge, metadata) in &mesh.edge_metadata {
        if edge.a == edge.b
            || edge.a as usize >= mesh.positions.len()
            || edge.b as usize >= mesh.positions.len()
        {
            push_issue(
                report,
                Some(format!("edge.{}.{}", edge.a, edge.b)),
                "invalid_edge_key",
                "Edge metadata key must reference two distinct vertices.",
            );
        }
        if !edge_uses.contains_key(edge) {
            push_issue(
                report,
                Some(format!("edge.{}.{}", edge.a, edge.b)),
                "invalid_region_metadata",
                "Edge metadata must reference an edge used by a face.",
            );
        }
        if metadata.operation.is_some_and(|id| id.0 == 0) {
            push_issue(
                report,
                Some(format!("edge.{}.{}.operation", edge.a, edge.b)),
                "invalid_provenance_metadata",
                "Edge provenance IDs must be non-zero when present.",
            );
        }
        if let Some((left, right)) = metadata.region_transition
            && (left.0 == 0 || right.0 == 0 || left == right)
        {
            push_issue(
                report,
                Some(format!("edge.{}.{}.region_transition", edge.a, edge.b)),
                "invalid_region_metadata",
                "Edge region transitions must contain two distinct non-zero region IDs.",
            );
        }
    }
}

fn validate_bounds(mesh: &PolygonMesh, report: &mut PolyValidationReport) {
    if !mesh.bounds.is_empty()
        && (!array_is_finite(mesh.bounds.min) || !array_is_finite(mesh.bounds.max))
    {
        push_issue(
            report,
            None,
            "non_finite_bounds",
            "Mesh bounds must be finite or empty.",
        );
    }
}

fn ensure_valid(mesh: &PolygonMesh) -> Result<(), PolyError> {
    let report = validate_polygon_mesh(mesh);
    if report.is_valid() {
        Ok(())
    } else {
        Err(PolyError::ValidationFailed(report))
    }
}

fn face_indices_are_in_range(mesh: &PolygonMesh, face: &PolygonFace) -> bool {
    face.vertices
        .iter()
        .all(|vertex| (*vertex as usize) < mesh.positions.len())
}

fn collect_edge_uses(
    face_index: usize,
    face: &PolygonFace,
    edge_uses: &mut BTreeMap<EdgeKey, Vec<EdgeUse>>,
    directed_uses: &mut BTreeMap<DirectedEdge, Vec<usize>>,
) {
    for edge in directed_face_edges(face) {
        if edge.from == edge.to {
            continue;
        }
        edge_uses.entry(edge.key()).or_default().push(EdgeUse {
            from: edge.from,
            to: edge.to,
        });
        directed_uses.entry(edge).or_default().push(face_index);
    }
}

fn collect_canonical_edges(
    mesh: &PolygonMesh,
) -> Result<BTreeMap<EdgeKey, Vec<EdgeUse>>, PolyError> {
    let mut edges = BTreeMap::new();
    for face in &mesh.faces {
        if !face_indices_are_in_range(mesh, face) || face.vertices.len() < 3 {
            return Err(PolyError::InvalidMesh(
                "cannot collect edges for invalid faces".to_owned(),
            ));
        }
        for edge in directed_face_edges(face) {
            if edge.from != edge.to {
                edges
                    .entry(edge.key())
                    .or_insert_with(Vec::new)
                    .push(EdgeUse {
                        from: edge.from,
                        to: edge.to,
                    });
            }
        }
    }
    Ok(edges)
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

fn face_geometry(mesh: &PolygonMesh, face: &PolygonFace) -> Option<FaceGeometry> {
    if face.vertices.len() < 3 || !face_indices_are_in_range(mesh, face) {
        return None;
    }
    let points = face
        .vertices
        .iter()
        .map(|vertex| Vec3::from_array(mesh.positions[*vertex as usize]))
        .collect::<Vec<_>>();
    let normal_sum = newell_normal(&points);
    if normal_sum.length() <= MIN_AREA_LENGTH || !normal_sum.is_finite() {
        return None;
    }
    let axis = projection_axis(normal_sum);
    let projected = points
        .iter()
        .map(|point| project_point(*point, axis))
        .collect();
    Some(FaceGeometry {
        normal_sum,
        normal: normalize_or_default(normal_sum),
        projected,
    })
}

fn newell_normal(points: &[Vec3]) -> Vec3 {
    let mut normal = Vec3::ZERO;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal
}

fn projection_axis(normal: Vec3) -> ProjectionAxis {
    let abs = normal.abs();
    if abs.x >= abs.y && abs.x >= abs.z {
        ProjectionAxis::X
    } else if abs.y >= abs.z {
        ProjectionAxis::Y
    } else {
        ProjectionAxis::Z
    }
}

fn project_point(point: Vec3, axis: ProjectionAxis) -> Vec2 {
    match axis {
        ProjectionAxis::X => Vec2::new(point.y, point.z),
        ProjectionAxis::Y => Vec2::new(point.x, point.z),
        ProjectionAxis::Z => Vec2::new(point.x, point.y),
    }
}

fn polygon_self_intersects(points: &[Vec2]) -> bool {
    if points.len() < 4 {
        return false;
    }
    for first in 0..points.len() {
        let first_next = (first + 1) % points.len();
        for second in first + 1..points.len() {
            let second_next = (second + 1) % points.len();
            if first == second
                || first_next == second
                || second_next == first
                || (first == 0 && second_next == 0)
            {
                continue;
            }
            if segments_intersect(
                points[first],
                points[first_next],
                points[second],
                points[second_next],
            ) {
                return true;
            }
        }
    }
    false
}

fn segments_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let ab_c = orient2(a, b, c);
    let ab_d = orient2(a, b, d);
    let cd_a = orient2(c, d, a);
    let cd_b = orient2(c, d, b);

    if ab_c.abs() <= GEOMETRY_EPSILON
        && point_on_segment(c, a, b)
        && !points_approx_equal(c, a)
        && !points_approx_equal(c, b)
    {
        return true;
    }
    if ab_d.abs() <= GEOMETRY_EPSILON
        && point_on_segment(d, a, b)
        && !points_approx_equal(d, a)
        && !points_approx_equal(d, b)
    {
        return true;
    }
    if cd_a.abs() <= GEOMETRY_EPSILON
        && point_on_segment(a, c, d)
        && !points_approx_equal(a, c)
        && !points_approx_equal(a, d)
    {
        return true;
    }
    if cd_b.abs() <= GEOMETRY_EPSILON
        && point_on_segment(b, c, d)
        && !points_approx_equal(b, c)
        && !points_approx_equal(b, d)
    {
        return true;
    }

    (ab_c > GEOMETRY_EPSILON && ab_d < -GEOMETRY_EPSILON
        || ab_c < -GEOMETRY_EPSILON && ab_d > GEOMETRY_EPSILON)
        && (cd_a > GEOMETRY_EPSILON && cd_b < -GEOMETRY_EPSILON
            || cd_a < -GEOMETRY_EPSILON && cd_b > GEOMETRY_EPSILON)
}

fn point_on_segment(point: Vec2, a: Vec2, b: Vec2) -> bool {
    point.x >= a.x.min(b.x) - GEOMETRY_EPSILON
        && point.x <= a.x.max(b.x) + GEOMETRY_EPSILON
        && point.y >= a.y.min(b.y) - GEOMETRY_EPSILON
        && point.y <= a.y.max(b.y) + GEOMETRY_EPSILON
}

fn points_approx_equal(left: Vec2, right: Vec2) -> bool {
    (left - right).length_squared() <= GEOMETRY_EPSILON * GEOMETRY_EPSILON
}

fn triangulate_face(mesh: &PolygonMesh, face_index: usize) -> Result<Vec<[u32; 3]>, PolyError> {
    let face = &mesh.faces[face_index];
    if face.vertices.len() == 3 {
        return Ok(vec![[face.vertices[0], face.vertices[1], face.vertices[2]]]);
    }
    let geometry = face_geometry(mesh, face)
        .ok_or_else(|| PolyError::InvalidMesh(format!("face {face_index} has zero area")))?;
    if polygon_self_intersects(&geometry.projected) {
        return Err(PolyError::InvalidMesh(format!(
            "face {face_index} self-intersects"
        )));
    }
    if face.vertices.len() == 4 && polygon_is_convex(&geometry.projected) {
        return Ok(triangulate_convex_quad(face));
    }
    if polygon_is_convex(&geometry.projected) {
        return Ok(triangulate_convex_ngon(face));
    }
    ear_clip_face(face, &geometry.projected, face_index)
}

fn triangulate_convex_quad(face: &PolygonFace) -> Vec<[u32; 3]> {
    let first = EdgeKey::new(face.vertices[0], face.vertices[2]);
    let second = EdgeKey::new(face.vertices[1], face.vertices[3]);
    if first <= second {
        vec![
            [face.vertices[0], face.vertices[1], face.vertices[2]],
            [face.vertices[0], face.vertices[2], face.vertices[3]],
        ]
    } else {
        vec![
            [face.vertices[1], face.vertices[2], face.vertices[3]],
            [face.vertices[1], face.vertices[3], face.vertices[0]],
        ]
    }
}

fn triangulate_convex_ngon(face: &PolygonFace) -> Vec<[u32; 3]> {
    let mut triangles = Vec::with_capacity(face.vertices.len() - 2);
    for index in 1..face.vertices.len() - 1 {
        triangles.push([
            face.vertices[0],
            face.vertices[index],
            face.vertices[index + 1],
        ]);
    }
    triangles
}

fn polygon_is_convex(points: &[Vec2]) -> bool {
    let orientation = polygon_signed_area(points).signum();
    if orientation.abs() <= GEOMETRY_EPSILON {
        return false;
    }
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        if orient2(previous, current, next) * orientation < -GEOMETRY_EPSILON {
            return false;
        }
    }
    true
}

fn ear_clip_face(
    face: &PolygonFace,
    projected: &[Vec2],
    face_index: usize,
) -> Result<Vec<[u32; 3]>, PolyError> {
    let orientation = polygon_signed_area(projected).signum();
    if orientation.abs() <= GEOMETRY_EPSILON {
        return Err(PolyError::InvalidMesh(format!(
            "face {face_index} has zero projected area"
        )));
    }
    let mut ring = (0..face.vertices.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(face.vertices.len() - 2);

    while ring.len() > 3 {
        let mut clipped = false;
        for ring_index in 0..ring.len() {
            let previous = ring[(ring_index + ring.len() - 1) % ring.len()];
            let current = ring[ring_index];
            let next = ring[(ring_index + 1) % ring.len()];
            if !is_ear(previous, current, next, &ring, projected, orientation) {
                continue;
            }
            triangles.push([
                face.vertices[previous],
                face.vertices[current],
                face.vertices[next],
            ]);
            ring.remove(ring_index);
            clipped = true;
            break;
        }
        if !clipped {
            return Err(PolyError::InvalidMesh(format!(
                "face {face_index} cannot be ear-clipped"
            )));
        }
    }

    triangles.push([
        face.vertices[ring[0]],
        face.vertices[ring[1]],
        face.vertices[ring[2]],
    ]);
    Ok(triangles)
}

fn is_ear(
    previous: usize,
    current: usize,
    next: usize,
    ring: &[usize],
    points: &[Vec2],
    orientation: f32,
) -> bool {
    if orient2(points[previous], points[current], points[next]) * orientation <= GEOMETRY_EPSILON {
        return false;
    }
    for candidate in ring {
        if *candidate == previous || *candidate == current || *candidate == next {
            continue;
        }
        if point_in_triangle(
            points[*candidate],
            points[previous],
            points[current],
            points[next],
            orientation,
        ) {
            return false;
        }
    }
    true
}

fn point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2, orientation: f32) -> bool {
    orient2(a, b, point) * orientation >= -GEOMETRY_EPSILON
        && orient2(b, c, point) * orientation >= -GEOMETRY_EPSILON
        && orient2(c, a, point) * orientation >= -GEOMETRY_EPSILON
}

fn polygon_signed_area(points: &[Vec2]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

fn orient2(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn build_boundary_loops(boundary_edges: &[DirectedEdge]) -> Vec<BoundaryLoop> {
    let mut outgoing: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for edge in boundary_edges {
        outgoing.entry(edge.from).or_default().insert(edge.to);
    }

    let mut visited = BTreeSet::new();
    let mut loops = Vec::new();
    for edge in boundary_edges {
        if visited.contains(edge) {
            continue;
        }
        let mut vertices = vec![edge.from];
        let mut edges = Vec::new();
        let mut current = *edge;
        loop {
            if !visited.insert(current) {
                break;
            }
            edges.push(current.key());
            vertices.push(current.to);
            if current.to == vertices[0] {
                break;
            }
            let Some(next_targets) = outgoing.get(&current.to) else {
                break;
            };
            let Some(next) = next_targets
                .iter()
                .copied()
                .map(|to| DirectedEdge {
                    from: current.to,
                    to,
                })
                .find(|candidate| !visited.contains(candidate))
            else {
                break;
            };
            current = next;
        }
        loops.push(BoundaryLoop { vertices, edges });
    }
    loops.sort_by(|left, right| left.vertices.cmp(&right.vertices));
    loops
}

fn build_connected_components(face_neighbors: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; face_neighbors.len()];
    let mut components = Vec::new();
    for start in 0..face_neighbors.len() {
        if seen[start] {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(face) = queue.pop_front() {
            component.push(face);
            for neighbor in &face_neighbors[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn compute_face_normal_vectors(mesh: &PolygonMesh) -> Result<Vec<Vec3>, PolyError> {
    ensure_valid(mesh)?;
    Ok(mesh
        .faces
        .iter()
        .map(|face| face_normal(mesh, face))
        .collect())
}

fn face_normal(mesh: &PolygonMesh, face: &PolygonFace) -> Vec3 {
    face_geometry(mesh, face)
        .map(|geometry| geometry.normal)
        .unwrap_or(Vec3::Y)
}

#[derive(Debug, Clone)]
struct SplitGroup {
    vertex: u32,
    faces: Vec<usize>,
    normal_index: usize,
}

#[derive(Debug, Clone)]
struct SplitNormalLookup {
    split_normals: Vec<Vec3>,
    vertex_face_split: BTreeMap<(u32, usize), usize>,
    splits: Vec<SplitGroup>,
}

fn build_split_normal_lookup(mesh: &PolygonMesh) -> Result<SplitNormalLookup, PolyError> {
    ensure_valid(mesh)?;
    let face_normals = compute_face_normal_vectors(mesh)?;
    let adjacency = build_adjacency(mesh)?;
    let mut split_normals = Vec::new();
    let mut vertex_face_split = BTreeMap::new();
    let mut splits = Vec::new();

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
            let mut normal = Vec3::ZERO;
            for face in &faces {
                normal += face_normals[*face];
            }
            let normal = normalize_or_default(normal);
            let normal_index = split_normals.len();
            split_normals.push(normal);
            for face in &faces {
                vertex_face_split.insert((vertex_index as u32, *face), normal_index);
            }
            splits.push(SplitGroup {
                vertex: vertex_index as u32,
                faces,
                normal_index,
            });
        }
    }

    Ok(SplitNormalLookup {
        split_normals,
        vertex_face_split,
        splits,
    })
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
    let left_group = mesh.face_metadata[left_face].smoothing_group;
    let right_group = mesh.face_metadata[right_face].smoothing_group;
    left_group == right_group
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

fn transform_orientation_flips(transform: &Transform3) -> bool {
    transform.scale[0] * transform.scale[1] * transform.scale[2] < 0.0
}

fn normal_transform_matrix(transform: &Transform3) -> Result<Mat3, PolyError> {
    let rotation = Quat::from_euler(
        EulerRot::XYZ,
        transform.rotation_degrees[0].to_radians(),
        transform.rotation_degrees[1].to_radians(),
        transform.rotation_degrees[2].to_radians(),
    );
    let matrix = Mat3::from_quat(rotation) * Mat3::from_diagonal(Vec3::from_array(transform.scale));
    let normal_matrix = matrix.inverse().transpose();
    mat3_is_finite(&normal_matrix)
        .then_some(normal_matrix)
        .ok_or_else(|| {
            PolyError::InvalidMesh(
                "transform scale must be invertible to transform normals".to_owned(),
            )
        })
}

fn normalize_or_default(vector: Vec3) -> Vec3 {
    let length = vector.length();
    if !length.is_finite() || length <= MIN_NORMAL_LENGTH {
        Vec3::Y
    } else {
        vector / length
    }
}

fn fnv_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn array_is_finite(values: [f32; 3]) -> bool {
    values.iter().copied().all(f32::is_finite)
}

fn push_issue(
    report: &mut PolyValidationReport,
    subject: Option<String>,
    code: &'static str,
    message: impl Into<String>,
) {
    report.issues.push(PolyValidationIssue {
        subject,
        code: code.to_owned(),
        message: message.into(),
    });
}

fn mat3_is_finite(matrix: &Mat3) -> bool {
    matrix.x_axis.is_finite() && matrix.y_axis.is_finite() && matrix.z_axis.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad_mesh() -> PolygonMesh {
        polygon_mesh_from_faces(
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            vec![vec![0, 1, 2, 3]],
            vec![FaceMetadata {
                part_definition: Some(PartDefinitionId(1)),
                part_instance: Some(PartInstanceId(2)),
                region: Some(RegionId(3)),
                operation: Some(OperationId(4)),
                smoothing_group: Some(1),
                surface_role: Some(SurfaceRole::Panel),
            }],
        )
        .expect("quad mesh should be valid")
    }

    fn cube_mesh() -> PolygonMesh {
        let mut mesh = polygon_mesh_from_faces(
            vec![
                [-1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
            vec![
                vec![0, 3, 2, 1],
                vec![4, 5, 6, 7],
                vec![0, 1, 5, 4],
                vec![1, 2, 6, 5],
                vec![2, 3, 7, 6],
                vec![3, 0, 4, 7],
            ],
            vec![FaceMetadata::default(); 6],
        )
        .expect("cube mesh should be valid");
        for edge in collect_canonical_edges(&mesh)
            .expect("edges")
            .keys()
            .copied()
        {
            mesh.edge_metadata.insert(edge, EdgeMetadata::hard());
        }
        mesh
    }

    fn offset_ids(mesh: &mut PolygonMesh, vertex_offset: u64, face_offset: u64) {
        for vertex_id in &mut mesh.vertex_ids {
            vertex_id.0 += vertex_offset;
        }
        for face in &mut mesh.faces {
            face.id.0 += face_offset;
        }
    }

    fn issue_codes(mesh: &PolygonMesh) -> BTreeSet<String> {
        validate_polygon_mesh(mesh)
            .issues
            .into_iter()
            .map(|issue| issue.code)
            .collect()
    }

    #[test]
    fn serde_json_round_trip_preserves_polygon_mesh() {
        let mesh = quad_mesh();

        let json = serde_json::to_string(&mesh).expect("mesh should serialize");
        let round_tripped: PolygonMesh =
            serde_json::from_str(&json).expect("mesh should deserialize");

        assert_eq!(mesh, round_tripped);
    }

    #[test]
    fn quad_triangulation_uses_stable_diagonal_and_provenance() {
        let mesh = quad_mesh();

        let triangles = triangulate_polygon_mesh(&mesh).expect("quad should triangulate");

        assert_eq!(triangles.mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(
            triangles.triangle_to_polygon_face,
            vec![ElementId(0), ElementId(0)]
        );
        assert_eq!(
            triangles.triangle_to_region,
            vec![Some(RegionId(3)), Some(RegionId(3))]
        );
        assert_eq!(
            triangles.triangle_to_part,
            vec![Some(PartInstanceId(2)), Some(PartInstanceId(2))]
        );
        assert_eq!(
            triangles.triangle_to_operation,
            vec![Some(OperationId(4)), Some(OperationId(4))]
        );
        assert_eq!(triangles.vertex_ids, mesh.vertex_ids);
    }

    #[test]
    fn concave_polygon_triangulates_with_ear_clipping() {
        let mesh = polygon_mesh_from_faces(
            vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 1.0, 0.0],
                [1.0, 0.35, 0.0],
                [0.0, 1.0, 0.0],
            ],
            vec![vec![0, 1, 2, 3, 4]],
            vec![FaceMetadata::default()],
        )
        .expect("concave mesh should validate");

        let triangles = triangulate_polygon_mesh(&mesh).expect("concave mesh should triangulate");

        assert_eq!(triangles.mesh.indices.len(), 9);
        assert_eq!(triangles.triangle_to_polygon_face.len(), 3);
    }

    #[test]
    fn invalid_bow_tie_polygon_is_rejected() {
        let mesh = PolygonMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
            vertex_ids: vec![ElementId(0), ElementId(1), ElementId(2), ElementId(3)],
            faces: vec![PolygonFace {
                id: ElementId(0),
                vertices: vec![0, 1, 2, 3],
            }],
            face_metadata: vec![FaceMetadata::default()],
            edge_metadata: BTreeMap::from([
                (EdgeKey::new(0, 1), EdgeMetadata::open_boundary()),
                (EdgeKey::new(1, 2), EdgeMetadata::open_boundary()),
                (EdgeKey::new(2, 3), EdgeMetadata::open_boundary()),
                (EdgeKey::new(0, 3), EdgeMetadata::open_boundary()),
            ]),
            topology_signature: 0,
            bounds: MeshBounds {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0],
            },
        };

        let codes = issue_codes(&mesh);

        assert!(codes.contains("self_intersecting_face"));
    }

    #[test]
    fn cube_adjacency_is_deterministic() {
        let mesh = cube_mesh();

        let adjacency = build_adjacency(&mesh).expect("adjacency should build");

        assert!(adjacency.boundary_loops.is_empty());
        assert_eq!(adjacency.connected_components, vec![vec![0, 1, 2, 3, 4, 5]]);
        assert_eq!(adjacency.vertex_faces[0], vec![0, 2, 5]);
        assert_eq!(adjacency.edge_faces.len(), 12);
        assert!(adjacency.face_neighbors[0].contains(&2));
        assert!(adjacency.face_neighbors[0].contains(&3));
        assert!(adjacency.face_neighbors[0].contains(&4));
        assert!(adjacency.face_neighbors[0].contains(&5));
    }

    #[test]
    fn open_plate_boundaries_are_reported_as_loop() {
        let mesh = quad_mesh();

        let adjacency = build_adjacency(&mesh).expect("adjacency should build");

        assert_eq!(adjacency.boundary_loops.len(), 1);
        assert_eq!(adjacency.boundary_loops[0].vertices, vec![0, 1, 2, 3, 0]);
    }

    #[test]
    fn nonmanifold_edge_is_detected() {
        let mesh = PolygonMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            vertex_ids: (0..5).map(ElementId).collect(),
            faces: vec![
                PolygonFace {
                    id: ElementId(0),
                    vertices: vec![0, 1, 2],
                },
                PolygonFace {
                    id: ElementId(1),
                    vertices: vec![1, 0, 3],
                },
                PolygonFace {
                    id: ElementId(2),
                    vertices: vec![0, 1, 4],
                },
            ],
            face_metadata: vec![FaceMetadata::default(); 3],
            edge_metadata: BTreeMap::new(),
            topology_signature: 0,
            bounds: MeshBounds {
                min: [0.0, -1.0, 0.0],
                max: [1.0, 1.0, 1.0],
            },
        };

        let codes = issue_codes(&mesh);

        assert!(codes.contains("nonmanifold_edge"));
    }

    #[test]
    fn split_hard_edge_normals_duplicate_triangle_vertices() {
        let mesh = cube_mesh();

        let triangles = triangulate_polygon_mesh(&mesh).expect("cube should triangulate");

        assert!(triangles.mesh.positions.len() > mesh.positions.len());
        assert_eq!(triangles.mesh.indices.len(), 36);
        assert!(triangles.mesh.normals.contains(&[0.0, 0.0, -1.0]));
        assert!(triangles.mesh.normals.contains(&[0.0, 0.0, 1.0]));
    }

    #[test]
    fn transform_preserves_ids_and_moves_positions() {
        let mesh = quad_mesh();
        let transform = Transform3 {
            translation: [2.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 90.0],
            scale: [2.0, 1.0, 1.0],
        };

        let transformed =
            transform_polygon_mesh(&mesh, &transform).expect("transform should succeed");

        assert_eq!(mesh.vertex_ids, transformed.vertex_ids);
        assert_eq!(mesh.faces[0].id, transformed.faces[0].id);
        assert_eq!(transformed.positions[0], [2.0, 0.0, 0.0]);
        assert_eq!(transformed.edge_metadata, mesh.edge_metadata);
    }

    #[test]
    fn mesh_combination_offsets_indices_and_rejects_id_collisions() {
        let first = quad_mesh();
        let mut second = transform_polygon_mesh(
            &quad_mesh(),
            &Transform3 {
                translation: [2.0, 0.0, 0.0],
                ..Transform3::default()
            },
        )
        .expect("transform should succeed");
        assert!(combine_polygon_meshes(&[first.clone(), second.clone()]).is_err());

        offset_ids(&mut second, 10, 20);
        let combined = combine_polygon_meshes(&[first, second]).expect("combine should succeed");

        assert_eq!(combined.positions.len(), 8);
        assert_eq!(combined.faces[1].vertices, vec![4, 5, 6, 7]);
        assert!(validate_polygon_mesh(&combined).is_valid());
    }

    #[test]
    fn validation_reports_required_issue_families() {
        let mesh = PolygonMesh {
            positions: vec![[0.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0]],
            vertex_ids: vec![ElementId(1), ElementId(1)],
            faces: vec![
                PolygonFace {
                    id: ElementId(1),
                    vertices: vec![0, 1, 1],
                },
                PolygonFace {
                    id: ElementId(1),
                    vertices: vec![0, 5],
                },
            ],
            face_metadata: vec![FaceMetadata {
                part_definition: Some(PartDefinitionId(0)),
                part_instance: None,
                region: Some(RegionId(0)),
                operation: Some(OperationId(0)),
                smoothing_group: None,
                surface_role: Some(SurfaceRole::Custom(String::new())),
            }],
            edge_metadata: BTreeMap::from([(EdgeKey::new(0, 7), EdgeMetadata::open_boundary())]),
            topology_signature: 0,
            bounds: MeshBounds::empty(),
        };

        let codes = issue_codes(&mesh);

        assert!(codes.contains("non_finite_position"));
        assert!(codes.contains("invalid_metadata_lengths"));
        assert!(codes.contains("duplicate_vertex_id"));
        assert!(codes.contains("duplicate_face_id"));
        assert!(codes.contains("invalid_face_index"));
        assert!(codes.contains("face_too_small"));
        assert!(codes.contains("repeated_consecutive_vertex"));
        assert!(codes.contains("repeated_face_vertex"));
        assert!(codes.contains("invalid_provenance_metadata"));
        assert!(codes.contains("invalid_region_metadata"));
        assert!(codes.contains("invalid_edge_key"));
    }

    #[test]
    fn undeclared_open_boundary_is_detected() {
        let mesh = PolygonMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vertex_ids: vec![ElementId(0), ElementId(1), ElementId(2)],
            faces: vec![PolygonFace {
                id: ElementId(0),
                vertices: vec![0, 1, 2],
            }],
            face_metadata: vec![FaceMetadata::default()],
            edge_metadata: BTreeMap::new(),
            topology_signature: 0,
            bounds: MeshBounds {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0],
            },
        };

        assert!(issue_codes(&mesh).contains("undeclared_open_boundary"));
    }

    #[test]
    fn deterministic_output_is_stable() {
        let mesh = cube_mesh();

        let first_adjacency = build_adjacency(&mesh).expect("adjacency");
        let second_adjacency = build_adjacency(&mesh).expect("adjacency");
        let first_triangles = triangulate_polygon_mesh(&mesh).expect("triangles");
        let second_triangles = triangulate_polygon_mesh(&mesh).expect("triangles");

        assert_eq!(first_adjacency, second_adjacency);
        assert_eq!(first_triangles, second_triangles);
    }
}
