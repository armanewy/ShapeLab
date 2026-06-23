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
    /// Optional local-only usability records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_usability: Option<FoundryUsabilityLog>,
}

/// Local-only foundry usability record log.
///
/// Records use milliseconds elapsed from the local session start. They do not
/// carry geometry payloads, output directories, or absolute file paths.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryUsabilityLog {
    /// Ordered local usability records.
    #[serde(default)]
    pub records: Vec<FoundryUsabilityRecord>,
}

impl FoundryUsabilityLog {
    /// Create an empty local usability log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a local usability record.
    pub fn record(&mut self, record: FoundryUsabilityRecord) {
        self.records.push(record);
    }

    /// Compute aggregate usability metrics from the local records.
    #[must_use]
    pub fn metrics(&self) -> FoundryUsabilityMetrics {
        let mut control_attempts = 0_u64;
        let mut control_successes = 0_u64;
        let mut requested_candidates = 0_u64;
        let mut survived_candidates = 0_u64;
        let mut accepted_change_count = 0_u64;
        let mut invalid_state_attempts = 0_u64;
        let mut advanced_view_visits = 0_u64;
        let mut total_session_time_ms = 0_u64;
        let mut time_to_first_build_ms = None;
        let mut time_to_first_export_ms = None;

        for record in &self.records {
            total_session_time_ms = total_session_time_ms.max(record.elapsed_ms);

            match record.event {
                FoundryUsabilityEvent::ProfileOpened
                | FoundryUsabilityEvent::Reset
                | FoundryUsabilityEvent::Lock
                | FoundryUsabilityEvent::Undo => {}
                FoundryUsabilityEvent::BuildCompleted => {
                    time_to_first_build_ms.get_or_insert(record.elapsed_ms);
                }
                FoundryUsabilityEvent::CandidateRequest { requested_count } => {
                    requested_candidates += u64::from(requested_count);
                }
                FoundryUsabilityEvent::CandidateSurvival { survived_count } => {
                    survived_candidates += u64::from(survived_count);
                }
                FoundryUsabilityEvent::CandidateAccepted { accepted_count } => {
                    accepted_change_count += u64::from(accepted_count);
                }
                FoundryUsabilityEvent::ControlChange { accepted } => {
                    control_attempts += 1;
                    if accepted {
                        control_successes += 1;
                        accepted_change_count += 1;
                    }
                }
                FoundryUsabilityEvent::InvalidAttempt => {
                    invalid_state_attempts += 1;
                }
                FoundryUsabilityEvent::Export => {
                    time_to_first_export_ms.get_or_insert(record.elapsed_ms);
                }
                FoundryUsabilityEvent::AdvancedRecipeViewOpened => {
                    advanced_view_visits += 1;
                }
            }
        }

        FoundryUsabilityMetrics {
            control_success_rate: ratio(control_successes, control_attempts),
            candidate_survival_rate: ratio(survived_candidates, requested_candidates),
            accepted_change_count,
            invalid_state_attempts,
            advanced_view_visits,
            total_session_time_ms,
            time_to_first_build_ms,
            time_to_first_export_ms,
        }
    }
}

/// One local-only usability event observed during a foundry session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryUsabilityRecord {
    /// Milliseconds elapsed from local session start.
    pub elapsed_ms: u64,
    /// Event payload.
    pub event: FoundryUsabilityEvent,
}

impl FoundryUsabilityRecord {
    /// Create a local usability record at a relative session time.
    #[must_use]
    pub fn new(elapsed_ms: u64, event: FoundryUsabilityEvent) -> Self {
        Self { elapsed_ms, event }
    }
}

/// Local foundry usability event kinds.
///
/// Event variants intentionally avoid model geometry, export directories, and
/// absolute file paths. Hosts that need richer diagnostics should keep those in
/// separate explicit debug artifacts, not in this default log.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FoundryUsabilityEvent {
    /// A customizer profile was opened.
    ProfileOpened,
    /// The session completed a build.
    BuildCompleted,
    /// Candidates were requested.
    CandidateRequest {
        /// Number of requested candidates.
        requested_count: u32,
    },
    /// Candidates survived validation or pruning.
    CandidateSurvival {
        /// Number of surviving candidates.
        survived_count: u32,
    },
    /// Candidate changes were accepted into the document.
    CandidateAccepted {
        /// Number of accepted candidates.
        accepted_count: u32,
    },
    /// A control edit was attempted.
    ControlChange {
        /// Whether the attempted control edit was accepted.
        accepted: bool,
    },
    /// Controls or local state were reset.
    Reset,
    /// A local lock was added or changed.
    Lock,
    /// An invalid state or command was attempted.
    InvalidAttempt,
    /// The session performed undo.
    Undo,
    /// The session exported an artifact.
    Export,
    /// The advanced recipe view was opened.
    AdvancedRecipeViewOpened,
}

/// Aggregate local usability metrics derived from a foundry session log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryUsabilityMetrics {
    /// Successful control changes divided by attempted control changes.
    pub control_success_rate: Option<f64>,
    /// Surviving candidates divided by requested candidates.
    pub candidate_survival_rate: Option<f64>,
    /// Accepted candidate and accepted control-change count.
    pub accepted_change_count: u64,
    /// Invalid state or command attempts.
    pub invalid_state_attempts: u64,
    /// Advanced recipe view visits.
    pub advanced_view_visits: u64,
    /// Last observed relative event time.
    pub total_session_time_ms: u64,
    /// Relative time of the first completed build.
    pub time_to_first_build_ms: Option<u64>,
    /// Relative time of the first export.
    pub time_to_first_export_ms: Option<u64>,
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}
