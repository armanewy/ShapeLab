# Stylized Lamp Template Hardening Report

## Scope

Prompt 5 reauthored only the Stylized Furniture Lamp catalog fixture, its focused tests, and this documentation. No app UI, gamekit, rig/motion, runtime LLM, root manifest, or lockfile changes were made.

## Geometry Hardening

- Overall Height now drives a taller swept stem and upper pivot position, making compact versus tall lamps read in clay silhouette.
- Base Weight widens the lathed base and slab-foot footprint while keeping the slab/body validation-clean.
- Stem Curvature increases the sweep endpoint and upper pivot offset so straight versus playful curved stems are visible at card size.
- Joint Size has a clear pivot-disc depth delta while retaining connected stem/joint attachment.
- Shade Style now includes `cone`, `drum`, `task`, `wide`, `minimal`, and `playful`.
- Shade Scale now acts on a child shade body under an attached mount collar, so attachment remains connected while body, trim, and bracket mass visibly scale.
- Edge Softness affects slab, stem, joint, and shade-body bevel language; tests treat it as subtle but explainable rather than a whole-silhouette change.

## Candidate Directions

Authored strategy labels:

- Compact Task Lamp
- Tall Reading Lamp
- Playful Curved Lamp
- Heavy Base
- Minimal Studio Lamp
- Wide Shade Lamp

The focused catalog tests compile all six authored direction states and require at least four distinct whole-model silhouettes. Playful Curved Lamp uses the new `playful_tilt_shade` provider.

## Contact-Sheet Gate

Command run:

```bash
cargo run -p shape-cli -- foundry-visual-benchmark --profile stylized-lamp --proposal-count 72 --out-dir target/stylized-lamp-template-hardening --skip-blender
```

Status: passed.

Generated evidence:

- `target/stylized-lamp-template-hardening/explore/contact-sheet.png`
- `target/stylized-lamp-template-hardening/refine/contact-sheet.png`
- `target/stylized-lamp-template-hardening/silhouette/contact-sheet.png`
- `target/stylized-lamp-template-hardening/structure/contact-sheet.png`
- `target/stylized-lamp-template-hardening/detail/contact-sheet.png`
- `target/stylized-lamp-template-hardening/validation.json`
- `target/stylized-lamp-template-hardening/conformance.json`

Explore output produced six candidate cards. The catalog test gate also compiles generated Explore candidates and requires at least four distinct quantized whole-model extents with no surviving `TooSubtle` or duplicate-looking candidates.

## Manual Review Notes

At least four lamp ideas are visibly distinct through combinations of compact/tall height, lighter/heavier bases, straighter/curvier stems, small/large joints, provider-specific shade silhouettes, and shade scale. Shade/body/stem differences are readable in untextured clay because they affect role bounds and structural provider geometry, not only hidden parameters.

Adversarial critic status: approved for Prompt 5. The pass rejects capsule-chain fallback by requiring lathe and sweep sources, rejects disconnected shade/stem/base assemblies through attachment and recipe-connection tests, rejects invisible Shade Scale by moving scale to a visible child shade body, and rejects TooSubtle whole-asset candidates through generated Explore legibility checks.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo test -p shape-foundry-catalog --test stylized_lamp --jobs 1`: passed, 9 tests.
- `cargo test -p shape-search foundry --jobs 1`: passed.
- `cargo test -p shape-render foundry --jobs 1`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo build --release --workspace`: passed.
