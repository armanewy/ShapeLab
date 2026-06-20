//! Schema-3 package manifest contracts.
//!
//! Schema 3 separates semantic evaluation from exact replay. Semantic
//! operators explain editable intent, while each stage's cumulative baked
//! positions file is authoritative for exact reconstruction.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;
use thiserror::Error;

use super::bend::BendParameters;
use super::blender::{BlenderAdapterOptions, blender_reconstruction_script_v3};
use super::diagnostics::{
    DIAGNOSTICS_SCHEMA_VERSION_V4, InferenceDiagnosticsV4, ProgramCorrectionDiagnostics,
    ProgramDiagnosticsInput, ProgramOperatorDiagnostics, StageDiagnosticsInput,
    build_program_diagnostics, build_stage_diagnostics, default_scoring_policy_v4,
    default_timing_by_phase_v4,
};
use super::program::{
    AffineOperator, OperatorId, OperatorProgram, ProgramOperator, SemanticVerificationMode,
    SemanticVerificationPolicy, SemanticVerificationReport, StageIndex, evaluate_operator,
    validate_program,
};
use crate::{
    AffineSemanticFamily, BLENDER_SCRIPT_FILE, DecompileError, INFERENCE_DIAGNOSTICS_FILE,
    MANIFEST_FILE, PACKAGE_VERIFICATION_FILE, PackagePaths, RESIDUAL_INDEX_FILE,
    RESIDUAL_POSITION_FILE, SOURCE_MESHBIN, StagedPackageDirectory, TARGET_MESHBIN,
    approximate_residual_cost, ensure_identical_topology, ensure_strictly_increasing_indices,
    invalid_package, package_path, path_io, position_slices_bit_equal, positions_bit_equal,
    read_meshbin, read_positions, read_u32s, resolve_package_asset, sum_squared_distance,
    topology_hash, topology_hash_from_parts, validate_decompile_mesh, vertex_area_weights,
    weighted_centered_sum_squared_distance, weighted_sum_squared_distance, write_json,
    write_meshbin, write_positions, write_text, write_u32s,
};

/// Decompiler package manifest schema version for schema 3.
pub const SCHEMA_VERSION_V3: u32 = 3;
const COORDINATE_SYSTEM_V3: &str = "right-handed-y-up";
const OPERATORS_DIR: &str = "operators";
const RESIDUAL_DIR: &str = "residual";
const AFFINE_STAGE_SLUG: &str = "affine";
const BEND_STAGE_SLUG: &str = "bend";
const LOSSLESS_STAGE_SLUG: &str = "lossless-correction";

/// Top-level schema-3 manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompileManifestV3 {
    /// Manifest schema version. Must be [`SCHEMA_VERSION_V3`].
    pub schema_version: u32,
    /// Coordinate convention label for package consumers.
    pub coordinate_system: String,
    /// Numeric encoding contract for all schema-3 sidecars.
    pub numeric_format: NumericFormatV3,
    /// Source mesh asset reference.
    pub source: MeshAssetV3,
    /// Target mesh asset reference.
    pub target: MeshAssetV3,
    /// Exact topology summary shared by source and target.
    pub topology: TopologySummaryV3,
    /// Ordered package operators, including the terminal lossless correction.
    pub operators: Vec<OperatorManifestV3>,
    /// Optional package replay report produced after reading sidecars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_verification: Option<PackageVerificationReportV3>,
}

/// Numeric metadata embedded in a schema-3 manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericFormatV3 {
    /// Scalar type for positions.
    pub scalar: String,
    /// Binary sidecar byte order.
    pub endian: String,
    /// Canonical affine arithmetic contract.
    pub affine_evaluation: String,
}

impl Default for NumericFormatV3 {
    fn default() -> Self {
        Self {
            scalar: "float32".to_owned(),
            endian: "little".to_owned(),
            affine_evaluation: "float32_stepwise_no_fma".to_owned(),
        }
    }
}

/// Mesh asset reference stored in a schema-3 package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAssetV3 {
    /// Package-relative mesh payload path.
    pub path: String,
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangles.
    pub triangle_count: usize,
}

/// Exact topology summary shared by source and target meshes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySummaryV3 {
    /// Number of vertices.
    pub vertex_count: usize,
    /// Number of triangles.
    pub triangle_count: usize,
    /// Number of triangle indices.
    pub index_count: usize,
    /// Diagnostic topology fingerprint.
    pub hash: String,
}

/// Cumulative baked stage manifest for a schema-3 package operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageManifestV3 {
    /// Zero-based stage index in package order.
    pub stage_index: StageIndex,
    /// Stable operator identifier for this stage.
    pub operator_id: OperatorId,
    /// Human-facing stage label.
    pub label: String,
    /// Package-relative cumulative baked positions file.
    pub baked_positions_file: String,
    /// Policy used to compare semantic evaluation to the baked stage.
    pub semantic_verification_policy: SemanticVerificationPolicy,
    /// Report from comparing semantic evaluation to the baked stage.
    pub semantic_verification_report: SemanticVerificationReport,
}

/// One schema-3 package operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperatorManifestV3 {
    /// Affine explanatory operator with an authoritative baked stage.
    Affine {
        /// Cumulative baked stage metadata.
        stage: StageManifestV3,
        /// Semantic affine parameters.
        operator: AffineOperator,
    },
    /// Bend explanatory operator with an authoritative baked stage.
    Bend {
        /// Cumulative baked stage metadata.
        stage: StageManifestV3,
        /// Semantic bend parameters.
        parameters: BendParameters,
    },
    /// Terminal lossless correction with an authoritative baked final stage.
    LosslessCorrection {
        /// Cumulative baked stage metadata.
        stage: StageManifestV3,
        /// Lossless correction sidecar metadata.
        correction: LosslessCorrectionManifestV3,
    },
}

/// Terminal lossless correction sidecar metadata for schema 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LosslessCorrectionManifestV3 {
    /// Package-relative u32 residual vertex index list.
    pub residual_index_file: String,
    /// Package-relative f32 absolute residual positions.
    pub residual_position_file: String,
    /// Number of vertices corrected by the residual.
    pub corrected_vertex_count: usize,
}

/// In-memory schema-3 package contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompilePackageV3 {
    /// Package manifest.
    pub manifest: DecompileManifestV3,
    /// Explanatory semantic program before the terminal lossless correction.
    pub semantic_program: OperatorProgram,
    /// Optional replay report generated from package sidecars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_verification: Option<PackageVerificationReportV3>,
}

