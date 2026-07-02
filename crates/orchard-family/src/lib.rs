#![forbid(unsafe_code)]

//! Theme-neutral asset-family and style-kit contracts.
//!
//! An asset family describes functional structure: roles, attachments,
//! parameters, constraints, variant rules, and export needs. A style kit
//! describes visual language that can be applied to compatible families.
//! Runtime-specific placement metadata belongs in adapter crates such as
//! downstream export packages, not here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de};

/// Current schema version for asset-family documents.
pub const ASSET_FAMILY_SCHEMA_VERSION: u32 = 3;

/// Current schema version for style-kit documents.
pub const STYLE_KIT_SCHEMA_VERSION: u32 = 4;
const LEGACY_STYLE_KIT_SCHEMA_VERSION: u32 = 3;

/// Theme-neutral functional grammar for one class of assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetFamilySchema {
    /// Asset-family schema version.
    pub schema_version: u32,
    /// Stable family identifier, such as `box_primitive`.
    pub id: String,
    /// Human-facing family name.
    pub display_name: String,
    /// Short description of the family contract.
    pub summary: String,
    /// Semantic roles that recipes in this family can contain.
    pub part_roles: Vec<PartRole>,
    /// Attachment rules between roles.
    pub attachment_rules: Vec<AttachmentRule>,
    /// Generic modeling operations this family may use.
    pub allowed_operations: Vec<AllowedOperationKind>,
    /// Parameter slots surfaced to novice workflows and search.
    pub parameter_slots: Vec<FamilyParameterSlot>,
    /// Geometric and semantic constraints.
    pub constraints: Vec<GeometricConstraint>,
    /// Variant-generation rules.
    pub variant_rules: Vec<VariantRule>,
    /// Optional export-profile requirements.
    pub export_requirements: Vec<ExportRequirement>,
    /// Style kits explicitly accepted by this family.
    pub compatible_style_kits: Vec<String>,
    /// Search and catalog tags.
    pub tags: Vec<String>,
}

/// One semantic part role in a family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartRole {
    /// Stable role identifier.
    pub id: String,
    /// Human-facing role name.
    pub display_name: String,
    /// Whether at least one occurrence is required.
    pub required: bool,
    /// Occurrence cardinality.
    pub multiplicity: RoleMultiplicity,
    /// How this role is expected to receive an executable provider.
    pub provision: RoleProvision,
    /// Functional tags, such as `body`, `edge`, or `primitive`.
    pub semantic_tags: Vec<String>,
}

/// Cardinality for a role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleMultiplicity {
    /// Exactly one occurrence.
    Single,
    /// Zero or one occurrence.
    Optional,
    /// Inclusive finite range.
    Range {
        /// Minimum occurrence count.
        min: u32,
        /// Maximum occurrence count.
        max: u32,
    },
    /// Any number of occurrences.
    Repeated,
}

/// How a family role is expected to be instantiated by executable bindings.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleProvision {
    /// The family implementation must provide a default fragment.
    FamilyDefault,
    /// The selected style kit must provide a prototype for this role.
    StyleRequired,
    /// Either the family implementation or selected style kit may provide it.
    FamilyOrStyle,
    /// The role is generated from other roles and does not need a direct provider.
    Derived,
}

/// Rule describing how roles can attach or depend on each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttachmentRule {
    /// Stable rule identifier.
    pub id: String,
    /// Source role.
    pub from_role: String,
    /// Destination role.
    pub to_role: String,
    /// Optional anchor role that mediates the relationship.
    pub anchor_role: Option<String>,
    /// Compatibility tags used by sockets, anchors, or adapters.
    pub compatibility_tags: Vec<String>,
    /// Whether the attachment is required for validity.
    pub required: bool,
    /// Whether this rule is enforced by the asset compiler, kept advisory, or deferred to runtime.
    pub execution_policy: FamilyRuleExecutionPolicy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentRuleWire {
    id: String,
    from_role: String,
    to_role: String,
    anchor_role: Option<String>,
    compatibility_tags: Vec<String>,
    required: bool,
    #[serde(default)]
    execution_policy: Option<FamilyRuleExecutionPolicy>,
}

impl<'de> Deserialize<'de> for AttachmentRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AttachmentRuleWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            from_role: wire.from_role,
            to_role: wire.to_role,
            anchor_role: wire.anchor_role,
            compatibility_tags: wire.compatibility_tags,
            required: wire.required,
            execution_policy: wire.execution_policy.unwrap_or_else(|| {
                FamilyRuleExecutionPolicy::from_required_attachment(wire.required)
            }),
        })
    }
}

/// Execution policy for family-level conformance rules.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FamilyRuleExecutionPolicy {
    /// The generated asset must satisfy this rule before it is accepted.
    Required,
    /// The rule is reported for quality and guidance but does not reject an asset.
    #[default]
    Advisory,
    /// The rule is intentionally deferred to a runtime/export adapter.
    RuntimeOnly,
}

impl FamilyRuleExecutionPolicy {
    fn from_required_attachment(required: bool) -> Self {
        if required {
            Self::Required
        } else {
            Self::Advisory
        }
    }
}

/// Theme-neutral modeling operation classes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AllowedOperationKind {
    /// Primitive part generation.
    Primitive,
    /// Analytic cut, such as panel, rectangular opening, or circular through-cut.
    Cut,
    /// Boundary or edge bevel treatment.
    Bevel,
    /// Repeated occurrences.
    Array,
    /// Transform-only structural edits.
    Transform,
    /// Sweep or path extrusion.
    Sweep,
    /// Lathe or surface of revolution.
    Lathe,
    /// Reserved loft/profile transition.
    LoftReserved,
    /// Reserved constrained constructive operation.
    BooleanReserved,
    /// Pack-authored extension key.
    Custom(String),
}

/// Search or inspector parameter slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyParameterSlot {
    /// Stable parameter key.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Optional role targeted by this parameter.
    pub target_role: Option<String>,
    /// Semantic parameter kind.
    pub kind: FamilyParameterKind,
    /// Optional numeric range.
    pub range: Option<ParameterRange>,
    /// Semantic default value used when an instantiation request omits this slot.
    pub default_value: Option<FamilyDefaultValue>,
    /// Whether this parameter must have executable compiler bindings.
    #[serde(default)]
    pub execution_policy: ParameterExecutionPolicy,
    /// Whether edits to this slot can change topology.
    pub topology_changing: bool,
}

/// Execution policy for a theme-neutral family parameter.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ParameterExecutionPolicy {
    /// The executable family implementation must bind this slot.
    #[default]
    RequiredBinding,
    /// The slot guides search or presentation but does not directly edit geometry yet.
    AdvisoryOnly,
    /// The slot is consumed by a runtime/export adapter rather than the asset compiler.
    RuntimeOnly,
}

/// Default value for a theme-neutral family parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FamilyDefaultValue {
    /// Floating-point scalar.
    Scalar(f32),
    /// Integer count.
    Integer(u32),
    /// Boolean toggle.
    Toggle(bool),
    /// Symbolic choice.
    Choice(String),
}

/// Theme-neutral parameter kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FamilyParameterKind {
    /// Length-like value.
    Length {
        /// Unit convention for this parameter.
        unit: LengthUnit,
    },
    /// Count-like value.
    Count,
    /// Unitless ratio.
    Ratio,
    /// Angle in radians unless the adapter says otherwise.
    Angle {
        /// Unit convention for this parameter.
        unit: AngleUnit,
    },
    /// Binary setting.
    Toggle,
    /// Closed set of symbolic choices.
    Choice(Vec<String>),
    /// Pack-authored parameter kind.
    Custom(String),
}

/// Length unit convention for family parameters and style policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LengthUnit {
    /// Meters in the exported model scale.
    Meters,
    /// Abstract family units interpreted by the family implementation.
    FamilyUnits,
    /// Ratio relative to another role.
    RelativeToRole {
        /// Reference role ID.
        role: String,
    },
}

/// Angle unit convention for family parameters.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AngleUnit {
    /// Radians.
    Radians,
}

/// Inclusive numeric range.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterRange {
    /// Minimum accepted value.
    pub minimum: f32,
    /// Maximum accepted value.
    pub maximum: f32,
    /// Suggested edit step.
    pub step: f32,
}

/// Generic geometry or semantic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometricConstraint {
    /// Stable constraint identifier.
    pub id: String,
    /// Role IDs governed by this constraint.
    pub roles: Vec<String>,
    /// Constraint class.
    pub kind: ConstraintKind,
    /// Whether this constraint is enforced by the asset compiler, kept advisory, or deferred.
    #[serde(default)]
    pub execution_policy: FamilyRuleExecutionPolicy,
}

