//! Built-in Foundry kit metadata derived from exact fixture catalogs.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use shape_family::{AssetFamilySchema, RoleProvision, StyleKit};
use shape_family_compile::{FamilyImplementation, StyleImplementation};
use shape_foundry::{
    CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION, CONTROL_PROFILE_SCHEMA_VERSION, CandidateStrategyPack,
    CatalogContentRef, CatalogVisibilityPolicy, ControlKind, ControlOptionVisibility,
    ControlProfile, ControlProfileControl, ControlProfileControlKind,
    ControlProfileTopologyBehavior, ControlTopologyBehavior, DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
    FAMILY_BLUEPRINT_SCHEMA_VERSION, FamilyBlueprint, FamilyBlueprintRole, FoundryKit,
    FoundryKitPackage, FoundryKitQualityTier, FutureMaterialVocabulary, HighLevelScalePolicy,
    KIT_CATALOG_MANIFEST_SCHEMA_VERSION, KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
    KIT_REVIEW_MANIFEST_SCHEMA_VERSION, KitCandidateStrategy, KitCatalogManifest,
    KitCompatibilityMatrix, KitReviewManifest, PROVIDER_PACK_SCHEMA_VERSION, PreviewCameraPolicy,
    ProviderPack, ProviderPackOption, ProviderSlotExpectation, QUALITY_GATE_PROFILE_SCHEMA_VERSION,
    QualityGateProfile, STYLE_PACK_SCHEMA_VERSION, StylePack, StyleProviderCompatibility,
};

use crate::{
    FoundryFixtureCatalog, box_primitive, built_in_fixture_catalogs_with_labels, flat_panel,
};

/// Return every built-in Visual Foundry profile as a curated kit package.
#[must_use]
pub fn built_in_foundry_kit_packages_with_labels() -> Vec<(&'static str, FoundryKitPackage)> {
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(label, fixture)| (label, kit_package_from_fixture(label, &fixture)))
        .collect()
}

/// Return one built-in kit package by fixture slug.
#[must_use]
pub fn built_in_foundry_kit_package(slug: &str) -> Option<FoundryKitPackage> {
    let slug = normalize_kit_slug(slug);
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .find(|(_, fixture)| fixture.slug == slug)
        .map(|(label, fixture)| kit_package_from_fixture(label, &fixture))
}

