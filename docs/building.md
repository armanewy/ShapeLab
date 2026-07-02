# Building Object Orchard

Object Orchard is a native Rust desktop application plus a headless CLI. The build does not require a browser toolchain, server runtime, Blender, network access from the application, or code-signing secrets.

## Toolchain

Use the Rust toolchain selected by `rust-toolchain.toml`.

```bash
rustup show
cargo --version
```

The workspace currently builds on stable Rust with `rustfmt` and `clippy` installed by the toolchain file.

## Native Dependencies

Windows requires the Microsoft C++ build tools that normally come with Visual Studio Build Tools or Visual Studio.

macOS requires Xcode Command Line Tools:

```bash
xcode-select --install
```

Linux desktop builds compile `eframe`/`egui` with native windowing and GPU backends. On Ubuntu GitHub runners and recent Debian/Ubuntu desktops, install:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  libgl1-mesa-dev \
  libwayland-dev \
  libx11-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libxcursor-dev \
  libxi-dev \
  libxinerama-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libxrandr-dev \
  pkg-config
```

Package names vary across Linux distributions. The important native capabilities are OpenGL/Vulkan-capable graphics drivers, X11 or Wayland development headers, xkbcommon headers, and `pkg-config`.

## Reproducible Command List

Run these from the repository root:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p orchard-app --release
cargo run -p orchard-cli -- --help
cargo run -p orchard-cli -- release-readiness --out target/release-readiness.json
cargo run -p orchard-cli -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
pwsh -File scripts/generate_demo_assets.ps1 -OutDir target/demo-assets -ProposalCount 12 -ResultCount 3 -MeshResolution 12
```

The release desktop binary is written under `target/release/` as `orchard-app` on Unix-like systems and `orchard-app.exe` on Windows.

`orchard-cli release-readiness` writes the Wave 30 machine-readable status report
for performance bounds, CPU/GPU rendering status, persistence support,
packaging/signing state, and window-regression coverage. Add
`--verify-visual-gate` when preparing a release-readiness artifact that must
include computed all-profile option-thumbnail evidence.

## Demo Assets

The demo asset script is a thin wrapper around `orchard-cli demo`. It generates deterministic project JSON, OBJ meshes, PNG previews, a contact sheet, and a summary JSON for each selected preset.

```bash
pwsh -File scripts/generate_demo_assets.ps1 -Help
pwsh -File scripts/generate_demo_assets.ps1 -Preset desk-lamp -OutDir target/demo-desk-lamp
pwsh -File scripts/generate_demo_assets.ps1 -ReleaseCli -OutDir target/demo-assets
```

The script is suitable for CI because it uses the headless CLI only.

## CI Coverage

CI runs `rustfmt`, workspace tests, clippy with warnings denied, a release-mode desktop build matrix for Linux, Windows, and macOS runners, and a headless CLI contact-sheet generation job.

These are build and automated smoke checks. They do not claim manual platform QA, hardware coverage, installer validation, code signing, notarization, package-manager publishing, or app-store submission.
