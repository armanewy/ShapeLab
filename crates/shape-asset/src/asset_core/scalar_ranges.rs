
/// Inclusive scalar range derived from semantic operation dependencies.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct OperationScalarRange {
    /// Smallest accepted value.
    pub minimum: f32,
    /// Largest accepted value.
    pub maximum: f32,
}

impl OperationScalarRange {
    fn new(minimum: f32, maximum: f32) -> Option<Self> {
        if minimum.is_finite() && maximum.is_finite() && minimum <= maximum {
            Some(Self { minimum, maximum })
        } else {
            None
        }
    }

    /// Return true when `value` lies inside the range, allowing a small
    /// floating-point tolerance for UI and search-generated scalars.
    #[must_use]
    pub fn contains(self, value: f32) -> bool {
        value.is_finite()
            && value + SCALAR_RANGE_TOLERANCE >= self.minimum
            && value - SCALAR_RANGE_TOLERANCE <= self.maximum
    }
}

/// Return a dependency-aware scalar range for an operation field.
///
/// The range is conservative and mirrors the cut generator's hard rejection
/// rules so UI controls, candidate search, and direct edit commands can avoid
/// creating compile-invalid recipes when boundary-loop bevels depend on a cut.
#[must_use]
pub fn feasible_operation_scalar_range(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    field: &str,
) -> Option<OperationScalarRange> {
    let definition_spec = recipe.definitions.get(&definition)?;
    let operation_spec = definition_spec
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)?;
    let host = operation_cut_face(operation_spec)
        .and_then(|face| cut_host_bounds_for_source(&definition_spec.geometry.source, face));
    match operation_spec {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            entry_loop,
            floor_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *floor_loop,
            );
            recessed_cut_scalar_range(
                field,
                host,
                RecessedCutScalars {
                    center: *center,
                    size: *size,
                    depth: *depth,
                    corner_radius: *corner_radius,
                    rim_width: *rim_width,
                },
                bevels,
            )
        }
        ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            corner_radius,
            rim_width,
            entry_loop,
            exit_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *exit_loop,
            );
            rectangular_through_cut_scalar_range(
                field,
                host,
                *center,
                *size,
                *corner_radius,
                *rim_width,
                bevels,
            )
        }
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            rim_width,
            entry_loop,
            exit_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *exit_loop,
            );
            circular_through_cut_scalar_range(field, host, *center, *radius, *rim_width, bevels)
        }
        ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            profile,
            ..
        } => match field {
            "bevel_boundary_loop.width" => {
                feasible_boundary_loop_bevel_width_range(recipe, definition, *target_loop)
            }
            "bevel_boundary_loop.profile" => OperationScalarRange::new(
                BOUNDARY_BEVEL_PROFILE_MIN.min(*profile),
                BOUNDARY_BEVEL_PROFILE_MAX.max(*profile),
            ),
            "bevel_boundary_loop.segments" => OperationScalarRange::new(1.0, 128.0),
            _ => None,
        },
        _ => None,
    }
}

/// Return the safe width range for adding or editing a boundary-loop bevel.
#[must_use]
pub fn feasible_boundary_loop_bevel_width_range(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    target_loop: BoundaryLoopId,
) -> Option<OperationScalarRange> {
    let definition_spec = recipe.definitions.get(&definition)?;
    let operations = &definition_spec.geometry.operations;
    for operation in operations {
        let host = operation_cut_face(operation)
            .and_then(|face| cut_host_bounds_for_source(&definition_spec.geometry.source, face));
        match operation {
            ModelingOperationSpec::RecessedPanelCut {
                size,
                depth,
                corner_radius,
                rim_width,
                entry_loop,
                floor_loop,
                ..
            } if *entry_loop == target_loop || *floor_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *floor_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(rect_loop_radius(*size) * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                maximum = maximum.min(*depth - sibling - CUT_SCALAR_SAFETY_MARGIN);
                if *floor_loop == target_loop && *corner_radius > 0.0 {
                    maximum = maximum.min(*corner_radius - CUT_SCALAR_SAFETY_MARGIN);
                }
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness * 0.45);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            ModelingOperationSpec::RectangularThroughCut {
                size,
                rim_width,
                entry_loop,
                exit_loop,
                ..
            } if *entry_loop == target_loop || *exit_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *exit_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(rect_loop_radius(*size) * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness - sibling - CUT_SCALAR_SAFETY_MARGIN);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            ModelingOperationSpec::CircularThroughCut {
                radius,
                rim_width,
                entry_loop,
                exit_loop,
                ..
            } if *entry_loop == target_loop || *exit_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *exit_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(radius.abs() * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness - sibling - CUT_SCALAR_SAFETY_MARGIN);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            _ => {}
        }
    }
    None
}

