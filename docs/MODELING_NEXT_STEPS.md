# Modeling Next Steps

1. Semantic cut composition and boundary treatment
   - Harden constrained semantic cuts before adding broader booleans.
   - Present: one boundary-loop ID per connected physical loop, target-region/host-face reconciliation, explicit rim width, explicit rectangular corner segments, and separate bevel eligibility metadata.
   - Support multiple non-overlapping cuts on one plate with cut-to-cut clearance validation.
   - Extend the same controlled cut path to rounded-box primary faces.
   - Propagate bevels through `bevel_eligible` boundary-loop metadata without overloading UV seam metadata.

2. GPU viewport default with CPU fallback
   - Prefer a hardware-accelerated interactive viewport when a compatible GPU backend is available.
   - Keep the deterministic CPU renderer as the fallback path for unsupported machines, tests, exports, thumbnails, and reproducible benchmark renders.
   - Expose renderer selection and fallback status in diagnostics so performance issues are visible instead of silent.

3. Broader constructive booleans
   - After controlled cuts are stable, add deterministic closed-solid union and broader subtract/intersect workflows without regressing provenance.
   - Preserve semantic provenance through boolean output faces and generated boundary loops.

4. Broader bevel support
   - Extend bevel language beyond current generator-specific edge treatments.
   - Support consistent bevel propagation across panels, trim, lathe rims, sweeps, and future boolean seams.

5. Loft and surface-patch generators
   - Add explicit lofts, ruled surfaces, and controllable patch grids for authored transitions.
   - Keep controls semantic rather than raw vertex manipulation.

6. User-authored part kits
   - Let users define reusable part definitions, sockets, replacement groups, optional groups, and parameter presets.
   - Keep kits serializable as explicit recipe fragments.

7. Structural imported-mesh decomposition
   - Decompose imported meshes into named structural regions and candidate part definitions.
   - Treat this as reconstruction into explicit asset recipes, not arbitrary mesh editing.

8. Transportable deformation programs
   - Add portable, semantic deformation programs that can move across compatible parts and templates.
   - Preserve revision history, parameter locks, and provenance.

9. Material and downstream pipelines
   - Add materials, UVs, texture generation, rigging, animation, and other downstream pipelines only after the structural modeling lane is robust.
