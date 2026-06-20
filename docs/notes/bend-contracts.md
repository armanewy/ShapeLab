# Bend Contracts Notes

## Exact Replay Versus Semantic Evaluation

Schema 3 treats semantic operators as explanations, not as the replay source of
truth. Affine and bend operators describe editable intent and can be evaluated
for diagnostics, but every package stage also declares a cumulative baked
positions file. Exact package replay must use those baked cumulative positions,
then apply the terminal lossless correction, so reconstruction does not depend
on whether a downstream tool implements the same semantic operator math.

This distinction is especially important for bend. The contract now records the
uniform-curvature frame convention and validation shape, but non-zero bend math
and bend inference are intentionally not implemented yet. The only implemented
bend evaluation path is near-zero identity, which must return exact input
positions.

## Why Schema 2 Remains Frozen

Schema 2 is the current shipped decompiler package format. It fixes affine
evaluation to stepwise binary32 arithmetic, keeps package verification
bit-exact, and drives the existing CLI and Blender output. This branch adds
schema-3 contracts under `shape_decompiler::v3` without changing schema-2
serialization, verification, CLI defaults, public behavior, or generated
Blender scripts.

Freezing schema 2 lets parallel work prepare bend inference, program search,
and schema-3 writing without risking regressions in the current same-topology
round-trip demo.

## Next-Wave Ownership Boundaries

- Bend math should live behind `v3::bend::evaluate_bend` and
  `v3::bend::evaluate_bend_point`, preserving the documented right-handed frame
  convention and exact identity path.
- Bend candidate generation should implement `v3::inference::BendCandidateGenerator`
  or replace the `generate_bend_candidates` stub without touching schema-2
  inference.
- Ordered program search should build on `v3::program::OperatorProgram` and
  `v3::inference::ProgramSearchSettings`, keeping the initial explanatory depth
  limit explicit until a later integration wave raises it.
- Schema-3 package writing should populate `v3::package` manifests and write
  cumulative baked positions for every stage. It should not move or refactor the
  schema-2 package writer.
- Blender schema-3 reconstruction should replace the `v3::blender` stub by
  loading baked cumulative stage positions as exact shape keys, not by relying
  on native Blender bend or affine deformers for replay.
