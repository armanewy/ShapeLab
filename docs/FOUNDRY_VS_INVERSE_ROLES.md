# Foundry vs Inverse Roles

Shape Lab has four distinct surfaces after Wave 25. Keeping them separate
prevents authored customization, compiler research, and import diagnostics from
being mistaken for one another.

## Visual Foundry

Audience:

- Novice and production-minded users who want coherent assets quickly.

Role:

- Generate whole-model directions.
- Expose authored controls and provider choices.
- Support locks, branches, packs, previews, and export.
- Hide technical recipe IDs unless the user opens Advanced Recipe.

Current scope:

- Roman Timber Bridge.
- Sci-Fi Industrial Crate.
- Stylized Furniture Lamp.

Success measure:

- A user can complete the core asset tasks by reading visible labels and using
  whole-model controls.

## Foundry Author

Audience:

- Technical content authors.

Role:

- Define asset families, style facets, provider vocabulary, attachment rules,
  conformance rules, controls, candidate strategies, preview cameras, and pack
  policies.
- Keep authored content reviewable and deterministic.

Current scope:

- Rust-authored profiles and style-kit bindings.
- Headless foundry benchmarks and conformance tests.

Future scope:

- A typed profile package format and authoring CLI.
- Validator, preview, and packaging commands for new family catalogs.

## Semantic Reconstruction Lab

Audience:

- Engine developers, researchers, and technical users evaluating import
  eligibility.

Role:

- Verify strict semantic programs.
- Recover covered synthetic hard-surface and known-base character cases.
- Diagnose why unsupported assets cannot be exact editable recoveries.
- Preserve useful failure evidence without hiding residual corrections.

Current scope:

- Strict semantic program IR.
- Hard-surface strict recovery gate.
- Known-base character recovery gate.
- External clean-character canonicalization and diagnostic classification.
- Product-facing external clean-character import triage.
- Same-topology deformation replay tooling.

Success measure:

- Exact success only when proof is complete.
- Otherwise, the report gives a specific, actionable failure.

## Prepared Template Customization

Audience:

- Users customizing an asset that was already prepared by a trusted authoring
  pipeline.

Role:

- Apply whole-model controls to prepared templates.
- Validate authored deformation cages, weights, landmarks, and base
  fingerprints before customization.
- Preserve required landmarks and emit deterministic cage-delta programs with
  no raw mesh payload.

Current scope:

- Known-base humanoid character template in `shape-character::prepared`.

Success measure:

- A user can adjust Body Proportions, Head Shape, Garment Fit, Pose Preset,
  Silhouette, and Detail Level without seeing cage, weight, vertex, or raw mesh
  internals.

## Boundary Rules

- Visual Foundry should not expose inverse compiler internals to novice users.
- Semantic Reconstruction Lab should not label diagnostic results as editable
  imports.
- Foundry Author should produce content packages and validation evidence, not
  one-off engine forks for every asset family.
- Prepared Template Customization should only operate on assets authored with
  cages, weights, landmarks, and current base fingerprints.
- Residual or correction-buffer workflows belong in diagnostics, not strict
  success.
- Import triage may suggest a foundry family only when the report includes a
  concrete family ID and evidence path. A suggestion is not recovery.
- Import triage may claim exact editable recovery only when the strict recovery
  proof embedded in the report accepts.

## Flow Between Surfaces

```text
Visual Foundry
  -> create, customize, pack, export authored families

Foundry Author
  -> add reviewed families and controls to Visual Foundry

Semantic Reconstruction Lab
  -> analyze imports and prove or reject strict recovery
  -> suggest known families only when evidence supports the suggestion

Prepared Template Customization
  -> customize trusted prepared templates through whole-model controls
  -> reject stale or cross-base preparation metadata
```

The surfaces can share semantic IDs, fingerprints, validation reports, and
export packages. They should not share user-facing claims unless the underlying
proof level matches the claim.
