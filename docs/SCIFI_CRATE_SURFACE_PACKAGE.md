# Sci-Fi Crate Surface Package

The Sci-Fi Industrial Crate is the first Shape Lab asset with deterministic
headless UV/material/texture evidence.

The package records:

- source artifact fingerprint and recipe hash;
- the frozen model-package reference;
- one ready UV set with finite coordinates for the exported vertices;
- one triangle binding per triangle;
- six human-facing material slots;
- per-slot triangle coverage counts and fractions;
- five deterministic material recipes;
- four generated texture channels per recipe.

Material slots are:

- Painted Metal Body
- Dark Recesses and Vents
- Exposed Edge Trim
- Handle Grip
- Fasteners and Mounts
- Fallback Hard Surface

The generated textures are evidence-grade procedural payloads. They are not
artist-approved final materials and do not imply textured Visual Foundry output
for every asset family.

The package also emits a surface-aware GLB with `POSITION`, `NORMAL`, and
`TEXCOORD_0`. Texture files remain package sidecars, and engine-native material
graphs are not implemented.
