//! Wave 40 MOBA-quality clay hero fixture for the Visual Foundry catalog.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::GeometrySource;
use shape_character::prepared::prepared_hero_template_v1;
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{
    CandidateStrategy, ControlValue, FoundryKitQualityTier, compile_foundry_document,
};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, family_implementation, family_schema,
    fragment, linear_array, ratio_slot, role, style_implementation, style_kit,
};

/// Wave 40 clay hero fixture slug.
pub const MOBA_HERO_CLAY_SLUG: &str = "moba-hero-clay";

const HERO_ARCHETYPES: [&str; 6] = [
    "armored_duelist",
    "arcane_ranger",
    "brutal_champion",
    "agile_assassin",
    "ceremonial_guardian",
    "monster_hunter",
];

const ARMOR_MASS_OPTIONS: [&str; 5] = [
    "light_armor",
    "duelist_mail",
    "heavy_plate",
    "ceremonial_plate",
    "hunter_leathers",
];

const HEAD_FACE_OPTIONS: [&str; 5] = [
    "focused_visor",
    "arcane_mask",
    "brutal_jaw",
    "guardian_brow",
    "hunter_jaw",
];

const HAIR_HEADGEAR_OPTIONS: [&str; 5] = [
    "swept_hair",
    "light_helmet",
    "crest_helmet",
    "hooded_mass",
    "horned_hood",
];

const WEAPON_ACCESSORY_OPTIONS: [&str; 5] = [
    "blade_and_scabbard",
    "staff_and_cloak",
    "axe_and_trophy",
    "dagger_and_smoke",
    "banner_and_mace",
];

/// One product-facing candidate direction for the clay hero fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobaHeroDirectionSpec {
    /// Direction name.
    pub name: String,
    /// Human-readable control summary.
    pub summary: Vec<String>,
}

/// Deterministic profile readiness metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobaHeroClayProfileReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Profile slug.
    pub profile_id: String,
    /// Product-facing display name.
    pub display_name: String,
    /// Source prepared template ID.
    pub source_template_id: String,
    /// Source base library fingerprint.
    pub source_base_fingerprint: String,
    /// Whether the authored Foundry profile covers the prepared hero v1 contract it cites.
    pub prepared_template_compatible: bool,
    /// Target kit tier before human review.
    pub quality_tier: String,
    /// Whether the profile remains hidden from default novice catalog.
    pub default_catalog_hidden: bool,
    /// Primary control labels.
    pub primary_controls: Vec<String>,
    /// Provider/sample groups and counts.
    pub provider_sample_counts: BTreeMap<String, usize>,
    /// Named candidate directions.
    pub candidate_directions: Vec<MobaHeroDirectionSpec>,
    /// Unsupported claims.
    pub unsupported_claims: Vec<String>,
}

