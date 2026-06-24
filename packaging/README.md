# Packaging Notes

Shape Lab packaging is currently manual and conservative. This directory records what should be included in platform artifacts without adding untested installer frameworks, automatic publishing, code-signing secrets, notarization, or app-store flows.

## Common Contents

Each platform archive should include:

- the `shape-app` desktop binary for that platform
- the `shape-cli` binary if demo or validation tooling is part of the release
- `README.md`
- `docs/building.md`
- `packaging/LICENSES.md`
- `packaging/THIRD_PARTY.md`
- `packaging/licenses/`
- generated demo contact sheets when useful for release review
- a generated `release-readiness.json` from `shape-cli release-readiness`

The app is offline-first. Do not add updater, telemetry, cloud sync, or network-required startup behavior to packaging.

## Windows

Build:

```powershell
cargo build -p shape-app --release
cargo build -p shape-cli --release
cargo run -p shape-cli --release -- release-readiness --out target\release-readiness.json
```

Package `target\release\shape-app.exe` and optionally `target\release\shape-cli.exe` in a `.zip` archive with the common contents.

No MSI, MSIX, automatic signing, or publishing flow is configured in this branch. If signing is added later, keep certificates and secrets outside the repository and document the exact signing command.

## macOS

Build:

```bash
cargo build -p shape-app --release
cargo build -p shape-cli --release
cargo run -p shape-cli --release -- release-readiness --out target/release-readiness.json
```

For now, package the raw executable in a `.tar.gz` or `.zip` archive with the common contents. A future `.app` bundle should include a finalized icon, `Info.plist`, signing, and notarization steps after those have been tested on macOS.

This branch does not claim notarized or signed macOS output.

## Linux

Install native build dependencies first; see `docs/building.md`.

Build:

```bash
cargo build -p shape-app --release
cargo build -p shape-cli --release
cargo run -p shape-cli --release -- release-readiness --out target/release-readiness.json
```

Package `target/release/shape-app` and optionally `target/release/shape-cli` in a `.tar.gz` archive with the common contents.

Runtime systems need working graphics drivers and either X11 or Wayland support. Distro packages such as `.deb`, `.rpm`, Flatpak, or AppImage are intentionally not added here until they can be tested on the target distribution.

## Icons

`packaging/icons/shape-lab-icon.svg` and `packaging/icons/shape-lab-icon-monochrome.svg` are original placeholders. They can be converted into platform-specific icon formats later, but they are not wired into the app binary by this branch.
