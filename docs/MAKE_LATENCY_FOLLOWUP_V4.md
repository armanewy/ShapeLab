# Make Latency Follow-Up v4

Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only, with
latency caveats that remain product follow-up work.

## Recorded v4 Timings

The release-app dogfood run recorded:

| Metric | Value |
| --- | ---: |
| First visible model or fallback | immediate local quick preview |
| `time_to_first_build_started_ms` | 0 ms |
| `time_to_first_build_finished_ms` | 403 ms |
| `time_to_first_preview_ready_ms` | 7290 ms |
| `time_to_first_candidate_request_ms` | 8255 ms |
| `time_to_first_candidate_result_ms` | 8676 ms |
| `time_to_first_selectable_candidate_ms` | 11260 ms |
| `total_jobs_ignored_as_stale` | 0 |
| `longest_preparing_span_ms` | 7290 ms |
| `longest_generating_span_ms` | 3005 ms |

Budgets missed:

- preview-ready target: 5000 ms; recorded: 7290 ms
- first selectable whole-asset idea target: 10000 ms; recorded: 11260 ms

Budgets met:

- first visible model or credible placeholder under 3000 ms
- no ambiguous Preparing state over 12000 ms
- no stale ignored result warning in the normal dogfood path

## Bottleneck Read

The first compile finished quickly at 403 ms. The measured wait is mostly full
preview rendering and UI readiness after the initial build. The first candidate
plans compiled in roughly 421 ms after the Try ideas request, but candidate
preview rendering added roughly 2584 ms before the first selectable idea.

That means the immediate product risk is perceived waiting, not candidate search
scoring or mesh compilation. The user needs useful local feedback while full
preview and candidate preview rendering continue.

## Implemented Follow-Up

- The trace summary now records first visible model, skeleton tray, candidate
  shell, candidate preview, selectable candidate, reused job count, and coalesced
  job count.
- Equivalent build, preview, and candidate requests continue to reuse running
  work and record `JobReused`.
- Candidate skeletons remain the pre-compile state.
- Candidate shells now show after candidate plans compile, even while previews
  are still rendering.
- Pending candidate cards stay unselectable until preview evidence exists.
- Rendered cards can be selected independently while other card previews remain
  pending.
- The deterministic dogfood hook writes:
  `target/make-latency-followup-v4/make-job-trace.json` and
  `target/make-latency-followup-v4/make-latency-summary.json`.

## Status

This follow-up improves the measured and reviewable trace surface and reduces
the hidden wait between candidate compilation and candidate preview readiness.
It does not claim that full preview-ready or first selectable idea latency is
solved. If the release-app numbers still miss 5000 ms or 10000 ms, the next
blocker is the full preview/candidate preview render path, not product scope.
