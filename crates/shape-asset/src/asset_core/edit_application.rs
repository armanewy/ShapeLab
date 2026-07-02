
fn apply_edit(recipe: &mut AssetRecipe, edit: &AssetEdit) -> Result<(), AssetError> {
    match edit {
        AssetEdit::SetScalar { parameter, value } => {
            if recipe.locks.contains(parameter) {
                return Err(AssetError::LockedParameter(*parameter));
            }
            let descriptor = recipe
                .parameters
                .get(parameter)
                .ok_or(AssetError::UnknownParameter(*parameter))?;
            if !parameter_range_is_valid(descriptor) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "invalid parameter descriptor {parameter:?}"
                )));
            }
            if *value < descriptor.minimum || *value > descriptor.maximum {
                return Err(AssetError::InvalidScalarValue {
                    path: descriptor.path.clone(),
                    value: *value,
                    reason: "value is outside the parameter range",
                });
            }
            if let Some(definition) = edits::definition_id_from_scalar_path(&descriptor.path)
                && descriptor.topology_changing
            {
                edits::ensure_topology_editable(recipe, definition)?;
            }
            if let Some(instance) = edits::instance_id_from_scalar_path(&descriptor.path) {
                edits::ensure_instance_editable(recipe, instance)?;
            }
            let path = descriptor.path.clone();
            set_scalar(recipe, path, *value)
        }
        AssetEdit::SetOperationScalar {
            definition,
            operation,
            field,
            value,
        } => set_modeling_operation_scalar(recipe, *definition, *operation, field, *value),
        AssetEdit::SetTransform {
            instance,
            transform,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.local_transform = transform.clone();
            Ok(())
        }
        AssetEdit::SetEnabled { instance, enabled } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.enabled = *enabled;
            Ok(())
        }
        AssetEdit::SetOptionalPartEnabled { instance, enabled } => {
            if !recipe.variation.optional_instances.contains(instance) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "instance {instance:?} is not optional"
                )));
            }
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.enabled = *enabled;
            Ok(())
        }
        AssetEdit::SetGeneratorDimension {
            definition,
            dimension,
        } => set_generator_dimension(recipe, *definition, dimension),
        AssetEdit::ReplaceGeometrySource { definition, source } => {
            edits::ensure_topology_editable(recipe, *definition)?;
            let target = recipe
                .definitions
                .get_mut(definition)
                .ok_or(AssetError::UnknownDefinition(*definition))?;
            target.geometry.source = source.clone();
            Ok(())
        }
        AssetEdit::SetBevelSettings {
            definition,
            operation,
            radius,
            segments,
        } => set_bevel_settings(recipe, *definition, *operation, *radius, *segments),
        AssetEdit::SetSweepProfilePoint {
            definition,
            index,
            point,
        } => set_sweep_profile_point(recipe, *definition, *index, *point),
        AssetEdit::SetSweepPathFrame {
            definition,
            index,
            frame,
        } => set_sweep_path_frame(recipe, *definition, *index, frame),
        AssetEdit::SetLatheProfilePoint {
            definition,
            index,
            point,
        } => set_lathe_profile_point(recipe, *definition, *index, *point),
        AssetEdit::AddInstance { instance } => {
            if recipe.instances.contains_key(&instance.id) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "duplicate instance {:?}",
                    instance.id
                )));
            }
            if let Some(parent) = instance.parent {
                edits::ensure_instance_editable(recipe, parent)?;
            }
            recipe.instances.insert(instance.id, instance.clone());
            if instance.parent.is_none() {
                insert_root_instance(recipe, instance.id);
            }
            bump_next_ids_for_instance(recipe, instance);
            Ok(())
        }
        AssetEdit::RemoveInstance { instance } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let descendants = descendants_of(recipe, *instance)?;
            if !descendants.is_empty() {
                return Err(AssetError::UnsupportedEdit(format!(
                    "cannot remove {instance:?} while descendants exist"
                )));
            }
            recipe
                .instances
                .remove(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            recipe.root_instances.retain(|root| root != instance);
            recipe.variation.optional_instances.remove(instance);
            Ok(())
        }
        AssetEdit::ReplaceDefinition { definition } => {
            if let Some(existing) = recipe.definitions.get(&definition.id)
                && edits::definition_topology_signature(existing)
                    != edits::definition_topology_signature(definition)
            {
                edits::ensure_topology_editable(recipe, definition.id)?;
            }
            recipe.definitions.insert(definition.id, definition.clone());
            bump_next_ids_for_definition(recipe, definition);
            Ok(())
        }
        AssetEdit::InsertModelingOperation {
            definition,
            index,
            operation,
        } => insert_modeling_operation(recipe, *definition, *index, operation),
        AssetEdit::RemoveModelingOperation {
            definition,
            operation,
            policy,
        } => remove_modeling_operation(recipe, *definition, *operation, *policy),
        AssetEdit::DuplicateCutOperation {
            definition,
            source,
            operation,
            entry_loop,
            secondary_loop,
            rim_region,
            wall_region,
            floor_region,
            center_offset,
            group_membership,
            dependent_bevels,
        } => duplicate_cut_operation(
            recipe,
            *definition,
            *source,
            DuplicateCutSpec {
                operation: *operation,
                entry_loop: *entry_loop,
                secondary_loop: *secondary_loop,
                rim_region: *rim_region,
                wall_region: *wall_region,
                floor_region: *floor_region,
                center_offset: *center_offset,
                group_membership: group_membership.clone(),
                dependent_bevels: dependent_bevels.clone(),
            },
        ),
        AssetEdit::MoveModelingOperation {
            definition,
            operation,
            new_index,
        } => move_modeling_operation(recipe, *definition, *operation, *new_index),
        AssetEdit::ReplaceInstanceDefinition {
            instance,
            definition,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let current_definition = recipe
                .instances
                .get(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?
                .definition;
            edits::ensure_topology_editable(recipe, current_definition)?;
            edits::ensure_compatible_replacement(recipe, current_definition, *definition)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.definition = *definition;
            Ok(())
        }
        AssetEdit::SetArrayCount {
            definition,
            operation,
            count,
        } => set_array_count(recipe, *definition, *operation, *count),
        AssetEdit::SetArraySpacing {
            definition,
            operation,
            spacing,
        } => set_array_spacing(recipe, *definition, *operation, spacing),
        AssetEdit::DuplicateInstance {
            source,
            instance,
            name,
            transform,
        } => duplicate_instance(
            recipe,
            *source,
            *instance,
            name.as_deref(),
            transform.as_ref(),
        ),
        AssetEdit::MirrorInstance {
            source,
            instance,
            plane,
            name,
        } => mirror_instance(recipe, *source, *instance, plane, name.as_deref()),
        AssetEdit::Attach {
            instance,
            attachment,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            if recipe.instances.contains_key(&attachment.parent_instance) {
                edits::ensure_instance_editable(recipe, attachment.parent_instance)?;
            }
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.attachment = Some(attachment.clone());
            target.parent = Some(attachment.parent_instance);
            recipe.root_instances.retain(|root| root != instance);
            Ok(())
        }
        AssetEdit::Detach { instance } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let previous_parent = recipe
                .instances
                .get(instance)
                .and_then(|target| target.parent);
            if let Some(parent) = previous_parent {
                edits::ensure_instance_editable(recipe, parent)?;
            }
            {
                let target = recipe
                    .instances
                    .get_mut(instance)
                    .ok_or(AssetError::UnknownInstance(*instance))?;
                target.attachment = None;
                target.parent = None;
            }
            insert_root_instance(recipe, *instance);
            Ok(())
        }
        AssetEdit::SetLock { parameter, locked } => {
            if !recipe.parameters.contains_key(parameter) {
                return Err(AssetError::UnknownParameter(*parameter));
            }
            if *locked {
                recipe.locks.insert(*parameter);
            } else {
                recipe.locks.remove(parameter);
            }
            Ok(())
        }
        AssetEdit::SetInstanceLock { instance, locked } => {
            if !recipe.instances.contains_key(instance) {
                return Err(AssetError::UnknownInstance(*instance));
            }
            if *locked {
                recipe.instance_locks.insert(*instance);
            } else {
                recipe.instance_locks.remove(instance);
            }
            Ok(())
        }
        AssetEdit::SetSubtreeLock { instance, locked } => {
            if !recipe.instances.contains_key(instance) {
                return Err(AssetError::UnknownInstance(*instance));
            }
            if *locked {
                recipe.subtree_locks.insert(*instance);
            } else {
                recipe.subtree_locks.remove(instance);
            }
            Ok(())
        }
        AssetEdit::SetTopologyLock { definition, locked } => {
            if !recipe.definitions.contains_key(definition) {
                return Err(AssetError::UnknownDefinition(*definition));
            }
            if *locked {
                recipe.topology_locks.insert(*definition);
            } else {
                recipe.topology_locks.remove(definition);
            }
            Ok(())
        }
        AssetEdit::ReorderChildInstances {
            parent,
            ordered_children,
        } => reorder_child_instances(recipe, *parent, ordered_children),
    }
}

