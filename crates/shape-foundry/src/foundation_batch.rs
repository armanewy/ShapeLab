//! Deterministic internal foundation draft batches.
//!
//! Wave 37 uses the SDK-free foundation schema to build a structured backlog
//! of weapon and armor kit foundations. These are Draft/Internal authoring
//! records only; they are not novice catalog content and do not carry geometry.

use serde::{Deserialize, Serialize};

use crate::{
    ControlProfileControlKind, ControlProfileTopologyBehavior, DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
    DraftCandidateStrategy, DraftCandidateStrategyPack, DraftCompatibilityMatrix,
    DraftCompatibilityRule, DraftControl, DraftControlProfile, DraftFamilyBlueprint,
    DraftFamilyRole, DraftProviderPack, DraftProviderSlot, DraftProviderTaxonomy,
    DraftQualityGateProfile, DraftReviewChecklist, DraftSocket, DraftStylePack, DraftTestPlan,
    FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION, FoundationCatalogVisibility,
    FoundationDraftSourceKind, FoundationQualityTarget, FoundryFoundationDraft,
    foundation_adversarial_report, validate_foundation_draft,
};

/// Expected Wave 37 family IDs.
pub const WAVE37_WEAPON_ARMOR_FAMILY_IDS: &[&str] = &[
    "sword",
    "dagger",
    "axe",
    "mace_hammer",
    "spear_polearm",
    "bow_crossbow",
    "staff_wand",
    "shield",
    "scifi_rifle_blaster",
    "grenade_device_prop",
    "helmet",
    "pauldron",
    "chest_armor",
    "gauntlet",
    "boot",
    "belt",
    "cape_back_accessory",
    "mask",
    "hero_accessory_set",
];

/// Style packs each Wave 37 foundation draft must reason about.
pub const WAVE37_STYLE_PACK_LABELS: &[&str] = &[
    "Rustic Medieval",
    "Clean Sci-Fi",
    "Ancient Ruin",
    "Stylized Cozy",
    "Dark Fantasy",
    "Toylike Low-Poly",
    "Industrial Heavy",
    "Elegant Elven",
    "Brutalist Stone",
    "MOBA Heroic",
];

/// Review category assigned to a foundation draft.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationBatchReviewCategory {
    /// Strong candidate for later curated kit authoring.
    Promising,
    /// Needs fewer roles, controls, or provider choices before authoring.
    NeedsSimplification,
    /// Too broad or abstract to become a focused kit without pruning.
    OverAbstracted,
    /// Needs distinctive authored visual ingredients before promotion.
    MissingArtIngredients,
    /// Likely to mix incompatible visual languages without strict pruning.
    HighRiskOfStyleSalad,
}

impl FoundationBatchReviewCategory {
    /// Product/report label for the category.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Promising => "promising",
            Self::NeedsSimplification => "needs simplification",
            Self::OverAbstracted => "over-abstracted",
            Self::MissingArtIngredients => "missing art ingredients",
            Self::HighRiskOfStyleSalad => "high risk of style salad",
        }
    }
}

/// Summary row for the Wave 37 foundation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationBatchSummaryRow {
    /// Foundation family ID.
    pub family_id: String,
    /// Draft ID.
    pub draft_id: String,
    /// Draft category.
    pub category: String,
    /// Human review category.
    pub review_category: FoundationBatchReviewCategory,
    /// Whether this is a good later candidate for Wave 38 curation.
    pub good_candidate_for_showcase_gear_pack: bool,
    /// Validation issue count at batch creation time.
    pub validation_issue_count: usize,
    /// Deterministic adversarial question count.
    pub adversarial_question_count: usize,
    /// Plain-language review notes.
    pub notes: Vec<String>,
}

/// Normalize a style-pack label into a deterministic compatibility ID.
#[must_use]
pub fn foundation_style_pack_id(label: &str) -> String {
    normalize_id(label)
}

/// Return the deterministic Wave 37 weapon and armor foundation draft batch.
#[must_use]
pub fn weapon_armor_foundation_draft_batch() -> Vec<FoundryFoundationDraft> {
    wave37_specs()
        .iter()
        .map(build_wave37_draft)
        .collect::<Vec<_>>()
}

/// Return summary rows for the deterministic Wave 37 foundation batch.
#[must_use]
pub fn weapon_armor_foundation_batch_summary() -> Vec<FoundationBatchSummaryRow> {
    wave37_specs()
        .into_iter()
        .map(|spec| {
            let draft = build_wave37_draft(&spec);
            FoundationBatchSummaryRow {
                family_id: draft.family_blueprint.family_id.clone(),
                draft_id: draft.draft_id.clone(),
                category: draft.category.clone(),
                review_category: spec.review_category,
                good_candidate_for_showcase_gear_pack: spec.showcase_candidate,
                validation_issue_count: validate_foundation_draft(&draft).issues.len(),
                adversarial_question_count: foundation_adversarial_report(&draft).questions.len(),
                notes: spec.notes.iter().map(|note| (*note).to_owned()).collect(),
            }
        })
        .collect()
}

struct FoundationBatchSpec {
    category: &'static str,
    family_id: &'static str,
    display_name: &'static str,
    roles: Vec<RoleSpec>,
    slots: Vec<SlotSpec>,
    provider_options: Vec<&'static str>,
    controls: Vec<ControlSpec>,
    strategy_names: Vec<&'static str>,
    review_category: FoundationBatchReviewCategory,
    showcase_candidate: bool,
    scale_policy: &'static str,
    notes: Vec<&'static str>,
}

#[derive(Clone)]
struct RoleSpec {
    role_id: &'static str,
    label: &'static str,
    required: bool,
    tags: Vec<&'static str>,
}

#[derive(Clone)]
struct SlotSpec {
    slot_id: &'static str,
    role_id: &'static str,
    required: bool,
    tags: Vec<&'static str>,
}

#[derive(Clone)]
struct ControlSpec {
    control_id: &'static str,
    label: &'static str,
    description: &'static str,
    kind: ControlProfileControlKind,
    owned_slots: Vec<&'static str>,
    topology_behavior: ControlProfileTopologyBehavior,
    visible_effect_expectation: &'static str,
}

