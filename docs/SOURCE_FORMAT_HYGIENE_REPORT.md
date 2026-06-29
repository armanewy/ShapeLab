# Source Format Hygiene Report

Date: 2026-06-29

This report audits the source and status files called out before the Cargo Case
architecture proof. The goal is raw-GitHub reviewability and future Codex edit
safety, not a behavior refactor.

## Results

| File | Physical Lines | Max Line Length | Audit-Friendly |
| --- | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 10808 | 172 | Yes |
| `README.md` | 357 | 149 | Yes |
| `docs/CURRENT_PRODUCT_STATUS.md` | 116 | 109 | Yes |
| `docs/SURFACE_CANDIDATE_V0_INTEGRATION_REPORT.md` | 107 | 168 | Yes |
| `docs/SCIFI_CRATE_VISUAL_SURFACE_CANDIDATES_V0.md` | 89 | 98 | Yes |
| `docs/SURFACE_MODE_DOGFOOD_V0_RESULTS.md` | 85 | 81 | Yes |
| `docs/NEXT_PRODUCT_STEP_AFTER_DOGFOOD_V4.md` | 49 | 80 | Yes |

## App Source

`crates/shape-app/src/foundry/app.rs` is not physically collapsed. It has normal
Rust formatting, more than ten thousand physical lines, and a maximum line length
below the 180-character hygiene limit used by the local app tests.

No behavior-preserving module split was required for this promotion branch. The
preferred future direction remains moving Make-specific code into smaller
modules when product work is already touching those surfaces.

## Markdown Source

The audited Markdown files are not physically collapsed and remain readable in
raw GitHub. Status docs now agree that:

- Sci-Fi Crate Make baseline passes Product Dogfood Gate v4.
- Sci-Fi Crate material-look preview baseline passes Surface Candidate v0.
- Material looks are preview-only and do not affect export payloads yet.
- Roman Bridge HQ remains `PreviewOnly`.
- Broad Surface, UV/Texturing, Rigging, Animation, and full game-ready product
  claims remain blocked.

Older historical docs outside this target list may still contain long lines.
They are not part of this hygiene gate.
