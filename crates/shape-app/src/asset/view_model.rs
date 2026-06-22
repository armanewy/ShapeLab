//! Adapter from reducer state to asset panel DTOs.

use std::collections::BTreeMap;

use shape_asset::{
    AssetRecipe, CutEdgeTreatment, GeometrySource, ModelingOperationSpec, ParameterDescriptor,
    enumerate_parameters, get_scalar,
};

use crate::asset::{
    AssetAppState, AssetAvailableEdgeTreatment, AssetCandidate, AssetCandidateEdit,
    AssetCutControl, AssetCutOperation, AssetCutOperationKind, AssetEdgeTreatment,
    AssetHistoryRevision, AssetJobProgress, AssetParameter, AssetParameterGroup, AssetPart,
    AssetUiJobKind, AssetUiState, AssetValidationMessage, AssetValidationState, BoundaryLoopId,
    GeneratedPartKind, OperationId, ParameterId, PartDefinitionId, PartInstanceId,
};
use shape_search::asset::AssetCandidateEditKind;

use super::jobs::AssetJobSlot;
use super::state::{AssetAppIssue, AssetCandidateSlot};

/// Build the complete UI DTO consumed by asset panels.
pub(crate) fn build_asset_ui_state(state: &AssetAppState, wireframe: bool) -> AssetUiState {
    let mut ui_state = AssetUiState::empty(state.recipe.title.clone());
    ui_state.selected_part = state.selected_part_instance;
    ui_state.selected_cut_operation = state.selected_cut_operation;
    ui_state.parts = parts_for_recipe(&state.recipe, &state.validation_issues);
    ui_state.parameters = parameters_for_recipe(&state.recipe);
    ui_state.cut_operations = cut_operations_for_recipe(
        &state.recipe,
        state.selected_part_instance,
        state.selected_cut_operation,
    );
    ui_state.candidates = candidates_for_state(state);
    ui_state.history = history_for_state(state);
    ui_state.active_job = active_job_for_state(state);
    ui_state.validation = validation_for_state(&state.validation_issues);
    ui_state.parameter_locks = state.locks.parameters.clone();
    ui_state.part_locks = state.locks.instances.clone();
    ui_state.subtree_locks = state.locks.subtrees.clone();
    ui_state.topology_locks = state.locks.topology.clone();
    ui_state.wireframe = wireframe;
    ui_state
}

fn parts_for_recipe(recipe: &AssetRecipe, issues: &[AssetAppIssue]) -> Vec<AssetPart> {
    recipe
        .instances
        .values()
        .map(|instance| {
            let definition = recipe.definitions.get(&instance.definition);
            AssetPart {
                id: instance.id,
                parent: instance.parent,
                definition: instance.definition,
                name: instance.name.clone(),
                definition_name: definition
                    .map(|definition| definition.name.clone())
                    .unwrap_or_else(|| format!("definition {}", instance.definition.0)),
                enabled: instance.enabled,
                optional: recipe.variation.optional_instances.contains(&instance.id),
                generated: instance
                    .generated_by
                    .map(|_| GeneratedPartKind::LinearArray { index: 1, count: 1 })
                    .unwrap_or(GeneratedPartKind::Authored),
                socket_count: definition
                    .map(|definition| definition.sockets.len())
                    .unwrap_or(0),
                region_count: definition
                    .map(|definition| definition.regions.len())
                    .unwrap_or(0),
                warning_count: issues_for_part(issues, instance.id),
            }
        })
        .collect()
}

fn parameters_for_recipe(recipe: &AssetRecipe) -> Vec<AssetParameter> {
    enumerate_parameters(recipe)
        .into_iter()
        .filter_map(|parameter| parameter_for_recipe(recipe, parameter))
        .collect()
}

