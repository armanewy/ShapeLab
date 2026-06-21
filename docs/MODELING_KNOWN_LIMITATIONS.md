# Modeling Known Limitations

- `AddPanel` and `AddTrim` are currently semantic operation records. The benchmark uses authored panel and trim parts for production geometry.
- Asset validation relationship metadata is supplied by the CLI for built-in benchmarks. The recipe schema does not yet serialize explicit part relationship policies.
- Search scoring therefore treats accidental intersection validation as baseline-relative for the current recipe, so existing authored contact state does not eliminate every semantic candidate before relationship metadata is carried through recipes.
- Lathe edge treatment is authored through profile loops and companion trim parts. `SetBevelProfile` is not supported on lathe sources.
- The construction timeline is deterministic and useful for users, but stage classification is name and operation based rather than a full procedural dependency graph.
- Asset Modeling Lab is now implemented as a native app mode. The older `docs/asset-app-contracts.md` file remains useful as historical design context, but the live implementation is in `crates/shape-app/src/asset`.
- The visible Refine/Explore loop uses `shape-search::asset` semantic proposal generation plus compile-time scoring and diversity selection. The scoring descriptors are deterministic approximations; they are not a substitute for human visual/artistic review.
- Candidate preview rendering is non-fatal per card, but a generation can still return fewer than six cards if the semantic proposal pool has too few valid, non-duplicate survivors under the requested budget.
