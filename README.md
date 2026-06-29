# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The current desktop app launches directly into **Visual Foundry**, a whole-model
customizer for authored semantic asset families. The old implicit editor,
explicit modeling workspace, and nested mode switchers are no longer product
surfaces.

Current dogfood status: Simple Crate is the novice Make baseline proof. Utility
Crate is the next family-maturity rung. Cargo Case remains the advanced
equipment-case proof. Product Dogfood Gate v4 still preserves the Sci-Fi Crate
Make regression baseline, but Sci-Fi Crate is a regression/advanced profile,
not the flagship. Shape Lab is not being built for any one specific model. The
family-authoring ladder grows through Simple Crate Primitive -> Utility Crate
Family -> Cargo Case -> Product profiles.
Roman Bridge remains PreviewOnly. Broad UV/Texturing/Rigging/Animation UI
remains blocked.
The Sci-Fi Crate material-look path remains a narrow preview-only baseline
backed by generated surface-candidate evidence; it does not change export
payloads, stale material evidence must be disabled after geometry changes, and
full game-ready remains blocked. Cargo Case remains valid but scoped to
equipment cases only and is the advanced equipment-case proof: Clean Utility
Case and Sci-Fi Industrial Crate share one Cargo Case family, controls, roles,
and semantic clay part groups. This is not a broad archetype library, broad
Surface mode, material editor, UV/texturing, rigging, animation, or full
game-ready approval. Clay mesh quality comes before surface or material
presentation.
The canonical status note is
[`docs/CURRENT_PRODUCT_STATUS.md`](docs/CURRENT_PRODUCT_STATUS.md).

The product slice targets a category-independent `Choose -> Make` loop:

1. Open a local desktop app.
2. Choose an asset family.
3. Enter Make and let the app prepare the first model and preview.
4. Try coherent whole-asset ideas.
5. Choose an idea, then adjust a clear control.
6. Export a single asset or a small family pack while keeping branchable
   history.

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

`scripts/dev_env.*` can share Cargo build output and enable `sccache` when it is
installed. `scripts/dev_gate.py` maps changed files to the smallest relevant
gate. `scripts/clean_targets.py` reports stale Cargo `target` directories and
refuses to delete active worktree targets unless explicitly allowed.

Full workspace test, clippy, release build, and human dogfood gates are still
required before main/release claims. They are not required for every local
prompt lane. See [`docs/DEVELOPMENT_SPEED.md`](docs/DEVELOPMENT_SPEED.md) and
[`docs/TEST_GATE_POLICY.md`](docs/TEST_GATE_POLICY.md).

On Windows, use the launcher script when you want only the app window and no
extra console window. The script rebuilds the selected profile before launch and
stops stale Shape Lab processes from this repo's `target` directory, so debug and
release binaries do not drift during local verification. The native app starts
as a standard decorated Windows window, maximized to the current monitor's work
area:

```powershell
pwsh -File scripts/run_shape_app.ps1 -PreviewCatalog
```

Detailed local and CI build instructions, including Linux native packages and
the reproducible release command list, are in
[`docs/building.md`](docs/building.md).

Release status and scope are documented in:

