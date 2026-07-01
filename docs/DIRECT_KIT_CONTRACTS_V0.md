# Direct Kit Contracts v0

Status: contracts only. No app UI, storage, generated candidate tray, runtime
LLM integration, public catalog publishing, material editor, UV editor,
rigging, animation, or game-ready claim is added by this milestone.

## Definition

A Direct Kit is a local reusable kit draft made from one supported primitive,
supported safe-anchor composition, or supported ObjectPlan Draft. It exposes a
selected subset of bounded primitive properties as user-changeable controls and
keeps the remaining properties fixed.

Direct Kits remain Draft or PersonalOnly in v0. Review, showcase promotion, and
public catalog publishing are later gates.

## Contract

`DirectKitDraft` includes:

- `kit_id`
- `display_name`
- `source_kind`
- `source_ref`
- `identity_summary`
- `changeable_properties`
- `locked_properties`
- `included_presets`
- `evidence_refs`
- `review_tier`
- `visibility`
- `created_from`

`DirectKitPropertyExposure` wraps one primitive property schema entry with its
current value, default value, domain, user description, and UI flags.

`DirectKitPresetRef` may reference deterministic presets such as built-in Box,
Flat Panel, and Sphere presets. Presets are named property bundles, not
generated variations.

`DirectKitEvidenceRef` may reference property endpoint sheets, preset contact
sheets, ObjectPlan render evidence, or geometry export reports. Evidence is
required for stronger review, but missing evidence is a warning for Draft kits.

## Validation

Validation checks:

- kit IDs are normalized
- display names and summaries are present and product-safe
- source kind is supported
- changeable and locked properties exist in the supported primitive schema
- property defaults and current values stay within the schema domain
- built-in presets match the source primitive or supported composition
- visibility is Draft or PersonalOnly only
- public catalog visibility is rejected
- Reviewed and Showcase visibility are rejected in v0
- evidence stays review-required
- user-facing copy hides internal technical terms

## Product Boundary

Direct Kits are local reusable kit drafts. They can reference ObjectPlan render
evidence and geometry export reports, but they are not public catalog items and
do not approve assets automatically.

Direct Kits do not use generated variation trays. Review and promotion are
later gates.
