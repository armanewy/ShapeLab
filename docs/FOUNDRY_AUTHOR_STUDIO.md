# Foundry Author Studio

Wave 35 adds Foundry Author Studio as a gated technical authoring lane for
internal and pro kit authors. It prepares and reviews Foundry kit packages; it
does not replace the novice Visual Foundry workflow.

## Product Split

Visual Foundry stays the default asset-user surface:

- choose a kit and style
- generate visual directions
- customize with a small set of meaningful controls
- pack and export assets

Foundry Author Studio is hidden unless an explicit developer/pro authoring gate
is enabled by the host. It may show technical package language because its users
are preparing kit metadata, quality evidence, and review manifests.

The Semantic Reconstruction Lab remains a separate import and diagnostic lane.
Author Studio does not expose inverse reconstruction, arbitrary raw mesh
editing, materials, UVs, textures, rigging, animation, marketplace workflow, or
LLM integration.

## Workflow Steps

Author Studio exposes this fixed workflow:

1. Kit Overview
2. Family Blueprint
3. Provider Packs
4. Style Compatibility
5. Controls
6. Candidate Strategies
7. Preview Cameras
8. Quality Gates
9. Review & Package

These steps are encoded by `foundry_author_studio_steps()` in
`shape-foundry`. The default release gate returns no steps and an unavailable
reason.

## Authoring Model

Author Studio descriptors are metadata over the existing exact Foundry kit
package. They do not bypass `FoundryKitPackage` validation or the exact catalog
compiler path.

The current descriptor model covers:

- role and export-part labeling
- provider registration metadata
- socket and port requirements
- style/provider compatibility policy
- control ownership and topology behavior
- candidate strategy policy
- preview camera policy
- CLI-backed quality gate launch rows
- package export refs for manifests, quality reports, and contact sheets

When a path is not implemented, the descriptor must say so plainly. For
example, component import is descriptor-only until mesh import support is
reviewed.

## Validation

Author Studio validators catch common authoring errors before a kit is reviewed:

- duplicate or missing role IDs
- provider roles and socket targets that do not reference the family blueprint
- missing required socket metadata or compatibility tags
- style tags marked both allowed and forbidden
- compatible and incompatible style packs with the same ID
- missing style-language policies
- more than seven visible primary novice controls by default
- duplicate visible control ownership of a family/provider slot
- topology-changing controls that are not discrete choices
- candidate strategies that reference unknown controls
- candidate explanations that expose raw recipe/scalar/internal terms
- preview cameras without fitted-scale or lighting policy
- unsupported cameras without an honest reason
- option galleries that mix camera or scale policy within one control
- contact-sheet policies missing front, side, back, or three-quarter views

Validation issues are structured with a subject, code, and human-readable
message so an app can show actionable author feedback without exposing these
terms to Visual Foundry users.

## Quality Gates

Author Studio does not create a separate quality system. It emits launch rows
for the existing CLI gates:

```bash
shape-cli foundry-kit validate <kit>
shape-cli foundry-kit preview <kit> --out-dir <dir>
shape-cli foundry-kit contact-sheet <kit> --out-dir <dir>
shape-cli hq-quality-benchmark --profile <slug> --out-dir <dir> --verify-export
shape-cli foundry-kit review <kit> --quality-report <report> --out <manifest>
shape-cli foundry-kit package <kit> --out-dir <dir>
```

If a task cannot run from the current metadata, the launch row is unsupported
and includes a plain reason. Examples include missing output directory, missing
quality report reference, missing package manifest reference, or no verified
canonical built-in backing for preview/contact sheet/HQ benchmark commands.

## Package Export

The package exporter records refs to the evidence and package components needed
for review:

- kit manifest
- provider pack manifests
- style pack manifests
- control profile
- candidate strategy pack
- quality gate profile
- review manifest
- quality report refs
- contact sheet refs

These refs align with `shape-cli foundry-kit package` output filenames such as
`kit-manifest.json`, `provider-pack.json`, `style-pack.json`,
`control-profile.json`, `candidate-strategy-pack.json`,
`quality-gate-profile.json`, and `review-manifest.json`.

These refs are package/review metadata. Human approval remains required before
a kit becomes visible in the default novice catalog, and adversarial visual
review remains required before any Showcase claim.
