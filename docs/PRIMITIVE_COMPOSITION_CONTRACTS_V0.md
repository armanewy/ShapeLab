# Primitive Composition Contracts v0

Date: 2026-06-30

Primitive composition is constrained attachment, not scene modeling. A
composition document contains primitive nodes, bounded property values, named
anchors, and validated attachments between those anchors.

## Contract Shape

`PrimitiveCompositionDocument` defines:

- schema version
- document ID
- primitive nodes
- constrained attachments
- root node ID

`PrimitiveNode` defines:

- node ID
- primitive kind
- property values
- product-safe local label
- visibility

`PrimitiveAnchor` defines:

- anchor ID
- owning node ID
- display name
- anchor kind
- normalized location
- normal and tangent directions
- allowed child primitive kinds
- product-safe description

`PrimitiveAttachment` defines:

- parent node and parent anchor
- child node and child anchor
- bounded offset policy
- derived orientation policy
- child scale policy

## Initial Anchor Vocabulary

Flat Panel anchors:

- `front_center`
- `front_handle_zone`
- `left_side_handle_zone`
- `hinge_edge_zone`

Sphere anchors:

- `back_mount_point`
- `front_center`

Box anchors:

- `top_center`
- `front_center`
- `side_centers`

## Validation

Validators reject unknown node references, unknown anchors, incompatible child
primitive kinds, self-attachments, invalid property values, invalid normalized
anchor locations, unsafe labels, absolute paths, and raw transform payloads.

Child placement is derived from anchors. Users may adjust only bounded offsets
where a policy allows it. Raw matrices, arbitrary mesh operations, vertex/face
editing, and unrestricted object transforms are outside this contract.

## Future Use

Future LLMs may suggest compositions, but validators decide legality. A
suggested composition that references unsupported primitives, unknown
properties, unsafe anchors, raw transforms, or out-of-range offsets remains
invalid and cannot become current state.
