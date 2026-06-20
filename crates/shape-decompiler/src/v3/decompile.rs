//! In-memory schema-3 package construction from an explicit operator program.
//!
//! This builder accepts an already selected explanatory program. It performs no
//! inference: each operator is evaluated in order, cumulative baked stages are
//! produced, and the terminal lossless correction makes the final positions
//! bit-exact with the target.

use std::path::Path;

use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;

use super::bend::{
    BendStageVerificationPolicy, BendStageVerificationReport, compare_bend_to_baked_stage,
};
use super::diagnostics::{
    DIAGNOSTICS_SCHEMA_VERSION_V4, InferenceDiagnosticsV4, ProgramCorrectionDiagnostics,
    ProgramDiagnosticsInput, ProgramOperatorDiagnostics, StageDiagnosticsInput,
    build_program_diagnostics, build_stage_diagnostics, default_scoring_policy_v4,
    default_timing_by_phase_v4,
};
use super::package::{
    DecompileManifestV3, LosslessCorrectionManifestV3, MeshAssetV3, NumericFormatV3,
    OperatorManifestV3, PackageVerificationReportV3, SCHEMA_VERSION_V3, StageManifestV3,
    TopologySummaryV3, validate_decompile_manifest_v3,
};
use super::program::{
    AffineOperator, OperatorId, OperatorProgram, ProgramOperator, SemanticVerificationMode,
    SemanticVerificationPolicy, SemanticVerificationReport, StageIndex, evaluate_operator,
    validate_program,
};
use crate::{
    AffineSemanticFamily, DecompileError, DecompileSettings, INFERENCE_DIAGNOSTICS_FILE,
    MANIFEST_FILE, RESIDUAL_INDEX_FILE, RESIDUAL_POSITION_FILE, SOURCE_MESHBIN, TARGET_MESHBIN,
    approximate_residual_cost, ensure_identical_topology, ensure_strictly_increasing_indices,
    invalid_package, position_slices_bit_equal, positions_bit_equal, sum_squared_distance,
    topology_hash, validate_decompile_mesh, validate_settings, vertex_area_weights,
    weighted_centered_sum_squared_distance, weighted_sum_squared_distance,
};

const COORDINATE_SYSTEM_V3: &str = "right-handed-y-up";
const OPERATORS_DIR: &str = "operators";
const LOSSLESS_STAGE_SLUG: &str = "lossless-correction";

/// Cumulative baked positions for one schema-3 package operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagePayloadV3 {
    /// Stable operator identifier this payload belongs to.
    pub operator_id: OperatorId,
    /// Human-facing stage label this payload belongs to.
    pub label: String,
    /// Package-relative cumulative baked positions file.
    pub positions_file: String,
    /// Cumulative baked positions after this operator.
    pub positions: Vec<[f32; 3]>,
}

/// In-memory schema-3 package with all sidecar payloads retained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompilePackageV3 {
    /// Package manifest.
    pub manifest: DecompileManifestV3,
    /// Explanatory semantic program supplied by the caller.
    pub semantic_program: OperatorProgram,
    /// Replay report for the in-memory package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_verification: Option<PackageVerificationReportV3>,
    /// Ordered cumulative baked positions for every manifest operator.
    pub stage_payloads: Vec<StagePayloadV3>,
    /// Strictly increasing vertex indices corrected by the terminal residual.
    pub residual_indices: Vec<u32>,
    /// Absolute target positions for each residual index.
    pub residual_positions: Vec<[f32; 3]>,
    /// Final cumulative positions after every operator and correction.
    pub final_positions: Vec<[f32; 3]>,
    /// Diagnostics schema 4 for the explicit selected program.
    pub inference_diagnostics: InferenceDiagnosticsV4,
}

impl DecompilePackageV3 {
    /// Validates this in-memory package against the mesh pair it was built for.
    pub fn validate_against_meshes(
        &self,
        source_mesh: &TriangleMesh,
        target_mesh: &TriangleMesh,
    ) -> Result<(), DecompileError> {
        validate_decompile_package_v3(self, source_mesh, target_mesh)
    }
}