fn cut_operations_for_recipe(
    recipe: &AssetRecipe,
    selected_part: Option<PartInstanceId>,
    selected_operation: Option<OperationId>,
) -> Vec<AssetCutOperation> {
    let Some(part_id) = selected_part else {
        return Vec::new();
    };
    let Some(part) = recipe.instances.get(&part_id) else {
        return Vec::new();
    };
    let Some(definition) = recipe.definitions.get(&part.definition) else {
        return Vec::new();
    };

    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| {
            cut_operation_for_spec(
                recipe,
                definition.id,
                part_id,
                operation,
                &definition.geometry.operations,
                selected_operation,
            )
        })
        .collect()
}

fn cut_operation_for_spec(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    part: PartInstanceId,
    operation: &ModelingOperationSpec,
    operations: &[ModelingOperationSpec],
    selected_operation: Option<OperationId>,
) -> Option<AssetCutOperation> {
    let operation_id = operation.operation_id();
    let host = cut_host_bounds(recipe, definition);
    let topology_locked = recipe.topology_locks.contains(&definition);
    let (kind, controls, loop_controls) = match operation {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            floor_loop,
            edge_treatment,
            ..
        } => {
            let ranges = rect_cut_ranges(host, *center, *size, *rim_width);
            let bevel_limit = rect_loop_bevel_limit(*size, *rim_width, *depth);
            let can_add_treatment = matches!(edge_treatment, CutEdgeTreatment::BevelEligible);
            (
                AssetCutOperationKind::RecessedPanel,
                vec![
                    cut_control(
                        "recessed_panel_cut.center.x",
                        "Position X",
                        center[0],
                        ranges.center_x.0,
                        ranges.center_x.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.center.y",
                        "Position Y",
                        center[1],
                        ranges.center_y.0,
                        ranges.center_y.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.size.x",
                        "Width",
                        size[0],
                        0.05,
                        ranges.size_x_max,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.size.y",
                        "Height",
                        size[1],
                        0.05,
                        ranges.size_y_max,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.depth",
                        "Depth",
                        *depth,
                        0.005,
                        host.map_or(1.0, |host| (host.thickness * 0.95).max(0.005)),
                        0.005,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.rim_width",
                        "Rim Width",
                        *rim_width,
                        0.001,
                        ranges.rim_width_max,
                        0.005,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.corner_radius",
                        "Corner Radius",
                        *corner_radius,
                        0.0,
                        (size[0].min(size[1]) * 0.5).max(0.0),
                        0.005,
                        false,
                    ),
                    cut_control(
                        "recessed_panel_cut.corner_segments",
                        "Corner Resolution",
                        *corner_segments as f32,
                        if topology_locked {
                            *corner_segments as f32
                        } else {
                            1.0
                        },
                        if topology_locked {
                            *corner_segments as f32
                        } else {
                            16.0
                        },
                        1.0,
                        true,
                    ),
                ],
                vec![
                    EdgeLoopControl::new(*entry_loop, "Entry edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_paired_depth(*depth),
                    EdgeLoopControl::new(*floor_loop, "Floor edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_paired_depth(*depth),
                ],
            )
        }
        ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            exit_loop,
            edge_treatment,
            ..
        } => {
            let ranges = rect_cut_ranges(host, *center, *size, *rim_width);
            let bevel_limit = rect_through_loop_bevel_limit(host, *size, *rim_width);
            let can_add_treatment = matches!(edge_treatment, CutEdgeTreatment::BevelEligible);
            (
                AssetCutOperationKind::RectangularOpening,
                vec![
                    cut_control(
                        "rectangular_through_cut.center.x",
                        "Position X",
                        center[0],
                        ranges.center_x.0,
                        ranges.center_x.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.center.y",
                        "Position Y",
                        center[1],
                        ranges.center_y.0,
                        ranges.center_y.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.size.x",
                        "Width",
                        size[0],
                        0.05,
                        ranges.size_x_max,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.size.y",
                        "Height",
                        size[1],
                        0.05,
                        ranges.size_y_max,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.rim_width",
                        "Rim Width",
                        *rim_width,
                        0.001,
                        ranges.rim_width_max,
                        0.005,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.corner_radius",
                        "Corner Radius",
                        *corner_radius,
                        0.0,
                        (size[0].min(size[1]) * 0.5).max(0.0),
                        0.005,
                        false,
                    ),
                    cut_control(
                        "rectangular_through_cut.corner_segments",
                        "Corner Resolution",
                        *corner_segments as f32,
                        if topology_locked {
                            *corner_segments as f32
                        } else {
                            1.0
                        },
                        if topology_locked {
                            *corner_segments as f32
                        } else {
                            16.0
                        },
                        1.0,
                        true,
                    ),
                ],
                vec![
                    EdgeLoopControl::new(*entry_loop, "Entry edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_optional_paired_depth(host.map(|host| host.thickness)),
                    EdgeLoopControl::new(*exit_loop, "Exit edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_optional_paired_depth(host.map(|host| host.thickness)),
                ],
            )
        }
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            radial_segments,
            rim_width,
            entry_loop,
            exit_loop,
            edge_treatment,
            ..
        } => {
            let ranges = circular_cut_ranges(host, *center, *radius, *rim_width);
            let bevel_limit = circular_through_loop_bevel_limit(host, *radius, *rim_width);
            let can_add_treatment = matches!(edge_treatment, CutEdgeTreatment::BevelEligible);
            (
                AssetCutOperationKind::CircularOpening,
                vec![
                    cut_control(
                        "circular_through_cut.center.x",
                        "Position X",
                        center[0],
                        ranges.center_x.0,
                        ranges.center_x.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "circular_through_cut.center.y",
                        "Position Y",
                        center[1],
                        ranges.center_y.0,
                        ranges.center_y.1,
                        0.01,
                        false,
                    ),
                    cut_control(
                        "circular_through_cut.radius",
                        "Radius",
                        *radius,
                        0.01,
                        ranges.radius_max,
                        0.005,
                        false,
                    ),
                    cut_control(
                        "circular_through_cut.rim_width",
                        "Rim Width",
                        *rim_width,
                        0.001,
                        ranges.rim_width_max,
                        0.005,
                        false,
                    ),
                    cut_control(
                        "circular_through_cut.radial_segments",
                        "Roundness",
                        *radial_segments as f32,
                        if topology_locked {
                            *radial_segments as f32
                        } else {
                            6.0
                        },
                        if topology_locked {
                            *radial_segments as f32
                        } else {
                            48.0
                        },
                        1.0,
                        true,
                    ),
                ],
                vec![
                    EdgeLoopControl::new(*entry_loop, "Entry edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_optional_paired_depth(host.map(|host| host.thickness)),
                    EdgeLoopControl::new(*exit_loop, "Exit edge", bevel_limit)
                        .with_can_add_treatment(can_add_treatment)
                        .with_optional_paired_depth(host.map(|host| host.thickness)),
                ],
            )
        }
        _ => return None,
    };
    let edge_treatments = edge_treatments_for_cut(
        definition,
        part,
        operation_id,
        operations,
        &loop_controls,
        topology_locked,
    );
    let available_edge_treatments = available_edge_treatments_for_cut(
        definition,
        part,
        operation_id,
        operations,
        &loop_controls,
    );

    Some(AssetCutOperation {
        definition,
        part,
        operation: operation_id,
        label: format!("{} {}", kind.label(), operation_id.0),
        kind,
        controls,
        edge_treatments,
        available_edge_treatments,
        selected: selected_operation == Some(operation_id),
    })
}

const BOUNDARY_BEVEL_PROFILE_MIN: f32 = 0.05;
const BOUNDARY_BEVEL_PROFILE_MAX: f32 = 8.0;
const BEVEL_CONTROL_SAFETY_MARGIN: f32 = 0.001;
const DEFAULT_BOUNDARY_BEVEL_SEGMENTS: u32 = 2;
const DEFAULT_BOUNDARY_BEVEL_PROFILE: f32 = 1.0;

#[derive(Debug, Copy, Clone)]
struct EdgeLoopControl {
    loop_id: BoundaryLoopId,
    label: &'static str,
    width_limit: f32,
    paired_depth: Option<f32>,
    can_add_treatment: bool,
}

impl EdgeLoopControl {
    fn new(loop_id: BoundaryLoopId, label: &'static str, width_limit: f32) -> Self {
        Self {
            loop_id,
            label,
            width_limit,
            paired_depth: None,
            can_add_treatment: true,
        }
    }

    fn with_can_add_treatment(mut self, can_add_treatment: bool) -> Self {
        self.can_add_treatment = can_add_treatment;
        self
    }

    fn with_paired_depth(mut self, depth: f32) -> Self {
        self.paired_depth = Some(depth.abs());
        self
    }

    fn with_optional_paired_depth(self, depth: Option<f32>) -> Self {
        match depth {
            Some(depth) => self.with_paired_depth(depth),
            None => self,
        }
    }
}

fn edge_treatments_for_cut(
    definition: PartDefinitionId,
    part: PartInstanceId,
    source_operation: OperationId,
    operations: &[ModelingOperationSpec],
    loop_controls: &[EdgeLoopControl],
    topology_locked: bool,
) -> Vec<AssetEdgeTreatment> {
    operations
        .iter()
        .filter_map(|operation| {
            let ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                ..
            } = operation
            else {
                return None;
            };
            let loop_control = loop_controls
                .iter()
                .find(|control| control.loop_id == *target_loop)?;
            let width_limit = sibling_aware_bevel_width_limit(
                loop_control,
                *target_loop,
                loop_controls,
                operations,
            );
            Some(AssetEdgeTreatment {
                definition,
                part,
                source_operation,
                operation: *operation,
                target_loop: *target_loop,
                label: format!("{}: Rounded", loop_control.label),
                controls: vec![
                    cut_control(
                        "bevel_boundary_loop.width",
                        "Width",
                        *width,
                        0.001,
                        bevel_width_control_max(*width, width_limit),
                        0.001,
                        false,
                    ),
                    cut_control(
                        "bevel_boundary_loop.segments",
                        "Segments",
                        *segments as f32,
                        if topology_locked {
                            *segments as f32
                        } else {
                            1.0
                        },
                        if topology_locked {
                            *segments as f32
                        } else {
                            8.0
                        },
                        1.0,
                        true,
                    ),
                    cut_control(
                        "bevel_boundary_loop.profile",
                        "Profile",
                        *profile,
                        BOUNDARY_BEVEL_PROFILE_MIN,
                        BOUNDARY_BEVEL_PROFILE_MAX,
                        0.05,
                        false,
                    ),
                ],
            })
        })
        .collect()
}

fn available_edge_treatments_for_cut(
    definition: PartDefinitionId,
    part: PartInstanceId,
    source_operation: OperationId,
    operations: &[ModelingOperationSpec],
    loop_controls: &[EdgeLoopControl],
) -> Vec<AssetAvailableEdgeTreatment> {
    loop_controls
        .iter()
        .filter(|control| control.can_add_treatment)
        .filter(|control| boundary_bevel_width_for_loop(operations, control.loop_id).is_none())
        .map(|control| {
            let width_limit = sibling_aware_bevel_width_limit(
                control,
                control.loop_id,
                loop_controls,
                operations,
            );
            AssetAvailableEdgeTreatment {
                definition,
                part,
                source_operation,
                target_loop: control.loop_id,
                label: format!("{}: Hard", control.label),
                width: default_boundary_bevel_width(width_limit),
                segments: DEFAULT_BOUNDARY_BEVEL_SEGMENTS,
                profile: DEFAULT_BOUNDARY_BEVEL_PROFILE,
            }
        })
        .collect()
}

fn sibling_aware_bevel_width_limit(
    loop_control: &EdgeLoopControl,
    target_loop: BoundaryLoopId,
    loop_controls: &[EdgeLoopControl],
    operations: &[ModelingOperationSpec],
) -> f32 {
    let Some(depth) = loop_control.paired_depth else {
        return loop_control.width_limit;
    };
    let sibling_width = loop_controls
        .iter()
        .filter(|candidate| candidate.loop_id != target_loop)
        .filter_map(|candidate| boundary_bevel_width_for_loop(operations, candidate.loop_id))
        .sum::<f32>();
    loop_control
        .width_limit
        .min((depth - sibling_width - BEVEL_CONTROL_SAFETY_MARGIN).max(0.001))
}

fn boundary_bevel_width_for_loop(
    operations: &[ModelingOperationSpec],
    target: BoundaryLoopId,
) -> Option<f32> {
    operations.iter().find_map(|operation| match operation {
        ModelingOperationSpec::BevelBoundaryLoop {
            target_loop, width, ..
        } if *target_loop == target => Some(*width),
        _ => None,
    })
}

fn bevel_width_control_max(current: f32, safe_limit: f32) -> f32 {
    safe_limit.max(current).max(0.001)
}

fn default_boundary_bevel_width(width_limit: f32) -> f32 {
    (width_limit * 0.35).clamp(0.001, width_limit.max(0.001))
}

fn rect_loop_bevel_limit(size: [f32; 2], rim_width: f32, depth: f32) -> f32 {
    (rim_width.abs().min(depth.abs()).min(rect_loop_radius(size)) * 0.9).max(0.001)
}

fn rect_through_loop_bevel_limit(
    host: Option<CutHostBounds>,
    size: [f32; 2],
    rim_width: f32,
) -> f32 {
    let thickness_limit = host.map_or(1.0, |host| host.thickness.abs() * 0.45);
    (rim_width
        .abs()
        .min(rect_loop_radius(size))
        .min(thickness_limit)
        * 0.9)
        .max(0.001)
}

fn circular_through_loop_bevel_limit(
    host: Option<CutHostBounds>,
    radius: f32,
    rim_width: f32,
) -> f32 {
    let thickness_limit = host.map_or(1.0, |host| host.thickness.abs() * 0.45);
    (rim_width.abs().min(radius.abs() * 0.5).min(thickness_limit) * 0.9).max(0.001)
}

fn rect_loop_radius(size: [f32; 2]) -> f32 {
    size[0].abs().min(size[1].abs()) * 0.25
}

#[derive(Debug, Copy, Clone)]
struct CutHostBounds {
    half_size: [f32; 2],
    thickness: f32,
}

#[derive(Debug, Copy, Clone)]
struct RectCutRanges {
    center_x: (f32, f32),
    center_y: (f32, f32),
    size_x_max: f32,
    size_y_max: f32,
    rim_width_max: f32,
}

#[derive(Debug, Copy, Clone)]
struct CircularCutRanges {
    center_x: (f32, f32),
    center_y: (f32, f32),
    radius_max: f32,
    rim_width_max: f32,
}

fn cut_host_bounds(recipe: &AssetRecipe, definition: PartDefinitionId) -> Option<CutHostBounds> {
    let definition = recipe.definitions.get(&definition)?;
    match definition.geometry.source {
        GeometrySource::Plate { size, thickness } => Some(CutHostBounds {
            half_size: [size[0].abs() * 0.5, size[1].abs() * 0.5],
            thickness: thickness.abs(),
        }),
        GeometrySource::RoundedBox { .. }
        | GeometrySource::Cylinder { .. }
        | GeometrySource::Frustum { .. }
        | GeometrySource::Sweep { .. }
        | GeometrySource::Lathe { .. }
        | GeometrySource::LiteralMesh { .. }
        | GeometrySource::ReservedBooleanResult { .. } => None,
    }
}

fn rect_cut_ranges(
    host: Option<CutHostBounds>,
    center: [f32; 2],
    size: [f32; 2],
    rim_width: f32,
) -> RectCutRanges {
    let Some(host) = host else {
        return RectCutRanges {
            center_x: (-2.0, 2.0),
            center_y: (-2.0, 2.0),
            size_x_max: 4.0,
            size_y_max: 4.0,
            rim_width_max: 0.5,
        };
    };
    let half_cut = [size[0].abs() * 0.5, size[1].abs() * 0.5];
    let clearance_x = (host.half_size[0] - center[0].abs() - rim_width.max(0.0)).max(0.025);
    let clearance_y = (host.half_size[1] - center[1].abs() - rim_width.max(0.0)).max(0.025);
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - half_cut[0]).max(0.001),
        (host.half_size[1] - center[1].abs() - half_cut[1]).max(0.001),
    ];
    RectCutRanges {
        center_x: ordered_range(
            -host.half_size[0] + half_cut[0] + rim_width,
            host.half_size[0] - half_cut[0] - rim_width,
            center[0],
        ),
        center_y: ordered_range(
            -host.half_size[1] + half_cut[1] + rim_width,
            host.half_size[1] - half_cut[1] - rim_width,
            center[1],
        ),
        size_x_max: (clearance_x * 2.0).max(size[0].abs()).max(0.05),
        size_y_max: (clearance_y * 2.0).max(size[1].abs()).max(0.05),
        rim_width_max: rim_clearance[0].min(rim_clearance[1]).clamp(0.001, 0.5),
    }
}

