# Surface Lab Static Prop v1

Surface Lab v1 is a headless, static-prop-first surface package path. It does
not add a Visual Foundry material editor, candidate-generation changes, runtime
LLM behavior, rigging, skinning, animation, or engine-native packages.

The first supported profile is `sci-fi-crate`. The CLI command:

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

emits a `surface/` directory with:

- `surface-artifact.json`
- `surface-validation-report.json`
- `surface-capabilities.json`
- `uv-layout.png`
- `material-swatch-sheet.png`
- `texture-contact-sheet.png`
- `triangle-slot-coverage.json`
- `textures/*.png`

The UV strategy is a deterministic normal-aware box projection into normalized
0..1 coordinates. Every exported triangle receives exactly one material slot
binding. Texture payloads are simple deterministic procedural PNG sidecars:
base color, metallic-roughness, flat normal, and neutral occlusion.

This is a real static-prop surface payload, not a full game-ready claim. Full
game-ready status still requires manual DCC/runtime review, engine import
proof, and any future engine-native package adapters.
