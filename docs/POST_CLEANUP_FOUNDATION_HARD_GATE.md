# Post-Cleanup Foundation Hard Gate

Date: 2026-07-02

Branch: `codex/post-cleanup-foundation-hard-gate`

## Verdict

`POST_CLEANUP_FOUNDATION_HARD_GATE_PASSED`

This gate keeps Object Orchard out of Phase E until the cleanup result, direct
editing lane, relationship lane, ObjectPlan lane, export lane, and Godot import
proof are all checked together.

## Gate Questions

- Is the repo now Object Orchard everywhere product-facing?
  Pass. Product-facing docs, commands, crate names, generated evidence paths, and
  metadata use Object Orchard naming. A later metadata finalization branch
  completed the GitHub repository host rename to `armanewy/object-orchard`.
- Are old ShapeLab / Shape Lab names gone except migration notes?
  Pass. Remaining old-name references are historical cleanup records, migration
  notes, or pre-rename remote URL records.
- Are `shape-core` and legacy systems clearly isolated?
  Pass. The retained crate is `orchard-core-legacy`; docs keep `ShapeDocument`
  in the legacy/implicit compatibility lane and out of future A-J product
  semantics.
- Are no Rust source files over the agreed non-test line limit?
  Pass for the current agreed script gate. `scripts/check_rust_file_size.py`
  reports no unapproved over-limit files. Temporary baseline exceptions remain
  documented with owner, plan, and deadline in
  `docs/RUST_FILE_SIZE_EXCEPTIONS.md`.
- Is dead candidate/search/product UI removed or fully internal?
  Pass. Default Direct Make disables generated candidate UI, product tests cover
  no candidate tray/comparison, and remaining legacy candidate/search paths are
  internal or gated.
- Do all active Direct Make properties route through AuthoringOp?
  Pass. Box, Flat Panel, Sphere, and Panel with Knob scalar controls now emit
  replayable `AuthoringOp::SetProperty` breadcrumbs.
- Can AuthoringOp replay reproduce active primitive edits deterministically?
  Pass. `cargo test -p orchard-app authoring_bridge --jobs 1` covers every
  active Direct Make scalar control and replays each emitted log against the
  compiled recipe.
- Does Panel with Knob still materialize through RelationshipContract?
  Pass. Existing relationship tests and ObjectPlan export reports keep Panel
  with Knob as a surface-mounted relationship-backed composition.
- Are fixed-distance and proportional attachment policies still tested?
  Pass. Relationship policy tests cover fixed edge distance and proportional
  placement behavior.
- Do export reports still include relationship realization and truthful
  includes/excludes?
  Pass. Panel with Knob export reports relationship realization. Geometry export
  reports keep textures, material looks, collision, rigging, animation, and
  game-ready status false.
- Does ObjectPlan materialization/export still work after cleanup?
  Pass. Box, Flat Panel, Sphere, and Panel with Knob ObjectPlans exported
  geometry-only GLB under `target/post-cleanup-foundation-hard-gate/`.
- Does the app still open Box, Flat Panel, Sphere, and Panel with Knob without
  generated-candidate UI?
  Pass by automated app foundry tests. Human visual review is still useful
  before UI-heavy handle work, but it is not required to start the non-UI
  foundation fixes from this gate.

## AuthoringOp Coverage

Covered active controls:

- Box Primitive: Width, Depth, Height, Edge Softness.
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness.
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten.
- Panel with Knob: Panel Width, Panel Height, Panel Thickness, Panel Edge
  Softness, Knob Width, Knob Height, Knob Depth, Knob Front Flatten, Knob Back
  Flatten, Knob Horizontal Position, Knob Vertical Position.

Some controls compile to one scalar path; controls that touch multiple generated
scalar paths emit one `AuthoringOp::SetProperty` entry for each changed scalar.
Replay checks every changed scalar path.

## Geometry Export Evidence

Evidence root:

```text
target/post-cleanup-foundation-hard-gate
```

| Plan | Export report | Result | Mesh count | Triangles | `game_ready` |
| --- | --- | --- | ---: | ---: | --- |
| Box Primitive | `box/geometry-export-report.json` | Passed | 1 | 12 | `false` |
| Flat Panel Primitive | `flat-panel/geometry-export-report.json` | Passed | 1 | 12 | `false` |
| Sphere Primitive | `sphere/geometry-export-report.json` | Passed | 1 | 1024 | `false` |
| Panel with Knob | `panel-knob/geometry-export-report.json` | Passed | 1 | 1036 | `false` |

All export reports keep:

- `includes_textures: false`
- `includes_material_looks: false`
- `includes_collision: false`
- `includes_rig: false`
- `includes_animation: false`
- `game_ready: false`

## Godot Proof

Godot binary:

```text
/Applications/Godot.app/Contents/MacOS/Godot
```

Godot version:

```text
4.7.stable.official.5b4e0cb0f
```

| Asset | Proof report | Result | Mesh imported | `game_ready` |
| --- | --- | --- | --- | --- |
| Box Primitive | `godot-box/godot-import-proof-report.json` | Passed | `true` | `false` |
| Flat Panel Primitive | `godot-flat-panel/godot-import-proof-report.json` | Passed | `true` | `false` |
| Sphere Primitive | `godot-sphere/godot-import-proof-report.json` | Passed | `true` | `false` |
| Panel with Knob | `godot-panel-knob/godot-import-proof-report.json` | Passed | `true` | `false` |

The passed Godot proof reports still keep material, collision, rig, and
animation import fields false.

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p orchard-app authoring_bridge --jobs 1` | Pass |
| ObjectPlan geometry export evidence | Pass |
| Godot geometry import proof with `/Applications/Godot.app/Contents/MacOS/Godot` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `python3 scripts/check_rust_file_size.py` | Pass |
| `python3 scripts/audit_cleanup_inventory.py` | Pass |
| `cargo test -p orchard-app foundry --jobs 1` | Pass |
| `cargo test -p orchard-cli object_plan --jobs 1` | Pass |
| `cargo test -p orchard-cli godot --jobs 1` | Pass |
| `cargo test -p orchard-compile export_realization --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Decision

This gate passed. Next allowed foundation work is Orchard stretch handles as
`AuthoringOp` emitters or relationship/attachment UI hardening. Phase E
implementation, Surface/material UI, UV editing, rigging, animation, runtime
LLM, public catalog publishing, and game-ready claims remain blocked.
