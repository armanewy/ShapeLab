//! Wave 38 showcase gear fixtures for the Visual Foundry catalog.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::GeometrySource;
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{
    ControlProfileControlKind, ControlValue, CustomizerControl, FoundryKitQualityTier,
    validate_foundry_kit_package,
};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, count_slot, family_implementation,
    family_schema, fragment, integer_control, linear_array, ratio_slot, role, style_implementation,
    style_kit, toggle_control, toggle_slot,
};

/// Wave 38 promoted showcase gear kit slugs.
pub const SHOWCASE_GEAR_SLUGS: [&str; 5] = [
    "fantasy-sword",
    "round-shield",
    "hero-helmet",
    "pauldron-pair",
    "chest-armor",
];

/// True when a built-in fixture belongs to the Wave 38 showcase gear pack.
#[must_use]
pub fn is_showcase_gear_slug(slug: &str) -> bool {
    SHOWCASE_GEAR_SLUGS.contains(&slug)
}

/// Deterministic product-level coherence report for the Wave 38 demo pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowcaseGearPackReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Stable product pack ID.
    pub pack_id: String,
    /// Human-facing pack name.
    pub display_name: String,
    /// Style language shared across the promoted kits.
    pub style_language: String,
    /// Slugs included in the demo pack.
    pub kit_slugs: Vec<String>,
    /// Target automated quality tier for all included kits.
    pub minimum_target_tier: String,
    /// Whether every kit intentionally remains out of Showcase without human approval.
    pub showcase_requires_human_approval: bool,
    /// Number of promoted kit packages found in the built-in catalog.
    pub selected_kit_count: usize,
    /// Number of promoted kit packages that pass package validation.
    pub valid_kit_count: usize,
    /// Number of promoted kit packages marked Usable.
    pub usable_kit_count: usize,
    /// Largest primary-control count across promoted kits.
    pub maximum_primary_control_count: usize,
    /// Number of promoted kits with at least one whole-model option control.
    pub option_preview_kit_count: usize,
    /// Number of promoted kits with benchmark and contact-sheet refs in review metadata.
    pub review_evidence_ref_count: usize,
    /// Number of promoted kits incorrectly marked Showcase without human approval.
    pub showcase_without_human_approval_count: usize,
    /// Product-safe readiness statement.
    pub readiness: String,
    /// Whether the pack coherence gate passes.
    pub passed: bool,
}

/// Return the deterministic Wave 38 showcase gear pack report.
#[must_use]
pub fn showcase_gear_pack_report() -> ShowcaseGearPackReport {
    let mut selected_kit_count = 0_usize;
    let mut valid_kit_count = 0_usize;
    let mut usable_kit_count = 0_usize;
    let mut maximum_primary_control_count = 0_usize;
    let mut option_preview_kit_count = 0_usize;
    let mut review_evidence_ref_count = 0_usize;
    let mut showcase_without_human_approval_count = 0_usize;

    for slug in SHOWCASE_GEAR_SLUGS {
        let Some(package) = crate::built_in_foundry_kit_package(slug) else {
            continue;
        };
        selected_kit_count += 1;
        if validate_foundry_kit_package(&package).is_valid() {
            valid_kit_count += 1;
        }
        if package.kit.quality_tier == FoundryKitQualityTier::Usable {
            usable_kit_count += 1;
        }
        let primary_control_count = package
            .control_profile
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count();
        maximum_primary_control_count = maximum_primary_control_count.max(primary_control_count);
        if package
            .control_profile
            .controls
            .iter()
            .any(|control| control.visible && control.kind == ControlProfileControlKind::Choice)
        {
            option_preview_kit_count += 1;
        }
        if !package.review_manifest.contact_sheet_paths.is_empty()
            && !package.review_manifest.benchmark_refs.is_empty()
        {
            review_evidence_ref_count += 1;
        }
        if package.kit.quality_tier == FoundryKitQualityTier::Showcase
            && !package.review_manifest.human_approval_marker
        {
            showcase_without_human_approval_count += 1;
        }
    }
    let passed = selected_kit_count == SHOWCASE_GEAR_SLUGS.len()
        && valid_kit_count == SHOWCASE_GEAR_SLUGS.len()
        && usable_kit_count == SHOWCASE_GEAR_SLUGS.len()
        && maximum_primary_control_count <= 7
        && option_preview_kit_count == SHOWCASE_GEAR_SLUGS.len()
        && review_evidence_ref_count == SHOWCASE_GEAR_SLUGS.len()
        && showcase_without_human_approval_count == 0;

    ShowcaseGearPackReport {
        schema_version: 1,
        pack_id: "heroic-gear-demo-pack".to_owned(),
        display_name: "Heroic Gear Demo Pack".to_owned(),
        style_language: "Crisp heroic hard-surface forms with readable silhouettes".to_owned(),
        kit_slugs: SHOWCASE_GEAR_SLUGS
            .iter()
            .map(|slug| (*slug).to_owned())
            .collect(),
        minimum_target_tier: "Usable".to_owned(),
        showcase_requires_human_approval: true,
        selected_kit_count,
        valid_kit_count,
        usable_kit_count,
        maximum_primary_control_count,
        option_preview_kit_count,
        review_evidence_ref_count,
        showcase_without_human_approval_count,
        readiness: "Five coherent whole-model gear assets are ready for benchmark export review."
            .to_owned(),
        passed,
    }
}

