# Fragment Port Contracts

Recipe fragments are private unless they export a contract through `RecipeFragmentExports`.

## Export Kinds

- `role_occurrence_roots`: externally visible roots that count toward family role cardinality.
- `internal_roots`: helper roots that are merged but do not count as role occurrences.
- `socket_ports`: named sockets exposed on fragment-local occurrences.
- `surface_ports`: named surface regions exposed on fragment-local definitions or occurrences.

Port IDs and tags use stable lowercase identifiers. Socket ports must reference an occurrence inside the fragment and a socket on that occurrence's local definition. Surface ports must reference a local definition or occurrence and a local region on that target.

## Attachment Bindings

`FamilyImplementation::attachment_bindings` connects selected fragments through exported ports. A binding names:

- the family attachment rule
- source role and source port
- destination role and destination port
- pairing policy
- finite offset
- attachment mode

Bindings are directional and must match the family attachment rule's source and destination roles. Future remap stages will resolve these bindings after provider selection through the `remap::ports` and `remap::relationships` boundaries.

Until those remap stages are executable, non-empty attachment bindings are rejected during implementation validation so authored attachments cannot be silently ignored.
