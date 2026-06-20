#![forbid(unsafe_code)]

//! Lossless same-topology deformation decompiler.
//!
//! This crate turns a source mesh and a target mesh with identical vertex and
//! triangle topology into a small explanatory operator stream plus a final
//! lossless residual. The current MVP intentionally starts with a strict
//! contract: same vertex order, same face order, same indices.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;
use thiserror::Error;

const SCHEMA_VERSION: u32 = 2;
const SOURCE_MESHBIN: &str = "source.meshbin";
const TARGET_MESHBIN: &str = "target.meshbin";
const AFFINE_POSITIONS_FILE: &str = "operators/0000-global-affine-positions.f32";
const RESIDUAL_INDEX_FILE: &str = "residual/indices.u32";
const RESIDUAL_POSITION_FILE: &str = "residual/positions.f32";
const MANIFEST_FILE: &str = "manifest.json";
const VERIFICATION_FILE: &str = "verification.json";
const PACKAGE_VERIFICATION_FILE: &str = "package-verification.json";
const BLENDER_SCRIPT_FILE: &str = "blender_reconstruct.py";
const MESHBIN_MAGIC: &[u8; 8] = b"SLMBIN01";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const JACOBI_MAX_ITERATIONS: usize = 96;
const PSEUDOINVERSE_RELATIVE_EPSILON: f64 = 1.0e-11;
const PACKAGE_TEMP_MARKER: &str = ".shapelab-package-tmp-";
const PACKAGE_BACKUP_MARKER: &str = ".shapelab-package-backup-";
static PACKAGE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Settings for a same-topology deformation decompile.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompileSettings {
    /// Minimum displacement fraction an affine fit must explain before it is
    /// emitted as an editable operator.
    pub affine_min_explained: f32,
    /// Verification tolerance used to count out-of-tolerance reconstructed
    /// vertices. The residual itself remains lossless.
    pub residual_epsilon: f32,
}

impl Default for DecompileSettings {
    fn default() -> Self {
        Self {
            affine_min_explained: 0.01,
            residual_epsilon: 0.0,
        }
    }
}

/// Top-level package manifest written beside the binary sidecars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompileManifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Coordinate convention used by Shape Lab meshes.
    pub coordinate_system: CoordinateSystem,
    /// Numeric encoding contract for all binary payloads.
    pub numeric_format: NumericFormat,
    /// Source mesh asset reference.
    pub source: MeshAsset,
    /// Target mesh asset reference.
    pub target: MeshAsset,
    /// Topology summary shared by source and target.
    pub topology: TopologySummary,
    /// Settings used for the decompile.
    pub settings: DecompileSettings,
    /// Ordered reconstruction operators.
    pub operators: Vec<OperatorManifest>,
    /// Verification report after applying every operator.
    pub verification: VerificationReport,
}

/// Coordinate metadata embedded in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinateSystem {
    /// Handedness of the coordinate system.
    pub handedness: String,
    /// Up axis.
    pub up_axis: String,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        Self {
            handedness: "right".to_owned(),
            up_axis: "y".to_owned(),
        }
    }
}

/// Numeric metadata embedded in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericFormat {
    /// Scalar type for positions.
    pub scalar: String,
    /// Binary sidecar byte order.
    pub endian: String,
    /// Canonical affine arithmetic contract. Every multiplication and addition
    /// is rounded to IEEE-754 binary32 in the declared left-to-right order;
    /// fused multiply-add contraction is not permitted.
    pub affine_evaluation: String,
}

impl Default for NumericFormat {
    fn default() -> Self {
        Self {
            scalar: "float32".to_owned(),
            endian: "little".to_owned(),
            affine_evaluation: "float32_stepwise_no_fma".to_owned(),
        }
    }
}

/// Mesh asset reference stored in the package manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAsset {
    /// Package-relative path.
    pub path: String,
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangles.
    pub triangle_count: usize,
}

/// Exact topology summary shared by source and target meshes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySummary {
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangles.
    pub triangle_count: usize,
    /// Number of triangle indices.
    pub index_count: usize,
    /// Stable diagnostic FNV-1a fingerprint over vertex count, index count, and ordered indices. Exact verification still compares the full arrays.
    pub hash: String,
}

/// One manifest operator in the reconstruction stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperatorManifest {
    /// Least-squares affine map emitted as the editable first pass.
    GlobalAffine {
        /// Stable operator ID.
        id: String,
        /// Human-facing label.
        label: String,
        /// Row-major 4x4 matrix mapping source positions to the affine stage.
        matrix_row_major_4x4: [f32; 16],
        /// Fraction of source-to-target squared displacement explained.
        explained_displacement_fraction: f32,
        /// Largest remaining Euclidean error after the affine stage.
        max_remaining_error: f32,
        /// Package-relative baked cumulative stage positions.
        baked_positions_file: String,
    },
    /// Lossless vertex correction to the final target positions.
    LosslessCorrection {
        /// Stable operator ID.
        id: String,
        /// Human-facing label.
        label: String,
        /// Package-relative u32 residual vertex index list.
        residual_index_file: String,
        /// Package-relative f32 absolute residual positions.
        residual_position_file: String,
        /// Number of vertices corrected by the residual.
        corrected_vertex_count: usize,
        /// Largest Euclidean error after applying the residual.
        max_error_after: f32,
    },
}

/// Verification metrics for a reconstructed mesh.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether topology matched exactly before decompilation.
    pub topology_exact: bool,
    /// Vertex count.
    pub vertex_count: usize,
    /// Triangle count.
    pub triangle_count: usize,
    /// Maximum per-component absolute error.
    pub max_component_error: f32,
    /// Maximum Euclidean vertex error.
    pub max_euclidean_error: f32,
    /// Mean Euclidean vertex error.
    pub mean_euclidean_error: f32,
    /// Root-mean-square Euclidean vertex error.
    pub rms_euclidean_error: f32,
    /// Verification tolerance.
    pub tolerance: f32,
    /// Number of vertices with Euclidean error greater than tolerance.
    pub outside_tolerance: usize,
}

/// Verification produced by reading a package back from disk and replaying
/// its serialized operator payloads.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageVerificationReport {
    /// Package schema version that was verified.
    pub schema_version: u32,
    /// Whether source and target ordered triangle topology matched exactly.
    pub topology_exact: bool,
    /// Whether the diagnostic topology fingerprint in the manifest matched the payload.
    pub topology_hash_matches_manifest: bool,
    /// Whether all final reconstructed position components matched the target
    /// as exact IEEE-754 `f32` bit patterns.
    pub positions_bit_exact: bool,
    /// Vertex count.
    pub vertex_count: usize,
    /// Triangle count.
    pub triangle_count: usize,
    /// Number of serialized operators replayed.
    pub operator_count: usize,
    /// Number of vertices carried by the final lossless correction.
    pub residual_vertex_count: usize,
    /// Maximum per-component absolute error after replay.
    pub max_component_error: f32,
    /// Maximum Euclidean vertex error after replay.
    pub max_euclidean_error: f32,
    /// Number of vertices outside the manifest verification tolerance.
    pub outside_tolerance: usize,
}

/// In-memory decompile result and package payloads.
#[derive(Debug, Clone, PartialEq)]
pub struct DecompileResult {
    /// Package manifest.
    pub manifest: DecompileManifest,
    /// Verification report after reconstruction.
    pub verification: VerificationReport,
    /// Baked cumulative positions after the affine stage, when emitted.
    pub affine_positions: Option<Vec<[f32; 3]>>,
    /// Vertex indices corrected by the lossless residual.
    pub residual_indices: Vec<u32>,
    /// Absolute target positions for each residual index.
    pub residual_positions: Vec<[f32; 3]>,
    /// Final reconstructed positions after every operator.
    pub reconstructed_positions: Vec<[f32; 3]>,
}

/// Paths produced by writing a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePaths {
    /// Package directory.
    pub directory: PathBuf,
    /// Manifest JSON path.
    pub manifest: PathBuf,
    /// Verification JSON path.
    pub verification: PathBuf,
    /// Package replay verification JSON path.
    pub package_verification: PathBuf,
    /// Blender reconstruction script path.
    pub blender_script: PathBuf,
}

/// Decompiler errors.
#[derive(Debug, Error)]
pub enum DecompileError {
    /// Settings are invalid.
    #[error("invalid decompile settings: {0}")]
    InvalidSettings(String),
    /// Mesh data is invalid for decompilation.
    #[error("invalid {mesh_name} mesh: {message}")]
    InvalidMesh {
        /// Mesh label.
        mesh_name: &'static str,
        /// Error details.
        message: String,
    },
    /// Source and target topology are not identical.
    #[error("topology mismatch: {0}")]
    TopologyMismatch(String),
    /// I/O failed for a package path.
    #[error("io error for {path}: {source}")]
    PathIo {
        /// Affected path.
        path: PathBuf,
        /// Source error.
        #[source]
        source: std::io::Error,
    },
    /// JSON serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A serialized package is malformed or inconsistent.
    #[error("invalid decompile package at {path}: {message}")]
    InvalidPackage {
        /// Package path or payload path associated with the failure.
        path: PathBuf,
        /// Validation details.
        message: String,
    },
    /// The package schema cannot be read by this build.
    #[error("unsupported decompile schema version {found}; supported version is {supported}")]
    UnsupportedSchema {
        /// Version found in the package.
        found: u32,
        /// Version supported by this build.
        supported: u32,
    },
}

/// Decompile a source mesh into operators that reconstruct a target mesh.
pub fn decompile_pair(
    source: &TriangleMesh,
    target: &TriangleMesh,
    settings: DecompileSettings,
) -> Result<DecompileResult, DecompileError> {
    validate_settings(settings)?;
    validate_decompile_mesh(source, "source")?;
    validate_decompile_mesh(target, "target")?;
    ensure_identical_topology(source, target)?;

    let identity_error = sum_squared_distance(&source.positions, &target.positions);
    let affine_matrix = fit_affine(&source.positions, &target.positions).unwrap_or(identity());
    let fitted_positions = apply_affine_to_positions(&source.positions, affine_matrix);
    let affine_error = sum_squared_distance(&fitted_positions, &target.positions);
    let explained = explained_fraction(identity_error, affine_error);
    let emit_affine = identity_error > 0.0
        && affine_error < identity_error
        && explained >= f64::from(settings.affine_min_explained);

    let current_positions = if emit_affine {
        fitted_positions.clone()
    } else {
        source.positions.clone()
    };

    let mut residual_indices = Vec::new();
    let mut residual_positions = Vec::new();
    for (index, (current, target_position)) in
        current_positions.iter().zip(&target.positions).enumerate()
    {
        if !positions_bit_equal(*current, *target_position) {
            residual_indices.push(u32::try_from(index).map_err(|_| {
                DecompileError::InvalidMesh {
                    mesh_name: "source",
                    message: "vertex count exceeds u32 residual index storage".to_owned(),
                }
            })?);
            residual_positions.push(*target_position);
        }
    }

    let mut reconstructed_positions = current_positions;
    for (index, position) in residual_indices.iter().zip(&residual_positions) {
        reconstructed_positions[*index as usize] = *position;
    }

    let verification = verify_positions(
        &reconstructed_positions,
        &target.positions,
        source.indices.len() / 3,
        settings.residual_epsilon,
    );
    let topology = TopologySummary {
        vertex_count: source.positions.len(),
        triangle_count: source.indices.len() / 3,
        index_count: source.indices.len(),
        hash: topology_hash(source),
    };

    let mut operators = Vec::new();
    if emit_affine {
        operators.push(OperatorManifest::GlobalAffine {
            id: "op-0000-global-affine".to_owned(),
            label: "Global affine fit".to_owned(),
            matrix_row_major_4x4: affine_matrix,
            explained_displacement_fraction: explained as f32,
            max_remaining_error: max_euclidean_distance(&fitted_positions, &target.positions),
            baked_positions_file: AFFINE_POSITIONS_FILE.to_owned(),
        });
    }
    operators.push(OperatorManifest::LosslessCorrection {
        id: "op-final-lossless-correction".to_owned(),
        label: "Lossless final correction".to_owned(),
        residual_index_file: RESIDUAL_INDEX_FILE.to_owned(),
        residual_position_file: RESIDUAL_POSITION_FILE.to_owned(),
        corrected_vertex_count: residual_indices.len(),
        max_error_after: verification.max_euclidean_error,
    });

    let manifest = DecompileManifest {
        schema_version: SCHEMA_VERSION,
        coordinate_system: CoordinateSystem::default(),
        numeric_format: NumericFormat::default(),
        source: MeshAsset {
            path: SOURCE_MESHBIN.to_owned(),
            vertex_count: source.positions.len(),
            triangle_count: source.indices.len() / 3,
        },
        target: MeshAsset {
            path: TARGET_MESHBIN.to_owned(),
            vertex_count: target.positions.len(),
            triangle_count: target.indices.len() / 3,
        },
        topology,
        settings,
        operators,
        verification,
    };

    Ok(DecompileResult {
        manifest,
        verification,
        affine_positions: emit_affine.then_some(fitted_positions),
        residual_indices,
        residual_positions,
        reconstructed_positions,
    })
}

