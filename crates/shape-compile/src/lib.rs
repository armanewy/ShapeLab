#![forbid(unsafe_code)]

//! Asset recipe compilation contracts for the explicit polygon production path.
//!
//! The compiler layer validates `AssetRecipe` values, calls the deterministic
//! modeling dispatch, transforms local meshes into asset space, combines preview
//! topology, and records provenance. Heavy production exporters that are not
//! part of Wave 0 return explicit unsupported errors.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetRecipe, OperationId, PartDefinitionId, PartInstanceId, RegionId, SocketId, SocketSpec,
    Transform3, validate_asset_recipe,
};
use shape_modeling::{GeneratedPart, GeneratorContext, ModelingError, generate_geometry};
use shape_poly::{
    PolyError, PolygonMesh, TriangulatedPolygonMesh, combine_polygon_meshes,
    transform_polygon_mesh, triangulate_polygon_mesh,
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

/// Compiled local and world-space mesh for one part instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledPart {
    /// Source part definition.
    pub definition_id: PartDefinitionId,
    /// Source part instance.
    pub instance_id: PartInstanceId,
    /// Local polygon mesh.
    pub local_mesh: PolygonMesh,
    /// World-space polygon mesh.
    pub world_mesh: PolygonMesh,
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
    /// Number of compiled parts.
    pub part_count: u64,
    /// Total world-space polygon vertices.
    pub polygon_vertex_count: u64,
    /// Total world-space polygon faces.
    pub polygon_face_count: u64,
    /// Combined preview triangle count.
    pub triangle_count: u64,
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
    /// Modeling dispatch failed.
    #[error("modeling error: {0}")]
    Modeling(#[from] ModelingError),
    /// Polygon topology helper failed.
    #[error("polygon error: {0}")]
    Polygon(#[from] PolyError),
    /// JSON serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Requested output is intentionally not implemented in Wave 0.
    #[error("unsupported compile feature: {0}")]
    UnsupportedFeature(String),
    /// Text output formatting failed.
    #[error("text formatting failed")]
    Format,
}

/// Compile a complete asset recipe.
pub fn compile_asset(recipe: &AssetRecipe) -> Result<AssetArtifact, CompileError> {
    ensure_valid_recipe(recipe)?;
    let mut compiled_parts = Vec::new();
    for instance in recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
    {
        compiled_parts.push(compile_part(recipe, instance.id)?);
    }
    let world_meshes = compiled_parts
        .iter()
        .map(|part| part.world_mesh.clone())
        .collect::<Vec<_>>();
    let combined_polygon = combine_polygon_meshes(&world_meshes)?;
    let combined_preview = triangulate_polygon_mesh(&combined_polygon)?;
    let provenance_report = build_provenance_report(&compiled_parts);
    let statistics = build_statistics(&compiled_parts, &combined_preview);
    let mut artifact = AssetArtifact {
        source_recipe_hash: source_recipe_hash(recipe)?,
        compiled_parts,
        combined_preview,
        provenance_report,
        validation_report: CompileValidationReport::default(),
        statistics,
    };
    artifact.validation_report = validate_compiled_asset(&artifact);
    Ok(artifact)
}

/// Compile one part instance.
pub fn compile_part(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
) -> Result<CompiledPart, CompileError> {
    ensure_valid_recipe(recipe)?;
    let instance = recipe
        .instances
        .get(&instance_id)
        .ok_or(CompileError::UnknownInstance(instance_id))?;
    let definition = recipe
        .definitions
        .get(&instance.definition)
        .ok_or(CompileError::UnknownDefinition(instance.definition))?;
    let mut context = GeneratorContext::new(
        definition.id,
        instance.id,
        recipe.next_ids.operation,
        recipe.next_ids.revision,
    );
    let generated = generate_geometry(definition, &mut context)?;
    let world_mesh = apply_instance_transform_chain(recipe, instance.id, &generated.mesh)?;
    let sockets_world = transform_sockets(recipe, instance.id, &generated)?;
    Ok(CompiledPart {
        definition_id: definition.id,
        instance_id: instance.id,
        local_mesh: generated.mesh,
        world_mesh,
        sockets_world,
        validation_report: CompileValidationReport::default(),
    })
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
    for part in &artifact.compiled_parts {
        if !part.validation_report.is_valid() {
            push_issue(
                &mut report,
                Some(format!("part.{}", part.instance_id.0)),
                "compiled_part_invalid",
                "Compiled part contains validation issues.",
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
        writeln!(&mut output, "g part_{}", part.instance_id.0).map_err(|_| CompileError::Format)?;
        let triangles = triangulate_polygon_mesh(&part.world_mesh)?;
        for position in &triangles.mesh.positions {
            writeln!(
                &mut output,
                "v {:.9} {:.9} {:.9}",
                position[0], position[1], position[2]
            )
            .map_err(|_| CompileError::Format)?;
        }
        for normal in &triangles.mesh.normals {
            writeln!(
                &mut output,
                "vn {:.9} {:.9} {:.9}",
                normal[0], normal[1], normal[2]
            )
            .map_err(|_| CompileError::Format)?;
        }
        for triangle in triangles.mesh.indices.chunks_exact(3) {
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
        let add =
            u32::try_from(triangles.mesh.positions.len()).map_err(|_| CompileError::Format)?;
        vertex_offset = vertex_offset.checked_add(add).ok_or(CompileError::Format)?;
    }
    Ok(output)
}

/// Write provenance as pretty JSON.
pub fn write_provenance_json(report: &ProvenanceReport) -> Result<String, CompileError> {
    serde_json::to_string_pretty(report).map_err(CompileError::Json)
}

/// Stub Blender reconstruction script writer.
pub fn write_blender_reconstruction_script(
    _artifact: &AssetArtifact,
) -> Result<String, CompileError> {
    Err(CompileError::UnsupportedFeature(
        "Blender reconstruction scripts are outside the Wave 0 explicit modeling contracts"
            .to_owned(),
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

fn apply_instance_transform_chain(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
    mesh: &PolygonMesh,
) -> Result<PolygonMesh, CompileError> {
    let mut world = mesh.clone();
    for transform in transform_chain(recipe, instance_id)? {
        world = transform_polygon_mesh(&world, &transform)?;
    }
    Ok(world)
}

fn transform_sockets(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
    generated: &GeneratedPart,
) -> Result<BTreeMap<SocketId, SocketSpec>, CompileError> {
    let transforms = transform_chain(recipe, instance_id)?;
    let mut sockets = generated.sockets.clone();
    for socket in sockets.values_mut() {
        let mut frame = socket.local_frame.clone();
        for transform in &transforms {
            frame = frame.transformed_by(transform);
        }
        socket.local_frame = frame;
    }
    Ok(sockets)
}

fn transform_chain(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
) -> Result<Vec<Transform3>, CompileError> {
    let mut chain = Vec::new();
    let mut cursor = Some(instance_id);
    while let Some(current) = cursor {
        let instance = recipe
            .instances
            .get(&current)
            .ok_or(CompileError::UnknownInstance(current))?;
        chain.push(instance.local_transform.clone());
        cursor = instance.parent;
    }
    chain.reverse();
    Ok(chain)
}

fn build_provenance_report(parts: &[CompiledPart]) -> ProvenanceReport {
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use shape_asset::{
        AssetId, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec, OperationId,
        PartDefinition, PartInstance, Transform3,
    };
    use shape_poly::{FaceMetadata, polygon_mesh_from_faces};

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
        let part = CompiledPart {
            definition_id: PartDefinitionId(1),
            instance_id: PartInstanceId(1),
            local_mesh: mesh.clone(),
            world_mesh: mesh,
            sockets_world: BTreeMap::new(),
            validation_report: CompileValidationReport::default(),
        };
        let combined = combine_polygon_meshes(std::slice::from_ref(&part.world_mesh))
            .expect("combine should work");
        let combined_preview = triangulate_polygon_mesh(&combined).expect("triangulate");
        let compiled_parts = vec![part];
        let provenance_report = build_provenance_report(&compiled_parts);
        let statistics = build_statistics(&compiled_parts, &combined_preview);
        AssetArtifact {
            source_recipe_hash: 42,
            compiled_parts,
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
    fn reserved_operations_surface_as_modeling_errors() {
        assert!(matches!(
            compile_asset(&reserved_recipe()),
            Err(CompileError::Modeling(
                ModelingError::UnsupportedOperation {
                    operation: OperationId(1),
                    ..
                }
            ))
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

        assert!(obj.contains("g part_1"));
        assert!(obj.contains("f 1//1 2//2 3//3"));
    }

    #[test]
    fn blender_reconstruction_script_is_explicitly_unsupported() {
        let artifact = manual_artifact();

        assert!(matches!(
            write_blender_reconstruction_script(&artifact),
            Err(CompileError::UnsupportedFeature(_))
        ));
    }
}
