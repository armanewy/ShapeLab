# Foundry Command API

The foundry command API is a serializable automation contract. It has no text or
LLM dependency and does not require a UI.

Commands:

- `SetControl`: set a customizer control value using `ControlValue`.
- `ResetControl`: return a control to its authored default.
- `SelectProvider`: choose a provider for a family role.
- `SetRolePresence`: enable or disable a role.
- `SetStyle`: switch style and style implementation references.
- `SetLock`: set a foundry lock.
- `GenerateCandidates`: request deterministic candidate proposals.
- `AcceptCandidate`: accept a generated candidate.
- `RejectCandidate`: reject a generated candidate.
- `Undo`: move to the parent revision.
- `SwitchRevision`: switch to an existing revision.
- `Export`: export the current build through a profile.
- `AddCurrentToPack`: add the current document to a pack.

`SetControl` values are validated against the referenced control kind and
`FeasibleControlDomain`. Provider-gallery controls use `ControlValue::Provider`.

Queries:

- `StateSnapshot`
- `ControlDomain`
- `Candidates`
- `Revisions`
- `ExportProfiles`
- `Conformance`

The runtime build order for later waves is:

```text
resolve catalog
→ verify exact lock
→ evaluate effective family request
→ instantiate base
→ optional preliminary conformance
→ apply local overrides
→ validate recipe
→ compile final artifact
→ run authoritative final conformance
```

The final conformance report governs acceptance and export. Required rows that
are failed, deferred, unsupported, missing, or not evaluated block acceptance.

## Optional LLM Adapter Boundary

`shape-foundry` also exposes an optional structured LLM command adapter contract.
It does not call an LLM, parse natural language, mutate recipes directly, or
generate geometry. External chat/model code may translate user language into a
small `FoundryLlmIntent` value; the adapter then returns visible control
descriptors, an LLM-safe state summary, or a validated `FoundryCommand`.

The LLM-facing surface is limited to visible controls, provider options exposed
by provider-gallery controls, explicit safe candidate strategies, current
proposed candidates, and host-approved export profiles. Mutating command plans
require preview before commit and an undo checkpoint. Export plans require host
confirmation, and the host chooses export destinations outside the adapter.

See `docs/FOUNDRY_LLM_COMMAND_ADAPTER.md`.