/// Return a dependency-aware range for descriptor-backed geometry scalars.
#[must_use]
pub fn feasible_scalar_path_range(
    recipe: &AssetRecipe,
    path: &str,
) -> Option<OperationScalarRange> {
    let parts = path.split('.').collect::<Vec<_>>();
    let ["definition", definition, "geometry", source, rest @ ..] = parts.as_slice() else {
        return None;
    };
    let definition = definition.parse().ok().map(PartDefinitionId)?;
    let definition_spec = recipe.definitions.get(&definition)?;
    match (*source, rest) {
        ("plate", ["thickness"]) => {
            let minimum = minimum_host_thickness_for_dependent_cuts(definition_spec);
            OperationScalarRange::new(minimum.max(0.001), f32::MAX)
        }
        ("rounded_box", ["radius"]) => {
            let GeometrySource::RoundedBox {
                half_extents,
                radius,
            } = definition_spec.geometry.source
            else {
                return None;
            };
            let maximum = rounded_box_radius_max_for_dependent_cuts(definition_spec)
                .min(half_extents[0].min(half_extents[1]).min(half_extents[2]))
                .max(radius);
            OperationScalarRange::new(0.0, maximum)
        }
        ("rounded_box", ["half_extents", component]) => {
            let GeometrySource::RoundedBox { half_extents, .. } = definition_spec.geometry.source
            else {
                return None;
            };
            let axis = axis_index(component)?;
            let minimum = rounded_box_half_extent_min_for_dependent_cuts(definition_spec, axis);
            OperationScalarRange::new(minimum.max(0.001), f32::MAX.max(half_extents[axis]))
        }
        _ => None,
    }
}

#[derive(Debug, Copy, Clone)]
struct CutHostBounds {
    half_size: [f32; 2],
    thickness: f32,
}

#[derive(Debug, Copy, Clone)]
struct CutLoopBevelWidths {
    entry: f32,
    secondary: f32,
}

impl CutLoopBevelWidths {
    fn for_loops(
        operations: &[ModelingOperationSpec],
        entry_loop: BoundaryLoopId,
        secondary_loop: BoundaryLoopId,
    ) -> Self {
        Self {
            entry: boundary_loop_bevel_width(operations, entry_loop),
            secondary: boundary_loop_bevel_width(operations, secondary_loop),
        }
    }

    fn maximum(self) -> f32 {
        self.entry.max(self.secondary)
    }

    fn combined(self) -> f32 {
        self.entry + self.secondary
    }
}

#[derive(Debug, Copy, Clone)]
struct RecessedCutScalars {
    center: [f32; 2],
    size: [f32; 2],
    depth: f32,
    corner_radius: f32,
    rim_width: f32,
}

fn recessed_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    scalars: RecessedCutScalars,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = rect_cut_ranges(host, scalars.center, scalars.size, scalars.rim_width);
    match field {
        "recessed_panel_cut.center.x" => Some(ranges.center_x),
        "recessed_panel_cut.center.y" => Some(ranges.center_y),
        "recessed_panel_cut.size.x" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_x_max.max(scalars.size[0].abs()),
        ),
        "recessed_panel_cut.size.y" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_y_max.max(scalars.size[1].abs()),
        ),
        "recessed_panel_cut.depth" => OperationScalarRange::new(
            (bevels.combined() + CUT_SCALAR_SAFETY_MARGIN).max(0.005),
            host.map_or(f32::MAX, |host| host.thickness * 0.95)
                .max(scalars.depth),
        ),
        "recessed_panel_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(scalars.rim_width),
        ),
        "recessed_panel_cut.corner_radius" => OperationScalarRange::new(
            recessed_corner_radius_min(scalars.corner_radius, bevels.secondary),
            (scalars.size[0].min(scalars.size[1]) * 0.5).max(scalars.corner_radius),
        ),
        "recessed_panel_cut.corner_segments" => OperationScalarRange::new(1.0, 128.0),
        _ => None,
    }
}

