# Lidded Box Make Baseline Gate

Date: 2026-06-30

## Verdict

`PASS`

Lidded Box is now exposed in the Make loop as the first incremental upgrade
after Box Primitive. It is Box Primitive plus one visible Lid Seam feature. It
is not a crate.

## Product Truth

- Box Primitive remains available.
- Lidded Box appears as the second box starting point after Lid Seam evidence.
- Lidded Box copy says "A simple box with a visible lid seam."
- Make uses "Try lidded box ideas", "Use this box", "Adjust lid seam", and
  "Export Lidded Box".
- The Make screen hides part/focus chips, Material Looks, surface-only panels,
  Family Studio entry points, and crate/case language for Lidded Box.
- Export says the current clay lidded box asset is not a textured, rigged,
  animated, or game-ready package.

## Dogfood Flow

```text
Choose Lidded Box
-> Make ready
-> Try lidded box ideas
-> Use this box
-> Adjust Lid Seam
-> Add to Pack
-> Export Lidded Box
```

Recorded evidence:

```text
target/lidded-box-make-baseline-gate/
```

Key artifacts:

- `lidded-box-parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `quality-report.json`
- `evidence-manifest.json`

The automated dogfood flow is covered by:

```text
foundry::app::tests::lidded_box_make_baseline_flow_is_plain_and_complete
```

## Pass / Fail

| Check | Result |
| --- | --- |
| User can see lid seam | Pass |
| Box still reads as a box | Pass |
| Candidates generate and can be selected | Pass |
| Lid Seam control appears | Pass |
| Add to Pack works | Pass |
| Export drawer opens with Lidded Box copy | Pass |
| No crate/case language | Pass |
| No Material Looks panel | Pass |
| No part/focus chips | Pass |
| Export copy is truthful | Pass |

## Tests

Passed locally:

```text
cargo test -p shape-foundry-catalog --test box_primitive --jobs 1
cargo test -p shape-app foundry --jobs 1
```

Full prompt gate commands are listed in the branch final report.

## Next Allowed Work

The next visible feature may be exactly one module:

```text
Trim Band Feature Module v0
```

Still blocked:

- Feet / Skids
- panels, handles, latches, vents
- crate language
- Material Looks, UV/texturing, rigging, animation
- public Family Studio flow
- runtime LLM

After Trimmed Box passes, stop the box ladder and prove Door Primitive before
Family Studio Lite.
