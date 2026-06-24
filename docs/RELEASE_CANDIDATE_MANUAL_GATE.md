# Release Candidate Manual Gate

This checklist is the manual product gate for a release-readiness claim. It
tests the default novice Visual Foundry path only: no technical recipe surface
should be exposed or required for any task below.

Also complete the screenshot-oriented UI checklist in
[`docs/FOUNDRY_UI_MANUAL_GATE.md`](FOUNDRY_UI_MANUAL_GATE.md). The automated
`--verify-product-ui-gate` report proves the headless shell contract, but it
does not prove visual polish, viewport dominance, or whether the next action is
obvious to a human.

Run the native app from the repository root:

```bash
cargo run -p shape-app --release
```

## Test Tasks

Record start time before opening the first profile.

- Create a reinforced Roman bridge from the Roman Timber Bridge profile.
- Create a compact vented sci-fi crate family from the Sci-Fi Industrial Crate
  profile.
- Create a tall stylized lamp with a changed shade from the Stylized Furniture
  Lamp profile.
- Open and customize one Wave 26 expansion profile: Market Stall Kit, Sci-Fi
  Door Panel, Coopered Storage Barrel, Wayfinding Signpost, Workshop Chair,
  Market Handcart, or Storybook Tree.
- Generate six direction candidates for at least one profile.
- Lock one visible control, regenerate candidates, and verify the locked trait
  is preserved.
- Accept one candidate and explicitly reject at least one other candidate.
- Create a coherent three-member pack.
- Export the current asset or pack, reopen the saved project, and confirm the
  reopened result matches the accepted direction.
- Complete the flow with zero technical surface exposure.
- Capture the required UI screenshots from the UI manual gate. Notes are
  supplemental and cannot replace screenshots.

## Required Observations

For each run, record:

- Time to first valid model.
- Time to first accepted direction.
- Time to first export.
- Confusing labels.
- Dead controls.
- Invisible controls.
- Invalid attempts.
- Undo count.
- Technical surface exposure, expected to remain zero.
- Option-thumbnail completeness.
- Perceived performance or stutter.
- Export and reopen success.

## Result Template

```text
Tester:
Date:
Build or commit:

Time to first valid model:
Time to first accepted direction:
Time to first export:

Profiles tested:
- Roman Timber Bridge:
- Sci-Fi Industrial Crate:
- Stylized Furniture Lamp:
- Wave 26 expansion profile:

Six direction candidates generated: yes/no
Lock and regenerate preserved locked trait: yes/no
Accepted candidate: yes/no
Rejected alternate candidate: yes/no
Three-member pack created: yes/no
Export succeeded: yes/no
Reopen succeeded: yes/no
Technical surface exposed or required: yes/no
UI manual gate completed: yes/no

Confusing labels:
Dead controls:
Invisible controls:
Invalid attempts:
Undo count:
Option-thumbnail completeness:
Perceived performance or stutter:
Notes:
```

## Pass Criteria

- All four profile tasks produce valid visible models.
- Direction generation returns six whole-model candidates where requested.
- Lock and regenerate preserves the locked trait.
- Candidate accept/reject, pack creation, export, and reopen complete with zero
  technical surface exposure.
- Option thumbnails are visible for the default product path.
- The UI manual gate has screenshot evidence for launch, profile selection,
  direction board, customize, pack, export, disabled reasons, and 1280x800 /
  1440x900 layouts.
- Any confusing labels, dead controls, invisible controls, invalid attempts,
  undo usage, or stutter are recorded for triage.
