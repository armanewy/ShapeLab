# Shape Lab

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling.

The MVP proves a category-independent loop:

1. Open a local desktop app.
2. View a non-humanoid procedural object.
3. Generate several coherent futures.
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

- a rendered current-shape viewport with orbit, pan, zoom, fit, and resize-triggered rerenders
- preset loading for Desk Lamp, Toy Submarine, Alien Plant, and Sky Shrine
- an outliner, inspector, revision history, status bar, and candidate gallery
- background preview, render, and candidate generation jobs that keep the UI responsive
- JSON project save/open and OBJ export

Startup loads the Desk Lamp preset and builds the first preview in the background.

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
AssetRecipe -> shape-modeling generators + assembly -> shape-compile exports
```

That lane is additive to the implicit editor and same-topology decompiler. Its first benchmark assets live in `crates/shape-modeling-assets`.

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
```

Each `model-demo` run writes `recipe.json`, grouped `asset.obj`, `provenance.json`, `validation.json`, `statistics.json`, `preview.png`, and `blender_reconstruct.py`.

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

The MVP is category-general because it contains no humanoid-specific engine concepts. Presets include a lamp, submarine, alien plant, and sky shrine, and the core vocabulary is nodes, primitives, transforms, tags, constraints, edits, candidates, and revisions.

The MVP is still representation-specific: the main editor works on implicit shape graphs. Imported triangle meshes are supported only in the same-topology deformation decompiler path.

## Non-Goals Before MVP Gate

- Natural-language modeling
- LLM integration
- General Blender integration beyond the decompiler reconstruction script
- General imported mesh editing without known vertex correspondence
- Rigging, animation, UVs, or texturing
- GPU compute or a custom GPU viewport
- Cloud or collaborative features
