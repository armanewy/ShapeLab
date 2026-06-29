# Make Job Lifecycle and Latency Audit

This audit instruments Make job lifecycle locally before any broader product work.
The trace is written by app runs to:

- `target/make-job-traces/make-job-trace.json`
- `target/make-job-traces/make-latency-summary.json`

The trace records event names, app-local job IDs, job slots, document/build fingerprints,
asset labels, queue depth, stale/cancel/reuse flags, and product-safe messages. It does
not record mesh payloads, rendered image bytes, or absolute user file paths.

## What Was Spending Time

The reviewed 540p dogfood video spent the user-visible time in Make preparation and
idea generation states, especially repeated `Preparing Sci-Fi Industrial Crate` and
candidate placeholder periods. The code audit shows these states map to three job
families:

- `CompileCurrent` / `ApplyEdit` for model build preparation.
- `RenderPreview` / `PreviewControlValue` for visible preview readiness.
- `GenerateCandidates` / `RenderCandidatePreviews` for idea text/cards and selectable previews.

The new trace separates those phases so a future dogfood run can say whether the user
was waiting on build, preview render, candidate compile, or candidate preview render.

## Duplicate Jobs

Current reducer behavior now records duplicate job suppression explicitly:

- Equivalent build requests reuse an active `CompileCurrent` job and emit `JobReused`.
- Equivalent preview requests reuse an active `RenderPreview` job and emit `JobReused`.
- Equivalent candidate requests reuse an active `GenerateCandidates` job and emit `JobReused`.
- Non-equivalent work in the same active slot emits `UserActionBlocked` rather than silently
  queuing ambiguous churn.

The deterministic dogfood trace hook recorded no duplicates:

- `duplicate_build_jobs`: 0
- `duplicate_preview_jobs`: 0
- `duplicate_candidate_jobs`: 0

## Stale Jobs

Stale results are now local trace events:

- Unknown or superseded job results emit `JobIgnoredAsStale`.
- User actions that intentionally invalidate active jobs emit `JobCanceled`.
- Summary counts include total canceled and ignored-as-stale jobs.

The deterministic dogfood trace hook recorded:

- `total_jobs_canceled`: 0
- `total_jobs_ignored_as_stale`: 0

## Top Latency Source

In the synthetic headless dogfood hook, the longest span was candidate generation:

- `longest_preparing_span_ms`: 220
- `longest_generating_span_ms`: 680

These are deterministic test-hook timestamps, not a human-video measurement. The release
app writes wall-clock `timestamp_relative_ms` values when run interactively, so the next
manual dogfood pass should use the same JSON files for real timing.

## Current Trace Hook Output

The short dogfood hook covers:

1. Start Sci-Fi Crate.
2. Build current model.
3. Render current preview.
4. Try whole-asset ideas.
5. Render candidate previews.
6. Focus Handles.
7. Try focused handle ideas.
8. Render focused candidate previews.
9. Add to Pack.

Latest hook summary:

```json
{
  "time_to_first_preview_ready_ms": 220,
  "time_to_first_candidate_request_ms": 300,
  "time_to_first_candidate_result_ms": 620,
  "time_to_first_selectable_candidate_ms": 940,
  "total_jobs_queued": 6,
  "total_jobs_canceled": 0,
  "total_jobs_ignored_as_stale": 0,
  "duplicate_build_jobs": 0,
  "duplicate_preview_jobs": 0,
  "duplicate_candidate_jobs": 0,
  "longest_preparing_span_ms": 220,
  "longest_generating_span_ms": 680
}
```

## What Must Be Fixed Before Product Work Continues

Prompt 1 does not claim the product is fixed. It adds the instrumentation needed to prove
or disprove progress in Prompt 2.

Before expanding to UV/Texturing/Rigging UI, the app still needs:

- A visible last-known-good or thumbnail state while build/preview work runs.
- Candidate skeletons and text cards that appear immediately after Try ideas.
- A visible stale/cancel policy that is not only bottom status text.
- A manual Product Dogfood Gate v4 video proving the user can stay in Shape Lab without
  ambiguous waits.

## Prompt 2 Follow-Up

`codex/make-preparation-candidate-responsiveness` addresses the first three items above:

- a deterministic quick template preview is visible before first full build;
- last-known-good preview remains visible while replacement preview work runs;
- idea generation timeout shows `Still trying ideas.` with `Cancel` and `Keep waiting`;
- canceling active ideas records `JobCanceled` and shows `Canceled earlier idea search.`
  in Make.

The remaining proof is manual: Product Dogfood Gate v4 must review a release-app video before
the project can claim Make is dogfood-acceptable.
