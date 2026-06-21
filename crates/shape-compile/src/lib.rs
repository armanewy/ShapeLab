#![forbid(unsafe_code)]

//! Asset recipe compilation for the explicit polygon production path.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetRecipe, OperationId, PartDefinitionId, PartInstanceId, RegionId, SocketId, SocketSpec,
    validate_asset_recipe,
};
use shape_modeling::assembly::{AssemblyError, evaluate_assembly};
use shape_poly::{
    BoundaryRole, ElementId, MeshAdjacency, PolyError, PolygonMesh, TriangleMesh,
    TriangulatedPolygonMesh, build_adjacency, triangulate_polygon_mesh, validate_polygon_mesh,
};
use thiserror::Error;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

type ProvenanceKey = (
    Option<PartDefinitionId>,
    Option<PartInstanceId>,
    Option<RegionId>,
    Option<OperationId>,
);

/// Compiled local and world-space mesh for one part occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPart {
    /// Source part definition.
    pub definition_id: PartDefinitionId,
    /// Source or generated part instance.
    pub instance_id: PartInstanceId,
    /// Stable object/group name.
    pub instance_name: String,
    /// Source prototype for generated assembly occurrences.
    pub prototype_instance_id: Option<PartInstanceId>,
    /// Operation that generated this occurrence.
    pub generated_by: Option<OperationId>,
    /// Whether this occurrence came directly from the recipe.
    pub source_recipe_instance: bool,
    /// Local polygon mesh.
    pub local_mesh: PolygonMesh,
    /// World-space polygon mesh.
    pub world_mesh: PolygonMesh,
    /// Triangulated world-space mesh used by preview/export.
    pub triangulated_world: TriangulatedPolygonMesh,
    /// Sockets transformed into world coordinates.
    pub sockets_world: BTreeMap<SocketId, SocketSpec>,
    /// Validation report for this compiled part.
    pub validation_report: CompileValidationReport,
}

/// Complete asset artifact produced by compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetArtifact {
    /// Stable non-cryptographic hash of the source recipe JSON.
    pub source_recipe_hash: u64,
    /// Compiled part payloads.
    pub compiled_parts: Vec<CompiledPart>,
    /// Combined world-space polygon mesh.
    pub combined_polygon: PolygonMesh,
    /// Combined preview triangle mesh.
    pub combined_preview: TriangulatedPolygonMesh,
    /// Semantic provenance report.
    pub provenance_report: ProvenanceReport,
    /// Compilation validation report.
    pub validation_report: CompileValidationReport,
    /// Compilation statistics.
    pub statistics: CompileStatistics,
}

/// Provenance summary for compiled topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceReport {
    /// Definition generation order.
    pub definition_generation_order: Vec<PartDefinitionId>,
    /// Occurrence output order.
    pub instance_order: Vec<PartInstanceId>,
    /// Per-part/region/operation face mappings.
    pub part_region_operation_mappings: Vec<ProvenanceMapping>,
    /// Element counts by stable label.
    pub element_counts: BTreeMap<String, u64>,
    /// Topology signatures by part instance.
    pub topology_signatures: BTreeMap<PartInstanceId, u64>,
}

/// One provenance mapping row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceMapping {
    /// Source part definition.
    pub part_definition: Option<PartDefinitionId>,
    /// Source part instance.
    pub part_instance: Option<PartInstanceId>,
    /// Source region, if known.
    pub region: Option<RegionId>,
    /// Source operation, if known.
    pub operation: Option<OperationId>,
    /// Number of faces with this mapping.
    pub polygon_face_count: u64,
}

/// Aggregate compile statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileStatistics {
    /// Number of compiled part occurrences.
    pub part_count: u64,
    /// Total world-space polygon vertices.
    pub polygon_vertex_count: u64,
    /// Total world-space polygon faces.
    pub polygon_face_count: u64,
    /// Combined preview triangle count.
    pub triangle_count: u64,
    /// Whether any reserved SDF/remeshing path was used.
    pub used_sdf_or_remeshing: bool,
}

/// Validation issue for compiled artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation report for compiled artifacts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileValidationReport {
    /// Discovered issues.
    pub issues: Vec<CompileValidationIssue>,
}

