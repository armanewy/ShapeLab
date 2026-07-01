# Safe Primitive Attachment Policy

Date: 2026-06-30

Safe primitive attachment lets users combine primitives through named places
without turning Shape Lab into a freeform modeling tool.

## Allowed

- Attach a child primitive to a parent primitive anchor.
- Use product-facing anchor names such as handle zones or center points.
- Derive placement and orientation from the selected anchors.
- Adjust offsets only inside bounded normalized ranges.
- Keep child size controlled by that child primitive property schema.
- Reject invalid attachments before they become current state.

## Not Allowed

- Arbitrary free transforms.
- Raw matrices.
- Vertex, face, loop, cage, sculpt, or boolean mesh operations.
- Raw topology or mesh payloads in user-facing plans.
- Absolute file paths or publishing metadata in composition documents.
- Runtime LLM control over geometry generation.

## Product Boundary

Composition is a structured graph of primitive nodes and safe attachments.
Users attach primitives through named places; they do not manipulate scene
objects like a DCC tool.

LLMs may draft candidate composition documents later, but they do not bypass
primitive schemas or anchor compatibility. Validators remain the authority.
