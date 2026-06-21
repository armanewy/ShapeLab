# Asset Graph Notes

## Prompt 1.1 Scope

`shape-asset` now owns a deterministic, serializable asset recipe graph for
part definitions, part instances, scalar parameter descriptors, socket
attachments, and authored variation hints. This crate remains contract-only:
it validates recipes and applies edits, but does not generate geometry.

## Edit Semantics

Edit programs apply to a cloned recipe and commit only after every operation
and full recipe validation succeeds. Failed edits leave the source recipe
unchanged.

Instance removal is explicit leaf removal. Removing an instance that still has
descendants is rejected; callers must remove or detach descendants first. Root
insertion, detaching, and manual instance additions keep root ordering stable by
semantic instance ID.

Locks apply to parameter mutation through `SetScalar`. Locked parameters remain
visible through parameter reflection and readable through scalar inspection.

## Variation Hints

Authored variation metadata is stored as recipe hints:

- optional instances
- replacement groups of interchangeable definitions
- array count ranges by operation ID
- parameter range overrides

These hints are validated for known IDs and sane ranges, but they do not change
hierarchy semantics or geometry generation.
