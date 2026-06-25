# Release Readiness

Wave 30 added an explicit release-readiness contract for the native Shape Lab
loop. Wave 31.5 extends it with a headless product UI gate for the redesigned
Visual Foundry shell. Wave 32 adds a separate HQ asset quality benchmark. Wave
33 adds Foundry kit package validation and review manifests. The intent is to
make performance, UI surface, packaging status, curated kit metadata, and
authored content quality evidence auditable without claiming unimplemented GPU,
installer, signing, app-store, UV, material, rigging, animation, marketplace, or
photoreal work.

## Machine-Readable Report

Run the report from the repository root:

```bash
cargo run -p shape-cli -- release-readiness --out target/release-readiness.json
cargo run -p shape-cli -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate --out target/release-readiness-product-ui.json
```

The JSON report records:

- the Visual Foundry product gate. Without `--verify-visual-gate`, this records
  the required command and marks CLI evidence as not run. With
  `--verify-visual-gate`, it compiles the built-in profiles and records computed
  CLI evidence for eleven profiles, seven primary controls per profile, and
  rendered whole-model option thumbnails. The native default-path gate remains
  the explicit ignored app-state release test listed in the report;
- the Visual Foundry product UI gate. Without `--verify-product-ui-gate`, this
  records the expected direct Visual Foundry shell contract. With
  `--verify-product-ui-gate`, it verifies the default product-visible copy
  inventory, direct nonblank startup, zero default-visible pending kits, eleven
  installed preview-catalog kits, the six-card direction board, five direction
  modes, core profile compile/start evidence, pack/export readiness
  representation, and disabled-state reasons;
- the deterministic CPU preview path and bounded preview-cache capacity;
- duplicate preview-cache miss coalescing for repeated keys in one batch;
- Foundry candidate generation proposal bounds and returned-candidate caps;
- project-file recovery/autosave support versus automatic timed autosave UI;
- manual packaging status, with installer, signing, and publishing marked as
  not configured;
- the current split between headless panel/reducer tests and deferred
  desktop-window pixel regression tests.

Release readiness is not the same as Showcase asset quality. A Visual Foundry
build can be product-ready while some kits remain Prototype. Use
`shape-cli hq-quality-benchmark` and the tier rules in
[`HQ_ASSET_QUALITY_BAR.md`](HQ_ASSET_QUALITY_BAR.md) before making Usable or
Showcase claims for individual kits.

## Performance Boundary

Foundry preview rendering remains a deterministic CPU reference path. The
in-memory preview cache is bounded and now coalesces duplicate miss keys within a
single batch so repeated preview requests do not render the same image more than
once before cache insertion.

Foundry candidate generation rejects unbounded proposal requests. The release
budget is:

```text
minimum proposals: 24
maximum proposals: 72
maximum returned candidates: 6
```

Requests for more than six representatives are accepted only as an input
preference; selection remains capped to six.

## Visual Product Gate

Wave 30 treats the native option-card path as an explicit release gate. The
reducer-level Foundry state must render option cards as 64x64 whole-model
thumbnails with RGBA bytes and a camera for every built-in profile. A 1x1
placeholder pixel is only acceptable in isolated panel helper tests that do not
have a compiled Foundry document.

The release gate also requires all eleven built-in profiles to compile and render
through the explicit preview/release evidence path with seven primary controls
and zero technical surface exposure. Default novice catalog exposure is now
controlled by Wave 33 kit review policy. Deterministic contact sheets and
sample export artifacts are produced by `shape-cli foundry-visual-benchmark`
for demo review.

The all-profile thumbnail test is marked `#[ignore]` so it does not slow every
workspace test run. It must be run explicitly before a release-readiness claim.
The CLI report can compute catalog/render evidence with
`shape-cli release-readiness --verify-visual-gate`, but it does not replace the
native app-state gate. A release-readiness claim requires both the CLI verified
report and the ignored app-state test command.

## Product UI Gate

Wave 31.5 adds `shape-cli release-readiness --verify-product-ui-gate`. This is
a fast headless gate over the native Visual Foundry product shell. It verifies:

- `app_shell: direct_visual_foundry`;
- no default Legacy, Implicit, Asset Modeling Lab, Modeling Workspace, Advanced
  Recipe, scalar path, provider ID, role binding, fragment/remap, operation ID,
  semantic ID, conformance binding, SDF, compiler, or decompiler copy;
- zero default-visible pending kits and eleven installed preview-catalog kits;
- startup is not blank;
- Advanced Recipe is not visible in the default path;
- the direction board reserves six whole-model candidate cards and exposes
  Refine, Explore, Silhouette, Structure, and Detail;
- Roman Timber Bridge, Sci-Fi Industrial Crate, and Stylized Furniture Lamp
  compile and expose one to seven primary controls;