fn build_wave37_draft(spec: &FoundationBatchSpec) -> FoundryFoundationDraft {
    let family_id = normalize_id(spec.family_id);
    let style_id = format!("{family_id}_wave37_style_policy");
    let taxonomy_id = format!("{family_id}_wave37_taxonomy");
    let provider_pack_ids = spec
        .provider_options
        .iter()
        .map(|option| format!("{}_{}", family_id, normalize_id(option)))
        .collect::<Vec<_>>();
    let provider_slots = spec
        .slots
        .iter()
        .map(|slot| slot.slot_id.to_owned())
        .collect::<Vec<_>>();
    let required_roles = spec
        .roles
        .iter()
        .filter(|role| role.required)
        .map(|role| role.role_id.to_owned())
        .collect::<Vec<_>>();
    let optional_roles = spec
        .roles
        .iter()
        .filter(|role| !role.required)
        .map(|role| role.role_id.to_owned())
        .collect::<Vec<_>>();
    let anchor_role = required_roles
        .first()
        .cloned()
        .unwrap_or_else(|| "body".to_owned());

    FoundryFoundationDraft {
        schema_version: FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION,
        draft_id: format!("{family_id}_wave37_foundation_draft"),
        source_kind: FoundationDraftSourceKind::GeneratedFixture,
        quality_target: FoundationQualityTarget::Draft,
        catalog_visibility: FoundationCatalogVisibility::InternalOnly,
        human_review_required: true,
        publish_allowed: false,
        category: spec.category.to_owned(),
        family_blueprint: DraftFamilyBlueprint {
            family_id: family_id.clone(),
            display_name: spec.display_name.to_owned(),
            roles: spec
                .roles
                .iter()
                .map(|role| DraftFamilyRole {
                    role_id: role.role_id.to_owned(),
                    label: role.label.to_owned(),
                    required: role.required,
                    tags: role.tags.iter().map(|tag| (*tag).to_owned()).collect(),
                })
                .collect(),
            required_roles,
            optional_roles,
            sockets: spec
                .roles
                .iter()
                .filter(|role| role.role_id != anchor_role)
                .map(|role| DraftSocket {
                    socket_id: format!("{}_to_{}", role.role_id, anchor_role),
                    from_role: role.role_id.to_owned(),
                    to_role: anchor_role.clone(),
                    compatibility_tags: vec![
                        "gear_attachment".to_owned(),
                        normalize_id(spec.category),
                    ],
                    required: role.required,
                })
                .collect(),
            export_part_names: spec
                .roles
                .iter()
                .map(|role| role.label.to_owned())
                .collect(),
        },
        provider_taxonomy: DraftProviderTaxonomy {
            taxonomy_id,
            provider_slots: spec
                .slots
                .iter()
                .map(|slot| DraftProviderSlot {
                    slot_id: slot.slot_id.to_owned(),
                    role_id: slot.role_id.to_owned(),
                    required: slot.required,
                    compatibility_tags: slot.tags.iter().map(|tag| (*tag).to_owned()).collect(),
                })
                .collect(),
            provider_packs: provider_pack_ids
                .iter()
                .zip(spec.provider_options.iter())
                .map(|(pack_id, label)| DraftProviderPack {
                    pack_id: pack_id.clone(),
                    label: (*label).to_owned(),
                    supplied_slots: provider_slots.clone(),
                    compatibility_tags: vec![
                        "wave37_foundation".to_owned(),
                        normalize_id(spec.category),
                    ],
                })
                .collect(),
        },
        style_pack: DraftStylePack {
            style_id: style_id.clone(),
            display_name: "Wave 37 Gear Style Policy".to_owned(),
            bevel_language: "Use readable clay forms; bevel taste must be reviewed before promotion."
                .to_owned(),
            proportion_language: spec.scale_policy.to_owned(),
            detail_density_policy: "Keep Draft detail placeholders sparse until authored contact sheets prove the controls matter."
                .to_owned(),
            silhouette_policy: "Whole-model silhouette must read at 128px before any Usable or Showcase claim."
                .to_owned(),
            symmetry_policy: "Symmetry is the default; asymmetric damage or hero identity needs explicit review."
                .to_owned(),
            allowed_provider_tags: vec![
                "wave37_foundation".to_owned(),
                normalize_id(spec.category),
            ],
            forbidden_provider_tags: vec![
                "photoreal_material".to_owned(),
                "raw_import".to_owned(),
                "unreviewed_mesh".to_owned(),
            ],
            compatibility_style_ids: WAVE37_STYLE_PACK_LABELS
                .iter()
                .map(|label| foundation_style_pack_id(label))
                .collect(),
        },
        control_profile: DraftControlProfile {
            profile_id: format!("{family_id}_wave37_controls"),
            maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
            controls: spec
                .controls
                .iter()
                .map(|control| DraftControl {
                    control_id: control.control_id.to_owned(),
                    label: control.label.to_owned(),
                    description: control.description.to_owned(),
                    kind: control.kind,
                    primary: true,
                    visible: true,
                    owned_family_slots: Vec::new(),
                    owned_provider_slots: control
                        .owned_slots
                        .iter()
                        .map(|slot| (*slot).to_owned())
                        .collect(),
                    topology_behavior: control.topology_behavior,
                    visible_effect_expectation: control.visible_effect_expectation.to_owned(),
                })
                .collect(),
        },
        candidate_strategy_pack: DraftCandidateStrategyPack {
            pack_id: format!("{family_id}_wave37_strategies"),
            strategies: spec
                .strategy_names
                .iter()
                .map(|name| DraftCandidateStrategy {
                    strategy_id: normalize_id(name),
                    name: (*name).to_owned(),
                    explanation: format!(
                        "{name} direction adjusts visible controls and whole-model choices."
                    ),
                    allowed_controls: spec
                        .controls
                        .iter()
                        .map(|control| control.control_id.to_owned())
                        .collect(),
                    allowed_provider_changes: provider_slots.clone(),
                })
                .collect(),
            diversity_goals: vec![
                "readable silhouette spread".to_owned(),
                "clear mass and proportion contrast".to_owned(),
                "distinct detail density without style salad".to_owned(),
            ],
        },
        compatibility_matrix: DraftCompatibilityMatrix {
            matrix_id: format!("{family_id}_wave37_compatibility"),
            rules: compatibility_rules(spec, &style_id, &provider_pack_ids),
        },
        quality_gate_profile: Some(DraftQualityGateProfile {
            profile_id: format!("{family_id}_wave37_quality"),
            validation_required: true,
            contact_sheet_required: false,
            package_required: true,
            human_review_required: true,
            adversarial_review_required: false,
            manual_review_gates: vec![
                "No missing required roles.".to_owned(),
                "No attachment failures between required parts.".to_owned(),
                "No self-intersections above the accepted threshold.".to_owned(),
                "Triangle budget target is recorded before authoring.".to_owned(),
                "Silhouette remains visible at 128px.".to_owned(),
                "All primary controls visibly matter.".to_owned(),
                "Six candidate survivors compile and render.".to_owned(),
                "Whole-model previews render without placeholders.".to_owned(),
                "Export and reopen succeeds before promotion.".to_owned(),
                "Contact sheet required for Usable or Showcase.".to_owned(),
                "Human review required for Showcase.".to_owned(),
            ],
        }),
        test_plan: DraftTestPlan {
            test_plan_id: format!("{family_id}_wave37_tests"),
            tests: vec![
                "Validate foundation draft schema.".to_owned(),
                "Generate deterministic adversarial report.".to_owned(),
                "Confirm Draft/Internal visibility only.".to_owned(),
                "Check seven-or-fewer primary controls.".to_owned(),
                "Verify all style compatibility notes have reasons.".to_owned(),
            ],
        },
        review_checklist: DraftReviewChecklist {
            checklist_id: format!("{family_id}_wave37_review"),
            items: vec![
                format!("Scale policy: {}", spec.scale_policy),
                "Review semantic part names against the role inventory.".to_owned(),
                "Use role labels as export part names until authored package review replaces them."
                    .to_owned(),
                "Prune provider choices before any novice-facing kit appears.".to_owned(),
                "Supply authored whole-model contact sheets before promotion.".to_owned(),
            ],
        },
        command_log: Vec::new(),
        rejected_command_attempts: Vec::new(),
        direct_geometry_payload_attempts: Vec::new(),
    }
}

