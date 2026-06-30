# Box Primitive UI Truth Pass

Date: 2026-06-30

## Goal

Make the first Make loop read as one honest object workflow:

```text
Choose Box Primitive
-> Make ready
-> Try box ideas
-> Use one idea
-> Adjust Proportions or Edge Softness
-> Add to Pack
-> Export
```

This pass keeps Box Primitive as a closed clay box-like volume. It does not add
a crate, case, part-focus workflow, material looks, surface UI, UV/texturing,
rigging, animation, runtime LLM behavior, or Family Studio entry point.

## Product Truth

- The novice Choose screen has a single large Box Primitive starting point.
- Category filters, template counts, catalog-browser search, and broad catalog
  wording are hidden in single-profile mode.
- The Make screen keeps the model stage, Try box ideas, candidate comparison,
  Proportions, Edge Softness, Add to Pack, and Export.
- Box Primitive does not show part chips, Body selectors, Focus buttons,
  focused generation, Material Looks, or surface-only panels.
- Export copy is specific to the clay box asset:

```text
Export Box Primitive
Exports the current clay box asset.
This is not a textured, rigged, animated, or game-ready package.
```

## Screenshot Gate

Evidence path:

```text
target/box-primitive-ui-truth-pass/screenshots/
```

| Screenshot | SHA-256 |
| --- | --- |
| `choose_box_primitive.png` | `534f65a07fe37d5e19cc4f61b220b9dc65213bacac59cf8c850f0185eff4eae2` |
| `make_ready_box_primitive.png` | `c6554e9280bb5d86fcefda6a8aa96720851c06fad307e9064a4ba0b3eae42a7d` |
| `generating_box_ideas.png` | `ecc23097d48bf31e6a0163f08dd71df13b68438b14790b7200be1c71e7bbeb0c` |
| `generated_box_ideas.png` | `a3961c552d54e9093836e6e0d0e475a5d623aaf3d258e946beb859ffad4382d6` |
| `selected_box_idea.png` | `b1773a1f476a47703b1060f836a9092253af203251010fe7e54f69838925144a` |
| `adjusted_box_control.png` | `b7ff0db02a7798a94c424f6b1dd38f9fa89ddce990a77b39486d8a5bf808b190` |
| `pack_drawer.png` | `2b4ee22687c50bab0460e48749d45d6b30112ccd493121a02e2c6abf8e4706e8` |
| `export_drawer.png` | `30fedc9f585707963a0e889a9b4a9cb651692899f6232c4d2f6e565b39c667b7` |

## Manual Gate

| Check | Result |
| --- | --- |
| No category chips | Pass |
| No Body chip | Pass |
| No material-look panel | Pass |
| No surface-only panel | Pass |
| No crate language | Pass |
| No Family Studio entry point | Pass |
| Next action is identifiable within five seconds | Pass |
| Ideas visibly differ | Pass |
| Export copy is truthful | Pass |

## Tests

The app tests now cover the single-profile catalog and Box Primitive UI truth:

- Box Primitive is the only novice catalog profile.
- Choose has no category filters in single-profile mode.
- Make has no Box Primitive part chips or focus controls.
- Make has no Material Looks or surface-only panels for Box Primitive.
- Export does not mention material looks for Box Primitive.
- Default Box Primitive UI strings avoid crate/case wording.
- Default Box Primitive UI strings avoid unsupported positive claims about
  UV/texturing, rigging, animation, or game-ready output.
- Try box ideas, Add to Pack, and Export remain covered.

## Next Work

The next branch may improve Box Primitive visual readability. New visible
features remain blocked until the Box Primitive baseline cleanup integration
passes.
