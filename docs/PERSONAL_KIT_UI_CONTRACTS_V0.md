# Personal Kit UI Contracts v0

Status: contracts/view-model only; no storage implementation.

Personal Kit contracts define how a future UI can present "Save as Personal
Kit" for a current primitive, ObjectPlan draft, or composition draft. V0 does
not add public catalog publishing, cloud sync, broad library UI, materials,
UV/texturing, rigging, animation, or runtime LLM integration.

## View Model

`PersonalKitSaveViewModel` includes:

- `source_kind`: `CurrentPrimitive`, `ObjectPlanDraft`, or `CompositionDraft`
- `display_name`
- `editable_name`
- `summary`
- `warnings`
- `save_enabled`
- `disabled_reason`
- `resulting_visibility`: `Draft` or `PersonalOnly`
- render evidence availability
- export proof availability

Allowed user copy:

- "Save as Personal Kit"
- "Only visible to you"
- "Needs review before sharing"

V0 warnings include:

- "No review image yet."
- "No engine export proof yet."

## Save Command

`PersonalKitSaveCommand` includes:

- `source_ref`
- `kit_name`
- `visibility`
- `include_preview`
- `include_object_plan`
- `include_export_reference`

The command is a contract for later UI/storage work. It does not write files in
this milestone.

## Validation

Validation requires:

- kit name is present
- source kind is supported
- visibility is local/private only
- public visibility is rejected
- missing review images are surfaced as a warning
- missing export proof is surfaced as a warning
- user copy does not claim materials, rigging, animation, marketplace, public
  publishing, or game-ready status

## Product Boundary

Personal Kits are local/private. They do not publish to a public catalog, do not
sync to cloud storage, and do not make an asset shareable by default. Review is
still required before any future sharing path.

Full storage, persistence migration, and app-wide library UI come later.
