# Clean Clone Verification Gate

Use this gate from a fresh clone of `main` before making a release-readiness or
product-stability claim.

## Setup

```bash
git clone https://github.com/armanewy/ShapeLab.git
cd ShapeLab
rustup show
```

## Required Commands

```bash
cargo fmt --all --check
cargo test -p shape-app --lib foundry --jobs 1
cargo test -p shape-search foundry --jobs 1
cargo test -p shape-render foundry --jobs 1
cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1
cargo test -p shape-foundry-catalog --test roman_bridge --jobs 1
cargo test -p shape-foundry-catalog --test stylized_lamp --jobs 1
cargo test -p shape-cli starter_template_dogfood --jobs 1
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
```

## Interpretation

Passing this gate proves the repository builds and the automated recovery
contracts run from a clean clone. It does not prove Make is dogfood-stable. A
clean human dogfood video and the manual gates are still required before a
stable Visual Foundry baseline can be claimed.
