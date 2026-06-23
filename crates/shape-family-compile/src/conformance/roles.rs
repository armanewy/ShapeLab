//! Role conformance report contracts.

use serde::{Deserialize, Serialize};

use super::ConformanceStatus;

/// Inclusive role occurrence expectation resolved from family multiplicity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMultiplicityExpectation {
    /// Minimum accepted occurrence count.
    pub min: u32,
    /// Maximum accepted occurrence count, or `None` for unbounded repeated roles.
    pub max: Option<u32>,
}

/// Conformance row for one family role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleConformance {
    /// Family role ID.
    pub role: String,
    /// Expected occurrence range.
    pub expected: RoleMultiplicityExpectation,
    /// Actual exported occurrence count.
    pub actual_occurrences: u32,
    /// Whether provider selection and presence controls left this role enabled.
    pub effective_enabled: bool,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this role.
    pub issue_codes: Vec<String>,
}
