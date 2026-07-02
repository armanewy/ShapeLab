# Third-Party Dependencies

Object Orchard uses Rust crates from crates.io plus first-party workspace crates. The resolved dependency graph is pinned by `Cargo.lock`; root dependency versions are not changed by the release-infrastructure branch.

## Inventory Commands

Run these from the repository root when preparing a release:

```bash
mkdir -p target
cargo metadata --format-version 1 --locked > target/cargo-metadata.json
cargo tree --locked --workspace > target/dependency-tree.txt
```

Those files are generated review artifacts. They are not committed by this branch because they include machine-specific absolute paths and can become stale.

## Direct Third-Party Crates

| Crate | Used for |
| --- | --- |
| `anyhow` | CLI and app error context |
| `clap` | CLI argument parsing |
| `criterion` | benchmark support |
| `crossbeam-channel` | desktop background job communication |
| `eframe` | native desktop application shell |
| `egui` | immediate-mode UI |
| `env_logger` | local logging setup |
| `glam` | vector and matrix math |
| `image` | PNG encoding and image buffers |
| `log` | logging facade |
| `proptest` | property tests |
| `rand`, `rand_chacha`, `rand_distr` | deterministic candidate generation |
| `rayon` | CPU parallelism |
| `rfd` | native file dialogs |
| `serde`, `serde_json` | project and document serialization |
| `tempfile` | CLI integration tests |
| `thiserror` | typed errors |

Most direct dependencies publish permissive Rust ecosystem licenses such as MIT, Apache-2.0, or dual MIT/Apache-2.0. Verify the full transitive license set from the resolved metadata during final packaging, especially before public binary distribution.

## Review Boundary

This document is dependency documentation, not a legal approval. It records how to reproduce the dependency inventory and which direct third-party crates the workspace uses.
