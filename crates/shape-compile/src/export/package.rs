use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use shape_asset::{AssetRecipe, OperationId, PartDefinitionId, PartInstanceId, RegionId};
use shape_poly::ElementId;

use crate::{
    AssetArtifact, CompileValidationIssue, CompiledPart, ProvenanceReport,
    validation::{ValidationIssue, validate_model, validation_config_from_recipe},
};

use super::{
    ASSET_MANIFEST_FILE, BLENDER_RECONSTRUCT_FILE, ExportCounts, ExportError,
    MODEL_EXPORT_SCHEMA_VERSION, PARTS_DIR, PROVENANCE_FILE, RECIPE_FILE, VALIDATION_FILE,
    blender_reconstruction_script, checksum_hex, export_counts, invalid_package, package_path,
    part_mesh_path, part_parent_instance, part_pivot_origin, part_regions, path_io, recipe_hash,
    resolve_package_asset, safe_part_name, validate_package_relative_path, write_json, write_text,
};

const PART_MESHBIN_MAGIC: &[u8] = b"SLABPARTMESH1\0";
const PART_MESHBIN_VERSION: u32 = 1;
const NONE_U64: u64 = u64::MAX;

/// Top-level manifest for an explicit model export package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Package schema version.
    pub schema_version: u32,
    /// Human-readable generator label.
    pub generator: String,
    /// Stable hash of `recipe.json`.
    pub source_recipe_hash: u64,
    /// Source recipe title.
    pub recipe_title: String,
    /// Binary numeric encoding.
    pub numeric_format: String,
    /// Canonical sidecar files.
    pub files: ModelExportPackageFiles,
    /// Exact aggregate counts.
    pub counts: ExportCounts,
    /// Part mesh entries in deterministic order.
    pub parts: Vec<PartManifest>,
}

/// Canonical package sidecar paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelExportPackageFiles {
    /// Source recipe JSON.
    pub recipe: String,
    /// Provenance sidecar JSON.
    pub provenance: String,
    /// Validation sidecar JSON.
    pub validation: String,
    /// Blender reconstruction script.
    pub blender_reconstruct: String,
}

impl Default for ModelExportPackageFiles {
    fn default() -> Self {
        Self {
            recipe: RECIPE_FILE.to_owned(),
            provenance: PROVENANCE_FILE.to_owned(),
            validation: VALIDATION_FILE.to_owned(),
            blender_reconstruct: BLENDER_RECONSTRUCT_FILE.to_owned(),
        }
    }
}

/// One canonical part mesh entry in `asset-manifest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartManifest {
    /// Stable package-local part label.
    pub part_id: String,
    /// Package-relative meshbin path.
    pub mesh: String,
    /// Stable source instance ID.
    pub instance_id: u64,
    /// Stable source definition ID.
    pub definition_id: u64,
    /// Stable object name.
    pub object_name: String,
    /// Source recipe parent instance, when authored.
    pub parent_instance_id: Option<u64>,
    /// Source prototype for generated occurrences.
    pub prototype_instance_id: Option<u64>,
    /// Operation that generated this occurrence.
    pub generated_by: Option<u64>,
    /// Whether this occurrence came directly from the source recipe.
    pub source_recipe_instance: bool,
    /// Best-effort recipe pivot origin in asset coordinates.
    pub pivot_origin: [f32; 3],
    /// Deterministic topology signature from the polygon mesh.
    pub topology_signature: u64,
    /// Exact counts for this part.
    pub counts: ExportCounts,
    /// Semantic regions present in this part.
    pub regions: Vec<PartRegionManifest>,
    /// FNV-1a checksum of the meshbin payload as 16 lowercase hex digits.
    pub checksum_fnv64: String,
}

/// Region metadata preserved in the package manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartRegionManifest {
    /// Stable source region ID.
    pub id: u64,
    /// Human-readable region name.
    pub name: String,
    /// Number of polygon faces assigned to this region.
    pub polygon_face_count: u64,
}

