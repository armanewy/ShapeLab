# Perceptual Variation Gate v0

Candidate directions must be visibly different in the card preview. Control
changes, hidden edits, or internal recipe differences do not count by
themselves.

The v0 gate compares parent and candidate previews with a fixed CPU-rendered
camera at card resolution. Scores are clamped to `0..1`; non-finite comparisons
are rejected; transparent background pixels are ignored.

Shape candidates need shape, silhouette, structure, selected-part, or
screen-space evidence. Surface-only evidence cannot pass Shape. Detail
candidates may pass smaller visible deltas and are labeled Detail change or
Subtle but visible.

When fewer than six candidates survive, the board reports the smaller count.
Weak, hidden-only, duplicate-looking, unsupported, or explanation-mismatched
candidates are not padded back into the result.

Product copy examples:

```text
Generated 4 visually distinct directions.
Rejected 2 subtle candidates that looked too similar.
```
