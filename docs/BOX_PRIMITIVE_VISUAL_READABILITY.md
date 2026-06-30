# Box Primitive Visual Readability

Date: 2026-06-30

## Goal

Improve Box Primitive readability while keeping the object a pure-clay closed
box-like volume.

This pass does not add a lid seam, trim band, feet, panels, handles, material
looks, UV/texturing, rigging, animation, or a new profile family.

## Change

- Foundry clay previews now use stronger directional face shading.
- Foundry clay previews use a display-only edge outline derived from depth and
  normal discontinuities.
- The outline is a viewport aid. It does not alter geometry, export payloads,
  material support, or model semantics.
- Box Primitive candidate naming now uses `Sharp Box`, matching the intended
  six baseline idea names.

## Candidate Ideas

- Compact Box
- Wide Box
- Tall Box
- Flat Box
- Soft-Edged Box
- Sharp Box

## Evidence

Generated with:

```bash
cargo run -p shape-cli -- box-primitive-visual-readability --out-dir target/box-primitive-visual-readability
```

| Artifact | SHA-256 |
| --- | --- |
| `target/box-primitive-visual-readability/parent.png` | `f604a9c80bb3a63fa9e28aa2a502b51f7f48afae3492ea589e42ca948e18b2f6` |
| `target/box-primitive-visual-readability/candidate-contact-sheet.png` | `2b3fb6d058a0d7adcd0740d21d3bf092248009332d27245428b9ea9c68843745` |
| `target/box-primitive-visual-readability/control-endpoint-sheet.png` | `c420a83b9772036e1c3bc680c54a388677569538bf8903790220d98ae37e04a1` |
| `target/box-primitive-visual-readability/readability-report.json` | `40afa89ada5b3a480257a5d8b465bb5158ba35d89e7bbc24555d2a237ac33486` |

## Report Summary

| Check | Result |
| --- | --- |
| Does it read as a box? | Pass |
| Are width, depth, and height visible? | Pass |
| Are edges readable? | Pass |
| Do candidates differ? | Pass |
| Did we avoid crate features? | Pass |
| Export clean? | Pass |

## Notes

The contact sheet labels use compact codes because the existing CLI contact
sheet font is intentionally tiny. The report JSON records the full candidate
labels.
