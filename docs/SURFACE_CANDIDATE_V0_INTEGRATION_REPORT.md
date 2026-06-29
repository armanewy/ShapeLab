# Surface Candidate v0 Integration Report

Date: 2026-06-29

## Verdict

`PASS - SCI-FI CRATE MAKE BASELINE + MATERIAL-LOOK PREVIEW BASELINE`

The integration branch merges:

1. Make latency follow-up.
2. Headless Sci-Fi Crate surface candidate evidence.
3. Sci-Fi Crate visual material-look candidates.

The result is ready to merge only for the narrow Sci-Fi Crate baseline. It does
not approve broad Surface mode, material editing, general UV/texturing, rigging,
animation, engine-native export, or full game-ready status.

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-render surface --jobs 1` | Pass |
| `cargo test -p shape-gamekit surface --jobs 1` | Pass |
| `cargo test -p shape-cli game_ready_static --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Video Evidence

Shape baseline video:

- `target/product-dogfood-v4/product-dogfood-v4-full.mov`
  - SHA-256: `06f76d3d2ea20b9e245ae74df08824908cce0f78c5c8b7ff5ac9d491796e808a`
- `target/product-dogfood-v4/product-dogfood-v4-540p.mov`
  - SHA-256: `ebd241f592b90f280d2e85efa8e8ba8f19da0fb005df80a400702c993c0b6888`

Material-look integration video:

- `target/surface-candidate-v0-integration/material-looks.mov`
  - SHA-256: `e9306861f012c60f005262e2b9b95362fb3d6389aedaf37037a36c79b5517934`

## Screenshot Evidence

- `target/surface-candidate-v0-integration/material-looks-review.png`
  - SHA-256: `f27fc0a551a36bca88414997ed97a97c0084c338ee5272bac7f2f24fad0d2167`
- `target/surface-candidate-v0-integration/material-looks-export.png`
  - SHA-256: `b4f5fb2bd5b9846f7e0ad479d123a8429fe93c8f29855b8f68cedc38ccd34f23`
- `target/product-dogfood-v4/screenshots/`
  - Existing Product Dogfood Gate v4 screenshot set for the shape baseline.

## Latency Summary

Integration uses the Make latency follow-up trace at:

`target/make-latency-followup-v4/make-latency-summary.json`

SHA-256: `9699c272adf19b98c2d2ed6e543f6c4dd359c56ea734e7092cc2d3ff6d06d17e`

| Metric | Value |
| --- | ---: |
| `time_to_first_visible_model_ms` | 0 |
| `time_to_first_preview_ready_ms` | 220 |
| `time_to_first_skeleton_idea_tray_ms` | 300 |
| `time_to_first_candidate_shell_ms` | 620 |
| `time_to_first_candidate_preview_ms` | 940 |
| `time_to_first_selectable_candidate_ms` | 940 |
| `total_jobs_queued` | 6 |
| `total_jobs_ignored_as_stale` | 0 |
| `longest_preparing_span_ms` | 220 |
| `longest_generating_span_ms` | 680 |

## Pass/Fail Table

| Requirement | Result | Notes |
| --- | --- | --- |
| Shape baseline still passes Product Dogfood Gate v4 flow | Pass | Existing full/540p v4 videos and screenshot set remain the canonical shape baseline evidence. |
| Shape baseline latency does not regress | Pass | Follow-up trace improves first visible model, preview-ready, skeleton tray, and first selectable candidate timings. |
| Material previews are visibly textured | Pass | Integration material video and screenshots show distinct preview thumbnails. |
| Material changes are distinguishable | Pass | Clean Lab White, Worn Hazard Yellow, and Dark Industrial Metal are visible and visually different. |
| Geometry unchanged | Pass | UI states `Geometry unchanged`; tests verify material selection does not mutate geometry/control state. |
| Surface scope is clearly narrow | Pass | UI labels the section `Material looks` and `Surface only`; docs prohibit broad Surface claims. |
| No full game-ready claim | Pass | Export states full game-ready remains blocked pending manual review and engine import proof. |
| No rigging/animation claim | Pass | Product copy and docs keep rigging and animation blocked. |
| Export copy truthful | Pass | Export states material looks are preview-only and do not affect export. |

## Decision

Merge recommendation: `GO` for the narrow Sci-Fi Crate material-look preview
baseline.

Go/no-go for broader Surface work: `NO-GO`. Broader Surface/texturing should not
begin until at least a second profile passes dogfood with comparable visual
evidence and without export or game-ready overclaim.

## Recommended Next Steps

1. Make visual polish.
2. Sci-Fi Crate material persistence/export inclusion, because material looks
   are preview-only in this build.
3. Stylized Lamp product dogfood pass.
4. Roman Bridge pass or continued `PreviewOnly` decision.
5. Broader texturing only after a second profile passes.

Do not start rigging or animation UI from this result.
