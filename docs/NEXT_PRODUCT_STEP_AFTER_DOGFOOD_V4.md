# Next Product Step After Dogfood v4

Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only. That
result is preserved as regression evidence, not as the next flagship direction.

## Current Status

Sci-Fi Crate visual Surface candidates v0 passed as a preview-only baseline.
The Cargo Case architecture proof also passed for the equipment-case lane:
Cargo Case supports both Clean Utility Case and Sci-Fi Industrial Crate as
profiles over one reusable clay base family.

Shape Lab is not being built for one specific model. Sci-Fi Crate is a
regression/advanced profile, not the flagship. Simple Crate is the next
flagship family-authoring proof.

Roman Bridge remains PreviewOnly. Broad UV/Texturing/Rigging/Animation UI
remains blocked.

## Allowed Next Steps

1. Simple Crate Primitive v0.
2. Simple Crate Make baseline.
3. Utility Crate v1.
4. Cargo Case ladder reconciliation.
5. Dev-speed improvements.
6. Headless backend work that does not overclaim product support.

Scope:

- start from a simple object grammar;
- keep controls few until the clay variation is visibly useful;
- keep the Make loop fast;
- prove clay mesh quality before UV/texturing/materials;
- keep Surface/material work narrow, preview-only or headless, and
  evidence-backed;
- preserve the `Choose -> Make -> ideas -> focus -> pack/export` loop;
- rerun Product Dogfood Gate v4 if the Make flow changes materially.

For Cargo Case follow-up, stay inside the declared equipment-case roles,
controls, and provider slots. Cargo Case remains valid but scoped to equipment
cases only. Do not use it to begin broad archetype expansion.

## Follow-Up Work

1. Simple Crate Primitive v0.
2. Simple Crate Make baseline.
3. Utility Crate v1.
4. Cargo Case ladder reconciliation.
5. Sci-Fi Crate regression and material-look compatibility checks only when
   needed.

The detailed allowed and blocked list is in
`docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md`.

## Current Latency Follow-Up

The v4 pass still recorded latency misses:

- preview-ready: 7290 ms versus the 5s target;
- first selectable whole-asset idea: 11260 ms versus the 10s target.

These are follow-up items, not approval for a broader product expansion.

The follow-up branch records the detailed analysis in
`docs/MAKE_LATENCY_FOLLOWUP_V4.md` and adds trace fields for first visible
model, skeleton idea tray, candidate shells, candidate previews, reused jobs,
and coalesced jobs.

## Still Prohibited

- Broad archetype library.
- Broad Surface mode.
- Material editor.
- Broad user-facing UV/Texturing UI.
- Rigging, skinning, or animation UI.
- New profile explosion.
- More Sci-Fi Crate polish unless needed for regression.
- Motion/gameplay claims.
- Full game-ready status.
- Roman Bridge or Lamp product-pass claims without their own gates.
