# Asset Project I/O Notes

- Explicit asset projects are stored separately from legacy `ShapeDocument` projects with the `.shapelab-asset.json` suffix and a `project_kind` value of `shape-lab.asset-project`.
- Each asset revision stores its parent, accepted `AssetEditProgram`, full `AssetRecipe` snapshot, compiled artifact hash, validation summary, label, and timestamp-free deterministic metadata. Restoring a revision reads the stored snapshot directly and does not replay prior edits.
- Project history is branchable: undo moves the current pointer to a parent without deleting children, and accepting an edit after undo creates a sibling revision.
- Save/load use deterministic pretty JSON, schema probing, future-schema rejection before full deserialization, and sibling temporary files for atomic replacement.
- `AssetProjectFile` tracks current path plus clean markers for save, save as, load, undo, switch, and accept-candidate dirty-state semantics.
- Autosave recovery snapshots are normal asset project files written to a deterministic sibling autosave path without changing dirty state.
- Export helpers compile the current revision on demand for model packages and grouped OBJ. Export success or failure does not clear dirty state.
- The app-side asset I/O bridge is intentionally separate from the current implicit editor flow so `.shapelab-asset.json` is not confused with legacy `.shapelab.json` project files.
