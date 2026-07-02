# Cleanup Wave 1 Integration Report

Date: 2026-07-01

Branch: `codex/cleanup-wave-1-integration`

## Merged Branches

| Order | Branch | Result |
| ---: | --- | --- |
| 1 | `codex/obsolete-documentation-purge` | Merged. Obsolete docs were removed and active docs index tests were preserved. |
| 2 | `codex/remove-legacy-candidate-search-paths` | Merged. Direct primitive Make remains candidate-tray free by default. |
| 3 | `codex/split-shape-app-foundry-modules` | Merged. `foundry/app.rs` was split into focused app modules. |
| 4 | `codex/split-semantic-core-crates` | Merged. Oversized semantic backend files were split into focused modules. |
| 5 | `codex/workspace-dead-code-dependency-purge` | Merged. Obsolete program/decompiler crates and stale fixtures were removed. |
| 6 | `codex/shape-core-legacy-retirement-pass` | Merged. `shape-app` no longer depends directly on `shape-core`. |

## Integration Fixes

- Resolved the `shape-app` split plus candidate-removal merge by preserving the
  split app layout and porting the legacy candidate UI gate into the new modules.
- Updated moved app tests so stale candidate-result recovery is exercised only
  through the explicitly gated non-direct candidate workflow.
- Resolved the `shape-asset` split plus dead-code purge by keeping the
  `asset_core/*` split modules and removing old JSON fixture dependencies from
  the moved tests.
- Updated `docs/RUST_FILE_SIZE_EXCEPTIONS.md` to the post-integration exception
  list. Deleted crates and already-split files are no longer listed.

## Removed Code And Fixtures

- Removed obsolete `shape-decompiler`.
- Removed obsolete `shape-program`.
- Removed obsolete `shape-program-verify`.
- Removed stale Shape Asset JSON fixtures that belonged to removed legacy
  fixture coverage.
- Removed the obsolete Box Primitive UI cleanup script.

## Dependency Result

- Workspace manifests and `Cargo.lock` were reconciled by the dead-code purge and
  shape-core retirement branches.
- `shape-app` no longer lists `shape-core` as a direct dependency.
- `shape-render` re-exports `Aabb` for app/render consumers that still need the
  low-level bounds type.

## Oversized Files

`python3 scripts/check_rust_file_size.py` passes. Remaining temporary exceptions
are documented in `docs/RUST_FILE_SIZE_EXCEPTIONS.md`.

Notable resolved exceptions:

- `crates/shape-app/src/foundry/app.rs` is now below the threshold.
- `crates/shape-asset/src/lib.rs` is now below the threshold.
- Deleted `shape-decompiler`, `shape-program`, and `shape-program-verify` files
  are no longer exceptions.

Remaining debt is concentrated in app state/jobs/panels, CLI dispatch,
render/project/search modules, and retained legacy/family compatibility crates.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `python3 scripts/check_source_hygiene.py` | Passed |
| `python3 scripts/check_rust_file_size.py` | Passed |
| `python3 scripts/audit_cleanup_inventory.py` | Passed |
| `cargo test --workspace --jobs 1` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo build --release --workspace` | Passed |

The first workspace test attempt was blocked by local disk exhaustion while
writing Rust test artifacts. Generated `target/` directories were removed, disk
space was recovered, and the gate passed on rerun.

## UI Snapshot Gate

No deterministic UI state snapshot harness is present in this branch. Native
macOS screenshot capture was not used as a hard gate for this cleanup
integration.

## Known Cleanup Debt Before Rename

- Split the remaining large app state, job, and panel files.
- Split CLI command dispatch by subsystem.
- Continue retiring or isolating legacy `shape-core::ShapeDocument` users.
- Split retained family/search/render/project compatibility files.
- Keep legacy candidate/search UI explicitly gated and out of direct primitive
  Make by default.
- Do not start broad rename work until the remaining cleanup exceptions are
  deliberately scheduled.
