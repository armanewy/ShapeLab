#![forbid(unsafe_code)]

//! Foundry catalog manifest contracts and headless fixture catalog entries.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, ModelingOperationSpec,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, SocketId, SocketSpec,
    Transform3, definition_scalar_path, validate_asset_recipe,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, BevelPolicy,
    ExaggerationPolicy, FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot,
    FamilyStyleFacet, FamilyStylePolicyOverrides, LengthValue, NormalizedBevelProfile,
    ParameterExecutionPolicy, ParameterRange, PartPrototype, PartRole, ProfileLanguage,
    RepetitionPolicy, RoleMultiplicity, RoleProvision, RuntimeMetadataRequirement,
    STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, FragmentSocketPort,
    ParameterBinding, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports,
    STYLE_IMPLEMENTATION_SCHEMA_VERSION, StyleImplementation, scalar_parameter,
};
use shape_foundry::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_FAMILY_IMPL,
    CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL, CUSTOMIZER_PROFILE_SCHEMA_VERSION,
    CandidateStrategy, CatalogContentRef, ChoiceOption, ClosedInterval, ControlKind,
    ControlSlotBinding, ControlTopologyBehavior, ControlValue, CustomizerControl,
    CustomizerProfile, DomainCertification, FeasibleControlDomain, FoundryAssetDocument,
    FoundryCatalogError, FoundryCatalogLock, FoundryCatalogResolver, FoundryDocumentId,
    ResponseCurve, WholeModelPreviewRef, catalog_content_fingerprint_from_json,
    document_catalog_refs,
};

pub mod authoring;
pub mod box_primitive;
pub mod kits;

pub use authoring::*;
pub use kits::*;

const LOCAL_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const LOCAL_INSTANCE: PartInstanceId = PartInstanceId(91);

/// Current schema version for catalog manifests.
pub const FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION: u32 = 1;
/// Package version for catalog contracts.
pub const SHAPE_FOUNDRY_CATALOG_CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One named catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogEntry {
    /// Content reference.
    pub content_ref: CatalogContentRef,
    /// Human-facing label.
    pub label: String,
    /// Catalog tags.
    pub tags: Vec<String>,
}

/// Catalog manifest that can produce exact locks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogManifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Stable catalog ID.
    pub catalog_id: String,
    /// Catalog version.
    pub catalog_version: u32,
    /// Entries keyed by stable content ID.
    pub entries: BTreeMap<String, FoundryCatalogEntry>,
}

/// Product-catalog curation state for a built-in Visual Foundry profile.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CatalogCurationState {
    /// Internal draft content. It is available to direct developer tests only.
    HiddenDraft,
    /// Compiling or experimental content visible only when preview catalog mode is enabled.
    PreviewOnly,
    /// Content with visual-direction and readable-control evidence for novice catalog exposure.
    Usable,
    /// Reviewed product-quality content. Requires human and adversarial review.
    Showcase,
}

impl CatalogCurationState {
    /// Human-facing curation label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HiddenDraft => "HiddenDraft",
            Self::PreviewOnly => "PreviewOnly",
            Self::Usable => "Usable",
            Self::Showcase => "Showcase",
        }
    }

    /// Whether this state appears in the default novice catalog.
    #[must_use]
    pub const fn default_novice_visible(self) -> bool {
        matches!(self, Self::Usable | Self::Showcase)
    }

    /// Whether this state appears when the preview catalog is enabled.
    #[must_use]
    pub const fn preview_catalog_visible(self) -> bool {
        matches!(self, Self::PreviewOnly | Self::Usable | Self::Showcase)
    }
}

/// Evidence summary used to gate novice-facing built-in catalog exposure.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CatalogCurationMetadata {
    /// Built-in fixture slug.
    pub profile_slug: &'static str,
    /// Catalog curation state.
    pub state: CatalogCurationState,
    /// Whether authored direction/contact-sheet evidence exists.
    pub has_visual_direction_evidence: bool,
    /// Whether primary controls have readable whole-model evidence.
    pub has_readable_control_evidence: bool,
    /// Whether a human reviewer approved Showcase status.
    pub has_human_showcase_review: bool,
    /// Short product-truth note for docs and tests.
    pub note: &'static str,
}

impl CatalogCurationMetadata {
    /// Whether this profile may appear in the default novice catalog.
    #[must_use]
    pub const fn default_novice_visible(self) -> bool {
        self.state.default_novice_visible()
    }

