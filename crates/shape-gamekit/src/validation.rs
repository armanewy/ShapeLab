#![forbid(unsafe_code)]

//! Validation for runtime-neutral game asset contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{Frame3, validate_asset_recipe};

use crate::{
    CellBounds, CollisionProxy, ConstructionPhase, ConstructionProfile,
    GAME_ASSET_PACK_SCHEMA_VERSION, GameAssetDefinition, GameAssetPack, LogicalFootprint,
    ModuleSemantics, MonotonicVisibilityPolicy, ReadabilityProfile, SnapAnchor, SnapRelationship,
    SurfaceShape, TriangleBudget, WalkableSurface,
};

/// One validation issue from game asset contract checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameAssetValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation report for game asset contracts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameAssetValidationReport {
    /// Discovered issues.
    pub issues: Vec<GameAssetValidationIssue>,
}

impl GameAssetValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn extend_prefixed(&mut self, prefix: &str, nested: GameAssetValidationReport) {
        for issue in nested.issues {
            self.issues.push(GameAssetValidationIssue {
                subject: issue
                    .subject
                    .map(|subject| format!("{prefix}.{subject}"))
                    .or_else(|| Some(prefix.to_owned())),
                code: issue.code,
                message: issue.message,
            });
        }
    }
}

/// Validate a complete game asset pack.
#[must_use]
pub fn validate_game_asset_pack(pack: &GameAssetPack) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    if pack.schema_version != GAME_ASSET_PACK_SCHEMA_VERSION {
        push_issue(
            &mut report,
            Some("schema_version"),
            "unsupported_game_asset_pack_schema",
            "Game asset pack schema version is not supported.",
        );
    }
    validate_non_empty(&mut report, Some("id"), &pack.id, "empty_pack_id");
    validate_non_empty(&mut report, Some("title"), &pack.title, "empty_pack_title");
    validate_non_empty(
        &mut report,
        Some("source_revision"),
        &pack.source_revision,
        "empty_source_revision",
    );

    let mut runtime_keys = BTreeMap::<&str, usize>::new();
    let mut previous_runtime_key: Option<&str> = None;
    for (index, asset) in pack.assets.iter().enumerate() {
        let runtime_key = asset.module_semantics.runtime_key.as_str();
        if let Some(previous_index) = runtime_keys.insert(runtime_key, index) {
            push_issue(
                &mut report,
                Some(format!("assets.{index}.module_semantics.runtime_key")),
                "duplicate_runtime_key",
                format!("Runtime key is already used by asset index {previous_index}."),
            );
        }
        if let Some(previous) = previous_runtime_key
            && previous > runtime_key
        {
            push_issue(
                &mut report,
                Some(format!("assets.{index}.module_semantics.runtime_key")),
                "asset_order_not_deterministic",
                "Game asset pack assets must be sorted by runtime key.",
            );
        }
        previous_runtime_key = Some(runtime_key);
        report.extend_prefixed(
            &format!("assets.{index}"),
            validate_game_asset_definition(asset),
        );
    }

    report
}

/// Validate one game asset definition.
#[must_use]
pub fn validate_game_asset_definition(
    definition: &GameAssetDefinition,
) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    validate_non_empty(&mut report, Some("id"), &definition.id, "empty_asset_id");
    validate_non_empty(
        &mut report,
        Some("display_name"),
        &definition.display_name,
        "empty_asset_display_name",
    );
    validate_non_empty(
        &mut report,
        Some("family"),
        &definition.family,
        "empty_asset_family",
    );

    let recipe_report = validate_asset_recipe(&definition.source_recipe);
    for issue in recipe_report.issues {
        report.issues.push(GameAssetValidationIssue {
            subject: issue
                .subject
                .map(|subject| format!("source_recipe.{subject}"))
                .or_else(|| Some("source_recipe".to_owned())),
            code: format!("source_recipe_{}", issue.code),
            message: issue.message,
        });
    }

    validate_module_semantics(&definition.module_semantics, &mut report);
    report.extend_prefixed(
        "construction_profile",
        validate_construction_profile(&definition.construction_profile),
    );
    report.extend_prefixed(
        "readability_profile",
        validate_readability_profile(&definition.readability_profile),
    );
    report.extend_prefixed("budgets", validate_triangle_budget(&definition.budgets));
    report
}