impl CompileValidationReport {
    /// Return true when no issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Error type for asset compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Source asset recipe is invalid.
    #[error("asset recipe validation failed")]
    AssetValidation(shape_asset::AssetValidationReport),
    /// Requested instance does not exist.
    #[error("unknown part instance {0:?}")]
    UnknownInstance(PartInstanceId),
    /// Requested definition does not exist.
    #[error("unknown part definition {0:?}")]
    UnknownDefinition(PartDefinitionId),
    /// Assembly evaluation failed.
    #[error("assembly error: {0}")]
    Assembly(#[from] AssemblyError),
    /// Polygon topology helper failed.
    #[error("polygon error: {0}")]
    Polygon(#[from] PolyError),
    /// JSON serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Text output formatting failed.
    #[error("text formatting failed")]
    Format,
}

/// Compile a complete asset recipe.
pub fn compile_asset(recipe: &AssetRecipe) -> Result<AssetArtifact, CompileError> {
    ensure_valid_recipe(recipe)?;
    if recipe.instances.values().all(|instance| !instance.enabled) {
        let combined_polygon = PolygonMesh::empty();
        let combined_preview = TriangulatedPolygonMesh {
            mesh: TriangleMesh {
                positions: Vec::new(),
                normals: Vec::new(),
                indices: Vec::new(),
                bounds: combined_polygon.bounds,
            },
            triangle_to_polygon_face: Vec::new(),
            triangle_to_region: Vec::new(),
            triangle_to_part: Vec::new(),
            triangle_to_operation: Vec::new(),
            vertex_ids: Vec::new(),
        };
        return Ok(AssetArtifact {
            source_recipe_hash: source_recipe_hash(recipe)?,
            compiled_parts: Vec::new(),
            combined_polygon,
            combined_preview,
            provenance_report: ProvenanceReport {
                definition_generation_order: Vec::new(),
                instance_order: Vec::new(),
                part_region_operation_mappings: Vec::new(),
                element_counts: BTreeMap::from([
                    ("polygon_vertices".to_owned(), 0),
                    ("polygon_faces".to_owned(), 0),
                ]),
                topology_signatures: BTreeMap::new(),
            },
            validation_report: CompileValidationReport::default(),
            statistics: CompileStatistics::default(),
        });
    }
    let evaluation = evaluate_assembly(recipe)?;
    let mut compiled_parts = Vec::new();

    for occurrence in &evaluation.instances {
        let local_part = evaluation
            .local_parts
            .get(&occurrence.definition_id)
            .ok_or(CompileError::UnknownDefinition(occurrence.definition_id))?;
        let world_mesh = evaluation
            .world_meshes
            .get(&occurrence.instance_id)
            .ok_or(CompileError::UnknownInstance(occurrence.instance_id))?
            .clone();
        let sockets_world = evaluation
            .world_sockets
            .get(&occurrence.instance_id)
            .cloned()
            .unwrap_or_default();
        let triangulated_world = triangulate_polygon_mesh(&world_mesh)?;
        let instance_name = recipe
            .instances
            .get(&occurrence.instance_id)
            .map(|instance| instance.name.clone())
            .or_else(|| {
                occurrence.prototype_instance_id.and_then(|prototype| {
                    recipe.instances.get(&prototype).map(|instance| {
                        format!("{} generated {}", instance.name, occurrence.instance_id.0)
                    })
                })
            })
            .unwrap_or_else(|| format!("part {}", occurrence.instance_id.0));

        let mut compiled = CompiledPart {
            definition_id: occurrence.definition_id,
            instance_id: occurrence.instance_id,
            instance_name,
            prototype_instance_id: occurrence.prototype_instance_id,
            generated_by: occurrence.generated_by,
            source_recipe_instance: occurrence.source_recipe_instance,
            local_mesh: local_part.local_mesh.clone(),
            world_mesh,
            triangulated_world,
            sockets_world,
            validation_report: CompileValidationReport::default(),
        };
        compiled.validation_report = validate_compiled_part(&compiled);
        compiled_parts.push(compiled);
    }

    let combined_polygon = evaluation.combined_preview_mesh;
    let combined_preview = evaluation.combined_preview;
    let provenance_report = build_provenance_report(
        &compiled_parts,
        evaluation.provenance.definition_generation_order,
        evaluation.provenance.instance_order,
    );
    let statistics = build_statistics(&compiled_parts, &combined_preview);
    let mut artifact = AssetArtifact {
        source_recipe_hash: source_recipe_hash(recipe)?,
        compiled_parts,
        combined_polygon,
        combined_preview,
        provenance_report,
        validation_report: CompileValidationReport::default(),
        statistics,
    };
    artifact.validation_report = validate_compiled_asset(&artifact);
    if !artifact.validation_report.is_valid() {
        return Ok(artifact);
    }
    Ok(artifact)
}

/// Validate a compiled asset artifact.
#[must_use]
pub fn validate_compiled_asset(artifact: &AssetArtifact) -> CompileValidationReport {
    let mut report = CompileValidationReport::default();
    if artifact.statistics.part_count != artifact.compiled_parts.len() as u64 {
        push_issue(
            &mut report,
            None,
            "part_count_mismatch",
            "Statistics part count does not match compiled parts.",
        );
    }
    if !artifact
        .combined_preview
        .mesh
        .indices
        .len()
        .is_multiple_of(3)
    {
        push_issue(
            &mut report,
            None,
            "triangle_index_count_invalid",
            "Combined preview index count must be divisible by three.",
        );
    }
    if artifact.statistics.used_sdf_or_remeshing {
        push_issue(
            &mut report,
            None,
            "sdf_or_remeshing_used",
            "Explicit asset compile must not use SDF or remeshing paths.",
        );
    }
    for (part_index, part) in artifact.compiled_parts.iter().enumerate() {
        if !part.validation_report.is_valid() {
            push_issue(
                &mut report,
                Some(format!("part.{}", part.instance_id.0)),
                "compiled_part_invalid",
                "Compiled part contains validation issues.",
            );
        }
        if part.triangulated_world.triangle_to_part.len()
            != part.triangulated_world.mesh.indices.len() / 3
        {
            push_issue(
                &mut report,
                Some(format!("part.{part_index}.triangles")),
                "triangle_provenance_count_mismatch",
                "Triangle provenance count must match triangle count.",
            );
        }
    }
    report
}

/// Write an OBJ string grouped by part instance.
pub fn write_grouped_obj(artifact: &AssetArtifact) -> Result<String, CompileError> {
    let mut output = String::new();
    writeln!(&mut output, "# Shape Lab explicit polygon artifact")
        .map_err(|_| CompileError::Format)?;
    writeln!(
        &mut output,
        "# source_recipe_hash {}",
        artifact.source_recipe_hash
    )
    .map_err(|_| CompileError::Format)?;
    let mut vertex_offset = 1_u32;
    for part in &artifact.compiled_parts {
        writeln!(
            &mut output,
            "g {}",
            obj_group_name(part.instance_id, &part.instance_name)
        )
        .map_err(|_| CompileError::Format)?;
        for position in &part.triangulated_world.mesh.positions {
            writeln!(
                &mut output,
                "v {:.9} {:.9} {:.9}",
                position[0], position[1], position[2]
            )
            .map_err(|_| CompileError::Format)?;
        }
        for normal in &part.triangulated_world.mesh.normals {
            writeln!(
                &mut output,
                "vn {:.9} {:.9} {:.9}",
                normal[0], normal[1], normal[2]
            )
            .map_err(|_| CompileError::Format)?;
        }
        for triangle in part.triangulated_world.mesh.indices.chunks_exact(3) {
            let a = triangle[0]
                .checked_add(vertex_offset)
                .ok_or(CompileError::Format)?;
            let b = triangle[1]
                .checked_add(vertex_offset)
                .ok_or(CompileError::Format)?;
            let c = triangle[2]
                .checked_add(vertex_offset)
                .ok_or(CompileError::Format)?;
            writeln!(&mut output, "f {a}//{a} {b}//{b} {c}//{c}")
                .map_err(|_| CompileError::Format)?;
        }
        let add = u32::try_from(part.triangulated_world.mesh.positions.len())
            .map_err(|_| CompileError::Format)?;
        vertex_offset = vertex_offset.checked_add(add).ok_or(CompileError::Format)?;
    }
    Ok(output)
}

/// Write provenance as pretty JSON.
pub fn write_provenance_json(report: &ProvenanceReport) -> Result<String, CompileError> {
    serde_json::to_string_pretty(report).map_err(CompileError::Json)
}

/// Write a Blender reconstruction script for the compiled artifact.
pub fn write_blender_reconstruction_script(
    artifact: &AssetArtifact,
) -> Result<String, CompileError> {
    #[derive(Serialize)]
    struct BlenderPart<'a> {
        name: String,
        instance_id: u64,
        definition_id: u64,
        generated_by: Option<u64>,
        positions: &'a [[f32; 3]],
        faces: Vec<Vec<u32>>,
    }

    let parts = artifact
        .compiled_parts
        .iter()
        .map(|part| BlenderPart {
            name: obj_group_name(part.instance_id, &part.instance_name),
            instance_id: part.instance_id.0,
            definition_id: part.definition_id.0,
            generated_by: part.generated_by.map(|operation| operation.0),
            positions: &part.world_mesh.positions,
            faces: part
                .world_mesh
                .faces
                .iter()
                .map(|face| face.vertices.clone())
                .collect(),
        })
        .collect::<Vec<_>>();
    let parts_json = serde_json::to_string_pretty(&parts)?;
    Ok(format!(
        r#"# Generated by Shape Lab explicit modeling compile.
import argparse
import json
import math
import os
import sys

import bpy

PARTS = json.loads(r'''{parts_json}''')

COLORS = [
    (0.72, 0.78, 0.86, 1.0),
    (0.86, 0.68, 0.58, 1.0),
    (0.62, 0.78, 0.66, 1.0),
    (0.80, 0.72, 0.50, 1.0),
    (0.70, 0.64, 0.82, 1.0),
]

def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-dir", default=os.path.dirname(os.path.abspath(__file__)))
    parser.add_argument("--verify-reopen", action="store_true")
    if "--" in sys.argv:
        return parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    return parser.parse_args([])

def finite_point(point):
    return len(point) == 3 and all(math.isfinite(float(value)) for value in point)

def validate_part(part):
    positions = part["positions"]
    if not all(finite_point(point) for point in positions):
        raise RuntimeError(f"non-finite position in {{part['name']}}")
    for face in part["faces"]:
        if len(face) < 3:
            raise RuntimeError(f"degenerate face in {{part['name']}}")
        if len(set(face)) != len(face):
            raise RuntimeError(f"repeated face index in {{part['name']}}")
        for index in face:
            if index < 0 or index >= len(positions):
                raise RuntimeError(f"face index out of bounds in {{part['name']}}")

def material(index):
    name = f"debug_part_{{index % len(COLORS)}}"
    existing = bpy.data.materials.get(name)
    if existing is not None:
        return existing
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = COLORS[index % len(COLORS)]
    return mat

def create_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    for index, part in enumerate(PARTS):
        validate_part(part)
        mesh = bpy.data.meshes.new(part["name"] + "_mesh")
        mesh.from_pydata(part["positions"], [], part["faces"])
        mesh.update(calc_edges=True)
        obj = bpy.data.objects.new(part["name"], mesh)
        obj["shape_lab_instance_id"] = part["instance_id"]
        obj["shape_lab_definition_id"] = part["definition_id"]
        if part["generated_by"] is not None:
            obj["shape_lab_generated_by"] = part["generated_by"]
        obj.data.materials.append(material(index))
        bpy.context.collection.objects.link(obj)
        if len(mesh.polygons) != len(part["faces"]):
            raise RuntimeError(f"topology mismatch in {{part['name']}}")
        if any(abs(coord) > 1.0e7 for vertex in mesh.vertices for coord in vertex.co):
            raise RuntimeError(f"position magnitude out of range in {{part['name']}}")

def verify_reopen(path):
    bpy.ops.wm.open_mainfile(filepath=path)
    for part in PARTS:
        obj = bpy.data.objects.get(part["name"])
        if obj is None:
            raise RuntimeError(f"missing object after reopen: {{part['name']}}")
        if obj.get("shape_lab_instance_id") != part["instance_id"]:
            raise RuntimeError(f"semantic id mismatch after reopen: {{part['name']}}")
        if len(obj.data.polygons) != len(part["faces"]):
            raise RuntimeError(f"face count mismatch after reopen: {{part['name']}}")

args = parse_args()
os.makedirs(args.out_dir, exist_ok=True)
create_scene()
blend_path = os.path.join(args.out_dir, "reconstructed.blend")
bpy.ops.wm.save_as_mainfile(filepath=blend_path)
if args.verify_reopen:
    verify_reopen(blend_path)
print(json.dumps({{"objects": len(PARTS), "blend": blend_path, "verify_reopen": args.verify_reopen}}, sort_keys=True))
"#
    ))
}

fn ensure_valid_recipe(recipe: &AssetRecipe) -> Result<(), CompileError> {
    let report = validate_asset_recipe(recipe);
    if report.is_valid() {
        Ok(())
    } else {
        Err(CompileError::AssetValidation(report))
    }
}

fn validate_compiled_part(part: &CompiledPart) -> CompileValidationReport {
    let mut report = CompileValidationReport::default();
    append_poly_report(
        &mut report,
        Some(format!("part.{}.local", part.instance_id.0)),
        validate_polygon_mesh(&part.local_mesh),
    );
    append_poly_report(
        &mut report,
        Some(format!("part.{}.world", part.instance_id.0)),
        validate_polygon_mesh(&part.world_mesh),
    );
    validate_topology_shape(
        &mut report,
        Some(format!("part.{}.world", part.instance_id.0)),
        &part.world_mesh,
    );
    validate_triangle_normals(
        &mut report,
        Some(format!("part.{}.triangulated", part.instance_id.0)),
        &part.triangulated_world,
    );
    validate_face_provenance(
        &mut report,
        Some(format!("part.{}.world", part.instance_id.0)),
        &part.world_mesh,
    );
    report
}

fn validate_topology_shape(
    report: &mut CompileValidationReport,
    subject: Option<String>,
    mesh: &PolygonMesh,
) {
    let Ok(adjacency) = build_adjacency(mesh) else {
        return;
    };
    let declared_open = mesh.edge_metadata.values().any(|metadata| {
        matches!(
            metadata.boundary_role,
            BoundaryRole::OpenBoundary | BoundaryRole::SeamCandidate
        )
    });
    if declared_open {
        validate_expected_boundaries(report, subject, mesh, &adjacency);
    } else if !adjacency.boundary_loops.is_empty() {
        push_issue(
            report,
            subject,
            "unexpected_boundary_loop",
            "Closed parts must not contain boundary loops.",
        );
    }
}

fn validate_expected_boundaries(
    report: &mut CompileValidationReport,
    subject: Option<String>,
    mesh: &PolygonMesh,
    adjacency: &MeshAdjacency,
) {
    for loop_ in &adjacency.boundary_loops {
        for edge in &loop_.edges {
            let expected = mesh.edge_metadata.get(edge).is_some_and(|metadata| {
                matches!(
                    metadata.boundary_role,
                    BoundaryRole::OpenBoundary | BoundaryRole::SeamCandidate
                )
            });
            if !expected {
                push_issue(
                    report,
                    subject.clone(),
                    "unexpected_open_boundary",
                    format!("Boundary edge {}.{} is not declared open.", edge.a, edge.b),
                );
            }
        }
    }
}

fn validate_triangle_normals(
    report: &mut CompileValidationReport,
    subject: Option<String>,
    triangles: &TriangulatedPolygonMesh,
) {
    for (index, normal) in triangles.mesh.normals.iter().enumerate() {
        let length_squared = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
        if !normal.iter().copied().all(f32::is_finite)
            || !length_squared.is_finite()
            || length_squared <= 1.0e-12
        {
            push_issue(
                report,
                subject.clone(),
                "invalid_split_normal",
                format!("Split normal {index} must be finite and non-zero."),
            );
        }
    }
}

fn validate_face_provenance(
    report: &mut CompileValidationReport,
    subject: Option<String>,
    mesh: &PolygonMesh,
) {
    for (index, metadata) in mesh.face_metadata.iter().enumerate() {
        if metadata.part_definition.is_none() || metadata.part_instance.is_none() {
            push_issue(
                report,
                subject.clone(),
                "missing_face_provenance",
                format!("Face {index} is missing part provenance."),
            );
        }
    }
}

fn append_poly_report(
    report: &mut CompileValidationReport,
    subject_prefix: Option<String>,
    poly_report: shape_poly::PolyValidationReport,
) {
    for issue in poly_report.issues {
        let subject = match (&subject_prefix, issue.subject) {
            (Some(prefix), Some(subject)) => Some(format!("{prefix}.{subject}")),
            (Some(prefix), None) => Some(prefix.clone()),
            (None, subject) => subject,
        };
        report.issues.push(CompileValidationIssue {
            subject,
            code: format!("poly_{}", issue.code),
            message: issue.message,
        });
    }
}

fn build_provenance_report(
    parts: &[CompiledPart],
    definition_generation_order: Vec<PartDefinitionId>,
    instance_order: Vec<PartInstanceId>,
) -> ProvenanceReport {
    let mut mapping_counts: BTreeMap<ProvenanceKey, u64> = BTreeMap::new();
    let mut element_counts = BTreeMap::new();
    let mut topology_signatures = BTreeMap::new();

    let mut total_vertices = 0_u64;
    let mut total_faces = 0_u64;
    for part in parts {
        total_vertices = total_vertices.saturating_add(part.world_mesh.positions.len() as u64);
        total_faces = total_faces.saturating_add(part.world_mesh.faces.len() as u64);
        topology_signatures.insert(part.instance_id, part.world_mesh.topology_signature);
        for metadata in &part.world_mesh.face_metadata {
            let key = (
                metadata.part_definition.or(Some(part.definition_id)),
                metadata.part_instance.or(Some(part.instance_id)),
                metadata.region,
                metadata.operation,
            );
            *mapping_counts.entry(key).or_insert(0) += 1;
        }
    }
    element_counts.insert("polygon_vertices".to_owned(), total_vertices);
    element_counts.insert("polygon_faces".to_owned(), total_faces);

    let part_region_operation_mappings = mapping_counts
        .into_iter()
        .map(
            |((part_definition, part_instance, region, operation), polygon_face_count)| {
                ProvenanceMapping {
                    part_definition,
                    part_instance,
                    region,
                    operation,
                    polygon_face_count,
                }
            },
        )
        .collect();

    ProvenanceReport {
        definition_generation_order,
        instance_order,
        part_region_operation_mappings,
        element_counts,
        topology_signatures,
    }
}

fn build_statistics(
    parts: &[CompiledPart],
    combined_preview: &TriangulatedPolygonMesh,
) -> CompileStatistics {
    CompileStatistics {
        part_count: parts.len() as u64,
        polygon_vertex_count: parts
            .iter()
            .map(|part| part.world_mesh.positions.len() as u64)
            .sum(),
        polygon_face_count: parts
            .iter()
            .map(|part| part.world_mesh.faces.len() as u64)
            .sum(),
        triangle_count: (combined_preview.mesh.indices.len() / 3) as u64,
        used_sdf_or_remeshing: false,
    }
}

fn source_recipe_hash(recipe: &AssetRecipe) -> Result<u64, CompileError> {
    let bytes = serde_json::to_vec(recipe)?;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    Ok(hash)
}

fn push_issue(
    report: &mut CompileValidationReport,
    subject: Option<String>,
    code: &'static str,
    message: impl Into<String>,
) {
    report.issues.push(CompileValidationIssue {
        subject,
        code: code.to_owned(),
        message: message.into(),
    });
}

fn obj_group_name(instance_id: PartInstanceId, name: &str) -> String {
    let mut output = format!("part_{:03}_", instance_id.0);
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    while output.ends_with('_') {
        output.pop();
    }
    output
}

#[allow(dead_code)]
fn _element_id_for_docs(_: ElementId) {}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use shape_asset::{
        AssetId, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec, OperationId,
        PartDefinition, PartInstance, Transform3,
    };
    use shape_poly::{FaceMetadata, combine_polygon_meshes, polygon_mesh_from_faces};