/// Build the Hero Foundry clay MVP fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family_id = "moba_hero_clay";
    let style_id = "moba_heroic_clay";
    let family = family_schema(FamilySchemaSpec {
        id: family_id,
        display_name: "Hero Foundry, Clay MVP",
        summary: "Clay-only hero family built from a prepared hero template and compatible gear parts.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("head", RoleMultiplicity::Single, true),
            role("hair_headgear", RoleMultiplicity::Single, true),
            role("shoulders", RoleMultiplicity::Single, true),
            role("torso_armor", RoleMultiplicity::Single, true),
            role("belt_skirt", RoleMultiplicity::Single, true),
            role("gauntlets", RoleMultiplicity::Repeated, true),
            role("boots", RoleMultiplicity::Repeated, true),
            role("weapon", RoleMultiplicity::Single, true),
            role("back_accessory", RoleMultiplicity::Single, true),
            role("small_detail", RoleMultiplicity::Repeated, true),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            choice_slot(
                "hero_archetype",
                "Hero Archetype",
                "body",
                strings(&HERO_ARCHETYPES),
            ),
            ratio_slot(
                "body_proportions",
                "Body Proportions",
                "body",
                0.0,
                1.0,
                0.05,
                0.5,
            ),
            ratio_slot("silhouette", "Silhouette", "body", 0.0, 1.0, 0.05, 0.5),
            choice_slot(
                "armor_mass",
                "Armor Mass",
                "torso_armor",
                strings(&ARMOR_MASS_OPTIONS),
            ),
            choice_slot(
                "head_face",
                "Head & Face",
                "head",
                strings(&HEAD_FACE_OPTIONS),
            ),
            choice_slot(
                "hair_headgear",
                "Hair / Headgear",
                "hair_headgear",
                strings(&HAIR_HEADGEAR_OPTIONS),
            ),
            choice_slot(
                "weapon_accessory",
                "Weapon / Accessory",
                "weapon",
                strings(&WEAPON_ACCESSORY_OPTIONS),
            ),
        ],
        compatible_style_kits: vec![style_id.to_owned()],
        tags: vec!["hero".to_owned(), "clay".to_owned(), "mvp".to_owned()],
    });
    let fragments = hero_fragments();
    let prototypes = fragments
        .iter()
        .map(|fragment| {
            (
                fragment.id.as_str(),
                fragment.id.replace('_', " "),
                fragment.provided_role.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let prototypes = prototypes
        .iter()
        .map(|(id, label, role)| (*id, label.as_str(), *role))
        .collect::<Vec<_>>();
    let style = style_kit(
        style_id,
        "MOBA Heroic Clay",
        family_id,
        &prototypes,
        vec!["hero".to_owned(), "clay".to_owned()],
    );
    let family_impl = family_implementation(
        family_id,
        "Hero Foundry, Clay MVP",
        hero_parameter_bindings(),
    );
    let style_impl = style_implementation(
        style_id,
        family_id,
        BTreeMap::from([
            ("body".to_owned(), "armored_duelist".to_owned()),
            ("head".to_owned(), "focused_visor".to_owned()),
            ("hair_headgear".to_owned(), "light_helmet".to_owned()),
            ("shoulders".to_owned(), "duelist_mail_shoulders".to_owned()),
            ("torso_armor".to_owned(), "duelist_mail_torso".to_owned()),
            ("belt_skirt".to_owned(), "duelist_mail_belt".to_owned()),
            ("gauntlets".to_owned(), "duelist_mail_gauntlets".to_owned()),
            ("boots".to_owned(), "duelist_mail_boots".to_owned()),
            ("weapon".to_owned(), "blade_and_scabbard_weapon".to_owned()),
            (
                "back_accessory".to_owned(),
                "blade_and_scabbard_back".to_owned(),
            ),
            ("small_detail".to_owned(), "duelist_mail_detail".to_owned()),
        ]),
        fragments,
    );
    build_fixture_catalog(FixtureCatalogSpec {
        slug: MOBA_HERO_CLAY_SLUG,
        document_id: "moba-hero-clay-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: hero_customizer_profile(family_id, style_id),
        control_state: BTreeMap::from([
            (
                "hero_archetype".to_owned(),
                ControlValue::Choice("armored_duelist".to_owned()),
            ),
            ("body_proportions".to_owned(), ControlValue::Scalar(0.5)),
            ("silhouette".to_owned(), ControlValue::Scalar(0.5)),
            (
                "armor_mass".to_owned(),
                ControlValue::Choice("duelist_mail".to_owned()),
            ),
            (
                "head_face".to_owned(),
                ControlValue::Choice("focused_visor".to_owned()),
            ),
            (
                "hair_headgear".to_owned(),
                ControlValue::Choice("light_helmet".to_owned()),
            ),
            (
                "weapon_accessory".to_owned(),
                ControlValue::Choice("blade_and_scabbard".to_owned()),
            ),
        ]),
    })
}

/// Return product-safe profile report metadata.
#[must_use]
pub fn profile_report() -> MobaHeroClayProfileReport {
    let template = prepared_hero_template_v1();
    let prepared_template_compatible = validate_prepared_template_compatibility().is_ok();
    MobaHeroClayProfileReport {
        schema_version: 1,
        profile_id: MOBA_HERO_CLAY_SLUG.to_owned(),
        display_name: "Hero Foundry, Clay MVP".to_owned(),
        source_template_id: template.template_id,
        source_base_fingerprint: template.base_topology.base_library_fingerprint.0,
        prepared_template_compatible,
        quality_tier: format!("{:?}", FoundryKitQualityTier::Prototype),
        default_catalog_hidden: true,
        primary_controls: vec![
            "Hero Archetype".to_owned(),
            "Body Proportions".to_owned(),
            "Silhouette".to_owned(),
            "Armor Mass".to_owned(),
            "Head & Face".to_owned(),
            "Hair / Headgear".to_owned(),
            "Weapon / Accessory".to_owned(),
        ],
        provider_sample_counts: BTreeMap::from([
            (
                "body/proportion/silhouette".to_owned(),
                HERO_ARCHETYPES.len(),
            ),
            (
                "head/headgear/hair".to_owned(),
                HEAD_FACE_OPTIONS.len() + HAIR_HEADGEAR_OPTIONS.len(),
            ),
            ("armor/torso/shoulders".to_owned(), ARMOR_MASS_OPTIONS.len()),
            (
                "weapon/accessory".to_owned(),
                WEAPON_ACCESSORY_OPTIONS.len(),
            ),
        ]),
        candidate_directions: hero_direction_specs(),
        unsupported_claims: vec![
            "no Dota or third-party IP reconstruction".to_owned(),
            "no textures, materials, UVs, rigging, or animation".to_owned(),
            "no marketplace-ready packaging".to_owned(),
            "no LLM mesh generation".to_owned(),
            "no arbitrary character mesh import".to_owned(),
        ],
    }
}

/// Validate that the authored clay profile covers the prepared hero v1 contract
/// it reports as its source compatibility contract.
pub fn validate_prepared_template_compatibility() -> Result<(), String> {
    let template = prepared_hero_template_v1();
    template
        .validate()
        .map_err(|error| format!("prepared hero template invalid: {error}"))?;

    let fixture = fixture_catalog();
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| format!("clay profile fixture failed to compile: {error:#?}"))?;
    let profile_roles = output
        .catalog
        .family
        .part_roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<BTreeSet<_>>();
    for slot in &template.provider_slots {
        let covered = match slot.slot_id.as_str() {
            "headgear" | "hair_head_mass" => profile_roles.contains("hair_headgear"),
            other => profile_roles.contains(other),
        };
        if !covered {
            return Err(format!(
                "prepared provider slot {} is not covered by the clay profile",
                slot.slot_id
            ));
        }
    }

    let profile_controls = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<BTreeSet<_>>();
    for required in ["Body Proportions", "Head & Face", "Hair / Headgear"] {
        if !profile_controls.contains(required) {
            return Err(format!("prepared control category {required} is missing"));
        }
    }
    if template.control_profile.maximum_primary_controls > 7
        || output.catalog.customizer_profile.maximum_primary_controls > 7
        || profile_controls.len() > 7
    {
        return Err("prepared/profile controls exceed the seven-control product limit".to_owned());
    }
    if template.base_topology.base_library_fingerprint.0.is_empty() {
        return Err("prepared base library fingerprint is missing".to_owned());
    }
    Ok(())
}

/// Six named whole-character hero directions.
#[must_use]
pub fn hero_direction_specs() -> Vec<MobaHeroDirectionSpec> {
    [
        (
            "Armored Duelist",
            [
                "Armor Mass: medium-heavy",
                "Weapon / Accessory: blade set",
                "Silhouette: agile",
                "Hair / Headgear: light helmet",
            ],
        ),
        (
            "Arcane Ranger",
            [
                "Hero Archetype: arcane ranger",
                "Weapon / Accessory: staff set",
                "Head & Face: arcane mask",
                "Silhouette: lean",
            ],
        ),
        (
            "Brutal Champion",
            [
                "Hero Archetype: brutal champion",
                "Armor Mass: heavy",
                "Weapon / Accessory: axe set",
                "Body Proportions: broad",
            ],
        ),
        (
            "Agile Assassin",
            [
                "Hero Archetype: agile assassin",
                "Armor Mass: light",
                "Weapon / Accessory: dagger set",
                "Silhouette: narrow",
            ],
        ),
        (
            "Ceremonial Guardian",
            [
                "Hero Archetype: ceremonial guardian",
                "Armor Mass: ornate",
                "Weapon / Accessory: banner set",
                "Head & Face: guardian brow",
            ],
        ),
        (
            "Monster Hunter",
            [
                "Hero Archetype: monster hunter",
                "Hair / Headgear: horned hood",
                "Weapon / Accessory: trophy set",
                "Body Proportions: grounded",
            ],
        ),
    ]
    .into_iter()
    .map(|(name, summary)| MobaHeroDirectionSpec {
        name: name.to_owned(),
        summary: summary.iter().map(|line| (*line).to_owned()).collect(),
    })
    .collect()
}

fn hero_customizer_profile(family_id: &str, style_id: &str) -> shape_foundry::CustomizerProfile {
    let mut profile = crate::customizer_profile(
        family_id,
        style_id,
        vec![
            choice_control(
                "hero_archetype",
                "Hero Archetype",
                "hero_archetype",
                &HERO_ARCHETYPES,
            ),
            continuous_control(
                "body_proportions",
                "Body Proportions",
                "body_proportions",
                0.5,
                0.0,
                1.0,
            ),
            continuous_control("silhouette", "Silhouette", "silhouette", 0.5, 0.0, 1.0),
            choice_control(
                "armor_mass",
                "Armor Mass",
                "armor_mass",
                &ARMOR_MASS_OPTIONS,
            ),
            choice_control("head_face", "Head & Face", "head_face", &HEAD_FACE_OPTIONS),
            choice_control(
                "hair_headgear",
                "Hair / Headgear",
                "hair_headgear",
                &HAIR_HEADGEAR_OPTIONS,
            ),
            choice_control(
                "weapon_accessory",
                "Weapon / Accessory",
                "weapon_accessory",
                &WEAPON_ACCESSORY_OPTIONS,
            ),
        ],
    );
    profile.maximum_primary_controls = 7;
    profile.candidate_strategies = vec![
        strategy(
            "explore",
            "Explore",
            &[
                "hero_archetype",
                "body_proportions",
                "silhouette",
                "armor_mass",
                "head_face",
                "hair_headgear",
                "weapon_accessory",
            ],
        ),
        strategy(
            "silhouette",
            "Silhouette",
            &["hero_archetype", "body_proportions", "silhouette"],
        ),
        strategy(
            "armor_gear",
            "Armor/Gear",
            &["armor_mass", "hair_headgear", "weapon_accessory"],
        ),
        strategy(
            "detail",
            "Detail",
            &["armor_mass", "head_face", "hair_headgear"],
        ),
        strategy(
            "armored_duelist",
            "Armored Duelist",
            &["armor_mass", "weapon_accessory", "silhouette"],
        ),
        strategy(
            "arcane_ranger",
            "Arcane Ranger",
            &["hero_archetype", "head_face", "weapon_accessory"],
        ),
        strategy(
            "brutal_champion",
            "Brutal Champion",
            &["hero_archetype", "armor_mass", "body_proportions"],
        ),
        strategy(
            "agile_assassin",
            "Agile Assassin",
            &["hero_archetype", "armor_mass", "silhouette"],
        ),
        strategy(
            "ceremonial_guardian",
            "Ceremonial Guardian",
            &["hero_archetype", "armor_mass", "head_face"],
        ),
        strategy(
            "monster_hunter",
            "Monster Hunter",
            &["hero_archetype", "hair_headgear", "weapon_accessory"],
        ),
    ];
    profile
}

fn strategy(id: &str, label: &str, controls: &[&str]) -> CandidateStrategy {
    CandidateStrategy {
        id: id.to_owned(),
        label: label.to_owned(),
        control_ids: controls
            .iter()
            .map(|control| (*control).to_owned())
            .collect(),
    }
}

fn hero_parameter_bindings() -> Vec<ParameterBinding> {
    let mut bindings = vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "hero_archetype".to_owned(),
            role: "body".to_owned(),
            choices: choice_map(&HERO_ARCHETYPES),
        },
        scalar_binding(
            "body_proportions",
            "body",
            "geometry.rounded_box.half_extents.y",
            0.78,
            1.12,
        ),
        scalar_binding(
            "body_proportions",
            "head",
            "geometry.rounded_box.half_extents.y",
            0.20,
            0.30,
        ),
        scalar_binding(
            "silhouette",
            "body",
            "geometry.rounded_box.half_extents.x",
            0.24,
            0.48,
        ),
        scalar_binding(
            "silhouette",
            "shoulders",
            "geometry.rounded_box.half_extents.x",
            0.42,
            0.82,
        ),
        ParameterBinding::ChoiceToPrototype {
            slot: "head_face".to_owned(),
            role: "head".to_owned(),
            choices: choice_map(&HEAD_FACE_OPTIONS),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "hair_headgear".to_owned(),
            role: "hair_headgear".to_owned(),
            choices: choice_map(&HAIR_HEADGEAR_OPTIONS),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "weapon_accessory".to_owned(),
            role: "weapon".to_owned(),
            choices: suffix_choice_map(&WEAPON_ACCESSORY_OPTIONS, "weapon"),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "weapon_accessory".to_owned(),
            role: "back_accessory".to_owned(),
            choices: suffix_choice_map(&WEAPON_ACCESSORY_OPTIONS, "back"),
        },
    ];
    for (role, suffix) in [
        ("shoulders", "shoulders"),
        ("torso_armor", "torso"),
        ("belt_skirt", "belt"),
        ("gauntlets", "gauntlets"),
        ("boots", "boots"),
        ("small_detail", "detail"),
    ] {
        bindings.push(ParameterBinding::ChoiceToPrototype {
            slot: "armor_mass".to_owned(),
            role: role.to_owned(),
            choices: suffix_choice_map(&ARMOR_MASS_OPTIONS, suffix),
        });
    }
    bindings
}

