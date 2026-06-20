# Diagnostics V4 Notes

Schema-4 diagnostics describe ordered explanatory programs before the terminal
lossless correction. A residual-only program serializes an empty `operators`
array; `NoOp` is intentionally not an operator family in this schema.

Each explanatory stage records the fitted operator parameters, weighted and raw
errors before and after the stage, the resulting explained increments, and the
semantic-to-baked verification policy and metrics. Exact package replay remains
the baked stage positions plus the final lossless correction.

Program scoring is serialized as independent components:

- normalized weighted final geometric error before final correction
- semantic parameter cost
- serialized scalar metadata cost
- approximate residual coverage cost
- exact residual byte cost
- sum of fixed family priors
- fixed per-operator overhead
- total component sum

Parameter counts reflect semantic degrees of freedom: translation 3, rigid 6,
similarity 7, general affine 12, and bend 9. Metadata byte cost is separate and
uses the actual serialized `f32` scalar payload. For example, a bend has 9
semantic degrees of freedom but serializes origin, axis, bend direction, angle,
and interval endpoints as 12 scalar values.

The default fixed family priors mirror the schema-2 affine priors and add Bend
at `1.5e-2`. This is a conservative starting value, higher than general
affine's `1.0e-2`, because non-zero bend evaluation and fit conditioning are
not implemented in this wave. The value is serialized in
`InferenceScoringPolicyV4.family_priors` so corpus behavior can be audited
without hidden tuning.

Tie-breaking is deterministic after total score: lower approximate residual
coverage, fewer exact residual bytes, fewer explanatory operators, fewer
semantic parameters, then lexical operator-family order.
