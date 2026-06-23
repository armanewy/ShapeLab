# Strict Semantic Modeling Contract

Wave 11 establishes Shape Lab's shared modeling-program IR for both forward modeling and inverse semantic reconstruction.

The source of truth is a `ModelingProgram`, not an anonymous target mesh plus a correction buffer. A strict-success result must replay through the canonical evaluator and produce the exact semantic topology, exact canonical positions, and exact serialization order without residual bytes.

## Crates

- `shape-program`: serializable IR, grammar profiles, semantic selections, operation descriptors, admissibility policy, and exactness contracts.
- `shape-program-verify`: strict-success verifier and operation admissibility checks.
- `shape-inverse`: inverse reconstruction failure reports and residual diagnostics that are explicitly excluded from strict success.

## ModelingProgram

Every program declares:

- `schema_version`
- `grammar_profile`
- optional versioned `base_topology`
- ordered `operations`
- reusable semantic `selections`
- explicit `dependency_graph`
- `canonical_evaluator_version`

Every operation must also declare enough accounting metadata for strict verification:

- direct semantic parameter count
- affected element count for direct parameters
- compact payload descriptors
- affected element count for payloads
- perturbation-validity status for payloads

Allowed strict grammar profiles are:

- `strict_from_primitives`
- `strict_from_versioned_library`

A versioned-library base must be cataloged, versioned, and independently fingerprinted. It must not be derived from the uploaded target during the same reconstruction.

## Semantic Admissibility

`SemanticAdmissibilityPolicy` defines anti-cheating limits:

- maximum parameter growth relative to affected elements
- maximum explicit selection payload size
- forbidden opaque payload kinds
- forbidden operation kinds
- minimum compression ratio
- perturbation-validity requirement

The maximum parameter-growth rule applies to direct `ModelingOperation.parameters` and to any payload descriptor. Explicit index payload descriptors must also stay within the maximum explicit-selection payload limit.

Forbidden strict-success operation or payload families include:

- `SetAllPositions`
- `MoveVertex`
- dense displacement
- literal target mesh
- opaque residual
- per-vertex independent positions
- one arbitrary cage weight per vertex

Compact semantic selections are allowed for parts, regions, boundary loops, edge classes, face patches, symmetry partners, geodesic neighborhoods, spatial primitives, compact falloff fields, and semantic landmark groups. Large explicit index lists are not valid semantic explanations.

## Exactness Split

`SemanticTopologyExact` covers:

- graph connectivity
- polygon boundaries
- winding
- part/object membership
- geometry-carrying topology

`SerializationOrderExact` is separate:

- vertex index order
- face index order

A target-index permutation may be recorded only as an audit/export adapter. It does not count toward semantic explanation size and cannot repair failed semantic topology.

## Strict Success

Strict success requires:

- exact canonical positions
- exact semantic topology
- exact serialization order
- zero residual bytes
- zero literal target mesh bytes
- zero per-vertex independent position parameters
- every operation admissible
- unique operation and selection IDs
- dependency graph edges that reference known IDs, are acyclic, and respect operation order
- sufficient compression ratio
- valid nearby semantic perturbations

Strict search may fail. Failure reports must include the best semantic program found, unexplained topology and geometry, missing operator capabilities, search limits reached, and any residual diagnostic clearly marked as excluded from strict success.
