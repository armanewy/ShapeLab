//! Local-only Make job lifecycle trace and latency summary.

use std::fs;
use std::io;
use std::path::Path;

use orchard_foundry::{FoundryAssetDocument, FoundryBuildStamp};
use serde::{Deserialize, Serialize};

use super::jobs::{FoundryJobRequest, FoundryJobSlot};

pub(crate) const MAKE_JOB_TRACE_DIR: &str = "target/make-job-traces";
pub(crate) const MAKE_JOB_TRACE_FILE: &str = "make-job-trace.json";
pub(crate) const MAKE_LATENCY_SUMMARY_FILE: &str = "make-latency-summary.json";

/// Local-only Make lifecycle trace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MakeJobTrace {
    /// Trace events recorded for this app session.
    pub events: Vec<MakeJobTraceEvent>,
    elapsed_ms: u64,
}

impl MakeJobTrace {
    /// Set the current app-relative clock used by subsequent events.
    pub(crate) fn set_elapsed_ms(&mut self, elapsed_ms: u64) {
        self.elapsed_ms = elapsed_ms;
    }

    /// Return the current app-relative clock used by new events.
    #[must_use]
    pub(crate) fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    /// Advance the deterministic test clock by one millisecond.
    pub(crate) fn tick_for_test(&mut self) {
        self.elapsed_ms = self.elapsed_ms.saturating_add(1);
    }

    /// Record a local trace event.
    pub(crate) fn record(&mut self, event: MakeJobTraceEvent) {
        self.events.push(event);
    }

    /// Build a per-session latency summary from the recorded trace.
    #[must_use]
    pub(crate) fn summary(&self) -> MakeLatencySummary {
        MakeLatencySummary::from_events(&self.events)
    }

    /// Serialize trace and summary files under the requested directory.
    pub(crate) fn write_outputs(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        write_json_pretty(&dir.join(MAKE_JOB_TRACE_FILE), &self.events)?;
        write_json_pretty(&dir.join(MAKE_LATENCY_SUMMARY_FILE), &self.summary())?;
        Ok(())
    }
}

/// One Make lifecycle trace event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MakeJobTraceEvent {
    pub timestamp_relative_ms: u64,
    pub event_kind: MakeJobTraceEventKind,
    pub job_id: Option<u64>,
    pub job_slot: Option<String>,
    pub document_generation_or_build_stamp: Option<String>,
    pub asset_name: Option<String>,
    pub trigger_action: Option<String>,
    pub queue_depth: usize,
    pub canceled_previous_job: bool,
    pub ignored_as_stale: bool,
    pub message: String,
}

/// Input for constructing a trace event without a long argument list.
pub(crate) struct MakeJobTraceEventInput {
    pub elapsed_ms: u64,
    pub event_kind: MakeJobTraceEventKind,
    pub job_id: Option<u64>,
    pub slot: Option<FoundryJobSlot>,
    pub stamp: Option<String>,
    pub asset_name: Option<String>,
    pub trigger_action: Option<String>,
    pub queue_depth: usize,
    pub canceled_previous_job: bool,
    pub ignored_as_stale: bool,
    pub message: String,
}

/// Stable Make lifecycle event kinds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MakeJobTraceEventKind {
    TemplateStarted,
    BuildQueued,
    BuildStarted,
    BuildFinished,
    BuildFailed,
    PreviewQueued,
    PreviewStarted,
    PreviewFinished,
    PreviewFailed,
    CandidateQueued,
    CandidateStarted,
    CandidateCompiled,
    CandidateRendered,
    CandidateFinished,
    CandidateFailed,
    JobCanceled,
    JobIgnoredAsStale,
    JobReused,
    UserActionBlocked,
    UserActionAccepted,
    StateTransition,
}