/// Builds a lossless schema-3 package from an explicit selected program.
///
/// `selected_program` is trusted only as a candidate to validate and evaluate;
/// this function does not fit, search, infer, reorder, or simplify operators.
pub fn build_v3_package_from_program(
    source_mesh: &TriangleMesh,
    target_mesh: &TriangleMesh,
    selected_program: &OperatorProgram,
    settings: DecompileSettings,
) -> Result<DecompilePackageV3, DecompileError> {
    validate_settings(settings)?;
    validate_decompile_mesh(source_mesh, "source")?;
    validate_decompile_mesh(target_mesh, "target")?;
    ensure_identical_topology(source_mesh, target_mesh)?;
    validate_program(selected_program).map_err(|source| {
        invalid_package(
            Path::new(MANIFEST_FILE),
            format!("schema-3 selected program is invalid: {source}"),
        )
    })?;

    let weights = vertex_area_weights(source_mesh);
    let raw_identity_error = sum_squared_distance(&source_mesh.positions, &target_mesh.positions);
    let weighted_identity_error =
        weighted_sum_squared_distance(&source_mesh.positions, &target_mesh.positions, &weights);
    let error_normalization_scale =
        weighted_centered_sum_squared_distance(&source_mesh.positions, &weights)
            .max(weighted_centered_sum_squared_distance(
                &target_mesh.positions,
                &weights,
            ))
            .max(f64::EPSILON);
    let literal_size_bytes = source_mesh.positions.len().saturating_mul(12).max(1);

    let mut manifest_operators = Vec::with_capacity(selected_program.operators.len() + 1);
    let mut stage_payloads = Vec::with_capacity(selected_program.operators.len() + 1);
    let mut diagnostic_operators = Vec::with_capacity(selected_program.operators.len());
    let mut diagnostic_stages = Vec::with_capacity(selected_program.operators.len());
    let mut current_positions = source_mesh.positions.clone();

    for (index, operator) in selected_program.operators.iter().copied().enumerate() {
        let previous_baked_positions = current_positions.clone();
        let raw_error_before =
            sum_squared_distance(&previous_baked_positions, &target_mesh.positions);
        let weighted_error_before = weighted_sum_squared_distance(
            &previous_baked_positions,
            &target_mesh.positions,
            &weights,
        );
        let semantic_positions = evaluate_package_operator(
            &operator,
            &previous_baked_positions,
            Path::new(MANIFEST_FILE),
        )?;
        let baked_positions = semantic_positions.clone();
        let raw_error_after = sum_squared_distance(&baked_positions, &target_mesh.positions);
        let weighted_error_after =
            weighted_sum_squared_distance(&baked_positions, &target_mesh.positions, &weights);
        let (slug, label) = program_operator_stage_identity(operator);
        let policy = semantic_policy_for_program_operator(operator);
        let report = semantic_stage_report(
            &operator,
            &previous_baked_positions,
            &semantic_positions,
            &baked_positions,
            policy,
            Path::new(MANIFEST_FILE),
        )?;
        if !report.passed {
            return Err(invalid_package(
                Path::new(MANIFEST_FILE),
                format!("semantic verification failed for stage {index}"),
            ));
        }

        let positions_file = stage_positions_file(index, slug);
        let stage = StageManifestV3 {
            stage_index: StageIndex(index),
            operator_id: OperatorId(format!("op-{index:04}-{slug}")),
            label: label.to_owned(),
            baked_positions_file: positions_file.clone(),
            semantic_verification_policy: policy,
            semantic_verification_report: report,
        };
        let diagnostic_operator = program_operator_diagnostics(operator, Path::new(MANIFEST_FILE))?;
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
        manifest_operators.push(match operator {
            ProgramOperator::Affine(operator) => OperatorManifestV3::Affine { stage, operator },
            ProgramOperator::Bend(parameters) => OperatorManifestV3::Bend { stage, parameters },
        });
        stage_payloads.push(StagePayloadV3 {
            operator_id: OperatorId(format!("op-{index:04}-{slug}")),
            label: label.to_owned(),
            positions_file,
            positions: baked_positions.clone(),
        });
        current_positions = baked_positions;
    }

    let mut residual_indices = Vec::new();
    let mut residual_positions = Vec::new();
    let mut final_positions = current_positions.clone();
    for (index, (current, target_position)) in current_positions
        .iter()
        .zip(&target_mesh.positions)
        .enumerate()
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
    ensure_strictly_increasing_indices(
        &residual_indices,
        source_mesh.positions.len(),
        Path::new(RESIDUAL_INDEX_FILE),
    )?;
    if !position_slices_bit_equal(&final_positions, &target_mesh.positions) {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "terminal lossless correction did not reconstruct target positions bit-exactly",
        ));
    }

    let lossless_index = manifest_operators.len();
    let lossless_policy = SemanticVerificationPolicy::default();
    let lossless_report = compare_positions_with_policy(
        &final_positions,
        &target_mesh.positions,
        &lossless_policy,
        Path::new(MANIFEST_FILE),
    )?;
    let lossless_stage_file = stage_positions_file(lossless_index, LOSSLESS_STAGE_SLUG);
    let lossless_stage = StageManifestV3 {
        stage_index: StageIndex(lossless_index),
        operator_id: OperatorId(format!("op-{lossless_index:04}-{LOSSLESS_STAGE_SLUG}")),
        label: "Lossless correction".to_owned(),
        baked_positions_file: lossless_stage_file.clone(),
        semantic_verification_policy: lossless_policy,
        semantic_verification_report: lossless_report,
    };
    manifest_operators.push(OperatorManifestV3::LosslessCorrection {
        stage: lossless_stage.clone(),
        correction: LosslessCorrectionManifestV3 {
            residual_index_file: RESIDUAL_INDEX_FILE.to_owned(),
            residual_position_file: RESIDUAL_POSITION_FILE.to_owned(),
            corrected_vertex_count: residual_indices.len(),
        },
    });
    stage_payloads.push(StagePayloadV3 {
        operator_id: lossless_stage.operator_id.clone(),
        label: lossless_stage.label.clone(),
        positions_file: lossless_stage_file,
        positions: final_positions.clone(),
    });

    let weighted_error_before_correction =
        weighted_sum_squared_distance(&current_positions, &target_mesh.positions, &weights);
    let raw_error_before_correction =
        sum_squared_distance(&current_positions, &target_mesh.positions);
    let exact_residual_bytes =
        exact_residual_storage_size(&current_positions, &target_mesh.positions);
    let scoring_policy = default_scoring_policy_v4();
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
            &target_mesh.positions,
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

    let mut manifest = DecompileManifestV3 {
        schema_version: SCHEMA_VERSION_V3,
        coordinate_system: COORDINATE_SYSTEM_V3.to_owned(),
        numeric_format: NumericFormatV3::default(),
        source: MeshAssetV3 {
            path: SOURCE_MESHBIN.to_owned(),
            vertex_count: source_mesh.positions.len(),
            triangle_count: source_mesh.indices.len() / 3,
        },
        target: MeshAssetV3 {
            path: TARGET_MESHBIN.to_owned(),
            vertex_count: target_mesh.positions.len(),
            triangle_count: target_mesh.indices.len() / 3,
        },
        topology: TopologySummaryV3 {
            vertex_count: source_mesh.positions.len(),
            triangle_count: source_mesh.indices.len() / 3,
            index_count: source_mesh.indices.len(),
            hash: topology_hash(source_mesh),
        },
        operators: manifest_operators,
        package_verification: None,
    };
    validate_manifest(&manifest, Path::new(MANIFEST_FILE))?;

    let verification = package_verification_report(
        &manifest,
        source_mesh,
        target_mesh,
        residual_indices.len(),
        &final_positions,
        true,
        Path::new(MANIFEST_FILE),
    )?;
    manifest.package_verification = Some(verification.clone());
    validate_manifest(&manifest, Path::new(MANIFEST_FILE))?;

    let package = DecompilePackageV3 {
        manifest,
        semantic_program: selected_program.clone(),
        package_verification: Some(verification),
        stage_payloads,
        residual_indices,
        residual_positions,
        final_positions,
        inference_diagnostics,
    };
    validate_decompile_package_v3(&package, source_mesh, target_mesh)?;
    Ok(package)
}

