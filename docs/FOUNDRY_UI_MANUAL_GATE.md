# Foundry UI Manual Gate

This is the human screenshot gate for the redesigned Visual Foundry product UI.
It complements, but is not replaced by:

```bash
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate
```

The automated gate proves the headless shell contract. This manual gate checks
whether the app actually looks and behaves like a usable product.

Unified Make Canvas evidence is tracked in
[`docs/MAKE_CANVAS_SCREENSHOT_GATE.md`](MAKE_CANVAS_SCREENSHOT_GATE.md). That
gate is required whenever the Make canvas is changed.

## Setup

Run from a clean release build:

```bash
cargo run -p shape-app --release
```

Record the commit hash and platform. Do not open developer tools or technical
recipe surfaces. Do not use docs to infer what a control means.

## Required Evidence

Capture screenshots for every item below. Written notes are allowed only as
additional observations and cannot replace the required screenshots.

- Clean launch home screen.
- Profile selection from the ten Visual Foundry profiles.
- Roman Timber Bridge Make canvas with generated ideas.
- Sci-Fi Industrial Crate Make canvas with contextual controls.
- Stylized Furniture Lamp option gallery.
- Pack drawer.
- Export drawer.
- Focus Handles active on Sci-Fi Industrial Crate.
- Focused handle candidate generation and selected A/B comparison.
- Make canvas showing a focused control card.
- One disabled-control or disabled-action reason.
- One stale, building, or working status message.
- 1280x800 layout.
- 1440x900 layout.

## Pass Criteria

- No legacy labels are visible.
- Launch is not blank.
- Advanced Recipe is not visible or required.
- No raw technical strings are visible.
- The central whole-model preview or whole-model cards dominate the screen.
- A novice can identify the next action within five seconds.
- Roman bridge, sci-fi crate, and stylized lamp tasks complete without docs.
- Whole-model direction cards are understandable.
- Make controls are meaningful and not raw IDs.
- Disabled reasons are plain English.
- Pack and export drawers use plain English readiness.
- Part chips are semantic nouns only and show plain unavailable reasons.
- Selected candidates show larger parent/candidate comparison previews and an
  exact what-changed summary.
- Surface options say they need textured previews before they can be shown.

## Fail Criteria

- The user lands on a blank, history-first, or debug-looking screen.
- Any task requires Advanced Recipe or implementation terms.
- Direction cards lack visible whole-model previews or differences.
- Option thumbnails are missing or placeholder-like in the real app.
- Candidate accept/reject, lock/regenerate, pack, export, or reopen fails.
- The next action is not obvious after five seconds.

## Result Template

```text
Tester:
Date:
Commit:
Platform:

Launch home screenshot:
Profile selection screenshot:
Roman bridge Make canvas screenshot:
Sci-fi crate Make canvas screenshot:
Stylized lamp option gallery screenshot:
Pack drawer screenshot:
Export drawer screenshot:
Focus Handles screenshot:
Focused handle candidates screenshot:
Selected candidate comparison screenshot:
Focused Make control screenshot:
Disabled reason screenshot:
Status message screenshot:
1280x800 screenshot:
1440x900 screenshot:

No legacy labels: pass/fail
No blank launch: pass/fail
No Advanced Recipe needed: pass/fail
No raw technical strings: pass/fail
Dominant central preview/cards: pass/fail
Next action obvious within five seconds: pass/fail
Three core tasks complete without docs: pass/fail

Confusing labels:
Dead controls:
Invisible controls:
Layout problems:
Performance/stutter:
Notes:
```