/// Theme-neutral constraint classes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Stay within authored bounds.
    Bounds,
    /// Maintain clearance.
    Clearance,
    /// Roles must connect.
    MustConnect,
    /// Role must provide physical support.
    MustSupport,
    /// Role becomes walkable only when a runtime profile asks for it.
    WalkableIfRuntimeRequires,
    /// Pack-authored constraint key.
    Custom(String),
}

/// Variant-generation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantRule {
    /// Stable variant rule identifier.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Variant mode.
    pub mode: VariantMode,
    /// Roles this rule is allowed to edit.
    pub editable_roles: Vec<String>,
    /// Semantic tags that prevent edits when locked.
    pub locked_by_tags: Vec<String>,
}

/// Variant rule class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariantMode {
    /// Add, remove, or replace roles.
    Structural,
    /// Add or change small details.
    Detail,
    /// Change dimensions and proportions.
    Proportion,
    /// Change counts, spacing, or rhythm.
    Repetition,
    /// Pack-authored variant mode.
    Custom(String),
}

/// Export requirement for a destination profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequirement {
    /// Export profile key, such as `asset-pack` or `game-runtime`.
    pub profile: String,
    /// Runtime or packaging metadata expected by that profile.
    pub required_metadata: Vec<RuntimeMetadataRequirement>,
    /// Optional approximate triangle budget.
    pub triangle_budget_hint: Option<u32>,
}

/// Runtime/export metadata categories. These remain optional at the family layer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuntimeMetadataRequirement {
    /// Authored pivot.
    Pivot,
    /// Snap or attachment anchors.
    SnapAnchors,
    /// Logical footprint.
    Footprint,
    /// Walkable surface declarations.
    WalkableSurfaces,
    /// Support surface declarations.
    SupportSurfaces,
    /// Collision proxy declarations.
    CollisionProxies,
    /// Construction phase declarations.
    ConstructionPhases,
    /// Level-of-detail contract.
    Lod,
    /// Preview renders or thumbnails.
    Previews,
    /// Pack-authored metadata key.
    Custom(String),
}

/// Explicit length-like scalar used by style kits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LengthValue {
    /// Meters in exported model scale.
    Meters(f32),
    /// Abstract family units.
    FamilyUnits(f32),
    /// Ratio relative to another role.
    RelativeToRole {
        /// Reference role ID.
        role: String,
        /// Relative value.
        ratio: f32,
    },
}

/// Concrete visual language that can be applied to compatible families.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StyleKit {
    /// Style-kit schema version.
    pub schema_version: u32,
    /// Stable kit identifier.
    pub id: String,
    /// Human-facing kit name.
    pub display_name: String,
    /// Family IDs this kit can style.
    pub compatible_families: Vec<String>,
    /// Global bevel guidance.
    pub bevel_policy: BevelPolicy,
    /// Preferred profile and curve vocabulary.
    pub profile_language: ProfileLanguage,
    /// Repetition density and rhythm.
    pub repetition: RepetitionPolicy,
    /// Symmetry preferences.
    pub symmetry: SymmetryPolicy,
    /// Shape exaggeration preferences.
    pub exaggeration: ExaggerationPolicy,
    /// Family-scoped role vocabulary for every compatible family.
    #[serde(default)]
    pub family_facets: BTreeMap<String, FamilyStyleFacet>,
    /// Search and catalog tags.
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleKitWire {
    schema_version: u32,
    id: String,
    display_name: String,
    compatible_families: Vec<String>,
    #[serde(default)]
    proportions: Option<Vec<RoleProportion>>,
    bevel_policy: BevelPolicy,
    profile_language: ProfileLanguage,
    #[serde(default)]
    part_prototypes: Option<Vec<PartPrototype>>,
    #[serde(default)]
    detail_modules: Option<Vec<DetailModule>>,
    repetition: RepetitionPolicy,
    symmetry: SymmetryPolicy,
    exaggeration: ExaggerationPolicy,
    #[serde(default)]
    family_facets: BTreeMap<String, FamilyStyleFacet>,
    tags: Vec<String>,
}

impl<'de> Deserialize<'de> for StyleKit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StyleKitWire::deserialize(deserializer)?;
        migrate_style_kit_wire(wire).map_err(de::Error::custom)
    }
}

fn migrate_style_kit_wire(mut wire: StyleKitWire) -> Result<StyleKit, String> {
    match wire.schema_version {
        STYLE_KIT_SCHEMA_VERSION
            if wire.proportions.is_some()
                || wire.part_prototypes.is_some()
                || wire.detail_modules.is_some() =>
        {
            return Err(
                "style kit schema v4 must not contain legacy global role-scoped fields".to_owned(),
            );
        }
        STYLE_KIT_SCHEMA_VERSION => {}
        LEGACY_STYLE_KIT_SCHEMA_VERSION => {
            migrate_style_kit_v3_role_data(&mut wire)?;
            wire.schema_version = STYLE_KIT_SCHEMA_VERSION;
        }
        _ => {}
    }
    Ok(StyleKit {
        schema_version: wire.schema_version,
        id: wire.id,
        display_name: wire.display_name,
        compatible_families: wire.compatible_families,
        bevel_policy: wire.bevel_policy,
        profile_language: wire.profile_language,
        repetition: wire.repetition,
        symmetry: wire.symmetry,
        exaggeration: wire.exaggeration,
        family_facets: wire.family_facets,
        tags: wire.tags,
    })
}

fn migrate_style_kit_v3_role_data(wire: &mut StyleKitWire) -> Result<(), String> {
    let global_proportions = wire.proportions.take();
    let global_part_prototypes = wire.part_prototypes.take();
    let global_detail_modules = wire.detail_modules.take();
    let has_global_data = global_proportions
        .as_ref()
        .is_some_and(|values| !values.is_empty())
        || global_part_prototypes
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || global_detail_modules
            .as_ref()
            .is_some_and(|values| !values.is_empty());
    if !has_global_data {
        return Ok(());
    }
    if wire.compatible_families.len() != 1 {
        return Err(
            "style kit schema v3 global role-scoped data is ambiguous for multiple compatible families"
                .to_owned(),
        );
    }
    let family_id = wire.compatible_families[0].clone();
    let facet = wire
        .family_facets
        .entry(family_id.clone())
        .or_insert_with(|| FamilyStyleFacet {
            family_id: family_id.clone(),
            proportions: Vec::new(),
            part_prototypes: Vec::new(),
            detail_modules: Vec::new(),
            policy_overrides: FamilyStylePolicyOverrides::default(),
        });
    if facet.family_id != family_id {
        return Err("style kit schema v3 facet key and family_id disagree".to_owned());
    }
    if let Some(proportions) = global_proportions
        && !proportions.is_empty()
    {
        if !facet.proportions.is_empty() && facet.proportions != proportions {
            return Err("style kit schema v3 global and facet proportions disagree".to_owned());
        }
        if facet.proportions.is_empty() {
            facet.proportions = proportions;
        }
    }
    if let Some(part_prototypes) = global_part_prototypes
        && !part_prototypes.is_empty()
    {
        if !facet.part_prototypes.is_empty() && facet.part_prototypes != part_prototypes {
            return Err("style kit schema v3 global and facet part prototypes disagree".to_owned());
        }
        if facet.part_prototypes.is_empty() {
            facet.part_prototypes = part_prototypes;
        }
    }
    if let Some(detail_modules) = global_detail_modules
        && !detail_modules.is_empty()
    {
        if !facet.detail_modules.is_empty() && facet.detail_modules != detail_modules {
            return Err("style kit schema v3 global and facet detail modules disagree".to_owned());
        }
        if facet.detail_modules.is_empty() {
            facet.detail_modules = detail_modules;
        }
    }
    Ok(())
}

/// Family-scoped portion of a style kit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyStyleFacet {
    /// Family ID this facet styles.
    pub family_id: String,
    /// Per-role proportion guidance for this family.
    #[serde(default)]
    pub proportions: Vec<RoleProportion>,
    /// Concrete part prototypes exposed to this family.
    #[serde(default)]
    pub part_prototypes: Vec<PartPrototype>,
    /// Optional detail modules for this family.
    #[serde(default)]
    pub detail_modules: Vec<DetailModule>,
    /// Optional per-family overrides for global style policies.
    #[serde(default)]
    pub policy_overrides: FamilyStylePolicyOverrides,
}

/// Optional style-policy overrides scoped to a family facet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct FamilyStylePolicyOverrides {
    /// Family-local bevel override.
    #[serde(default)]
    pub bevel_policy: Option<BevelPolicy>,
    /// Family-local repetition override.
    #[serde(default)]
    pub repetition: Option<RepetitionPolicy>,
}

