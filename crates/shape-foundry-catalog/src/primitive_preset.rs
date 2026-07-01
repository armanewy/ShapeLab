//! Built-in primitive preset catalog.

use shape_foundry::{PrimitivePreset, built_in_primitive_presets, validate_primitive_preset};

/// Return the reviewed built-in primitive presets exposed to offline tooling.
#[must_use]
pub fn built_in_catalog_primitive_presets() -> Vec<PrimitivePreset> {
    built_in_primitive_presets()
}

/// Return true when every built-in catalog preset validates.
#[must_use]
pub fn built_in_catalog_primitive_presets_validate() -> bool {
    built_in_catalog_primitive_presets()
        .iter()
        .all(|preset| validate_primitive_preset(preset).is_valid())
}
