# Make Pipeline Reliability

## Contract

The Make canvas must not expose novice-facing `Build Asset` or `Refresh Preview`
actions. Starting a template automatically queues model preparation and then a
preview render. The visible state progresses through:

- `Preparing model`
- `Rendering preview`
- `Ready`

When the current preview is missing, stale, or rendering, the local copy is
`Preview is updating...`. The manual recovery action is `Update preview`.

## Timeout Recovery

If preparation takes longer than the local timeout, the Make canvas shows:

```text
Still preparing. You can keep waiting or retry.
```

The visible recovery actions are:

- `Retry preparation`
- `Choose another template`
- `Open Project`

`Retry preparation` cancels stale local preparation work, clears candidates, and
requests a fresh build. Busy build, preview, and idea-generation requests are
deduplicated by the reducer so repeated clicks do not create duplicate active
jobs.

If idea generation exceeds the local timeout, Make shows:

```text
Still trying ideas.
```

The visible recovery actions are:

- `Cancel`
- `Keep waiting`

`Cancel` marks active candidate work stale, records `JobCanceled` in the Make trace,
clears pending idea cards, and shows `Canceled earlier idea search.` locally in Make.

## Local Status

The Make inspector owns the local workflow banner. Required banner states are:

- `Ready to try ideas`
- `Preparing <asset>`
- `Trying ideas`
- `Older result ignored` with `Try again`
- `Idea search canceled` with `Try again`
- `No clear focused ideas survived` with recovery actions

The bottom status strip may repeat supporting details, but it is not the primary
Make workflow state.