fn rectangular_through_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    rim_width: f32,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = rect_cut_ranges(host, center, size, rim_width);
    match field {
        "rectangular_through_cut.center.x" => Some(ranges.center_x),
        "rectangular_through_cut.center.y" => Some(ranges.center_y),
        "rectangular_through_cut.size.x" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_x_max.max(size[0].abs()),
        ),
        "rectangular_through_cut.size.y" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_y_max.max(size[1].abs()),
        ),
        "rectangular_through_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(rim_width),
        ),
        "rectangular_through_cut.corner_radius" => {
            OperationScalarRange::new(0.0, (size[0].min(size[1]) * 0.5).max(corner_radius))
        }
        "rectangular_through_cut.corner_segments" => OperationScalarRange::new(1.0, 128.0),
        _ => None,
    }
}

fn circular_through_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    center: [f32; 2],
    radius: f32,
    rim_width: f32,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = circular_cut_ranges(host, center, radius, rim_width);
    match field {
        "circular_through_cut.center.x" => Some(ranges.center_x),
        "circular_through_cut.center.y" => Some(ranges.center_y),
        "circular_through_cut.radius" => OperationScalarRange::new(
            (bevels.maximum() * 2.0 + CUT_SCALAR_SAFETY_MARGIN).max(0.01),
            ranges.radius_max.max(radius),
        ),
        "circular_through_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(rim_width),
        ),
        "circular_through_cut.radial_segments" => OperationScalarRange::new(6.0, 128.0),
        _ => None,
    }
}

fn operation_cut_face(operation: &ModelingOperationSpec) -> Option<PlanarCutFace> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut { face, .. }
        | ModelingOperationSpec::RectangularThroughCut { face, .. }
        | ModelingOperationSpec::CircularThroughCut { face, .. } => Some(*face),
        _ => None,
    }
}

fn cut_host_bounds_for_source(
    source: &GeometrySource,
    face: PlanarCutFace,
) -> Option<CutHostBounds> {
    match source {
        GeometrySource::Plate { size, thickness } => match face {
            PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => Some(CutHostBounds {
                half_size: [size[0].abs() * 0.5, size[1].abs() * 0.5],
                thickness: thickness.abs(),
            }),
            _ => None,
        },
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        } => {
            let usable = |axis: usize| (half_extents[axis].abs() - radius.max(0.0)).max(0.0);
            let (u_axis, v_axis, normal_axis) = match face {
                PlanarCutFace::PositiveX | PlanarCutFace::NegativeX => (2, 1, 0),
                PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => (0, 2, 1),
                PlanarCutFace::PositiveZ | PlanarCutFace::NegativeZ => (0, 1, 2),
            };
            Some(CutHostBounds {
                half_size: [usable(u_axis), usable(v_axis)],
                thickness: half_extents[normal_axis].abs() * 2.0,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Copy, Clone)]
struct RectCutRanges {
    center_x: OperationScalarRange,
    center_y: OperationScalarRange,
    size_x_max: f32,
    size_y_max: f32,
    rim_width_max: f32,
}

#[derive(Debug, Copy, Clone)]
struct CircularCutRanges {
    center_x: OperationScalarRange,
    center_y: OperationScalarRange,
    radius_max: f32,
    rim_width_max: f32,
}

fn rect_cut_ranges(
    host: Option<CutHostBounds>,
    center: [f32; 2],
    size: [f32; 2],
    rim_width: f32,
) -> RectCutRanges {
    let Some(host) = host else {
        return RectCutRanges {
            center_x: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            center_y: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            size_x_max: 4.0,
            size_y_max: 4.0,
            rim_width_max: 0.5,
        };
    };
    let half_cut = [size[0].abs() * 0.5, size[1].abs() * 0.5];
    let rim = rim_width.max(0.0);
    let clearance_x = (host.half_size[0] - center[0].abs() - rim).max(0.025);
    let clearance_y = (host.half_size[1] - center[1].abs() - rim).max(0.025);
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - half_cut[0]).max(0.001),
        (host.half_size[1] - center[1].abs() - half_cut[1]).max(0.001),
    ];
    RectCutRanges {
        center_x: ordered_scalar_range(
            -host.half_size[0] + half_cut[0] + rim,
            host.half_size[0] - half_cut[0] - rim,
            center[0],
        ),
        center_y: ordered_scalar_range(
            -host.half_size[1] + half_cut[1] + rim,
            host.half_size[1] - half_cut[1] - rim,
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
            center_x: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            center_y: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            radius_max: 2.0,
            rim_width_max: 0.5,
        };
    };
    let cut_radius = radius.abs();
    let rim = rim_width.max(0.0);
    let radius_clearance = [
        (host.half_size[0] - center[0].abs() - rim).max(0.01),
        (host.half_size[1] - center[1].abs() - rim).max(0.01),
    ];
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - cut_radius).max(0.001),
        (host.half_size[1] - center[1].abs() - cut_radius).max(0.001),
    ];
    CircularCutRanges {
        center_x: ordered_scalar_range(
            -host.half_size[0] + cut_radius + rim,
            host.half_size[0] - cut_radius - rim,
            center[0],
        ),
        center_y: ordered_scalar_range(
            -host.half_size[1] + cut_radius + rim,
            host.half_size[1] - cut_radius - rim,
            center[1],
        ),
        radius_max: radius_clearance[0]
            .min(radius_clearance[1])
            .max(cut_radius)
            .max(0.01),
        rim_width_max: rim_clearance[0].min(rim_clearance[1]).clamp(0.001, 0.5),
    }
}

