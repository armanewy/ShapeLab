
impl ModelingOperationSpecWire {
    fn into_operation(self, schema_version: u32) -> Result<ModelingOperationSpec, String> {
        Ok(match self {
            Self::TransformGeometry {
                operation,
                transform,
            } => ModelingOperationSpec::TransformGeometry {
                operation,
                transform,
            },
            Self::SetBevelProfile {
                operation,
                radius,
                segments,
            } => ModelingOperationSpec::SetBevelProfile {
                operation,
                radius,
                segments,
            },
            Self::AddPanel {
                operation,
                region,
                inset,
                depth,
            } => ModelingOperationSpec::AddPanel {
                operation,
                region,
                inset,
                depth,
            },
            Self::AddTrim {
                operation,
                region,
                width,
                height,
            } => ModelingOperationSpec::AddTrim {
                operation,
                region,
                width,
                height,
            },
            Self::RecessedPanelCut(wire) => ModelingOperationSpec::RecessedPanelCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                size: wire.size,
                depth: wire.depth,
                corner_radius: wire.corner_radius,
                rim_width: required_or_legacy_rect_rim_width(
                    wire.rim_width,
                    wire.size,
                    "RecessedPanelCut.rim_width",
                    schema_version,
                )?,
                corner_segments: required_or_legacy_corner_segments(
                    wire.corner_segments,
                    "RecessedPanelCut.corner_segments",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "RecessedPanelCut.entry_loop",
                    schema_version,
                )?,
                floor_loop: legacy_or_required_secondary_loop(
                    wire.floor_loop,
                    wire.boundary_loop,
                    "RecessedPanelCut.floor_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                floor_region: wire.floor_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::RectangularThroughCut(wire) => ModelingOperationSpec::RectangularThroughCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                size: wire.size,
                corner_radius: wire.corner_radius,
                rim_width: required_or_legacy_rect_rim_width(
                    wire.rim_width,
                    wire.size,
                    "RectangularThroughCut.rim_width",
                    schema_version,
                )?,
                corner_segments: required_or_legacy_corner_segments(
                    wire.corner_segments,
                    "RectangularThroughCut.corner_segments",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "RectangularThroughCut.entry_loop",
                    schema_version,
                )?,
                exit_loop: legacy_or_required_secondary_loop(
                    wire.exit_loop,
                    wire.boundary_loop,
                    "RectangularThroughCut.exit_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::CircularThroughCut(wire) => ModelingOperationSpec::CircularThroughCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                radius: wire.radius,
                radial_segments: wire.radial_segments,
                rim_width: required_or_legacy_circular_rim_width(
                    wire.rim_width,
                    wire.radius,
                    "CircularThroughCut.rim_width",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "CircularThroughCut.entry_loop",
                    schema_version,
                )?,
                exit_loop: legacy_or_required_secondary_loop(
                    wire.exit_loop,
                    wire.boundary_loop,
                    "CircularThroughCut.exit_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
            } => ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
            },
            Self::MirrorInstances {
                operation,
                plane_normal,
                plane_offset,
            } => ModelingOperationSpec::MirrorInstances {
                operation,
                plane_normal,
                plane_offset,
            },
            Self::LinearArray {
                operation,
                count,
                offset,
            } => ModelingOperationSpec::LinearArray {
                operation,
                count,
                offset,
            },
            Self::RadialArray {
                operation,
                count,
                axis,
                angle_degrees,
            } => ModelingOperationSpec::RadialArray {
                operation,
                count,
                axis,
                angle_degrees,
            },
            Self::ReservedBoolean { operation, label } => {
                ModelingOperationSpec::ReservedBoolean { operation, label }
            }
            Self::ReservedDeformationProgram { operation, label } => {
                ModelingOperationSpec::ReservedDeformationProgram { operation, label }
            }
        })
    }
}

impl<'de> Deserialize<'de> for ModelingOperationSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ModelingOperationSpecWire::deserialize(deserializer)?
            .into_operation(ASSET_RECIPE_SCHEMA_VERSION)
            .map_err(de::Error::custom)
    }
}

fn required_current_field<T>(current: Option<T>, field: &'static str) -> Result<T, String> {
    current.ok_or_else(|| format!("{field} is missing"))
}

fn legacy_boundary_loop_schema(schema_version: u32) -> bool {
    matches!(schema_version, 1..=3)
}

fn legacy_rim_field_schema(schema_version: u32) -> bool {
    matches!(schema_version, 1..=4)
}

fn required_or_legacy_loop(
    current: Option<BoundaryLoopId>,
    legacy: Option<BoundaryLoopId>,
    field: &'static str,
    schema_version: u32,
) -> Result<BoundaryLoopId, String> {
    if legacy_boundary_loop_schema(schema_version) {
        current
            .or(legacy)
            .ok_or_else(|| format!("{field} is missing"))
    } else {
        required_current_field(current, field)
    }
}

fn legacy_or_required_secondary_loop(
    current: Option<BoundaryLoopId>,
    legacy: Option<BoundaryLoopId>,
    field: &'static str,
    schema_version: u32,
) -> Result<BoundaryLoopId, String> {
    if legacy_boundary_loop_schema(schema_version) {
        if let Some(current) = current {
            Ok(current)
        } else if legacy.is_some() {
            Ok(LEGACY_MISSING_BOUNDARY_LOOP)
        } else {
            Err(format!("{field} is missing"))
        }
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_rect_rim_width(
    current: Option<f32>,
    size: [f32; 2],
    field: &'static str,
    schema_version: u32,
) -> Result<f32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or_else(|| default_rect_cut_rim_width(size)))
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_circular_rim_width(
    current: Option<f32>,
    radius: f32,
    field: &'static str,
    schema_version: u32,
) -> Result<f32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or_else(|| default_circular_cut_rim_width(radius)))
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_corner_segments(
    current: Option<u32>,
    field: &'static str,
    schema_version: u32,
) -> Result<u32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or(DEFAULT_RECT_CUT_CORNER_SEGMENTS))
    } else {
        required_current_field(current, field)
    }
}

fn default_rect_cut_rim_width(size: [f32; 2]) -> f32 {
    size[0].min(size[1]).max(0.0) * 0.16
}

fn default_circular_cut_rim_width(radius: f32) -> f32 {
    radius.max(0.0) * 2.0 * 0.16
}
