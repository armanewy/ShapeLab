# Product Quality Recovery Integration Report

## Status

`AUTOMATED GATES PASS; HUMAN DOGFOOD NO-GO`

The integration branch merges the Make recovery, candidate legibility, starter
template hardening, and starter-template benchmark work. Automated Rust gates
pass, the starter-template benchmark emits all required evidence, and the Make
Canvas screenshot automation can capture the required states.

This report is not a product-stability verdict. The latest human video audit
still finds the Make tab confusing and not dogfood-stable. Treat current `main`
as an unstable product-recovery baseline.

Retry note: the original visual gate used a temporary `.app` wrapper because
the raw `target/release/shape-app` process did not expose a normal macOS bundle
identity to Computer Use. The repository now includes
`scripts/package_macos_app.sh` and `packaging/macos/Info.plist`, which create
`target/release/Shape Lab.app` for local macOS smoke tests and screenshot
capture.

## Branches Merged

- `codex/make-canvas-interaction-recovery`
- `codex/candidate-legibility-engine-v2`
- `codex/harden-scifi-crate-template`
- `codex/harden-roman-bridge-template`
- `codex/harden-stylized-lamp-template`
- `codex/starter-template-quality-benchmark`

Integration note: app home-screen tests now expect three usable default novice
starters: Sci-Fi Crate, Roman Timber Bridge HQ, and Stylized Lamp.

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-app --test foundry_direction_board --jobs 1` | Pass |
| `cargo test -p shape-search foundry --jobs 1` | Pass |
| `cargo test -p shape-render foundry --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test roman_bridge --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test stylized_lamp --jobs 1` | Pass |
| `cargo test -p shape-cli starter_template_quality --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

Logs:

```text
target/product-quality-integration/logs/
```

## Starter Template Benchmark

Command:

```bash
cargo run -p shape-cli -- starter-template-quality-benchmark --out-dir target/starter-template-quality
```

Result: command passed; benchmark summary `all_passed = true`.

Output:

```text
target/starter-template-quality/
```

| Template | Benchmark Result | Catalog Recommendation | Blocker |
| --- | --- | --- | --- |
| Sci-Fi Industrial Crate | Pass | Usable | None |
| Roman Timber Bridge HQ | Pass | Usable | None |
| Stylized Furniture Lamp | Pass | Usable | None |

Each template directory contains:

- `parent.png`
- `generated-ideas-contact-sheet.png`
- `selected-comparison-sheet.png`
- `control-endpoint-sheet.png`
- `option-gallery-sheet.png`
- `legibility-report.json`
- `adversarial-review.md`

## Automated Visual Gate

Result: `AUTOMATED PASS; HUMAN DOGFOOD NO-GO`

Attempt:

- Built `target/release/shape-app`.
- Confirmed the raw binary registers with macOS as `shape-app`, but Computer
  Use cannot attach to it by app name or executable path.
- Wrapped the release binary in a temporary app bundle at
  `target/visual-retry/Shape Lab.app`.
- Computer Use `list_apps` then listed `Shape Lab` with bundle id
  `com.shapelab.visual-retry`; `get_app_state` remained blocked by pending
  local Screen Recording/Accessibility permission setup.
- Used macOS window-id `screencapture` against the visible Shape Lab window.
- Captured all eleven Make Canvas screenshots from the built-in screenshot
  scenario driver.
- Ran `bash crates/shape-app/tests/check_make_canvas_screenshots.sh target/visual-retry/make-canvas-screenshots`.

Screenshot output path:

```text
target/visual-retry/make-canvas-screenshots/
```

Captured files:

- `01_choose.png`
- `02_make_ready.png`
- `03_generating_ideas.png`
- `04_generated_ideas.png`
- `05_selected_comparison.png`
- `06_focus_handles.png`
- `07_generating_handle_ideas.png`
- `08_handle_ideas.png`
- `09_focus_vents.png`
- `10_pack_drawer.png`
- `11_export_drawer.png`

Image sanity result: pass. Every screenshot is at least 1000x700, all required
files exist, and the required state-transition screenshot hashes differ. This is
weaker than a human product gate and must not be treated as proof that the UI is
usable.

Retry fixes made after the first visual attempt:

- Focused Handles idea generation no longer stalls on `NoEditableControls`;
  focused `Refine` requests may use topology-changing controls when the scope
  is a concrete focused part group.
- The Make Canvas layout now gives the candidate tray enough room for readable
  selected comparisons and prevents control labels from colliding with
  Focus/Lock/Reset actions in drawers.

## Automated Evidence Table

| Review Item | Result | Notes |
| --- | --- | --- |
| I know what to do next on Make. | Human Fail | Latest video audit says the next action is still not obvious enough. |
| Buttons look clickable. | Human Fail | Core actions exist, but priority and behavior remain ambiguous in video. |
| Running actions are visible locally. | Human Fail | Local state exists, but status-strip interpretation still carries too much UX. |
| I can tell what changed in at least four crate ideas. | Pass | Benchmark passes Sci-Fi Crate as Usable. |
| I can tell what changed in at least four bridge ideas. | Pass | Benchmark passes Roman Bridge HQ as Usable. |
| I can tell what changed in at least four lamp ideas. | Pass | Benchmark passes Stylized Lamp as Usable. |
| Focus Part visibly changes the workspace. | Human Fail | State changes exist, but video says focus still feels like filtering rather than direct model selection. |
| Candidate comparison is readable. | Human Fail | Screenshot existence does not prove human-readable difference at decision size. |
| Pack/export are visible drawers. | Not Proven | Drawer screenshots exist, but full novice workflow clarity is not proven. |
| Surface/rig/motion/game-ready are not overclaimed. | Pass | Automated copy gates pass; visual screenshots do not expose unsupported surface/rig/motion/game-ready claims. |
| No technical/internal terms appear. | Pass | Automated copy gates pass; reviewed screenshots use product-facing labels. |

## Adversarial Notes

- Make canvas: screenshot capture now passes. The retry exposed and fixed a
  focused Handles generation stall plus two layout issues in the comparison tray
  and drawer control rows.
- Sci-Fi Crate: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.
- Roman Bridge HQ: starter benchmark and adversarial report pass; candidate and
  endpoint evidence are usable for the novice catalog.
- Stylized Lamp: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.

## Recommendation

Keep as an unstable integration checkpoint. Do not use this report to justify
new app-facing feature work. The next branch should be Make Canvas interaction
recovery v2, followed by candidate/template hardening and a human dogfood
integration gate.

Current release-distribution caveats:

1. macOS public distribution still needs signing, notarization, final icon
   packaging, and installer/archive validation.