fn ordered_scalar_range(minimum: f32, maximum: f32, current: f32) -> OperationScalarRange {
    if minimum <= maximum {
        OperationScalarRange { minimum, maximum }
    } else {
        OperationScalarRange {
            minimum: current,
            maximum: current,
        }
    }
}

fn boundary_loop_bevel_width(operations: &[ModelingOperationSpec], target: BoundaryLoopId) -> f32 {
    operations
        .iter()
        .find_map(|operation| match operation {
            ModelingOperationSpec::BevelBoundaryLoop {
                target_loop, width, ..
            } if *target_loop == target => Some(*width),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn rect_size_min(attached_bevel: f32) -> f32 {
    (attached_bevel * 4.0 + CUT_SCALAR_SAFETY_MARGIN).max(0.05)
}

fn rect_loop_radius(size: [f32; 2]) -> f32 {
    size[0].abs().min(size[1].abs()) * 0.25
}

fn recessed_corner_radius_min(current: f32, floor_bevel: f32) -> f32 {
    if current > 0.0 || floor_bevel > 0.0 {
        (floor_bevel + CUT_SCALAR_SAFETY_MARGIN).max(0.0)
    } else {
        0.0
    }
}

fn minimum_host_thickness_for_dependent_cuts(definition: &PartDefinition) -> f32 {
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| match operation {
            ModelingOperationSpec::RecessedPanelCut { depth, .. } => {
                Some(*depth + CUT_SCALAR_SAFETY_MARGIN)
            }
            ModelingOperationSpec::RectangularThroughCut {
                entry_loop,
                exit_loop,
                ..
            }
            | ModelingOperationSpec::CircularThroughCut {
                entry_loop,
                exit_loop,
                ..
            } => {
                let bevels = CutLoopBevelWidths::for_loops(
                    &definition.geometry.operations,
                    *entry_loop,
                    *exit_loop,
                );
                Some(bevels.combined() + CUT_SCALAR_SAFETY_MARGIN)
            }
            _ => None,
        })
        .fold(0.001, f32::max)
}

fn rounded_box_radius_max_for_dependent_cuts(definition: &PartDefinition) -> f32 {
    let GeometrySource::RoundedBox { half_extents, .. } = definition.geometry.source else {
        return f32::MAX;
    };
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| rounded_box_radius_max_for_cut(operation, half_extents))
        .fold(f32::MAX, f32::min)
}

fn rounded_box_radius_max_for_cut(
    operation: &ModelingOperationSpec,
    half_extents: [f32; 3],
) -> Option<f32> {
    let face = operation_cut_face(operation)?;
    let (u_axis, v_axis, _normal_axis) = rounded_box_face_axes(face);
    let required = cut_required_half_size(operation)?;
    Some(
        (half_extents[u_axis].abs() - required[0])
            .min(half_extents[v_axis].abs() - required[1])
            .max(0.0),
    )
}

fn rounded_box_half_extent_min_for_dependent_cuts(definition: &PartDefinition, axis: usize) -> f32 {
    let GeometrySource::RoundedBox { radius, .. } = definition.geometry.source else {
        return 0.001;
    };
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| {
            let face = operation_cut_face(operation)?;
            let (u_axis, v_axis, normal_axis) = rounded_box_face_axes(face);
            let required = cut_required_half_size(operation)?;
            if axis == u_axis {
                Some(radius + required[0])
            } else if axis == v_axis {
                Some(radius + required[1])
            } else if axis == normal_axis {
                Some(normal_half_extent_required(operation))
            } else {
                None
            }
        })
        .fold(radius.max(0.001), f32::max)
}

