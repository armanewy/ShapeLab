//! Versioned Foundry kit, provider pack, style pack, and review contracts.
//!
//! Kits are a curated content packaging layer. They summarize exact Foundry
//! catalog content for novice product flows, but they do not bypass the
//! compiler, family/style compatibility checks, or HQ review gates.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Current schema version for Foundry kit packages.
pub const FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION: u32 = 1;
/// Current schema version for Foundry kit manifests.
pub const FOUNDRY_KIT_SCHEMA_VERSION: u32 = 1;
/// Current schema version for family blueprints.
pub const FAMILY_BLUEPRINT_SCHEMA_VERSION: u32 = 1;
/// Current schema version for provider packs.
pub const PROVIDER_PACK_SCHEMA_VERSION: u32 = 1;
/// Current schema version for style packs.
pub const STYLE_PACK_SCHEMA_VERSION: u32 = 1;
/// Current schema version for novice control profiles.
pub const CONTROL_PROFILE_SCHEMA_VERSION: u32 = 1;
/// Current schema version for candidate strategy packs.
pub const CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION: u32 = 1;
/// Current schema version for quality gate profiles.
pub const QUALITY_GATE_PROFILE_SCHEMA_VERSION: u32 = 1;
/// Current schema version for kit compatibility matrices.
pub const KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION: u32 = 1;
/// Current schema version for kit review manifests.
pub const KIT_REVIEW_MANIFEST_SCHEMA_VERSION: u32 = 1;
/// Current schema version for kit catalog manifests.
pub const KIT_CATALOG_MANIFEST_SCHEMA_VERSION: u32 = 1;
/// Default maximum number of primary controls in the novice surface.
pub const DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS: u32 = 7;

/// Product-quality tier assigned to curated Foundry content.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundryKitQualityTier {
    /// Internal work-in-progress content.
    Draft,
    /// Preview content that compiles and renders, but is not default novice content.
    Prototype,
    /// Content eligible for novice product exposure after review evidence passes.
    Usable,
    /// Strong product-quality claim requiring human and adversarial review.
    Showcase,
}

impl FoundryKitQualityTier {
    /// Human-facing tier label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Prototype => "Prototype",
            Self::Usable => "Usable",
            Self::Showcase => "Showcase",
        }
    }
}

/// Default catalog visibility policy for one kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogVisibilityPolicy {
    /// Whether this kit may appear in the default novice catalog.
    pub default_novice_catalog: bool,
    /// Whether this kit may appear when developer/preview content is enabled.
    pub developer_preview_catalog: bool,
    /// Whether a Showcase badge may be displayed.
    pub showcase_badge_allowed: bool,
    /// Product-safe reason when hidden from the default novice catalog.
    pub hidden_reason: Option<String>,
}

impl CatalogVisibilityPolicy {
    /// Hidden policy for content that is not default novice catalog ready.
    #[must_use]
    pub fn hidden(reason: impl Into<String>) -> Self {
        Self {
            default_novice_catalog: false,
            developer_preview_catalog: true,
            showcase_badge_allowed: false,
            hidden_reason: Some(reason.into()),
        }
    }

    /// Visible policy for reviewed Usable content.
    #[must_use]
    pub fn novice_visible() -> Self {
        Self {
            default_novice_catalog: true,
            developer_preview_catalog: true,
            showcase_badge_allowed: false,
            hidden_reason: None,
        }
    }
}

/// Preview policy carried by a Foundry kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewCameraPolicy {
    /// Stable policy identifier.
    pub policy_id: String,
    /// Required review views such as `front` or `three-quarter`.
    pub required_views: Vec<String>,
    /// Whether a clay preview is required.
    pub clay_preview_required: bool,
    /// Whether a contact sheet is required for review.
    pub contact_sheet_required: bool,
}

/// Top-level curated Foundry kit manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryKit {
    /// Kit schema version.
    pub schema_version: u32,
    /// Stable kit ID.
    pub kit_id: String,
    /// Human-facing display name.
    pub display_name: String,
    /// Referenced family blueprint ID.
    pub family_blueprint_id: String,
    /// Referenced provider pack ID.
    pub provider_pack_id: String,
    /// Referenced style pack ID.
    pub style_pack_id: String,
    /// Referenced control profile ID.
    pub control_profile_id: String,
    /// Referenced candidate strategy pack ID.
    pub candidate_strategy_pack_id: String,
    /// Referenced quality gate profile ID.
    pub quality_gate_profile_id: String,
    /// Referenced compatibility matrix ID.
    pub compatibility_matrix_id: String,
    /// Referenced review manifest ID.
    pub review_manifest_id: String,
    /// Referenced catalog manifest ID.
    pub catalog_manifest_id: String,
    /// Preview and contact-sheet camera policy.
    pub preview_camera_policy: PreviewCameraPolicy,
    /// Achieved or requested product tier for catalog handling.
    pub quality_tier: FoundryKitQualityTier,
    /// Default novice/developer catalog visibility.
    pub catalog_visibility_policy: CatalogVisibilityPolicy,
    /// Optional built-in fixture slug used by local tooling.
    #[serde(default)]
    pub source_profile_slug: Option<String>,
    /// Product-safe category chips for Choose/home surfaces.
    #[serde(default)]
    pub category_chips: Vec<String>,
}

/// One role summarized by a family blueprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyBlueprintRole {
    /// Stable role ID.
    pub role_id: String,
    /// Human-facing role label.
    pub label: String,
    /// Whether the role is required for valid output.
    pub required: bool,
    /// Product-safe role tags.
    pub tags: Vec<String>,
}

/// Expected provider slot for a family role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSlotExpectation {
    /// Stable slot ID.
    pub slot_id: String,
    /// Role supplied through this slot.
    pub role_id: String,
    /// Whether the slot is required for the kit.
    pub required: bool,
    /// Attachment tags expected by authored fragments.
    pub attachment_tags: Vec<String>,
}

/// Attachment expectation summarized for review and packaging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentExpectation {
    /// Stable expectation ID.
    pub expectation_id: String,
    /// Source role.
    pub from_role: String,
    /// Destination role.
    pub to_role: String,
    /// Product-safe compatibility tags.
    pub compatibility_tags: Vec<String>,
    /// Whether the attachment is required for validity.
    pub required: bool,
}

/// High-level scale policy for one asset family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighLevelScalePolicy {
    /// Plain-language scale label.
    pub label: String,
    /// Optional normalized or approximate unit range.
    pub allowed_range: Option<String>,
}

/// Export part naming policy for authored families.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPartNamingPolicy {
    /// Naming strategy label.
    pub strategy: String,
    /// Required exported part prefixes or labels.
    pub required_part_names: Vec<String>,
}

/// Family blueprint consumed by curated Foundry kits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyBlueprint {
    /// Family blueprint schema version.
    pub schema_version: u32,
    /// Stable family ID.
    pub family_id: String,
    /// Human-facing family label.
    pub display_name: String,
    /// Semantic role inventory.
    pub semantic_roles: Vec<FamilyBlueprintRole>,
    /// Required role IDs.
    pub required_roles: Vec<String>,
    /// Optional role IDs.
    pub optional_roles: Vec<String>,
    /// Provider slots expected by this family.
    pub provider_slots: Vec<ProviderSlotExpectation>,
    /// Attachment expectations.
    pub attachment_expectations: Vec<AttachmentExpectation>,
    /// High-level scale policy.
    pub scale_policy: HighLevelScalePolicy,
    /// Export part naming policy.
    pub export_part_naming_policy: ExportPartNamingPolicy,
}

