# Geometry-Only Export Contracts v0

Status: contract defined; exporter implementation comes next.

Geometry-only export is the first proof toward moving supported ObjectPlan drafts
from internal review evidence into an engine-importable asset package. V0 defines
the request, policy, report, and user-summary contracts. It does not add the GLB
writer, app UI, Godot proof harness, materials workflow, UV editing, collision,
rigging, animation, runtime LLM integration, or public catalog publishing.

## Contract Scope

`GeometryExportRequest` identifies a supported source:

- `CurrentPrimitive`
- `ObjectPlan`
- `MaterializedObjectDraft`

The request includes:

- `source_ref`
- `export_format`
- `export_policy`
- `output_name`
- `output_dir`

V0 supports `Glb` as the intended format. `GltfDirectory` is reserved for a
later proof and is blocked by the V0 validator.

## Export Policy

`GeometryExportPolicy` must keep V0 narrow:

- `geometry_only: true`
- `require_valid_materialization: true`
- `allow_placeholder_neutral_material: true`
- `forbid_textures: true`
- `forbid_uv_claims: true`
- `forbid_rigging: true`
- `forbid_animation: true`
- `forbid_collision_claims: true`
- `forbid_game_ready_claims: true`

A neutral placeholder material is allowed only so the mesh is visible in tools
that require a material slot. It is not a material look, surface workflow, or
texture claim.

## Supported V0 Sources

Supported primitive sources:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Panel with Knob composition when the materialized draft has no unresolved
  nodes or attachments
- ObjectPlan materialized drafts made only from supported primitives and
  supported compositions

Unsupported sources:

- material or surface packages
- UV or textured assets
- collision or gameplay metadata
- rigged or animated assets
- arbitrary mesh imports
- public catalog packages

## Blocking Rules

Export must block when:

- source materialization did not pass
- source materialization has unresolved nodes
- source materialization has unresolved attachments
- a source primitive is outside the V0 supported scope
- the request asks for textures or material looks
- the request asks for UV support claims
- the request asks for collision or gameplay metadata
- the request asks for rigging or animation
- the request asks for game-ready status
- the source contains a raw mesh payload outside the supported pipeline

## Export Report

`GeometryExportReport` must truthfully report:

- `status`: `Passed`, `Blocked`, or `Failed`
- `output_files`
- `source_plan_id`, when applicable
- `primitive_count`
- `mesh_count`
- `triangle_count`
- `warning_count`
- `blockers`
- `includes_uvs: false`
- `includes_textures: false`
- `includes_material_looks: false`
- `includes_collision: false`
- `includes_rig: false`
- `includes_animation: false`
- `game_ready: false`
- `human_review_required: true`

Any report that sets `game_ready: true` is invalid for V0.

## Product-Safe Summary

The V0 user summary may say:

- "Geometry-only GLB exported."
- "No textures, collision, rigging, or animation are included."
- "Godot import proof is required before calling this Godot-ready."

The summary must not imply that the output is textured, rigged, animated,
collision-enabled, Godot-ready, or game-ready.

## Separate Godot Gate

This contract does not prove Godot import. A later harness must import the GLB
into Godot or produce an honest blocked report. Until that proof passes, Shape
Lab must not claim Godot-ready geometry.
