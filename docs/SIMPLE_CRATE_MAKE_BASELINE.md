# Simple Crate Make Baseline

Date: 2026-06-29

## Verdict

Simple Crate is the default novice Make baseline and the first starter in the
default catalog.

The intended dogfood path is:

```text
Choose Simple Crate
-> Make ready
-> Try crate ideas
-> Use this crate
-> Adjust crate
-> Add to Pack
-> Export
```

## Product Copy

Simple Crate uses plain Make copy:

- Try crate ideas
- Use this crate
- Adjust crate
- Add to Pack
- Export

The baseline does not require Focus Part. Simple Crate currently shows no part
chips in the novice path. If focused part chips are added later, they should be
limited to Body, Lid, Trim, and Feet, and each chip should appear only when it
has useful candidates.

## Catalog Position

Simple Crate appears first in the default novice catalog when its starter
quality evidence passes. Sci-Fi Industrial Crate remains available as a
regression and advanced-profile check, but it is no longer the primary novice
baseline.

## Evidence

Dogfood evidence for this branch:

- Video: `target/simple-crate-make-baseline/simple-crate-dogfood.mov`
- Latency summary: `target/simple-crate-make-baseline/make-latency-summary.json`

The dogfood pass checks that the buttons are obvious, the user can see what
changed, Add to Pack and Export are visible, there are no dead ends, and the
flow avoids technical terms.

## Scope

This baseline stays in native offline Shape Lab. It adds no browser flow,
server, Blender path, runtime LLM path, imported mesh editing, rigging,
animation, UV/texturing UI, or broad game-ready claim.