/// Write a decompile package directory.
///
/// The package is assembled and replay-verified in a sibling staging
/// directory before it replaces the requested output directory. A failed
/// write therefore leaves an existing valid package untouched.
pub fn write_decompile_package(
    result: &DecompileResult,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: impl AsRef<Path>,
) -> Result<PackagePaths, DecompileError> {
    let out_dir = out_dir.as_ref();
    validate_result_consistency(result, source, target)?;

    let staging = StagedPackageDirectory::create(out_dir)?;
    write_decompile_package_contents(result, source, target, staging.path())?;
    staging.publish(out_dir)?;

    Ok(package_paths(out_dir))
}

fn write_decompile_package_contents(
    result: &DecompileResult,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: &Path,
) -> Result<(), DecompileError> {
    fs::create_dir_all(out_dir).map_err(|source| path_io(out_dir, source))?;
    fs::create_dir_all(out_dir.join("operators"))
        .map_err(|source| path_io(&out_dir.join("operators"), source))?;
    fs::create_dir_all(out_dir.join("residual"))
        .map_err(|source| path_io(&out_dir.join("residual"), source))?;

    write_meshbin(&package_path(out_dir, SOURCE_MESHBIN), source)?;
    write_meshbin(&package_path(out_dir, TARGET_MESHBIN), target)?;
    if let Some(positions) = &result.affine_positions {
        write_positions(&package_path(out_dir, AFFINE_POSITIONS_FILE), positions)?;
    }
    write_u32s(
        &package_path(out_dir, RESIDUAL_INDEX_FILE),
        &result.residual_indices,
    )?;
    write_positions(
        &package_path(out_dir, RESIDUAL_POSITION_FILE),
        &result.residual_positions,
    )?;

    write_json(&package_path(out_dir, MANIFEST_FILE), &result.manifest)?;
    write_json(
        &package_path(out_dir, VERIFICATION_FILE),
        &result.verification,
    )?;
    write_text(
        &package_path(out_dir, BLENDER_SCRIPT_FILE),
        blender_reconstruction_script(),
    )?;

    let package_verification_report = verify_decompile_package(out_dir)?;
    write_json(
        &package_path(out_dir, PACKAGE_VERIFICATION_FILE),
        &package_verification_report,
    )?;
    Ok(())
}

struct StagedPackageDirectory {
    path: PathBuf,
    published: bool,
}

impl StagedPackageDirectory {
    fn create(target: &Path) -> Result<Self, DecompileError> {
        let parent = sibling_directory(target);
        fs::create_dir_all(parent).map_err(|source| path_io(parent, source))?;
        let target_name = target.file_name().ok_or_else(|| {
            invalid_package(target, "package output must have a final directory name")
        })?;
        let target_name = target_name.to_string_lossy();
        for _ in 0..128 {
            let counter = PACKAGE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".{target_name}{PACKAGE_TEMP_MARKER}{}-{counter}",
                process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        published: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(path_io(target, error)),
            }
        }
        Err(path_io(
            target,
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not allocate a unique staging directory",
            ),
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn publish(mut self, target: &Path) -> Result<(), DecompileError> {
        match fs::symlink_metadata(target) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(invalid_package(
                        target,
                        "package output already exists and is not a regular directory",
                    ));
                }
                let backup = reserve_backup_path(target)?;
                fs::rename(target, &backup).map_err(|source| path_io(target, source))?;
                match fs::rename(&self.path, target) {
                    Ok(()) => {
                        self.published = true;
                        let _ = fs::remove_dir_all(&backup);
                        Ok(())
                    }
                    Err(publish_error) => {
                        let restore_result = fs::rename(&backup, target);
                        if let Err(restore_error) = restore_result {
                            return Err(invalid_package(
                                target,
                                format!(
                                    "publishing the verified package failed ({publish_error}); restoring the previous package also failed ({restore_error})"
                                ),
                            ));
                        }
                        Err(path_io(target, publish_error))
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::rename(&self.path, target).map_err(|source| path_io(target, source))?;
                self.published = true;
                Ok(())
            }
            Err(error) => Err(path_io(target, error)),
        }
    }
}

impl Drop for StagedPackageDirectory {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn sibling_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn reserve_backup_path(target: &Path) -> Result<PathBuf, DecompileError> {
    let parent = sibling_directory(target);
    let target_name = target.file_name().ok_or_else(|| {
        invalid_package(target, "package output must have a final directory name")
    })?;
    let target_name = target_name.to_string_lossy();
    for _ in 0..128 {
        let counter = PACKAGE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".{target_name}{PACKAGE_BACKUP_MARKER}{}-{counter}",
            process::id()
        ));
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(path),
            Ok(_) => continue,
            Err(error) => return Err(path_io(target, error)),
        }
    }
    Err(path_io(
        target,
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique package backup path",
        ),
    ))
}

fn package_paths(out_dir: &Path) -> PackagePaths {
    PackagePaths {
        directory: out_dir.to_path_buf(),
        manifest: package_path(out_dir, MANIFEST_FILE),
        verification: package_path(out_dir, VERIFICATION_FILE),
        package_verification: package_path(out_dir, PACKAGE_VERIFICATION_FILE),
        blender_script: package_path(out_dir, BLENDER_SCRIPT_FILE),
    }
}

/// Read a serialized decompile package, replay all operators from the binary
/// sidecars, and verify exact topology and final `f32` positions.
pub fn verify_decompile_package(
    package_dir: impl AsRef<Path>,
) -> Result<PackageVerificationReport, DecompileError> {
    let package_dir = package_dir.as_ref();
    let manifest_path = resolve_package_asset(package_dir, MANIFEST_FILE)?;
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|source| path_io(&manifest_path, source))?;
    let manifest: DecompileManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(DecompileError::UnsupportedSchema {
            found: manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    validate_manifest_contract(&manifest, &manifest_path)?;

    let verification_path = resolve_package_asset(package_dir, VERIFICATION_FILE)?;
    let standalone_verification: VerificationReport = serde_json::from_slice(
        &fs::read(&verification_path).map_err(|source| path_io(&verification_path, source))?,
    )?;
    if standalone_verification != manifest.verification {
        return Err(invalid_package(
            &verification_path,
            "verification.json does not match manifest.json",
        ));
    }

    let source_path = resolve_package_asset(package_dir, &manifest.source.path)?;
    let target_path = resolve_package_asset(package_dir, &manifest.target.path)?;
    let source = read_meshbin(&source_path)?;
    let target = read_meshbin(&target_path)?;
    ensure_payload_counts(&manifest.source, &source, &source_path)?;
    ensure_payload_counts(&manifest.target, &target, &target_path)?;

    let topology_exact =
        source.indices == target.indices && source.positions.len() == target.positions.len();
    if !topology_exact {
        return Err(invalid_package(
            package_dir,
            "source and target payload topology is not identical",
        ));
    }
    let payload_topology_hash = topology_hash_from_parts(source.positions.len(), &source.indices);
    let topology_hash_matches_manifest = payload_topology_hash == manifest.topology.hash;
    if !topology_hash_matches_manifest {
        return Err(invalid_package(
            &manifest_path,
            format!(
                "topology fingerprint mismatch: manifest={} payload={payload_topology_hash}",
                manifest.topology.hash
            ),
        ));
    }
    if manifest.topology.vertex_count != source.positions.len()
        || manifest.topology.index_count != source.indices.len()
        || manifest.topology.triangle_count != source.indices.len() / 3
    {
        return Err(invalid_package(
            &manifest_path,
            "manifest topology counts do not match source.meshbin",
        ));
    }
    if manifest.verification.vertex_count != source.positions.len()
        || manifest.verification.triangle_count != source.indices.len() / 3
    {
        return Err(invalid_package(
            &manifest_path,
            "manifest verification counts do not match the mesh payload",
        ));
    }

    let identity_error = sum_squared_distance(&source.positions, &target.positions);
    let mut current_positions = source.positions.clone();
    let mut residual_vertex_count = 0_usize;
    let mut saw_lossless = false;
    let mut declared_lossless_max_error = None;
    let mut operator_ids = BTreeSet::new();

    for (operator_index, operator) in manifest.operators.iter().enumerate() {
        if saw_lossless {
            return Err(invalid_package(
                &manifest_path,
                "the lossless correction must be the final operator",
            ));
        }
        let (operator_id, operator_label) = operator_identity(operator);
        if operator_id.trim().is_empty() || operator_label.trim().is_empty() {
            return Err(invalid_package(
                &manifest_path,
                "operator IDs and labels must not be empty",
            ));
        }
        if !operator_ids.insert(operator_id) {
            return Err(invalid_package(
                &manifest_path,
                format!("duplicate operator ID '{operator_id}'"),
            ));
        }

        match operator {
            OperatorManifest::GlobalAffine {
                matrix_row_major_4x4,
                explained_displacement_fraction,
                max_remaining_error,
                baked_positions_file,
                ..
            } => {
                if operator_index != 0 {
                    return Err(invalid_package(
                        &manifest_path,
                        "the global affine operator must be first",
                    ));
                }
                validate_affine_matrix(*matrix_row_major_4x4, &manifest_path)?;
                let path = resolve_package_asset(package_dir, baked_positions_file)?;
                let baked = read_positions(&path, source.positions.len())?;
                let evaluated = apply_affine_to_positions(&source.positions, *matrix_row_major_4x4);
                if !position_slices_bit_equal(&evaluated, &baked) {
                    return Err(invalid_package(
                        &path,
                        "baked affine positions do not match the serialized affine matrix",
                    ));
                }

                let affine_error = sum_squared_distance(&evaluated, &target.positions);
                let expected_explained = explained_fraction(identity_error, affine_error) as f32;
                let expected_max_error = max_euclidean_distance(&evaluated, &target.positions);
                if !f32_bits_equal(*explained_displacement_fraction, expected_explained) {
                    return Err(invalid_package(
                        &manifest_path,
                        "global affine explained-displacement metadata is inconsistent",
                    ));
                }
                if !f32_bits_equal(*max_remaining_error, expected_max_error) {
                    return Err(invalid_package(
                        &manifest_path,
                        "global affine remaining-error metadata is inconsistent",
                    ));
                }
                if identity_error <= 0.0
                    || affine_error >= identity_error
                    || expected_explained < manifest.settings.affine_min_explained
                {
                    return Err(invalid_package(
                        &manifest_path,
                        "global affine operator does not satisfy the package emission threshold",
                    ));
                }
                current_positions = baked;
            }
            OperatorManifest::LosslessCorrection {
                residual_index_file,
                residual_position_file,
                corrected_vertex_count,
                max_error_after,
                ..
            } => {
                saw_lossless = true;
                let index_path = resolve_package_asset(package_dir, residual_index_file)?;
                let position_path = resolve_package_asset(package_dir, residual_position_file)?;
                let indices = read_u32s(&index_path)?;
                let positions = read_positions(&position_path, indices.len())?;
                if indices.len() != *corrected_vertex_count {
                    return Err(invalid_package(
                        &manifest_path,
                        format!(
                            "lossless operator declares {} corrected vertices but stores {}",
                            corrected_vertex_count,
                            indices.len()
                        ),
                    ));
                }
                ensure_strictly_increasing_indices(&indices, current_positions.len(), &index_path)?;
                for (index, position) in indices.iter().zip(&positions) {
                    let index = *index as usize;
                    if positions_bit_equal(current_positions[index], target.positions[index]) {
                        return Err(invalid_package(
                            &index_path,
                            format!("lossless correction contains no-op vertex index {index}"),
                        ));
                    }
                    current_positions[index] = *position;
                }
                residual_vertex_count = indices.len();
                declared_lossless_max_error = Some(*max_error_after);
            }
        }
    }
    if !saw_lossless {
        return Err(invalid_package(
            &manifest_path,
            "package is missing the final lossless correction",
        ));
    }

    let verification = verify_positions(
        &current_positions,
        &target.positions,
        target.indices.len() / 3,
        manifest.verification.tolerance,
    );
    let positions_bit_exact = position_slices_bit_equal(&current_positions, &target.positions);
    if !positions_bit_exact {
        return Err(invalid_package(
            package_dir,
            format!(
                "serialized operators did not reconstruct target positions exactly; max error={}",
                verification.max_euclidean_error
            ),
        ));
    }
    if !f32_bits_equal(
        declared_lossless_max_error.unwrap_or(f32::NAN),
        verification.max_euclidean_error,
    ) {
        return Err(invalid_package(
            &manifest_path,
            "lossless correction max-error metadata is inconsistent",
        ));
    }
    if verification != manifest.verification {
        return Err(invalid_package(
            &manifest_path,
            "manifest verification report does not match replayed package data",
        ));
    }

    Ok(PackageVerificationReport {
        schema_version: manifest.schema_version,
        topology_exact,
        topology_hash_matches_manifest,
        positions_bit_exact,
        vertex_count: source.positions.len(),
        triangle_count: source.indices.len() / 3,
        operator_count: manifest.operators.len(),
        residual_vertex_count,
        max_component_error: verification.max_component_error,
        max_euclidean_error: verification.max_euclidean_error,
        outside_tolerance: verification.outside_tolerance,
    })
}

fn operator_identity(operator: &OperatorManifest) -> (&str, &str) {
    match operator {
        OperatorManifest::GlobalAffine { id, label, .. }
        | OperatorManifest::LosslessCorrection { id, label, .. } => (id, label),
    }
}

fn validate_affine_matrix(matrix: [f32; 16], path: &Path) -> Result<(), DecompileError> {
    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(invalid_package(
            path,
            "global affine matrix contains a non-finite value",
        ));
    }
    let expected_bottom_row = [0.0_f32, 0.0, 0.0, 1.0];
    if !matrix[12..16]
        .iter()
        .zip(expected_bottom_row)
        .all(|(actual, expected)| f32_bits_equal(*actual, expected))
    {
        return Err(invalid_package(
            path,
            "global affine matrix bottom row must be exactly [0, 0, 0, 1]",
        ));
    }
    Ok(())
}

