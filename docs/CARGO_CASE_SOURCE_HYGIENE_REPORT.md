# Cargo Case Source Hygiene Report

Date: 2026-06-29

Scope: Cargo Case proof stabilization source and report readability audit.

## Verification Command

```bash
python3 scripts/check_source_hygiene.py
```

Result: PASS.

This report claims audit-friendly source and docs only because the script passed
for the checked file list.

## Script Criteria

- Report physical line count for each checked file.
- Report maximum physical line length for each checked file.
- Report non-code-block lines longer than 180 characters.
- Report whether each checked file is likely physically collapsed.
- Fail if `crates/shape-app/src/foundry/app.rs` has fewer than 500 physical
  lines while still containing major app logic.
- Fail if any listed Markdown report has fewer than 20 physical lines.
- Fail if any non-code-block line exceeds 220 characters.
- Fail if any file has more than 5 non-code-block lines over 180 characters.
- Fail if any file contains a physical line over 1000 characters.

## File Metrics

| File | Physical lines | Max line | Non-code >180 | Likely collapsed |
| --- | ---: | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 10941 | 172 | 0 | No |
| `crates/shape-foundry-catalog/src/cargo_case.rs` | 1239 | 129 | 0 | No |
| `crates/shape-foundry-catalog/src/scifi_crate.rs` | 31 | 93 | 0 | No |
| `crates/shape-foundry-catalog/src/lib.rs` | 1633 | 145 | 0 | No |
| `README.md` | 362 | 149 | 0 | No |
| `docs/CARGO_CASE_STABILIZATION_REPORT.md` | 118 | 84 | 0 | No |
| `docs/CARGO_CASE_SOURCE_HYGIENE_REPORT.md` | 56 | 80 | 0 | No |
| `docs/CARGO_CASE_ARCHITECTURE_INTEGRATION_REPORT.md` | 97 | 173 | 0 | No |
| `docs/CARGO_CASE_BASE_FAMILY_V1_REPORT.md` | 44 | 80 | 0 | No |
| `docs/CLEAN_UTILITY_CASE_PROFILE_REPORT.md` | 73 | 80 | 0 | No |
| `docs/SCIFI_INDUSTRIAL_CARGO_CASE_PROFILE_REPORT.md` | 93 | 127 | 0 | No |
| `docs/CURRENT_PRODUCT_STATUS.md` | 128 | 121 | 0 | No |
| `docs/CARGO_CASE_FOUNDATION_STRATEGY.md` | 163 | 81 | 0 | No |

## Notes

- `app.rs` remains large, but it is physically expanded and has no line over
  the 180-character report threshold.
- The requested Cargo Case Markdown docs are physically expanded, with headings,
  bullets, table rows, and fenced code blocks split onto real lines.
- Future source hygiene claims should be regenerated from
  `scripts/check_source_hygiene.py` before merge.
