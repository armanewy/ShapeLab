//! Candidate proposal contracts.

use serde::{Deserialize, Serialize};

use crate::{CandidateVariationMetadata, VariationIntent};

/// Stable candidate ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryCandidateId(pub String);

/// Candidate generation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateCandidatesRequest {
    /// Optional customizer strategy ID.
    pub strategy_id: Option<String>,
    /// Requested candidate count.
    pub count: u32,
    /// Deterministic seed.
    pub seed: u64,
    /// Product-safe variation intent.
    #[serde(default)]
    pub variation_intent: VariationIntent,
}

/// Status of a candidate in the foundry session.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCandidateStatus {
    /// Candidate is available for preview/acceptance.
    Proposed,
    /// Candidate was accepted into the document.
    Accepted,
    /// Candidate was rejected by the user or validation.
    Rejected,
    /// Candidate failed to compile or validate.
    Invalid,
}

/// Candidate summary row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidateSummary {
    /// Candidate ID.
    pub id: FoundryCandidateId,
    /// Human-facing label.
    pub label: String,
    /// Status.
    pub status: FoundryCandidateStatus,
    /// Deterministic changed-control IDs.
    pub changed_controls: Vec<String>,
    /// Optional preview ID.
    pub preview_id: Option<String>,
    /// Product-safe variation metadata.
    #[serde(default)]
    pub variation_metadata: CandidateVariationMetadata,
}
