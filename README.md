# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The current product baseline is **Box Primitive**. It is intentionally small:
a closed clay box-like volume with readable proportions and edge softness.
The active built-in Visual Foundry catalog contains only this profile.

This branch starts fresh from the Box Primitive baseline. It does not claim a
non-box model family, surface/material workflow, UV/texturing, rigging,
animation, runtime LLM integration, or public catalog publishing.

The product slice targets a narrow `Choose -> Make` loop:

1. Choose `Box Primitive`.
2. Enter Make and wait for the box and preview to become ready.
3. Try box ideas.
4. Use one box idea.
5. Adjust Proportions or Edge Softness.
6. Add the current box to Pack.
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

- Box Primitive is the novice baseline.
- The built-in catalog and curation metadata are Box Primitive only.
- Box Primitive has two controls: Proportions and Edge Softness.
- The app may create, vary, pack, and export a box-like asset.
- The Box Primitive screenshot/manual visual gate passed with release-app
  evidence under `target/box-primitive-dogfood-gate/`.
- The Box Primitive UI truth-pass gate passed with release-app evidence under
  `target/box-primitive-ui-truth-pass/screenshots/`.
- Surface/material, UV/texturing, rigging, animation, and game-ready UI remain
  blocked.
- Richer box-family features must be added later one visible module at a time,
  after the Box Primitive baseline gate.

Status details are documented in:

- [`docs/CURRENT_PRODUCT_STATUS.md`](docs/CURRENT_PRODUCT_STATUS.md)
- [`docs/BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md`](docs/BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md)
- [`docs/BOX_PRIMITIVE_UI_TRUTH_PASS.md`](docs/BOX_PRIMITIVE_UI_TRUTH_PASS.md)
- [`docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md`](docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md)
- [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md)