fn set_generator_dimension(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    dimension: &GeneratorDimensionEdit,
) -> Result<(), AssetError> {
    if dimension.topology_changing() {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    match (&mut definition_ref.geometry.source, dimension) {
        (
            GeometrySource::RoundedBox { half_extents, .. },
            GeneratorDimensionEdit::RoundedBoxHalfExtents(value),
        ) => {
            *half_extents = *value;
            Ok(())
        }
        (
            GeometrySource::RoundedBox { radius, .. },
            GeneratorDimensionEdit::RoundedBoxRadius(value),
        ) => {
            *radius = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder { radius, .. },
            GeneratorDimensionEdit::CylinderRadius(value),
        ) => {
            *radius = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder { height, .. },
            GeneratorDimensionEdit::CylinderHeight(value),
        ) => {
            *height = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            GeneratorDimensionEdit::CylinderRadialSegments(value),
        ) => {
            *radial_segments = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum { bottom_radius, .. },
            GeneratorDimensionEdit::FrustumBottomRadius(value),
        ) => {
            *bottom_radius = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum { top_radius, .. },
            GeneratorDimensionEdit::FrustumTopRadius(value),
        ) => {
            *top_radius = *value;
            Ok(())
        }
        (GeometrySource::Frustum { height, .. }, GeneratorDimensionEdit::FrustumHeight(value)) => {
            *height = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            GeneratorDimensionEdit::FrustumRadialSegments(value),
        ) => {
            *radial_segments = *value;
            Ok(())
        }
        (GeometrySource::Plate { size, .. }, GeneratorDimensionEdit::PlateSize(value)) => {
            *size = *value;
            Ok(())
        }
        (
            GeometrySource::Plate { thickness, .. },
            GeneratorDimensionEdit::PlateThickness(value),
        ) => {
            *thickness = *value;
            Ok(())
        }
        (GeometrySource::Lathe { segments, .. }, GeneratorDimensionEdit::LatheSegments(value)) => {
            *segments = *value;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "dimension edit does not match definition {definition:?}"
        ))),
    }
}

