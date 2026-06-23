//! Typed automation command and query contracts.

use serde::{Deserialize, Serialize};
use shape_asset::RevisionId;

use crate::{
    CatalogContentRef, ControlValue, FoundryAssetDocument, FoundryCandidateId,
    FoundryCandidateSummary, FoundryLock, FoundryLockTarget, FoundryValidationReport,
    GenerateCandidatesRequest,
};

/// Serializable command API for foundry automation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum FoundryCommand {
    /// Set one customizer control.
    SetControl {
        /// Control ID.
        control_id: String,
        /// New value.
        value: ControlValue,
    },
    /// Reset one customizer control to its authored default.
    ResetControl {
        /// Control ID.
        control_id: String,
    },
    /// Select a provider for a role.
    SelectProvider {
        /// Family role.
        role: String,
        /// Provider content reference.
        provider_ref: CatalogContentRef,
    },
    /// Enable or disable a family role.
    SetRolePresence {
        /// Family role.
        role: String,
        /// New enabled state.
        enabled: bool,
    },
    /// Switch the style and style implementation references.
    SetStyle {
        /// Style content reference.
        style_content_ref: CatalogContentRef,
        /// Style implementation reference.
        style_implementation_ref: CatalogContentRef,
    },
    /// Set or replace one foundry lock.
    SetLock {
        /// Lock row.
        lock: FoundryLock,
    },
    /// Remove one foundry lock/protection row.
    ClearLock {
        /// Lock target to clear.
        target: FoundryLockTarget,
    },
    /// Generate candidates.
    GenerateCandidates(GenerateCandidatesRequest),
    /// Accept a candidate.
    AcceptCandidate {
        /// Candidate ID.
        candidate_id: FoundryCandidateId,
    },
    /// Reject a candidate.
    RejectCandidate {
        /// Candidate ID.
        candidate_id: FoundryCandidateId,
    },
    /// Undo to the parent revision.
    Undo,
    /// Switch to a revision.
    SwitchRevision {
        /// Revision ID.
        revision_id: RevisionId,
    },
    /// Export the current asset.
    Export {
        /// Export profile key.
        profile: String,
        /// Optional output directory token/path decided by the host.
        out_dir: Option<String>,
    },
    /// Add the current document to a pack.
    AddCurrentToPack {
        /// Pack ID.
        pack_id: String,
        /// Member ID inside the pack.
        member_id: String,
    },
}

/// Query contract for automation clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum FoundryQuery {
    /// Return a state snapshot.
    StateSnapshot,
    /// Return the feasible domain for one control.
    ControlDomain {
        /// Control ID.
        control_id: String,
    },
    /// Return candidate summaries.
    Candidates,
    /// Return revision graph.
    Revisions,
    /// Return export profiles.
    ExportProfiles,
    /// Return conformance report.
    Conformance,
}

/// Lightweight conformance summary embedded in state/project contracts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryConformanceSummary {
    /// Whether required conformance passed.
    pub accepted: bool,
    /// Number of required failures.
    pub required_failure_count: usize,
    /// Number of advisory issues.
    pub advisory_issue_count: usize,
    /// Number of runtime-deferred rows.
    pub runtime_deferred_count: usize,
}

/// Deterministic foundry state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryStateSnapshot {
    /// Current semantic source document.
    pub document: FoundryAssetDocument,
    /// Validation report.
    pub validation: FoundryValidationReport,
    /// Conformance summary.
    pub conformance: FoundryConformanceSummary,
    /// Candidate summaries.
    pub candidates: Vec<FoundryCandidateSummary>,
}