/// One provider option summarized by a provider pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPackOption {
    /// Stable option ID.
    pub option_id: String,
    /// Slot supplied by this option.
    pub slot_id: String,
    /// Human-facing option label.
    pub label: String,
    /// Roles covered by this option.
    pub semantic_roles: Vec<String>,
    /// Compatibility tags.
    pub compatibility_tags: Vec<String>,
    /// Detail-density tags.
    pub detail_density_tags: Vec<String>,
    /// Estimated triangle budget contribution.
    pub triangle_budget_estimate: Option<u32>,
}

/// Expert-authored provider pack summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPack {
    /// Provider pack schema version.
    pub schema_version: u32,
    /// Stable pack ID.
    pub pack_id: String,
    /// Primary family ID when the pack is family-scoped.
    pub family_id: Option<String>,
    /// Compatible family IDs.
    pub compatible_family_ids: Vec<String>,
    /// Provider slots supplied by this pack.
    pub provider_slots_supplied: Vec<String>,
    /// Provider options.
    pub provider_options: Vec<ProviderPackOption>,
    /// Semantic role coverage.
    pub semantic_role_coverage: Vec<String>,
    /// Socket and attachment compatibility tags.
    pub socket_attachment_tags: Vec<String>,
    /// Detail density tags.
    pub detail_density_tags: Vec<String>,
    /// Triangle budget estimates keyed by slot or option ID.
    pub triangle_budget_estimates: BTreeMap<String, u32>,
    /// General compatibility tags.
    pub compatibility_tags: Vec<String>,
}

impl ProviderPack {
    fn supports_family(&self, family_id: &str) -> bool {
        self.family_id.as_deref() == Some(family_id)
            || self
                .compatible_family_ids
                .iter()
                .any(|candidate| candidate == family_id)
    }
}

/// Metadata-only future material vocabulary for style packs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FutureMaterialVocabulary {
    /// Plain-language vocabulary label.
    pub label: String,
    /// Metadata-only tags reserved for future material systems.
    pub tags: Vec<String>,
}

/// Style pack compatibility and visual-language summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StylePack {
    /// Style pack schema version.
    pub schema_version: u32,
    /// Stable style ID.
    pub style_id: String,
    /// Human-facing style label.
    pub display_name: String,
    /// Compatible family IDs.
    pub compatible_family_ids: Vec<String>,
    /// Plain-language bevel language.
    pub bevel_language: String,
    /// Plain-language proportion language.
    pub proportion_language: String,
    /// Detail-density policy.
    pub detail_density_policy: String,
    /// Silhouette exaggeration policy.
    pub silhouette_exaggeration_policy: String,
    /// Symmetry/asymmetry policy.
    pub symmetry_asymmetry_policy: String,
    /// Allowed provider tags.
    pub allowed_provider_tags: Vec<String>,
    /// Forbidden provider tags.
    pub forbidden_provider_tags: Vec<String>,
    /// Compatible provider pack IDs.
    pub compatible_provider_packs: Vec<String>,
    /// Incompatible provider pack IDs.
    pub incompatible_provider_packs: Vec<String>,
    /// Optional metadata-only future material vocabulary.
    pub future_material_vocabulary: Option<FutureMaterialVocabulary>,
}

impl StylePack {
    fn supports_family(&self, family_id: &str) -> bool {
        self.compatible_family_ids
            .iter()
            .any(|candidate| candidate == family_id)
    }

    fn allows_provider_pack(&self, provider_pack_id: &str) -> bool {
        !self
            .incompatible_provider_packs
            .iter()
            .any(|candidate| candidate == provider_pack_id)
            && (self.compatible_provider_packs.is_empty()
                || self
                    .compatible_provider_packs
                    .iter()
                    .any(|candidate| candidate == provider_pack_id))
    }
}

/// Control kind exposed by the novice control profile.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlProfileControlKind {
    /// Continuous slider or dial.
    Continuous,
    /// Integer stepper.
    Integer,
    /// Binary toggle.
    Toggle,
    /// Whole-model option tile gallery.
    Choice,
}

/// Topology behavior for a visible control.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlProfileTopologyBehavior {
    /// Control preserves topology.
    TopologyPreserving,
    /// Control can change topology.
    TopologyChanging,
    /// Control is consumed by export/runtime adapters only.
    RuntimeOnly,
}

/// Visibility policy for one option or gallery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlOptionVisibility {
    /// Whether unavailable options are hidden from novices.
    pub hide_invalid_from_novices: bool,
    /// Whether product-safe reasons are shown when unavailable.
    pub show_plain_language_reasons: bool,
}

/// One novice control row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlProfileControl {
    /// Stable control ID.
    pub control_id: String,
    /// Product-facing label.
    pub label: String,
    /// Product-facing description.
    pub description: String,
    /// UI control kind.
    pub kind: ControlProfileControlKind,
    /// Family slots owned by this visible control.
    pub owned_family_slots: Vec<String>,
    /// Provider slots owned by this visible control.
    #[serde(default)]
    pub owned_provider_slots: Vec<String>,
    /// Visible effect expectation used by review.
    pub visible_effect_expectation: String,
    /// Topology behavior.
    pub topology_behavior: ControlProfileTopologyBehavior,
    /// Option visibility policy.
    pub option_visibility: ControlOptionVisibility,
    /// Whether the control starts locked.
    pub default_locked: bool,
    /// Whether the control is one of the primary novice controls.
    pub primary: bool,
    /// Whether the control is product-visible.
    pub visible: bool,
}

/// Default novice control profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlProfile {
    /// Control profile schema version.
    pub schema_version: u32,
    /// Stable control profile ID.
    pub profile_id: String,
    /// Family ID owned by this profile.
    pub family_id: String,
    /// Optional style ID.
    pub style_id: Option<String>,
    /// Maximum primary controls for the default novice surface.
    pub maximum_primary_controls: u32,
    /// Visible and hidden controls.
    pub controls: Vec<ControlProfileControl>,
}

/// One candidate strategy summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitCandidateStrategy {
    /// Stable strategy ID.
    pub strategy_id: String,
    /// Product-facing strategy name.
    pub name: String,
    /// Controls the strategy may change.
    pub allowed_controls: Vec<String>,
    /// Explanation templates for generated directions.
    pub explanation_templates: Vec<String>,
}

/// Candidate strategy pack for a curated kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateStrategyPack {
    /// Candidate strategy pack schema version.
    pub schema_version: u32,
    /// Stable pack ID.
    pub pack_id: String,
    /// Strategy summaries.
    pub strategies: Vec<KitCandidateStrategy>,
    /// Allowed controls across every strategy.
    pub allowed_controls: Vec<String>,
    /// Allowed provider choices keyed by slot.
    pub allowed_provider_choices: BTreeMap<String, Vec<String>>,
    /// Diversity goals used to select six coherent options.
    pub diversity_goals: Vec<String>,
    /// Invalid-state rejection policy.
    pub invalid_state_rejection_policy: String,
    /// Lock respect policy.
    pub lock_respect_policy: String,
}

/// Required quality gates for one kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityGateProfile {
    /// Quality gate profile schema version.
    pub schema_version: u32,
    /// Stable quality gate profile ID.
    pub profile_id: String,
    /// Required tier before catalog exposure.
    pub required_tier: FoundryKitQualityTier,
    /// Mesh gates.
    pub mesh_gates: Vec<String>,
    /// Candidate gates.
    pub candidate_gates: Vec<String>,
    /// Contact-sheet gates.
    pub contact_sheet_gates: Vec<String>,
    /// Export gates.
    pub export_gates: Vec<String>,
    /// Manual review gates.
    pub manual_review_gates: Vec<String>,
}