fn validate_module_semantics(semantics: &ModuleSemantics, report: &mut GameAssetValidationReport) {
    validate_non_empty(
        report,
        Some("module_semantics.runtime_key"),
        &semantics.runtime_key,
        "empty_runtime_key",
    );
    report.extend_prefixed(
        "module_semantics.logical_footprint",
        validate_logical_footprint(&semantics.logical_footprint),
    );
    report.extend_prefixed(
        "module_semantics.snap_anchors",
        validate_snap_anchors(&semantics.snap_anchors),
    );
    report.extend_prefixed(
        "module_semantics.walkable_surfaces",
        validate_walkable_surfaces(semantics),
    );
    validate_support_surfaces(semantics, report);
    validate_traversal_links(semantics, report);
    validate_collision_proxies(semantics, report);
}

/// Validate an integer logical footprint.
#[must_use]
pub fn validate_logical_footprint(footprint: &LogicalFootprint) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    if footprint.cell_bounds.min[0] > footprint.cell_bounds.max[0]
        || footprint.cell_bounds.min[1] > footprint.cell_bounds.max[1]
    {
        push_issue(
            &mut report,
            Some("cell_bounds"),
            "invalid_footprint_bounds",
            "Logical footprint minimum cell bounds must not exceed maximum bounds.",
        );
    }
    if footprint.vertical_layers.min > footprint.vertical_layers.max {
        push_issue(
            &mut report,
            Some("vertical_layers"),
            "invalid_vertical_layer_bounds",
            "Logical footprint minimum vertical layer must not exceed maximum layer.",
        );
    }
    if !cell_contains(&footprint.cell_bounds, footprint.origin_cell) {
        push_issue(
            &mut report,
            Some("origin_cell"),
            "origin_cell_outside_footprint",
            "Authored origin cell must be inside logical footprint bounds.",
        );
    }
    if footprint.permitted_rotations.is_empty() {
        push_issue(
            &mut report,
            Some("permitted_rotations"),
            "missing_permitted_rotation",
            "Logical footprint must permit at least one rotation.",
        );
    }
    report
}

/// Validate snap anchors without assuming a specific game runtime.
#[must_use]
pub fn validate_snap_anchors(anchors: &[SnapAnchor]) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    let mut ids = BTreeSet::new();
    for (index, anchor) in anchors.iter().enumerate() {
        validate_non_empty(
            &mut report,
            Some(format!("{index}.id")),
            &anchor.id,
            "empty_snap_anchor_id",
        );
        if !ids.insert(anchor.id.as_str()) {
            push_issue(
                &mut report,
                Some(format!("{index}.id")),
                "duplicate_snap_anchor_id",
                "Snap anchor IDs must be unique within one asset.",
            );
        }
        if !frame_is_finite(&anchor.local_frame) {
            push_issue(
                &mut report,
                Some(format!("{index}.local_frame")),
                "non_finite_snap_anchor_frame",
                "Snap anchor local frame must contain only finite values.",
            );
        }
        if anchor.relationship != SnapRelationship::Optional && anchor.compatibility_tags.is_empty()
        {
            push_issue(
                &mut report,
                Some(format!("{index}.relationship")),
                "invalid_snap_relationship",
                "Required or supporting snap anchors must declare at least one compatibility tag.",
            );
        }
    }
    report
}

/// Validate walkable surfaces and their anchor references.
#[must_use]
pub fn validate_walkable_surfaces(semantics: &ModuleSemantics) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    let anchor_ids = semantics
        .snap_anchors
        .iter()
        .map(|anchor| anchor.id.as_str())
        .collect::<BTreeSet<_>>();
    for (index, surface) in semantics.walkable_surfaces.iter().enumerate() {
        validate_walkable_surface(
            &mut report,
            &semantics.logical_footprint.cell_bounds,
            &anchor_ids,
            index,
            surface,
        );
    }
    report
}

