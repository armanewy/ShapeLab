#![forbid(unsafe_code)]

//! Lossless same-topology deformation decompiler.
//!
//! This crate turns a source mesh and a target mesh with identical vertex and
//! triangle topology into a small explanatory operator stream plus a final
//! lossless residual. The current MVP intentionally starts with a strict
//! contract: same vertex order, same face order, same indices.

use std::collections::{BTreeMap, BTreeSet};
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
const INFERENCE_DIAGNOSTICS_FILE: &str = "inference-diagnostics.json";
const BLENDER_SCRIPT_FILE: &str = "blender_reconstruct.py";
const MESHBIN_MAGIC: &[u8; 8] = b"SLMBIN01";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const JACOBI_MAX_ITERATIONS: usize = 96;
const PSEUDOINVERSE_RELATIVE_EPSILON: f64 = 1.0e-11;
const OPERATOR_PARAMETER_SCORE_WEIGHT: f64 = 2.0e-3;
const SEMANTIC_METADATA_SCORE_WEIGHT: f64 = 5.0e-4;
const APPROXIMATE_RESIDUAL_SCORE_WEIGHT: f64 = 1.0;
const EXACT_RESIDUAL_BYTES_SCORE_WEIGHT: f64 = 1.0e-3;
const APPROXIMATE_RESIDUAL_ABSOLUTE_EPSILON: f64 = 1.0e-6;
const APPROXIMATE_RESIDUAL_RELATIVE_EPSILON: f64 = 1.0e-5;
const APPROXIMATE_RESIDUAL_ULP_MULTIPLIER: f64 = 2.0;
const ROTATION_ORTHONORMAL_TOLERANCE: f64 = 1.0e-3;
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

/// Semantic interpretation of a serialized affine stage.
///
/// The exact replay contract is still the baked affine matrix and stage
/// positions. This field lets the decompiler prefer simpler editable controls
/// when they explain the target nearly as well as a general affine.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffineSemanticFamily {
    /// Arbitrary affine transform.
    #[default]
    GeneralAffine,
    /// Pure translation.
    Translation,
    /// Proper rotation plus translation.
    RigidTransform,
    /// Proper rotation, uniform scale, and translation.
    SimilarityTransform,
}

/// Candidate operator family reported by inference diagnostics.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorFamily {
    /// No explanatory operator; the exact correction carries the deformation.
    NoOp,
    /// Pure translation.
    Translation,
    /// Proper rotation plus translation.
    RigidTransform,
    /// Proper rotation, uniform scale, and translation.
    SimilarityTransform,
    /// Arbitrary affine transform.
    GeneralAffine,
}

/// Scoring constants and tolerance policy serialized with inference diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceScoringPolicy {
    /// Human-readable scoring model identifier.
    pub model: String,
    /// Weight applied to semantic parameter count.
    pub parameter_weight: f64,
    /// Weight applied to additional semantic metadata bytes.
    pub semantic_metadata_weight: f64,
    /// Weight applied to tolerance-based approximate residual coverage.
    pub approximate_residual_weight: f64,
    /// Weight applied to exact audit-correction bytes.
    pub exact_residual_weight: f64,
    /// Absolute residual tolerance floor.
    pub absolute_residual_epsilon: f64,
    /// Residual tolerance multiplier for intrinsic shape scale.
    pub relative_residual_epsilon: f64,
    /// Residual tolerance multiplier for local `f32` coordinate spacing.
    pub ulp_multiplier: f64,
    /// Fixed family prior penalties used until measured conditioning is added.
    pub family_priors: BTreeMap<OperatorFamily, f64>,
}

/// Out-of-band inference report written beside the replay-verified package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceDiagnostics {
    /// Diagnostics format version, independent of the replay manifest schema.
    pub diagnostics_schema_version: u32,
    /// Replay manifest schema version the diagnostics were produced for.
    pub package_schema_version: u32,
    /// Weighting model used for semantic fitting and eligibility.
    pub surface_weighting: String,
    /// Unweighted source-to-target vertex squared displacement.
    pub raw_identity_error: f64,
    /// Triangle-area-weighted source-to-target squared displacement.
    pub weighted_identity_error: f64,
    /// Scoring constants and tolerance policy used for this report.
    pub scoring_policy: InferenceScoringPolicy,
    /// Index of the selected hypothesis in `hypotheses`.
    pub selected_hypothesis_index: usize,
    /// Candidate score breakdowns in deterministic inference order.
    pub hypotheses: Vec<HypothesisDiagnostics>,
}

