# Semantic Core Module Split

Cleanup Wave 1 Prompt 4 split oversized semantic backend files without changing
public contracts or export behavior. The split keeps each public module name in
place and moves implementation sections into same-namespace shards included by
the original facade file.

## Shape Asset

`crates/shape-asset/src/lib.rs` remains the crate root for schema constants,
public modules, and crate-root re-exports. Its implementation is now split under
`crates/shape-asset/src/asset_core/`:

| Shard | Responsibility |
| --- | --- |
| `ids.rs` | Stable semantic ID newtypes and ID macro. |
| `spatial.rs` | `Transform3` and `Frame3`. |
| `recipe.rs` | `AssetRecipe`, wire migration, and ID counters. |
| `semantic_shells.rs` | Relationship, pattern, surface, export, review, and variation shell metadata. |
| `geometry_types.rs` | Part definitions, geometry sources, operation specs, and operation metadata. |
| `scalar_ranges.rs` | Feasible scalar range calculation for geometry and operation paths. |
| `geometry_wire.rs` | Modeling-operation wire compatibility and migration helpers. |
| `references.rs` | Sockets, attachments, regions, parameters, constraints, selectors, and relationship policy. |
| `edit_contracts.rs` | Asset edit program contracts and edit operation enums. |
| `validation_api.rs` | Validation report/error types and public asset helper entry points. |
| `validation_structure.rs` | Definition, instance, attachment, parameter, lock, constraint, and selector validation. |
| `validation_semantic.rs` | Variation metadata, semantic shells, relationship, and pattern policy validation. |
| `validation_identifiers.rs` | Identifier, review, include, counter, and semantic cut host validation. |
| `validation_operations.rs` | Geometry source and modeling operation validation. |
| `scalar_access.rs` | Scalar path read/write helpers. |
| `edit_application.rs` | Core edit application and structural mutation helpers. |
| `edit_cut_duplication.rs` | Cut duplication, operation ordering, metadata cascade, and ID remapping helpers. |
| `utility.rs` | Shared parsing, validation, and issue helpers. |
| `tests.rs` | Existing crate-root tests. |

## Shape Foundry

The public modules in `crates/shape-foundry/src/lib.rs` are unchanged. The
oversized foundry modules now delegate to same-namespace shard directories:

| Module | Shards |
| --- | --- |
| `author_studio` | `contracts.rs`, `descriptor_validation.rs`, `quality_exports.rs`, `tests.rs` |
| `compile` | `contracts_and_entrypoints.rs`, `family_requests.rs`, `overrides.rs`, `conformance.rs`, `fingerprints.rs` |
| `control` | `contracts.rs`, `evaluation.rs`, `canonicalization.rs`, `domains.rs`, `lookup_and_describe.rs` |
| `foundation` | `draft_contracts.rs`, `commands_and_templates.rs`, `archetype_box_fixture.rs`, `validation.rs`, `materialization.rs`, `adversarial_repairs.rs`, `tests.rs` |
| `kit` | `contracts.rs`, `visibility_validation.rs`, `pack_validation.rs`, `tests.rs` |
| `object_plan` | `contracts.rs`, `materialization_summary.rs`, `composition_materialization.rs`, `validation_text_helpers.rs` |
| `pack` | `contracts_compile.rs`, `report_building.rs`, `coherence_checks.rs` |
| `primitive_property` | `contracts_schemas.rs`, `validation.rs`, `descriptors.rs` |
| `validation` | `document_profile_command.rs`, `control_domains.rs`, `references_values_variation.rs` |
| `variation` | `surface_capability.rs`, `intent_reports.rs` |

## Shape Modeling

The public modeling module paths are unchanged. Oversized implementation files
are split into same-namespace shards:

| Module | Shards |
| --- | --- |
| `assembly` | `planning_transform.rs`, `evaluation_state.rs`, `validation_transform_helpers.rs` |
| `features` | `contracts_panel_trim.rs`, `ribs_fasteners.rs`, `frames_instances.rs`, `mesh_builder.rs`, `geometry_helpers.rs` |
| `generators::basic` | `contracts_and_entrypoints.rs`, `rounded_box_cuts.rs`, `plate_cuts.rs`, `frustum_mesh.rs`, `plate_cut_helpers.rs`, `shell_and_loop_helpers.rs`, `mesh_and_regions.rs`, `validation_math.rs` |
| `generators::profile` | `contracts_sweep_lathe.rs`, `profile_frames.rs`, `lathe_mesh.rs` |

## Shape Authoring

`shape-authoring` exists in this baseline but had no oversized source file in
this wave, so no source split was required.

## Remaining Exceptions

No owned semantic backend source file remains over the 1000 non-test line rule
after this split. `python3 scripts/check_rust_file_size.py` still reports
temporary exceptions in out-of-scope crates owned by other cleanup waves.
