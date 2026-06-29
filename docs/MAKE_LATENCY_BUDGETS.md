# Make Latency Budgets

These are product budgets for Make dogfood runs. Prompt 1 records and reports them; later
branches may turn selected budgets into hard gates.

## Budgets

| Milestone | Target |
| --- | ---: |
| First visible model or deterministic template visual | under 3 seconds |
| First preview-ready Make state | under 5 seconds |
| Candidate placeholders after Try ideas | under 500 ms |
| First selectable Sci-Fi Crate idea | under 10 seconds |
| Ambiguous Preparing without retry/recovery | never longer than 12 seconds |

## Required Trace Fields

Use `target/make-job-traces/make-latency-summary.json` for the budget readout:

- `time_to_first_visible_model_ms`
- `time_to_first_build_started_ms`
- `time_to_first_build_finished_ms`
- `time_to_first_preview_ready_ms`
- `time_to_first_candidate_request_ms`
- `time_to_first_skeleton_idea_tray_ms`
- `time_to_first_candidate_result_ms`
- `time_to_first_candidate_shell_ms`
- `time_to_first_candidate_preview_ms`
- `time_to_first_selectable_candidate_ms`
- `reused_job_count`
- `coalesced_job_count`
- `longest_preparing_span_ms`
- `longest_generating_span_ms`

## Interpretation

Passing automated scenario assertions is not enough. A Make run is product-acceptable only
when the user sees local progress, always has a next action, and does not need to switch
away to wait through ambiguous preparation.

If a budget is missed, the UI must show a local recovery state:

- Preparation: `Still preparing`, `Retry preparation`, `Choose another template`.
- Idea generation: `Still trying ideas`, `Cancel`, `Keep waiting`.

Prompt 2 should use these budgets to remove blank or ambiguous waiting states.

## Current Implementation Notes

Prompt 2 adds local recovery for missed idea-generation budget:

- the Make banner says `Still trying ideas.`;
- `Cancel` explicitly cancels active candidate work and records `JobCanceled`;
- `Keep waiting` restarts the local timeout window without queuing duplicate work.

These are local product recovery states. They do not replace the Product Dogfood Gate v4
requirement for a human-reviewed video.

## Product Dogfood Gate v4 Follow-Up

The release-app v4 trace passed the narrow Sci-Fi Crate Make baseline but missed
two target budgets:

- first preview-ready state: 7290 ms versus the 5000 ms target;
- first selectable whole-asset idea: 11260 ms versus the 10000 ms target.

The follow-up trace also writes:

- `target/make-latency-followup-v4/make-job-trace.json`
- `target/make-latency-followup-v4/make-latency-summary.json`

Candidate readiness is split into skeleton tray, compiled shell, rendered
preview, and selectable candidate. A candidate card can be shown as a shell
before its preview is rendered, but `Use this idea` remains disabled until the
card has rendered preview evidence.
