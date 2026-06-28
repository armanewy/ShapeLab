# Source Format Hygiene Report

Date: 2026-06-28

Prompt 0 audited the source and documentation files called out by the recovery
prompt. The goal is reviewability in raw GitHub and future Codex edits, not a
repo-wide prose reflow.

## Results

| File | Physical Lines | Max Line Length | Audit-Friendly |
| --- | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 7113 | 135 | Yes |
| `docs/PRODUCT_RECOVERY_INTEGRATION_V2_REPORT.md` | 210 | 174 | Yes |
| `docs/MAKE_CANVAS_SCREENSHOT_GATE_RESULTS.md` | 125 | 172 | Yes |
| `docs/ROMAN_BRIDGE_TEMPLATE_HARDENING_REPORT.md` | 108 | 165 | Yes |
| `docs/STARTER_TEMPLATE_DOGFOOD_BENCHMARK_V2.md` | 90 | 102 | Yes |
| `README.md` | 319 | 149 | Yes |

## App Source

`crates/shape-app/src/foundry/app.rs` is not physically collapsed in the local
branch. It has normal Rust formatting, thousands of physical lines, and a max
line length below the 180-character hygiene limit.

No behavior-only app split was required for Prompt 0. Future app work should
still prefer moving Make-specific code into smaller modules when touching the
same area for product changes.

## Markdown Source

The listed Markdown files are audit-friendly after this pass. `README.md` had
several long prose paragraphs and was reflowed without changing product scope.

Older docs outside this prompt's target list still contain some long historical
lines. They are not part of this hygiene gate.