    /// Whether this profile may appear when preview catalog mode is enabled.
    #[must_use]
    pub const fn preview_catalog_visible(self) -> bool {
        self.state.preview_catalog_visible()
    }

    /// Whether this metadata satisfies the curation policy invariants.
    #[must_use]
    pub const fn policy_invariants_pass(self) -> bool {
        let usable_has_evidence = match self.state {
            CatalogCurationState::Usable | CatalogCurationState::Showcase => {
                self.has_visual_direction_evidence && self.has_readable_control_evidence
            }
            CatalogCurationState::HiddenDraft | CatalogCurationState::PreviewOnly => true,
        };
        let showcase_has_human_review = match self.state {
            CatalogCurationState::Showcase => self.has_human_showcase_review,
            CatalogCurationState::HiddenDraft
            | CatalogCurationState::PreviewOnly
            | CatalogCurationState::Usable => true,
        };
        usable_has_evidence && showcase_has_human_review
    }
}

/// Minimal starter-template benchmark evidence used to gate novice catalog exposure.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StarterTemplateQualityEvidence {
    /// Built-in fixture slug.
    pub profile_slug: &'static str,
    /// Count of returned selectable whole-asset ideas.
    pub visible_idea_count: usize,
    /// Count of distinct selectable whole-asset ideas.
    pub distinct_visible_idea_count: usize,
    /// Count of primary controls in the novice profile.
    pub primary_control_count: usize,
    /// Count of primary controls with endpoint report rows.
    pub endpoint_reported_primary_control_count: usize,
    /// Count of primary controls whose endpoint report is readable.
    pub endpoint_readable_primary_control_count: usize,
    /// Count of returned whole-asset candidates classified TooSubtle.
    pub returned_too_subtle_candidate_count: usize,
    /// Count of candidates with broken, floating, invalid, or non-conformant parts.
    pub broken_or_floating_part_count: usize,
    /// Whether package export and conformance checks passed.
    pub export_conformance_clean: bool,
    /// Whether the benchmark needed Advanced Recipe or another authoring lane.
    pub advanced_recipe_required: bool,
    /// Count of candidate summaries exposing raw technical terms.
    pub raw_technical_summary_count: usize,
}

impl StarterTemplateQualityEvidence {
    /// Return true when the evidence satisfies the starter-template quality bar.
    #[must_use]
    pub const fn passes_benchmark(self) -> bool {
        self.visible_idea_count >= 4
            && self.distinct_visible_idea_count >= 4
            && self.primary_control_count > 0
            && self.endpoint_reported_primary_control_count == self.primary_control_count
            && self.endpoint_readable_primary_control_count == self.primary_control_count
            && self.returned_too_subtle_candidate_count == 0
            && self.broken_or_floating_part_count == 0
            && self.export_conformance_clean
            && !self.advanced_recipe_required
            && self.raw_technical_summary_count == 0
    }
}

/// Convert starter-template benchmark evidence into catalog curation state.
///
/// Failed starters remain available for preview/developer review, but they may
/// not claim Usable or appear in the default novice catalog.
#[must_use]
pub const fn starter_template_curation_state_from_quality(
    evidence: StarterTemplateQualityEvidence,
) -> CatalogCurationState {
    if evidence.passes_benchmark() {
        CatalogCurationState::Usable
    } else {
        CatalogCurationState::PreviewOnly
    }
}

/// One JSON catalog payload with its exact content reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogSerializedEntry {
    /// Content reference.
    pub content_ref: CatalogContentRef,
    /// Human-facing label.
    pub label: String,
    /// Catalog tags.
    pub tags: Vec<String>,
    /// Canonical JSON payload.
    pub canonical_json: String,
}

/// Self-contained fixture catalog and foundry document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryFixtureCatalog {
    /// Fixture slug.
    pub slug: String,
    /// Catalog manifest version.
    #[serde(default = "default_catalog_version")]
    pub catalog_version: u32,
    /// Foundry document that references this catalog.
    pub document: FoundryAssetDocument,
    /// JSON entries keyed by content stable ID.
    pub entries: BTreeMap<String, FoundryCatalogSerializedEntry>,
}

