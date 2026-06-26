# Motion Artifact Readiness Contract

Motion readiness is represented by clip and lineage evidence. The contract does
not generate animation curves and does not include a humanoid retargeter.

Contracts:

- `MotionArtifact`
- `MotionClipDescriptor`
- `MotionValidationReport`
- `MotionClipEvidenceStatus`
- `MotionReadinessStatus`

A motion-ready claim requires:

- a motion layer bound to a rig artifact;
- rig evidence marked ready;
- at least one clip with authored clip evidence and an evidence reference.

Descriptor-only clips can be serialized and validated, but they do not pass a
motion-ready claim.
