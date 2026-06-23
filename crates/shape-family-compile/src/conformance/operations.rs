//! Operation inventory conformance report contracts.

use serde::{Deserialize, Serialize};
use shape_family::AllowedOperationKind;

use super::ConformanceStatus;

/// Conformance row for operation-class inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationConformance {
    /// Operation class.
    pub operation: AllowedOperationKind,
    /// Number of compiled operations in this class.
    pub actual_count: u32,
    /// Whether this operation class is allowed by the family/style contract.
    pub allowed: bool,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this operation class.
    pub issue_codes: Vec<String>,
}
