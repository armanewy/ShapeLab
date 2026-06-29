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


def cargo(*args: str) -> Command:
    return ("cargo", *args)


CATALOG_FOUNDRY_ADJACENCY_TESTS: tuple[Command, ...] = (
    cargo("test", "-p", "shape-search", "foundry", "--jobs", "1"),
    cargo("test", "-p", "shape-render", "foundry", "--jobs", "1"),
)

BUILD_PROFILE_RELEASE_EXPORT_PATHS: tuple[str, ...] = (
    ".cargo/",
    "packaging/",
    "crates/shape-app/Cargo.toml",
    "crates/shape-cli/Cargo.toml",
    "crates/shape-cli/src/game_ready_static.rs",
    "crates/shape-compile/src/export/",
    "scripts/package_macos_app.sh",
    "scripts/run_shape_app.ps1",
)

STATIC_SURFACE_PACKAGE_PATHS: tuple[str, ...] = (
    "crates/shape-cli/src/game_ready_static.rs",
    "crates/shape-gamekit/src/surface.rs",
    "crates/shape-render/src/surface_preview.rs",
)

PRODUCT_CODE_PATHS: tuple[str, ...] = (
    "crates/",
    "packaging/",
)


@dataclass(frozen=True)
class GatePlan:
    """Commands selected for a gate tier."""

    tier: str
    changed_paths: tuple[str, ...]
    commands: tuple[Command, ...]


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


def is_catalog_test_path(path: str, test_name: str) -> bool:
    return path in {
        f"crates/shape-foundry-catalog/src/{test_name}.rs",
        f"crates/shape-foundry-catalog/tests/{test_name}.rs",
    }


def touches_static_surface_package(path: str) -> bool:
    return path in STATIC_SURFACE_PACKAGE_PATHS or path.startswith(
        "crates/shape-gamekit/src/surface/"
    )


def touches_branch_release_stack(path: str) -> bool:
    return (
        path in {"Cargo.toml", "Cargo.lock"}
        or any(path.startswith(prefix) for prefix in BUILD_PROFILE_RELEASE_EXPORT_PATHS)
    )


def touches_product_code(path: str) -> bool:
    return (
        path in {"Cargo.toml", "Cargo.lock"}
        or any(path.startswith(prefix) for prefix in PRODUCT_CODE_PATHS)
    )


def commands_for_paths(paths: Iterable[str], tier: str) -> tuple[Command, ...]:
    commands: list[Command] = [] if tier == "inner" else [FMT]
    path_set = set(paths)
    static_surface_package_changed = any(touches_static_surface_package(path) for path in path_set)
    branch_release_stack_changed = any(touches_branch_release_stack(path) for path in path_set)

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

        for catalog_test in ("simple_crate", "utility_crate", "cargo_case"):
            if is_catalog_test_path(path, catalog_test):
                commands.extend(
                    [
                        cargo(
                            "test",
                            "-p",
                            "shape-foundry-catalog",
                            "--test",
                            catalog_test,
                            "--jobs",
                            "1",
                        ),
                        *CATALOG_FOUNDRY_ADJACENCY_TESTS,
                    ]
                )

        if is_catalog_test_path(path, "scifi_crate"):
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
            if static_surface_package_changed:
                commands.append(cargo("test", "-p", "shape-cli", "game_ready_static", "--jobs", "1"))

        if is_catalog_test_path(path, "roman_bridge"):
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

        if is_catalog_test_path(path, "stylized_lamp"):
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

    if tier == "branch" and branch_release_stack_changed:
        commands.append(cargo("build", "--release", "--workspace"))

    return dedupe(commands)


def integration_commands(paths: Iterable[str]) -> tuple[Command, ...]:
    path_tuple = tuple(paths)
    commands: list[Command] = [
        FMT,
        cargo("test", "-p", "shape-app", "--lib", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-search", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-render", "foundry", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "scifi_crate", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "roman_bridge", "--jobs", "1"),
        cargo("test", "-p", "shape-foundry-catalog", "--test", "stylized_lamp", "--jobs", "1"),
        cargo("clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
    ]
    if not path_tuple or any(touches_product_code(path) for path in path_tuple):
        commands.append(cargo("build", "--release", "--workspace"))
    return tuple(commands)


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
        commands = integration_commands(path_tuple)
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
