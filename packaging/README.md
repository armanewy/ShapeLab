# Packaging Notes

Shape Lab packaging is currently manual and conservative. This directory records what should be included in platform artifacts without adding untested installer frameworks, automatic publishing, code-signing secrets, notarization, or app-store flows.

## Common Contents

Each platform archive should include:

- the `orchard-app` desktop binary for that platform
- the `orchard-cli` binary if demo or validation tooling is part of the release
- `README.md`
- `docs/building.md`
- `packaging/LICENSES.md`
- `packaging/THIRD_PARTY.md`
- `packaging/licenses/`
- generated demo contact sheets when useful for release review
- a generated `release-readiness-verified.json` from
  `orchard-cli release-readiness --verify-visual-gate`

The app is offline-first. Do not add updater, telemetry, cloud sync, or network-required startup behavior to packaging.

## Windows

Build:

```powershell
cargo build -p orchard-app --release
cargo build -p orchard-cli --release
cargo run -p orchard-cli --release -- release-readiness --verify-visual-gate --out target\release-readiness-verified.json
```

Package `target\release\orchard-app.exe` and optionally `target\release\orchard-cli.exe` in a `.zip` archive with the common contents.

No MSI, MSIX, automatic signing, or publishing flow is configured in this branch. If signing is added later, keep certificates and secrets outside the repository and document the exact signing command.

## macOS

Build:

```bash
cargo build -p orchard-app --release
cargo build -p orchard-cli --release
cargo run -p orchard-cli --release -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
```

For local macOS app-bundle smoke tests, create `target/release/Shape Lab.app`:

```bash
scripts/package_macos_app.sh
```

The script wraps the release `orchard-app` binary with
`packaging/macos/Info.plist`, giving Shape Lab a stable LaunchServices identity
for local launch, screenshot, and Computer Use checks.

This branch does not claim notarized or signed macOS output. Public
distribution still needs a finalized `.icns`, code signing, notarization, and
installer/archive validation.

## Linux

Install native build dependencies first; see `docs/building.md`.

Build:

```bash
cargo build -p orchard-app --release
cargo build -p orchard-cli --release
cargo run -p orchard-cli --release -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
```

Package `target/release/orchard-app` and optionally `target/release/orchard-cli` in a `.tar.gz` archive with the common contents.

Runtime systems need working graphics drivers and either X11 or Wayland support. Distro packages such as `.deb`, `.rpm`, Flatpak, or AppImage are intentionally not added here until they can be tested on the target distribution.

## Icons

`packaging/icons/shape-lab-icon.svg` and `packaging/icons/shape-lab-icon-monochrome.svg` are original placeholders. They can be converted into platform-specific icon formats later, but they are not wired into the app binary by this branch.
