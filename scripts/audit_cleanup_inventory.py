#!/usr/bin/env python3
"""Report cleanup inventory findings for docs, code, and product-claim strings."""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


OBSOLETE_TERMS = (
    "Sci-Fi Crate",
    "Cargo Case",
    "Crate family",
    "generated variations",
    "Try ideas",
    "candidate tray",
    "Shape Lab",
)

COMMENT_TERMS = (
    "TODO",
    "FIXME",
    "deprecated",
    "legacy",
)

PRODUCT_CLAIM_TERMS = (
    "game-ready",
    "Godot-ready",
    "textured",
    "rigged",
    "animated",
    "collision-enabled",
    "public catalog",
)

TEXT_SUFFIXES = {
    ".md",
    ".rs",
    ".toml",
    ".json",
    ".py",
    ".sh",
    ".yml",
    ".yaml",
}

IGNORED_DIRS = {
    ".git",
    "target",
}


@dataclass(frozen=True)
class Finding:
    category: str
    path: str
    line: int
    text: str


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


def tracked_text_files(root: Path) -> list[Path]:
    try:
        output = subprocess.check_output(
            ["git", "ls-files"],
            cwd=root,
            text=True,
        )
        candidates = [root / line for line in output.splitlines()]
    except (FileNotFoundError, subprocess.CalledProcessError):
        candidates = [path for path in root.rglob("*") if path.is_file()]

    files: list[Path] = []
    for path in candidates:
        rel_parts = path.relative_to(root).parts
        if any(part in IGNORED_DIRS for part in rel_parts):
            continue
        if path.suffix in TEXT_SUFFIXES:
            files.append(path)
    return sorted(files)


def cargo_metadata(root: Path) -> dict:
    try:
        output = subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=root,
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return {}
    return json.loads(output)


def workspace_crate_usage(root: Path) -> list[str]:
    metadata = cargo_metadata(root)
    packages = metadata.get("packages", [])
    members = {package["name"] for package in packages}
    referenced: set[str] = set()
    for package in packages:
        for dependency in package.get("dependencies", []):
            name = dependency.get("rename") or dependency.get("name")
            if name in members:
                referenced.add(name)
    roots = {"orchard-app", "orchard-cli"}
    unused = sorted(members - referenced - roots)
    return unused


def find_terms(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    term_groups = [
        ("obsolete", OBSOLETE_TERMS),
        ("comment", COMMENT_TERMS),
        ("product-claim", PRODUCT_CLAIM_TERMS),
    ]
    for path in tracked_text_files(root):
        rel = path.relative_to(root).as_posix()
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except FileNotFoundError:
            continue
        except UnicodeDecodeError:
            continue
        for line_number, line in enumerate(lines, start=1):
            for category, terms in term_groups:
                for term in terms:
                    if term.lower() in line.lower():
                        findings.append(Finding(category, rel, line_number, line.strip()))
                        break
    return findings


def rust_oversize_report(root: Path) -> str:
    script = root / "scripts/check_rust_file_size.py"
    if not script.exists():
        return "check_rust_file_size.py is not present yet."
    output = subprocess.run(
        [sys.executable, str(script)],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return output.stdout.strip()


def main() -> int:
    root = repo_root()
    print("Cleanup inventory report")
    print("========================")
    print()
    print("Oversized Rust files")
    print("--------------------")
    print(rust_oversize_report(root))
    print()
    print("Potentially unused workspace crates")
    print("-----------------------------------")
    unused = workspace_crate_usage(root)
    if unused:
        for crate in unused:
            print(f"- {crate}")
    else:
        print("No potentially unused workspace crates detected.")
    print()
    print("Term findings")
    print("-------------")
    findings = find_terms(root)
    if findings:
        for finding in findings:
            print(f"[{finding.category}] {finding.path}:{finding.line}: {finding.text}")
    else:
        print("No cleanup terms found.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
