# Product Dogfood Gate v4 Results

Status: `PASS - SCI-FI CRATE BASELINE ONLY`

Product Dogfood Gate v4 was rerun from the release app with full video,
540p video, screenshots, and Make trace evidence. The recorded flow stayed in
Shape Lab, completed the required Sci-Fi Industrial Crate scenario, and provided
visible local progress and recovery states.

This is a narrow pass for the Sci-Fi Crate Make baseline. It is not approval for
broad Surface, UV/Texturing, Rigging, motion, or full game-ready product UI.

## Evidence Manifest

Path:

```text
target/product-dogfood-v4/evidence-manifest.json
```

The manifest is generated locally under `target/`, which is ignored by the
repository. It records the release binary, videos, screenshots, trace files,
hashes, latency summary, and reviewer notes.

## Available Evidence

| Artifact | Status |
| --- | --- |
| Release app binary | Present: `target/release/shape-app` |
| Full video | Present: `target/product-dogfood-v4/product-dogfood-v4-full.mov` |
| 540p video | Present: `target/product-dogfood-v4/product-dogfood-v4-540p.mov` |
| Required screenshot set | Present: `target/product-dogfood-v4/screenshots/` |
| Make job trace | Present: `target/make-job-traces/make-job-trace.json` |
| Make latency summary | Present: `target/make-job-traces/make-latency-summary.json` |
| Reviewer verdict | Pass for Sci-Fi Crate baseline only |

Full video hash:

```text
06f76d3d2ea20b9e245ae74df08824908cce0f78c5c8b7ff5ac9d491796e808a
```

540p video hash:

```text
ebd241f592b90f280d2e85efa8e8ba8f19da0fb005df80a400702c993c0b6888
```

Trace hash:

```text
a844aaa81a0e9fd7a44b6c11312b4e379d5e5e8aebabac2c731b64f8cbabe756
```

Latency summary hash:

```text
806fd50fe292b43bfc8497ca39fe8a8330a608622d78c74be134510ba40ad093
```

## Latency Summary

This summary comes from the recorded release-app run.

| Metric | Value |
| --- | ---: |
| `time_to_make_open_ms` | 0 |
| `time_to_first_build_started_ms` | 0 |
| `time_to_first_build_finished_ms` | 403 |
| `time_to_first_preview_ready_ms` | 7290 |
| `time_to_first_candidate_request_ms` | 8255 |
| `time_to_first_candidate_result_ms` | 8676 |
| `time_to_first_selectable_candidate_ms` | 11260 |
| `total_jobs_queued` | 10 |
| `total_jobs_canceled` | 0 |
| `total_jobs_ignored_as_stale` | 0 |
| `duplicate_build_jobs` | 0 |
| `duplicate_preview_jobs` | 0 |
| `duplicate_candidate_jobs` | 0 |
| `longest_preparing_span_ms` | 7290 |
| `longest_generating_span_ms` | 3005 |

## Pass/Fail Table

| Requirement | Result | Notes |
| --- | --- | --- |
| User can stay in the app the whole time | Pass | The video remains in Shape Lab for the recorded run. |
| First visual response appears within 3 seconds | Pass | `make_visible_initial.png` shows a local preparing visual immediately after Start. |
| No ambiguous `Preparing` over budget without recovery | Pass | Preparing stayed under the 12s ambiguous-state budget and had local progress copy. |
| Buttons clearly look clickable | Pass | Primary and recovery actions are visible buttons in the captured flow. |
| Running actions are locally visible | Pass | Idea generation shows local stage copy and candidate skeleton cards. |
| User always has a next action | Pass | Whole-asset, focused, pack, and export states expose next actions. |
| Generated ideas visibly appear | Pass | Whole-asset and handle ideas render candidate cards. |
| `Use this idea` enabled only when ready | Pass | The captured card has rendered comparison evidence before `Use this idea` is enabled. |
| Focused unavailable states provide recovery | Pass | Vents shows `Try whole-asset ideas`, `Clear focus`, and lock/unlock context. |
| Pack drawer visibly opens | Pass | `pack_drawer.png` shows one asset in pack and export readiness. |
| Export drawer visibly opens | Pass | `export_drawer.png` shows export-ready current asset state. |
| No Surface, rig, motion, or game-ready overclaim | Pass | The captured product flow stays scoped to current asset and pack export readiness. |
| No internal technical vocabulary | Pass | The primary captured flow uses product-facing copy rather than implementation status. |

## Latency Follow-Up

The gate passes because the user-visible flow remained understandable and local
progress was visible, but the trace still shows latency work to tighten:

| Budget | Result | Follow-up |
| --- | ---: | --- |
| First preview-ready state under 5 seconds | 7290 ms | Improve full preview readiness after the immediate visual state. |
| First selectable Sci-Fi Crate idea under 10 seconds | 11260 ms | Tighten candidate render latency; the visible skeleton flow prevented a dead wait. |
| No ambiguous Preparing over 12 seconds | 7290 ms | Within budget. |
| No stale-job churn | 0 stale, 0 canceled | Within budget. |

## Reviewer Verdict

`PASS - SCI-FI CRATE BASELINE ONLY`

Reviewer notes: Codex performed the release-app dogfood flow at the user's
request, captured full and 540p video, reviewed the screenshots, and found the
flow product-acceptable for the Sci-Fi Crate baseline. The pass is intentionally
narrow and keeps the latency follow-up above.

## Remaining Blockers

- Reduce first preview-ready latency toward the 5s target.
- Reduce first selectable whole-asset candidate latency toward the 10s target.
- Keep Roman Bridge HQ at `PreviewOnly` until its separate gate changes.
- Do not expose broader UV/Texturing/Rigging UI from this result.

## Go/No-Go

Next product work is `GO` only for the narrow next step:

```text
Sci-Fi Crate visual Surface candidates v0
```

Broader user-facing UV, Texturing, Rigging, motion, and full game-ready UI work
remain `NO-GO`.