fn validate_manifest_contract(
    manifest: &DecompileManifest,
    path: &Path,
) -> Result<(), DecompileError> {
    if manifest.coordinate_system != CoordinateSystem::default() {
        return Err(invalid_package(
            path,
            "unsupported coordinate system; expected right-handed Y-up coordinates",
        ));
    }
    if manifest.numeric_format != NumericFormat::default() {
        return Err(invalid_package(
            path,
            "unsupported numeric format; expected little-endian float32 payloads and stepwise non-fused affine arithmetic",
        ));
    }
    validate_settings(manifest.settings)
        .map_err(|error| invalid_package(path, error.to_string()))?;
    if manifest.operators.is_empty() || manifest.operators.len() > 2 {
        return Err(invalid_package(
            path,
            "schema version 2 requires one lossless operator and at most one affine operator",
        ));
    }
    let mut operator_ids = BTreeSet::new();
    let mut operator_labels = BTreeSet::new();
    for operator in &manifest.operators {
        let (id, label) = operator_identity(operator);
        if id.trim().is_empty() || label.trim().is_empty() {
            return Err(invalid_package(
                path,
                "operator IDs and labels must not be empty",
            ));
        }
        if !operator_ids.insert(id) {
            return Err(invalid_package(
                path,
                format!("duplicate operator ID '{id}'"),
            ));
        }
        if label == "Basis" || !operator_labels.insert(label) {
            return Err(invalid_package(
                path,
                format!("operator label '{label}' is reserved or duplicated"),
            ));
        }
    }
    if !manifest.verification.topology_exact {
        return Err(invalid_package(
            path,
            "same-topology packages must declare exact topology",
        ));
    }
    if !f32_bits_equal(
        manifest.verification.tolerance,
        manifest.settings.residual_epsilon,
    ) {
        return Err(invalid_package(
            path,
            "verification tolerance must match residual_epsilon",
        ));
    }
    for (label, value) in [
        (
            "max_component_error",
            manifest.verification.max_component_error,
        ),
        (
            "max_euclidean_error",
            manifest.verification.max_euclidean_error,
        ),
        (
            "mean_euclidean_error",
            manifest.verification.mean_euclidean_error,
        ),
        (
            "rms_euclidean_error",
            manifest.verification.rms_euclidean_error,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(invalid_package(
                path,
                format!("verification field {label} must be finite and non-negative"),
            ));
        }
    }
    Ok(())
}

fn validate_settings(settings: DecompileSettings) -> Result<(), DecompileError> {
    if !settings.affine_min_explained.is_finite()
        || !(0.0..=1.0).contains(&settings.affine_min_explained)
    {
        return Err(DecompileError::InvalidSettings(
            "affine_min_explained must be finite and between 0 and 1".to_owned(),
        ));
    }
    if !settings.residual_epsilon.is_finite() || settings.residual_epsilon < 0.0 {
        return Err(DecompileError::InvalidSettings(
            "residual_epsilon must be finite and non-negative".to_owned(),
        ));
    }
    Ok(())
}

fn validate_decompile_mesh(
    mesh: &TriangleMesh,
    mesh_name: &'static str,
) -> Result<(), DecompileError> {
    if mesh.positions.is_empty() {
        return Err(invalid_mesh(
            mesh_name,
            "mesh must contain at least one vertex",
        ));
    }
    if mesh.positions.len() > u32::MAX as usize {
        return Err(invalid_mesh(
            mesh_name,
            "vertex count exceeds the u32 topology/index contract",
        ));
    }
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(invalid_mesh(
            mesh_name,
            "index count must be divisible by three",
        ));
    }
    if mesh.indices.is_empty() {
        return Err(invalid_mesh(
            mesh_name,
            "mesh must contain at least one triangle",
        ));
    }
    for position in &mesh.positions {
        if !array_is_finite(*position) {
            return Err(invalid_mesh(mesh_name, "all positions must be finite"));
        }
    }
    for index in &mesh.indices {
        if *index as usize >= mesh.positions.len() {
            return Err(invalid_mesh(
                mesh_name,
                "all indices must reference existing vertices",
            ));
        }
    }
    for triangle in mesh.indices.chunks_exact(3) {
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(invalid_mesh(
                mesh_name,
                "triangles must reference three distinct vertex indices",
            ));
        }
    }
    Ok(())
}

fn ensure_identical_topology(
    source: &TriangleMesh,
    target: &TriangleMesh,
) -> Result<(), DecompileError> {
    if source.positions.len() != target.positions.len() {
        return Err(DecompileError::TopologyMismatch(format!(
            "vertex count differs: source={} target={}",
            source.positions.len(),
            target.positions.len()
        )));
    }
    if source.indices.len() != target.indices.len() {
        return Err(DecompileError::TopologyMismatch(format!(
            "index count differs: source={} target={}",
            source.indices.len(),
            target.indices.len()
        )));
    }
    if source.indices != target.indices {
        let first_difference = source
            .indices
            .iter()
            .zip(&target.indices)
            .position(|(left, right)| left != right)
            .unwrap_or(0);
        return Err(DecompileError::TopologyMismatch(format!(
            "ordered triangle indices differ at index {first_difference}"
        )));
    }
    Ok(())
}

fn validate_result_consistency(
    result: &DecompileResult,
    source: &TriangleMesh,
    target: &TriangleMesh,
) -> Result<(), DecompileError> {
    validate_decompile_mesh(source, "source")?;
    validate_decompile_mesh(target, "target")?;
    ensure_identical_topology(source, target)?;
    if result.manifest.schema_version != SCHEMA_VERSION {
        return Err(DecompileError::UnsupportedSchema {
            found: result.manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    validate_manifest_contract(&result.manifest, Path::new(MANIFEST_FILE))?;

    if result.manifest.source.path != SOURCE_MESHBIN
        || result.manifest.target.path != TARGET_MESHBIN
        || result.manifest.source.vertex_count != source.positions.len()
        || result.manifest.target.vertex_count != target.positions.len()
        || result.manifest.source.triangle_count != source.indices.len() / 3
        || result.manifest.target.triangle_count != target.indices.len() / 3
    {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "decompile result mesh asset metadata does not match the supplied meshes",
        ));
    }

    let expected_hash = topology_hash(source);
    if result.manifest.topology.hash != expected_hash
        || result.manifest.topology.vertex_count != source.positions.len()
        || result.manifest.topology.index_count != source.indices.len()
        || result.manifest.topology.triangle_count != source.indices.len() / 3
    {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "decompile result topology metadata does not match the supplied meshes",
        ));
    }
    if result.residual_indices.len() != result.residual_positions.len() {
        return Err(invalid_package(
            Path::new(RESIDUAL_INDEX_FILE),
            "residual index and position counts differ",
        ));
    }
    ensure_strictly_increasing_indices(
        &result.residual_indices,
        source.positions.len(),
        Path::new(RESIDUAL_INDEX_FILE),
    )?;

    let identity_error = sum_squared_distance(&source.positions, &target.positions);
    let mut current_positions = source.positions.clone();
    let mut saw_affine = false;
    let mut saw_lossless = false;
    for (operator_index, operator) in result.manifest.operators.iter().enumerate() {
        if saw_lossless {
            return Err(invalid_package(
                Path::new(MANIFEST_FILE),
                "the lossless correction must be the final operator",
            ));
        }
        match operator {
            OperatorManifest::GlobalAffine {
                matrix_row_major_4x4,
                explained_displacement_fraction,
                max_remaining_error,
                baked_positions_file,
                ..
            } => {
                if operator_index != 0
                    || saw_affine
                    || baked_positions_file != AFFINE_POSITIONS_FILE
                {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "decompile result contains an invalid global affine operator",
                    ));
                }
                validate_affine_matrix(*matrix_row_major_4x4, Path::new(MANIFEST_FILE))?;
                let affine_positions = result.affine_positions.as_ref().ok_or_else(|| {
                    invalid_package(
                        Path::new(AFFINE_POSITIONS_FILE),
                        "affine operator is missing its baked positions",
                    )
                })?;
                let evaluated = apply_affine_to_positions(&source.positions, *matrix_row_major_4x4);
                if !position_slices_bit_equal(&evaluated, affine_positions) {
                    return Err(invalid_package(
                        Path::new(AFFINE_POSITIONS_FILE),
                        "baked affine positions do not match the affine matrix",
                    ));
                }
                let affine_error = sum_squared_distance(&evaluated, &target.positions);
                let expected_explained = explained_fraction(identity_error, affine_error) as f32;
                let expected_max = max_euclidean_distance(&evaluated, &target.positions);
                if !f32_bits_equal(*explained_displacement_fraction, expected_explained)
                    || !f32_bits_equal(*max_remaining_error, expected_max)
                    || identity_error <= 0.0
                    || affine_error >= identity_error
                    || expected_explained < result.manifest.settings.affine_min_explained
                {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "global affine metadata or emission decision is inconsistent",
                    ));
                }
                current_positions = evaluated;
                saw_affine = true;
            }
            OperatorManifest::LosslessCorrection {
                residual_index_file,
                residual_position_file,
                corrected_vertex_count,
                max_error_after,
                ..
            } => {
                if residual_index_file != RESIDUAL_INDEX_FILE
                    || residual_position_file != RESIDUAL_POSITION_FILE
                    || *corrected_vertex_count != result.residual_indices.len()
                {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "lossless correction metadata does not match the result payload",
                    ));
                }
                for (index, position) in result
                    .residual_indices
                    .iter()
                    .zip(&result.residual_positions)
                {
                    let index = *index as usize;
                    if positions_bit_equal(current_positions[index], target.positions[index]) {
                        return Err(invalid_package(
                            Path::new(RESIDUAL_INDEX_FILE),
                            format!("lossless correction contains no-op vertex index {index}"),
                        ));
                    }
                    if !positions_bit_equal(*position, target.positions[index]) {
                        return Err(invalid_package(
                            Path::new(RESIDUAL_POSITION_FILE),
                            format!(
                                "lossless correction position at vertex {index} does not equal the target"
                            ),
                        ));
                    }
                    current_positions[index] = *position;
                }
                let after = max_euclidean_distance(&current_positions, &target.positions);
                if !f32_bits_equal(*max_error_after, after) {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "lossless correction max-error metadata is inconsistent",
                    ));
                }
                saw_lossless = true;
            }
        }
    }
    if !saw_lossless || saw_affine != result.affine_positions.is_some() {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "decompile operator stream does not match its in-memory payloads",
        ));
    }
    if result.reconstructed_positions.len() != target.positions.len()
        || !position_slices_bit_equal(&current_positions, &result.reconstructed_positions)
        || !position_slices_bit_equal(&result.reconstructed_positions, &target.positions)
    {
        return Err(invalid_package(
            Path::new(VERIFICATION_FILE),
            "decompile result does not reconstruct the supplied target exactly",
        ));
    }
    let expected_verification = verify_positions(
        &result.reconstructed_positions,
        &target.positions,
        target.indices.len() / 3,
        result.manifest.settings.residual_epsilon,
    );
    if result.verification != expected_verification
        || result.manifest.verification != expected_verification
    {
        return Err(invalid_package(
            Path::new(VERIFICATION_FILE),
            "decompile result verification metadata is inconsistent",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct MeshPayload {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn ensure_payload_counts(
    asset: &MeshAsset,
    payload: &MeshPayload,
    path: &Path,
) -> Result<(), DecompileError> {
    if asset.vertex_count != payload.positions.len()
        || asset.triangle_count != payload.indices.len() / 3
    {
        return Err(invalid_package(
            path,
            format!(
                "manifest counts ({}, {} triangles) do not match payload counts ({}, {} triangles)",
                asset.vertex_count,
                asset.triangle_count,
                payload.positions.len(),
                payload.indices.len() / 3
            ),
        ));
    }
    Ok(())
}

fn resolve_package_asset(root: &Path, relative: &str) -> Result<PathBuf, DecompileError> {
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
            root,
            format!("unsafe package-relative path '{relative}'"),
        ));
    }

    let canonical_root = fs::canonicalize(root).map_err(|source| path_io(root, source))?;
    let joined = root.join(relative_path);
    let canonical_path = fs::canonicalize(&joined).map_err(|source| path_io(&joined, source))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(invalid_package(
            &joined,
            format!("package asset '{relative}' resolves outside the package root"),
        ));
    }
    Ok(canonical_path)
}