- [`docs/CURRENT_PRODUCT_STATUS.md`](docs/CURRENT_PRODUCT_STATUS.md)
- [`docs/MAINLINE_MAKE_CANVAS_FAILURE_AUDIT.md`](docs/MAINLINE_MAKE_CANVAS_FAILURE_AUDIT.md)
- [`docs/NEXT_PRODUCT_RECOVERY_PLAN.md`](docs/NEXT_PRODUCT_RECOVERY_PLAN.md)
- [`docs/MVP_REPORT.md`](docs/MVP_REPORT.md)
- [`docs/RELEASE_READINESS.md`](docs/RELEASE_READINESS.md)
- [`docs/RELEASE_CANDIDATE_MANUAL_GATE.md`](docs/RELEASE_CANDIDATE_MANUAL_GATE.md)
- [`docs/PRODUCT_DOGFOOD_GATE_V4.md`](docs/PRODUCT_DOGFOOD_GATE_V4.md)
- [`docs/PRODUCT_DOGFOOD_GATE_V4_RESULTS.md`](docs/PRODUCT_DOGFOOD_GATE_V4_RESULTS.md)
- [`docs/FAMILY_FOUNDATION_PIVOT.md`](docs/FAMILY_FOUNDATION_PIVOT.md)
- [`docs/FAMILY_MATURITY_LADDER.md`](docs/FAMILY_MATURITY_LADDER.md)
- [`docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md`](docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md)
- [`docs/SIMPLE_CRATE_MAKE_BASELINE.md`](docs/SIMPLE_CRATE_MAKE_BASELINE.md)
- [`docs/UTILITY_CRATE_FAMILY_V1_REPORT.md`](docs/UTILITY_CRATE_FAMILY_V1_REPORT.md)
- [`docs/SCIFI_CRATE_REGRESSION_ROLE.md`](docs/SCIFI_CRATE_REGRESSION_ROLE.md)
- [`docs/SCIFI_CRATE_VISUAL_SURFACE_CANDIDATES_V0.md`](docs/SCIFI_CRATE_VISUAL_SURFACE_CANDIDATES_V0.md)
- [`docs/SURFACE_MODE_DOGFOOD_V0_RESULTS.md`](docs/SURFACE_MODE_DOGFOOD_V0_RESULTS.md)
- [`docs/CARGO_CASE_ARCHITECTURE_INTEGRATION_REPORT.md`](docs/CARGO_CASE_ARCHITECTURE_INTEGRATION_REPORT.md)
- [`docs/FOUNDRY_UI_MANUAL_GATE.md`](docs/FOUNDRY_UI_MANUAL_GATE.md)
- [`docs/HQ_ASSET_QUALITY_BAR.md`](docs/HQ_ASSET_QUALITY_BAR.md)
- [`docs/VARIATION_SCOPE_CHANNEL_CONTRACT.md`](docs/VARIATION_SCOPE_CHANNEL_CONTRACT.md)
- [`docs/VARIATION_LEGIBILITY_GATE.md`](docs/VARIATION_LEGIBILITY_GATE.md)
- [`docs/FOCUS_PART_MODE.md`](docs/FOCUS_PART_MODE.md)
- [`docs/FOUNDRY_KIT_SYSTEM.md`](docs/FOUNDRY_KIT_SYSTEM.md)
- [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md)
- [`docs/NEXT_BACKENDS.md`](docs/NEXT_BACKENDS.md)
- [`docs/MANUAL_TEST_CHECKLIST.md`](docs/MANUAL_TEST_CHECKLIST.md)

The native app opens a local `egui` desktop workspace with:

- Visual Foundry as the product app and primary novice-facing surface
- four default novice-visible profiles: Simple Crate, Utility Crate, Sci-Fi
  Industrial Crate, and Stylized Furniture Lamp. Simple Crate is the novice
  baseline; Utility Crate is the next family-maturity rung; Sci-Fi Industrial
  Crate is visible for regression continuity only while its dogfood status
  remains current
- nineteen installed Visual Foundry kit-backed profiles available to
  preview/developer catalogs, including Roman Timber Bridge HQ, promoted gear
  kits, and the Hero Character preview kit
- an asset-family grouped Choose list, Make workspace, candidate tray, focused
  part chips, contextual controls, pack workspace, and export flow
- whole-model candidate cards and DPI-aware whole-model previews
- reducer-backed locks, undo, save/open, current export, and pack export
- background compile, preview, semantic candidate generation, candidate render, save/open, and export jobs
- branch-preserving `.shapelab-foundry.json` save/open, grouped OBJ export, and canonical model-package export

Startup shows the Visual Foundry "Choose what to make" home screen. The Choose
catalog is a grouped list by asset family, with one Start action per template.
Pending kits are hidden from the default novice catalog until review approval is
recorded; set `SHAPE_LAB_PREVIEW_CATALOG=1` for internal preview catalog work.

## Simple Crate Baseline

Use this pass for the default novice dogfood path.

1. Launch the release app with `cargo run -p shape-app --release`.
2. On Choose, start `Simple Crate`.
3. In Make, wait for the model and preview to become ready.
4. Use `Try crate ideas`.
5. Use one crate idea.
6. Adjust one crate control.
7. Add the current crate to Pack.
8. Open Export.

The baseline copy is intentionally plain: `Try crate ideas`, `Use this crate`,
`Adjust crate`, `Add to Pack`, and `Export`. Focus Part is not required for
this path.

## Sci-Fi Regression Pass

Use this pass to make one Sci-Fi Crate and export it through the product UI only
when validating regressions, Cargo Case compatibility, or narrow material-look
preview work. Product Dogfood Gate v4 passed this Sci-Fi Crate baseline, but it
is an advanced regression profile and no longer the flagship proof.

1. Launch the release app with `cargo run -p shape-app --release`.
2. On Choose, find the `Crate` family group and start `Sci-Fi Industrial Crate`.
3. In Make, wait for the local preparing or preview-ready state to complete.
4. Use `Try ideas` and confirm the candidate tray or comparison visibly changes.
5. Choose one coherent whole-model idea, or focus a visible part such as
   Handles or Vents if that action is obvious.
