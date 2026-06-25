# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The current desktop app launches directly into **Visual Foundry**, a whole-model
customizer for authored semantic asset families. The old implicit editor,
explicit modeling workspace, and nested mode switchers are no longer product
surfaces.

The product slice proves a category-independent loop:

1. Open a local desktop app.
2. Choose an asset family.
3. Generate several coherent whole-model directions.
4. Choose one direction.
5. Customize primary controls and lock traits.
6. Export a single asset or a small family pack while keeping branchable history.

## Build

```bash
cargo check --workspace
cargo run -p shape-app --release
```

Detailed local and CI build instructions, including Linux native packages and the reproducible release command list, are in [`docs/building.md`](docs/building.md).

Release status and scope are documented in [`docs/MVP_REPORT.md`](docs/MVP_REPORT.md), [`docs/RELEASE_READINESS.md`](docs/RELEASE_READINESS.md), [`docs/RELEASE_CANDIDATE_MANUAL_GATE.md`](docs/RELEASE_CANDIDATE_MANUAL_GATE.md), [`docs/FOUNDRY_UI_MANUAL_GATE.md`](docs/FOUNDRY_UI_MANUAL_GATE.md), [`docs/HQ_ASSET_QUALITY_BAR.md`](docs/HQ_ASSET_QUALITY_BAR.md), [`docs/FOUNDRY_KIT_SYSTEM.md`](docs/FOUNDRY_KIT_SYSTEM.md), [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md), [`docs/NEXT_BACKENDS.md`](docs/NEXT_BACKENDS.md), and [`docs/MANUAL_TEST_CHECKLIST.md`](docs/MANUAL_TEST_CHECKLIST.md).

The native app opens a local `egui` desktop workspace with:

- Visual Foundry as the product app and primary novice-facing surface
- seventeen installed Visual Foundry kit-backed profiles available to preview/developer catalogs, including the original Visual Foundry families, Roman Timber Bridge HQ, promoted gear kits, and the hidden MOBA Hero Clay MVP
- a Choose screen, direction board, customizer deck, pack workspace, and export flow
- whole-model candidate cards and whole-model customizer previews
- reducer-backed locks, undo, save/open, current export, and pack export
- background compile, preview, semantic candidate generation, candidate render, save/open, and export jobs
- branch-preserving `.shapelab-foundry.json` save/open, grouped OBJ export, and canonical model-package export

Startup shows the Visual Foundry "Choose what to make" home screen. Pending
kits are hidden from the default novice catalog until review approval is
recorded; set `SHAPE_LAB_PREVIEW_CATALOG=1` for internal preview catalog work.

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

`shape-family` owns theme-neutral family and style-kit contracts. `shape-family-compile` owns the first executable binding layer from those contracts to concrete `AssetRecipe` output. `shape-gamekit` owns runtime-neutral game metadata. `shape-caesar-assets` is the first content-pack customer, not a core engine dependency. See [`docs/asset-family-foundry.md`](docs/asset-family-foundry.md), [`docs/adr/0011-asset-family-style-kit-layer.md`](docs/adr/0011-asset-family-style-kit-layer.md), and [`docs/adr/0012-executable-family-bindings.md`](docs/adr/0012-executable-family-bindings.md).

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
proxy LODs, material-slot assignments, collision proxy, visual evidence, and a
validation report. The report intentionally blocks a full game-ready claim until
UV layout, direct GLB handoff, and manual DCC/runtime review are complete.

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

Each `model-demo` run writes `recipe.json`, grouped `asset.obj`, `provenance.json`, `validation.json`, `model-validation.json`, `statistics.json`, `preview.png`, and `blender_reconstruct.py`. Package validation carries compile issues plus recipe-derived model validation issues.

Render fixed-camera shaded and wireframe benchmark sheets for the explicit asset search loop:

```bash
cargo run -p shape-cli -- asset-visual-benchmark --out-dir target/asset-visual-benchmark
```

Each asset directory contains original renders, six Refine candidates, six Explore candidates, an accepted branch, a final canonical package, contact sheets, wireframe contact sheets, and `visual-benchmark-summary.json`. Candidate selection uses compiled mesh descriptors derived from fixed-camera silhouette masks, perimeter, visible z-buffer depth histograms, mesh volume, and recipe structure.

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

The decompiler requires identical ordered topology and writes canonical binary mesh sidecars, an ordered reconstruction stream, cumulative baked stage sidecars, an exact final correction, `manifest.json`, two verification reports, schema-3 program-hypothesis diagnostics, and `blender_reconstruct.py`. Package schema 2 specifies deterministic stepwise binary32 affine arithmetic and manifest-declared stage files so Rust and Python replay can verify every serialized stage bit-for-bit; diagnostics are versioned separately and describe scored ordered programs rather than single operator families. Package output is staged and replay-verified before replacing an existing directory. The generated Blender script reconstructs editable shape-key stages, bakes the final object from replayed operators, and verifies exact topology and final positions. Details are in [`docs/deformation-decompiler.md`](docs/deformation-decompiler.md).

## Scope

The MVP is category-general because it contains no humanoid-specific engine concepts. Visual Foundry profiles span props, furniture, environment pieces, and structures. The core vocabulary is parts, generators, transforms, semantic edits, visual descriptors, candidates, validation relationship selectors, and revisions.

The MVP is still representation-specific: Visual Foundry works on authored semantic asset-family documents that compile to explicit `AssetRecipe` graphs. Triangle-mesh import handling exists only in headless/research strict reconstruction and same-topology deformation decompiler paths, not as Visual Foundry product editability. Arbitrary imported meshes are not semantically editable unless they fit a known grammar and strict verification proves exact recovery.

Release readiness is archive-first only. The repository documents manual package contents, but installers, code signing, notarization, package-manager publishing, and app-store publishing are not implemented.

## Non-Goals Before MVP Gate

- Natural-language modeling
- LLM integration, LLM geometry generation, or direct LLM recipe mutation
- General Blender integration beyond the decompiler reconstruction script
- General imported mesh editing without known vertex correspondence
- Rigging, animation, UVs, or texturing
- Materials or marketplace publishing workflows
- GPU compute or a custom GPU viewport
- Cloud or collaborative features
