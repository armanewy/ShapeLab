# Asset Scoring Notes

## Prompt 5.2 Scope

`shape-search::asset::scoring` implements deterministic, non-AI scoring for
compiled asset candidates. It does not generate candidates. Callers provide
recipe, compilation, topology, provenance, and coarse geometry facts; the module
returns hard rejections, descriptors, quality penalties, duplicate groups, and
up to six representatives.

## Hard Rejection

Hard gates run before descriptor comparison:

- invalid recipe
- compile failure
- nonmanifold required closed part
- accidental intersection above tolerance
- missing attachment
- triangle budget exceeded
- non-finite or empty geometry
- incomplete provenance

Rejected candidates are reported with the first failing reason and do not enter
duplicate collapse or diversity selection.

## Descriptor And Quality Channels

Descriptors stay separate rather than becoming a universal beauty score:
world bounds, volume approximation, fixed-camera silhouette occupancy, part
count, major-part proportions, region/detail count, symmetry, repeated elements,
bevel-to-size ratios, and topology cost.

Quality is also reported as separate penalties for tiny parts, extreme
thinness, near-coincident surfaces, inconsistent bevel scale, detached visual
components, and excessive detail relative to primary forms. The weighted
penalty is only a selection policy input and duplicate tie-breaker.

## Selection Policy

The default policy first collapses duplicates by recipe fingerprint or very
small weighted descriptor distance. Within a duplicate group, the retained
candidate is the one with lower weighted quality penalty, then lower triangle
count, then lexicographically smaller candidate ID. This prevents the same
recipe under different tessellation density from dominating results.

Representative selection is deterministic max-min diversity over weighted
descriptor distance. The seed is the lowest-penalty unique candidate. Each
following representative maximizes its minimum weighted descriptor distance to
the selected set, with a small quality-penalty adjustment and stable ID
tie-breaks. The default representative count is six.
