# Next Product Step After Dogfood v4

Product Dogfood Gate v4 passed for the Sci-Fi Crate Make baseline only.

## Allowed Next Step

1. Sci-Fi Crate visual Surface candidates v0.

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

## Follow-Up Work

2. Make visual polish.
3. Sci-Fi Crate material persistence/export inclusion, if preview-only material
   looks pass manual review.
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
