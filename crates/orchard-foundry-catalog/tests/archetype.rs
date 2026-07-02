#![forbid(unsafe_code)]

use orchard_foundry_catalog::built_in_foundry_kit_packages_with_labels;

#[test]
fn visual_foundry_product_labels_do_not_include_archetype_internals() {
    for (label, package) in built_in_foundry_kit_packages_with_labels() {
        assert_product_label_safe(label);
        assert_product_label_safe(&package.kit.display_name);
        for chip in &package.kit.category_chips {
            assert_product_label_safe(chip);
        }
        for control in &package.control_profile.controls {
            assert_product_label_safe(&control.label);
            assert_product_label_safe(&control.description);
        }
        for strategy in &package.candidate_strategy_pack.strategies {
            assert_product_label_safe(&strategy.name);
            for template in &strategy.explanation_templates {
                assert_product_label_safe(template);
            }
        }
    }
}

fn assert_product_label_safe(value: &str) {
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "archetype",
        "role template",
        "provider slot",
        "control axis",
        "candidate strategy template",
        "quality gate template",
        "semantic part group",
        "raw vertex",
        "geometry payload",
    ] {
        assert!(
            !lower.contains(forbidden),
            "Visual Foundry product label leaks archetype internals {forbidden:?}: {value}"
        );
    }
}
