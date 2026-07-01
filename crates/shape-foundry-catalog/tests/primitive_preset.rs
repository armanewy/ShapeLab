use shape_foundry::{
    ObjectPlanReviewTier, PresetSource, primitive_preset_public_catalog_publish_allowed,
    primitive_preset_public_catalog_visible, validate_primitive_preset,
};
use shape_foundry_catalog::{
    built_in_catalog_primitive_presets, built_in_catalog_primitive_presets_validate,
};

#[test]
fn primitive_preset_catalog_built_ins_validate() {
    let presets = built_in_catalog_primitive_presets();

    assert_eq!(presets.len(), 12);
    assert!(built_in_catalog_primitive_presets_validate());
    for preset in presets {
        assert!(validate_primitive_preset(&preset).is_valid());
        assert_eq!(preset.source, PresetSource::BuiltIn);
        assert_eq!(preset.review_tier, ObjectPlanReviewTier::Reviewed);
    }
}

#[test]
fn primitive_preset_catalog_does_not_publish() {
    for preset in built_in_catalog_primitive_presets() {
        assert!(
            !primitive_preset_public_catalog_publish_allowed(&preset),
            "{} must not publish automatically",
            preset.preset_id
        );
        assert!(
            !primitive_preset_public_catalog_visible(&preset),
            "{} must not be public catalog visible by default",
            preset.preset_id
        );
    }
}