/// Per-role proportion guidance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleProportion {
    /// Target family role.
    pub role: String,
    /// Preferred width:depth:height scale with explicit units.
    pub preferred_scale: [LengthValue; 3],
    /// Optional taper from 0 to 1.
    pub taper: f32,
}

/// Global bevel guidance for a style kit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BevelPolicy {
    /// Preferred bevel width.
    pub width: LengthValue,
    /// Preferred bevel segment count.
    pub segments: u32,
    /// Normalized style profile from 0 to 1.
    pub profile: NormalizedBevelProfile,
}

/// Normalized bevel profile independent from engine-specific ranges.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedBevelProfile {
    /// Style profile from 0 to 1.
    pub normalized: f32,
}

/// Style-level profile vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileLanguage {
    /// Preferred curve family, such as `straight`, `rounded`, or `faceted`.
    pub curve_family: String,
    /// Allowed profile keys.
    pub allowed_profiles: Vec<String>,
    /// Whether asymmetric profiles are acceptable.
    pub allow_asymmetry: bool,
}

/// Concrete prototype for a semantic role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPrototype {
    /// Stable prototype identifier.
    pub id: String,
    /// Human-facing prototype name.
    pub display_name: String,
    /// Family role this prototype can satisfy.
    pub role: String,
    /// Generic operation vocabulary this prototype expects.
    pub operation_tags: Vec<AllowedOperationKind>,
    /// Style tags used by search.
    pub style_tags: Vec<String>,
}

/// Concrete detail module for one or more roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailModule {
    /// Stable detail identifier.
    pub id: String,
    /// Human-facing detail name.
    pub display_name: String,
    /// Roles this detail can decorate.
    pub target_roles: Vec<String>,
    /// Minimum rendered size where this detail remains readable.
    pub minimum_readability: ReadabilityThreshold,
    /// Detail tags used by search.
    pub tags: Vec<String>,
}

/// Pixel threshold tied to an authored camera profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadabilityThreshold {
    /// Minimum readable size in pixels.
    pub pixels: u32,
    /// Camera profile key used for the threshold.
    pub camera_profile: String,
}

/// Repetition policy for style modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepetitionPolicy {
    /// Preferred density from 0 to 1.
    pub density: f32,
    /// Preferred spacing.
    pub preferred_spacing: LengthValue,
    /// Maximum repeated count the kit should propose by default.
    pub maximum_default_count: u32,
}

/// Symmetry policy for a style kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymmetryPolicy {
    /// Whether mirrored layouts are preferred.
    pub prefer_mirrors: bool,
    /// Allowed local mirror axes, such as `x`, `y`, or `z`.
    pub allowed_axes: Vec<String>,
}

/// Shape exaggeration policy for a style kit.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExaggerationPolicy {
    /// Silhouette exaggeration from 0 to 1.
    pub silhouette: f32,
    /// Detail exaggeration from 0 to 1.
    pub detail: f32,
}

/// One validation issue from family or style-kit validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation report for family and style-kit contracts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FamilyValidationReport {
    /// Discovered issues.
    pub issues: Vec<FamilyValidationIssue>,
}

impl FamilyValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn extend_prefixed(&mut self, prefix: &str, nested: FamilyValidationReport) {
        for issue in nested.issues {
            self.issues.push(FamilyValidationIssue {
                subject: issue
                    .subject
                    .map(|subject| format!("{prefix}.{subject}"))
                    .or_else(|| Some(prefix.to_owned())),
                code: issue.code,
                message: issue.message,
            });
        }
    }
}

/// Validate a theme-neutral asset-family schema.
#[must_use]
pub fn validate_asset_family_schema(family: &AssetFamilySchema) -> FamilyValidationReport {
    let mut report = FamilyValidationReport::default();
    if family.schema_version != ASSET_FAMILY_SCHEMA_VERSION {
        push_issue(
            &mut report,
            Some("schema_version"),
            "unsupported_asset_family_schema",
            "Asset-family schema version is not supported.",
        );
    }
    validate_non_empty(&mut report, Some("id"), &family.id, "empty_family_id");
    validate_identifier(&mut report, Some("id"), &family.id, "invalid_family_id");
    validate_non_empty(
        &mut report,
        Some("display_name"),
        &family.display_name,
        "empty_family_display_name",
    );
    validate_non_empty(
        &mut report,
        Some("summary"),
        &family.summary,
        "empty_family_summary",
    );

    let role_ids = validate_part_roles(family, &mut report);
    validate_attachment_rules(family, &role_ids, &mut report);
    validate_allowed_operations(family, &mut report);
    validate_parameter_slots(family, &role_ids, &mut report);
    validate_constraints(family, &role_ids, &mut report);
    validate_variant_rules(family, &role_ids, &mut report);
    validate_export_requirements(family, &mut report);
    validate_unique_strings(
        &mut report,
        "compatible_style_kits",
        &family.compatible_style_kits,
        "duplicate_compatible_style_kit",
    );
    validate_identifier_list(
        &mut report,
        "compatible_style_kits",
        &family.compatible_style_kits,
        "invalid_compatible_style_kit",
    );
    validate_identifier_list(&mut report, "tags", &family.tags, "invalid_family_tag");
    validate_unique_strings(&mut report, "tags", &family.tags, "duplicate_family_tag");
    report
}

/// Validate a style-kit schema without assuming a specific family document.
#[must_use]
pub fn validate_style_kit(kit: &StyleKit) -> FamilyValidationReport {
    let mut report = FamilyValidationReport::default();
    if kit.schema_version != STYLE_KIT_SCHEMA_VERSION {
        push_issue(
            &mut report,
            Some("schema_version"),
            "unsupported_style_kit_schema",
            "Style-kit schema version is not supported.",
        );
    }
    validate_non_empty(&mut report, Some("id"), &kit.id, "empty_style_kit_id");
    validate_identifier(&mut report, Some("id"), &kit.id, "invalid_style_kit_id");
    validate_non_empty(
        &mut report,
        Some("display_name"),
        &kit.display_name,
        "empty_style_kit_display_name",
    );
    if kit.compatible_families.is_empty() {
        push_issue(
            &mut report,
            Some("compatible_families"),
            "missing_compatible_family",
            "Style kit must declare at least one compatible family.",
        );
    }
    validate_unique_strings(
        &mut report,
        "compatible_families",
        &kit.compatible_families,
        "duplicate_compatible_family",
    );
    validate_identifier_list(
        &mut report,
        "compatible_families",
        &kit.compatible_families,
        "invalid_compatible_family",
    );
    validate_bevel_policy(&kit.bevel_policy, &mut report);
    validate_profile_language(&kit.profile_language, &mut report);
    validate_repetition_policy(&kit.repetition, &mut report);
    validate_symmetry_policy(&kit.symmetry, &mut report);
    validate_exaggeration_policy(&kit.exaggeration, &mut report);
    validate_family_style_facets(kit, &mut report);
    validate_identifier_list(&mut report, "tags", &kit.tags, "invalid_style_kit_tag");
    validate_unique_strings(&mut report, "tags", &kit.tags, "duplicate_style_kit_tag");
    report
}

/// Validate that a style kit and asset family explicitly fit together.
#[must_use]
pub fn validate_family_style_compatibility(
    family: &AssetFamilySchema,
    kit: &StyleKit,
) -> FamilyValidationReport {
    let mut report = FamilyValidationReport::default();
    report.extend_prefixed("family", validate_asset_family_schema(family));
    report.extend_prefixed("style_kit", validate_style_kit(kit));

    if !family.compatible_style_kits.iter().any(|id| id == &kit.id) {
        push_issue(
            &mut report,
            Some("family.compatible_style_kits"),
            "style_kit_not_accepted_by_family",
            "Family must explicitly list the style kit as compatible.",
        );
    }
    if !kit.compatible_families.iter().any(|id| id == &family.id) {
        push_issue(
            &mut report,
            Some("style_kit.compatible_families"),
            "family_not_accepted_by_style_kit",
            "Style kit must explicitly list the family as compatible.",
        );
    }

    let role_ids = family
        .part_roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<BTreeSet<_>>();
    let Some(facet) = kit.family_facets.get(&family.id) else {
        push_issue(
            &mut report,
            Some(format!("style_kit.family_facets.{}", family.id)),
            "missing_family_style_facet",
            "Style kit must declare a family-scoped facet for the selected family.",
        );
        return report;
    };
    validate_kit_role_references(&family.id, facet, &role_ids, &mut report);
    validate_kit_operation_compatibility(family, &family.id, facet, &mut report);
    report
}

