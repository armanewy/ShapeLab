# Surface Candidate Backend v0

Surface Candidate Backend v0 is a headless Sci-Fi Crate package path that
generates deterministic material-only candidates for future Surface UI work.
It does not enable Surface mode in the app.

## Outputs

The Sci-Fi Crate static-prop package writes candidate outputs under
`surface/variants/`:

- `candidates.json`
- `contact-sheet.png`
- `surface-candidate-report.json`
- one directory per material variant
- per-variant `material-override.json`
- per-variant `surface-artifact.json`
- per-variant `material-pack.json`
- per-variant `surface-delta.json`
- per-variant `validation.json`
- per-variant `textured-preview.png`
- per-variant generated texture files

Each candidate record includes a title, summary, changed material slots,
embedded surface delta, generated texture files, preview path, frozen mesh
fingerprint, and blocked full-ready status.

## Invariants

Candidates are material-only. They preserve the base frozen mesh fingerprint,
frozen mesh reference, UV sets, triangle indices, material slot vocabulary, and
triangle-to-slot bindings. Surface deltas cannot claim shape changes.

Duplicate-looking variants are rejected by candidate-set validation. Candidates
with missing preview evidence or leaked shape delta cannot pass the surface
delta gate.

The aggregate report explicitly records that Visual Foundry Surface mode is not
enabled by this headless evidence.

## Readiness Boundary

This backend is package evidence, not product UI enablement and not final
game-ready proof. Full-ready status remains blocked until manual review, engine
import proof, and engine-native handoff proof exist.

No UV unwrapping, texturing pipeline, imported mesh editing, plugin system,
server, browser code, Blender integration, LLM integration, rigging, animation,
or humanoid-specific engine concept is added here.
