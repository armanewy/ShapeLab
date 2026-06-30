# Next Work After Family Pivot

Date: 2026-06-30

The family pivot is disciplined around one visible concept per gate. The current
box ladder is:

```text
Box Primitive -> Lidded Box -> Trimmed Box
```

Only Box Primitive, Lidded Box, Flat Panel Primitive, and Hinged Panel are
app-visible product baselines today. Trimmed Box is internal feature-module
evidence until a later Make gate chooses to expose it.

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

The Hinge Edge feature-module gate passed catalog and visual evidence tests on
2026-06-30. Evidence is recorded in:

```text
docs/HINGE_EDGE_FEATURE_MODULE_V0.md
target/hinge-edge-feature-module-v0/
```

The Hinged Panel Make baseline gate passed release-app Computer Use validation
on 2026-06-30. Evidence is recorded in:

```text
docs/HINGED_PANEL_MAKE_BASELINE_GATE.md
target/hinged-panel-make-baseline-gate/
```

The second-kernel Flat Panel integration gate passed release-app Computer Use
regression validation and automated workspace gates on 2026-06-30. Evidence is
recorded in:

```text
docs/SECOND_KERNEL_FLAT_PANEL_INTEGRATION_REPORT.md
target/second-kernel-flat-panel-integration/
```

## Preferred Next Step

Stop the box ladder. The next single visible feature may be:

```text
Door Handle / Knob
```

The integration gate verified the whole visible set:

- Box Primitive
- Lidded Box
- Flat Panel Primitive
- Hinged Panel

Door Handle / Knob must still be only one visible concept in one branch:

- visible handle or knob clay geometry
- no inset panel
- no open/close motion
- no Door naming

Do not add Feet / Skids, crate language, material looks, Family Studio public
UI, open/close motion, or broad archetype work before Door Handle / Knob passes
its own visual gate.

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
- Feet / Skids before a deliberate return to the box ladder.
- Material looks, UV/texturing, rigging, animation, or game-ready UI.
- Runtime LLM integration.
- Full Family Studio public flow before two different kernels pass visual gates.
- Base-family field sprawl such as optional handles, vents, materials, or rigs
  on Box Primitive.