fn scalar_binding(
    slot: &str,
    role: &str,
    path: &str,
    minimum: f32,
    maximum: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role.to_owned(),
        local_path: shape_asset::definition_scalar_path(crate::LOCAL_DEFINITION, path),
        transform: ScalarTransform::Ratio { minimum, maximum },
    }
}

fn hero_fragments() -> Vec<RecipeFragment> {
    let mut fragments = Vec::new();
    for spec in [
        fragment_spec(
            "armored_duelist",
            "body",
            [0.34, 0.92, 0.20],
            0.09,
            [0.0, 0.96, 0.0],
            None,
        ),
        fragment_spec(
            "arcane_ranger",
            "body",
            [0.28, 0.98, 0.18],
            0.08,
            [0.0, 0.98, 0.0],
            None,
        ),
        fragment_spec(
            "brutal_champion",
            "body",
            [0.44, 0.90, 0.24],
            0.10,
            [0.0, 0.94, 0.0],
            None,
        ),
        fragment_spec(
            "agile_assassin",
            "body",
            [0.25, 0.88, 0.16],
            0.07,
            [0.0, 0.92, 0.0],
            None,
        ),
        fragment_spec(
            "ceremonial_guardian",
            "body",
            [0.39, 0.94, 0.22],
            0.10,
            [0.0, 0.96, 0.0],
            None,
        ),
        fragment_spec(
            "monster_hunter",
            "body",
            [0.42, 1.00, 0.26],
            0.11,
            [0.0, 1.0, 0.0],
            None,
        ),
    ] {
        fragments.push(rounded_fragment(spec));
    }
    for spec in [
        fragment_spec(
            "focused_visor",
            "head",
            [0.22, 0.24, 0.20],
            0.08,
            [0.0, 2.40, 0.0],
            None,
        ),
        fragment_spec(
            "arcane_mask",
            "head",
            [0.20, 0.26, 0.18],
            0.07,
            [0.0, 2.42, 0.0],
            None,
        ),
        fragment_spec(
            "brutal_jaw",
            "head",
            [0.26, 0.24, 0.22],
            0.07,
            [0.0, 2.40, 0.0],
            None,
        ),
        fragment_spec(
            "guardian_brow",
            "head",
            [0.24, 0.27, 0.21],
            0.08,
            [0.0, 2.42, 0.0],
            None,
        ),
        fragment_spec(
            "hunter_jaw",
            "head",
            [0.25, 0.25, 0.23],
            0.07,
            [0.0, 2.41, 0.0],
            None,
        ),
    ] {
        fragments.push(rounded_fragment(spec));
    }
    for spec in [
        fragment_spec(
            "swept_hair",
            "hair_headgear",
            [0.26, 0.12, 0.18],
            0.06,
            [0.0, 2.96, -0.02],
            None,
        ),
        fragment_spec(
            "light_helmet",
            "hair_headgear",
            [0.27, 0.13, 0.22],
            0.05,
            [0.0, 2.95, 0.0],
            None,
        ),
        fragment_spec(
            "crest_helmet",
            "hair_headgear",
            [0.31, 0.17, 0.20],
            0.04,
            [0.0, 3.00, 0.0],
            None,
        ),
        fragment_spec(
            "hooded_mass",
            "hair_headgear",
            [0.28, 0.18, 0.24],
            0.08,
            [0.0, 3.00, 0.0],
            None,
        ),
        fragment_spec(
            "horned_hood",
            "hair_headgear",
            [0.36, 0.16, 0.23],
            0.05,
            [0.0, 2.98, 0.0],
            None,
        ),
    ] {
        fragments.push(rounded_fragment(spec));
    }
    for armor in armor_fragment_specs() {
        fragments.push(rounded_fragment(armor));
    }
    for weapon in weapon_fragment_specs() {
        fragments.push(rounded_fragment(weapon));
    }
    fragments
}

