
fn duplicate_cut_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    source: OperationId,
    spec: DuplicateCutSpec,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let group_membership = spec.group_membership.clone();
    let (duplicate, dependent_duplicates) = {
        let definition_ref = recipe
            .definitions
            .get(&definition)
            .ok_or(AssetError::UnknownDefinition(definition))?;
        let source_operation = definition_ref
            .geometry
            .operations
            .iter()
            .find(|candidate| candidate.operation_id() == source)
            .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {source:?}")))?;
        let duplicate = remap_cut_operation(source_operation, spec.clone())?;
        let dependent_duplicates = remap_dependent_boundary_bevels(
            &definition_ref.geometry.operations,
            source_operation,
            &duplicate,
            &spec.dependent_bevels,
        )?;
        (duplicate, dependent_duplicates)
    };
    let mut operation_ids = vec![duplicate.operation_id()];
    operation_ids.extend(
        dependent_duplicates
            .iter()
            .map(ModelingOperationSpec::operation_id),
    );
    ensure_new_operation_ids_available(recipe, operation_ids)?;
    let mut boundary_loop_ids = duplicate.boundary_loop_ids();
    boundary_loop_ids.extend(
        dependent_duplicates
            .iter()
            .flat_map(ModelingOperationSpec::boundary_loop_ids),
    );
    ensure_new_boundary_loops_available(recipe, boundary_loop_ids)?;
    let mut generated_regions = operation_detail_region_ids(&duplicate);
    generated_regions.extend(
        dependent_duplicates
            .iter()
            .flat_map(operation_detail_region_ids),
    );
    ensure_new_generated_regions_available(recipe, generated_regions)?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|candidate| candidate.operation_id() == source)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {source:?}")))?
        + 1;
    definition_ref
        .geometry
        .operations
        .insert(index, duplicate.clone());
    let dependent_insert_index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|operation| operation.phase() > OperationPhase::BoundaryTreatment)
        .unwrap_or(definition_ref.geometry.operations.len());
    for (offset, dependent) in dependent_duplicates.iter().enumerate() {
        definition_ref
            .geometry
            .operations
            .insert(dependent_insert_index + offset, dependent.clone());
    }
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.retain(|operation| {
            operation.operation_id() != duplicate.operation_id()
                && !dependent_duplicates
                    .iter()
                    .any(|dependent| dependent.operation_id() == operation.operation_id())
        });
    })?;
    bump_next_ids_for_operation(recipe, &duplicate);
    for dependent in &dependent_duplicates {
        bump_next_ids_for_operation(recipe, dependent);
    }
    apply_duplicate_cut_group_membership(
        recipe,
        definition,
        source,
        duplicate.operation_id(),
        &group_membership,
    )?;
    Ok(())
}

fn apply_duplicate_cut_group_membership(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    source: OperationId,
    duplicate: OperationId,
    membership: &DuplicateCutGroupMembership,
) -> Result<(), AssetError> {
    match membership {
        DuplicateCutGroupMembership::PreserveSource => {
            for group in recipe.variation.semantic_cut_groups.values_mut() {
                if group.definition == definition
                    && group.operations.contains(&source)
                    && !group.operations.contains(&duplicate)
                {
                    group.operations.push(duplicate);
                }
            }
            Ok(())
        }
        DuplicateCutGroupMembership::Ungrouped => Ok(()),
        DuplicateCutGroupMembership::AddTo(group_id) => {
            let group = recipe
                .variation
                .semantic_cut_groups
                .get_mut(group_id)
                .ok_or_else(|| {
                    AssetError::UnsupportedEdit(format!("unknown semantic cut group {group_id}"))
                })?;
            if group.definition != definition {
                return Err(AssetError::UnsupportedEdit(format!(
                    "semantic cut group {group_id} belongs to definition {:?}",
                    group.definition
                )));
            }
            if !group.operations.contains(&duplicate) {
                group.operations.push(duplicate);
            }
            Ok(())
        }
    }
}