fn kit_package_from_fixture(label: &str, fixture: &FoundryFixtureCatalog) -> FoundryKitPackage {
    let family: AssetFamilySchema =
        decode_fixture_ref(fixture, &fixture.document.family_content_ref);
    let style: StyleKit = decode_fixture_ref(fixture, &fixture.document.style_content_ref);
    let family_impl: FamilyImplementation =
        decode_fixture_ref(fixture, &fixture.document.family_implementation_ref);
    let style_impl: StyleImplementation =
        decode_fixture_ref(fixture, &fixture.document.style_implementation_ref);
    let customizer: shape_foundry::CustomizerProfile =
        decode_fixture_ref(fixture, &fixture.document.customizer_profile_ref);

    let kit_id = format!("{}-kit", fixture.slug);
    let provider_pack_id = format!("{}-provider-pack", fixture.slug);
    let style_pack_id = style.id.clone();
    let control_profile_id = format!("{}-controls", fixture.slug);
    let strategy_pack_id = format!("{}-candidate-strategies", fixture.slug);
    let quality_gate_id = format!("{}-quality-gate", fixture.slug);
    let compatibility_id = format!("{}-compatibility", fixture.slug);
    let review_id = format!("{}-review", fixture.slug);
    let catalog_id = format!("{}-kit-catalog", fixture.slug);
    let tier = built_in_quality_tier(&fixture.slug);
    let hidden_reason = match tier {
        FoundryKitQualityTier::Draft => "Draft kits are hidden from the default catalog.",
        FoundryKitQualityTier::Prototype => {
            "Prototype kits require preview catalog mode before novice exposure."
        }
        FoundryKitQualityTier::Usable => {
            "Manual review is pending before default catalog exposure."
        }
        FoundryKitQualityTier::Showcase => {
            "Showcase approval is pending before default catalog exposure."
        }
    };
    let visibility = CatalogVisibilityPolicy::hidden(hidden_reason);
    let category_chips = product_category_chips(&fixture.slug);

    FoundryKitPackage {
        schema_version: shape_foundry::FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
        kit: FoundryKit {
            schema_version: shape_foundry::FOUNDRY_KIT_SCHEMA_VERSION,
            kit_id: kit_id.clone(),
            display_name: label.to_owned(),
            family_blueprint_id: family.id.clone(),
            provider_pack_id: provider_pack_id.clone(),
            style_pack_id: style_pack_id.clone(),
            control_profile_id: control_profile_id.clone(),
            candidate_strategy_pack_id: strategy_pack_id.clone(),
            quality_gate_profile_id: quality_gate_id.clone(),
            compatibility_matrix_id: compatibility_id.clone(),
            review_manifest_id: review_id.clone(),
            catalog_manifest_id: catalog_id.clone(),
            preview_camera_policy: PreviewCameraPolicy {
                policy_id: format!("{}-clay-review", fixture.slug),
                required_views: vec![
                    "front".to_owned(),
                    "three-quarter".to_owned(),
                    "side".to_owned(),
                    "back".to_owned(),
                ],
                clay_preview_required: true,
                contact_sheet_required: tier >= FoundryKitQualityTier::Usable,
            },
            quality_tier: tier,
            catalog_visibility_policy: visibility.clone(),
            source_profile_slug: Some(fixture.slug.clone()),
            category_chips,
        },
        family_blueprint: family_blueprint_from_schema(&family),
        provider_pack: provider_pack_from_fixture(
            &provider_pack_id,
            &family,
            &family_impl,
            &style_impl,
        ),
        style_pack: style_pack_from_schema(&style_pack_id, &style, &provider_pack_id),
        control_profile: control_profile_from_customizer(
            &control_profile_id,
            &family,
            &style_pack_id,
            &customizer,
        ),
        candidate_strategy_pack: candidate_strategy_pack_from_customizer(
            &strategy_pack_id,
            &customizer,
        ),
        quality_gate_profile: quality_gate_profile(&quality_gate_id, tier),
        compatibility_matrix: KitCompatibilityMatrix {
            schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
            matrix_id: compatibility_id,
            compatible_style_provider_pairs: vec![StyleProviderCompatibility {
                style_id: style_pack_id.clone(),
                provider_pack_id: provider_pack_id.clone(),
                reason: "Authored built-in family and style produce coherent whole-model output."
                    .to_owned(),
            }],
            incompatible_style_provider_pairs: Vec::new(),
        },
        review_manifest: review_manifest(&review_id, &fixture.slug, tier),
        catalog_manifest: KitCatalogManifest {
            schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id,
            kit_ids: vec![kit_id.clone()],
            default_visible_kit_ids: if visibility.default_novice_catalog {
                vec![kit_id.clone()]
            } else {
                Vec::new()
            },
            developer_preview_kit_ids: if visibility.developer_preview_catalog {
                vec![kit_id.clone()]
            } else {
                Vec::new()
            },
            hidden_kit_reasons: BTreeMap::from([(
                kit_id,
                visibility
                    .hidden_reason
                    .clone()
                    .unwrap_or_else(|| "Not enabled for the default catalog.".to_owned()),
            )]),
        },
    }
}

