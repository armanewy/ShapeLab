#![forbid(unsafe_code)]

//! Inverse semantic reconstruction contracts.
//!
//! This crate intentionally does not claim strict success. It defines the
//! failure report shape that inverse search must return when exact semantic
//! reconstruction cannot be proven.

pub mod analysis;
pub mod character_recovery;
pub mod deformation_recovery;
pub mod external_character;
pub mod hypotheses;
pub mod import_triage;
pub mod recovery_gate;
pub mod search;
pub mod strict;

use serde::{Deserialize, Serialize};
use shape_program::{
    ModelingOperationKind, ModelingProgram, SemanticBoundaryLoopId, SemanticPartId,
};
use shape_program_verify::StrictSemanticVerification;

/// Report returned when strict inverse reconstruction fails or is partial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrictReconstructionFailureReport {
    /// Best semantic program found before failure, if any.
    pub best_semantic_program: Option<ModelingProgram>,
    /// Topology regions not explained by the best program.
    pub unexplained_topology_regions: Vec<UnexplainedTopologyRegion>,
    /// Geometry regions not explained by the best program.
    pub unexplained_geometry: Vec<UnexplainedGeometryRegion>,
    /// Operator capabilities missing from the forward language.
    pub missing_operator_capabilities: Vec<MissingOperatorCapability>,
    /// Search limits reached before proof.
    pub search_limits_reached: Vec<SearchLimitReached>,
    /// Residual diagnostic, excluded from strict success.
    pub residual_diagnostic: Option<ResidualDiagnostic>,
    /// Verification report for the best attempt, when available.
    pub verification: Option<StrictSemanticVerification>,
}

impl StrictReconstructionFailureReport {
    /// Return true when this report is only a failure/partial explanation.
    #[must_use]
    pub fn strict_success_excluded(&self) -> bool {
        self.verification
            .as_ref()
            .is_none_or(|verification| !verification.accepted)
            || self.residual_diagnostic.is_some()
            || !self.unexplained_topology_regions.is_empty()
            || !self.unexplained_geometry.is_empty()
            || !self.missing_operator_capabilities.is_empty()
            || !self.search_limits_reached.is_empty()
    }

    /// Construct a failure report from a residual-only diagnostic.
    #[must_use]
    pub fn residual_only(
        best_semantic_program: Option<ModelingProgram>,
        residual_diagnostic: ResidualDiagnostic,
    ) -> Self {
        Self {
            best_semantic_program,
            unexplained_topology_regions: Vec::new(),
            unexplained_geometry: Vec::new(),
            missing_operator_capabilities: Vec::new(),
            search_limits_reached: Vec::new(),
            residual_diagnostic: Some(residual_diagnostic),
            verification: None,
        }
    }
}

/// Unexplained topology region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnexplainedTopologyRegion {
    /// Stable diagnostic ID.
    pub id: String,
    /// Optional part ID.
    pub part: Option<SemanticPartId>,
    /// Optional boundary loop involved in the failure.
    pub boundary_loop: Option<SemanticBoundaryLoopId>,
    /// Human-readable reason.
    pub reason: String,
}

/// Unexplained geometry region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnexplainedGeometryRegion {
    /// Stable diagnostic ID.
    pub id: String,
    /// Approximate affected vertex count.
    pub affected_vertices: usize,
    /// Maximum position error from the best semantic program.
    pub max_error: f64,
    /// Human-readable reason.
    pub reason: String,
}

/// Missing capability needed for exact reconstruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingOperatorCapability {
    /// Operation kind that would be needed, if known.
    pub operation_kind: Option<ModelingOperationKind>,
    /// Capability label.
    pub capability: String,
    /// Human-readable evidence.
    pub evidence: String,
}

/// Search limit that prevented proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchLimitReached {
    /// Search stage.
    pub stage: String,
    /// Numeric limit.
    pub limit: usize,
    /// Number of candidates explored.
    pub explored: usize,
}

/// Residual diagnostic that is explicitly excluded from strict success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidualDiagnostic {
    /// Residual byte count.
    pub residual_bytes: usize,
    /// Residual carrier kind.
    pub carrier: ResidualCarrier,
    /// Human-readable message.
    pub message: String,
}

/// Residual carrier kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidualCarrier {
    VertexDeltaBuffer,
    TextureLikeDisplacement,
    OpaqueCorrectionBlob,
    AuditOnlyHeatmap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_diagnostic_never_counts_as_strict_success() {
        let report = StrictReconstructionFailureReport::residual_only(
            Some(ModelingProgram::strict_from_primitives()),
            ResidualDiagnostic {
                residual_bytes: 16,
                carrier: ResidualCarrier::VertexDeltaBuffer,
                message: "semantic search did not explain one bevel band".to_owned(),
            },
        );

        assert!(report.strict_success_excluded());
        assert_eq!(report.residual_diagnostic.unwrap().residual_bytes, 16);
    }

    #[test]
    fn missing_operator_and_search_limits_keep_report_partial() {
        let report = StrictReconstructionFailureReport {
            best_semantic_program: Some(ModelingProgram::strict_from_primitives()),
            unexplained_topology_regions: vec![UnexplainedTopologyRegion {
                id: "topology.region.1".to_owned(),
                part: None,
                boundary_loop: Some(SemanticBoundaryLoopId("loop.vent.1".to_owned())),
                reason: "opening boundary has unsupported branch".to_owned(),
            }],
            unexplained_geometry: Vec::new(),
            missing_operator_capabilities: vec![MissingOperatorCapability {
                operation_kind: Some(ModelingOperationKind::ConstrainedBoolean),
                capability: "non-planar cutter support".to_owned(),
                evidence: "detected curved through-cut".to_owned(),
            }],
            search_limits_reached: vec![SearchLimitReached {
                stage: "beam.hard_surface".to_owned(),
                limit: 256,
                explored: 256,
            }],
            residual_diagnostic: None,
            verification: None,
        };

        assert!(report.strict_success_excluded());
        assert_eq!(report.search_limits_reached[0].explored, 256);
    }
}
