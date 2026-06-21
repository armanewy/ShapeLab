# Asset Edits

`shape-asset` exposes an explicit edit program for part-aware asset recipes. An
`AssetEditProgram` is ordered, deterministic, and applied atomically: edits run
against a cloned recipe, the complete clone is validated, and the original recipe
is left unchanged on any edit or validation failure.

Use `apply_edit_program` when only the edited recipe is needed. Use
`apply_edit_program_with_report` when callers need a deterministic
`AssetEditReport` containing the program label, seed, per-edit subjects,
topology-change flags, applied count, and final validation report.

## Numeric Edits

The edit language supports descriptor-backed scalar parameters and direct edits
for common modeling controls:

- instance transforms
- generator dimensions and segment counts
- base generator replacement
- bevel radius and segments
- sweep profile points and path frames
- lathe profile points and segments
- linear and radial array count and spacing

Scalar paths also cover generator dimensions, transforms, bevel fields, sweep
profile/path components, lathe profile components, and array spacing fields so
parameter descriptors can target the same controls.

## Structural Edits

Structural edits support:

- adding and removing leaf part instances
- enabling and disabling authored optional parts
- duplicating and mirroring instances with caller-provided semantic IDs
- replacing an instance definition with a compatible variant-group definition
- attaching and detaching socket relationships
- changing array counts
- accepting semantically harmless child reorder requests

Compatible replacement is constrained by `variant_group`; when a matching
`replacement_groups` hint exists, both old and new definitions must be listed in
that group.

## Locks

Locks are represented directly on `AssetRecipe`:

- `locks` protects parameter IDs
- `instance_locks` protects individual part instances
- `subtree_locks` protects a root instance and every descendant
- `topology_locks` protects a part definition's topology

A topology lock permits non-topology-changing parameter edits, such as a cylinder
radius change, but rejects topology-changing controls such as segment counts,
array counts, bevel segment changes, definition topology replacement, and base
generator replacement.

## Beginner Reflection

`reflect_beginner_parameters` returns the fixed beginner-facing groups:

- Size
- Proportions
- Placement
- Curvature
- Edge Softness
- Repetition
- Part Presence
- Detail Density

The reflection API includes current scalar values, lock state, topology-change
flags, and optional part presence controls. Raw literal vertex, face, position,
and index controls are filtered out.
