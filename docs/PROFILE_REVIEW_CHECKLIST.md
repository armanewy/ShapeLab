# Profile Review Checklist

Use this checklist before accepting a Foundry Author package into the built-in
catalog or distributing it as a local package.

## Contract

- The package validates with `shape-cli foundry-validate-profile`.
- The package compiles through `shape-cli foundry-preview-profile`.
- The package can be emitted with `shape-cli foundry-package-profile`.
- The packaged `catalog` directory builds through `shape-cli foundry-build`.
- Family, style, family implementation, style implementation, and customizer
  profile IDs all target the same family/style pair.
- Catalog locks are exact and generated from the packaged content refs.

## Novice Surface

- The primary surface has seven or fewer visible controls.
- Control labels describe user intent, not scalar paths or implementation IDs.
- Choice and provider controls have whole-model preview IDs.
- Disabled or unavailable options have deterministic reasons.
- A user can create a useful asset with zero technical surface exposure.
- Preview cameras use stable IDs and produce nonempty images at their requested
  dimensions.

## Family And Style

- Roles cover the functional structure of the asset family.
- Required roles have executable providers.
- Attachment rules distinguish required, advisory, and runtime-only behavior.
- Conformance failures explain missing roles, disconnected attachments,
  unsupported operations, or export-rule failures.
- Style facets declare provider vocabulary for the family they target.

## Candidate And Pack Behavior

- Candidate strategies reference known controls only.
- Refine-style strategies keep edits local enough for iteration.
- Explore-style strategies can produce visible structural differences without
  invalidating the current asset.
- Pack policies declare an export profile and a bounded member range.
- Shared pack controls reference known controls only.

## Boundaries

- The profile does not imply arbitrary mesh import or editable import.
- The profile does not depend on LLM, DCC, marketplace, material, UV, rigging, or
  animation work.
- Any unsupported feature is represented as a validation issue or documented
  limitation, not a hidden correction buffer.
