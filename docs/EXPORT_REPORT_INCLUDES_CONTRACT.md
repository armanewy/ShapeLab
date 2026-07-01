# Export Report Includes Contract

Status: Phase A contract, active.

Every export or proof report that describes product capability must carry
explicit include/exclude flags. Missing capability proof must be represented as
`false`, not implied by copy.

## Required Include Flags

Reports should normalize into this includes contract:

- `includes_geometry`
- `includes_uvs`
- `includes_textures`
- `includes_material_looks`
- `includes_collision`
- `includes_gameplay_metadata`
- `includes_rig`
- `includes_skinning`
- `includes_animation`
- `includes_terrain_collision`
- `includes_godot_scene`
- `game_ready`
- `human_review_required`

## Geometry-Only Export v0

Geometry-only GLB export may set:

- `includes_geometry: true`
- `human_review_required: true`

Geometry-only GLB export must keep:

- `includes_uvs: false`
- `includes_textures: false`
- `includes_material_looks: false`
- `includes_collision: false`
- `includes_gameplay_metadata: false`
- `includes_rig: false`
- `includes_skinning: false`
- `includes_animation: false`
- `includes_terrain_collision: false`
- `includes_godot_scene: false`
- `game_ready: false`

The neutral placeholder material in a GLB is only a visibility aid. It is not a
material-look workflow, texture workflow, or surface workflow.

## Godot Proof v0

The Godot proof harness may report whether a mesh imported. It must not claim a
Godot-ready package from a blocked proof. In the current contract:

- `includes_godot_scene: false`
- `includes_collision: false`
- `includes_rig: false`
- `includes_animation: false`
- `game_ready: false`
- `human_review_required: true`

If Godot is unavailable, the proof status must be `Blocked`, not `Passed`.

## Report Failures

The product claim gate rejects a report if any unsupported include flag is true:

- UV support
- textures
- material looks
- collision or gameplay metadata
- rigging or skinning
- animation
- terrain collision
- Godot scene output
- blocked game-ready status

The gate also rejects reports that set `human_review_required: false`.

## Future Gates

Later phases may add real capability-specific reports, but they must update this
contract before product-facing copy can claim the capability. Until then, current
exports are geometry-only drafts and remain review-required.
