# Foundry Usability Test Plan

## Scope

Foundry usability instrumentation is local-only session data. It is intended for
developer and user research builds that need aggregate workflow metrics without
external telemetry.

The default session payload records only relative event timing and coarse event
counts. It must not record model geometry, exported artifact contents, output
directories, or absolute file paths by default.

## Local Records

The local usability log supports these optional records:

- profile opened
- build completed, used for time to first build
- candidate request count
- candidate survival count
- candidate accepted count
- control change attempt with accepted/rejected outcome
- reset
- lock
- invalid attempt
- undo
- export, used for time to first export
- technical surface exposure, expected to remain zero in the default product app

Each record stores `elapsed_ms`, measured from local session start. The log does
not store wall-clock timestamps, model geometry, export directories, or absolute
paths.

## Metrics

The aggregate metrics are computed from the local log:

- `control_success_rate`: accepted control changes divided by all control change
  attempts. The value is absent when no control changes were attempted.
- `candidate_survival_rate`: survived candidates divided by requested
  candidates. The value is absent when no candidates were requested.
- `accepted_change_count`: accepted candidates plus accepted control changes.
- `invalid_state_attempts`: invalid state or command attempts.
- `advanced_view_visits`: historical technical-surface openings; this should
  remain zero in the default product app.
- `total_session_time_ms`: the largest relative event timestamp.
- `time_to_first_build_ms`: the first build-completed event timestamp, when one
  exists.
- `time_to_first_export_ms`: the first export event timestamp, when one exists.

## Verification

`crates/shape-foundry/tests/session_metrics.rs` covers:

- rate, count, total-time, and first-time calculations
- absent rate values when no denominator exists
- backward-compatible sessions without local usability data
- default serialized payloads omitting path, output-directory, mesh, vertices,
  and geometry fields

Required command coverage for this branch:

```text
cargo fmt --all --check
cargo test -p shape-foundry --test session_metrics
cargo clippy -p shape-foundry --all-targets -- -D warnings
```
