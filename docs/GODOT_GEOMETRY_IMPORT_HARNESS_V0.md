# Godot Geometry Import Harness v0

Status: CLI harness implemented.

The Godot import harness proves whether a geometry-only GLB can be imported by
Godot in the current environment. It does not add materials, UV editing,
collision, rigging, animation, runtime LLM integration, public catalog
publishing, or game-ready status.

## Command

```bash
shape-cli godot-proof geometry-import \
  --glb target/object-plan-geometry-export/box/asset.glb \
  --out-dir target/godot-geometry-proof/box
```

The harness accepts either:

- `--godot-bin <path>`
- `GODOT_BIN`

If neither resolves to an executable file, the report is `Blocked`, not
`Passed`.

## Outputs

Always written when the output directory can be created:

- `godot-import-proof-report.json`

Written only when Godot runs:

- `godot-project/`
- `imported-asset-report.json`
- `stdout.log`
- `stderr.log`
- optional version logs

## Report Contract

`godot-import-proof-report.json` includes:

- `status`: `Passed`, `Blocked`, or `Failed`
- `godot_available`
- `godot_version`, when available
- `source_glb`
- `mesh_imported`
- `material_imported: false`
- `collision_imported: false`
- `rig_imported: false`
- `animation_imported: false`
- `game_ready: false`
- `blockers`
- `logs`

V0 proves geometry import only. It does not require or claim material import,
collision import, rig import, or animation import.

## Status Meanings

- `Passed`: Godot ran and imported a mesh resource from the GLB.
- `Blocked`: proof could not be completed in this environment, or Godot ran but
  no imported mesh resource was found.
- `Failed`: the source GLB path is invalid or the Godot import command failed.

If Godot is unavailable, the harness writes a `Blocked` report saying import
proof was not run. Object Orchard must not claim Godot-ready geometry from a blocked
report.

## Product Boundary

Godot import proof is required before Godot-ready claims. Even when this V0
harness passes, the result is still geometry-only and remains not game-ready:

- no texture workflow
- no material looks
- no collision or gameplay metadata
- no rigging
- no animation
- no public catalog publishing
