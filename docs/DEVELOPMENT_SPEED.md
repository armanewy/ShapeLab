# Development Speed

Shape Lab is still moving through product recovery, so local development should
optimize for fast, relevant proof instead of treating every prompt lane as a
release candidate.

The goal is:

- inner loop: under 60-90 seconds
- branch handoff: under 5 minutes
- integration gate: under 15 minutes
- main/release gate: slow but rare

## Local Environment

Use the local environment helpers when working across multiple Codex worktrees:

```bash
source scripts/dev_env.sh
```

```powershell
. .\scripts\dev_env.ps1
```

The helpers set `CARGO_TARGET_DIR` to a shared Shape Lab cache, configure
`SCCACHE_DIR`, and set `RUSTC_WRAPPER=sccache` only when `sccache` is available.
They do not make `sccache` a project dependency.

A shared `CARGO_TARGET_DIR` saves disk and avoids rebuilding the same crates in
every worktree. The tradeoff is target-lock contention when many agents build at
the same time. For heavy parallel work, prefer per-worktree `target` directories
with shared `sccache`.

## Target Cleanup

List target directories:

```bash
python3 scripts/clean_targets.py --list --root /Users/arman/Desktop
```

Preview deletion of stale inactive targets:

```bash
python3 scripts/clean_targets.py --root /Users/arman/Desktop --older-than-days 7 --dry-run
```

Delete stale inactive targets:

```bash
python3 scripts/clean_targets.py --root /Users/arman/Desktop --older-than-days 7 --delete
```

The cleanup script detects active git worktrees and refuses to delete active
worktree targets unless `--include-active` is passed.

## Gate Selection

Use `scripts/dev_gate.py` instead of choosing slow commands ad hoc:

```bash
python3 scripts/dev_gate.py --tier inner --changed
python3 scripts/dev_gate.py --tier branch --changed
python3 scripts/dev_gate.py --tier branch --changed --run
python3 scripts/dev_gate.py --tier integration
python3 scripts/dev_gate.py --tier release
```

By default the script prints commands. It only executes them with `--run`.

For a clean feature branch, `--changed` falls back from worktree changes to
`origin/main...HEAD`, so committed branch work still maps to relevant gates.

## Optional Nextest

`.config/nextest.toml` defines optional `quick`, `branch`, `integration`, and
`heavy` profiles. `cargo-nextest` is not required. Use it when installed:

```bash
cargo nextest run --workspace -P quick
cargo nextest run --workspace -P branch
```

Doctests are not covered by nextest, so keep release gates on Cargo unless a
separate doctest command is added.
