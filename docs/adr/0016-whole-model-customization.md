# ADR 0016: Whole-Model Customization

## Status

Accepted.

## Context

Low-level parameter rows are difficult for novice users to interpret. Family
parameters also do not always map one-to-one to visible controls: a "Structural
Heft" axis may drive several required slots, while provider choices need
whole-model previews.

## Decision

Represent novice controls with `CustomizerProfile`.

Controls own family slots through `ControlSlotBinding`. A slot may have only
one visible control owner. Controls expose a `FeasibleControlDomain` instead of
a single numeric range:

```text
continuous_intervals
discrete_values
unavailable_options
certification
```

Continuous sliders require `CertifiedContinuous` domains. Topology-changing
controls must use discrete values or galleries. Choice and provider gallery
options require whole-model preview references. Provider-gallery state uses
`ControlValue::Provider`, not a separate provider-only side channel.

Controls can report:

- `Synced`
- `DivergedByOverride`
- `Unavailable`
- `ConstraintLimited`

## Consequences

The customizer can avoid misleading sliders, represent nonconvex feasible
domains, and show users whole-model consequences instead of isolated parameter
names. Candidate search can target semantic controls without bypassing family
conformance or local-override tracking.
