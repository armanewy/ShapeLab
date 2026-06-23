# Foundry Project Format

Foundry projects persist replayable semantic foundry history in
`.shapelab-foundry.json` files. The project source of truth is the
`FoundryAssetDocument` snapshot for each revision. Generated `AssetRecipe`
snapshots and build stamps are stored beside that source so a loader can verify
or invalidate prior builds without silently changing the semantic project.

## File Shape

Top-level fields:

- `project_kind`: must be `shape-lab.foundry-project`.
- `schema_version`: current version is `1`.
- `title`: human-facing project title.
- `current_revision`: selected revision.
- `next_revision_id`: next monotonic revision ID.
- `revisions`: deterministic map keyed by revision ID text.

Each revision stores:

- `id`: stable revision ID.
- `parent`: parent revision ID, or `null` for revision `0`.
- `label`: human-facing revision label.
- `document`: complete `FoundryAssetDocument` snapshot.
- `program`: `FoundryEdit` or labeled `FoundryCommand` program that produced a
  non-root revision.
- `catalog_lock`: exact `FoundryCatalogLock` for this revision.
- `build_stamp`: optional `FoundryBuildStamp` captured by the compiler.
- `recipe_snapshot`: optional exact generated `AssetRecipe` canonical JSON plus
  `RecipeFingerprint`.
- `conformance`: stored `FoundryConformanceSummary`.

The revision document's embedded `catalog_lock` and `build_stamp` must match the
revision-level `catalog_lock` and `build_stamp`. They are duplicated so the
document snapshot remains standalone, but the loader rejects disagreement.

## Replay Rules

Non-root revisions must store a replay program. Load validation replays the
program from the parent semantic document and compares it with the stored child
semantic document. Catalog locks and build stamps are checked separately and are
excluded from this semantic comparison.

Replayable document commands are:

- `SetControl`
- `ResetControl`
- `SelectProvider`
- `SetStyle`
- `SetLock`

Runtime commands such as candidate generation, export, undo, revision switch,
and pack insertion are rejected as revision edit programs because they do not
directly produce a `FoundryAssetDocument` child snapshot. `SetRolePresence` is
also rejected until `FoundryAssetDocument` has a persisted role-presence field.

Style and provider choices are never migrated by the loader. A style or
provider command must replay to the exact stored content references. If a later
catalog offers a different compatible-looking style or provider, the loader
does not substitute it.

## Load Verification

All loads validate:

- project kind and schema version;
- revision graph shape, root revision, monotonic parent IDs, and current
  revision existence;
- each foundry document via `validate_foundry_document`;
- catalog locks against the document's family, style, implementation, profile,
  and locked provider references;
- embedded catalog snapshots against the exact references they claim to
  recover;
- build-stamp recipe fingerprint against the stored recipe snapshot when both
  are present;
- semantic replay for every non-root revision.

When callers provide load-time catalog references, each provided reference must
match the revision's exact catalog lock. A mismatch marks the build stale. If an
embedded snapshot satisfies the locked reference, loading succeeds in read-only
recovery mode. Without an embedded snapshot, the loader rejects the project
instead of silently rebuilding against changed catalog content.

When callers provide generated `AssetRecipe` inputs for revisions, the loader
verifies the exact stored recipe snapshot. Missing or mismatched snapshots are
hard load errors for those revisions.

Builds are marked stale, but the semantic project remains readable, when:

- catalog format version changes;
- catalog compiler version changes;
- a locked catalog reference changes but an embedded recovery snapshot exists;
- the shape-foundry crate version changes;
- the caller reports a changed shape-family-compile version.

## Editing And Persistence

Foundry project history is branchable. Undo moves the current revision pointer
to the parent without deleting children, and accepting a new edit after undo
creates a sibling revision with the next monotonic ID. Revision switching only
changes `current_revision`.

Writes use sibling temporary files, `sync_all`, and atomic rename replacement.
Recovery snapshots use the same validated JSON writer and do not change the
clean/dirty marker for the open project file.
