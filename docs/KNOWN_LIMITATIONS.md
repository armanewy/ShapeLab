# Known Limitations

- Only implicit shape-graph models are natively editable.
- Imported arbitrary meshes are not semantically editable.
- Topology is generated from the implicit field and is not stable between revisions.
- There are no UVs, materials, rigging, or animation.
- Candidate generation edits existing scalar parameters only; it does not add, remove, or replace structural parts.
- The viewport and thumbnails use a CPU renderer and are intentionally limited.
- User selections do not yet train a persistent preference model.
- Project files are versioned JSON, but there are no schema migrations yet.
- Autosave and crash recovery snapshots are not part of the MVP.
- The desktop app does not yet have automated window-level visual regression tests.
- Packaging notes and icons exist, but installers, code signing, and publishing are not implemented.

