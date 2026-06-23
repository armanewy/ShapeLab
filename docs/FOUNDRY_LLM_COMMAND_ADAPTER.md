# Foundry LLM Command Adapter

Wave 27 adds an optional adapter contract for LLM-facing command clients. It is
not an LLM integration, prompt parser, or geometry generator.

The adapter accepts a small structured intent enum and returns either:

- visible control descriptors,
- an LLM-safe state summary, or
- a validated `FoundryCommand`.

The host still owns natural-language parsing, preview rendering, user
confirmation, undo checkpoints, and command execution.

## Allowed Surface

The adapter maps only these intents:

- list visible controls,
- describe state,
- set a visible control,
- select a provider through a visible provider-gallery control,
- generate bounded candidate directions,
- lock or unlock a visible control,
- accept a currently proposed candidate,
- export through a host-approved export profile.

Provider selection is intentionally routed through visible provider controls.
The adapter does not expose catalog content references as the normal chat
surface.

## Safety Rules

The adapter enforces the Wave 27 product boundary:

- no direct recipe mutation,
- no geometry generation,
- no model SDK dependency,
- no hidden scalar paths in the LLM-facing surface,
- no command output for hidden controls,
- no command output for locked controls,
- no invalid `SetControl` values,
- no default/global candidate generation without an explicit safe strategy,
- no stale or non-proposed candidate acceptance,
- no lower-level provider override IDs outside visible provider options,
- no export without a host allow-list,
- no adapter-provided export destination,
- bounded candidate generation.

Every mutating command plan is marked as requiring preview before commit and an
undo checkpoint. Export plans are marked as host-side effects and require
explicit host confirmation; the host chooses the export destination.

## Host Contract

The adapter is a planning layer. A host must:

1. Show the planned command and preview result before committing it.
2. Create an undo checkpoint before mutating the Foundry document.
3. Re-run normal Foundry validation/runtime checks at execution time.
4. Choose export destinations outside the adapter/model path.
5. Keep rejected adapter plans visible as explainable failures.

This preserves the core product promise:

```text
LLM translates intent into the same typed commands the UI already uses.
Shape Lab remains the semantic source of truth.
```
