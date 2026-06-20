# Schema 3 Blender Bend Verification

The schema-3 Blender adapter treats bend operators as semantic verification
only. Editable shape-key geometry is still loaded from each cumulative baked
positions sidecar, and the baked reconstruction object is built from the final
baked stage.

For bend stages, the generated Python now mirrors the narrow Rust evaluator:

- validates finite parameters, strict positive interval length, angle magnitude
  at most pi radians, non-degenerate longitudinal axis, and a bend direction
  with a usable orthogonal component
- normalizes the longitudinal axis, orthogonalizes and normalizes the bend
  direction, canonicalizes direction sign with the angle, and builds the
  right-handed binormal
- returns an exact identity result for near-zero angles
- evaluates non-zero bends with double-precision accumulation and the same
  small-angle series helpers used by the Rust evaluator
- compares the semantic result from the prior baked stage to the declared
  baked stage using the manifest tolerance policy, then reports max component,
  max Euclidean, mean Euclidean, RMS Euclidean, outside-tolerance count, and
  pass state
- fails reconstruction when the semantic-to-baked comparison does not satisfy
  the policy

Saved-blend verification reloads the manifest and sidecars, then checks shape
key order, labels, cumulative stage positions, final enabled stage, persistent
vertex IDs, baked object topology, and exact final position bits.