fn circular_cut_ranges(
    host: Option<CutHostBounds>,
    center: [f32; 2],
    radius: f32,
    rim_width: f32,
) -> CircularCutRanges {
    let Some(host) = host else {
        return CircularCutRanges {
            center_x: (-2.0, 2.0),
            center_y: (-2.0, 2.0),
            radius_max: 2.0,
            rim_width_max: 0.5,
        };
    };
    let cut_radius = radius.abs();
    let radius_clearance = [
        (host.half_size[0] - center[0].abs() - rim_width.max(0.0)).max(0.01),
        (host.half_size[1] - center[1].abs() - rim_width.max(0.0)).max(0.01),
    ];
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - cut_radius).max(0.001),
        (host.half_size[1] - center[1].abs() - cut_radius).max(0.001),
    ];
    CircularCutRanges {
        center_x: ordered_range(
            -host.half_size[0] + cut_radius + rim_width,
            host.half_size[0] - cut_radius - rim_width,
            center[0],
        ),
        center_y: ordered_range(
            -host.half_size[1] + cut_radius + rim_width,
            host.half_size[1] - cut_radius - rim_width,
            center[1],
        ),
        radius_max: radius_clearance[0]
            .min(radius_clearance[1])
            .max(cut_radius)
            .max(0.01),
        rim_width_max: rim_clearance[0].min(rim_clearance[1]).clamp(0.001, 0.5),
    }
}

