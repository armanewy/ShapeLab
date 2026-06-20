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
cargo run -p shape-app
```

The current headless vertical slice can generate visible demo artifacts from the CLI. The desktop app still opens a bootstrap shell until the Wave 3 and Wave 4 UI work is merged.

On Linux, native GUI builds may require the platform packages expected by `eframe`/`wgpu`/`winit` and file-dialog backends, such as X11/Wayland development libraries.

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

`shape-app` is the native `eframe`/`egui` shell. `shape-cli` is the headless integration driver.

## CLI

```bash
cargo run -p shape-cli -- --help
```

Generate deterministic demo artifacts:

```bash
cargo run -p shape-cli -- demo --preset desk-lamp --seed 42 --out-dir target/demo-lamp
cargo run -p shape-cli -- demo --preset toy-submarine --seed 42 --out-dir target/demo-submarine
cargo run -p shape-cli -- demo --preset alien-plant --seed 42 --out-dir target/demo-plant
```

Each run writes project JSON, OBJ meshes, PNG previews, a contact sheet, and a summary JSON file.

Validate and export:

```bash
cargo run -p shape-cli -- validate target/demo-lamp/project-after.json
cargo run -p shape-cli -- export target/demo-lamp/project-after.json --obj target/demo-lamp/export.obj --png target/demo-lamp/export.png
```

## Scope

The MVP is category-general because it contains no humanoid-specific engine concepts. Presets include a lamp, submarine, and alien plant, and the core vocabulary is nodes, primitives, transforms, tags, constraints, edits, candidates, and revisions.

The MVP is still representation-specific: it edits implicit shape graphs. Arbitrary imported triangle meshes are not semantically editable yet.

## Non-Goals Before MVP Gate

- Natural-language modeling
- LLM integration
- Blender integration
- Imported mesh editing
- Rigging, animation, UVs, or texturing
- GPU compute or a custom GPU viewport
- Cloud or collaborative features
