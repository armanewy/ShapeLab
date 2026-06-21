#![forbid(unsafe_code)]

//! Explicit indexed polygon topology contracts for the part-aware modeling lane.
//!
//! Vertex and face IDs in this crate are deterministic for a given topology
//! signature. They are not guaranteed to survive topology-changing parameters
//! such as segment counts. Semantic provenance is carried through metadata that
//! points back to asset part, region, and operation IDs.

use std::collections::{BTreeMap, BTreeSet};

use glam::Vec3;
use serde::{Deserialize, Serialize};
use shape_asset::{
    OperationId, PartDefinitionId, PartInstanceId, RegionId, SurfaceRole, Transform3,
};
use thiserror::Error;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const MIN_NORMAL_LENGTH: f32 = 1.0e-6;

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
    pub edge_metadata: BTreeMap<EdgeKey, EdgeMetadata>,
    /// Deterministic topology signature.
    pub topology_signature: u64,
    /// Mesh bounds.
    pub bounds: MeshBounds,
}

impl PolygonMesh {
    /// Return an empty polygon mesh.
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
    /// Stable vertex IDs copied from the polygon mesh.
    pub vertex_ids: Vec<ElementId>,
}

/// Polygon mesh adjacency by face and edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAdjacency {
    /// Neighboring face indices for each face.
    pub face_neighbors: Vec<BTreeSet<usize>>,
    /// Face indices incident to each edge.
    pub edge_faces: BTreeMap<EdgeKey, Vec<usize>>,
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

/// Validate polygon mesh invariants and collect every issue.
#[must_use]
pub fn validate_polygon_mesh(mesh: &PolygonMesh) -> PolyValidationReport {
    let mut report = PolyValidationReport::default();

    if mesh.positions.len() != mesh.vertex_ids.len() {
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
            "face_metadata_count_mismatch",
            "Face and face metadata counts must match.",
        );
    }
    for (index, position) in mesh.positions.iter().enumerate() {
        if !array_is_finite(*position) {
            push_issue(
                &mut report,
                Some(format!("position.{index}")),
                "non_finite_position",
                "All positions must be finite.",
            );
        }
    }
    for (face_index, face) in mesh.faces.iter().enumerate() {
        if face.vertices.len() < 3 {
            push_issue(
                &mut report,
                Some(format!("face.{face_index}")),
                "face_too_small",
                "Polygon faces must have at least three vertices.",
            );
        }
        let mut unique_vertices = BTreeSet::new();
        for vertex in &face.vertices {
            if *vertex as usize >= mesh.positions.len() {
                push_issue(
                    &mut report,
                    Some(format!("face.{face_index}")),
                    "vertex_index_out_of_range",
                    "Face references a missing vertex.",
                );
            }
            unique_vertices.insert(*vertex);
        }
        if unique_vertices.len() != face.vertices.len() {
            push_issue(
                &mut report,
                Some(format!("face.{face_index}")),
                "duplicate_face_vertex",
                "Polygon face repeats a vertex index.",
            );
        }
    }
    for edge in mesh.edge_metadata.keys() {
        if edge.a == edge.b
            || edge.a as usize >= mesh.positions.len()
            || edge.b as usize >= mesh.positions.len()
        {
            push_issue(
                &mut report,
                Some(format!("edge.{}.{}", edge.a, edge.b)),
                "invalid_edge_key",
                "Edge metadata key must reference two distinct vertices.",
            );
        }
    }
    if !mesh.bounds.is_empty()
        && (!array_is_finite(mesh.bounds.min) || !array_is_finite(mesh.bounds.max))
    {
        push_issue(
            &mut report,
            None,
            "non_finite_bounds",
            "Mesh bounds must be finite or empty.",
        );
    }

    report
}

/// Build face and edge adjacency for a valid polygon mesh.
pub fn build_adjacency(mesh: &PolygonMesh) -> Result<MeshAdjacency, PolyError> {
    ensure_valid(mesh)?;
    let mut edge_faces: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for edge in face_edges(face) {
            edge_faces.entry(edge).or_default().push(face_index);
        }
    }

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

    Ok(MeshAdjacency {
        face_neighbors,
        edge_faces,
    })
}

