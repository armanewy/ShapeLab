# Make Preparation and Candidate Responsiveness Fix

This branch builds on `codex/make-job-lifecycle-latency-audit`.

## Implemented

- Make keeps the last rendered preview visible while replacement preview work is active.
- Template start now has a deterministic quick preview marker while the first full build is
  still preparing.
- Equivalent build, preview, and candidate jobs are reused and traced as `JobReused`.
- Active idea searches can be canceled explicitly with `Cancel`.
- Canceling an idea search records `JobCanceled`, clears pending candidate state, and shows
  `Canceled earlier idea search.` locally in Make.
- If idea generation exceeds the local budget, Make shows `Still trying ideas.` with
  `Cancel` and `Keep waiting`.
- Candidate skeletons continue to appear immediately while generation or preview rendering
  is active.
- `Use this idea` remains disabled until the selected candidate has rendered preview evidence.
- Novice Make does not show `Build Asset` or `Refresh Preview`; `Update preview` is retained
  only when a stale/missing preview can be recovered locally.

## Automated Evidence

The Prompt 1 hook still writes:

- `target/make-job-traces/make-job-trace.json`
- `target/make-job-traces/make-latency-summary.json`

Focused tests cover:

- quick template preview visible before first build;
- previous preview visible while a newer preview is required;
- equivalent build/preview/candidate job reuse;
- cancellation of active idea generation;
- local cancellation warning, not only bottom status;
- generation timeout recovery actions;
- skeleton candidate tray while ideas are generating;
- candidate acceptance disabled until preview evidence exists.

## Dogfood Gate Status

Automated checks passed, but this branch does not claim a human product pass.
The release app still needs a recorded manual dogfood clip for Sci-Fi Crate before
Product Dogfood Gate v4 can pass.

Current status: `NEEDS HUMAN VIDEO REVIEW`.