fn armor_fragment_specs() -> Vec<FragmentSpec> {
    let mut fragments = Vec::new();
    for (option, shoulder, torso, belt, gauntlet, boot, detail_count) in [
        (
            "light_armor",
            [0.44, 0.09, 0.14],
            [0.34, 0.34, 0.09],
            [0.33, 0.08, 0.10],
            [0.10, 0.20, 0.10],
            [0.13, 0.18, 0.12],
            4,
        ),
        (
            "duelist_mail",
            [0.56, 0.11, 0.16],
            [0.39, 0.38, 0.11],
            [0.39, 0.10, 0.12],
            [0.12, 0.23, 0.11],
            [0.15, 0.20, 0.13],
            5,
        ),
        (
            "heavy_plate",
            [0.70, 0.14, 0.20],
            [0.46, 0.42, 0.14],
            [0.45, 0.13, 0.14],
            [0.15, 0.26, 0.13],
            [0.18, 0.23, 0.15],
            6,
        ),
        (
            "ceremonial_plate",
            [0.64, 0.13, 0.18],
            [0.43, 0.44, 0.13],
            [0.46, 0.14, 0.13],
            [0.13, 0.24, 0.12],
            [0.16, 0.22, 0.14],
            7,
        ),
        (
            "hunter_leathers",
            [0.60, 0.12, 0.19],
            [0.41, 0.36, 0.12],
            [0.43, 0.12, 0.16],
            [0.14, 0.24, 0.14],
            [0.17, 0.24, 0.16],
            5,
        ),
    ] {
        fragments.push(fragment_spec(
            &format!("{option}_shoulders"),
            "shoulders",
            shoulder,
            0.04,
            [0.0, 1.78, 0.50],
            None,
        ));
        fragments.push(fragment_spec(
            &format!("{option}_torso"),
            "torso_armor",
            torso,
            0.045,
            [0.0, 1.14, 0.52],
            None,
        ));
        fragments.push(fragment_spec(
            &format!("{option}_belt"),
            "belt_skirt",
            belt,
            0.035,
            [0.0, 0.52, 0.50],
            None,
        ));
        fragments.push(fragment_spec(
            &format!("{option}_gauntlets"),
            "gauntlets",
            gauntlet,
            0.025,
            [-0.62, 0.95, 0.46],
            Some(ArraySpec {
                count: 2,
                offset: [1.24, 0.0, 0.0],
            }),
        ));
        fragments.push(fragment_spec(
            &format!("{option}_boots"),
            "boots",
            boot,
            0.035,
            [-0.24, 0.10, 0.48],
            Some(ArraySpec {
                count: 2,
                offset: [0.48, 0.0, 0.0],
            }),
        ));
        fragments.push(fragment_spec(
            &format!("{option}_detail"),
            "small_detail",
            [0.035, 0.035, 0.025],
            0.008,
            [-0.22, 1.18, 0.70],
            Some(ArraySpec {
                count: detail_count,
                offset: [0.09, -0.08, 0.0],
            }),
        ));
    }
    fragments
}