fn set_bevel_settings(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    radius: Option<f32>,
    segments: Option<u32>,
) -> Result<(), AssetError> {
    if segments.is_some() {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_ref = definition_ref
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown bevel operation {operation:?}"))
        })?;
    let ModelingOperationSpec::SetBevelProfile {
        radius: target_radius,
        segments: target_segments,
        ..
    } = operation_ref
    else {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation {operation:?} is not a bevel"
        )));
    };
    if let Some(radius) = radius {
        *target_radius = radius;
    }
    if let Some(segments) = segments {
        *target_segments = segments;
    }
    Ok(())
}

fn set_sweep_profile_point(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    point: [f32; 2],
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Sweep { profile, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a sweep"
        )));
    };
    let target = profile.get_mut(index).ok_or_else(|| {
        AssetError::UnsupportedEdit(format!("unknown sweep profile index {index}"))
    })?;
    *target = point;
    Ok(())
}

fn set_sweep_path_frame(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    frame: &Frame3,
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Sweep { path, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a sweep"
        )));
    };
    let target = path
        .get_mut(index)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown sweep path index {index}")))?;
    *target = frame.clone();
    Ok(())
}

fn set_lathe_profile_point(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    point: [f32; 2],
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Lathe { profile, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a lathe"
        )));
    };
    let target = profile.get_mut(index).ok_or_else(|| {
        AssetError::UnsupportedEdit(format!("unknown lathe profile index {index}"))
    })?;
    *target = point;
    Ok(())
}

