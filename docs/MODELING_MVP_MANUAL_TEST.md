# Asset Modeling Lab Manual Test

## Launch

1. Run `cargo run -p shape-app`.
2. Confirm the app opens in **Asset Modeling Lab**.
3. Confirm the startup template choices show:
   - Industrial Crate
   - Explicit Desk Lamp
   - Stylized Stool

## Core Workflow

1. Choose **Industrial Crate**.
2. Confirm the named part tree appears on the left.
3. Select `crate body`.
4. In the inspector, increase `Body width`.
5. Select `left swept side handle`.
6. Increase `Handle thickness`.
7. Select `front fastener row`.
8. Change `Bolt count`.
9. Select an optional skid rail trim part.
10. Disable the optional part.
11. Select `crate body` and enable `Lock part`.
12. Click **Explore**.
13. Confirm six candidate slots appear and generated changes avoid locked body parameters.
14. Choose one candidate.
15. Click **Undo**.
16. Generate or choose a different direction to create a branch.
17. Confirm History shows a branch point.

## Save And Reload

1. Click **Save As** and save a `.shapelab-asset.json` project.
2. Close or switch templates.
3. Click **Open** and load the saved file.
4. Confirm:
   - current template state is restored
   - branch history is restored
   - selected parts and parameters remain editable

## Export

1. Use **Export -> OBJ** and choose an `.obj` path.
2. Confirm the OBJ file exists and contains part object/group records.
3. Use **Export -> Canonical Package** and choose an output folder.
4. Confirm the package contains:
   - `asset-manifest.json`
   - `recipe.json`
   - `provenance.json`
   - `validation.json`
   - `blender_reconstruct.py`
   - `parts/*.meshbin`

## Blender Verification

Run the generated reconstruction script with Blender:

```powershell
& "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe" --background --python "<package>\blender_reconstruct.py" -- --out-dir "<package>\blender-check" --verify-reopen
```

Expected result: Blender exits successfully and prints JSON containing `verify_reopen: true`.

## Required Commands

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```
