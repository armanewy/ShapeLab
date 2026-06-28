#!/usr/bin/env python3
"""List and optionally remove stale Cargo target directories."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class TargetInfo:
    path: Path
    size_bytes: int
    modified_at: float
    active: bool


def human_size(size: int) -> str:
    units = ["B", "KiB", "MiB", "GiB", "TiB"]
    value = float(size)
    for unit in units:
        if value < 1024 or unit == units[-1]:
            return f"{value:.1f} {unit}" if unit != "B" else f"{int(value)} B"
        value /= 1024
    return f"{size} B"


def repo_root(start: Path | None = None) -> Path:
    start = start or Path.cwd()
    try:
        output = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        if output:
            return Path(output)
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    return start


def active_worktrees(cwd: Path) -> set[Path]:
    try:
        output = subprocess.check_output(
            ["git", "worktree", "list", "--porcelain"],
            cwd=cwd,
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return set()
    roots: set[Path] = set()
    for line in output.splitlines():
        if line.startswith("worktree "):
            roots.add(Path(line.removeprefix("worktree ")).resolve())
    return roots


def find_target_dirs(root: Path) -> list[Path]:
    targets: list[Path] = []
    for current, dirs, _files in os.walk(root):
        if "target" in dirs:
            target = Path(current) / "target"
            targets.append(target)
            dirs.remove("target")
        ignored = {".git", ".hg", ".svn", "node_modules"}
        dirs[:] = [directory for directory in dirs if directory not in ignored]
    return sorted(targets)


def size_and_mtime(path: Path) -> tuple[int, float]:
    total = 0
    newest = path.stat().st_mtime
    for current, dirs, files in os.walk(path):
        for name in files:
            file_path = Path(current) / name
            try:
                stat = file_path.stat()
            except OSError:
                continue
            total += stat.st_size
            newest = max(newest, stat.st_mtime)
        for name in dirs:
            dir_path = Path(current) / name
            try:
                newest = max(newest, dir_path.stat().st_mtime)
            except OSError:
                continue
    return total, newest


def collect_targets(root: Path, active_roots: set[Path]) -> list[TargetInfo]:
    infos: list[TargetInfo] = []
    for target in find_target_dirs(root):
        resolved = target.resolve()
        active = any(resolved == active_root / "target" for active_root in active_roots)
        size, modified_at = size_and_mtime(resolved)
        infos.append(TargetInfo(resolved, size, modified_at, active))
    return infos


def is_old_enough(info: TargetInfo, older_than_days: int | None, now: float | None = None) -> bool:
    if older_than_days is None:
        return True
    now = now or time.time()
    age_seconds = now - info.modified_at
    return age_seconds >= older_than_days * 24 * 60 * 60


def deletion_candidates(
    targets: list[TargetInfo],
    older_than_days: int | None,
    include_active: bool,
) -> list[TargetInfo]:
    return [
        info
        for info in targets
        if is_old_enough(info, older_than_days) and (include_active or not info.active)
    ]


def print_targets(targets: list[TargetInfo], older_than_days: int | None) -> None:
    if not targets:
        print("No target directories found.")
        return
    print("Target directories:")
    for info in targets:
        age_days = max(0.0, (time.time() - info.modified_at) / (24 * 60 * 60))
        eligible = is_old_enough(info, older_than_days)
        active = "active" if info.active else "inactive"
        status = "eligible" if eligible else "too-new"
        print(
            f"  {info.path} | {human_size(info.size_bytes)} | "
            f"{age_days:.1f} days old | {active} | {status}"
        )


def delete_targets(targets: list[TargetInfo], dry_run: bool) -> None:
    for info in targets:
        action = "Would remove" if dry_run else "Removing"
        print(f"{action}: {info.path} ({human_size(info.size_bytes)})")
        if not dry_run:
            shutil.rmtree(info.path)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="list target directories")
    parser.add_argument("--dry-run", action="store_true", help="show deletions without removing")
    parser.add_argument("--delete", action="store_true", help="delete eligible target directories")
    parser.add_argument("--root", default=".", help="root to scan; default is current directory")
    parser.add_argument("--older-than-days", type=int, help="only delete targets older than N days")
    parser.add_argument(
        "--include-active",
        action="store_true",
        help="allow active git worktree target directories to be deleted",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    root = Path(args.root).expanduser().resolve()
    git_root = repo_root(Path.cwd())
    active_roots = active_worktrees(git_root)
    targets = collect_targets(root, active_roots)
    if args.list or (not args.delete and not args.dry_run):
        print_targets(targets, args.older_than_days)
    if args.delete or args.dry_run:
        candidates = deletion_candidates(targets, args.older_than_days, args.include_active)
        skipped_active = [
            info
            for info in targets
            if info.active and is_old_enough(info, args.older_than_days) and not args.include_active
        ]
        if skipped_active:
            print("Refusing active worktree targets without --include-active:")
            for info in skipped_active:
                print(f"  {info.path}")
        delete_targets(candidates, dry_run=args.dry_run or not args.delete)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
