# Modeling Next Steps

1. Broader constructive booleans
   - Constrained semantic plate cuts are present for recessed panels, rectangular through-cuts, and circular through-cuts.
   - Next add deterministic closed-solid union and broader subtract/intersect workflows without regressing provenance.
   - Preserve semantic provenance through boolean output faces and generated boundary loops.

2. Broader bevel support
   - Extend bevel language beyond current generator-specific edge treatments.
   - Support consistent bevel propagation across panels, trim, lathe rims, sweeps, and future boolean seams.

3. Loft and surface-patch generators
   - Add explicit lofts, ruled surfaces, and controllable patch grids for authored transitions.
   - Keep controls semantic rather than raw vertex manipulation.

4. User-authored part kits
   - Let users define reusable part definitions, sockets, replacement groups, optional groups, and parameter presets.
   - Keep kits serializable as explicit recipe fragments.

5. Structural imported-mesh decomposition
   - Decompose imported meshes into named structural regions and candidate part definitions.
   - Treat this as reconstruction into explicit asset recipes, not arbitrary mesh editing.

6. Transportable deformation programs
   - Add portable, semantic deformation programs that can move across compatible parts and templates.
   - Preserve revision history, parameter locks, and provenance.

7. Material and downstream pipelines
   - Add materials, UVs, texture generation, rigging, animation, and other downstream pipelines only after the structural modeling lane is robust.
