# Foundation Draft Schema

`shape-foundry::foundation` defines the Wave 36 draft schema. The current schema
version is `FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION = 1`.

## Top-Level Draft

`FoundryFoundationDraft` contains:

- `schema_version`
- `draft_id`
- `source_kind`: `human`, `llm_assisted`, or `generated_fixture`
- `quality_target`: `draft`, `prototype`, `usable`, or `showcase`
- `catalog_visibility`: `internal_only`, `developer_preview`, or
  `novice_catalog`
- `human_review_required`
- `publish_allowed`
- `category`
- `family_blueprint`
- `provider_taxonomy`
- `style_pack`
- `control_profile`
- `candidate_strategy_pack`
- `compatibility_matrix`
- optional `quality_gate_profile`
- `test_plan`
- `review_checklist`
- `command_log`
- `rejected_command_attempts`
- `direct_geometry_payload_attempts`

New drafts are internal-only, require human review, and cannot publish. In Wave
36, validation rejects any `catalog_visibility` value other than
`internal_only` and rejects `publish_allowed = true`, even for Usable or
Showcase targets. Promotion must use a later reviewed package path, not the
foundation draft schema.

Foundation draft JSON uses strict typed parsing. Unknown fields, including raw
geometry keys such as `mesh_payload` or `raw_vertex_positions`, fail
deserialization instead of being ignored.

## Validation

Foundation draft validation detects:

- missing required roles
- required roles without provider slots
- too many primary controls
- technical/internal terms in novice labels
- duplicate visible slot ownership
- missing provider slots
- incoherent style/provider compatibility
- empty candidate strategies
- candidate strategies outside visible control space
- candidate provider changes referencing unknown provider slots
- missing quality gate profile
- Usable/Showcase targets without contact sheet gates
- any direct publish flag
- any non-internal catalog visibility
- forbidden command attempts
- direct geometry payload attempts

## Materialization

`materialize_foundation_draft_package()` converts a valid draft into an internal
`FoundryKitPackage`. The package remains `Draft`, uses hidden catalog visibility,
is not placed in the developer preview catalog, has no built-in source profile
slug, and records review blockers for missing authored geometry and human
approval.

Materialization does not produce geometry. It writes package metadata that an
author can inspect and improve through the existing Foundry kit review path.
The materializer validates the emitted kit package and reports mapped
foundation errors instead of writing an invalid kit.