fn compatibility_rules(
    spec: &FoundationBatchSpec,
    style_id: &str,
    provider_pack_ids: &[String],
) -> Vec<DraftCompatibilityRule> {
    let mut rules = provider_pack_ids
        .iter()
        .map(|provider_pack_id| DraftCompatibilityRule {
            style_id: style_id.to_owned(),
            provider_pack_id: provider_pack_id.clone(),
            compatible: true,
            reason: "Wave 37 draft style policy accepts this internal provider option for author review."
                .to_owned(),
        })
        .collect::<Vec<_>>();
    for label in WAVE37_STYLE_PACK_LABELS {
        let compatible = style_is_reasonable_for_family(spec, label);
        for provider_pack_id in provider_pack_ids {
            rules.push(DraftCompatibilityRule {
                style_id: foundation_style_pack_id(label),
                provider_pack_id: provider_pack_id.clone(),
                compatible,
                reason: style_reason(spec, label, compatible),
            });
        }
    }
    rules
}

fn style_is_reasonable_for_family(spec: &FoundationBatchSpec, label: &str) -> bool {
    let family = spec.family_id;
    match label {
        "Clean Sci-Fi" => {
            family.contains("scifi")
                || family.contains("grenade")
                || family.contains("mask")
                || family.contains("hero_accessory")
        }
        "Industrial Heavy" => {
            family.contains("scifi")
                || family.contains("grenade")
                || family.contains("mace")
                || family.contains("chest")
                || family.contains("gauntlet")
                || family.contains("boot")
                || family.contains("helmet")
        }
        "Elegant Elven" => {
            !family.contains("scifi") && !family.contains("grenade") && !family.contains("mace")
        }
        "Brutalist Stone" => {
            family.contains("shield")
                || family.contains("mace")
                || family.contains("chest")
                || family.contains("pauldron")
                || family.contains("mask")
        }
        _ => true,
    }
}

fn style_reason(spec: &FoundationBatchSpec, label: &str, compatible: bool) -> String {
    if compatible {
        format!(
            "{label} can be explored for {} only after a coherent whole-model art pass.",
            spec.display_name
        )
    } else {
        format!(
            "{label} is hidden for {} until a dedicated provider set prevents style salad.",
            spec.display_name
        )
    }
}