/// Exact validation sidecar for an export package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelExportValidationReport {
    /// Package schema version.
    pub schema_version: u32,
    /// Stable hash of `recipe.json`.
    pub source_recipe_hash: u64,
    /// Exact aggregate counts.
    pub counts: ExportCounts,
    /// Compile validation issues carried into the package.
    pub compile_issues: Vec<CompileValidationIssue>,
    /// Recipe-derived model validation issues carried into the package.
    pub model_issues: Vec<ValidationIssue>,
}

/// Verification result returned by package readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelExportVerificationReport {
    /// Package schema version.
    pub schema_version: u32,
    /// Stable hash of `recipe.json`.
    pub source_recipe_hash: u64,
    /// Exact aggregate counts decoded from meshbins.
    pub counts: ExportCounts,
    /// Whether all part payload checksums matched the manifest.
    pub checksums_match: bool,
    /// Whether decoded topology counts matched the manifest.
    pub topology_matches_manifest: bool,
    /// Whether all decoded positions and normals were finite.
    pub finite_numeric_payloads: bool,
}

/// Paths written by [`write_model_package`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelExportPackagePaths {
    /// Package root directory.
    pub directory: PathBuf,
    /// Top-level manifest.
    pub manifest: PathBuf,
    /// Source recipe JSON.
    pub recipe: PathBuf,
    /// Provenance sidecar JSON.
    pub provenance: PathBuf,
    /// Validation sidecar JSON.
    pub validation: PathBuf,
    /// Blender reconstruction script.
    pub blender_reconstruct: PathBuf,
    /// Part meshbin paths.
    pub parts: Vec<PathBuf>,
}

/// In-memory package decoded from disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelExportPackage {
    /// Top-level manifest.
    pub manifest: AssetManifest,
    /// Source recipe.
    pub recipe: AssetRecipe,
    /// Compile provenance report.
    pub provenance: ProvenanceReport,
    /// Validation sidecar.
    pub validation: ModelExportValidationReport,
    /// Decoded canonical part meshes.
    pub parts: Vec<CanonicalPartMesh>,
}

/// Canonical binary payload for one part mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalPartMesh {
    /// Source part definition.
    pub definition_id: PartDefinitionId,
    /// Source part instance.
    pub instance_id: PartInstanceId,
    /// Source prototype for generated occurrences.
    pub prototype_instance_id: Option<PartInstanceId>,
    /// Operation that generated this occurrence.
    pub generated_by: Option<OperationId>,
    /// Whether this occurrence came directly from the source recipe.
    pub source_recipe_instance: bool,
    /// Canonical polygon positions.
    pub positions: Vec<[f32; 3]>,
    /// Stable vertex element IDs.
    pub vertex_element_ids: Vec<ElementId>,
    /// Number of vertices in each polygon face.
    pub polygon_face_counts: Vec<u32>,
    /// Flattened polygon indices.
    pub polygon_indices: Vec<u32>,
    /// One split normal per polygon loop.
    pub loop_normals: Vec<[f32; 3]>,
    /// Source region ID per polygon face.
    pub face_region_ids: Vec<Option<RegionId>>,
    /// Stable face element IDs.
    pub face_element_ids: Vec<ElementId>,
}

impl CanonicalPartMesh {
    /// Build the canonical meshbin payload from a compiled part.
    pub fn from_compiled_part(part: &CompiledPart) -> Result<Self, ExportError> {
        let mut polygon_face_counts = Vec::with_capacity(part.world_mesh.faces.len());
        let mut polygon_indices = Vec::new();
        let mut face_element_ids = Vec::with_capacity(part.world_mesh.faces.len());
        for face in &part.world_mesh.faces {
            let count = u32::try_from(face.vertices.len()).map_err(|_| {
                ExportError::InvalidArtifact("polygon face has too many vertices".to_owned())
            })?;
            polygon_face_counts.push(count);
            polygon_indices.extend(face.vertices.iter().copied());
            face_element_ids.push(face.id);
        }

        let face_region_ids = part
            .world_mesh
            .face_metadata
            .iter()
            .map(|metadata| metadata.region)
            .collect::<Vec<_>>();
        if face_region_ids.len() != polygon_face_counts.len() {
            return Err(ExportError::InvalidArtifact(format!(
                "part {} face metadata count does not match face count",
                part.instance_id.0
            )));
        }

        Ok(Self {
            definition_id: part.definition_id,
            instance_id: part.instance_id,
            prototype_instance_id: part.prototype_instance_id,
            generated_by: part.generated_by,
            source_recipe_instance: part.source_recipe_instance,
            positions: part.world_mesh.positions.clone(),
            vertex_element_ids: part.world_mesh.vertex_ids.clone(),
            polygon_face_counts,
            polygon_indices,
            loop_normals: loop_normals_for_part(part)?,
            face_region_ids,
            face_element_ids,
        })
    }

