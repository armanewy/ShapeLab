# Showcase Gear Pack

The Wave 38 demo pack is `heroic-gear-demo-pack`.

Pack members:

- `fantasy-sword`
- `round-shield`
- `hero-helmet`
- `pauldron-pair`
- `chest-armor`

Shared product language:

- crisp heroic hard-surface silhouettes
- readable whole-model forms
- controlled detail density
- plain-language novice controls
- export/reopen evidence before Usable claims

The pack is a product-level demo pack across five asset families, not a single
compiled same-family Foundry pack document. Each member compiles and exports
through its own exact family/style catalog contract, while the pack report
records the coherence of the promoted set.

Required evidence is generated with:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile fantasy-sword --out-dir target/hq-benchmark/fantasy-sword
cargo run -p shape-cli -- hq-quality-benchmark --profile round-shield --out-dir target/hq-benchmark/round-shield
cargo run -p shape-cli -- hq-quality-benchmark --profile hero-helmet --out-dir target/hq-benchmark/hero-helmet
cargo run -p shape-cli -- hq-quality-benchmark --profile pauldron-pair --out-dir target/hq-benchmark/pauldron-pair
cargo run -p shape-cli -- hq-quality-benchmark --profile chest-armor --out-dir target/hq-benchmark/chest-armor
```

Promoted gear benchmarks verify export/reopen by default. `--verify-export`
remains accepted for explicit runs.

The minimum automated target is Usable. Showcase remains blocked until human
approval and adversarial visual review are recorded.
