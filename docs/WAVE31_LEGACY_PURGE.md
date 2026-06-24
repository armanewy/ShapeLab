# Wave 31 Legacy Product Surface Purge

Wave 31.1 removes legacy product surfaces from the native Shape Lab app. The
desktop product now starts directly in Visual Foundry.

## Removed Product Surfaces

- Asset Modeling Lab wrapper.
- Visual Foundry / Modeling Workspace switcher.
- Legacy Implicit Mode.
- Explicit Modeling Workspace native route.
- From Existing Recipe route from the default New flow.
- Default Advanced Recipe product tab.
- Old blank startup path with no visible asset-family choice.

## Deleted App-Crate Modules

The native app crate now compiles only the Visual Foundry product shell. Removed
files were limited to UI modules for the deleted product surfaces:

- `crates/shape-app/src/desktop.rs`
- `crates/shape-app/src/app.rs`
- `crates/shape-app/src/commands.rs`
- `crates/shape-app/src/jobs.rs`
- `crates/shape-app/src/state.rs`
- `crates/shape-app/src/viewport.rs`
- `crates/shape-app/src/asset/**`
- `crates/shape-app/src/panels/**`

Path-imported tests for those removed UI modules were also deleted:

- `crates/shape-app/tests/asset_state.rs`
- `crates/shape-app/tests/asset_ui.rs`
- `crates/shape-app/tests/inspector_tests.rs`
- `crates/shape-app/tests/job_tests.rs`
- `crates/shape-app/tests/panel_tests.rs`
- `crates/shape-app/tests/state_tests.rs`
- `crates/shape-app/tests/viewport_tests.rs`

## Retained Internal And Research Crates

These remain because they are required by Visual Foundry, release gates,
headless asset generation, exports, or strict reconstruction research:

- `shape-foundry`
- `shape-foundry-catalog`
- `shape-family`
- `shape-family-compile`
- `shape-modeling`
- `shape-render`
- `shape-search`
- `shape-compile`
- `shape-program`
- `shape-program-verify`
- `shape-inverse`
- `shape-character`
- `shape-decompiler`
- `shape-cli`

The explicit asset recipe lane remains a core/headless modeling lane, not a
native product UI route.

## Updated Docs

- `README.md`
- `docs/KNOWN_LIMITATIONS.md`
- `docs/foundry-app-contracts.md`
- `docs/IMPORT_CAPABILITY_MATRIX.md`
- `docs/POST_WAVE20_TRUTH_GATE.md`
- `docs/VISUAL_FOUNDRY_MANUAL_TEST.md`
- `docs/VISUAL_FOUNDRY_MVP_REPORT.md`
- `docs/VISUAL_FOUNDRY_NEXT_STEPS.md`
- `docs/VISUAL_FOUNDRY_USABILITY_FINDINGS.md`
- `docs/MODELING_KNOWN_LIMITATIONS.md`
- `docs/MODELING_MVP_REPORT.md`

## Tests Added Or Updated

`crates/shape-app/src/foundry/app.rs` now tests:

- the product app launches on the Choose home state;
- the home data includes ten built-in profiles;
- default product-visible strings do not include Legacy, Implicit, Asset
  Modeling Lab, Modeling Workspace, Advanced Recipe, or From Existing Recipe;
- the default product steps are novice-facing;
- the Foundry shell still loads the Stylized Furniture Lamp profile and reaches
  the Directions step after startup.

Existing Visual Foundry reducer, panel-helper, pack, history, option-thumbnail,
and release-gate tests remain in place.

## Intentionally Retained Historical References

Historical Wave 6 and Wave 10 reports may still mention Asset Modeling Lab,
Modeling Workspace, or legacy implicit mode when describing what existed at that
time. Current product docs must describe Shape Lab as direct Visual Foundry
startup.
