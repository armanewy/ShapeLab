# Current Product Status

Date: 2026-06-28

## Verdict

`PRODUCT-RECOVERY BASELINE`

Current `main` is not a stable Visual Foundry baseline. Automated gates,
starter-template benchmarks, screenshot assertions, and recorded videos are
useful evidence, but the latest human dogfood review still identifies Make UX
blockers.

## Current Truth

- Current `main` is a product-recovery baseline.
- Automated gates may pass while the user-facing Make flow still fails.
- The latest human dogfood review found that Make can still strand the user in
  unclear preparing, disabled-action, no-ideas, focused-part, or stale-job
  states.
- Do not start larger user-facing UV/Texturing/Rigging integration until Make
  passes the new manual gate.
- Do not claim broad texturing, rigging, animation, or game-ready product
  support from the current Visual Foundry UI.
- Headless/backend-only work may continue if it does not touch product UI and
  does not overclaim product support.

## Evidence Interpretation

Passing evidence means the checked contract passed. It does not automatically
mean the product is dogfood-stable.

| Evidence | Current Interpretation |
| --- | --- |
| Rust tests, clippy, release build | Required engineering gates; not UX proof |
| Starter dogfood benchmark | Useful template evidence; not human review |
| Screenshot hashes/assertions | Useful state proof; not product-stability proof |
| Prompt 5 recording | Historical evidence; latest human review is no-go |
| Roman Bridge HQ benchmark | Passes four-direction recovery evidence, but remains PreviewOnly |

## Roman Bridge HQ

Roman Bridge HQ is downgraded to `PreviewOnly` for default catalog purposes.
The template has useful generated-idea evidence, but the broader HQ Usable-tier
gate still requires six surviving direction candidates or an approved exception.
No exception is approved.

## Required Next Gate

Before claiming stable Visual Foundry status, a human reviewer must complete the
default Make flow without implementation docs and without relying on the bottom
status strip to understand progress or recovery.

The next manual gate must specifically prove:

- template preparation does not look stuck;
- disabled actions always have visible local reasons;
- no-ideas and no-surviving-ideas states provide recovery actions;
- stale or ignored work is explained locally;
- focused-part generation cannot dead-end;
- candidate differences are readable in the actual app;
- pack/export readiness is visible without interpreting logs or reports.
