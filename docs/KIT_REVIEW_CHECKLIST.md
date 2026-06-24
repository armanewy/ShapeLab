# Kit Review Checklist

Use this checklist before enabling a kit in the default novice catalog.

## Automated Evidence

- `shape-cli foundry-kit validate <kit>`
- `shape-cli foundry-kit inspect <kit>`
- `shape-cli foundry-kit contact-sheet <kit> --out-dir <dir>`
- `shape-cli foundry-kit package <kit> --out-dir <dir>`
- `shape-cli hq-quality-benchmark --profile <slug> --out-dir <dir> --verify-export`

## Review Criteria

- The kit compiles through the exact Foundry catalog path.
- Required roles and provider slots are covered.
- Primary controls are product-sized, with at most seven visible primary
  controls by default.
- Every visible primary control has a meaningful whole-model effect.
- Direction candidates are coherent whole-model alternatives.
- Contact sheets are clay Shape Lab output, not photoreal reference images.
- Export and package-reopen evidence is present for Usable or Showcase claims.
- Any hidden or disabled state has a plain-language reason.
- No novice-facing text exposes technical authoring internals.

## Approval

Usable catalog exposure requires human approval in `KitReviewManifest`.
Showcase exposure and badges require both human approval and adversarial visual
review. Automation can collect evidence; it cannot approve Showcase quality by
itself.
