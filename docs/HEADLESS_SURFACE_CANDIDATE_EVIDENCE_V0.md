# Headless Surface Candidate Evidence v0

This branch adds deterministic headless evidence for Sci-Fi Crate material-only
Surface candidates. It does not enable Surface mode in the app.

## Scope

Approved:

- Sci-Fi Crate only.
- Six material-only candidate looks.
- Deterministic textured previews and contact sheet.
- Surface delta reports that prove geometry did not change.

Not approved:

- app Surface mode;
- a material editor;
- broad UV or texturing support;
- rigging, animation, or motion UI;
- full game-ready claims.

## Candidates

- Clean Lab White
- Worn Hazard Yellow
- Dark Industrial Metal
- Field Blue Utility
- Graphite Cargo
- Orange Warning Trim

Each candidate preserves the frozen mesh fingerprint, UV coordinates, material
slot IDs, and triangle/material bindings. Candidates may change only material
recipes, color values, roughness/metalness values, deterministic texture
payloads, and material-only wear/accent metadata.

## Evidence Files

Aggregate files:

- `surface/variants/candidates.json`
- `surface/variants/contact-sheet.png`
- `surface/variants/surface-candidate-report.json`

Per variant:

- `material-override.json`
- `textured-preview.png`
- `surface-delta.json`
- `validation.json`

Compatibility outputs are still emitted:

- `surface-artifact.json`
- `material-pack.json`
- `textures/*.png`

## Validity Rules

A candidate is valid only when:

- `shape_delta_leak_detected` is false;
- frozen mesh fingerprint matches the baseline;
- textured preview evidence exists;
- generated texture payloads exist;
- the surface delta is not duplicate-looking or unsupported.

The aggregate report explicitly records
`visual_foundry_surface_mode_enabled: false`.

## Game-Ready Boundary

The static package remains blocked from full game-ready status without manual
review and engine import proof. Required blockers include:

- `manual_review_pending`
- `engine_import_proof_missing`
- `engine_native_package_not_implemented`
- `surface_manual_review_required`
