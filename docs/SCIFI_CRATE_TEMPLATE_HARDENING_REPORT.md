# Sci-Fi Crate Template Hardening Report

Status: PASS for authored Sci-Fi Crate template hardening. Visual benchmark evidence was generated under `target/foundry-benchmark/scifi-crate-hq/`.

## Art Direction Target

The Sci-Fi Industrial Crate targets a hard-surface equipment crate that remains readable in untextured clay previews:

- readable body silhouette from compact through broad/tall endpoints;
- attached handle assemblies with visible mounts or cheeks;
- front panel relief that catches shadows at card size;
- sparse, standard, and dense vent options that differ by count, spacing, size, placement, and rim mass;
- visible trim and fasteners, with low/medium/high detail-density reads;
- no floating parts and no validation-reported accidental intersections;
- no near-identical Explore ideas returned as normal whole-asset candidates.

## Internal Authoring Roles

Art Director:

- Increased body endpoint spread so `Body Proportions` changes whole-model silhouette, not just metadata.
- Required larger panel recesses, heavier clay-visible vent rims, and a six-slot dense vent bank.
- Kept the final object quiet and industrial rather than decorative.

Geometry Author:

- Enlarged the body envelope enough for compact variants to keep all cuts inside the host face.
- Bound both front recessed panel cuts to `Panel Depth`.
- Repositioned access plate, trim, handles, and fasteners so visible details are separated and validation-clean.
- Hardened handle providers as flush inset grip, side rail with brackets, and cargo bar with mounts.

Variation Designer:

- Preserved the six Prompt 3 candidate strategy labels: Compact Vented, Reinforced Cargo, Clean Lab Crate, Heavy Utility, Deep Panel Equipment, and Minimal Industrial.
- Expanded Detail Density from 4-12 to 4-16 fasteners.
- Kept candidate strategies multi-control so whole-asset ideas combine visible silhouette, vents, handles, panels, and detail changes.

Validation Engineer:

- Added export/package verification for the compiled crate.
- Strengthened tests for attached handles, structural vent differences, both panel-depth cuts, detail-density mesh growth, candidate distinctness, no returned `TooSubtle` whole-asset ideas, locks, conformance, and zero accidental intersections.
- `cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1` passes with 13 tests.

## Contact-Sheet Gate

Prompt 3 visual evidence was generated from the repository root:

```bash
cargo run -p shape-cli -- foundry-visual-benchmark --profile sci-fi-crate --proposal-count 72 --out-dir target/foundry-benchmark/scifi-crate-hq --skip-blender
```

Expected outputs:

- parent preview: `target/foundry-benchmark/scifi-crate-hq/parent/preview.png`
- candidate contact sheet: `target/foundry-benchmark/scifi-crate-hq/explore/contact-sheet.png`
- control endpoint sheets: `target/foundry-benchmark/scifi-crate-hq/control-strips/`
- option gallery sheets: `target/foundry-benchmark/scifi-crate-hq/option-galleries/`
- legibility/metrics report: `target/foundry-benchmark/scifi-crate-hq/metrics.json`

Generated metrics:

- parent mesh: 5,744 vertices, 6,524 triangles, 18 parts;
- Explore: 6 returned candidates; summary says `Generated 6 visually distinct ideas. Rejected 1 that looked too similar.`;
- Refine: 6 returned candidates;
- Silhouette: 4 returned candidates;
- Detail: 5 returned candidates;
- all primary controls measurable: true;
- package verification: checksums, topology, and numeric payloads valid; Blender runtime check skipped by `--skip-blender`.

Do not commit generated benchmark images or JSON unless a later integration prompt requests binary evidence in source control.

## Adversarial Critic

Can a user tell what changed in at least four crate ideas?

Yes. Returned Explore candidates must be selectable non-`TooSubtle` ideas with at least four distinct changed-control signatures.

Do handles look attached?

Yes. All three handle options compile, pass validation, include grip plus mounts/cheeks, and sit near the body front without accidental intersections.

Do vents read?

Yes. Sparse has two large framed slots, standard has three mid-size slots, and dense has six smaller slots in two rows.

Does detail density read?

Yes. The control spans 4-16 fasteners and the high endpoint increases mesh triangle count over the low endpoint.

Does any variant look like a broken procedural toy?

No automated evidence indicates that. Prompt 3 strategy states compile uniquely, pass conformance/model validation, export cleanly, and avoid accidental intersections.

## Verification

Verification passed:

```bash
cargo fmt --all --check
cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1
cargo test -p shape-search foundry --jobs 1
cargo test -p shape-render foundry --jobs 1
cargo run -p shape-cli -- foundry-visual-benchmark --profile sci-fi-crate --proposal-count 72 --out-dir target/foundry-benchmark/scifi-crate-hq --skip-blender
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
```

Blockers: none.
