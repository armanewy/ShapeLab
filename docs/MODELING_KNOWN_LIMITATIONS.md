# Modeling Known Limitations

- `AddPanel` and `AddTrim` are currently semantic operation records. The benchmark uses authored panel and trim parts for production geometry.
- Asset recipes serialize authored relationship policies such as `MayOverlap`, `MinimumClearance`, containment, touch, and socket attachment intent. Relationship endpoints can target concrete instances, generated occurrences from an operation, a prototype plus generated occurrences, part tags, or definition role tags. These selectors currently expand to compiled part IDs during validation; richer authored policies for future Boolean-generated boundary loops are still pending.
- Lathe edge treatment is authored through profile loops and companion trim parts. `SetBevelProfile` is not supported on lathe sources.
- The construction timeline is deterministic and useful for users, but stage classification is name and operation based rather than a full procedural dependency graph.
- Asset Modeling Lab is now implemented as a native app mode. The older `docs/asset-app-contracts.md` file remains useful as historical design context, but the live implementation is in `crates/shape-app/src/asset`.
- The visible Refine/Explore loop uses `shape-search::asset` semantic proposal generation plus compile-time scoring and diversity selection. Visual descriptors now come from fixed-camera mesh masks, perimeter, visible z-buffer depth histograms, mesh volume, and recipe structure, but they are still deterministic heuristics rather than human visual/artistic judgment.
- Candidate preview rendering is non-fatal per card, but a generation can still return fewer than six cards if the semantic proposal pool has too few valid, non-duplicate survivors under the requested budget.
