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

Wave 0 intentionally opens only a bootstrap window. Later waves implement the visible modeling loop.

## CLI

```bash
cargo run -p shape-cli -- --help
```
