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

- `surface-artifact.json`
- `material-pack.json`
- `textured-preview.png`
- `surface-delta.json`
- `textures/*.png`

The candidate set is written to `surface/variants/candidates.json`, and the
variant preview contact sheet is written to `surface/variants/contact-sheet.png`.
Duplicate-looking or unsupported candidates must be marked with diagnostics; a
surface delta cannot claim a shape delta.
