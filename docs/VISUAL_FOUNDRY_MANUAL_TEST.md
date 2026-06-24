# Visual Foundry Manual Test

Run the native app:

```text
cargo run -p shape-app --release
```

## Roman Timber Bridge

1. Launch Shape Lab.
2. Choose Roman Timber Bridge from the Visual Foundry home screen.
3. Wait for the current preview.
4. Generate Explore directions.
5. Choose a reinforced-looking direction.
6. Open Customize.
7. Increase Deck Width.
8. Change Bracing Style.
9. Lock Deck Width.
10. Generate six new directions.
11. Confirm chosen directions remain whole-model cards and do not expose raw
    scalar paths.

## Sci-Fi Industrial Crate

1. Choose Sci-Fi Industrial Crate from the Visual Foundry home screen.
2. Generate Explore directions.
3. Choose a compact vented crate direction.
4. Open Customize.
5. Change Handle Style.
6. Increase Edge Softness.
7. Increase Panel Depth.
8. Open Pack.
9. Add Current Asset.
10. Create two more accepted variants and add each to the pack.
11. Batch Export to a folder.
12. Confirm each pack member folder contains `asset-manifest.json`.

## Stylized Furniture Lamp

1. Choose Stylized Furniture Lamp from the Visual Foundry home screen.
2. Generate Explore directions.
3. Choose a tall reading lamp direction.
4. Open Customize.
5. Change Stem Curvature.
6. Change Shade Style.
7. Increase Base Weight.
8. Export Current Asset.
9. Confirm the export folder contains `asset-manifest.json`.

## Wave 26 Expansion Profiles

For each profile below, choose it from the Visual Foundry home screen, wait for
the preview, open Customize, change at least one continuous control, change one
choice or toggle control, generate Explore directions, and confirm six
whole-model cards appear without a technical recipe surface.

- Market Stall Kit
- Sci-Fi Door Panel
- Coopered Storage Barrel
- Wayfinding Signpost
- Workshop Chair
- Market Handcart
- Storybook Tree

## Save, Reload, and History

1. Save As a `.shapelab-foundry.json` project.
2. Accept at least one direction.
3. Undo.
4. Accept a different direction to create a branch.
5. Save and reopen the project.
6. Confirm the branch history and current preview are restored after rebuild.

## Headless Gate

```text
cargo build -p shape-cli --release
target/release/shape-cli.exe foundry-visual-benchmark --profile roman-bridge --proposal-count 72 --out-dir target/visual-foundry-mvp/roman-bridge --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
target/release/shape-cli.exe foundry-visual-benchmark --profile sci-fi-crate --proposal-count 72 --out-dir target/visual-foundry-mvp/sci-fi-crate --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
target/release/shape-cli.exe foundry-visual-benchmark --profile stylized-lamp --proposal-count 72 --out-dir target/visual-foundry-mvp/stylized-lamp --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
```

Expected result: six Refine and six Explore candidates for each profile,
coherent three-member pack export, no invalid state becoming current, and
`verify_reopen: true` in each parent Blender verification report.
