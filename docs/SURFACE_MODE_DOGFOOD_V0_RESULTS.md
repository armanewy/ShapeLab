# Surface Mode Dogfood v0 Results

Date: 2026-06-29

## Verdict

`PASS - PREVIEW-ONLY MATERIAL LOOK BASELINE`

Sci-Fi Crate material-look previews are implemented as a narrow, preview-only
Make UI path. The branch does not claim broad Surface mode, material editing,
general texturing, rigging, animation, or full game-ready export.

The release app was recorded and visually reviewed for this branch. The flow is
acceptable for the narrow Sci-Fi Crate material-look preview baseline only.

## Scenario

Required manual scenario:

1. Launch the release app.
2. Choose Sci-Fi Industrial Crate.
3. Wait for Make ready.
4. Use the existing crate or Try ideas.
5. Click `Try material looks`.
6. Compare at least three material candidates.
7. Select or preview one material candidate.
8. Open Export.
9. Confirm export copy is truthful.

## Current Evidence

Automated evidence:

- `cargo fmt --all --check`: pass
- `cargo test -p shape-app foundry --jobs 1`: pass
- `cargo test -p shape-render surface --jobs 1`: pass
- `cargo test -p shape-gamekit surface --jobs 1`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo build --release --workspace`: pass

Release-app video and screenshots:

- `target/surface-mode-dogfood-v0/surface-mode-dogfood-v0.mov`
  - SHA-256: `d9c9363820b4c578dcff118087848d24e575c4a1cca4a9a940ec3b84aba08e02`
- `target/surface-mode-dogfood-v0/surface-mode-dogfood-v0-material-tray.png`
  - SHA-256: `a5cb1345ad5b28b3762c6304cf85f8ccd8a29f0004073c142635b49df40de6ec`
- `target/surface-mode-dogfood-v0/surface-mode-dogfood-v0-export.png`
  - SHA-256: `f2c33bc3637f1b2b0a441137fbeb27ffda5946a570dce10a2ed333b8163c8563`

Implemented UI evidence:

- secondary Make action: `Try material looks`
- material tray title: `Material looks`
- material-only labels: `Surface only`, `Geometry unchanged`
- comparison labels: `Current Material`, `Candidate Material`
- approved six candidate titles
- preview-only export copy
- full game-ready blocked copy

## Pass Criteria Status

| Criterion | Status |
| --- | --- |
| Material candidate previews are visibly textured | Pass in release-app review |
| Geometry does not change | Pass in automated state tests and visual review |
| User can tell material mode from shape mode | Pass in copy/UI structure tests |
| No broad texturing claim | Pass in copy tests |
| No game-ready claim | Pass in copy tests |
| Export behavior is honest | Pass in copy/state tests and export screenshot |
| Flow remains understandable | Pass for narrow preview-only baseline |

## Review Notes

The reviewed app flow shows:

- `Try material looks` remains secondary to shape ideas.
- The material section uses `Surface only` and `Geometry unchanged`.
- The compact review area shows Current Material vs Candidate Material.
- At least three textured material candidates are visible.
- Export states that material looks are preview-only and will not affect export.
- Export states that full game-ready remains blocked until manual review and
  engine import proof.

Remaining blocker: material looks are not persistent and are not included in
export in this build.
