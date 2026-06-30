# Next Work After Family Pivot

Date: 2026-06-30

The family pivot is disciplined around one visible concept per gate. The current
box ladder is:

```text
Box Primitive -> Lidded Box -> Trimmed Box
```

Only Box Primitive and Lidded Box are app-visible product baselines today.
Trimmed Box is internal feature-module evidence until a later Make gate chooses
to expose it.

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

## Preferred Next Step

Stop the box ladder. The next visible family proof should be:

```text
Door Primitive
```

Door Primitive is preferred because it proves the kernel/module protocol on a
different object identity while staying simple:

- flat vertical panel identity
- front/back orientation
- hinge side and handle side
- panel zones
- future open/close capability, still blocked for now

Do not add Feet / Skids, panels, handles, crate language, material looks,
Family Studio public UI, or broad archetype work before the Door Primitive gate.

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
- Feet / Skids before the Door Primitive proof.
- Material looks, UV/texturing, rigging, animation, or game-ready UI.
- Runtime LLM integration.
- Full Family Studio public flow before two different kernels pass visual gates.
- Base-family field sprawl such as optional handles, vents, materials, or rigs
  on Box Primitive.
