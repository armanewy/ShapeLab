# Modeling Next Steps

1. Ordered local operation execution and multi-cut composition
   - Harden constrained semantic cuts before adding broader booleans.
   - Present: one boundary-loop ID per connected physical loop, target-region/host-face reconciliation, explicit rim width, explicit rectangular corner segments, separate bevel eligibility metadata, and structural recipe edits for inserting, removing, duplicating, and moving local modeling operations.
   - Replace the current one-cut plate path with ordered local operation execution: generate a base source, apply operation stages in recipe order, validate each stage, then apply boundary treatments.
   - Present: multiple same-face plate cuts are composed when their frames are separated and their rectangular projections do not split another cut window.
   - Next: relax the aligned-projection constraint with a fuller local face subdivision composer.
   - The first acceptance benchmark is one plate containing one recessed rectangular panel, four circular fastener holes, and three rectangular vents.

2. Boundary-loop bevels
   - Present: boundary-loop lifecycle metadata distinguishes historically produced loops, live loops required in the final mesh, consumed loops, and replacement outputs.
   - Add boundary-loop-targeted bevel operations for cut entry, exit, floor, and rim loops.
   - Propagate bevels through `bevel_eligible` boundary-loop metadata without overloading UV seam metadata.
   - Emit bevel-band regions, replacement loops, safe-width diagnostics, deterministic topology, and complete provenance.

3. Rounded-box face cuts
   - Extend the controlled cut path to flat primary patches of rounded-box faces.
   - Keep cuts out of host bevel bands until the topology and provenance contracts are stable.

4. GPU viewport default with CPU fallback
   - Prefer a hardware-accelerated interactive viewport when a compatible GPU backend is available.
   - Keep the deterministic CPU renderer as the fallback path for unsupported machines, tests, exports, thumbnails, and reproducible benchmark renders.
   - Expose renderer selection and fallback status in diagnostics so performance issues are visible instead of silent.

5. Broader constructive booleans
   - After controlled cuts are stable, add deterministic closed-solid union and broader subtract/intersect workflows without regressing provenance.
   - Preserve semantic provenance through boolean output faces and generated boundary loops.

6. Broader bevel support
   - Extend bevel language beyond current generator-specific edge treatments.
   - Support consistent bevel propagation across panels, trim, lathe rims, sweeps, and future boolean seams.

7. Loft and surface-patch generators
   - Add explicit lofts, ruled surfaces, and controllable patch grids for authored transitions.
   - Keep controls semantic rather than raw vertex manipulation.

8. User-authored part kits
   - Let users define reusable part definitions, sockets, replacement groups, optional groups, and parameter presets.
   - Keep kits serializable as explicit recipe fragments.

9. Structural imported-mesh decomposition
   - Decompose imported meshes into named structural regions and candidate part definitions.
   - Treat this as reconstruction into explicit asset recipes, not arbitrary mesh editing.

10. Transportable deformation programs
   - Add portable, semantic deformation programs that can move across compatible parts and templates.
   - Preserve revision history, parameter locks, and provenance.

11. Material and downstream pipelines
   - Add materials, UVs, texture generation, rigging, animation, and other downstream pipelines only after the structural modeling lane is robust.
