# Family Foundation Pivot Integration Report

## Merged Branches

Merged into `codex/family-foundation-pivot-integration` from `origin/main`, in order:

| Order | Branch | Source commit |
| --- | --- | --- |
| 1 | `origin/codex/source-hygiene-reality-gate` | `3abbeca7f4b8` |
| 2 | `origin/codex/family-foundation-pivot-decision` | `4961e8670fd8` |
| 3 | `origin/codex/simple-crate-primitive-family-v0` | `bd97148aed99` |
| 4 | `origin/codex/simple-crate-make-baseline` | `b3ea34fe3caa` |
| 5 | `origin/codex/utility-crate-family-v1` | `76432657f98d` |
| 6 | `origin/codex/scifi-crate-regression-repositioning` | `ed46bf749e45` |
| 7 | `origin/codex/dev-gate-policy-tightening` | `9a010f63a0f2` |

## Product Truth

The integrated product stance is:

- Simple Crate is the novice baseline proof and current family-authoring proof.
- Utility Crate is the next family-maturity rung after Simple Crate.
- Cargo Case remains the advanced equipment-case proof.
- Sci-Fi Crate remains an advanced regression/profile, not the flagship.
- Material looks remain narrow and preview-only unless a later persistence/export branch lands.
- Broad Surface mode, material editor, UV/Texturing UI, Rigging, Animation, and full game-ready product claims remain blocked.

The integration also rejects model-invalid generated candidates before they become selectable in search/app candidate flows.

## Automated Gates

Logs are under `target/family-foundation-pivot-integration/gates/`.