fn read_meshbin(path: &Path) -> Result<MeshPayload, DecompileError> {
    let bytes = fs::read(path).map_err(|source| path_io(path, source))?;
    if bytes.len() < MESHBIN_MAGIC.len() + 16 {
        return Err(invalid_package(path, "meshbin header is truncated"));
    }
    if &bytes[..MESHBIN_MAGIC.len()] != MESHBIN_MAGIC {
        return Err(invalid_package(path, "unsupported meshbin magic"));
    }
    let mut offset = MESHBIN_MAGIC.len();
    let vertex_count = read_le_u64(&bytes, &mut offset, path)?;
    let index_count = read_le_u64(&bytes, &mut offset, path)?;
    let vertex_count = usize::try_from(vertex_count)
        .map_err(|_| invalid_package(path, "vertex count does not fit this platform"))?;
    let index_count = usize::try_from(index_count)
        .map_err(|_| invalid_package(path, "index count does not fit this platform"))?;
    if vertex_count == 0 {
        return Err(invalid_package(path, "meshbin contains no vertices"));
    }
    if index_count == 0 || !index_count.is_multiple_of(3) {
        return Err(invalid_package(
            path,
            "meshbin index count must describe at least one triangle",
        ));
    }
    if vertex_count > u32::MAX as usize {
        return Err(invalid_package(
            path,
            "meshbin vertex count exceeds u32 topology storage",
        ));
    }
    let position_bytes = vertex_count
        .checked_mul(12)
        .ok_or_else(|| invalid_package(path, "meshbin position byte count overflow"))?;
    let index_bytes = index_count
        .checked_mul(4)
        .ok_or_else(|| invalid_package(path, "meshbin index byte count overflow"))?;
    let expected_len = offset
        .checked_add(position_bytes)
        .and_then(|value| value.checked_add(index_bytes))
        .ok_or_else(|| invalid_package(path, "meshbin total byte count overflow"))?;
    if bytes.len() != expected_len {
        return Err(invalid_package(
            path,
            format!(
                "meshbin byte length is {}; expected {expected_len}",
                bytes.len()
            ),
        ));
    }

    let mut positions = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let position = [
            read_le_f32(&bytes, &mut offset, path)?,
            read_le_f32(&bytes, &mut offset, path)?,
            read_le_f32(&bytes, &mut offset, path)?,
        ];
        if !array_is_finite(position) {
            return Err(invalid_package(
                path,
                "meshbin contains a non-finite position",
            ));
        }
        positions.push(position);
    }
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        let index = read_le_u32(&bytes, &mut offset, path)?;
        if index as usize >= vertex_count {
            return Err(invalid_package(
                path,
                "meshbin contains an out-of-range triangle index",
            ));
        }
        indices.push(index);
    }
    for triangle in indices.chunks_exact(3) {
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(invalid_package(
                path,
                "meshbin contains a triangle with repeated vertex indices",
            ));
        }
    }
    Ok(MeshPayload { positions, indices })
}

fn read_positions(path: &Path, count: usize) -> Result<Vec<[f32; 3]>, DecompileError> {
    let bytes = fs::read(path).map_err(|source| path_io(path, source))?;
    let expected = count
        .checked_mul(12)
        .ok_or_else(|| invalid_package(path, "position payload byte count overflow"))?;
    if bytes.len() != expected {
        return Err(invalid_package(
            path,
            format!(
                "position payload has {} bytes; expected {expected}",
                bytes.len()
            ),
        ));
    }
    let mut offset = 0;
    let mut positions = Vec::with_capacity(count);
    for _ in 0..count {
        let position = [
            read_le_f32(&bytes, &mut offset, path)?,
            read_le_f32(&bytes, &mut offset, path)?,
            read_le_f32(&bytes, &mut offset, path)?,
        ];
        if !array_is_finite(position) {
            return Err(invalid_package(
                path,
                "position payload contains a non-finite value",
            ));
        }
        positions.push(position);
    }
    Ok(positions)
}

fn read_u32s(path: &Path) -> Result<Vec<u32>, DecompileError> {
    let bytes = fs::read(path).map_err(|source| path_io(path, source))?;
    if !bytes.len().is_multiple_of(4) {
        return Err(invalid_package(
            path,
            "u32 payload byte count is not divisible by four",
        ));
    }
    let mut offset = 0;
    let mut values = Vec::with_capacity(bytes.len() / 4);
    while offset < bytes.len() {
        values.push(read_le_u32(&bytes, &mut offset, path)?);
    }
    Ok(values)
}