    /// Return exact counts for this canonical mesh.
    #[must_use]
    pub fn counts(&self) -> ExportCounts {
        ExportCounts {
            part_count: 1,
            vertex_count: self.positions.len() as u64,
            polygon_face_count: self.polygon_face_counts.len() as u64,
            polygon_index_count: self.polygon_indices.len() as u64,
            split_normal_count: self.loop_normals.len() as u64,
        }
    }

    fn validate(&self, path: &Path) -> Result<(), ExportError> {
        if self.positions.is_empty() {
            return Err(invalid_package(path, "part mesh contains no vertices"));
        }
        if self.polygon_face_counts.is_empty() {
            return Err(invalid_package(path, "part mesh contains no faces"));
        }
        if self.positions.len() != self.vertex_element_ids.len() {
            return Err(invalid_package(
                path,
                "vertex element ID count does not match vertex count",
            ));
        }
        if self.polygon_face_counts.len() != self.face_region_ids.len()
            || self.polygon_face_counts.len() != self.face_element_ids.len()
        {
            return Err(invalid_package(
                path,
                "face semantic metadata counts do not match face count",
            ));
        }
        let expected_indices = self
            .polygon_face_counts
            .iter()
            .try_fold(0_usize, |sum, count| {
                if *count < 3 {
                    return None;
                }
                sum.checked_add(*count as usize)
            })
            .ok_or_else(|| invalid_package(path, "polygon index count overflow"))?;
        if expected_indices != self.polygon_indices.len() {
            return Err(invalid_package(
                path,
                "polygon face counts do not match polygon index count",
            ));
        }
        if self.loop_normals.len() != self.polygon_indices.len() {
            return Err(invalid_package(
                path,
                "split-normal count does not match polygon loop count",
            ));
        }
        for position in &self.positions {
            if !array_is_finite(*position) {
                return Err(invalid_package(
                    path,
                    "part mesh contains a non-finite position",
                ));
            }
        }
        for normal in &self.loop_normals {
            if !array_is_finite(*normal) {
                return Err(invalid_package(
                    path,
                    "part mesh contains a non-finite normal",
                ));
            }
        }
        for index in &self.polygon_indices {
            if *index as usize >= self.positions.len() {
                return Err(invalid_package(
                    path,
                    "part mesh contains an out-of-range polygon index",
                ));
            }
        }
        Ok(())
    }
}

