# Mainline Make Canvas Failure Audit

## Status

Current `main` is an unstable product-recovery baseline. Automated Rust tests,
headless starter-template benchmarks, and screenshot file/hash checks pass, but
the latest human video audit still finds the Make flow confusing. This document
is the baseline audit for the next recovery wave; it is not a success report.

## Source Hygiene

`crates/shape-app/src/foundry/app.rs` is no longer physically collapsed in the
local checkout audited for this pass. It has 6,797 physical lines and no lines
longer than 180 characters. The earlier "21 physical lines" observation appears
to describe stale remote rendering or an older artifact.

The file is still too large to be a comfortable UX iteration surface. Make
Canvas state, layout, model stage, inspector, candidate tray, drawers, and
actions are still hosted together in `app.rs`. Prompt 0 does not redesign or
move behavior, but the next Make recovery branch should split this into focused
modules after tests are in place.

## Current User-Facing Make Flow

The default product path is Choose -> Make:

- user starts a template from Choose;
- Make opens with a model stage, right inspector, and bottom candidate tray;
- primary action is `Try ideas`, or a part-specific variant such as `Try handle ideas`;
- part chips can focus semantic groups such as Body, Panels, Vents, and Handles;
- candidates appear in the tray and selected comparison area;
- Pack and Export open right-side drawers.

The intended flow is reasonable, but the video audit shows the user still has
to infer too much about preparation, generation, stale jobs, and candidate
meaning.

## Visible Make Actions

Core visible actions include:

- `Try ideas` / `Trying ideas...`;
- part-chip focus buttons;
- `Try handle ideas`;
- `Lock handles`;
- `Clear focus`;
- `Use this idea`;
- `Reject`;
- `Add to Pack`;
- `Open Pack`;
- `Open Export`;
- pack drawer add/export actions;
- export drawer actions.

These use the app's action-button styling in code, but the human audit still
reports weak priority and ambiguity because too many actions share similar
visual weight and compete with contextual controls.

## Local Busy Feedback

`MakeCanvasViewState` tracks `local_busy_label` and `local_busy_visible`.
Current rendering uses:

- a model-stage status pill;
- a translucent busy overlay drawn over the model stage;
- skeleton candidate cards while generating;
- disabled reasons for build-dependent actions;
- local stale-result warning banners in the inspector and candidate tray.

This is better than the first recovery pass, but the human video still reports
that bottom status text does too much interpretive work. The next branch must
make local ownership of running work visually dominant enough that the bottom
strip is only secondary.

## Bottom-Status Reliance

Known risky states:

- stale or ignored background results are still originally sourced from status
  text and then mapped into a local warning;
- readiness still appears in the bottom strip as well as local state;
- screenshot tests only prove that specific state flags are set, not that the
  visible hierarchy makes those states obvious to a novice.

The next recovery branch should add stronger view-state fields such as
`next_action_hint`, `model_stage_mode`, `candidate_stage_mode`,
`has_blocking_overlay`, and `disabled_conflict_reason`.

## Build/Preview Internals

The intended novice path should hide direct build/preview sequencing. Current
source has auto-preparation logic and build-dependent disabled reasons, but the
latest video audit reports user-facing copy such as "Build the current asset to
render a preview" and adjacent build/preview/generation choices. That is a
product failure if still visible in the default path.

Prompt 1 must remove novice-facing Build Asset and Refresh Preview from Make,
or hide them behind a debug/developer affordance.

## Generated Ideas

The current screenshot scenario asserts `candidate_tray_visible` and
`selected_comparison_visible`, and the image sanity script checks dimensions and
hash differences. That proves files exist and change. It does not prove:

- generated ideas are understandable;
- candidate cards show differences at app decision size;
- comparison is visually dominant;
- the user knows which candidate is selected;
- rejected/too-similar ideas are explained in a product-facing way.

The candidate legibility benchmark must be reconciled with the actual app card
scale and camera context.

## Focus Part

Focus state currently changes the active chip, title, primary action, focused
part tray, filtered controls, and model-stage callout. The video audit still
reports that it feels like a filter rather than direct model selection.

Prompt 1 should make focus own the model stage more strongly: clearer callout,
focused-part affordance, contextual action row, and immediately visible focused
candidate comparison.

## Pack And Export

Pack and Export drawers visibly open in screenshot scenarios. That is not
enough for dogfood readiness. The human gate must also prove:

- users can tell what was added to pack;
- current asset readiness is clear;
- export limitations are plain English;
- Sci-Fi surface package copy does not imply unsupported full game-readiness.

## Stale Or Contradictory Docs

Resolved or partially resolved in this Prompt 0 pass:

- `MAKE_CANVAS_SCREENSHOT_GATE_RESULTS.md` said BLOCKED while the integration
  report said visual PASS;
- README dogfood steps still pointed users to Directions and Customize instead
  of Choose -> Make;
- product-quality reporting overclaimed human success from automated screenshot
  and benchmark evidence.

Remaining policy:

- automated screenshots and benchmarks are evidence, not a dogfood verdict;
- generated adversarial review from benchmark signals is not human review;
- current main must not be used as proof that Make UX is stable.

## Screenshot Gate Weakness

`crates/shape-app/tests/check_make_canvas_screenshots.sh` verifies:

- required files exist;
- dimensions meet a minimum;
- selected screenshot pairs have different hashes.

It does not verify visual dominance, readable candidate differences, button
clarity, local work ownership, or novice task completion. Prompt 1 should extend
the gate with state assertions and a human verdict section. Prompt 5 should
require a clean dogfood video for all three starters.

## Prompt 0 Verdict

Prompt 0 does not claim Make is fixed. It records the current failure shape and
removes contradictory documentation so Prompt 1 can make product changes
against an honest baseline.
