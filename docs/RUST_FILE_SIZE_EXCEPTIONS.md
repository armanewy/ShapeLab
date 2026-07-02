# Rust File Size Exceptions

Date: 2026-07-01

These are temporary baseline exceptions after Cleanup Wave 1 integration. The
target rule is that no Rust source file should exceed roughly 1000 non-test
lines. Exceptions must have an owner, split/removal plan, and deadline.

Cleanup Wave 1 removed the obsolete `shape-decompiler`, `shape-program`, and
`shape-program-verify` crates, split the largest semantic modules, and reduced
`crates/shape-app/src/foundry/app.rs` below the exception threshold. The
remaining exceptions are real and must be handled by targeted follow-up splits
or retirements.

| Path | Current non-test line count | Owner | Split plan | Deadline |
| --- | ---: | --- | --- | --- |
| `crates/shape-app/src/foundry/jobs.rs` | 1353 | App cleanup | Split job orchestration from preview, candidate, pack export, and trace helpers. | Cleanup Wave 2 |
| `crates/shape-app/src/foundry/panels/directions.rs` | 1105 | App cleanup | Split direction board model, validation, intent mapping, and rendering helpers. | Cleanup Wave 2 |
| `crates/shape-app/src/foundry/panels/history.rs` | 1207 | App cleanup | Split history model, status copy, rendering, and command helpers. | Cleanup Wave 2 |
| `crates/shape-app/src/foundry/state.rs` | 2319 | App cleanup | Split Foundry state into workflow, build readiness, pack, project, warning, and persistence modules. | Cleanup Wave 2 |
| `crates/shape-cli/src/main.rs` | 3293 | CLI cleanup | Split command registration and dispatch by subsystem; keep implementation details in subsystem modules. | Cleanup Wave 2 |
| `crates/shape-cli/src/object_plan_cli.rs` | 2378 | CLI cleanup | Split ObjectPlan validation, materialization, render evidence, geometry export, and batch review commands. | Cleanup Wave 2 |
| `crates/shape-compile/src/export/package.rs` | 1045 | Compile cleanup | Split export package writing, reading, validation, and DCC sidecar helpers. | Cleanup Wave 2 |
| `crates/shape-compile/src/lib.rs` | 1485 | Compile cleanup | Split compiler reports, compile pipeline, pattern evaluation, and tests. | Cleanup Wave 2 |
| `crates/shape-compile/src/validation/mod.rs` | 2028 | Compile cleanup | Split validation metrics, intersections, sockets, overlap, and budget checks. | Cleanup Wave 2 |
| `crates/shape-core/src/lib.rs` | 1432 | Legacy boundary cleanup | Continue isolating low-level helpers and legacy `ShapeDocument` compatibility. | Cleanup Wave 2 |
| `crates/shape-family/src/lib.rs` | 2560 | Family cleanup | Split retained family contracts or retire obsolete family-generation surfaces. | Cleanup Wave 2 |
| `crates/shape-family-compile/src/lib.rs` | 3258 | Family compile cleanup | Split retained deterministic compile helpers and conformance bridges. | Cleanup Wave 2 |
| `crates/shape-family-compile/src/remap/assembly.rs` | 1210 | Family compile cleanup | Split remap assembly helpers by transform, role matching, and report output. | Cleanup Wave 2 |
| `crates/shape-family-compile/src/remap/ports.rs` | 1133 | Family compile cleanup | Split remap port contracts, validation, and adapter helpers. | Cleanup Wave 2 |
| `crates/shape-foundry-catalog/src/box_primitive.rs` | 1051 | Catalog cleanup | Split catalog fixture, controls, presets, and tests/helpers. | Cleanup Wave 2 |
| `crates/shape-foundry-catalog/src/lib.rs` | 1095 | Catalog cleanup | Split catalog registry, curated profile grouping, and helper APIs. | Cleanup Wave 2 |
| `crates/shape-poly/src/lib.rs` | 1888 | Polygon cleanup | Split polygon mesh types, validation, adjacency, triangulation, and tests. | Cleanup Wave 2 |
| `crates/shape-project/src/asset.rs` | 1017 | Project cleanup | Split asset project persistence, export helpers, and status/report helpers. | Cleanup Wave 2 |
| `crates/shape-project/src/foundry.rs` | 1465 | Project cleanup | Split Foundry project persistence by pack, recent projects, embedded catalog, and validation. | Cleanup Wave 2 |
| `crates/shape-render/src/foundry/mod.rs` | 1496 | Render cleanup | Split Foundry render presets, preview rasterization, cache, and evidence helpers. | Cleanup Wave 2 |
| `crates/shape-render/src/lib.rs` | 1540 | Render cleanup | Split renderer core, camera, materials, surface preview, and image helpers. | Cleanup Wave 2 |
| `crates/shape-search/src/asset/mod.rs` | 2818 | Search cleanup | Split semantic asset search from legacy candidate/search scoring, or retire unused surfaces. | Cleanup Wave 2 |
| `crates/shape-search/src/asset/scoring.rs` | 1087 | Search cleanup | Split scoring policy, duplicate collapse, and representative selection. | Cleanup Wave 2 |
| `crates/shape-search/src/foundry/mod.rs` | 4323 | Search cleanup | Split Foundry search adapters, candidate copy, render requests, and legacy-gated helpers. | Cleanup Wave 2 |
| `crates/shape-search/src/lib.rs` | 1935 | Search cleanup | Split crate exports, generic search helpers, and tests. | Cleanup Wave 2 |
