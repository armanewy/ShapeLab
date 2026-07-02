# Rust File Size Exceptions

Date: 2026-07-01

These are temporary baseline exceptions for the cleanup wave. The target rule is
that no Rust source file should exceed roughly 1000 non-test lines. Exceptions
must have an owner, split/removal plan, and deadline.

| Path | Current non-test line count | Owner | Split plan | Deadline |
| --- | ---: | --- | --- | --- |
| `crates/shape-app/src/foundry/app.rs` | 10681 | Cleanup Prompt 3 | Split Foundry app into app/state/commands/jobs/home/make/stage/inspector/drawers/review/screenshot modules. | Cleanup Wave 1 integration |
| `crates/shape-app/src/foundry/jobs.rs` | 1354 | Cleanup Prompt 3 | Split Foundry job orchestration from preview/candidate/recovery-specific helpers. | Cleanup Wave 1 integration |
| `crates/shape-app/src/foundry/panels/directions.rs` | 1105 | Cleanup Prompt 3 | Split direction panel state, actions, and rendering helpers. | Cleanup Wave 1 integration |
| `crates/shape-app/src/foundry/panels/history.rs` | 1207 | Cleanup Prompt 3 | Split history model, rendering, and command helpers. | Cleanup Wave 1 integration |
| `crates/shape-app/src/foundry/state.rs` | 2320 | Cleanup Prompt 3 | Split Foundry state into focused state, workflow, pack, warning, and readiness modules. | Cleanup Wave 1 integration |
| `crates/shape-asset/src/lib.rs` | 9164 | Cleanup Prompt 4 | Split AssetRecipe semantic contracts into ids, recipe, relationships, patterns, export, review, validation, and migration modules. | Cleanup Wave 1 integration |
| `crates/shape-cli/src/main.rs` | 3733 | Cleanup Prompt 5 | Split CLI command implementations by subsystem and remove obsolete command paths. | Cleanup Wave 1 integration |
| `crates/shape-cli/src/object_plan_cli.rs` | 2378 | Cleanup Prompt 5 | Split ObjectPlan CLI into validation/materialization/render/export/batch modules. | Cleanup Wave 1 integration |
| `crates/shape-compile/src/export/package.rs` | 1045 | Cleanup Prompt 4 | Split export package writing, reading, validation, and DCC sidecar helpers. | Cleanup Wave 1 integration |
| `crates/shape-compile/src/lib.rs` | 1485 | Cleanup Prompt 4 | Split compiler reports, compile pipeline, pattern evaluation, and tests. | Cleanup Wave 1 integration |
| `crates/shape-compile/src/validation/mod.rs` | 2028 | Cleanup Prompt 4 | Split validation metrics, intersections, sockets, overlap, and budget checks. | Cleanup Wave 1 integration |
| `crates/shape-core/src/lib.rs` | 1432 | Cleanup Prompt 6 | Retire or isolate legacy ShapeDocument and low-level helpers. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/lib.rs` | 4725 | Cleanup Prompt 5 | Delete if obsolete or split into package, diagnostics, adapters, and active proof helpers. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/v3/bend_fit.rs` | 1111 | Cleanup Prompt 5 | Delete if obsolete or split bend detection from evidence/reporting. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/v3/blender.rs` | 1400 | Cleanup Prompt 5 | Delete Blender path if obsolete under current boundaries. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/v3/decompile.rs` | 1044 | Cleanup Prompt 5 | Delete if obsolete or split decompile orchestration from helpers. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/v3/diagnostics.rs` | 1145 | Cleanup Prompt 5 | Delete if obsolete or split diagnostics by concern. | Cleanup Wave 1 integration |
| `crates/shape-decompiler/src/v3/package.rs` | 1497 | Cleanup Prompt 5 | Delete if obsolete or split package parsing/writing/reporting. | Cleanup Wave 1 integration |
| `crates/shape-family/src/lib.rs` | 2560 | Cleanup Prompt 5 | Delete obsolete family-generation lane or split retained contracts. | Cleanup Wave 1 integration |
| `crates/shape-family-compile/src/lib.rs` | 3258 | Cleanup Prompt 5 | Delete obsolete family compiler or split retained deterministic evidence helpers. | Cleanup Wave 1 integration |
| `crates/shape-family-compile/src/remap/assembly.rs` | 1210 | Cleanup Prompt 5 | Delete obsolete remap code or split if still required by tests. | Cleanup Wave 1 integration |
| `crates/shape-family-compile/src/remap/ports.rs` | 1133 | Cleanup Prompt 5 | Delete obsolete remap code or split if still required by tests. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/author_studio.rs` | 1200 | Cleanup Prompt 4 | Split authoring studio contracts, validation, and internal UI models. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/compile.rs` | 1406 | Cleanup Prompt 4 | Split compile adapters by primitive/composition/report concern. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/control.rs` | 1719 | Cleanup Prompt 4 | Split control schema, runtime values, validation, and reflection helpers. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/foundation.rs` | 2283 | Cleanup Prompt 4 | Split foundation contracts, descriptors, package helpers, and validation. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/kit.rs` | 1521 | Cleanup Prompt 4 | Split kit contracts, persistence, review, and user-summary helpers. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/object_plan.rs` | 1215 | Cleanup Prompt 4 | Split ObjectPlan contracts, validation, materialization, relationships, and summaries. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/pack.rs` | 1121 | Cleanup Prompt 4 | Split pack contracts, validation, and reporting helpers. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/primitive_property.rs` | 1059 | Cleanup Prompt 4 | Split primitive schemas, descriptor bridge, validation, and defaults. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/validation.rs` | 1393 | Cleanup Prompt 4 | Split validation reports, issue builders, and subsystem-specific validators. | Cleanup Wave 1 integration |
| `crates/shape-foundry/src/variation.rs` | 1079 | Cleanup Prompt 5 | Delete or isolate obsolete variation lane after candidate removal. | Cleanup Wave 1 integration |
| `crates/shape-foundry-catalog/src/box_primitive.rs` | 1051 | Cleanup Prompt 4 | Split catalog fixture, controls, presets, and tests/helpers. | Cleanup Wave 1 integration |
| `crates/shape-foundry-catalog/src/lib.rs` | 1095 | Cleanup Prompt 4 | Split catalog registry and profile grouping helpers. | Cleanup Wave 1 integration |
| `crates/shape-modeling/src/assembly.rs` | 1320 | Cleanup Prompt 4 | Split assembly evaluation, attachment, patterns, and validation. | Cleanup Wave 1 integration |
| `crates/shape-modeling/src/features/mod.rs` | 1926 | Cleanup Prompt 4 | Split feature contracts/generators by feature family. | Cleanup Wave 1 integration |
| `crates/shape-modeling/src/generators/basic.rs` | 4681 | Cleanup Prompt 4 | Split primitive generators by rounded box, plate, cylinder, and cut helpers. | Cleanup Wave 1 integration |
| `crates/shape-modeling/src/generators/profile.rs` | 1275 | Cleanup Prompt 4 | Split sweep/lathe/profile validation and generation helpers. | Cleanup Wave 1 integration |
| `crates/shape-poly/src/lib.rs` | 1888 | Cleanup Prompt 4 | Split polygon mesh types, validation, adjacency, triangulation, and tests. | Cleanup Wave 1 integration |
| `crates/shape-program/src/evaluator.rs` | 1083 | Cleanup Prompt 5 | Delete obsolete program lane if unused or split evaluator/runtime concerns. | Cleanup Wave 1 integration |
| `crates/shape-program/src/runtime.rs` | 1157 | Cleanup Prompt 5 | Delete obsolete program lane if unused or split runtime helpers. | Cleanup Wave 1 integration |
| `crates/shape-project/src/asset.rs` | 1017 | Cleanup Prompt 5 | Split project asset persistence and status/report helpers. | Cleanup Wave 1 integration |
| `crates/shape-project/src/foundry.rs` | 1465 | Cleanup Prompt 5 | Split Foundry project persistence by pack, recent projects, and validation. | Cleanup Wave 1 integration |
| `crates/shape-render/src/foundry/mod.rs` | 1497 | Cleanup Prompt 5 | Split Foundry render presets, preview rasterization, and evidence helpers. | Cleanup Wave 1 integration |
| `crates/shape-render/src/lib.rs` | 1539 | Cleanup Prompt 5 | Split renderer core, camera, materials, and image helpers. | Cleanup Wave 1 integration |
| `crates/shape-search/src/asset/mod.rs` | 2818 | Cleanup Prompt 2 | Remove or isolate legacy candidate/search scoring path. | Cleanup Wave 1 integration |
| `crates/shape-search/src/asset/scoring.rs` | 1087 | Cleanup Prompt 2 | Remove or isolate legacy candidate/search scoring path. | Cleanup Wave 1 integration |
| `crates/shape-search/src/foundry/mod.rs` | 4323 | Cleanup Prompt 2 | Remove or isolate legacy candidate/search Foundry path. | Cleanup Wave 1 integration |
| `crates/shape-search/src/lib.rs` | 1935 | Cleanup Prompt 2 | Remove or isolate legacy candidate/search crate. | Cleanup Wave 1 integration |