fn ordered_range(minimum: f32, maximum: f32, current: f32) -> (f32, f32) {
    if minimum <= maximum {
        (minimum, maximum)
    } else {
        (current, current)
    }
}

fn cut_control(
    field: &str,
    label: &str,
    value: f32,
    minimum: f32,
    maximum: f32,
    step: f32,
    topology_changing: bool,
) -> AssetCutControl {
    AssetCutControl {
        field: field.to_owned(),
        label: label.to_owned(),
        value,
        minimum,
        maximum,
        step,
        topology_changing,
    }
}

fn parameter_for_recipe(
    recipe: &AssetRecipe,
    parameter: ParameterDescriptor,
) -> Option<AssetParameter> {
    let value = get_scalar(recipe, &parameter.path).ok()?;
    let definition = definition_id_from_scalar_path(&parameter.path);
    let part = instance_id_from_scalar_path(&parameter.path).or_else(|| {
        definition.and_then(|definition| first_instance_of_definition(recipe, definition))
    });
    Some(AssetParameter {
        id: parameter.id,
        part,
        definition,
        label: parameter.label.clone(),
        technical_name: parameter.path.clone(),
        group: parameter_group(&parameter),
        value,
        minimum: parameter.minimum,
        maximum: parameter.maximum,
        step: parameter.step,
        locked: recipe.locks.contains(&parameter.id),
        topology_changing: parameter.topology_changing,
        beginner_description: parameter.beginner_description.clone(),
    })
}

