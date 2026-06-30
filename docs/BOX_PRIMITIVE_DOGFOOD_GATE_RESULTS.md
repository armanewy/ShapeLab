# Box Primitive Dogfood Gate Results

Date: 2026-06-30

## Status

`PASS - BOX PRIMITIVE SCREENSHOT AND MANUAL VISUAL GATE`

This gate validates the reset baseline only: Box Primitive. It does not approve
any non-box family, public catalog publishing, surface/material workflow,
UV/texturing, rigging, animation, runtime LLM integration, or game-ready claim.

## Scenario

The release app was captured through the native Make screenshot scenario hook:

```text
Choose Box Primitive
-> Make ready
-> Try box ideas
-> Review generated box ideas
-> Select a comparison
-> Focus Body
-> Try Body ideas
-> Open Pack
-> Open Export
```

Direct coordinate click-through automation was attempted, but the first Start
click was blocked by macOS assistive-access permission. The pass verdict below
therefore relies on the release-app screenshot scenario assertions, screenshot
sanity checks, and manual visual review of those captured states.

## Evidence

Evidence path:

```text
target/box-primitive-dogfood-gate/
```

Manifest:

```text
target/box-primitive-dogfood-gate/evidence-manifest.json
```

Release app binary:

```text
target/release/shape-app
```

Commit:

```text
291ed7eab42e118ec177d9bc3cf6353000eb298c
```

Screen size:

```text
2940x1912
```

Video:

```text
box-primitive-dogfood-gate.mov
sha256 b4d7dba3ac098dd9b0b65fa0c6e0e75148759f768e379eec14a1144b19a8d1eb
```

## Screenshots

| Screenshot | SHA-256 |
| --- | --- |
| `01_choose.png` | `c8b548d943129825fed4ce4c61a3ca4fa6d8ff0e9506660932f41a275dac5623` |
| `02_make_ready.png` | `348d820d17dff6917edded926952412ddda9cb8dec335cb04ee17c92810dec03` |
| `03_generating_ideas.png` | `b17168b045e7ba94a0232ee4dd008f0cb643fdead67ac8264ae8d16358a0ff4c` |
| `04_generated_ideas.png` | `17dbd420a990cc51ded5d5f9b9cdcb27df86c9ee2f4d34d8138c53db11ba358d` |
| `05_selected_comparison.png` | `a6a96773f28647c21d42c12a5ba7c19b7dc48ba714ab8143e5c1dba5a600ba60` |
| `06_focus_body.png` | `8b1b073e67e9f2d1726042a438d62a196ac222caf31ad1b0b4165b387b8c00c5` |
| `07_generating_body_ideas.png` | `12f5e7b43702b96e90f6f597b20e81812df5b3d582cb3955875f8c54a0f86ce0` |
| `08_body_ideas.png` | `4d126d75ba4570a196f09a6918049ffecbbb4725cb9f62005c91fd84c459c226` |
| `09_pack_drawer.png` | `6ca9b765e0a6091e129bb791d5173b27d607678f985add9c6b3e6dbab341e64c` |
| `10_export_drawer.png` | `5178a896b2661afa48617691afec265167d73642fa64deb13d935d5edd01bc88` |

## State Assertions

```text
MakeInitialBox: PASS; mode=Ready; asset=Box Primitive; busy=false; tray=true; comparison=false; focus=None; pack=false; export=false
GeneratingWholeAssetIdeas: PASS; mode=GeneratingWholeAssetIdeas; asset=Box Primitive; busy=true; tray=true; comparison=false; focus=None; pack=false; export=false
GeneratedWholeAssetIdeas: PASS; mode=ReviewingIdeas; asset=Box Primitive; busy=false; tray=true; comparison=true; focus=None; pack=false; export=false
SelectedComparison: PASS; mode=ReviewingIdeas; asset=Box Primitive; busy=false; tray=true; comparison=true; focus=None; pack=false; export=false
FocusBody: PASS; mode=FocusedPart; asset=Box Primitive; busy=false; tray=true; comparison=false; focus=Body; pack=false; export=false
GeneratingBodyIdeas: PASS; mode=GeneratingFocusedPartIdeas; asset=Box Primitive; busy=true; tray=true; comparison=false; focus=Body; pack=false; export=false
BodyIdeas: PASS; mode=ReviewingIdeas; asset=Box Primitive; busy=false; tray=true; comparison=true; focus=Body; pack=false; export=false
PackDrawer: PASS; mode=PackDrawerOpen; asset=Box Primitive; busy=false; tray=true; comparison=false; focus=None; pack=true; export=false
ExportDrawer: PASS; mode=ExportDrawerOpen; asset=Box Primitive; busy=false; tray=true; comparison=false; focus=None; pack=false; export=true
```

## Verification

Screenshot sanity command:

```bash
bash crates/shape-app/tests/check_make_canvas_screenshots.sh target/box-primitive-dogfood-gate/screenshots
```

Result:

```text
Make Canvas screenshot sanity passed.
```

## Pass/Fail Table

| Criterion | Result | Notes |
| --- | --- | --- |
| Box Primitive reads as a box | Pass | The captured model is a closed clay box-like volume. |
| Generated ideas visibly differ | Pass | Six ideas are visible: Compact, Wide, Tall, Flat, Soft-Edged, and Sharp Box. |
| UI avoids non-box family claims | Pass | Captured UI remains Box Primitive / box-focused. |
| UI avoids unsupported feature claims | Pass | No surface/material, UV, rigging, animation, runtime LLM, or game-ready claim appears. |
| Technical authoring terms stay out of the default flow | Pass | The visible Make flow uses product terms such as box, ideas, Body, Pack, and Export. |
| Pack and Export are reachable states | Pass | Dedicated Pack and Export drawer screenshots passed state assertions. |
| Continuous coordinate click-through proof | Not used | macOS assistive-access permission blocked the first Start click. Scenario screenshots are the gate evidence. |

## Current Status

Box Primitive is approved as the active internal novice baseline for the next
development step. This does not approve any richer model family or public release
claim.

## Merge Recommendation

Merge recommendation for the Box Primitive baseline branch: `PASS`, scoped to
Box Primitive only.

The next product step should be either:

- Box Primitive repair loop for weak variations.
- One visible Box Primitive module, starting with Lid Seam, on a separate branch.