/// Auditable score breakdown for one inferred operator candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HypothesisDiagnostics {
    /// Candidate operator family.
    pub family: OperatorFamily,
    /// Triangle-area-weighted squared geometric error.
    pub weighted_geometric_error: f64,
    /// Surface-weighted displacement fraction explained by the candidate.
    pub weighted_explained_fraction: f64,
    /// Unweighted vertex squared geometric error.
    pub raw_geometric_error: f64,
    /// Unweighted vertex displacement fraction. Schema-2 affine metadata uses this value.
    pub raw_explained_fraction: f64,
    /// Weighted error scale used to normalize geometric error.
    pub error_normalization_scale: f64,
    /// Weighted geometric error divided by `error_normalization_scale`.
    pub normalized_geometric_error_cost: f64,
    /// Full target literal position payload size used for byte-normalized costs.
    pub literal_size_bytes: usize,
    /// Semantic parameter count used for parameter cost.
    pub parameter_count: usize,
    /// Score contribution from semantic parameter count.
    pub parameter_cost: f64,
    /// Additional semantic metadata bytes used for metadata cost.
    pub semantic_metadata_bytes: usize,
    /// Score contribution from additional semantic metadata.
    pub semantic_metadata_cost: f64,
    /// Tolerance-based residual coverage before applying the policy weight.
    pub approximate_residual_coverage: f64,
    /// Tolerance-based residual cost used for semantic model selection.
    pub approximate_residual_cost: f64,
    /// Exact audit-correction payload size in bytes.
    pub exact_residual_bytes: usize,
    /// Score contribution from exact audit-correction bytes.
    pub exact_residual_cost: f64,
    /// Family prior until measured fit-conditioning penalties are introduced.
    pub prior_penalty: f64,
    /// Sum of every serialized score component.
    pub score_component_sum: f64,
    /// Total model-selection score.
    pub total_score: f64,
    /// Whether this candidate was selected.
    pub selected: bool,
    /// Why the candidate was not eligible for selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
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
        /// More editable semantic family represented by this affine matrix.
        #[serde(default)]
        semantic_family: AffineSemanticFamily,
        /// Translation vector for translation, rigid, and similarity semantics.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        translation: Option<[f32; 3]>,
        /// Row-major 3x3 rotation basis for rigid and similarity semantics.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rotation_row_major_3x3: Option<[f32; 9]>,
        /// Uniform scale for similarity semantics.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uniform_scale: Option<f32>,
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
    /// Inference score breakdown for every operator candidate considered.
    pub inference_diagnostics: InferenceDiagnostics,
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
    /// Out-of-band inference diagnostics JSON path.
    pub inference_diagnostics: PathBuf,
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

    let raw_identity_error = sum_squared_distance(&source.positions, &target.positions);
    let inference = choose_affine_candidate(source, target, settings, raw_identity_error);
    let affine_hypothesis = inference.selected;
    let emit_affine = !affine_hypothesis.operator.is_no_op();

    let current_positions = if emit_affine {
        affine_hypothesis.reconstructed_positions.clone()
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
        let operator = affine_hypothesis.operator;
        let semantic_family = operator.semantic_family();
        let parameters = operator.parameters();
        let matrix = operator.matrix();
        let (id, label) = match semantic_family {
            AffineSemanticFamily::Translation => ("op-0000-translation", "Translation"),
            AffineSemanticFamily::RigidTransform => ("op-0000-rigid-transform", "Rigid transform"),
            AffineSemanticFamily::SimilarityTransform => {
                ("op-0000-similarity-transform", "Similarity transform")
            }
            AffineSemanticFamily::GeneralAffine => ("op-0000-global-affine", "Global affine fit"),
        };
        operators.push(OperatorManifest::GlobalAffine {
            id: id.to_owned(),
            label: label.to_owned(),
            matrix_row_major_4x4: matrix,
            semantic_family,
            translation: parameters.translation,
            rotation_row_major_3x3: parameters.rotation,
            uniform_scale: parameters.uniform_scale,
            explained_displacement_fraction: affine_hypothesis.raw_explained_fraction as f32,
            max_remaining_error: max_euclidean_distance(
                &affine_hypothesis.reconstructed_positions,
                &target.positions,
            ),
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
        affine_positions: emit_affine.then_some(affine_hypothesis.reconstructed_positions),
        residual_indices,
        residual_positions,
        reconstructed_positions,
        inference_diagnostics: inference.diagnostics,
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
    write_json(
        &package_path(out_dir, INFERENCE_DIAGNOSTICS_FILE),
        &result.inference_diagnostics,
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
        inference_diagnostics: package_path(out_dir, INFERENCE_DIAGNOSTICS_FILE),
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
    let weights = vertex_area_weights_from_parts(&source.positions, &source.indices);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source.positions, &target.positions, &weights);
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
                semantic_family,
                translation,
                rotation_row_major_3x3,
                uniform_scale,
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
                validate_affine_semantics(
                    *matrix_row_major_4x4,
                    *semantic_family,
                    *translation,
                    *rotation_row_major_3x3,
                    *uniform_scale,
                    &manifest_path,
                )?;
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
                let weighted_affine_error =
                    weighted_sum_squared_distance(&evaluated, &target.positions, &weights);
                let expected_explained = explained_fraction(identity_error, affine_error) as f32;
                let expected_weighted_explained =
                    explained_fraction(weighted_identity_error, weighted_affine_error);
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
                if weighted_identity_error <= 0.0
                    || weighted_affine_error >= weighted_identity_error
                    || expected_weighted_explained
                        < f64::from(manifest.settings.affine_min_explained)
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

fn validate_affine_semantics(
    matrix: [f32; 16],
    semantic_family: AffineSemanticFamily,
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 9]>,
    uniform_scale: Option<f32>,
    path: &Path,
) -> Result<(), DecompileError> {
    match semantic_family {
        AffineSemanticFamily::GeneralAffine => {
            if translation.is_some() || rotation.is_some() || uniform_scale.is_some() {
                return Err(invalid_package(
                    path,
                    "general affine operator must not declare semantic parameters",
                ));
            }
        }
        AffineSemanticFamily::Translation => {
            reject_rotation_or_scale(rotation, uniform_scale, path, "translation operator")?;
            let translation = translation.ok_or_else(|| {
                invalid_package(
                    path,
                    "translation operator is missing translation parameters",
                )
            })?;
            if !array_is_finite(translation) {
                return Err(invalid_package(
                    path,
                    "translation operator contains non-finite parameters",
                ));
            }
            let expected = translation_matrix(translation);
            if !matrices_bit_equal(matrix, expected) {
                return Err(invalid_package(
                    path,
                    "translation operator matrix does not match its translation parameters",
                ));
            }
        }
        AffineSemanticFamily::RigidTransform => {
            let translation = require_translation(translation, path, "rigid transform operator")?;
            let rotation = require_rotation(rotation, path, "rigid transform operator")?;
            if uniform_scale.is_some() {
                return Err(invalid_package(
                    path,
                    "rigid transform operator must not declare uniform scale",
                ));
            }
            let expected = rigid_matrix(rotation, translation);
            if !matrices_bit_equal(matrix, expected) {
                return Err(invalid_package(
                    path,
                    "rigid transform matrix does not match its semantic parameters",
                ));
            }
        }
        AffineSemanticFamily::SimilarityTransform => {
            let translation =
                require_translation(translation, path, "similarity transform operator")?;
            let rotation = require_rotation(rotation, path, "similarity transform operator")?;
            let scale = uniform_scale.ok_or_else(|| {
                invalid_package(
                    path,
                    "similarity transform operator is missing uniform scale",
                )
            })?;
            if !scale.is_finite() || scale <= 0.0 {
                return Err(invalid_package(
                    path,
                    "similarity transform operator contains invalid uniform scale",
                ));
            }
            let expected = similarity_matrix(rotation, scale, translation);
            if !matrices_bit_equal(matrix, expected) {
                return Err(invalid_package(
                    path,
                    "similarity transform matrix does not match its semantic parameters",
                ));
            }
        }
    }
    Ok(())
}

fn require_translation(
    translation: Option<[f32; 3]>,
    path: &Path,
    label: &str,
) -> Result<[f32; 3], DecompileError> {
    let translation = translation
        .ok_or_else(|| invalid_package(path, format!("{label} is missing translation")))?;
    if !array_is_finite(translation) {
        return Err(invalid_package(
            path,
            format!("{label} contains non-finite translation parameters"),
        ));
    }
    Ok(translation)
}

fn require_rotation(
    rotation: Option<[f32; 9]>,
    path: &Path,
    label: &str,
) -> Result<[f32; 9], DecompileError> {
    let rotation =
        rotation.ok_or_else(|| invalid_package(path, format!("{label} is missing rotation")))?;
    if !array_is_finite(rotation) {
        return Err(invalid_package(
            path,
            format!("{label} contains non-finite rotation parameters"),
        ));
    }
    if !is_proper_rotation(rotation) {
        return Err(invalid_package(
            path,
            format!("{label} rotation is not a proper orthonormal basis"),
        ));
    }
    Ok(rotation)
}

fn reject_rotation_or_scale(
    rotation: Option<[f32; 9]>,
    uniform_scale: Option<f32>,
    path: &Path,
    label: &str,
) -> Result<(), DecompileError> {
    if rotation.is_some() || uniform_scale.is_some() {
        return Err(invalid_package(
            path,
            format!("{label} must not declare rotation or scale parameters"),
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
    let weights = vertex_area_weights(source);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source.positions, &target.positions, &weights);
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
                semantic_family,
                translation,
                rotation_row_major_3x3,
                uniform_scale,
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
                validate_affine_semantics(
                    *matrix_row_major_4x4,
                    *semantic_family,
                    *translation,
                    *rotation_row_major_3x3,
                    *uniform_scale,
                    Path::new(MANIFEST_FILE),
                )?;
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
                let weighted_affine_error =
                    weighted_sum_squared_distance(&evaluated, &target.positions, &weights);
                let expected_explained = explained_fraction(identity_error, affine_error) as f32;
                let expected_weighted_explained =
                    explained_fraction(weighted_identity_error, weighted_affine_error);
                let expected_max = max_euclidean_distance(&evaluated, &target.positions);
                if !f32_bits_equal(*explained_displacement_fraction, expected_explained)
                    || !f32_bits_equal(*max_remaining_error, expected_max)
                    || weighted_identity_error <= 0.0
                    || weighted_affine_error >= weighted_identity_error
                    || expected_weighted_explained
                        < f64::from(result.manifest.settings.affine_min_explained)
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

#[derive(Debug, Clone)]
struct OperatorHypothesis {
    operator: InferredOperator,
    reconstructed_positions: Vec<[f32; 3]>,
    weighted_geometric_error: f64,
    weighted_explained_fraction: f64,
    raw_geometric_error: f64,
    raw_explained_fraction: f64,
    parameter_count: usize,
    semantic_metadata_size: usize,
    literal_size_bytes: usize,
    approximate_residual_coverage: f64,
    approximate_residual_cost: f64,
    exact_residual_bytes: usize,
    error_normalization_scale: f64,
    normalized_geometric_error_cost: f64,
    parameter_cost: f64,
    semantic_metadata_cost: f64,
    exact_residual_cost: f64,
    operator_prior_penalty: f64,
    score_component_sum: f64,
    score: f64,
}

#[derive(Debug, Clone)]
struct InferenceResult {
    selected: OperatorHypothesis,
    diagnostics: InferenceDiagnostics,
}

#[derive(Debug, Copy, Clone, Default)]
struct AffineSemanticParameters {
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 9]>,
    uniform_scale: Option<f32>,
}

#[derive(Debug, Copy, Clone)]
struct SemanticAffineMatrix {
    matrix: [f32; 16],
    parameters: AffineSemanticParameters,
}

#[derive(Debug, Copy, Clone)]
enum InferredOperator {
    NoOp,
    Translation {
        matrix: [f32; 16],
        translation: [f32; 3],
    },
    RigidTransform {
        matrix: [f32; 16],
        translation: [f32; 3],
        rotation: [f32; 9],
    },
    SimilarityTransform {
        matrix: [f32; 16],
        translation: [f32; 3],
        rotation: [f32; 9],
        uniform_scale: f32,
    },
    GeneralAffine {
        matrix: [f32; 16],
    },
}

impl InferredOperator {
    fn family(self) -> OperatorFamily {
        match self {
            Self::NoOp => OperatorFamily::NoOp,
            Self::Translation { .. } => OperatorFamily::Translation,
            Self::RigidTransform { .. } => OperatorFamily::RigidTransform,
            Self::SimilarityTransform { .. } => OperatorFamily::SimilarityTransform,
            Self::GeneralAffine { .. } => OperatorFamily::GeneralAffine,
        }
    }

    fn semantic_family(self) -> AffineSemanticFamily {
        match self {
            Self::NoOp => AffineSemanticFamily::GeneralAffine,
            Self::Translation { .. } => AffineSemanticFamily::Translation,
            Self::RigidTransform { .. } => AffineSemanticFamily::RigidTransform,
            Self::SimilarityTransform { .. } => AffineSemanticFamily::SimilarityTransform,
            Self::GeneralAffine { .. } => AffineSemanticFamily::GeneralAffine,
        }
    }

    fn matrix(self) -> [f32; 16] {
        match self {
            Self::NoOp => identity(),
            Self::Translation { matrix, .. }
            | Self::RigidTransform { matrix, .. }
            | Self::SimilarityTransform { matrix, .. }
            | Self::GeneralAffine { matrix } => matrix,
        }
    }

    fn parameters(self) -> AffineSemanticParameters {
        match self {
            Self::NoOp => AffineSemanticParameters::default(),
            Self::Translation { translation, .. } => AffineSemanticParameters {
                translation: Some(translation),
                ..AffineSemanticParameters::default()
            },
            Self::RigidTransform {
                translation,
                rotation,
                ..
            } => AffineSemanticParameters {
                translation: Some(translation),
                rotation: Some(rotation),
                uniform_scale: None,
            },
            Self::SimilarityTransform {
                translation,
                rotation,
                uniform_scale,
                ..
            } => AffineSemanticParameters {
                translation: Some(translation),
                rotation: Some(rotation),
                uniform_scale: Some(uniform_scale),
            },
            Self::GeneralAffine { .. } => AffineSemanticParameters::default(),
        }
    }

    fn parameter_count(self) -> usize {
        match self {
            Self::NoOp => 0,
            Self::Translation { .. } => 3,
            Self::RigidTransform { .. } => 6,
            Self::SimilarityTransform { .. } => 7,
            Self::GeneralAffine { .. } => 12,
        }
    }

    fn semantic_metadata_size(self) -> usize {
        match self {
            Self::NoOp => 0,
            Self::Translation { .. } => 3 * 4,
            Self::RigidTransform { .. } => (3 + 9) * 4,
            Self::SimilarityTransform { .. } => (3 + 9 + 1) * 4,
            Self::GeneralAffine { .. } => 0,
        }
    }

    fn operator_prior_penalty(self) -> f64 {
        match self {
            Self::NoOp => 0.0,
            Self::Translation { .. } => 0.0,
            Self::RigidTransform { .. } => 1.0e-3,
            Self::SimilarityTransform { .. } => 2.0e-3,
            Self::GeneralAffine { .. } => 1.0e-2,
        }
    }

    fn preference_order(self) -> usize {
        match self {
            Self::NoOp => 0,
            Self::Translation { .. } => 0,
            Self::RigidTransform { .. } => 1,
            Self::SimilarityTransform { .. } => 2,
            Self::GeneralAffine { .. } => 3,
        }
    }

    fn is_no_op(self) -> bool {
        matches!(self, Self::NoOp)
    }
}

fn inference_scoring_policy() -> InferenceScoringPolicy {
    InferenceScoringPolicy {
        model: "weighted_affine_origin_invariant_residual_v1".to_owned(),
        parameter_weight: OPERATOR_PARAMETER_SCORE_WEIGHT,
        semantic_metadata_weight: SEMANTIC_METADATA_SCORE_WEIGHT,
        approximate_residual_weight: APPROXIMATE_RESIDUAL_SCORE_WEIGHT,
        exact_residual_weight: EXACT_RESIDUAL_BYTES_SCORE_WEIGHT,
        absolute_residual_epsilon: APPROXIMATE_RESIDUAL_ABSOLUTE_EPSILON,
        relative_residual_epsilon: APPROXIMATE_RESIDUAL_RELATIVE_EPSILON,
        ulp_multiplier: APPROXIMATE_RESIDUAL_ULP_MULTIPLIER,
        family_priors: [
            (
                OperatorFamily::NoOp,
                InferredOperator::NoOp.operator_prior_penalty(),
            ),
            (
                OperatorFamily::Translation,
                InferredOperator::Translation {
                    matrix: identity(),
                    translation: [0.0; 3],
                }
                .operator_prior_penalty(),
            ),
            (
                OperatorFamily::RigidTransform,
                InferredOperator::RigidTransform {
                    matrix: identity(),
                    translation: [0.0; 3],
                    rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                }
                .operator_prior_penalty(),
            ),
            (
                OperatorFamily::SimilarityTransform,
                InferredOperator::SimilarityTransform {
                    matrix: identity(),
                    translation: [0.0; 3],
                    rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                    uniform_scale: 1.0,
                }
                .operator_prior_penalty(),
            ),
            (
                OperatorFamily::GeneralAffine,
                InferredOperator::GeneralAffine { matrix: identity() }.operator_prior_penalty(),
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn choose_affine_candidate(
    source: &TriangleMesh,
    target: &TriangleMesh,
    settings: DecompileSettings,
    raw_identity_error: f64,
) -> InferenceResult {
    let weights = vertex_area_weights(source);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source.positions, &target.positions, &weights);
    let mut hypotheses = Vec::new();
    hypotheses.push(operator_hypothesis(
        InferredOperator::NoOp,
        &source.positions,
        &target.positions,
        &weights,
        raw_identity_error,
        weighted_identity_error,
    ));
    let general_matrix =
        fit_affine(&source.positions, &target.positions, &weights).unwrap_or(identity());
    hypotheses.push(operator_hypothesis(
        InferredOperator::GeneralAffine {
            matrix: general_matrix,
        },
        &source.positions,
        &target.positions,
        &weights,
        raw_identity_error,
        weighted_identity_error,
    ));

    if let Some(translation_matrix) =
        fit_translation_matrix(&source.positions, &target.positions, &weights)
    {
        let translation_delta = [
            translation_matrix[3],
            translation_matrix[7],
            translation_matrix[11],
        ];
        hypotheses.push(operator_hypothesis(
            InferredOperator::Translation {
                matrix: translation_matrix,
                translation: translation_delta,
            },
            &source.positions,
            &target.positions,
            &weights,
            raw_identity_error,
            weighted_identity_error,
        ));
    }

    if let Some(rigid) = fit_rigid_matrix(&source.positions, &target.positions, &weights)
        && let AffineSemanticParameters {
            translation: Some(translation),
            rotation: Some(rotation),
            uniform_scale: None,
        } = rigid.parameters
    {
        hypotheses.push(operator_hypothesis(
            InferredOperator::RigidTransform {
                matrix: rigid.matrix,
                translation,
                rotation,
            },
            &source.positions,
            &target.positions,
            &weights,
            raw_identity_error,
            weighted_identity_error,
        ));
    }
    if let Some(similarity) = fit_similarity_matrix(&source.positions, &target.positions, &weights)
        && let AffineSemanticParameters {
            translation: Some(translation),
            rotation: Some(rotation),
            uniform_scale: Some(uniform_scale),
        } = similarity.parameters
    {
        hypotheses.push(operator_hypothesis(
            InferredOperator::SimilarityTransform {
                matrix: similarity.matrix,
                translation,
                rotation,
                uniform_scale,
            },
            &source.positions,
            &target.positions,
            &weights,
            raw_identity_error,
            weighted_identity_error,
        ));
    }

    let rejections = hypotheses
        .iter()
        .map(|hypothesis| hypothesis_rejection(hypothesis, settings, weighted_identity_error))
        .collect::<Vec<_>>();
    let selected_index = hypotheses
        .iter()
        .enumerate()
        .filter(|(index, _)| rejections[*index].is_none())
        .min_by(|(_, left), (_, right)| compare_hypotheses(left, right))
        .map(|(index, _)| index)
        .unwrap_or(0);
    let hypothesis_diagnostics = hypotheses
        .iter()
        .enumerate()
        .map(|(index, hypothesis)| {
            hypothesis.diagnostics(index == selected_index, rejections[index].clone())
        })
        .collect();

    InferenceResult {
        selected: hypotheses[selected_index].clone(),
        diagnostics: InferenceDiagnostics {
            diagnostics_schema_version: 1,
            package_schema_version: SCHEMA_VERSION,
            surface_weighting: "triangle_area_derived_vertex_weights".to_owned(),
            raw_identity_error,
            weighted_identity_error,
            scoring_policy: inference_scoring_policy(),
            selected_hypothesis_index: selected_index,
            hypotheses: hypothesis_diagnostics,
        },
    }
}

fn operator_hypothesis(
    operator: InferredOperator,
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
    raw_identity_error: f64,
    weighted_identity_error: f64,
) -> OperatorHypothesis {
    let reconstructed_positions = apply_affine_to_positions(source, operator.matrix());
    let raw_geometric_error = sum_squared_distance(&reconstructed_positions, target);
    let weighted_geometric_error =
        weighted_sum_squared_distance(&reconstructed_positions, target, weights);
    let raw_explained_fraction = explained_fraction(raw_identity_error, raw_geometric_error);
    let weighted_explained_fraction =
        explained_fraction(weighted_identity_error, weighted_geometric_error);
    let parameter_count = operator.parameter_count();
    let semantic_metadata_size = operator.semantic_metadata_size();
    let exact_residual_bytes = exact_residual_storage_size(&reconstructed_positions, target);
    let score = hypothesis_score(HypothesisScoreInputs {
        operator,
        weighted_geometric_error,
        parameter_count,
        semantic_metadata_size,
        exact_residual_bytes,
        source,
        target,
        weights,
        reconstructed: &reconstructed_positions,
    });
    OperatorHypothesis {
        operator,
        reconstructed_positions,
        weighted_geometric_error,
        weighted_explained_fraction,
        raw_geometric_error,
        raw_explained_fraction,
        parameter_count,
        semantic_metadata_size,
        literal_size_bytes: score.literal_size_bytes,
        approximate_residual_coverage: score.approximate_residual_coverage,
        approximate_residual_cost: score.approximate_residual_cost,
        exact_residual_bytes,
        error_normalization_scale: score.error_normalization_scale,
        normalized_geometric_error_cost: score.normalized_geometric_error_cost,
        parameter_cost: score.parameter_cost,
        semantic_metadata_cost: score.semantic_metadata_cost,
        exact_residual_cost: score.exact_residual_cost,
        operator_prior_penalty: score.operator_prior_penalty,
        score_component_sum: score.score_component_sum,
        score: score.total_score,
    }
}

impl OperatorHypothesis {
    fn diagnostics(
        &self,
        selected: bool,
        rejection_reason: Option<String>,
    ) -> HypothesisDiagnostics {
        HypothesisDiagnostics {
            family: self.operator.family(),
            weighted_geometric_error: self.weighted_geometric_error,
            weighted_explained_fraction: self.weighted_explained_fraction,
            raw_geometric_error: self.raw_geometric_error,
            raw_explained_fraction: self.raw_explained_fraction,
            error_normalization_scale: self.error_normalization_scale,
            normalized_geometric_error_cost: self.normalized_geometric_error_cost,
            literal_size_bytes: self.literal_size_bytes,
            parameter_count: self.parameter_count,
            parameter_cost: self.parameter_cost,
            semantic_metadata_bytes: self.semantic_metadata_size,
            semantic_metadata_cost: self.semantic_metadata_cost,
            approximate_residual_coverage: self.approximate_residual_coverage,
            approximate_residual_cost: self.approximate_residual_cost,
            exact_residual_bytes: self.exact_residual_bytes,
            exact_residual_cost: self.exact_residual_cost,
            prior_penalty: self.operator_prior_penalty,
            score_component_sum: self.score_component_sum,
            total_score: self.score,
            selected,
            rejection_reason,
        }
    }
}

fn hypothesis_rejection(
    hypothesis: &OperatorHypothesis,
    settings: DecompileSettings,
    weighted_identity_error: f64,
) -> Option<String> {
    if hypothesis.operator.is_no_op() {
        return None;
    }
    if weighted_identity_error <= f64::EPSILON {
        return Some("source and target have no surface-weighted displacement".to_owned());
    }
    if hypothesis.weighted_geometric_error >= weighted_identity_error {
        return Some("candidate does not improve surface-weighted geometric error".to_owned());
    }
    let threshold = f64::from(settings.affine_min_explained);
    if hypothesis.weighted_explained_fraction < threshold {
        return Some(format!(
            "surface-weighted explained fraction {:.9} is below affine_min_explained {:.9}",
            hypothesis.weighted_explained_fraction, threshold
        ));
    }
    None
}

fn compare_hypotheses(left: &OperatorHypothesis, right: &OperatorHypothesis) -> std::cmp::Ordering {
    left.score
        .total_cmp(&right.score)
        .then_with(|| left.exact_residual_bytes.cmp(&right.exact_residual_bytes))
        .then_with(|| left.parameter_count.cmp(&right.parameter_count))
        .then_with(|| {
            left.semantic_metadata_size
                .cmp(&right.semantic_metadata_size)
        })
        .then_with(|| {
            left.operator
                .preference_order()
                .cmp(&right.operator.preference_order())
        })
}

struct HypothesisScoreInputs<'a> {
    operator: InferredOperator,
    weighted_geometric_error: f64,
    parameter_count: usize,
    semantic_metadata_size: usize,
    exact_residual_bytes: usize,
    source: &'a [[f32; 3]],
    target: &'a [[f32; 3]],
    weights: &'a [f64],
    reconstructed: &'a [[f32; 3]],
}

#[derive(Debug, Copy, Clone)]
struct HypothesisScore {
    error_normalization_scale: f64,
    literal_size_bytes: usize,
    normalized_geometric_error_cost: f64,
    parameter_cost: f64,
    semantic_metadata_cost: f64,
    approximate_residual_coverage: f64,
    approximate_residual_cost: f64,
    exact_residual_cost: f64,
    operator_prior_penalty: f64,
    score_component_sum: f64,
    total_score: f64,
}

fn hypothesis_score(inputs: HypothesisScoreInputs<'_>) -> HypothesisScore {
    let error_scale = weighted_centered_sum_squared_distance(inputs.source, inputs.weights)
        .max(weighted_centered_sum_squared_distance(
            inputs.target,
            inputs.weights,
        ))
        .max(f64::EPSILON);
    let literal_size = inputs.source.len().saturating_mul(12).max(1) as f64;
    let normalized_geometric_error_cost = inputs.weighted_geometric_error / error_scale;
    let parameter_cost = inputs.parameter_count as f64 * OPERATOR_PARAMETER_SCORE_WEIGHT;
    let semantic_metadata_cost =
        inputs.semantic_metadata_size as f64 / literal_size * SEMANTIC_METADATA_SCORE_WEIGHT;
    let approximate_residual_coverage =
        approximate_residual_cost(inputs.reconstructed, inputs.target, inputs.weights);
    let approximate_residual_cost =
        approximate_residual_coverage * APPROXIMATE_RESIDUAL_SCORE_WEIGHT;
    let exact_residual_cost =
        inputs.exact_residual_bytes as f64 / literal_size * EXACT_RESIDUAL_BYTES_SCORE_WEIGHT;
    let operator_prior_penalty = inputs.operator.operator_prior_penalty();
    let score_component_sum = normalized_geometric_error_cost
        + parameter_cost
        + semantic_metadata_cost
        + approximate_residual_cost
        + exact_residual_cost
        + operator_prior_penalty;
    HypothesisScore {
        error_normalization_scale: error_scale,
        literal_size_bytes: literal_size as usize,
        normalized_geometric_error_cost,
        parameter_cost,
        semantic_metadata_cost,
        approximate_residual_coverage,
        approximate_residual_cost,
        exact_residual_cost,
        operator_prior_penalty,
        score_component_sum,
        total_score: score_component_sum,
    }
}

fn exact_residual_storage_size(reconstructed: &[[f32; 3]], target: &[[f32; 3]]) -> usize {
    reconstructed
        .iter()
        .zip(target)
        .filter(|(left, right)| !positions_bit_equal(**left, **right))
        .count()
        * (std::mem::size_of::<u32>() + 3 * std::mem::size_of::<f32>())
}

fn approximate_residual_cost(
    reconstructed: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
) -> f64 {
    let total_weight = weights
        .iter()
        .copied()
        .filter(|weight| weight.is_finite() && *weight > 0.0)
        .sum::<f64>()
        .max(f64::EPSILON);
    let intrinsic_scale = weighted_rms_radius(target, weights)
        .max(weighted_rms_radius(reconstructed, weights))
        .max(bounding_box_diagonal(target))
        .max(bounding_box_diagonal(reconstructed));
    let base_epsilon = APPROXIMATE_RESIDUAL_ABSOLUTE_EPSILON
        .max(intrinsic_scale * APPROXIMATE_RESIDUAL_RELATIVE_EPSILON);
    let reconstructed_centroid = weighted_centroid_f64(reconstructed, weights);
    let target_centroid = weighted_centroid_f64(target, weights);
    let residual_weight = reconstructed
        .iter()
        .zip(target)
        .enumerate()
        .filter_map(|(index, (left, right))| {
            let dx = f64::from(left[0]) - f64::from(right[0]);
            let dy = f64::from(left[1]) - f64::from(right[1]);
            let dz = f64::from(left[2]) - f64::from(right[2]);
            let distance_squared = dx * dx + dy * dy + dz * dz;
            let epsilon = base_epsilon.max(
                APPROXIMATE_RESIDUAL_ULP_MULTIPLIER
                    * position_local_coordinate_ulp(
                        *left,
                        *right,
                        reconstructed_centroid,
                        target_centroid,
                    ),
            );
            let threshold_squared = epsilon * epsilon;
            if distance_squared <= threshold_squared {
                None
            } else {
                Some(weights.get(index).copied().unwrap_or(1.0).max(0.0))
            }
        })
        .sum::<f64>();
    let cost = (residual_weight / total_weight).clamp(0.0, 1.0);
    if cost <= 0.0 { 0.0 } else { cost }
}

fn weighted_rms_radius(positions: &[[f32; 3]], weights: &[f64]) -> f64 {
    let total_weight = weights
        .iter()
        .copied()
        .filter(|weight| weight.is_finite() && *weight > 0.0)
        .sum::<f64>()
        .max(f64::EPSILON);
    (weighted_centered_sum_squared_distance(positions, weights) / total_weight).sqrt()
}

fn bounding_box_diagonal(positions: &[[f32; 3]]) -> f64 {
    let Some(first) = positions.first() else {
        return 0.0;
    };
    let mut min = [
        f64::from(first[0]),
        f64::from(first[1]),
        f64::from(first[2]),
    ];
    let mut max = min;
    for position in positions.iter().skip(1) {
        for axis in 0..3 {
            let value = f64::from(position[axis]);
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    let dx = max[0] - min[0];
    let dy = max[1] - min[1];
    let dz = max[2] - min[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn position_local_coordinate_ulp(
    left: [f32; 3],
    right: [f32; 3],
    left_centroid: [f64; 3],
    right_centroid: [f64; 3],
) -> f64 {
    (0..3)
        .flat_map(|axis| {
            [
                (f64::from(left[axis]) - left_centroid[axis]) as f32,
                (f64::from(right[axis]) - right_centroid[axis]) as f32,
            ]
        })
        .map(f32_ulp)
        .fold(0.0_f64, f64::max)
}

fn f32_ulp(value: f32) -> f64 {
    if !value.is_finite() {
        return f64::INFINITY;
    }
    if value == 0.0 {
        return f64::from(f32::from_bits(1));
    }
    let bits = value.to_bits();
    let next_bits = if value.is_sign_negative() {
        bits.wrapping_sub(1)
    } else {
        bits.wrapping_add(1)
    };
    let next = f32::from_bits(next_bits);
    if next.is_finite() {
        (f64::from(next) - f64::from(value)).abs()
    } else {
        let previous_bits = if value.is_sign_negative() {
            bits.wrapping_add(1)
        } else {
            bits.wrapping_sub(1)
        };
        (f64::from(value) - f64::from(f32::from_bits(previous_bits))).abs()
    }
}

fn vertex_area_weights(mesh: &TriangleMesh) -> Vec<f64> {
    vertex_area_weights_from_parts(&mesh.positions, &mesh.indices)
}

fn vertex_area_weights_from_parts(positions: &[[f32; 3]], indices: &[u32]) -> Vec<f64> {
    let mut weights = vec![0.0_f64; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        if a >= positions.len() || b >= positions.len() || c >= positions.len() {
            continue;
        }
        let area = triangle_area(positions[a], positions[b], positions[c]);
        if area.is_finite() && area > 0.0 {
            let share = area / 3.0;
            weights[a] += share;
            weights[b] += share;
            weights[c] += share;
        }
    }
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= f64::EPSILON {
        return vec![1.0; positions.len()];
    }
    let average = total / positions.len().max(1) as f64;
    for weight in &mut weights {
        if !weight.is_finite() || *weight <= 0.0 {
            *weight = average;
        }
        *weight /= average;
    }
    weights
}

fn triangle_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f64 {
    let ab = [
        f64::from(b[0]) - f64::from(a[0]),
        f64::from(b[1]) - f64::from(a[1]),
        f64::from(b[2]) - f64::from(a[2]),
    ];
    let ac = [
        f64::from(c[0]) - f64::from(a[0]),
        f64::from(c[1]) - f64::from(a[1]),
        f64::from(c[2]) - f64::from(a[2]),
    ];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    0.5 * dot_f64(cross, cross).sqrt()
}

fn weighted_centroid_f64(positions: &[[f32; 3]], weights: &[f64]) -> [f64; 3] {
    let mut total = [0.0_f64; 3];
    let mut total_weight = 0.0_f64;
    for (index, position) in positions.iter().enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        total[0] += f64::from(position[0]) * weight;
        total[1] += f64::from(position[1]) * weight;
        total[2] += f64::from(position[2]) * weight;
        total_weight += weight;
    }
    if !total_weight.is_finite() || total_weight <= f64::EPSILON {
        return centroid_f64(positions);
    }
    [
        total[0] / total_weight,
        total[1] / total_weight,
        total[2] / total_weight,
    ]
}

fn fit_translation_matrix(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
) -> Option<[f32; 16]> {
    if source.is_empty() || source.len() != target.len() {
        return None;
    }
    if let Some(matrix) = exact_translation_matrix(source, target) {
        return Some(matrix);
    }
    let source_center = weighted_centroid_f64(source, weights);
    let target_center = weighted_centroid_f64(target, weights);
    let translation = [
        (target_center[0] - source_center[0]) as f32,
        (target_center[1] - source_center[1]) as f32,
        (target_center[2] - source_center[2]) as f32,
    ];
    array_is_finite(translation).then_some(translation_matrix(translation))
}

fn translation_matrix(translation: [f32; 3]) -> [f32; 16] {
    [
        1.0,
        0.0,
        0.0,
        translation[0],
        0.0,
        1.0,
        0.0,
        translation[1],
        0.0,
        0.0,
        1.0,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn fit_rigid_matrix(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
) -> Option<SemanticAffineMatrix> {
    let rotation = fit_rotation(source, target, weights)?;
    let source_center = weighted_centroid_f64(source, weights);
    let target_center = weighted_centroid_f64(target, weights);
    let translation = exact_translation_for_linear(source, target, rotation)
        .or_else(|| transform_translation(rotation, 1.0, source_center, target_center))?;
    let candidate = rigid_matrix(rotation, translation);
    candidate
        .iter()
        .all(|value| value.is_finite())
        .then_some(SemanticAffineMatrix {
            matrix: candidate,
            parameters: AffineSemanticParameters {
                translation: Some(translation),
                rotation: Some(rotation),
                uniform_scale: None,
            },
        })
}

fn fit_similarity_matrix(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
) -> Option<SemanticAffineMatrix> {
    let rotation = fit_rotation(source, target, weights)?;
    let scale = fit_uniform_scale(source, target, weights, rotation)?;
    let source_center = weighted_centroid_f64(source, weights);
    let target_center = weighted_centroid_f64(target, weights);
    let linear = scaled_rotation_row_major_3x3(rotation, scale);
    let translation = exact_translation_for_linear(source, target, linear).or_else(|| {
        transform_translation(rotation, f64::from(scale), source_center, target_center)
    })?;
    let candidate = similarity_matrix(rotation, scale, translation);
    candidate
        .iter()
        .all(|value| value.is_finite())
        .then_some(SemanticAffineMatrix {
            matrix: candidate,
            parameters: AffineSemanticParameters {
                translation: Some(translation),
                rotation: Some(rotation),
                uniform_scale: Some(scale),
            },
        })
}

fn weighted_sum_squared_distance(left: &[[f32; 3]], right: &[[f32; 3]], weights: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .enumerate()
        .map(|(index, (a, b))| {
            let weight = weights.get(index).copied().unwrap_or(1.0);
            let dx = f64::from(a[0]) - f64::from(b[0]);
            let dy = f64::from(a[1]) - f64::from(b[1]);
            let dz = f64::from(a[2]) - f64::from(b[2]);
            weight * (dx * dx + dy * dy + dz * dz)
        })
        .sum()
}

fn weighted_centered_sum_squared_distance(positions: &[[f32; 3]], weights: &[f64]) -> f64 {
    let center = weighted_centroid_f64(positions, weights);
    positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let weight = weights.get(index).copied().unwrap_or(1.0);
            let dx = f64::from(position[0]) - center[0];
            let dy = f64::from(position[1]) - center[1];
            let dz = f64::from(position[2]) - center[2];
            weight * (dx * dx + dy * dy + dz * dz)
        })
        .sum()
}

fn fit_rotation(source: &[[f32; 3]], target: &[[f32; 3]], weights: &[f64]) -> Option<[f32; 9]> {
    if source.is_empty() || source.len() != target.len() {
        return None;
    }
    let source_center = weighted_centroid_f64(source, weights);
    let target_center = weighted_centroid_f64(target, weights);
    let mut covariance = [[0.0_f64; 3]; 3];
    let mut source_variance = 0.0_f64;
    let mut target_variance = 0.0_f64;
    for (index, (source_position, target_position)) in source.iter().zip(target).enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let source_centered = [
            f64::from(source_position[0]) - source_center[0],
            f64::from(source_position[1]) - source_center[1],
            f64::from(source_position[2]) - source_center[2],
        ];
        let target_centered = [
            f64::from(target_position[0]) - target_center[0],
            f64::from(target_position[1]) - target_center[1],
            f64::from(target_position[2]) - target_center[2],
        ];
        source_variance += weight * dot_f64(source_centered, source_centered);
        target_variance += weight * dot_f64(target_centered, target_centered);
        for row in 0..3 {
            for col in 0..3 {
                covariance[row][col] += weight * source_centered[row] * target_centered[col];
            }
        }
    }
    if source_variance <= f64::EPSILON || target_variance <= f64::EPSILON {
        return None;
    }
    let rotation = quaternion_to_rotation(largest_eigenvector_4x4(davenport_matrix(covariance))?);
    let rotation = snap_rotation(rotation);
    is_proper_rotation(rotation).then_some(rotation)
}

fn davenport_matrix(covariance: [[f64; 3]; 3]) -> [[f64; 4]; 4] {
    let sxx = covariance[0][0];
    let sxy = covariance[0][1];
    let sxz = covariance[0][2];
    let syx = covariance[1][0];
    let syy = covariance[1][1];
    let syz = covariance[1][2];
    let szx = covariance[2][0];
    let szy = covariance[2][1];
    let szz = covariance[2][2];
    [
        [sxx + syy + szz, syz - szy, szx - sxz, sxy - syx],
        [syz - szy, sxx - syy - szz, sxy + syx, szx + sxz],
        [szx - sxz, sxy + syx, -sxx + syy - szz, syz + szy],
        [sxy - syx, szx + sxz, syz + szy, -sxx - syy + szz],
    ]
}

fn largest_eigenvector_4x4(matrix: [[f64; 4]; 4]) -> Option<[f64; 4]> {
    let (eigenvalues, eigenvectors) = symmetric_eigendecomposition(matrix);
    let (column, _) = eigenvalues
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .max_by(|(_, left), (_, right)| left.total_cmp(right))?;
    let mut vector = [
        eigenvectors[0][column],
        eigenvectors[1][column],
        eigenvectors[2][column],
        eigenvectors[3][column],
    ];
    let length = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return None;
    }
    for value in &mut vector {
        *value /= length;
    }
    Some(vector)
}

fn quaternion_to_rotation(quaternion: [f64; 4]) -> [f32; 9] {
    let [w, x, y, z] = quaternion;
    [
        (1.0 - 2.0 * (y * y + z * z)) as f32,
        (2.0 * (x * y - z * w)) as f32,
        (2.0 * (x * z + y * w)) as f32,
        (2.0 * (x * y + z * w)) as f32,
        (1.0 - 2.0 * (x * x + z * z)) as f32,
        (2.0 * (y * z - x * w)) as f32,
        (2.0 * (x * z - y * w)) as f32,
        (2.0 * (y * z + x * w)) as f32,
        (1.0 - 2.0 * (x * x + y * y)) as f32,
    ]
}

fn snap_rotation(mut rotation: [f32; 9]) -> [f32; 9] {
    for value in &mut rotation {
        if value.abs() <= 1.0e-6 {
            *value = 0.0;
        } else if (*value - 1.0).abs() <= 1.0e-6 {
            *value = 1.0;
        } else if (*value + 1.0).abs() <= 1.0e-6 {
            *value = -1.0;
        }
    }
    rotation
}

fn fit_uniform_scale(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    weights: &[f64],
    rotation: [f32; 9],
) -> Option<f32> {
    let source_center = weighted_centroid_f64(source, weights);
    let target_center = weighted_centroid_f64(target, weights);
    let mut numerator = 0.0_f64;
    let mut denominator = 0.0_f64;
    for (index, (source_position, target_position)) in source.iter().zip(target).enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let source_centered = [
            f64::from(source_position[0]) - source_center[0],
            f64::from(source_position[1]) - source_center[1],
            f64::from(source_position[2]) - source_center[2],
        ];
        let target_centered = [
            f64::from(target_position[0]) - target_center[0],
            f64::from(target_position[1]) - target_center[1],
            f64::from(target_position[2]) - target_center[2],
        ];
        let rotated = apply_rotation_f64(rotation, source_centered);
        numerator += weight * dot_f64(target_centered, rotated);
        denominator += weight * dot_f64(source_centered, source_centered);
    }
    if !numerator.is_finite() || !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let scale = (numerator / denominator) as f32;
    (scale.is_finite() && scale > 0.0).then_some(scale)
}

fn transform_translation(
    rotation: [f32; 9],
    scale: f64,
    source_center: [f64; 3],
    target_center: [f64; 3],
) -> Option<[f32; 3]> {
    let rotated_center = apply_rotation_f64(rotation, source_center);
    let translation = [
        (target_center[0] - scale * rotated_center[0]) as f32,
        (target_center[1] - scale * rotated_center[1]) as f32,
        (target_center[2] - scale * rotated_center[2]) as f32,
    ];
    array_is_finite(translation).then_some(translation)
}

fn apply_rotation_f64(rotation: [f32; 9], position: [f64; 3]) -> [f64; 3] {
    [
        f64::from(rotation[0]) * position[0]
            + f64::from(rotation[1]) * position[1]
            + f64::from(rotation[2]) * position[2],
        f64::from(rotation[3]) * position[0]
            + f64::from(rotation[4]) * position[1]
            + f64::from(rotation[5]) * position[2],
        f64::from(rotation[6]) * position[0]
            + f64::from(rotation[7]) * position[1]
            + f64::from(rotation[8]) * position[2],
    ]
}

fn exact_translation_for_linear(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    linear: [f32; 9],
) -> Option<[f32; 3]> {
    if source.is_empty() || source.len() != target.len() {
        return None;
    }
    let first = apply_linear(source[0], linear);
    let translation = [
        target[0][0] - first[0],
        target[0][1] - first[1],
        target[0][2] - first[2],
    ];
    if !array_is_finite(translation) {
        return None;
    }
    let exact = source.iter().zip(target).all(|(source, target)| {
        let linear_position = apply_linear(*source, linear);
        (0..3).all(|axis| {
            canonical_f32_add(linear_position[axis], translation[axis]).to_bits()
                == target[axis].to_bits()
        })
    });
    exact.then_some(translation)
}

fn apply_linear(position: [f32; 3], linear: [f32; 9]) -> [f32; 3] {
    [
        apply_linear_row(position, &linear[0..3]),
        apply_linear_row(position, &linear[3..6]),
        apply_linear_row(position, &linear[6..9]),
    ]
}

fn apply_linear_row(position: [f32; 3], row: &[f32]) -> f32 {
    let mut value = canonical_f32_mul(row[0], position[0]);
    value = canonical_f32_add(value, canonical_f32_mul(row[1], position[1]));
    canonical_f32_add(value, canonical_f32_mul(row[2], position[2]))
}

fn scaled_rotation_row_major_3x3(rotation: [f32; 9], scale: f32) -> [f32; 9] {
    [
        canonical_f32_mul(scale, rotation[0]),
        canonical_f32_mul(scale, rotation[1]),
        canonical_f32_mul(scale, rotation[2]),
        canonical_f32_mul(scale, rotation[3]),
        canonical_f32_mul(scale, rotation[4]),
        canonical_f32_mul(scale, rotation[5]),
        canonical_f32_mul(scale, rotation[6]),
        canonical_f32_mul(scale, rotation[7]),
        canonical_f32_mul(scale, rotation[8]),
    ]
}

fn rigid_matrix(rotation: [f32; 9], translation: [f32; 3]) -> [f32; 16] {
    [
        rotation[0],
        rotation[1],
        rotation[2],
        translation[0],
        rotation[3],
        rotation[4],
        rotation[5],
        translation[1],
        rotation[6],
        rotation[7],
        rotation[8],
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn similarity_matrix(rotation: [f32; 9], scale: f32, translation: [f32; 3]) -> [f32; 16] {
    [
        canonical_f32_mul(scale, rotation[0]),
        canonical_f32_mul(scale, rotation[1]),
        canonical_f32_mul(scale, rotation[2]),
        translation[0],
        canonical_f32_mul(scale, rotation[3]),
        canonical_f32_mul(scale, rotation[4]),
        canonical_f32_mul(scale, rotation[5]),
        translation[1],
        canonical_f32_mul(scale, rotation[6]),
        canonical_f32_mul(scale, rotation[7]),
        canonical_f32_mul(scale, rotation[8]),
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn is_proper_rotation(rotation: [f32; 9]) -> bool {
    if !array_is_finite(rotation) {
        return false;
    }
    let rows = [&rotation[0..3], &rotation[3..6], &rotation[6..9]];
    let unit_rows = rows
        .iter()
        .all(|row| (row_length(row) - 1.0).abs() <= ROTATION_ORTHONORMAL_TOLERANCE);
    let orthogonal_rows = dot(rows[0], rows[1]).abs() <= ROTATION_ORTHONORMAL_TOLERANCE
        && dot(rows[0], rows[2]).abs() <= ROTATION_ORTHONORMAL_TOLERANCE
        && dot(rows[1], rows[2]).abs() <= ROTATION_ORTHONORMAL_TOLERANCE;
    let determinant = determinant_3x3(rotation);
    unit_rows
        && orthogonal_rows
        && determinant.is_finite()
        && (determinant - 1.0).abs() <= ROTATION_ORTHONORMAL_TOLERANCE
}

fn row_length(row: &[f32]) -> f64 {
    dot(row, row).sqrt()
}

fn dot(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum()
}

fn dot_f64(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn determinant_3x3(matrix: [f32; 9]) -> f64 {
    let a = f64::from(matrix[0]);
    let b = f64::from(matrix[1]);
    let c = f64::from(matrix[2]);
    let d = f64::from(matrix[3]);
    let e = f64::from(matrix[4]);
    let f = f64::from(matrix[5]);
    let g = f64::from(matrix[6]);
    let h = f64::from(matrix[7]);
    let i = f64::from(matrix[8]);
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

fn fit_affine(source: &[[f32; 3]], target: &[[f32; 3]], weights: &[f64]) -> Option<[f32; 16]> {
    if source.is_empty() || source.len() != target.len() {
        return None;
    }
    if let Some(matrix) = exact_translation_matrix(source, target) {
        return Some(matrix);
    }

    let source_center = weighted_centroid_f64(source, weights);
    let mut source_scale = 0.0_f64;
    for position in source {
        for axis in 0..3 {
            source_scale =
                source_scale.max((f64::from(position[axis]) - source_center[axis]).abs());
        }
    }
    if !source_scale.is_finite() || source_scale <= f64::EPSILON {
        let target_center = weighted_centroid_f64(target, weights);
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
    for (index, (source_position, target_position)) in source.iter().zip(target).enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let p = [
            (f64::from(source_position[0]) - source_center[0]) / source_scale,
            (f64::from(source_position[1]) - source_center[1]) / source_scale,
            (f64::from(source_position[2]) - source_center[2]) / source_scale,
            1.0,
        ];
        for row in 0..4 {
            for col in 0..4 {
                normal[row][col] += weight * p[row] * p[col];
            }
            rhs[0][row] += weight * p[row] * f64::from(target_position[0]);
            rhs[1][row] += weight * p[row] * f64::from(target_position[1]);
            rhs[2][row] += weight * p[row] * f64::from(target_position[2]);
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
    exact.then_some(translation_matrix(delta))
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

fn matrices_bit_equal<const N: usize>(left: [f32; N], right: [f32; N]) -> bool {
    left.iter()
        .zip(right)
        .all(|(left, right)| f32_bits_equal(*left, right))
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

fn array_is_finite<const N: usize>(value: [f32; N]) -> bool {
    value.iter().all(|component| component.is_finite())
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
ROTATION_ORTHONORMAL_TOLERANCE = 1.0e-3


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


def is_finite_number(value):
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
    )


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
    if len(matrix) != 16 or not all(is_finite_number(value) for value in matrix):
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


def triangle_area(a, b, c):
    ab = (
        float(b[0]) - float(a[0]),
        float(b[1]) - float(a[1]),
        float(b[2]) - float(a[2]),
    )
    ac = (
        float(c[0]) - float(a[0]),
        float(c[1]) - float(a[1]),
        float(c[2]) - float(a[2]),
    )
    cross = (
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    )
    return 0.5 * math.sqrt(sum(component * component for component in cross))


def vertex_area_weights(positions, indices):
    weights = [0.0] * len(positions)
    for offset in range(0, len(indices), 3):
        a, b, c = indices[offset : offset + 3]
        if a >= len(positions) or b >= len(positions) or c >= len(positions):
            continue
        area = triangle_area(positions[a], positions[b], positions[c])
        if math.isfinite(area) and area > 0.0:
            share = area / 3.0
            weights[a] += share
            weights[b] += share
            weights[c] += share
    total = sum(weights)
    if not math.isfinite(total) or total <= sys.float_info.epsilon:
        return [1.0] * len(positions)
    average = total / max(len(positions), 1)
    normalized = []
    for weight in weights:
        if not math.isfinite(weight) or weight <= 0.0:
            weight = average
        normalized.append(weight / average)
    return normalized


def weighted_sum_squared_distance(left, right, weights):
    total = 0.0
    for index, (left_position, right_position) in enumerate(zip(left, right)):
        weight = weights[index] if index < len(weights) else 1.0
        if not math.isfinite(weight) or weight <= 0.0:
            continue
        error = sum(
            (float(a) - float(b)) ** 2 for a, b in zip(left_position, right_position)
        )
        total += error * weight
    return total


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


def matrices_bit_equal(left, right):
    return all(f32_bits(actual) == f32_bits(expected) for actual, expected in zip(left, right))


def require_vector(operator, key, length, label):
    value = operator.get(key)
    if (
        not isinstance(value, list)
        or len(value) != length
        or not all(is_finite_number(component) for component in value)
    ):
        raise ValueError(f"{label} is missing finite {key}")
    return [f32(component) for component in value]


def reject_semantic_parameters(operator, keys, label):
    for key in keys:
        if operator.get(key) is not None:
            raise ValueError(f"{label} must not declare {key}")


def translation_matrix(translation):
    tx, ty, tz = translation
    return [
        1.0, 0.0, 0.0, tx,
        0.0, 1.0, 0.0, ty,
        0.0, 0.0, 1.0, tz,
        0.0, 0.0, 0.0, 1.0,
    ]


def rigid_matrix(rotation, translation):
    tx, ty, tz = translation
    return [
        rotation[0], rotation[1], rotation[2], tx,
        rotation[3], rotation[4], rotation[5], ty,
        rotation[6], rotation[7], rotation[8], tz,
        0.0, 0.0, 0.0, 1.0,
    ]


def similarity_matrix(rotation, scale, translation):
    tx, ty, tz = translation
    return [
        f32(scale * rotation[0]), f32(scale * rotation[1]), f32(scale * rotation[2]), tx,
        f32(scale * rotation[3]), f32(scale * rotation[4]), f32(scale * rotation[5]), ty,
        f32(scale * rotation[6]), f32(scale * rotation[7]), f32(scale * rotation[8]), tz,
        0.0, 0.0, 0.0, 1.0,
    ]


def dot(left, right):
    return sum(float(a) * float(b) for a, b in zip(left, right))


def row_length(row):
    return math.sqrt(dot(row, row))


def determinant_3x3(matrix):
    a, b, c, d, e, f, g, h, i = (float(value) for value in matrix)
    return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)


def is_proper_rotation(rotation):
    rows = [rotation[0:3], rotation[3:6], rotation[6:9]]
    unit_rows = all(
        abs(row_length(row) - 1.0) <= ROTATION_ORTHONORMAL_TOLERANCE for row in rows
    )
    orthogonal_rows = (
        abs(dot(rows[0], rows[1])) <= ROTATION_ORTHONORMAL_TOLERANCE
        and abs(dot(rows[0], rows[2])) <= ROTATION_ORTHONORMAL_TOLERANCE
        and abs(dot(rows[1], rows[2])) <= ROTATION_ORTHONORMAL_TOLERANCE
    )
    determinant = determinant_3x3(rotation)
    return (
        unit_rows
        and orthogonal_rows
        and math.isfinite(determinant)
        and abs(determinant - 1.0) <= ROTATION_ORTHONORMAL_TOLERANCE
    )


def replay_operators(manifest, source_positions, source_indices, target_positions):
    current = list(source_positions)
    stages = []
    saw_lossless = False
    operator_ids = set()
    operator_labels = set()
    identity_error = sum_squared_distance(source_positions, target_positions)
    weights = vertex_area_weights(source_positions, source_indices)
    weighted_identity_error = weighted_sum_squared_distance(
        source_positions, target_positions, weights
    )
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
            if len(matrix) != 16 or not all(is_finite_number(value) for value in matrix):
                raise ValueError("global affine matrix must contain sixteen finite values")
            if any(
                f32_bits(actual) != f32_bits(expected)
                for actual, expected in zip(matrix[12:16], (0.0, 0.0, 0.0, 1.0))
            ):
                raise ValueError("global affine matrix bottom row must be [0, 0, 0, 1]")
            semantic_family = operator.get("semantic_family", "general_affine")
            if semantic_family == "translation":
                reject_semantic_parameters(
                    operator, ("rotation_row_major_3x3", "uniform_scale"), "translation affine"
                )
                translation = require_vector(operator, "translation", 3, "translation affine")
                if not matrices_bit_equal(matrix, translation_matrix(translation)):
                    raise ValueError("translation affine matrix does not match its parameters")
            elif semantic_family == "general_affine":
                reject_semantic_parameters(
                    operator,
                    ("translation", "rotation_row_major_3x3", "uniform_scale"),
                    "general affine",
                )
            elif semantic_family == "rigid_transform":
                if operator.get("uniform_scale") is not None:
                    raise ValueError("rigid transform must not declare uniform_scale")
                translation = require_vector(operator, "translation", 3, "rigid transform")
                rotation = require_vector(
                    operator, "rotation_row_major_3x3", 9, "rigid transform"
                )
                if not is_proper_rotation(rotation):
                    raise ValueError("rigid transform rotation is not a proper basis")
                if not matrices_bit_equal(matrix, rigid_matrix(rotation, translation)):
                    raise ValueError("rigid transform matrix does not match its parameters")
            elif semantic_family == "similarity_transform":
                translation = require_vector(operator, "translation", 3, "similarity transform")
                rotation = require_vector(
                    operator, "rotation_row_major_3x3", 9, "similarity transform"
                )
                scale = operator.get("uniform_scale")
                if not is_finite_number(scale):
                    raise ValueError("similarity transform is missing a valid uniform_scale")
                scale = f32(scale)
                if not math.isfinite(scale) or scale <= 0.0:
                    raise ValueError("similarity transform is missing a valid uniform_scale")
                if not is_proper_rotation(rotation):
                    raise ValueError("similarity transform rotation is not a proper basis")
                if not matrices_bit_equal(matrix, similarity_matrix(rotation, scale, translation)):
                    raise ValueError("similarity transform matrix does not match its parameters")
            else:
                raise ValueError(f"unsupported affine semantic family: {semantic_family}")
            baked = read_positions(operator["baked_positions_file"], len(source_positions))
            evaluated = apply_affine(source_positions, matrix)
            if not positions_bit_equal(evaluated, baked):
                raise ValueError("baked affine positions do not match the serialized matrix")
            affine_error = sum_squared_distance(evaluated, target_positions)
            weighted_affine_error = weighted_sum_squared_distance(
                evaluated, target_positions, weights
            )
            expected_explained = f32(explained_fraction(identity_error, affine_error))
            expected_weighted_explained = explained_fraction(
                weighted_identity_error, weighted_affine_error
            )
            expected_max_error = max_euclidean_distance(evaluated, target_positions)
            if f32_bits(operator["explained_displacement_fraction"]) != f32_bits(
                expected_explained
            ):
                raise ValueError("global affine explained-displacement metadata is inconsistent")
            if f32_bits(operator["max_remaining_error"]) != f32_bits(expected_max_error):
                raise ValueError("global affine remaining-error metadata is inconsistent")
            if (
                weighted_identity_error <= 0.0
                or weighted_affine_error >= weighted_identity_error
                or expected_weighted_explained
                < float(f32(manifest["settings"]["affine_min_explained"]))
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
        or not is_finite_number(affine_min_explained)
        or not 0.0 <= affine_min_explained <= 1.0
    ):
        raise ValueError("affine_min_explained must be finite and between zero and one")
    if (
        not isinstance(residual_epsilon, (int, float))
        or not is_finite_number(residual_epsilon)
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
        manifest, source_positions, source_indices, target_positions
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
    use std::fmt::Write as _;
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
            Some(OperatorManifest::GlobalAffine {
                semantic_family: AffineSemanticFamily::GeneralAffine,
                translation: None,
                ..
            })
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

        let Some(OperatorManifest::GlobalAffine {
            id,
            label,
            semantic_family,
            translation,
            matrix_row_major_4x4,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected translation affine operator");
        };
        assert_eq!(id, "op-0000-translation");
        assert_eq!(label, "Translation");
        assert_eq!(*semantic_family, AffineSemanticFamily::Translation);
        assert_eq!(*translation, Some([0.5, -0.25, 2.0]));
        assert_eq!(*matrix_row_major_4x4, translation_matrix([0.5, -0.25, 2.0]));
        assert!(result.residual_indices.is_empty());
        assert!(result.residual_positions.is_empty());
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn exact_rigid_transform_is_labeled_as_rigid() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [-position[1] + 0.25, position[0] - 0.5, position[2] + 1.0]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family,
            translation,
            rotation_row_major_3x3,
            uniform_scale,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected rigid affine operator");
        };
        assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
        assert_eq!(*translation, Some([0.25, -0.5, 1.0]));
        assert_eq!(
            *rotation_row_major_3x3,
            Some([0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(*uniform_scale, None);
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn large_translation_does_not_hide_rigid_transform() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                -position[1] + 1000.0,
                position[0] - 1000.0,
                position[2] + 500.0,
            ]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family,
            translation,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected rigid affine operator");
        };
        assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
        assert_eq!(*translation, Some([1000.0, -1000.0, 500.0]));
        assert!(result.residual_indices.is_empty());
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn planar_rigid_transform_is_labeled_as_rigid() {
        let source = square_mesh();
        let target = transformed_mesh(&source, |position| {
            [-position[1] + 0.25, position[0] - 0.5, position[2]]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family,
            translation,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected rigid affine operator");
        };
        assert_eq!(*semantic_family, AffineSemanticFamily::RigidTransform);
        assert_eq!(*translation, Some([0.25, -0.5, 0.0]));
        assert!(result.residual_indices.is_empty());
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn area_weighted_translation_fit_resists_dense_small_patch_bias() {
        let source = uneven_area_mesh();
        let mut target = source.clone();
        for (index, position) in target.positions.iter_mut().enumerate() {
            if index < 4 {
                position[0] += 10.0;
            } else {
                position[0] -= 10.0;
            }
        }
        let weights = vertex_area_weights(&source);

        let matrix = fit_translation_matrix(&source.positions, &target.positions, &weights)
            .expect("translation fit");

        assert!(
            matrix[3] > 9.0,
            "large sparse surface should dominate x translation, got {}",
            matrix[3]
        );
        assert_eq!(matrix[7], 0.0);
        assert_eq!(matrix[11], 0.0);
    }

    #[test]
    fn surface_weighted_eligibility_resists_dense_small_patch_bias() {
        let source = uneven_area_mesh();
        let mut target = source.clone();
        for position in target.positions.iter_mut().take(4) {
            position[0] += 10.0;
        }

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family,
            explained_displacement_fraction,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected weighted translation operator");
        };
        assert_eq!(*semantic_family, AffineSemanticFamily::Translation);
        assert_eq!(*explained_displacement_fraction, 0.0);
        let selected = result
            .inference_diagnostics
            .hypotheses
            .iter()
            .find(|hypothesis| hypothesis.selected)
            .expect("selected hypothesis");
        assert_eq!(selected.family, OperatorFamily::Translation);
        assert!(selected.weighted_explained_fraction > 0.99);
        assert_eq!(selected.raw_explained_fraction, 0.0);
        assert!(selected.exact_residual_bytes > 0);
        assert!(position_slices_bit_equal(
            &result.reconstructed_positions,
            &target.positions
        ));
    }

    #[test]
    fn approximate_residual_cost_ignores_one_ulp_audit_differences() {
        let source = cube_mesh();
        let mut target = source.positions.clone();
        for position in &mut target {
            position[0] = f32::from_bits(position[0].to_bits() + 1);
        }
        let weights = vertex_area_weights(&source);

        assert_eq!(
            exact_residual_storage_size(&source.positions, &target),
            source.positions.len() * (std::mem::size_of::<u32>() + 3 * std::mem::size_of::<f32>())
        );
        assert_eq!(
            approximate_residual_cost(&source.positions, &target, &weights),
            0.0
        );
    }

    #[test]
    fn approximate_residual_cost_is_origin_invariant() {
        let base = cube_mesh();
        let mut expected_family: Option<OperatorFamily> = None;
        let mut expected_cost: Option<f64> = None;
        for offset in [
            [0.0, 0.0, 0.0],
            [1_000.0, -1_000.0, 500.0],
            [1_000_000.0, -1_000_000.0, 500_000.0],
        ] {
            let source = transformed_mesh(&base, |position| {
                [
                    position[0] + offset[0],
                    position[1] + offset[1],
                    position[2] + offset[2],
                ]
            });
            let mut target = source.clone();
            target.positions[6][0] += 0.125;
            let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
            let selected = result
                .inference_diagnostics
                .hypotheses
                .iter()
                .find(|hypothesis| hypothesis.selected)
                .expect("selected hypothesis");

            if let Some(expected_family) = expected_family {
                assert_eq!(selected.family, expected_family);
            } else {
                expected_family = Some(selected.family);
            }
            if let Some(expected_cost) = expected_cost {
                assert!(
                    (selected.approximate_residual_cost - expected_cost).abs() <= 1.0e-12,
                    "offset {offset:?} changed approximate residual cost from {expected_cost} to {}",
                    selected.approximate_residual_cost
                );
            } else {
                expected_cost = Some(selected.approximate_residual_cost);
            }
        }
    }

    #[test]
    fn approximate_residual_cost_keeps_small_object_residuals_visible() {
        let source = transformed_mesh(&cube_mesh(), |position| {
            [
                position[0] * 1.0e-3,
                position[1] * 1.0e-3,
                position[2] * 1.0e-3,
            ]
        });
        let mut target = source.clone();
        target.positions[6][0] += 2.0e-6;
        let weights = vertex_area_weights(&source);

        assert!(
            approximate_residual_cost(&source.positions, &target.positions, &weights) > 0.0,
            "absolute floor should not be replaced by a one-unit relative scale floor"
        );
    }

    #[test]
    fn diagnostics_scores_are_reproducible_from_components() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                -1.01 * position[1] + 0.25,
                1.01 * position[0] - 0.5,
                1.01 * position[2] + 1.5,
            ]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        assert_eq!(
            result.inference_diagnostics.scoring_policy.model,
            "weighted_affine_origin_invariant_residual_v1"
        );
        assert_eq!(
            result
                .inference_diagnostics
                .scoring_policy
                .family_priors
                .get(&OperatorFamily::GeneralAffine),
            Some(&1.0e-2)
        );
        for hypothesis in &result.inference_diagnostics.hypotheses {
            let policy = &result.inference_diagnostics.scoring_policy;
            let recomputed_normalized =
                hypothesis.weighted_geometric_error / hypothesis.error_normalization_scale;
            let recomputed_parameter = hypothesis.parameter_count as f64 * policy.parameter_weight;
            let recomputed_metadata = hypothesis.semantic_metadata_bytes as f64
                / hypothesis.literal_size_bytes as f64
                * policy.semantic_metadata_weight;
            let recomputed_approximate =
                hypothesis.approximate_residual_coverage * policy.approximate_residual_weight;
            let recomputed_exact = hypothesis.exact_residual_bytes as f64
                / hypothesis.literal_size_bytes as f64
                * policy.exact_residual_weight;
            let recomputed_prior = *policy
                .family_priors
                .get(&hypothesis.family)
                .expect("family prior");
            assert!(
                (recomputed_normalized - hypothesis.normalized_geometric_error_cost).abs()
                    <= 1.0e-12,
                "normalized error mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (recomputed_parameter - hypothesis.parameter_cost).abs() <= 1.0e-12,
                "parameter cost mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (recomputed_metadata - hypothesis.semantic_metadata_cost).abs() <= 1.0e-12,
                "metadata cost mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (recomputed_approximate - hypothesis.approximate_residual_cost).abs() <= 1.0e-12,
                "approximate residual cost mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (recomputed_exact - hypothesis.exact_residual_cost).abs() <= 1.0e-12,
                "exact residual cost mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (recomputed_prior - hypothesis.prior_penalty).abs() <= 1.0e-12,
                "prior mismatch for {:?}",
                hypothesis.family
            );
            let component_sum = recomputed_normalized
                + recomputed_parameter
                + recomputed_metadata
                + recomputed_approximate
                + recomputed_exact
                + recomputed_prior;
            assert!(
                (component_sum - hypothesis.score_component_sum).abs() <= 1.0e-12,
                "component sum mismatch for {:?}",
                hypothesis.family
            );
            assert!(
                (hypothesis.score_component_sum - hypothesis.total_score).abs() <= 1.0e-12,
                "total score mismatch for {:?}",
                hypothesis.family
            );
        }
    }

    #[test]
    fn exact_similarity_transform_is_labeled_as_similarity() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                -2.0 * position[1] + 0.25,
                2.0 * position[0] - 0.5,
                2.0 * position[2] + 1.0,
            ]
        });

        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();

        let Some(OperatorManifest::GlobalAffine {
            semantic_family,
            translation,
            rotation_row_major_3x3,
            uniform_scale,
            ..
        }) = result.manifest.operators.first()
        else {
            panic!("expected similarity affine operator");
        };
        assert_eq!(*semantic_family, AffineSemanticFamily::SimilarityTransform);
        assert_eq!(*translation, Some([0.25, -0.5, 1.0]));
        assert_eq!(
            *rotation_row_major_3x3,
            Some([0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(*uniform_scale, Some(2.0));
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
        assert!(paths.inference_diagnostics.exists());
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

        let diagnostics: InferenceDiagnostics =
            serde_json::from_str(&fs::read_to_string(paths.inference_diagnostics).unwrap())
                .unwrap();
        assert_eq!(diagnostics.diagnostics_schema_version, 1);
        assert_eq!(diagnostics.package_schema_version, SCHEMA_VERSION);
        assert_eq!(diagnostics.selected_hypothesis_index, 2);
        assert_eq!(
            diagnostics
                .hypotheses
                .iter()
                .filter(|hypothesis| hypothesis.selected)
                .count(),
            1
        );

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
    fn package_verifier_rejects_tampered_translation_metadata() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.5, position[1] - 0.25, position[2] + 2.0]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let translation = manifest
            .operators
            .iter_mut()
            .find_map(|operator| match operator {
                OperatorManifest::GlobalAffine { translation, .. } => translation.as_mut(),
                OperatorManifest::LosslessCorrection { .. } => None,
            })
            .unwrap();
        translation[0] += 0.25;
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_verifier_rejects_tampered_similarity_metadata() {
        let source = cube_mesh();
        let target = transformed_mesh(&source, |position| {
            [
                -2.0 * position[1] + 0.25,
                2.0 * position[0] - 0.5,
                2.0 * position[2] + 1.0,
            ]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: DecompileManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let uniform_scale = manifest
            .operators
            .iter_mut()
            .find_map(|operator| match operator {
                OperatorManifest::GlobalAffine { uniform_scale, .. } => uniform_scale.as_mut(),
                OperatorManifest::LosslessCorrection { .. } => None,
            })
            .unwrap();
        *uniform_scale = 2.25;
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = verify_decompile_package(&package).unwrap_err();

        assert!(matches!(error, DecompileError::InvalidPackage { .. }));
    }

    #[test]
    fn package_verifier_accepts_schema_two_affine_without_semantic_metadata() {
        let source = tetra_mesh();
        let target = transformed_mesh(&source, |position| {
            [position[0] + 0.5, position[1] - 0.25, position[2] + 2.0]
        });
        let result = decompile_pair(&source, &target, DecompileSettings::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("package");
        write_decompile_package(&result, &source, &target, &package).unwrap();
        let manifest_path = package.join(MANIFEST_FILE);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let operators = manifest["operators"].as_array_mut().unwrap();
        let affine = operators
            .iter_mut()
            .find(|operator| operator["kind"] == "global_affine")
            .unwrap();
        affine.as_object_mut().unwrap().remove("semantic_family");
        affine.as_object_mut().unwrap().remove("translation");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let report = verify_decompile_package(&package).unwrap();

        assert!(report.positions_bit_exact);
        assert_eq!(report.max_euclidean_error, 0.0);
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
        assert!(script.contains("def is_finite_number(value):"));
        assert!(script.contains("matrix = [f32(value) for value in matrix]"));
        assert!(
            script.contains(
                "semantic_family = operator.get(\"semantic_family\", \"general_affine\")"
            )
        );
        assert!(script.contains("translation affine matrix does not match its parameters"));
        assert!(script.contains("general affine"));
        assert!(script.contains("must not declare"));
        assert!(script.contains("semantic_family == \"rigid_transform\""));
        assert!(script.contains("semantic_family == \"similarity_transform\""));
        assert!(script.contains("rotation is not a proper basis"));
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

    fn square_mesh() -> TriangleMesh {
        read_obj(Cursor::new(
            "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3
f 1 3 4
",
        ))
        .unwrap()
    }

    fn uneven_area_mesh() -> TriangleMesh {
        let grid = 5_usize;
        let mut obj = String::from(
            "\
v -50 -50 0
v 50 -50 0
v 50 50 0
v -50 50 0
f 1 2 3
f 1 3 4
",
        );
        for y in 0..=grid {
            for x in 0..=grid {
                let px = 200.0 + x as f32 / grid as f32;
                let py = 200.0 + y as f32 / grid as f32;
                writeln!(&mut obj, "v {px} {py} 0").unwrap();
            }
        }
        let start = 5_usize;
        let row = grid + 1;
        for y in 0..grid {
            for x in 0..grid {
                let a = start + y * row + x;
                let b = a + 1;
                let c = a + row;
                let d = c + 1;
                writeln!(&mut obj, "f {a} {b} {d}").unwrap();
                writeln!(&mut obj, "f {a} {d} {c}").unwrap();
            }
        }
        read_obj(Cursor::new(obj)).unwrap()
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
