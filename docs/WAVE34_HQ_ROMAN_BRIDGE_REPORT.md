# Wave 34 - HQ Roman Timber Bridge Vertical Slice

Wave 34 adds `roman-bridge-hq`, a high-quality Visual Foundry bridge profile
for the Roman Timber Engineering style kit.

## Implemented

- Added `roman_bridge::hq_fixture_catalog()`.
- Registered Roman Timber Bridge HQ as the eleventh built-in Visual Foundry
  profile.
- Added HQ provider defaults for stone piers, segmented deck, X brace, guard
  rail courses, and bolted joinery details.
- Added provider variants for supports, deck, bracing, railing, and connector
  detail density.
- Added a required HQ connector/detail role and `connector_to_deck` attachment.
- Added seven product-facing controls and seven whole-model direction
  strategies.
- Marked `roman-bridge-hq` as a Usable kit candidate that stays hidden from the
  default catalog until manual review.
- Added tests for HQ controls, strategy labels, conformance, model validation,
  and deterministic Explore candidate survival.

## Quality Evidence

Command:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export --json
```

Result:

- quality tier achieved: Usable
- quality blockers: none
- model valid: true
- validation issue count: 0
- triangle count: 2,568
- required roles covered: 7 / 7
- candidate survival count: 6
- six direction availability: true
- primary controls checked: 7
- controls with visible deltas: 7
- export/reopen: verified

## Remaining Gate

Manual review is still pending. The kit remains hidden from the default novice
catalog until that review is approved. Adversarial review remains required
before any Showcase claim.