/// Package replay verification report for schema 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageVerificationReportV3 {
    /// Package schema version that was verified.
    pub schema_version: u32,
    /// Whether ordered source and target topology matched exactly.
    pub topology_exact: bool,
    /// Whether the manifest topology hash matched the sidecar payload.
    pub topology_hash_matches_manifest: bool,
    /// Whether final replayed positions matched target position bits exactly.
    pub positions_bit_exact: bool,
    /// Vertex count.
    pub vertex_count: usize,
    /// Triangle count.
    pub triangle_count: usize,
    /// Number of package operators replayed.
    pub operator_count: usize,
    /// Number of cumulative baked stages replayed.
    pub stage_count: usize,
    /// Number of vertices carried by the terminal lossless correction.
    pub residual_vertex_count: usize,
    /// Maximum absolute per-component replay error.
    pub max_component_error: f64,
    /// Maximum Euclidean replay error.
    pub max_euclidean_error: f64,
    /// Number of vertices outside package verification tolerance.
    pub outside_tolerance: usize,
    /// Whether every semantic-to-baked stage report passed.
    pub semantic_stage_reports_passed: bool,
}

/// Validation failures for schema-3 package contracts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PackageValidationErrorV3 {
    /// The manifest schema version was not 3.
    #[error("unsupported schema version {found}; expected 3")]
    UnsupportedSchema {
        /// Schema version found in the manifest.
        found: u32,
    },
    /// A mesh asset path was empty.
    #[error("mesh asset path must not be empty")]
    EmptyMeshAssetPath,
    /// A package operator ID was empty.
    #[error("operator id must not be empty")]
    EmptyOperatorId,
    /// Two package operators declared the same ID.
    #[error("operator id '{0}' is duplicated")]
    DuplicateOperatorId(String),
    /// A stage index did not match package operator order.
    #[error("stage index {actual} does not match expected index {expected}")]
    StageIndexMismatch {
        /// Expected zero-based package operator index.
        expected: usize,
        /// Actual stage index in the manifest.
        actual: usize,
    },
    /// A cumulative baked positions file was missing.
    #[error("stage {0} must declare a cumulative baked positions file")]
    EmptyBakedPositionsFile(usize),
    /// The package did not end in exactly one terminal lossless correction.
    #[error("schema-3 packages must end with exactly one lossless correction")]
    LosslessCorrectionNotTerminal,
    /// A lossless correction sidecar path was empty.
    #[error("lossless correction sidecar paths must not be empty")]
    EmptyLosslessCorrectionPath,
}

/// Validates schema-3 manifest-level invariants without reading sidecars.
pub fn validate_decompile_manifest_v3(
    manifest: &DecompileManifestV3,
) -> Result<(), PackageValidationErrorV3> {
    if manifest.schema_version != SCHEMA_VERSION_V3 {
        return Err(PackageValidationErrorV3::UnsupportedSchema {
            found: manifest.schema_version,
        });
    }
    if manifest.source.path.is_empty() || manifest.target.path.is_empty() {
        return Err(PackageValidationErrorV3::EmptyMeshAssetPath);
    }

    let mut ids = BTreeSet::new();
    let mut lossless_count = 0_usize;
    for (expected, operator) in manifest.operators.iter().enumerate() {
        let stage = operator.stage();
        if stage.stage_index.0 != expected {
            return Err(PackageValidationErrorV3::StageIndexMismatch {
                expected,
                actual: stage.stage_index.0,
            });
        }
        if stage.operator_id.0.is_empty() {
            return Err(PackageValidationErrorV3::EmptyOperatorId);
        }
        if !ids.insert(stage.operator_id.0.clone()) {
            return Err(PackageValidationErrorV3::DuplicateOperatorId(
                stage.operator_id.0.clone(),
            ));
        }
        if stage.baked_positions_file.is_empty() {
            return Err(PackageValidationErrorV3::EmptyBakedPositionsFile(expected));
        }
        if let OperatorManifestV3::LosslessCorrection { correction, .. } = operator {
            lossless_count += 1;
            if expected + 1 != manifest.operators.len() || lossless_count > 1 {
                return Err(PackageValidationErrorV3::LosslessCorrectionNotTerminal);
            }
            if correction.residual_index_file.is_empty()
                || correction.residual_position_file.is_empty()
            {
                return Err(PackageValidationErrorV3::EmptyLosslessCorrectionPath);
            }
        }
    }
    if lossless_count != 1 {
        return Err(PackageValidationErrorV3::LosslessCorrectionNotTerminal);
    }
    Ok(())
}

impl OperatorManifestV3 {
    fn stage(&self) -> &StageManifestV3 {
        match self {
            OperatorManifestV3::Affine { stage, .. }
            | OperatorManifestV3::Bend { stage, .. }
            | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
        }
    }
}

#[derive(Debug, Clone)]
struct StagePayloadV3 {
    positions_file: String,
    positions: Vec<[f32; 3]>,
}

#[derive(Debug, Clone)]
struct BuiltPackageV3 {
    manifest: DecompileManifestV3,
    stage_payloads: Vec<StagePayloadV3>,
    residual_indices: Vec<u32>,
    residual_positions: Vec<[f32; 3]>,
    inference_diagnostics: InferenceDiagnosticsV4,
}

#[derive(Debug, Copy, Clone)]
struct PositionComparisonMetrics {
    max_component_error: f64,
    max_euclidean_error: f64,
    mean_euclidean_error: f64,
    rms_euclidean_error: f64,
    outside_tolerance: usize,
}

/// Write a schema-3 decompile package directory for an ordered semantic
/// program plus a terminal lossless correction.
///
/// The package is assembled and replay-verified in a sibling staging directory
/// before it atomically replaces the requested output directory.
pub fn write_decompile_package_v3(
    semantic_program: &OperatorProgram,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: impl AsRef<Path>,
) -> Result<PackagePaths, DecompileError> {
    build_v3_package_from_program(semantic_program, source, target, out_dir)
}

/// Build, write, and replay-verify a schema-3 package from an ordered
/// semantic program.
///
/// This is the stable integration point for Wave 3 program search: callers
/// provide only explanatory operators in [`OperatorProgram`]. The package
/// builder appends the terminal lossless correction, writes one cumulative
/// baked stage per package operator, emits diagnostics schema 4, and publishes
/// through the staged package directory flow.
pub fn build_v3_package_from_program(
    semantic_program: &OperatorProgram,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: impl AsRef<Path>,
) -> Result<PackagePaths, DecompileError> {
    build_v3_package_from_program_with_diagnostics(semantic_program, source, target, out_dir, None)
}

/// Build, write, and replay-verify a schema-3 package using externally
/// produced diagnostics for the complete ordered program search.
pub fn build_v3_package_from_program_with_diagnostics(
    semantic_program: &OperatorProgram,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: impl AsRef<Path>,
    inference_diagnostics: Option<InferenceDiagnosticsV4>,
) -> Result<PackagePaths, DecompileError> {
    let out_dir = out_dir.as_ref();
    let mut built = build_decompile_package_v3(semantic_program, source, target)?;
    if let Some(inference_diagnostics) = inference_diagnostics {
        validate_inference_diagnostics_for_program_v3(
            &inference_diagnostics,
            semantic_program,
            source,
            target,
        )?;
        built.inference_diagnostics = inference_diagnostics;
    }

    let staging = StagedPackageDirectory::create(out_dir)?;
    write_decompile_package_v3_contents(&built, source, target, staging.path())?;
    staging.publish(out_dir)?;

    Ok(crate::package_paths(out_dir))
}

