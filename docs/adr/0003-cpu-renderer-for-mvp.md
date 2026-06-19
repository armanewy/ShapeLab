# ADR 0003: CPU Renderer For MVP

## Decision

The MVP uses a CPU preview renderer before a custom GPU viewport.

## Rationale

The central unknown is the candidate-selection interaction, not renderer architecture. CPU rendering is easier to test, debug, and keep deterministic while the modeling pipeline stabilizes.
