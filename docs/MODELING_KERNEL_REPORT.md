# Modeling Kernel Report

## Scope

Wave 4 integrates the explicit polygon modeling lane for the two benchmark assets and keeps the implicit editor plus schema-2/schema-3 decompiler paths separate.

## Benchmarks

`industrial-crate` now contains a rounded primary body, semantic recessed panels, corner reinforcement trim, mirrored swept handles, linear fastener arrays, feet, skid rails, and a top ventilation group with rectangular through-cuts. The compiled asset validates with complete face provenance and semantic region coverage, no SDF/remeshing, and no accidental intersections.

`explicit-desk-lamp` now contains a lathed weighted base, swept angled stem, explicit pivot joints, collar trim, support bracket, lathed shade, rim trim, sockets, and an optional switch detail group. Intentional socket/collar/trim contact is declared in validation metadata so accidental intersection metrics remain zero.

## Construction Timeline

`shape-compile` exposes `build_construction_timeline_report(recipe, artifact)`. The report is deterministic and generated from recipe definitions, instances, operation specs, and generated occurrence provenance. The eight stages are primary body, supporting parts, panels, semantic cuts, trim, repeated details, edge treatment, and final assembly.

`shape-cli inspect-asset <recipe>` prints the timeline. `shape-cli compile-asset <recipe> --out-dir <dir>` writes `construction-timeline.json`.

## CLI

New commands:

- `shape-cli inspect-asset <recipe>`
- `shape-cli compile-asset <recipe> --out-dir <dir>`

`<recipe>` accepts a built-in benchmark slug or a recipe JSON path. `compile-asset` writes the canonical model package, grouped OBJ, preview PNG, statistics, model validation, package verification, and construction timeline sidecars.

## Validation

The model validator now uses signed volume for closed-part winding checks instead of per-face center tests. This preserves inside-out detection while allowing concave swept mechanical handles and brackets.

Closed sweep face winding was corrected so swept tubes produce outward orientation.

## Quality Gate Summary

- Explicit benchmark assets compile through the polygon path.
- `used_sdf_or_remeshing` remains false.
- Face provenance coverage is complete.
- Semantic region coverage is complete.
- Built-in validation budgets are 30k triangles for the crate and 25k triangles for the lamp.
- Canonical package export and Blender reconstruction are used for benchmark verification.
