//! Semantic ID allocation/remap helpers.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::AssetRecipe;

use crate::RecipeFragment;

use super::{AllocatedSemanticIds, FragmentRemap};

/// Prepared typed ID remap with its allocation audit data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedIdRemap {
    /// Typed source-to-target map.
    pub remap: FragmentRemap,
    /// Newly allocated semantic IDs.
    pub allocated: AllocatedSemanticIds,
}

/// Allocate all currently supported semantic IDs for one fragment.
pub fn prepare_fragment_id_remap(
    target: &mut AssetRecipe,
    fragment: &RecipeFragment,
) -> PreparedIdRemap {
    let mut remap = FragmentRemap::default();
    let mut allocated = AllocatedSemanticIds::default();
    let mut used_definitions = target.definitions.keys().copied().collect::<BTreeSet<_>>();
    let mut used_instances = target.instances.keys().copied().collect::<BTreeSet<_>>();
    let mut used_parameters = target.parameters.keys().copied().collect::<BTreeSet<_>>();
    let mut used_operations = BTreeSet::new();
    let mut used_regions = BTreeSet::new();
    let mut used_boundary_loops = BTreeSet::new();
    let mut used_sockets = BTreeSet::new();

    used_definitions.extend(fragment.recipe.definitions.keys().copied());
    used_instances.extend(fragment.recipe.instances.keys().copied());
    used_parameters.extend(fragment.recipe.parameters.keys().copied());

    for definition in target.definitions.values() {
        used_regions.extend(definition.regions.keys().copied());
        used_sockets.extend(definition.sockets.keys().copied());
        for operation in &definition.geometry.operations {
            used_operations.insert(operation.operation_id());
            used_regions.extend(operation.generated_region_ids());
            used_boundary_loops.extend(operation.all_declared_boundary_loop_outputs());
        }
    }
    for definition in fragment.recipe.definitions.values() {
        used_regions.extend(definition.regions.keys().copied());
        used_sockets.extend(definition.sockets.keys().copied());
        for operation in &definition.geometry.operations {
            used_operations.insert(operation.operation_id());
            used_regions.extend(operation.generated_region_ids());
            used_boundary_loops.extend(operation.all_declared_boundary_loop_outputs());
        }
    }

    for definition_id in fragment.recipe.definitions.keys() {
        allocate_mapped_id(
            target,
            &mut used_definitions,
            &mut remap.definitions,
            &mut allocated.definitions,
            *definition_id,
            AssetRecipe::allocate_part_definition_id,
        );
    }
    for instance_id in fragment.recipe.instances.keys() {
        allocate_mapped_id(
            target,
            &mut used_instances,
            &mut remap.instances,
            &mut allocated.instances,
            *instance_id,
            AssetRecipe::allocate_part_instance_id,
        );
    }
    for parameter_id in fragment.recipe.parameters.keys() {
        allocate_mapped_id(
            target,
            &mut used_parameters,
            &mut remap.parameters,
            &mut allocated.parameters,
            *parameter_id,
            AssetRecipe::allocate_parameter_id,
        );
    }
    for definition in fragment.recipe.definitions.values() {
        for region_id in definition.regions.keys() {
            allocate_mapped_id(
                target,
                &mut used_regions,
                &mut remap.regions,
                &mut allocated.regions,
                *region_id,
                AssetRecipe::allocate_region_id,
            );
        }
        for socket_id in definition.sockets.keys() {
            allocate_mapped_id(
                target,
                &mut used_sockets,
                &mut remap.sockets,
                &mut allocated.sockets,
                *socket_id,
                AssetRecipe::allocate_socket_id,
            );
        }
        for operation in &definition.geometry.operations {
            allocate_mapped_id(
                target,
                &mut used_operations,
                &mut remap.operations,
                &mut allocated.operations,
                operation.operation_id(),
                AssetRecipe::allocate_operation_id,
            );
            for region_id in operation.generated_region_ids() {
                allocate_mapped_id(
                    target,
                    &mut used_regions,
                    &mut remap.regions,
                    &mut allocated.regions,
                    region_id,
                    AssetRecipe::allocate_region_id,
                );
            }
            for boundary_loop_id in operation.all_declared_boundary_loop_outputs() {
                allocate_mapped_id(
                    target,
                    &mut used_boundary_loops,
                    &mut remap.boundary_loops,
                    &mut allocated.boundary_loops,
                    boundary_loop_id,
                    AssetRecipe::allocate_boundary_loop_id,
                );
            }
        }
    }

    PreparedIdRemap { remap, allocated }
}

fn allocate_mapped_id<T>(
    target: &mut AssetRecipe,
    used: &mut BTreeSet<T>,
    map: &mut BTreeMap<T, T>,
    allocated: &mut Vec<T>,
    source: T,
    allocate: fn(&mut AssetRecipe) -> T,
) where
    T: Copy + Ord,
{
    if map.contains_key(&source) {
        return;
    }
    let new_id = allocate_unused_id(target, used, source, allocate);
    map.insert(source, new_id);
    allocated.push(new_id);
}

fn allocate_unused_id<T>(
    target: &mut AssetRecipe,
    used: &mut BTreeSet<T>,
    source: T,
    allocate: fn(&mut AssetRecipe) -> T,
) -> T
where
    T: Copy + Ord,
{
    loop {
        let new_id = allocate(target);
        if new_id != source && used.insert(new_id) {
            return new_id;
        }
    }
}
