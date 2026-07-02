
fn get_definition_scalar(
    definition: &PartDefinition,
    rest: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match rest {
        ["geometry", source_kind, field @ ..] => {
            get_geometry_source_scalar(&definition.geometry.source, source_kind, field, path)
        }
        ["operation", operation_id, field @ ..] => {
            let operation_id = parse_id(operation_id, path).map(OperationId)?;
            let operation = definition
                .geometry
                .operations
                .iter()
                .find(|operation| operation.operation_id() == operation_id)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            get_operation_scalar(operation, field, path)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_geometry_source_scalar(
    source: &GeometrySource,
    source_kind: &str,
    field: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match (source, source_kind, field) {
        (
            GeometrySource::RoundedBox {
                half_extents,
                radius: _,
            },
            "rounded_box",
            ["half_extents", component],
        ) => component_value(half_extents, component, path),
        (GeometrySource::RoundedBox { radius, .. }, "rounded_box", ["radius"]) => Ok(*radius),
        (GeometrySource::Cylinder { radius, .. }, "cylinder", ["radius"]) => Ok(*radius),
        (GeometrySource::Cylinder { height, .. }, "cylinder", ["height"]) => Ok(*height),
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            "cylinder",
            ["radial_segments"],
        ) => Ok(*radial_segments as f32),
        (GeometrySource::Frustum { bottom_radius, .. }, "frustum", ["bottom_radius"]) => {
            Ok(*bottom_radius)
        }
        (GeometrySource::Frustum { top_radius, .. }, "frustum", ["top_radius"]) => Ok(*top_radius),
        (GeometrySource::Frustum { height, .. }, "frustum", ["height"]) => Ok(*height),
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            "frustum",
            ["radial_segments"],
        ) => Ok(*radial_segments as f32),
        (GeometrySource::Plate { size, .. }, "plate", ["size", component]) => {
            component_value(size, component, path)
        }
        (GeometrySource::Plate { thickness, .. }, "plate", ["thickness"]) => Ok(*thickness),
        (GeometrySource::Sweep { profile, .. }, "sweep", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            component_value(point, component, path)
        }
        (
            GeometrySource::Sweep { path: frames, .. },
            "sweep",
            ["path", index, frame_field, component],
        ) => {
            let index = parse_index(index, path)?;
            let frame = frames
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            get_frame_scalar(frame, frame_field, component, path)
        }
        (GeometrySource::Lathe { profile, .. }, "lathe", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            component_value(point, component, path)
        }
        (GeometrySource::Lathe { segments, .. }, "lathe", ["segments"]) => Ok(*segments as f32),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_operation_scalar(
    operation: &ModelingOperationSpec,
    field: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match (operation, field) {
        (ModelingOperationSpec::SetBevelProfile { radius, .. }, ["bevel", "radius"]) => Ok(*radius),
        (ModelingOperationSpec::SetBevelProfile { segments, .. }, ["bevel", "segments"]) => {
            Ok(*segments as f32)
        }
        (ModelingOperationSpec::AddPanel { inset, .. }, ["panel", "inset"]) => Ok(*inset),
        (ModelingOperationSpec::AddPanel { depth, .. }, ["panel", "depth"]) => Ok(*depth),
        (ModelingOperationSpec::AddTrim { width, .. }, ["trim", "width"]) => Ok(*width),
        (ModelingOperationSpec::AddTrim { height, .. }, ["trim", "height"]) => Ok(*height),
        (
            ModelingOperationSpec::RecessedPanelCut { size, .. },
            ["recessed_panel_cut", "size", component],
        ) => component_value(size, component, path),
        (
            ModelingOperationSpec::RecessedPanelCut { center, .. },
            ["recessed_panel_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::RecessedPanelCut { depth, .. },
            ["recessed_panel_cut", "depth"],
        ) => Ok(*depth),
        (
            ModelingOperationSpec::RecessedPanelCut { corner_radius, .. },
            ["recessed_panel_cut", "corner_radius"],
        ) => Ok(*corner_radius),
        (
            ModelingOperationSpec::RecessedPanelCut { rim_width, .. },
            ["recessed_panel_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::RecessedPanelCut {
                corner_segments, ..
            },
            ["recessed_panel_cut", "corner_segments"],
        ) => Ok(*corner_segments as f32),
        (
            ModelingOperationSpec::RectangularThroughCut { size, .. },
            ["rectangular_through_cut", "size", component],
        ) => component_value(size, component, path),
        (
            ModelingOperationSpec::RectangularThroughCut { center, .. },
            ["rectangular_through_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::RectangularThroughCut { corner_radius, .. },
            ["rectangular_through_cut", "corner_radius"],
        ) => Ok(*corner_radius),
        (
            ModelingOperationSpec::RectangularThroughCut { rim_width, .. },
            ["rectangular_through_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::RectangularThroughCut {
                corner_segments, ..
            },
            ["rectangular_through_cut", "corner_segments"],
        ) => Ok(*corner_segments as f32),
        (
            ModelingOperationSpec::CircularThroughCut { center, .. },
            ["circular_through_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::CircularThroughCut { radius, .. },
            ["circular_through_cut", "radius"],
        ) => Ok(*radius),
        (
            ModelingOperationSpec::CircularThroughCut {
                radial_segments, ..
            },
            ["circular_through_cut", "radial_segments"],
        ) => Ok(*radial_segments as f32),
        (
            ModelingOperationSpec::CircularThroughCut { rim_width, .. },
            ["circular_through_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::BevelBoundaryLoop { width, .. },
            ["bevel_boundary_loop", "width"],
        ) => Ok(*width),
        (
            ModelingOperationSpec::BevelBoundaryLoop { segments, .. },
            ["bevel_boundary_loop", "segments"],
        ) => Ok(*segments as f32),
        (
            ModelingOperationSpec::BevelBoundaryLoop { profile, .. },
            ["bevel_boundary_loop", "profile"],
        ) => Ok(*profile),
        (ModelingOperationSpec::LinearArray { count, .. }, ["linear_array", "count"]) => {
            Ok(*count as f32)
        }
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ["linear_array", "offset", component],
        ) => component_value(offset, component, path),
        (ModelingOperationSpec::RadialArray { count, .. }, ["radial_array", "count"]) => {
            Ok(*count as f32)
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ["radial_array", "axis", component]) => {
            component_value(axis, component, path)
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ["radial_array", "angle_degrees"],
        ) => Ok(*angle_degrees),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_transform_scalar(
    transform: &Transform3,
    rest: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match rest {
        ["transform", "translation", component] => {
            component_value(&transform.translation, component, path)
        }
        ["transform", "rotation_degrees", component] => {
            component_value(&transform.rotation_degrees, component, path)
        }
        ["transform", "scale", component] => component_value(&transform.scale, component, path),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_frame_scalar(
    frame: &Frame3,
    frame_field: &str,
    component: &str,
    path: &str,
) -> Result<f32, AssetError> {
    match frame_field {
        "origin" => component_value(&frame.origin, component, path),
        "x_axis" => component_value(&frame.x_axis, component, path),
        "y_axis" => component_value(&frame.y_axis, component, path),
        "z_axis" => component_value(&frame.z_axis, component, path),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_scalar_in_place(recipe: &mut AssetRecipe, path: &str, value: f32) -> Result<(), AssetError> {
    let parts = path.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        ["definition", id, rest @ ..] => {
            let definition_id = parse_id(id, path).map(PartDefinitionId)?;
            let definition = recipe
                .definitions
                .get_mut(&definition_id)
                .ok_or(AssetError::UnknownDefinition(definition_id))?;
            set_definition_scalar(definition, rest, path, value)
        }
        ["instance", id, rest @ ..] => {
            let instance_id = parse_id(id, path).map(PartInstanceId)?;
            let instance = recipe
                .instances
                .get_mut(&instance_id)
                .ok_or(AssetError::UnknownInstance(instance_id))?;
            set_transform_scalar(&mut instance.local_transform, rest, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_definition_scalar(
    definition: &mut PartDefinition,
    rest: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match rest {
        ["geometry", source_kind, field @ ..] => set_geometry_source_scalar(
            &mut definition.geometry.source,
            source_kind,
            field,
            path,
            value,
        ),
        ["operation", operation_id, field @ ..] => {
            let operation_id = parse_id(operation_id, path).map(OperationId)?;
            let operation = definition
                .geometry
                .operations
                .iter_mut()
                .find(|operation| operation.operation_id() == operation_id)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_operation_scalar(operation, field, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_geometry_source_scalar(
    source: &mut GeometrySource,
    source_kind: &str,
    field: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match (source, source_kind, field) {
        (
            GeometrySource::RoundedBox { half_extents, .. },
            "rounded_box",
            ["half_extents", component],
        ) => set_component_value(half_extents, component, path, value),
        (GeometrySource::RoundedBox { radius, .. }, "rounded_box", ["radius"]) => {
            *radius = value;
            Ok(())
        }
        (GeometrySource::Cylinder { radius, .. }, "cylinder", ["radius"]) => {
            *radius = value;
            Ok(())
        }
        (GeometrySource::Cylinder { height, .. }, "cylinder", ["height"]) => {
            *height = value;
            Ok(())
        }
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            "cylinder",
            ["radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (GeometrySource::Frustum { bottom_radius, .. }, "frustum", ["bottom_radius"]) => {
            *bottom_radius = value;
            Ok(())
        }
        (GeometrySource::Frustum { top_radius, .. }, "frustum", ["top_radius"]) => {
            *top_radius = value;
            Ok(())
        }
        (GeometrySource::Frustum { height, .. }, "frustum", ["height"]) => {
            *height = value;
            Ok(())
        }
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            "frustum",
            ["radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (GeometrySource::Plate { size, .. }, "plate", ["size", component]) => {
            set_component_value(size, component, path, value)
        }
        (GeometrySource::Plate { thickness, .. }, "plate", ["thickness"]) => {
            *thickness = value;
            Ok(())
        }
        (GeometrySource::Sweep { profile, .. }, "sweep", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_component_value(point, component, path, value)
        }
        (
            GeometrySource::Sweep { path: frames, .. },
            "sweep",
            ["path", index, frame_field, component],
        ) => {
            let index = parse_index(index, path)?;
            let frame = frames
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_frame_scalar(frame, frame_field, component, path, value)
        }
        (GeometrySource::Lathe { profile, .. }, "lathe", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_component_value(point, component, path, value)
        }
        (GeometrySource::Lathe { segments, .. }, "lathe", ["segments"]) => {
            *segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_operation_scalar(
    operation: &mut ModelingOperationSpec,
    field: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match (operation, field) {
        (ModelingOperationSpec::SetBevelProfile { radius, .. }, ["bevel", "radius"]) => {
            *radius = value;
            Ok(())
        }
        (ModelingOperationSpec::SetBevelProfile { segments, .. }, ["bevel", "segments"]) => {
            *segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (ModelingOperationSpec::AddPanel { inset, .. }, ["panel", "inset"]) => {
            *inset = value;
            Ok(())
        }
        (ModelingOperationSpec::AddPanel { depth, .. }, ["panel", "depth"]) => {
            *depth = value;
            Ok(())
        }
        (ModelingOperationSpec::AddTrim { width, .. }, ["trim", "width"]) => {
            *width = value;
            Ok(())
        }
        (ModelingOperationSpec::AddTrim { height, .. }, ["trim", "height"]) => {
            *height = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut { size, .. },
            ["recessed_panel_cut", "size", component],
        ) => set_component_value(size, component, path, value),
        (
            ModelingOperationSpec::RecessedPanelCut { center, .. },
            ["recessed_panel_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::RecessedPanelCut { depth, .. },
            ["recessed_panel_cut", "depth"],
        ) => {
            *depth = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut { corner_radius, .. },
            ["recessed_panel_cut", "corner_radius"],
        ) => {
            *corner_radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut { rim_width, .. },
            ["recessed_panel_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut {
                corner_segments, ..
            },
            ["recessed_panel_cut", "corner_segments"],
        ) => {
            *corner_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut { size, .. },
            ["rectangular_through_cut", "size", component],
        ) => set_component_value(size, component, path, value),
        (
            ModelingOperationSpec::RectangularThroughCut { center, .. },
            ["rectangular_through_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::RectangularThroughCut { corner_radius, .. },
            ["rectangular_through_cut", "corner_radius"],
        ) => {
            *corner_radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut { rim_width, .. },
            ["rectangular_through_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut {
                corner_segments, ..
            },
            ["rectangular_through_cut", "corner_segments"],
        ) => {
            *corner_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut { center, .. },
            ["circular_through_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::CircularThroughCut { radius, .. },
            ["circular_through_cut", "radius"],
        ) => {
            *radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut {
                radial_segments, ..
            },
            ["circular_through_cut", "radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut { rim_width, .. },
            ["circular_through_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { width, .. },
            ["bevel_boundary_loop", "width"],
        ) => {
            *width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { segments, .. },
            ["bevel_boundary_loop", "segments"],
        ) => {
            *segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { profile, .. },
            ["bevel_boundary_loop", "profile"],
        ) => {
            *profile = value;
            Ok(())
        }
        (ModelingOperationSpec::LinearArray { count, .. }, ["linear_array", "count"]) => {
            *count = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ["linear_array", "offset", component],
        ) => set_component_value(offset, component, path, value),
        (ModelingOperationSpec::RadialArray { count, .. }, ["radial_array", "count"]) => {
            *count = scalar_to_u32(path, value)?;
            Ok(())
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ["radial_array", "axis", component]) => {
            set_component_value(axis, component, path, value)
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ["radial_array", "angle_degrees"],
        ) => {
            *angle_degrees = value;
            Ok(())
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_transform_scalar(
    transform: &mut Transform3,
    rest: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match rest {
        ["transform", "translation", component] => {
            set_component_value(&mut transform.translation, component, path, value)
        }
        ["transform", "rotation_degrees", component] => {
            set_component_value(&mut transform.rotation_degrees, component, path, value)
        }
        ["transform", "scale", component] => {
            set_component_value(&mut transform.scale, component, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_frame_scalar(
    frame: &mut Frame3,
    frame_field: &str,
    component: &str,
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match frame_field {
        "origin" => set_component_value(&mut frame.origin, component, path, value),
        "x_axis" => set_component_value(&mut frame.x_axis, component, path, value),
        "y_axis" => set_component_value(&mut frame.y_axis, component, path, value),
        "z_axis" => set_component_value(&mut frame.z_axis, component, path, value),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}
