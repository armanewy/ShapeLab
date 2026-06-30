# Hinged Panel Make Baseline Gate

Date: 2026-06-30

## Verdict

`PASS`

Hinged Panel is now exposed in Make as the first single-feature upgrade after
Flat Panel Primitive. It is Flat Panel Primitive plus one visible Hinge Edge
feature. It is not Door, does not claim open/close motion, and does not expose
material, surface, rigging, animation, or Family Studio UI.

## Scope

Allowed:

- Show Hinged Panel after Flat Panel Primitive.
- Keep Proportions, Edge Softness, and Hinge Edge.
- Use simple copy: Try hinged panel ideas, Use this panel, Adjust hinge edge,
  Adjust proportions, Export Hinged Panel.
- Keep Add to Pack and Export.

Blocked:

- Door naming.
- Handle, knob, inset panel, open/close motion, rigging, animation, materials,
  textures, or game-ready claims.
- Part/focus chips, Material Looks, surface-only panels, or Family Studio entry
  points.

## Manual Gate

Release app validation was run with Computer Use. Evidence is under:

```text
target/hinged-panel-make-baseline-gate/
```

Captured checkpoints:

- `choose_hinged_panel.png`
- `choose_hinged_panel_selected.png`
- `make_ready_hinged_panel.png`
- `generating_hinged_panel_ideas.png`
- `generated_hinged_panel_ideas.png`
- `selected_hinged_panel_idea.png`
- `adjusted_hinged_panel_proportions.png`
- `pack_drawer.png`
- `export_drawer.png`

## Pass Table

| Gate | Result |
| --- | --- |
| Hinged Panel appears after Flat Panel Primitive | Pass |
| Hinge edge is visible in the Make preview | Pass |
| Candidate ideas visibly differ | Pass |
| Hinge Edge control appears | Pass |
| No Material Looks panel appears | Pass |
| No focus chips appear | Pass |
| No Door naming appears | Pass |
| No open/close motion appears | Pass |
| Add to Pack works | Pass |
| Export drawer works | Pass |
| Export says it is not textured, rigged, animated, or game-ready | Pass |

## Notes

The manual adjustment checkpoint used Proportions because the gate permits
adjusting Hinge Edge or Proportions, and the Proportions options were fully
visible in the release-app viewport. The Hinge Edge control remained visible in
the Make panel.

## Next Work

Run the second-kernel integration gate. If it passes, the next allowed visible
feature can be Door Handle / Knob. Still blocked: Door naming, open/close
motion, material looks, UV/texturing, rigging, animation, broad archetype
expansion, and Family Studio public flow.
