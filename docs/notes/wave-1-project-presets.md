# Wave 1 Project Presets Notes

## Project History

- `Project::new` creates revision `RevisionId(0)` and stores the initial `ShapeDocument` snapshot directly.
- Accepted candidates create monotonically increasing child revisions and preserve the candidate `EditProgram` as revision metadata.
- Undo changes only the current revision pointer. Existing children remain in the `BTreeMap`, so accepting a new candidate after undo creates a branch.
- Project JSON uses schema version `1`, deterministic `BTreeMap` revision ordering, and validates loaded revision graph invariants before returning a project.
- UI helpers added: `current_document`, `can_undo`, `revision_path_to_root`, `revision_path_to_root_from`, `persistence_marker`, and `is_dirty_since`.

## Preset Structures

- Desk Lamp: root `Union` combines a rounded cylinder base, torus rim, a `SmoothUnion` stem assembly, and a rounded-box shade. The stem assembly contains two angled capsules and a spherical joint.
- Toy Submarine: root `SmoothUnion` combines a difference-based hull with porthole recess cutters, a rounded conning tower, a small periscope cylinder, and three rounded-box fins.
- Alien Plant: root `SmoothUnion` combines a scaled bulb, central capsule stem, angled branching capsules, three pod spheres, and two flattened rounded-box blades.

All presets use only generic shape graph nodes, primitive parameters, transforms, names, and tags. Node counts and primitive/CSG mixes differ across the three presets, and every node is reachable from its root.

## Contract Issues

- This worktree's current `shape-core::validate_document` is still the bootstrap implementation and does not yet enforce dangling references, cycles, primitive ranges, or full CSG validity. `shape-project` and `shape-presets` call `validate_document` at their validation boundaries and add local tests for reachable preset nodes, but stricter graph rejection depends on merging the Wave 1 core implementation.
- `Project::new` keeps the established infallible bootstrap signature. A fallible `Project::try_new` was added for callers that need explicit initial-document validation.