fn move_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    new_index: usize,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let old_index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {operation:?}")))?;
    if new_index >= definition_ref.geometry.operations.len() {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation move index {new_index} is out of bounds"
        )));
    }
    let operation = definition_ref.geometry.operations.remove(old_index);
    definition_ref
        .geometry
        .operations
        .insert(new_index, operation.clone());
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.remove(new_index);
        definition_ref
            .geometry
            .operations
            .insert(old_index, operation);
    })?;
    Ok(())
}

fn ensure_operation_phase_order(operations: &[ModelingOperationSpec]) -> Result<(), AssetError> {
    for pair in operations.windows(2) {
        let previous = pair[0].phase();
        let next = pair[1].phase();
        if previous > next {
            return Err(AssetError::UnsupportedEdit(format!(
                "operation phase order violation: {:?} cannot appear before {:?}",
                pair[1].operation_id(),
                pair[0].operation_id()
            )));
        }
    }
    Ok(())
}

fn operation_metadata_references(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Vec<String> {
    let owned_parameters = operation_parameter_ids(recipe, definition, operation);
    let mut references = Vec::new();
    references.extend(
        owned_parameters
            .iter()
            .map(|parameter| format!("parameter.{}", parameter.0)),
    );
    references.extend(
        owned_parameters
            .iter()
            .filter(|parameter| recipe.locks.contains(parameter))
            .map(|parameter| format!("lock.{}", parameter.0)),
    );
    if recipe.variation.count_ranges.contains_key(&operation) {
        references.push(format!("variation.count_range.{}", operation.0));
    }
    if let Ok(dependents) = dependent_operation_closure(recipe, definition, operation) {
        references.extend(
            dependents
                .into_iter()
                .filter(|dependent| *dependent != operation)
                .map(|dependent| format!("operation.{}.dependency", dependent.0)),
        );
    }
    references.extend(
        recipe
            .variation
            .semantic_cut_groups
            .iter()
            .filter(|(_, group)| {
                group.definition == definition && group.operations.contains(&operation)
            })
            .map(|(group, _)| format!("variation.semantic_cut_group.{group}")),
    );
    references.extend(
        owned_parameters
            .iter()
            .filter(|parameter| {
                recipe
                    .variation
                    .parameter_range_overrides
                    .contains_key(parameter)
            })
            .map(|parameter| format!("variation.parameter_range.{}", parameter.0)),
    );
    references
}

fn dependent_operation_closure(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Result<BTreeSet<OperationId>, AssetError> {
    let definition_ref = recipe
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if !definition_ref
        .geometry
        .operations
        .iter()
        .any(|candidate| candidate.operation_id() == operation)
    {
        return Err(AssetError::UnsupportedEdit(format!(
            "unknown operation {operation:?}"
        )));
    }
    let mut removal = BTreeSet::from([operation]);
    loop {
        let produced_loops = definition_ref
            .geometry
            .operations
            .iter()
            .filter(|candidate| removal.contains(&candidate.operation_id()))
            .flat_map(ModelingOperationSpec::all_declared_boundary_loop_outputs)
            .collect::<BTreeSet<_>>();
        let before = removal.len();
        for candidate in &definition_ref.geometry.operations {
            if removal.contains(&candidate.operation_id()) {
                continue;
            }
            if candidate
                .boundary_loop_dependencies()
                .iter()
                .any(|dependency| produced_loops.contains(&dependency.input))
            {
                removal.insert(candidate.operation_id());
            }
        }
        if removal.len() == before {
            return Ok(removal);
        }
    }
}

fn operation_id_list(operations: &[OperationId]) -> String {
    operations
        .iter()
        .map(|operation| operation.0.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn cascade_operation_metadata(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) {
    let owned_parameters = operation_parameter_ids(recipe, definition, operation);
    for parameter in &owned_parameters {
        recipe.parameters.remove(parameter);
        recipe.locks.remove(parameter);
        recipe.variation.parameter_range_overrides.remove(parameter);
    }
    recipe.variation.count_ranges.remove(&operation);
    recipe.variation.semantic_cut_groups.retain(|_, group| {
        if group.definition != definition {
            return true;
        }
        group.operations.retain(|candidate| *candidate != operation);
        !group.operations.is_empty()
    });
}

fn operation_parameter_ids(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Vec<ParameterId> {
    let prefix = format!("definition.{}.operation.{}.", definition.0, operation.0);
    recipe
        .parameters
        .iter()
        .filter_map(|(id, descriptor)| descriptor.path.starts_with(&prefix).then_some(*id))
        .collect()
}

fn remap_cut_operation(
    operation: &ModelingOperationSpec,
    spec: DuplicateCutSpec,
) -> Result<ModelingOperationSpec, AssetError> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            region,
            face,
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            outer_region,
            edge_treatment,
            ..
        } => {
            let floor_region = spec.floor_region.ok_or_else(|| {
                AssetError::UnsupportedEdit(
                    "duplicated recessed cuts require a floor region".to_owned(),
                )
            })?;
            Ok(ModelingOperationSpec::RecessedPanelCut {
                operation: spec.operation,
                region: *region,
                face: *face,
                center: [
                    center[0] + spec.center_offset[0],
                    center[1] + spec.center_offset[1],
                ],
                size: *size,
                depth: *depth,
                corner_radius: *corner_radius,
                rim_width: *rim_width,
                corner_segments: *corner_segments,
                entry_loop: spec.entry_loop,
                floor_loop: spec.secondary_loop,
                outer_region: *outer_region,
                rim_region: spec.rim_region,
                wall_region: spec.wall_region,
                floor_region,
                edge_treatment: *edge_treatment,
            })
        }
        ModelingOperationSpec::RectangularThroughCut {
            region,
            face,
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            outer_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::RectangularThroughCut {
            operation: spec.operation,
            region: *region,
            face: *face,
            center: [
                center[0] + spec.center_offset[0],
                center[1] + spec.center_offset[1],
            ],
            size: *size,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: spec.entry_loop,
            exit_loop: spec.secondary_loop,
            outer_region: *outer_region,
            rim_region: spec.rim_region,
            wall_region: spec.wall_region,
            edge_treatment: *edge_treatment,
        }),
        ModelingOperationSpec::CircularThroughCut {
            region,
            face,
            center,
            radius,
            radial_segments,
            rim_width,
            outer_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::CircularThroughCut {
            operation: spec.operation,
            region: *region,
            face: *face,
            center: [
                center[0] + spec.center_offset[0],
                center[1] + spec.center_offset[1],
            ],
            radius: *radius,
            radial_segments: *radial_segments,
            rim_width: *rim_width,
            entry_loop: spec.entry_loop,
            exit_loop: spec.secondary_loop,
            outer_region: *outer_region,
            rim_region: spec.rim_region,
            wall_region: spec.wall_region,
            edge_treatment: *edge_treatment,
        }),
        _ => Err(AssetError::UnsupportedEdit(format!(
            "operation {:?} is not a cut",
            operation.operation_id()
        ))),
    }
}

fn remap_dependent_boundary_bevels(
    operations: &[ModelingOperationSpec],
    source_cut: &ModelingOperationSpec,
    duplicate_cut: &ModelingOperationSpec,
    specs: &[DuplicateBoundaryBevelSpec],
) -> Result<Vec<ModelingOperationSpec>, AssetError> {
    let mut copied = Vec::with_capacity(specs.len());
    let mut seen_sources = BTreeSet::new();
    for spec in specs {
        if !seen_sources.insert(spec.source) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate dependent bevel source {:?}",
                spec.source
            )));
        }
        let source_bevel = operations
            .iter()
            .find(|operation| operation.operation_id() == spec.source)
            .ok_or_else(|| {
                AssetError::UnsupportedEdit(format!(
                    "unknown dependent bevel operation {:?}",
                    spec.source
                ))
            })?;
        let ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            width,
            segments,
            profile,
            ..
        } = source_bevel
        else {
            return Err(AssetError::UnsupportedEdit(format!(
                "dependent operation {:?} is not a boundary-loop bevel",
                spec.source
            )));
        };
        let Some(remapped_target_loop) =
            remapped_cut_boundary_loop(source_cut, duplicate_cut, *target_loop)
        else {
            return Err(AssetError::UnsupportedEdit(format!(
                "dependent bevel {:?} does not target a loop produced by {:?}",
                spec.source,
                source_cut.operation_id()
            )));
        };
        copied.push(ModelingOperationSpec::BevelBoundaryLoop {
            operation: spec.operation,
            target_loop: remapped_target_loop,
            width: *width,
            segments: *segments,
            profile: *profile,
            bevel_region: spec.bevel_region,
            outer_replacement_loop: spec.outer_replacement_loop,
            inner_replacement_loop: spec.inner_replacement_loop,
        });
    }
    Ok(copied)
}

