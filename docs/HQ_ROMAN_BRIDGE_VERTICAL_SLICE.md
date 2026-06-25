# HQ Roman Timber Bridge Vertical Slice

`roman-bridge-hq` is the Wave 34 high-quality clay/mesh vertical slice for the
Visual Foundry bridge family. It is still procedural Foundry content: no
materials, UVs, rigging, animation, marketplace packaging, or LLM generation are
part of this slice.

## Product Controls

The HQ kit exposes seven novice-facing controls:

- Span Length
- Deck Width
- Structural Heft
- Support Style
- Bracing Style
- Railing Style
- Detail Density

Support, bracing, railing, and detail density are whole-model galleries. Span,
deck width, and structural heft are continuous controls with visible whole-model
effect.

## Direction Strategies

The authored whole-model direction set is:

- Reinforced
- Light Crossing
- Wide Deck
- Compact Span
- Stone-Pier Outpost
- Detailed Timberwork
- Minimal Clean Span

The deterministic HQ gate requires six generated Explore directions to compile,
pass model validation, and render non-placeholder clay previews.

## Evidence

Run:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export --json
```

Latest local evidence:

- quality tier achieved: Usable
- model validation: valid, zero issues
- triangle count: 2,568
- primary controls checked: 7
- controls with visible deltas: 7
- candidate survival count: 6
- export status: verified
- reopen status: verified
- default novice-catalog exposure: blocked pending manual review