impl FoundryFixtureCatalog {
    /// Return a manifest for this fixture catalog.
    #[must_use]
    pub fn manifest(&self) -> FoundryCatalogManifest {
        FoundryCatalogManifest {
            schema_version: FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id: format!("shape-lab-headless-{}", self.slug),
            catalog_version: self.catalog_version,
            entries: self
                .entries
                .iter()
                .map(|(id, entry)| {
                    (
                        id.clone(),
                        FoundryCatalogEntry {
                            content_ref: entry.content_ref.clone(),
                            label: entry.label.clone(),
                            tags: entry.tags.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    /// Write document, manifest, and catalog JSON payloads to a directory.
    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        write_json(dir.join("foundry-document.json"), &self.document)?;
        write_json(dir.join("catalog-manifest.json"), &self.manifest())?;
        for entry in self.entries.values() {
            fs::write(
                dir.join(format!("{}.json", entry.content_ref.stable_id)),
                &entry.canonical_json,
            )?;
        }
        Ok(())
    }
}

impl FoundryCatalogResolver for FoundryFixtureCatalog {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.entries
            .get(&content_ref.stable_id)
            .map(|entry| entry.canonical_json.clone())
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

/// Return every built-in headless fixture catalog.
#[must_use]
pub fn headless_fixture_catalogs() -> Vec<FoundryFixtureCatalog> {
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture)
        .collect()
}

/// Return every built-in headless fixture catalog with its product label.
#[must_use]
pub fn built_in_fixture_catalogs_with_labels() -> Vec<(&'static str, FoundryFixtureCatalog)> {
    vec![
        ("Box Primitive", box_primitive::fixture_catalog()),
        ("Lidded Box", box_primitive::lidded_box_fixture_catalog()),
    ]
}

/// Return product-catalog curation metadata for every built-in profile.
#[must_use]
pub fn built_in_catalog_curation_metadata() -> Vec<CatalogCurationMetadata> {
    vec![
        box_primitive::curation_metadata(),
        box_primitive::lidded_box_curation_metadata(),
    ]
}

/// Return curation metadata for one built-in profile slug.
#[must_use]
pub fn catalog_curation_metadata_for_slug(slug: &str) -> Option<CatalogCurationMetadata> {
    built_in_catalog_curation_metadata()
        .into_iter()
        .find(|metadata| metadata.profile_slug == slug)
}

/// Return built-in fixture catalogs visible to the app home catalog.
#[must_use]
pub fn curated_fixture_catalogs_with_labels(
    preview_catalog_enabled: bool,
) -> Vec<(&'static str, FoundryFixtureCatalog)> {
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .filter(|(_, fixture)| {
            catalog_curation_metadata_for_slug(&fixture.slug).is_some_and(|metadata| {
                if preview_catalog_enabled {
                    metadata.preview_catalog_visible()
                } else {
                    metadata.default_novice_visible()
                }
            })
        })
        .collect()
}

pub(crate) struct FixtureCatalogSpec {
    pub(crate) slug: &'static str,
    pub(crate) document_id: &'static str,
    pub(crate) family: AssetFamilySchema,
    pub(crate) style: StyleKit,
    pub(crate) family_implementation: FamilyImplementation,
    pub(crate) style_implementation: StyleImplementation,
    pub(crate) customizer_profile: CustomizerProfile,
    pub(crate) control_state: BTreeMap<String, ControlValue>,
}

fn build_fixture_catalog(spec: FixtureCatalogSpec) -> FoundryFixtureCatalog {
    let FixtureCatalogSpec {
        slug,
        document_id,
        family,
        style,
        family_implementation,
        style_implementation,
        customizer_profile,
        control_state,
    } = spec;
    let family_id = format!("{slug}-family");
    let style_id = format!("{slug}-style");
    let family_impl_id = format!("{slug}-family-impl");
    let style_impl_id = format!("{slug}-style-impl");
    let profile_id = format!("{slug}-profile");

    let entries = [
        catalog_entry(
            &family_id,
            ASSET_FAMILY_SCHEMA_VERSION,
            "Family",
            vec!["family".to_owned()],
            &family,
        ),
        catalog_entry(
            &style_id,
            STYLE_KIT_SCHEMA_VERSION,
            "Style",
            vec!["style".to_owned()],
            &style,
        ),
        catalog_entry(
            &family_impl_id,
            FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
            "Family implementation",
            vec!["implementation".to_owned()],
            &family_implementation,
        ),
        catalog_entry(
            &style_impl_id,
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            "Style implementation",
            vec!["implementation".to_owned()],
            &style_implementation,
        ),
        catalog_entry(
            &profile_id,
            CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            "Customizer profile",
            vec!["profile".to_owned()],
            &customizer_profile,
        ),
    ];
    let entries = entries
        .into_iter()
        .map(|entry| (entry.content_ref.stable_id.clone(), entry))
        .collect::<BTreeMap<_, _>>();

    let mut document = FoundryAssetDocument {
        schema_version: shape_foundry::FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
        document_id: FoundryDocumentId(document_id.to_owned()),
        family_content_ref: entries[&family_id].content_ref.clone(),
        style_content_ref: entries[&style_id].content_ref.clone(),
        family_implementation_ref: entries[&family_impl_id].content_ref.clone(),
        style_implementation_ref: entries[&style_impl_id].content_ref.clone(),
        customizer_profile_ref: entries[&profile_id].content_ref.clone(),
        control_state,
        provider_overrides: BTreeMap::new(),
        foundry_locks: Vec::new(),
        variation_state: shape_foundry::FoundryVariationState::default(),
        local_recipe_overrides: Vec::new(),
        seed: 42,
        catalog_lock: None,
        build_stamp: None,
    };
    document.catalog_lock = Some(FoundryCatalogLock {
        exact_refs: document_catalog_refs(&document),
        embedded_snapshots: Vec::new(),
        compiler_version: shape_foundry::SHAPE_FOUNDRY_CRATE_VERSION.to_owned(),
        catalog_version: 1,
    });

    // Keep all lock-key constants live in this crate; directory catalogs use the
    // same semantic keys as the compiler.
    let expected_keys = [
        CATALOG_LOCK_KEY_FAMILY,
        CATALOG_LOCK_KEY_STYLE,
        CATALOG_LOCK_KEY_FAMILY_IMPL,
        CATALOG_LOCK_KEY_STYLE_IMPL,
        CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE,
    ];
    debug_assert!(expected_keys.iter().all(|key| {
        document
            .catalog_lock
            .as_ref()
            .unwrap()
            .exact_refs
            .contains_key(*key)
    }));

    FoundryFixtureCatalog {
        slug: slug.to_owned(),
        catalog_version: 1,
        document,
        entries,
    }
}

fn default_catalog_version() -> u32 {
    1
}

fn catalog_entry<T: Serialize>(
    stable_id: &str,
    schema_version: u32,
    label: &str,
    tags: Vec<String>,
    value: &T,
) -> FoundryCatalogSerializedEntry {
    let canonical_json = serde_json::to_string(value).expect("fixture catalog JSON serializes");
    let fingerprint = catalog_content_fingerprint_from_json(stable_id, &canonical_json)
        .expect("fixture catalog content fingerprints");
    FoundryCatalogSerializedEntry {
        content_ref: CatalogContentRef {
            stable_id: stable_id.to_owned(),
            schema_version,
            fingerprint,
        },
        label: label.to_owned(),
        tags,
        canonical_json,
    }
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(path, json)
}

pub(crate) struct FamilySchemaSpec {
    pub(crate) id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) roles: Vec<PartRole>,
    pub(crate) allowed_operations: Vec<AllowedOperationKind>,
    pub(crate) parameter_slots: Vec<FamilyParameterSlot>,
    pub(crate) compatible_style_kits: Vec<String>,
    pub(crate) tags: Vec<String>,
}

fn family_schema(spec: FamilySchemaSpec) -> AssetFamilySchema {
    let FamilySchemaSpec {
        id,
        display_name,
        summary,
        roles,
        allowed_operations,
        parameter_slots,
        compatible_style_kits,
        tags,
    } = spec;
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        summary: summary.to_owned(),
        part_roles: roles,
        attachment_rules: Vec::new(),
        allowed_operations,
        parameter_slots,
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: vec![default_export_requirement()],
        compatible_style_kits,
        tags,
    }
}

fn default_export_requirement() -> shape_family::ExportRequirement {
    shape_family::ExportRequirement {
        profile: "canonical-model-package".to_owned(),
        required_metadata: vec![
            RuntimeMetadataRequirement::Pivot,
            RuntimeMetadataRequirement::SnapAnchors,
            RuntimeMetadataRequirement::Footprint,
            RuntimeMetadataRequirement::Previews,
        ],
        triangle_budget_hint: Some(250_000),
    }
}

fn role(id: &str, multiplicity: RoleMultiplicity, required: bool) -> PartRole {
    PartRole {
        id: id.to_owned(),
        display_name: id.replace('_', " "),
        required,
        multiplicity,
        provision: if required {
            RoleProvision::StyleRequired
        } else {
            RoleProvision::FamilyOrStyle
        },
        semantic_tags: vec![id.to_owned()],
    }
}

fn ratio_slot(
    id: &str,
    label: &str,
    role: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
    default: f32,
) -> FamilyParameterSlot {
    FamilyParameterSlot {
        id: id.to_owned(),
        label: label.to_owned(),
        target_role: Some(role.to_owned()),
        kind: FamilyParameterKind::Ratio,
        range: Some(ParameterRange {
            minimum,
            maximum,
            step,
        }),
        default_value: Some(FamilyDefaultValue::Scalar(default)),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: false,
    }
}

fn choice_slot(id: &str, label: &str, role: &str, choices: Vec<String>) -> FamilyParameterSlot {
    FamilyParameterSlot {
        id: id.to_owned(),
        label: label.to_owned(),
        target_role: Some(role.to_owned()),
        kind: FamilyParameterKind::Choice(choices.clone()),
        range: None,
        default_value: choices.first().cloned().map(FamilyDefaultValue::Choice),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: true,
    }
}

fn style_kit(
    id: &str,
    display_name: &str,
    family_id: &str,
    prototypes: &[(&str, &str, &str)],
    tags: Vec<String>,
) -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        compatible_families: vec![family_id.to_owned()],
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(0.035),
            segments: 1,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "hard_surface".to_owned(),
            allowed_profiles: vec!["box".to_owned(), "round".to_owned()],
            allow_asymmetry: false,
        },
        repetition: RepetitionPolicy {
            density: 0.65,
            preferred_spacing: LengthValue::FamilyUnits(0.25),
            maximum_default_count: 10,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: true,
            allowed_axes: vec!["x".to_owned(), "z".to_owned()],
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.25,
            detail: 0.35,
        },
        family_facets: BTreeMap::from([(
            family_id.to_owned(),
            FamilyStyleFacet {
                family_id: family_id.to_owned(),
                proportions: Vec::new(),
                part_prototypes: prototypes
                    .iter()
                    .map(|(id, label, role)| PartPrototype {
                        id: (*id).to_owned(),
                        display_name: (*label).to_owned(),
                        role: (*role).to_owned(),
                        operation_tags: vec![
                            AllowedOperationKind::Primitive,
                            AllowedOperationKind::Array,
                            AllowedOperationKind::Transform,
                            AllowedOperationKind::Bevel,
                        ],
                        style_tags: vec![id.replace('_', "-")],
                    })
                    .collect(),
                detail_modules: Vec::new(),
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags,
    }
}

fn customizer_profile(
    family_id: &str,
    style_id: &str,
    controls: Vec<CustomizerControl>,
) -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty(family_id, Some(style_id.to_owned()));
    profile.controls = controls;
    profile.candidate_strategies = vec![CandidateStrategy {
        id: "whole-model-directions".to_owned(),
        label: "Whole-model directions".to_owned(),
        control_ids: profile
            .controls
            .iter()
            .map(|control| control.id.clone())
            .collect(),
    }];
    profile
}

fn continuous_control(
    id: &str,
    label: &str,
    slot: &str,
    default: f32,
    minimum: f32,
    maximum: f32,
) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default },
        bindings: vec![ControlSlotBinding {
            slot: slot.to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval { minimum, maximum }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn choice_control(id: &str, label: &str, slot: &str, values: &[&str]) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ChoiceGallery {
            options: values
                .iter()
                .map(|value| ChoiceOption {
                    value: (*value).to_owned(),
                    label: value.replace('_', " "),
                    preview: WholeModelPreviewRef {
                        preview_id: format!("{id}-{value}"),
                        artifact_fingerprint: None,
                    },
                })
                .collect(),
        },
        bindings: vec![ControlSlotBinding {
            slot: slot.to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: values
                .iter()
                .map(|value| ControlValue::Choice((*value).to_owned()))
                .collect(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn family_implementation(
    family_id: &str,
    title: &str,
    parameter_bindings: Vec<ParameterBinding>,
) -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: family_id.to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), title),
        parameter_bindings,
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn style_implementation(
    style_id: &str,
    family_id: &str,
    default_role_providers: BTreeMap<String, String>,
    prototypes: Vec<RecipeFragment>,
) -> StyleImplementation {
    StyleImplementation {
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: style_id.to_owned(),
        family_id: family_id.to_owned(),
        default_role_providers,
        prototypes: prototypes
            .into_iter()
            .map(|fragment| (fragment.id.clone(), fragment))
            .collect(),
        detail_modules: BTreeMap::new(),
    }
}

fn rounded_box_fragment(
    id: &str,
    role: &str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
    operations: Vec<ModelingOperationSpec>,
) -> RecipeFragment {
    fragment(
        id,
        role,
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        },
        translation,
        operations,
        &[
            ("geometry.rounded_box.half_extents.x", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.half_extents.y", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.half_extents.z", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.radius", 0.0, 0.5, 0.01),
        ],
    )
}

fn fragment(
    id: &str,
    role: &str,
    source: GeometrySource,
    translation: [f32; 3],
    operations: Vec<ModelingOperationSpec>,
    scalar_paths: &[(&str, f32, f32, f32)],
) -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        PartDefinition {
            id: LOCAL_DEFINITION,
            name: format!("{id} definition"),
            tags: BTreeSet::from([role.to_owned(), format!("role:{role}")]),
            geometry: GeometryRecipe { source, operations },
            regions: BTreeMap::new(),
            sockets: BTreeMap::from([(
                SocketId(7),
                SocketSpec {
                    id: SocketId(7),
                    name: format!("{role} origin"),
                    local_frame: Frame3::default(),
                    role: role.to_owned(),
                    tags: BTreeSet::from([role.to_owned()]),
                },
            )]),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    recipe.instances.insert(
        LOCAL_INSTANCE,
        PartInstance {
            id: LOCAL_INSTANCE,
            definition: LOCAL_DEFINITION,
            name: format!("{id} {role}"),
            parent: None,
            local_transform: Transform3 {
                translation,
                ..Transform3::default()
            },
            attachment: None,
            enabled: true,
            tags: BTreeSet::from([role.to_owned(), format!("role:{role}")]),
            generated_by: None,
        },
    );
    recipe.root_instances.push(LOCAL_INSTANCE);
    for (index, (path, minimum, maximum, step)) in scalar_paths.iter().enumerate() {
        let parameter_id = (index + 1) as u64;
        recipe.parameters.insert(
            shape_asset::ParameterId(parameter_id),
            scalar_parameter(
                parameter_id,
                definition_scalar_path(LOCAL_DEFINITION, path),
                format!("{id} {path}"),
                *minimum,
                *maximum,
                *step,
                path.ends_with("radial_segments"),
            ),
        );
    }
    let next_operation = recipe
        .definitions
        .get(&LOCAL_DEFINITION)
        .expect("definition exists")
        .geometry
        .operations
        .iter()
        .map(ModelingOperationSpec::operation_id)
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        + 1;
    recipe.next_ids.part_definition = LOCAL_DEFINITION.0 + 1;
    recipe.next_ids.part_instance = LOCAL_INSTANCE.0 + 1;
    recipe.next_ids.parameter = scalar_paths.len() as u64 + 1;
    recipe.next_ids.operation = next_operation;
    recipe.next_ids.socket = 8;
    let validation_report = validate_asset_recipe(&recipe);
    assert!(
        validation_report.is_valid(),
        "fixture fragment {id} should validate: {validation_report:#?}"
    );
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: role.to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![LOCAL_INSTANCE],
            internal_roots: Vec::new(),
            socket_ports: vec![FragmentSocketPort {
                id: format!("{role}-origin"),
                local_occurrence_root: LOCAL_INSTANCE,
                local_socket: SocketId(7),
                compatibility_tags: vec![role.to_owned()],
            }],
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

/// Error while constructing a catalog lock from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCatalogManifestError {
    /// A requested content ID was not present in the manifest.
    UnknownContentId {
        /// Semantic lock key, such as `family`.
        lock_key: String,
        /// Missing content ID.
        content_id: String,
    },
}

impl FoundryCatalogManifest {
    /// Build an exact lock for selected `(lock_key, content_id)` pairs.
    ///
    /// Lock keys are semantic roles such as `family`, `style`, `family_impl`,
    /// `style_impl`, or `customizer_profile`; they are not content IDs.
    pub fn lock_selected(
        &self,
        selected_refs: impl IntoIterator<Item = (String, String)>,
        compiler_version: impl Into<String>,
    ) -> Result<FoundryCatalogLock, FoundryCatalogManifestError> {
        let mut exact_refs = BTreeMap::new();
        for (lock_key, content_id) in selected_refs {
            let Some(entry) = self.entries.get(&content_id) else {
                return Err(FoundryCatalogManifestError::UnknownContentId {
                    lock_key,
                    content_id,
                });
            };
            exact_refs.insert(lock_key, entry.content_ref.clone());
        }
        Ok(FoundryCatalogLock {
            exact_refs,
            embedded_snapshots: Vec::new(),
            compiler_version: compiler_version.into(),
            catalog_version: self.catalog_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_family_compile::identity::{CatalogContentFingerprint, ContentFingerprint};
    use shape_foundry::CatalogContentRef;

    #[test]
    fn lock_selected_uses_semantic_lock_keys() {
        let manifest = FoundryCatalogManifest {
            schema_version: FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id: "test-catalog".to_owned(),
            catalog_version: 4,
            entries: BTreeMap::from([(
                "box-family-content".to_owned(),
                FoundryCatalogEntry {
                    content_ref: content_ref("box-family-content", 1),
                    label: "Box Family".to_owned(),
                    tags: Vec::new(),
                },
            )]),
        };

        let lock = manifest
            .lock_selected(
                [("family".to_owned(), "box-family-content".to_owned())],
                "0.1.0",
            )
            .expect("lock should resolve");

        assert!(lock.exact_refs.contains_key("family"));
        assert!(!lock.exact_refs.contains_key("box-family-content"));
    }

    #[test]
    fn lock_selected_reports_unknown_content_ids() {
        let manifest = FoundryCatalogManifest {
            schema_version: FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id: "test-catalog".to_owned(),
            catalog_version: 4,
            entries: BTreeMap::new(),
        };

        assert_eq!(
            manifest.lock_selected([("family".to_owned(), "missing".to_owned())], "0.1.0"),
            Err(FoundryCatalogManifestError::UnknownContentId {
                lock_key: "family".to_owned(),
                content_id: "missing".to_owned(),
            })
        );
    }

    fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
        CatalogContentRef {
            stable_id: stable_id.to_owned(),
            schema_version: 1,
            fingerprint: CatalogContentFingerprint(ContentFingerprint([byte; 32])),
        }
    }

    #[test]
    fn headless_fixtures_compile_through_foundry() {
        for fixture in headless_fixture_catalogs() {
            let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
                .unwrap_or_else(|error| panic!("{} should compile: {error:#?}", fixture.slug));

            assert!(
                output.final_conformance.is_accepted(),
                "{} conformance should pass",
                fixture.slug
            );
            assert!(
                output.artifact.validation_report.is_valid(),
                "{} compile validation should pass",
                fixture.slug
            );
            let model_config = shape_compile::validation::validation_config_from_recipe_with_limits(
                &output.recipe,
                &output.artifact,
                shape_compile::validation::ValidationLimits::default(),
            );
            let model_report =
                shape_compile::validation::validate_model(&output.artifact, &model_config);
            assert!(
                model_report.is_valid(),
                "{} model validation should pass: {:#?}",
                fixture.slug,
                model_report.issues
            );
            assert!(
                !output.recipe.definitions.contains_key(&LOCAL_DEFINITION),
                "{} should remap fragment-local definition IDs",
                fixture.slug
            );
            assert!(
                !output.recipe.instances.contains_key(&LOCAL_INSTANCE),
                "{} should remap fragment-local instance IDs",
                fixture.slug
            );
            assert!(
                !output.final_conformance.roles.is_empty(),
                "{} should report role conformance",
                fixture.slug
            );
            assert!(
                output
                    .final_conformance
                    .exports
                    .iter()
                    .any(|row| row.profile == "canonical-model-package"
                        && row.status
                            == shape_family_compile::conformance::ConformanceStatus::Passed),
                "{} should pass export conformance",
                fixture.slug
            );
        }
    }

    #[test]
    fn choose_visible_fixtures_keep_default_parts_visually_connected() {
        let visible_slugs = BTreeSet::from(["box-primitive", "lidded-box"]);
        for fixture in headless_fixture_catalogs()
            .into_iter()
            .filter(|fixture| visible_slugs.contains(fixture.slug.as_str()))
        {
            let output = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
                .unwrap_or_else(|error| panic!("{} should compile: {error:#?}", fixture.slug));
            let disconnected_parts = visually_disconnected_parts(
                &output.artifact.compiled_parts,
                max_nearest_part_gap_for_fixture(&fixture.slug),
            );

            assert!(
                disconnected_parts.is_empty(),
                "{} should not have visibly disconnected default parts: {disconnected_parts:?}",
                fixture.slug
            );
        }
    }

    fn max_nearest_part_gap_for_fixture(slug: &str) -> f32 {
        let _ = slug;
        0.36
    }

    fn visually_disconnected_parts(
        parts: &[shape_compile::CompiledPart],
        max_nearest_part_gap: f32,
    ) -> Vec<(String, f32)> {
        let parts = parts
            .iter()
            .filter(|part| !part.world_mesh.bounds.is_empty())
            .collect::<Vec<_>>();
        if parts.len() <= 1 {
            return Vec::new();
        }

        parts
            .iter()
            .filter_map(|part| {
                let nearest_gap = parts
                    .iter()
                    .filter(|other| other.instance_id != part.instance_id)
                    .map(|other| {
                        bounds_gap(
                            part.world_mesh.bounds.min,
                            part.world_mesh.bounds.max,
                            other.world_mesh.bounds.min,
                            other.world_mesh.bounds.max,
                        )
                    })
                    .fold(f32::INFINITY, f32::min);
                (nearest_gap > max_nearest_part_gap)
                    .then(|| (part.instance_name.clone(), nearest_gap))
            })
            .collect()
    }

    fn bounds_gap(
        left_min: [f32; 3],
        left_max: [f32; 3],
        right_min: [f32; 3],
        right_max: [f32; 3],
    ) -> f32 {
        let dx = axis_gap(left_min[0], left_max[0], right_min[0], right_max[0]);
        let dy = axis_gap(left_min[1], left_max[1], right_min[1], right_max[1]);
        let dz = axis_gap(left_min[2], left_max[2], right_min[2], right_max[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn axis_gap(left_min: f32, left_max: f32, right_min: f32, right_max: f32) -> f32 {
        if left_max < right_min {
            right_min - left_max
        } else if right_max < left_min {
            left_min - right_max
        } else {
            0.0
        }
    }

    #[test]
    fn effective_family_request_contains_defaulted_slots() {
        let fixture = box_primitive::fixture_catalog();
        let mut document = fixture.document.clone();
        document.control_state.remove("proportions");

        let output = shape_foundry::compile_foundry_document(&document, &fixture)
            .expect("defaulted required control should compile");
        assert_eq!(
            output.family_request.parameters.get("proportions"),
            Some(&shape_family_compile::FamilyValue::Choice(
                "compact_box".to_owned()
            ))
        );
    }

    #[test]
    fn local_override_divergence_is_reported() {
        use shape_asset::{AssetEdit, AssetEditProgram};
        use shape_family_compile::identity::GeometryInputFingerprint;
        use shape_foundry::{
            LocalOverrideApplicationStatus, LocalRecipeOverride, LocalRecipeOverrideId,
            OverrideSurvivalPolicy, TouchedSemanticTarget,
        };

        let fixture = box_primitive::fixture_catalog();
        let base = shape_foundry::compile_foundry_document(&fixture.document, &fixture)
            .expect("base box should compile");
        let parameter = base
            .recipe
            .parameters
            .values()
            .find(|parameter| parameter.path.contains("rounded_box.radius"))
            .expect("edge softness parameter should survive remapping")
            .id;

        let mut document = fixture.document.clone();
        document.local_recipe_overrides.push(LocalRecipeOverride {
            id: LocalRecipeOverrideId("widen-body".to_owned()),
            base_geometry_fingerprint: GeometryInputFingerprint(ContentFingerprint([7; 32])),
            edit_program: AssetEditProgram {
                label: "widen body".to_owned(),
                seed: 11,
                operations: vec![AssetEdit::SetScalar {
                    parameter,
                    value: 0.12,
                }],
            },
            touched_targets: vec![TouchedSemanticTarget::Parameter(parameter)],
            survival_policy: OverrideSurvivalPolicy::Revalidate,
        });

        let output = shape_foundry::compile_foundry_document(&document, &fixture)
            .expect("revalidated override should compile");
        assert_eq!(output.local_override_reports.len(), 1);
        assert_eq!(
            output.local_override_reports[0].status,
            LocalOverrideApplicationStatus::Revalidated
        );
        assert_eq!(
            output.control_divergence.get("edge_softness"),
            Some(&shape_foundry::ControlDivergence::DivergedByOverride)
        );
        assert_eq!(output.local_override_divergence_reports.len(), 1);
        assert_eq!(
            output.local_override_divergence_reports[0]
                .diverged_controls
                .iter()
                .map(|control| control.control_id.as_str())
                .collect::<Vec<_>>(),
            vec!["edge_softness"]
        );
    }
}
