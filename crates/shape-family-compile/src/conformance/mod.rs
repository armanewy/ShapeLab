//! Family-level conformance report contracts.
//!
//! Wave 3 freezes the serializable report surface. The evaluators that fill
//! these reports live in later waves.

pub mod attachments;
pub mod export;
pub mod geometry;
pub mod operations;
pub mod roles;

use serde::{Deserialize, Serialize};
use shape_family::FamilyRuleExecutionPolicy;

pub use attachments::*;
pub use export::*;
pub use geometry::*;
pub use operations::*;
pub use roles::*;

/// Current schema version for family conformance reports.
pub const FAMILY_CONFORMANCE_REPORT_SCHEMA_VERSION: u32 = 1;

/// Overall conformance status for one rule or report row.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConformanceStatus {
    /// The rule was evaluated and passed.
    Passed,
    /// The rule was evaluated and failed.
    Failed,
    /// The rule is deferred by policy to runtime/export code.
    Deferred,
    /// The implementation has no evaluator for the required contract yet.
    Unsupported,
    /// Required data was absent.
    Missing,
    /// The row is present for deterministic reporting but was not evaluated.
    NotEvaluated,
}

impl ConformanceStatus {
    /// Return true when this status rejects a required conformance contract.
    #[must_use]
    pub fn rejects_required(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Deferred | Self::Unsupported | Self::Missing | Self::NotEvaluated
        )
    }
}

/// One deterministic conformance issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Policy that controls whether this issue rejects the asset.
    pub policy: FamilyRuleExecutionPolicy,
    /// Evaluated status.
    pub status: ConformanceStatus,
}

/// Complete family conformance report for one instantiated asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyConformanceReport {
    /// Conformance report schema version.
    pub schema_version: u32,
    /// Family ID.
    pub family_id: String,
    /// Style-kit ID.
    pub style_kit_id: String,
    /// Role conformance rows.
    pub roles: Vec<RoleConformance>,
    /// Attachment conformance rows.
    pub attachments: Vec<AttachmentConformance>,
    /// Geometric constraint conformance rows.
    pub constraints: Vec<ConstraintConformance>,
    /// Operation inventory conformance rows.
    pub operations: Vec<OperationConformance>,
    /// Export profile conformance rows.
    pub exports: Vec<ExportRequirementConformance>,
    /// Flattened deterministic issue list.
    pub issues: Vec<ConformanceIssue>,
}

impl Default for FamilyConformanceReport {
    fn default() -> Self {
        Self {
            schema_version: FAMILY_CONFORMANCE_REPORT_SCHEMA_VERSION,
            family_id: String::new(),
            style_kit_id: String::new(),
            roles: Vec::new(),
            attachments: Vec::new(),
            constraints: Vec::new(),
            operations: Vec::new(),
            exports: Vec::new(),
            issues: Vec::new(),
        }
    }
}

impl FamilyConformanceReport {
    /// Return true when no required conformance row or issue rejects the asset.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| !required_issue_rejects(issue))
            && self.roles.iter().all(|row| !row.status.rejects_required())
            && self.attachments.iter().all(|row| {
                row.policy != FamilyRuleExecutionPolicy::Required || !row.status.rejects_required()
            })
            && self.constraints.iter().all(|row| {
                row.policy != FamilyRuleExecutionPolicy::Required || !row.status.rejects_required()
            })
            && self
                .operations
                .iter()
                .all(|row| !row.status.rejects_required())
            && self
                .exports
                .iter()
                .all(|row| !row.status.rejects_required())
    }
}

fn required_issue_rejects(issue: &ConformanceIssue) -> bool {
    issue.policy == FamilyRuleExecutionPolicy::Required && issue.status.rejects_required()
}
