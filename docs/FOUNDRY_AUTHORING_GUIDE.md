# Foundry Authoring Guide

Wave 23 introduces a typed local authoring package for Visual Foundry profiles.
The goal is to make new asset families possible without editing core engine
crates for every profile.

Foundry Author is a technical content-author lane. It produces exact local
catalog packages that the existing Foundry compiler can build. It is not a
visual node editor, arbitrary mesh importer, LLM surface, DCC integration, or
materials/UV/rigging workflow.

## Package Shape

A Foundry Author profile is stored as JSON using
`FoundryAuthorProfilePackage` from `shape-foundry-catalog`.

The package contains:

- `family`: the theme-neutral asset-family contract, including roles,
  attachment rules, conformance rules, export requirements, parameter slots,
  variant rules, and compatible styles.
- `style`: the style-kit vocabulary, family facets, provider vocabulary,
  repetition/symmetry/exaggeration policy, and style tags.
- `family_implementation`: executable family bindings and base recipe data.
- `style_implementation`: executable style providers and fragments.
- `customizer_profile`: novice controls, sections, domains, and candidate
  strategies.
- `control_state`: initial values for a new Foundry document.
- `preview_cameras`: author-declared preview requests for profile tooling.
- `pack_policies`: pack-size and shared-control guidance for coherent exports.

Validation converts the package into the existing exact catalog layout, then
compiles it through `shape_foundry`. A package is valid only when its contracts
parse, cross-references match, controls validate, catalog locks are exact, and
the generated asset compiles with accepted conformance.

## CLI Workflow

Create a starting profile:

```powershell
cargo run -p shape-cli -- foundry-new-profile --template roman-bridge --out author-profile.json
```

Built-in templates are `roman-bridge`, `roman-bridge-hq`, `sci-fi-crate`,
`stylized-lamp`, `market-stall`, `sci-fi-door`, `storage-barrel`, `signpost`,
`workshop-chair`, `handcart`, and `stylized-tree`. The compact `scifi-crate`
spelling is accepted as a compatibility alias.

Validate the profile:

```powershell
cargo run -p shape-cli -- foundry-validate-profile author-profile.json
```

Render a build proof and preview:

```powershell
cargo run -p shape-cli -- foundry-preview-profile author-profile.json --out-dir preview-out
```

The preview command writes `preview-cameras.json`, `preview.png`, and one
camera-specific image at `previews/<camera-id>.png`. Camera width, height, yaw,
and pitch are honored by the preview renderer; roll is recorded for authoring
metadata but is not applied by the current CPU renderer.

Package the profile into an exact local catalog:

```powershell
cargo run -p shape-cli -- foundry-package-profile author-profile.json --out-dir packaged-profile
```

The package command writes:

- `foundry-author-profile.json`
- `foundry-author-validation.json`
- `catalog/foundry-document.json`
- `catalog/catalog-manifest.json`
- `catalog/<stable-id>.json` entries
- `build-proof/*` compile, conformance, model-validation, OBJ, preview manifest,
  default preview, and camera-specific preview files

The generated `catalog` directory can be built with the existing command:

```powershell
cargo run -p shape-cli -- foundry-build --catalog packaged-profile/catalog --document packaged-profile/catalog/foundry-document.json --out-dir build
```

## Kit Packaging

Wave 33 adds a curated kit packaging layer above Foundry Author profiles. A
Foundry kit summarizes the exact authored family/style/provider/control content
with product-facing metadata, quality gates, compatibility policy, and review
evidence. It does not replace the exact catalog or compiler path.

Built-in kits can be inspected and packaged with:

```powershell
cargo run -p shape-cli -- foundry-kit inspect roman-bridge
cargo run -p shape-cli -- foundry-kit package roman-bridge --out-dir kit-package
```

Authoring remains technical. The Visual Foundry kit-card layer consumes only
product-safe data such as display name, quality badge, style name, category
chips, review status, clay-preview status, and plain-language hidden reasons.
Pending kits stay hidden from the default novice catalog until review approval
is recorded.

## Authoring Rules

Keep the novice surface small. A profile should expose no more than seven
primary controls unless there is a deliberate product reason and tests proving
the surface remains usable.

Every visible control should use human labels, certified domains, and
whole-model preview IDs. Authors should not require users to understand scalar
paths, provider IDs, role bindings, or semantic IDs.

Every candidate strategy must reference known customizer controls. Strategies
should describe useful whole-model directions, not isolated part tweaks.

Every package must include at least one preview camera and at least one pack
policy. These are metadata for tooling and review; they do not replace compiler
validation.

Do not claim arbitrary import editability from an author profile. Foundry Author
packages create authored family catalogs. Import and reconstruction remain in
the Semantic Reconstruction Lab boundary described in
`docs/FOUNDRY_VS_INVERSE_ROLES.md`.
