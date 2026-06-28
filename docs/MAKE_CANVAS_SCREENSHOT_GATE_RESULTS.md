# Make Canvas Screenshot Gate Results

## Status

`PROMPT 1 AUTOMATED VISUAL GATE PASSED; FINAL HUMAN DOGFOOD STILL REQUIRED`

Prompt 1 captured the required Make Canvas scenario screenshots from the
release macOS app bundle and recorded state assertions before capture. This is
stronger than the earlier file-existence/hash-only gate, but it is still not a
final product-stability verdict. Prompt 5 must record a full human dogfood video
for Sci-Fi Crate, Roman Bridge HQ, and Stylized Lamp.

Final dogfood status remains `HUMAN DOGFOOD NOT PASSED` / `NO-GO` until that
Prompt 5 video gate passes.

## Build And Capture

App bundle:

```text
target/release/Shape Lab.app
```

Screenshot path:

```text
target/make-canvas-interaction-recovery-v2/screenshots/
```

State assertion output:

```text
target/make-canvas-interaction-recovery-v2/shape-lab-screenshot-state-assertions.txt
```

## Captured Screenshots

- `01_choose.png`
- `02_make_ready.png`
- `03_generating_ideas.png`
- `04_generated_ideas.png`
- `05_selected_comparison.png`
- `06_focus_handles.png`
- `07_generating_handle_ideas.png`
- `08_handle_ideas.png`
- `09_focus_vents.png`
- `10_pack_drawer.png`
- `11_export_drawer.png`

## State Assertion Output

```text
MakeInitialCrate: PASS; mode=Ready; asset=Sci-Fi Industrial Crate; busy=false; tray=false; comparison=false; focus=None; pack=false; export=false
GeneratingWholeAssetIdeas: PASS; mode=GeneratingWholeAssetIdeas; asset=Sci-Fi Industrial Crate; busy=true; tray=true; comparison=false; focus=None; pack=false; export=false
GeneratedWholeAssetIdeas: PASS; mode=ReviewingIdeas; asset=Sci-Fi Industrial Crate; busy=false; tray=true; comparison=true; focus=None; pack=false; export=false
SelectedComparison: PASS; mode=ReviewingIdeas; asset=Sci-Fi Industrial Crate; busy=false; tray=true; comparison=true; focus=None; pack=false; export=false
FocusHandles: PASS; mode=FocusedPart; asset=Sci-Fi Industrial Crate; busy=false; tray=false; comparison=false; focus=Handles; pack=false; export=false
GeneratingHandleIdeas: PASS; mode=GeneratingFocusedPartIdeas; asset=Sci-Fi Industrial Crate; busy=true; tray=true; comparison=false; focus=Handles; pack=false; export=false
HandleIdeas: PASS; mode=ReviewingIdeas; asset=Sci-Fi Industrial Crate; busy=false; tray=true; comparison=true; focus=Handles; pack=false; export=false
FocusVents: PASS; mode=FocusedPart; asset=Sci-Fi Industrial Crate; busy=false; tray=false; comparison=false; focus=Vents; pack=false; export=false
PackDrawer: PASS; mode=PackDrawerOpen; asset=Sci-Fi Industrial Crate; busy=false; tray=false; comparison=false; focus=None; pack=true; export=false
ExportDrawer: PASS; mode=ExportDrawerOpen; asset=Sci-Fi Industrial Crate; busy=false; tray=false; comparison=false; focus=None; pack=false; export=true
```

## Image Sanity Output

Command:

```bash
bash crates/shape-app/tests/check_make_canvas_screenshots.sh target/make-canvas-interaction-recovery-v2/screenshots
```

Result:

```text
01_choose.png 2940x1912 2f17fecf2a2c8e1d236eebc4b477964ee7e6376b2ee9a64c3caf5cfd4acbeae3
02_make_ready.png 2940x1912 3701ed6178ec81dfae91021ba13c47cd83a57cfa59d90bb7d9fd0f6e8336a17b
03_generating_ideas.png 2940x1912 b35150bf54e214a681dda9c567d771cc93ef1b22df3800c3786d2993fd79d7e3
04_generated_ideas.png 2940x1912 9f81d9654537ff9a70d90e7a9f9efac533ec8547c6667871d61bf764585e6a5c
05_selected_comparison.png 2940x1912 93bdabf19967d14193e34c80eff79b90af085db01480ecbbcac067447b18a46a
06_focus_handles.png 2940x1912 8a1d1434bb723cc8c68999aa2e41526ecd36844e0734fef92b610790210aed05
07_generating_handle_ideas.png 2940x1912 f6e156c8c0c6a19a64edc5a0bfe9fdacf142080dc4cdef7e6448d0aecbbb5f07
08_handle_ideas.png 2940x1912 8974d2809c4d1bf72666309c308a972b3b4e6aa7a79bc9e8b2bf1b825d4c3fda
09_focus_vents.png 2940x1912 1997b2dba32d43d4890a3eb2a2de72437664b5c9265ed03d3f503337ec6840b9
10_pack_drawer.png 2940x1912 4dde8dd77bb8a22f382535897d755806b566edfb87e209716cc3ce0037379370
11_export_drawer.png 2940x1912 3e182e3a034e0404fa0418a11ab23552fe1c9d25f403e8b4ddc4da2c919276db
Make Canvas screenshot sanity passed.
```

## Pass/Fail Table

| Item | Result | Notes |
| --- | --- | --- |
| Make ready state has model and preview ready | Pass | State assertion and screenshot captured. |
| Whole-asset generation shows local busy state | Pass | Screenshot shows overlay, disabled actions, and skeleton tray. |
| Generated ideas are visible above the fold | Pass | Candidate tray and selected comparison are visible. |
| Selected comparison appears | Pass | Current/candidate/what-changed area is visible. |
| Handles focus changes the screen | Pass | Chip, title, callout, part action tray, and filtered controls change. |
| Focused generation shows local busy state | Pass | Screenshot shows focused overlay and skeleton tray. |
| Vents focus changes the screen | Pass | State assertion and screenshot captured. |
| Pack drawer opens visibly | Pass | Drawer state assertion and screenshot captured. |
| Export drawer opens visibly | Pass | Drawer state assertion and screenshot captured. |
| Screenshot hashes differ across adjacent states | Pass | Script checks all required adjacent pairs. |
| Full human dogfood video | Not yet | Required in Prompt 5. |

## Visual Review Verdict

Codex visual inspection of the captured screenshots: `PASS` for Prompt 1
scenario evidence.

This verdict is scoped to required screenshots only. It does not replace a
fresh human dogfood video of the full starter-template flow.

## Remaining Blockers

- Prompt 2 must keep unreadable candidates out of the UI.
- Prompt 3A/3B/3C must harden starter-template geometry.
- Prompt 4 must produce dogfood benchmark evidence for all three starters.
- Prompt 5 must capture a clean human dogfood video and issue the final merge
  recommendation.