/// Validate an authored construction profile.
#[must_use]
pub fn validate_construction_profile(profile: &ConstructionProfile) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    if profile.phases.is_empty() {
        push_issue(
            &mut report,
            Some("phases"),
            "missing_construction_phase",
            "Construction profile must contain at least one phase.",
        );
        return report;
    }

    let mut ids = BTreeSet::new();
    let mut thresholds = Vec::new();
    for (index, phase) in profile.phases.iter().enumerate() {
        validate_phase(&mut report, index, phase);
        if !ids.insert(phase.id.as_str()) {
            push_issue(
                &mut report,
                Some(format!("phases.{index}.id")),
                "duplicate_construction_phase_id",
                "Construction phase IDs must be unique.",
            );
        }
        thresholds.push(phase.progress_threshold);
    }

    if !ids.contains(profile.final_phase.as_str()) {
        push_issue(
            &mut report,
            Some("final_phase"),
            "unknown_final_construction_phase",
            "Final construction phase must reference an authored phase.",
        );
    }
    if let Some(damaged_state) = &profile.optional_damaged_state
        && !ids.contains(damaged_state.as_str())
    {
        push_issue(
            &mut report,
            Some("optional_damaged_state"),
            "unknown_damaged_construction_phase",
            "Optional damaged construction state must reference an authored phase.",
        );
    }

    for (index, phase) in profile.phases.iter().enumerate() {
        if let Some(predecessor) = &phase.required_predecessor {
            if !ids.contains(predecessor.as_str()) {
                push_issue(
                    &mut report,
                    Some(format!("phases.{index}.required_predecessor")),
                    "unknown_construction_phase_predecessor",
                    "Construction phase predecessor must reference an authored phase.",
                );
            } else if predecessor == &phase.id {
                push_issue(
                    &mut report,
                    Some(format!("phases.{index}.required_predecessor")),
                    "construction_phase_cycle",
                    "Construction phase cannot require itself.",
                );
            }
        }
    }
    validate_predecessor_cycles(profile, &mut report);
    validate_threshold_order(&thresholds, &mut report);
    validate_phase_visibility(profile, &mut report);
    report
}

/// Validate fixed-camera readability requirements.
#[must_use]
pub fn validate_readability_profile(profile: &ReadabilityProfile) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    if profile.fixed_camera_profiles.is_empty() {
        push_issue(
            &mut report,
            Some("fixed_camera_profiles"),
            "missing_fixed_camera_profile",
            "Readability profile must contain at least one fixed camera.",
        );
    }
    if profile.minimum_recognizable_pixel_size == 0 {
        push_issue(
            &mut report,
            Some("minimum_recognizable_pixel_size"),
            "invalid_minimum_recognizable_pixel_size",
            "Minimum recognizable pixel size must be greater than zero.",
        );
    }
    validate_fraction(
        &mut report,
        Some("silhouette_importance"),
        profile.silhouette_importance,
        "invalid_silhouette_importance",
    );
    validate_fraction(
        &mut report,
        Some("maximum_hidden_area_fraction"),
        profile.maximum_hidden_area_fraction,
        "invalid_hidden_area_fraction",
    );
    if profile.orientation_coverage.is_empty() {
        push_issue(
            &mut report,
            Some("orientation_coverage"),
            "missing_orientation_coverage",
            "Readability profile must include at least one orientation.",
        );
    }
    report
}

/// Validate triangle budgets.
#[must_use]
pub fn validate_triangle_budget(budget: &TriangleBudget) -> GameAssetValidationReport {
    let mut report = GameAssetValidationReport::default();
    if budget.preview_maximum == 0
        || budget.game_maximum == 0
        || budget.repeated_instance_maximum == 0
    {
        push_issue(
            &mut report,
            None::<String>,
            "invalid_triangle_budget",
            "Triangle budgets must be greater than zero.",
        );
    }
    if budget.preview_maximum > budget.game_maximum {
        push_issue(
            &mut report,
            Some("preview_maximum"),
            "preview_budget_exceeds_game_budget",
            "Preview triangle budget must not exceed game triangle budget.",
        );
    }
    if budget.repeated_instance_maximum > budget.game_maximum {
        push_issue(
            &mut report,
            Some("repeated_instance_maximum"),
            "repeated_budget_exceeds_game_budget",
            "Repeated-instance triangle budget must not exceed game triangle budget.",
        );
    }
    report
}