fn wave37_specs() -> Vec<FoundationBatchSpec> {
    vec![
        weapon(
            "sword",
            "Sword",
            vec![
                req("blade", "Blade", &["cutting", "silhouette"]),
                opt("fuller", "Fuller", &["detail"]),
                req("guard", "Guard", &["hand_protection"]),
                req("grip", "Grip", &["handle"]),
                req("pommel", "Pommel", &["counterweight"]),
                opt("ornament", "Ornament", &["detail"]),
                opt("scabbard", "Scabbard", &["accessory"]),
            ],
            vec![
                slot("blade_silhouette", "blade", true, &["blade", "foundation"]),
                slot("guard_style", "guard", true, &["guard", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("pommel_style", "pommel", true, &["pommel", "foundation"]),
                slot(
                    "detail_module",
                    "ornament",
                    false,
                    &["detail", "foundation"],
                ),
                slot("wear_module", "blade", false, &["wear", "foundation"]),
                slot(
                    "scabbard_style",
                    "scabbard",
                    false,
                    &["accessory", "foundation"],
                ),
            ],
            vec![
                "Arming Sword Set",
                "Longsword Set",
                "Curved Saber Set",
                "Leaf Blade Set",
                "Heavy Cleaver Set",
            ],
            weapon_controls("Blade/Head Shape", "blade_silhouette"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Strong silhouette and compact role set make this a likely Wave 38 candidate."],
        ),
        weapon(
            "dagger",
            "Dagger",
            vec![
                req("blade", "Blade", &["cutting", "silhouette"]),
                req("guard", "Guard", &["hand_protection"]),
                req("grip", "Grip", &["handle"]),
                req("pommel", "Pommel", &["counterweight"]),
                opt("sheath", "Sheath", &["accessory"]),
                opt("ornament", "Ornament", &["detail"]),
            ],
            vec![
                slot("blade_silhouette", "blade", true, &["blade", "foundation"]),
                slot("guard_style", "guard", true, &["guard", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("pommel_style", "pommel", true, &["pommel", "foundation"]),
                slot(
                    "detail_module",
                    "ornament",
                    false,
                    &["detail", "foundation"],
                ),
                slot(
                    "sheath_style",
                    "sheath",
                    false,
                    &["accessory", "foundation"],
                ),
            ],
            vec![
                "Rondel Dagger Set",
                "Curved Knife Set",
                "Stiletto Set",
                "Ritual Dirk Set",
                "Utility Knife Set",
            ],
            weapon_controls("Blade/Head Shape", "blade_silhouette"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec![
                "Compact shape language should author quickly if detail density stays disciplined.",
            ],
        ),
        weapon(
            "axe",
            "Axe",
            vec![
                req("head", "Head", &["impact", "silhouette"]),
                req("haft", "Haft", &["handle"]),
                req("grip", "Grip", &["handle"]),
                opt("back_hook", "Back Hook", &["secondary_shape"]),
                opt("spike", "Spike", &["secondary_shape"]),
                opt("ornament", "Ornament", &["detail"]),
            ],
            vec![
                slot("head_shape", "head", true, &["head", "foundation"]),
                slot("haft_style", "haft", true, &["handle", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot(
                    "secondary_profile",
                    "back_hook",
                    false,
                    &["profile", "foundation"],
                ),
                slot(
                    "detail_module",
                    "ornament",
                    false,
                    &["detail", "foundation"],
                ),
                slot("wear_module", "head", false, &["wear", "foundation"]),
            ],
            vec![
                "Single Bit Axe Set",
                "Double Bit Axe Set",
                "Hooked Raider Set",
                "Broad Executioner Set",
                "Compact Hatchet Set",
            ],
            weapon_controls("Blade/Head Shape", "head_shape"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Head shape alternatives are visually legible at small preview sizes."],
        ),
        weapon(
            "mace_hammer",
            "Mace / Hammer",
            vec![
                req("head", "Head", &["impact", "silhouette"]),
                req("haft", "Haft", &["handle"]),
                req("grip", "Grip", &["handle"]),
                opt("flanges", "Flanges", &["detail"]),
                opt("spike", "Spike", &["secondary_shape"]),
                opt("pommel", "Pommel", &["counterweight"]),
            ],
            vec![
                slot("head_shape", "head", true, &["head", "foundation"]),
                slot("haft_style", "haft", true, &["handle", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("impact_detail", "flanges", false, &["detail", "foundation"]),
                slot("spike_style", "spike", false, &["profile", "foundation"]),
                slot("wear_module", "head", false, &["wear", "foundation"]),
            ],
            vec![
                "Flanged Mace Set",
                "War Hammer Set",
                "Brutal Maul Set",
                "Spiked Club Set",
                "Ceremonial Scepter Set",
            ],
            weapon_controls("Blade/Head Shape", "head_shape"),
            FoundationBatchReviewCategory::HighRiskOfStyleSalad,
            false,
            vec!["Many cultures share this silhouette; style pruning is needed before curation."],
        ),
        weapon(
            "spear_polearm",
            "Spear / Polearm",
            vec![
                req("head", "Head", &["piercing", "silhouette"]),
                req("shaft", "Shaft", &["handle"]),
                req("grip", "Grip", &["handle"]),
                opt("counterweight", "Counterweight", &["balance"]),
                opt("banner", "Banner", &["accessory"]),
                opt("hook", "Hook", &["secondary_shape"]),
            ],
            vec![
                slot("head_shape", "head", true, &["head", "foundation"]),
                slot("shaft_style", "shaft", true, &["handle", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot(
                    "counterweight_style",
                    "counterweight",
                    false,
                    &["balance", "foundation"],
                ),
                slot(
                    "banner_style",
                    "banner",
                    false,
                    &["accessory", "foundation"],
                ),
                slot("hook_style", "hook", false, &["profile", "foundation"]),
            ],
            vec![
                "Simple Spear Set",
                "Glaive Set",
                "Halberd Set",
                "Banner Lance Set",
                "Hooked Polearm Set",
            ],
            weapon_controls("Blade/Head Shape", "head_shape"),
            FoundationBatchReviewCategory::NeedsSimplification,
            false,
            vec!["Long thin previews need extra camera and silhouette testing."],
        ),
        weapon(
            "bow_crossbow",
            "Bow / Crossbow",
            vec![
                req("body", "Body", &["silhouette"]),
                req("grip", "Grip", &["handle"]),
                req("string", "String", &["tension"]),
                opt("limbs", "Limbs", &["profile"]),
                opt("prod", "Prod", &["crossbow"]),
                opt("bolt_quiver", "Bolt Quiver", &["accessory"]),
                opt("sight", "Sight", &["detail"]),
            ],
            vec![
                slot("body_shape", "body", true, &["body", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("string_style", "string", true, &["string", "foundation"]),
                slot("limb_profile", "limbs", false, &["profile", "foundation"]),
                slot("crossbow_module", "prod", false, &["module", "foundation"]),
                slot(
                    "accessory_module",
                    "bolt_quiver",
                    false,
                    &["accessory", "foundation"],
                ),
            ],
            vec![
                "Recurve Bow Set",
                "Longbow Set",
                "Compact Crossbow Set",
                "Heavy Arbalest Set",
                "Hunter Bow Set",
            ],
            weapon_controls("Main Form", "body_shape"),
            FoundationBatchReviewCategory::NeedsSimplification,
            false,
            vec!["Bow and crossbow variants may need separate authored kits after pruning."],
        ),
        weapon(
            "staff_wand",
            "Staff / Wand",
            vec![
                req("shaft", "Shaft", &["silhouette", "handle"]),
                req("focus", "Focus", &["identity"]),
                req("grip", "Grip", &["handle"]),
                opt("crystal", "Crystal", &["detail"]),
                opt("wrapping", "Wrapping", &["detail"]),
                opt("hanging_charm", "Hanging Charm", &["accessory"]),
            ],
            vec![
                slot("shaft_style", "shaft", true, &["shaft", "foundation"]),
                slot("focus_style", "focus", true, &["focus", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("crystal_style", "crystal", false, &["detail", "foundation"]),
                slot("wrap_style", "wrapping", false, &["detail", "foundation"]),
                slot(
                    "charm_style",
                    "hanging_charm",
                    false,
                    &["accessory", "foundation"],
                ),
            ],
            vec![
                "Plain Wanderer Set",
                "Crystal Focus Set",
                "Druidic Branch Set",
                "Arcane Wand Set",
                "Ceremonial Staff Set",
            ],
            weapon_controls("Main Form", "shaft_style"),
            FoundationBatchReviewCategory::MissingArtIngredients,
            false,
            vec!["Identity depends heavily on authored focus shapes and silhouette reference."],
        ),
        weapon(
            "shield",
            "Shield",
            vec![
                req("body", "Body", &["silhouette", "protection"]),
                req("rim", "Rim", &["edge"]),
                req("grip", "Grip", &["handle"]),
                opt("boss", "Boss", &["center_detail"]),
                opt("emblem", "Emblem", &["identity"]),
                opt("strap", "Strap", &["attachment"]),
            ],
            vec![
                slot("body_shape", "body", true, &["body", "foundation"]),
                slot("rim_style", "rim", true, &["edge", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("boss_style", "boss", false, &["detail", "foundation"]),
                slot("emblem_style", "emblem", false, &["identity", "foundation"]),
                slot("wear_module", "body", false, &["wear", "foundation"]),
            ],
            vec![
                "Round Buckler Set",
                "Kite Shield Set",
                "Tower Shield Set",
                "Heater Shield Set",
                "Energy Shield Set",
            ],
            armor_controls("Coverage", "body_shape"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Simple planar silhouette makes this a strong early curation candidate."],
        ),
        weapon(
            "scifi_rifle_blaster",
            "Sci-Fi Rifle / Blaster",
            vec![
                req("body", "Body", &["silhouette", "receiver"]),
                req("barrel", "Barrel", &["muzzle"]),
                req("grip", "Grip", &["handle"]),
                req("stock", "Stock", &["support"]),
                opt("magazine", "Magazine", &["accessory"]),
                opt("optic", "Optic", &["detail"]),
                opt("rail", "Rail", &["detail"]),
            ],
            vec![
                slot("body_shape", "body", true, &["body", "foundation"]),
                slot("barrel_style", "barrel", true, &["barrel", "foundation"]),
                slot("grip_style", "grip", true, &["grip", "foundation"]),
                slot("stock_style", "stock", true, &["stock", "foundation"]),
                slot(
                    "magazine_style",
                    "magazine",
                    false,
                    &["accessory", "foundation"],
                ),
                slot("optic_style", "optic", false, &["detail", "foundation"]),
                slot("rail_style", "rail", false, &["detail", "foundation"]),
            ],
            vec![
                "Compact Blaster Set",
                "Long Rifle Set",
                "Bullpup Carbine Set",
                "Heavy Cannon Set",
                "Clean Energy Tool Set",
            ],
            weapon_controls("Main Form", "body_shape"),
            FoundationBatchReviewCategory::HighRiskOfStyleSalad,
            false,
            vec!["Needs strict sci-fi language controls to avoid generic greeble buildup."],
        ),
        weapon(
            "grenade_device_prop",
            "Grenade / Device Prop",
            vec![
                req("shell", "Shell", &["silhouette"]),
                req("trigger", "Trigger", &["interaction"]),
                req("cap", "Cap", &["top_detail"]),
                opt("fins", "Fins", &["secondary_shape"]),
                opt("display", "Display", &["detail"]),
                opt("handle", "Handle", &["handle"]),
            ],
            vec![
                slot("shell_shape", "shell", true, &["shell", "foundation"]),
                slot(
                    "trigger_style",
                    "trigger",
                    true,
                    &["interaction", "foundation"],
                ),
                slot("cap_style", "cap", true, &["detail", "foundation"]),
                slot("fin_style", "fins", false, &["profile", "foundation"]),
                slot("display_style", "display", false, &["detail", "foundation"]),
                slot("wear_module", "shell", false, &["wear", "foundation"]),
            ],
            vec![
                "Fragmentation Prop Set",
                "Smoke Device Set",
                "Energy Cell Set",
                "Thrown Sensor Set",
                "Industrial Charge Set",
            ],
            weapon_controls("Main Form", "shell_shape"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Small prop scope and clear silhouettes fit the draft-to-kit pipeline well."],
        ),
        armor(
            "helmet",
            "Helmet",
            vec![
                req("shell", "Shell", &["silhouette", "protection"]),
                req("brow", "Brow", &["face_frame"]),
                opt("cheek_guards", "Cheek Guards", &["side_detail"]),
                opt("visor", "Visor", &["face_cover"]),
                opt("crest", "Crest", &["identity"]),
                opt("horns", "Horns", &["identity"]),
                opt("neck_guard", "Neck Guard", &["coverage"]),
            ],
            vec![
                slot("shell_form", "shell", true, &["shell", "foundation"]),
                slot("brow_style", "brow", true, &["face_frame", "foundation"]),
                slot(
                    "cheek_guard_style",
                    "cheek_guards",
                    false,
                    &["coverage", "foundation"],
                ),
                slot("visor_style", "visor", false, &["face_cover", "foundation"]),
                slot(
                    "crest_horn_emblem",
                    "crest",
                    false,
                    &["identity", "foundation"],
                ),
                slot(
                    "neck_guard_style",
                    "neck_guard",
                    false,
                    &["coverage", "foundation"],
                ),
                slot("wear_module", "shell", false, &["wear", "foundation"]),
            ],
            vec![
                "Open Barbute Set",
                "Great Helm Set",
                "Horned Raider Set",
                "Sci-Fi Pilot Set",
                "Hero Crest Set",
            ],
            armor_controls("Coverage", "shell_form"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Strong face-framing silhouette makes this a good prepared-hero dependency."],
        ),
        armor(
            "pauldron",
            "Pauldron",
            vec![
                req(
                    "shoulder_shell",
                    "Shoulder Shell",
                    &["silhouette", "protection"],
                ),
                req("rim", "Rim", &["edge"]),
                req("strap", "Strap", &["attachment"]),
                opt("layered_plate", "Layered Plate", &["coverage"]),
                opt("emblem", "Emblem", &["identity"]),
                opt("spike", "Spike", &["secondary_shape"]),
            ],
            vec![
                slot(
                    "shell_form",
                    "shoulder_shell",
                    true,
                    &["shell", "foundation"],
                ),
                slot("rim_style", "rim", true, &["edge", "foundation"]),
                slot("strap_style", "strap", true, &["attachment", "foundation"]),
                slot(
                    "layer_style",
                    "layered_plate",
                    false,
                    &["coverage", "foundation"],
                ),
                slot("emblem_style", "emblem", false, &["identity", "foundation"]),
                slot("spike_style", "spike", false, &["profile", "foundation"]),
            ],
            vec![
                "Rounded Guard Set",
                "Layered Knight Set",
                "Spiked Raider Set",
                "Sci-Fi Shoulder Set",
                "Hero Mantle Set",
            ],
            armor_controls("Coverage", "shell_form"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["A single wearable form can show meaningful style variation quickly."],
        ),
        armor(
            "chest_armor",
            "Chest Armor",
            vec![
                req("torso_shell", "Torso Shell", &["silhouette", "protection"]),
                req("collar", "Collar", &["upper_edge"]),
                req("waist", "Waist", &["lower_edge"]),
                req("straps", "Straps", &["attachment"]),
                opt("emblem", "Emblem", &["identity"]),
                opt("skirt", "Skirt", &["coverage"]),
                opt("pauldrons", "Pauldrons", &["shoulder"]),
            ],
            vec![
                slot("torso_form", "torso_shell", true, &["shell", "foundation"]),
                slot("collar_style", "collar", true, &["edge", "foundation"]),
                slot("waist_style", "waist", true, &["edge", "foundation"]),
                slot("strap_style", "straps", true, &["attachment", "foundation"]),
                slot("emblem_style", "emblem", false, &["identity", "foundation"]),
                slot("skirt_style", "skirt", false, &["coverage", "foundation"]),
                slot(
                    "shoulder_module",
                    "pauldrons",
                    false,
                    &["module", "foundation"],
                ),
            ],
            vec![
                "Breastplate Set",
                "Segmented Cuirass Set",
                "Sci-Fi Harness Set",
                "Heroic Plate Set",
                "Leather Jerkin Set",
            ],
            armor_controls("Coverage", "torso_form"),
            FoundationBatchReviewCategory::OverAbstracted,
            false,
            vec!["Torso gear touches too many hero-body fit questions for automatic curation."],
        ),
        armor(
            "gauntlet",
            "Gauntlet",
            vec![
                req("hand_plate", "Hand Plate", &["silhouette", "protection"]),
                req("cuff", "Cuff", &["wrist"]),
                req("knuckle_guard", "Knuckle Guard", &["detail"]),
                opt("fingers", "Fingers", &["articulation"]),
                opt("wrist_strap", "Wrist Strap", &["attachment"]),
                opt("claw", "Claw", &["secondary_shape"]),
            ],
            vec![
                slot("hand_form", "hand_plate", true, &["shell", "foundation"]),
                slot("cuff_style", "cuff", true, &["edge", "foundation"]),
                slot(
                    "knuckle_style",
                    "knuckle_guard",
                    true,
                    &["detail", "foundation"],
                ),
                slot(
                    "finger_style",
                    "fingers",
                    false,
                    &["coverage", "foundation"],
                ),
                slot(
                    "strap_style",
                    "wrist_strap",
                    false,
                    &["attachment", "foundation"],
                ),
                slot("claw_style", "claw", false, &["profile", "foundation"]),
            ],
            vec![
                "Plate Glove Set",
                "Leather Bracer Set",
                "Clawed Gauntlet Set",
                "Sci-Fi Glove Set",
                "Hero Wrist Guard Set",
            ],
            armor_controls("Coverage", "hand_form"),
            FoundationBatchReviewCategory::MissingArtIngredients,
            false,
            vec!["Finger and hand fit need authored reference before becoming a kit."],
        ),
        armor(
            "boot",
            "Boot",
            vec![
                req("foot_shell", "Foot Shell", &["silhouette", "protection"]),
                req("ankle_cuff", "Ankle Cuff", &["edge"]),
                req("sole", "Sole", &["base"]),
                opt("shin_guard", "Shin Guard", &["coverage"]),
                opt("buckle", "Buckle", &["detail"]),
                opt("toe_cap", "Toe Cap", &["front_detail"]),
            ],
            vec![
                slot("foot_form", "foot_shell", true, &["shell", "foundation"]),
                slot("cuff_style", "ankle_cuff", true, &["edge", "foundation"]),
                slot("sole_style", "sole", true, &["base", "foundation"]),
                slot(
                    "shin_guard_style",
                    "shin_guard",
                    false,
                    &["coverage", "foundation"],
                ),
                slot("buckle_style", "buckle", false, &["detail", "foundation"]),
                slot("toe_cap_style", "toe_cap", false, &["front", "foundation"]),
            ],
            vec![
                "Armored Sabaton Set",
                "Traveler Boot Set",
                "Sci-Fi Greave Boot Set",
                "Hero High Boot Set",
                "Heavy Work Boot Set",
            ],
            armor_controls("Coverage", "foot_form"),
            FoundationBatchReviewCategory::NeedsSimplification,
            false,
            vec!["Footwear needs paired left/right and hero-scale fit rules before promotion."],
        ),
        armor(
            "belt",
            "Belt",
            vec![
                req("band", "Band", &["silhouette", "waist"]),
                req("buckle", "Buckle", &["front_detail"]),
                opt("pouches", "Pouches", &["accessory"]),
                opt("hanging_straps", "Hanging Straps", &["accessory"]),
                opt("emblem", "Emblem", &["identity"]),
            ],
            vec![
                slot("band_style", "band", true, &["band", "foundation"]),
                slot("buckle_style", "buckle", true, &["detail", "foundation"]),
                slot(
                    "pouch_style",
                    "pouches",
                    false,
                    &["accessory", "foundation"],
                ),
                slot(
                    "strap_style",
                    "hanging_straps",
                    false,
                    &["accessory", "foundation"],
                ),
                slot("emblem_style", "emblem", false, &["identity", "foundation"]),
                slot("wear_module", "band", false, &["wear", "foundation"]),
            ],
            vec![
                "Plain Utility Set",
                "Hero Buckle Set",
                "Pouch Belt Set",
                "Sci-Fi Harness Set",
                "Ceremonial Sash Set",
            ],
            armor_controls("Coverage", "band_style"),
            FoundationBatchReviewCategory::Promising,
            true,
            vec!["Low-risk accessory scope and clear option tiles make this useful for Wave 38."],
        ),
        armor(
            "cape_back_accessory",
            "Cape / Back Accessory",
            vec![
                req("cape_panel", "Cape Panel", &["silhouette", "cloth"]),
                req("clasp", "Clasp", &["attachment"]),
                opt("hood", "Hood", &["head_cover"]),
                opt("banner_tail", "Banner Tail", &["silhouette"]),
                opt("backpack_frame", "Backpack Frame", &["support"]),
                opt("ornament", "Ornament", &["detail"]),
            ],
            vec![
                slot("cape_shape", "cape_panel", true, &["cloth", "foundation"]),
                slot("clasp_style", "clasp", true, &["attachment", "foundation"]),
                slot("hood_style", "hood", false, &["head_cover", "foundation"]),
                slot(
                    "tail_style",
                    "banner_tail",
                    false,
                    &["profile", "foundation"],
                ),
                slot(
                    "back_frame_style",
                    "backpack_frame",
                    false,
                    &["support", "foundation"],
                ),
                slot(
                    "detail_module",
                    "ornament",
                    false,
                    &["detail", "foundation"],
                ),
            ],
            vec![
                "Short Cape Set",
                "Hero Cloak Set",
                "Banner Back Set",
                "Sci-Fi Pack Set",
                "Travel Mantle Set",
            ],
            armor_controls("Coverage", "cape_shape"),
            FoundationBatchReviewCategory::OverAbstracted,
            false,
            vec!["Cape, cloak, banner, and pack should likely split after review."],
        ),
        armor(
            "mask",
            "Mask",
            vec![
                req("face_shell", "Face Shell", &["silhouette", "face"]),
                req("eye_openings", "Eye Openings", &["face_frame"]),
                opt("respirator", "Respirator", &["front_detail"]),
                opt("brow", "Brow", &["expression"]),
                opt("cheek_plates", "Cheek Plates", &["coverage"]),
                opt("ornament", "Ornament", &["identity"]),
            ],
            vec![
                slot("face_form", "face_shell", true, &["shell", "foundation"]),
                slot(
                    "eye_style",
                    "eye_openings",
                    true,
                    &["face_frame", "foundation"],
                ),
                slot(
                    "respirator_style",
                    "respirator",
                    false,
                    &["front", "foundation"],
                ),
                slot("brow_style", "brow", false, &["expression", "foundation"]),
                slot(
                    "cheek_style",
                    "cheek_plates",
                    false,
                    &["coverage", "foundation"],
                ),
                slot(
                    "ornament_style",
                    "ornament",
                    false,
                    &["identity", "foundation"],
                ),
            ],
            vec![
                "Hero Half Mask Set",
                "Full Face Guard Set",
                "Respirator Mask Set",
                "Elven Visor Set",
                "Dark Ritual Mask Set",
            ],
            armor_controls("Coverage", "face_form"),
            FoundationBatchReviewCategory::HighRiskOfStyleSalad,
            false,
            vec!["Facial identity is style-sensitive and needs adversarial visual review."],
        ),
        armor(
            "hero_accessory_set",
            "Hero Accessory Set",
            vec![
                req("charm", "Charm", &["identity"]),
                req("sash", "Sash", &["silhouette"]),
                req("emblem", "Emblem", &["identity"]),
                opt("trophy", "Trophy", &["accessory"]),
                opt("shoulder_token", "Shoulder Token", &["accessory"]),
                opt("back_token", "Back Token", &["accessory"]),
                opt("waist_token", "Waist Token", &["accessory"]),
            ],
            vec![
                slot("charm_style", "charm", true, &["identity", "foundation"]),
                slot("sash_style", "sash", true, &["cloth", "foundation"]),
                slot("emblem_style", "emblem", true, &["identity", "foundation"]),
                slot(
                    "trophy_style",
                    "trophy",
                    false,
                    &["accessory", "foundation"],
                ),
                slot(
                    "shoulder_token_style",
                    "shoulder_token",
                    false,
                    &["accessory", "foundation"],
                ),
                slot(
                    "back_token_style",
                    "back_token",
                    false,
                    &["accessory", "foundation"],
                ),
                slot(
                    "waist_token_style",
                    "waist_token",
                    false,
                    &["accessory", "foundation"],
                ),
            ],
            vec![
                "Champion Token Set",
                "Faction Emblem Set",
                "Monster Trophy Set",
                "Arcane Trinket Set",
                "Sci-Fi Badge Set",
            ],
            armor_controls("Coverage", "charm_style"),
            FoundationBatchReviewCategory::OverAbstracted,
            false,
            vec!["Useful for hero theming, but too broad for uncurated kit publication."],
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn weapon(
    family_id: &'static str,
    display_name: &'static str,
    roles: Vec<RoleSpec>,
    slots: Vec<SlotSpec>,
    provider_options: Vec<&'static str>,
    controls: Vec<ControlSpec>,
    review_category: FoundationBatchReviewCategory,
    showcase_candidate: bool,
    notes: Vec<&'static str>,
) -> FoundationBatchSpec {
    FoundationBatchSpec {
        category: "weapons",
        family_id,
        display_name,
        roles,
        slots,
        provider_options,
        controls,
        strategy_names: vec!["Light", "Heavy", "Ornate", "Minimal", "Heroic", "Worn"],
        review_category,
        showcase_candidate,
        scale_policy: "Weapon proportions stay readable in hand and in a whole-model preview before promotion.",
        notes,
    }
}

#[allow(clippy::too_many_arguments)]
fn armor(
    family_id: &'static str,
    display_name: &'static str,
    roles: Vec<RoleSpec>,
    slots: Vec<SlotSpec>,
    provider_options: Vec<&'static str>,
    controls: Vec<ControlSpec>,
    review_category: FoundationBatchReviewCategory,
    showcase_candidate: bool,
    notes: Vec<&'static str>,
) -> FoundationBatchSpec {
    FoundationBatchSpec {
        category: "armor_gear",
        family_id,
        display_name,
        roles,
        slots,
        provider_options,
        controls,
        strategy_names: vec![
            "Compact",
            "Reinforced",
            "Elegant",
            "Brutal",
            "Heroic",
            "Relic",
        ],
        review_category,
        showcase_candidate,
        scale_policy: "Wearable proportions must fit a prepared hero template before promotion.",
        notes,
    }
}

fn weapon_controls(main_label: &'static str, main_slot: &'static str) -> Vec<ControlSpec> {
    vec![
        control(
            "weapon_class",
            "Weapon Class",
            "Choose the broad weapon family direction.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The whole-model silhouette switches to the chosen weapon family.",
        ),
        control(
            "silhouette",
            "Silhouette",
            "Choose a readable whole-model outline.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The preview reads as a different outline at small size.",
        ),
        control(
            "main_shape",
            main_label,
            "Adjust the primary striking or focal shape.",
            &[main_slot],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The primary shape changes visibly without relying on tiny details.",
        ),
        control(
            "handle_length",
            "Handle Length",
            "Adjust the grip and reach proportions.",
            &[],
            ControlProfileControlKind::Continuous,
            ControlProfileTopologyBehavior::TopologyPreserving,
            "The handle length changes whole-model balance.",
        ),
        control(
            "grip_style",
            "Guard / Grip Style",
            "Choose the hand area language.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The hand area changes in a clearly visible way.",
        ),
        control(
            "ornamentation",
            "Ornamentation",
            "Set the amount of identity detail.",
            &[],
            ControlProfileControlKind::Continuous,
            ControlProfileTopologyBehavior::TopologyChanging,
            "Detail presence changes without obscuring the main silhouette.",
        ),
        control(
            "detail_density",
            "Detail Density",
            "Tune small forms after the large shape is chosen.",
            &[],
            ControlProfileControlKind::Continuous,
            ControlProfileTopologyBehavior::TopologyPreserving,
            "Fine detail density changes while the model remains readable.",
        ),
    ]
}

fn armor_controls(main_label: &'static str, main_slot: &'static str) -> Vec<ControlSpec> {
    vec![
        control(
            "armor_mass",
            "Armor Mass",
            "Choose the perceived weight and bulk.",
            &[],
            ControlProfileControlKind::Continuous,
            ControlProfileTopologyBehavior::TopologyPreserving,
            "The piece reads as lighter or heavier at whole-model scale.",
        ),
        control(
            "silhouette",
            "Silhouette",
            "Choose the broad wearable outline.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The outline changes clearly in the preview.",
        ),
        control(
            "coverage",
            main_label,
            "Choose how much of the body or prop area is covered.",
            &[main_slot],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "Coverage changes with an obvious whole-model result.",
        ),
        control(
            "edge_language",
            "Edge Language",
            "Choose soft, plated, sharp, or blocky edges.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "Outer edges change before small detail is considered.",
        ),
        control(
            "crest_emblem",
            "Crest / Horn / Emblem",
            "Choose a visible identity marker.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "Identity markers appear, disappear, or change shape.",
        ),
        control(
            "symmetry",
            "Symmetry",
            "Choose clean symmetry or reviewed asymmetry.",
            &[],
            ControlProfileControlKind::Choice,
            ControlProfileTopologyBehavior::TopologyChanging,
            "The piece becomes balanced or intentionally uneven.",
        ),
        control(
            "detail_density",
            "Detail Density",
            "Tune the amount of small detail.",
            &[],
            ControlProfileControlKind::Continuous,
            ControlProfileTopologyBehavior::TopologyPreserving,
            "Small details increase or reduce without changing the role set.",
        ),
    ]
}

fn req(role_id: &'static str, label: &'static str, tags: &[&'static str]) -> RoleSpec {
    role(role_id, label, true, tags)
}

fn opt(role_id: &'static str, label: &'static str, tags: &[&'static str]) -> RoleSpec {
    role(role_id, label, false, tags)
}

fn role(
    role_id: &'static str,
    label: &'static str,
    required: bool,
    tags: &[&'static str],
) -> RoleSpec {
    RoleSpec {
        role_id,
        label,
        required,
        tags: tags.to_vec(),
    }
}

fn slot(
    slot_id: &'static str,
    role_id: &'static str,
    required: bool,
    tags: &[&'static str],
) -> SlotSpec {
    SlotSpec {
        slot_id,
        role_id,
        required,
        tags: tags.to_vec(),
    }
}

fn control(
    control_id: &'static str,
    label: &'static str,
    description: &'static str,
    owned_slots: &[&'static str],
    kind: ControlProfileControlKind,
    topology_behavior: ControlProfileTopologyBehavior,
    visible_effect_expectation: &'static str,
) -> ControlSpec {
    ControlSpec {
        control_id,
        label,
        description,
        kind,
        owned_slots: owned_slots.to_vec(),
        topology_behavior,
        visible_effect_expectation,
    }
}

fn normalize_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn wave37_batch_contains_expected_families_once() {
        let drafts = weapon_armor_foundation_draft_batch();
        assert_eq!(drafts.len(), WAVE37_WEAPON_ARMOR_FAMILY_IDS.len());
        let ids = drafts
            .iter()
            .map(|draft| draft.family_blueprint.family_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), WAVE37_WEAPON_ARMOR_FAMILY_IDS.len());
        for expected in WAVE37_WEAPON_ARMOR_FAMILY_IDS {
            assert!(ids.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn wave37_batch_drafts_are_internal_and_validate() {
        for draft in weapon_armor_foundation_draft_batch() {
            assert_eq!(
                draft.source_kind,
                FoundationDraftSourceKind::GeneratedFixture
            );
            assert_eq!(draft.quality_target, FoundationQualityTarget::Draft);
            assert_eq!(
                draft.catalog_visibility,
                FoundationCatalogVisibility::InternalOnly
            );
            assert!(draft.human_review_required);
            assert!(!draft.publish_allowed);
            let report = validate_foundation_draft(&draft);
            assert!(
                report.is_valid(),
                "{} validation issues: {:#?}",
                draft.draft_id,
                report.issues
            );
        }
    }

    #[test]
    fn wave37_batch_has_controls_strategies_styles_and_quality_gates() {
        for draft in weapon_armor_foundation_draft_batch() {
            let primary_count = draft
                .control_profile
                .controls
                .iter()
                .filter(|control| control.visible && control.primary)
                .count();
            assert!(
                primary_count <= DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS as usize,
                "{} has too many primary controls",
                draft.draft_id
            );
            assert_eq!(draft.candidate_strategy_pack.strategies.len(), 6);
            assert!(draft.provider_taxonomy.provider_packs.len() >= 5);
            assert!(
                draft.quality_gate_profile.is_some(),
                "{} missing quality gate",
                draft.draft_id
            );
            let quality = draft.quality_gate_profile.as_ref().expect("quality gate");
            for expected in [
                "No missing required roles.",
                "No attachment failures between required parts.",
                "No self-intersections above the accepted threshold.",
                "Triangle budget target is recorded before authoring.",
                "Silhouette remains visible at 128px.",
                "All primary controls visibly matter.",
                "Six candidate survivors compile and render.",
                "Whole-model previews render without placeholders.",
                "Export and reopen succeeds before promotion.",
                "Contact sheet required for Usable or Showcase.",
                "Human review required for Showcase.",
            ] {
                assert!(
                    quality
                        .manual_review_gates
                        .iter()
                        .any(|gate| gate == expected),
                    "{} missing quality gate {expected}",
                    draft.draft_id
                );
            }
            let style_ids = draft
                .compatibility_matrix
                .rules
                .iter()
                .map(|rule| rule.style_id.as_str())
                .collect::<BTreeSet<_>>();
            let provider_pack_ids = draft
                .provider_taxonomy
                .provider_packs
                .iter()
                .map(|pack| pack.pack_id.as_str())
                .collect::<BTreeSet<_>>();
            for label in WAVE37_STYLE_PACK_LABELS {
                let id = foundation_style_pack_id(label);
                assert!(
                    style_ids.contains(id.as_str()),
                    "{} missing style note for {label}",
                    draft.draft_id
                );
                for provider_pack_id in &provider_pack_ids {
                    assert!(
                        draft.compatibility_matrix.rules.iter().any(|rule| {
                            rule.style_id == id && rule.provider_pack_id == *provider_pack_id
                        }),
                        "{} missing style/provider note for {label} and {provider_pack_id}",
                        draft.draft_id
                    );
                }
            }
            assert!(
                draft
                    .compatibility_matrix
                    .rules
                    .iter()
                    .any(|rule| !rule.compatible),
                "{} should hide at least one risky style combination",
                draft.draft_id
            );
            assert!(
                draft
                    .compatibility_matrix
                    .rules
                    .iter()
                    .all(|rule| !rule.reason.trim().is_empty()),
                "{} has a compatibility rule without a reason",
                draft.draft_id
            );
        }
    }

    #[test]
    fn wave37_adversarial_reports_are_deterministic() {
        for draft in weapon_armor_foundation_draft_batch() {
            let first = foundation_adversarial_report(&draft);
            let second = foundation_adversarial_report(&draft);
            assert_eq!(first, second);
            assert!(!first.missing_geometry_art_ingredients.is_empty());
            assert!(!first.human_review_required.is_empty());
        }
    }

    #[test]
    fn wave37_summary_covers_report_categories_and_showcase_candidates() {
        let summary = weapon_armor_foundation_batch_summary();
        assert_eq!(summary.len(), WAVE37_WEAPON_ARMOR_FAMILY_IDS.len());
        assert!(summary.iter().all(|row| row.validation_issue_count == 0));
        assert!(
            summary
                .iter()
                .any(|row| row.review_category == FoundationBatchReviewCategory::Promising)
        );
        assert!(
            summary.iter().any(
                |row| row.review_category == FoundationBatchReviewCategory::NeedsSimplification
            )
        );
        assert!(
            summary
                .iter()
                .any(|row| row.review_category == FoundationBatchReviewCategory::OverAbstracted)
        );
        assert!(
            summary
                .iter()
                .any(|row| row.review_category
                    == FoundationBatchReviewCategory::MissingArtIngredients)
        );
        assert!(
            summary
                .iter()
                .any(|row| row.review_category
                    == FoundationBatchReviewCategory::HighRiskOfStyleSalad)
        );
        assert!(
            summary
                .iter()
                .any(|row| row.good_candidate_for_showcase_gear_pack)
        );
    }
}