/// Read a schema-3 package manifest and package-verification sidecar, then
/// structurally validate its ordered program and declared asset paths.
pub fn read_decompile_package_v3(
    package_dir: impl AsRef<Path>,
) -> Result<DecompilePackageV3, DecompileError> {
    let package_dir = package_dir.as_ref();
    let manifest_path = resolve_package_asset(package_dir, MANIFEST_FILE)?;
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|source| path_io(&manifest_path, source))?;
    let mut manifest: DecompileManifestV3 = serde_json::from_slice(&manifest_bytes)?;
    validate_manifest_contract_v3(&manifest, &manifest_path)?;
    validate_manifest_asset_paths_v3(package_dir, &manifest, &manifest_path)?;

    let package_verification_sidecar = read_optional_package_verification_v3(package_dir)?;
    if let (Some(manifest_report), Some(sidecar_report)) = (
        &manifest.package_verification,
        &package_verification_sidecar,
    ) && manifest_report != sidecar_report
    {
        return Err(invalid_package(
            package_path(package_dir, PACKAGE_VERIFICATION_FILE),
            "package verification sidecar does not match manifest.json",
        ));
    }
    let package_verification = package_verification_sidecar
        .clone()
        .or_else(|| manifest.package_verification.clone());
    if manifest.package_verification.is_none() {
        manifest.package_verification = package_verification.clone();
    }

    let semantic_program = semantic_program_from_manifest_v3(&manifest, &manifest_path)?;

    Ok(DecompilePackageV3 {
        manifest,
        semantic_program,
        package_verification,
    })
}

/// Read a schema-3 package from disk, replay its sidecars, and verify the
/// semantic-to-baked and final lossless reconstruction contracts.
pub fn verify_decompile_package_v3(
    package_dir: impl AsRef<Path>,
) -> Result<PackageVerificationReportV3, DecompileError> {
    let package_dir = package_dir.as_ref();
    let package = read_decompile_package_v3(package_dir)?;
    let manifest = &package.manifest;
    let manifest_path = resolve_package_asset(package_dir, MANIFEST_FILE)?;
    validate_operator_stage_file_count_v3(package_dir, manifest.operators.len(), &manifest_path)?;

    let source_path =
        resolve_required_package_asset_v3(package_dir, &manifest.source.path, &manifest_path)?;
    let target_path =
        resolve_required_package_asset_v3(package_dir, &manifest.target.path, &manifest_path)?;
    let source = read_meshbin(&source_path)?;
    let target = read_meshbin(&target_path)?;
    ensure_mesh_asset_counts_v3(&manifest.source, &source, &source_path)?;
    ensure_mesh_asset_counts_v3(&manifest.target, &target, &target_path)?;

    let topology_exact =
        source.indices == target.indices && source.positions.len() == target.positions.len();
    if !topology_exact {
        return Err(invalid_package(
            package_dir,
            "source and target payload topology is not identical",
        ));
    }
    let topology_hash_matches_manifest =
        topology_hash_from_parts(source.positions.len(), &source.indices) == manifest.topology.hash;
    if manifest.topology.vertex_count != source.positions.len()
        || manifest.topology.index_count != source.indices.len()
        || manifest.topology.triangle_count != source.indices.len() / 3
    {
        return Err(invalid_package(
            &manifest_path,
            "manifest topology counts do not match source.meshbin",
        ));
    }

    let mut current_positions = source.positions.clone();
    let mut residual_vertex_count = 0_usize;
    let mut semantic_stage_reports_passed = true;

    for (operator_index, operator) in manifest.operators.iter().enumerate() {
        let stage = operator.stage();
        let baked_path = resolve_required_package_asset_v3(
            package_dir,
            &stage.baked_positions_file,
            &manifest_path,
        )?;
        let baked = read_positions(&baked_path, source.positions.len())?;

        match operator {
            OperatorManifestV3::Affine {
                stage, operator, ..
            } => {
                let program_operator = ProgramOperator::Affine(*operator);
                let semantic_positions = evaluate_package_operator_v3(
                    &program_operator,
                    &current_positions,
                    &manifest_path,
                )?;
                let report = compare_positions_with_policy_v3(
                    &semantic_positions,
                    &baked,
                    &stage.semantic_verification_policy,
                    &baked_path,
                )?;
                validate_stage_report_v3(stage, report, &baked_path)?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed {
                    return Err(invalid_package(
                        &baked_path,
                        format!(
                            "semantic affine stage {operator_index} does not match its baked positions"
                        ),
                    ));
                }
                current_positions = baked;
            }
            OperatorManifestV3::Bend {
                stage, parameters, ..
            } => {
                let program_operator = ProgramOperator::Bend(*parameters);
                let semantic_positions = evaluate_package_operator_v3(
                    &program_operator,
                    &current_positions,
                    &manifest_path,
                )?;
                let report = compare_positions_with_policy_v3(
                    &semantic_positions,
                    &baked,
                    &stage.semantic_verification_policy,
                    &baked_path,
                )?;
                validate_stage_report_v3(stage, report, &baked_path)?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed {
                    return Err(invalid_package(
                        &baked_path,
                        format!(
                            "semantic bend stage {operator_index} does not match its baked positions"
                        ),
                    ));
                }
                current_positions = baked;
            }
            OperatorManifestV3::LosslessCorrection { stage, correction } => {
                let index_path = resolve_required_package_asset_v3(
                    package_dir,
                    &correction.residual_index_file,
                    &manifest_path,
                )?;
                let position_path = resolve_required_package_asset_v3(
                    package_dir,
                    &correction.residual_position_file,
                    &manifest_path,
                )?;
                let indices = read_u32s(&index_path)?;
                let positions = read_positions(&position_path, indices.len())?;
                if indices.len() != correction.corrected_vertex_count {
                    return Err(invalid_package(
                        &manifest_path,
                        format!(
                            "lossless correction declares {} corrected vertices but stores {}",
                            correction.corrected_vertex_count,
                            indices.len()
                        ),
                    ));
                }
                ensure_strictly_increasing_indices(&indices, current_positions.len(), &index_path)?;
                for (index, position) in indices.iter().zip(&positions) {
                    let index = *index as usize;
                    if !positions_bit_equal(*position, target.positions[index]) {
                        return Err(invalid_package(
                            &position_path,
                            format!(
                                "lossless correction position for vertex {index} is not the absolute target position"
                            ),
                        ));
                    }
                    current_positions[index] = *position;
                }

                let report = compare_positions_with_policy_v3(
                    &current_positions,
                    &baked,
                    &stage.semantic_verification_policy,
                    &baked_path,
                )?;
                validate_stage_report_v3(stage, report, &baked_path)?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed {
                    return Err(invalid_package(
                        &baked_path,
                        "baked lossless positions do not match the replayed correction",
                    ));
                }
                current_positions = baked;
                residual_vertex_count = indices.len();
            }
        }
    }

    let final_metrics = compare_position_metrics_v3(
        &current_positions,
        &target.positions,
        &SemanticVerificationPolicy::default(),
        package_dir,
    )?;
    let positions_bit_exact = position_slices_bit_equal(&current_positions, &target.positions);
    if !positions_bit_exact {
        return Err(invalid_package(
            package_dir,
            format!(
                "serialized operators did not reconstruct target positions exactly; max error={}",
                final_metrics.max_euclidean_error
            ),
        ));
    }

    let report = PackageVerificationReportV3 {
        schema_version: manifest.schema_version,
        topology_exact,
        topology_hash_matches_manifest,
        positions_bit_exact,
        vertex_count: source.positions.len(),
        triangle_count: source.indices.len() / 3,
        operator_count: manifest.operators.len(),
        stage_count: manifest.operators.len(),
        residual_vertex_count,
        max_component_error: final_metrics.max_component_error,
        max_euclidean_error: final_metrics.max_euclidean_error,
        outside_tolerance: final_metrics.outside_tolerance,
        semantic_stage_reports_passed,
    };

    if let Some(stored_report) = &manifest.package_verification
        && stored_report != &report
    {
        return Err(invalid_package(
            &manifest_path,
            "manifest package verification report does not match replayed package data",
        ));
    }

    Ok(report)
}

