# Roman Bridge Dogfood Hardening v3

## Tier Decision

Current decision: `PreviewOnly`.

Roman Bridge HQ remains downgraded honestly. The fixture has useful dogfood
coverage, but the broader Usable gate still requires six surviving whole-asset
directions or an approved explicit exception. Current catalog evidence records
four surviving directions, and no exception is approved.

## Catalog-Owned Checks

Prompt 4B is scoped to catalog files. The catalog now records a local v3
dogfood report for `roman-bridge-hq`:

- Usable direction gate: 6 surviving directions.
- Observed surviving directions: 4.
- Preview/dogfood visible idea floor: 4 distinct ideas.
- Local preparation threshold: 10,000 ms for compile plus preview-mesh
  availability.
- Visible idea controls: supports, deck width, bracing, railing, and structural
  heft.

The Roman Bridge integration tests now cover:

- HQ preparation compiles and produces a whole-model preview mesh, then either
  completes under the local threshold or reports a long-running preparation
  state.
- Supports, deck, bracing, railing, and structural heft remain primary visible
  quick controls.
- Explore returns six generated cards, rejects `TooSubtle` survivors, compiles
  candidates, and requires at least four distinct whole-model signatures.
- The profile remains `PreviewOnly` while fewer than six directions survive.

## Preparation Reliability

Catalog scope can verify deterministic compilation, model validity, connected
attachments, export/reopen, preview-mesh availability, and whether the local
preparation check should be considered long-running. It cannot own the
interactive Make path after a user starts the template.

These reliability concerns remain app-owned:

- automatic model/preview preparation queueing;
- disabled action reasons while preparation is blocked or still running;
- stale background job rejection after the document changes.

Those behaviors already live in `shape-app` job and state reducers, outside this
prompt's ownership. No app UI, runtime LLM, surface, rig, or motion code was
changed for this pass.

## Visible Ideas

The dogfood controls intentionally map to plain bridge ideas:

- `support_style`: round piles, squared posts, stone piers, and trestle frames;
- `deck_width`: separated deck planks that visibly widen the walkable surface;
- `bracing_style`: minimal ties, X brace, K brace, and heavy reinforcement;
- `railing_style`: low curb, guard courses, and lookout courses;
- `structural_heft`: heavier spans, supports, braces, and deck thickness.

These are suitable for preview-mode dogfooding. They are not enough to claim
Usable until the six-direction gate passes or an exception is approved.

## Result

Prompt 4B hardening keeps `roman-bridge-hq` available in preview catalog mode
and out of the default novice catalog. The right next fix for blocked or stale
preparation is in `shape-app`, not the catalog authoring layer.