/// Validates an in-memory schema-3 package and all retained sidecar payloads.
pub fn validate_decompile_package_v3(
    package: &DecompilePackageV3,
    source_mesh: &TriangleMesh,
    target_mesh: &TriangleMesh,
) -> Result<(), DecompileError> {
    validate_decompile_mesh(source_mesh, "source")?;
    validate_decompile_mesh(target_mesh, "target")?;
    ensure_identical_topology(source_mesh, target_mesh)?;
    validate_program(&package.semantic_program).map_err(|source| {
        invalid_package(
            Path::new(MANIFEST_FILE),
            format!("schema-3 semantic program is invalid: {source}"),
        )
    })?;
    validate_manifest(&package.manifest, Path::new(MANIFEST_FILE))?;

    if package.manifest.operators.len() != package.semantic_program.operators.len() + 1 {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "manifest operator count does not match selected program plus lossless correction",
        ));
    }
    if package.stage_payloads.len() != package.manifest.operators.len() {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "stage payload count does not match manifest operator count",
        ));
    }
    if package.residual_indices.len() != package.residual_positions.len() {
        return Err(invalid_package(
            Path::new(RESIDUAL_INDEX_FILE),
            "residual index and position payload counts differ",
        ));
    }
    ensure_strictly_increasing_indices(
        &package.residual_indices,
        source_mesh.positions.len(),
        Path::new(RESIDUAL_INDEX_FILE),
    )?;

    let mut current_positions = source_mesh.positions.clone();
    let mut semantic_stage_reports_passed = true;
    for (index, operator_manifest) in package.manifest.operators.iter().enumerate() {
        let stage = operator_stage(operator_manifest);
        let payload = &package.stage_payloads[index];
        let expected_slug = operator_slug(operator_manifest);
        let expected_path = stage_positions_file(index, expected_slug);
        let expected_id = OperatorId(format!("op-{index:04}-{expected_slug}"));
        if stage.operator_id != expected_id {
            return Err(invalid_package(
                Path::new(MANIFEST_FILE),
                format!("stage {index} operator id is not stable"),
            ));
        }
        if stage.baked_positions_file != expected_path || payload.positions_file != expected_path {
            return Err(invalid_package(
                Path::new(MANIFEST_FILE),
                format!("stage {index} path does not match its deterministic operator path"),
            ));
        }
        if payload.operator_id != stage.operator_id || payload.label != stage.label {
            return Err(invalid_package(
                Path::new(MANIFEST_FILE),
                format!("stage {index} payload metadata does not match manifest"),
            ));
        }
        if payload.positions.len() != source_mesh.positions.len() {
            return Err(invalid_package(
                Path::new(&stage.baked_positions_file),
                format!("stage {index} baked position count does not match source vertex count"),
            ));
        }

        match operator_manifest {
            OperatorManifestV3::Affine {
                stage, operator, ..
            } => {
                let expected_operator = package.semantic_program.operators[index];
                if expected_operator != ProgramOperator::Affine(*operator) {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        format!("manifest affine operator {index} does not match selected program"),
                    ));
                }
                let semantic_positions = evaluate_package_operator(
                    &expected_operator,
                    &current_positions,
                    Path::new(MANIFEST_FILE),
                )?;
                let report = semantic_stage_report(
                    &expected_operator,
                    &current_positions,
                    &semantic_positions,
                    &payload.positions,
                    stage.semantic_verification_policy,
                    Path::new(&stage.baked_positions_file),
                )?;
                validate_stage_report(stage, report, Path::new(&stage.baked_positions_file))?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed {
                    return Err(invalid_package(
                        Path::new(&stage.baked_positions_file),
                        format!("semantic affine stage {index} does not match its baked payload"),
                    ));
                }
                current_positions = payload.positions.clone();
            }
            OperatorManifestV3::Bend {
                stage, parameters, ..
            } => {
                let expected_operator = package.semantic_program.operators[index];
                if expected_operator != ProgramOperator::Bend(*parameters) {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        format!("manifest bend operator {index} does not match selected program"),
                    ));
                }
                let semantic_positions = evaluate_package_operator(
                    &expected_operator,
                    &current_positions,
                    Path::new(MANIFEST_FILE),
                )?;
                let report = semantic_stage_report(
                    &expected_operator,
                    &current_positions,
                    &semantic_positions,
                    &payload.positions,
                    stage.semantic_verification_policy,
                    Path::new(&stage.baked_positions_file),
                )?;
                validate_stage_report(stage, report, Path::new(&stage.baked_positions_file))?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed {
                    return Err(invalid_package(
                        Path::new(&stage.baked_positions_file),
                        format!("semantic bend stage {index} is outside tolerance"),
                    ));
                }
                current_positions = payload.positions.clone();
            }
            OperatorManifestV3::LosslessCorrection { stage, correction } => {
                if index + 1 != package.manifest.operators.len() {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "lossless correction must be terminal",
                    ));
                }
                if correction.residual_index_file != RESIDUAL_INDEX_FILE
                    || correction.residual_position_file != RESIDUAL_POSITION_FILE
                    || correction.corrected_vertex_count != package.residual_indices.len()
                {
                    return Err(invalid_package(
                        Path::new(MANIFEST_FILE),
                        "lossless correction metadata does not match residual payloads",
                    ));
                }

                let mut corrected = current_positions.clone();
                for (residual_index, residual_position) in package
                    .residual_indices
                    .iter()
                    .zip(&package.residual_positions)
                {
                    let vertex_index = *residual_index as usize;
                    if !positions_bit_equal(*residual_position, target_mesh.positions[vertex_index])
                    {
                        return Err(invalid_package(
                            Path::new(RESIDUAL_POSITION_FILE),
                            "residual positions must be absolute target positions",
                        ));
                    }
                    corrected[vertex_index] = *residual_position;
                }
                let report = compare_positions_with_policy(
                    &corrected,
                    &payload.positions,
                    &stage.semantic_verification_policy,
                    Path::new(&stage.baked_positions_file),
                )?;
                validate_stage_report(stage, report, Path::new(&stage.baked_positions_file))?;
                semantic_stage_reports_passed &= report.passed;
                if !report.passed || !position_slices_bit_equal(&corrected, &target_mesh.positions)
                {
                    return Err(invalid_package(
                        Path::new(&stage.baked_positions_file),
                        "lossless correction does not reconstruct target positions bit-exactly",
                    ));
                }
                if !position_slices_bit_equal(&payload.positions, &target_mesh.positions)
                    || !position_slices_bit_equal(&package.final_positions, &target_mesh.positions)
                {
                    return Err(invalid_package(
                        Path::new(&stage.baked_positions_file),
                        "final baked positions must match target positions bit-exactly",
                    ));
                }
                current_positions = payload.positions.clone();
            }
        }
    }

    let verification = package_verification_report(
        &package.manifest,
        source_mesh,
        target_mesh,
        package.residual_indices.len(),
        &package.final_positions,
        semantic_stage_reports_passed,
        Path::new(MANIFEST_FILE),
    )?;
    if package.package_verification.as_ref() != Some(&verification)
        || package.manifest.package_verification.as_ref() != Some(&verification)
    {
        return Err(invalid_package(
            Path::new(MANIFEST_FILE),
            "package verification report does not match in-memory payloads",
        ));
    }

    Ok(())
}

