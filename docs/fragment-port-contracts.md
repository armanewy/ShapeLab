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
- parent role and parent port
- child role and child port
- pairing policy
- finite rigid offset: translation plus canonical normalized quaternion
- attachment mode

Bindings are directional. `AttachmentRule::from_role` is the child role and `AttachmentRule::to_role` is the parent role. The concrete `AssetRecipe` attachment is written onto the child occurrence and points at the parent occurrence/socket.

The current executable remapper supports explicit exported occurrence roots only. Generated array or mirror occurrence expansion is rejected until generated occurrences have an addressable port-expansion policy. `AllPairs` is valid only when each child receives one parent.
