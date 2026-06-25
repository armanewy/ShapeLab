# Foundry Author Studio Manual Gate

Use this gate before relying on an authored kit package outside internal
development.

## Visibility Gate

- The default Visual Foundry launch path does not show Foundry Author Studio.
- The default product-visible string set does not expose provider packs,
  sockets, ports, scalar paths, semantic IDs, operation IDs, or compiler terms.
- Author Studio appears only when an explicit developer/pro authoring gate is
  enabled.
- Any unavailable authoring task has a plain reason.

## Descriptor Gate

- Kit overview refs match the `FoundryKitPackage` section IDs.
- Family roles have stable IDs, display names, descriptions, required/repeated
  policy, default visibility, and export part names.
- Provider descriptors reference known family roles and provider slots.
- Required sockets include socket ID, port ID, target role, compatibility tags,
  and allowed attachment modes.
- Descriptor-only provider/component rows are labeled as descriptor-only.
- Source-backed preview, contact-sheet, review, HQ benchmark, and package gates
  are enabled only after canonical built-in backing is verified.
- Authored package validation uses an explicit package manifest ref rather than
  a built-in slug.
- Style compatibility lists compatible and incompatible packs without conflicts.
- Style policy fields cover detail density, bevel language, proportions, and
  symmetry/asymmetry.
- Visible controls have human labels, descriptions, disabled-reason policy, and
  non-overlapping slot ownership.
- Topology-changing controls are discrete whole-model choices.
- Candidate strategies reference visible controls, respect locks, and use
  product-facing labels in their explanations.
- Preview cameras define fitted-scale and lighting policy.
- Contact sheets include front, side, back, and three-quarter views.
- Option galleries use a consistent camera and fitted scale within each
  control.

## Quality Evidence Gate

Run and archive evidence where supported:

```bash
cargo run -p shape-cli -- foundry-kit validate <kit>
cargo run -p shape-cli -- foundry-kit inspect <kit>
cargo run -p shape-cli -- foundry-kit preview <kit> --out-dir <dir>/preview
cargo run -p shape-cli -- foundry-kit contact-sheet <kit> --out-dir <dir>/contact-sheet
cargo run -p shape-cli -- hq-quality-benchmark --profile <slug> --out-dir <dir>/hq-quality --verify-export
cargo run -p shape-cli -- foundry-kit review <kit> --quality-report <report> --out <dir>/review-manifest.json
cargo run -p shape-cli -- foundry-kit package <kit> --out-dir <dir>/package
```

Unsupported rows are acceptable only when the reason is truthful and the package
visibility reflects the missing evidence. Do not promote a kit to default
novice catalog exposure without human approval in the review manifest. Do not
claim Showcase without adversarial visual review.

## Release Gate

- `cargo test -p shape-app foundry_author`
- `cargo test -p shape-foundry`
- `cargo test -p shape-cli foundry_kit`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --no-fail-fast`
- `cargo build --release --workspace`

Manual screenshot review is still required for any UI host that renders Author
Studio. The default novice Visual Foundry screenshot must remain free of
technical authoring language.
