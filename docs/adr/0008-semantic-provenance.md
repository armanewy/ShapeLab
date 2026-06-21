# ADR 0008: Semantic Provenance

## Decision

Compiled explicit assets must preserve provenance from generated elements back to semantic part, region, and operation IDs.

## Rationale

Future editing, material assignment, UV generation, diagnostics, and export adapters need to know why topology exists. Position-only inference is fragile after topology-changing parameters. Semantic provenance lets later systems reason about generated panels, trim, bevel bands, attachment regions, and repeated parts without coupling to generator internals.

## Consequences

- `FaceMetadata` stores optional part definition, part instance, region, operation, smoothing group, and generic surface role.
- `EdgeMetadata` stores boundary role, hard or smooth classification, seam candidacy, operation, and optional region transition.
- `AssetArtifact` includes a `ProvenanceReport` with mapping counts and topology signatures.
- Vertex and face IDs remain topology-signature scoped, while semantic IDs remain stable across unrelated parameter changes.