fn read_le_u64(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u64, DecompileError> {
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

fn read_le_u32(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<u32, DecompileError> {
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

fn read_le_f32(bytes: &[u8], offset: &mut usize, path: &Path) -> Result<f32, DecompileError> {
    Ok(f32::from_bits(read_le_u32(bytes, offset, path)?))
}

fn ensure_strictly_increasing_indices(
    indices: &[u32],
    vertex_count: usize,
    path: &Path,
) -> Result<(), DecompileError> {
    let mut previous = None;
    for index in indices {
        if *index as usize >= vertex_count {
            return Err(invalid_package(
                path,
                "residual vertex index is out of range",
            ));
        }
        if previous.is_some_and(|value| *index <= value) {
            return Err(invalid_package(
                path,
                "residual indices must be unique and strictly increasing",
            ));
        }
        previous = Some(*index);
    }
    Ok(())
}

fn fit_affine(source: &[[f32; 3]], target: &[[f32; 3]]) -> Option<[f32; 16]> {
    if source.is_empty() || source.len() != target.len() {
        return None;
    }
    if let Some(matrix) = exact_translation_matrix(source, target) {
        return Some(matrix);
    }

    let source_center = centroid_f64(source);
    let mut source_scale = 0.0_f64;
    for position in source {
        for axis in 0..3 {
            source_scale =
                source_scale.max((f64::from(position[axis]) - source_center[axis]).abs());
        }
    }
    if !source_scale.is_finite() || source_scale <= f64::EPSILON {
        let target_center = centroid_f64(target);
        let translation = [
            target_center[0] - source_center[0],
            target_center[1] - source_center[1],
            target_center[2] - source_center[2],
        ];
        let matrix = [
            1.0,
            0.0,
            0.0,
            translation[0] as f32,
            0.0,
            1.0,
            0.0,
            translation[1] as f32,
            0.0,
            0.0,
            1.0,
            translation[2] as f32,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        return matrix
            .iter()
            .all(|value| value.is_finite())
            .then_some(matrix);
    }

    let mut normal = [[0.0_f64; 4]; 4];
    let mut rhs = [[0.0_f64; 4]; 3];
    for (source_position, target_position) in source.iter().zip(target) {
        let p = [
            (f64::from(source_position[0]) - source_center[0]) / source_scale,
            (f64::from(source_position[1]) - source_center[1]) / source_scale,
            (f64::from(source_position[2]) - source_center[2]) / source_scale,
            1.0,
        ];
        for row in 0..4 {
            for col in 0..4 {
                normal[row][col] += p[row] * p[col];
            }
            rhs[0][row] += p[row] * f64::from(target_position[0]);
            rhs[1][row] += p[row] * f64::from(target_position[1]);
            rhs[2][row] += p[row] * f64::from(target_position[2]);
        }
    }

    let x = solve_symmetric_pseudoinverse(normal, rhs[0])?;
    let y = solve_symmetric_pseudoinverse(normal, rhs[1])?;
    let z = solve_symmetric_pseudoinverse(normal, rhs[2])?;
    let rows = [x, y, z];
    let mut matrix = [0.0_f32; 16];
    for (row_index, row) in rows.iter().enumerate() {
        let linear = [
            row[0] / source_scale,
            row[1] / source_scale,
            row[2] / source_scale,
        ];
        let translation = row[3]
            - linear[0] * source_center[0]
            - linear[1] * source_center[1]
            - linear[2] * source_center[2];
        let offset = row_index * 4;
        matrix[offset] = linear[0] as f32;
        matrix[offset + 1] = linear[1] as f32;
        matrix[offset + 2] = linear[2] as f32;
        matrix[offset + 3] = translation as f32;
    }
    matrix[15] = 1.0;
    matrix
        .iter()
        .all(|value| value.is_finite())
        .then_some(matrix)
}

fn exact_translation_matrix(source: &[[f32; 3]], target: &[[f32; 3]]) -> Option<[f32; 16]> {
    let delta = [
        target[0][0] - source[0][0],
        target[0][1] - source[0][1],
        target[0][2] - source[0][2],
    ];
    if !array_is_finite(delta) {
        return None;
    }
    let exact = source.iter().zip(target).all(|(source, target)| {
        (0..3).all(|axis| {
            canonical_f32_add(source[axis], delta[axis]).to_bits() == target[axis].to_bits()
        })
    });
    exact.then_some([
        1.0, 0.0, 0.0, delta[0], 0.0, 1.0, 0.0, delta[1], 0.0, 0.0, 1.0, delta[2], 0.0, 0.0, 0.0,
        1.0,
    ])
}

fn centroid_f64(positions: &[[f32; 3]]) -> [f64; 3] {
    let mut total = [0.0_f64; 3];
    for position in positions {
        total[0] += f64::from(position[0]);
        total[1] += f64::from(position[1]);
        total[2] += f64::from(position[2]);
    }
    let divisor = positions.len().max(1) as f64;
    [total[0] / divisor, total[1] / divisor, total[2] / divisor]
}

fn solve_symmetric_pseudoinverse(matrix: [[f64; 4]; 4], rhs: [f64; 4]) -> Option<[f64; 4]> {
    let (eigenvalues, eigenvectors) = symmetric_eigendecomposition(matrix);
    let largest = eigenvalues
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    if !largest.is_finite() || largest <= f64::EPSILON {
        return None;
    }
    let threshold = largest * PSEUDOINVERSE_RELATIVE_EPSILON;
    let mut solution = [0.0_f64; 4];
    for (column, eigenvalue) in eigenvalues.iter().copied().enumerate() {
        if !eigenvalue.is_finite() || eigenvalue.abs() <= threshold {
            continue;
        }
        let projection = (0..4)
            .map(|row| eigenvectors[row][column] * rhs[row])
            .sum::<f64>();
        let coefficient = projection / eigenvalue;
        for row in 0..4 {
            solution[row] += eigenvectors[row][column] * coefficient;
        }
    }
    solution
        .iter()
        .all(|value| value.is_finite())
        .then_some(solution)
}

fn symmetric_eigendecomposition(mut matrix: [[f64; 4]; 4]) -> ([f64; 4], [[f64; 4]; 4]) {
    let mut eigenvectors = [[0.0_f64; 4]; 4];
    for (index, row) in eigenvectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    for _ in 0..JACOBI_MAX_ITERATIONS {
        let mut pivot_row = 0;
        let mut pivot_col = 1;
        let mut largest_off_diagonal = 0.0_f64;
        for (row, values) in matrix.iter().enumerate() {
            for (col, value) in values.iter().enumerate().skip(row + 1) {
                let candidate = value.abs();
                if candidate > largest_off_diagonal {
                    largest_off_diagonal = candidate;
                    pivot_row = row;
                    pivot_col = col;
                }
            }
        }
        let diagonal_scale = (0..4)
            .map(|index| matrix[index][index].abs())
            .fold(1.0_f64, f64::max);
        if largest_off_diagonal <= diagonal_scale * PSEUDOINVERSE_RELATIVE_EPSILON {
            break;
        }

        let p = pivot_row;
        let q = pivot_col;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let angle = 0.5 * (2.0 * apq).atan2(aqq - app);
        let cosine = angle.cos();
        let sine = angle.sin();

        for index in [0_usize, 1, 2, 3] {
            if index == p || index == q {
                continue;
            }
            let aip = matrix[index][p];
            let aiq = matrix[index][q];
            let rotated_p = cosine * aip - sine * aiq;
            let rotated_q = sine * aip + cosine * aiq;
            matrix[index][p] = rotated_p;
            matrix[p][index] = rotated_p;
            matrix[index][q] = rotated_q;
            matrix[q][index] = rotated_q;
        }
        matrix[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        matrix[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for row in &mut eigenvectors {
            let vip = row[p];
            let viq = row[q];
            row[p] = cosine * vip - sine * viq;
            row[q] = sine * vip + cosine * viq;
        }
    }

    let eigenvalues = [matrix[0][0], matrix[1][1], matrix[2][2], matrix[3][3]];
    (eigenvalues, eigenvectors)
}

fn apply_affine_to_positions(positions: &[[f32; 3]], matrix: [f32; 16]) -> Vec<[f32; 3]> {
    positions
        .iter()
        .map(|position| {
            [
                apply_affine_row(*position, &matrix[0..4]),
                apply_affine_row(*position, &matrix[4..8]),
                apply_affine_row(*position, &matrix[8..12]),
            ]
        })
        .collect()
}

/// Evaluate one affine row using the package's canonical arithmetic contract.
///
/// Performing each binary operation in `f64` and explicitly narrowing its
/// result to `f32` gives a deterministic IEEE-754 binary32 rounding boundary
/// after every multiplication and addition. It also prevents compiler or CPU
/// fused-multiply-add contraction from changing serialized stage positions.
fn apply_affine_row(position: [f32; 3], row: &[f32]) -> f32 {
    let mut value = canonical_f32_mul(row[0], position[0]);
    value = canonical_f32_add(value, canonical_f32_mul(row[1], position[1]));
    value = canonical_f32_add(value, canonical_f32_mul(row[2], position[2]));
    canonical_f32_add(value, row[3])
}

fn canonical_f32_mul(left: f32, right: f32) -> f32 {
    (f64::from(left) * f64::from(right)) as f32
}

fn canonical_f32_add(left: f32, right: f32) -> f32 {
    (f64::from(left) + f64::from(right)) as f32
}

fn identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn sum_squared_distance(left: &[[f32; 3]], right: &[[f32; 3]]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| {
            let dx = f64::from(a[0]) - f64::from(b[0]);
            let dy = f64::from(a[1]) - f64::from(b[1]);
            let dz = f64::from(a[2]) - f64::from(b[2]);
            dx * dx + dy * dy + dz * dz
        })
        .sum()
}

fn explained_fraction(identity_error: f64, candidate_error: f64) -> f64 {
    if identity_error <= f64::EPSILON {
        1.0
    } else {
        (1.0 - candidate_error / identity_error).clamp(0.0, 1.0)
    }
}

fn max_euclidean_distance(left: &[[f32; 3]], right: &[[f32; 3]]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(a, b)| euclidean_distance(*a, *b))
        .fold(0.0, f32::max)
}

fn verify_positions(
    reconstructed: &[[f32; 3]],
    target: &[[f32; 3]],
    triangle_count: usize,
    tolerance: f32,
) -> VerificationReport {
    let mut max_component_error = 0.0_f32;
    let mut max_euclidean_error = 0.0_f32;
    let mut total_euclidean_error = 0.0_f64;
    let mut total_squared_euclidean_error = 0.0_f64;
    let mut outside_tolerance = 0;

    for (left, right) in reconstructed.iter().zip(target) {
        let component = max_component_distance(*left, *right);
        let euclidean = euclidean_distance(*left, *right);
        max_component_error = max_component_error.max(component);
        max_euclidean_error = max_euclidean_error.max(euclidean);
        total_euclidean_error += f64::from(euclidean);
        total_squared_euclidean_error += f64::from(euclidean) * f64::from(euclidean);
        if euclidean > tolerance {
            outside_tolerance += 1;
        }
    }

    let count = reconstructed.len().max(1) as f64;
    VerificationReport {
        topology_exact: true,
        vertex_count: reconstructed.len(),
        triangle_count,
        max_component_error,
        max_euclidean_error,
        mean_euclidean_error: (total_euclidean_error / count) as f32,
        rms_euclidean_error: (total_squared_euclidean_error / count).sqrt() as f32,
        tolerance,
        outside_tolerance,
    }
}

fn euclidean_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = f64::from(left[0]) - f64::from(right[0]);
    let dy = f64::from(left[1]) - f64::from(right[1]);
    let dz = f64::from(left[2]) - f64::from(right[2]);
    (dx * dx + dy * dy + dz * dz).sqrt() as f32
}

fn max_component_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    (left[0] - right[0])
        .abs()
        .max((left[1] - right[1]).abs())
        .max((left[2] - right[2]).abs())
}

fn f32_bits_equal(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

fn positions_bit_equal(left: [f32; 3], right: [f32; 3]) -> bool {
    left[0].to_bits() == right[0].to_bits()
        && left[1].to_bits() == right[1].to_bits()
        && left[2].to_bits() == right[2].to_bits()
}

fn position_slices_bit_equal(left: &[[f32; 3]], right: &[[f32; 3]]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| positions_bit_equal(*left, *right))
}

fn topology_hash(mesh: &TriangleMesh) -> String {
    topology_hash_from_parts(mesh.positions.len(), &mesh.indices)
}

fn topology_hash_from_parts(vertex_count: usize, indices: &[u32]) -> String {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, vertex_count as u64);
    hash = hash_u64(hash, indices.len() as u64);
    for index in indices {
        hash = hash_u32(hash, *index);
    }
    format!("fnv1a64:{hash:016x}")
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn write_meshbin(path: &Path, mesh: &TriangleMesh) -> Result<(), DecompileError> {
    let file = File::create(path).map_err(|source| path_io(path, source))?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(MESHBIN_MAGIC)
        .map_err(|source| path_io(path, source))?;
    write_u64(&mut writer, mesh.positions.len() as u64, path)?;
    write_u64(&mut writer, mesh.indices.len() as u64, path)?;
    for position in &mesh.positions {
        for component in position {
            write_f32(&mut writer, *component, path)?;
        }
    }
    for index in &mesh.indices {
        write_u32(&mut writer, *index, path)?;
    }
    writer.flush().map_err(|source| path_io(path, source))
}

fn write_positions(path: &Path, positions: &[[f32; 3]]) -> Result<(), DecompileError> {
    let file = File::create(path).map_err(|source| path_io(path, source))?;
    let mut writer = BufWriter::new(file);
    for position in positions {
        for component in position {
            write_f32(&mut writer, *component, path)?;
        }
    }
    writer.flush().map_err(|source| path_io(path, source))
}

fn write_u32s(path: &Path, values: &[u32]) -> Result<(), DecompileError> {
    let file = File::create(path).map_err(|source| path_io(path, source))?;
    let mut writer = BufWriter::new(file);
    for value in values {
        write_u32(&mut writer, *value, path)?;
    }
    writer.flush().map_err(|source| path_io(path, source))
}

fn write_f32(writer: &mut impl Write, value: f32, path: &Path) -> Result<(), DecompileError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|source| path_io(path, source))
}

fn write_u32(writer: &mut impl Write, value: u32, path: &Path) -> Result<(), DecompileError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|source| path_io(path, source))
}

fn write_u64(writer: &mut impl Write, value: u64, path: &Path) -> Result<(), DecompileError> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|source| path_io(path, source))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), DecompileError> {
    let json = serde_json::to_string_pretty(value)?;
    write_text(path, &json)
}

fn write_text(path: &Path, text: &str) -> Result<(), DecompileError> {
    fs::write(path, text).map_err(|source| path_io(path, source))
}

fn package_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

fn path_io(path: &Path, source: std::io::Error) -> DecompileError {
    DecompileError::PathIo {
        path: path.to_path_buf(),
        source,
    }
}

fn invalid_mesh(mesh_name: &'static str, message: impl Into<String>) -> DecompileError {
    DecompileError::InvalidMesh {
        mesh_name,
        message: message.into(),
    }
}

fn invalid_package(path: impl AsRef<Path>, message: impl Into<String>) -> DecompileError {
    DecompileError::InvalidPackage {
        path: path.as_ref().to_path_buf(),
        message: message.into(),
    }
}

fn array_is_finite(value: [f32; 3]) -> bool {
    value[0].is_finite() && value[1].is_finite() && value[2].is_finite()
}

fn blender_reconstruction_script() -> &'static str {
    r####"# Generated by Shape Lab's lossless deformation decompiler.
from pathlib import Path
import argparse
import json
import math
import struct
import sys
import bpy

ROOT = Path(__file__).resolve().parent
SUPPORTED_SCHEMA_VERSION = 2
SOURCE_OBJECT_NAME = "ShapeLab_Decompiled"
BAKED_OBJECT_NAME = "ShapeLab_Reconstructed_Baked"
VERTEX_ID_ATTRIBUTE = "shapelab_vertex_id"
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x00000100000001B3


def command_line_arguments():
    parser = argparse.ArgumentParser(description="Reconstruct and verify a Shape Lab package")
    parser.add_argument(
        "--verify-existing",
        action="store_true",
        help="verify the baked object already stored in the opened .blend file",
    )
    parser.add_argument(
        "--output-blend",
        default="reconstructed.blend",
        help="output .blend path, relative to the package unless absolute",
    )
    parser.add_argument(
        "--report",
        default="blender-verification.json",
        help="verification JSON path, relative to the package unless absolute",
    )
    parser.add_argument("--no-save", action="store_true", help="do not save a .blend file")
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    return parser.parse_args(argv)


def output_path(value):
    path = Path(value)
    return path if path.is_absolute() else ROOT / path


def package_path(relative_path):
    relative = Path(relative_path)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError(f"unsafe package-relative path: {relative_path}")
    path = (ROOT / relative).resolve()
    try:
        path.relative_to(ROOT.resolve())
    except ValueError as error:
        raise ValueError(f"package path escapes package root: {relative_path}") from error
    return path


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def f32_bits(value):
    return struct.pack("<f", value)


def read_meshbin(relative_path):
    path = package_path(relative_path)
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"SLMBIN01":
        raise ValueError(f"unsupported or truncated meshbin: {relative_path}")
    vertex_count, index_count = struct.unpack_from("<QQ", data, 8)
    if vertex_count == 0 or index_count == 0 or index_count % 3 != 0:
        raise ValueError(f"invalid counts in {relative_path}")
    expected = 24 + vertex_count * 12 + index_count * 4
    if len(data) != expected:
        raise ValueError(f"{relative_path} has {len(data)} bytes; expected {expected}")
    offset = 24
    positions = []
    for _ in range(vertex_count):
        position = struct.unpack_from("<fff", data, offset)
        if not all(math.isfinite(component) for component in position):
            raise ValueError(f"non-finite position in {relative_path}")
        positions.append(position)
        offset += 12
    indices = list(struct.unpack_from(f"<{index_count}I", data, offset))
    if any(index >= vertex_count for index in indices):
        raise ValueError(f"out-of-range triangle index in {relative_path}")
    for offset in range(0, len(indices), 3):
        triangle = indices[offset : offset + 3]
        if len(set(triangle)) != 3:
            raise ValueError(f"triangle with repeated indices in {relative_path}")
    return positions, indices