#[derive(Clone)]
struct GearProfileSpec {
    slug: &'static str,
    document_id: &'static str,
    family_id: &'static str,
    family_name: &'static str,
    family_summary: &'static str,
    style_id: &'static str,
    style_name: &'static str,
    core_role: &'static str,
    accent_role: &'static str,
    detail_role: &'static str,
    accessory_role: &'static str,
    tags: &'static [&'static str],
    core_variants: Vec<FragmentSpec>,
    accent_variants: Vec<FragmentSpec>,
    detail_fragment: FragmentSpec,
    accessory_fragment: FragmentSpec,
}

#[derive(Clone, Copy)]
struct FragmentSpec {
    id: &'static str,
    label: &'static str,
    role: &'static str,
    shape: FragmentShape,
    translation: [f32; 3],
    array: Option<ArraySpec>,
}

#[derive(Clone, Copy)]
struct ArraySpec {
    count: u32,
    offset: [f32; 3],
}

#[derive(Clone, Copy)]
struct CylinderSpec {
    radius: f32,
    height: f32,
    radial_segments: u32,
}

#[derive(Clone, Copy)]
enum FragmentShape {
    RoundedBox {
        half_extents: [f32; 3],
        radius: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        radial_segments: u32,
    },
}

/// Build the promoted Fantasy Sword fixture catalog.
#[must_use]
pub fn fantasy_sword_fixture_catalog() -> FoundryFixtureCatalog {
    gear_fixture(GearProfileSpec {
        slug: "fantasy-sword",
        document_id: "fantasy-sword-doc",
        family_id: "fantasy_sword",
        family_name: "Fantasy Sword",
        family_summary: "Whole-model heroic sword with blade, guard, runes, and pommel accessory.",
        style_id: "heroic_sword_steel",
        style_name: "Heroic Sword Steel",
        core_role: "blade",
        accent_role: "guard",
        detail_role: "rune_set",
        accessory_role: "pommel",
        tags: &["gear", "weapon", "heroic", "showcase-candidate"],
        core_variants: vec![
            rounded(
                "tapered_blade",
                "Tapered Blade",
                "blade",
                [0.11, 1.35, 0.035],
                0.018,
                [0.0, 1.0, 0.0],
            ),
            rounded(
                "broad_blade",
                "Broad Blade",
                "blade",
                [0.16, 1.18, 0.045],
                0.015,
                [0.0, 1.0, 0.0],
            ),
            rounded(
                "leaf_blade",
                "Leaf Blade",
                "blade",
                [0.19, 1.05, 0.04],
                0.05,
                [0.0, 1.0, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "straight_guard",
                "Straight Guard",
                "guard",
                [0.52, 0.055, 0.06],
                0.018,
                [0.0, -0.95, 0.0],
            ),
            rounded(
                "winged_guard",
                "Winged Guard",
                "guard",
                [0.68, 0.065, 0.065],
                0.025,
                [0.0, -0.96, 0.0],
            ),
            rounded(
                "compact_guard",
                "Compact Guard",
                "guard",
                [0.38, 0.075, 0.06],
                0.02,
                [0.0, -0.96, 0.0],
            ),
        ],
        detail_fragment: rounded_array(
            "etched_runes",
            "Etched Runes",
            "rune_set",
            [0.028, 0.05, 0.012],
            0.004,
            [-0.05, 0.2, 0.09],
            array(5, [0.0, 0.25, 0.0]),
        ),
        accessory_fragment: cylinder(
            "faceted_pommel",
            "Faceted Pommel",
            "pommel",
            cylinder_spec(0.12, 0.18, 10),
            [0.0, -1.2, 0.0],
        ),
    })
}

/// Build the promoted Round Shield fixture catalog.
#[must_use]
pub fn round_shield_fixture_catalog() -> FoundryFixtureCatalog {
    gear_fixture(GearProfileSpec {
        slug: "round-shield",
        document_id: "round-shield-doc",
        family_id: "round_shield",
        family_name: "Round Shield",
        family_summary: "Whole-model round shield with face, rim, boss pattern, and rear handle.",
        style_id: "heroic_round_shield",
        style_name: "Heroic Round Shield",
        core_role: "shield_face",
        accent_role: "rim",
        detail_role: "boss_set",
        accessory_role: "handle",
        tags: &["gear", "armor", "shield", "showcase-candidate"],
        core_variants: vec![
            rounded(
                "round_face",
                "Round Face",
                "shield_face",
                [0.72, 0.72, 0.08],
                0.28,
                [0.0, 0.0, 0.0],
            ),
            rounded(
                "kite_face",
                "Kite Face",
                "shield_face",
                [0.44, 0.98, 0.08],
                0.18,
                [0.0, -0.05, 0.0],
            ),
            rounded(
                "tower_face",
                "Tower Face",
                "shield_face",
                [0.36, 1.08, 0.075],
                0.07,
                [0.0, -0.12, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "raised_rim",
                "Raised Rim",
                "rim",
                [0.58, 0.58, 0.035],
                0.3,
                [0.0, 0.0, 0.2],
            ),
            rounded(
                "heavy_rim",
                "Heavy Rim",
                "rim",
                [0.66, 0.66, 0.045],
                0.32,
                [0.0, 0.0, 0.22],
            ),
            rounded(
                "thin_rim",
                "Thin Rim",
                "rim",
                [0.5, 0.5, 0.03],
                0.26,
                [0.0, 0.0, 0.19],
            ),
        ],
        detail_fragment: cylinder_array(
            "shield_bosses",
            "Shield Bosses",
            "boss_set",
            cylinder_spec(0.075, 0.04, 12),
            [-0.42, -0.32, 0.35],
            array(5, [0.21, 0.16, 0.0]),
        ),
        accessory_fragment: rounded(
            "rear_handle",
            "Rear Handle",
            "handle",
            [0.18, 0.48, 0.045],
            0.018,
            [0.0, 0.0, -0.26],
        ),
    })
}

/// Build the promoted Hero Helmet fixture catalog.
#[must_use]
pub fn hero_helmet_fixture_catalog() -> FoundryFixtureCatalog {
    gear_fixture(GearProfileSpec {
        slug: "hero-helmet",
        document_id: "hero-helmet-doc",
        family_id: "hero_helmet",
        family_name: "Hero Helmet",
        family_summary: "Whole-model hero helmet with shell, visor, vents, and crest accessory.",
        style_id: "heroic_helmet_plate",
        style_name: "Heroic Helmet Plate",
        core_role: "helmet_shell",
        accent_role: "visor",
        detail_role: "vent_set",
        accessory_role: "crest",
        tags: &["gear", "armor", "helmet", "showcase-candidate"],
        core_variants: vec![
            rounded(
                "rounded_shell",
                "Rounded Shell",
                "helmet_shell",
                [0.58, 0.5, 0.52],
                0.18,
                [0.0, 0.18, 0.0],
            ),
            rounded(
                "tall_shell",
                "Tall Shell",
                "helmet_shell",
                [0.5, 0.62, 0.48],
                0.16,
                [0.0, 0.24, 0.0],
            ),
            rounded(
                "wide_shell",
                "Wide Shell",
                "helmet_shell",
                [0.68, 0.44, 0.5],
                0.15,
                [0.0, 0.12, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "narrow_visor",
                "Narrow Visor",
                "visor",
                [0.48, 0.075, 0.055],
                0.018,
                [0.0, 0.05, 0.78],
            ),
            rounded(
                "heavy_visor",
                "Heavy Visor",
                "visor",
                [0.58, 0.12, 0.065],
                0.02,
                [0.0, -0.02, 0.8],
            ),
            rounded(
                "open_visor",
                "Open Visor",
                "visor",
                [0.36, 0.065, 0.05],
                0.02,
                [0.0, 0.1, 0.78],
            ),
        ],
        detail_fragment: rounded_array(
            "breather_vents",
            "Breather Vents",
            "vent_set",
            [0.035, 0.08, 0.018],
            0.006,
            [-0.24, -0.18, 0.9],
            array(5, [0.12, 0.0, 0.0]),
        ),
        accessory_fragment: rounded(
            "helmet_crest",
            "Helmet Crest",
            "crest",
            [0.09, 0.48, 0.08],
            0.02,
            [0.0, 1.45, 0.0],
        ),
    })
}

/// Build the promoted Pauldron Pair fixture catalog.
#[must_use]
pub fn pauldron_pair_fixture_catalog() -> FoundryFixtureCatalog {
    gear_fixture(GearProfileSpec {
        slug: "pauldron-pair",
        document_id: "pauldron-pair-doc",
        family_id: "pauldron_pair",
        family_name: "Pauldron Pair",
        family_summary: "Whole-model paired shoulder armor with shells, trim, studs, and straps.",
        style_id: "heroic_shoulder_plate",
        style_name: "Heroic Shoulder Plate",
        core_role: "shoulder_shell",
        accent_role: "rim_trim",
        detail_role: "stud_set",
        accessory_role: "strap",
        tags: &["gear", "armor", "shoulder", "showcase-candidate"],
        core_variants: vec![
            rounded(
                "paired_shell",
                "Paired Shell",
                "shoulder_shell",
                [0.86, 0.28, 0.42],
                0.12,
                [0.0, 0.15, 0.0],
            ),
            rounded(
                "high_shell",
                "High Shell",
                "shoulder_shell",
                [0.78, 0.38, 0.38],
                0.13,
                [0.0, 0.22, 0.0],
            ),
            rounded(
                "compact_shell",
                "Compact Shell",
                "shoulder_shell",
                [0.68, 0.22, 0.34],
                0.1,
                [0.0, 0.08, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "clean_trim",
                "Clean Trim",
                "rim_trim",
                [0.92, 0.06, 0.45],
                0.025,
                [0.0, -0.35, 0.02],
            ),
            rounded(
                "wide_trim",
                "Wide Trim",
                "rim_trim",
                [1.0, 0.075, 0.5],
                0.03,
                [0.0, -0.37, 0.02],
            ),
            rounded(
                "split_trim",
                "Split Trim",
                "rim_trim",
                [0.8, 0.05, 0.36],
                0.018,
                [0.0, -0.35, 0.03],
            ),
        ],
        detail_fragment: cylinder_array(
            "rim_studs",
            "Rim Studs",
            "stud_set",
            cylinder_spec(0.045, 0.035, 10),
            [-0.48, -0.18, 0.68],
            array(6, [0.19, 0.0, 0.0]),
        ),
        accessory_fragment: rounded(
            "leather_strap",
            "Leather Strap",
            "strap",
            [0.74, 0.045, 0.055],
            0.012,
            [0.0, -0.55, -0.22],
        ),
    })
}

/// Build the promoted Chest Armor fixture catalog.
#[must_use]
pub fn chest_armor_fixture_catalog() -> FoundryFixtureCatalog {
    gear_fixture(GearProfileSpec {
        slug: "chest-armor",
        document_id: "chest-armor-doc",
        family_id: "chest_armor",
        family_name: "Chest Armor",
        family_summary: "Whole-model chest armor with torso shell, collar, rivets, and emblem.",
        style_id: "heroic_chest_plate",
        style_name: "Heroic Chest Plate",
        core_role: "torso_shell",
        accent_role: "collar",
        detail_role: "rivet_set",
        accessory_role: "emblem",
        tags: &["gear", "armor", "chest", "showcase-candidate"],
        core_variants: vec![
            rounded(
                "fitted_cuirass",
                "Fitted Cuirass",
                "torso_shell",
                [0.58, 0.82, 0.18],
                0.08,
                [0.0, 0.0, 0.0],
            ),
            rounded(
                "broad_cuirass",
                "Broad Cuirass",
                "torso_shell",
                [0.72, 0.74, 0.2],
                0.075,
                [0.0, -0.04, 0.0],
            ),
            rounded(
                "long_cuirass",
                "Long Cuirass",
                "torso_shell",
                [0.52, 0.96, 0.17],
                0.07,
                [0.0, -0.1, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "low_collar",
                "Low Collar",
                "collar",
                [0.5, 0.07, 0.08],
                0.02,
                [0.0, 1.28, 0.04],
            ),
            rounded(
                "raised_collar",
                "Raised Collar",
                "collar",
                [0.62, 0.11, 0.09],
                0.025,
                [0.0, 1.32, 0.05],
            ),
            rounded(
                "split_collar",
                "Split Collar",
                "collar",
                [0.4, 0.08, 0.08],
                0.018,
                [0.0, 1.28, 0.05],
            ),
        ],
        detail_fragment: cylinder_array(
            "armor_rivets",
            "Armor Rivets",
            "rivet_set",
            cylinder_spec(0.035, 0.025, 10),
            [-0.42, 0.42, 0.44],
            array(6, [0.17, -0.16, 0.0]),
        ),
        accessory_fragment: rounded(
            "front_emblem",
            "Front Emblem",
            "emblem",
            [0.16, 0.22, 0.035],
            0.025,
            [0.0, 0.02, 0.34],
        ),
    })
}

fn gear_fixture(spec: GearProfileSpec) -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: spec.family_id,
        display_name: spec.family_name,
        summary: spec.family_summary,
        roles: vec![
            role(spec.core_role, RoleMultiplicity::Single, true),
            role(spec.accent_role, RoleMultiplicity::Single, true),
            role(spec.detail_role, RoleMultiplicity::Repeated, true),
            role(spec.accessory_role, RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            ratio_slot("mass", "Mass", spec.core_role, 0.0, 1.0, 0.05, 0.5),
            ratio_slot("coverage", "Coverage", spec.core_role, 0.0, 1.0, 0.05, 0.5),
            ratio_slot(
                "profile_depth",
                "Profile Depth",
                spec.core_role,
                0.0,
                1.0,
                0.05,
                0.5,
            ),
            choice_slot(
                "silhouette",
                "Silhouette",
                spec.core_role,
                fragment_ids(&spec.core_variants),
            ),
            choice_slot(
                "ornament",
                "Ornament",
                spec.accent_role,
                fragment_ids(&spec.accent_variants),
            ),
            count_slot(
                "detail_density",
                "Detail Density",
                spec.detail_role,
                2.0,
                8.0,
                1.0,
                spec.detail_fragment.array.map_or(4, |array| array.count),
            ),
            toggle_slot("has_accessory", "Accessory", spec.accessory_role, true),
        ],
        compatible_style_kits: vec![spec.style_id.to_owned()],
        tags: spec.tags.iter().map(|tag| (*tag).to_owned()).collect(),
    });
    let prototypes = prototypes(&spec);
    let style = style_kit(
        spec.style_id,
        spec.style_name,
        spec.family_id,
        &prototypes,
        spec.tags.iter().map(|tag| (*tag).to_owned()).collect(),
    );
    let core_dimensions = rounded_half_extents(spec.core_variants[0]);
    let family_impl = family_implementation(
        spec.family_id,
        spec.family_name,
        vec![
            ParameterBinding::Scalar {
                slot: "mass".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[0]),
                    maximum: ratio_maximum(core_dimensions[0]),
                },
            },
            ParameterBinding::Scalar {
                slot: "coverage".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.y",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[1]),
                    maximum: ratio_maximum(core_dimensions[1]),
                },
            },
            ParameterBinding::Scalar {
                slot: "profile_depth".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.z",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[2]),
                    maximum: ratio_maximum(core_dimensions[2]),
                },
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "silhouette".to_owned(),
                role: spec.core_role.to_owned(),
                choices: choice_map(&spec.core_variants),
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "ornament".to_owned(),
                role: spec.accent_role.to_owned(),
                choices: choice_map(&spec.accent_variants),
            },
            ParameterBinding::Scalar {
                slot: "detail_density".to_owned(),
                role: spec.detail_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "operation.1.linear_array.count",
                ),
                transform: ScalarTransform::IntegerCount,
            },
            ParameterBinding::TogglePartPresence {
                slot: "has_accessory".to_owned(),
                role: spec.accessory_role.to_owned(),
            },
        ],
    );
    let style_impl = style_implementation(
        spec.style_id,
        spec.family_id,
        BTreeMap::from([
            (
                spec.core_role.to_owned(),
                spec.core_variants[0].id.to_owned(),
            ),
            (
                spec.accent_role.to_owned(),
                spec.accent_variants[0].id.to_owned(),
            ),
            (
                spec.detail_role.to_owned(),
                spec.detail_fragment.id.to_owned(),
            ),
            (
                spec.accessory_role.to_owned(),
                spec.accessory_fragment.id.to_owned(),
            ),
        ]),
        fragments(&spec),
    );
    build_fixture_catalog(FixtureCatalogSpec {
        slug: spec.slug,
        document_id: spec.document_id,
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: gear_customizer_profile(&spec),
        control_state: BTreeMap::from([
            ("mass".to_owned(), ControlValue::Scalar(0.5)),
            ("coverage".to_owned(), ControlValue::Scalar(0.5)),
            ("profile_depth".to_owned(), ControlValue::Scalar(0.5)),
            (
                "silhouette".to_owned(),
                ControlValue::Choice(spec.core_variants[0].id.to_owned()),
            ),
            (
                "ornament".to_owned(),
                ControlValue::Choice(spec.accent_variants[0].id.to_owned()),
            ),
            (
                "detail_density".to_owned(),
                ControlValue::Integer(
                    spec.detail_fragment.array.map_or(4, |array| array.count) as i64
                ),
            ),
            ("has_accessory".to_owned(), ControlValue::Toggle(true)),
        ]),
    })
}

