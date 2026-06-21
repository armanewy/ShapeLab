# Known Limitations

- The legacy implicit shape-graph editor remains available, and Asset Modeling Lab now provides the primary explicit asset-editing workflow.
- Imported arbitrary meshes are not semantically editable.
- Topology is generated from the implicit field and is not stable between revisions.
- There are no UVs, materials, rigging, or animation.
- Candidate generation can propose semantic scalar edits, structural part choices, duplicated cuts, and grouped cut-operation edits, but it is still not a general-purpose modeler.
- The viewport and thumbnails use a CPU renderer and are intentionally limited.
- User selections do not yet train a persistent preference model.
- Asset recipe JSON has targeted migrations for older authored relationship and cut metadata. Broad cross-version project migrations are still limited.
- Autosave and crash recovery snapshots are not part of the MVP.
- The desktop app does not yet have automated window-level visual regression tests.
- Packaging notes and icons exist, but installers, code signing, and publishing are not implemented.
- Schema-3 bend inference is experimental and limited to a single uniform-curvature bend plus at most one affine-family stage before or after it.
- Bend inference requires `--package-schema 3 --enable-bend`; schema 2 remains affine-only by default.
- Ambiguous affine/bend compositions may select deterministic approximation programs rather than the exact generating affine/bend order, then rely on the final lossless correction for exact replay.
- There are no falloff masks, multiple bends, arbitrary handle deformations, topology changes, vertex correspondence solving, Maya adapter, native Blender deformers, or LLM/reference-image workflows in the decompiler.