/// Triangulate polygon faces with deterministic fan triangulation.
pub fn triangulate_polygon_mesh(mesh: &PolygonMesh) -> Result<TriangulatedPolygonMesh, PolyError> {
    ensure_valid(mesh)?;
    let normals = compute_split_vertex_normals(mesh)?;
    let mut indices = Vec::new();
    let mut triangle_to_polygon_face = Vec::new();
    let mut triangle_to_region = Vec::new();
    let mut triangle_to_part = Vec::new();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        let metadata = mesh
            .face_metadata
            .get(face_index)
            .ok_or_else(|| PolyError::InvalidMesh("missing face metadata".to_owned()))?;
        for vertex_index in 1..face.vertices.len() - 1 {
            indices.push(face.vertices[0]);
            indices.push(face.vertices[vertex_index]);
            indices.push(face.vertices[vertex_index + 1]);
            triangle_to_polygon_face.push(face.id);
            triangle_to_region.push(metadata.region);
            triangle_to_part.push(metadata.part_instance);
        }
    }

    Ok(TriangulatedPolygonMesh {
        mesh: TriangleMesh {
            positions: mesh.positions.clone(),
            normals,
            indices,
            bounds: mesh.bounds,
        },
        triangle_to_polygon_face,
        triangle_to_region,
        triangle_to_part,
        vertex_ids: mesh.vertex_ids.clone(),
    })
}

/// Compute one normal per polygon face.
pub fn compute_face_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, PolyError> {
    ensure_valid(mesh)?;
    let mut normals = Vec::with_capacity(mesh.faces.len());
    for face in &mesh.faces {
        normals.push(face_normal(mesh, face));
    }
    Ok(normals)
}

/// Compute averaged split vertex normals.
pub fn compute_split_vertex_normals(mesh: &PolygonMesh) -> Result<Vec<[f32; 3]>, PolyError> {
    ensure_valid(mesh)?;
    let face_normals = compute_face_normals(mesh)?;
    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    for (face, normal) in mesh.faces.iter().zip(face_normals) {
        let normal = Vec3::from_array(normal);
        for vertex in &face.vertices {
            let Some(target) = normals.get_mut(*vertex as usize) else {
                return Err(PolyError::InvalidMesh(
                    "face references missing vertex".to_owned(),
                ));
            };
            *target += normal;
        }
    }
    Ok(normals
        .into_iter()
        .map(|normal| normalize_or_default(normal).to_array())
        .collect())
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
    let mut transformed = mesh.clone();
    transformed.positions = positions;
    transformed.bounds = bounds_from_positions(&transformed.positions)?;
    Ok(transformed)
}

