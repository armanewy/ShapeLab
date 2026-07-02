//! Deterministic export helpers for compiled explicit asset artifacts.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use orchard_asset::{
    AssetRecipe, ExportRealizationPolicy, RegionId, RelationshipContract, RelationshipId,
    RelationshipType,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AssetArtifact, CompiledPart};

mod blender;
mod dcc;
mod obj;
mod package;

pub use blender::blender_reconstruction_script;
pub use dcc::{
    DCC_ADAPTER_MANIFEST_FILE, DCC_ADAPTER_SCHEMA_VERSION, DCC_REBUILD_SCRIPT_FILE,
    DCC_VERIFICATION_FILE, DccAdapterFiles, DccAdapterManifest, DccAdapterOptions,
    DccAdapterVerificationReport, DccCollection, DccMetadataField, DccSemanticPart,
    DccSourceOfTruth, DccVariantControl, dcc_adapter_manifest, dcc_adapter_verification_report,
    dcc_rebuild_script,
};
pub use obj::{
    GroupedObjExport, GroupedObjObjectReport, GroupedObjReport, write_grouped_obj_export,
};
pub use package::{
    AssetManifest, CanonicalPartMesh, ModelExportPackage, ModelExportPackageFiles,
    ModelExportPackagePaths, ModelExportValidationReport, ModelExportVerificationReport,
    PartManifest, PartRegionManifest, decode_part_meshbin, encode_part_meshbin, read_model_package,
    read_part_meshbin, verify_model_package, write_model_package,
    write_model_package_with_dcc_options, write_part_meshbin,
};

/// Schema version for the explicit model export package.
pub const MODEL_EXPORT_SCHEMA_VERSION: u32 = 1;
/// Top-level package manifest.
pub const ASSET_MANIFEST_FILE: &str = "asset-manifest.json";
/// Serialized source recipe.
pub const RECIPE_FILE: &str = "recipe.json";
/// Serialized compile provenance report.
pub const PROVENANCE_FILE: &str = "provenance.json";
/// Package validation report.
pub const VALIDATION_FILE: &str = "validation.json";
/// Blender reconstruction helper.
pub const BLENDER_RECONSTRUCT_FILE: &str = "blender_reconstruct.py";
/// Directory containing canonical part mesh payloads.
pub const PARTS_DIR: &str = "parts";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Aggregate exact counts emitted by model export.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportCounts {
    /// Number of exported part instances.
    pub part_count: u64,
    /// Number of canonical polygon vertices.
    pub vertex_count: u64,
    /// Number of canonical polygon faces.
    pub polygon_face_count: u64,
    /// Number of polygon loop indices.
    pub polygon_index_count: u64,
    /// Number of split-normal loop entries.
    pub split_normal_count: u64,
}

/// How an authored relationship's child appeared in the exported geometry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipChildOutput {
    /// The child is preserved as its own exported node.
    PreservedNode,
    /// The child is preserved as a distinct submesh.
    PreservedSubmesh,
    /// The child is part of the current combined geometry mesh.
    CombinedMesh,
    /// The child was baked into a union by an evidence-backed exporter.
    BakedUnion,
}

/// Truthful export realization summary for one authored relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipRealizationSummary {
    /// Authored relationship ID.
    pub relationship_id: RelationshipId,
    /// Authored relationship semantic kind.
    pub relationship_type: RelationshipType,
    /// Requested or authored export realization policy.
    pub realization_policy: ExportRealizationPolicy,
    /// Exported child node reference, if the child is preserved as a node.
    pub output_node: Option<String>,
    /// Exported mesh reference, if geometry is present in a mesh.
    pub output_mesh: Option<String>,
    /// Actual V0 child output shape.
    pub child_output: RelationshipChildOutput,
    /// Whether this export actually baked the relationship into geometry.
    pub baked: bool,
    /// Whether relationship semantics remain available in report/sidecar data.
    pub semantics_preserved_in_sidecar: bool,
}

impl ExportCounts {
    pub(crate) fn add_part(&mut self, part: &CompiledPart) {
        self.part_count += 1;
        self.vertex_count += part.world_mesh.positions.len() as u64;
        self.polygon_face_count += part.world_mesh.faces.len() as u64;
        self.polygon_index_count += part
            .world_mesh
            .faces
            .iter()
            .map(|face| face.vertices.len() as u64)
            .sum::<u64>();
        self.split_normal_count += part
            .world_mesh
            .faces
            .iter()
            .map(|face| face.vertices.len() as u64)
            .sum::<u64>();
    }
}

