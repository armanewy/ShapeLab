# Handled Panel Make Baseline Gate

Status: PASS.

This gate exposes the first post-Hinged Panel feature in the Make loop:
Handle / Knob. The app-visible profile is Handled Panel.

Handled Panel is not a Door. It does not open or close. It does not claim
surface/material, UV/texturing, rigging, animation, runtime LLM, or game-ready
support.

## Product Boundary

- Parent baseline: Hinged Panel.
- New visible feature: Handle / Knob.
- App-visible profile: Handled Panel.
- Door naming remains blocked.
- Open/close motion remains blocked.
- Material looks, surface-only panels, part/focus chips, and Family Studio UI
  remain hidden for this profile.

## Make Flow

The validated user flow is:

```text
Choose Handled Panel
-> Make ready
-> Try handled panel ideas
-> Use one
-> Adjust handle or proportions
-> Add to Pack
-> Export
```

## UI Copy

Approved copy:

- Try handled panel ideas
- Use this panel
- Adjust handle
- Export Handled Panel
- Exports the current clay handled panel asset.
- This is not a textured, rigged, animated, or game-ready package.

Rejected copy:

- Door
- open / close
- motion / rig / animation
- material looks
- surface package
- focused part workflow

## Tests

Automated coverage verifies:

- Handled Panel is visible in the curated catalog after the Handle / Knob gate.
- The Make screen has no Material Looks panel for Handled Panel.
- The Make screen has no focus chips for Handled Panel.
- The Handle / Knob control appears.
- Handled Panel candidates generate and are selectable.
- Export copy is truthful.
- No Door, open/close, or animation claim appears in Handled Panel Make copy.

## Evidence

Release-app Computer Use evidence is recorded under:

```text
target/handled-panel-make-baseline-gate/
```

Screenshots:

- `choose-handled-panel.png`
- `make-ready-handled-panel.png`
- `generating-handled-panel-ideas.png`
- `generated-handled-panel-ideas.png`
- `selected-handled-panel-idea.png`
- `adjusted-handled-panel-control.png`
- `pack-drawer.png`
- `export-drawer.png`
- `evidence-manifest.json`

SHA-256 evidence hashes:

```text
cc7c3c97021416cbbdf1c189c0674f840b6875341cab34fa8504c9c5f38ca67a  choose-handled-panel.png
fcd3aa50cd7ffda4180525a1160beee15ccd9fc4fbcd76560941982ff76001fd  make-ready-handled-panel.png
11d4bc8d51adcf6198b57acb7c1ac29f4f0467f47e3a09464ed73984ae886cf1  generating-handled-panel-ideas.png
30228af93c2578f0067a8fc8faee600dd8f846cc7351708cdce48dc8e8dd96a0  generated-handled-panel-ideas.png
1381cb90e786554e440bdacd529748ac11f68b2da1bd2ccb5ce7ce6e9156f4e8  selected-handled-panel-idea.png
5b4c927fdaa10d7723ff210416b500750b844d5d927caf398273e158fb9ee243  adjusted-handled-panel-control.png
d03c569796f1320ba38d39eb852dc6cd4bb813b645bb0fc9f7081c1d8f573e3b  pack-drawer.png
b9add7c726ee33fd7b34d04fe918048ef925882471fb0d29218322237bb2959a  export-drawer.png
```

## Result

Pass criteria:

- user can see hinge edge and handle;
- candidates visibly differ;
- UI remains as simple as Flat Panel and Hinged Panel;
- no Door, motion, material, or part/focus distractions appear;
- Add to Pack and Export drawers still work.

The next allowed work must remain one visible concept per gate. Door naming is
still blocked until a human visual gate approves that the object has earned it.