/// Combine multiple polygon meshes into one indexed polygon mesh.
pub fn combine_polygon_meshes(meshes: &[PolygonMesh]) -> Result<PolygonMesh, PolyError> {
    let mut combined = PolygonMesh::empty();
    let mut vertex_offset: u32 = 0;

    for mesh in meshes {
        ensure_valid(mesh)?;
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
    ensure_valid(&combined)?;
    Ok(combined)
}

/// Construct a polygon mesh from positions, faces, and per-face metadata.
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
    let mesh = PolygonMesh {
        positions,
        vertex_ids,
        faces,
        face_metadata,
        edge_metadata: BTreeMap::new(),
        topology_signature,
        bounds,
    };
    ensure_valid(&mesh)?;
    Ok(mesh)
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

fn ensure_valid(mesh: &PolygonMesh) -> Result<(), PolyError> {
    let report = validate_polygon_mesh(mesh);
    if report.is_valid() {
        Ok(())
    } else {
        Err(PolyError::ValidationFailed(report))
    }
}

fn face_edges(face: &PolygonFace) -> Vec<EdgeKey> {
    let mut edges = Vec::with_capacity(face.vertices.len());
    for index in 0..face.vertices.len() {
        let next = (index + 1) % face.vertices.len();
        edges.push(EdgeKey::new(face.vertices[index], face.vertices[next]));
    }
    edges
}

fn face_normal(mesh: &PolygonMesh, face: &PolygonFace) -> [f32; 3] {
    let Some(first) = face.vertices.first() else {
        return [0.0, 1.0, 0.0];
    };
    let Some(origin) = mesh.positions.get(*first as usize) else {
        return [0.0, 1.0, 0.0];
    };
    let origin = Vec3::from_array(*origin);
    let mut normal = Vec3::ZERO;
    for window in face.vertices[1..].windows(2) {
        let Some(a) = mesh.positions.get(window[0] as usize) else {
            continue;
        };
        let Some(b) = mesh.positions.get(window[1] as usize) else {
            continue;
        };
        normal += (Vec3::from_array(*a) - origin).cross(Vec3::from_array(*b) - origin);
    }
    normalize_or_default(normal).to_array()
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

    #[test]
    fn serde_json_round_trip_preserves_polygon_mesh() {
        let mesh = quad_mesh();

        let json = serde_json::to_string(&mesh).expect("mesh should serialize");
        let round_tripped: PolygonMesh =
            serde_json::from_str(&json).expect("mesh should deserialize");

        assert_eq!(mesh, round_tripped);
    }

    #[test]
    fn triangulation_maps_back_to_semantic_sources() {
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
    }

    #[test]
    fn adjacency_finds_faces_sharing_an_edge() {
        let mesh = polygon_mesh_from_faces(
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            vec![vec![0, 1, 2], vec![0, 2, 3]],
            vec![FaceMetadata::default(), FaceMetadata::default()],
        )
        .expect("two triangles should be valid");

        let adjacency = build_adjacency(&mesh).expect("adjacency should build");

        assert!(adjacency.face_neighbors[0].contains(&1));
        assert_eq!(
            adjacency.edge_faces.get(&EdgeKey::new(0, 2)),
            Some(&vec![0, 1])
        );
    }

    #[test]
    fn transform_preserves_topology_signature() {
        let mesh = quad_mesh();
        let transform = Transform3 {
            translation: [2.0, 0.0, 0.0],
            ..Transform3::default()
        };

        let transformed =
            transform_polygon_mesh(&mesh, &transform).expect("transform should succeed");

        assert_eq!(mesh.topology_signature, transformed.topology_signature);
        assert_eq!(transformed.positions[0], [2.0, 0.0, 0.0]);
    }

    #[test]
    fn combine_offsets_indices() {
        let first = quad_mesh();
        let second = transform_polygon_mesh(
            &quad_mesh(),
            &Transform3 {
                translation: [2.0, 0.0, 0.0],
                ..Transform3::default()
            },
        )
        .expect("transform should succeed");

        let combined = combine_polygon_meshes(&[first, second]).expect("combine should succeed");

        assert_eq!(combined.positions.len(), 8);
        assert_eq!(combined.faces[1].vertices, vec![4, 5, 6, 7]);
        assert!(validate_polygon_mesh(&combined).is_valid());
    }

    #[test]
    fn validation_reports_bad_indices_and_metadata_counts() {
        let mesh = PolygonMesh {
            positions: vec![[0.0, 0.0, 0.0]],
            vertex_ids: Vec::new(),
            faces: vec![PolygonFace {
                id: ElementId(1),
                vertices: vec![0, 1, 1],
            }],
            face_metadata: Vec::new(),
            edge_metadata: BTreeMap::new(),
            topology_signature: 0,
            bounds: MeshBounds::empty(),
        };

        let codes = validate_polygon_mesh(&mesh)
            .issues
            .into_iter()
            .map(|issue| issue.code)
            .collect::<BTreeSet<_>>();

        assert!(codes.contains("vertex_id_count_mismatch"));
        assert!(codes.contains("face_metadata_count_mismatch"));
        assert!(codes.contains("vertex_index_out_of_range"));
        assert!(codes.contains("duplicate_face_vertex"));
    }
}
