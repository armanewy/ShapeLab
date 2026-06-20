# Schema 3 Blender Reconstruction

The schema-3 Blender adapter generates a standalone Python script. It is not
wired into the CLI in this wave.

Replay authority is always the cumulative baked stage payload declared by each
manifest operator. The generated script creates the source mesh topology
directly, adds one cumulative shape key per operator, loads each shape key from
its baked positions file, leaves earlier stage values at zero, enables only the
final stage, and creates a separate baked reconstruction object from the final
stage positions.

Semantic operators are validation inputs only:

- Affine stages are reevaluated with the schema-2 stepwise float32 arithmetic
  contract and must match their baked stage bit-for-bit.
- Bend stages validate the bend frame and parameters, reevaluate the bend
  formula in Python, and compare against the baked stage with the stage
  tolerance policy.
- The terminal lossless correction applies absolute residual positions and must
  match its baked stage bit-for-bit.

The script validates schema version, numeric format, mesh payload counts,
topology hash, duplicate operator IDs, stage ordering, path safety, lossless
terminality, stage positions, final target topology, final target positions, and
the persistent `shapelab_vertex_id` attribute. Package asset paths are required
to be package-relative, non-absolute, traversal-free, and contained within the
package root after symlink resolution.

`--verify-existing` reloads the manifest and sidecars, then verifies the saved
editable and baked objects in the currently opened blend file. A JSON report is
written for success or failure, and failures exit nonzero.
