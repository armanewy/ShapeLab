#!/usr/bin/env python3
"""Apply a narrow Box Primitive UI cleanup.

This script is intentionally a short-lived codemod for the current Shape Lab
UI state.  The current app source is large and has gone through several
generation passes; this script performs explicit, guarded replacements rather
than asking a future agent to hand-edit long UI blocks.

It makes Box Primitive the only novice-facing baseline:

* removes category filter chips from the Choose screen;
* removes Focus Part / Body chip affordances for Box Primitive;
* hides Material Looks surfaces for Box Primitive, because they are out of
  scope for the baseline;
* updates the focused app test expectations to match the simpler baseline.

Run from the repository root, then run `cargo fmt --all`.
"""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"Pattern not found in {path}: {old[:120]!r}")
    text = text.replace(old, new, 1)
    path.write_text(text, encoding="utf-8")


def replace_all(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"Pattern not found in {path}: {old[:120]!r}")
    text = text.replace(old, new)
    path.write_text(text, encoding="utf-8")


def main() -> None:
    app = ROOT / "crates/shape-app/src/foundry/app.rs"
    directions = ROOT / "crates/shape-app/src/foundry/panels/directions.rs"

    # 1. The current product is intentionally only Box Primitive.  Category
    # chips such as Architecture/Gear/Furniture imply a catalog breadth that
    # no longer exists and create visual noise on the Choose screen.
    replace_once(
        app,
        """const HOME_TEMPLATE_FILTERS: [HomeTemplateFilter; 6] = [
    HomeTemplateFilter::All,
    HomeTemplateFilter::Props,
    HomeTemplateFilter::Architecture,
    HomeTemplateFilter::Gear,
    HomeTemplateFilter::Furniture,
    HomeTemplateFilter::Environment,
];""",
        "const HOME_TEMPLATE_FILTERS: [HomeTemplateFilter; 0] = [];",
    )

    replace_all(
        app,
        'const HOME_SUBTITLE: &str = "Start with an asset template, then make clear whole-asset ideas.";',
        'const HOME_SUBTITLE: &str = "Start with the Box Primitive baseline, then make clear whole-box ideas.";',
    )
    replace_all(
        app,
        '"Choose a template below to start a new project."',
        '"Choose the Box Primitive starting point below."',
    )
    replace_all(app, '"Search assets..."', '"Search starting point..."')
    replace_all(app, '"No matching templates"', '"No matching starting point"')

    # 2. Box Primitive has no meaningful part focus yet.  A single Body chip
    # reads like a broken selector and makes users ask why a box has parts.
    replace_once(
        directions,
        """let profile_hint = format!(
        "{} {}",
        document.family_content_ref.stable_id, document.customizer_profile_ref.stable_id
    );
    built_in_part_group_descriptors_for_profile(&profile_hint)
        .into_iter()
        .map(|descriptor| DirectionPartGroup {
            group_id: descriptor.group_id,
            label: descriptor.display_name,
            focusable: descriptor.focusable && descriptor.capability.shape_ready,
            unavailable_reason: descriptor.capability.unavailable_reasons.first().cloned(),
        })
        .collect()""",
        """let profile_hint = format!(
        "{} {}",
        document.family_content_ref.stable_id, document.customizer_profile_ref.stable_id
    );
    let normalized = profile_hint.replace('_', "-").to_ascii_lowercase();
    if normalized.contains("box-primitive") {
        return Vec::new();
    }
    built_in_part_group_descriptors_for_profile(&profile_hint)
        .into_iter()
        .map(|descriptor| DirectionPartGroup {
            group_id: descriptor.group_id,
            label: descriptor.display_name,
            focusable: descriptor.focusable && descriptor.capability.shape_ready,
            unavailable_reason: descriptor.capability.unavailable_reasons.first().cloned(),
        })
        .collect()""",
    )

    # 3. Material Looks are not part of the Box Primitive baseline.  Do not show
    # a large blocked material panel in Make, even if a screenshot scenario or
    # old state toggled the material tray open.
    replace_once(
        app,
        "let material_look_tray_visible = self.material_looks.tray_open;",
        "let material_look_tray_visible = self.material_looks.tray_open && !box_primitive_baseline;",
    )

    # 4. Update the focused unit test that previously expected a Body group.
    replace_once(
        app,
        """let groups = app
            .state
            .document
            .as_ref()
            .map(directions::direction_part_groups_for_document)
            .expect("box document has direction groups");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group_id, "body");""",
        """let groups = app
            .state
            .document
            .as_ref()
            .map(directions::direction_part_groups_for_document)
            .expect("box document has direction groups");
        assert!(
            groups.is_empty(),
            "Box Primitive should not expose part focus chips"
        );""",
    )

    # 5. The material look tray is now forcibly hidden for the Box baseline;
    # keep the existing action-hidden assertion and add a view-state assertion.
    replace_once(
        app,
        "assert!(!app.material_look_action_visible(&ready));",
        "assert!(!app.material_look_action_visible(&ready)); assert!(!ready.material_look_tray_visible);",
    )

    print("Applied Box Primitive UI cleanup codemod.")


if __name__ == "__main__":
    main()
