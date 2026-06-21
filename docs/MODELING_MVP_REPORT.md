# Asset Modeling Lab MVP Report

## Summary

Wave 6 adds a native application mode named **Asset Modeling Lab**. The desktop app now starts in this mode and keeps the legacy implicit editor available through the mode switcher.

The MVP uses the explicit asset recipe path only. It compiles authored polygon parts through `shape-compile`, renders CPU previews off the UI thread, generates deterministic semantic candidate recipes through `shape-search::asset`, derives mesh visual descriptors through `shape-render`, scores and de-duplicates compiled proposals, and exports both grouped OBJ and the canonical model package.

## Implemented Scope

- Startup template choices:
  - Industrial Crate
  - Explicit Desk Lamp
  - Stylized Stool
- Asset Modeling Lab layout:
  - Top toolbar: New Template, Open, Save, Save As, Export, Undo, Refine, Explore
  - Left panel: named part tree and branch history
  - Center: viewport with current preview and asset overlay
  - Right panel: inspector, locks, validation
  - Bottom: six-slot candidate gallery with generated thumbnails
- Off-thread work:
  - current asset compile
  - current preview render
  - semantic candidate proposal generation
  - bounded parallel candidate compile/scoring/diversity selection through one reused worker pool
  - candidate preview render using selected compiled artifacts when recipe hashes match
  - candidate acceptance promotes the selected compiled artifact into current state before rendering
  - OBJ export
  - canonical package export
- Stale job handling:
  - reducer rejects stale job IDs, stale generation IDs, and stale recipe revisions
  - previous preview remains visible while rebuild work is pending
  - individual candidate preview failures are reported on the affected card instead of aborting the whole batch
  - repaint is requested only while jobs are active or when new results arrive
- Persistence:
  - branch-preserving Asset Modeling Lab project snapshots
  - raw asset recipe load fallback for earlier asset JSON files
- Export:
  - grouped OBJ with stable part object/group names
  - canonical package with manifest, recipe, provenance, compile/model validation, Blender reconstruction script, and part meshbin payloads

## Template Quality

All three templates use explicit authored topology sources and semantic part hierarchy:

- Industrial Crate: rounded body, panels, side handles, fastener rows, feet, ventilation slats, skid/trim parts.
- Explicit Desk Lamp: lathed base/shade, swept stem/support, pivot collars, rim trim, optional switch detail.
- Stylized Stool: rounded seat, four tapered legs, foot pads, support rails, bevel controls, optional seat trim.

Production geometry stays on explicit polygon generators. No SDF production geometry, raw vertex controls, texturing, UVs, rigging, animation, cloud services, LLMs, or imported-mesh reconstruction were added.

## Acceptance Coverage

- Crate body width: `Body width`
- Handle thickness: `Handle thickness`
- Bolt count: `Bolt count`
- Optional trim disable: optional skid rail trim instances
- Body lock: part instance lock filters semantic candidate programs that would modify the body, including definition-level body edits
- Six variants: Refine/Explore over-generate semantic proposals, compile and score the pool, compare fixed-camera mesh silhouettes, visible z-buffer depth histograms, volume, and recipe channels, collapse duplicates, and select six diverse survivors when available
- Accept/undo/branch: reducer revisions are branchable and serialized
- Save/reload: Asset Modeling Lab snapshots preserve revision history
- Export: OBJ and canonical model package jobs write files off the UI thread
- Blender verification: canonical packages include `blender_reconstruct.py` for create/reopen validation

## Known Caveats

- Candidate generation is deterministic, lock-aware, semantic, and score-selected from mesh-derived visual descriptors, but the metrics remain heuristics rather than visual taste or artistic quality.
- Authored relationship policies travel with recipes and can target concrete instances, generated operation occurrences, prototype occurrence families, part tags, and definition role tags. Future Boolean boundary-loop relationships still need richer selectors.
- The viewport overlay exposes selected-part context, validation, and wireframe hinting; direct viewport part picking is not in this MVP.
- Current explicit generators avoid booleans; interlocking geometry is modeled as separate clean parts rather than fused constructive solids.

## Verification

- `cargo fmt --all --check`: passed
- `cargo test --workspace`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo build --workspace --release`: passed
- `cargo run -p shape-cli -- asset-visual-benchmark --out-dir target/asset-visual-benchmark`: writes fixed-camera shaded and wireframe sheets for original templates, Refine outputs, Explore outputs, accepted branches, and final exported packages
- Template export:
  - `industrial-crate`: 31 parts, 4212 triangles
  - `explicit-desk-lamp`: 12 parts, 2776 triangles
  - `stylized-stool`: 13 parts, 2140 triangles
- Blender 4.5 create/reopen verification:
  - `industrial-crate`: `verify_reopen=true`
  - `explicit-desk-lamp`: `verify_reopen=true`
  - `stylized-stool`: `verify_reopen=true`
