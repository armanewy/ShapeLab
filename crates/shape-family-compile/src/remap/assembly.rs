//! Instance hierarchy and assembly remap boundary.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetRecipe, AttachmentSpec, BoundaryLoopId, ModelingOperationSpec, OperationId,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, RegionId, SocketId, SocketSpec,
    SurfaceRegionSpec,
};

use crate::{FragmentSurfaceTarget, RecipeFragment, RecipeFragmentExports};

use super::{AllocatedSemanticIds, FragmentRemap, FragmentRemapError, generated_socket_ids};

/// Remapped fragment assembly content inserted into a target recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemappedFragmentAssembly {
    /// Typed source-to-target map used by the assembly remap.
    pub remap: FragmentRemap,
    /// Newly allocated semantic IDs.
    pub allocated: AllocatedSemanticIds,
    /// Fragment exports rewritten to target semantic IDs.
    pub exports: RecipeFragmentExports,
}

/// Remap and insert fragment-local definitions, instances, hierarchy, sockets, and exports.
pub fn remap_fragment_assembly(
    target: &mut AssetRecipe,
    fragment: &RecipeFragment,
) -> Result<RemappedFragmentAssembly, FragmentRemapError> {
    validate_fragment_references(fragment)?;
    let mut working = target.clone();
    let prepared = prepare_fragment_assembly_remap(&mut working, fragment)?;

    for (old_id, definition) in &fragment.recipe.definitions {
        let cloned = remap_definition(fragment, definition, old_id, &prepared.remap)?;
        insert_target(
            &fragment.id,
            "part_definition",
            cloned.id.0.to_string(),
            working.definitions.insert(cloned.id, cloned),
        )?;
    }

    for (old_id, instance) in &fragment.recipe.instances {
        let cloned = remap_instance(fragment, instance, old_id, &prepared.remap)?;
        insert_target(
            &fragment.id,
            "part_instance",
            cloned.id.0.to_string(),
            working.instances.insert(cloned.id, cloned),
        )?;
    }

    let mut roots = remap_instance_list(
        fragment,
        &fragment.recipe.root_instances,
        &prepared.remap,
        "root instance",
    )?;
    working.root_instances.append(&mut roots);
    working.root_instances.sort_unstable();

    let exports = remap_exports(fragment, &prepared.remap)?;
    *target = working;
    Ok(RemappedFragmentAssembly {
        remap: prepared.remap,
        allocated: prepared.allocated,
        exports,
    })
}

