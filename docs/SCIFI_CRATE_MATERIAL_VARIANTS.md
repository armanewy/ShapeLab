# Sci-Fi Crate Material Variants

The Sci-Fi Crate static-prop package now emits headless material-only variants
under `surface/variants/`.

Generated variants:

- Clean Lab White
- Worn Hazard Yellow
- Dark Industrial Metal
- Field Blue Utility
- Graphite Cargo
- Orange Warning Trim

Each candidate preserves the frozen mesh fingerprint, UV coordinates, triangle
indices, material slot vocabulary, and triangle-to-slot bindings. The generator
only changes deterministic material recipe colors and variant-local generated
texture payloads.

Per-variant outputs include:

- `material-override.json`
- `surface-artifact.json`
- `material-pack.json`
- `textured-preview.png`
- `surface-delta.json`
- `validation.json`
- `textures/*.png`

The candidate set is written to `surface/variants/candidates.json`, and the
variant preview contact sheet is written to `surface/variants/contact-sheet.png`.
The aggregate validation and UI-boundary report is written to
`surface/variants/surface-candidate-report.json`.

Duplicate-looking or unsupported candidates must be marked with diagnostics; a
surface delta cannot claim a shape delta.

This is headless evidence only. It does not enable app Surface mode, a material
editor, broad texturing support, or full game-ready status.
