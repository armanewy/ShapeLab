# Object Intent + Handled Panel Integration Report

Date: 2026-06-30

## Verdict

`PAUSED_AFTER_UI_AND_PARTIAL_AUTOMATED_GATES`

This checkpoint records Object Intent Brief / Kernel Readiness classifier
integration evidence with the current app-visible starter set:

- Box Primitive
- Lidded Box
- Flat Panel Primitive
- Hinged Panel
- Handled Panel

Trimmed Box remains internal evidence only. No Door naming, open/close motion,
surface/material workflow, UV/texturing, rigging, animation, runtime LLM
integration, or game-ready package claim is active.

## Base

- Starting main hash: `1d51d4563e9d43d9301fc7518280920a636f817c`
- Integration branch: `codex/object-intent-handled-panel-integration`

## Scope Integrated

- Object Intent Brief and Kernel Readiness classify requests against existing
  kernels without exposing runtime LLM or mesh-authoring behavior.
- Box Primitive remains the box-like primitive baseline.
- Lidded Box remains Box Primitive plus one visible Lid Seam feature.
- Flat Panel Primitive remains the second primitive kernel proof.
- Hinged Panel remains Flat Panel Primitive plus one visible Hinge Edge feature.
- Handled Panel remains Hinged Panel plus one visible Handle / Knob feature.

## Fixes During Gate

- Handled Panel empty idea-tray copy now says
  "Try handled panel ideas when the panel is ready." It no longer falls through
  to Box Primitive copy.
- Box-like current previews now use the same direct clay readability render path
  as panel-like previews. This keeps Box Primitive edges and the Lidded Box seam
  visible in the large Make stage instead of flattening them through the cached
  square-preview path.

## Computer Use Evidence

Evidence directory:

```text
target/object-intent-handled-panel-integration/
```

Manifest:

```text
target/object-intent-handled-panel-integration/evidence-manifest.json
```

Screenshot hashes:

| Flow | Screenshot | SHA-256 |
| --- | --- | --- |
| Box Primitive | `box-choose.png` | `bae1997c4cda4d0a74d0dab1a0521bc878046d4a8de099559e9730da67c4e5ca` |
| Box Primitive | `box-ready.png` | `213ab5eb85d171088ef46991d60e2a2064bf257ca0d23dbc56c79a3d7e1263e6` |
| Box Primitive | `box-generated.png` | `f0fc31a2ca279e12b41cf07bccd792d14a4dce39c6ce8b3d8f551fee99e13813` |
| Box Primitive | `box-selected.png` | `41c29eb7d1e4a0411f2611e88cf6312c36d3629eb2c6f3aef465ecc719ce841c` |
| Box Primitive | `box-export.png` | `e3ab801bd1ea1cc280f4b3e062c68e0e77b6bef017270a6f17c19aadad9d0255` |
| Lidded Box | `lidded-box-choose.png` | `9f406582f0ba76d2b6582907f0d9e7bace131deecf5dc18438d6f751bcde86f8` |
| Lidded Box | `lidded-box-ready.png` | `5ef32b1dc51fb5f954c39e9a7ac50d421e49d2ee0fab7687706ba0a3f776359d` |
| Lidded Box | `lidded-box-generated.png` | `4b9ec819694add677b8e987fb758815237c3846faee2cb82cf475d35da38d9bc` |
| Lidded Box | `lidded-box-selected.png` | `74c1084a2e9b4be27a8863314a1efbefaba2ec5f7720e9bef8a9255f627bcfb0` |
| Lidded Box | `lidded-box-export.png` | `42b6679cb914aaaa9c5ee5d90e31f3156dadc1a47bcc46d9f04d2e20d33f2341` |
| Flat Panel Primitive | `flat-panel-choose.png` | `e2561ac5c37738e11b4daa9c539e4e4a6a245ab705df9c66d843ff4d117834b0` |
| Flat Panel Primitive | `flat-panel-ready.png` | `c5144ef44fb6c28704bc5861628ee6189b99913912c87d8011595ed2383b0078` |
| Flat Panel Primitive | `flat-panel-generated.png` | `70783d45c58e5da5de4f31ac7dafa0dd2a29a3df693289e5f0add8992d7d22ac` |
| Flat Panel Primitive | `flat-panel-selected.png` | `0cc67730cd9d2fc723f78afe231b4a10f56d09ea169efbd23030d5a09c72ea5a` |
| Flat Panel Primitive | `flat-panel-export.png` | `d4d9353589fca6de95df83ccae6d22cdb77e19bad545726f14e40f0074abb051` |
| Hinged Panel | `hinged-panel-choose.png` | `fc7fb2174adff4f4c66d9b6a63445e69ed4ecbf0c328442b23630e6d3b886da2` |
| Hinged Panel | `hinged-panel-ready.png` | `82b72788c89efd6250480dfcfcaf2c322b34dbd2586c3575b71b2079fdc80db8` |
| Hinged Panel | `hinged-panel-generated.png` | `1b29ce778409776b070122a829aca2c13395c1e3c365647ed15dcabef115c9ab` |
| Hinged Panel | `hinged-panel-selected.png` | `7689c6a4cb81ebf0542eb13f19cad471d8c042e329ec106df9a1193a12487a6d` |
| Hinged Panel | `hinged-panel-export.png` | `8ec7e99a618763027918a328af4b6615cfe5738c0368789b419131ebb7388a21` |
| Handled Panel | `handled-panel-choose.png` | `557ae1fa753120f1d53cb43d56217647c0ff24167bd2369bfde5dd3660d3765e` |
| Handled Panel | `handled-panel-ready.png` | `b2ae0dc2494b5bced29489f575622ce9a5d532de3b40c82908886db5d2cdd3ae` |
| Handled Panel | `handled-panel-generated.png` | `94c9d2238fd50dda71cb28021a76ebf2c672ae419cd47d8ec93f17bf7f094706` |
| Handled Panel | `handled-panel-selected.png` | `c1f1b85fb52cc8695642f480ffe982e071f60014407457efead6dbde97c6242e` |
| Handled Panel | `handled-panel-export.png` | `1c41bdfd7553eaefe5cea3c5d8062eb3a1461b5aa9d187a6af5bc0ae9d224f5f` |