/// Write an explicit model package directory.
pub fn write_model_package(
    recipe: &AssetRecipe,
    artifact: &AssetArtifact,
    out_dir: impl AsRef<Path>,
) -> Result<ModelExportPackagePaths, ExportError> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir).map_err(|source| path_io(out_dir, source))?;
    let parts_dir = package_path(out_dir, PARTS_DIR);
    fs::create_dir_all(&parts_dir).map_err(|source| path_io(&parts_dir, source))?;

    let mut part_manifests = Vec::new();
    let mut part_paths = Vec::new();
    for part in super::ordered_parts(artifact) {
        let mesh = CanonicalPartMesh::from_compiled_part(part)?;
        let bytes = encode_part_meshbin(&mesh)?;
        let mesh_relative = part_mesh_path(part);
        let mesh_path = package_path(out_dir, &mesh_relative);
        fs::write(&mesh_path, &bytes).map_err(|source| path_io(&mesh_path, source))?;
        let counts = mesh.counts();
        part_manifests.push(PartManifest {
            part_id: format!("part-{:03}", part.instance_id.0),
            mesh: mesh_relative,
            instance_id: part.instance_id.0,
            definition_id: part.definition_id.0,
            object_name: safe_part_name(part),
            parent_instance_id: part_parent_instance(recipe, part),
            prototype_instance_id: part.prototype_instance_id.map(|id| id.0),
            generated_by: part.generated_by.map(|id| id.0),
            source_recipe_instance: part.source_recipe_instance,
            pivot_origin: part_pivot_origin(recipe, part),
            topology_signature: part.world_mesh.topology_signature,
            counts,
            regions: part_regions(recipe, part)
                .into_iter()
                .map(|(region, name, polygon_face_count)| PartRegionManifest {
                    id: region.0,
                    name,
                    polygon_face_count,
                })
                .collect(),
            checksum_fnv64: checksum_hex(&bytes),
        });
        part_paths.push(mesh_path);
    }

    let source_recipe_hash = recipe_hash(recipe)?;
    if source_recipe_hash != artifact.source_recipe_hash {
        return Err(ExportError::InvalidArtifact(
            "recipe hash does not match compiled artifact".to_owned(),
        ));
    }
    let validation = model_export_validation_report(recipe, artifact);
    let manifest = AssetManifest {
        schema_version: MODEL_EXPORT_SCHEMA_VERSION,
        generator: "shape-compile model export".to_owned(),
        source_recipe_hash,
        recipe_title: recipe.title.clone(),
        numeric_format: "little-endian f32/u32/u64".to_owned(),
        files: ModelExportPackageFiles::default(),
        counts: export_counts(artifact),
        parts: part_manifests,
    };

    let recipe_path = package_path(out_dir, RECIPE_FILE);
    let provenance_path = package_path(out_dir, PROVENANCE_FILE);
    let validation_path = package_path(out_dir, VALIDATION_FILE);
    let blender_path = package_path(out_dir, BLENDER_RECONSTRUCT_FILE);
    let manifest_path = package_path(out_dir, ASSET_MANIFEST_FILE);
    write_json(&recipe_path, recipe)?;
    write_json(&provenance_path, &artifact.provenance_report)?;
    write_json(&validation_path, &validation)?;
    write_text(&blender_path, blender_reconstruction_script())?;
    write_json(&manifest_path, &manifest)?;

    Ok(ModelExportPackagePaths {
        directory: out_dir.to_path_buf(),
        manifest: manifest_path,
        recipe: recipe_path,
        provenance: provenance_path,
        validation: validation_path,
        blender_reconstruct: blender_path,
        parts: part_paths,
    })
}