| Gate | Result | Log |
| --- | --- | --- |
| `cargo fmt --all --check` | PASS, rerun after final edits | `01-cargo-fmt-check.log` |
| `cargo test -p shape-foundry-catalog --test simple_crate --jobs 1` | PASS, 8 tests | `02-simple-crate-test.log` |
| `cargo test -p shape-foundry-catalog --test utility_crate --jobs 1` | PASS, 11 tests | `03-utility-crate-test.log` |
| `cargo test -p shape-foundry-catalog --test cargo_case --jobs 1` | PASS, 14 tests | `04-cargo-case-test.log` |
| `cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1` | PASS, 8 tests, rerun after validation hardening | `05-scifi-crate-test.log` |
| `cargo test -p shape-app foundry --jobs 1` | PASS before validation hardening; post-hardening rerun interrupted by operator request after multiple green binaries | `06-shape-app-foundry-test-before-model-validation.log`, `06-shape-app-foundry-test.log` |
| `cargo test -p shape-search foundry --jobs 1` | PASS after validation hardening, 9 tests | `07-shape-search-foundry-test.log` |
| `cargo test -p shape-render foundry --jobs 1` | PASS, 12 tests | `08-shape-render-foundry-test.log` |
| `cargo test -p shape-cli game_ready_static --jobs 1` | PASS | `09-shape-cli-game-ready-static-test.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS before validation hardening; not rerun after operator stop request | `10-cargo-clippy-workspace.log` |
| `cargo build --release --workspace` | PASS before validation hardening; not rerun after operator stop request | `11-cargo-build-release-workspace.log` |

## Dogfood Evidence

Artifact hashes are recorded in:

- `target/family-foundation-pivot-integration/artifact-hashes.sha256`

No native UI video was captured for this integration pass. The release binaries were no longer present after cleanup, disk space was tight, and the operator requested moving on before a rebuild/capture. Dogfood evidence is generated/headless and recorded through contact sheets, summaries, OBJ/package outputs, and hashes.

Key evidence:

| Area | Evidence | Hash |
| --- | --- | --- |
| Simple Crate ideas | `target/family-foundation-pivot-integration/dogfood/simple-crate/candidate-contact-sheet.png` | `09411c3649f2e8bd22fc754140771b667ba99fbe375615b754984c975a5f06f4` |
| Simple Crate controls | `target/family-foundation-pivot-integration/dogfood/simple-crate/control-endpoint-sheet.png` | `ba193dbc8878087de7b7a179bdfebe70da65d32873d4c95be5060ea655ba047a` |
| Simple Crate summary | `target/family-foundation-pivot-integration/dogfood/simple-crate/dogfood-summary.json` | `5e4c84f63143ab85039153af7cc9a8066f1c31f8c74f9d8a0db8d47dbda15ea6` |
| Utility Crate ideas | `target/family-foundation-pivot-integration/dogfood/utility-crate/candidate-contact-sheet.png` | `46f2e7b098988e9af457d00676207dcc8b92a47dd44249c715b4236904741526` |
| Utility vs Simple comparison | `target/family-foundation-pivot-integration/dogfood/utility-crate/comparison-simple-vs-utility.png` | `85d09a010b25340c10f6e707ad6040e8b236e5a2cc1cee83121b085443ac5995` |
| Utility Crate report | `target/family-foundation-pivot-integration/dogfood/utility-crate/quality-report.json` | `e822f43586ad4233add0b8c24669e852cc356d8e6a1aab5c3c5f286117d51d30` |
| Sci-Fi ideas | `target/family-foundation-pivot-integration/dogfood/starter-template-dogfood/sci-fi-crate/generated-ideas-contact-sheet.png` | `a93761886f675b1d149e4b011dfb400c4abe29f842f8449e0cce5331d87cd479` |
| Sci-Fi dogfood summary | `target/family-foundation-pivot-integration/dogfood/starter-template-dogfood/sci-fi-crate/dogfood-summary.json` | `db93a77b89a27a6c2cc5f7723b485495a6a47ed42f9dd6f7d896a52a2e148891` |
| Starter benchmark summary | `target/family-foundation-pivot-integration/dogfood/starter-template-dogfood/dogfood-summary.json` | `13620ac9d75c89d59bb1f5eb6407ce1ee42c683db64d3aae9b7cf7507afe5b6a` |

Dogfood observations:

- Simple Crate produced 6 visible, 6 distinct ideas, had 5 readable primary controls, exported cleanly, and the summary marks it faster/simpler than Sci-Fi Crate.
- Utility Crate produced 6 visible ideas with 5 distinct visible ideas, 7 readable primary controls, exported cleanly, and the summary marks it richer than Simple Crate while simpler than Cargo Case.
- Sci-Fi Crate remained advanced/regression evidence with 5 visible, 5 distinct ideas, but the starter benchmark still marked it FAIL because 1 output failed conformance/model validation/visible disconnected-part checks. The final integration hardened candidate selection to reject model-invalid outputs, but the benchmark was not rerun after that change.
- Material-look evidence remains preview-only and may become stale/disabled if geometry changes.

## Pass/Fail Table

| Criterion | Result | Notes |
| --- | --- | --- |
| Simple Crate is easier/faster than Sci-Fi Crate | PASS | Simple Crate dogfood summary records `faster_and_simpler_than_scifi_crate: true`. |
| Simple Crate ideas are visible | PASS | 6 visible, 6 distinct ideas. |
| Utility Crate feels richer than Simple Crate | PASS | Utility report records `richer_than_simple_crate: true`. |
| Sci-Fi remains non-regressed but not flagship | PARTIAL | Catalog/profile tests pass; starter dogfood summary still fails on one invalid output before the final validation hardening rerun. Docs no longer treat Sci-Fi as flagship. |
| No broad UV/Texturing/Rigging claim | PASS | Docs keep those areas blocked. |
| No full game-ready product claim | PASS | Docs keep full game-ready product support blocked. |
| Full required automated gate suite on final code | PARTIAL | Final format, catalog, search, render, and CLI gates passed. App gate was interrupted after green progress; clippy/release passed before final hardening only. |

## Next Decision

Allowed next work if this integration is accepted:

- Simple Crate material slots / surface evidence.
- Utility Crate product dogfood polish.
- Third Cargo Case profile.
- Sci-Fi material persistence only if still desired.

Still blocked:

- Broad Surface mode.
- Material editor.
- Rigging/skinning/animation UI.
- Broad archetype library.
- Broad UV/Texturing/Rigging/Animation product UI.
- Structural candidate mutations.

Because the final evidence is partial, the next integration owner should rerun `cargo test -p shape-app foundry --jobs 1`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --release --workspace`, and the starter-template dogfood benchmark before using this branch as a release gate.
