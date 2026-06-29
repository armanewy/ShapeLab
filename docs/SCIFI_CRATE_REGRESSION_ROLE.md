# Sci-Fi Crate Regression Role

Date: 2026-06-29

## Decision

Sci-Fi Crate remains in Shape Lab as an advanced regression profile. It is not
the flagship product proof and must not be polished as the primary product
objective.

Simple Crate is the new novice baseline proof. Cargo Case is the advanced
equipment-case proof. Sci-Fi Crate is kept because it exercises existing
advanced paths that still need protection.

## What Sci-Fi Crate Proves

Sci-Fi Crate is allowed to prove only these things:

- the stable `sci-fi-crate` profile ID still resolves;
- the Sci-Fi Crate Make baseline still prepares, generates ideas, and exports;
- Cargo Case compatibility remains intact for the Sci-Fi Industrial profile;
- material-look preview gating stays truthful and preview-only;
- stale material-look evidence is disabled after geometry changes;
- the static surface package command still works;
- full game-ready remains blocked.

## What Sci-Fi Crate Does Not Prove

Sci-Fi Crate does not prove:

- the first novice crate experience;
- broad asset-family authoring;
- broad Surface mode;
- persistent/exported material looks;
- UV/texturing product support;
- rigging, skinning, animation, or motion support;
- full game-ready status.

## Catalog Policy

If Simple Crate exists and passes its baseline, Simple Crate becomes the
first/featured novice crate. Sci-Fi Crate may remain visible only while current
dogfood evidence says it is non-regressed. If that evidence is stale or failing,
Sci-Fi Crate moves to the preview/developer catalog until the regression
baseline is restored.

On the current baseline, Simple Crate is not implemented catalog content. Do
not add placeholder Simple Crate metadata or reorder catalog entries as if it
already passed.

## Material-Look Boundary

Material looks remain a narrow preview-only path. They are backed by generated
surface-candidate evidence and must not affect the exported shape payload.

After any geometry-changing Make operation, material-look evidence tied to the
old frozen geometry fingerprint is stale. The app must disable that evidence or
require regeneration instead of silently reusing it.

## Static Package Boundary

The supported regression command remains:

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

This command may emit the static prop package and surface sidecar evidence, but
the package must keep `game_ready` false until manual DCC/runtime review,
engine import proof, and engine-native package handoff are complete.
