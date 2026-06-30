# Next Work After Family Pivot

Date: 2026-06-30

The family pivot is disciplined around one visible concept per gate. The current
box ladder is:

```text
Box Primitive -> Lidded Box -> Trimmed Box
```

Only Box Primitive, Lidded Box, and Flat Panel Primitive are app-visible
product baselines today. Trimmed Box is internal feature-module evidence until a
later Make gate chooses to expose it.

## Gate Results

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

The Trim Band feature-module gate passed locally on 2026-06-30. Evidence is
recorded in:

```text
target/trim-band-feature-module-v0/
docs/TRIM_BAND_FEATURE_MODULE_V0.md
```

The Flat Panel Primitive baseline passed automated catalog/app gates on
2026-06-30. Evidence is recorded in:

```text
docs/FLAT_PANEL_PRIMITIVE_BASELINE.md
target/flat-panel-primitive-baseline/
```

## Preferred Next Step

Stop the box ladder. The next visible family proof should be:

```text
Hinge Edge
```

Hinge Edge is preferred because Flat Panel Primitive already proves the second
kernel baseline. The next gate should add exactly one visible feature to that
panel:

- visible hinge-side edge geometry
- no handle/knob
- no open/close motion
- no Door naming unless a later human visual gate approves it

Do not add Feet / Skids, panels, handles, crate language, material looks,
Family Studio public UI, or broad archetype work before the Hinge Edge gate.

## Continuing Criteria

- The object name must match the visual result.
- Every idea visibly differs.
- New features must be behavior-bearing modules, not nullable fields on a base
  family object.
- Contact sheets and endpoint sheets must decide visual readability.
- No UI copy claims surface/material, UV, rigging, animation, runtime LLM, or
  game-ready support.
- The user-facing flow must use reusable-kit language, not kernel/module terms.

## Blocked

- Crate language before a model visually earns the name.
- Door naming before visible door cues pass a later gate.
- Feet / Skids before the Flat Panel and Hinge Edge proof.
- Material looks, UV/texturing, rigging, animation, or game-ready UI.
- Runtime LLM integration.
- Full Family Studio public flow before two different kernels pass visual gates.
- Base-family field sprawl such as optional handles, vents, materials, or rigs
  on Box Primitive.
