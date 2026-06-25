# Wave 35 - Foundry Author Studio v0

Wave 35 adds a gated Foundry Author Studio v0 for internal/pro kit authors.

## Implemented

- Added `shape_foundry::author_studio` contracts and validators.
- Added an explicit `FoundryAuthorStudioGate`; default release mode hides the
  authoring surface.
- Added the nine-step authoring workflow: Kit Overview, Family Blueprint,
  Provider Packs, Style Compatibility, Controls, Candidate Strategies, Preview
  Cameras, Quality Gates, Review & Package.
- Added role, provider, socket/port, style compatibility, control mapping,
  candidate strategy, preview camera, quality gate, and package export
  descriptors.
- Added app-side gated view contracts under `shape-app` without changing the
  default Visual Foundry product shell.
- Added tests proving default non-exposure, gated exposure, descriptor
  validation, plain unsupported quality-gate states, and package export refs.

## Product Boundary

Visual Foundry remains the novice workflow. Author Studio is not shown unless
developer/pro authoring mode is explicitly enabled. Technical terms such as
provider packs, sockets, ports, and descriptor-only import notes stay out of the
default product-visible string set.

Author Studio does not add LLM integration, arbitrary mesh editing, materials,
UVs, rigging, animation, marketplace workflow, or Semantic Reconstruction Lab
diagnostics.

## Quality Gate Behavior

Author Studio launches only existing CLI-backed gates. Unsupported states are
reported honestly with reasons, such as missing output directories, missing
quality-report refs, missing package manifest refs, or missing verified
canonical built-in backing.

Package export records refs to kit manifests, provider/style manifests, control
profiles, candidate strategies, quality gate profiles, review manifests,
quality reports, and contact sheets using the current `shape-cli foundry-kit
package` flat filename layout. Human review still controls default catalog
visibility.

## Verification

Wave 35 verification commands:

```bash
cargo fmt --all --check
cargo test -p shape-app foundry_author
cargo test -p shape-foundry
cargo test -p shape-cli foundry_kit
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```

The final Wave 35 commit should be `Add Foundry Author Studio v0`.
