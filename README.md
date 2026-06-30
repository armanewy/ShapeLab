# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The current product baseline is a small set of honest clay starter profiles:
Box Primitive, Lidded Box, Flat Panel Primitive, and Hinged Panel. It is
intentionally small: box-like volumes with readable proportions, edge softness,
and one visible lid seam feature, plus upright panel proofs with one visible
hinge-edge feature. The active built-in Visual Foundry catalog contains only
these four profiles.

This branch starts from the Box Primitive and Lidded Box baselines and proves a
second primitive kernel with Flat Panel Primitive and Hinged Panel. It does not
claim Door behavior, open/close motion, surface/material workflow,
UV/texturing, rigging, animation, runtime LLM integration, or public catalog
publishing.

The product slice targets a narrow `Choose -> Make` loop:

1. Choose `Box Primitive`, `Lidded Box`, `Flat Panel Primitive`, or
   `Hinged Panel`.
2. Enter Make and wait for the clay asset and preview to become ready.
3. Try box, lidded box, panel, or hinged panel ideas.
4. Use one idea.
5. Adjust Proportions, Edge Softness, Lid Seam, or Hinge Edge when present.
6. Add the current asset to Pack.
7. Export.

## Build

```bash
cargo check --workspace
cargo build -p shape-app --release
cargo run -p shape-app --release
```

## Fast Local Development

Use the dev-speed helpers before running broad release gates:

```bash
source scripts/dev_env.sh
python3 scripts/dev_gate.py --tier branch --changed
python3 scripts/dev_gate.py --tier branch --changed --run
python3 scripts/clean_targets.py --list --root ..
```

On Windows:

```powershell
. .\scripts\dev_env.ps1
python scripts/dev_gate.py --tier branch --changed --run
python scripts/clean_targets.py --list --root ..
```

The Box Primitive screenshot/manual visual gate and UI truth-pass gate passed
locally on 2026-06-30. Full workspace test, clippy, release build, and any
broader human dogfood gates are still required before main/release claims. Local
branch verification should stay to affected crates, adjacent tests, and targeted
clippy unless the branch touches build/profile/release/export code.

## Current Scope

- Box Primitive is the baseline box-like object.
- Lidded Box is Box Primitive plus one visible Lid Seam feature.
- Flat Panel Primitive is the second primitive kernel proof.
- Hinged Panel is Flat Panel Primitive plus one visible Hinge Edge feature.
- The built-in catalog and curation metadata contain only Box Primitive,
  Lidded Box, Flat Panel Primitive, and Hinged Panel.
- Box Primitive has two controls: Proportions and Edge Softness.
- Lidded Box has three controls: Proportions, Edge Softness, and Lid Seam.
- Flat Panel Primitive has two controls: Proportions and Edge Softness.
- Hinged Panel has three controls: Proportions, Edge Softness, and Hinge Edge.
- The app may create, vary, pack, and export a box-like or panel-like clay
  asset.
- The Box Primitive screenshot/manual visual gate passed with release-app
  evidence under `target/box-primitive-dogfood-gate/`.
- The Box Primitive UI truth-pass gate passed with release-app evidence under
  `target/box-primitive-ui-truth-pass/screenshots/`.
- The Box Primitive visual-readability gate passed with evidence under
  `target/box-primitive-visual-readability/`.
- The Lidded Box Make baseline gate passed with evidence under
  `target/lidded-box-make-baseline-gate/`.
- The Trim Band feature-module gate passed with internal evidence under
  `target/trim-band-feature-module-v0/`; Trimmed Box is not app-visible yet.
- The Flat Panel Primitive baseline is documented in
  `docs/FLAT_PANEL_PRIMITIVE_BASELINE.md`.
- The Hinged Panel Make baseline gate passed with release-app evidence under
  `target/hinged-panel-make-baseline-gate/`.
- The second-kernel Flat Panel integration gate passed with release-app
  regression evidence under `target/second-kernel-flat-panel-integration/`.
- Surface/material, UV/texturing, rigging, animation, and game-ready UI remain
  blocked.
- The next allowed visible feature is Door Handle / Knob; Door naming remains
  blocked until visible door cues pass a later gate.

Status details are documented in:

- [`docs/CURRENT_PRODUCT_STATUS.md`](docs/CURRENT_PRODUCT_STATUS.md)
- [`docs/BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md`](docs/BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md)
- [`docs/BOX_PRIMITIVE_UI_TRUTH_PASS.md`](docs/BOX_PRIMITIVE_UI_TRUTH_PASS.md)
- [`docs/BOX_PRIMITIVE_VISUAL_READABILITY.md`](docs/BOX_PRIMITIVE_VISUAL_READABILITY.md)
- [`docs/LIDDED_BOX_MAKE_BASELINE_GATE.md`](docs/LIDDED_BOX_MAKE_BASELINE_GATE.md)
- [`docs/TRIM_BAND_FEATURE_MODULE_V0.md`](docs/TRIM_BAND_FEATURE_MODULE_V0.md)
- [`docs/FLAT_PANEL_PRIMITIVE_BASELINE.md`](docs/FLAT_PANEL_PRIMITIVE_BASELINE.md)
- [`docs/HINGED_PANEL_MAKE_BASELINE_GATE.md`](docs/HINGED_PANEL_MAKE_BASELINE_GATE.md)
- [`docs/SECOND_KERNEL_FLAT_PANEL_INTEGRATION_REPORT.md`](docs/SECOND_KERNEL_FLAT_PANEL_INTEGRATION_REPORT.md)
- [`docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md`](docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md)
- [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md)