fn validate_walkable_surface(
    report: &mut GameAssetValidationReport,
    bounds: &CellBounds,
    anchor_ids: &BTreeSet<&str>,
    index: usize,
    surface: &WalkableSurface,
) {
    validate_non_empty(
        report,
        Some(format!("{index}.id")),
        &surface.id,
        "empty_walkable_surface_id",
    );
    if surface.polygon.len() < 3 {
        push_issue(
            report,
            Some(format!("{index}.polygon")),
            "invalid_walkable_surface_polygon",
            "Walkable surface polygon must contain at least three points.",
        );
    }
    for (point_index, point) in surface.polygon.iter().enumerate() {
        if !point.iter().all(|value| value.is_finite()) {
            push_issue(
                report,
                Some(format!("{index}.polygon.{point_index}")),
                "non_finite_walkable_surface",
                "Walkable surface polygon points must be finite.",
            );
        } else if !point_inside_cell_bounds(bounds, *point) {
            push_issue(
                report,
                Some(format!("{index}.polygon.{point_index}")),
                "walkable_surface_outside_bounds",
                "Walkable surface polygon points must stay inside logical footprint bounds.",
            );
        }
    }
    if !surface.elevation.is_finite() {
        push_issue(
            report,
            Some(format!("{index}.elevation")),
            "non_finite_walkable_elevation",
            "Walkable surface elevation must be finite.",
        );
    }
    for anchor in &surface.entry_exit_anchors {
        if !anchor_ids.contains(anchor.as_str()) {
            push_issue(
                report,
                Some(format!("{index}.entry_exit_anchors")),
                "unknown_walkable_anchor",
                "Walkable surface entry/exit anchor must reference a snap anchor.",
            );
        }
    }
}

fn validate_support_surfaces(semantics: &ModuleSemantics, report: &mut GameAssetValidationReport) {
    let mut ids = BTreeSet::new();
    for (index, surface) in semantics.support_surfaces.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("module_semantics.support_surfaces.{index}.id")),
            &surface.id,
            "empty_support_surface_id",
        );
        if !ids.insert(surface.id.as_str()) {
            push_issue(
                report,
                Some(format!("module_semantics.support_surfaces.{index}.id")),
                "duplicate_support_surface_id",
                "Support surface IDs must be unique within one asset.",
            );
        }
        match support_surface_points(&surface.shape) {
            Ok(points) => {
                for (point_index, point) in points.iter().enumerate() {
                    if !point.iter().all(|value| value.is_finite()) {
                        push_issue(
                            report,
                            Some(format!(
                                "module_semantics.support_surfaces.{index}.shape.{point_index}"
                            )),
                            "non_finite_support_surface",
                            "Support surface points must be finite.",
                        );
                    } else if !point_inside_cell_bounds(
                        &semantics.logical_footprint.cell_bounds,
                        *point,
                    ) {
                        push_issue(
                            report,
                            Some(format!(
                                "module_semantics.support_surfaces.{index}.shape.{point_index}"
                            )),
                            "support_surface_outside_bounds",
                            "Support surface points must stay inside logical footprint bounds.",
                        );
                    }
                }
            }
            Err((code, message)) => push_issue(
                report,
                Some(format!("module_semantics.support_surfaces.{index}.shape")),
                code,
                message,
            ),
        }
    }
}