fn prepare_fragment_assembly_remap(
    target: &mut AssetRecipe,
    fragment: &RecipeFragment,
) -> Result<PreparedAssemblyRemap, FragmentRemapError> {
    let mut remap = FragmentRemap::default();
    let mut allocated = AllocatedSemanticIds::default();
    let fragment_id = fragment.id.as_str();
    let operation_ids = collect_operation_ids(fragment)?;
    let region_ids = collect_region_ids(fragment)?;
    let boundary_loop_ids = collect_boundary_loop_ids(fragment)?;
    let socket_ids = collect_socket_ids(fragment)?;

    let mut used_definitions = target.definitions.keys().copied().collect::<BTreeSet<_>>();
    used_definitions.extend(fragment.recipe.definitions.keys().copied());
    let mut used_instances = target.instances.keys().copied().collect::<BTreeSet<_>>();
    used_instances.extend(fragment.recipe.instances.keys().copied());
    let mut used_parameters = target.parameters.keys().copied().collect::<BTreeSet<_>>();
    used_parameters.extend(fragment.recipe.parameters.keys().copied());
    let mut used_operations = BTreeSet::new();
    let mut used_regions = BTreeSet::new();
    let mut used_boundary_loops = BTreeSet::new();
    let mut used_sockets = BTreeSet::new();

    for definition in target.definitions.values() {
        used_regions.extend(definition.regions.keys().copied());
        used_sockets.extend(definition.sockets.keys().copied());
        used_sockets.extend(generated_socket_ids(&definition.geometry.source));
        for operation in &definition.geometry.operations {
            used_operations.insert(operation.operation_id());
            used_regions.extend(operation.generated_region_ids());
            used_boundary_loops.extend(operation.all_declared_boundary_loop_outputs());
        }
    }
    used_operations.extend(operation_ids.iter().copied());
    used_regions.extend(region_ids.iter().copied());
    used_boundary_loops.extend(boundary_loop_ids.iter().copied());
    used_sockets.extend(socket_ids.iter().copied());
    for definition in fragment.recipe.definitions.values() {
        used_sockets.extend(generated_socket_ids(&definition.geometry.source));
    }

    for definition_id in fragment.recipe.definitions.keys() {
        let new_id = allocate_unused_id(
            target,
            &mut used_definitions,
            AssetRecipe::allocate_part_definition_id,
        );
        insert_unique(
            fragment_id,
            "part_definition",
            *definition_id,
            new_id,
            &mut remap.definitions,
        )?;
        allocated.definitions.push(new_id);
    }
    for instance_id in fragment.recipe.instances.keys() {
        let new_id = allocate_unused_id(
            target,
            &mut used_instances,
            AssetRecipe::allocate_part_instance_id,
        );
        insert_unique(
            fragment_id,
            "part_instance",
            *instance_id,
            new_id,
            &mut remap.instances,
        )?;
        allocated.instances.push(new_id);
    }
    for parameter_id in fragment.recipe.parameters.keys() {
        let new_id = allocate_unused_id(
            target,
            &mut used_parameters,
            AssetRecipe::allocate_parameter_id,
        );
        insert_unique(
            fragment_id,
            "parameter",
            *parameter_id,
            new_id,
            &mut remap.parameters,
        )?;
        allocated.parameters.push(new_id);
    }
    for operation_id in operation_ids {
        let new_id = allocate_unused_id(
            target,
            &mut used_operations,
            AssetRecipe::allocate_operation_id,
        );
        insert_unique(
            fragment_id,
            "operation",
            operation_id,
            new_id,
            &mut remap.operations,
        )?;
        allocated.operations.push(new_id);
    }
    for region_id in region_ids {
        let new_id = allocate_unused_id(target, &mut used_regions, AssetRecipe::allocate_region_id);
        insert_unique(fragment_id, "region", region_id, new_id, &mut remap.regions)?;
        allocated.regions.push(new_id);
    }
    for boundary_loop_id in boundary_loop_ids {
        let new_id = allocate_unused_id(
            target,
            &mut used_boundary_loops,
            AssetRecipe::allocate_boundary_loop_id,
        );
        insert_unique(
            fragment_id,
            "boundary_loop",
            boundary_loop_id,
            new_id,
            &mut remap.boundary_loops,
        )?;
        allocated.boundary_loops.push(new_id);
    }
    for socket_id in socket_ids {
        let new_id = allocate_unused_id(target, &mut used_sockets, AssetRecipe::allocate_socket_id);
        insert_unique(fragment_id, "socket", socket_id, new_id, &mut remap.sockets)?;
        allocated.sockets.push(new_id);
    }

    Ok(PreparedAssemblyRemap { remap, allocated })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedAssemblyRemap {
    remap: FragmentRemap,
    allocated: AllocatedSemanticIds,
}

fn collect_operation_ids(
    fragment: &RecipeFragment,
) -> Result<Vec<OperationId>, FragmentRemapError> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for definition in fragment.recipe.definitions.values() {
        for operation in &definition.geometry.operations {
            let operation_id = operation.operation_id();
            if !seen.insert(operation_id) {
                return Err(duplicate(
                    &fragment.id,
                    "operation",
                    operation_id.0.to_string(),
                ));
            }
            ids.push(operation_id);
        }
    }
    Ok(ids)
}

fn collect_region_ids(fragment: &RecipeFragment) -> Result<Vec<RegionId>, FragmentRemapError> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for definition in fragment.recipe.definitions.values() {
        for region_id in definition.regions.keys() {
            if !seen.insert(*region_id) {
                return Err(duplicate(&fragment.id, "region", region_id.0.to_string()));
            }
            ids.push(*region_id);
        }
        for operation in &definition.geometry.operations {
            collect_operation_region_ids(fragment, operation, &mut ids, &mut seen)?;
        }
    }
    Ok(ids)
}