/// Per-session latency and churn summary derived from a Make trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MakeLatencySummary {
    pub time_to_make_open_ms: Option<u64>,
    pub time_to_first_visible_model_ms: Option<u64>,
    pub time_to_first_build_started_ms: Option<u64>,
    pub time_to_first_build_finished_ms: Option<u64>,
    pub time_to_first_preview_ready_ms: Option<u64>,
    pub time_to_first_candidate_request_ms: Option<u64>,
    pub time_to_first_skeleton_idea_tray_ms: Option<u64>,
    pub time_to_first_candidate_result_ms: Option<u64>,
    pub time_to_first_candidate_shell_ms: Option<u64>,
    pub time_to_first_candidate_preview_ms: Option<u64>,
    pub time_to_first_selectable_candidate_ms: Option<u64>,
    pub total_jobs_queued: u64,
    pub total_jobs_canceled: u64,
    pub total_jobs_ignored_as_stale: u64,
    pub reused_job_count: u64,
    pub coalesced_job_count: u64,
    pub duplicate_build_jobs: u64,
    pub duplicate_preview_jobs: u64,
    pub duplicate_candidate_jobs: u64,
    pub longest_preparing_span_ms: Option<u64>,
    pub longest_generating_span_ms: Option<u64>,
    pub warnings: Vec<String>,
}

impl MakeLatencySummary {
    #[must_use]
    pub(crate) fn from_events(events: &[MakeJobTraceEvent]) -> Self {
        let mut summary = Self {
            time_to_make_open_ms: first_event_ms(events, MakeJobTraceEventKind::TemplateStarted),
            time_to_first_visible_model_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::TemplateStarted,
            ),
            time_to_first_build_started_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::BuildStarted,
            ),
            time_to_first_build_finished_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::BuildFinished,
            ),
            time_to_first_preview_ready_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::PreviewFinished,
            ),
            time_to_first_candidate_request_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateQueued,
            ),
            time_to_first_skeleton_idea_tray_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateQueued,
            ),
            time_to_first_candidate_result_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateCompiled,
            ),
            time_to_first_candidate_shell_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateCompiled,
            ),
            time_to_first_candidate_preview_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateRendered,
            ),
            time_to_first_selectable_candidate_ms: first_event_ms(
                events,
                MakeJobTraceEventKind::CandidateRendered,
            ),
            total_jobs_queued: 0,
            total_jobs_canceled: 0,
            total_jobs_ignored_as_stale: 0,
            reused_job_count: 0,
            coalesced_job_count: 0,
            duplicate_build_jobs: 0,
            duplicate_preview_jobs: 0,
            duplicate_candidate_jobs: 0,
            longest_preparing_span_ms: longest_span(
                events,
                &[
                    MakeJobTraceEventKind::BuildQueued,
                    MakeJobTraceEventKind::PreviewQueued,
                ],
                &[
                    MakeJobTraceEventKind::PreviewFinished,
                    MakeJobTraceEventKind::PreviewFailed,
                ],
            ),
            longest_generating_span_ms: longest_span(
                events,
                &[MakeJobTraceEventKind::CandidateQueued],
                &[
                    MakeJobTraceEventKind::CandidateFinished,
                    MakeJobTraceEventKind::CandidateFailed,
                ],
            ),
            warnings: Vec::new(),
        };

        for event in events {
            match event.event_kind {
                MakeJobTraceEventKind::BuildQueued
                | MakeJobTraceEventKind::PreviewQueued
                | MakeJobTraceEventKind::CandidateQueued => summary.total_jobs_queued += 1,
                MakeJobTraceEventKind::JobCanceled => summary.total_jobs_canceled += 1,
                MakeJobTraceEventKind::JobIgnoredAsStale => {
                    summary.total_jobs_ignored_as_stale += 1;
                }
                MakeJobTraceEventKind::JobReused => {
                    summary.reused_job_count += 1;
                    summary.coalesced_job_count += 1;
                    match event.job_slot.as_deref() {
                        Some("CompileCurrent") | Some("ApplyEdit") => {
                            summary.duplicate_build_jobs += 1;
                        }
                        Some("RenderPreview") => summary.duplicate_preview_jobs += 1,
                        Some("GenerateCandidates") => summary.duplicate_candidate_jobs += 1,
                        _ => {}
                    }
                }
                _ => {}
            }

            if matches!(
                event.event_kind,
                MakeJobTraceEventKind::JobCanceled
                    | MakeJobTraceEventKind::JobIgnoredAsStale
                    | MakeJobTraceEventKind::JobReused
                    | MakeJobTraceEventKind::UserActionBlocked
            ) {
                summary.warnings.push(event.message.clone());
            }
        }

        summary.warnings.sort();
        summary.warnings.dedup();
        summary
    }
}

