# Kit Review Checklist

Use this checklist before enabling a kit in the default novice catalog.

## Automated Evidence

- `shape-cli foundry-kit validate <kit>`
- `shape-cli foundry-kit inspect <kit>`
- `shape-cli foundry-kit contact-sheet <kit> --out-dir <dir>`
- `shape-cli foundry-kit package <kit> --out-dir <dir>`
- `shape-cli hq-quality-benchmark --profile <slug> --out-dir <dir> --verify-export`

For kits prepared through Foundry Author Studio, also archive the Author Studio
package export manifest refs for provider packs, style packs, controls,
candidate strategies, quality gate profile, review manifest, quality reports,
and contact sheets. These refs should match the `shape-cli foundry-kit package`
output filenames.

## Review Criteria

- The kit compiles through the exact Foundry catalog path.
- Required roles and provider slots are covered.
- Required socket/port descriptors reference known family roles and include
  compatibility tags.
- Style compatibility does not mark the same provider/style tag or pack as both
  allowed and forbidden.
- Primary controls are product-sized, with at most seven visible primary
  controls by default.
- Every visible primary control has a meaningful whole-model effect.
- Topology-changing controls are discrete whole-model choices.
- Direction candidates are coherent whole-model alternatives.
- Candidate explanations use product-facing control labels, not scalar paths,
  provider IDs, semantic IDs, or operation IDs.
- Contact sheets are clay Shape Lab output, not photoreal reference images.
- Export and package-reopen evidence is present for Usable or Showcase claims.
- Any hidden or disabled state has a plain-language reason.
- No novice-facing text exposes technical authoring internals.

## Approval

Usable catalog exposure requires human approval in `KitReviewManifest`.
Showcase exposure and badges require both human approval and adversarial visual
review. Automation can collect evidence; it cannot approve Showcase quality by
itself.
