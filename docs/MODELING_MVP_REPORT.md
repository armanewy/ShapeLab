# Asset Modeling Lab MVP Report

## Summary

Wave 6 adds a native application mode named **Asset Modeling Lab**. The desktop app now starts in this mode and keeps the legacy implicit editor available through the mode switcher.

The MVP uses the explicit asset recipe path only. It compiles authored polygon parts through `shape-compile`, renders CPU previews off the UI thread, generates deterministic candidate recipes, and exports both grouped OBJ and the canonical model package.

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
  - candidate generation
  - candidate preview compile/render
  - OBJ export
  - canonical package export
- Stale job handling:
  - reducer rejects stale job IDs, stale generation IDs, and stale recipe revisions
  - previous preview remains visible while rebuild work is pending
  - repaint is requested only while jobs are active or when new results arrive
- Persistence:
  - branch-preserving Asset Modeling Lab project snapshots
  - raw asset recipe load fallback for earlier asset JSON files
- Export:
  - grouped OBJ with stable part object/group names
  - canonical package with manifest, recipe, provenance, validation, Blender reconstruction script, and part meshbin payloads

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
- Body lock: part instance lock skips body definition parameters during candidate generation
- Six variants: default candidate count is six; generation respects locks and deterministic parameter order
- Accept/undo/branch: reducer revisions are branchable and serialized
- Save/reload: Asset Modeling Lab snapshots preserve revision history
- Export: OBJ and canonical model package jobs write files off the UI thread
- Blender verification: canonical packages include `blender_reconstruct.py` for create/reopen validation

## Known Caveats

- Candidate generation is deterministic and lock-aware, but still parameter-neighborhood based rather than design-intent search.
- The viewport overlay exposes selected-part context, validation, and wireframe hinting; direct viewport part picking is not in this MVP.
- Current explicit generators avoid booleans; interlocking geometry is modeled as separate clean parts rather than fused constructive solids.

## Verification

- `cargo fmt --all --check`: passed
- `cargo test --workspace`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo build --workspace --release`: passed
- Template export:
  - `industrial-crate`: 31 parts, 4212 triangles
  - `explicit-desk-lamp`: 12 parts, 2776 triangles
  - `stylized-stool`: 13 parts, 2140 triangles
- Blender 4.5 create/reopen verification:
  - `industrial-crate`: `verify_reopen=true`
  - `explicit-desk-lamp`: `verify_reopen=true`
  - `stylized-stool`: `verify_reopen=true`