/// Validate that a mutually compatible family/style pair can satisfy required roles.
#[must_use]
pub fn validate_family_style_completeness(
    family: &AssetFamilySchema,
    kit: &StyleKit,
) -> FamilyValidationReport {
    let mut report = FamilyValidationReport::default();
    if let Some(facet) = kit.family_facets.get(&family.id) {
        validate_style_required_role_providers(family, &family.id, facet, &mut report);
    } else {
        push_issue(
            &mut report,
            Some(format!("style_kit.family_facets.{}", family.id)),
            "missing_family_style_facet",
            "Style kit must declare a family-scoped facet for the selected family.",
        );
    }
    report
}

fn validate_part_roles<'family>(
    family: &'family AssetFamilySchema,
    report: &mut FamilyValidationReport,
) -> BTreeSet<&'family str> {
    let mut role_ids = BTreeSet::new();
    if family.part_roles.is_empty() {
        push_issue(
            report,
            Some("part_roles"),
            "missing_part_role",
            "Asset family must declare at least one part role.",
        );
    }
    for (index, role) in family.part_roles.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("part_roles.{index}.id")),
            &role.id,
            "empty_part_role_id",
        );
        validate_identifier(
            report,
            Some(format!("part_roles.{index}.id")),
            &role.id,
            "invalid_part_role_id",
        );
        validate_non_empty(
            report,
            Some(format!("part_roles.{index}.display_name")),
            &role.display_name,
            "empty_part_role_display_name",
        );
        if !role_ids.insert(role.id.as_str()) {
            push_issue(
                report,
                Some(format!("part_roles.{index}.id")),
                "duplicate_part_role_id",
                "Part role IDs must be unique within one family.",
            );
        }
        if let RoleMultiplicity::Range { min, max } = role.multiplicity
            && min > max
        {
            push_issue(
                report,
                Some(format!("part_roles.{index}.multiplicity")),
                "invalid_role_multiplicity_range",
                "Role multiplicity minimum must not exceed maximum.",
            );
        }
        validate_role_requiredness(report, index, role);
        validate_identifier_list(
            report,
            &format!("part_roles.{index}.semantic_tags"),
            &role.semantic_tags,
            "invalid_part_role_tag",
        );
        validate_unique_strings(
            report,
            &format!("part_roles.{index}.semantic_tags"),
            &role.semantic_tags,
            "duplicate_part_role_tag",
        );
    }
    role_ids
}

fn validate_role_requiredness(report: &mut FamilyValidationReport, index: usize, role: &PartRole) {
    match (&role.multiplicity, role.required) {
        (RoleMultiplicity::Optional, true) => push_issue(
            report,
            Some(format!("part_roles.{index}.required")),
            "required_optional_role",
            "Required roles cannot use Optional multiplicity.",
        ),
        (RoleMultiplicity::Range { min: 0, .. }, true) => push_issue(
            report,
            Some(format!("part_roles.{index}.required")),
            "required_zero_minimum_role",
            "Required ranged roles must have a minimum greater than zero.",
        ),
        (RoleMultiplicity::Single, false) => push_issue(
            report,
            Some(format!("part_roles.{index}.required")),
            "optional_single_role",
            "Non-required roles cannot use Single multiplicity.",
        ),
        (RoleMultiplicity::Range { min, .. }, false) if *min > 0 => push_issue(
            report,
            Some(format!("part_roles.{index}.required")),
            "optional_positive_minimum_role",
            "Non-required ranged roles must allow zero occurrences.",
        ),
        _ => {}
    }
}

fn validate_attachment_rules(
    family: &AssetFamilySchema,
    role_ids: &BTreeSet<&str>,
    report: &mut FamilyValidationReport,
) {
    let mut rule_ids = BTreeSet::new();
    for (index, rule) in family.attachment_rules.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("attachment_rules.{index}.id")),
            &rule.id,
            "empty_attachment_rule_id",
        );
        validate_identifier(
            report,
            Some(format!("attachment_rules.{index}.id")),
            &rule.id,
            "invalid_attachment_rule_id",
        );
        if !rule_ids.insert(rule.id.as_str()) {
            push_issue(
                report,
                Some(format!("attachment_rules.{index}.id")),
                "duplicate_attachment_rule_id",
                "Attachment rule IDs must be unique within one family.",
            );
        }
        validate_role_reference(
            report,
            role_ids,
            format!("attachment_rules.{index}.from_role"),
            &rule.from_role,
            "unknown_attachment_from_role",
        );
        validate_identifier(
            report,
            Some(format!("attachment_rules.{index}.from_role")),
            &rule.from_role,
            "invalid_attachment_from_role",
        );
        validate_role_reference(
            report,
            role_ids,
            format!("attachment_rules.{index}.to_role"),
            &rule.to_role,
            "unknown_attachment_to_role",
        );
        validate_identifier(
            report,
            Some(format!("attachment_rules.{index}.to_role")),
            &rule.to_role,
            "invalid_attachment_to_role",
        );
        if let Some(anchor_role) = &rule.anchor_role {
            validate_role_reference(
                report,
                role_ids,
                format!("attachment_rules.{index}.anchor_role"),
                anchor_role,
                "unknown_attachment_anchor_role",
            );
            validate_identifier(
                report,
                Some(format!("attachment_rules.{index}.anchor_role")),
                anchor_role,
                "invalid_attachment_anchor_role",
            );
        }
        if rule.required && rule.compatibility_tags.is_empty() {
            push_issue(
                report,
                Some(format!("attachment_rules.{index}.compatibility_tags")),
                "missing_required_attachment_tag",
                "Required attachment rules must include at least one compatibility tag.",
            );
        }
        match (rule.required, rule.execution_policy) {
            (true, FamilyRuleExecutionPolicy::Required)
            | (
                false,
                FamilyRuleExecutionPolicy::Advisory | FamilyRuleExecutionPolicy::RuntimeOnly,
            ) => {}
            (
                true,
                FamilyRuleExecutionPolicy::Advisory | FamilyRuleExecutionPolicy::RuntimeOnly,
            ) => {
                push_issue(
                    report,
                    Some(format!("attachment_rules.{index}.execution_policy")),
                    "required_attachment_policy_mismatch",
                    "Required attachment rules must use Required execution policy.",
                );
            }
            (false, FamilyRuleExecutionPolicy::Required) => {
                push_issue(
                    report,
                    Some(format!("attachment_rules.{index}.execution_policy")),
                    "optional_attachment_policy_mismatch",
                    "Optional attachment rules must not use Required execution policy.",
                );
            }
        }
        validate_identifier_list(
            report,
            &format!("attachment_rules.{index}.compatibility_tags"),
            &rule.compatibility_tags,
            "invalid_attachment_compatibility_tag",
        );
        validate_unique_strings(
            report,
            &format!("attachment_rules.{index}.compatibility_tags"),
            &rule.compatibility_tags,
            "duplicate_attachment_compatibility_tag",
        );
    }
}

fn validate_allowed_operations(family: &AssetFamilySchema, report: &mut FamilyValidationReport) {
    if family.allowed_operations.is_empty() {
        push_issue(
            report,
            Some("allowed_operations"),
            "missing_allowed_operation",
            "Asset family must declare at least one allowed operation.",
        );
    }
    let mut seen = BTreeSet::new();
    for (index, operation) in family.allowed_operations.iter().enumerate() {
        if !seen.insert(operation) {
            push_issue(
                report,
                Some(format!("allowed_operations.{index}")),
                "duplicate_allowed_operation",
                "Allowed operations must be unique within one family.",
            );
        }
        if let AllowedOperationKind::Custom(key) = operation {
            validate_identifier(
                report,
                Some(format!("allowed_operations.{index}")),
                key,
                "invalid_custom_allowed_operation",
            );
        }
    }
}

fn validate_parameter_slots(
    family: &AssetFamilySchema,
    role_ids: &BTreeSet<&str>,
    report: &mut FamilyValidationReport,
) {
    let mut ids = BTreeSet::new();
    for (index, slot) in family.parameter_slots.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("parameter_slots.{index}.id")),
            &slot.id,
            "empty_parameter_slot_id",
        );
        validate_identifier(
            report,
            Some(format!("parameter_slots.{index}.id")),
            &slot.id,
            "invalid_parameter_slot_id",
        );
        validate_non_empty(
            report,
            Some(format!("parameter_slots.{index}.label")),
            &slot.label,
            "empty_parameter_slot_label",
        );
        if !ids.insert(slot.id.as_str()) {
            push_issue(
                report,
                Some(format!("parameter_slots.{index}.id")),
                "duplicate_parameter_slot_id",
                "Parameter slot IDs must be unique within one family.",
            );
        }
        if let Some(role) = &slot.target_role {
            validate_role_reference(
                report,
                role_ids,
                format!("parameter_slots.{index}.target_role"),
                role,
                "unknown_parameter_target_role",
            );
            validate_identifier(
                report,
                Some(format!("parameter_slots.{index}.target_role")),
                role,
                "invalid_parameter_target_role",
            );
        }
        validate_parameter_kind(report, role_ids, index, slot);
        validate_parameter_default(report, index, slot);
    }
}

