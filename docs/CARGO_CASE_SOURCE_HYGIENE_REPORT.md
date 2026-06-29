# Cargo Case Source Hygiene Report

Date: 2026-06-29

Scope: Cargo Case source and report readability reality gate.

## Verdict

PASS.

The audit-friendly claim is based on `python3 scripts/check_source_hygiene.py`
passing on the checked file list in this branch.

No product behavior changes were made for this reality gate.

## Verification Command

```bash
python3 scripts/check_source_hygiene.py
```

## Script Output

```text
File                                                Physical lines  Max line  Lines >180  Lines >220  Likely collapsed
--------------------------------------------------  --------------  --------  ----------  ----------  ----------------
crates/shape-app/src/foundry/app.rs                 10941           172       0           0           no
crates/shape-foundry-catalog/src/cargo_case.rs      1239            129       0           0           no
crates/shape-foundry-catalog/src/scifi_crate.rs     31              93        0           0           no
crates/shape-foundry-catalog/src/lib.rs             1633            145       0           0           no
README.md                                           362             149       0           0           no
docs/CARGO_CASE_STABILIZATION_REPORT.md             118             84        0           0           no
docs/CARGO_CASE_SOURCE_HYGIENE_REPORT.md            86              118       0           0           no
docs/CARGO_CASE_ARCHITECTURE_INTEGRATION_REPORT.md  97              173       0           0           no
docs/CARGO_CASE_BASE_FAMILY_V1_REPORT.md            44              80        0           0           no
docs/CLEAN_UTILITY_CASE_PROFILE_REPORT.md           73              80        0           0           no
docs/SCIFI_INDUSTRIAL_CARGO_CASE_PROFILE_REPORT.md  93              127       0           0           no
docs/CURRENT_PRODUCT_STATUS.md                      128             121       0           0           no
docs/CARGO_CASE_FOUNDATION_STRATEGY.md              163             81        0           0           no

Source hygiene check passed.
```

## Script Criteria

- Reports physical line count for each checked file.
- Reports maximum physical line length for each checked file.
- Reports physical lines longer than 180 characters.
- Reports physical lines longer than 220 characters.
- Reports whether each checked file is likely physically collapsed.
- Fails if `crates/shape-app/src/foundry/app.rs` has fewer than 500 physical
  lines while still containing major app logic.
- Fails if any listed Markdown report has fewer than 20 physical lines.
- Fails if any non-code-block Markdown line exceeds 220 characters.
- Fails if any important Rust source file has more than 5 lines over 220
  characters.

## Local Raw Checks

| File | LF lines | CRLF sequences | Max line | Likely collapsed |
| --- | ---: | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 10941 | 0 | 172 | No |
| `docs/CARGO_CASE_SOURCE_HYGIENE_REPORT.md` | 86 | 0 | 118 | No |

## GitHub Raw Checks

Baseline remote checked:
`origin/codex/cargo-case-source-hygiene-fix` at
`aec8e32afc84be0440571ed4a3315558523d5d29`.

| File | HTTP status | LF lines | Max line | Likely collapsed |
| --- | ---: | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 200 | 10941 | 172 | No |
| `docs/CARGO_CASE_SOURCE_HYGIENE_REPORT.md` | 200 | 56 | 80 | No |

The target branch must also be checked in GitHub raw view after push because
that is the only point at which the new branch exists remotely.

## Notes

- `app.rs` remains large, but it is physically expanded and has no line over
  the 180-character report threshold.
- The requested Cargo Case Markdown docs are physically expanded, with headings,
  bullets, table rows, and fenced code blocks split onto real lines.
- Future source hygiene claims should be regenerated from
  `scripts/check_source_hygiene.py` before merge.
