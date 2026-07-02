# Workspace Dead Code and Dependency Purge

This branch removes unused research-era crates, command surfaces, fixtures, and
dependencies that are outside the current Object Orchard architecture.

## Removed Crates

- `shape-decompiler`
- `shape-program`
- `shape-program-verify`

These crates supported the old same-topology decompiler and strict semantic
program lanes. They are not part of the current product path. The canonical lane
is now `shape-asset::AssetRecipe`, Orchard IR, `AuthoringOpLog`,
`RelationshipContract`, `PatternContract`, ObjectPlan materialization, and
geometry/export reports.

## Removed CLI Surface

- `shape-cli decompile`
- `shape-cli verify-decompile`

The removed commands depended on the deleted decompiler crate. No active
ObjectPlan, Direct Make, primitive preset, geometry export, or Godot proof path
depends on them.

## Removed Fixtures and Scripts

- obsolete `fixtures/shape-asset/*_asset_recipe_v*.json` migration samples from
  the retired fixture path
- `scripts/apply_box_primitive_ui_cleanup.py`, a one-off cleanup helper that is
  no longer part of the active workflow

Current ObjectPlan, export, relationship, pattern, direct primitive, and claim
gate fixtures remain in place.

## Dependency Cleanup

The workspace manifest no longer lists the retired crates as members or
workspace dependencies. Crate manifests were trimmed where dependencies were only
used by the removed decompiler/program paths.

## Kept Internal/Legacy Areas

- `shape-core` remains for legacy/low-level types until the separate retirement
  pass finishes its dependency audit.
- `shape-search` remains as an internal crate because active workspace members
  still depend on it.
- historical documentation references are intentionally left for the obsolete
  documentation purge branch, which owns broad doc deletion.

## Remaining Debt

- Remove historical docs that describe the deleted decompiler/program lanes.
- Continue reducing the `shape-core` surface through the dedicated retirement
  branch.
- Re-run the workspace audit after integration to confirm no orphan references
  remain.