/// Compatible style/provider pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleProviderCompatibility {
    /// Style ID.
    pub style_id: String,
    /// Provider pack ID.
    pub provider_pack_id: String,
    /// Product-safe reason.
    pub reason: String,
}

/// Incompatible style/provider pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleProviderIncompatibility {
    /// Style ID.
    pub style_id: String,
    /// Provider pack ID.
    pub provider_pack_id: String,
    /// Product-safe hidden reason.
    pub hidden_reason: String,
}

/// Explicit style/provider compatibility matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitCompatibilityMatrix {
    /// Compatibility matrix schema version.
    pub schema_version: u32,
    /// Stable matrix ID.
    pub matrix_id: String,
    /// Compatible style/provider pairs.
    pub compatible_style_provider_pairs: Vec<StyleProviderCompatibility>,
    /// Incompatible style/provider pairs.
    pub incompatible_style_provider_pairs: Vec<StyleProviderIncompatibility>,
}

impl KitCompatibilityMatrix {
    fn style_provider_incompatibility(
        &self,
        style_id: &str,
        provider_pack_id: &str,
    ) -> Option<&StyleProviderIncompatibility> {
        self.incompatible_style_provider_pairs
            .iter()
            .find(|row| row.style_id == style_id && row.provider_pack_id == provider_pack_id)
    }
}

/// Manual and automatic review evidence for one kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitReviewManifest {
    /// Kit review manifest schema version.
    pub schema_version: u32,
    /// Stable review manifest ID.
    pub manifest_id: String,
    /// Tier requested by authoring or review.
    pub tier_requested: FoundryKitQualityTier,
    /// Tier achieved by evidence.
    pub tier_achieved: FoundryKitQualityTier,
    /// Optional local reviewer marker.
    pub reviewer: Option<String>,
    /// Human approval marker.
    pub human_approval_marker: bool,
    /// Adversarial visual review marker.
    pub adversarial_review_marker: bool,
    /// Product-safe visual review notes.
    pub visual_review_notes: Vec<String>,
    /// Contact-sheet paths or artifact references.
    pub contact_sheet_paths: Vec<String>,
    /// Benchmark report references.
    pub benchmark_refs: Vec<String>,
    /// Known limitations.
    pub known_limitations: Vec<String>,
    /// Reasons blocking catalog exposure or tier claims.
    pub blocked_reasons: Vec<String>,
}

/// Catalog manifest for a group of kits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitCatalogManifest {
    /// Kit catalog manifest schema version.
    pub schema_version: u32,
    /// Stable catalog ID.
    pub catalog_id: String,
    /// Kit IDs available to this catalog.
    pub kit_ids: Vec<String>,
    /// Kit IDs visible in the default novice catalog.
    pub default_visible_kit_ids: Vec<String>,
    /// Kit IDs visible only in developer/preview catalogs.
    pub developer_preview_kit_ids: Vec<String>,
    /// Product-safe hidden reasons keyed by kit ID.
    pub hidden_kit_reasons: BTreeMap<String, String>,
}

/// Complete self-contained Foundry kit package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryKitPackage {
    /// Package schema version.
    pub schema_version: u32,
    /// Top-level kit manifest.
    pub kit: FoundryKit,
    /// Family blueprint.
    pub family_blueprint: FamilyBlueprint,
    /// Provider pack.
    pub provider_pack: ProviderPack,
    /// Style pack.
    pub style_pack: StylePack,
    /// Novice control profile.
    pub control_profile: ControlProfile,
    /// Candidate strategy pack.
    pub candidate_strategy_pack: CandidateStrategyPack,
    /// Quality gate profile.
    pub quality_gate_profile: QualityGateProfile,
    /// Style/provider compatibility matrix.
    pub compatibility_matrix: KitCompatibilityMatrix,
    /// Review manifest.
    pub review_manifest: KitReviewManifest,
    /// Catalog manifest.
    pub catalog_manifest: KitCatalogManifest,
}

/// One Foundry kit validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryKitValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Foundry kit validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryKitValidationReport {
    /// Validation issues.
    pub issues: Vec<FoundryKitValidationIssue>,
}

impl FoundryKitValidationReport {
    /// Return true if no validation issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(FoundryKitValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Product visibility decision for one kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryKitVisibilityDecision {
    /// Whether the kit should be visible in the requested catalog mode.
    pub visible: bool,
    /// Whether the kit may display a Showcase badge.
    pub showcase_badge_allowed: bool,
    /// Product-safe reason when hidden.
    pub reason: Option<String>,
}

/// Validate a complete Foundry kit package.
#[must_use]
pub fn validate_foundry_kit_package(package: &FoundryKitPackage) -> FoundryKitValidationReport {
    let mut report = FoundryKitValidationReport::default();
    validate_schema_versions(package, &mut report);
    validate_cross_references(package, &mut report);
    validate_family_provider_style_compatibility(package, &mut report);
    validate_control_profile(package, &mut report);
    validate_quality_and_visibility(package, &mut report);
    validate_catalog_manifest(package, &mut report);
    report
}

/// Return a catalog visibility decision for one kit and review manifest.
#[must_use]
pub fn foundry_kit_visibility_decision(
    kit: &FoundryKit,
    review: &KitReviewManifest,
    developer_preview_enabled: bool,
) -> FoundryKitVisibilityDecision {
    let hidden = |reason: String| FoundryKitVisibilityDecision {
        visible: false,
        showcase_badge_allowed: false,
        reason: Some(reason),
    };

    match kit.quality_tier {
        FoundryKitQualityTier::Draft => {
            return hidden("Draft kits stay hidden from the default catalog.".to_owned());
        }
        FoundryKitQualityTier::Prototype if !developer_preview_enabled => {
            return hidden("Prototype kits require preview catalog mode.".to_owned());
        }
        FoundryKitQualityTier::Prototype => {
            return FoundryKitVisibilityDecision {
                visible: kit.catalog_visibility_policy.developer_preview_catalog,
                showcase_badge_allowed: false,
                reason: kit
                    .catalog_visibility_policy
                    .hidden_reason
                    .clone()
                    .filter(|_| !kit.catalog_visibility_policy.developer_preview_catalog),
            };
        }
        FoundryKitQualityTier::Usable => {
            if !kit.catalog_visibility_policy.default_novice_catalog {
                return hidden(
                    kit.catalog_visibility_policy
                        .hidden_reason
                        .clone()
                        .unwrap_or_else(|| {
                            "This kit is not enabled for the default catalog.".to_owned()
                        }),
                );
            }
            if !usable_review_evidence_passes(review) {
                return hidden("Usable kits require completed review evidence.".to_owned());
            }
        }
        FoundryKitQualityTier::Showcase => {
            if !kit.catalog_visibility_policy.default_novice_catalog
                && !kit.catalog_visibility_policy.showcase_badge_allowed
            {
                return hidden(
                    kit.catalog_visibility_policy
                        .hidden_reason
                        .clone()
                        .unwrap_or_else(|| {
                            "This kit is not enabled for the default catalog.".to_owned()
                        }),
                );
            }
            if !showcase_review_evidence_passes(review) {
                return hidden(
                    "Showcase kits require human approval and adversarial review.".to_owned(),
                );
            }
        }
    }

    if kit.catalog_visibility_policy.default_novice_catalog {
        FoundryKitVisibilityDecision {
            visible: true,
            showcase_badge_allowed: kit.quality_tier == FoundryKitQualityTier::Showcase
                && kit.catalog_visibility_policy.showcase_badge_allowed
                && showcase_review_evidence_passes(review),
            reason: None,
        }
    } else {
        hidden(
            kit.catalog_visibility_policy
                .hidden_reason
                .clone()
                .unwrap_or_else(|| "This kit is not enabled for the default catalog.".to_owned()),
        )
    }
}

