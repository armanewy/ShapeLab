# Foundry Pack Generation

`FoundryPackDocument` groups multiple foundry asset documents into one coherent
asset pack. Pack compilation applies pack-authored shared state to each member,
uses the document compiler for the effective member documents, then builds a
deterministic pack report over the compiled outputs.

## Compile API

Use `compile_foundry_pack(&pack, &resolver)` for default member compilation, or
`compile_foundry_pack_with_options(&pack, &resolver, options)` when the caller
needs explicit `FoundryCompilationOptions`.

The compiler returns `FoundryPackCompilationOutput`:

- `pack`: the compiled pack document with a shared catalog lock when all members
  agree on lockable references.
- `member_outputs`: the normal `FoundryCompilationOutput` for every named
  member, keyed by member ID.
- `report`: a deterministic `FoundryPackReport`.

## Report Contents

The pack report contains:

- `members`: document ID, family/style refs, selected providers, controls,
  triangle count, visual descriptor, and member conformance summary.
- `shared_controls`: controls whose values are identical across every member.
- `differences`: member-specific control/provider differences from the first
  deterministic member baseline.
- `triangle_totals`: total, minimum, maximum, and per-member triangle counts.
- `visual_descriptor_spread`: coarse bounds/statistics descriptors and maximum
  normalized pairwise spread.
- `conformance_status`: accepted flag and stable pack-level issue rows.
- `report_fingerprint`: deterministic fingerprint of the report payload.

## Shared Pack Inputs

`shared_controls` is pack-authored control state keyed by control ID. These
controls are injected into every member before validation, catalog lock checks,
compilation, and report generation. A member-local value for the same control is
replaced by the pack-authored value in the effective member document.

`SharedProviderPolicy::SharedExact` injects the shared provider choices into each
member before compilation. Pack `shared_locks` are also merged into member
documents for compilation output. If a shared provider is injected, the member
catalog lock exact refs are regenerated around the effective member document so
document-level lock verification remains exact.

When the pack carries `catalog_lock`, its exact refs and embedded snapshots are
merged into each effective member catalog lock before member compilation. A
pack-level lock mismatch is therefore reported by member compilation instead of
only being reflected in validation or reconstructed output.

The report's `shared_controls` rows are computed from effective member control
state. They include explicit pack-authored shared controls and any other control
values that are identical across every compiled member.

## Coherence Checks

Pack compilation rejects a pack after member compilation when any coherence
issue is present:

- style facets must match across members
- provider vocabulary must be semantically compatible across members
- edge language must be semantically compatible across members
- detail-density spread must remain within the coherent range
- scale family must match across members
- source document IDs must be unique
- duplicate compiled geometry is rejected unless intentionally allowed
- all member conformance summaries must be accepted

Pack member compilation still fails on hard document, catalog, control, asset
validation, and asset compilation errors. If a member reaches final conformance
and that final conformance is rejected, pack compilation keeps the compiled
member output, includes the member row, totals, and conformance summary in the
report, and rejects the pack with `pack_member_conformance_rejected`.

Duplicate geometry is intentional only when the pack uses
`PackCoherencePolicy::Custom("allow_duplicate_geometry")` or includes a shared
lock with `FoundryLockTarget::Custom("allow_duplicate_geometry")`. Duplicate
geometry detection fingerprints the compiled preview mesh content, not the full
artifact metadata, so two members with identical geometry but different source or
artifact metadata are still treated as duplicates.

Edge-language coherence compares edge/profile/bevel semantics without requiring
the same style-kit ID. Provider-vocabulary coherence compares role provider
fragment semantics without requiring matching provider IDs when the provider
content is otherwise compatible.

All member maps are ordered with `BTreeMap`, and the report fingerprint is built
from deterministic serializable report content, so repeated batch generation of
the same pack and catalog produces the same report.