fn build_decompile_package_v3(
    semantic_program: &OperatorProgram,
    source: &TriangleMesh,
    target: &TriangleMesh,
) -> Result<BuiltPackageV3, DecompileError> {
    validate_decompile_mesh(source, "source")?;
    validate_decompile_mesh(target, "target")?;
    ensure_identical_topology(source, target)?;
    validate_program(semantic_program).map_err(|source| {
        invalid_package(
            Path::new(MANIFEST_FILE),
            format!("schema-3 semantic program is invalid: {source}"),
        )
    })?;

    let mut operators = Vec::with_capacity(semantic_program.operators.len() + 1);
    let mut stage_payloads = Vec::with_capacity(semantic_program.operators.len() + 1);
    let mut current_positions = source.positions.clone();
    let weights = vertex_area_weights(source);
    let raw_identity_error = sum_squared_distance(&source.positions, &target.positions);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source.positions, &target.positions, &weights);
    let error_normalization_scale =
        weighted_centered_sum_squared_distance(&source.positions, &weights)
            .max(weighted_centered_sum_squared_distance(
                &target.positions,
                &weights,
            ))
            .max(f64::EPSILON);
    let literal_size_bytes = source.positions.len().saturating_mul(12).max(1);
    let mut diagnostic_operators = Vec::with_capacity(semantic_program.operators.len());
    let mut diagnostic_stages = Vec::with_capacity(semantic_program.operators.len());

    for (index, operator) in semantic_program.operators.iter().copied().enumerate() {
        let raw_error_before = sum_squared_distance(&current_positions, &target.positions);
        let weighted_error_before =
            weighted_sum_squared_distance(&current_positions, &target.positions, &weights);
        let semantic_positions =
            evaluate_package_operator_v3(&operator, &current_positions, Path::new(MANIFEST_FILE))?;
        let raw_error_after = sum_squared_distance(&semantic_positions, &target.positions);
        let weighted_error_after =
            weighted_sum_squared_distance(&semantic_positions, &target.positions, &weights);
        let (slug, label) = program_operator_stage_identity_v3(&operator);
        let policy = semantic_policy_for_program_operator_v3(&operator);
        let report = compare_positions_with_policy_v3(
            &semantic_positions,
            &semantic_positions,
            &policy,
            Path::new(MANIFEST_FILE),
        )?;
        let stage = StageManifestV3 {
            stage_index: StageIndex(index),
            operator_id: OperatorId(format!("op-{index:04}-{slug}")),
            label: label.to_owned(),
            baked_positions_file: stage_positions_file_v3(index, slug),
            semantic_verification_policy: policy,
            semantic_verification_report: report,
        };
        let diagnostic_operator =
            program_operator_diagnostics_v3(&operator, Path::new(MANIFEST_FILE))?;
        let diagnostic_stage = build_stage_diagnostics(StageDiagnosticsInput {
            stage_index: index,
            operator: diagnostic_operator.clone(),
            weighted_error_before,
            weighted_error_after,
            raw_error_before,
            raw_error_after,
            semantic_verification_policy: policy,
            semantic_verification_report: report,
        })
        .map_err(|source| {
            invalid_package(
                Path::new(INFERENCE_DIAGNOSTICS_FILE),
                format!("schema-4 stage diagnostics are invalid: {source}"),
            )
        })?;
        diagnostic_operators.push(diagnostic_operator);
        diagnostic_stages.push(diagnostic_stage);
        operators.push(match operator {
            ProgramOperator::Affine(operator) => OperatorManifestV3::Affine { stage, operator },
            ProgramOperator::Bend(parameters) => OperatorManifestV3::Bend { stage, parameters },
        });
        stage_payloads.push(StagePayloadV3 {
            positions_file: stage_positions_file_v3(index, slug),
            positions: semantic_positions.clone(),
        });
        current_positions = semantic_positions;
    }

    let mut residual_indices = Vec::new();
    let mut residual_positions = Vec::new();
    let mut final_positions = current_positions.clone();
    for (index, (current, target_position)) in
        current_positions.iter().zip(&target.positions).enumerate()
    {
        if !positions_bit_equal(*current, *target_position) {
            residual_indices.push(u32::try_from(index).map_err(|_| {
                DecompileError::InvalidMesh {
                    mesh_name: "source",
                    message: "vertex count exceeds the u32 residual index contract".to_owned(),
                }
            })?);
            residual_positions.push(*target_position);
            final_positions[index] = *target_position;
        }
    }

    let lossless_index = operators.len();
    let lossless_policy = SemanticVerificationPolicy::default();
    let lossless_report = compare_positions_with_policy_v3(
        &final_positions,
        &final_positions,
        &lossless_policy,
        Path::new(MANIFEST_FILE),
    )?;
    let lossless_stage_file = stage_positions_file_v3(lossless_index, LOSSLESS_STAGE_SLUG);
    operators.push(OperatorManifestV3::LosslessCorrection {
        stage: StageManifestV3 {
            stage_index: StageIndex(lossless_index),
            operator_id: OperatorId(format!("op-{lossless_index:04}-{LOSSLESS_STAGE_SLUG}")),
            label: "Lossless correction".to_owned(),
            baked_positions_file: lossless_stage_file.clone(),
            semantic_verification_policy: lossless_policy,
            semantic_verification_report: lossless_report,
        },
        correction: LosslessCorrectionManifestV3 {
            residual_index_file: RESIDUAL_INDEX_FILE.to_owned(),
            residual_position_file: RESIDUAL_POSITION_FILE.to_owned(),
            corrected_vertex_count: residual_indices.len(),
        },
    });
    stage_payloads.push(StagePayloadV3 {
        positions_file: lossless_stage_file,
        positions: final_positions,
    });

    let scoring_policy = default_scoring_policy_v4();
    let weighted_error_before_correction =
        weighted_sum_squared_distance(&current_positions, &target.positions, &weights);
    let raw_error_before_correction = sum_squared_distance(&current_positions, &target.positions);
    let exact_residual_bytes =
        exact_residual_storage_size_v3(&current_positions, &target.positions);
    let program_hypothesis = build_program_diagnostics(ProgramDiagnosticsInput {
        operators: diagnostic_operators,
        stages: diagnostic_stages,
        final_correction: ProgramCorrectionDiagnostics {
            corrected_vertex_count: residual_indices.len(),
            exact_residual_bytes,
            weighted_error_before: weighted_error_before_correction,
            weighted_error_after: 0.0,
            raw_error_before: raw_error_before_correction,
            raw_error_after: 0.0,
        },
        raw_identity_error,
        weighted_identity_error,
        error_normalization_scale,
        literal_size_bytes,
        approximate_residual_coverage: approximate_residual_cost(
            &current_positions,
            &target.positions,
            &weights,
        ),
        scoring_policy: scoring_policy.clone(),
        selected: true,
        rejection_reason: None,
    })
    .map_err(|source| {
        invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            format!("schema-4 program diagnostics are invalid: {source}"),
        )
    })?;
    let inference_diagnostics = InferenceDiagnosticsV4 {
        diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION_V4,
        package_schema_version: SCHEMA_VERSION_V3,
        surface_weighting: "triangle_area_derived_vertex_weights".to_owned(),
        raw_identity_error,
        weighted_identity_error,
        scoring_policy,
        selected_program_hypothesis_index: 0,
        program_hypotheses: vec![program_hypothesis],
        timing_by_phase_ms: default_timing_by_phase_v4(),
    };

    let manifest = DecompileManifestV3 {
        schema_version: SCHEMA_VERSION_V3,
        coordinate_system: COORDINATE_SYSTEM_V3.to_owned(),
        numeric_format: NumericFormatV3::default(),
        source: MeshAssetV3 {
            path: SOURCE_MESHBIN.to_owned(),
            vertex_count: source.positions.len(),
            triangle_count: source.indices.len() / 3,
        },
        target: MeshAssetV3 {
            path: TARGET_MESHBIN.to_owned(),
            vertex_count: target.positions.len(),
            triangle_count: target.indices.len() / 3,
        },
        topology: TopologySummaryV3 {
            vertex_count: source.positions.len(),
            triangle_count: source.indices.len() / 3,
            index_count: source.indices.len(),
            hash: topology_hash(source),
        },
        operators,
        package_verification: None,
    };

    validate_manifest_contract_v3(&manifest, Path::new(MANIFEST_FILE))?;
    ensure_strictly_increasing_indices(
        &residual_indices,
        source.positions.len(),
        Path::new(RESIDUAL_INDEX_FILE),
    )?;

    Ok(BuiltPackageV3 {
        manifest,
        stage_payloads,
        residual_indices,
        residual_positions,
        inference_diagnostics,
    })
}

