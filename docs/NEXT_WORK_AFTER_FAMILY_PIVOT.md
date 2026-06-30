# Next Work After Family Pivot

Date: 2026-06-30

The family pivot has been reset to the smallest honest visible ladder:
Box Primitive -> Lidded Box.

## Allowed

- Prove the Box Primitive Make loop end to end.
- Keep the built-in catalog limited to Box Primitive and Lidded Box until the
  next visual gate passes.
- Improve Box Primitive variation readability.
- Add local save/pack/export polish for Box Primitive and Lidded Box.
- Add exactly one visible module only after the baseline passes a manual gate.

## Gate Result

The Box Primitive screenshot/manual visual gate passed locally on 2026-06-30.
Evidence is recorded in:

```text
target/box-primitive-dogfood-gate/
```

See [`BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md`](BOX_PRIMITIVE_DOGFOOD_GATE_RESULTS.md).

The Lidded Box Make baseline gate passed locally on 2026-06-30. Evidence is
recorded in:

```text
target/lidded-box-make-baseline-gate/
docs/LIDDED_BOX_MAKE_BASELINE_GATE.md
```

## Preferred Next Step

The next branch should add the single next user-visible concept:

- Trim Band Feature Module v0.

Lid Seam Feature Module v0 and the Lidded Box Make baseline have local visual
evidence in:

```text
target/lid-seam-feature-module-v0/
target/lidded-box-make-baseline-gate/
docs/LID_SEAM_FEATURE_MODULE_V0.md
docs/LIDDED_BOX_MAKE_BASELINE_GATE.md
```

Do not add another module in the Trim Band gate.

Repair-loop polish may still happen as non-feature stabilization, but it must
not displace the next visible object step: prove Lidded Box -> Trimmed Box.

## Continuing Criteria

- The object reads as a box.
- The lid seam is visible in pure clay.
- The trim band is visible in pure clay when Trim Band is added.
- Every idea visibly differs.
- No UI copy claims a non-box family, surface/material, UV, rigging, animation,
  runtime LLM, or game-ready support.
- The user can complete the flow without technical authoring terms.

## Blocked

- Any non-box naming before visual-readability evidence exists.
- Any non-box built-in model profile.
- Feet / Skids before the Stool Primitive gate passes.
- Crate language before a model visually earns the name.
- Broad archetype library work.
- Surface/material editor work.
- UV/texturing UI.
- Rigging/skinning/animation UI.
- Runtime LLM integration.
- Full game-ready claims.
