//! Schema-3 package manifest contracts.
//!
//! Schema 3 separates semantic evaluation from exact replay. Semantic
//! operators explain editable intent, while each stage's cumulative baked
//! positions file is authoritative for exact reconstruction.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::bend::BendParameters;
use super::program::{
    AffineOperator, OperatorId, OperatorProgram, SemanticVerificationPolicy,
    SemanticVerificationReport, StageIndex,
};

/// Decompiler package manifest schema version for schema 3.
pub const SCHEMA_VERSION_V3: u32 = 3;

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