fn validate_parameter_kind(
    report: &mut FamilyValidationReport,
    role_ids: &BTreeSet<&str>,
    index: usize,
    slot: &FamilyParameterSlot,
) {
    match &slot.kind {
        FamilyParameterKind::Length { unit } => {
            validate_length_unit(
                report,
                Some(format!("parameter_slots.{index}.kind.unit")),
                unit,
            );
            validate_length_unit_role_reference(
                report,
                role_ids,
                format!("parameter_slots.{index}.kind.unit"),
                unit,
                "unknown_relative_length_unit_role",
            );
            require_range(report, index, slot.range, "missing_length_parameter_range");
            if let Some(range) = slot.range {
                validate_parameter_range(
                    report,
                    Some(format!("parameter_slots.{index}.range")),
                    range,
                );
                validate_positive_parameter_range(
                    report,
                    index,
                    range,
                    "non_positive_length_parameter_range",
                    "Length parameter ranges must be greater than zero.",
                );
            }
        }
        FamilyParameterKind::Count => {
            require_range(report, index, slot.range, "missing_count_parameter_range");
            if let Some(range) = slot.range {
                validate_parameter_range(
                    report,
                    Some(format!("parameter_slots.{index}.range")),
                    range,
                );
                validate_integral_parameter_range(report, index, range);
                validate_positive_parameter_range(
                    report,
                    index,
                    range,
                    "non_positive_count_parameter_range",
                    "Count parameter ranges must be greater than zero.",
                );
            }
        }
        FamilyParameterKind::Ratio => {
            require_range(report, index, slot.range, "missing_ratio_parameter_range");
            if let Some(range) = slot.range {
                validate_parameter_range(
                    report,
                    Some(format!("parameter_slots.{index}.range")),
                    range,
                );
                if range.minimum < 0.0 || range.maximum > 1.0 {
                    push_issue(
                        report,
                        Some(format!("parameter_slots.{index}.range")),
                        "ratio_parameter_range_outside_unit_interval",
                        "Ratio parameter ranges must stay within 0..1.",
                    );
                }
            }
        }
        FamilyParameterKind::Angle { unit } => {
            validate_angle_unit(
                report,
                Some(format!("parameter_slots.{index}.kind.unit")),
                *unit,
            );
            require_range(report, index, slot.range, "missing_angle_parameter_range");
            if let Some(range) = slot.range {
                validate_parameter_range(
                    report,
                    Some(format!("parameter_slots.{index}.range")),
                    range,
                );
            }
        }
        FamilyParameterKind::Toggle => {
            reject_range(report, index, slot.range, "toggle_parameter_has_range");
        }
        FamilyParameterKind::Choice(choices) => {
            reject_range(report, index, slot.range, "choice_parameter_has_range");
            if choices.is_empty() {
                push_issue(
                    report,
                    Some(format!("parameter_slots.{index}.kind")),
                    "empty_parameter_choice_set",
                    "Choice parameters must include at least one option.",
                );
            }
            validate_unique_strings(
                report,
                &format!("parameter_slots.{index}.kind.choices"),
                choices,
                "duplicate_parameter_choice",
            );
            validate_identifier_list(
                report,
                &format!("parameter_slots.{index}.kind.choices"),
                choices,
                "invalid_parameter_choice",
            );
        }
        FamilyParameterKind::Custom(key) => {
            validate_identifier(
                report,
                Some(format!("parameter_slots.{index}.kind")),
                key,
                "invalid_custom_parameter_kind",
            );
            if let Some(range) = slot.range {
                validate_parameter_range(
                    report,
                    Some(format!("parameter_slots.{index}.range")),
                    range,
                );
            }
        }
    }
}

fn require_range(
    report: &mut FamilyValidationReport,
    index: usize,
    range: Option<ParameterRange>,
    code: &'static str,
) {
    if range.is_none() {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.range")),
            code,
            "This parameter kind requires an explicit numeric range.",
        );
    }
}

fn reject_range(
    report: &mut FamilyValidationReport,
    index: usize,
    range: Option<ParameterRange>,
    code: &'static str,
) {
    if range.is_some() {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.range")),
            code,
            "This parameter kind must not declare a numeric range.",
        );
    }
}

fn validate_integral_parameter_range(
    report: &mut FamilyValidationReport,
    index: usize,
    range: ParameterRange,
) {
    if !is_integral(range.minimum) || !is_integral(range.maximum) || !is_integral(range.step) {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.range")),
            "non_integral_count_parameter_range",
            "Count parameter minimum, maximum, and step must be integral.",
        );
    }
}

fn validate_positive_parameter_range(
    report: &mut FamilyValidationReport,
    index: usize,
    range: ParameterRange,
    code: &'static str,
    message: &'static str,
) {
    if range.minimum <= 0.0 || range.maximum <= 0.0 || range.step <= 0.0 {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.range")),
            code,
            message,
        );
    }
}

fn is_integral(value: f32) -> bool {
    value.is_finite() && (value.fract().abs() <= f32::EPSILON)
}

fn validate_parameter_default(
    report: &mut FamilyValidationReport,
    index: usize,
    slot: &FamilyParameterSlot,
) {
    let Some(default_value) = &slot.default_value else {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.default_value")),
            "missing_parameter_default",
            "Parameter slots must declare a semantic default value.",
        );
        return;
    };
    match (&slot.kind, default_value) {
        (
            FamilyParameterKind::Length { .. } | FamilyParameterKind::Angle { .. },
            FamilyDefaultValue::Scalar(value),
        ) => validate_default_numeric(report, index, slot, *value),
        (FamilyParameterKind::Ratio, FamilyDefaultValue::Scalar(value)) => {
            validate_default_numeric(report, index, slot, *value);
            if !(0.0..=1.0).contains(value) {
                push_issue(
                    report,
                    Some(format!("parameter_slots.{index}.default_value")),
                    "default_ratio_out_of_range",
                    "Ratio defaults must stay within 0..1.",
                );
            }
        }
        (FamilyParameterKind::Count, FamilyDefaultValue::Integer(value)) => {
            validate_default_numeric(report, index, slot, *value as f32);
        }
        (FamilyParameterKind::Toggle, FamilyDefaultValue::Toggle(_)) => {}
        (FamilyParameterKind::Choice(choices), FamilyDefaultValue::Choice(value)) => {
            if !choices.iter().any(|choice| choice == value) {
                push_issue(
                    report,
                    Some(format!("parameter_slots.{index}.default_value")),
                    "default_choice_not_declared",
                    "Choice defaults must be declared by the parameter slot.",
                );
            }
        }
        (FamilyParameterKind::Custom(_), FamilyDefaultValue::Scalar(value)) => {
            validate_default_numeric(report, index, slot, *value);
        }
        (FamilyParameterKind::Custom(_), FamilyDefaultValue::Integer(value)) => {
            validate_default_numeric(report, index, slot, *value as f32);
        }
        _ => push_issue(
            report,
            Some(format!("parameter_slots.{index}.default_value")),
            "default_parameter_type_mismatch",
            "Default parameter value type must match the family parameter kind.",
        ),
    }
}

fn validate_default_numeric(
    report: &mut FamilyValidationReport,
    index: usize,
    slot: &FamilyParameterSlot,
    value: f32,
) {
    if !value.is_finite() {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.default_value")),
            "default_parameter_non_finite",
            "Default numeric values must be finite.",
        );
        return;
    }
    if let Some(range) = slot.range
        && (value < range.minimum || value > range.maximum)
    {
        push_issue(
            report,
            Some(format!("parameter_slots.{index}.default_value")),
            "default_parameter_out_of_range",
            "Default parameter value must fall within the parameter range.",
        );
    }
}

