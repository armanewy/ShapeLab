# Surface Preview Static Prop v2

Surface Preview v2 adds a deterministic headless CPU preview path for static
prop surface artifacts. It is currently wired to the Sci-Fi Crate package path
only.

The preview input is:

- frozen mesh positions, normals, and triangle indices;
- `TEXCOORD_0` coordinates from the Surface Lab artifact;
- triangle-to-material-slot bindings;
- material-slot-to-recipe bindings;
- decoded generated texture payloads from package PNG sidecars.

The output evidence is:

- `surface/textured-preview.png`;
- `surface/textured-contact-sheet.png`;
- `surface/material-slot-overlay.png`;
- `surface/surface-preview-report.json`.

The renderer is CPU-only and deterministic. It does not use GPU compute, app UI,
browser code, a server, or an external material system. Missing base-color
texture payloads are validation failures. Surface Preview uses bilinear sampling
for package previews because it reduces aliasing on generated UVs. Nearest
sampling remains available for diagnostics and tests where exact texel selection
is required.

This is preview evidence, not artist approval. Full game-ready status remains
blocked until manual review and engine/DCC import proof exist.