fn validate_inference_diagnostics_for_program_v3(
    diagnostics: &InferenceDiagnosticsV4,
    semantic_program: &OperatorProgram,
    source: &TriangleMesh,
    target: &TriangleMesh,
) -> Result<(), DecompileError> {
    if diagnostics.diagnostics_schema_version != DIAGNOSTICS_SCHEMA_VERSION_V4 {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "inference diagnostics schema version must be 4",
        ));
    }
    if diagnostics.package_schema_version != SCHEMA_VERSION_V3 {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "schema-4 diagnostics must describe a schema-3 package",
        ));
    }
    if diagnostics.selected_program_hypothesis_index >= diagnostics.program_hypotheses.len() {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "selected program index is outside the diagnostics hypothesis list",
        ));
    }
    let selected = &diagnostics.program_hypotheses[diagnostics.selected_program_hypothesis_index];
    if !selected.selected {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "selected program hypothesis is not marked selected",
        ));
    }
    if diagnostics
        .program_hypotheses
        .iter()
        .enumerate()
        .any(|(index, hypothesis)| {
            hypothesis.selected != (index == diagnostics.selected_program_hypothesis_index)
        })
    {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "exactly one diagnostics hypothesis must be marked selected",
        ));
    }

    let selected_operators = semantic_program
        .operators
        .iter()
        .map(|operator| program_operator_diagnostics_v3(operator, Path::new(MANIFEST_FILE)))
        .collect::<Result<Vec<_>, _>>()?;
    if selected.operators != selected_operators {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "selected diagnostics operators do not match the baked schema-3 program",
        ));
    }

    let weights = vertex_area_weights(source);
    let raw_identity_error = sum_squared_distance(&source.positions, &target.positions);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source.positions, &target.positions, &weights);
    if !diagnostic_f64_matches(diagnostics.raw_identity_error, raw_identity_error)
        || !diagnostic_f64_matches(diagnostics.weighted_identity_error, weighted_identity_error)
    {
        return Err(invalid_package(
            Path::new(INFERENCE_DIAGNOSTICS_FILE),
            "diagnostics identity errors do not match the mesh pair",
        ));
    }

    Ok(())
}

fn diagnostic_f64_matches(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= 1.0e-9
}