fn validate_schema_versions(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let versions = [
        (
            package.schema_version,
            FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            "schema_version",
            "unsupported_foundry_kit_package_schema",
        ),
        (
            package.kit.schema_version,
            FOUNDRY_KIT_SCHEMA_VERSION,
            "kit.schema_version",
            "unsupported_foundry_kit_schema",
        ),
        (
            package.family_blueprint.schema_version,
            FAMILY_BLUEPRINT_SCHEMA_VERSION,
            "family_blueprint.schema_version",
            "unsupported_family_blueprint_schema",
        ),
        (
            package.provider_pack.schema_version,
            PROVIDER_PACK_SCHEMA_VERSION,
            "provider_pack.schema_version",
            "unsupported_provider_pack_schema",
        ),
        (
            package.style_pack.schema_version,
            STYLE_PACK_SCHEMA_VERSION,
            "style_pack.schema_version",
            "unsupported_style_pack_schema",
        ),
        (
            package.control_profile.schema_version,
            CONTROL_PROFILE_SCHEMA_VERSION,
            "control_profile.schema_version",
            "unsupported_control_profile_schema",
        ),
        (
            package.candidate_strategy_pack.schema_version,
            CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
            "candidate_strategy_pack.schema_version",
            "unsupported_candidate_strategy_pack_schema",
        ),
        (
            package.quality_gate_profile.schema_version,
            QUALITY_GATE_PROFILE_SCHEMA_VERSION,
            "quality_gate_profile.schema_version",
            "unsupported_quality_gate_profile_schema",
        ),
        (
            package.compatibility_matrix.schema_version,
            KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
            "compatibility_matrix.schema_version",
            "unsupported_compatibility_matrix_schema",
        ),
        (
            package.review_manifest.schema_version,
            KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
            "review_manifest.schema_version",
            "unsupported_review_manifest_schema",
        ),
        (
            package.catalog_manifest.schema_version,
            KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
            "catalog_manifest.schema_version",
            "unsupported_kit_catalog_manifest_schema",
        ),
    ];
    for (actual, expected, subject, code) in versions {
        if actual != expected {
            report.push(
                subject,
                code,
                format!("Expected schema version {expected}, found {actual}."),
            );
        }
    }
}

fn validate_cross_references(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    validate_ref_eq(
        report,
        "kit.family_blueprint_id",
        &package.kit.family_blueprint_id,
        &package.family_blueprint.family_id,
        "missing_family_blueprint_ref",
        "Kit family blueprint reference must match the embedded family blueprint.",
    );
    validate_ref_eq(
        report,
        "kit.provider_pack_id",
        &package.kit.provider_pack_id,
        &package.provider_pack.pack_id,
        "missing_provider_pack_ref",
        "Kit provider pack reference must match the embedded provider pack.",
    );
    validate_ref_eq(
        report,
        "kit.style_pack_id",
        &package.kit.style_pack_id,
        &package.style_pack.style_id,
        "missing_style_pack_ref",
        "Kit style pack reference must match the embedded style pack.",
    );
    validate_ref_eq(
        report,
        "kit.control_profile_id",
        &package.kit.control_profile_id,
        &package.control_profile.profile_id,
        "missing_control_profile_ref",
        "Kit control profile reference must match the embedded control profile.",
    );
    validate_ref_eq(
        report,
        "kit.candidate_strategy_pack_id",
        &package.kit.candidate_strategy_pack_id,
        &package.candidate_strategy_pack.pack_id,
        "missing_candidate_strategy_pack_ref",
        "Kit candidate strategy reference must match the embedded strategy pack.",
    );
    validate_ref_eq(
        report,
        "kit.quality_gate_profile_id",
        &package.kit.quality_gate_profile_id,
        &package.quality_gate_profile.profile_id,
        "missing_quality_gate_profile_ref",
        "Kit quality gate reference must match the embedded quality gate profile.",
    );
    validate_ref_eq(
        report,
        "kit.compatibility_matrix_id",
        &package.kit.compatibility_matrix_id,
        &package.compatibility_matrix.matrix_id,
        "missing_compatibility_matrix_ref",
        "Kit compatibility matrix reference must match the embedded matrix.",
    );
    validate_ref_eq(
        report,
        "kit.review_manifest_id",
        &package.kit.review_manifest_id,
        &package.review_manifest.manifest_id,
        "missing_review_manifest_ref",
        "Kit review manifest reference must match the embedded review manifest.",
    );
    validate_ref_eq(
        report,
        "kit.catalog_manifest_id",
        &package.kit.catalog_manifest_id,
        &package.catalog_manifest.catalog_id,
        "missing_catalog_manifest_ref",
        "Kit catalog manifest reference must match the embedded catalog manifest.",
    );
}

fn validate_ref_eq(
    report: &mut FoundryKitValidationReport,
    subject: &str,
    actual: &str,
    expected: &str,
    code: &str,
    message: &str,
) {
    if actual != expected {
        report.push(subject, code, message);
    }
}

