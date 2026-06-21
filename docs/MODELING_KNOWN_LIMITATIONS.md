# Modeling Known Limitations

- `AddPanel` and `AddTrim` are currently semantic operation records. The benchmark uses authored panel and trim parts for production geometry.
- Asset validation relationship metadata is supplied by the CLI for built-in benchmarks. The recipe schema does not yet serialize explicit part relationship policies.
- Lathe edge treatment is authored through profile loops and companion trim parts. `SetBevelProfile` is not supported on lathe sources.
- The construction timeline is deterministic and useful for users, but stage classification is name and operation based rather than a full procedural dependency graph.
- `compile-asset` writes static packages and previews. It does not implement interactive asset app mode.
- Texture ownership, stale job handling, and candidate acceptance are contract-only in `docs/asset-app-contracts.md`.