#[must_use]
pub(crate) fn trace_event_for_request(
    elapsed_ms: u64,
    event_kind: MakeJobTraceEventKind,
    request: &FoundryJobRequest,
    queue_depth: usize,
    canceled_previous_job: bool,
    message: impl Into<String>,
) -> MakeJobTraceEvent {
    MakeJobTraceEvent {
        timestamp_relative_ms: elapsed_ms,
        event_kind,
        job_id: Some(request.job_id()),
        job_slot: Some(job_slot_label(request.slot()).to_owned()),
        document_generation_or_build_stamp: request_trace_stamp(request),
        asset_name: request_asset_name(request),
        trigger_action: Some(trigger_action_for_request(request).to_owned()),
        queue_depth,
        canceled_previous_job,
        ignored_as_stale: false,
        message: message.into(),
    }
}

#[must_use]
pub(crate) fn trace_event(input: MakeJobTraceEventInput) -> MakeJobTraceEvent {
    MakeJobTraceEvent {
        timestamp_relative_ms: input.elapsed_ms,
        event_kind: input.event_kind,
        job_id: input.job_id,
        job_slot: input.slot.map(job_slot_label).map(str::to_owned),
        document_generation_or_build_stamp: input.stamp,
        asset_name: input.asset_name,
        trigger_action: input.trigger_action,
        queue_depth: input.queue_depth,
        canceled_previous_job: input.canceled_previous_job,
        ignored_as_stale: input.ignored_as_stale,
        message: input.message,
    }
}

#[must_use]
pub(crate) fn queued_kind_for_request(request: &FoundryJobRequest) -> MakeJobTraceEventKind {
    match request {
        FoundryJobRequest::CompileCurrent { .. } | FoundryJobRequest::ApplyEdit { .. } => {
            MakeJobTraceEventKind::BuildQueued
        }
        FoundryJobRequest::RenderPreview { .. } | FoundryJobRequest::PreviewControlValue { .. } => {
            MakeJobTraceEventKind::PreviewQueued
        }
        FoundryJobRequest::GenerateCandidates { .. }
        | FoundryJobRequest::RenderCandidatePreviews { .. } => {
            MakeJobTraceEventKind::CandidateQueued
        }
        FoundryJobRequest::CompilePack { .. }
        | FoundryJobRequest::ExportPack { .. }
        | FoundryJobRequest::Export { .. } => MakeJobTraceEventKind::StateTransition,
    }
}

#[must_use]
pub(crate) fn started_kind_for_request(request: &FoundryJobRequest) -> MakeJobTraceEventKind {
    match request {
        FoundryJobRequest::CompileCurrent { .. } | FoundryJobRequest::ApplyEdit { .. } => {
            MakeJobTraceEventKind::BuildStarted
        }
        FoundryJobRequest::RenderPreview { .. } | FoundryJobRequest::PreviewControlValue { .. } => {
            MakeJobTraceEventKind::PreviewStarted
        }
        FoundryJobRequest::GenerateCandidates { .. }
        | FoundryJobRequest::RenderCandidatePreviews { .. } => {
            MakeJobTraceEventKind::CandidateStarted
        }
        FoundryJobRequest::CompilePack { .. }
        | FoundryJobRequest::ExportPack { .. }
        | FoundryJobRequest::Export { .. } => MakeJobTraceEventKind::StateTransition,
    }
}

#[must_use]
pub(crate) fn job_slot_label(slot: FoundryJobSlot) -> &'static str {
    match slot {
        FoundryJobSlot::CompileCurrent => "CompileCurrent",
        FoundryJobSlot::RenderPreview => "RenderPreview",
        FoundryJobSlot::GenerateCandidates => "GenerateCandidates",
        FoundryJobSlot::ApplyEdit => "ApplyEdit",
        FoundryJobSlot::CompilePack => "CompilePack",
        FoundryJobSlot::Export => "Export",
    }
}

#[must_use]
pub(crate) fn document_trace_stamp(document: &FoundryAssetDocument) -> String {
    document
        .build_stamp
        .as_ref()
        .map(build_trace_stamp)
        .unwrap_or_else(|| format!("document:{}", sanitize_trace_text(&document.document_id.0)))
}