fn program_operator_diagnostics_v3(
    operator: &ProgramOperator,
    path: &Path,
) -> Result<ProgramOperatorDiagnostics, DecompileError> {
    match operator {
        ProgramOperator::Affine(affine) => match affine.semantic_family {
            AffineSemanticFamily::Translation => Ok(ProgramOperatorDiagnostics::Translation {
                translation: affine.translation.ok_or_else(|| {
                    invalid_package(
                        path,
                        "translation operator is missing translation parameters",
                    )
                })?,
            }),
            AffineSemanticFamily::RigidTransform => {
                Ok(ProgramOperatorDiagnostics::RigidTransform {
                    translation: affine.translation.ok_or_else(|| {
                        invalid_package(path, "rigid transform is missing translation parameters")
                    })?,
                    rotation_row_major_3x3: affine.rotation_row_major_3x3.ok_or_else(|| {
                        invalid_package(path, "rigid transform is missing rotation parameters")
                    })?,
                })
            }
            AffineSemanticFamily::SimilarityTransform => {
                Ok(ProgramOperatorDiagnostics::SimilarityTransform {
                    translation: affine.translation.ok_or_else(|| {
                        invalid_package(
                            path,
                            "similarity transform is missing translation parameters",
                        )
                    })?,
                    rotation_row_major_3x3: affine.rotation_row_major_3x3.ok_or_else(|| {
                        invalid_package(path, "similarity transform is missing rotation parameters")
                    })?,
                    uniform_scale: affine.uniform_scale.ok_or_else(|| {
                        invalid_package(path, "similarity transform is missing uniform scale")
                    })?,
                })
            }
            AffineSemanticFamily::GeneralAffine => Ok(ProgramOperatorDiagnostics::GeneralAffine {
                matrix_row_major_4x4: affine.matrix_row_major_4x4,
            }),
        },
        ProgramOperator::Bend(parameters) => Ok(ProgramOperatorDiagnostics::Bend {
            parameters: *parameters,
        }),
    }
}

fn exact_residual_storage_size_v3(reconstructed: &[[f32; 3]], target: &[[f32; 3]]) -> usize {
    reconstructed
        .iter()
        .zip(target)
        .filter(|(left, right)| !positions_bit_equal(**left, **right))
        .count()
        * (std::mem::size_of::<u32>() + 3 * std::mem::size_of::<f32>())
}

fn write_decompile_package_v3_contents(
    built: &BuiltPackageV3,
    source: &TriangleMesh,
    target: &TriangleMesh,
    out_dir: &Path,
) -> Result<(), DecompileError> {
    fs::create_dir_all(out_dir).map_err(|source| path_io(out_dir, source))?;
    fs::create_dir_all(out_dir.join(OPERATORS_DIR))
        .map_err(|source| path_io(&out_dir.join(OPERATORS_DIR), source))?;
    fs::create_dir_all(out_dir.join(RESIDUAL_DIR))
        .map_err(|source| path_io(&out_dir.join(RESIDUAL_DIR), source))?;

    write_meshbin(&package_path(out_dir, SOURCE_MESHBIN), source)?;
    write_meshbin(&package_path(out_dir, TARGET_MESHBIN), target)?;
    for stage in &built.stage_payloads {
        write_positions(
            &package_path(out_dir, &stage.positions_file),
            &stage.positions,
        )?;
    }
    write_u32s(
        &package_path(out_dir, RESIDUAL_INDEX_FILE),
        &built.residual_indices,
    )?;
    write_positions(
        &package_path(out_dir, RESIDUAL_POSITION_FILE),
        &built.residual_positions,
    )?;
    write_json(
        &package_path(out_dir, INFERENCE_DIAGNOSTICS_FILE),
        &built.inference_diagnostics,
    )?;
    write_text(
        &package_path(out_dir, BLENDER_SCRIPT_FILE),
        &blender_reconstruction_script_v3(&BlenderAdapterOptions::default()),
    )?;

    let mut manifest = built.manifest.clone();
    write_json(&package_path(out_dir, MANIFEST_FILE), &manifest)?;
    let package_verification = verify_decompile_package_v3(out_dir)?;
    manifest.package_verification = Some(package_verification.clone());
    write_json(&package_path(out_dir, MANIFEST_FILE), &manifest)?;
    write_json(
        &package_path(out_dir, PACKAGE_VERIFICATION_FILE),
        &package_verification,
    )?;
    Ok(())
}

fn validate_manifest_contract_v3(
    manifest: &DecompileManifestV3,
    path: &Path,
) -> Result<(), DecompileError> {
    validate_decompile_manifest_v3(manifest)
        .map_err(|source| invalid_package(path, source.to_string()))?;
    if manifest.coordinate_system != COORDINATE_SYSTEM_V3 {
        return Err(invalid_package(
            path,
            format!(
                "unsupported schema-3 coordinate system '{}'",
                manifest.coordinate_system
            ),
        ));
    }
    if manifest.numeric_format != NumericFormatV3::default() {
        return Err(invalid_package(path, "unsupported schema-3 numeric format"));
    }
    if manifest.source.vertex_count != manifest.topology.vertex_count
        || manifest.target.vertex_count != manifest.topology.vertex_count
        || manifest.source.triangle_count != manifest.topology.triangle_count
        || manifest.target.triangle_count != manifest.topology.triangle_count
        || manifest.topology.index_count != manifest.topology.triangle_count * 3
    {
        return Err(invalid_package(
            path,
            "manifest mesh asset counts do not match topology summary",
        ));
    }
    for (index, operator) in manifest.operators.iter().enumerate() {
        let stage = operator.stage();
        if stage.label.trim().is_empty() {
            return Err(invalid_package(path, "stage labels must not be empty"));
        }
        validate_stage_positions_file_v3(index, &stage.baked_positions_file, path)?;
        validate_stage_policy_for_operator_v3(operator, path)?;
    }
    let semantic_program = semantic_program_from_manifest_v3(manifest, path)?;
    validate_program(&semantic_program).map_err(|source| {
        invalid_package(path, format!("schema-3 program is invalid: {source}"))
    })?;
    Ok(())
}

fn validate_manifest_asset_paths_v3(
    package_dir: &Path,
    manifest: &DecompileManifestV3,
    manifest_path: &Path,
) -> Result<(), DecompileError> {
    resolve_required_package_asset_v3(package_dir, &manifest.source.path, manifest_path)?;
    resolve_required_package_asset_v3(package_dir, &manifest.target.path, manifest_path)?;
    for operator in &manifest.operators {
        let stage = operator.stage();
        resolve_required_package_asset_v3(package_dir, &stage.baked_positions_file, manifest_path)?;
        if let OperatorManifestV3::LosslessCorrection { correction, .. } = operator {
            resolve_required_package_asset_v3(
                package_dir,
                &correction.residual_index_file,
                manifest_path,
            )?;
            resolve_required_package_asset_v3(
                package_dir,
                &correction.residual_position_file,
                manifest_path,
            )?;
        }
    }
    Ok(())
}

