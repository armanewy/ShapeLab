# Box Primitive Baseline Cleanup Integration Report

Date: 2026-06-30

## Result

Pass.

The integrated baseline is one clean Box Primitive workflow:

```text
Choose Box Primitive
-> Make ready
-> Try box ideas
-> Use one box idea
-> Adjust Proportions or Edge Softness
-> Add to Pack
-> Export
```

## Merged Branches

| Order | Branch | Commit |
| --- | --- | --- |
| 1 | `codex/object-orchard-family-authoring-vision` | `a95a0af` |
| 2 | `codex/box-primitive-ui-truth-pass` | `06c4105` |
| 3 | `codex/box-primitive-visual-readability` | `a44d058` |

## Product Truth

| Check | Result |
| --- | --- |
| Box Primitive is the only novice baseline | Pass |
| It is a box, not a richer non-box family | Pass |
| No part/focus workflow is active for Box Primitive | Pass |
| No Material Looks panel is active for Box Primitive | Pass |
| Family Studio remains blocked/internal until the box baseline is stable | Pass |
| Future family authoring follows the Object Orchard vision | Pass |

## Automated Gates

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-foundry-catalog --test box_primitive --jobs 1` | Pass |
| `cargo test -p shape-search foundry --jobs 1` | Pass |
| `cargo test -p shape-render foundry --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Dogfood Gate

| Check | Result |
| --- | --- |
| Flow starts at Choose Box Primitive | Pass |
| Make screen is ready without broad catalog UI | Pass |
| Try box ideas produces visibly different ideas | Pass |
| One idea can be selected | Pass |
| Proportions or Edge Softness can be adjusted | Pass |
| Add to Pack opens the pack drawer | Pass |
| Export opens the export drawer | Pass |
| The preview reads as a box | Pass |
| No crate or material language appears | Pass |
| No part chips appear | Pass |
| Export copy is truthful | Pass |

## Screenshot Evidence

UI truth-pass screenshots:

```text
target/box-primitive-ui-truth-pass/screenshots/
```

| Screenshot | SHA-256 |
| --- | --- |
| `choose_box_primitive.png` | `534f65a07fe37d5e19cc4f61b220b9dc65213bacac59cf8c850f0185eff4eae2` |
| `make_ready_box_primitive.png` | `c6554e9280bb5d86fcefda6a8aa96720851c06fad307e9064a4ba0b3eae42a7d` |
| `generating_box_ideas.png` | `ecc23097d48bf31e6a0163f08dd71df13b68438b14790b7200be1c71e7bbeb0c` |
| `generated_box_ideas.png` | `a3961c552d54e9093836e6e0d0e475a5d623aaf3d258e946beb859ffad4382d6` |
| `selected_box_idea.png` | `b1773a1f476a47703b1060f836a9092253af203251010fe7e54f69838925144a` |
| `adjusted_box_control.png` | `b7ff0db02a7798a94c424f6b1dd38f9fa89ddce990a77b39486d8a5bf808b190` |
| `pack_drawer.png` | `2b4ee22687c50bab0460e48749d45d6b30112ccd493121a02e2c6abf8e4706e8` |
| `export_drawer.png` | `30fedc9f585707963a0e889a9b4a9cb651692899f6232c4d2f6e565b39c667b7` |

Visual-readability artifacts:

```text
target/box-primitive-visual-readability/
```

| Artifact | SHA-256 |
| --- | --- |
| `parent.png` | `f604a9c80bb3a63fa9e28aa2a502b51f7f48afae3492ea589e42ca948e18b2f6` |
| `candidate-contact-sheet.png` | `2b3fb6d058a0d7adcd0740d21d3bf092248009332d27245428b9ea9c68843745` |
| `control-endpoint-sheet.png` | `c420a83b9772036e1c3bc680c54a388677569538bf8903790220d98ae37e04a1` |
| `readability-report.json` | `40afa89ada5b3a480257a5d8b465bb5158ba35d89e7bbc24555d2a237ac33486` |

## Next Decision

Allowed next work:

- One visible feature module, probably Lid Seam.

Still blocked:

- Crate/case family work.
- Material looks and surface UI.
- UV/texturing.
- Rigging or animation.
- Public Family Studio flow.
- Multiple visible features in one branch.
