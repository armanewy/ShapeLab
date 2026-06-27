# Rig Artifact Readiness Contract

Rig readiness is represented by descriptor contracts, not by generated rigging.

Contracts:

- `RigArtifact`
- `SkeletonTemplate`
- `JointDescriptor`
- `AttachmentSocketDescriptor`
- `PivotDescriptor`
- `SkinBindingStatus`
- `RigValidationReport`

Static props must report rig status as `NotApplicable` and must not imply a
hidden rig. Mechanical props may carry descriptor-only pivots or attachment
sockets without claiming skinning; those descriptors validate as
`DescriptorOnly`, not as full deforming rig readiness.

Prepared humanoid templates require a skeleton, bind pose evidence, complete
skin binding status, and skin weight evidence before a rig-ready claim can pass.
A prepared-template descriptor that attempts a rig-ready claim without complete
skin-weight evidence is invalid.

Shape Lab does not auto-rig arbitrary meshes and does not generate skin weights
in this contract.
