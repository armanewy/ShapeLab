# Make Canvas Interaction Redesign v3

Status: implementation in progress on `main`.

## Goal

Make the Make tab feel like an asset workspace instead of a settings page.

## Product Contract

- The model stage is the anchor and receives the largest share of the Make
  screen.
- The visible primary action is state-driven: `Try ideas`, focused `Try ...
  ideas`, disabled `Trying ideas...`, `Use this idea`, or focused recovery
  `Try whole-asset ideas`.
- Focused part actions are attached to the model stage and read as
  `Focused: Handles`, `Try handle ideas`, `Lock handles`, and `Clear focus`.
- The inspector starts short. Whole-asset view shows four controls before
  `More controls`; focused views show relevant controls before `Show all
  controls`.
- Generated ideas promote the comparison panel above thumbnails. Current and
  candidate previews are large, with `What changed` directly below.
- Pack and Export remain drawer surfaces with readiness copy and clear actions.

## Copy Rules

Forbidden visible product copy for this surface:

- `Candidate tray`
- `Variation Mode`
- `Complete Looks`
- `Model workspace`
- `Focus Part`
- user-facing build/refresh internals

Preferred copy:

- `Try ideas`
- `Ideas`
- `Compare`
- `What changed`
- `Focused: Handles`
- `Try handle ideas`
- `Ready`

## Required Evidence

Before marking this gate passed, capture:

- Make ready
- Focused Handles
- Focused Vents
- Generated ideas
- Selected comparison
- Focused no-clear-ideas recovery
- Pack drawer
- Export drawer
- Short dogfood video

Passing unit tests or screenshot state assertions is not enough for a product
pass; a human must be able to identify the next action within five seconds.
