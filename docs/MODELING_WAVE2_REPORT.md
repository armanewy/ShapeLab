# Modeling Wave 2 Report

Prompt: Wave 2, Prompt 2.1 - Compile and export the first explicit assets.

Branch: `codex/modeling-kernel-integration`

Baseline note: the requested baseline SHA `f1616c58c7e2d283bd0d94ab2b75b93ef3918c23` was not present locally or after fetch. The worktree was created from the available matching baseline prefix `f1616c547c76c6283c0eb77f0e1ed54639ed6c61` (`Resolve assembled preview element IDs`).

## Implemented

- Wired `shape-modeling` dispatch for `RoundedBox`, `Cylinder`, `Frustum`, `Plate`, `Sweep`, `Lathe`, and `LiteralMesh`.
- Reworked `shape-compile` around assembly evaluation so compile validates recipes, generates definitions, evaluates parent transforms and assembly operations, transforms meshes, validates local/world topology, triangulates for preview/export, builds provenance, records statistics, and returns typed contextual errors.
- Added grouped OBJ export and Blender reconstruction script generation.
- Added `shape-modeling-assets` with benchmark recipe constructors and checked-in JSON recipes.
- Added `shape-cli model-demo --asset industrial-crate|explicit-desk-lamp --out-dir ...`.
- Preserved the existing implicit editor path and schema-2/schema-3 bend/deformation decompiler paths.

## Benchmark Results

Industrial Crate:

- model-demo output: `target/model-demo/crate`
- parts: 21
- exported triangles: 2684
- budget: below 25,000
- validation issues: 0
- SDF/remeshing used: false
- Blender create/reopen: passed with Blender 4.5.10 LTS at `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe`

Explicit Desk Lamp:

- model-demo output: `target/model-demo/lamp`
- parts: 5
- exported triangles: 1092
- budget: below 20,000
- validation issues: 0
- SDF/remeshing used: false
- Blender create/reopen: passed with Blender 4.5.10 LTS at `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe`

## Output Contract

Each model-demo run writes:

- `recipe.json`
- `asset.obj`
- `provenance.json`
- `validation.json`
- `statistics.json`
- `preview.png`
- `blender_reconstruct.py`

The Blender script creates one object per part occurrence, creates meshes from canonical arrays, preserves object names, writes semantic custom properties, uses simple debug colors, verifies topology and positions, saves `reconstructed.blend`, and supports `--verify-reopen`.

## Verification

Final required verification:

- `cargo fmt --all --check`: passed
- `cargo test --workspace`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo build --workspace --release`: passed
- `cargo run -p shape-cli -- model-demo --asset industrial-crate --out-dir target/model-demo/crate`: passed, 21 parts, 2684 triangles
- `cargo run -p shape-cli -- model-demo --asset explicit-desk-lamp --out-dir target/model-demo/lamp`: passed, 5 parts, 1092 triangles
- `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe --background --python target/model-demo/crate/blender_reconstruct.py -- --out-dir target/model-demo/crate --verify-reopen`: passed, 21 objects
- `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe --background --python target/model-demo/lamp/blender_reconstruct.py -- --out-dir target/model-demo/lamp --verify-reopen`: passed, 5 objects
