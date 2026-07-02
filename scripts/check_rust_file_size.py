#!/usr/bin/env python3
"""Fail on oversized Rust source files, with explicit temporary exceptions."""

from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

LIMIT = 1000
EXCEPTIONS_PATH = Path("docs/RUST_FILE_SIZE_EXCEPTIONS.md")
IGNORED_DIRS = {
    ".git",
    "target",
}


@dataclass(frozen=True)
class RustFileSize:
    path: str
    non_test_lines: int
    test_lines: int


def repo_root(start: Path | None = None) -> Path:
    start = start or Path.cwd()
    try:
        output = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start,
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
    except (FileNotFoundError, subprocess.CalledProcessError):
        output = ""
    return Path(output) if output else Path(__file__).resolve().parents[1]


def rust_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for path in root.rglob("*.rs"):
        rel_parts = path.relative_to(root).parts
        if any(part in IGNORED_DIRS for part in rel_parts):
            continue
        files.append(path)
    return sorted(files)


def find_test_module_starts(lines: list[str]) -> set[int]:
    starts: set[int] = set()
    pending_cfg = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("#[cfg(test)]"):
            pending_cfg = True
            continue
        if pending_cfg and (not stripped or stripped.startswith("#[")):
            continue
        if pending_cfg and re.match(r"(pub\s+)?mod\s+tests\b", stripped):
            starts.add(index)
        pending_cfg = False
    return starts


def measure(path: Path, root: Path) -> RustFileSize:
    rel = path.relative_to(root).as_posix()
    lines = path.read_text(encoding="utf-8").splitlines()
    if "/tests/" in f"/{rel}" or rel.endswith("_test.rs"):
        return RustFileSize(rel, 0, len(lines))

    test_starts = find_test_module_starts(lines)
    non_test = 0
    test = 0
    in_test_block = False
    brace_depth = 0

    for index, line in enumerate(lines):
        if index in test_starts and not in_test_block:
            in_test_block = True
            brace_depth = 0

        if in_test_block:
            test += 1
            brace_depth += line.count("{")
            brace_depth -= line.count("}")
            if brace_depth <= 0 and "{" in line:
                in_test_block = False
            continue

        non_test += 1

    return RustFileSize(rel, non_test, test)


def exception_paths(root: Path) -> set[str]:
    path = root / EXCEPTIONS_PATH
    if not path.exists():
        return set()
    paths: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = [cell.strip(" `") for cell in stripped.strip("|").split("|")]
        if not cells or cells[0] in {"Path", "---"}:
            continue
        if cells[0].endswith(".rs"):
            paths.add(cells[0])
    return paths


def main() -> int:
    root = repo_root()
    exceptions = exception_paths(root)
    measurements = [measure(path, root) for path in rust_files(root)]
    oversized = [
        item
        for item in measurements
        if item.non_test_lines > LIMIT and item.path not in exceptions
    ]
    excepted = [
        item
        for item in measurements
        if item.non_test_lines > LIMIT and item.path in exceptions
    ]

    if excepted:
        print("Temporary Rust file size exceptions:")
        for item in excepted:
            print(
                f"  {item.path}: {item.non_test_lines} non-test lines "
                f"({item.test_lines} test lines)"
            )

    if oversized:
        print(f"Rust files over {LIMIT} non-test lines:")
        for item in oversized:
            print(
                f"  {item.path}: {item.non_test_lines} non-test lines "
                f"({item.test_lines} test lines)"
            )
        print(f"Add temporary exceptions to {EXCEPTIONS_PATH} only with owner, plan, and deadline.")
        return 1

    print(f"Rust file size check passed. Limit: {LIMIT} non-test lines.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
