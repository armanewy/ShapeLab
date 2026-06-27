# Product Quality Recovery Integration Report

## Status

`BLOCKED`

The integration branch merges the Make recovery, candidate legibility, starter
template hardening, and starter-template benchmark work. Automated Rust gates
pass, and the starter-template benchmark emits all required evidence. The
mandatory app screenshot/video gate is blocked because the release app process
does not expose a visible or Computer Use-addressable Shape Lab window.

Do not merge this branch to `main` until the visual gate can be captured and
reviewed.

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

Result: `BLOCKED`

Attempt:

- Built `target/release/shape-app`.
- Launched `./target/release/shape-app`.
- Computer Use `list_apps` did not list Shape Lab.
- Computer Use `get_app_state` failed for `Shape Lab`.
- Computer Use `get_app_state` failed for the full release executable path.

Screenshot/video output path: none. No valid Shape Lab window was available to
capture.

## Pass/Fail Table

| Review Item | Result | Notes |
| --- | --- | --- |
| I know what to do next on Make. | Blocked | Requires screenshots/video. |
| Buttons look clickable. | Blocked | Requires screenshots/video. |
| Running actions are visible locally. | Blocked | Requires screenshots/video. |
| I can tell what changed in at least four crate ideas. | Pass | Benchmark passes Sci-Fi Crate as Usable. |
| I can tell what changed in at least four bridge ideas. | Fail | Roman Bridge HQ remains PreviewOnly due disconnected-part validation. |
| I can tell what changed in at least four lamp ideas. | Pass | Benchmark passes Stylized Lamp as Usable. |
| Focus Part visibly changes the workspace. | Blocked | Requires screenshots/video. |
| Candidate comparison is readable. | Blocked | Requires screenshots/video. |
| Pack/export are visible drawers. | Blocked | Requires screenshots/video. |
| Surface/rig/motion/game-ready are not overclaimed. | Not visually reviewed | Automated/product copy gates passed, but screenshot review is blocked. |
| No technical/internal terms appear. | Not visually reviewed | Automated copy gates passed, but screenshot review is blocked. |

## Adversarial Notes

- Make canvas: code-level state tests pass, but dogfood-quality review is
  blocked until screenshots or video can be captured from a visible app window.
- Sci-Fi Crate: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.
- Roman Bridge HQ: authored hardening tests pass, but the new starter benchmark
  keeps it out of the default novice catalog because the visible
  disconnected-part gate fails generated outputs.
- Stylized Lamp: benchmark and adversarial report pass; candidate and endpoint
  evidence are usable for the novice catalog.

## Recommendation

Do not merge to `main`.

Keep `codex/product-quality-recovery-integration` as an integration branch with
passing automated gates, but treat release readiness as blocked until:

1. The release app creates a visible, Computer Use-addressable window.
2. The required Make screenshots/video are captured and manually reviewed.
3. Roman Bridge HQ either passes the starter-template benchmark or remains
   hidden from the default novice catalog.