fn read_optional_package_verification_v3(
    package_dir: &Path,
) -> Result<Option<PackageVerificationReportV3>, DecompileError> {
    let path = package_path(package_dir, PACKAGE_VERIFICATION_FILE);
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            let path = resolve_package_asset(package_dir, PACKAGE_VERIFICATION_FILE)?;
            let bytes = fs::read(&path).map_err(|source| path_io(&path, source))?;
            Ok(Some(serde_json::from_slice(&bytes)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(path_io(&path, error)),
    }
}

fn semantic_program_from_manifest_v3(
    manifest: &DecompileManifestV3,
    path: &Path,
) -> Result<OperatorProgram, DecompileError> {
    let mut operators = Vec::new();
    for operator in &manifest.operators {
        match operator {
            OperatorManifestV3::Affine { operator, .. } => {
                operators.push(ProgramOperator::Affine(*operator));
            }
            OperatorManifestV3::Bend { parameters, .. } => {
                operators.push(ProgramOperator::Bend(*parameters));
            }
            OperatorManifestV3::LosslessCorrection { .. } => {}
        }
    }
    let program = OperatorProgram { operators };
    validate_program(&program).map_err(|source| {
        invalid_package(path, format!("schema-3 program is invalid: {source}"))
    })?;
    Ok(program)
}

fn evaluate_package_operator_v3(
    operator: &ProgramOperator,
    positions: &[[f32; 3]],
    path: impl AsRef<Path>,
) -> Result<Vec<[f32; 3]>, DecompileError> {
    evaluate_operator(operator, positions).map_err(|source| {
        invalid_package(
            path,
            format!("schema-3 operator semantic evaluation failed: {source}"),
        )
    })
}

fn validate_stage_policy_for_operator_v3(
    operator: &OperatorManifestV3,
    path: &Path,
) -> Result<(), DecompileError> {
    let stage = operator.stage();
    validate_semantic_verification_policy_v3(&stage.semantic_verification_policy, path)?;
    let expected_mode = match operator {
        OperatorManifestV3::Affine { .. } | OperatorManifestV3::LosslessCorrection { .. } => {
            SemanticVerificationMode::BitExact
        }
        OperatorManifestV3::Bend { .. } => SemanticVerificationMode::Tolerance,
    };
    if stage.semantic_verification_policy.mode != expected_mode {
        return Err(invalid_package(
            path,
            format!(
                "stage {} uses {:?} semantic verification but {:?} is required",
                stage.stage_index.0, stage.semantic_verification_policy.mode, expected_mode
            ),
        ));
    }
    Ok(())
}

fn validate_semantic_verification_policy_v3(
    policy: &SemanticVerificationPolicy,
    path: &Path,
) -> Result<(), DecompileError> {
    if !policy.absolute_epsilon.is_finite()
        || !policy.relative_epsilon.is_finite()
        || !policy.ulp_multiplier.is_finite()
        || policy.absolute_epsilon < 0.0
        || policy.relative_epsilon < 0.0
        || policy.ulp_multiplier < 0.0
    {
        return Err(invalid_package(
            path,
            "semantic verification tolerances must be finite and non-negative",
        ));
    }
    if policy.mode == SemanticVerificationMode::BitExact
        && (policy.absolute_epsilon != 0.0
            || policy.relative_epsilon != 0.0
            || policy.ulp_multiplier != 0.0)
    {
        return Err(invalid_package(
            path,
            "bit-exact semantic verification must use zero tolerances",
        ));
    }
    Ok(())
}

fn compare_positions_with_policy_v3(
    left: &[[f32; 3]],
    right: &[[f32; 3]],
    policy: &SemanticVerificationPolicy,
    path: &Path,
) -> Result<SemanticVerificationReport, DecompileError> {
    let metrics = compare_position_metrics_v3(left, right, policy, path)?;
    Ok(SemanticVerificationReport {
        max_component_error: metrics.max_component_error,
        max_euclidean_error: metrics.max_euclidean_error,
        mean_euclidean_error: metrics.mean_euclidean_error,
        rms_euclidean_error: metrics.rms_euclidean_error,
        outside_tolerance: metrics.outside_tolerance,
        passed: metrics.outside_tolerance == 0,
    })
}

fn compare_position_metrics_v3(
    left: &[[f32; 3]],
    right: &[[f32; 3]],
    policy: &SemanticVerificationPolicy,
    path: impl AsRef<Path>,
) -> Result<PositionComparisonMetrics, DecompileError> {
    let path = path.as_ref();
    validate_semantic_verification_policy_v3(policy, path)?;
    if left.len() != right.len() {
        return Err(invalid_package(
            path,
            format!(
                "position counts differ: left={} right={}",
                left.len(),
                right.len()
            ),
        ));
    }

    let mut max_component_error = 0.0_f64;
    let mut max_euclidean_error = 0.0_f64;
    let mut total_euclidean_error = 0.0_f64;
    let mut total_squared_euclidean_error = 0.0_f64;
    let mut outside_tolerance = 0_usize;

    for (left_position, right_position) in left.iter().zip(right) {
        let mut outside_vertex = false;
        let mut squared_euclidean = 0.0_f64;
        for component in 0..3 {
            let left_component = left_position[component];
            let right_component = right_position[component];
            let component_error = (f64::from(left_component) - f64::from(right_component)).abs();
            max_component_error = max_component_error.max(component_error);
            squared_euclidean += component_error * component_error;
            if component_outside_policy_v3(left_component, right_component, policy) {
                outside_vertex = true;
            }
        }
        let euclidean = squared_euclidean.sqrt();
        max_euclidean_error = max_euclidean_error.max(euclidean);
        total_euclidean_error += euclidean;
        total_squared_euclidean_error += squared_euclidean;
        if outside_vertex {
            outside_tolerance += 1;
        }
    }

    let count = left.len().max(1) as f64;
    Ok(PositionComparisonMetrics {
        max_component_error,
        max_euclidean_error,
        mean_euclidean_error: total_euclidean_error / count,
        rms_euclidean_error: (total_squared_euclidean_error / count).sqrt(),
        outside_tolerance,
    })
}

fn component_outside_policy_v3(left: f32, right: f32, policy: &SemanticVerificationPolicy) -> bool {
    match policy.mode {
        SemanticVerificationMode::BitExact => left.to_bits() != right.to_bits(),
        SemanticVerificationMode::Tolerance => {
            let error = (f64::from(left) - f64::from(right)).abs();
            let magnitude = f64::from(left.abs().max(right.abs()));
            let tolerance = policy
                .absolute_epsilon
                .max(policy.relative_epsilon * magnitude)
                .max(policy.ulp_multiplier * f32_ulp_spacing_v3(right));
            error > tolerance
        }
    }
}

fn f32_ulp_spacing_v3(value: f32) -> f64 {
    let value = value.abs();
    if value == 0.0 {
        return f64::from(f32::from_bits(1));
    }
    let bits = value.to_bits();
    let next = f32::from_bits(bits.saturating_add(1));
    if next.is_finite() {
        f64::from(next - value)
    } else {
        f64::from(value - f32::from_bits(bits.saturating_sub(1)))
    }
}

fn validate_stage_report_v3(
    stage: &StageManifestV3,
    report: SemanticVerificationReport,
    path: &Path,
) -> Result<(), DecompileError> {
    if stage.semantic_verification_report != report {
        return Err(invalid_package(
            path,
            format!(
                "stage {} semantic verification report does not match replay",
                stage.stage_index.0
            ),
        ));
    }
    Ok(())
}

fn ensure_mesh_asset_counts_v3(
    asset: &MeshAssetV3,
    payload: &crate::MeshPayload,
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

fn resolve_required_package_asset_v3(
    package_dir: &Path,
    relative: &str,
    owner: impl AsRef<Path>,
) -> Result<PathBuf, DecompileError> {
    match resolve_package_asset(package_dir, relative) {
        Ok(path) => Ok(path),
        Err(DecompileError::PathIo { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Err(invalid_package(
                owner,
                format!("declared package asset '{relative}' is missing"),
            ))
        }
        Err(error) => Err(error),
    }
}

fn validate_operator_stage_file_count_v3(
    package_dir: &Path,
    expected: usize,
    manifest_path: &Path,
) -> Result<(), DecompileError> {
    let operators_dir = package_dir.join(OPERATORS_DIR);
    let metadata = fs::symlink_metadata(&operators_dir).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            invalid_package(manifest_path, "operators directory is missing")
        } else {
            path_io(&operators_dir, source)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(invalid_package(
            &operators_dir,
            "operators path must be a regular package directory",
        ));
    }

    let mut count = 0_usize;
    for entry in fs::read_dir(&operators_dir).map_err(|source| path_io(&operators_dir, source))? {
        let entry = entry.map_err(|source| path_io(&operators_dir, source))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| path_io(&path, source))?;
        if metadata.file_type().is_symlink() {
            return Err(invalid_package(
                &path,
                "operator stage files must not be symlinks",
            ));
        }
        if metadata.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-positions.f32"))
        {
            count += 1;
        }
    }
    if count != expected {
        return Err(invalid_package(
            manifest_path,
            format!("manifest declares {expected} operators but stores {count} baked stages"),
        ));
    }
    Ok(())
}

fn validate_stage_positions_file_v3(
    index: usize,
    path: &str,
    manifest_path: &Path,
) -> Result<(), DecompileError> {
    let prefix = format!("{OPERATORS_DIR}/{index:04}-");
    let suffix = "-positions.f32";
    if !path.starts_with(&prefix) || !path.ends_with(suffix) {
        return Err(invalid_package(
            manifest_path,
            format!("stage {index} path '{path}' does not follow schema-3 stage naming"),
        ));
    }
    let slug = &path[prefix.len()..path.len() - suffix.len()];
    if !is_stable_slug_v3(slug) {
        return Err(invalid_package(
            manifest_path,
            format!("stage {index} path slug '{slug}' is not stable"),
        ));
    }
    Ok(())
}

fn is_stable_slug_v3(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn stage_positions_file_v3(index: usize, slug: &str) -> String {
    format!("{OPERATORS_DIR}/{index:04}-{slug}-positions.f32")
}

fn program_operator_stage_identity_v3(operator: &ProgramOperator) -> (&'static str, &'static str) {
    match operator {
        ProgramOperator::Affine(_) => (AFFINE_STAGE_SLUG, "Affine"),
        ProgramOperator::Bend(_) => (BEND_STAGE_SLUG, "Bend"),
    }
}

fn semantic_policy_for_program_operator_v3(
    operator: &ProgramOperator,
) -> SemanticVerificationPolicy {
    match operator {
        ProgramOperator::Affine(_) => SemanticVerificationPolicy::default(),
        ProgramOperator::Bend(_) => SemanticVerificationPolicy {
            mode: SemanticVerificationMode::Tolerance,
            absolute_epsilon: 0.0,
            relative_epsilon: 0.0,
            ulp_multiplier: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AffineSemanticFamily;

    #[test]
    fn schema_three_lossless_only_manifest_roundtrips_and_validates() {
        let manifest = lossless_only_manifest("operators/0000-lossless.f32");

        let json = serde_json::to_string(&manifest).unwrap();
        let decoded: DecompileManifestV3 = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, manifest);
        validate_decompile_manifest_v3(&decoded).unwrap();
    }

    #[test]
    fn schema_three_stage_requires_baked_positions_file() {
        let manifest = lossless_only_manifest("");

        let error = validate_decompile_manifest_v3(&manifest).unwrap_err();

        assert_eq!(error, PackageValidationErrorV3::EmptyBakedPositionsFile(0));
    }

    #[test]
    fn lossless_correction_must_be_terminal() {
        let mut manifest = lossless_only_manifest("operators/0000-lossless.f32");
        manifest.operators.push(OperatorManifestV3::Affine {
            stage: StageManifestV3 {
                stage_index: StageIndex(1),
                operator_id: OperatorId("op-0001-affine".to_owned()),
                label: "Affine".to_owned(),
                baked_positions_file: "operators/0001-affine.f32".to_owned(),
                semantic_verification_policy: SemanticVerificationPolicy::default(),
                semantic_verification_report: SemanticVerificationReport::default(),
            },
            operator: AffineOperator {
                semantic_family: AffineSemanticFamily::GeneralAffine,
                matrix_row_major_4x4: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                translation: None,
                rotation_row_major_3x3: None,
                uniform_scale: None,
            },
        });

        let error = validate_decompile_manifest_v3(&manifest).unwrap_err();

        assert_eq!(
            error,
            PackageValidationErrorV3::LosslessCorrectionNotTerminal
        );
    }

    fn lossless_only_manifest(stage_file: &str) -> DecompileManifestV3 {
        DecompileManifestV3 {
            schema_version: SCHEMA_VERSION_V3,
            coordinate_system: "right-handed-y-up".to_owned(),
            numeric_format: NumericFormatV3::default(),
            source: MeshAssetV3 {
                path: "source.meshbin".to_owned(),
                vertex_count: 1,
                triangle_count: 0,
            },
            target: MeshAssetV3 {
                path: "target.meshbin".to_owned(),
                vertex_count: 1,
                triangle_count: 0,
            },
            topology: TopologySummaryV3 {
                vertex_count: 1,
                triangle_count: 0,
                index_count: 0,
                hash: "fnv:0".to_owned(),
            },
            operators: vec![OperatorManifestV3::LosslessCorrection {
                stage: StageManifestV3 {
                    stage_index: StageIndex(0),
                    operator_id: OperatorId("op-0000-lossless".to_owned()),
                    label: "Lossless correction".to_owned(),
                    baked_positions_file: stage_file.to_owned(),
                    semantic_verification_policy: SemanticVerificationPolicy::default(),
                    semantic_verification_report: SemanticVerificationReport::default(),
                },
                correction: LosslessCorrectionManifestV3 {
                    residual_index_file: "residual/indices.u32".to_owned(),
                    residual_position_file: "residual/positions.f32".to_owned(),
                    corrected_vertex_count: 0,
                },
            }],
            package_verification: None,
        }
    }
}