fn validate_manifest(manifest: &DecompileManifestV3, path: &Path) -> Result<(), DecompileError> {
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
    if manifest.source.path != SOURCE_MESHBIN || manifest.target.path != TARGET_MESHBIN {
        return Err(invalid_package(
            path,
            "schema-3 in-memory packages use canonical source and target mesh asset paths",
        ));
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
        let stage = operator_stage(operator);
        if stage.label.trim().is_empty() {
            return Err(invalid_package(path, "stage labels must not be empty"));
        }
        validate_stage_positions_file(index, &stage.baked_positions_file, path)?;
        validate_stage_policy_for_operator(operator, path)?;
    }
    Ok(())
}

fn validate_stage_positions_file(
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
    if !is_stable_slug(slug) {
        return Err(invalid_package(
            manifest_path,
            format!("stage {index} path slug '{slug}' is not stable"),
        ));
    }
    Ok(())
}

fn validate_stage_policy_for_operator(
    operator: &OperatorManifestV3,
    path: &Path,
) -> Result<(), DecompileError> {
    let stage = operator_stage(operator);
    validate_semantic_verification_policy(&stage.semantic_verification_policy, path)?;
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

fn validate_semantic_verification_policy(
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

fn semantic_stage_report(
    operator: &ProgramOperator,
    previous_baked_positions: &[[f32; 3]],
    semantic_positions: &[[f32; 3]],
    baked_positions: &[[f32; 3]],
    policy: SemanticVerificationPolicy,
    path: &Path,
) -> Result<SemanticVerificationReport, DecompileError> {
    match operator {
        ProgramOperator::Affine(_) => {
            compare_positions_with_policy(semantic_positions, baked_positions, &policy, path)
        }
        ProgramOperator::Bend(parameters) => {
            let bend_policy = bend_stage_policy_from_semantic(policy);
            let bend_report = compare_bend_to_baked_stage(
                parameters,
                previous_baked_positions,
                baked_positions,
                bend_policy,
            )
            .map_err(|source| {
                invalid_package(
                    path,
                    format!("semantic bend stage verification failed: {source}"),
                )
            })?;
            Ok(semantic_report_from_bend(bend_report))
        }
    }
}

fn semantic_report_from_bend(report: BendStageVerificationReport) -> SemanticVerificationReport {
    SemanticVerificationReport {
        max_component_error: report.max_component_error,
        max_euclidean_error: report.max_euclidean_error,
        mean_euclidean_error: report.mean_euclidean_error,
        rms_euclidean_error: report.rms_euclidean_error,
        outside_tolerance: report.outside_tolerance,
        passed: report.passed,
    }
}

fn compare_positions_with_policy(
    left: &[[f32; 3]],
    right: &[[f32; 3]],
    policy: &SemanticVerificationPolicy,
    path: &Path,
) -> Result<SemanticVerificationReport, DecompileError> {
    validate_semantic_verification_policy(policy, path)?;
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
            if component_outside_policy(left_component, right_component, policy) {
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
    Ok(SemanticVerificationReport {
        max_component_error,
        max_euclidean_error,
        mean_euclidean_error: total_euclidean_error / count,
        rms_euclidean_error: (total_squared_euclidean_error / count).sqrt(),
        outside_tolerance,
        passed: outside_tolerance == 0,
    })
}

fn component_outside_policy(left: f32, right: f32, policy: &SemanticVerificationPolicy) -> bool {
    match policy.mode {
        SemanticVerificationMode::BitExact => left.to_bits() != right.to_bits(),
        SemanticVerificationMode::Tolerance => {
            let error = (f64::from(left) - f64::from(right)).abs();
            let magnitude = f64::from(left.abs().max(right.abs()));
            let tolerance = policy
                .absolute_epsilon
                .max(policy.relative_epsilon * magnitude)
                .max(policy.ulp_multiplier * f32_ulp_spacing(right));
            error > tolerance
        }
    }
}

fn f32_ulp_spacing(value: f32) -> f64 {
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

fn validate_stage_report(
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

fn package_verification_report(
    manifest: &DecompileManifestV3,
    source_mesh: &TriangleMesh,
    target_mesh: &TriangleMesh,
    residual_vertex_count: usize,
    final_positions: &[[f32; 3]],
    semantic_stage_reports_passed: bool,
    path: &Path,
) -> Result<PackageVerificationReportV3, DecompileError> {
    let final_policy = SemanticVerificationPolicy::default();
    let final_metrics = compare_positions_with_policy(
        final_positions,
        &target_mesh.positions,
        &final_policy,
        path,
    )?;
    Ok(PackageVerificationReportV3 {
        schema_version: SCHEMA_VERSION_V3,
        topology_exact: source_mesh.indices == target_mesh.indices
            && source_mesh.positions.len() == target_mesh.positions.len(),
        topology_hash_matches_manifest: topology_hash(source_mesh) == manifest.topology.hash,
        positions_bit_exact: position_slices_bit_equal(final_positions, &target_mesh.positions),
        vertex_count: source_mesh.positions.len(),
        triangle_count: source_mesh.indices.len() / 3,
        operator_count: manifest.operators.len(),
        stage_count: manifest.operators.len(),
        residual_vertex_count,
        max_component_error: final_metrics.max_component_error,
        max_euclidean_error: final_metrics.max_euclidean_error,
        outside_tolerance: final_metrics.outside_tolerance,
        semantic_stage_reports_passed,
    })
}

fn evaluate_package_operator(
    operator: &ProgramOperator,
    positions: &[[f32; 3]],
    path: &Path,
) -> Result<Vec<[f32; 3]>, DecompileError> {
    evaluate_operator(operator, positions).map_err(|source| {
        invalid_package(
            path,
            format!("schema-3 operator semantic evaluation failed: {source}"),
        )
    })
}

fn program_operator_diagnostics(
    operator: ProgramOperator,
    path: &Path,
) -> Result<ProgramOperatorDiagnostics, DecompileError> {
    match operator {
        ProgramOperator::Affine(affine) => affine_operator_diagnostics(affine, path),
        ProgramOperator::Bend(parameters) => Ok(ProgramOperatorDiagnostics::Bend { parameters }),
    }
}

fn affine_operator_diagnostics(
    affine: AffineOperator,
    path: &Path,
) -> Result<ProgramOperatorDiagnostics, DecompileError> {
    match affine.semantic_family {
        AffineSemanticFamily::Translation => Ok(ProgramOperatorDiagnostics::Translation {
            translation: affine.translation.ok_or_else(|| {
                invalid_package(
                    path,
                    "translation operator is missing translation parameters",
                )
            })?,
        }),
        AffineSemanticFamily::RigidTransform => Ok(ProgramOperatorDiagnostics::RigidTransform {
            translation: affine.translation.ok_or_else(|| {
                invalid_package(path, "rigid transform is missing translation parameters")
            })?,
            rotation_row_major_3x3: affine.rotation_row_major_3x3.ok_or_else(|| {
                invalid_package(path, "rigid transform is missing rotation parameters")
            })?,
        }),
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

fn program_operator_stage_identity(operator: ProgramOperator) -> (&'static str, &'static str) {
    match operator {
        ProgramOperator::Affine(affine) => affine_stage_identity(affine.semantic_family),
        ProgramOperator::Bend(_) => ("bend", "Bend"),
    }
}

fn affine_stage_identity(family: AffineSemanticFamily) -> (&'static str, &'static str) {
    match family {
        AffineSemanticFamily::Translation => ("translation", "Translation"),
        AffineSemanticFamily::RigidTransform => ("rigid-transform", "Rigid transform"),
        AffineSemanticFamily::SimilarityTransform => {
            ("similarity-transform", "Similarity transform")
        }
        AffineSemanticFamily::GeneralAffine => ("general-affine", "General affine"),
    }
}

fn operator_slug(operator: &OperatorManifestV3) -> &'static str {
    match operator {
        OperatorManifestV3::Affine { operator, .. } => {
            affine_stage_identity(operator.semantic_family).0
        }
        OperatorManifestV3::Bend { .. } => "bend",
        OperatorManifestV3::LosslessCorrection { .. } => LOSSLESS_STAGE_SLUG,
    }
}

fn semantic_policy_for_program_operator(operator: ProgramOperator) -> SemanticVerificationPolicy {
    match operator {
        ProgramOperator::Affine(_) => SemanticVerificationPolicy::default(),
        ProgramOperator::Bend(_) => {
            let policy = bend_stage_policy();
            SemanticVerificationPolicy {
                mode: SemanticVerificationMode::Tolerance,
                absolute_epsilon: policy.absolute_epsilon,
                relative_epsilon: policy.shape_relative_epsilon,
                ulp_multiplier: policy.ulp_multiplier,
            }
        }
    }
}

fn bend_stage_policy() -> BendStageVerificationPolicy {
    BendStageVerificationPolicy::default()
}

fn bend_stage_policy_from_semantic(
    policy: SemanticVerificationPolicy,
) -> BendStageVerificationPolicy {
    BendStageVerificationPolicy {
        absolute_epsilon: policy.absolute_epsilon,
        shape_relative_epsilon: policy.relative_epsilon,
        ulp_multiplier: policy.ulp_multiplier,
    }
}

fn operator_stage(operator: &OperatorManifestV3) -> &StageManifestV3 {
    match operator {
        OperatorManifestV3::Affine { stage, .. }
        | OperatorManifestV3::Bend { stage, .. }
        | OperatorManifestV3::LosslessCorrection { stage, .. } => stage,
    }
}

fn stage_positions_file(index: usize, slug: &str) -> String {
    format!("{OPERATORS_DIR}/{index:04}-{slug}-positions.f32")
}

fn is_stable_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
