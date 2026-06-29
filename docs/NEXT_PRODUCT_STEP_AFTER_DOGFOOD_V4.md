# Next Product Step After Dogfood v4

Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only.

## Current Status

Sci-Fi Crate visual Surface candidates v0 passed as a preview-only baseline.
The Cargo Case architecture proof also passed for the equipment-case lane:
Cargo Case now supports both Clean Utility Case and Sci-Fi Industrial Crate as
profiles over one reusable clay base family.

## Allowed Next Steps

1. One more Cargo Case profile, if it stays inside the proven family and uses
   contact-sheet plus human review gates.
2. Sci-Fi Crate material persistence/export inclusion, only if explicitly
   scoped.
3. Stylized Lamp product dogfood pass.
4. Roman Bridge pass or continued PreviewOnly decision.

Scope:

- stay inside Sci-Fi Crate;
- require headless material-only candidate evidence before any app UI exposure;
- show visual material/surface candidate differences only when the app can
  render them clearly;
- keep the first app exposure preview-only unless project persistence and export
  inclusion are implemented and reviewed;
- keep all Surface, UV, texture, and game-ready claims caveated;
- preserve the `Choose -> Make -> ideas -> focus -> pack/export` loop;
- rerun Product Dogfood Gate v4 if the Make flow changes materially.

For Cargo Case follow-up, stay inside the declared Cargo Case roles, controls,
and provider slots. Do not use the proof to begin broad archetype expansion.

## Follow-Up Work

1. Make visual polish.
2. One more Cargo Case profile or Stylized Lamp product pass.
3. Sci-Fi Crate material persistence/export inclusion, if explicitly scoped.
4. Lamp and Roman Bridge product passes.
5. Broader UV/Texturing/Rigging only after separate dogfood evidence.

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

- Broad user-facing UV/Texturing UI.
- Rigging or animation UI.
- Motion/gameplay claims.
- Full game-ready status.
- Roman Bridge or Lamp product-pass claims without their own gates.