def read_positions(relative_path, count):
    path = package_path(relative_path)
    data = path.read_bytes()
    expected = count * 12
    if len(data) != expected:
        raise ValueError(f"{relative_path} has {len(data)} bytes; expected {expected}")
    positions = [struct.unpack_from("<fff", data, index * 12) for index in range(count)]
    if not all(math.isfinite(component) for position in positions for component in position):
        raise ValueError(f"non-finite position in {relative_path}")
    return positions


def read_u32s(relative_path):
    path = package_path(relative_path)
    data = path.read_bytes()
    if len(data) % 4 != 0:
        raise ValueError(f"{relative_path} byte length is not divisible by four")
    return list(struct.unpack(f"<{len(data) // 4}I", data)) if data else []


def fnv1a_update(value, payload):
    for byte in payload:
        value ^= byte
        value = (value * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return value


def topology_hash(vertex_count, indices):
    value = FNV_OFFSET
    value = fnv1a_update(value, struct.pack("<Q", vertex_count))
    value = fnv1a_update(value, struct.pack("<Q", len(indices)))
    for index in indices:
        value = fnv1a_update(value, struct.pack("<I", index))
    return f"fnv1a64:{value:016x}"


def apply_affine(positions, matrix):
    if len(matrix) != 16 or not all(math.isfinite(value) for value in matrix):
        raise ValueError("global affine matrix must contain sixteen finite values")
    # JSON numbers are Python binary64 values. Normalize every serialized
    # matrix coefficient back to its declared binary32 value before applying
    # the package's stepwise, non-fused arithmetic contract.
    matrix = [f32(value) for value in matrix]
    result = []
    for x, y, z in positions:
        transformed = []
        for offset in (0, 4, 8):
            value = f32(matrix[offset] * x)
            value = f32(value + f32(matrix[offset + 1] * y))
            value = f32(value + f32(matrix[offset + 2] * z))
            value = f32(value + matrix[offset + 3])
            transformed.append(value)
        result.append(tuple(transformed))
    return result


def positions_bit_equal(left, right):
    return len(left) == len(right) and all(
        f32_bits(a) == f32_bits(b)
        for left_position, right_position in zip(left, right)
        for a, b in zip(left_position, right_position)
    )


def sum_squared_distance(left, right):
    return sum(
        sum((float(a) - float(b)) ** 2 for a, b in zip(left_position, right_position))
        for left_position, right_position in zip(left, right)
    )


def max_euclidean_distance(left, right):
    return max(
        (
            f32(
                math.sqrt(
                    sum(
                        (float(a) - float(b)) ** 2
                        for a, b in zip(left_position, right_position)
                    )
                )
            )
            for left_position, right_position in zip(left, right)
        ),
        default=0.0,
    )


def explained_fraction(identity_error, candidate_error):
    if identity_error <= sys.float_info.epsilon:
        return 1.0
    return max(0.0, min(1.0, 1.0 - candidate_error / identity_error))


def replay_operators(manifest, source_positions, target_positions):
    current = list(source_positions)
    stages = []
    saw_lossless = False
    operator_ids = set()
    operator_labels = set()
    identity_error = sum_squared_distance(source_positions, target_positions)
    for operator_index, operator in enumerate(manifest["operators"]):
        if saw_lossless:
            raise ValueError("the lossless correction must be the final operator")
        operator_id = operator.get("id", "")
        label = operator.get("label", "")
        if not operator_id.strip() or not label.strip():
            raise ValueError("operator IDs and labels must not be empty")
        if operator_id in operator_ids:
            raise ValueError(f"duplicate operator ID: {operator_id}")
        if label == "Basis" or label in operator_labels:
            raise ValueError(f"operator label is reserved or duplicated: {label}")
        operator_ids.add(operator_id)
        operator_labels.add(label)

        kind = operator["kind"]
        if kind == "global_affine":
            if operator_index != 0:
                raise ValueError("the global affine operator must be first")
            matrix = operator["matrix_row_major_4x4"]
            if len(matrix) != 16 or not all(math.isfinite(value) for value in matrix):
                raise ValueError("global affine matrix must contain sixteen finite values")
            if any(
                f32_bits(actual) != f32_bits(expected)
                for actual, expected in zip(matrix[12:16], (0.0, 0.0, 0.0, 1.0))
            ):
                raise ValueError("global affine matrix bottom row must be [0, 0, 0, 1]")
            baked = read_positions(operator["baked_positions_file"], len(source_positions))
            evaluated = apply_affine(source_positions, matrix)
            if not positions_bit_equal(evaluated, baked):
                raise ValueError("baked affine positions do not match the serialized matrix")
            affine_error = sum_squared_distance(evaluated, target_positions)
            expected_explained = f32(explained_fraction(identity_error, affine_error))
            expected_max_error = max_euclidean_distance(evaluated, target_positions)
            if f32_bits(operator["explained_displacement_fraction"]) != f32_bits(
                expected_explained
            ):
                raise ValueError("global affine explained-displacement metadata is inconsistent")
            if f32_bits(operator["max_remaining_error"]) != f32_bits(expected_max_error):
                raise ValueError("global affine remaining-error metadata is inconsistent")
            if (
                identity_error <= 0.0
                or affine_error >= identity_error
                or expected_explained
                < f32(manifest["settings"]["affine_min_explained"])
            ):
                raise ValueError("global affine does not satisfy its emission threshold")
            current = baked
            stages.append((label, list(current)))
        elif kind == "lossless_correction":
            saw_lossless = True
            indices = read_u32s(operator["residual_index_file"])
            positions = read_positions(operator["residual_position_file"], len(indices))
            if len(indices) != operator["corrected_vertex_count"]:
                raise ValueError("lossless correction count does not match its payload")
            if any(index >= len(current) for index in indices):
                raise ValueError("lossless correction contains an out-of-range vertex index")
            if any(left >= right for left, right in zip(indices, indices[1:])):
                raise ValueError("lossless correction indices must be unique and increasing")
            current = list(current)
            for vertex_index, position in zip(indices, positions):
                if positions_bit_equal([current[vertex_index]], [target_positions[vertex_index]]):
                    raise ValueError(
                        f"lossless correction contains no-op vertex index {vertex_index}"
                    )
                current[vertex_index] = position
            expected_after = max_euclidean_distance(current, target_positions)
            if f32_bits(operator["max_error_after"]) != f32_bits(expected_after):
                raise ValueError("lossless correction max-error metadata is inconsistent")
            stages.append((label, list(current)))
        else:
            raise ValueError(f"unsupported operator kind: {kind}")
    if not saw_lossless:
        raise ValueError("package is missing its final lossless correction")
    return current, stages


def faces_from_indices(indices):
    return [tuple(indices[index : index + 3]) for index in range(0, len(indices), 3)]


def remove_object(name):
    obj = bpy.data.objects.get(name)
    if obj is not None:
        bpy.data.objects.remove(obj, do_unlink=True)


def add_vertex_ids(mesh):
    attribute = mesh.attributes.get(VERTEX_ID_ATTRIBUTE)
    if attribute is None:
        attribute = mesh.attributes.new(name=VERTEX_ID_ATTRIBUTE, type="INT", domain="POINT")
    if len(attribute.data) != len(mesh.vertices):
        raise RuntimeError("vertex ID attribute has the wrong size")
    for index, value in enumerate(attribute.data):
        value.value = index


def create_mesh_object(name, positions, indices):
    mesh = bpy.data.meshes.new(f"{name}_Mesh")
    mesh.from_pydata(positions, [], faces_from_indices(indices))
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    add_vertex_ids(mesh)
    return obj


def set_shape_key_positions(key, positions):
    if len(key.data) != len(positions):
        raise ValueError("shape key vertex count does not match position payload")
    for vertex, position in zip(key.data, positions):
        vertex.co = position


def mesh_arrays(mesh):
    positions = [tuple(f32(component) for component in vertex.co) for vertex in mesh.vertices]
    indices = []
    for polygon in mesh.polygons:
        vertices = tuple(polygon.vertices)
        if len(vertices) != 3:
            raise ValueError("reconstructed Blender mesh contains a non-triangular face")
        indices.extend(vertices)
    return positions, indices


def vertex_ids_exact(mesh):
    attribute = mesh.attributes.get(VERTEX_ID_ATTRIBUTE)
    return attribute is not None and len(attribute.data) == len(mesh.vertices) and all(
        value.value == index for index, value in enumerate(attribute.data)
    )


def verification_metrics(positions, target_positions, indices, target_indices, tolerance):
    if len(positions) != len(target_positions):
        topology_exact = False
    else:
        topology_exact = indices == target_indices
    max_component_error = 0.0
    max_euclidean_error = 0.0
    total_euclidean_error = 0.0
    total_squared_euclidean_error = 0.0
    outside_tolerance = 0
    positions_bit_exact = positions_bit_equal(positions, target_positions)
    if len(positions) == len(target_positions):
        for left, right in zip(positions, target_positions):
            differences = [abs(float(a) - float(b)) for a, b in zip(left, right)]
            euclidean = math.sqrt(sum(value * value for value in differences))
            max_component_error = max(max_component_error, max(differences))
            max_euclidean_error = max(max_euclidean_error, euclidean)
            total_euclidean_error += euclidean
            total_squared_euclidean_error += euclidean * euclidean
            if euclidean > tolerance:
                outside_tolerance += 1
    count = max(len(positions), 1)
    return {
        "topology_exact": topology_exact,
        "positions_bit_exact": positions_bit_exact,
        "vertex_count": len(positions),
        "triangle_count": len(indices) // 3,
        "max_component_error": max_component_error,
        "max_euclidean_error": max_euclidean_error,
        "mean_euclidean_error": total_euclidean_error / count,
        "rms_euclidean_error": math.sqrt(total_squared_euclidean_error / count),
        "tolerance": tolerance,
        "outside_tolerance": outside_tolerance,
    }


def load_and_validate_package():
    manifest = json.loads(package_path("manifest.json").read_text(encoding="utf-8"))
    if manifest.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
        raise ValueError(
            f"unsupported schema version {manifest.get('schema_version')}; "
            f"expected {SUPPORTED_SCHEMA_VERSION}"
        )
    if manifest.get("coordinate_system") != {"handedness": "right", "up_axis": "y"}:
        raise ValueError("unsupported coordinate system; expected right-handed Y-up")
    if manifest.get("numeric_format") != {
        "scalar": "float32",
        "endian": "little",
        "affine_evaluation": "float32_stepwise_no_fma",
    }:
        raise ValueError(
            "unsupported numeric format; expected little-endian float32 with "
            "stepwise non-fused affine arithmetic"
        )
    settings = manifest.get("settings", {})
    affine_min_explained = settings.get("affine_min_explained")
    residual_epsilon = settings.get("residual_epsilon")
    if (
        not isinstance(affine_min_explained, (int, float))
        or not math.isfinite(affine_min_explained)
        or not 0.0 <= affine_min_explained <= 1.0
    ):
        raise ValueError("affine_min_explained must be finite and between zero and one")
    if (
        not isinstance(residual_epsilon, (int, float))
        or not math.isfinite(residual_epsilon)
        or residual_epsilon < 0.0
    ):
        raise ValueError("residual_epsilon must be finite and non-negative")
    affine_min_explained = f32(affine_min_explained)
    residual_epsilon = f32(residual_epsilon)
    operators = manifest.get("operators")
    if not isinstance(operators, list) or not 1 <= len(operators) <= 2:
        raise ValueError("schema version 2 requires one or two operators")

    standalone_verification = json.loads(
        package_path("verification.json").read_text(encoding="utf-8")
    )
    if standalone_verification != manifest.get("verification"):
        raise ValueError("verification.json does not match manifest.json")

    source_positions, source_indices = read_meshbin(manifest["source"]["path"])
    target_positions, target_indices = read_meshbin(manifest["target"]["path"])
    if source_indices != target_indices or len(source_positions) != len(target_positions):
        raise ValueError("source and target meshbin topology differs")
    expected_hash = topology_hash(len(source_positions), source_indices)
    if expected_hash != manifest["topology"]["hash"]:
        raise ValueError("manifest topology fingerprint does not match package payload")
    if manifest["source"]["vertex_count"] != len(source_positions):
        raise ValueError("source vertex count does not match manifest")
    if manifest["source"]["triangle_count"] != len(source_indices) // 3:
        raise ValueError("source triangle count does not match manifest")
    if manifest["target"]["vertex_count"] != len(target_positions):
        raise ValueError("target vertex count does not match manifest")
    if manifest["target"]["triangle_count"] != len(target_indices) // 3:
        raise ValueError("target triangle count does not match manifest")
    topology = manifest["topology"]
    if (
        topology["vertex_count"] != len(source_positions)
        or topology["triangle_count"] != len(source_indices) // 3
        or topology["index_count"] != len(source_indices)
    ):
        raise ValueError("topology counts do not match source.meshbin")

    verification = manifest["verification"]
    if not verification.get("topology_exact"):
        raise ValueError("same-topology package must declare exact topology")
    if verification["vertex_count"] != len(source_positions):
        raise ValueError("verification vertex count does not match package")
    if verification["triangle_count"] != len(source_indices) // 3:
        raise ValueError("verification triangle count does not match package")
    if f32_bits(verification["tolerance"]) != f32_bits(residual_epsilon):
        raise ValueError("verification tolerance does not match residual_epsilon")
    for field in (
        "max_component_error",
        "max_euclidean_error",
        "mean_euclidean_error",
        "rms_euclidean_error",
    ):
        value = verification[field]
        if not math.isfinite(value) or value < 0.0:
            raise ValueError(f"verification field {field} must be finite and non-negative")

    final_positions, stages = replay_operators(
        manifest, source_positions, target_positions
    )
    if not positions_bit_equal(final_positions, target_positions):
        raise ValueError("serialized operators do not reconstruct target positions exactly")
    replay_metrics = verification_metrics(
        final_positions,
        target_positions,
        source_indices,
        target_indices,
        residual_epsilon,
    )
    for field in (
        "max_component_error",
        "max_euclidean_error",
        "mean_euclidean_error",
        "rms_euclidean_error",
        "tolerance",
    ):
        if f32_bits(replay_metrics[field]) != f32_bits(verification[field]):
            raise ValueError(f"verification field {field} does not match replayed data")
    if replay_metrics["outside_tolerance"] != verification["outside_tolerance"]:
        raise ValueError("outside_tolerance does not match replayed data")
    return manifest, source_positions, source_indices, target_positions, target_indices, stages


def report_for_object(obj, manifest, target_positions, target_indices, mode):
    positions, indices = mesh_arrays(obj.data)
    tolerance = float(manifest["verification"]["tolerance"])
    metrics = verification_metrics(positions, target_positions, indices, target_indices, tolerance)
    metrics.update(
        {
            "mode": mode,
            "blender_version": bpy.app.version_string,
            "object_name": obj.name,
            "topology_hash": topology_hash(len(positions), indices),
            "topology_hash_matches_manifest": topology_hash(len(positions), indices)
            == manifest["topology"]["hash"],
            "vertex_ids_exact": vertex_ids_exact(obj.data),
            "object_topology_property_matches": obj.get("shape_lab_topology_hash")
            == manifest["topology"]["hash"],
            "object_schema_property_matches": obj.get("shape_lab_schema_version")
            == manifest["schema_version"],
            "object_coordinate_property_matches": obj.get("shape_lab_coordinate_up_axis")
            == manifest["coordinate_system"]["up_axis"],
        }
    )
    metrics["verification_passed"] = all(
        [
            metrics["topology_exact"],
            metrics["positions_bit_exact"],
            metrics["topology_hash_matches_manifest"],
            metrics["vertex_ids_exact"],
            metrics["object_topology_property_matches"],
            metrics["object_schema_property_matches"],
            metrics["object_coordinate_property_matches"],
            metrics["outside_tolerance"] == 0,
        ]
    )
    return metrics


def report_for_editable_object(
    obj,
    manifest,
    source_positions,
    target_positions,
    target_indices,
    stages,
    mode,
):
    basis_positions, indices = mesh_arrays(obj.data)
    shape_keys = obj.data.shape_keys
    expected_shape_key_count = len(stages) + 1
    if shape_keys is None:
        raise ValueError("editable reconstruction is missing its cumulative shape keys")

    key_blocks = list(shape_keys.key_blocks)
    expected_names = ["Basis"] + [label for label, _positions in stages]
    actual_names = [key.name for key in key_blocks]
    shape_key_count_exact = len(key_blocks) == expected_shape_key_count
    shape_key_names_exact = actual_names == expected_names
    basis_mesh_positions_exact = positions_bit_equal(basis_positions, source_positions)

    stage_results = []
    stage_positions_exact = shape_key_count_exact
    if shape_key_count_exact:
        expected_payloads = [source_positions] + [positions for _label, positions in stages]
        for index, (key, expected_name, expected_positions) in enumerate(
            zip(key_blocks, expected_names, expected_payloads)
        ):
            actual_positions = [
                tuple(f32(component) for component in point.co) for point in key.data
            ]
            positions_exact = positions_bit_equal(actual_positions, expected_positions)
            stage_positions_exact = stage_positions_exact and positions_exact
            stage_results.append(
                {
                    "index": index,
                    "expected_name": expected_name,
                    "actual_name": key.name,
                    "positions_bit_exact": positions_exact,
                    "value": float(key.value),
                }
            )
    else:
        stage_positions_exact = False

    final_key = key_blocks[-1] if key_blocks else None
    final_positions = (
        [tuple(f32(component) for component in point.co) for point in final_key.data]
        if final_key is not None
        else []
    )
    tolerance = float(manifest["verification"]["tolerance"])
    metrics = verification_metrics(
        final_positions, target_positions, indices, target_indices, tolerance
    )
    topology_fingerprint = topology_hash(len(final_positions), indices)
    preceding_shape_key_values_zero = all(
        abs(float(key.value)) <= 1.0e-7 for key in key_blocks[1:-1]
    )
    final_shape_key_value = float(final_key.value) if final_key is not None else 0.0
    metrics.update(
        {
            "mode": mode,
            "blender_version": bpy.app.version_string,
            "object_name": obj.name,
            "topology_hash": topology_fingerprint,
            "topology_hash_matches_manifest": topology_fingerprint
            == manifest["topology"]["hash"],
            "vertex_ids_exact": vertex_ids_exact(obj.data),
            "object_topology_property_matches": obj.get("shape_lab_topology_hash")
            == manifest["topology"]["hash"],
            "object_schema_property_matches": obj.get("shape_lab_schema_version")
            == manifest["schema_version"],
            "object_coordinate_property_matches": obj.get("shape_lab_coordinate_up_axis")
            == manifest["coordinate_system"]["up_axis"],
            "shape_key_count": len(key_blocks),
            "expected_shape_key_count": expected_shape_key_count,
            "shape_key_count_exact": shape_key_count_exact,
            "shape_key_names": actual_names,
            "expected_shape_key_names": expected_names,
            "shape_key_names_exact": shape_key_names_exact,
            "basis_mesh_positions_exact": basis_mesh_positions_exact,
            "stage_positions_exact": stage_positions_exact,
            "stage_results": stage_results,
            "preceding_shape_key_values_zero": preceding_shape_key_values_zero,
            "final_shape_key_name": final_key.name if final_key is not None else None,
            "final_shape_key_value": final_shape_key_value,
        }
    )
    metrics["verification_passed"] = all(
        [
            metrics["topology_exact"],
            metrics["positions_bit_exact"],
            metrics["topology_hash_matches_manifest"],
            metrics["vertex_ids_exact"],
            metrics["object_topology_property_matches"],
            metrics["object_schema_property_matches"],
            metrics["object_coordinate_property_matches"],
            metrics["outside_tolerance"] == 0,
            metrics["shape_key_count_exact"],
            metrics["shape_key_names_exact"],
            metrics["basis_mesh_positions_exact"],
            metrics["stage_positions_exact"],
            metrics["preceding_shape_key_values_zero"],
            abs(metrics["final_shape_key_value"] - 1.0) <= 1.0e-7,
        ]
    )
    return metrics


def write_report(path, report):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def reconstruct(manifest, source_positions, source_indices, target_positions, target_indices, stages):
    remove_object(SOURCE_OBJECT_NAME)
    remove_object(BAKED_OBJECT_NAME)

    editable = create_mesh_object(SOURCE_OBJECT_NAME, source_positions, source_indices)
    bpy.context.view_layer.objects.active = editable
    editable.select_set(True)
    editable.shape_key_add(name="Basis")
    final_key = None
    for label, positions in stages:
        key = editable.shape_key_add(name=label)
        set_shape_key_positions(key, positions)
        key.value = 0.0
        final_key = key
    if final_key is None:
        raise RuntimeError("operator replay produced no inspectable stages")
    final_key.value = 1.0
    editable.active_shape_key_index = len(editable.data.shape_keys.key_blocks) - 1
    editable["shape_lab_topology_hash"] = manifest["topology"]["hash"]
    editable["shape_lab_schema_version"] = manifest["schema_version"]
    editable["shape_lab_coordinate_up_axis"] = manifest["coordinate_system"]["up_axis"]

    reconstructed_positions = stages[-1][1]
    baked = create_mesh_object(BAKED_OBJECT_NAME, reconstructed_positions, source_indices)
    baked["shape_lab_topology_hash"] = manifest["topology"]["hash"]
    baked["shape_lab_schema_version"] = manifest["schema_version"]
    baked["shape_lab_coordinate_up_axis"] = manifest["coordinate_system"]["up_axis"]

    final_key_positions = [tuple(f32(component) for component in point.co) for point in final_key.data]
    if not positions_bit_equal(final_key_positions, target_positions):
        raise RuntimeError("final editable shape key is not bit-exact with target positions")
    return editable, baked


def main():
    args = command_line_arguments()
    report_path = output_path(args.report)
    manifest, source_positions, source_indices, target_positions, target_indices, stages = (
        load_and_validate_package()
    )

    if args.verify_existing:
        baked = bpy.data.objects.get(BAKED_OBJECT_NAME)
        editable = bpy.data.objects.get(SOURCE_OBJECT_NAME)
        if baked is None:
            raise RuntimeError(f"{BAKED_OBJECT_NAME} was not found in the opened .blend file")
        if editable is None:
            raise RuntimeError(f"{SOURCE_OBJECT_NAME} was not found in the opened .blend file")
        report = report_for_object(
            baked, manifest, target_positions, target_indices, "verify_existing_saved_blend"
        )
        editable_report = report_for_editable_object(
            editable,
            manifest,
            source_positions,
            target_positions,
            target_indices,
            stages,
            "verify_existing_saved_blend_shape_key",
        )
    else:
        editable, baked = reconstruct(
            manifest,
            source_positions,
            source_indices,
            target_positions,
            target_indices,
            stages,
        )
        report = report_for_object(
            baked, manifest, target_positions, target_indices, "reconstruct_in_memory"
        )
        editable_report = report_for_editable_object(
            editable,
            manifest,
            source_positions,
            target_positions,
            target_indices,
            stages,
            "reconstruct_in_memory_shape_key",
        )
        if not args.no_save:
            blend_path = output_path(args.output_blend)
            blend_path.parent.mkdir(parents=True, exist_ok=True)
            bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
            report["saved_blend"] = str(blend_path)

    report["editable_shape_key"] = editable_report
    report["verification_passed"] = bool(
        report["verification_passed"] and editable_report["verification_passed"]
    )
    write_report(report_path, report)
    print(json.dumps(report, indent=2, sort_keys=True))
    if not report["verification_passed"]:
        raise RuntimeError(f"Shape Lab Blender verification failed; see {report_path}")


if __name__ == "__main__":
    main()
"####
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use shape_mesh::read_obj;

    use super::*;

    #[test]
    fn affine_stage_is_emitted_when_it_explains_the_deformation() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                position[0] * 1.5 + 0.25,
                position[1] * 0.75 - 0.5,
                position[2] * 1.25 + 0.125,
            ]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        assert!(matches!(
            result.manifest.operators.first(),
            Some(OperatorManifest::GlobalAffine { .. })
        ));
        assert_eq!(result.reconstructed_positions, target.positions);
        assert_eq!(result.verification.max_euclidean_error, 0.0);
    }

    #[test]
    fn affine_evaluation_uses_canonical_stepwise_float32_rounding() {
        let position = [[
            f32::from_bits(0x3f7e_7e92),
            f32::from_bits(0xbf80_2a10),
            f32::from_bits(0xbf7f_a514),
        ]];
        let matrix = [
            f32::from_bits(0xbda0_7359),
            f32::from_bits(0x3f73_68b4),
            f32::from_bits(0x3fad_290e),
            f32::from_bits(0xbf8c_b836),
            f32::from_bits(0x3fb9_8184),
            f32::from_bits(0xbf78_b131),
            f32::from_bits(0x3f18_9150),
            f32::from_bits(0xc039_a62e),
            f32::from_bits(0xbf96_330c),
            f32::from_bits(0xbf82_756f),
            f32::from_bits(0xbf9c_5776),
            f32::from_bits(0x4007_f472),
            0.0,
            0.0,
            0.0,
            1.0,
        ];

        let evaluated = apply_affine_to_positions(&position, matrix)[0];

        assert_eq!(evaluated[0].to_bits(), 0xc05e_bc1d);
        assert_eq!(evaluated[1].to_bits(), 0xbf8a_8e3e);
        assert_eq!(evaluated[2].to_bits(), 0x404c_ac1c);
    }

    #[test]
    fn exact_translation_needs_no_lossless_residual() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.5, position[1] - 0.25, position[2] + 2.0]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        assert!(matches!(
            result.manifest.operators.first(),
            Some(OperatorManifest::GlobalAffine { .. })
        ));
        assert!(result.residual_indices.is_empty());
        assert!(result.residual_positions.is_empty());
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn identical_pair_emits_only_empty_lossless_stage() {
        let source = tetra_mesh();

        let result = decompile_pair(&source, &source, DecompileSettings::default()).unwrap();

        assert_eq!(result.manifest.operators.len(), 1);
        assert!(matches!(
            result.manifest.operators.first(),
            Some(OperatorManifest::LosslessCorrection {
                corrected_vertex_count: 0,
                ..
            })
        ));
        assert!(result.residual_indices.is_empty());
    }

    #[test]
    fn residual_reconstructs_non_affine_changes_exactly() {
        let source = cube_mesh();
        let mut target = source.clone();
        target.positions[6][0] += 0.23;
        target.positions[6][1] -= 0.17;
        target.positions[6][2] += 0.31;

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        assert_eq!(result.reconstructed_positions, target.positions);
        assert_eq!(result.verification.outside_tolerance, 0);
        assert!(!result.residual_indices.is_empty());
        let affine_max_error =
            result
                .manifest
                .operators
                .iter()
                .find_map(|operator| match operator {
                    OperatorManifest::GlobalAffine {
                        max_remaining_error,
                        ..
                    } => Some(*max_remaining_error),
                    OperatorManifest::LosslessCorrection { .. } => None,
                });
        assert!(affine_max_error.is_none_or(|error| error > 0.0));
    }

    #[test]
    fn topology_mismatch_is_rejected() {
        let source = tetra_mesh();
        let mut target = source.clone();
        target.indices.swap(0, 1);

        let error = decompile_pair(&source, &target, DecompileSettings::default()).unwrap_err();

        assert!(matches!(error, DecompileError::TopologyMismatch(_)));
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let source = tetra_mesh();
        for settings in [
            DecompileSettings {
                affine_min_explained: -0.1,
                ..DecompileSettings::default()
            },
            DecompileSettings {
                affine_min_explained: 1.1,
                ..DecompileSettings::default()
            },
            DecompileSettings {
                residual_epsilon: f32::NAN,
                ..DecompileSettings::default()
            },
        ] {
            assert!(matches!(
                decompile_pair(&source, &source, settings),
                Err(DecompileError::InvalidSettings(_))
            ));
        }
    }

    #[test]
    fn decompiler_does_not_require_normals_or_bounds() {
        let mut source = tetra_mesh();
        source.normals.clear();
        let mut target = source.clone();
        target.positions[0][0] += 0.125;

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn repeated_triangle_indices_are_rejected() {
        let mut source = tetra_mesh();
        source.indices[1] = source.indices[0];

        let error = decompile_pair(&source, &source, DecompileSettings::default()).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidMesh { .. }));
    }

    #[test]
    fn package_writer_emits_manifest_sidecars_and_blender_script() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.5, position[1], position[2] - 0.25]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();

        let paths = write_decompile_package(&result, &source, &target, dir.path()).unwrap();

        assert!(paths.manifest.exists());
        assert!(paths.verification.exists());
        assert!(paths.package_verification.exists());
        assert!(paths.blender_script.exists());
        assert!(dir.path().join(SOURCE_MESHBIN).exists());
        assert!(dir.path().join(TARGET_MESHBIN).exists());
        assert!(dir.path().join("residual").join("indices.u32").exists());

        let manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(paths.manifest).unwrap()).unwrap();
        assert_eq!(manifest.schema_version, 2);
        assert_eq!(
            manifest.numeric_format.affine_evaluation,
            "float32_stepwise_no_fma"
        );
        assert_eq!(manifest.verification.max_euclidean_error, 0.0);
        assert_eq!(manifest.topology.vertex_count, source.positions.len());

        let package_verification = verify_decompile_package(dir.path()).unwrap();
        assert!(package_verification.topology_exact);
        assert!(package_verification.positions_bit_exact);
        assert_eq!(package_verification.max_euclidean_error, 0.0);
    }

    #[test]
    fn package_verifier_detects_corrupted_residual_payload() {
        let source = pyramid_mesh();
        let mut target = source.clone();
        target.positions[4][2] += 0.25;
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        write_decompile_package(&result, &source, &target, dir.path()).unwrap();
        let residual_path = dir.path().join(RESIDUAL_POSITION_FILE);
        let mut residual = fs::read(&residual_path).unwrap();
        residual[0] ^= 1;
        fs::write(&residual_path, residual).unwrap();

        let error = verify_decompile_package(dir.path()).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_writer_rejects_result_from_different_mesh_pair() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.25, position[1], position[2]]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let unrelated_target = transformed_mesh(&source, |position| {
            [position[0], position[1] + 1.0, position[2]]
        });
        let dir = tempfile::tempdir().unwrap();

        let error =
            write_decompile_package(&result, &source, &unrelated_target, dir.path()).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_asset_paths_cannot_escape_the_package_root() {
        let source = tetra_mesh();
        let target = source.clone();
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        write_decompile_package(&result, &source, &target, dir.path()).unwrap();
        let manifest_path = dir.path().join(MANIFEST_FILE);
        let mut manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest.source.path = "../source.meshbin".to_owned();
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(dir.path()).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_verifier_rejects_legacy_schema_one() {
        let source = tetra_mesh();
        let result = decompile_pair(&source, &source, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &source, &package).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["schema_version"] = serde_json::Value::from(1);
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(
            error,
            DecompileError::UnsupportedSchema {
                found: 1,
                supported: 2
            }
        ));
    }

    #[test]
    fn package_verifier_rejects_mismatched_verification_sidecar() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.25, position[1], position[2]]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let verification_path = package.join(VERIFICATION_FILE);
        let mut verification: VerificationReport =
            serde_json::from_str(&fs::read_to_string(&verification_path).unwrap()).unwrap();
        verification.outside_tolerance = 1;
        fs::write(
            &verification_path,
            serde_json::to_string_pretty(&verification).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_verifier_rejects_tampered_affine_metadata() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                position[0] * 1.2 + 0.25,
                position[1] * 0.8 - 0.5,
                position[2] * 1.1,
            ]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let affine = manifest
            .operators
            .iter_mut()
            .find_map(|operator| match operator {
                OperatorManifest::GlobalAffine {
                    explained_displacement_fraction,
                    ..
                } => Some(explained_displacement_fraction),
                OperatorManifest::LosslessCorrection { .. } => None,
            })
            .unwrap();
        *affine = 0.25;
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_replacement_removes_stale_files_and_stays_verifiable() {
        let source = tetra_mesh();
        let first_target = transformed_mesh(&source, |position| {
            [position[0] + 0.25, position[1], position[2]]
        });
        let second_target = transformed_mesh(&source, |position| {
            [position[0], position[1] - 0.75, position[2] + 0.5]
        });
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        let first = decompile_pair(&source, &first_target, DecompileSettings::default()).unwrap();
        write_decompile_package(&first, &source, &first_target, &package).unwrap();
        fs::write(package.join("stale.txt"), "must disappear").unwrap();

        let second = decompile_pair(&source, &second_target, DecompileSettings::default()).unwrap();
        write_decompile_package(&second, &source, &second_target, &package).unwrap();

        assert!(!package.join("stale.txt").exists());
        assert!(
            verify_decompile_package(&package)
                .unwrap()
                .positions_bit_exact
        );
        let staging_leftovers = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.contains(PACKAGE_TEMP_MARKER) || name.contains(PACKAGE_BACKUP_MARKER)
            })
            .count();
        assert_eq!(staging_leftovers, 0);
    }

    #[cfg(unix)]
    #[test]
    fn package_verifier_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let source = tetra_mesh();
        let target = source.clone();
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let outside = dir.path().join("outside.meshbin");
        fs::copy(package.join(SOURCE_MESHBIN), &outside).unwrap();
        let linked = package.join("linked.meshbin");
        symlink(&outside, &linked).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest.source.path = "linked.meshbin".to_owned();
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn affine_fit_is_stable_across_deterministic_transform_suite() {
        let source = cube_mesh();
        for step in 1..=24 {
            let t = step as f32 / 24.0;
            let target = transformed_mesh(&source, |position| {
                [
                    position[0] * (0.75 + 0.5 * t) + position[1] * (0.1 * t) + 0.25 * t,
                    position[1] * (1.2 - 0.3 * t) - position[2] * (0.08 * t) - 0.5 * t,
                    position[2] * (0.9 + 0.4 * t) + position[0] * (0.05 * t) + 0.125,
                ]
            });
            let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
            let explained = result
                .manifest
                .operators
                .iter()
                .find_map(|operator| match operator {
                    OperatorManifest::GlobalAffine {
                        explained_displacement_fraction,
                        ..
                    } => Some(*explained_displacement_fraction),
                    OperatorManifest::LosslessCorrection { .. } => None,
                })
                .unwrap();
            assert!(
                explained > 0.999_99,
                "step {step} explained only {explained}"
            );
            assert!(position_slices_bit_equal(
                &result.reconstructed_positions,
                &target.positions
            ));
        }
    }

    #[test]
    fn blender_script_contains_exact_and_saved_roundtrip_verification() {
        let script = blender_reconstruction_script();

        assert!(script.contains("positions_bit_exact"));
        assert!(script.contains("vertex_ids_exact"));
        assert!(script.contains("--verify-existing"));
        assert!(script.contains("editable_shape_key"));
        assert!(script.contains("stage_positions_exact"));
        assert!(script.contains("float32_stepwise_no_fma"));
        assert!(script.contains("matrix = [f32(value) for value in matrix]"));
        assert!(script.contains(
            "create_mesh_object(BAKED_OBJECT_NAME, reconstructed_positions, source_indices)"
        ));
        assert!(script.contains("verification.json does not match manifest.json"));
        assert!(script.contains("bpy.ops.wm.save_as_mainfile"));
    }

    fn tetra_mesh() -> TriangleMesh {
        read_obj(Cursor::new(
            "\
v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
f 1 2 3
f 1 2 4
f 2 3 4
f 1 3 4
",
        ))
        .unwrap()
    }

    fn cube_mesh() -> TriangleMesh {
        read_obj(Cursor::new(
            "\
v -1 -1 -1
v 1 -1 -1
v 1 1 -1
v -1 1 -1
v -1 -1 1
v 1 -1 1
v 1 1 1
v -1 1 1
f 1 3 2
f 1 4 3
f 5 6 7
f 5 7 8
f 1 2 6
f 1 6 5
f 4 8 7
f 4 7 3
f 1 5 8
f 1 8 4
f 2 3 7
f 2 7 6
",
        ))
        .unwrap()
    }

    fn pyramid_mesh() -> TriangleMesh {
        read_obj(Cursor::new(
            "\
v -1 -1 0
v 1 -1 0
v 1 1 0
v -1 1 0
v 0 0 1
f 1 2 3
f 1 3 4
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
",
        ))
        .unwrap()
    }

    fn transformed_mesh(
        source: &TriangleMesh,
        transform: impl Fn([f32; 3]) -> [f32; 3],
    ) -> TriangleMesh {
        let mut target = source.clone();
        target.positions = target.positions.iter().copied().map(transform).collect();
        target
    }
}
