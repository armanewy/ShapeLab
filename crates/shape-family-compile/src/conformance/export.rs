//! Export requirement conformance report contracts.

use serde::{Deserialize, Serialize};
use shape_family::RuntimeMetadataRequirement;

use super::ConformanceStatus;

/// Availability of runtime/export metadata in a compiled asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportMetadataAvailability {
    /// Metadata is present in the compiled package.
    Available,
    /// Metadata is expected to be supplied by an adapter after compilation.
    AdapterDeferred,
    /// Required metadata is absent.
    Missing,
}

/// Conformance row for one metadata requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportMetadataConformance {
    /// Metadata category.
    pub requirement: RuntimeMetadataRequirement,
    /// Availability result.
    pub availability: ExportMetadataAvailability,
    /// Deterministic issue codes attached to this requirement.
    pub issue_codes: Vec<String>,
}

/// Conformance row for one export profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequirementConformance {
    /// Export profile key.
    pub profile: String,
    /// Metadata rows.
    pub metadata: Vec<ExportMetadataConformance>,
    /// Optional triangle budget hint from the family contract.
    pub triangle_budget_hint: Option<u32>,
    /// Actual triangle count when an artifact is available.
    pub actual_triangle_count: Option<u32>,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this export profile.
    pub issue_codes: Vec<String>,
}
