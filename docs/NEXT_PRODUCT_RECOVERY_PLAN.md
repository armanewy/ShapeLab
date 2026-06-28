# Next Product Recovery Plan

## Rule

Do not start new app-facing UV, texturing, rigging, animation, or game-ready UI
work until this recovery plan passes its integration gate. Headless backend work
may continue only when it does not touch `shape-app` or change novice-facing
product claims.

## 0. Mainline Recovery Audit And Code Hygiene

Status: this branch.

Purpose:

- record the current Make Canvas failure shape;
- normalize contradictory docs;
- update README dogfood flow to Choose -> Make;
- keep behavior stable before product changes begin.

Exit criteria:

- audit document exists;
- screenshot/product-quality docs no longer conflict;
- README no longer tells dogfood testers to open removed Directions/Customize
  tabs as the current path;
- code remains formatted and current tests pass.

## 1. Make Canvas Interaction Recovery

Purpose: make Make function as a clear asset workspace.

Required outcomes:

- template start auto-prepares the asset without exposing build/preview
  sequencing to novices;
- `MakeCanvasViewState` is the single product-level contract for Make UI;
- running work owns the local model/candidate area with obvious overlays and
  skeleton trays;
- conflicts are disabled with plain-language local reasons;
- focused part state visibly changes the model stage and action row;
- generated ideas are above the fold with selected comparison visible;
- Pack and Export drawers communicate readiness and limitations.

No merge unless a human reviewer can complete the Sci-Fi Crate Make flow without
docs and can explain what changed.

## 2. Candidate Legibility Hardening

Purpose: prevent candidates that pass headless metrics but fail app perception.

Required outcomes:

- compare candidates at the same practical scale/camera context as app decision
  cards;
- reject DuplicateLooking and TooSubtle whole-asset results;
- focused candidates must show the selected part as the main visible change;
- diagnostics report generated, rejected-too-similar, hidden/internal, and wrong
  scope counts honestly;
- endpoint visibility reports remain deterministic.

## 3A. Sci-Fi Crate Template Hardening

Purpose: make crate generated ideas visibly authored in clay previews.

Required outcomes:

- body, panels, vents, handles, detail density, and trim read at card size;
- handles stay attached;
- vents and detail density are not tiny noise;
- at least four Explore ideas are visibly distinct in app-scale evidence.

## 3B. Roman Bridge Template Hardening

Purpose: make bridge generated ideas structurally readable in clay previews.

Required outcomes:

- deck width, span length, support style, bracing, railing, and detail density
  differ visibly;
- no floating or disconnected supports/braces/rails;
- at least four Explore ideas are visibly distinct in app-scale evidence.

## 3C. Stylized Lamp Template Hardening

Purpose: make lamp generated ideas readable without relying on subtle parameter
changes.

Required outcomes:

- height, base weight, curvature, shade style, shade scale, and joint size read;
- no disconnected shade/stem/base assembly;
- no capsule-chain fallback look;
- at least four Explore ideas are visibly distinct in app-scale evidence.

## 4. Starter Template Dogfood Benchmark

Purpose: connect headless evidence to human-visible product evidence.

Required outputs per starter:

- parent preview;
- generated ideas contact sheet at app decision size;
- selected comparison sheet;
- control endpoint sheet;
- option gallery sheet;
- legibility report;
- human dogfood notes.

Passing a headless benchmark is not enough. If a template passes metrics but a
human cannot tell what changed in the app, it must remain or return to
`PreviewOnly`.

## 5. Product Recovery Integration Gate

Purpose: integrate only when the product loop is usable.

Required gates:

- automated Rust gates pass;
- starter dogfood benchmark passes or downgrades failing templates;
- one clean video for Sci-Fi Crate, Roman Bridge HQ, and Stylized Lamp;
- Make Canvas screenshots include state assertion output and human verdict;
- docs agree with the actual state;
- no surface/rig/motion/full-game-ready overclaims.

Merge recommendation can be `PASS` only when the human video shows a novice can
complete Choose -> Make -> Try ideas -> Select/Focus -> Pack/Export without
reading implementation docs.
