# Motion Artifact Readiness Contract

Motion readiness is represented by clip and lineage evidence. The contract does
not generate animation curves and does not include a humanoid retargeter.

Contracts:

- `MotionArtifact`
- `MotionClipDescriptor`
- `MotionValidationReport`
- `MotionClipEvidenceStatus`
- `MotionTargetKind`
- `MotionReadinessStatus`

Static props must report motion status as `NotApplicable`.

A motion-ready claim requires:

- a motion layer bound to a rig artifact;
- rig evidence marked ready;
- at least one clip with authored clip evidence and an evidence reference.

An animation-ready claim uses the same evidence gate. It cannot pass without a
validated rig binding and authored clip evidence.

Descriptor-only clips can be serialized and validated, but they do not pass a
motion-ready claim.