/// Read and verify an explicit model package.
pub fn read_model_package(
    package_dir: impl AsRef<Path>,
) -> Result<ModelExportPackage, ExportError> {
    let package_dir = package_dir.as_ref();
    let manifest_path = resolve_package_asset(package_dir, ASSET_MANIFEST_FILE)?;
    let manifest: AssetManifest = read_json(&manifest_path)?;
    validate_manifest(&manifest, &manifest_path)?;

    let recipe_path = resolve_package_asset(package_dir, &manifest.files.recipe)?;
    let provenance_path = resolve_package_asset(package_dir, &manifest.files.provenance)?;
    let validation_path = resolve_package_asset(package_dir, &manifest.files.validation)?;
    let _blender_path = resolve_package_asset(package_dir, &manifest.files.blender_reconstruct)?;

    let recipe: AssetRecipe = read_json(&recipe_path)?;
    let provenance: ProvenanceReport = read_json(&provenance_path)?;
    let validation: ModelExportValidationReport = read_json(&validation_path)?;
    let source_recipe_hash = recipe_hash(&recipe)?;
    if manifest.source_recipe_hash != source_recipe_hash {
        return Err(invalid_package(
            &recipe_path,
            "recipe hash does not match asset-manifest.json",
        ));
    }
    if validation.schema_version != manifest.schema_version
        || validation.source_recipe_hash != manifest.source_recipe_hash
    {
        return Err(invalid_package(
            &validation_path,
            "validation.json does not match asset-manifest.json",
        ));
    }

    let mut parts = Vec::with_capacity(manifest.parts.len());
    let mut counts = ExportCounts::default();
    for part_manifest in &manifest.parts {
        validate_package_relative_path(&part_manifest.mesh, &manifest_path)?;
        let mesh_path = resolve_package_asset(package_dir, &part_manifest.mesh)?;
        let bytes = fs::read(&mesh_path).map_err(|source| path_io(&mesh_path, source))?;
        let checksum = checksum_hex(&bytes);
        if checksum != part_manifest.checksum_fnv64 {
            return Err(invalid_package(
                &mesh_path,
                "part meshbin checksum does not match asset-manifest.json",
            ));
        }
        let part = decode_part_meshbin(&bytes, &mesh_path)?;
        validate_part_manifest(part_manifest, &part, &mesh_path)?;
        add_counts(&mut counts, part.counts());
        parts.push(part);
    }

    if counts != manifest.counts {
        return Err(invalid_package(
            &manifest_path,
            "decoded mesh counts do not match asset-manifest.json",
        ));
    }
    if validation.counts != manifest.counts {
        return Err(invalid_package(
            &validation_path,
            "validation.json counts do not match asset-manifest.json",
        ));
    }

    Ok(ModelExportPackage {
        manifest,
        recipe,
        provenance,
        validation,
        parts,
    })
}

/// Verify an explicit model package and return exact decoded counts.
pub fn verify_model_package(
    package_dir: impl AsRef<Path>,
) -> Result<ModelExportVerificationReport, ExportError> {
    let package = read_model_package(package_dir)?;
    Ok(ModelExportVerificationReport {
        schema_version: package.manifest.schema_version,
        source_recipe_hash: package.manifest.source_recipe_hash,
        counts: package.manifest.counts,
        checksums_match: true,
        topology_matches_manifest: true,
        finite_numeric_payloads: true,
    })
}

/// Write one canonical part meshbin file.
pub fn write_part_meshbin(
    path: impl AsRef<Path>,
    mesh: &CanonicalPartMesh,
) -> Result<(), ExportError> {
    let path = path.as_ref();
    let bytes = encode_part_meshbin(mesh)?;
    fs::write(path, bytes).map_err(|source| path_io(path, source))
}

/// Read one canonical part meshbin file.
pub fn read_part_meshbin(path: impl AsRef<Path>) -> Result<CanonicalPartMesh, ExportError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| path_io(path, source))?;
    decode_part_meshbin(&bytes, path)
}

/// Encode one canonical part mesh into deterministic meshbin bytes.
pub fn encode_part_meshbin(mesh: &CanonicalPartMesh) -> Result<Vec<u8>, ExportError> {
    mesh.validate(Path::new("part.meshbin"))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PART_MESHBIN_MAGIC);
    write_u32(&mut bytes, PART_MESHBIN_VERSION);
    write_u64(&mut bytes, mesh.definition_id.0);
    write_u64(&mut bytes, mesh.instance_id.0);
    write_u64(
        &mut bytes,
        mesh.prototype_instance_id
            .map(|id| id.0)
            .unwrap_or(NONE_U64),
    );
    write_u64(
        &mut bytes,
        mesh.generated_by.map(|id| id.0).unwrap_or(NONE_U64),
    );
    write_u8(&mut bytes, u8::from(mesh.source_recipe_instance));
    write_u64(&mut bytes, mesh.positions.len() as u64);
    write_u64(&mut bytes, mesh.polygon_face_counts.len() as u64);
    write_u64(&mut bytes, mesh.polygon_indices.len() as u64);
    write_u64(&mut bytes, mesh.loop_normals.len() as u64);

    for position in &mesh.positions {
        for component in position {
            write_f32(&mut bytes, *component);
        }
    }
    for id in &mesh.vertex_element_ids {
        write_u64(&mut bytes, id.0);
    }
    for count in &mesh.polygon_face_counts {
        write_u32(&mut bytes, *count);
    }
    for index in &mesh.polygon_indices {
        write_u32(&mut bytes, *index);
    }
    for normal in &mesh.loop_normals {
        for component in normal {
            write_f32(&mut bytes, *component);
        }
    }
    for region in &mesh.face_region_ids {
        write_u64(&mut bytes, region.map(|id| id.0).unwrap_or(NONE_U64));
    }
    for id in &mesh.face_element_ids {
        write_u64(&mut bytes, id.0);
    }
    Ok(bytes)
}