fn rounded_box_face_axes(face: PlanarCutFace) -> (usize, usize, usize) {
    match face {
        PlanarCutFace::PositiveX | PlanarCutFace::NegativeX => (2, 1, 0),
        PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => (0, 2, 1),
        PlanarCutFace::PositiveZ | PlanarCutFace::NegativeZ => (0, 1, 2),
    }
}

fn cut_required_half_size(operation: &ModelingOperationSpec) -> Option<[f32; 2]> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            rim_width,
            ..
        }
        | ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            rim_width,
            ..
        } => Some([
            center[0].abs() + size[0].abs() * 0.5 + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
            center[1].abs() + size[1].abs() * 0.5 + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
        ]),
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            rim_width,
            ..
        } => Some([
            center[0].abs() + radius.abs() + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
            center[1].abs() + radius.abs() + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
        ]),
        _ => None,
    }
}

fn normal_half_extent_required(operation: &ModelingOperationSpec) -> f32 {
    match operation {
        ModelingOperationSpec::RecessedPanelCut { depth, .. } => {
            (*depth + CUT_SCALAR_SAFETY_MARGIN).max(0.001)
        }
        ModelingOperationSpec::RectangularThroughCut { .. }
        | ModelingOperationSpec::CircularThroughCut { .. } => CUT_SCALAR_SAFETY_MARGIN.max(0.001),
        _ => 0.001,
    }
}

fn axis_index(component: &str) -> Option<usize> {
    match component {
        "x" => Some(0),
        "y" => Some(1),
        "z" => Some(2),
        _ => None,
    }
}

#[derive(Deserialize)]
enum ModelingOperationSpecWire {
    TransformGeometry {
        operation: OperationId,
        transform: Transform3,
    },
    SetBevelProfile {
        operation: OperationId,
        radius: f32,
        segments: u32,
    },
    AddPanel {
        operation: OperationId,
        region: RegionId,
        inset: f32,
        depth: f32,
    },
    AddTrim {
        operation: OperationId,
        region: RegionId,
        width: f32,
        height: f32,
    },
    RecessedPanelCut(RecessedPanelCutWire),
    RectangularThroughCut(RectangularThroughCutWire),
    CircularThroughCut(CircularThroughCutWire),
    BevelBoundaryLoop {
        operation: OperationId,
        target_loop: BoundaryLoopId,
        width: f32,
        segments: u32,
        profile: f32,
        bevel_region: RegionId,
        outer_replacement_loop: BoundaryLoopId,
        inner_replacement_loop: BoundaryLoopId,
    },
    MirrorInstances {
        operation: OperationId,
        plane_normal: [f32; 3],
        plane_offset: f32,
    },
    LinearArray {
        operation: OperationId,
        count: u32,
        offset: [f32; 3],
    },
    RadialArray {
        operation: OperationId,
        count: u32,
        axis: [f32; 3],
        angle_degrees: f32,
    },
    ReservedBoolean {
        operation: OperationId,
        label: String,
    },
    ReservedDeformationProgram {
        operation: OperationId,
        label: String,
    },
}

#[derive(Deserialize)]
struct RecessedPanelCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    size: [f32; 2],
    depth: f32,
    corner_radius: f32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    corner_segments: Option<u32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    floor_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    floor_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}

#[derive(Deserialize)]
struct RectangularThroughCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    corner_segments: Option<u32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    exit_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}

#[derive(Deserialize)]
struct CircularThroughCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    radius: f32,
    radial_segments: u32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    exit_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}