fn candidates_for_state(state: &AssetAppState) -> Vec<AssetCandidate> {
    state
        .candidate_slots
        .iter()
        .map(candidate_for_slot)
        .collect()
}

#[must_use]
pub(crate) fn candidate_for_slot(slot: &AssetCandidateSlot) -> AssetCandidate {
    let edits = slot
        .candidate
        .changes
        .iter()
        .map(candidate_edit_from_change)
        .collect::<Vec<_>>();
    let structural_changes = slot
        .candidate
        .changes
        .iter()
        .filter(|change| change.structural)
        .count();
    let numeric_changes = edits.len().saturating_sub(structural_changes);
    AssetCandidate {
        id: slot.candidate.id,
        title: slot.candidate.label.clone(),
        structural_changes,
        numeric_changes,
        edits,
        validation: slot
            .preview
            .as_ref()
            .map(|preview| {
                if preview.validation_summary.valid {
                    AssetValidationState::Valid
                } else {
                    AssetValidationState::Error(format!(
                        "{} validation issue(s)",
                        preview.validation_summary.issue_count
                    ))
                }
            })
            .or_else(|| {
                slot.preview_failure
                    .as_ref()
                    .map(|message| AssetValidationState::Error(message.clone()))
            })
            .unwrap_or(AssetValidationState::Pending),
    }
}