/// Decode deterministic meshbin bytes for one canonical part mesh.
pub fn decode_part_meshbin(
    bytes: &[u8],
    path: impl AsRef<Path>,
) -> Result<CanonicalPartMesh, ExportError> {
    let path = path.as_ref();
    if bytes.len() < PART_MESHBIN_MAGIC.len() + 69 {
        return Err(invalid_package(path, "part meshbin header is truncated"));
    }
    if &bytes[..PART_MESHBIN_MAGIC.len()] != PART_MESHBIN_MAGIC {
        return Err(invalid_package(path, "unsupported part meshbin magic"));
    }
    let mut offset = PART_MESHBIN_MAGIC.len();
    let version = read_u32(bytes, &mut offset, path)?;
    if version != PART_MESHBIN_VERSION {
        return Err(invalid_package(
            path,
            format!("unsupported part meshbin version {version}"),
        ));
    }
    let definition_id = PartDefinitionId(read_u64(bytes, &mut offset, path)?);
    let instance_id = PartInstanceId(read_u64(bytes, &mut offset, path)?);
    let prototype_instance_id = option_part_instance(read_u64(bytes, &mut offset, path)?);
    let generated_by = option_operation(read_u64(bytes, &mut offset, path)?);
    let source_recipe_instance = match read_u8(bytes, &mut offset, path)? {
        0 => false,
        1 => true,
        value => {
            return Err(invalid_package(
                path,
                format!("invalid source_recipe_instance flag {value}"),
            ));
        }
    };
    let vertex_count = read_count(bytes, &mut offset, path, "vertex count")?;
    let face_count = read_count(bytes, &mut offset, path, "face count")?;
    let polygon_index_count = read_count(bytes, &mut offset, path, "polygon index count")?;
    let split_normal_count = read_count(bytes, &mut offset, path, "split-normal count")?;

    let mut positions = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        positions.push([
            read_f32(bytes, &mut offset, path)?,
            read_f32(bytes, &mut offset, path)?,
            read_f32(bytes, &mut offset, path)?,
        ]);
    }
    let mut vertex_element_ids = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        vertex_element_ids.push(ElementId(read_u64(bytes, &mut offset, path)?));
    }
    let mut polygon_face_counts = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        polygon_face_counts.push(read_u32(bytes, &mut offset, path)?);
    }
    let mut polygon_indices = Vec::with_capacity(polygon_index_count);
    for _ in 0..polygon_index_count {
        polygon_indices.push(read_u32(bytes, &mut offset, path)?);
    }
    let mut loop_normals = Vec::with_capacity(split_normal_count);
    for _ in 0..split_normal_count {
        loop_normals.push([
            read_f32(bytes, &mut offset, path)?,
            read_f32(bytes, &mut offset, path)?,
            read_f32(bytes, &mut offset, path)?,
        ]);
    }
    let mut face_region_ids = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        face_region_ids.push(option_region(read_u64(bytes, &mut offset, path)?));
    }
    let mut face_element_ids = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        face_element_ids.push(ElementId(read_u64(bytes, &mut offset, path)?));
    }
    if offset != bytes.len() {
        return Err(invalid_package(
            path,
            format!(
                "part meshbin has {} trailing bytes",
                bytes.len().saturating_sub(offset)
            ),
        ));
    }

    let mesh = CanonicalPartMesh {
        definition_id,
        instance_id,
        prototype_instance_id,
        generated_by,
        source_recipe_instance,
        positions,
        vertex_element_ids,
        polygon_face_counts,
        polygon_indices,
        loop_normals,
        face_region_ids,
        face_element_ids,
    };
    mesh.validate(path)?;
    Ok(mesh)
}

