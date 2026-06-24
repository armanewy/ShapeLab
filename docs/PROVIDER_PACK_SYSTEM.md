# Provider Pack System

Provider packs are expert-authored summaries of the concrete options a kit can
use to fill family roles. They are package metadata over existing exact
catalogs, not a bypass around family/style compilation.

## Contents

A `ProviderPack` records:

- `pack_id`
- primary `family_id` and compatible family IDs
- provider slots supplied
- provider options with labels, covered roles, compatibility tags,
  detail-density tags, and triangle budget estimates
- semantic role coverage
- attachment tags
- package-level compatibility tags

The built-in mapper derives provider slots from non-derived family roles and
records default family/style role providers from the executable bindings.

## Validation Rules

- Every required family provider slot must be supplied.
- Every required family role must be covered.
- A provider pack must support the family blueprint referenced by the kit.
- A style pack may explicitly allow or reject a provider pack.
- Incompatible style/provider pairs are invalid for novice exposure.

Power/developer tools may show detailed incompatibility reasons. The default
Visual Foundry UI uses plain-language hidden reasons and does not expose slots,
ports, or provider IDs.
