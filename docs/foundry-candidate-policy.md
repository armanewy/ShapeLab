# Foundry Candidate Policy

Foundry candidate generation operates in customizer control space. A surviving
candidate is accepted by replaying its `FoundryEdit` commands against the
unchanged parent document.

## Modes

- `Refine` edits one or two unlocked topology-preserving controls. Numeric
  movement is capped at 15% of the effective control domain.
- `Explore` edits two to four unlocked controls. Numeric movement is capped at
  45% of the effective control domain, and a proposal may contain at most one
  provider or role-presence change. If the provider/role cap leaves fewer than
  two compatible controls, Explore emits no one-control fallback proposal.
- `Silhouette` edits only silhouette and major proportion controls.
- `Structure` edits provider, role-presence, and repetition controls.
- `Detail` edits detail-density and edge-treatment controls.

Locked and search-protected controls, providers, and roles are skipped before
proposal generation. Strategy IDs restrict the editable controls to the matching
customizer strategy from the resolved profile.

## Pipeline

1. Compile the unchanged parent Foundry document to resolve the catalog,
   profile, family slots, conformance context, and baseline descriptor inputs.
2. Generate 24 to 72 deterministic proposal programs from the request seed.
3. Replay each proposal as Foundry commands on a cloned document.
4. Compile the candidate document and require accepted conformance.
5. Derive visual descriptors from the compiled mesh artifact.
6. Hard-reject invalid candidates, collapse duplicates, and select at most six
   max-min diverse survivors through the existing asset scoring policy.
7. Return deterministic explanations that name the visible control labels.

Rejected proposals remain isolated: one failed replay, compile, conformance, or
descriptor pass does not stop later proposals from being evaluated.