fn history_for_state(state: &AssetAppState) -> Vec<AssetHistoryRevision> {
    let mut child_counts = BTreeMap::<_, usize>::new();
    for revision in state.revision_history.revisions.values() {
        if let Some(parent) = revision.parent {
            *child_counts.entry(parent).or_insert(0) += 1;
        }
    }
    state
        .revision_history
        .revisions
        .values()
        .map(|revision| AssetHistoryRevision {
            id: revision.id,
            parent: revision.parent,
            label: revision.label.clone(),
            operation_summary: revision.label.clone(),
            child_count: child_counts.get(&revision.id).copied().unwrap_or(0),
            selected: revision.id == state.revision_history.current,
        })
        .collect()
}

fn active_job_for_state(state: &AssetAppState) -> Option<AssetJobProgress> {
    let (slot, active) = state.active_jobs.iter().next()?;
    let (kind, phase, completed, total) = match slot {
        AssetJobSlot::CompileCurrentAsset => (AssetUiJobKind::Compile, "Compiling asset", 0, 1),
        AssetJobSlot::RenderCurrentPreview => (AssetUiJobKind::Preview, "Rendering preview", 0, 1),
        AssetJobSlot::GenerateCandidates => (
            AssetUiJobKind::CandidateSearch,
            "Generating directions",
            state.candidate_slots.len(),
            6,
        ),
        AssetJobSlot::CompileCandidatePreviews => (
            AssetUiJobKind::CandidateSearch,
            "Rendering candidates",
            state
                .candidate_slots
                .iter()
                .filter(|slot| slot.preview.is_some())
                .count(),
            6,
        ),
        AssetJobSlot::ExportObj | AssetJobSlot::ExportPackage => {
            (AssetUiJobKind::Inspect, "Exporting asset", 0, 1)
        }
    };
    Some(AssetJobProgress {
        job_id: active.job_id,
        kind,
        phase: phase.to_owned(),
        completed,
        total,
    })
}