fn rounded_half_extents(fragment: FragmentSpec) -> [f32; 3] {
    match fragment.shape {
        FragmentShape::RoundedBox { half_extents, .. } => half_extents,
        _ => panic!("core gear fragments must be rounded boxes"),
    }
}

fn ratio_minimum(default: f32) -> f32 {
    (default * 0.65).max(0.01)
}

fn ratio_maximum(default: f32) -> f32 {
    (default * 1.35).max(ratio_minimum(default) + 0.01)
}

fn gear_customizer_profile(spec: &GearProfileSpec) -> shape_foundry::CustomizerProfile {
    let silhouette_values = fragment_id_refs(&spec.core_variants);
    let ornament_values = fragment_id_refs(&spec.accent_variants);
    let detail_default = spec.detail_fragment.array.map_or(4, |array| array.count) as i64;
    let controls: Vec<CustomizerControl> = vec![
        continuous_control("mass", "Mass", "mass", 0.5, 0.0, 1.0),
        continuous_control("coverage", "Coverage", "coverage", 0.5, 0.0, 1.0),
        continuous_control(
            "profile_depth",
            "Profile Depth",
            "profile_depth",
            0.5,
            0.0,
            1.0,
        ),
        choice_control("silhouette", "Silhouette", "silhouette", &silhouette_values),
        choice_control("ornament", "Ornament", "ornament", &ornament_values),
        integer_control(
            "detail_density",
            "Detail Density",
            "detail_density",
            detail_default,
            2,
            8,
        ),
        toggle_control("has_accessory", "Accessory", "has_accessory", true),
    ];
    crate::customizer_profile(spec.family_id, spec.style_id, controls)
}

