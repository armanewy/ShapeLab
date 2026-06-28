# Product Recovery Integration v2 Report

Date: 2026-06-28

## Status

`SUPERSEDED - AUTOMATED PROMPT 5 EVIDENCE PASSED; LATEST HUMAN DOGFOOD NO-GO`

This report originally covered the Prompt 5 integration gate for the previous
recovery prompt set. It is now superseded by
[`CURRENT_PRODUCT_STATUS.md`](CURRENT_PRODUCT_STATUS.md).

Current truth: automated gates and screenshot/video capture can pass, but the
latest human review of the 540p dogfood video still identifies Make UX blockers.
Treat `main` as a product-recovery baseline, not as a stable Visual Foundry
baseline.

The integration branch is:

```text
codex/product-recovery-integration-v2
```

## Merged Branches

Merged in the required order:

1. `codex/mainline-recovery-audit-code-hygiene`
2. `codex/make-canvas-interaction-recovery-v2`
3. `codex/candidate-legibility-engine-v2`
4. `codex/harden-scifi-crate-template-v2`
5. `codex/harden-roman-bridge-template-v2`
6. `codex/harden-stylized-lamp-template-v2`
7. `codex/starter-template-dogfood-benchmark-v2`

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
| `cargo test -p shape-cli starter_template_dogfood --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

After the final screenshot fixture hook change, these affected gates were rerun:

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p shape-app screenshot --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Starter Template Benchmark

Command:

```bash
cargo run -p shape-cli -- starter-template-dogfood-benchmark --out-dir target/starter-template-dogfood
```

Output:

```text
target/starter-template-dogfood/
```

Root summary:

- `all_passed = true`
- `required_visible_ideas = 4`
- `benchmark_preview_resolution_px = 512`
- `app_decision_preview_resolution_px = 512`
- `evidence_matches_actual_app_camera_scale = true`
- `manual_review_required_for_showcase = true`

| Template | Result | Catalog Recommendation | Blockers |
| --- | --- | --- | --- |
| Sci-Fi Industrial Crate | Pass | Usable | None |
| Roman Timber Bridge HQ | Pass | PreviewOnly | Six-direction Usable gate is not met and no exception is approved. |
| Stylized Furniture Lamp | Pass | Usable | None |

## Screenshot And Video Gate

Release app launched from:

```text
target/release/Shape Lab.app
```

Screenshot output:

```text
target/product-recovery-integration-v2/screenshots/
```

Assertion output:

```text
target/product-recovery-integration-v2/assertions/
```

Video output:

```text
target/product-recovery-integration-v2/product-recovery-dogfood-video.mov
```

Video file check:

```text
ISO Media, Apple QuickTime movie, Apple QuickTime (.MOV/QT)
size: 144M
```

Captured screenshots:

| File | State |
| --- | --- |
| `01_choose.png` | Choose screen |
| `02_scifi_make_ready.png` | Sci-Fi Crate Make ready |
| `03_scifi_generating_ideas.png` | Sci-Fi Crate whole-asset ideas running |
| `04_scifi_generated_ideas.png` | Sci-Fi Crate generated ideas |
| `05_scifi_selected_comparison.png` | Sci-Fi Crate selected comparison |
| `06_scifi_focus_handles.png` | Sci-Fi Crate Handles focused |
| `07_scifi_generating_handle_ideas.png` | Sci-Fi Crate handle ideas running |
| `08_scifi_handle_ideas.png` | Sci-Fi Crate handle ideas |
| `09_scifi_focus_vents.png` | Sci-Fi Crate Vents focused |
| `10_scifi_pack_drawer.png` | Pack drawer |
| `11_scifi_export_drawer.png` | Export drawer |
| `12_bridge_make_ready.png` | Roman Timber Bridge Make ready |
| `13_bridge_generated_ideas.png` | Roman Timber Bridge generated ideas |
| `14_bridge_selected_comparison.png` | Roman Timber Bridge selected comparison |
| `15_lamp_make_ready.png` | Stylized Lamp Make ready |
| `16_lamp_generated_ideas.png` | Stylized Lamp generated ideas |
| `17_lamp_selected_comparison.png` | Stylized Lamp selected comparison |

Screenshot sanity passed: all required files exist, every screenshot is
2940x1912, and no adjacent captures are byte-identical. The final capture hid
the macOS Dock during recording and restored the previous Dock setting
afterward.

## Superseded Human Product Gate

The table below records the original Prompt 5 self-review. It must not be used
as the current product-stability verdict. The later human video audit found that
Make can still stall or dead-end around preparation, disabled actions, no clear
ideas, focused-part failures, and bottom-status-dependent feedback.

| Review Item | Result | Evidence |
| --- | --- | --- |
| I know what to do next on Make. | Pass | Make ready states show the asset name, ready badge, primary "Try ideas" action, and next-action hint. |
| Buttons look clickable. | Pass | Primary and secondary actions render as button controls with clear enabled/disabled states. |
| Running actions are visible locally. | Pass | Generating states show local busy mode and skeleton tray evidence. |
| I can tell what changed in at least four crate ideas. | Pass | Benchmark passes; generated screenshot shows current/candidate comparison and "What changed" copy. |
| I can tell what changed in at least four bridge ideas. | Pass | Benchmark passes; Bridge generated screenshot shows comparison and change summary. |
| I can tell what changed in at least four lamp ideas. | Pass | Benchmark passes; Lamp generated screenshot shows comparison and change summary. |
| Focus Part visibly changes the workspace. | Pass | Handles and Vents focus screenshots show focused mode, scoped actions, and part chips. |
| Candidate comparison is readable. | Pass | Generated and selected-comparison screenshots show current/candidate previews, change text, affected parts, and action buttons. |
| Pack/Export are visible drawers. | Pass | Pack and Export screenshots show right-side drawers with readiness and blocked-pack state. |
| Surface/rig/motion/game-ready are not overclaimed. | Pass | Screenshots and README keep the product loop scoped to Visual Foundry assets and canonical export. |
| No technical/internal terms appear. | Pass | Make flow no longer exposes novice-facing Build Asset or Refresh Preview internals. |

## Adversarial Critic Notes

- The Make canvas now launches from template start into an asset workspace
  instead of exposing build/preview sequencing as the novice path.
- Local running states are visible in the Make workspace, not only in the
  bottom status strip.
- Generated idea evidence is now tied to the same app-scale 512 px candidate
  preview path used by the decision UI.
- All three starter templates pass the v2 dogfood benchmark with app-scale
  preview evidence and no hidden raw-term requirement.
- Pack and Export drawers are visible as workspace drawers and communicate
  readiness or blocked pack export state.

## Current Remaining Blockers

- Make pipeline reliability and no-dead-end handling are still blocking product
  stability.
- Focused-part generation needs an explicit recovery path when no clear ideas
  survive.
- Evidence reports must stay reproducible and tied to hashes, commit, platform,
  and reviewer identity.
- Roman Bridge HQ remains PreviewOnly until the six-direction Usable gate passes
  or an explicit exception is approved.

Remaining non-blocking caveats:

- Human review is still required before any Showcase claim.
- Candidate thumbnails in the tray can still be made larger in a future UX pass.
- macOS public distribution still needs signing, notarization, archive
  validation, and release packaging checks.
- Broad UV/texturing, rigging, animation, and game-ready editing remain outside
  the product-supported UI surface.

## Merge Recommendation

Historical note: this branch was merged to `main` after the Prompt 5 evidence
run. Current recommendation: do not treat that merge as a stable product
baseline. Continue recovery work from `main`, and do not start larger
user-facing UV/Texturing/Rigging/Animation integration until Make passes the new
manual gate.