fn validate_family_provider_style_compatibility(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let family_id = &package.family_blueprint.family_id;
    if package.control_profile.family_id != *family_id {
        report.push(
            "control_profile.family_id",
            "control_profile_family_mismatch",
            "Control profile must target the kit family blueprint.",
        );
    }
    if !package.provider_pack.supports_family(family_id) {
        report.push(
            "provider_pack.compatible_family_ids",
            "provider_pack_family_incompatible",
            "Provider pack must support the kit family.",
        );
    }
    if !package.style_pack.supports_family(family_id) {
        report.push(
            "style_pack.compatible_family_ids",
            "style_pack_family_incompatible",
            "Style pack must support the kit family.",
        );
    }
    if !package
        .style_pack
        .allows_provider_pack(&package.provider_pack.pack_id)
    {
        report.push(
            "style_pack.compatible_provider_packs",
            "style_provider_pack_incompatible",
            "Style pack must allow the selected provider pack.",
        );
    }
    if let Some(incompatibility) = package.compatibility_matrix.style_provider_incompatibility(
        &package.style_pack.style_id,
        &package.provider_pack.pack_id,
    ) {
        report.push(
            "compatibility_matrix.incompatible_style_provider_pairs",
            "style_provider_pair_incompatible",
            format!(
                "Incompatible style/provider pair is hidden from novice catalog: {}",
                incompatibility.hidden_reason
            ),
        );
    }
    let provider_tags = provider_pack_tags(&package.provider_pack);
    if let Some(tag) = package
        .style_pack
        .forbidden_provider_tags
        .iter()
        .find(|tag| provider_tags.contains(tag.as_str()))
    {
        report.push(
            "style_pack.forbidden_provider_tags",
            "style_forbidden_provider_tag",
            format!("Provider pack carries forbidden style tag '{tag}'."),
        );
    }
    if !package.style_pack.allowed_provider_tags.is_empty()
        && !package
            .style_pack
            .allowed_provider_tags
            .iter()
            .any(|tag| provider_tags.contains(tag.as_str()))
    {
        report.push(
            "style_pack.allowed_provider_tags",
            "style_missing_allowed_provider_tag",
            "Provider pack must carry at least one tag allowed by the style pack.",
        );
    }

    let role_ids = package
        .family_blueprint
        .semantic_roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect::<BTreeSet<_>>();
    for required_role in &package.family_blueprint.required_roles {
        if !role_ids.contains(required_role.as_str()) {
            report.push(
                format!("family_blueprint.required_roles.{required_role}"),
                "unknown_required_role",
                "Required role must exist in the family role inventory.",
            );
        }
        if !package
            .provider_pack
            .semantic_role_coverage
            .iter()
            .any(|role| role == required_role)
        {
            report.push(
                format!("provider_pack.semantic_role_coverage.{required_role}"),
                "missing_required_role_coverage",
                "Provider pack must cover every required family role.",
            );
        }
    }
    for optional_role in &package.family_blueprint.optional_roles {
        if !role_ids.contains(optional_role.as_str()) {
            report.push(
                format!("family_blueprint.optional_roles.{optional_role}"),
                "unknown_optional_role",
                "Optional role must exist in the family role inventory.",
            );
        }
    }

    let supplied_slots = package
        .provider_pack
        .provider_slots_supplied
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let declared_slots = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let slot_expected_roles = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| (slot.slot_id.as_str(), slot.role_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    for supplied_slot in &package.provider_pack.provider_slots_supplied {
        if !declared_slots.contains(supplied_slot.as_str()) {
            report.push(
                format!("provider_pack.provider_slots_supplied.{supplied_slot}"),
                "unknown_provider_slot_supplied",
                "Provider pack supplies a slot not declared by the family blueprint.",
            );
        }
    }
    for slot in &package.family_blueprint.provider_slots {
        if slot.required && !supplied_slots.contains(slot.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_slots_supplied.{}", slot.slot_id),
                "missing_required_provider_slot",
                "Provider pack must supply every required family provider slot.",
            );
        }
    }
    let mut option_ids = BTreeSet::new();
    for (index, option) in package.provider_pack.provider_options.iter().enumerate() {
        if !option_ids.insert(option.option_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.option_id"),
                "duplicate_provider_option_id",
                "Provider option IDs must be unique within a provider pack.",
            );
        }
        if !declared_slots.contains(option.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.slot_id"),
                "provider_option_unknown_slot",
                "Provider option slot must be declared by the family blueprint.",
            );
        }
        if !supplied_slots.contains(option.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.slot_id"),
                "provider_option_unsupplied_slot",
                "Provider option slot must be included in provider_slots_supplied.",
            );
        }
        if let Some(expected_role) = slot_expected_roles.get(option.slot_id.as_str())
            && !option
                .semantic_roles
                .iter()
                .any(|role| role.as_str() == *expected_role)
        {
            report.push(
                format!("provider_pack.provider_options.{index}.semantic_roles"),
                "provider_option_missing_slot_role",
                "Provider option semantic roles must include the role expected by its slot.",
            );
        }
        for role in &option.semantic_roles {
            if !role_ids.contains(role.as_str()) {
                report.push(
                    format!("provider_pack.provider_options.{index}.semantic_roles.{role}"),
                    "provider_option_unknown_role",
                    "Provider option semantic role must exist in the family blueprint.",
                );
            }
        }
    }
    for covered_role in &package.provider_pack.semantic_role_coverage {
        if !role_ids.contains(covered_role.as_str()) {
            report.push(
                format!("provider_pack.semantic_role_coverage.{covered_role}"),
                "provider_pack_unknown_covered_role",
                "Provider pack role coverage must reference a known family role.",
            );
        }
        if !package.provider_pack.provider_options.iter().any(|option| {
            option
                .semantic_roles
                .iter()
                .any(|role| role == covered_role)
        }) {
            report.push(
                format!("provider_pack.semantic_role_coverage.{covered_role}"),
                "provider_role_coverage_not_backed_by_option",
                "Provider pack role coverage must be backed by at least one provider option.",
            );
        }
    }
}

fn validate_control_profile(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let maximum_primary = package
        .control_profile
        .maximum_primary_controls
        .min(DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS);
    let primary_count = package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count() as u32;
    if primary_count > maximum_primary {
        report.push(
            "control_profile.controls",
            "too_many_primary_controls",
            format!(
                "Default novice control profiles may expose at most {maximum_primary} primary controls."
            ),
        );
    }

    let family_slots = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut family_slot_owners = BTreeMap::<&str, &str>::new();
    let mut provider_slot_owners = BTreeMap::<&str, &str>::new();
    for control in package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
    {
        for slot in &control.owned_family_slots {
            if let Some(previous) = family_slot_owners.insert(slot.as_str(), &control.control_id) {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_family_slots",
                        control.control_id
                    ),
                    "duplicate_visible_control_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own family slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
        for slot in &control.owned_provider_slots {
            if !family_slots.is_empty() && !family_slots.contains(slot.as_str()) {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "unknown_provider_slot_ownership",
                    "Visible control owns a provider slot not declared by the family blueprint.",
                );
            }
            if let Some(previous) = provider_slot_owners.insert(slot.as_str(), &control.control_id)
            {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "duplicate_visible_control_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own provider slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
    }
    validate_candidate_strategy_pack(package, report);
}

fn validate_candidate_strategy_pack(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let visible_control_ids = package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| control.control_id.as_str())
        .collect::<BTreeSet<_>>();
    let allowed_controls = package
        .candidate_strategy_pack
        .allowed_controls
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for control_id in &package.candidate_strategy_pack.allowed_controls {
        if !visible_control_ids.contains(control_id.as_str()) {
            report.push(
                format!("candidate_strategy_pack.allowed_controls.{control_id}"),
                "candidate_strategy_unknown_control",
                "Candidate strategy allowed controls must exist in the visible control profile.",
            );
        }
    }
    for (index, strategy) in package
        .candidate_strategy_pack
        .strategies
        .iter()
        .enumerate()
    {
        for control_id in &strategy.allowed_controls {
            if !visible_control_ids.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{index}.allowed_controls.{control_id}"
                    ),
                    "candidate_strategy_unknown_control",
                    "Candidate strategies must reference visible controls.",
                );
            }
            if !allowed_controls.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{index}.allowed_controls.{control_id}"
                    ),
                    "candidate_strategy_control_not_allowed",
                    "Candidate strategy controls must be listed in the pack allowed_controls set.",
                );
            }
        }
    }

    let supplied_slots = package
        .provider_pack
        .provider_slots_supplied
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for (slot, choices) in &package.candidate_strategy_pack.allowed_provider_choices {
        if !supplied_slots.contains(slot.as_str()) {
            report.push(
                format!("candidate_strategy_pack.allowed_provider_choices.{slot}"),
                "candidate_strategy_unknown_provider_slot",
                "Candidate strategy provider choices must reference supplied provider slots.",
            );
        }
        for choice in choices {
            if !package
                .provider_pack
                .provider_options
                .iter()
                .any(|option| option.slot_id == *slot && option.option_id == *choice)
            {
                report.push(
                    format!("candidate_strategy_pack.allowed_provider_choices.{slot}.{choice}"),
                    "candidate_strategy_unknown_provider_choice",
                    "Candidate strategy provider choices must reference provider options for the same slot.",
                );
            }
        }
    }
}

