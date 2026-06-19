# ADR 0002: Implicit First Backend

## Decision

The first editable geometry backend is an implicit SDF shape graph.

## Rationale

SDF primitives and CSG make robust category-independent form changes possible without requiring a humanoid schema or imported mesh semantics. Other backends can later plug into the same candidate/history loop.
