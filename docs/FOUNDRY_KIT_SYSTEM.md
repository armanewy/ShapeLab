# Foundry Kit System

Wave 33 adds curated Foundry kits as the product packaging layer above exact
Foundry catalogs. A kit is not a second geometry source. It is a versioned
manifest that summarizes an existing family, style, provider set, controls,
candidate strategy, quality gate, compatibility matrix, and review manifest.

## Package Shape

`FoundryKitPackage` contains:

- `FoundryKit`: kit ID, display name, refs to package sections, preview camera
  policy, quality tier, catalog visibility policy, source fixture slug when
  available, and product category chips.
- `FamilyBlueprint`: role inventory, required/optional roles, provider slot
  expectations, attachment expectations, high-level scale policy, and export
  naming policy.
- `ProviderPack`: supplied slots, provider options, role coverage,
  attachment/detail tags, triangle budget estimates, and compatibility tags.
- `StylePack`: visual-language policy, allowed/forbidden tags, compatible and
  incompatible provider packs, and metadata-only future material vocabulary.
- `ControlProfile`: novice-facing controls, with seven primary controls by
  default, ownership declarations, visible-effect expectations, topology
  behavior, option visibility, and default locks.
- `CandidateStrategyPack`: strategy names, allowed controls, explanation
  templates, diversity goals, invalid-state rejection, and lock-respect policy.
- `QualityGateProfile`: required tier and mesh, candidate, contact-sheet,
  export, and manual-review gates.
- `KitCompatibilityMatrix`: compatible and incompatible style/provider pairs.
- `KitReviewManifest`: requested/achieved tier, optional local reviewer, human
  approval, adversarial review, contact sheets, benchmark refs, limitations,
  and blocked reasons.
- `KitCatalogManifest`: kit IDs, default-visible IDs, preview-catalog IDs, and
  product-safe hidden reasons.

## Built-In Kits

The ten existing Visual Foundry profiles are exposed as kit metadata through
`shape-foundry-catalog`. The kit mapper derives package metadata from each
fixture catalog and its exact family, style, implementation, and customizer
documents. It does not rewrite geometry.

Current automated Wave 32 baseline tiers are recorded as kit quality tiers:

| Kit | Tier |
| --- | --- |
| Roman Timber Bridge | Prototype |
| Sci-Fi Industrial Crate | Usable |
| Stylized Furniture Lamp | Usable |
| Market Stall Kit | Prototype |
| Sci-Fi Door Panel | Usable |
| Coopered Storage Barrel | Prototype |
| Wayfinding Signpost | Usable |
| Workshop Chair | Prototype |
| Market Handcart | Prototype |
| Storybook Tree | Prototype |

Usable means automated evidence exists. It does not mean default novice catalog
exposure is enabled. Manual review approval is still required before exposure.

## CLI

```bash
cargo run -p shape-cli -- foundry-kit validate roman-bridge
cargo run -p shape-cli -- foundry-kit inspect roman-bridge
cargo run -p shape-cli -- foundry-kit preview roman-bridge --out-dir target/foundry-kit/roman-preview
cargo run -p shape-cli -- foundry-kit contact-sheet roman-bridge --out-dir target/foundry-kit/roman-contact
cargo run -p shape-cli -- foundry-kit package roman-bridge --out-dir target/foundry-kit/roman-package
cargo run -p shape-cli -- foundry-kit review roman-bridge --quality-report target/hq-benchmark/roman-bridge/quality-report.json --out target/foundry-kit/roman-review.json
```

The kit argument may be a built-in slug, a package JSON file, or a directory
containing `foundry-kit-package.json`. The `storybook-tree` and `scifi-crate`
aliases resolve to the canonical built-in slugs.

Validation checks schema versions, section refs, family/style/provider
compatibility, required role coverage, required provider slots, duplicate
visible control ownership, primary-control count, visibility policy, contact
sheet evidence, Showcase approval, and catalog manifest consistency.

## UI Boundary

The app-side kit card view consumes only product-safe data: display name,
quality badge, style name, category chips, review badge, clay-preview badge,
and a plain-language hidden reason. The default novice catalog hides pending
kits; preview/developer catalog mode can show installed kits for internal QA.
Technical terms such as provider packs, sockets, ports, family facets, scalar
paths, conformance bindings, recipe internals, and operation IDs stay out of
the default Visual Foundry surface.
