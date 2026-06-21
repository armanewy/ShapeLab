# Assembly Runtime Notes

## Scope

This branch adds deterministic assembly evaluation in `shape-modeling::assembly`.
It is additive to the existing explicit modeling contracts and does not change
the implicit editor, schema-2 deformation decompiler, schema-3 bend code, or any
primitive generator implementation.

## Evaluation Model

- Enabled source instances are ordered deterministically from recipe roots and
  sorted child IDs.
- Each referenced `PartDefinition` is generated once through an injected
  `GeometryGenerator`; repeated instances reuse the local compiled part.
- Definition-local sockets and generator-returned sockets are merged before
  attachment solving, so declared sockets are available even when a fixture or
  future generator returns no socket payload.
- World meshes are cloned from the definition-local mesh, transformed per
  occurrence, and annotated with the occurrence `PartInstanceId` while preserving
  region and operation metadata.

## Attachments

`RigidSeparate` attachments align the child socket frame to the parent socket
frame. The attachment `local_offset` is applied after alignment in the aligned
socket frame. `WeldBoundaryReserved` returns an explicit unsupported assembly
error; boundary welding remains future work.

Attachment and parent cycles are rejected before assembly. Missing parent or
child sockets are rejected with typed missing-socket errors.

## Generated Occurrences

The assembly plan supports mirror, linear array, and radial array operations.
Generated instance IDs are allocated deterministically from
`max(recipe.next_ids.part_instance, max_existing_instance_id + 1)` in operation
and copy order.

- Mirror operations reflect prototype world transforms across an explicit plane,
  preserve the prototype definition, reverse polygon winding for negative
  determinant transforms, and record `generated_by` provenance.
- Linear arrays treat `count` as the total occurrence count including the
  prototype. Non-centered arrays generate copies at positive step indices.
  Centered arrays generate integer step indices around zero while leaving the
  prototype at index zero.
- Radial arrays place copies around an explicit center and axis over the angular
  span. When `rotate_instances` is true, occurrence orientation follows the
  radial rotation.

## Output

`AssemblyEvaluation` returns local compiled parts, occurrence records, world
transforms, world sockets, world meshes, a combined polygon preview mesh, a
triangulated preview mesh, per-instance bounds, and assembly provenance.