fn model_export_validation_report(
    recipe: &AssetRecipe,
    artifact: &AssetArtifact,
) -> ModelExportValidationReport {
    let model_config = validation_config_from_recipe(recipe, artifact);
    let model_report = validate_model(artifact, &model_config);
    ModelExportValidationReport {
        schema_version: MODEL_EXPORT_SCHEMA_VERSION,
        source_recipe_hash: artifact.source_recipe_hash,
        counts: export_counts(artifact),
        compile_issues: artifact.validation_report.issues.clone(),
        model_issues: model_report.issues,
    }
}

fn validate_manifest(manifest: &AssetManifest, path: &Path) -> Result<(), ExportError> {
    if manifest.schema_version != MODEL_EXPORT_SCHEMA_VERSION {
        return Err(invalid_package(
            path,
            format!(
                "unsupported model export schema {}; expected {MODEL_EXPORT_SCHEMA_VERSION}",
                manifest.schema_version
            ),
        ));
    }
    validate_expected_file(&manifest.files.recipe, RECIPE_FILE, path)?;
    validate_expected_file(&manifest.files.provenance, PROVENANCE_FILE, path)?;
    validate_expected_file(&manifest.files.validation, VALIDATION_FILE, path)?;
    validate_expected_file(
        &manifest.files.blender_reconstruct,
        BLENDER_RECONSTRUCT_FILE,
        path,
    )?;

    let mut previous_instance = None;
    for part in &manifest.parts {
        validate_package_relative_path(&part.mesh, path)?;
        let expected_mesh = format!("{PARTS_DIR}/part-{:03}.meshbin", part.instance_id);
        if part.mesh != expected_mesh {
            return Err(invalid_package(
                path,
                format!(
                    "part {} mesh path '{}' must be '{}'",
                    part.instance_id, part.mesh, expected_mesh
                ),
            ));
        }
        if previous_instance.is_some_and(|previous| part.instance_id <= previous) {
            return Err(invalid_package(
                path,
                "part manifests must be ordered by strictly increasing instance ID",
            ));
        }
        previous_instance = Some(part.instance_id);
    }
    Ok(())
}

fn validate_expected_file(actual: &str, expected: &str, path: &Path) -> Result<(), ExportError> {
    validate_package_relative_path(actual, path)?;
    if actual != expected {
        return Err(invalid_package(
            path,
            format!("package file path '{actual}' must be '{expected}'"),
        ));
    }
    Ok(())
}