fn validate_quality_and_visibility(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let kit = &package.kit;
    let review = &package.review_manifest;
    if review.tier_achieved < package.quality_gate_profile.required_tier {
        report.push(
            "review_manifest.tier_achieved",
            "quality_gate_tier_not_met",
            "Review manifest must meet the quality gate profile required tier.",
        );
    }
    if kit.quality_tier >= FoundryKitQualityTier::Usable && review.contact_sheet_paths.is_empty() {
        report.push(
            "review_manifest.contact_sheet_paths",
            "missing_contact_sheet_evidence",
            "Usable and Showcase kit claims require contact-sheet evidence.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Showcase {
        if !review.human_approval_marker {
            report.push(
                "review_manifest.human_approval_marker",
                "showcase_missing_human_approval",
                "Showcase kits require human approval.",
            );
        }
        if !review.adversarial_review_marker {
            report.push(
                "review_manifest.adversarial_review_marker",
                "showcase_missing_adversarial_review",
                "Showcase kits require adversarial visual review.",
            );
        }
    }

    if matches!(
        kit.quality_tier,
        FoundryKitQualityTier::Draft | FoundryKitQualityTier::Prototype
    ) && kit.catalog_visibility_policy.default_novice_catalog
    {
        report.push(
            "kit.catalog_visibility_policy.default_novice_catalog",
            "draft_or_prototype_visible_by_default",
            "Draft and Prototype kits must stay hidden from the default novice catalog.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Usable
        && kit.catalog_visibility_policy.default_novice_catalog
        && !usable_review_evidence_passes(review)
    {
        report.push(
            "kit.catalog_visibility_policy.default_novice_catalog",
            "usable_visible_without_review_evidence",
            "Usable kits can be novice-visible only after required review evidence passes.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Showcase
        && (kit.catalog_visibility_policy.default_novice_catalog
            || kit.catalog_visibility_policy.showcase_badge_allowed)
        && !showcase_review_evidence_passes(review)
    {
        report.push(
            "kit.catalog_visibility_policy.showcase_badge_allowed",
            "showcase_visible_without_review_evidence",
            "Showcase visibility and badges require human approval and adversarial review.",
        );
    }
}

fn validate_catalog_manifest(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let kit_id = &package.kit.kit_id;
    let kit_ids = package
        .catalog_manifest
        .kit_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if !package.catalog_manifest.kit_ids.contains(kit_id) {
        report.push(
            "catalog_manifest.kit_ids",
            "catalog_manifest_missing_kit",
            "Catalog manifest must include the package kit ID.",
        );
    }
    for visible_kit_id in &package.catalog_manifest.default_visible_kit_ids {
        if !kit_ids.contains(visible_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.default_visible_kit_ids.{visible_kit_id}"),
                "catalog_manifest_unknown_default_visible_kit",
                "Default-visible catalog IDs must be listed in kit_ids.",
            );
        }
    }
    for preview_kit_id in &package.catalog_manifest.developer_preview_kit_ids {
        if !kit_ids.contains(preview_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.developer_preview_kit_ids.{preview_kit_id}"),
                "catalog_manifest_unknown_developer_preview_kit",
                "Developer-preview catalog IDs must be listed in kit_ids.",
            );
        }
    }
    for (hidden_kit_id, reason) in &package.catalog_manifest.hidden_kit_reasons {
        if !kit_ids.contains(hidden_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.hidden_kit_reasons.{hidden_kit_id}"),
                "catalog_manifest_unknown_hidden_kit",
                "Hidden kit reasons must reference IDs listed in kit_ids.",
            );
        }
        if reason.trim().is_empty() {
            report.push(
                format!("catalog_manifest.hidden_kit_reasons.{hidden_kit_id}"),
                "catalog_manifest_empty_hidden_reason",
                "Hidden kit reasons must use plain-language non-empty text.",
            );
        }
    }
    if package
        .catalog_manifest
        .default_visible_kit_ids
        .contains(kit_id)
        != package.kit.catalog_visibility_policy.default_novice_catalog
    {
        report.push(
            "catalog_manifest.default_visible_kit_ids",
            "catalog_manifest_visibility_mismatch",
            "Catalog manifest default visibility must match the kit visibility policy.",
        );
    }
    if package
        .catalog_manifest
        .developer_preview_kit_ids
        .contains(kit_id)
        != package
            .kit
            .catalog_visibility_policy
            .developer_preview_catalog
    {
        report.push(
            "catalog_manifest.developer_preview_kit_ids",
            "catalog_manifest_developer_preview_mismatch",
            "Catalog manifest developer-preview visibility must match the kit visibility policy.",
        );
    }
    if !package.kit.catalog_visibility_policy.default_novice_catalog {
        if package
            .kit
            .catalog_visibility_policy
            .hidden_reason
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            report.push(
                "kit.catalog_visibility_policy.hidden_reason",
                "kit_visibility_missing_hidden_reason",
                "Hidden kits must carry a plain-language reason.",
            );
        }
        if package
            .catalog_manifest
            .hidden_kit_reasons
            .get(kit_id)
            .map(String::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            report.push(
                "catalog_manifest.hidden_kit_reasons",
                "catalog_manifest_missing_hidden_reason",
                "Catalog manifest must explain why hidden kits are not default-visible.",
            );
        }
    }
}

fn usable_review_evidence_passes(review: &KitReviewManifest) -> bool {
    review.tier_achieved >= FoundryKitQualityTier::Usable
        && review.human_approval_marker
        && !review.contact_sheet_paths.is_empty()
        && !review.benchmark_refs.is_empty()
        && review.blocked_reasons.is_empty()
}

fn showcase_review_evidence_passes(review: &KitReviewManifest) -> bool {
    usable_review_evidence_passes(review) && review.adversarial_review_marker
}

fn provider_pack_tags(provider_pack: &ProviderPack) -> BTreeSet<&str> {
    let mut tags = BTreeSet::new();
    tags.extend(provider_pack.compatibility_tags.iter().map(String::as_str));
    tags.extend(
        provider_pack
            .socket_attachment_tags
            .iter()
            .map(String::as_str),
    );
    tags.extend(provider_pack.detail_density_tags.iter().map(String::as_str));
    for option in &provider_pack.provider_options {
        tags.extend(option.compatibility_tags.iter().map(String::as_str));
        tags.extend(option.detail_density_tags.iter().map(String::as_str));
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundry_kit_schema_roundtrips() {
        let package = valid_package(FoundryKitQualityTier::Prototype);
        let json = serde_json::to_string_pretty(&package).expect("serialize kit package");
        let decoded: FoundryKitPackage =
            serde_json::from_str(&json).expect("deserialize kit package");
        assert_eq!(decoded, package);
        assert!(validate_foundry_kit_package(&decoded).is_valid());
    }

    #[test]
    fn provider_pack_schema_roundtrips() {
        let package = valid_package(FoundryKitQualityTier::Prototype);
        let json = serde_json::to_string(&package.provider_pack).expect("serialize provider pack");
        let decoded: ProviderPack = serde_json::from_str(&json).expect("deserialize provider pack");
        assert_eq!(decoded, package.provider_pack);
    }

    #[test]
    fn style_pack_compatibility_rejects_incompatible_provider() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .style_pack
            .incompatible_provider_packs
            .push(package.provider_pack.pack_id.clone());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_provider_pack_incompatible"));
    }

    #[test]
    fn incompatible_provider_style_pair_is_not_novice_visible() {
        let mut package = reviewed_usable_package();
        package
            .compatibility_matrix
            .incompatible_style_provider_pairs
            .push(StyleProviderIncompatibility {
                style_id: package.style_pack.style_id.clone(),
                provider_pack_id: package.provider_pack.pack_id.clone(),
                hidden_reason: "Style and kit do not produce coherent output.".to_owned(),
            });
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_provider_pair_incompatible"));
    }

    #[test]
    fn draft_and_prototype_are_hidden_by_default() {
        let draft = valid_package(FoundryKitQualityTier::Draft);
        let prototype = valid_package(FoundryKitQualityTier::Prototype);
        assert!(
            !foundry_kit_visibility_decision(&draft.kit, &draft.review_manifest, false).visible
        );
        assert!(
            !foundry_kit_visibility_decision(&prototype.kit, &prototype.review_manifest, false)
                .visible
        );
        assert!(
            foundry_kit_visibility_decision(&prototype.kit, &prototype.review_manifest, true)
                .visible
        );
    }

    #[test]
    fn usable_visibility_requires_review_evidence() {
        let mut package = reviewed_usable_package();
        assert!(validate_foundry_kit_package(&package).is_valid());
        assert!(
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
        );
        package.review_manifest.human_approval_marker = false;
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "usable_visible_without_review_evidence"));
        assert!(
            !foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
        );
    }

    #[test]
    fn showcase_visibility_requires_human_and_adversarial_review() {
        let mut package = reviewed_usable_package();
        package.kit.quality_tier = FoundryKitQualityTier::Showcase;
        package.kit.catalog_visibility_policy.showcase_badge_allowed = true;
        package.review_manifest.tier_requested = FoundryKitQualityTier::Showcase;
        package.review_manifest.tier_achieved = FoundryKitQualityTier::Showcase;
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "showcase_missing_adversarial_review"));
        package.review_manifest.adversarial_review_marker = true;
        assert!(validate_foundry_kit_package(&package).is_valid());
        let decision =
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
        assert!(decision.visible);
        assert!(decision.showcase_badge_allowed);
    }

    #[test]
    fn primary_controls_are_limited_to_seven_by_default() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        for index in 1..=8 {
            package.control_profile.controls.push(control(
                &format!("control-{index}"),
                &format!("slot-{index}"),
            ));
        }
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "too_many_primary_controls"));
    }

    #[test]
    fn duplicate_visible_control_ownership_is_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.control_profile.controls =
            vec![control("width-a", "width"), control("width-b", "width")];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "duplicate_visible_control_ownership"));
    }

    #[test]
    fn missing_required_provider_slot_is_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_slots_supplied.clear();
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "missing_required_provider_slot"));
    }

    #[test]
    fn forbidden_provider_tags_are_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .style_pack
            .forbidden_provider_tags
            .push("timber".to_owned());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_forbidden_provider_tag"));
    }

    #[test]
    fn allowed_provider_tags_must_overlap_when_declared() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.style_pack.allowed_provider_tags = vec!["metal".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_missing_allowed_provider_tag"));
    }

    #[test]
    fn provider_options_must_reference_known_slots_and_roles() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_options[0].slot_id = "missing-slot".to_owned();
        package.provider_pack.provider_options[0].semantic_roles = vec!["missing-role".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "provider_option_unknown_slot"));
        assert!(has_issue(&report, "provider_option_unsupplied_slot"));
        assert!(has_issue(&report, "provider_option_unknown_role"));
    }

    #[test]
    fn provider_options_must_cover_their_slot_role() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_options[0].semantic_roles = vec!["brace".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "provider_option_missing_slot_role"));
    }

    #[test]
    fn candidate_strategy_controls_must_exist() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .candidate_strategy_pack
            .allowed_controls
            .push("missing-control".to_owned());
        package.candidate_strategy_pack.strategies[0]
            .allowed_controls
            .push("missing-control".to_owned());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "candidate_strategy_unknown_control"));
    }

    #[test]
    fn catalog_manifest_matches_visibility_policy_and_known_ids() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .catalog_manifest
            .default_visible_kit_ids
            .push(package.kit.kit_id.clone());
        package
            .catalog_manifest
            .default_visible_kit_ids
            .push("missing-kit".to_owned());
        package.catalog_manifest.developer_preview_kit_ids.clear();
        package
            .catalog_manifest
            .hidden_kit_reasons
            .insert("missing-kit".to_owned(), String::new());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(
            &report,
            "catalog_manifest_unknown_default_visible_kit"
        ));
        assert!(has_issue(&report, "catalog_manifest_visibility_mismatch"));
        assert!(has_issue(
            &report,
            "catalog_manifest_developer_preview_mismatch"
        ));
        assert!(has_issue(&report, "catalog_manifest_unknown_hidden_kit"));
        assert!(has_issue(&report, "catalog_manifest_empty_hidden_reason"));
    }

    fn reviewed_usable_package() -> FoundryKitPackage {
        let mut package = valid_package(FoundryKitQualityTier::Usable);
        package.kit.catalog_visibility_policy = CatalogVisibilityPolicy::novice_visible();
        package.review_manifest.tier_requested = FoundryKitQualityTier::Usable;
        package.review_manifest.tier_achieved = FoundryKitQualityTier::Usable;
        package.review_manifest.human_approval_marker = true;
        package.review_manifest.contact_sheet_paths =
            vec!["target/hq/contact-sheet.png".to_owned()];
        package.review_manifest.benchmark_refs = vec!["target/hq/quality-report.json".to_owned()];
        package.catalog_manifest.default_visible_kit_ids = vec![package.kit.kit_id.clone()];
        package.catalog_manifest.hidden_kit_reasons.clear();
        package
    }

    fn valid_package(tier: FoundryKitQualityTier) -> FoundryKitPackage {
        let visibility = match tier {
            FoundryKitQualityTier::Draft => CatalogVisibilityPolicy {
                default_novice_catalog: false,
                developer_preview_catalog: false,
                showcase_badge_allowed: false,
                hidden_reason: Some("Draft content is hidden.".to_owned()),
            },
            FoundryKitQualityTier::Prototype => {
                CatalogVisibilityPolicy::hidden("Prototype content requires preview catalog mode.")
            }
            FoundryKitQualityTier::Usable | FoundryKitQualityTier::Showcase => {
                CatalogVisibilityPolicy::hidden("Review evidence is pending.")
            }
        };
        let review_tier = if tier >= FoundryKitQualityTier::Usable {
            FoundryKitQualityTier::Usable
        } else {
            tier
        };
        FoundryKitPackage {
            schema_version: FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            kit: FoundryKit {
                schema_version: FOUNDRY_KIT_SCHEMA_VERSION,
                kit_id: "bridge-kit".to_owned(),
                display_name: "Bridge Kit".to_owned(),
                family_blueprint_id: "bridge".to_owned(),
                provider_pack_id: "bridge-providers".to_owned(),
                style_pack_id: "timber-style".to_owned(),
                control_profile_id: "bridge-controls".to_owned(),
                candidate_strategy_pack_id: "bridge-strategies".to_owned(),
                quality_gate_profile_id: "usable-gates".to_owned(),
                compatibility_matrix_id: "bridge-compatibility".to_owned(),
                review_manifest_id: "bridge-review".to_owned(),
                catalog_manifest_id: "bridge-catalog".to_owned(),
                preview_camera_policy: PreviewCameraPolicy {
                    policy_id: "clay-review".to_owned(),
                    required_views: vec!["front".to_owned(), "three-quarter".to_owned()],
                    clay_preview_required: true,
                    contact_sheet_required: tier >= FoundryKitQualityTier::Usable,
                },
                quality_tier: tier,
                catalog_visibility_policy: visibility,
                source_profile_slug: Some("roman-bridge".to_owned()),
                category_chips: vec!["Bridge".to_owned()],
            },
            family_blueprint: FamilyBlueprint {
                schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
                family_id: "bridge".to_owned(),
                display_name: "Bridge".to_owned(),
                semantic_roles: vec![
                    FamilyBlueprintRole {
                        role_id: "deck".to_owned(),
                        label: "Deck".to_owned(),
                        required: true,
                        tags: vec!["structure".to_owned()],
                    },
                    FamilyBlueprintRole {
                        role_id: "brace".to_owned(),
                        label: "Brace".to_owned(),
                        required: false,
                        tags: vec!["detail".to_owned()],
                    },
                ],
                required_roles: vec!["deck".to_owned()],
                optional_roles: vec!["brace".to_owned()],
                provider_slots: vec![ProviderSlotExpectation {
                    slot_id: "deck-slot".to_owned(),
                    role_id: "deck".to_owned(),
                    required: true,
                    attachment_tags: vec!["support".to_owned()],
                }],
                attachment_expectations: Vec::new(),
                scale_policy: HighLevelScalePolicy {
                    label: "Prop scale".to_owned(),
                    allowed_range: Some("normalized".to_owned()),
                },
                export_part_naming_policy: ExportPartNamingPolicy {
                    strategy: "role-labels".to_owned(),
                    required_part_names: vec!["deck".to_owned()],
                },
            },
            provider_pack: ProviderPack {
                schema_version: PROVIDER_PACK_SCHEMA_VERSION,
                pack_id: "bridge-providers".to_owned(),
                family_id: Some("bridge".to_owned()),
                compatible_family_ids: vec!["bridge".to_owned()],
                provider_slots_supplied: vec!["deck-slot".to_owned()],
                provider_options: vec![ProviderPackOption {
                    option_id: "deck-basic".to_owned(),
                    slot_id: "deck-slot".to_owned(),
                    label: "Straight Deck".to_owned(),
                    semantic_roles: vec!["deck".to_owned()],
                    compatibility_tags: vec!["timber".to_owned()],
                    detail_density_tags: vec!["medium".to_owned()],
                    triangle_budget_estimate: Some(1200),
                }],
                semantic_role_coverage: vec!["deck".to_owned()],
                socket_attachment_tags: vec!["support".to_owned()],
                detail_density_tags: vec!["medium".to_owned()],
                triangle_budget_estimates: BTreeMap::from([("deck-basic".to_owned(), 1200)]),
                compatibility_tags: vec!["timber".to_owned()],
            },
            style_pack: StylePack {
                schema_version: STYLE_PACK_SCHEMA_VERSION,
                style_id: "timber-style".to_owned(),
                display_name: "Timber".to_owned(),
                compatible_family_ids: vec!["bridge".to_owned()],
                bevel_language: "soft structural edges".to_owned(),
                proportion_language: "sturdy broad spans".to_owned(),
                detail_density_policy: "medium detail".to_owned(),
                silhouette_exaggeration_policy: "readable from three-quarter view".to_owned(),
                symmetry_asymmetry_policy: "mostly symmetric with optional wear".to_owned(),
                allowed_provider_tags: vec!["timber".to_owned()],
                forbidden_provider_tags: vec!["sci-fi".to_owned()],
                compatible_provider_packs: vec!["bridge-providers".to_owned()],
                incompatible_provider_packs: Vec::new(),
                future_material_vocabulary: Some(FutureMaterialVocabulary {
                    label: "Future material notes".to_owned(),
                    tags: vec!["wood".to_owned()],
                }),
            },
            control_profile: ControlProfile {
                schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
                profile_id: "bridge-controls".to_owned(),
                family_id: "bridge".to_owned(),
                style_id: Some("timber-style".to_owned()),
                maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
                controls: vec![control("deck-width", "width")],
            },
            candidate_strategy_pack: CandidateStrategyPack {
                schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
                pack_id: "bridge-strategies".to_owned(),
                strategies: vec![KitCandidateStrategy {
                    strategy_id: "refine".to_owned(),
                    name: "Refine".to_owned(),
                    allowed_controls: vec!["deck-width".to_owned()],
                    explanation_templates: vec!["Adjusted whole-model proportions.".to_owned()],
                }],
                allowed_controls: vec!["deck-width".to_owned()],
                allowed_provider_choices: BTreeMap::new(),
                diversity_goals: vec!["six coherent options".to_owned()],
                invalid_state_rejection_policy: "Reject invalid compile states.".to_owned(),
                lock_respect_policy: "Preserve locked controls.".to_owned(),
            },
            quality_gate_profile: QualityGateProfile {
                schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
                profile_id: "usable-gates".to_owned(),
                required_tier: review_tier,
                mesh_gates: vec!["valid mesh".to_owned()],
                candidate_gates: vec!["six candidates".to_owned()],
                contact_sheet_gates: vec!["clay contact sheet".to_owned()],
                export_gates: vec!["package reopen".to_owned()],
                manual_review_gates: vec!["human review".to_owned()],
            },
            compatibility_matrix: KitCompatibilityMatrix {
                schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
                matrix_id: "bridge-compatibility".to_owned(),
                compatible_style_provider_pairs: vec![StyleProviderCompatibility {
                    style_id: "timber-style".to_owned(),
                    provider_pack_id: "bridge-providers".to_owned(),
                    reason: "Coherent timber bridge style.".to_owned(),
                }],
                incompatible_style_provider_pairs: Vec::new(),
            },
            review_manifest: KitReviewManifest {
                schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
                manifest_id: "bridge-review".to_owned(),
                tier_requested: tier,
                tier_achieved: review_tier,
                reviewer: None,
                human_approval_marker: false,
                adversarial_review_marker: false,
                visual_review_notes: vec!["Review pending.".to_owned()],
                contact_sheet_paths: if tier >= FoundryKitQualityTier::Usable {
                    vec!["target/hq/contact-sheet.png".to_owned()]
                } else {
                    Vec::new()
                },
                benchmark_refs: Vec::new(),
                known_limitations: Vec::new(),
                blocked_reasons: Vec::new(),
            },
            catalog_manifest: KitCatalogManifest {
                schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
                catalog_id: "bridge-catalog".to_owned(),
                kit_ids: vec!["bridge-kit".to_owned()],
                default_visible_kit_ids: Vec::new(),
                developer_preview_kit_ids: if tier == FoundryKitQualityTier::Draft {
                    Vec::new()
                } else {
                    vec!["bridge-kit".to_owned()]
                },
                hidden_kit_reasons: BTreeMap::from([(
                    "bridge-kit".to_owned(),
                    "Review pending.".to_owned(),
                )]),
            },
        }
    }

    fn control(control_id: &str, slot: &str) -> ControlProfileControl {
        ControlProfileControl {
            control_id: control_id.to_owned(),
            label: control_id.replace('-', " "),
            description: "Changes the whole-model result.".to_owned(),
            kind: ControlProfileControlKind::Continuous,
            owned_family_slots: vec![slot.to_owned()],
            owned_provider_slots: Vec::new(),
            visible_effect_expectation: "Visible whole-model difference.".to_owned(),
            topology_behavior: ControlProfileTopologyBehavior::TopologyPreserving,
            option_visibility: ControlOptionVisibility {
                hide_invalid_from_novices: true,
                show_plain_language_reasons: true,
            },
            default_locked: false,
            primary: true,
            visible: true,
        }
    }

    fn has_issue(report: &FoundryKitValidationReport, code: &str) -> bool {
        report.issues.iter().any(|issue| issue.code == code)
    }
}
