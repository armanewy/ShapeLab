# Release Readiness

Wave 30 adds an explicit release-readiness contract for the native Shape Lab
loop. The intent is to make performance and packaging status auditable without
claiming unimplemented GPU, installer, signing, or app-store work.

## Machine-Readable Report

Run the report from the repository root:

```bash
cargo run -p shape-cli -- release-readiness --out target/release-readiness.json
```

The JSON report records:

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

## Release Boundary

The release build remains archive-first. `packaging/README.md` describes manual
Windows, macOS, and Linux package contents. The repository does not contain
private signing certificates, installer framework configuration, notarization
automation, package-manager publishing, or app-store submission logic.

GPU viewport/rendering work remains a future optional backend. Release checks do
not require GPU-specific code beyond the native graphics support already needed
by the desktop framework.

## Verification

Wave 30 release verification should include:

```bash
cargo fmt --all --check
cargo test -p shape-render --test foundry_preview
cargo test -p shape-search --test foundry_candidates
cargo test -p shape-cli release_readiness
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```