## Dogfood Results

| Check | Result |
| --- | --- |
| Box Primitive name matches visual | Pass |
| Lidded Box lid seam visible | Pass |
| Flat Panel Primitive reads as an upright panel | Pass |
| Hinged Panel hinge edge visible | Pass |
| Handled Panel handle visible | Pass |
| Idea sets visibly differ | Pass |
| No Door/open/close motion claim appears | Pass |
| No material/surface/UV/rig/animation/game-ready overclaim appears | Pass |
| Export copy is truthful for every profile | Pass |
| Object Intent classifier remains contract-only, no runtime LLM | Pass |

## Automated Gates

Requested before merge:

```text
cargo fmt --all --check
python3 scripts/check_source_hygiene.py
cargo test -p shape-foundry object_intent --jobs 1
cargo test -p shape-foundry flat_panel --jobs 1
cargo test -p shape-app foundry --jobs 1
cargo test -p shape-foundry-catalog --test box_primitive --jobs 1
cargo test -p shape-foundry-catalog --test flat_panel --jobs 1
cargo test -p shape-search foundry --jobs 1
cargo test -p shape-render foundry --jobs 1
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
```

Status: `PAUSED`

Completed before the user-requested stop:

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p shape-foundry object_intent --jobs 1` | Pass |
| `cargo test -p shape-foundry flat_panel --jobs 1` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test box_primitive --jobs 1` | Pass after updating a stale five-profile catalog expectation |
| `cargo test -p shape-foundry-catalog --test flat_panel --jobs 1` | Pass |

Not run before the user-requested stop:

| Gate | Status |
| --- | --- |
| `cargo test -p shape-search foundry --jobs 1` | Remaining |
| `cargo test -p shape-render foundry --jobs 1` | Remaining |
| `cargo clippy --workspace --all-targets -- -D warnings` | Remaining |
| `cargo build --release --workspace` | Remaining |

## Next Allowed Work

- Door naming review only after another visible Door cue earns it.
- Family Studio draft flow using Box and Flat Panel as two proven kernels.
- Local Personal Kit persistence for the five app-visible clay baselines.

## Still Blocked

- Door naming by default.
- Open/close motion.
- Surface/material looks.
- UV/texturing.
- Rigging or animation.
- Runtime LLM integration.
- Broad archetype expansion.
