# Wave 5.6 Release Infrastructure Notes

## Scope

This branch adds release infrastructure only. It does not change Rust source, dependency versions, application behavior, publishing behavior, installer frameworks, web builds, code-signing secrets, or network-dependent application behavior.

## CI

The GitHub Actions workflow now separates:

- `rustfmt`
- workspace tests
- workspace clippy with `-D warnings`
- release-mode `shape-app` builds on Linux, Windows, and macOS runners
- headless `shape-cli` demo asset generation with uploaded contact-sheet artifacts

Linux jobs install native desktop build headers for `eframe`/`egui` windowing and graphics backends. Cargo dependency caching remains enabled through the Rust toolchain setup action in each job.

The desktop matrix checks that the app builds on hosted CI runners. It is not a substitute for manual QA on real end-user machines.

## Script

`scripts/generate_demo_assets.ps1` wraps `cargo run -p shape-cli -- demo` for the built-in presets. It validates that each preset produced at least a contact sheet, summary JSON, and accepted project JSON.

Example:

```bash
pwsh -File scripts/generate_demo_assets.ps1 -OutDir target/demo-assets -ProposalCount 12 -ResultCount 3 -MeshResolution 12
```

## Packaging Docs

The `packaging/` directory documents manual packaging expectations for Windows, macOS, and Linux. It intentionally avoids untested installer frameworks, automatic publishing, signing secrets, or notarization claims.

Icon files under `packaging/icons/` are original SVG placeholders for future package metadata. They are not wired into app binaries in this branch.

## Local Verification

The requested local verification commands for this branch are:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p shape-app --release
pwsh -File scripts/generate_demo_assets.ps1 -Help
pwsh -File scripts/generate_demo_assets.ps1 -Preset desk-lamp -OutDir target/demo-smoke -ProposalCount 8 -ResultCount 2 -MeshResolution 10
```

Only the local Windows environment is available to this worker. Linux and macOS coverage is provided as CI configuration, not as a local test claim.
