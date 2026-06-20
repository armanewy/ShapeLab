# Wave 2 Report

## Contract Changes

- Added `docs/app-contracts.md`.
- Added app-facing placeholder contracts: `AppCommand`, `AppEffect`, `AppState`, `JobRequest`, `JobEvent`, `CandidatePreview`, and `ViewportAction`.
- `AppEffect::StartJob` stores `Box<JobRequest>` to avoid a large enum variant.
- `shape-cli` now depends directly on `image`, `serde`, and `serde_json`, with `tempfile` as a dev-dependency.

## Generated Artifacts

Wave 2 demo commands produced:

- `target/demo-lamp`
- `target/demo-submarine`
- `target/demo-plant`

Each directory contains project JSON, OBJ meshes, PNG previews, a contact sheet, and `summary.json`.

The contact sheets were visually inspected and contained visible, framed parent/candidate images.

## Observed Performance

On the local Windows machine, default demo runs completed in roughly 8-10 seconds per preset after the binary was built.

`summary.json` intentionally contains a deterministic `timings_ms` placeholder rather than wall-clock timings so repeated commands can produce stable JSON. Measured performance belongs in reports or logs, not deterministic artifacts.

## Known Limitations

- The CLI vertical slice is visible through generated PNGs and OBJ files, not through the desktop UI yet.
- Candidate generation can still make broad Explore changes that separate implicit components.
- `summary.json` currently estimates rejections as `proposal_count - generated_candidates`; detailed rejection diagnostics are deferred to later search-quality work.
- Contact-sheet labels use a tiny built-in bitmap label renderer rather than a font system.
- The desktop app still shows the bootstrap shell, with Wave 3 contracts prepared.