fn collect_operation_region_ids(
    fragment: &RecipeFragment,
    operation: &ModelingOperationSpec,
    ids: &mut Vec<RegionId>,
    seen: &mut BTreeSet<RegionId>,
) -> Result<(), FragmentRemapError> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            region,
            outer_region,
            rim_region,
            wall_region,
            floor_region,
            ..
        } => {
            collect_cut_outer_region(fragment, ids, seen, *region, *outer_region)?;
            collect_detail_region(fragment, ids, seen, *rim_region)?;
            collect_detail_region(fragment, ids, seen, *wall_region)?;
            collect_detail_region(fragment, ids, seen, *floor_region)?;
        }
        ModelingOperationSpec::RectangularThroughCut {
            region,
            outer_region,
            rim_region,
            wall_region,
            ..
        }
        | ModelingOperationSpec::CircularThroughCut {
            region,
            outer_region,
            rim_region,
            wall_region,
            ..
        } => {
            collect_cut_outer_region(fragment, ids, seen, *region, *outer_region)?;
            collect_detail_region(fragment, ids, seen, *rim_region)?;
            collect_detail_region(fragment, ids, seen, *wall_region)?;
        }
        ModelingOperationSpec::BevelBoundaryLoop { bevel_region, .. } => {
            collect_detail_region(fragment, ids, seen, *bevel_region)?;
        }
        ModelingOperationSpec::TransformGeometry { .. }
        | ModelingOperationSpec::SetBevelProfile { .. }
        | ModelingOperationSpec::AddPanel { .. }
        | ModelingOperationSpec::AddTrim { .. }
        | ModelingOperationSpec::MirrorInstances { .. }
        | ModelingOperationSpec::LinearArray { .. }
        | ModelingOperationSpec::RadialArray { .. }
        | ModelingOperationSpec::ReservedBoolean { .. }
        | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
    }
    Ok(())
}

fn collect_cut_outer_region(
    fragment: &RecipeFragment,
    ids: &mut Vec<RegionId>,
    seen: &mut BTreeSet<RegionId>,
    host_region: RegionId,
    outer_region: RegionId,
) -> Result<(), FragmentRemapError> {
    if seen.insert(outer_region) {
        ids.push(outer_region);
        return Ok(());
    }
    if outer_region == host_region {
        return Ok(());
    }
    Err(duplicate(
        &fragment.id,
        "region",
        outer_region.0.to_string(),
    ))
}

fn collect_detail_region(
    fragment: &RecipeFragment,
    ids: &mut Vec<RegionId>,
    seen: &mut BTreeSet<RegionId>,
    region: RegionId,
) -> Result<(), FragmentRemapError> {
    if !seen.insert(region) {
        return Err(duplicate(&fragment.id, "region", region.0.to_string()));
    }
    ids.push(region);
    Ok(())
}

fn collect_boundary_loop_ids(
    fragment: &RecipeFragment,
) -> Result<Vec<BoundaryLoopId>, FragmentRemapError> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for definition in fragment.recipe.definitions.values() {
        for operation in &definition.geometry.operations {
            for boundary_loop_id in operation.all_declared_boundary_loop_outputs() {
                if !seen.insert(boundary_loop_id) {
                    return Err(duplicate(
                        &fragment.id,
                        "boundary_loop",
                        boundary_loop_id.0.to_string(),
                    ));
                }
                ids.push(boundary_loop_id);
            }
        }
    }
    Ok(ids)
}

fn collect_socket_ids(fragment: &RecipeFragment) -> Result<Vec<SocketId>, FragmentRemapError> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for definition in fragment.recipe.definitions.values() {
        for socket_id in definition.sockets.keys() {
            if !seen.insert(*socket_id) {
                return Err(duplicate(&fragment.id, "socket", socket_id.0.to_string()));
            }
            ids.push(*socket_id);
        }
    }
    Ok(ids)
}