/// Errors produced by deterministic model export and package verification.
#[derive(Debug, Error)]
pub enum ExportError {
    /// Filesystem operation failed.
    #[error("path IO error at {path}: {source}")]
    PathIo {
        /// Path being accessed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// JSON serialization or parsing failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Package contents are inconsistent or unsafe.
    #[error("invalid package at {path}: {message}")]
    InvalidPackage {
        /// File or directory that failed validation.
        path: PathBuf,
        /// Explanation.
        message: String,
    },
    /// The compiled artifact cannot be represented in the export format.
    #[error("invalid export artifact: {0}")]
    InvalidArtifact(String),
    /// Text output formatting failed.
    #[error("text formatting failed")]
    Format,
}

/// Return compiled parts in stable export order.
#[must_use]
pub fn ordered_parts(artifact: &AssetArtifact) -> Vec<&CompiledPart> {
    let mut parts = artifact.compiled_parts.iter().collect::<Vec<_>>();
    parts.sort_by(|left, right| {
        left.instance_id
            .cmp(&right.instance_id)
            .then(left.definition_id.cmp(&right.definition_id))
            .then(left.instance_name.cmp(&right.instance_name))
    });
    parts
}

/// Return exact package counts for a compiled artifact.
#[must_use]
pub fn export_counts(artifact: &AssetArtifact) -> ExportCounts {
    let mut counts = ExportCounts::default();
    for part in ordered_parts(artifact) {
        counts.add_part(part);
    }
    counts
}

/// Summarize how relationship contracts are realized by V0 geometry export.
///
/// Geometry export V0 writes one geometry-only GLB. It may report authored
/// relationships, but it does not claim node hierarchy, submesh separation,
/// collision, or baked union output unless a later exporter proves that path.
#[must_use]
pub fn relationship_realization_summaries_for_geometry_export(
    relationships: &[RelationshipContract],
    output_mesh: &str,
) -> Vec<RelationshipRealizationSummary> {
    relationships
        .iter()
        .map(|relationship| RelationshipRealizationSummary {
            relationship_id: relationship.id,
            relationship_type: relationship.relationship_type.clone(),
            realization_policy: relationship.export_realization.clone(),
            output_node: None,
            output_mesh: Some(output_mesh.to_owned()),
            child_output: RelationshipChildOutput::CombinedMesh,
            baked: false,
            semantics_preserved_in_sidecar: true,
        })
        .collect()
}

/// Stable non-cryptographic hash used for package checksums and recipe identity.
pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Stable recipe hash matching the compile artifact's source hash contract.
pub fn recipe_hash(recipe: &AssetRecipe) -> Result<u64, ExportError> {
    Ok(fnv64(&serde_json::to_vec(recipe)?))
}

pub(crate) fn checksum_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv64(bytes))
}

pub(crate) fn safe_part_name(part: &CompiledPart) -> String {
    let mut output = format!("part_{:03}_", part.instance_id.0);
    for character in part.instance_name.chars() {
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

pub(crate) fn part_mesh_path(part: &CompiledPart) -> String {
    format!("{PARTS_DIR}/part-{:03}.meshbin", part.instance_id.0)
}

pub(crate) fn package_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

pub(crate) fn validate_package_relative_path(
    relative: &str,
    context: &Path,
) -> Result<(), ExportError> {
    let relative_path = Path::new(relative);
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid_package(
            context,
            format!("unsafe package-relative path '{relative}'"),
        ));
    }
    Ok(())
}

pub(crate) fn resolve_package_asset(root: &Path, relative: &str) -> Result<PathBuf, ExportError> {
    validate_package_relative_path(relative, root)?;
    let canonical_root = std::fs::canonicalize(root).map_err(|source| path_io(root, source))?;
    let joined = package_path(root, relative);
    let canonical_path =
        std::fs::canonicalize(&joined).map_err(|source| path_io(&joined, source))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(invalid_package(
            &joined,
            format!("package asset '{relative}' resolves outside the package root"),
        ));
    }
    Ok(canonical_path)
}

pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), ExportError> {
    let json = serde_json::to_string_pretty(value)?;
    write_text(path, &json)
}

pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), ExportError> {
    std::fs::write(path, text).map_err(|source| path_io(path, source))
}

pub(crate) fn path_io(path: &Path, source: std::io::Error) -> ExportError {
    ExportError::PathIo {
        path: path.to_path_buf(),
        source,
    }
}

pub(crate) fn invalid_package(path: impl AsRef<Path>, message: impl Into<String>) -> ExportError {
    ExportError::InvalidPackage {
        path: path.as_ref().to_path_buf(),
        message: message.into(),
    }
}

pub(crate) fn part_parent_instance(recipe: &AssetRecipe, part: &CompiledPart) -> Option<u64> {
    let authored_instance = part.prototype_instance_id.unwrap_or(part.instance_id);
    recipe
        .instances
        .get(&authored_instance)
        .and_then(|instance| instance.parent)
        .map(|parent| parent.0)
}

pub(crate) fn part_pivot_origin(recipe: &AssetRecipe, part: &CompiledPart) -> [f32; 3] {
    let Some(definition) = recipe.definitions.get(&part.definition_id) else {
        return [0.0, 0.0, 0.0];
    };
    let mut point = definition.local_pivot.origin;
    let mut current = Some(part.prototype_instance_id.unwrap_or(part.instance_id));
    while let Some(instance_id) = current {
        let Some(instance) = recipe.instances.get(&instance_id) else {
            break;
        };
        point = instance.local_transform.transform_point(point);
        current = instance.parent;
    }
    point
}

pub(crate) fn part_regions(
    recipe: &AssetRecipe,
    part: &CompiledPart,
) -> Vec<(RegionId, String, u64)> {
    let mut counts = BTreeMap::<RegionId, u64>::new();
    for metadata in &part.world_mesh.face_metadata {
        if let Some(region) = metadata.region {
            *counts.entry(region).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(region, face_count)| {
            let name = recipe
                .definitions
                .get(&part.definition_id)
                .and_then(|definition| definition.regions.get(&region))
                .map(|region| region.name.clone())
                .unwrap_or_else(|| format!("region_{}", region.0));
            (region, name, face_count)
        })
        .collect()
}
