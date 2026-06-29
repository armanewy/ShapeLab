# Product Dogfood Gate v4

This is the human product gate for the Make recovery flow. It replaces weak
scenario-pass claims with a recorded release-app review.

Automated tests, trace files, screenshot existence, and benchmark logs are evidence
only. This gate passes only after a human reviewer watches the full dogfood video
and confirms the rubric below.

## Scenario

Run from a release build.

1. Launch the release app.
2. Choose `Sci-Fi Industrial Crate`.
3. Wait for Make ready.
4. Try ideas.
5. Use one idea.
6. Focus `Handles`.
7. Try handle ideas, or see clear focused-unavailable recovery.
8. Focus `Vents`.
9. Try vent ideas, or see clear focused-unavailable recovery.
10. Add to Pack.
11. Open Export.

Optional passes:

- Run a Stylized Lamp quick pass.
- Run a Roman Bridge quick pass only if it is visible in the novice catalog.

## Required Artifacts

Write the manifest to:

```text
target/product-dogfood-v4/evidence-manifest.json
```

Capture:

- full video
- 540p video
- `choose` screenshot
- `make_visible_initial` screenshot
- `make_ready` screenshot
- `generating_ideas` screenshot
- `ideas_ready` screenshot
- `selected_idea` screenshot
- `focus_handles` screenshot
- `handle_result_or_recovery` screenshot
- `focus_vents` screenshot
- `vents_result_or_recovery` screenshot
- `pack_drawer` screenshot
- `export_drawer` screenshot
- `target/make-job-traces/make-job-trace.json`
- `target/make-job-traces/make-latency-summary.json`

The evidence manifest must include:

- commit
- OS
- app binary
- screen size
- video path
- video hash
- trace path
- trace hash
- screenshots
- screenshot hashes
- latency summary
- pass/fail
- human reviewer notes

## Pass Rubric

Pass only if all are true:

- The user can stay in the app the whole time.
- First visual response appears within 3 seconds.
- No ambiguous `Preparing` state lasts over budget without recovery.
- Buttons clearly look clickable.
- Running actions are locally visible.
- The user always has a next action.
- Generated ideas visibly appear.
- `Use this idea` is enabled only when the candidate is ready.
- Focused unavailable states provide recovery.
- Pack drawer visibly opens.
- Export drawer visibly opens.
- The app does not overclaim Surface, rig, motion, or game-ready output.
- The app does not expose internal technical vocabulary.

## Fail Rubric

Fail if any are true:

- The user switches away because the app appears stuck.
- The user has to guess whether an action worked.
- The user sees a dead end with no next action.
- Only bottom status explains an important state.
- Screenshots or video contradict the scenario log.

## Review Rule

Do not mark this gate as passed without human video review.
