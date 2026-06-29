# Test Gate Policy

Do not run release gates for every local edit. Run the smallest gate that proves
the touched code still works, then widen the gate when branches are handed off,
merged, or prepared for release.

## Tier 0: Inner Loop

Use while editing. Target: under 90 seconds. This tier should stay to affected
crate checks and focused affected-crate tests only.

Typical commands:

```bash
cargo check -p <touched-crate>
cargo test -p <touched-crate> <focused_filter> --jobs 1
```

Examples:

```bash
cargo check -p shape-app
cargo test -p shape-search foundry --jobs 1
cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1
```

## Tier 1: Branch Handoff

Use before handing off or committing a focused branch. Target: 2-5 minutes.
Run affected-crate tests, adjacent tests that cover the touched contract, and
targeted clippy for the owned crate or app surface.

Typical commands:

```bash
cargo fmt --all --check
cargo test -p <touched-crate> --jobs 1
cargo test -p <adjacent-crate> <relevant_filter> --jobs 1
cargo clippy -p <touched-crate> --all-targets -- -D warnings
```

Use `python3 scripts/dev_gate.py --tier branch --changed` to select the concrete
commands from changed paths.

The script prints commands by default. It executes them only when `--run` is
passed.

Do not add a full release build to branch handoff unless the branch touches the
build/profile/release/export stack.

For `shape-app`, run foundry app tests as a library gate:

```bash
cargo test -p shape-app --lib foundry --jobs 1
```

Do not use `cargo test -p shape-app foundry --jobs 1` as a development gate.
The broad filter also matches integration test binaries that path-import the
foundry module and can re-run the same app unit tests several times.

## Tier 2: Integration Branch

Use after merging related branches. Target: 8-15 minutes.
Run merged slice tests and workspace clippy. Add a release build when product
code changed; skip it for docs-only and policy-only integrations.

```bash
cargo fmt --all --check
cargo test -p shape-app --lib foundry --jobs 1
cargo test -p shape-search foundry --jobs 1
cargo test -p shape-render foundry --jobs 1
cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1
cargo test -p shape-foundry-catalog --test roman_bridge --jobs 1
cargo test -p shape-foundry-catalog --test stylized_lamp --jobs 1
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace  # only when product code changed
```

## Tier 3: Main/Release Gate

Use before pushing major integrations to `main` or cutting a release candidate.
This gate is intentionally slow and should be rare.

```bash
cargo fmt --all --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
```

Manual dogfood remains required where product-quality claims or release prompts
require it. It is not a normal unit-test gate.

## Heavy And Manual Gates

These checks are necessary, but should be explicit and rare:

- screenshot/video dogfood
- starter-template full benchmark
- HQ contact sheet generation
- release readiness
- game-ready static package generation
- inverse/recovery corpus
- strict external import/reconstruction
- slow render/contact-sheet tests

When new heavy tests are added, mark them with `#[ignore = "..."]` or place them
behind a named gate/profile so they do not run accidentally during Tier 0 or
Tier 1 work.

## Path Mapping

`scripts/dev_gate.py` encodes the first gate map:

- `crates/shape-app/**`: app check, foundry library tests, direction-board test, app clippy
- `crates/shape-search/**`: shape-search foundry tests, shape-render foundry adjacency
- `crates/shape-render/**`: shape-render foundry tests, surface filter when surface files change
- `crates/shape-foundry-catalog/src/simple_crate.rs`: Simple Crate test plus search/render foundry adjacency
- `crates/shape-foundry-catalog/src/utility_crate.rs`: Utility Crate test plus search/render foundry adjacency
- `crates/shape-foundry-catalog/src/cargo_case.rs`: Cargo Case test plus search/render foundry adjacency
- `crates/shape-foundry-catalog/src/scifi_crate.rs`: Sci-Fi Crate test plus search foundry adjacency; add
  `shape-cli game_ready_static` only when the static surface package path also changed
- `crates/shape-foundry-catalog/src/roman_bridge.rs`: Roman Bridge test
- `crates/shape-foundry-catalog/src/stylized_lamp.rs`: Stylized Lamp test
- `crates/shape-gamekit/**`: surface, rig, and motion filters
- `crates/shape-cli/src/game_ready_static.rs`: game-ready static filter
- `docs/**`: formatting and doc/status tests only; no release build by default

Full workspace test, clippy, and release build are required for main/release.
They are not required for every prompt lane.