fn remapped_cut_boundary_loop(
    source_cut: &ModelingOperationSpec,
    duplicate_cut: &ModelingOperationSpec,
    source_loop: BoundaryLoopId,
) -> Option<BoundaryLoopId> {
    let source_loops = source_cut.direct_boundary_loop_outputs();
    let duplicate_loops = duplicate_cut.direct_boundary_loop_outputs();
    source_loops
        .iter()
        .position(|candidate| *candidate == source_loop)
        .and_then(|index| duplicate_loops.get(index).copied())
}

fn operation_id_exists(recipe: &AssetRecipe, operation: OperationId) -> bool {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .any(|candidate| candidate.operation_id() == operation)
}

fn ensure_new_operation_ids_available(
    recipe: &AssetRecipe,
    operations: Vec<OperationId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .map(ModelingOperationSpec::operation_id)
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for operation in operations {
        if !local.insert(operation) || !used.insert(operation) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate operation {operation:?}"
            )));
        }
    }
    Ok(())
}

fn ensure_new_boundary_loops_available(
    recipe: &AssetRecipe,
    boundary_loops: Vec<BoundaryLoopId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for boundary_loop in boundary_loops {
        if boundary_loop == LEGACY_MISSING_BOUNDARY_LOOP
            || !local.insert(boundary_loop)
            || !used.insert(boundary_loop)
        {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate boundary loop {boundary_loop:?}"
            )));
        }
    }
    Ok(())
}