fn prototypes(spec: &GearProfileSpec) -> Vec<(&str, &str, &str)> {
    all_fragment_specs(spec)
        .into_iter()
        .map(|fragment| (fragment.id, fragment.label, fragment.role))
        .collect()
}

fn fragments(spec: &GearProfileSpec) -> Vec<RecipeFragment> {
    all_fragment_specs(spec)
        .into_iter()
        .map(recipe_fragment)
        .collect()
}

fn all_fragment_specs(spec: &GearProfileSpec) -> Vec<FragmentSpec> {
    spec.core_variants
        .iter()
        .chain(spec.accent_variants.iter())
        .copied()
        .chain([spec.detail_fragment, spec.accessory_fragment])
        .collect()
}

fn fragment_ids(fragments: &[FragmentSpec]) -> Vec<String> {
    fragments
        .iter()
        .map(|fragment| fragment.id.to_owned())
        .collect()
}

fn fragment_id_refs(fragments: &[FragmentSpec]) -> Vec<&str> {
    fragments.iter().map(|fragment| fragment.id).collect()
}

fn choice_map(fragments: &[FragmentSpec]) -> BTreeMap<String, String> {
    fragments
        .iter()
        .map(|fragment| (fragment.id.to_owned(), fragment.id.to_owned()))
        .collect()
}