- Pack and Export readiness states and disabled reasons are represented.

This gate does not inspect pixels and does not replace human UI review. Manual
screenshots and observations are required by
[`docs/FOUNDRY_UI_MANUAL_GATE.md`](FOUNDRY_UI_MANUAL_GATE.md).

## Release Boundary

The release build remains archive-first. `packaging/README.md` describes manual
Windows, macOS, and Linux package contents. The repository does not contain
private signing certificates, installer framework configuration, notarization
automation, package-manager publishing, or app-store submission logic.

GPU viewport/rendering work remains a future optional backend. Release checks do
not require GPU-specific code beyond the native graphics support already needed
by the desktop framework.

## HQ Asset Quality Boundary

Wave 32 adds:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge --out-dir target/hq-benchmark/roman-bridge --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile all --out-dir target/hq-benchmark --verify-export
```

The benchmark emits `quality-report.json`, clay view PNGs, `contact-sheet.png`,
`mesh-stats.json`, `semantic-parts.json`, `candidate-report.json`,
`controls-visibility-report.json`, and `export-reopen-report.json`.

The report records Draft, Prototype, Usable, or Showcase status. Showcase cannot
be achieved from automation alone; it requires human approval and adversarial
visual review markers. Photoreal screenshots, textures, materials, UVs,
rigging, animation, and marketplace packages are recorded as unsupported rather
than implied.

An automated Usable result is evidence, not automatic catalog exposure. Novice
catalog exposure remains disabled by default until human review approval is
recorded for the kit.

The Wave 32 baseline is intentionally honest: some current built-in kits remain
Prototype because fewer than six candidates survive compile, model validation,
and preview rendering. That does not invalidate release readiness; it records
catalog quality work separately from the product shell gate.

## Foundry Kit Package Boundary

Wave 33 adds:

```bash
cargo run -p shape-cli -- foundry-kit validate roman-bridge
cargo run -p shape-cli -- foundry-kit validate roman-bridge-hq
cargo run -p shape-cli -- foundry-kit inspect roman-bridge
cargo run -p shape-cli -- foundry-kit inspect roman-bridge-hq
cargo run -p shape-cli -- foundry-kit contact-sheet roman-bridge --out-dir target/foundry-kit/roman-contact
cargo run -p shape-cli -- foundry-kit package roman-bridge --out-dir target/foundry-kit/roman-package
cargo run -p shape-cli -- foundry-kit review roman-bridge --quality-report target/hq-benchmark/roman-bridge/quality-report.json --out target/foundry-kit/roman-review.json
cargo run -p shape-cli -- foundry-kit review roman-bridge-hq --quality-report target/hq-benchmark/roman-bridge-hq/quality-report.json --out target/foundry-kit/roman-hq-review.json
```

Kit validation checks versioned metadata, refs, compatibility, required role and
provider-slot coverage, duplicate visible control ownership, the seven-control
default novice limit, quality evidence, and visibility policy. Built-in kit
metadata records automated Wave 32 tiers, but default novice exposure remains
off until manual review approval is recorded. The review command converts an HQ
quality report into review-manifest evidence after checking that the report
matches the kit profile/style.

## Release Candidate Claim

A release-candidate readiness claim is valid only after the archive-first
product boundary is verified. It does not claim installers, signing,
notarization, package-manager publishing, app-store publishing, arbitrary
imported-mesh editability, LLM integration, UVs, materials, rigging, animation,
or marketplace workflows.

Run and preserve evidence for:

```bash
cargo fmt --all --check
cargo test -p shape-app release_gate_all_builtin_profiles_render_real_option_thumbnails -- --ignored
cargo test -p shape-cli release_readiness_verifies_visual_product_gate_when_requested -- --ignored
cargo run -p shape-cli -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate --out target/release-readiness-product-ui.json
cargo test -p shape-render --test foundry_preview
cargo test -p shape-search --test foundry_candidates
cargo test -p shape-cli release_readiness
cargo test -p shape-cli hq_quality
cargo test -p shape-cli foundry_kit
cargo run -p shape-cli -- foundry-kit inspect roman-bridge
cargo run -p shape-cli -- foundry-kit inspect roman-bridge-hq
cargo run -p shape-cli -- foundry-kit review roman-bridge --quality-report target/hq-benchmark/roman-bridge/quality-report.json --out target/foundry-kit/roman-review.json
cargo run -p shape-cli -- foundry-kit review roman-bridge-hq --quality-report target/hq-benchmark/roman-bridge-hq/quality-report.json --out target/foundry-kit/roman-hq-review.json
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge --out-dir target/hq-benchmark/roman-bridge --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```

Manual completion of [`docs/RELEASE_CANDIDATE_MANUAL_GATE.md`](RELEASE_CANDIDATE_MANUAL_GATE.md)
is also required before making the claim.
