# Prepared Template Customization

Prepared template customization is a whole-model editing surface for assets that
were authored against Shape Lab semantic contracts before the user customizes
them. It is not an arbitrary mesh editing or import-recovery path.

## Contract

A prepared character template records:

- The character base library version and fingerprint.
- Exact versioned base fingerprints.
- Landmark bindings required by the prepared asset.
- Deformation cages and weight sets authored against those bases.
- A small novice-facing control set.

Wave 25 adds the first prepared known-base character contract in
`shape-character::prepared`. The built-in humanoid template exposes these
primary controls:

- Body Proportions
- Head Shape
- Garment Fit
- Pose Preset
- Silhouette
- Detail Level

Customization requests produce a deterministic prepared customization program.
The program references authored cage and weight-set IDs, preserves required
landmarks, and explicitly carries no raw mesh payload.

Rejected customization requests do not emit cage deltas. Invalid templates,
wrong template IDs, unknown controls, duplicate controls, wrong value domains,
and out-of-range values return an inert no-raw-mesh program with rejection
reasons.

## Validation Boundaries

Prepared templates must validate before customization:

- The template schema, base library version, and base library fingerprint must
  match the current known-base library.
- Every referenced base fingerprint must match the current base contract.
- Landmark bindings, cages, and weight sets must reference known IDs from the
  same owning base.
- Cages must have finite ordered bounds and enough control points.
- Weight sets must record normalized weight-sum tolerances.
- The primary user controls must be exactly the six whole-model controls listed
  above.

These checks are intentionally strict. They prevent a prepared template from
becoming a disguised residual buffer or a target-derived mesh edit.

## Product Rules

Use "Customize prepared template" for this surface.

Do not use:

- "Edit any mesh"
- "General mesh import"
- "Automatic proceduralization"
- "Recovered editable mesh"

Prepared template customization may coexist with import triage, but it does not
turn an arbitrary external mesh into an editable Shape Lab program. External
meshes remain diagnostic-only unless a strict recovery proof accepts.
