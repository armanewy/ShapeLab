# Shape Lab Agent Rules

Shape Lab is a native, offline desktop application for preference-guided procedural 3D modeling. It must not contain browser code, a server, Blender integration, an LLM integration, or humanoid-specific engine concepts.

## Required Commands

Every implementation branch must finish with:

```bash
cargo fmt --all --check
cargo test -p <owned-crate>
cargo clippy -p <owned-crate> --all-targets -- -D warnings
```

Integration branches must run the workspace-level commands requested by their wave prompt.

## Path Ownership

Parallel workers may edit only the paths explicitly assigned by their prompt. Do not edit root manifests, `Cargo.lock`, shared contracts, or another worker's files unless the prompt explicitly grants that ownership.

If a dependency addition is needed during a parallel wave, document it in the branch notes file instead of editing the root manifest. The following integration wave owns dependency reconciliation.

## Public Contracts

The public names and basic signatures in `docs/contracts.md` are shared contracts. Parallel workers may add private helpers and tests, but must not break these contracts.

## Worktree Safety

Workers are not alone in the codebase. Do not revert edits made by other workers. If incoming changes affect your task, adapt to them and document the decision in your wave notes.

## Scope Boundaries

Before the MVP release gate passes, do not add:

- Natural-language modeling
- LLM integration
- Blender integration
- Imported mesh editing
- Rigging
- Animation
- UV unwrapping
- Texturing
- GPU compute
- Adaptive octrees
- Dual contouring
- Collaborative/cloud features
- Plugin systems
- Structural candidate mutations
