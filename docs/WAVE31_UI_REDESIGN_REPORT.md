# Wave 31 UI Redesign Report

Wave 31 replaced the developer-era desktop shell with a direct Visual Foundry
product app and added an automated product UI gate.

## Status

- Wave 31.1 removed the legacy product shell.
- Wave 31.2 added the Visual Foundry design system.
- Wave 31.3 added the product home and main shell.
- Wave 31.4 redesigned Directions, Customize, Pack, Export, and History into a
  coherent workflow.
- Wave 31.5 added `shape-cli release-readiness --verify-product-ui-gate` and
  the manual screenshot gate in `docs/FOUNDRY_UI_MANUAL_GATE.md`.

## Product Questions

Does the app launch directly into Visual Foundry?

- Yes. The native app root constructs the Visual Foundry product app directly.
  The product UI gate reports `app_shell: direct_visual_foundry`.

Are all legacy product surfaces gone?

- Yes for the default product app. The automated product-visible copy audit
  rejects Legacy, Implicit, Asset Modeling Lab, Modeling Workspace, Advanced
  Recipe, scalar path, provider ID, role binding, fragment/remap, operation ID,
  semantic ID, conformance binding, SDF, compiler, and decompiler terms.

Is the launch screen useful?

- Automated evidence says startup is not blank and the home screen exposes ten
  built-in profiles. Human screenshot inspection is still required before an RC
  claim.

Is the main preview visually dominant?

- The workflow shell and panel code make the current whole-model preview or
  whole-model cards the central content. Human screenshot inspection remains
  required for actual viewport dominance.

Can a novice identify the next action?

- The default flow exposes Start, Generate Directions, Direction focus actions,
  Customize, Add Current Asset, Export Pack, and Export readiness states in
  product copy. The five-second human check is part of
  `FOUNDRY_UI_MANUAL_GATE.md`.

Are directions visually understandable?

- The Directions workflow reserves six whole-model candidate slots and exposes
  Refine, Explore, Silhouette, Structure, and Detail. The visual thumbnail
  release gate remains separate from the product UI gate.

Are customize controls clear?

- The product UI gate compiles Roman Timber Bridge, Sci-Fi Industrial Crate,
  and Stylized Furniture Lamp and verifies each exposes one to seven primary
  controls. Product copy guards prevent raw implementation labels in the
  default surface.

Are disabled reasons visible?

- Yes in the automated copy gate: required disabled reasons for project, build,
  pack, and unavailable options are represented in product-visible copy.

Can the three core manual tasks complete without Advanced Recipe?

- Automated compile/start evidence passes for Roman Timber Bridge, Sci-Fi
  Industrial Crate, and Stylized Furniture Lamp with no default Advanced Recipe
  exposure. Full task completion remains a manual RC gate item.

What still feels ugly or confusing?

- Not assessed by automation. Record visual/layout/friction findings in
  `docs/FOUNDRY_UI_MANUAL_GATE.md` and
  `docs/RELEASE_CANDIDATE_MANUAL_GATE.md`.

## Verification

Run for Wave 31.5 implementation:

```bash
cargo fmt --all --check
cargo test -p shape-app product_ui_gate_passes_for_default_visual_foundry_shell
cargo test -p shape-cli release_readiness -- --nocapture
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate --out target/release-readiness-product-ui.json
```

Final release-candidate verification still requires:

```bash
cargo test -p shape-app release_gate_all_builtin_profiles_render_real_option_thumbnails -- --ignored
cargo test -p shape-cli release_readiness_verifies_visual_product_gate_when_requested -- --ignored
cargo run -p shape-cli -- release-readiness --verify-visual-gate --out target/release-readiness-verified.json
cargo run -p shape-cli -- release-readiness --verify-product-ui-gate --out target/release-readiness-product-ui.json
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```