fn validation_for_state(issues: &[AssetAppIssue]) -> Vec<AssetValidationMessage> {
    issues
        .iter()
        .map(|issue| AssetValidationMessage {
            part: issue
                .subject
                .as_deref()
                .and_then(part_id_from_validation_subject),
            state: AssetValidationState::Warning(issue.code.clone()),
            message: issue.message.clone(),
        })
        .collect()
}

fn candidate_edit_from_change(change: &super::jobs::AssetCandidateChange) -> AssetCandidateEdit {
    AssetCandidateEdit {
        subject: change.subject.clone(),
        label: change_label(change),
        before: change.before.as_deref().and_then(parse_scalar_summary),
        after: change.after.as_deref().and_then(parse_scalar_summary),
        structural: change.structural,
    }
}

fn change_label(change: &super::jobs::AssetCandidateChange) -> String {
    if change
        .before
        .as_deref()
        .and_then(parse_scalar_summary)
        .is_some()
        || change
            .after
            .as_deref()
            .and_then(parse_scalar_summary)
            .is_some()
    {
        return change.label.clone();
    }

    match change.kind {
        AssetCandidateEditKind::Parameter => change.label.clone(),
        AssetCandidateEditKind::Transform => "placement".to_owned(),
        AssetCandidateEditKind::GeneratorDimension => "proportions".to_owned(),
        AssetCandidateEditKind::Bevel => "edge softness".to_owned(),
        AssetCandidateEditKind::Sweep => "profile curve".to_owned(),
        AssetCandidateEditKind::Lathe => "turned profile".to_owned(),
        AssetCandidateEditKind::ArrayCount => "repeat count".to_owned(),
        AssetCandidateEditKind::ArraySpacing => "spacing".to_owned(),
        AssetCandidateEditKind::OptionalPart => "presence".to_owned(),
        AssetCandidateEditKind::Replacement => "part choice".to_owned(),
        AssetCandidateEditKind::DetailDensity => "detail density".to_owned(),
        AssetCandidateEditKind::ModelingOperation => "operation".to_owned(),
    }
}