fn weapon_fragment_specs() -> Vec<FragmentSpec> {
    let mut fragments = Vec::new();
    for (option, weapon, back) in [
        (
            "blade_and_scabbard",
            [0.08, 0.72, 0.045],
            [0.09, 0.60, 0.05],
        ),
        ("staff_and_cloak", [0.055, 0.88, 0.055], [0.34, 0.58, 0.045]),
        ("axe_and_trophy", [0.18, 0.58, 0.06], [0.25, 0.30, 0.08]),
        ("dagger_and_smoke", [0.07, 0.42, 0.04], [0.30, 0.36, 0.035]),
        ("banner_and_mace", [0.13, 0.64, 0.055], [0.22, 0.52, 0.055]),
    ] {
        fragments.push(fragment_spec(
            &format!("{option}_weapon"),
            "weapon",
            weapon,
            0.025,
            [0.92, 1.02, 0.18],
            None,
        ));
        fragments.push(fragment_spec(
            &format!("{option}_back"),
            "back_accessory",
            back,
            0.03,
            [0.0, 1.16, -0.56],
            None,
        ));
    }
    fragments
}

#[derive(Clone)]
struct FragmentSpec {
    id: String,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
    array: Option<ArraySpec>,
}

#[derive(Clone, Copy)]
struct ArraySpec {
    count: u32,
    offset: [f32; 3],
}