fn family_blueprint_from_schema(family: &AssetFamilySchema) -> FamilyBlueprint {
    let semantic_roles = family
        .part_roles
        .iter()
        .map(|role| FamilyBlueprintRole {
            role_id: role.id.clone(),
            label: role.display_name.clone(),
            required: role.required,
            tags: role.semantic_tags.clone(),
        })
        .collect::<Vec<_>>();
    let required_roles = family
        .part_roles
        .iter()
        .filter(|role| role.required)
        .map(|role| role.id.clone())
        .collect::<Vec<_>>();
    let optional_roles = family
        .part_roles
        .iter()
        .filter(|role| !role.required)
        .map(|role| role.id.clone())
        .collect::<Vec<_>>();
    let provider_slots = family
        .part_roles
        .iter()
        .filter(|role| role.provision != RoleProvision::Derived)
        .map(|role| ProviderSlotExpectation {
            slot_id: provider_slot_id(&role.id),
            role_id: role.id.clone(),
            required: role.required,
            attachment_tags: role.semantic_tags.clone(),
        })
        .collect::<Vec<_>>();
    let attachment_expectations = family
        .attachment_rules
        .iter()
        .map(|rule| shape_foundry::AttachmentExpectation {
            expectation_id: rule.id.clone(),
            from_role: rule.from_role.clone(),
            to_role: rule.to_role.clone(),
            compatibility_tags: rule.compatibility_tags.clone(),
            required: rule.required,
        })
        .collect();
    FamilyBlueprint {
        schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
        family_id: family.id.clone(),
        display_name: family.display_name.clone(),
        semantic_roles,
        required_roles,
        optional_roles,
        provider_slots,
        attachment_expectations,
        scale_policy: HighLevelScalePolicy {
            label: "Asset-family normalized scale".to_owned(),
            allowed_range: Some("Authored family controls define valid scale.".to_owned()),
        },
        export_part_naming_policy: shape_foundry::ExportPartNamingPolicy {
            strategy: "role-based readable part names".to_owned(),
            required_part_names: family
                .part_roles
                .iter()
                .map(|role| role.id.clone())
                .collect(),
        },
    }
}

fn provider_pack_from_fixture(
    provider_pack_id: &str,
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
) -> ProviderPack {
    let mut provider_by_role = family_impl.default_role_providers.clone();
    provider_by_role.extend(style_impl.default_role_providers.clone());
    let roles = family
        .part_roles
        .iter()
        .filter(|role| role.provision != RoleProvision::Derived)
        .collect::<Vec<_>>();
    let provider_options = roles
        .iter()
        .map(|role| {
            let provider_id = provider_by_role
                .get(&role.id)
                .cloned()
                .unwrap_or_else(|| format!("{}-default", role.id));
            ProviderPackOption {
                option_id: provider_id.clone(),
                slot_id: provider_slot_id(&role.id),
                label: role.display_name.clone(),
                semantic_roles: vec![role.id.clone()],
                compatibility_tags: role.semantic_tags.clone(),
                detail_density_tags: family.tags.clone(),
                triangle_budget_estimate: Some(8_000),
            }
        })
        .collect::<Vec<_>>();
    let provider_slots_supplied = roles
        .iter()
        .map(|role| provider_slot_id(&role.id))
        .collect::<Vec<_>>();
    let semantic_role_coverage = roles.iter().map(|role| role.id.clone()).collect::<Vec<_>>();
    let triangle_budget_estimates = provider_options
        .iter()
        .map(|option| {
            (
                option.option_id.clone(),
                option.triangle_budget_estimate.unwrap_or(8_000),
            )
        })
        .collect();
    ProviderPack {
        schema_version: PROVIDER_PACK_SCHEMA_VERSION,
        pack_id: provider_pack_id.to_owned(),
        family_id: Some(family.id.clone()),
        compatible_family_ids: vec![family.id.clone()],
        provider_slots_supplied,
        provider_options,
        semantic_role_coverage,
        socket_attachment_tags: family
            .attachment_rules
            .iter()
            .flat_map(|rule| rule.compatibility_tags.iter().cloned())
            .collect(),
        detail_density_tags: family.tags.clone(),
        triangle_budget_estimates,
        compatibility_tags: family.tags.clone(),
    }
}

fn style_pack_from_schema(
    style_pack_id: &str,
    style: &StyleKit,
    provider_pack_id: &str,
) -> StylePack {
    StylePack {
        schema_version: STYLE_PACK_SCHEMA_VERSION,
        style_id: style_pack_id.to_owned(),
        display_name: style.display_name.clone(),
        compatible_family_ids: style.compatible_families.clone(),
        bevel_language: format!("{:?}", style.bevel_policy),
        proportion_language: "Family-scoped authored proportions".to_owned(),
        detail_density_policy: format!("{:?}", style.repetition),
        silhouette_exaggeration_policy: format!("{:?}", style.exaggeration),
        symmetry_asymmetry_policy: format!("{:?}", style.symmetry),
        allowed_provider_tags: Vec::new(),
        forbidden_provider_tags: Vec::new(),
        compatible_provider_packs: vec![provider_pack_id.to_owned()],
        incompatible_provider_packs: Vec::new(),
        future_material_vocabulary: Some(FutureMaterialVocabulary {
            label: "Metadata-only future material vocabulary".to_owned(),
            tags: style.tags.clone(),
        }),
    }
}

