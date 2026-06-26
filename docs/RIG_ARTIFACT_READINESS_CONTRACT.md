# Rig Artifact Readiness Contract

Rig readiness is represented by descriptor contracts, not by generated rigging.

Contracts:

- `RigArtifact`
- `SkeletonTemplate`
- `JointDescriptor`
- `AttachmentSocketDescriptor`
- `SkinBindingStatus`
- `RigValidationReport`

Static props may report rig status as `NotApplicable`. Mechanical props may
carry descriptor-only pivots or attachment sockets without claiming skinning.
Prepared humanoid templates require a skeleton, bind pose evidence, complete
skin binding status, and skin weight evidence before a rig-ready claim can pass.

Shape Lab does not auto-rig arbitrary meshes and does not generate skin weights
in this contract.