6. Open Export and export the current asset to a local test directory.
7. Confirm the export path contains the expected package files and grouped OBJ output.

During this pass, the app should not expose raw provider IDs, semantic IDs,
compiler/decompiler wording, scalar paths, or internal kit planning labels in the
asset-user UI. Passing this flow does not approve broader Surface,
UV/Texturing, Rigging, animation, or full game-ready product UI.

## Next Family Proof

Simple Crate is the current novice baseline proof. It starts with a small
object grammar, few controls, a fast Make loop, and visible clay variation.
Utility Crate v1 is the next family-maturity rung after the primitive reads
clearly in clay. Cargo Case remains the advanced equipment-case proof.

Surface and material work must not lead this proof. Clay mesh quality comes
first, and no texture or material treatment may mask weak geometry.

## Architecture

```text
Preset shape document
        |
        v
shape-core semantic graph and edit programs
        |
        v
shape-field CPU implicit field
        |
        v
shape-mesh marching tetrahedra mesh + OBJ
        |
        +--> shape-render CPU PNG preview
        |
        v
shape-search deterministic candidate generation
        |
        v
shape-project branchable revision history
```

`shape-app` is the native `eframe`/`egui` desktop app. `shape-cli` is the headless integration driver.

Shape Lab also includes an explicit polygon modeling lane for part-aware assets:

```text
AssetRecipe
  -> shape-modeling generators + assembly
  -> shape-search semantic candidates
  -> bounded proposal compilation
  -> shape-render mesh visual descriptors + scoring
  -> shape-compile exports
```

That lane remains available through core crates and headless CLI tests, not as a
separate desktop product surface. Its benchmark assets live in
`crates/shape-modeling-assets`.

The next product layer is the asset-family foundry:

```text
AssetFamilySchema
  + StyleKit
  + FamilyImplementation/StyleImplementation bindings
  + optional runtime/export profile
  -> concrete AssetRecipe variants
  -> validation, preview, export, and adapter packaging
```

`shape-family` owns theme-neutral family and style-kit contracts.
`shape-family-compile` owns the first executable binding layer from those
contracts to concrete `AssetRecipe` output. `shape-gamekit` owns
runtime-neutral game metadata. `shape-caesar-assets` is the first content-pack
customer, not a core engine dependency. See
[`docs/asset-family-foundry.md`](docs/asset-family-foundry.md),
[`docs/adr/0011-asset-family-style-kit-layer.md`](docs/adr/0011-asset-family-style-kit-layer.md),
and
[`docs/adr/0012-executable-family-bindings.md`](docs/adr/0012-executable-family-bindings.md).

## CLI

```bash
cargo run -p shape-cli -- --help
```

Generate deterministic demo artifacts:

```bash
cargo run -p shape-cli -- demo --preset desk-lamp --seed 42 --out-dir target/demo-lamp
cargo run -p shape-cli -- demo --preset toy-submarine --seed 42 --out-dir target/demo-submarine
cargo run -p shape-cli -- demo --preset alien-plant --seed 42 --out-dir target/demo-plant
cargo run -p shape-cli -- demo --preset sky-shrine --seed 42 --out-dir target/demo-shrine
pwsh -File scripts/generate_demo_assets.ps1 -OutDir target/demo-assets
```

Each run writes project JSON, OBJ meshes, PNG previews, a contact sheet, and a summary JSON file.

Generate release-readiness reports with computed Visual Foundry evidence:

```bash
cargo run -p shape-cli -- release-readiness --verify-visual-gate
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate
```

The visual gate computes catalog/render evidence. The product UI gate verifies
the direct Visual Foundry shell contract and still requires the human screenshot
checklist in [`docs/FOUNDRY_UI_MANUAL_GATE.md`](docs/FOUNDRY_UI_MANUAL_GATE.md).

