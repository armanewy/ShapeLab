# Known Limitations

- Shape Lab now launches directly into Visual Foundry. The legacy implicit
  editor and explicit Modeling Workspace product surfaces were removed from the
  native app in Wave 31; their underlying research and CLI-era crates remain
  only where needed by tests, benchmarks, and strict reconstruction work.
- Imported arbitrary meshes are not semantically editable.
- Topology is generated from the implicit field and is not stable between revisions.
- General Visual Foundry assets do not have UVs, materials, rigging, or
  animation. The headless Sci-Fi Crate static-prop package is the narrow
  exception: Surface Lab v1 emits deterministic UV/material/texture evidence
  for that one profile only.
- Cargo Case is the first proven reusable clay family architecture, but only
  for equipment-case profiles. It does not prove a broad archetype system, and
  archetype materializer outputs remain internal drafts until validation,
  contact sheets, and human/adversarial review pass.
- Product Dogfood Gate v4 approves only the Sci-Fi Crate Make baseline. It does
  not approve broader Surface, UV/Texturing, Rigging, animation, or full
  game-ready product UI.
- Surface is not a general visual Directions mode. The Sci-Fi Crate can expose
  preview-only `Try material looks` candidates when the generated
  surface-candidate evidence package is present and matches the current crate
  build. These previews do not mutate geometry, shape controls, project state,
  pack contents, or export payloads.
- Candidate generation can propose semantic scalar edits, structural part choices, duplicated cuts, and grouped cut-operation edits, but it is still not a general-purpose modeler.
- The viewport and thumbnails use a deterministic CPU renderer and bounded
  preview caches. A native GPU viewport/render backend is still future work.
- User selections can feed the Wave 29 local-only Foundry preference profile,
  but there is no cloud preference model, hidden telemetry, or semantic rewrite
  learner.
- Asset recipe JSON has targeted migrations for older authored relationship and cut metadata. Broad cross-version project migrations are still limited.
- Asset and Foundry project files support deterministic sibling recovery
  snapshots, but automatic timed autosave UI and full crash-restore prompts are
  still limited.
- The desktop app has headless panel/reducer regression tests but does not yet
  have automated OS-window pixel regression tests. Wave 31.5 adds a headless
  product UI gate for default copy, flow, and shell evidence, but screenshots
  and human layout inspection remain manual.
- Packaging notes and icons exist, but installers, code signing, and publishing are not implemented.
- HQ quality reports are clay-mesh truth reports. They do not claim UVs,
  materials, textures, rigging, animation, photoreal renders, marketplace-ready
  packages, or automatic Showcase approval.
- HQ adversarial review records evidence gaps and required manual art-review
  questions. It cannot automatically decide whether output looks toy-like,
  art-directed, or better than a curated Blender/Houdini kit.
- The Sci-Fi Crate static-prop readiness package emits a real frozen model
  package, grouped OBJ, proxy LOD OBJs, collision proxy, visual evidence,
  geometry-only GLB, and a Surface Lab v1 package with deterministic UVs,
  material slots, procedural texture PNGs, and surface evidence. It is still
  blocked for a full game-ready claim because manual DCC/runtime review,
  engine import proof, and engine-native package handoff are not complete.
- `moba-hero-clay` remains clay mesh only. It does not claim Dota/IP
  reconstruction, UVs, materials, textures, rigging, animation, marketplace
  readiness, or arbitrary imported mesh editability.
- Schema-3 bend inference is experimental and limited to a single uniform-curvature bend plus at most one affine-family stage before or after it.
- Bend inference requires `--package-schema 3 --enable-bend`; schema 2 remains affine-only by default.
- Ambiguous affine/bend compositions may select deterministic approximation programs rather than the exact generating affine/bend order, then rely on the final lossless correction for exact replay.
- There are no falloff masks, multiple bends, arbitrary handle deformations, topology changes, vertex correspondence solving, Maya adapter, native Blender deformers, or LLM/reference-image workflows in the decompiler.