fn set_array_count(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    count: u32,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    if let Some(range) = recipe.variation.count_ranges.get(&operation)
        && (count < range.minimum || count > range.maximum)
    {
        return Err(AssetError::InvalidScalarValue {
            path: format!(
                "definition.{}.operation.{}.array.count",
                definition.0, operation.0
            ),
            value: count as f32,
            reason: "count is outside the authored array range",
        });
    }
    let definition = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_spec = definition
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown array operation {operation:?}"))
        })?;
    match operation_spec {
        ModelingOperationSpec::LinearArray { count: target, .. }
        | ModelingOperationSpec::RadialArray { count: target, .. } => {
            *target = count;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "operation {operation:?} is not an array"
        ))),
    }
}

fn set_modeling_operation_scalar(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    field: &str,
    value: f32,
) -> Result<(), AssetError> {
    let path = definition_scalar_path(definition, format!("operation.{}.{}", operation.0, field));
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar { path, value });
    }
    let definition_spec = recipe
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_spec = definition_spec
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| AssetError::UnknownScalarPath(path.clone()))?;
    let field_parts = field.split('.').collect::<Vec<_>>();
    get_operation_scalar(operation_spec, &field_parts, &path)?;
    let before_signature = edits::definition_topology_signature(definition_spec);
    let mut edited = recipe.clone();
    set_scalar_in_place(&mut edited, &path, value)?;
    let after_definition = edited
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if edits::definition_topology_signature(after_definition) != before_signature {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    *recipe = edited;
    Ok(())
}

fn set_array_spacing(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    spacing: &ArraySpacingEdit,
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_ref = definition_ref
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown array operation {operation:?}"))
        })?;
    match (operation_ref, spacing) {
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ArraySpacingEdit::LinearOffset(value),
        ) => {
            *offset = *value;
            Ok(())
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ArraySpacingEdit::RadialAxis(value)) => {
            *axis = *value;
            Ok(())
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ArraySpacingEdit::RadialAngleDegrees(value),
        ) => {
            *angle_degrees = *value;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "spacing edit does not match array operation {operation:?}"
        ))),
    }
}

#[derive(Debug, Clone)]
struct DuplicateCutSpec {
    operation: OperationId,
    entry_loop: BoundaryLoopId,
    secondary_loop: BoundaryLoopId,
    rim_region: RegionId,
    wall_region: RegionId,
    floor_region: Option<RegionId>,
    center_offset: [f32; 2],
    group_membership: DuplicateCutGroupMembership,
    dependent_bevels: Vec<DuplicateBoundaryBevelSpec>,
}

fn insert_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    operation: &ModelingOperationSpec,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    if operation_id_exists(recipe, operation.operation_id()) {
        return Err(AssetError::UnsupportedEdit(format!(
            "duplicate operation {:?}",
            operation.operation_id()
        )));
    }
    ensure_new_boundary_loops_available(recipe, operation.boundary_loop_ids())?;
    ensure_new_generated_regions_available(recipe, operation_detail_region_ids(operation))?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if index > definition_ref.geometry.operations.len() {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation insertion index {index} is out of bounds"
        )));
    }
    definition_ref
        .geometry
        .operations
        .insert(index, operation.clone());
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.remove(index);
    })?;
    bump_next_ids_for_operation(recipe, operation);
    Ok(())
}

fn remove_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    policy: OperationRemovalPolicy,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let dependents = dependent_operation_closure(recipe, definition, operation)?;
    let dependent_only = dependents
        .iter()
        .copied()
        .filter(|dependent| *dependent != operation)
        .collect::<Vec<_>>();
    let references = operation_metadata_references(recipe, definition, operation);
    match policy {
        OperationRemovalPolicy::RejectIfReferenced if !references.is_empty() => {
            return Err(AssetError::UnsupportedEdit(format!(
                "operation {:?} is still referenced by {}",
                operation,
                references.join(", ")
            )));
        }
        OperationRemovalPolicy::CascadeOwnedMetadata => {
            if !dependent_only.is_empty() {
                return Err(AssetError::UnsupportedEdit(format!(
                    "operation {:?} has dependent operation(s) {}",
                    operation,
                    operation_id_list(&dependent_only)
                )));
            }
            cascade_operation_metadata(recipe, definition, operation);
        }
        OperationRemovalPolicy::CascadeDependentOperations => {
            for dependent in &dependents {
                cascade_operation_metadata(recipe, definition, *dependent);
            }
        }
        OperationRemovalPolicy::RejectIfReferenced => {}
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    definition_ref
        .geometry
        .operations
        .retain(|candidate| !dependents.contains(&candidate.operation_id()));
    Ok(())
}
