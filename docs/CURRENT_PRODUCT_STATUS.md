# Current Product Status

Date: 2026-06-29

## Verdict

`SCI-FI CRATE MAKE BASELINE DOGFOOD-ACCEPTABLE`

Product Dogfood Gate v4 passed for the Sci-Fi Industrial Crate baseline only.
The approved scope is the default novice `Choose -> Make -> ideas -> focus
parts -> add to pack -> export` flow for Sci-Fi Crate.

This is not a broad Visual Foundry stability claim. Roman Bridge HQ remains
`PreviewOnly`, Surface remains limited, and larger user-facing UV/Texturing,
Rigging, motion, and full game-ready UI work remains blocked.

## Current Truth

- Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only.
- The recorded release-app run stayed in Shape Lab and completed the required
  scenario with full video, 540p video, screenshots, Make trace, and latency
  summary evidence.
- First visual response was immediate enough for the gate, and no ambiguous
  `Preparing` state exceeded the 12s recovery budget.
- Latency still needs tightening: the recorded run reached preview-ready at
  7290 ms and first selectable whole-asset idea at 11260 ms.
- Roman Bridge HQ remains downgraded to `PreviewOnly`.
- Do not start broader user-facing UV/Texturing/Rigging integration from this
  result.
- The only allowed next product step is narrow: Sci-Fi Crate visual Surface
  candidates v0.
- The Sci-Fi Crate visual Surface candidates v0 branch exposes `Try material
  looks` as a crate-only, preview-only Make UI path backed by headless evidence.
  It does not affect export payloads yet.
- Do not claim broad texturing, rigging, animation, or full game-ready product
  support from the current Visual Foundry UI.
- Headless/backend-only work may continue if it does not touch product UI and
  does not overclaim product support.

## Evidence Interpretation

Passing evidence means the checked contract passed. Product Dogfood Gate v4
passes the Sci-Fi Crate baseline; it does not automatically make every template
or every future product surface dogfood-stable.

| Evidence | Current Interpretation |
| --- | --- |
| Product Dogfood Gate v4 | Pass for Sci-Fi Crate baseline only |
| Full and 540p v4 video | Required visual evidence for the narrow pass |
| Make trace and latency summary | Useful timing evidence; shows latency follow-up remains |
| Rust tests, clippy, release build | Required engineering gates; not UX proof by themselves |
| Starter dogfood benchmark | Useful template evidence; not human review |
| Screenshot hashes/assertions | Useful state proof; not broad product-stability proof |
| Prompt 5 recording | Historical no-go evidence superseded only for Sci-Fi Crate baseline |
| Roman Bridge HQ benchmark | Passes four-direction recovery evidence, but remains PreviewOnly |

## Roman Bridge HQ

Roman Bridge HQ is downgraded to `PreviewOnly` for default catalog purposes.
The template has useful generated-idea evidence, but the broader HQ Usable-tier
gate still requires six surviving direction candidates or an approved exception.
No exception is approved.

## Approved Next Product Step

The next product step may be:

```text
Sci-Fi Crate visual Surface candidates v0
```

That work must stay narrow:

- use the Sci-Fi Crate baseline only;
- require matching generated surface-candidate evidence and textured previews;
- keep material looks preview-only unless persistence/export inclusion is
  explicitly implemented and reviewed;
- keep material and texture claims visibly caveated;
- use visual candidate evidence, not broad Surface mode claims;
- preserve Product Dogfood Gate v4 evidence paths and hashes;
- rerun the dogfood gate if the Make flow changes materially.

## Still Blocked

Do not proceed with:

- broad user-facing UV/Texturing UI;
- rigging or animation UI;
- motion/gameplay claims;
- full game-ready status;
- Roman Bridge or Lamp product-pass claims without their own gates.
