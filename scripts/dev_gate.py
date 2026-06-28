#!/usr/bin/env python3
"""Select fast Shape Lab development gates from changed paths."""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


Command = tuple[str, ...]

FMT: Command = ("cargo", "fmt", "--all", "--check")
PYTHON: str = os.environ.get("PYTHON", sys.executable or "python3")


@dataclass(frozen=True)
class GatePlan:
    """Commands selected for a gate tier."""

    tier: str
    changed_paths: tuple[str, ...]
    commands: tuple[Command, ...]


def cargo(*args: str) -> Command:
    return ("cargo", *args)


def repo_root(start: Path | None = None) -> Path:
    """Return the git repository root, falling back to the script parent."""

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
    return Path(__file__).resolve().parents[1]


def git_lines(args: list[str], cwd: Path) -> list[str]:
    try:
        output = subprocess.check_output(
            ["git", *args],
            cwd=cwd,
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []
    return [line.strip() for line in output.splitlines() if line.strip()]


def changed_paths(cwd: Path, base: str | None) -> list[str]:
    """Return changed paths from the worktree and, when useful, a base ref."""

    paths: set[str] = set(git_lines(["diff", "--name-only", "HEAD"], cwd))
    paths.update(git_lines(["ls-files", "--others", "--exclude-standard"], cwd))
    if base:
        paths.update(git_lines(["diff", "--name-only", f"{base}...HEAD"], cwd))
    elif not paths:
        paths.update(git_lines(["diff", "--name-only", "origin/main...HEAD"], cwd))
    return sorted(paths)


def dedupe(commands: Iterable[Command]) -> tuple[Command, ...]:
    seen: set[Command] = set()
    result: list[Command] = []
    for command in commands:
        if command in seen:
            continue
        seen.add(command)
        result.append(command)
    return tuple(result)


def rust_crate_from_path(path: str) -> str | None:
    parts = Path(path).parts
    if len(parts) >= 2 and parts[0] == "crates":
        return parts[1]
    return None


def docs_status_commands() -> list[Command]:
    return [
        cargo(
            "test",
            "-p",
            "shape-app",
            "product_truth_docs_agree_on_no_go_status_and_roman_preview_only",
            "--lib",
            "--jobs",
            "1",
        ),
        cargo(
            "test",
            "-p",
            "shape-app",
            "source_and_markdown_hygiene_targets_are_audit_friendly",
            "--lib",
            "--jobs",
            "1",
        ),
    ]


def commands_for_paths(paths: Iterable[str], tier: str) -> tuple[Command, ...]:
    commands: list[Command] = [FMT]
    path_set = set(paths)

    for path in sorted(path_set):
        crate = rust_crate_from_path(path)

        if path.startswith("docs/") or path == "README.md":
            commands.extend(docs_status_commands())

        if path == "Cargo.toml" or path == "Cargo.lock":
            commands.append(cargo("check", "--workspace"))

        if path.startswith("scripts/"):
            commands.append((PYTHON, "scripts/test_dev_speed.py"))

        if path.startswith("crates/shape-app/"):
            commands.extend(
                [
                    cargo("check", "-p", "shape-app"),
                    cargo("test", "-p", "shape-app", "--lib", "foundry", "--jobs", "1"),
                ]
            )
            if tier != "inner":
                commands.extend(
                    [
                        cargo(
                            "test",
                            "-p",
                            "shape-app",
                            "--test",
                            "foundry_direction_board",
                            "--jobs",
                            "1",
                            "--",
                            "--skip",
                            "foundry::",
                        ),
                        cargo(
                            "clippy",
                            "-p",
                            "shape-app",
                            "--all-targets",
                            "--",
                            "-D",
                            "warnings",
                        ),
                    ]
                )

        if path.startswith("crates/shape-search/"):
            commands.append(cargo("test", "-p", "shape-search", "foundry", "--jobs", "1"))
            if tier != "inner":
                commands.append(cargo("test", "-p", "shape-render", "foundry", "--jobs", "1"))

        if path.startswith("crates/shape-render/"):
            commands.append(cargo("test", "-p", "shape-render", "foundry", "--jobs", "1"))
            if "surface" in path:
                commands.append(cargo("test", "-p", "shape-render", "surface", "--jobs", "1"))

        if path in {
            "crates/shape-foundry-catalog/src/scifi_crate.rs",
            "crates/shape-foundry-catalog/tests/scifi_crate.rs",
        }:
            commands.extend(
                [
                    cargo(
                        "test",
                        "-p",
                        "shape-foundry-catalog",
                        "--test",
                        "scifi_crate",
                        "--jobs",
                        "1",
                    ),
                    cargo("test", "-p", "shape-search", "foundry", "--jobs", "1"),
                ]
            )

        if path in {
            "crates/shape-foundry-catalog/src/roman_bridge.rs",
            "crates/shape-foundry-catalog/tests/roman_bridge.rs",
        }:
            commands.append(
                cargo(
                    "test",
                    "-p",
                    "shape-foundry-catalog",
                    "--test",
                    "roman_bridge",
                    "--jobs",
                    "1",
                )
            )

        if path in {
            "crates/shape-foundry-catalog/src/stylized_lamp.rs",
            "crates/shape-foundry-catalog/tests/stylized_lamp.rs",
        }:
            commands.append(
                cargo(
                    "test",
                    "-p",
                    "shape-foundry-catalog",
                    "--test",
                    "stylized_lamp",
                    "--jobs",
                    "1",
                )
            )

        if path.startswith("crates/shape-gamekit/"):
            commands.extend(
                [
                    cargo("test", "-p", "shape-gamekit", "surface", "--jobs", "1"),
                    cargo("test", "-p", "shape-gamekit", "rig", "--jobs", "1"),
                    cargo("test", "-p", "shape-gamekit", "motion", "--jobs", "1"),
                ]
            )

        if path == "crates/shape-cli/src/game_ready_static.rs":
            commands.append(cargo("test", "-p", "shape-cli", "game_ready_static", "--jobs", "1"))

        if crate and crate not in {
            "shape-app",
            "shape-search",
            "shape-render",
            "shape-foundry-catalog",
            "shape-gamekit",
            "shape-cli",
        }:
            commands.append(cargo("check", "-p", crate))

    return dedupe(commands)


def integration_commands() -> tuple[Command, ...]:
    return (
        FMT,
        cargo("test", "-p", "shape-app", "--lib", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-search", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-render", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "scifi_crate", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "roman_bridge", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "stylized_lamp", "--jobs", "1"),
        cargo("clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
        cargo("build", "--release", "--workspace"),
    )


def release_commands() -> tuple[Command, ...]:
    return (
        FMT,
        cargo("test", "--workspace", "--no-fail-fast"),
        cargo("clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
        cargo("build", "--release", "--workspace"),
    )


def select_plan(tier: str, paths: Iterable[str]) -> GatePlan:
    path_tuple = tuple(sorted(set(paths)))
    if tier == "integration":
        commands = integration_commands()
    elif tier == "release":
        commands = release_commands()
    elif path_tuple:
        commands = commands_for_paths(path_tuple, tier)
    else:
        commands = (FMT,)
    return GatePlan(tier=tier, changed_paths=path_tuple, commands=commands)


def print_plan(plan: GatePlan) -> None:
    if plan.changed_paths:
        print("Changed paths:")
        for path in plan.changed_paths:
            print(f"  {path}")
    else:
        print("Changed paths: none detected")
    print(f"Tier: {plan.tier}")
    print("Commands:")
    for command in plan.commands:
        print(f"  {shlex.join(command)}")


def run_plan(plan: GatePlan, cwd: Path) -> int:
    for command in plan.commands:
        print(f"+ {shlex.join(command)}", flush=True)
        result = subprocess.run(command, cwd=cwd)
        if result.returncode != 0:
            return result.returncode
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tier", choices=["inner", "branch", "integration", "release"], default="inner")
    parser.add_argument("--changed", action="store_true", help="select gates from git-changed paths")
    parser.add_argument("--base", help="base ref for --changed, for example origin/main")
    parser.add_argument(
        "--path",
        action="append",
        default=[],
        help="explicit changed path; may be passed more than once",
    )
    parser.add_argument("--run", action="store_true", help="execute selected commands")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    root = repo_root()
    paths = list(args.path)
    if args.changed:
        paths.extend(changed_paths(root, args.base))
    plan = select_plan(args.tier, paths)
    print_plan(plan)
    if args.run:
        return run_plan(plan, root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
