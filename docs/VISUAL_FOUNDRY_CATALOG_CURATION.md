# Visual Foundry Catalog Curation

Visual Foundry uses catalog curation state to decide what appears in the app
home catalog. This is separate from whether a profile compiles or has internal
tests. A weak profile can remain in source and continue to run in developer
tests without being shown to novice users.

## States

| State | Default Novice Catalog | Preview Catalog | Meaning |
| --- | --- | --- | --- |
| HiddenDraft | Hidden | Hidden | Direct developer-test content only. |
| PreviewOnly | Hidden | Visible | Internal or experimental profile with incomplete novice-readiness evidence. |
| Usable | Visible | Visible | Has current visual direction and readable primary-control evidence. |
| Showcase | Visible | Visible | Usable plus human and adversarial review approval. |

Rules:

- default novice catalog shows only Usable or Showcase profiles;
- PreviewOnly appears only when the preview catalog switch is enabled;
- HiddenDraft never appears in the app catalog;
- Usable requires visual direction evidence and readable primary-control evidence;
- Showcase requires human review and adversarial review approval.

## Built-In Profile Curation

| Profile | State | Reason |
| --- | --- | --- |
| sci-fi-crate | Usable | Current crate evidence covers whole-model clay directions and controls. |
| stylized-lamp | Usable | Current lamp evidence covers legible directions and primary-control endpoints. |
| roman-bridge-hq | PreviewOnly | Current benchmark evidence reports a visibly disconnected default brace. |
| roman-bridge | PreviewOnly | Legacy bridge remains available for testing while the HQ profile is under refreshed review. |
| market-stall | PreviewOnly | Needs refreshed six-direction legibility evidence. |
| sci-fi-door | PreviewOnly | Older evidence exists; refreshed review is required before novice exposure. |
| storage-barrel | PreviewOnly | Needs refreshed direction and control evidence. |
| signpost | PreviewOnly | Older evidence exists; refreshed review is required before novice exposure. |
| workshop-chair | PreviewOnly | Needs refreshed six-direction legibility evidence. |
| handcart | PreviewOnly | Needs refreshed six-direction legibility evidence. |
| stylized-tree | PreviewOnly | Needs refreshed clay legibility evidence. |
| fantasy-sword | PreviewOnly | Automated gear evidence exists; novice exposure remains review-gated. |
| round-shield | PreviewOnly | Automated gear evidence exists; novice exposure remains review-gated. |
| hero-helmet | PreviewOnly | Automated gear evidence exists; novice exposure remains review-gated. |
| pauldron-pair | PreviewOnly | Automated gear evidence exists; novice exposure remains review-gated. |
| chest-armor | PreviewOnly | Automated gear evidence exists; novice exposure remains review-gated. |
| moba-hero-clay | HiddenDraft | Direct-test clay profile, not a product catalog entry. |

The preview catalog switch is for internal testing and review. It must not be
treated as product readiness or Showcase approval.
