#!/usr/bin/env python3
"""Check source and report files for physically audit-friendly formatting."""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


APP_SOURCE = "crates/shape-app/src/foundry/app.rs"

DEFAULT_FILES = (
    APP_SOURCE,
    "crates/shape-foundry-catalog/src/box_primitive.rs",
    "crates/shape-foundry-catalog/src/lib.rs",
    "crates/shape-foundry-catalog/src/kits.rs",
    "README.md",
    "docs/CURRENT_PRODUCT_STATUS.md",
    "docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md",
    "docs/KNOWN_LIMITATIONS.md",
)

DEFAULT_MARKDOWN_REPORTS = frozenset(
    {
        "docs/CURRENT_PRODUCT_STATUS.md",
        "docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md",
        "docs/KNOWN_LIMITATIONS.md",
    }
)

IMPORTANT_RUST_SOURCES = frozenset(
    path for path in DEFAULT_FILES if path.endswith(".rs")
)

APP_LOGIC_MARKERS = (
    "struct FoundryDesktopApp",
    "impl eframe::App for FoundryDesktopApp",
    "enum MakeCanvasMode",
    "show_make",
    "MakeMaterialLookState",
)


@dataclass(frozen=True)
class FileMetrics:
    path: str
    physical_line_count: int
    max_line_length: int
    lines_over_180: int
    lines_over_220: int
    non_code_markdown_lines_over_220: int
    likely_collapsed: bool
    failures: tuple[str, ...]


def repo_root(start: Path | None = None) -> Path:
    """Return the git repository root, falling back to this script's parent."""

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


def relative_path(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def is_markdown_report(path: str) -> bool:
    return path in DEFAULT_MARKDOWN_REPORTS


def is_important_rust_source(path: str) -> bool:
    return path in IMPORTANT_RUST_SOURCES


def has_major_app_logic(path: str, text: str) -> bool:
    if path != APP_SOURCE:
        return False
    return sum(1 for marker in APP_LOGIC_MARKERS if marker in text) >= 2


def non_code_line_lengths(path: str, lines: list[str]) -> list[int]:
    if not path.endswith(".md"):
        return [len(line) for line in lines]

    lengths: list[int] = []
    in_fenced_block = False
    for line in lines:
        stripped = line.lstrip()
        is_fence = stripped.startswith("```") or stripped.startswith("~~~")
        if is_fence:
            in_fenced_block = not in_fenced_block
            continue
        if not in_fenced_block:
            lengths.append(len(line))
    return lengths


def measure_file(path: Path, root: Path) -> FileMetrics:
    rel_path = relative_path(path, root)
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    line_lengths = [len(line) for line in lines]
    non_code_lengths = non_code_line_lengths(rel_path, lines)

    physical_line_count = len(lines)
    max_line_length = max(line_lengths, default=0)
    lines_over_180 = sum(1 for length in line_lengths if length > 180)
    lines_over_220 = sum(1 for length in line_lengths if length > 220)
    non_code_markdown_lines_over_220 = (
        sum(1 for length in non_code_lengths if length > 220)
        if rel_path.endswith(".md")
        else 0
    )

    failures: list[str] = []
    if rel_path == APP_SOURCE and physical_line_count < 500 and has_major_app_logic(rel_path, text):
        failures.append("app.rs has major app logic but fewer than 500 physical lines")
    if is_markdown_report(rel_path) and physical_line_count < 20:
        failures.append("Markdown report has fewer than 20 physical lines")
    if non_code_markdown_lines_over_220:
        failures.append("non-code-block line exceeds 220 characters")
    if is_important_rust_source(rel_path) and lines_over_220 > 5:
        failures.append("important Rust source has more than 5 lines over 220 characters")

    likely_collapsed = (
        (physical_line_count <= 5 and max_line_length > 500)
        or max_line_length > 1000
        or non_code_markdown_lines_over_220 > 0
        or (is_important_rust_source(rel_path) and lines_over_220 > 5)
        or (
            rel_path == APP_SOURCE
            and physical_line_count < 500
            and has_major_app_logic(rel_path, text)
        )
        or (is_markdown_report(rel_path) and physical_line_count < 20)
    )

    return FileMetrics(
        path=rel_path,
        physical_line_count=physical_line_count,
        max_line_length=max_line_length,
        lines_over_180=lines_over_180,
        lines_over_220=lines_over_220,
        non_code_markdown_lines_over_220=non_code_markdown_lines_over_220,
        likely_collapsed=likely_collapsed,
        failures=tuple(failures),
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Report physical line counts and long-line hygiene for source and "
            "Markdown report files."
        )
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Files to check. Defaults to the Box Primitive source/report hygiene list.",
    )
    parser.add_argument(
        "--extra-file",
        action="append",
        default=[],
        help="Additional file to check alongside the default list.",
    )
    parser.add_argument(
        "--list-defaults",
        action="store_true",
        help="Print the default file list and exit.",
    )
    return parser.parse_args()


def selected_files(args: argparse.Namespace) -> tuple[str, ...]:
    if args.files:
        files = list(args.files)
    else:
        files = list(DEFAULT_FILES)
    files.extend(args.extra_file)
    return tuple(dict.fromkeys(files))


def print_metrics(metrics: list[FileMetrics]) -> None:
    headers = (
        "File",
        "Physical lines",
        "Max line",
        "Lines >180",
        "Lines >220",
        "Likely collapsed",
    )
    rows = [
        (
            metric.path,
            str(metric.physical_line_count),
            str(metric.max_line_length),
            str(metric.lines_over_180),
            str(metric.lines_over_220),
            "yes" if metric.likely_collapsed else "no",
        )
        for metric in metrics
    ]
    widths = [
        max(len(row[index]) for row in (headers, *rows))
        for index in range(len(headers))
    ]
    print(
        "  ".join(
            header.ljust(widths[index]) for index, header in enumerate(headers)
        )
    )
    print("  ".join("-" * width for width in widths))
    for row in rows:
        print(
            "  ".join(
                value.ljust(widths[index]) for index, value in enumerate(row)
            )
        )


def main() -> int:
    args = parse_args()
    root = repo_root()

    if args.list_defaults:
        for path in DEFAULT_FILES:
            print(path)
        return 0

    metrics: list[FileMetrics] = []
    missing: list[str] = []
    for file_name in selected_files(args):
        path = Path(file_name)
        if not path.is_absolute():
            path = root / path
        if path.is_file():
            metrics.append(measure_file(path, root))
        else:
            missing.append(file_name)

    if metrics:
        print_metrics(metrics)

    failed = False
    if missing:
        failed = True
        print("\nMissing files:", file=sys.stderr)
        for path in missing:
            print(f"- {path}", file=sys.stderr)

    failing_metrics = [metric for metric in metrics if metric.failures]
    if failing_metrics:
        failed = True
        print("\nHygiene failures:", file=sys.stderr)
        for metric in failing_metrics:
            for failure in metric.failures:
                print(f"- {metric.path}: {failure}", file=sys.stderr)

    if failed:
        return 1

    print("\nSource hygiene check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
