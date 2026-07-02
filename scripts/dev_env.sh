#!/usr/bin/env bash
set -euo pipefail

cache_root="${OBJECT_ORCHARD_CACHE_DIR:-"$HOME/Library/Caches/ObjectOrchard"}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-"$cache_root/cargo-target"}"
export SCCACHE_DIR="${SCCACHE_DIR:-"$cache_root/sccache"}"
export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-50G}"

mkdir -p "$CARGO_TARGET_DIR" "$SCCACHE_DIR"

if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
  sccache --start-server >/dev/null 2>&1 || true
  sccache_status="found: using RUSTC_WRAPPER=$RUSTC_WRAPPER"
else
  sccache_status="not found: install with 'brew install sccache' to enable compiler caching"
fi

cat <<EOF
Object Orchard development environment
  CARGO_TARGET_DIR=$CARGO_TARGET_DIR
  SCCACHE_DIR=$SCCACHE_DIR
  SCCACHE_CACHE_SIZE=$SCCACHE_CACHE_SIZE
  sccache: $sccache_status

Use this script with:
  source scripts/dev_env.sh

To unset:
  unset CARGO_TARGET_DIR RUSTC_WRAPPER SCCACHE_DIR SCCACHE_CACHE_SIZE

Warning:
  A shared CARGO_TARGET_DIR reduces disk and rebuild time, but parallel Cargo
  builds may contend on target locks. For heavy parallel Codex work, prefer
  per-worktree targets plus RUSTC_WRAPPER=sccache.
EOF