fn validate_constraints(
    family: &AssetFamilySchema,
    role_ids: &BTreeSet<&str>,
    report: &mut FamilyValidationReport,
) {
    let mut ids = BTreeSet::new();
    for (index, constraint) in family.constraints.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("constraints.{index}.id")),
            &constraint.id,
            "empty_constraint_id",
        );
        validate_identifier(
            report,
            Some(format!("constraints.{index}.id")),
            &constraint.id,
            "invalid_constraint_id",
        );
        if !ids.insert(constraint.id.as_str()) {
            push_issue(
                report,
                Some(format!("constraints.{index}.id")),
                "duplicate_constraint_id",
                "Constraint IDs must be unique within one family.",
            );
        }
        if constraint.roles.is_empty() {
            push_issue(
                report,
                Some(format!("constraints.{index}.roles")),
                "missing_constraint_role",
                "Constraints must reference at least one role.",
            );
        }
        for role in &constraint.roles {
            validate_role_reference(
                report,
                role_ids,
                format!("constraints.{index}.roles"),
                role,
                "unknown_constraint_role",
            );
            validate_identifier(
                report,
                Some(format!("constraints.{index}.roles")),
                role,
                "invalid_constraint_role",
            );
        }
        validate_unique_strings(
            report,
            &format!("constraints.{index}.roles"),
            &constraint.roles,
            "duplicate_constraint_role",
        );
        if let ConstraintKind::Custom(key) = &constraint.kind {
            validate_identifier(
                report,
                Some(format!("constraints.{index}.kind")),
                key,
                "invalid_custom_constraint_kind",
            );
        }
    }
}

fn validate_variant_rules(
    family: &AssetFamilySchema,
    role_ids: &BTreeSet<&str>,
    report: &mut FamilyValidationReport,
) {
    let mut ids = BTreeSet::new();
    for (index, rule) in family.variant_rules.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("variant_rules.{index}.id")),
            &rule.id,
            "empty_variant_rule_id",
        );
        validate_identifier(
            report,
            Some(format!("variant_rules.{index}.id")),
            &rule.id,
            "invalid_variant_rule_id",
        );
        validate_non_empty(
            report,
            Some(format!("variant_rules.{index}.label")),
            &rule.label,
            "empty_variant_rule_label",
        );
        if !ids.insert(rule.id.as_str()) {
            push_issue(
                report,
                Some(format!("variant_rules.{index}.id")),
                "duplicate_variant_rule_id",
                "Variant rule IDs must be unique within one family.",
            );
        }
        if rule.editable_roles.is_empty() {
            push_issue(
                report,
                Some(format!("variant_rules.{index}.editable_roles")),
                "missing_variant_editable_role",
                "Variant rules must declare at least one editable role.",
            );
        }
        for role in &rule.editable_roles {
            validate_role_reference(
                report,
                role_ids,
                format!("variant_rules.{index}.editable_roles"),
                role,
                "unknown_variant_editable_role",
            );
            validate_identifier(
                report,
                Some(format!("variant_rules.{index}.editable_roles")),
                role,
                "invalid_variant_editable_role",
            );
        }
        validate_unique_strings(
            report,
            &format!("variant_rules.{index}.editable_roles"),
            &rule.editable_roles,
            "duplicate_variant_editable_role",
        );
        validate_identifier_list(
            report,
            &format!("variant_rules.{index}.locked_by_tags"),
            &rule.locked_by_tags,
            "invalid_variant_lock_tag",
        );
        validate_unique_strings(
            report,
            &format!("variant_rules.{index}.locked_by_tags"),
            &rule.locked_by_tags,
            "duplicate_variant_lock_tag",
        );
        if let VariantMode::Custom(key) = &rule.mode {
            validate_identifier(
                report,
                Some(format!("variant_rules.{index}.mode")),
                key,
                "invalid_custom_variant_mode",
            );
        }
    }
}

fn validate_export_requirements(family: &AssetFamilySchema, report: &mut FamilyValidationReport) {
    let mut profiles = BTreeMap::<&str, usize>::new();
    for (index, requirement) in family.export_requirements.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("export_requirements.{index}.profile")),
            &requirement.profile,
            "empty_export_profile",
        );
        validate_identifier(
            report,
            Some(format!("export_requirements.{index}.profile")),
            &requirement.profile,
            "invalid_export_profile",
        );
        if let Some(previous_index) = profiles.insert(requirement.profile.as_str(), index) {
            push_issue(
                report,
                Some(format!("export_requirements.{index}.profile")),
                "duplicate_export_profile",
                format!("Export profile is already used at index {previous_index}."),
            );
        }
        if requirement.required_metadata.is_empty() {
            push_issue(
                report,
                Some(format!("export_requirements.{index}.required_metadata")),
                "missing_export_metadata_requirement",
                "Export requirement entries must request at least one metadata category.",
            );
        }
        if matches!(requirement.triangle_budget_hint, Some(0)) {
            push_issue(
                report,
                Some(format!("export_requirements.{index}.triangle_budget_hint")),
                "invalid_triangle_budget_hint",
                "Triangle budget hints must be greater than zero when present.",
            );
        }
        validate_runtime_metadata_requirements(report, index, &requirement.required_metadata);
    }
}

fn validate_runtime_metadata_requirements(
    report: &mut FamilyValidationReport,
    export_index: usize,
    requirements: &[RuntimeMetadataRequirement],
) {
    let mut seen = BTreeSet::new();
    for (index, requirement) in requirements.iter().enumerate() {
        if !seen.insert(requirement) {
            push_issue(
                report,
                Some(format!(
                    "export_requirements.{export_index}.required_metadata.{index}"
                )),
                "duplicate_runtime_metadata_requirement",
                "Runtime metadata requirements must be unique within one export profile.",
            );
        }
        if let RuntimeMetadataRequirement::Custom(key) = requirement {
            validate_identifier(
                report,
                Some(format!(
                    "export_requirements.{export_index}.required_metadata.{index}"
                )),
                key,
                "invalid_custom_runtime_metadata_requirement",
            );
        }
    }
}

fn validate_role_proportion_list(
    proportions: &[RoleProportion],
    subject: &str,
    report: &mut FamilyValidationReport,
) {
    let mut roles = BTreeSet::new();
    for (index, proportion) in proportions.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.role")),
            &proportion.role,
            "empty_proportion_role",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.{index}.role")),
            &proportion.role,
            "invalid_proportion_role",
        );
        if !roles.insert(proportion.role.as_str()) {
            push_issue(
                report,
                Some(format!("{subject}.{index}.role")),
                "duplicate_role_proportion",
                "A style kit can declare at most one proportion policy per role.",
            );
        }
        for (axis, value) in proportion.preferred_scale.iter().enumerate() {
            validate_length_value(
                report,
                Some(format!("{subject}.{index}.preferred_scale.{axis}")),
                value,
                "invalid_role_preferred_scale",
            );
        }
        validate_fraction(
            report,
            Some(format!("{subject}.{index}.taper")),
            proportion.taper,
            "invalid_role_taper",
        );
    }
}

fn validate_bevel_policy(policy: &BevelPolicy, report: &mut FamilyValidationReport) {
    validate_bevel_policy_scoped(policy, "bevel_policy", report);
}

fn validate_bevel_policy_scoped(
    policy: &BevelPolicy,
    subject: &str,
    report: &mut FamilyValidationReport,
) {
    validate_global_length_value(
        report,
        Some(format!("{subject}.width")),
        &policy.width,
        "invalid_bevel_width",
    );
    if policy.segments == 0 {
        push_issue(
            report,
            Some(format!("{subject}.segments")),
            "invalid_bevel_segments",
            "Bevel policy must request at least one segment.",
        );
    }
    validate_fraction(
        report,
        Some(format!("{subject}.profile.normalized")),
        policy.profile.normalized,
        "invalid_bevel_profile",
    );
}

fn validate_profile_language(language: &ProfileLanguage, report: &mut FamilyValidationReport) {
    validate_non_empty(
        report,
        Some("profile_language.curve_family"),
        &language.curve_family,
        "empty_profile_curve_family",
    );
    validate_identifier(
        report,
        Some("profile_language.curve_family"),
        &language.curve_family,
        "invalid_profile_curve_family",
    );
    if language.allowed_profiles.is_empty() {
        push_issue(
            report,
            Some("profile_language.allowed_profiles"),
            "missing_allowed_profile",
            "Style kit must include at least one allowed profile key.",
        );
    }
    validate_unique_strings(
        report,
        "profile_language.allowed_profiles",
        &language.allowed_profiles,
        "duplicate_allowed_profile",
    );
    validate_identifier_list(
        report,
        "profile_language.allowed_profiles",
        &language.allowed_profiles,
        "invalid_allowed_profile",
    );
}

