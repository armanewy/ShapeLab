# Geometry Export v0 Integration Report

Date: 2026-07-01

## Verdict

`GEOMETRY_EXPORT_V0_PASSED_GODOT_PROOF_RERUN_PASSED`

Supported ObjectPlans can export geometry-only GLB packages. Export reports
truthfully exclude UVs, textures, material looks, collision, rigging,
animation, and game-ready status. Godot import proof is available as a harness,
and the post-cleanup local proof rerun passed mesh import with Godot 4.7. The
original July 1 integration run was blocked because no Godot binary was
available in that environment.

## Integrated Work

- `302a704` - Geometry export truth gate
- `c82e9ee` - Geometry-only export contracts
- `2fa4069` - ObjectPlan geometry export CLI
- `477fda1` - Godot geometry import harness

## CLI Evidence

Evidence path: `target/geometry-export-v0`

| Plan | GLB | Export report | Result |
| --- | --- | --- | --- |
| Box Primitive | `box/asset.glb` | `box/geometry-export-report.json` | Passed |
| Flat Panel Primitive | `panel/asset.glb` | `panel/geometry-export-report.json` | Passed |
| Panel with Knob | `panel-knob/asset.glb` | `panel-knob/geometry-export-report.json` | Passed |

Observed export report values:

| Plan | Primitive count | Mesh count | Triangle count | `game_ready` |
| --- | ---: | ---: | ---: | --- |
| Box Primitive | 1 | 1 | 12 | `false` |
| Flat Panel Primitive | 1 | 1 | 12 | `false` |
| Panel with Knob | 2 | 1 | 1036 | `false` |

All export reports keep:

- `includes_uvs: false`
- `includes_textures: false`
- `includes_material_looks: false`
- `includes_collision: false`
- `includes_rig: false`
- `includes_animation: false`
- `game_ready: false`
- `human_review_required: true`

## Godot Proof

Original Godot proof path: `target/geometry-export-v0`

| Asset | Proof report | Result | Reason |
| --- | --- | --- | --- |
| Box Primitive | `godot-box/godot-import-proof-report.json` | Blocked | Godot binary was not found; import proof was not run. |
| Flat Panel Primitive | `godot-panel/godot-import-proof-report.json` | Blocked | Godot binary was not found; import proof was not run. |
| Panel with Knob | `godot-panel-knob/godot-import-proof-report.json` | Blocked | Godot binary was not found; import proof was not run. |

The blocked Godot proof reports keep:

- `godot_available: false`
- `mesh_imported: false`
- `material_imported: false`
- `collision_imported: false`
- `rig_imported: false`
- `animation_imported: false`
- `game_ready: false`

Post-cleanup rerun path: `target/post-cleanup-foundation-hard-gate`

| Asset | Proof report | Result | Godot |
| --- | --- | --- | --- |
| Box Primitive | `godot-box/godot-import-proof-report.json` | Passed | `4.7.stable.official.5b4e0cb0f` |
| Flat Panel Primitive | `godot-flat-panel/godot-import-proof-report.json` | Passed | `4.7.stable.official.5b4e0cb0f` |
| Sphere Primitive | `godot-sphere/godot-import-proof-report.json` | Passed | `4.7.stable.official.5b4e0cb0f` |
| Panel with Knob | `godot-panel-knob/godot-import-proof-report.json` | Passed | `4.7.stable.official.5b4e0cb0f` |

The passed post-cleanup proof reports keep:

- `mesh_imported: true`
- `material_imported: false`
- `collision_imported: false`
- `rig_imported: false`
- `animation_imported: false`
- `game_ready: false`

## Proof Questions

| Question | Result |
| --- | --- |
| Can supported ObjectPlans export geometry-only GLB? | Pass. Box, Flat Panel, and Panel with Knob emitted non-empty `asset.glb` files. |
| Do export reports truthfully say no textures/material looks/collision/rig/animation? | Pass. Reports keep all unsupported feature flags false. |
| Does export report keep `game_ready: false`? | Pass. All export reports keep `game_ready: false`. |
| Did Godot import proof pass, block, or fail? | Passed in the post-cleanup rerun. The original July 1 run was blocked because Godot was unavailable locally. |
| If blocked, why? | Not blocked in the post-cleanup rerun. |
| Are any docs overclaiming Godot-ready or game-ready? | Pass. Docs keep Godot-ready claims blocked until a passed import proof and keep game-ready claims blocked. |

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p orchard-foundry geometry_export --jobs 1` | Pass |
| `cargo test -p orchard-cli object_plan --jobs 1` | Pass |
| `cargo test -p orchard-cli godot --jobs 1` | Pass |
| `cargo test -p orchard-app foundry --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Current Allowed Status

- ObjectPlan can export geometry-only GLB for supported plans.
- Geometry-only GLB is not game-ready.
- Godot import proof passed in the post-cleanup rerun for Box, Flat Panel,
  Sphere, and Panel with Knob geometry-only GLBs. This is still not a
  game-ready package claim.
- No materials/surface workflow, UV/texturing, collision, rigging, or animation
  is included.
- No runtime LLM integration exists.
- No public catalog publishing exists.

## Next Allowed Work

- Primitive Surface V0 contracts
- Personal Kit persistence UI
- Prototype Pack brief contracts
- future material/surface proof must run separately if Phase E begins

## Still Blocked

- Material editor UI
- UV editing UI
- Collision/gameplay metadata
- Rigging/animation UI
- Godot-ready claims until a passed import proof
- Game-ready claims
- Automatic catalog publishing
