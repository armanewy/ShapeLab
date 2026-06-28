# Make No-Dead-End States

## Candidate Tray State

The Make candidate tray renders from `MakeCandidateTrayState`:

- `EmptyReady`
- `GeneratingSkeletons`
- `HasCandidates`
- `NoCandidatesWithRecovery`
- `ErrorWithRecovery`

`EmptyReady` tells the user the asset is ready to try ideas.
`GeneratingSkeletons` keeps the tray visibly busy while generation or candidate
preview filtering is active. `HasCandidates` shows the comparison and candidate
cards. `NoCandidatesWithRecovery` and `ErrorWithRecovery` always include local
recovery actions.

## Focused No-Candidate Recovery

When a focused part search returns zero clear candidates, the user must see why
the search did not produce usable ideas and at least one local recovery path:

- `Try again`
- `Choose another part`
- `Unlock controls`

For Vents, the copy must acknowledge:

```text
Vents have limited visible variation in this template.
```

before offering recovery.

## Stale Work Recovery

When the reducer ignores stale background work, Make shows:

```text
An older result was ignored because you changed the asset.
```

The same local region must offer `Try again`. Stale results must not replace the
current model, preview, candidates, pack readiness, or export readiness.