fn validate_part_prototype_list(
    prototypes: &[PartPrototype],
    subject: &str,
    report: &mut FamilyValidationReport,
) {
    let mut ids = BTreeSet::new();
    for (index, prototype) in prototypes.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.id")),
            &prototype.id,
            "empty_part_prototype_id",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.{index}.id")),
            &prototype.id,
            "invalid_part_prototype_id",
        );
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.display_name")),
            &prototype.display_name,
            "empty_part_prototype_display_name",
        );
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.role")),
            &prototype.role,
            "empty_part_prototype_role",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.{index}.role")),
            &prototype.role,
            "invalid_part_prototype_role",
        );
        if !ids.insert(prototype.id.as_str()) {
            push_issue(
                report,
                Some(format!("{subject}.{index}.id")),
                "duplicate_part_prototype_id",
                "Part prototype IDs must be unique within one style kit.",
            );
        }
        if prototype.operation_tags.is_empty() {
            push_issue(
                report,
                Some(format!("{subject}.{index}.operation_tags")),
                "missing_part_prototype_operation_tag",
                "Part prototypes must declare at least one expected operation class.",
            );
        }
        validate_operation_tags(report, subject, index, &prototype.operation_tags);
        validate_identifier_list(
            report,
            &format!("{subject}.{index}.style_tags"),
            &prototype.style_tags,
            "invalid_part_prototype_style_tag",
        );
        validate_unique_strings(
            report,
            &format!("{subject}.{index}.style_tags"),
            &prototype.style_tags,
            "duplicate_part_prototype_style_tag",
        );
    }
}

fn validate_operation_tags(
    report: &mut FamilyValidationReport,
    subject: &str,
    prototype_index: usize,
    operations: &[AllowedOperationKind],
) {
    let mut seen = BTreeSet::new();
    for (index, operation) in operations.iter().enumerate() {
        if !seen.insert(operation) {
            push_issue(
                report,
                Some(format!(
                    "{subject}.{prototype_index}.operation_tags.{index}"
                )),
                "duplicate_part_prototype_operation_tag",
                "Part prototype operation tags must be unique.",
            );
        }
        if let AllowedOperationKind::Custom(key) = operation {
            validate_identifier(
                report,
                Some(format!(
                    "{subject}.{prototype_index}.operation_tags.{index}"
                )),
                key,
                "invalid_custom_operation_tag",
            );
        }
    }
}

fn validate_detail_module_list(
    modules: &[DetailModule],
    subject: &str,
    report: &mut FamilyValidationReport,
) {
    let mut ids = BTreeSet::new();
    for (index, module) in modules.iter().enumerate() {
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.id")),
            &module.id,
            "empty_detail_module_id",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.{index}.id")),
            &module.id,
            "invalid_detail_module_id",
        );
        validate_non_empty(
            report,
            Some(format!("{subject}.{index}.display_name")),
            &module.display_name,
            "empty_detail_module_display_name",
        );
        if !ids.insert(module.id.as_str()) {
            push_issue(
                report,
                Some(format!("{subject}.{index}.id")),
                "duplicate_detail_module_id",
                "Detail module IDs must be unique within one style kit.",
            );
        }
        if module.target_roles.is_empty() {
            push_issue(
                report,
                Some(format!("{subject}.{index}.target_roles")),
                "missing_detail_target_role",
                "Detail modules must target at least one role.",
            );
        }
        validate_identifier_list(
            report,
            &format!("{subject}.{index}.target_roles"),
            &module.target_roles,
            "invalid_detail_target_role",
        );
        validate_unique_strings(
            report,
            &format!("{subject}.{index}.target_roles"),
            &module.target_roles,
            "duplicate_detail_target_role",
        );
        validate_readability_threshold(
            report,
            Some(format!("{subject}.{index}.minimum_readability")),
            &module.minimum_readability,
        );
        if module.minimum_readability.pixels == 0 {
            push_issue(
                report,
                Some(format!("{subject}.{index}.minimum_readability.pixels")),
                "invalid_detail_minimum_readability",
                "Detail modules must declare a positive readability threshold.",
            );
        }
        validate_identifier_list(
            report,
            &format!("{subject}.{index}.tags"),
            &module.tags,
            "invalid_detail_tag",
        );
        validate_unique_strings(
            report,
            &format!("{subject}.{index}.tags"),
            &module.tags,
            "duplicate_detail_tag",
        );
    }
}

fn validate_repetition_policy(policy: &RepetitionPolicy, report: &mut FamilyValidationReport) {
    validate_repetition_policy_scoped(policy, "repetition", report);
}

fn validate_repetition_policy_scoped(
    policy: &RepetitionPolicy,
    subject: &str,
    report: &mut FamilyValidationReport,
) {
    validate_fraction(
        report,
        Some(format!("{subject}.density")),
        policy.density,
        "invalid_repetition_density",
    );
    validate_global_length_value(
        report,
        Some(format!("{subject}.preferred_spacing")),
        &policy.preferred_spacing,
        "invalid_repetition_spacing",
    );
    if policy.maximum_default_count == 0 {
        push_issue(
            report,
            Some(format!("{subject}.maximum_default_count")),
            "invalid_repetition_count",
            "Maximum default repetition count must be greater than zero.",
        );
    }
}

fn validate_symmetry_policy(policy: &SymmetryPolicy, report: &mut FamilyValidationReport) {
    if policy.prefer_mirrors && policy.allowed_axes.is_empty() {
        push_issue(
            report,
            Some("symmetry.allowed_axes"),
            "missing_symmetry_axis",
            "Mirror-preferring style kits must declare at least one allowed axis.",
        );
    }
    validate_unique_strings(
        report,
        "symmetry.allowed_axes",
        &policy.allowed_axes,
        "duplicate_symmetry_axis",
    );
    validate_identifier_list(
        report,
        "symmetry.allowed_axes",
        &policy.allowed_axes,
        "invalid_symmetry_axis",
    );
}

fn validate_exaggeration_policy(policy: &ExaggerationPolicy, report: &mut FamilyValidationReport) {
    validate_fraction(
        report,
        Some("exaggeration.silhouette"),
        policy.silhouette,
        "invalid_silhouette_exaggeration",
    );
    validate_fraction(
        report,
        Some("exaggeration.detail"),
        policy.detail,
        "invalid_detail_exaggeration",
    );
}

fn validate_family_style_facets(kit: &StyleKit, report: &mut FamilyValidationReport) {
    for family_id in &kit.compatible_families {
        if !kit.family_facets.contains_key(family_id) {
            push_issue(
                report,
                Some(format!("family_facets.{family_id}")),
                "missing_family_style_facet",
                "Every compatible family must have a family-scoped style facet.",
            );
        }
    }
    for (family_id, facet) in &kit.family_facets {
        validate_identifier(
            report,
            Some(format!("family_facets.{family_id}")),
            family_id,
            "invalid_family_style_facet_key",
        );
        if facet.family_id != *family_id {
            push_issue(
                report,
                Some(format!("family_facets.{family_id}.family_id")),
                "family_style_facet_id_mismatch",
                "Family style facet ID must match its map key.",
            );
        }
        if !kit
            .compatible_families
            .iter()
            .any(|compatible| compatible == family_id)
        {
            push_issue(
                report,
                Some(format!("family_facets.{family_id}")),
                "undeclared_family_style_facet",
                "Family style facets must target a declared compatible family.",
            );
        }
        validate_role_proportion_list(
            &facet.proportions,
            &format!("family_facets.{family_id}.proportions"),
            report,
        );
        validate_part_prototype_list(
            &facet.part_prototypes,
            &format!("family_facets.{family_id}.part_prototypes"),
            report,
        );
        validate_detail_module_list(
            &facet.detail_modules,
            &format!("family_facets.{family_id}.detail_modules"),
            report,
        );
        validate_family_style_policy_overrides(family_id, &facet.policy_overrides, report);
    }
}

fn validate_family_style_policy_overrides(
    family_id: &str,
    overrides: &FamilyStylePolicyOverrides,
    report: &mut FamilyValidationReport,
) {
    if let Some(policy) = &overrides.bevel_policy {
        validate_bevel_policy_scoped(
            policy,
            &format!("family_facets.{family_id}.policy_overrides.bevel_policy"),
            report,
        );
    }
    if let Some(policy) = &overrides.repetition {
        validate_repetition_policy_scoped(
            policy,
            &format!("family_facets.{family_id}.policy_overrides.repetition"),
            report,
        );
    }
}

