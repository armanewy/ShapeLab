//! In-memory session contract types.

use serde::{Deserialize, Serialize};

use crate::{FoundryCandidateSummary, FoundryDocumentId};

/// Foundry session identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundrySessionId(pub String);

/// Lightweight session state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundrySession {
    /// Stable session ID.
    pub id: FoundrySessionId,
    /// Open document ID.
    pub document_id: FoundryDocumentId,
    /// Candidate summaries currently associated with the session.
    pub candidates: Vec<FoundryCandidateSummary>,
}
