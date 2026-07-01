# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D
modeling.

The current product direction is direct primitive editing. Object Orchard starts
from simple clay primitives whose visible properties are bounded and explicit,
not from generated variation trays. The active primitive workflow is:

```text
Choose Primitive
-> Make
-> edit Width / Height / Depth / Radius / Thickness / Edge Softness
-> orbit and inspect
-> Add to Pack
-> Export
```

## Current Baseline

- Box Primitive is the direct box baseline.
- Flat Panel Primitive is the direct panel baseline.
- Sphere Primitive is the direct round baseline.
- Panel with Knob is the first approved safe-anchor composition proof.
- Lidded Box and Hinged Panel are feature proofs that remain useful evidence,
  but future active primitive work favors direct property editing.
- Generated idea workflows are retired from active primitive UI.
- Candidate generation is inactive in the current primitive product flow.
- ObjectPlan v0 exists for structured offline validation and review of
  supported primitive and safe-anchor composition plans.
- Offline LLMs may draft ObjectPlan JSON outside the app, but every plan is
  validated by Object Orchard before review.
- ObjectPlan batch review and the internal review drawer are available as
  offline/internal review infrastructure; broad family generation is not
  implemented.
- Future suggestions may return only as deterministic property presets, not
  opaque random candidate generation.
- Family Studio Lite is paused until direct primitive and composition flows are
  stable.
- Surface/material work, UV/texturing, rigging, animation, runtime LLM
  integration, public catalog publishing, and game-ready UI remain blocked.

## Milestone Rule

Use one visible operation per milestone. The current sequence is:

1. direct-edit Box
2. direct-edit Flat Panel
3. direct-edit Sphere
4. make knob-like form from Sphere
5. attach knob-like form to panel through safe anchor

## Product Boundary

Users edit immutable primitive property schemas. They manipulate bounded
properties such as Width, Height, Depth, Radius, Thickness, Edge Softness, and
Flattening. They do not manipulate vertices, faces, loops, cages, booleans, raw
mesh transforms, or Blender-like modeling controls.

Composition will happen through safe anchors and constrained attachment zones,
not arbitrary scene transforms. LLMs may later draft preset or repair plans, but
validators own the legal property domains and mesh generation remains native and
offline.

The current app does not run LLMs. Offline LLMs may only become useful through
validated ObjectPlan JSON, honest render-blocked or visual evidence reports,
and human review gates.

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

## Status Docs

- [`docs/CURRENT_PRODUCT_STATUS.md`](docs/CURRENT_PRODUCT_STATUS.md)
- [`docs/OBJECT_PLAN_V0_INTEGRATION_REPORT.md`](docs/OBJECT_PLAN_V0_INTEGRATION_REPORT.md)
- [`docs/POST_PRIMITIVE_COMPOSITION_TRUTH_GATE.md`](docs/POST_PRIMITIVE_COMPOSITION_TRUTH_GATE.md)
- [`docs/OBJECT_PLAN_DSL_CONTRACTS.md`](docs/OBJECT_PLAN_DSL_CONTRACTS.md)
- [`docs/OBJECT_PLAN_BATCH_RUNNER_V0.md`](docs/OBJECT_PLAN_BATCH_RUNNER_V0.md)
- [`docs/PRIMITIVE_DIRECT_MAKE_VISION.md`](docs/PRIMITIVE_DIRECT_MAKE_VISION.md)
- [`docs/ACTIVE_VARIATION_UI_RETIREMENT.md`](docs/ACTIVE_VARIATION_UI_RETIREMENT.md)
- [`docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md`](docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md)
- [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md)
