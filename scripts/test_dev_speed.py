#!/usr/bin/env python3
"""Unit tests for development-speed helper scripts."""

from __future__ import annotations

import tempfile
import time
import unittest
from pathlib import Path

import clean_targets
import dev_gate


class DevGateTests(unittest.TestCase):
    def command_strings(self, paths: list[str], tier: str = "branch") -> list[str]:
        plan = dev_gate.select_plan(tier, paths)
        return [" ".join(command) for command in plan.commands]

    def test_shape_app_branch_gate_includes_adjacent_app_checks(self) -> None:
        commands = self.command_strings(["crates/shape-app/src/foundry/app.rs"])
        self.assertIn("cargo check -p shape-app", commands)
        self.assertIn("cargo test -p shape-app --lib foundry --jobs 1", commands)
        self.assertIn(
            "cargo test -p shape-app --test foundry_direction_board --jobs 1 -- --skip foundry::",
            commands,
        )
        self.assertIn("cargo clippy -p shape-app --all-targets -- -D warnings", commands)

    def test_box_catalog_gate_includes_foundry_adjacency(self) -> None:
        commands = self.command_strings(["crates/shape-foundry-catalog/src/box_primitive.rs"])
        self.assertIn(
            "cargo test -p shape-foundry-catalog --test box_primitive --jobs 1",
            commands,
        )
        self.assertIn("cargo test -p shape-foundry --test contracts --jobs 1", commands)

    def test_docs_gate_avoids_release_build(self) -> None:
        commands = self.command_strings(["docs/CURRENT_PRODUCT_STATUS.md"])
        joined = "\n".join(commands)
        self.assertIn("cargo fmt --all --check", joined)
        self.assertNotIn("cargo build --release --workspace", joined)

    def test_release_tier_is_explicit_full_gate(self) -> None:
        plan = dev_gate.select_plan("release", [])
        commands = [" ".join(command) for command in plan.commands]
        self.assertIn("cargo test --workspace --no-fail-fast", commands)
        self.assertIn("cargo build --release --workspace", commands)


class CleanTargetsTests(unittest.TestCase):
    def test_dry_run_candidates_exclude_active_by_default(self) -> None:
        now = time.time()
        active = clean_targets.TargetInfo(
            Path("/repo/target"),
            size_bytes=10,
            modified_at=now - 9 * 24 * 60 * 60,
            active=True,
        )
        inactive = clean_targets.TargetInfo(
            Path("/old/target"),
            size_bytes=10,
            modified_at=now - 9 * 24 * 60 * 60,
            active=False,
        )
        candidates = clean_targets.deletion_candidates([active, inactive], 7, include_active=False)
        self.assertEqual(candidates, [inactive])

    def test_include_active_allows_active_candidate(self) -> None:
        info = clean_targets.TargetInfo(
            Path("/repo/target"),
            size_bytes=10,
            modified_at=time.time() - 9 * 24 * 60 * 60,
            active=True,
        )
        self.assertEqual(
            clean_targets.deletion_candidates([info], 7, include_active=True),
            [info],
        )

    def test_collect_targets_marks_active_worktree_target(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            worktree = root / "ShapeLab"
            target = worktree / "target"
            target.mkdir(parents=True)
            (target / "artifact").write_text("x", encoding="utf-8")
            infos = clean_targets.collect_targets(root, {worktree.resolve()})
            self.assertEqual(len(infos), 1)
            self.assertTrue(infos[0].active)


if __name__ == "__main__":
    unittest.main()
