# Release Readiness

Wave 30 adds an explicit release-readiness contract for the native Shape Lab
loop. The intent is to make performance and packaging status auditable without
claiming unimplemented GPU, installer, signing, or app-store work.

## Machine-Readable Report

Run the report from the repository root:

```bash
cargo run -p shape-cli -- release-readiness --out target/release-readiness.json
cargo run -p shape-cli -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
```

The JSON report records:

- the Visual Foundry product gate. Without `--verify-visual-gate`, this records
  the required command and marks CLI evidence as not run. With
  `--verify-visual-gate`, it compiles the built-in profiles and records computed
  CLI evidence for ten profiles, seven primary controls per profile, and
  rendered whole-model option thumbnails. The native default-path gate remains
  the explicit ignored app-state release test listed in the report;
- the deterministic CPU preview path and bounded preview-cache capacity;
- duplicate preview-cache miss coalescing for repeated keys in one batch;
- Foundry candidate generation proposal bounds and returned-candidate caps;
- project-file recovery/autosave support versus automatic timed autosave UI;
- manual packaging status, with installer, signing, and publishing marked as
  not configured;
- the current split between headless panel/reducer tests and deferred
  desktop-window pixel regression tests.

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

The release gate also requires all ten built-in profiles to open through the
default novice surface with seven primary controls and zero technical surface
exposure. Deterministic contact sheets and sample export artifacts are produced
by `shape-cli foundry-visual-benchmark` for demo review.

The all-profile thumbnail test is marked `#[ignore]` so it does not slow every
workspace test run. It must be run explicitly before a release-readiness claim.
The CLI report can compute catalog/render evidence with
`shape-cli release-readiness --verify-visual-gate`, but it does not replace the
native app-state gate. A release-readiness claim requires both the CLI verified
report and the ignored app-state test command.

## Release Boundary

The release build remains archive-first. `packaging/README.md` describes manual
Windows, macOS, and Linux package contents. The repository does not contain
private signing certificates, installer framework configuration, notarization
automation, package-manager publishing, or app-store submission logic.

GPU viewport/rendering work remains a future optional backend. Release checks do
not require GPU-specific code beyond the native graphics support already needed
by the desktop framework.

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
cargo test -p shape-render --test foundry_preview
cargo test -p shape-search --test foundry_candidates
cargo test -p shape-cli release_readiness
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```

Manual completion of [`docs/RELEASE_CANDIDATE_MANUAL_GATE.md`](RELEASE_CANDIDATE_MANUAL_GATE.md)
is also required before making the claim.
