# Shape App Foundry Module Split

Date: 2026-07-01

## Old Large File

`crates/shape-app/src/foundry/app.rs` previously held the Foundry desktop shell, all app-level UI helpers, job polling, project I/O, preview rendering, home thumbnails, material-look UI, pack/export UI, screenshot fixtures, and inline tests in one file.

Baseline size before this split:

| Path | Non-test lines | Test lines |
| --- | ---: | ---: |
| `crates/shape-app/src/foundry/app.rs` | 10681 | 4352 |

## Module Map

`crate::foundry::app::FoundryDesktopApp` remains the external app type. The root `app.rs` now keeps shared app state, enums, constants, and public helper re-exports. Cohesive implementation moved under `crates/shape-app/src/foundry/app/`:

| Module | Responsibility |
| --- | --- |
| `catalog.rs` | Built-in catalog resolver used by Foundry jobs. |
| `customize_ui.rs` | Customize panel controls, direct numeric control helpers, and control display filtering. |
| `direction_ideas.rs` | Direction candidate comparison, grids, cards, and candidate display copy helpers. |
| `effects.rs` | Command/effect application and background job polling. |
| `feature_gates.rs` | Environment flag readers for preview/review kit gates. |
| `family_studio_lite.rs` | Reusable kit drawer state, draft, test, and save UI. |
| `home.rs` | Home starting-point profiles, grouping, search, selection, and home rendering. |
| `home_thumbnails.rs` | Home thumbnail queue, turntable frames, and thumbnail rendering helpers. |
| `history_panel.rs` | Project history panel rendering and dispatch. |
| `job_coordinator.rs` | Background job channel coordinator and preview-cache routing. |
| `make_actions.rs` | Make candidate commands, recovery cards, and direction board orchestration. |
| `make_copy.rs` | Make canvas banner, status, direct-property, and product-safe copy helpers. |
| `make_layout.rs` | Make canvas layout and action enablement helpers. |
| `make_state.rs` | Make canvas view-state derivation, timers, status suppression, and profile detection. |
| `make_view.rs` | Main Make screen, stage, inspector, focus tray, and exact-value panel rendering. |
| `material_looks.rs` | Material-look evidence loading, validation, tray state, and rendering. |
| `object_plan_review.rs` | ObjectPlan review drawer and visibility state. |
| `pack_export.rs` | Pack/export panels, drawers, contact sheet, readiness, and pack member helpers. |
| `preview.rs` | Preview texture cache, current preview stage, orbit state, and preview drawing. |
| `product_contracts.rs` | Existing product-gate helper functions re-exported from `crate::foundry::app`. |
| `product_ui.rs` | Shared app UI widgets, cards, tabs, view controls, and empty states. |
| `project_io.rs` | Project file dialogs, status sanitization, save/load fixture/project helpers. |
| `screenshot.rs` | Screenshot scenario parsing, assertions, and scenario-driving helpers. |
| `shell.rs` | `FoundryDesktopApp` default construction, app shell, app bar, tabs, status strip, and `eframe::App` bridge. |
| `tests/mod.rs` | Former inline app tests, moved under a test-only path. |

## Remaining Debt

All new `foundry/app/*.rs` non-test modules are under 1000 physical lines, and `app.rs` is below the 1000 non-test-line target. Existing Foundry exceptions outside this app split remain documented in `docs/RUST_FILE_SIZE_EXCEPTIONS.md` with the `Cleanup Wave 1 integration` deadline:

- `crates/shape-app/src/foundry/jobs.rs`
- `crates/shape-app/src/foundry/panels/directions.rs`
- `crates/shape-app/src/foundry/panels/history.rs`
- `crates/shape-app/src/foundry/state.rs`

## Behavior

This branch is a mechanical organization change. It preserves the previous product behavior, schemas, export behavior, and `crate::foundry::app::FoundryDesktopApp` access path.
