# Visual Foundry MVP Report

Wave 10 delivers the whole-model visual family-foundry MVP inside Asset
Modeling Lab. The novice-facing path is now:

```text
Asset Modeling Lab
  -> Visual Foundry
  -> New / From Asset Family
  -> Directions
  -> Customize
  -> Pack
  -> Export
```

The older explicit Modeling Workspace remains available inside Asset Modeling
Lab, and the legacy implicit editor remains a separate top-level mode.

## Delivered Scope

- Built-in asset-family profiles:
  - Roman Timber Bridge
  - Sci-Fi Industrial Crate
  - Stylized Furniture Lamp
- Whole-model direction cards for Refine, Explore, Silhouette, Structure, and
  Detail requests where a profile has matching editable controls.
- Whole-model customizer controls with preview/apply/reset/lock actions.
- Branchable semantic history with undo, branch switching, save, save-as, and
  load.
- Native Advanced Recipe tab for technical IDs and binding paths.
- Native Export tab for the current asset.
- Native Pack tab with add-current-asset and real per-member package export.
- Background jobs for compile, preview, transient control preview, candidate
  generation, edit application, pack compilation, current export, and pack
  export.

## Verification

Commands run locally:

```text
cargo build -p shape-cli --release
target/release/shape-cli.exe foundry-visual-benchmark --profile roman-bridge --proposal-count 72 --out-dir target/visual-foundry-mvp/roman-bridge --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
target/release/shape-cli.exe foundry-visual-benchmark --profile scifi-crate --proposal-count 72 --out-dir target/visual-foundry-mvp/scifi-crate --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
target/release/shape-cli.exe foundry-visual-benchmark --profile stylized-lamp --proposal-count 72 --out-dir target/visual-foundry-mvp/stylized-lamp --blender-exe "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe"
```

Results:

| Profile | Refine | Explore | Primary controls | Provider options | Pack members | Invalid current state | Blender reopen |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| roman-bridge | 6 | 6 | 7 | 9 / 9 | 3 | false | true |
| scifi-crate | 6 | 6 | 7 | 0 / 0 | 3 | false | true |
| stylized-lamp | 6 | 6 | 7 | 0 / 0 | 3 | false | true |

All three runs reported `advanced_recipe_required: false` and candidate
survival rate `1.0`.

## Acceptance Tasks

Roman Bridge:

- The profile exposes deck width, bracing style, structural heft, support
  rhythm, railing, edge finish, and span controls.
- Direction generation returns six Refine and six Explore alternatives.
- Controls can be locked through the customizer deck, and search honors locks.

Sci-Fi Crate:

- The profile exposes body proportions, vent density, handle style, edge
  softness, panel depth, trim, and detail density.
- Pack export writes three member model packages after pack conformance passes.

Stylized Lamp:

- The profile exposes overall height, stem curvature, shade style, base weight,
  joint size, shade scale, and edge softness.
- The native New flow can open the Stylized Furniture Lamp profile.

## Remaining Product Gaps

- Native option tiles currently reuse the current whole-model thumbnail; the
  transient preview path shows sampled values, but persistent per-option cached
  thumbnails should be added.
- Native thumbnail textures are uploaded on every paint instead of cached by
  preview ID and build stamp.
- Candidate preview generation is still all-or-nothing if one selected
  candidate fails to compile or render.