fn validate_traversal_links(semantics: &ModuleSemantics, report: &mut GameAssetValidationReport) {
    let anchor_ids = semantics
        .snap_anchors
        .iter()
        .map(|anchor| anchor.id.as_str())
        .collect::<BTreeSet<_>>();
    for (index, link) in semantics.traversal_links.iter().enumerate() {
        if !anchor_ids.contains(link.from_anchor.as_str()) {
            push_issue(
                report,
                Some(format!(
                    "module_semantics.traversal_links.{index}.from_anchor"
                )),
                "unknown_traversal_link_anchor",
                "Traversal link source must reference a snap anchor.",
            );
        }
        if !anchor_ids.contains(link.to_anchor.as_str()) {
            push_issue(
                report,
                Some(format!(
                    "module_semantics.traversal_links.{index}.to_anchor"
                )),
                "unknown_traversal_link_anchor",
                "Traversal link destination must reference a snap anchor.",
            );
        }
    }
}

fn validate_collision_proxies(semantics: &ModuleSemantics, report: &mut GameAssetValidationReport) {
    for (index, proxy) in semantics.collision_proxies.iter().enumerate() {
        match proxy {
            CollisionProxy::Box {
                center,
                half_extents,
            } => {
                if !array_is_finite(center) || !array_is_positive(half_extents) {
                    push_issue(
                        report,
                        Some(format!("module_semantics.collision_proxies.{index}")),
                        "invalid_collision_proxy",
                        "Box collision proxies need finite centers and positive half-extents.",
                    );
                }
            }
            CollisionProxy::Capsule { a, b, radius } => {
                if !array_is_finite(a)
                    || !array_is_finite(b)
                    || !radius.is_finite()
                    || *radius <= 0.0
                {
                    push_issue(
                        report,
                        Some(format!("module_semantics.collision_proxies.{index}")),
                        "invalid_collision_proxy",
                        "Capsule collision proxies need finite endpoints and positive radius.",
                    );
                }
            }
            CollisionProxy::Cylinder {
                center,
                radius,
                height,
            } => {
                if !array_is_finite(center)
                    || !radius.is_finite()
                    || *radius <= 0.0
                    || !height.is_finite()
                    || *height <= 0.0
                {
                    push_issue(
                        report,
                        Some(format!("module_semantics.collision_proxies.{index}")),
                        "invalid_collision_proxy",
                        "Cylinder collision proxies need finite center, positive radius, and positive height.",
                    );
                }
            }
            CollisionProxy::ConvexHullReserved { reason } => {
                validate_non_empty(
                    report,
                    Some(format!("module_semantics.collision_proxies.{index}.reason")),
                    reason,
                    "empty_reserved_collision_proxy_reason",
                );
            }
        }
    }
}

fn validate_phase(report: &mut GameAssetValidationReport, index: usize, phase: &ConstructionPhase) {
    validate_non_empty(
        report,
        Some(format!("phases.{index}.id")),
        &phase.id,
        "empty_construction_phase_id",
    );
    validate_non_empty(
        report,
        Some(format!("phases.{index}.label")),
        &phase.label,
        "empty_construction_phase_label",
    );
    validate_fraction(
        report,
        Some(format!("phases.{index}.progress_threshold")),
        phase.progress_threshold,
        "invalid_construction_phase_threshold",
    );
}