fn control_profile_from_customizer(
    profile_id: &str,
    family: &AssetFamilySchema,
    style_pack_id: &str,
    customizer: &shape_foundry::CustomizerProfile,
) -> ControlProfile {
    ControlProfile {
        schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
        profile_id: profile_id.to_owned(),
        family_id: family.id.clone(),
        style_id: Some(style_pack_id.to_owned()),
        maximum_primary_controls: customizer
            .maximum_primary_controls
            .min(DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS),
        controls: customizer
            .controls
            .iter()
            .map(control_profile_control)
            .collect(),
    }
}

fn control_profile_control(control: &shape_foundry::CustomizerControl) -> ControlProfileControl {
    let (kind, provider_slot) = match &control.kind {
        ControlKind::ContinuousAxis { .. } => (ControlProfileControlKind::Continuous, None),
        ControlKind::IntegerStepper { .. } => (ControlProfileControlKind::Integer, None),
        ControlKind::Toggle { .. } => (ControlProfileControlKind::Toggle, None),
        ControlKind::ChoiceGallery { .. } => (ControlProfileControlKind::Choice, None),
        ControlKind::ProviderGallery { role, .. } => (
            ControlProfileControlKind::Choice,
            Some(provider_slot_id(role)),
        ),
    };
    let topology_behavior = match control.topology_behavior {
        ControlTopologyBehavior::TopologyPreserving => {
            ControlProfileTopologyBehavior::TopologyPreserving
        }
        ControlTopologyBehavior::TopologyChanging => {
            ControlProfileTopologyBehavior::TopologyChanging
        }
        ControlTopologyBehavior::RuntimeOnly => ControlProfileTopologyBehavior::RuntimeOnly,
    };
    ControlProfileControl {
        control_id: control.id.clone(),
        label: product_safe_catalog_label(&control.label),
        description: "Changes the whole-model result.".to_owned(),
        kind,
        owned_family_slots: control
            .bindings
            .iter()
            .map(|binding| binding.slot.clone())
            .collect(),
        owned_provider_slots: provider_slot.into_iter().collect(),
        visible_effect_expectation: "A reviewer should see a whole-model visual difference."
            .to_owned(),
        topology_behavior,
        option_visibility: ControlOptionVisibility {
            hide_invalid_from_novices: true,
            show_plain_language_reasons: true,
        },
        default_locked: false,
        primary: control.primary,
        visible: control.visible,
    }
}

