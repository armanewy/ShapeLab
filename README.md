# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The current desktop app starts in **Asset Modeling Lab**, a part-aware forward-modeling workflow for explicit polygon assets. The legacy implicit editor remains available from the mode switcher.

The product slice proves a category-independent loop:

1. Open a local desktop app.
2. Choose a modeled asset template.
3. View named parts, parameters, locks, validation, and branch history.
4. Generate several coherent semantic futures.
4. Choose one direction.
5. Repeat while keeping branchable history.

## Build

```bash
cargo check --workspace
cargo run -p shape-app --release
```

Detailed local and CI build instructions, including Linux native packages and the reproducible release command list, are in [`docs/building.md`](docs/building.md).

Release status and scope are documented in [`docs/MVP_REPORT.md`](docs/MVP_REPORT.md), [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md), [`docs/NEXT_BACKENDS.md`](docs/NEXT_BACKENDS.md), and [`docs/MANUAL_TEST_CHECKLIST.md`](docs/MANUAL_TEST_CHECKLIST.md).

The native app opens a local `egui` desktop workspace with:

- Asset Modeling Lab template choices for Industrial Crate, Explicit Desk Lamp, and Stylized Stool
- a rendered asset viewport with orbit, pan, zoom, fit, selected-part overlays, and wireframe display
- a part tree, inspector, locks, validation, branch history, status bar, and candidate gallery
- background compile, preview, semantic candidate generation, candidate render, save/open, and export jobs
- branch-preserving `.shapelab-asset.json` save/open, grouped OBJ export, and canonical model-package export
- a switchable legacy implicit editor for the older SDF shape-document workflow

Startup shows the Asset Modeling Lab template picker. The hidden legacy implicit mode is initialized only after the user switches to it.

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

That lane is additive to the implicit editor and same-topology decompiler. Its benchmark assets live in `crates/shape-modeling-assets`.

The next product layer is the asset-family foundry:

```text
AssetFamilySchema
  + StyleKit
  + optional runtime/export profile
  -> authored AssetRecipe variants
  -> validation, preview, export, and adapter packaging
```

`shape-family` owns theme-neutral family and style-kit contracts. `shape-gamekit` owns runtime-neutral game metadata. `shape-caesar-assets` is the first content-pack customer, not a core engine dependency. See [`docs/asset-family-foundry.md`](docs/asset-family-foundry.md) and [`docs/adr/0011-asset-family-style-kit-layer.md`](docs/adr/0011-asset-family-style-kit-layer.md).

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

Compile and export explicit benchmark assets:

```bash
cargo run -p shape-cli -- model-demo --asset industrial-crate --out-dir target/model-demo/crate
cargo run -p shape-cli -- model-demo --asset explicit-desk-lamp --out-dir target/model-demo/lamp
cargo run -p shape-cli -- model-demo --asset stylized-stool --out-dir target/model-demo/stool
```

Each `model-demo` run writes `recipe.json`, grouped `asset.obj`, `provenance.json`, `validation.json`, `model-validation.json`, `statistics.json`, `preview.png`, and `blender_reconstruct.py`. Package validation carries compile issues plus recipe-derived model validation issues.

Render fixed-camera shaded and wireframe benchmark sheets for the Asset Modeling Lab search loop:

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

The MVP is category-general because it contains no humanoid-specific engine concepts. Asset templates include a crate, desk lamp, and stool; legacy implicit presets include a lamp, submarine, alien plant, and sky shrine. The core vocabulary is parts, generators, transforms, semantic edits, visual descriptors, candidates, validation relationship selectors, and revisions.

The MVP is still representation-specific: Asset Modeling Lab works on explicit `AssetRecipe` graphs, while the legacy mode works on implicit shape graphs. Imported triangle meshes are supported only in the same-topology deformation decompiler path.

## Non-Goals Before MVP Gate

- Natural-language modeling
- LLM integration
- General Blender integration beyond the decompiler reconstruction script
- General imported mesh editing without known vertex correspondence
- Rigging, animation, UVs, or texturing
- GPU compute or a custom GPU viewport
- Cloud or collaborative features
