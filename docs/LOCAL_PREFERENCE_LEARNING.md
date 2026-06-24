# Local Preference Learning

Wave 29 adds a local preference-learning contract for Visual Foundry candidate
generation. It records explicit user choices and uses a bounded derived profile
to bias future whole-model candidate selection.

## Boundary

Preference learning is local and candidate-biasing only.

It may use:

- accepted candidates
- rejected candidates
- visible controls changed by those candidates
- explicit locks and resets
- exported variants
- pack membership

It must not use:

- mesh vertices or hidden geometry payloads
- recipe snapshots or literal generated assets
- file paths, export directories, or cloud identifiers
- direct semantic document mutation
- LLM prompts, embeddings, or model-specific state

## Runtime Contract

`shape_foundry::FoundryPreferenceLog` stores local-only explicit events. A log
derives a `FoundryPreferenceProfile` for a single
`FoundryPreferenceScope { family_id, customizer_profile_id }`.

`shape_search::foundry::FoundryCandidateRequest` accepts an optional
`preference_profile`. If the profile is wrong-scope, empty, non-local, or uses an
unsupported schema, candidate generation ignores it and reports the reason in
`FoundryCandidatePreferenceReport`.

When a profile applies, candidate generation still runs the same validity,
compilation, conformance, descriptor, duplicate-collapse, and diversity gates.
The profile contributes only a bounded selection bonus after valid candidates
exist. Novelty is preserved through the descriptor-distance floor.

## App Integration

The native Foundry reducer records explicit candidate accept/reject actions into
its local preference log. Future candidate requests automatically attach a
same-scope derived profile when useful local signals exist.

The semantic Foundry document and project revision graph remain deterministic and
replayable. Preference events are local session data, not semantic edits.