#[must_use]
pub(crate) fn build_trace_stamp(build: &FoundryBuildStamp) -> String {
    format!("build:{}", short_hash(&build.build_fingerprint.0.to_hex()))
}

#[must_use]
pub(crate) fn document_asset_name(document: &FoundryAssetDocument) -> String {
    sanitize_trace_text(&document.document_id.0)
}

#[must_use]
pub(crate) fn request_trace_stamp(request: &FoundryJobRequest) -> Option<String> {
    match request {
        FoundryJobRequest::CompileCurrent { document, .. }
        | FoundryJobRequest::PreviewControlValue { document, .. }
        | FoundryJobRequest::GenerateCandidates { document, .. }
        | FoundryJobRequest::RenderCandidatePreviews { document, .. }
        | FoundryJobRequest::ApplyEdit { document, .. } => Some(document_trace_stamp(document)),
        FoundryJobRequest::RenderPreview { output, .. } => {
            Some(build_trace_stamp(&output.build_stamp))
        }
        FoundryJobRequest::CompilePack { .. }
        | FoundryJobRequest::ExportPack { .. }
        | FoundryJobRequest::Export { .. } => None,
    }
}

#[must_use]
pub(crate) fn request_asset_name(request: &FoundryJobRequest) -> Option<String> {
    match request {
        FoundryJobRequest::CompileCurrent { document, .. }
        | FoundryJobRequest::PreviewControlValue { document, .. }
        | FoundryJobRequest::GenerateCandidates { document, .. }
        | FoundryJobRequest::RenderCandidatePreviews { document, .. }
        | FoundryJobRequest::ApplyEdit { document, .. } => Some(document_asset_name(document)),
        FoundryJobRequest::RenderPreview { output, .. } => {
            Some(document_asset_name(&output.document))
        }
        FoundryJobRequest::CompilePack { .. }
        | FoundryJobRequest::ExportPack { .. }
        | FoundryJobRequest::Export { .. } => None,
    }
}

#[must_use]
pub(crate) fn trigger_action_for_request(request: &FoundryJobRequest) -> &'static str {
    match request {
        FoundryJobRequest::CompileCurrent { .. } => "RequestBuild",
        FoundryJobRequest::RenderPreview { .. } => "RequestPreview",
        FoundryJobRequest::PreviewControlValue { .. } => "PreviewControlValue",
        FoundryJobRequest::GenerateCandidates { .. } => "RequestCandidates",
        FoundryJobRequest::RenderCandidatePreviews { .. } => "RequestCandidatePreviews",
        FoundryJobRequest::ApplyEdit { .. } => "ApplyEdit",
        FoundryJobRequest::CompilePack { .. } => "CompilePack",
        FoundryJobRequest::ExportPack { .. } => "ExportPack",
        FoundryJobRequest::Export { .. } => "Export",
    }
}

#[must_use]
pub(crate) fn sanitize_trace_text(raw: &str) -> String {
    let without_user_roots = raw
        .replace('\\', "/")
        .split('/')
        .next_back()
        .unwrap_or(raw)
        .to_owned();
    without_user_roots
        .chars()
        .map(|ch| match ch {
            ':' | '"' | '\'' | '\n' | '\r' | '\t' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn first_event_ms(events: &[MakeJobTraceEvent], kind: MakeJobTraceEventKind) -> Option<u64> {
    events
        .iter()
        .find(|event| event.event_kind == kind)
        .map(|event| event.timestamp_relative_ms)
}

fn longest_span(
    events: &[MakeJobTraceEvent],
    starts: &[MakeJobTraceEventKind],
    ends: &[MakeJobTraceEventKind],
) -> Option<u64> {
    let mut active_start = None;
    let mut longest = None;
    for event in events {
        if starts.contains(&event.event_kind) && active_start.is_none() {
            active_start = Some(event.timestamp_relative_ms);
        }
        if ends.contains(&event.event_kind)
            && let Some(start) = active_start.take()
        {
            let duration = event.timestamp_relative_ms.saturating_sub(start);
            longest = Some(longest.unwrap_or(0).max(duration));
        }
    }
    longest
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(12).collect()
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    fs::write(path, bytes)
}
