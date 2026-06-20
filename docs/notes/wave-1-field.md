# Wave 1 Field Notes

## Scope

Implemented the `shape-field` implicit compiler behind the Wave 0 public contract:

- Compact immutable arena with `NodeId` references resolved to `usize` indices.
- Analytic SDFs for sphere, rounded box, Y-axis capsule, rounded capped Y-axis cylinder, and Y-axis torus.
- CSG union, intersection, difference, and polynomial smooth union.
- Per-node transforms applied to full subtrees, including inverse translation, XYZ Euler rotation, inverse scale, and minimum-absolute-scale distance correction.
- Conservative compiled AABBs and deterministic X-fastest grid sampling.

## Assumptions

- `PrimitiveKind::RoundedBox::half_extents` and `PrimitiveKind::Cylinder::half_height`/`radius` describe the total intended half dimensions. The `roundness` parameter rounds inside those dimensions rather than expanding the total bounds.
- `PrimitiveKind::Capsule::half_length` is the half length of the center segment between spherical caps, so capsule bounds extend by `radius` along local Y.
- Disabled nodes compile to empty fields. Their references are still structurally validated because the core contract says referenced nodes must exist.
- Negative transform scale is allowed as long as every absolute component is safely non-zero. Distance correction uses the minimum absolute component as specified.

## Contract Issues

- In this worktree, `shape-core::validate_document` is still the Wave 0 minimal implementation and does not yet reject all dangling references, cycles, invalid primitive dimensions, invalid CSG arity, or unsafe scales. `shape-field` first honors `shape_core::validate_document`, then performs field-specific defensive validation before compiling so invalid data cannot reach sampling.
