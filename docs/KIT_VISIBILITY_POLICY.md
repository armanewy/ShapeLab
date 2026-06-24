# Kit Visibility Policy

Kit visibility is controlled by quality tier and review evidence.

| Tier | Default Novice Catalog | Preview/Developer Catalog |
| --- | --- | --- |
| Draft | Hidden | Hidden or local-only |
| Prototype | Hidden | Eligible |
| Usable | Eligible only after review evidence | Eligible |
| Showcase | Eligible with badge only after human and adversarial review | Eligible |

## Rules

- Draft kits must not appear in the default novice catalog.
- Prototype kits must not appear in the default novice catalog.
- Usable kits may appear only when review evidence is complete: achieved tier,
  human approval, contact-sheet refs, benchmark refs, and no blocked reasons.
- Showcase kits require Usable evidence plus human approval and adversarial
  visual review.
- Incompatible style/provider combinations are hidden from novice users.
- Default Visual Foundry copy must not expose provider packs, sockets, ports,
  family facets, scalar paths, conformance bindings, semantic IDs, fragment
  remaps, raw recipe terms, or operation IDs.

The current built-in kit metadata records Usable automated evidence where
available, but keeps default novice exposure disabled until manual review is
recorded.
