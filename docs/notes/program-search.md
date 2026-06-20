# Ordered Program Search

Wave 3 program search enumerates explanatory depth up to two operators. The
terminal lossless correction is scored as the final correction, but it is not an
explanatory operator and does not count against depth.

The allowed shapes are:

- empty program
- affine
- bend
- affine then bend
- bend then affine

Enumeration is deterministic. Search adds the empty program first, retains all
finite non-no-op affine candidates, retains the top configured finite non-no-op
bend candidates, then composes affine-to-bend and bend-to-affine by requesting
the second provider from the cumulative first-stage positions. Equivalent
operator sequences are collapsed with stable bit-pattern keys, and the final
hypothesis list is sorted by diagnostics schema-4 score with deterministic
operator-key tie-breaks before applying `maximum_total_programs`.

Eligibility keeps the empty program unconditionally. Nonempty programs must
reduce final weighted error and meet `minimum_weighted_explained_fraction`.
Every stage must evaluate to finite geometry and exact no-op stages are dropped.
A stage with no meaningful incremental weighted-error reduction may remain only
when the complete sequence's schema-4 score is at least `1.0e-6` better than the
best retained program whose stages all made meaningful incremental progress.
This permits setup stages that enable a strong second operator while preventing
free extra operators from surviving on tie scores.

Scoring uses diagnostics schema 4 for every retained complete pre-correction
program. The score includes normalized final weighted error, total semantic
degrees of freedom, semantic metadata bytes, approximate residual coverage,
exact final residual bytes, family priors, and per-operator overhead. Per-stage
diagnostics record the weighted and raw error immediately before and after that
operator.
