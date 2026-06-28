# Sci-Fi Crate Dogfood Hardening v3

## Scope

Prompt 4A was limited to the Sci-Fi Crate catalog, its crate-specific tests, and this documentation. No app, surface, rig, motion, or runtime LLM code was changed.

## Changes

- Whole-asset strategies now combine more visible controls so returned crate ideas differ by body silhouette, panel relief/spacing, vents, handles, detail density, and edge treatment.
- Panel Depth was widened and a secondary non-primary Panel Spacing control was added so paired front panels visibly move apart or together without increasing the primary app control surface.
- Part-group reporting is now crate-local and explicit instead of inheriting broad built-in claims.
- Body is reported as focused shape candidate-ready and is tested against actual focused candidate generation.
- Panels are reported as inspection-only, not candidate-ready. Blocker: Panels have visible depth and spacing controls, but focused panel search currently collapses to one surviving idea in this build.
- Handles are reported as inspection-only, not candidate-ready. Blocker: Handles have authored style variants, but focused handle search currently collapses to one surviving idea in this build.
- Vents are focusable for inspection, not candidate-ready, with the required reason: “Vents can be adjusted through Vent Density, but focused vent ideas are limited in this build.”
- Edge Trim and Fasteners are not exposed as focused candidate-ready because at least two useful focused candidates have not been proven.

## Adversarial Review

- Overclaim risk: reduced. The catalog no longer claims focused candidate readiness for Panels, Handles, Vents, Edge Trim, or Fasteners.
- Whole-asset difference risk: mitigated by broader strategy control sets and existing candidate tests that reject returned TooSubtle whole-asset ideas.
- Handle plausibility risk: covered by tests that compile all handle styles and assert handle parts sit near the front body shell.
- Remaining weakness: focused Panels and Handles are visibly adjustable but not candidate-ready until the focused generator can return at least two non-duplicate scoped ideas for each group.
- Remaining weakness: Vents, Edge Trim, and Fasteners have useful controls for normal crate variation, but their focused candidate surfaces are still too limited or not isolated enough to expose honestly.