fn validate_kit_role_references(
    family_id: &str,
    facet: &FamilyStyleFacet,
    role_ids: &BTreeSet<&str>,
    report: &mut FamilyValidationReport,
) {
    for (index, proportion) in facet.proportions.iter().enumerate() {
        validate_role_reference(
            report,
            role_ids,
            format!("style_kit.family_facets.{family_id}.proportions.{index}.role"),
            &proportion.role,
            "unknown_style_proportion_role",
        );
        for (axis, value) in proportion.preferred_scale.iter().enumerate() {
            validate_length_value_role_reference(
                report,
                role_ids,
                format!(
                    "style_kit.family_facets.{family_id}.proportions.{index}.preferred_scale.{axis}"
                ),
                value,
                "unknown_style_relative_length_role",
            );
        }
    }
    for (index, prototype) in facet.part_prototypes.iter().enumerate() {
        validate_role_reference(
            report,
            role_ids,
            format!("style_kit.family_facets.{family_id}.part_prototypes.{index}.role"),
            &prototype.role,
            "unknown_style_prototype_role",
        );
    }
    for (index, module) in facet.detail_modules.iter().enumerate() {
        for role in &module.target_roles {
            validate_role_reference(
                report,
                role_ids,
                format!("style_kit.family_facets.{family_id}.detail_modules.{index}.target_roles"),
                role,
                "unknown_style_detail_role",
            );
        }
    }
}

fn validate_kit_operation_compatibility(
    family: &AssetFamilySchema,
    family_id: &str,
    facet: &FamilyStyleFacet,
    report: &mut FamilyValidationReport,
) {
    let allowed = family.allowed_operations.iter().collect::<BTreeSet<_>>();
    for (prototype_index, prototype) in facet.part_prototypes.iter().enumerate() {
        for (operation_index, operation) in prototype.operation_tags.iter().enumerate() {
            if !allowed.contains(operation) {
                push_issue(
                    report,
                    Some(format!(
                        "style_kit.family_facets.{family_id}.part_prototypes.{prototype_index}.operation_tags.{operation_index}"
                    )),
                    "style_prototype_operation_not_allowed",
                    "Style-kit prototype requires an operation not allowed by the family.",
                );
            }
        }
    }
}

fn validate_style_required_role_providers(
    family: &AssetFamilySchema,
    family_id: &str,
    facet: &FamilyStyleFacet,
    report: &mut FamilyValidationReport,
) {
    let style_roles = facet
        .part_prototypes
        .iter()
        .map(|prototype| prototype.role.as_str())
        .collect::<BTreeSet<_>>();
    for role in &family.part_roles {
        if role.required
            && role.provision == RoleProvision::StyleRequired
            && !style_roles.contains(role.id.as_str())
        {
            push_issue(
                report,
                Some(format!(
                    "style_kit.family_facets.{family_id}.part_prototypes"
                )),
                "missing_style_required_role_provider",
                "Required style-provided roles need at least one style-kit prototype.",
            );
        }
    }
}

fn validate_length_value_role_reference(
    report: &mut FamilyValidationReport,
    role_ids: &BTreeSet<&str>,
    subject: impl Into<String>,
    value: &LengthValue,
    code: &'static str,
) {
    if let LengthValue::RelativeToRole { role, .. } = value {
        validate_role_reference(report, role_ids, subject, role, code);
    }
}

fn validate_length_unit_role_reference(
    report: &mut FamilyValidationReport,
    role_ids: &BTreeSet<&str>,
    subject: impl Into<String>,
    unit: &LengthUnit,
    code: &'static str,
) {
    if let LengthUnit::RelativeToRole { role } = unit {
        validate_role_reference(report, role_ids, subject, role, code);
    }
}

fn validate_length_unit(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    unit: &LengthUnit,
) {
    if let LengthUnit::RelativeToRole { role } = unit {
        validate_identifier(report, subject, role, "invalid_relative_length_unit_role");
    }
}

fn validate_angle_unit(
    _report: &mut FamilyValidationReport,
    _subject: Option<impl Into<String>>,
    _unit: AngleUnit,
) {
}

fn validate_global_length_value(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &LengthValue,
    code: &'static str,
) {
    let subject = subject.map(Into::into);
    validate_length_value(report, subject.clone(), value, code);
    if matches!(value, LengthValue::RelativeToRole { .. }) {
        push_issue(
            report,
            subject,
            "global_style_policy_relative_to_role",
            "Global style policies must use absolute style lengths; role-relative lengths belong in family-scoped facets.",
        );
    }
}

fn validate_length_value(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &LengthValue,
    code: &'static str,
) {
    let subject = subject.map(Into::into);
    match value {
        LengthValue::Meters(value) | LengthValue::FamilyUnits(value) => {
            if !value.is_finite() || *value <= 0.0 {
                push_issue(
                    report,
                    subject.clone(),
                    code,
                    "Length values must be finite and greater than zero.",
                );
            }
        }
        LengthValue::RelativeToRole { role, ratio } => {
            if !ratio.is_finite() || *ratio <= 0.0 {
                push_issue(
                    report,
                    subject.clone(),
                    code,
                    "Relative length ratios must be finite and greater than zero.",
                );
            }
            validate_identifier(report, subject, role, "invalid_relative_length_role");
        }
    }
}

fn validate_readability_threshold(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    threshold: &ReadabilityThreshold,
) {
    let subject = subject.map(Into::into);
    if threshold.pixels == 0 {
        push_issue(
            report,
            subject.clone(),
            "invalid_readability_threshold_pixels",
            "Readability thresholds must be greater than zero pixels.",
        );
    }
    validate_identifier(
        report,
        subject,
        &threshold.camera_profile,
        "invalid_readability_camera_profile",
    );
}

fn validate_parameter_range(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    range: ParameterRange,
) {
    if !range.minimum.is_finite() || !range.maximum.is_finite() || !range.step.is_finite() {
        push_issue(
            report,
            subject,
            "non_finite_parameter_range",
            "Parameter ranges must contain only finite values.",
        );
    } else if range.minimum > range.maximum {
        push_issue(
            report,
            subject,
            "invalid_parameter_range",
            "Parameter range minimum must not exceed maximum.",
        );
    } else if range.step <= 0.0 {
        push_issue(
            report,
            subject,
            "invalid_parameter_step",
            "Parameter range step must be greater than zero.",
        );
    }
}

fn validate_role_reference(
    report: &mut FamilyValidationReport,
    role_ids: &BTreeSet<&str>,
    subject: impl Into<String>,
    role: &str,
    code: &'static str,
) {
    if !role_ids.contains(role) {
        push_issue(
            report,
            Some(subject),
            code,
            "Role reference is not declared.",
        );
    }
}

fn validate_unique_strings(
    report: &mut FamilyValidationReport,
    subject: &str,
    values: &[String],
    code: &'static str,
) {
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let normalized = value.trim();
        if normalized.is_empty() {
            push_issue(
                report,
                Some(format!("{subject}.{index}")),
                "empty_identifier",
                "Identifier values cannot be empty.",
            );
        }
        if !seen.insert(normalized) {
            push_issue(
                report,
                Some(format!("{subject}.{index}")),
                code,
                "Identifier values must be unique.",
            );
        }
    }
}

fn validate_identifier_list(
    report: &mut FamilyValidationReport,
    subject: &str,
    values: &[String],
    code: &'static str,
) {
    for (index, value) in values.iter().enumerate() {
        validate_identifier(report, Some(format!("{subject}.{index}")), value, code);
    }
}

fn validate_identifier(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if !stable_identifier_is_valid(value) {
        push_issue(
            report,
            subject,
            code,
            "Stable identifiers must start with a lowercase ASCII letter, end with an alphanumeric character, and use non-repeated lowercase ASCII letters, digits, `_`, `-`, `.`, or `:` separators.",
        );
    }
}

fn stable_identifier_is_valid(value: &str) -> bool {
    if value.trim() != value || value.is_empty() || value == "." || value == ".." {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut previous_was_separator = false;
    let mut last = first;
    for character in std::iter::once(first).chain(chars) {
        if !is_identifier_char(character) {
            return false;
        }
        let is_separator = is_identifier_separator(character);
        if is_separator && previous_was_separator {
            return false;
        }
        previous_was_separator = is_separator;
        last = character;
    }
    last.is_ascii_lowercase() || last.is_ascii_digit()
}

fn is_identifier_char(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '_' | '-' | '.' | ':')
}

fn is_identifier_separator(character: char) -> bool {
    matches!(character, '_' | '-' | '.' | ':')
}

fn validate_fraction(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: f32,
    code: &'static str,
) {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        push_issue(
            report,
            subject,
            code,
            "Value must be a finite fraction from 0 to 1.",
        );
    }
}

fn validate_non_empty(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if value.trim().is_empty() {
        push_issue(report, subject, code, "Value cannot be empty.");
    }
}

fn push_issue(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    report.issues.push(FamilyValidationIssue {
        subject: subject.map(Into::into),
        code: code.into(),
        message: message.into(),
    });
}
