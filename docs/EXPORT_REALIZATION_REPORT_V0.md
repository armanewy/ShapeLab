# Export Realization Report v0

Export Realization Report v0 makes geometry-only export reports state how authored relationships are represented in the exported package.

## Scope

- Supported geometry-only ObjectPlan exports now include `relationship_realizations`.
- Assets with no authored relationships report an empty list.
- Relationship-backed Panel with Knob exports report the authored surface-mounted relationship.
- Current GLB output is a combined geometry mesh, so Panel with Knob reports `child_output: CombinedMesh`.
- Relationship semantics are preserved in report/sidecar data for review.

## Truth Boundaries

- This does not add materials, textures, UV editing, collision, rigging, animation, or motion.
- This does not claim Godot-ready or game-ready output.
- `baked` remains `false` unless a later export gate proves a real bake path.
- Current Godot proof reports include hierarchy fields, but hierarchy inspection remains unchecked in V0 when the harness cannot prove it.

## Review Meaning

The realization summary answers how an authored relationship survived export:

- `relationship_id`
- `relationship_type`
- `realization_policy`
- `output_node`
- `output_mesh`
- `child_output`
- `baked`
- `semantics_preserved_in_sidecar`

For the current geometry-only GLB, this is evidence for review only. It is not approval, public catalog publishing, collision support, or a game-engine integration claim.
