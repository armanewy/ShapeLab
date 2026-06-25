# Wave 36 - LLM Foundry Foundation Generator

Wave 36 adds an SDK-free foundation draft system for LLM-assisted Foundry kit
authoring.

## Implemented

- Added `shape_foundry::foundation` draft schema and validation.
- Added serializable allowed authoring commands without forbidden mesh,
  validation-bypass, publishing, or recipe-mutation commands.
- Added deterministic internal fixtures:
  - `fantasy_sword_core_draft`
  - `round_shield_core_draft`
  - `helmet_core_draft`
  - `rustic_bridge_detail_pack_draft`
- Added deterministic adversarial reports and repair suggestions.
- Added materialization from valid foundation drafts into internal Draft kit
  packages.
- Added `shape-cli foundry-foundation` commands for new, validate,
  materialize, adversarial-report, and suggest-repair.
- Added gated Author Studio Foundation Draft panel metadata.

## Boundary

This wave does not add external LLM SDKs, network calls, raw mesh generation,
raw vertex positions, hidden mesh payloads, direct recipe mutation, validation
bypass, automatic novice publishing, materials, UVs, rigging, animation, or
arbitrary imported-mesh editability.

Generated drafts are internal-only in Wave 36. Validation rejects publish flags
and non-internal catalog visibility. Materialized packages remain Draft and
hidden from both novice and developer preview catalogs until authored geometry,
validation, contact sheets where required, adversarial review where required,
and human approval are complete through a later reviewed path.

## Verification

Wave 36 verification commands:

```bash
cargo fmt --all --check
cargo test -p shape-cli foundry_foundation
cargo test -p shape-foundry
cargo test -p shape-app
cargo run -p shape-cli -- foundry-foundation new --category weapons --family sword --out target/foundation/sword-draft.json
cargo run -p shape-cli -- foundry-foundation validate target/foundation/sword-draft.json
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```

The Wave 36 commit should be `Add LLM Foundry foundation draft system`.
