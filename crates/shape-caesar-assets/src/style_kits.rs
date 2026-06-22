//! Project Caesar style kits authored against generic Shape Lab families.

use shape_family::{
    AllowedOperationKind, BevelPolicy, DetailModule, ExaggerationPolicy, LengthValue,
    NormalizedBevelProfile, PartPrototype, ProfileLanguage, ReadabilityThreshold, RepetitionPolicy,
    RoleProportion, STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
};

/// Roman field-engineering geometry language for Project Caesar dogfooding.
#[must_use]
pub fn roman_timber_engineering_style_kit() -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: "roman_timber_engineering".to_owned(),
        display_name: "Roman Timber Engineering".to_owned(),
        compatible_families: vec!["bridge".to_owned()],
        proportions: vec![
            RoleProportion {
                role: "support".to_owned(),
                preferred_scale: [
                    LengthValue::FamilyUnits(0.28),
                    LengthValue::FamilyUnits(0.28),
                    LengthValue::FamilyUnits(1.4),
                ],
                taper: 0.18,
            },
            RoleProportion {
                role: "span".to_owned(),
                preferred_scale: [
                    LengthValue::FamilyUnits(2.8),
                    LengthValue::FamilyUnits(0.75),
                    LengthValue::FamilyUnits(0.18),
                ],
                taper: 0.0,
            },
            RoleProportion {
                role: "deck".to_owned(),
                preferred_scale: [
                    LengthValue::FamilyUnits(2.8),
                    LengthValue::FamilyUnits(0.7),
                    LengthValue::FamilyUnits(0.08),
                ],
                taper: 0.0,
            },
        ],
        bevel_policy: BevelPolicy {
            width: LengthValue::RelativeToRole {
                role: "deck".to_owned(),
                ratio: 0.025,
            },
            segments: 1,
            profile: NormalizedBevelProfile { normalized: 0.45 },
        },
        profile_language: ProfileLanguage {
            curve_family: "rough_straight_timber".to_owned(),
            allowed_profiles: vec![
                "squared_beam".to_owned(),
                "round_pile".to_owned(),
                "plank".to_owned(),
                "stake".to_owned(),
            ],
            allow_asymmetry: true,
        },
        part_prototypes: vec![
            PartPrototype {
                id: "pointed_round_pile".to_owned(),
                display_name: "Pointed round pile".to_owned(),
                role: "support".to_owned(),
                operation_tags: vec![
                    AllowedOperationKind::Primitive,
                    AllowedOperationKind::Lathe,
                    AllowedOperationKind::Bevel,
                ],
                style_tags: vec!["timber".to_owned(), "foundation".to_owned()],
            },
            PartPrototype {
                id: "lashed_deck_plank".to_owned(),
                display_name: "Lashed deck plank".to_owned(),
                role: "deck".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Array],
                style_tags: vec!["timber".to_owned(), "walkable".to_owned()],
            },
            PartPrototype {
                id: "hewn_span_beam".to_owned(),
                display_name: "Hewn span beam".to_owned(),
                role: "span".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive],
                style_tags: vec!["timber".to_owned(), "load_path".to_owned()],
            },
            PartPrototype {
                id: "cross_brace_beam".to_owned(),
                display_name: "Cross-brace beam".to_owned(),
                role: "brace".to_owned(),
                operation_tags: vec![
                    AllowedOperationKind::Primitive,
                    AllowedOperationKind::Transform,
                    AllowedOperationKind::Array,
                ],
                style_tags: vec!["timber".to_owned(), "reinforcement".to_owned()],
            },
        ],
        detail_modules: vec![
            DetailModule {
                id: "rope_lashing".to_owned(),
                display_name: "Rope lashing".to_owned(),
                target_roles: vec!["connector".to_owned(), "brace".to_owned()],
                minimum_readability: ReadabilityThreshold {
                    pixels: 32,
                    camera_profile: "oblique".to_owned(),
                },
                tags: vec!["binding".to_owned()],
            },
            DetailModule {
                id: "end_grain_cut".to_owned(),
                display_name: "End-grain cut".to_owned(),
                target_roles: vec!["support".to_owned(), "deck".to_owned()],
                minimum_readability: ReadabilityThreshold {
                    pixels: 24,
                    camera_profile: "oblique".to_owned(),
                },
                tags: vec!["timber_detail".to_owned()],
            },
        ],
        repetition: RepetitionPolicy {
            density: 0.7,
            preferred_spacing: LengthValue::FamilyUnits(0.18),
            maximum_default_count: 18,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: true,
            allowed_axes: vec!["x".to_owned(), "y".to_owned()],
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.35,
            detail: 0.4,
        },
        tags: vec![
            "project_caesar".to_owned(),
            "roman".to_owned(),
            "timber".to_owned(),
            "field_engineering".to_owned(),
        ],
    }
}