fn candidate_strategy_pack_from_customizer(
    pack_id: &str,
    customizer: &shape_foundry::CustomizerProfile,
) -> CandidateStrategyPack {
    let visible_controls = customizer
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| control.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let strategies = customizer
        .candidate_strategies
        .iter()
        .map(|strategy| {
            let name = product_safe_catalog_label(&strategy.label);
            KitCandidateStrategy {
                strategy_id: strategy.id.clone(),
                name: name.clone(),
                allowed_controls: strategy
                    .control_ids
                    .iter()
                    .filter(|control_id| visible_controls.contains(control_id.as_str()))
                    .cloned()
                    .collect(),
                explanation_templates: vec![format!("Generate a {name} whole-model option.")],
            }
        })
        .collect::<Vec<_>>();
    let allowed_controls = strategies
        .iter()
        .flat_map(|strategy| strategy.allowed_controls.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    CandidateStrategyPack {
        schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
        pack_id: pack_id.to_owned(),
        strategies,
        allowed_controls,
        allowed_provider_choices: BTreeMap::new(),
        diversity_goals: vec![
            "Six coherent whole-model options".to_owned(),
            "Respect locked controls".to_owned(),
        ],
        invalid_state_rejection_policy: "Reject candidates that fail compile or validation."
            .to_owned(),
        lock_respect_policy: "Candidate generation must not alter locked controls.".to_owned(),
    }
}

fn product_safe_catalog_label(value: &str) -> String {
    value
        .replace("Archetype", "Type")
        .replace("archetype", "type")
}

fn quality_gate_profile(profile_id: &str, tier: FoundryKitQualityTier) -> QualityGateProfile {
    QualityGateProfile {
        schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
        profile_id: profile_id.to_owned(),
        required_tier: if tier >= FoundryKitQualityTier::Usable {
            FoundryKitQualityTier::Usable
        } else {
            tier
        },
        mesh_gates: vec![
            "Compile succeeds".to_owned(),
            "Model validation has no blocking errors".to_owned(),
            "Required roles are covered".to_owned(),
        ],
        candidate_gates: vec![
            "Direction candidates compile".to_owned(),
            "Whole-model previews are non-placeholder".to_owned(),
        ],
        contact_sheet_gates: vec![
            "Clay contact sheet is present for Usable or Showcase".to_owned(),
        ],
        export_gates: vec!["Model package export and reopen verification pass".to_owned()],
        manual_review_gates: vec![
            "Human approval required before default novice exposure".to_owned(),
            "Adversarial visual review required for Showcase".to_owned(),
        ],
    }
}

fn review_manifest(
    manifest_id: &str,
    slug: &str,
    tier: FoundryKitQualityTier,
) -> KitReviewManifest {
    let has_hq_artifacts = tier >= FoundryKitQualityTier::Usable;
    KitReviewManifest {
        schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
        manifest_id: manifest_id.to_owned(),
        tier_requested: tier,
        tier_achieved: tier,
        reviewer: None,
        human_approval_marker: false,
        adversarial_review_marker: false,
        visual_review_notes: vec![
            "Automated built-in metadata generated from exact fixture catalog.".to_owned(),
        ],
        contact_sheet_paths: if has_hq_artifacts {
            vec![format!("target/hq-benchmark/{slug}/contact-sheet.png")]
        } else {
            Vec::new()
        },
        benchmark_refs: if has_hq_artifacts {
            vec![format!("target/hq-benchmark/{slug}/quality-report.json")]
        } else {
            Vec::new()
        },
        known_limitations: Vec::new(),
        blocked_reasons: vec!["Manual review pending before novice catalog exposure.".to_owned()],
    }
}

fn decode_fixture_ref<T: DeserializeOwned>(
    fixture: &FoundryFixtureCatalog,
    content_ref: &CatalogContentRef,
) -> T {
    let entry = fixture
        .entries
        .get(&content_ref.stable_id)
        .unwrap_or_else(|| {
            panic!(
                "fixture {} is missing {}",
                fixture.slug, content_ref.stable_id
            )
        });
    serde_json::from_str(&entry.canonical_json).unwrap_or_else(|error| {
        panic!(
            "fixture {} failed to decode {}: {error}",
            fixture.slug, content_ref.stable_id
        )
    })
}

fn provider_slot_id(role_id: &str) -> String {
    format!("{role_id}-slot")
}

fn built_in_quality_tier(slug: &str) -> FoundryKitQualityTier {
    match slug {
        box_primitive::BOX_PRIMITIVE_SLUG
        | box_primitive::LIDDED_BOX_SLUG
        | flat_panel::FLAT_PANEL_PRIMITIVE_SLUG
        | flat_panel::HINGED_PANEL_SLUG => FoundryKitQualityTier::Usable,
        _ => FoundryKitQualityTier::Prototype,
    }
}

fn product_category_chips(slug: &str) -> Vec<String> {
    match slug {
        box_primitive::BOX_PRIMITIVE_SLUG => vec!["Primitive", "Box"],
        box_primitive::LIDDED_BOX_SLUG => vec!["Box", "Lidded"],
        flat_panel::FLAT_PANEL_PRIMITIVE_SLUG => vec!["Primitive", "Panel"],
        flat_panel::HINGED_PANEL_SLUG => vec!["Panel", "Hinged"],
        _ => vec!["Asset"],
    }
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn normalize_kit_slug(slug: &str) -> String {
    match slug {
        "box" | "box_primitive" => box_primitive::BOX_PRIMITIVE_SLUG.to_owned(),
        "lidded_box" => box_primitive::LIDDED_BOX_SLUG.to_owned(),
        "flat_panel" | "panel" => flat_panel::FLAT_PANEL_PRIMITIVE_SLUG.to_owned(),
        "hinged_panel" => flat_panel::HINGED_PANEL_SLUG.to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use shape_foundry::{FoundryKitQualityTier, validate_foundry_kit_package};

    use super::*;

    #[test]
    fn built_in_profiles_have_valid_kit_metadata() {
        let packages = built_in_foundry_kit_packages_with_labels();
        assert_eq!(packages.len(), 4);
        for (label, package) in packages {
            let report = validate_foundry_kit_package(&package);
            assert!(
                report.is_valid(),
                "{label} kit metadata failed validation: {:?}",
                report.issues
            );
            assert_eq!(package.kit.display_name, label);
            assert!(package.kit.source_profile_slug.is_some());
            assert!(
                package
                    .control_profile
                    .controls
                    .iter()
                    .filter(|control| control.primary && control.visible)
                    .count()
                    <= 7
            );
        }
    }

    #[test]
    fn box_family_kits_are_usable_builtins() {
        let package =
            built_in_foundry_kit_package(box_primitive::BOX_PRIMITIVE_SLUG).expect("box kit");
        assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Usable);
        assert_eq!(
            package.kit.source_profile_slug.as_deref(),
            Some(box_primitive::BOX_PRIMITIVE_SLUG)
        );
        assert_eq!(package.kit.category_chips, vec!["Primitive", "Box"]);

        let lidded_package =
            built_in_foundry_kit_package(box_primitive::LIDDED_BOX_SLUG).expect("lidded kit");
        assert_eq!(
            lidded_package.kit.quality_tier,
            FoundryKitQualityTier::Usable
        );
        assert_eq!(
            lidded_package.kit.source_profile_slug.as_deref(),
            Some(box_primitive::LIDDED_BOX_SLUG)
        );
        assert_eq!(lidded_package.kit.category_chips, vec!["Box", "Lidded"]);

        let panel_package = built_in_foundry_kit_package(flat_panel::FLAT_PANEL_PRIMITIVE_SLUG)
            .expect("flat panel kit");
        assert_eq!(
            panel_package.kit.quality_tier,
            FoundryKitQualityTier::Usable
        );
        assert_eq!(
            panel_package.kit.source_profile_slug.as_deref(),
            Some(flat_panel::FLAT_PANEL_PRIMITIVE_SLUG)
        );
        assert_eq!(panel_package.kit.category_chips, vec!["Primitive", "Panel"]);

        let hinged_package =
            built_in_foundry_kit_package(flat_panel::HINGED_PANEL_SLUG).expect("hinged kit");
        assert_eq!(
            hinged_package.kit.quality_tier,
            FoundryKitQualityTier::Usable
        );
        assert_eq!(
            hinged_package.kit.source_profile_slug.as_deref(),
            Some(flat_panel::HINGED_PANEL_SLUG)
        );
        assert_eq!(hinged_package.kit.category_chips, vec!["Panel", "Hinged"]);
    }

    #[test]
    fn kit_slug_aliases_resolve_to_canonical_built_ins() {
        let box_kit = built_in_foundry_kit_package("box").expect("box alias");
        assert_eq!(
            box_kit.kit.source_profile_slug.as_deref(),
            Some(box_primitive::BOX_PRIMITIVE_SLUG)
        );

        let panel_kit = built_in_foundry_kit_package("panel").expect("panel alias");
        assert_eq!(
            panel_kit.kit.source_profile_slug.as_deref(),
            Some(flat_panel::FLAT_PANEL_PRIMITIVE_SLUG)
        );

        let hinged_kit = built_in_foundry_kit_package("hinged_panel").expect("hinged alias");
        assert_eq!(
            hinged_kit.kit.source_profile_slug.as_deref(),
            Some(flat_panel::HINGED_PANEL_SLUG)
        );
    }
}
