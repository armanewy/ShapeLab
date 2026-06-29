# Sci-Fi Crate Foundry Profile

The public profile ID remains `sci-fi-crate`, but the executable profile is now
Cargo Case family plus Sci-Fi Industrial style/profile defaults.

Sci-Fi Crate is an advanced regression profile. It is not the flagship product
proof and must not be polished as the primary product objective. Simple Crate is
the novice baseline proof, Utility Crate is the next family-maturity rung, and
Cargo Case is the advanced equipment-case proof.

## Architecture

- Family: Cargo Case
- Style/profile: Sci-Fi Industrial
- Compatibility slug: `sci-fi-crate`
- Shared roles: body, lid, panel fields, edge trim, corner guards, feet/skids,
  handles, vents, fasteners, and optional closure/detail roles
- Shared controls: Overall Proportions, Structural Heft, Panel Complexity,
  Handle Style, Vent Density, Trim Style, Detail Density

Sci-Fi Industrial is not a separate bespoke crate family and must not carry a
hidden duplicate implementation outside Cargo Case slots.

## Catalog Role

The profile may remain default-visible only while current dogfood evidence says
the Sci-Fi Crate Make baseline is non-regressed. If that evidence goes stale or
fails, move the profile to preview/developer catalog visibility until the
regression baseline is restored.

Simple Crate is the first novice crate baseline. Do not treat Sci-Fi Crate as
the novice crate proof.

## Sci-Fi Industrial Defaults

- stronger corner guards
- hard chamfer language through armor-block guards and reinforced trim
- deep framed panel fields
- mechanical side-rail handle provider
- dense vent grille provider
- reinforced edge trim
- high detail density
- darker Semantic Clay display groups for vents/recesses/trim

No textures, decals, labels, material maps, UVs, rigging, or animation are
introduced by this profile.

## Candidate Strategies

- Light Industrial
- Reinforced Cargo
- Compact Vented
- Wide Equipment Case
- Minimal Industrial
- Detailed Utility Case

At least four Explore candidates must remain visibly distinct in untextured
clay.

## Material-Look Preview Compatibility

Existing Sci-Fi Crate material-look preview evidence remains preview-only. If
the Cargo Case migration changes the frozen geometry fingerprint, old
material-look evidence is stale and must be regenerated or disabled. It must not
be silently reused against changed geometry.

The static surface package command remains:

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

That command may package the static prop path, but it does not make full
game-ready status pass. `game_ready` must remain false until later manual
DCC/runtime review, engine import proof, and engine-native package handoff
exist.
