# Foundry Author Descriptor Format

Author Studio descriptors are versioned metadata used to prepare and review
Foundry kit packages. The current schema version is
`FOUNDRY_AUTHOR_STUDIO_SCHEMA_VERSION = 1`.

The descriptors live in `shape-foundry::author_studio`. They are intentionally
separate from novice Visual Foundry view models.

## Gate and Shell

`FoundryAuthorStudioGate` controls availability:

- `default_release()` hides the surface.
- `developer_enabled()` exposes the authoring workflow.

`FoundryAuthorStudioShell` contains:

- `available`
- `unavailable_reason`
- ordered workflow `steps`

## Role Descriptor

`AuthorRoleDescriptor` defines a family role for guided labeling:

- `role_id`
- `display_name`
- `description`
- `required`
- `repeated`
- `default_visibility`
- `export_part_name`

IDs must be unique. Display copy and export part names must be present.

## Provider Descriptor

`ProviderDescriptor` records provider registration metadata:

- `provider_id`
- `display_name`
- `semantic_role`
- `provider_slot`
- `tags`
- `compatibility_tags`
- optional `approximate_triangle_budget`
- `socket_requirements`
- `preview_available`
- `descriptor_only`

The `descriptor_only` flag is required for rows that describe a component before
real component/mesh import support exists.

## Socket and Port Descriptor

`SocketPortDescriptor` records attachment metadata:

- `socket_id`
- `port_id`
- `target_role`
- `compatibility_tags`
- `allowed_attachment_modes`
- `required`
- `author_notes`

Required sockets must include socket ID, port ID, target role, and compatibility
tags. Target roles must reference the family role inventory.

## Style Compatibility Descriptor

`StyleCompatibilityDescriptor` records style/provider compatibility policy:

- `compatible_style_packs`
- `incompatible_style_packs` with author reasons
- `allowed_provider_tags`
- `forbidden_provider_tags`
- `detail_density_policy`
- `bevel_language_notes`
- `proportion_language_notes`
- `symmetry_asymmetry_policy`

Tags and style pack IDs cannot be both allowed/compatible and
forbidden/incompatible.

## Control Mapping Descriptor

`ControlMappingDescriptor` records a customizer control mapping:

- `control_id`
- `label`
- `description`
- `kind`
- `primary`
- `visible`
- `owned_family_slots`
- `owned_provider_slots`
- `response_curve_descriptor`
- `discrete_options`
- optional `provider_slot_binding`
- `topology_behavior`
- `disabled_reason_policy`

Visible primary controls are capped by
`DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS`. Visible controls cannot duplicate slot
ownership. Topology-changing controls must be discrete choices.

## Candidate Strategy Descriptor

`CandidateStrategyDescriptor` records a direction-generation strategy:

- `strategy_id`
- `name`
- `explanation`
- `allowed_controls`
- `allowed_provider_changes`
- `intensity_policy`
- `diversity_policy`
- `lock_respect_policy`
- `rejection_policy`
- `explanation_template`

Strategies must operate in visible customizer space. Explanation templates must
use user-facing control labels and must not expose scalar paths, recipe paths,
operation IDs, semantic IDs, or other raw internal terms.

## Preview Camera Descriptor

`PreviewCameraDescriptor` records one camera:

- `camera_id`
- `label`
- `view`
- `fitted_scale_policy`
- `lighting_policy`
- `supported`
- optional `unsupported_reason`

`PreviewCameraPolicyDescriptor` groups the default, direction-board,
option-gallery, and contact-sheet cameras. Contact sheets must declare front,
side, back, and three-quarter views. Each option gallery must use a consistent
camera and fitted-scale policy across options.

## Quality Gate Launch Descriptor

`AuthorQualityArtifactRefs` records the evidence context used to decide whether
existing CLI gates can run:

- optional `package_manifest_ref`
- `verified_built_in_backing`
- `out_dir`
- optional `quality_report_ref`
- optional `review_manifest_ref`
- `contact_sheet_refs`

Authored packages should provide `package_manifest_ref`. Built-in slug launch
rows are emitted only when `verified_built_in_backing` is true.

`AuthorQualityGateLaunch` records one CLI-backed gate:

- `task`
- `supported`
- optional `invocation`
- optional `unsupported_reason`

`author_quality_gate_launches()` emits rows for validate, preview, contact
sheet, HQ benchmark, review manifest, and package export. A row is unsupported
when required metadata is missing.

## Package Export Manifest

`AuthorPackageExportManifest` records review/package refs:

- `kit_manifest_ref`
- `provider_pack_refs`
- `style_pack_refs`
- `control_profile_ref`
- `candidate_strategy_pack_ref`
- `quality_gate_profile_ref`
- `review_manifest_ref`
- `quality_report_refs`
- `contact_sheet_refs`

The refs match the current `shape-cli foundry-kit package` output layout:

- `kit-manifest.json`
- `provider-pack.json`
- `style-pack.json`
- `control-profile.json`
- `candidate-strategy-pack.json`
- `quality-gate-profile.json`
- `review-manifest.json`

The manifest is a reference list for package review. It is not a second source
of geometry and it does not grant catalog visibility.