fn ensure_new_generated_regions_available(
    recipe: &AssetRecipe,
    regions: Vec<RegionId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| {
            definition.regions.keys().copied().chain(
                definition
                    .geometry
                    .operations
                    .iter()
                    .flat_map(ModelingOperationSpec::generated_region_ids),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for region in regions {
        if !local.insert(region) || !used.insert(region) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate generated region {region:?}"
            )));
        }
    }
    Ok(())
}

fn operation_detail_region_ids(operation: &ModelingOperationSpec) -> Vec<RegionId> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            rim_region,
            wall_region,
            floor_region,
            ..
        } => vec![*rim_region, *wall_region, *floor_region],
        ModelingOperationSpec::RectangularThroughCut {
            rim_region,
            wall_region,
            ..
        }
        | ModelingOperationSpec::CircularThroughCut {
            rim_region,
            wall_region,
            ..
        } => vec![*rim_region, *wall_region],
        ModelingOperationSpec::BevelBoundaryLoop { bevel_region, .. } => vec![*bevel_region],
        _ => Vec::new(),
    }
}

fn duplicate_instance(
    recipe: &mut AssetRecipe,
    source: PartInstanceId,
    instance: PartInstanceId,
    name: Option<&str>,
    transform: Option<&Transform3>,
) -> Result<(), AssetError> {
    edits::ensure_instance_editable(recipe, source)?;
    if recipe.instances.contains_key(&instance) {
        return Err(AssetError::UnsupportedEdit(format!(
            "duplicate instance {instance:?}"
        )));
    }
    let mut duplicate = recipe
        .instances
        .get(&source)
        .ok_or(AssetError::UnknownInstance(source))?
        .clone();
    if let Some(parent) = duplicate.parent {
        edits::ensure_instance_editable(recipe, parent)?;
    }
    duplicate.id = instance;
    duplicate.name = name
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} copy", duplicate.name));
    if let Some(transform) = transform {
        duplicate.local_transform = transform.clone();
    }
    recipe.instances.insert(instance, duplicate.clone());
    if duplicate.parent.is_none() {
        insert_root_instance(recipe, instance);
    }
    recipe.variation.optional_instances.remove(&instance);
    bump_next_ids_for_instance(recipe, &duplicate);
    Ok(())
}

