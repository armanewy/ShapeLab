# Bend Fitting Notes

## Scope

Wave 3 Prompt 3.1 adds deterministic uniform-curvature bend hypothesis
generation and local parameter fitting. The implementation lives in
`v3::bend_fit` and the existing `v3::inference::generate_bend_candidates`
entry point delegates to it with empty topology/weight slices for compatibility.

## Candidate Generation

- Source positions are analyzed with weighted PCA.
- Longitudinal axes are the three PCA axes plus world X/Y/Z, sign-canonicalized
  and deduplicated by absolute dot product.
- Bend directions are orthogonal projections of the other PCA axes, world axes,
  weighted mean residual direction, and a stable dominant projected residual
  covariance direction.
- Origins include the weighted source centroid, source bounding-box center, and
  centroid offsets along bend direction and binormal.
- Intervals are deterministic weighted quantile ranges over source projections.
- Coarse angles are fixed signed degree values; no random sampling is used.

## Fitting And Ranking

All valid combinations are evaluated with the schema-3 bend evaluator and
rejected unless they improve weighted SSE. Coarse candidates are deduplicated by
canonicalized parameters and strict output-geometry tolerance, then the best
eight are refined with deterministic coordinate descent over angle, interval
start/end, and origin offsets along bend direction and binormal. The final
result is capped at sixteen candidates sorted by weighted error, raw error, and
stable parameter keys.

Each returned candidate carries cumulative positions, weighted/raw errors,
semantic parameter and metadata counts, a stable parameter-derived candidate ID,
and fitting diagnostics.