Generate HQ asset quality evidence for authored Visual Foundry content:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge --out-dir target/hq-benchmark/roman-bridge --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile all --out-dir target/hq-benchmark --verify-export
```

The HQ benchmark records clay views, contact sheets, mesh validity, semantic
parts, candidate survival, visible-control evidence, export/reopen status, and
unsupported outputs. It separates release readiness from Showcase-quality asset
approval.

Generate the first static-prop game-readiness package for the Sci-Fi Crate:

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

The package emits a frozen canonical model package, OBJ handoff, deterministic
proxy LODs, collision proxy, visual evidence, a geometry-only GLB, and a
Surface Lab v1 sidecar for the Sci-Fi Crate with deterministic UV/material/
texture evidence. The report intentionally blocks a full game-ready claim until
manual DCC/runtime review, engine import proof, and engine-native package
handoff are complete.

Inspect and package curated Foundry kit metadata:

```bash
cargo run -p shape-cli -- foundry-kit inspect roman-bridge
cargo run -p shape-cli -- foundry-kit package roman-bridge --out-dir target/foundry-kit/roman-package
```

Kits summarize authored family/style/provider/control content, quality gates,
compatibility policy, and review evidence while keeping the exact Foundry
compiler path as the geometry source of truth.

Compile and export explicit benchmark assets:

```bash
cargo run -p shape-cli -- model-demo --asset industrial-crate --out-dir target/model-demo/crate
cargo run -p shape-cli -- model-demo --asset explicit-desk-lamp --out-dir target/model-demo/lamp
cargo run -p shape-cli -- model-demo --asset stylized-stool --out-dir target/model-demo/stool
```

Each `model-demo` run writes `recipe.json`, grouped `asset.obj`,
`provenance.json`, `validation.json`, `model-validation.json`,
`statistics.json`, `preview.png`, and `blender_reconstruct.py`. Package
validation carries compile issues plus recipe-derived model validation issues.

Render fixed-camera shaded and wireframe benchmark sheets for the explicit asset search loop:

```bash
cargo run -p shape-cli -- asset-visual-benchmark --out-dir target/asset-visual-benchmark
```

Each asset directory contains original renders, six Refine candidates, six
Explore candidates, an accepted branch, a final canonical package, contact
sheets, wireframe contact sheets, and `visual-benchmark-summary.json`.
Candidate selection uses compiled mesh descriptors derived from fixed-camera
silhouette masks, perimeter, visible z-buffer depth histograms, mesh volume,
and recipe structure.

Packaging notes, third-party dependency documentation, and placeholder icon assets live under [`packaging/`](packaging/).

Validate and export:

```bash
cargo run -p shape-cli -- validate target/demo-lamp/project-after.json
cargo run -p shape-cli -- export target/demo-lamp/project-after.json --obj target/demo-lamp/export.obj --png target/demo-lamp/export.png
```

Decompile a same-topology mesh pair into deformation IR and a Blender reconstruction script, then independently replay-verify the serialized package:

```bash
cargo run -p shape-cli -- decompile source.obj target.obj --out-dir target/decompile-package
cargo run -p shape-cli -- verify-decompile target/decompile-package
```

The decompiler requires identical ordered topology and writes canonical binary
mesh sidecars, an ordered reconstruction stream, cumulative baked stage
sidecars, an exact final correction, `manifest.json`, two verification reports,
schema-3 program-hypothesis diagnostics, and `blender_reconstruct.py`. Package
schema 2 specifies deterministic stepwise binary32 affine arithmetic and
manifest-declared stage files so Rust and Python replay can verify every
serialized stage bit-for-bit. Diagnostics are versioned separately and describe
scored ordered programs rather than single operator families. Package output is
staged and replay-verified before replacing an existing directory. The generated
Blender script reconstructs editable shape-key stages, bakes the final object
from replayed operators, and verifies exact topology and final positions.
Details are in [`docs/deformation-decompiler.md`](docs/deformation-decompiler.md).

## Scope

The MVP is category-general because it contains no humanoid-specific engine
concepts. Visual Foundry profiles span props, furniture, environment pieces,
and structures. The core vocabulary is parts, generators, transforms, semantic
edits, visual descriptors, candidates, validation relationship selectors, and
revisions.

The MVP is still representation-specific: Visual Foundry works on authored
semantic asset-family documents that compile to explicit `AssetRecipe` graphs.
Triangle-mesh import handling exists only in headless/research strict
reconstruction and same-topology deformation decompiler paths, not as Visual
Foundry product editability. Arbitrary imported meshes are not semantically
editable unless they fit a known grammar and strict verification proves exact
recovery.

Release readiness is archive-first only. The repository documents manual
package contents, but installers, code signing, notarization, package-manager
publishing, and app-store publishing are not implemented.

## Non-Goals Before MVP Gate

- Natural-language modeling
- LLM integration, LLM geometry generation, or direct LLM recipe mutation
- General Blender integration beyond the decompiler reconstruction script
- General imported mesh editing without known vertex correspondence
- Broad Surface mode, broad UV/texturing, rigging, skinning, or animation
  beyond narrow evidence-backed preview/headless paths
- Materials or marketplace publishing workflows
- Material editor or profile explosion before another family proof exists
- GPU compute or a custom GPU viewport
- Cloud or collaborative features