    use super::*;

    fn empty_recipe() -> AssetRecipe {
        AssetRecipe::new(AssetId(1), "Empty")
    }

    fn reserved_recipe() -> AssetRecipe {
        let definition = PartDefinition {
            id: PartDefinitionId(1),
            name: "Future".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source: GeometrySource::Plate {
                    size: [1.0, 1.0],
                    thickness: 0.1,
                },
                operations: vec![ModelingOperationSpec::ReservedBoolean {
                    operation: OperationId(1),
                    label: "future".to_owned(),
                }],
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        };
        let instance = PartInstance {
            id: PartInstanceId(1),
            definition: PartDefinitionId(1),
            name: "Future".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let mut recipe = AssetRecipe::new(AssetId(1), "Reserved");
        recipe.definitions.insert(definition.id, definition);
        recipe.instances.insert(instance.id, instance);
        recipe.root_instances.push(PartInstanceId(1));
        recipe.next_ids.part_definition = 2;
        recipe.next_ids.part_instance = 2;
        recipe.next_ids.operation = 2;
        recipe
    }

    fn manual_artifact() -> AssetArtifact {
        let mesh = polygon_mesh_from_faces(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![vec![0, 1, 2]],
            vec![FaceMetadata {
                part_definition: Some(PartDefinitionId(1)),
                part_instance: Some(PartInstanceId(1)),
                ..FaceMetadata::default()
            }],
        )
        .expect("manual mesh should be valid");
        let triangulated = triangulate_polygon_mesh(&mesh).expect("triangulate");
        let part = CompiledPart {
            definition_id: PartDefinitionId(1),
            instance_id: PartInstanceId(1),
            instance_name: "Body".to_owned(),
            prototype_instance_id: None,
            generated_by: None,
            source_recipe_instance: true,
            local_mesh: mesh.clone(),
            world_mesh: mesh,
            triangulated_world: triangulated,
            sockets_world: BTreeMap::new(),
            validation_report: CompileValidationReport::default(),
        };
        let combined = combine_polygon_meshes(std::slice::from_ref(&part.world_mesh))
            .expect("combine should work");
        let combined_preview = triangulate_polygon_mesh(&combined).expect("triangulate");
        let compiled_parts = vec![part];
        let provenance_report = build_provenance_report(
            &compiled_parts,
            vec![PartDefinitionId(1)],
            vec![PartInstanceId(1)],
        );
        let statistics = build_statistics(&compiled_parts, &combined_preview);
        AssetArtifact {
            source_recipe_hash: 42,
            compiled_parts,
            combined_polygon: combined,
            combined_preview,
            provenance_report,
            validation_report: CompileValidationReport::default(),
            statistics,
        }
    }

    #[test]
    fn empty_recipe_compiles_to_empty_artifact() {
        let artifact = compile_asset(&empty_recipe()).expect("empty recipe should compile");

        assert_eq!(artifact.statistics.part_count, 0);
        assert!(artifact.compiled_parts.is_empty());
        assert!(artifact.combined_preview.mesh.indices.is_empty());
        assert!(artifact.validation_report.is_valid());
    }

    #[test]
    fn invalid_recipe_is_rejected_before_modeling() {
        let mut recipe = empty_recipe();
        recipe.title.clear();

        assert!(matches!(
            compile_asset(&recipe),
            Err(CompileError::AssetValidation(_))
        ));
    }

    #[test]
    fn reserved_operations_surface_as_assembly_errors() {
        assert!(matches!(
            compile_asset(&reserved_recipe()),
            Err(CompileError::Assembly(AssemblyError::Modeling(_)))
        ));
    }

    #[test]
    fn provenance_writes_json() {
        let artifact = manual_artifact();

        let json = write_provenance_json(&artifact.provenance_report)
            .expect("provenance should serialize");

        assert!(json.contains("polygon_faces"));
        assert!(json.contains("part_region_operation_mappings"));
    }

    #[test]
    fn grouped_obj_contains_part_group_and_faces() {
        let artifact = manual_artifact();

        let obj = write_grouped_obj(&artifact).expect("obj should serialize");

        assert!(obj.contains("g part_001_body"));
        assert!(obj.contains("f 1//1 2//2 3//3"));
    }

    #[test]
    fn blender_reconstruction_script_contains_parts() {
        let artifact = manual_artifact();

        let script = write_blender_reconstruction_script(&artifact).expect("script");

        assert!(script.contains("PARTS ="));
        assert!(script.contains("shape_lab_instance_id"));
    }
}