fn validate_fragment_references(fragment: &RecipeFragment) -> Result<(), FragmentRemapError> {
    validate_parent_cycles(fragment)?;
    validate_disjoint_exports(fragment)?;

    let definition_ids = fragment
        .recipe
        .definitions
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let instance_ids = fragment
        .recipe
        .instances
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let operation_ids = fragment
        .recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .map(ModelingOperationSpec::operation_id)
        .collect::<BTreeSet<_>>();
    let region_ids = fragment
        .recipe
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
    let boundary_loop_ids = fragment
        .recipe
        .definitions
        .values()
        .flat_map(|definition| {
            definition
                .geometry
                .operations
                .iter()
                .flat_map(ModelingOperationSpec::boundary_loop_ids)
        })
        .collect::<BTreeSet<_>>();
    let socket_ids = fragment
        .recipe
        .definitions
        .values()
        .flat_map(|definition| definition.sockets.keys().copied())
        .collect::<BTreeSet<_>>();

    for definition in fragment.recipe.definitions.values() {
        validate_operation_references(fragment, definition, &region_ids, &boundary_loop_ids)?;
    }
    for instance in fragment.recipe.instances.values() {
        if !definition_ids.contains(&instance.definition) {
            return Err(external(
                &fragment.id,
                "part_definition",
                instance.definition.0.to_string(),
            ));
        }
        if let Some(parent) = instance.parent
            && !instance_ids.contains(&parent)
        {
            return Err(external(
                &fragment.id,
                "part_instance",
                parent.0.to_string(),
            ));
        }
        if let Some(attachment) = &instance.attachment {
            validate_attachment_references(fragment, attachment, &socket_ids, &instance_ids)?;
        }
        if let Some(operation) = instance.generated_by
            && !operation_ids.contains(&operation)
        {
            return Err(external(&fragment.id, "operation", operation.0.to_string()));
        }
    }
    for root in &fragment.recipe.root_instances {
        if !instance_ids.contains(root) {
            return Err(external(&fragment.id, "part_instance", root.0.to_string()));
        }
    }
    validate_export_instance_list(
        fragment,
        &fragment.exports.role_occurrence_roots,
        &instance_ids,
    )?;
    validate_export_instance_list(fragment, &fragment.exports.internal_roots, &instance_ids)?;
    validate_port_references(
        fragment,
        &definition_ids,
        &instance_ids,
        &region_ids,
        &socket_ids,
    )
}

fn validate_operation_references(
    fragment: &RecipeFragment,
    definition: &PartDefinition,
    region_ids: &BTreeSet<RegionId>,
    boundary_loop_ids: &BTreeSet<BoundaryLoopId>,
) -> Result<(), FragmentRemapError> {
    for operation in &definition.geometry.operations {
        match operation {
            ModelingOperationSpec::AddPanel { region, .. }
            | ModelingOperationSpec::AddTrim { region, .. }
            | ModelingOperationSpec::RecessedPanelCut { region, .. }
            | ModelingOperationSpec::RectangularThroughCut { region, .. }
            | ModelingOperationSpec::CircularThroughCut { region, .. } => {
                if !region_ids.contains(region) {
                    return Err(external(&fragment.id, "region", region.0.to_string()));
                }
            }
            ModelingOperationSpec::BevelBoundaryLoop { target_loop, .. } => {
                if !boundary_loop_ids.contains(target_loop) {
                    return Err(external(
                        &fragment.id,
                        "boundary_loop",
                        target_loop.0.to_string(),
                    ));
                }
            }
            ModelingOperationSpec::TransformGeometry { .. }
            | ModelingOperationSpec::SetBevelProfile { .. }
            | ModelingOperationSpec::MirrorInstances { .. }
            | ModelingOperationSpec::LinearArray { .. }
            | ModelingOperationSpec::RadialArray { .. }
            | ModelingOperationSpec::ReservedBoolean { .. }
            | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
        }
    }
    Ok(())
}

fn validate_attachment_references(
    fragment: &RecipeFragment,
    attachment: &AttachmentSpec,
    socket_ids: &BTreeSet<SocketId>,
    instance_ids: &BTreeSet<PartInstanceId>,
) -> Result<(), FragmentRemapError> {
    if !instance_ids.contains(&attachment.parent_instance) {
        return Err(external(
            &fragment.id,
            "part_instance",
            attachment.parent_instance.0.to_string(),
        ));
    }
    if !socket_ids.contains(&attachment.parent_socket) {
        return Err(external(
            &fragment.id,
            "socket",
            attachment.parent_socket.0.to_string(),
        ));
    }
    if !socket_ids.contains(&attachment.child_socket) {
        return Err(external(
            &fragment.id,
            "socket",
            attachment.child_socket.0.to_string(),
        ));
    }
    Ok(())
}

