# Sci-Fi Crate Visual Surface Candidates v0

Date: 2026-06-29

## Scope

This branch exposes material-look previews for the Sci-Fi Industrial Crate only.
It does not enable broad Surface mode, a material editor, general UV/texturing,
rigging, animation, or engine-native export.

## Enablement

The Make UI can open material looks only for a prepared Sci-Fi Crate. The app
then loads the generated headless evidence from:

`target/surface-candidate-evidence-v0/sci-fi-crate/surface/variants/surface-candidate-report.json`

The loader rejects the package unless:

- the report is for `sci-fi-crate`;
- all six approved candidate titles are present;
- textured preview PNGs load;
- per-candidate validation and surface-delta files pass;
- the frozen mesh fingerprint matches the current crate build;
- no candidate has `shape_delta_leak_detected`;
- full game-ready status remains blocked with manual review and engine import
  proof blockers.

Roman Bridge, Stylized Lamp, gear, hero, and any other profile remain disabled.

## Product UI

When the crate is ready, Make offers the secondary action `Try material looks`.
The primary shape idea flow remains unchanged.

The material tray uses product-facing copy:

- `Material looks`
- `Surface only`
- `Geometry unchanged`
- `Current Material`
- `Candidate Material`

Approved candidate titles:

- Clean Lab White
- Worn Hazard Yellow
- Dark Industrial Metal
- Field Blue Utility
- Graphite Cargo
- Orange Warning Trim

The UI shows textured previews and a material-summary comparison. It does not
show UV set names, material slot IDs, texture file paths, GLTF primitive terms,
or shape-change claims.

## Behavior

Material looks are preview-only in this build. Comparing a material look changes
only app-local preview selection state. It does not mutate geometry, shape
controls, document state, current build fingerprint, pack contents, or export
payloads.

The export drawer states:

`Material looks are preview-only in this build and will not affect export.`

It also states:

`Full game-ready remains blocked until manual review and engine import proof.`

If the evidence package is missing, the tray says:

`Material looks are not generated yet.`

and shows the static package command to run.

## Verification

Automated coverage includes:

- missing evidence rejection;
- valid Sci-Fi Crate evidence loading;
- Roman Bridge and Stylized Lamp exclusion;
- geometry fingerprint mismatch rejection;
- shape-delta leak rejection;
- approved product copy and candidate refs;
- preview-only selection preserving current geometry/control state;
- export copy remaining truthful.
