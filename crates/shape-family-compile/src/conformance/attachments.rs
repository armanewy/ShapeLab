//! Attachment conformance report contracts.

use serde::{Deserialize, Serialize};
use shape_asset::{PartInstanceId, SocketId};
use shape_family::FamilyRuleExecutionPolicy;

use super::ConformanceStatus;

/// Concrete part/socket endpoint used by an attachment conformance row.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AttachmentEndpointConformance {
    /// Concrete part occurrence.
    pub instance: PartInstanceId,
    /// Socket on the occurrence definition.
    pub socket: SocketId,
}

/// Concrete pair evaluated for one attachment rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentPairConformance {
    /// Parent endpoint.
    pub parent: AttachmentEndpointConformance,
    /// Child endpoint.
    pub child: AttachmentEndpointConformance,
    /// Whether socket tags and frames are compatible.
    pub socket_compatible: bool,
    /// Whether the expected relationship was found.
    pub connected: bool,
}

/// Coverage summary for repeated attachment rules.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AttachmentCoverageConformance {
    /// First-side endpoints not covered by any evaluated pair.
    pub unmatched_first: Vec<AttachmentEndpointConformance>,
    /// Second-side endpoints not covered by any evaluated pair.
    pub unmatched_second: Vec<AttachmentEndpointConformance>,
    /// Whether pairing produced at least one row.
    pub produced_pairs: bool,
}

/// Conformance row for one family attachment rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentConformance {
    /// Family attachment-rule ID.
    pub rule_id: String,
    /// Family rule `from_role`, the child/dependent side of the attachment.
    pub from_role: String,
    /// Family rule `to_role`, the parent/destination side of the attachment.
    pub to_role: String,
    /// Rule policy.
    pub policy: FamilyRuleExecutionPolicy,
    /// Evaluated pairs.
    pub pairs: Vec<AttachmentPairConformance>,
    /// Pairing coverage summary.
    pub coverage: AttachmentCoverageConformance,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this rule.
    pub issue_codes: Vec<String>,
}