fn recipe_fragment(spec: FragmentSpec) -> RecipeFragment {
    let operations = spec
        .array
        .map(|array| vec![linear_array(1, array.count, array.offset)])
        .unwrap_or_default();
    match spec.shape {
        FragmentShape::RoundedBox {
            half_extents,
            radius,
        } => fragment(
            spec.id,
            spec.role,
            GeometrySource::RoundedBox {
                half_extents,
                radius,
            },
            spec.translation,
            operations,
            &scalar_paths(
                &[
                    ("geometry.rounded_box.half_extents.x", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.half_extents.y", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.half_extents.z", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.radius", 0.0, 0.5, 0.01),
                ],
                spec.array,
            ),
        ),
        FragmentShape::Cylinder {
            radius,
            height,
            radial_segments,
        } => fragment(
            spec.id,
            spec.role,
            GeometrySource::Cylinder {
                radius,
                height,
                radial_segments,
            },
            spec.translation,
            operations,
            &scalar_paths(
                &[
                    ("geometry.cylinder.radius", 0.01, 2.0, 0.01),
                    ("geometry.cylinder.height", 0.01, 5.0, 0.01),
                    ("geometry.cylinder.radial_segments", 6.0, 64.0, 1.0),
                ],
                spec.array,
            ),
        ),
    }
}

fn scalar_paths(
    base_paths: &[(&'static str, f32, f32, f32)],
    array: Option<ArraySpec>,
) -> Vec<(&'static str, f32, f32, f32)> {
    let mut paths = base_paths.to_vec();
    if array.is_some() {
        paths.push(("operation.1.linear_array.count", 1.0, 12.0, 1.0));
    }
    paths
}

const fn rounded(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::RoundedBox {
            half_extents,
            radius,
        },
        translation,
        array: None,
    }
}

const fn rounded_array(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
    array: ArraySpec,
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::RoundedBox {
            half_extents,
            radius,
        },
        translation,
        array: Some(array),
    }
}

const fn cylinder(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    cylinder: CylinderSpec,
    translation: [f32; 3],
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::Cylinder {
            radius: cylinder.radius,
            height: cylinder.height,
            radial_segments: cylinder.radial_segments,
        },
        translation,
        array: None,
    }
}

const fn cylinder_array(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    cylinder: CylinderSpec,
    translation: [f32; 3],
    array: ArraySpec,
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::Cylinder {
            radius: cylinder.radius,
            height: cylinder.height,
            radial_segments: cylinder.radial_segments,
        },
        translation,
        array: Some(array),
    }
}

const fn array(count: u32, offset: [f32; 3]) -> ArraySpec {
    ArraySpec { count, offset }
}

const fn cylinder_spec(radius: f32, height: f32, radial_segments: u32) -> CylinderSpec {
    CylinderSpec {
        radius,
        height,
        radial_segments,
    }
}
