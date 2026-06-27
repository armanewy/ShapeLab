# HQ Kit Review Tiers

Kit exposure is controlled by quality tier.

The app home catalog additionally applies catalog curation state. See
[`VISUAL_FOUNDRY_CATALOG_CURATION.md`](VISUAL_FOUNDRY_CATALOG_CURATION.md).
This prevents profiles with weak or stale visual evidence from appearing to
novice users even when the profile compiles or has older automated metadata.

| Curation State | Default Novice Catalog | Required Evidence |
| --- | --- | --- |
| HiddenDraft | Hidden | Direct developer-test use only |
| PreviewOnly | Hidden unless preview catalog is enabled | Compile/preview evidence, incomplete or stale novice-readiness evidence |
| Usable | Eligible | Current visual direction and readable primary-control evidence |
| Showcase | Eligible with badge after approval | Usable evidence plus human and adversarial review |

| Tier | Default Novice Catalog | Required Evidence |
| --- | --- | --- |
| Draft | Hidden | Internal compile/render evidence when available |
| Prototype | Hidden unless explicitly enabled | Compile plus at least one preview |
| Usable | Eligible after review | Preview, contact sheet, controls, candidates, export/reopen |
| Showcase | Eligible after approval | Usable evidence plus human and adversarial review |

Draft and Prototype content may exist in authoring and test catalogs. They must
not be presented as production-ready assets to novice users.

Usable curation content can appear in Visual Foundry only after current visual
direction evidence and readable primary-control evidence are recorded. The
intended task must stay within the novice workflow: choose a template, generate
directions, customize, pack, and export.

Showcase content is a stronger product claim. The report can record automatic
evidence, but only a reviewer can mark human approval and adversarial visual
review complete.

As of the starter-template quality benchmark pass, the default novice catalog is
limited to `sci-fi-crate` and `stylized-lamp`. `roman-bridge-hq`, older
expansion profiles, and automated gear profiles remain PreviewOnly until review
evidence is refreshed or approved. `moba-hero-clay` remains HiddenDraft and does
not appear in the app catalog.