fn fragment_spec(
    id: &str,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
    array: Option<ArraySpec>,
) -> FragmentSpec {
    FragmentSpec {
        id: id.to_owned(),
        role,
        half_extents,
        radius,
        translation,
        array,
    }
}

fn rounded_fragment(spec: FragmentSpec) -> RecipeFragment {
    let operations = spec
        .array
        .map(|array| vec![linear_array(1, array.count, array.offset)])
        .unwrap_or_default();
    let mut scalar_paths = vec![
        ("geometry.rounded_box.half_extents.x", 0.01, 5.0, 0.01),
        ("geometry.rounded_box.half_extents.y", 0.01, 5.0, 0.01),
        ("geometry.rounded_box.half_extents.z", 0.01, 5.0, 0.01),
        ("geometry.rounded_box.radius", 0.0, 0.5, 0.01),
    ];
    if spec.array.is_some() {
        scalar_paths.push(("operation.1.linear_array.count", 1.0, 12.0, 1.0));
    }
    fragment(
        &spec.id,
        spec.role,
        GeometrySource::RoundedBox {
            half_extents: spec.half_extents,
            radius: spec.radius,
        },
        spec.translation,
        operations,
        &scalar_paths,
    )
}

fn choice_map(values: &[&str]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|value| ((*value).to_owned(), (*value).to_owned()))
        .collect()
}

fn suffix_choice_map(values: &[&str], suffix: &str) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|value| ((*value).to_owned(), format!("{value}_{suffix}")))
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