fn parse_scalar_summary(value: &str) -> Option<f32> {
    value.parse::<f32>().ok()
}

fn issues_for_part(issues: &[AssetAppIssue], part: PartInstanceId) -> usize {
    let needle = format!("part.{}", part.0);
    issues
        .iter()
        .filter(|issue| {
            issue
                .subject
                .as_deref()
                .is_some_and(|subject| subject.contains(&needle))
        })
        .count()
}

fn first_instance_of_definition(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
) -> Option<PartInstanceId> {
    recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id)
}

fn parameter_group(parameter: &ParameterDescriptor) -> AssetParameterGroup {
    match parameter.group.as_str() {
        "Size" => AssetParameterGroup::Size,
        "Proportions" => AssetParameterGroup::Proportions,
        "Placement" => AssetParameterGroup::Placement,
        "Curvature" => AssetParameterGroup::Curvature,
        "Edge Softness" => AssetParameterGroup::EdgeSoftness,
        "Repetition" => AssetParameterGroup::Repetition,
        "Part Presence" => AssetParameterGroup::PartPresence,
        "Detail Density" => AssetParameterGroup::DetailDensity,
        _ => {
            let lower = parameter.path.to_ascii_lowercase();
            if lower.contains("linear_array") || lower.contains("radial_array") {
                AssetParameterGroup::Repetition
            } else if lower.contains("bevel") || lower.contains("radius") {
                AssetParameterGroup::EdgeSoftness
            } else if lower.contains("translation") || lower.contains("rotation") {
                AssetParameterGroup::Placement
            } else if lower.contains("sweep") || lower.contains("lathe") {
                AssetParameterGroup::Curvature
            } else {
                AssetParameterGroup::Size
            }
        }
    }
}

fn definition_id_from_scalar_path(path: &str) -> Option<PartDefinitionId> {
    let mut parts = path.split('.');
    if parts.next()? != "definition" {
        return None;
    }
    parts.next()?.parse().ok().map(PartDefinitionId)
}

fn instance_id_from_scalar_path(path: &str) -> Option<PartInstanceId> {
    let mut parts = path.split('.');
    if parts.next()? != "instance" {
        return None;
    }
    parts.next()?.parse().ok().map(PartInstanceId)
}

fn part_id_from_validation_subject(subject: &str) -> Option<PartInstanceId> {
    let parts = subject.split('.').collect::<Vec<_>>();
    parts.windows(2).find_map(|window| {
        (window[0] == "part").then(|| window[1].parse().ok().map(PartInstanceId))?
    })
}

#[allow(dead_code)]
fn _parameter_id_for_docs(_: ParameterId) {}