fn mirror_instance(
    recipe: &mut AssetRecipe,
    source: PartInstanceId,
    instance: PartInstanceId,
    plane: &MirrorInstanceSpec,
    name: Option<&str>,
) -> Result<(), AssetError> {
    if !array_is_finite(&plane.plane_normal) || !plane.plane_offset.is_finite() {
        return Err(AssetError::UnsupportedEdit(
            "mirror plane must be finite".to_owned(),
        ));
    }
    let normal = Vec3::from_array(plane.plane_normal);
    let length = normal.length();
    if length == 0.0 {
        return Err(AssetError::UnsupportedEdit(
            "mirror plane normal must be non-zero".to_owned(),
        ));
    }
    duplicate_instance(recipe, source, instance, name, None)?;
    let target = recipe
        .instances
        .get_mut(&instance)
        .ok_or(AssetError::UnknownInstance(instance))?;
    let unit_normal = normal / length;
    let point = Vec3::from_array(target.local_transform.translation);
    let distance = point.dot(unit_normal) - plane.plane_offset;
    target.local_transform.translation = (point - 2.0 * distance * unit_normal).to_array();
    Ok(())
}

fn reorder_child_instances(
    recipe: &mut AssetRecipe,
    parent: Option<PartInstanceId>,
    ordered_children: &[PartInstanceId],
) -> Result<(), AssetError> {
    let actual_children = match parent {
        Some(parent) => {
            edits::ensure_instance_editable(recipe, parent)?;
            if !recipe.instances.contains_key(&parent) {
                return Err(AssetError::UnknownInstance(parent));
            }
            recipe
                .instances
                .values()
                .filter(|candidate| candidate.parent == Some(parent))
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>()
        }
        None => recipe.root_instances.clone(),
    };
    let requested = ordered_children.iter().copied().collect::<BTreeSet<_>>();
    let actual = actual_children.iter().copied().collect::<BTreeSet<_>>();
    if requested != actual {
        return Err(AssetError::UnsupportedEdit(
            "reorder must contain exactly the current children".to_owned(),
        ));
    }
    if parent.is_none() {
        let mut roots = ordered_children.to_vec();
        roots.sort_unstable();
        recipe.root_instances = roots;
    }
    Ok(())
}

fn insert_root_instance(recipe: &mut AssetRecipe, instance: PartInstanceId) {
    if !recipe.root_instances.contains(&instance) {
        recipe.root_instances.push(instance);
        recipe.root_instances.sort_unstable();
    }
}

fn bump_next_ids_for_instance(recipe: &mut AssetRecipe, instance: &PartInstance) {
    bump_counter(&mut recipe.next_ids.part_instance, instance.id.0);
}

fn bump_next_ids_for_definition(recipe: &mut AssetRecipe, definition: &PartDefinition) {
    bump_counter(&mut recipe.next_ids.part_definition, definition.id.0);
    for operation in &definition.geometry.operations {
        bump_next_ids_for_operation(recipe, operation);
    }
    for region in definition.regions.keys() {
        bump_counter(&mut recipe.next_ids.region, region.0);
    }
    for socket in definition.sockets.keys() {
        bump_counter(&mut recipe.next_ids.socket, socket.0);
    }
}

fn bump_next_ids_for_operation(recipe: &mut AssetRecipe, operation: &ModelingOperationSpec) {
    bump_counter(&mut recipe.next_ids.operation, operation.operation_id().0);
    for boundary_loop in operation.boundary_loop_ids() {
        bump_counter(&mut recipe.next_ids.boundary_loop, boundary_loop.0);
    }
    for region in operation.generated_region_ids() {
        bump_counter(&mut recipe.next_ids.region, region.0);
    }
}

fn bump_counter(counter: &mut u64, used: u64) {
    *counter = (*counter).max(used.saturating_add(1));
}
