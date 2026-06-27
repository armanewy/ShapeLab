# Product Quality Recovery Integration Report

## Status

`PASS WITH PACKAGING FOLLOW-UP`

The integration branch merges the Make recovery, candidate legibility, starter
template hardening, and starter-template benchmark work. Automated Rust gates
pass, the starter-template benchmark emits all required evidence, and the
mandatory Make Canvas screenshot gate now captures and passes image sanity
checks.

Retry note: the raw `target/release/shape-app` process still does not expose a
normal macOS bundle identity to Computer Use. A temporary `.app` wrapper under
`target/visual-retry/Shape Lab.app` made the release binary visible to
LaunchServices, after which window-id `screencapture` captured the visual gate.
Before external release packaging, Shape Lab should ship a first-class macOS app
bundle instead of relying on a raw binary process.

## Branches Merged

- `codex/make-canvas-interaction-recovery`
- `codex/candidate-legibility-engine-v2`
- `codex/harden-scifi-crate-template`
- `codex/harden-roman-bridge-template`
- `codex/harden-stylized-lamp-template`
- `codex/starter-template-quality-benchmark`

Integration note: app home-screen tests were updated after Prompt 6 because
`roman-bridge-hq` is now intentionally `PreviewOnly`, so the default novice
home catalog contains two usable starters: Sci-Fi Crate and Stylized Lamp.

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

Result: command passed; benchmark summary `all_passed = false` because Roman
Bridge HQ is correctly recommended as `PreviewOnly`.

Output:

```text
target/starter-template-quality/
```

| Template | Benchmark Result | Catalog Recommendation | Blocker |
| --- | --- | --- | --- |
| Sci-Fi Industrial Crate | Pass | Usable | None |
| Roman Timber Bridge HQ | Fail | PreviewOnly | Six parent/candidate outputs failed conformance, model validation, or the visible disconnected-part check. |
| Stylized Furniture Lamp | Pass | Usable | None |

Each template directory contains:

- `parent.png`
- `generated-ideas-contact-sheet.png`
- `selected-comparison-sheet.png`
- `control-endpoint-sheet.png`
- `option-gallery-sheet.png`
- `legibility-report.json`
- `adversarial-review.md`

## Visual Gate

Result: `PASS`

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
files exist, and the required state-transition screenshot hashes differ.

Retry fixes made after the first visual attempt:

- Focused Handles idea generation no longer stalls on `NoEditableControls`;
  focused `Refine` requests may use topology-changing controls when the scope
  is a concrete focused part group.
- The Make Canvas layout now gives the candidate tray enough room for readable
  selected comparisons and prevents control labels from colliding with
  Focus/Lock/Reset actions in drawers.

## Pass/Fail Table

| Review Item | Result | Notes |
| --- | --- | --- |
| I know what to do next on Make. | Pass | Screenshots show Choose, Make, Try ideas, focused part actions, and drawer actions. |
| Buttons look clickable. | Pass | Primary and secondary actions render as filled buttons; disabled actions show muted state. |
| Running actions are visible locally. | Pass | Generating whole-asset and Handles states show local busy treatment. |
| I can tell what changed in at least four crate ideas. | Pass | Benchmark passes Sci-Fi Crate as Usable. |
| I can tell what changed in at least four bridge ideas. | PreviewOnly | Roman Bridge HQ remains hidden from the default novice catalog due disconnected-part validation. |
| I can tell what changed in at least four lamp ideas. | Pass | Benchmark passes Stylized Lamp as Usable. |
| Focus Part visibly changes the workspace. | Pass | Handles and Vents focus screenshots show focused chip/callout and local tray copy. |
| Candidate comparison is readable. | Pass | Selected comparison screenshot shows current vs candidate previews and visible change copy. |
| Pack/export are visible drawers. | Pass | Pack and Export screenshots show right-side drawers with ready states. |
| Surface/rig/motion/game-ready are not overclaimed. | Pass | Automated copy gates pass; visual screenshots do not expose unsupported surface/rig/motion/game-ready claims. |
| No technical/internal terms appear. | Pass | Automated copy gates pass; reviewed screenshots use product-facing labels. |

## Adversarial Notes

- Make canvas: screenshot capture now passes. The retry exposed and fixed a
  focused Handles generation stall plus two layout issues in the comparison tray
  and drawer control rows.
- Sci-Fi Crate: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.
- Roman Bridge HQ: authored hardening tests pass, but the new starter benchmark
  keeps it out of the default novice catalog because the visible
  disconnected-part gate fails generated outputs.
- Stylized Lamp: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.

## Recommendation

Mergeable as a product-quality recovery candidate, with one packaging follow-up.

Keep Roman Bridge HQ out of the default novice catalog until it passes the
starter-template benchmark. Before external release packaging, add a real macOS
app bundle so the release app has a stable LaunchServices/Computer Use identity.

Current release-readiness caveats:

1. The screenshot gate used a temporary `target/visual-retry/Shape Lab.app`
   bundle, not a checked-in packaging artifact.
2. Roman Bridge HQ remains `PreviewOnly`, which is intentional for this
   integration branch.
