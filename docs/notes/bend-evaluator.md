# Bend Evaluator Notes

## Scope

This wave implements only the schema-3 semantic evaluator for the narrow
uniform-curvature bend operator. It does not add bend inference, schema-3 package
writing, Blender-native deformers, or schema-2 behavior.

## Numerical Contract

- Parameter validation rejects non-finite inputs, zero-length axes, nearly
  parallel bend directions, non-positive intervals, and angles with magnitude
  greater than pi radians.
- Bend directions are orthogonalized against the longitudinal axis, normalized,
  and sign-canonicalized by flipping both `bend_direction` and `angle_radians`
  when that preserves the same deformation.
- The longitudinal axis sign is normalized and signed-zero-canonicalized, but it
  is not flipped: the sign defines which physical side of the interval remains
  unchanged and which side becomes the rigid tail.
- Angles with magnitude at or below `1e-7` radians return the original `f32`
  point bits exactly.
- Trigonometry and bend formula accumulation run in `f64`, with final positions
  converted to finite `f32` coordinates.
- Small `phi` values use series expansions for `sin(phi) / k` and
  `(1 - cos(phi)) / k` to avoid cancellation near the interval start and for
  near-zero non-identity bends.

## Continuity

The evaluator preserves position continuity at both interval boundaries. The
neutral-axis tangent is also continuous: the after-interval tail uses the
terminal bend tangent. This does not imply full off-axis Jacobian continuity, and
callers should not treat the bend as a globally smooth volume deformation.

## Semantic Stage Verification

`compare_bend_to_baked_stage` evaluates a bend and compares it to an
authoritative baked cumulative stage. Its tolerance policy mirrors the schema-2
approximate residual idea without depending on private schema-2 functions:

- absolute Euclidean tolerance floor
- intrinsic shape-relative tolerance floor
- local `f32` ULP floor computed from coordinates centered at each compared
  shape's centroid

The helper reports maximum component error, Euclidean error statistics, the
number of vertices outside tolerance, and the pass/fail result.
