# ADR 0011: Asset-Family And Style-Kit Layer

## Status

Accepted

## Context

Project Caesar exposed real production needs for modular assets, but the first dogfood project should not redefine Shape Lab as a Roman asset generator. Shape Lab's modeling kernel already uses generic concepts: parts, sockets, cuts, bevels, arrays, regions, boundary loops, provenance, deterministic search, and export validation.

The missing layer is a generic contract that describes asset families and visual style kits before any runtime adapter or game-specific pack is involved.

## Decision

Add `shape-family` as a runtime-neutral schema crate for:

- asset-family functional grammar
- part roles and attachment rules
- allowed operation classes
- parameter slots and ranges
- constraints and variant rules
- export metadata requirements
- compatible style-kit declarations
- style-kit proportions, bevels, profiles, prototypes, details, repetition, symmetry, and exaggeration

Keep Project Caesar style language in `shape-caesar-assets`. The Roman timber engineering kit is content-pack data that depends on generic family contracts.

## Consequences

- Core modeling crates stay theme-neutral.
- Caesar remains the first demanding customer without becoming the engine's identity.
- Sci-fi, furniture, architecture, weapons, and other packs can use the same family/style split.
- Runtime metadata remains optional and adapter-owned.
- Future modeling work should be justified against multiple unrelated families where possible.
