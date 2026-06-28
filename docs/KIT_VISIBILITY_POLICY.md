# Kit Visibility Policy

Kit visibility is controlled by quality tier and review evidence.

Visual Foundry app-home exposure is controlled by the catalog curation states in
[`VISUAL_FOUNDRY_CATALOG_CURATION.md`](VISUAL_FOUNDRY_CATALOG_CURATION.md).
Kit quality tiers still record review maturity, but they do not by themselves
make a profile novice-facing.

## Catalog Curation States

| State | Default Novice Catalog | Preview/Developer Catalog |
| --- | --- | --- |
| HiddenDraft | Hidden | Hidden |
| PreviewOnly | Hidden | Eligible |
| Usable | Eligible | Eligible |
| Showcase | Eligible with badge only after human and adversarial review | Eligible |

Only Usable and Showcase profiles may appear in the default novice catalog.
PreviewOnly profiles require the explicit preview catalog switch. HiddenDraft
profiles remain direct-test content and do not appear in the app catalog.

## Kit Quality Tiers

| Tier | Default Novice Catalog | Preview/Developer Catalog |
| --- | --- | --- |
| Draft | Hidden | Hidden or local-only |
| Prototype | Hidden | Eligible |
| Usable | Eligible only after review evidence | Eligible |
| Showcase | Eligible with badge only after human and adversarial review | Eligible |

## Rules

- Draft kits must not appear in the default novice catalog.
- Prototype kits must not appear in the default novice catalog.
- Curation state must also allow exposure; a Usable kit can remain PreviewOnly
  when refreshed legibility or review evidence is missing.
- Usable kits may appear only when curation evidence is complete: visual
  direction evidence, readable primary-control evidence, and no blocked reasons.
- Showcase kits require Usable evidence plus human approval and adversarial
  visual review.
- No profile may be marked Usable in catalog curation without visual direction
  evidence and readable primary-control evidence.
- No profile may be marked Showcase in catalog curation without human review.
- Incompatible style/provider combinations are hidden from novice users.
- Default Visual Foundry copy must not expose provider packs, sockets, ports,
  family facets, scalar paths, conformance bindings, semantic IDs, fragment
  remaps, raw recipe terms, or operation IDs.

The current built-in kit metadata records Usable automated evidence where
available. Catalog curation narrows novice exposure to the current evidence set:
`sci-fi-crate`, `roman-bridge-hq`, and `stylized-lamp`. PreviewOnly profiles are
available only with the preview catalog switch.