fn validate_export_instance_list(
    fragment: &RecipeFragment,
    instances: &[PartInstanceId],
    instance_ids: &BTreeSet<PartInstanceId>,
) -> Result<(), FragmentRemapError> {
    let mut seen = BTreeSet::new();
    for instance in instances {
        if !instance_ids.contains(instance) {
            return Err(external(
                &fragment.id,
                "part_instance",
                instance.0.to_string(),
            ));
        }
        if !seen.insert(*instance) {
            return Err(duplicate(
                &fragment.id,
                "part_instance",
                instance.0.to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_disjoint_exports(fragment: &RecipeFragment) -> Result<(), FragmentRemapError> {
    let occurrence_roots = fragment
        .exports
        .role_occurrence_roots
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for internal_root in &fragment.exports.internal_roots {
        if occurrence_roots.contains(internal_root) {
            return Err(unsupported_assembly_remap(
                &fragment.id,
                "role occurrence roots and internal roots must be disjoint",
            ));
        }
    }
    Ok(())
}

fn validate_port_references(
    fragment: &RecipeFragment,
    definition_ids: &BTreeSet<PartDefinitionId>,
    instance_ids: &BTreeSet<PartInstanceId>,
    region_ids: &BTreeSet<RegionId>,
    socket_ids: &BTreeSet<SocketId>,
) -> Result<(), FragmentRemapError> {
    for port in &fragment.exports.socket_ports {
        if !instance_ids.contains(&port.local_occurrence_root) {
            return Err(external(
                &fragment.id,
                "part_instance",
                port.local_occurrence_root.0.to_string(),
            ));
        }
        if !socket_ids.contains(&port.local_socket) {
            return Err(external(
                &fragment.id,
                "socket",
                port.local_socket.0.to_string(),
            ));
        }
    }
    for port in &fragment.exports.surface_ports {
        match port.target {
            FragmentSurfaceTarget::Definition(definition) => {
                if !definition_ids.contains(&definition) {
                    return Err(external(
                        &fragment.id,
                        "part_definition",
                        definition.0.to_string(),
                    ));
                }
            }
            FragmentSurfaceTarget::Occurrence(instance) => {
                if !instance_ids.contains(&instance) {
                    return Err(external(
                        &fragment.id,
                        "part_instance",
                        instance.0.to_string(),
                    ));
                }
            }
        }
        if !region_ids.contains(&port.local_region) {
            return Err(external(
                &fragment.id,
                "region",
                port.local_region.0.to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_parent_cycles(fragment: &RecipeFragment) -> Result<(), FragmentRemapError> {
    for instance_id in fragment.recipe.instances.keys() {
        let mut seen = BTreeSet::new();
        let mut current = Some(*instance_id);
        while let Some(current_id) = current {
            if !seen.insert(current_id) {
                return Err(unsupported_assembly_remap(
                    &fragment.id,
                    "instance parent cycle is not allowed",
                ));
            }
            current = fragment
                .recipe
                .instances
                .get(&current_id)
                .and_then(|instance| instance.parent);
        }
    }
    Ok(())
}

fn remap_definition(
    fragment: &RecipeFragment,
    definition: &PartDefinition,
    old_id: &PartDefinitionId,
    remap: &FragmentRemap,
) -> Result<PartDefinition, FragmentRemapError> {
    let mut cloned = definition.clone();
    cloned.id = lookup(&fragment.id, "part_definition", old_id, &remap.definitions)?;
    cloned.regions = remap_regions(fragment, &definition.regions, remap)?;
    cloned.sockets = remap_sockets(fragment, &definition.sockets, remap)?;
    cloned.geometry.operations = definition
        .geometry
        .operations
        .iter()
        .map(|operation| remap_operation(fragment, operation, remap))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cloned)
}

fn remap_regions(
    fragment: &RecipeFragment,
    regions: &BTreeMap<RegionId, SurfaceRegionSpec>,
    remap: &FragmentRemap,
) -> Result<BTreeMap<RegionId, SurfaceRegionSpec>, FragmentRemapError> {
    regions
        .iter()
        .map(|(old_id, region)| {
            let new_id = lookup(&fragment.id, "region", old_id, &remap.regions)?;
            let mut cloned = region.clone();
            cloned.id = new_id;
            Ok((new_id, cloned))
        })
        .collect()
}

fn remap_sockets(
    fragment: &RecipeFragment,
    sockets: &BTreeMap<SocketId, SocketSpec>,
    remap: &FragmentRemap,
) -> Result<BTreeMap<SocketId, SocketSpec>, FragmentRemapError> {
    sockets
        .iter()
        .map(|(old_id, socket)| {
            let new_id = lookup(&fragment.id, "socket", old_id, &remap.sockets)?;
            let mut cloned = socket.clone();
            cloned.id = new_id;
            Ok((new_id, cloned))
        })
        .collect()
}

fn remap_instance(
    fragment: &RecipeFragment,
    instance: &PartInstance,
    old_id: &PartInstanceId,
    remap: &FragmentRemap,
) -> Result<PartInstance, FragmentRemapError> {
    let mut cloned = instance.clone();
    cloned.id = lookup(&fragment.id, "part_instance", old_id, &remap.instances)?;
    cloned.definition = lookup(
        &fragment.id,
        "part_definition",
        &instance.definition,
        &remap.definitions,
    )?;
    cloned.parent = instance
        .parent
        .map(|parent| lookup(&fragment.id, "part_instance", &parent, &remap.instances))
        .transpose()?;
    cloned.attachment = instance
        .attachment
        .as_ref()
        .map(|attachment| remap_attachment(fragment, attachment, remap))
        .transpose()?;
    cloned.generated_by = instance
        .generated_by
        .map(|operation| lookup(&fragment.id, "operation", &operation, &remap.operations))
        .transpose()?;
    Ok(cloned)
}

fn remap_attachment(
    fragment: &RecipeFragment,
    attachment: &AttachmentSpec,
    remap: &FragmentRemap,
) -> Result<AttachmentSpec, FragmentRemapError> {
    Ok(AttachmentSpec {
        parent_instance: lookup(
            &fragment.id,
            "part_instance",
            &attachment.parent_instance,
            &remap.instances,
        )?,
        parent_socket: lookup(
            &fragment.id,
            "socket",
            &attachment.parent_socket,
            &remap.sockets,
        )?,
        child_socket: lookup(
            &fragment.id,
            "socket",
            &attachment.child_socket,
            &remap.sockets,
        )?,
        local_offset: attachment.local_offset.clone(),
        mode: attachment.mode,
    })
}

fn remap_exports(
    fragment: &RecipeFragment,
    remap: &FragmentRemap,
) -> Result<RecipeFragmentExports, FragmentRemapError> {
    let mut exports = fragment.exports.clone();
    exports.role_occurrence_roots = remap_instance_list(
        fragment,
        &fragment.exports.role_occurrence_roots,
        remap,
        "role occurrence root",
    )?;
    exports.internal_roots = remap_instance_list(
        fragment,
        &fragment.exports.internal_roots,
        remap,
        "internal root",
    )?;
    for port in &mut exports.socket_ports {
        port.local_occurrence_root = lookup(
            &fragment.id,
            "part_instance",
            &port.local_occurrence_root,
            &remap.instances,
        )?;
        port.local_socket = lookup(&fragment.id, "socket", &port.local_socket, &remap.sockets)?;
    }
    for port in &mut exports.surface_ports {
        port.target = match port.target {
            FragmentSurfaceTarget::Definition(definition) => {
                FragmentSurfaceTarget::Definition(lookup(
                    &fragment.id,
                    "part_definition",
                    &definition,
                    &remap.definitions,
                )?)
            }
            FragmentSurfaceTarget::Occurrence(instance) => FragmentSurfaceTarget::Occurrence(
                lookup(&fragment.id, "part_instance", &instance, &remap.instances)?,
            ),
        };
        port.local_region = lookup(&fragment.id, "region", &port.local_region, &remap.regions)?;
    }
    Ok(exports)
}

fn remap_instance_list(
    fragment: &RecipeFragment,
    local_instances: &[PartInstanceId],
    remap: &FragmentRemap,
    label: &str,
) -> Result<Vec<PartInstanceId>, FragmentRemapError> {
    local_instances
        .iter()
        .map(|instance| {
            remap.instances.get(instance).copied().ok_or_else(|| {
                missing(&fragment.id, label, format!("part_instance {}", instance.0))
            })
        })
        .collect()
}

fn remap_operation(
    fragment: &RecipeFragment,
    operation: &ModelingOperationSpec,
    remap: &FragmentRemap,
) -> Result<ModelingOperationSpec, FragmentRemapError> {
    Ok(match operation {
        ModelingOperationSpec::TransformGeometry {
            operation,
            transform,
        } => ModelingOperationSpec::TransformGeometry {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            transform: transform.clone(),
        },
        ModelingOperationSpec::SetBevelProfile {
            operation,
            radius,
            segments,
        } => ModelingOperationSpec::SetBevelProfile {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            radius: *radius,
            segments: *segments,
        },
        ModelingOperationSpec::AddPanel {
            operation,
            region,
            inset,
            depth,
        } => ModelingOperationSpec::AddPanel {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            region: lookup(&fragment.id, "region", region, &remap.regions)?,
            inset: *inset,
            depth: *depth,
        },
        ModelingOperationSpec::AddTrim {
            operation,
            region,
            width,
            height,
        } => ModelingOperationSpec::AddTrim {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            region: lookup(&fragment.id, "region", region, &remap.regions)?,
            width: *width,
            height: *height,
        },
        ModelingOperationSpec::RecessedPanelCut {
            operation,
            region,
            face,
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            floor_loop,
            outer_region,
            rim_region,
            wall_region,
            floor_region,
            edge_treatment,
        } => ModelingOperationSpec::RecessedPanelCut {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            region: lookup(&fragment.id, "region", region, &remap.regions)?,
            face: *face,
            center: *center,
            size: *size,
            depth: *depth,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: lookup(
                &fragment.id,
                "boundary_loop",
                entry_loop,
                &remap.boundary_loops,
            )?,
            floor_loop: lookup(
                &fragment.id,
                "boundary_loop",
                floor_loop,
                &remap.boundary_loops,
            )?,
            outer_region: lookup(&fragment.id, "region", outer_region, &remap.regions)?,
            rim_region: lookup(&fragment.id, "region", rim_region, &remap.regions)?,
            wall_region: lookup(&fragment.id, "region", wall_region, &remap.regions)?,
            floor_region: lookup(&fragment.id, "region", floor_region, &remap.regions)?,
            edge_treatment: *edge_treatment,
        },
        ModelingOperationSpec::RectangularThroughCut {
            operation,
            region,
            face,
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            exit_loop,
            outer_region,
            rim_region,
            wall_region,
            edge_treatment,
        } => ModelingOperationSpec::RectangularThroughCut {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            region: lookup(&fragment.id, "region", region, &remap.regions)?,
            face: *face,
            center: *center,
            size: *size,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: lookup(
                &fragment.id,
                "boundary_loop",
                entry_loop,
                &remap.boundary_loops,
            )?,
            exit_loop: lookup(
                &fragment.id,
                "boundary_loop",
                exit_loop,
                &remap.boundary_loops,
            )?,
            outer_region: lookup(&fragment.id, "region", outer_region, &remap.regions)?,
            rim_region: lookup(&fragment.id, "region", rim_region, &remap.regions)?,
            wall_region: lookup(&fragment.id, "region", wall_region, &remap.regions)?,
            edge_treatment: *edge_treatment,
        },
        ModelingOperationSpec::CircularThroughCut {
            operation,
            region,
            face,
            center,
            radius,
            radial_segments,
            rim_width,
            entry_loop,
            exit_loop,
            outer_region,
            rim_region,
            wall_region,
            edge_treatment,
        } => ModelingOperationSpec::CircularThroughCut {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            region: lookup(&fragment.id, "region", region, &remap.regions)?,
            face: *face,
            center: *center,
            radius: *radius,
            radial_segments: *radial_segments,
            rim_width: *rim_width,
            entry_loop: lookup(
                &fragment.id,
                "boundary_loop",
                entry_loop,
                &remap.boundary_loops,
            )?,
            exit_loop: lookup(
                &fragment.id,
                "boundary_loop",
                exit_loop,
                &remap.boundary_loops,
            )?,
            outer_region: lookup(&fragment.id, "region", outer_region, &remap.regions)?,
            rim_region: lookup(&fragment.id, "region", rim_region, &remap.regions)?,
            wall_region: lookup(&fragment.id, "region", wall_region, &remap.regions)?,
            edge_treatment: *edge_treatment,
        },
        ModelingOperationSpec::BevelBoundaryLoop {
            operation,
            target_loop,
            width,
            segments,
            profile,
            bevel_region,
            outer_replacement_loop,
            inner_replacement_loop,
        } => ModelingOperationSpec::BevelBoundaryLoop {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            target_loop: lookup(
                &fragment.id,
                "boundary_loop",
                target_loop,
                &remap.boundary_loops,
            )?,
            width: *width,
            segments: *segments,
            profile: *profile,
            bevel_region: lookup(&fragment.id, "region", bevel_region, &remap.regions)?,
            outer_replacement_loop: lookup(
                &fragment.id,
                "boundary_loop",
                outer_replacement_loop,
                &remap.boundary_loops,
            )?,
            inner_replacement_loop: lookup(
                &fragment.id,
                "boundary_loop",
                inner_replacement_loop,
                &remap.boundary_loops,
            )?,
        },
        ModelingOperationSpec::MirrorInstances {
            operation,
            plane_normal,
            plane_offset,
        } => ModelingOperationSpec::MirrorInstances {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            plane_normal: *plane_normal,
            plane_offset: *plane_offset,
        },
        ModelingOperationSpec::LinearArray {
            operation,
            count,
            offset,
        } => ModelingOperationSpec::LinearArray {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            count: *count,
            offset: *offset,
        },
        ModelingOperationSpec::RadialArray {
            operation,
            count,
            axis,
            angle_degrees,
        } => ModelingOperationSpec::RadialArray {
            operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
            count: *count,
            axis: *axis,
            angle_degrees: *angle_degrees,
        },
        ModelingOperationSpec::ReservedBoolean { operation, label } => {
            ModelingOperationSpec::ReservedBoolean {
                operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
                label: label.clone(),
            }
        }
        ModelingOperationSpec::ReservedDeformationProgram { operation, label } => {
            ModelingOperationSpec::ReservedDeformationProgram {
                operation: lookup(&fragment.id, "operation", operation, &remap.operations)?,
                label: label.clone(),
            }
        }
    })
}

fn lookup<K: IdNumber + Ord + Copy>(
    fragment: &str,
    id_kind: &str,
    id: &K,
    map: &BTreeMap<K, K>,
) -> Result<K, FragmentRemapError> {
    map.get(id)
        .copied()
        .ok_or_else(|| missing(fragment, id_kind, id.id_number().to_string()))
}

fn insert_unique<K: IdNumber + Ord + Copy>(
    fragment: &str,
    id_kind: &str,
    old_id: K,
    new_id: K,
    map: &mut BTreeMap<K, K>,
) -> Result<(), FragmentRemapError> {
    if map.insert(old_id, new_id).is_some() {
        return Err(duplicate(fragment, id_kind, old_id.id_number().to_string()));
    }
    Ok(())
}

fn insert_target<T>(
    fragment: &str,
    id_kind: &str,
    id: String,
    previous: Option<T>,
) -> Result<(), FragmentRemapError> {
    if previous.is_some() {
        return Err(duplicate(fragment, id_kind, id));
    }
    Ok(())
}

fn allocate_unused_id<K>(
    target: &mut AssetRecipe,
    used: &mut BTreeSet<K>,
    allocate: fn(&mut AssetRecipe) -> K,
) -> K
where
    K: Copy + Ord,
{
    loop {
        let candidate = allocate(target);
        if used.insert(candidate) {
            return candidate;
        }
    }
}

trait IdNumber {
    fn id_number(self) -> u64;
}

impl IdNumber for PartDefinitionId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for PartInstanceId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for shape_asset::ParameterId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for OperationId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for RegionId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for BoundaryLoopId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl IdNumber for SocketId {
    fn id_number(self) -> u64 {
        self.0
    }
}

fn missing(fragment: &str, id_kind: &str, id: String) -> FragmentRemapError {
    FragmentRemapError::MissingMapping {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id,
    }
}

fn duplicate(fragment: &str, id_kind: &str, id: String) -> FragmentRemapError {
    FragmentRemapError::DuplicateMapping {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id,
    }
}

fn external(fragment: &str, id_kind: &str, id: String) -> FragmentRemapError {
    FragmentRemapError::ExternalReference {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id,
    }
}

/// Validate that assembly remapping is intentionally routed through this module.
pub fn unsupported_assembly_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "assembly".to_owned(),
        reason: reason.to_owned(),
    }
}
