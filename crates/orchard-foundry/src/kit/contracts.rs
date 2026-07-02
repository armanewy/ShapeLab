
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
