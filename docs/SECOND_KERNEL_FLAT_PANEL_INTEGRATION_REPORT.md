# Second Kernel Flat Panel Integration Report

Date: 2026-06-30

## Verdict

`PASS`

Object Orchard now has two proven visible kernel paths in Shape Lab:

- Box-like objects: Box Primitive and Lidded Box.
- Upright panel-like objects: Flat Panel Primitive and Hinged Panel.

Trimmed Box remains internal feature-module evidence only. None of the visible
profiles claim crate, Door, open/close motion, material looks, UV/texturing,
rigging, animation, or game-ready packaging.

## Included Work

- Flat Panel Kernel Contracts.
- Flat Panel Primitive Make Baseline.
- Hinge Edge Feature Module.
- Hinged Panel Make Baseline.

## Product Truth

| Statement | Result |
| --- | --- |
| Box Primitive is the baseline box-like object. | Pass |
| Lidded Box is Box Primitive plus Lid Seam. | Pass |
| Trimmed Box is internal evidence only. | Pass |
| Flat Panel Primitive is the second kernel proof. | Pass |
| Hinged Panel is Flat Panel Primitive plus Hinge Edge. | Pass |
| No visible profile claims crate or Door motion. | Pass |
| No material/surface/rigging/animation UI is active. | Pass |
| Family Studio remains blocked/internal. | Pass |

## Computer Use Dogfood Evidence

Release app regression flow was run with Computer Use. Evidence is under:

```text
target/second-kernel-flat-panel-integration/
```

Screenshot hashes:

| File | SHA-256 |
| --- | --- |
| `catalog_choose.png` | `06e0d84d761af891523a0300bb229bc33e77698219df8cd142eda5d03aa2a66f` |
| `box_primitive_export.png` | `b3006209ff569cd6c2e52296b46fe0082c126db5f00187c16e07d3f175ceb5ae` |
| `lidded_box_export.png` | `b7dbeb2b7c74c6ea1b5c034799d0908560b50f4178abf55b2c922779a51cb381` |
| `flat_panel_export.png` | `d3ecfab04f2ad2fc0d2234527e7269be46ecc32c12bfe3e2c26a9f397654fa47` |
| `hinged_panel_export.png` | `23ce50a8fe0d003341fa5ffd8758bc2e3de0b68d266ed1b56d840877062ee07b` |

## Dogfood Flows

| Flow | Result |
| --- | --- |
| Box Primitive: Choose -> Make -> Try ideas -> Use one -> Export | Pass |
| Lidded Box: Choose -> Make -> Try ideas -> Use one -> Adjust -> Export | Pass |
| Flat Panel Primitive: Choose -> Make -> Try ideas -> Use one -> Export | Pass |
| Hinged Panel: Choose -> Make -> Try ideas -> Use one -> Adjust -> Export | Pass |

## Automated Gates

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-foundry flat_panel --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test box_primitive --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test flat_panel --jobs 1` | Pass |
| `cargo test -p shape-search foundry --jobs 1` | Pass |
| `cargo test -p shape-render foundry --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Notes

The integration gate corrected one stale box-catalog test that still expected
only Box Primitive and Lidded Box. The correct app-visible catalog is now Box
Primitive, Lidded Box, Flat Panel Primitive, and Hinged Panel.

## Next Decision

Allowed next work:

- Door Handle / Knob feature.
- Family Studio draft flow using Box and Flat Panel as two proven kernels.
- Personal Kit persistence.

Still blocked:

- Door naming.
- Open/close motion.
- Material looks.
- UV/texturing.
- Rigging/animation.
- Broad archetype expansion.
