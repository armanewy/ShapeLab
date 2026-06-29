# Current Product Status

Date: 2026-06-29

## Verdict

`SIMPLE CRATE MAKE BASELINE ACTIVE; FAMILY FOUNDATION PIVOT RECORDED; SCI-FI CRATE REGRESSION BASELINE PRESERVED; CARGO CASE EQUIPMENT PROOF PASS`

Shape Lab is not being built for any one specific model. Sci-Fi Crate was useful
as a stress test and Product Dogfood Gate v4 baseline, but Sci-Fi Crate is an
advanced regression profile, not the flagship; in the existing family-pivot
wording, it is a regression/advanced profile, not the flagship.

Simple Crate is now the default novice Make baseline and the novice
family-authoring proof. Utility Crate is the next family-maturity rung. Quality
must still be proven gradually:

```text
Simple Crate Primitive
-> Utility Crate Family
-> Cargo Case
-> Product profiles
```

Clay mesh quality comes before UVs, texturing, materials, decals, or other
surface presentation. Surface/material work remains narrow and evidence-backed.

The Surface Candidate Integration Gate passed only for the narrow Sci-Fi Crate
material-look preview baseline. Material looks are preview-only in this build
and do not affect export payloads or full game-ready status. Material evidence
tied to stale geometry must be disabled or regenerated instead of reused.

Roman Bridge HQ remains `PreviewOnly`. Broad Surface mode, broad
UV/Texturing/Rigging/Animation UI, material editor work, rigging, skinning,
motion, and full game-ready UI work remain blocked.

The Cargo Case architecture proof passed for the equipment-case lane only:
Cargo Case is a reusable clay base family, Clean Utility Case and Sci-Fi
Industrial Crate are profiles over that same family, and `sci-fi-crate` remains
the stable compatibility ID. Cargo Case remains valid but scoped to equipment
cases only. This does not approve broad archetype expansion.

## Current Truth

- Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only.
- Simple Crate is first in the default novice catalog and is the new novice
  Make baseline.
- Utility Crate is the next family-maturity rung and should feel richer than
  Simple Crate while staying simpler than Cargo Case.
- Sci-Fi Crate remains a regression profile, advanced profile, material-look
  preview test, and Cargo Case compatibility test.
- Sci-Fi Crate is not the flagship proof.
- Simple Crate is implemented catalog content and the current novice
  family-authoring proof.
- Simple Crate Primitive is the next starting point; Utility Crate, Cargo Case,
  and product profiles must follow the maturity ladder.
- The recorded release-app run stayed in Shape Lab and completed the required
  Sci-Fi scenario with full video, 540p video, screenshots, Make trace, and
  latency summary evidence.
- First visual response was immediate enough for the gate, and no ambiguous
  `Preparing` state exceeded the 12s recovery budget.
- Latency still needs tightening: the recorded run reached preview-ready at
  7290 ms and first selectable whole-asset idea at 11260 ms.
- Roman Bridge HQ remains downgraded to `PreviewOnly`.
- Cargo Case architecture proof passes for equipment cases only.
- Clean Utility Case and Sci-Fi Industrial Crate share the Cargo Case family.
- `Try material looks` is crate-only, preview-only, and backed by generated
  surface-candidate evidence. It does not affect export payloads yet.
- Stale material-look evidence must be disabled after geometry changes.
- The Sci-Fi Crate static surface package command remains a regression check,
  not a full game-ready approval.
- Do not claim broad UV/texturing, rigging, skinning, animation, or full
  game-ready product support from the current Visual Foundry UI.
- Broad archetype expansion is forbidden until another family proof exists and
  passes its own clay, catalog, contact-sheet, and human review gates.
- Headless/backend-only work may continue if it does not touch product UI and
  does not overclaim product support.

## Current Allowed Product Claims

- Shape Lab is category-general, not built for one specific model.
- Simple Crate is the default novice Make baseline.
- Utility Crate is the next family-maturity rung after Simple Crate.
- Sci-Fi Crate Make baseline passes as a regression baseline.
- Sci-Fi Crate material-look preview baseline passes as a narrow preview-only
  baseline.
- Simple Crate is the current novice family-authoring proof.
- Cargo Case is a proven reusable clay family architecture, scoped to equipment
  cases only.
- Material looks are preview-only unless a later persistence branch says
  otherwise.
- Clean game-ready export is not yet supported.
- Roman Bridge remains `PreviewOnly`.
- Broad UV/Texturing/Rigging/Animation UI remains blocked.

## Current Allowed Next Work

- Simple Crate Primitive v0.
- Simple Crate Make baseline is active as the novice baseline.
- Utility Crate v1.
- Cargo Case ladder reconciliation.
- Dev-speed improvements.
- Headless backend work that does not overclaim product support.

See `docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md` for the controlling allowed and
blocked work list.

## Evidence Interpretation

Passing evidence means the checked contract passed. Product Dogfood Gate v4
passes the Sci-Fi Crate baseline as a regression baseline; it does not make
Sci-Fi Crate the flagship and does not automatically make every template or
future product surface dogfood-stable.

| Evidence | Current Interpretation |
| --- | --- |
| Simple Crate Make Baseline | Active novice baseline; first default catalog starter |
| Product Dogfood Gate v4 | Pass for Sci-Fi Crate regression baseline only |
| Surface Candidate Integration Gate v0 | Pass for Sci-Fi Crate material-look preview baseline only |
| Cargo Case Architecture Integration Gate | Pass for Cargo Case base + Clean Utility + Sci-Fi Industrial profiles only |
| Full and 540p v4 video | Required visual evidence for the narrow Sci-Fi regression pass |
| Material-look release video and screenshots | Required visual evidence for the preview-only material pass |
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

The approved product ladder remains the family foundation ladder:

```text
Simple Crate Primitive
-> Simple Crate Make baseline active
-> Utility Crate v1
-> Cargo Case ladder reconciliation
-> Product profiles
```

That work must stay narrow:

- keep Simple Crate as the novice baseline, not more Sci-Fi Crate polish;
- keep Utility Crate as the next family-maturity rung after Simple Crate;
- keep Sci-Fi Crate default-visible only while current dogfood status says it
  is non-regressed, otherwise move it to preview/developer visibility;
- prove visible pure-clay variation before semantic clay, material looks, UVs,
  textures, decals, or export-surface claims;
- keep material looks preview-only unless persistence/export inclusion is
  explicitly implemented and reviewed;
- keep material and texture claims visibly caveated;
- use visual candidate evidence, not broad Surface mode claims;
- preserve Sci-Fi Crate as regression, material-look preview, and Cargo Case
  compatibility evidence;
- rerun the dogfood gate if the Make flow changes materially.

## Still Blocked

Do not proceed with:

- broad archetype library;
- broad Surface mode;
- broad user-facing UV/Texturing UI;
- material editor;
- rigging, skinning, or animation UI;
- new profile explosion;
- motion/gameplay claims;
- full game-ready status;
- more Sci-Fi Crate polish unless needed for regression;
- Roman Bridge or Lamp product-pass claims without their own gates.