fn validate_part_manifest(
    manifest: &PartManifest,
    mesh: &CanonicalPartMesh,
    path: &Path,
) -> Result<(), ExportError> {
    if manifest.instance_id != mesh.instance_id.0
        || manifest.definition_id != mesh.definition_id.0
        || manifest.prototype_instance_id != mesh.prototype_instance_id.map(|id| id.0)
        || manifest.generated_by != mesh.generated_by.map(|id| id.0)
        || manifest.source_recipe_instance != mesh.source_recipe_instance
        || manifest.counts != mesh.counts()
    {
        return Err(invalid_package(
            path,
            "part meshbin semantic metadata does not match asset-manifest.json",
        ));
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ExportError> {
    let bytes = fs::read(path).map_err(|source| path_io(path, source))?;
    serde_json::from_slice(&bytes).map_err(ExportError::Json)
}

fn add_counts(total: &mut ExportCounts, part: ExportCounts) {
    total.part_count += part.part_count;
    total.vertex_count += part.vertex_count;
    total.polygon_face_count += part.polygon_face_count;
    total.polygon_index_count += part.polygon_index_count;
    total.split_normal_count += part.split_normal_count;
}

fn loop_normals_for_part(part: &CompiledPart) -> Result<Vec<[f32; 3]>, ExportError> {
    let mut by_face_vertex = BTreeMap::<(ElementId, ElementId), [f32; 3]>::new();
    for (triangle_index, face_id) in part
        .triangulated_world
        .triangle_to_polygon_face
        .iter()
        .enumerate()
    {
        let index_offset = triangle_index
            .checked_mul(3)
            .ok_or_else(|| ExportError::InvalidArtifact("triangle index overflow".to_owned()))?;
        let indices = part
            .triangulated_world
            .mesh
            .indices
            .get(index_offset..index_offset + 3)
            .ok_or_else(|| {
                ExportError::InvalidArtifact("triangle index payload is truncated".to_owned())
            })?;
        for index in indices {
            let split_index = *index as usize;
            let vertex_id = part
                .triangulated_world
                .vertex_ids
                .get(split_index)
                .copied()
                .ok_or_else(|| {
                    ExportError::InvalidArtifact(
                        "triangulated vertex ID payload is truncated".to_owned(),
                    )
                })?;
            let normal = part
                .triangulated_world
                .mesh
                .normals
                .get(split_index)
                .copied()
                .ok_or_else(|| {
                    ExportError::InvalidArtifact(
                        "triangulated normal payload is truncated".to_owned(),
                    )
                })?;
            by_face_vertex
                .entry((*face_id, vertex_id))
                .or_insert(normal);
        }
    }

    let mut loop_normals = Vec::new();
    for face in &part.world_mesh.faces {
        for vertex in &face.vertices {
            let vertex_id = part
                .world_mesh
                .vertex_ids
                .get(*vertex as usize)
                .copied()
                .ok_or_else(|| {
                    ExportError::InvalidArtifact(
                        "polygon vertex ID payload is truncated".to_owned(),
                    )
                })?;
            let normal = by_face_vertex
                .get(&(face.id, vertex_id))
                .copied()
                .ok_or_else(|| {
                    ExportError::InvalidArtifact(format!(
                        "missing split normal for face {} vertex {}",
                        face.id.0, vertex_id.0
                    ))
                })?;
            loop_normals.push(normal);
        }
    }
    Ok(loop_normals)
}

fn option_part_instance(value: u64) -> Option<PartInstanceId> {
    (value != NONE_U64).then_some(PartInstanceId(value))
}

fn option_operation(value: u64) -> Option<OperationId> {
    (value != NONE_U64).then_some(OperationId(value))
}

fn option_region(value: u64) -> Option<RegionId> {
    (value != NONE_U64).then_some(RegionId(value))
}

fn write_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn read_u8(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u8, ExportError> {
    let end = offset
        .checked_add(1)
        .ok_or_else(|| invalid_package(path, "binary offset overflow"))?;
    let value = *bytes
        .get(*offset)
        .ok_or_else(|| invalid_package(path, "binary payload is truncated"))?;
    *offset = end;
    Ok(value)
}

fn read_u32(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u32, ExportError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid_package(path, "binary offset overflow"))?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| invalid_package(path, "binary payload is truncated"))?;
    *offset = end;
    Ok(u32::from_le_bytes(slice.try_into().map_err(|_| {
        invalid_package(path, "invalid u32 payload")
    })?))
}

fn read_u64(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u64, ExportError> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| invalid_package(path, "binary offset overflow"))?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| invalid_package(path, "binary payload is truncated"))?;
    *offset = end;
    Ok(u64::from_le_bytes(slice.try_into().map_err(|_| {
        invalid_package(path, "invalid u64 payload")
    })?))
}

fn read_f32(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<f32, ExportError> {
    Ok(f32::from_bits(read_u32(bytes, offset, path)?))
}

fn read_count(
    bytes: &[u8],
    offset: &mut usize,
    path: &Path,
    label: &'static str,
) -> Result<usize, ExportError> {
    usize::try_from(read_u64(bytes, offset, path)?)
        .map_err(|_| invalid_package(path, format!("{label} does not fit this platform")))
}

fn array_is_finite<const N: usize>(value: [f32; N]) -> bool {
    value.iter().all(|component| component.is_finite())
}
