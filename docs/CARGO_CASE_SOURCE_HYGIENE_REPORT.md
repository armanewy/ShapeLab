# Cargo Case Source Hygiene Report

Date: 2026-06-29

Scope: Cargo Case proof stabilization source/readability audit.

## Summary

All checked files are audit-friendly. No checked app or catalog source file is
physically collapsed, and the checked files stay under the 180-character line
length threshold after stabilization cleanup.

## File Metrics

| File | Physical line count | Max line length | Audit-friendly |
| --- | ---: | ---: | --- |
| `crates/shape-app/src/foundry/app.rs` | 10936 | 172 | Yes |
| `crates/shape-foundry-catalog/src/cargo_case.rs` | 1239 | 129 | Yes |
| `crates/shape-foundry-catalog/src/scifi_crate.rs` | 31 | 93 | Yes |
| `crates/shape-foundry-catalog/src/lib.rs` | 1633 | 145 | Yes |
| `README.md` | 362 | 149 | Yes |

## Notes

- `app.rs` is large, but it is not collapsed into a handful of physical lines.
- `scifi_crate.rs` is intentionally small because it is now a compatibility
  shim over the Cargo Case profile implementation.
- Important Cargo Case reports are Markdown-formatted with headings on their
  own lines, bullets/table rows on separate physical lines, and no giant
  single-line sections.
