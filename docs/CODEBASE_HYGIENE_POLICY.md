# Codebase Hygiene Policy

Date: 2026-07-01

## Scope

This policy applies during the cleanup and Object Orchard rename phase. It does
not add product features. It makes the repository easier to audit before more
Orchard controls, surfaces, collision, motion, terrain, or export work lands.

## Rust File Size

No Rust source file should exceed roughly 1000 non-test lines. Test modules
behind `#[cfg(test)] mod tests` are counted separately. Temporary exceptions
must be listed in `docs/RUST_FILE_SIZE_EXCEPTIONS.md` with:

- path
- current non-test line count
- owner
- split plan
- deadline

## Documentation

- No obsolete docs from abandoned pivots should remain active.
- No stale comments should describe removed behavior as current.
- Historical notes are allowed only when clearly marked as historical or
  migration context.
- Product-facing docs must not imply materials, UVs, collision, motion,
  terrain, rigging, animation, runtime LLM, public catalog publishing,
  Godot-ready, or game-ready support without a passed evidence gate.

## Public APIs

- No dead public APIs should remain just to support unreleased legacy behavior.
- No compatibility shims are required for old pivots because the product has not
  shipped.
- Public contracts that remain must support the canonical semantic asset lane:
  `AssetRecipe`, `AuthoringOpLog`, `RelationshipContract`, `PatternContract`,
  and truthful export/proof reports.

## UI Terms

User-visible abandoned terms must be removed or dev-gated. Active direct
primitive workflows must not expose generated variation trays, Try ideas,
candidate comparisons, or obsolete crate/cargo/scifi language.

## Design Principles

DRY and SRP matter. Correctness comes first. Cleanup should reduce accidental
coupling, but not by hiding behavior behind vague abstractions.