fn validate_predecessor_cycles(
    profile: &ConstructionProfile,
    report: &mut GameAssetValidationReport,
) {
    let predecessors = profile
        .phases
        .iter()
        .filter_map(|phase| {
            phase
                .required_predecessor
                .as_ref()
                .map(|predecessor| (phase.id.as_str(), predecessor.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    for phase in &profile.phases {
        let mut seen = BTreeSet::new();
        let mut current = phase.id.as_str();
        while let Some(next) = predecessors.get(current).copied() {
            if !seen.insert(current) {
                push_issue(
                    report,
                    Some(format!("phases.{}", phase.id)),
                    "construction_phase_cycle",
                    "Construction phase predecessor graph must not contain cycles.",
                );
                break;
            }
            current = next;
        }
    }
}

fn validate_threshold_order(thresholds: &[f32], report: &mut GameAssetValidationReport) {
    for (index, pair) in thresholds.windows(2).enumerate() {
        if pair[0].is_finite() && pair[1].is_finite() && pair[0] > pair[1] {
            push_issue(
                report,
                Some(format!("phases.{}", index + 1)),
                "construction_phase_threshold_order",
                "Construction phase thresholds must be non-decreasing.",
            );
        }
    }
}

fn validate_phase_visibility(
    profile: &ConstructionProfile,
    report: &mut GameAssetValidationReport,
) {
    let allowed_temporary = match &profile.monotonic_visibility_policy {
        MonotonicVisibilityPolicy::Strict => BTreeSet::new(),
        MonotonicVisibilityPolicy::AllowTemporaryHidden { tags } => {
            tags.iter().map(String::as_str).collect()
        }
    };
    let mut previous_visible = BTreeSet::<&str>::new();
    for (index, phase) in profile.phases.iter().enumerate() {
        let current_visible = phase
            .visible_part_tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for hidden in previous_visible.difference(&current_visible) {
            if !allowed_temporary.contains(hidden) {
                push_issue(
                    report,
                    Some(format!("phases.{index}.visible_part_tags")),
                    "non_monotonic_phase_visibility",
                    "Construction phase visibility must be monotonic unless a tag is explicitly temporary.",
                );
            }
        }
        previous_visible = current_visible;
    }
}

fn support_surface_points(
    shape: &SurfaceShape,
) -> Result<Vec<[f32; 2]>, (&'static str, &'static str)> {
    match shape {
        SurfaceShape::Rectangle { center, size } => {
            if !size.iter().all(|value| value.is_finite() && *value > 0.0) {
                return Err((
                    "invalid_support_surface_shape",
                    "Rectangle support surfaces must have positive finite size.",
                ));
            }
            let half = [size[0] * 0.5, size[1] * 0.5];
            Ok(vec![
                [center[0] - half[0], center[1] - half[1]],
                [center[0] + half[0], center[1] - half[1]],
                [center[0] + half[0], center[1] + half[1]],
                [center[0] - half[0], center[1] + half[1]],
            ])
        }
        SurfaceShape::Polygon { points } => {
            if points.len() < 3 {
                Err((
                    "invalid_support_surface_shape",
                    "Polygon support surfaces must contain at least three points.",
                ))
            } else {
                Ok(points.clone())
            }
        }
    }
}

fn point_inside_cell_bounds(bounds: &CellBounds, point: [f32; 2]) -> bool {
    let min = [bounds.min[0] as f32, bounds.min[1] as f32];
    let max = [bounds.max[0] as f32 + 1.0, bounds.max[1] as f32 + 1.0];
    point[0] >= min[0] && point[0] <= max[0] && point[1] >= min[1] && point[1] <= max[1]
}

fn cell_contains(bounds: &CellBounds, cell: [i32; 2]) -> bool {
    cell[0] >= bounds.min[0]
        && cell[0] <= bounds.max[0]
        && cell[1] >= bounds.min[1]
        && cell[1] <= bounds.max[1]
}

fn frame_is_finite(frame: &Frame3) -> bool {
    array_is_finite(&frame.origin)
        && array_is_finite(&frame.x_axis)
        && array_is_finite(&frame.y_axis)
        && array_is_finite(&frame.z_axis)
}

fn array_is_finite<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn array_is_positive<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().all(|value| value.is_finite() && *value > 0.0)
}

fn validate_fraction(
    report: &mut GameAssetValidationReport,
    subject: Option<impl Into<String>>,
    value: f32,
    code: &'static str,
) {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        push_issue(
            report,
            subject,
            code,
            "Value must be a finite fraction from 0 to 1.",
        );
    }
}

fn validate_non_empty(
    report: &mut GameAssetValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if value.trim().is_empty() {
        push_issue(report, subject, code, "Value cannot be empty.");
    }
}

fn push_issue(
    report: &mut GameAssetValidationReport,
    subject: Option<impl Into<String>>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    report.issues.push(GameAssetValidationIssue {
        subject: subject.map(Into::into),
        code: code.into(),
        message: message.into(),
    });
}
