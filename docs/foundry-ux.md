# Foundry UX Contract

Shape Lab's foundry surface treats the semantic foundry document as the source
of truth and the generated `AssetRecipe` as a reproducible build output.

The novice-facing customizer is whole-model based:

- Every visible option must have a whole-model preview.
- A primary profile should expose at most seven controls by default.
- One family parameter slot may have only one visible control owner.
- Continuous sliders are allowed only for certified continuous domains.
- Topology-changing controls must be shown as discrete values or galleries.
- Provider galleries use `ControlValue::Provider` in control state and commands.
- A local override touching a controlled target marks that control as diverged.

The control state is separate from local overrides. Controls express family-level
intent; local overrides are explicit recipe edits applied after base family
instantiation. Overrides carry the geometry fingerprint they were authored
against and a survival policy:

- `Pinned`: keep only when the base geometry input is unchanged.
- `Revalidate`: replay and validate after compatible upstream changes.
- `DropOnStyleChange`: remove instead of reinterpreting across style changes.

Control feasibility is nonconvex. A control exposes a `FeasibleControlDomain`
with continuous intervals, discrete values, unavailable options, and a
certification. UI code may render a live slider only when the domain is
`CertifiedContinuous`; otherwise it should render discrete samples or a gallery.

Conformance is reported separately from foundry validation. Foundry validation
checks document/control/catalog consistency. Family conformance checks whether a
compiled asset satisfies required, advisory, and runtime-deferred family rules.
Required conformance rows must pass; deferred required rows are blockers, while
runtime-only deferred rows are explicit non-blocking deferrals.
