//! Semantic ID allocation/remap helpers.

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

    for definition_id in fragment.recipe.definitions.keys() {
        let new_id = target.allocate_part_definition_id();
        remap.definitions.insert(*definition_id, new_id);
        allocated.definitions.push(new_id);
    }
    for instance_id in fragment.recipe.instances.keys() {
        let new_id = target.allocate_part_instance_id();
        remap.instances.insert(*instance_id, new_id);
        allocated.instances.push(new_id);
    }
    for parameter_id in fragment.recipe.parameters.keys() {
        let new_id = target.allocate_parameter_id();
        remap.parameters.insert(*parameter_id, new_id);
        allocated.parameters.push(new_id);
    }
    for definition in fragment.recipe.definitions.values() {
        for region_id in definition.regions.keys() {
            let new_id = target.allocate_region_id();
            remap.regions.insert(*region_id, new_id);
            allocated.regions.push(new_id);
        }
    }

    PreparedIdRemap { remap, allocated }
}
