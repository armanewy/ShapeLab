#![forbid(unsafe_code)]

//! Executable bindings from asset-family/style-kit contracts to concrete recipes.
//!
//! This crate deliberately keeps the first compiler small: recipe fragments are
//! deterministic Shape Lab recipes, parameter bindings are simple scalar or
//! presence/prototype choices, and unsupported fragment features are rejected
//! instead of implicitly remapped.

pub mod identity;
pub mod remap;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetId, AssetRecipe, AssetValidationReport, AttachmentMode, ParameterDescriptor,
    PartDefinitionId, PartInstanceId, RegionId, SocketId, SurfaceRegionSpec, validate_asset_recipe,
};
use shape_compile::{AssetArtifact, CompileError, CompileValidationReport, compile_asset};
use shape_family::{
    AssetFamilySchema, FamilyDefaultValue, FamilyParameterKind, FamilyValidationIssue,
    FamilyValidationReport, ParameterExecutionPolicy, ParameterRange, PartRole, RoleMultiplicity,
    RoleProvision, StyleKit, validate_family_style_compatibility,
    validate_family_style_completeness,
};
use thiserror::Error;

use crate::identity::{
    ArtifactFingerprint, FingerprintError, FoundryIntentFingerprint, GeometryInputFingerprint,
    RecipeFingerprint, fingerprint_serializable,
};

/// Current schema version for executable family implementations.
pub const FAMILY_IMPLEMENTATION_SCHEMA_VERSION: u32 = 2;

/// Current schema version for executable style implementations.
pub const STYLE_IMPLEMENTATION_SCHEMA_VERSION: u32 = 3;

/// Current schema version for executable recipe fragments.
pub const RECIPE_FRAGMENT_SCHEMA_VERSION: u32 = 2;

/// Executable family binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyImplementation {
    /// Executable family implementation schema version.
    pub schema_version: u32,
    /// Family ID implemented by this binding.
    pub family_id: String,
    /// Base recipe used as the deterministic merge target.
    pub base_recipe: AssetRecipe,
    /// Parameter bindings applied after fragments are merged.
    pub parameter_bindings: Vec<ParameterBinding>,
    /// Family-owned default providers keyed by role ID.
    pub default_role_providers: BTreeMap<String, String>,
    /// Family-owned recipe fragments keyed by fragment ID.
    pub fragments: BTreeMap<String, RecipeFragment>,
    /// Explicit cross-fragment attachment bindings resolved through exported ports.
    #[serde(default)]
    pub attachment_bindings: Vec<FragmentAttachmentBinding>,
}

/// Executable style binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleImplementation {
    /// Executable style implementation schema version.
    pub schema_version: u32,
    /// Style kit ID implemented by this binding.
    pub style_kit_id: String,
    /// Family ID this style implementation targets.
    pub family_id: String,
    /// Explicit default style providers keyed by family role ID.
    pub default_role_providers: BTreeMap<String, String>,
    /// Style prototypes keyed by prototype ID.
    pub prototypes: BTreeMap<String, RecipeFragment>,
    /// Detail fragments keyed by detail module ID.
    pub detail_modules: BTreeMap<String, RecipeFragment>,
}

/// Concrete fragment that can provide a family role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeFragment {
    /// Executable fragment schema version.
    pub schema_version: u32,
    /// Fragment ID.
    pub id: String,
    /// Family role provided by this fragment.
    pub provided_role: String,
    /// Explicit ports and occurrence roots exported by this fragment.
    pub exports: RecipeFragmentExports,
    /// Source recipe whose roots will be merged into the target recipe.
    pub recipe: AssetRecipe,
}

/// Explicit exports from a self-contained recipe fragment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RecipeFragmentExports {
    /// Root instances that count as externally visible role occurrences.
    pub role_occurrence_roots: Vec<PartInstanceId>,
    /// Internal helper roots that do not count toward role cardinality.
    #[serde(default)]
    pub internal_roots: Vec<PartInstanceId>,
    /// Socket ports that other fragments may attach to.
    #[serde(default)]
    pub socket_ports: Vec<FragmentSocketPort>,
    /// Surface ports that other fragments may use for placement or conformance.
    #[serde(default)]
    pub surface_ports: Vec<FragmentSurfacePort>,
}

/// Exported socket port on a fragment-local occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentSocketPort {
    /// Stable port ID within this fragment.
    pub id: String,
    /// Local occurrence root exposing the socket.
    pub local_occurrence_root: PartInstanceId,
    /// Socket ID on the occurrence definition.
    pub local_socket: SocketId,
    /// Compatibility tags used by attachment bindings.
    pub compatibility_tags: Vec<String>,
}

/// Exported surface port on a fragment-local definition or occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentSurfacePort {
    /// Stable port ID within this fragment.
    pub id: String,
    /// Fragment-local surface target.
    pub target: FragmentSurfaceTarget,
    /// Local region ID exposed by the target.
    pub local_region: RegionId,
    /// Semantic tags used by conformance or placement.
    pub semantic_tags: Vec<String>,
}

/// Local target for an exported surface port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FragmentSurfaceTarget {
    /// Surface region on a definition.
    Definition(PartDefinitionId),
    /// Surface region on an occurrence's definition.
    Occurrence(PartInstanceId),
}

/// Binding that assembles selected fragments through exported ports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentAttachmentBinding {
    /// Family attachment-rule ID this binding implements.
    pub family_attachment_rule: String,
    /// Source family role.
    pub source_role: String,
    /// Source fragment port ID.
    pub source_port: String,
    /// Destination family role.
    pub destination_role: String,
    /// Destination fragment port ID.
    pub destination_port: String,
    /// Pairing policy for repeated occurrences.
    pub pairing: FragmentAttachmentPairing,
    /// Finite offset applied after port alignment.
    pub offset: [f32; 3],
    /// Attachment mode applied to generated instance attachments.
    pub attachment_mode: AttachmentMode,
}

/// Pairing policy for cross-fragment port bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FragmentAttachmentPairing {
    /// Every source occurrence pairs with every destination occurrence.
    AllPairs,
    /// Pair occurrences by deterministic ordinal.
    ByOccurrenceIndex,
    /// Pair each source to the nearest available destination.
    NearestOneToOne,
    /// Explicit source/destination occurrence ordinals.
    ExplicitOrdinalPairs(Vec<(u32, u32)>),
}

/// Request to instantiate one family/style pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyInstantiationRequest {
    /// Requested family ID.
    pub family_id: String,
    /// Requested style-kit ID.
    pub style_kit_id: String,
    /// Semantic parameter values keyed by family slot ID.
    pub parameters: BTreeMap<String, FamilyValue>,
    /// Deterministic seed reserved for future randomized choices.
    pub seed: u64,
}

/// Semantic family parameter value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FamilyValue {
    /// Floating-point scalar.
    Scalar(f32),
    /// Integer count.
    Integer(u32),
    /// Boolean toggle.
    Toggle(bool),
    /// Symbolic choice.
    Choice(String),
}

/// Binding from a family parameter slot to an executable action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterBinding {
    /// Bind a scalar-like parameter to one scalar path inside a role provider fragment.
    Scalar {
        /// Family parameter slot ID.
        slot: String,
        /// Provider role containing the local path.
        role: String,
        /// Scalar path in the provider fragment before ID remapping.
        local_path: String,
        /// Scalar transformation.
        transform: ScalarTransform,
    },
    /// Bind a toggle to role presence.
    TogglePartPresence {
        /// Family parameter slot ID.
        slot: String,
        /// Role whose merged instances should be enabled or omitted.
        role: String,
    },
    /// Bind a choice to a style prototype ID for one role.
    ChoiceToPrototype {
        /// Family parameter slot ID.
        slot: String,
        /// Role whose provider should be selected.
        role: String,
        /// Choice value to prototype ID.
        choices: BTreeMap<String, String>,
    },
}

impl ParameterBinding {
    fn slot(&self) -> &str {
        match self {
            Self::Scalar { slot, .. }
            | Self::TogglePartPresence { slot, .. }
            | Self::ChoiceToPrototype { slot, .. } => slot,
        }
    }
}

/// Supported scalar binding transforms.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarTransform {
    /// Use the value directly.
    Direct,
    /// Apply `value * scale + offset`.
    ScaleOffset {
        /// Multiplicative scale.
        scale: f32,
        /// Additive offset.
        offset: f32,
    },
    /// Map ratio 0..1 into a scalar range.
    Ratio {
        /// Output minimum.
        minimum: f32,
        /// Output maximum.
        maximum: f32,
    },
    /// Use an integer count as a scalar.
    IntegerCount,
}

/// Successful instantiation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyInstantiation {
    /// Instantiated recipe.
    pub recipe: AssetRecipe,
    /// Compiled artifact.
    pub artifact: AssetArtifact,
    /// Deterministic instantiation report.
    pub report: FamilyInstantiationReport,
}

/// Deterministic report for one instantiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyInstantiationReport {
    /// Family ID.
    pub family_id: String,
    /// Style-kit ID.
    pub style_kit_id: String,
    /// Selected provider fragment by role.
    pub selected_providers: BTreeMap<String, String>,
    /// Applied parameter bindings.
    pub parameter_applications: Vec<ParameterApplication>,
    /// Typed fragment remaps used while merging providers.
    pub fragment_remaps: Vec<remap::FragmentRemapReport>,
    /// Stable source recipe hash from the compiled artifact.
    pub source_recipe_hash: u64,
    /// Canonical content fingerprint used to derive the recipe ID.
    pub instantiation_fingerprint: String,
    /// Fingerprint for all semantic request intent, including advisory/runtime values.
    pub foundry_intent_fingerprint: String,
    /// Fingerprint for values consumed by executable geometry generation.
    pub geometry_input_fingerprint: String,
    /// Fingerprint for the instantiated asset recipe.
    pub recipe_fingerprint: String,
    /// Fingerprint for the compiled artifact contract.
    pub artifact_fingerprint: String,
    /// Number of compiled part occurrences.
    pub compiled_part_count: u64,
}

/// One parameter application row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterApplication {
    /// Family parameter slot ID.
    pub slot: String,
    /// Target role.
    pub role: String,
    /// Remapped concrete scalar path or role-presence target.
    pub target: String,
    /// Applied scalar or symbolic value.
    pub value: String,
}

/// Family instantiation failure.
#[derive(Debug, Error)]
pub enum FamilyCompileError {
    /// Family implementation does not match request/schema.
    #[error("family implementation ID `{implementation}` does not match `{expected}`")]
    FamilyImplementationMismatch {
        /// Expected family ID.
        expected: String,
        /// Implementation family ID.
        implementation: String,
    },
    /// Style implementation does not match request/schema.
    #[error("style implementation ID `{implementation}` does not match `{expected}`")]
    StyleImplementationMismatch {
        /// Expected style ID.
        expected: String,
        /// Implementation style ID.
        implementation: String,
    },
    /// Family/style schema validation failed.
    #[error("family/style schema validation failed")]
    SchemaValidationFailed(FamilyValidationReport),
    /// Family implementation or style implementation validation failed.
    #[error("family/style implementation validation failed")]
    ImplementationValidationFailed(FamilyValidationReport),
    /// Instantiation request validation failed.
    #[error("family instantiation request validation failed")]
    RequestValidationFailed(FamilyValidationReport),
    /// Instantiated recipe does not satisfy family role cardinality.
    #[error("family role cardinality validation failed")]
    RoleValidationFailed(FamilyValidationReport),
    /// Role has no executable provider.
    #[error("missing executable provider for role `{role}`")]
    MissingRoleProvider {
        /// Role ID.
        role: String,
    },
    /// Provider fragment ID is unknown.
    #[error("unknown recipe fragment `{fragment}` for role `{role}`")]
    UnknownFragment {
        /// Role ID.
        role: String,
        /// Fragment ID.
        fragment: String,
    },
    /// Parameter value type does not match binding type.
    #[error("parameter `{slot}` has incompatible value")]
    IncompatibleParameterValue {
        /// Parameter slot.
        slot: String,
    },
    /// Parameter binding points at a role/path that was not merged.
    #[error("parameter binding `{slot}` target was not merged")]
    UnresolvedParameterBinding {
        /// Parameter slot.
        slot: String,
    },
    /// Fragment contains unsupported features for this first compiler slice.
    #[error("unsupported recipe fragment `{fragment}`: {reason}")]
    UnsupportedFragment {
        /// Fragment ID.
        fragment: String,
        /// Reason.
        reason: String,
    },
    /// Fragment recipe is invalid before remapping.
    #[error("recipe fragment `{fragment}` failed validation")]
    FragmentValidationFailed {
        /// Fragment ID.
        fragment: String,
        /// Validation report.
        report: AssetValidationReport,
    },
    /// Scalar path rewrite failed.
    #[error("unsupported scalar path `{path}`")]
    UnsupportedScalarPath {
        /// Scalar path.
        path: String,
    },
    /// Canonical fingerprint serialization failed.
    #[error("failed to serialize `{subject}` for instantiation fingerprint: {error}")]
    FingerprintSerializationFailed {
        /// Fingerprinted subject.
        subject: String,
        /// Serialization error.
        error: String,
    },
    /// Asset recipe validation failed.
    #[error("instantiated asset recipe validation failed")]
    AssetValidationFailed(AssetValidationReport),
    /// Compiled artifact reported validation issues.
    #[error("compiled artifact validation failed")]
    CompileValidationFailed(CompileValidationReport),
    /// Shape compile failed.
    #[error(transparent)]
    Compile(#[from] CompileError),
}

/// Instantiate and compile one family/style pair.
pub fn instantiate_family(
    family: &AssetFamilySchema,
    style_kit: &StyleKit,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
    request: &FamilyInstantiationRequest,
) -> Result<FamilyInstantiation, FamilyCompileError> {
    ensure_ids_match(family, style_kit, family_impl, style_impl, request)?;
    let compatibility = validate_family_style_compatibility(family, style_kit);
    if !compatibility.is_valid() {
        return Err(FamilyCompileError::SchemaValidationFailed(compatibility));
    }
    let completeness = validate_family_style_completeness(family, style_kit);
    if !completeness.is_valid() {
        return Err(FamilyCompileError::SchemaValidationFailed(completeness));
    }
    validate_implementations(family, style_kit, family_impl, style_impl)?;
    let effective_request = effective_request(family, request);
    validate_request(family, &effective_request)?;

    let choice_overrides = resolve_choice_overrides(
        style_impl,
        &family_impl.parameter_bindings,
        &effective_request,
    )?;
    let presence_overrides =
        resolve_presence_overrides(&family_impl.parameter_bindings, &effective_request)?;
    let selected_providers = select_role_providers(
        family,
        family_impl,
        style_impl,
        &choice_overrides,
        &presence_overrides,
    )?;
    let mut recipe = family_impl.base_recipe.clone();
    let fingerprints = derive_instantiation_fingerprints(
        family,
        style_kit,
        family_impl,
        style_impl,
        &effective_request,
        &selected_providers,
    )?;
    recipe.id = AssetId(fingerprints.geometry_input.0.to_nonzero_u64());
    recipe.title = format!("{} / {}", family.display_name, style_kit.display_name);

    let mut merge_state = MergeState::default();
    for (role, selection) in &selected_providers {
        let fragment = find_fragment(family_impl, style_impl, role, selection)?;
        merge_fragment(&mut recipe, fragment, &mut merge_state)?;
    }

    let mut parameter_applications = Vec::new();
    apply_parameter_bindings(
        &mut recipe,
        &family_impl.parameter_bindings,
        &effective_request,
        &merge_state,
        &mut parameter_applications,
    )?;
    validate_role_cardinality(family, &recipe, &merge_state)?;

    let asset_report = validate_asset_recipe(&recipe);
    if !asset_report.is_valid() {
        return Err(FamilyCompileError::AssetValidationFailed(asset_report));
    }
    let recipe_fingerprint = RecipeFingerprint(
        fingerprint_serializable("shape-lab.recipe.v1", "instantiated_recipe", &recipe)
            .map_err(fingerprint_error)?,
    );
    let artifact = compile_asset(&recipe)?;
    if !artifact.validation_report.is_valid() {
        return Err(FamilyCompileError::CompileValidationFailed(
            artifact.validation_report.clone(),
        ));
    }
    let artifact_fingerprint = ArtifactFingerprint(
        fingerprint_serializable("shape-lab.artifact.v1", "compiled_artifact", &artifact)
            .map_err(fingerprint_error)?,
    );

    let report = FamilyInstantiationReport {
        family_id: family.id.clone(),
        style_kit_id: style_kit.id.clone(),
        selected_providers: selected_providers
            .iter()
            .map(|(role, selection)| (role.clone(), selection.fragment.clone()))
            .collect(),
        parameter_applications,
        fragment_remaps: merge_state.remap_reports,
        source_recipe_hash: artifact.source_recipe_hash,
        instantiation_fingerprint: fingerprints.geometry_input.0.to_hex(),
        foundry_intent_fingerprint: fingerprints.foundry_intent.0.to_hex(),
        geometry_input_fingerprint: fingerprints.geometry_input.0.to_hex(),
        recipe_fingerprint: recipe_fingerprint.0.to_hex(),
        artifact_fingerprint: artifact_fingerprint.0.to_hex(),
        compiled_part_count: artifact.statistics.part_count,
    };
    Ok(FamilyInstantiation {
        recipe,
        artifact,
        report,
    })
}

fn ensure_ids_match(
    family: &AssetFamilySchema,
    style_kit: &StyleKit,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
    request: &FamilyInstantiationRequest,
) -> Result<(), FamilyCompileError> {
    if family.id != request.family_id || family.id != family_impl.family_id {
        return Err(FamilyCompileError::FamilyImplementationMismatch {
            expected: family.id.clone(),
            implementation: family_impl.family_id.clone(),
        });
    }
    if family.id != style_impl.family_id {
        return Err(FamilyCompileError::FamilyImplementationMismatch {
            expected: family.id.clone(),
            implementation: style_impl.family_id.clone(),
        });
    }
    if style_kit.id != request.style_kit_id || style_kit.id != style_impl.style_kit_id {
        return Err(FamilyCompileError::StyleImplementationMismatch {
            expected: style_kit.id.clone(),
            implementation: style_impl.style_kit_id.clone(),
        });
    }
    Ok(())
}

fn effective_request(
    family: &AssetFamilySchema,
    request: &FamilyInstantiationRequest,
) -> FamilyInstantiationRequest {
    let mut effective = request.clone();
    for slot in &family.parameter_slots {
        if !effective.parameters.contains_key(&slot.id)
            && let Some(default_value) = &slot.default_value
        {
            effective
                .parameters
                .insert(slot.id.clone(), family_value_from_default(default_value));
        }
    }
    effective
}

fn family_value_from_default(default_value: &FamilyDefaultValue) -> FamilyValue {
    match default_value {
        FamilyDefaultValue::Scalar(value) => FamilyValue::Scalar(*value),
        FamilyDefaultValue::Integer(value) => FamilyValue::Integer(*value),
        FamilyDefaultValue::Toggle(value) => FamilyValue::Toggle(*value),
        FamilyDefaultValue::Choice(value) => FamilyValue::Choice(value.clone()),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct InstantiationFingerprints {
    foundry_intent: FoundryIntentFingerprint,
    geometry_input: GeometryInputFingerprint,
}

#[derive(Serialize)]
struct FoundryIntentFingerprintPayload<'a> {
    family: &'a AssetFamilySchema,
    style_kit: &'a StyleKit,
    family_implementation: &'a FamilyImplementation,
    style_implementation: &'a StyleImplementation,
    effective_parameters: &'a BTreeMap<String, FamilyValue>,
    seed: u64,
    selected_providers: Vec<SelectedProviderFingerprint<'a>>,
}

#[derive(Serialize)]
struct GeometryInputFingerprintPayload<'a> {
    family_id: &'a str,
    family_schema_version: u32,
    style_kit_id: &'a str,
    style_kit_schema_version: u32,
    family_part_roles: serde_json::Value,
    family_attachment_rules: serde_json::Value,
    family_constraints: serde_json::Value,
    required_parameter_slots: serde_json::Value,
    style_family_facet: serde_json::Value,
    style_bevel_policy: &'a shape_family::BevelPolicy,
    style_profile_language: &'a shape_family::ProfileLanguage,
    style_repetition: &'a shape_family::RepetitionPolicy,
    style_symmetry: &'a shape_family::SymmetryPolicy,
    style_exaggeration: &'a shape_family::ExaggerationPolicy,
    base_recipe: serde_json::Value,
    required_parameter_bindings: Vec<&'a ParameterBinding>,
    family_default_role_providers: &'a BTreeMap<String, String>,
    style_default_role_providers: &'a BTreeMap<String, String>,
    selected_providers: Vec<SelectedProviderFingerprint<'a>>,
    executable_parameters: BTreeMap<String, FamilyValue>,
    seed: u64,
}

#[derive(Clone, Serialize)]
struct SelectedProviderFingerprint<'a> {
    role: &'a str,
    fragment: &'a str,
    source: &'static str,
    fragment_contract: Option<&'a RecipeFragment>,
}

fn derive_instantiation_fingerprints(
    family: &AssetFamilySchema,
    style_kit: &StyleKit,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
    request: &FamilyInstantiationRequest,
    selected_providers: &BTreeMap<String, ProviderSelection>,
) -> Result<InstantiationFingerprints, FamilyCompileError> {
    let selected_provider_payload =
        selected_provider_fingerprints(family_impl, style_impl, selected_providers);
    let foundry_payload = FoundryIntentFingerprintPayload {
        family,
        style_kit,
        family_implementation: family_impl,
        style_implementation: style_impl,
        effective_parameters: &request.parameters,
        seed: request.seed,
        selected_providers: selected_provider_payload.clone(),
    };
    let foundry_intent = FoundryIntentFingerprint(
        fingerprint_serializable(
            "shape-lab.foundry-intent.v1",
            "foundry_intent",
            &foundry_payload,
        )
        .map_err(fingerprint_error)?,
    );

    let required_parameter_slots = family
        .parameter_slots
        .iter()
        .filter(|slot| slot.execution_policy == ParameterExecutionPolicy::RequiredBinding)
        .collect::<Vec<_>>();
    let required_parameter_bindings =
        executable_parameter_bindings(family, &family_impl.parameter_bindings);
    let executable_parameters = executable_parameter_values(family, request);
    let style_family_facet = style_kit.family_facets.get(&family.id);
    let geometry_payload = GeometryInputFingerprintPayload {
        family_id: &family.id,
        family_schema_version: family.schema_version,
        style_kit_id: &style_kit.id,
        style_kit_schema_version: style_kit.schema_version,
        family_part_roles: fingerprint_value("family.part_roles", &family.part_roles)?,
        family_attachment_rules: fingerprint_value(
            "family.attachment_rules",
            &family.attachment_rules,
        )?,
        family_constraints: fingerprint_value("family.constraints", &family.constraints)?,
        required_parameter_slots: fingerprint_value(
            "family.required_parameter_slots",
            &required_parameter_slots,
        )?,
        style_family_facet: fingerprint_value("style.family_facet", &style_family_facet)?,
        style_bevel_policy: &style_kit.bevel_policy,
        style_profile_language: &style_kit.profile_language,
        style_repetition: &style_kit.repetition,
        style_symmetry: &style_kit.symmetry,
        style_exaggeration: &style_kit.exaggeration,
        base_recipe: fingerprint_value(
            "family_implementation.base_recipe.geometry_inputs",
            &geometry_base_recipe(&family_impl.base_recipe),
        )?,
        required_parameter_bindings,
        family_default_role_providers: &family_impl.default_role_providers,
        style_default_role_providers: &style_impl.default_role_providers,
        selected_providers: selected_provider_payload,
        executable_parameters,
        seed: request.seed,
    };
    let geometry_input = GeometryInputFingerprint(
        fingerprint_serializable(
            "shape-lab.geometry-input.v1",
            "geometry_input",
            &geometry_payload,
        )
        .map_err(fingerprint_error)?,
    );
    Ok(InstantiationFingerprints {
        foundry_intent,
        geometry_input,
    })
}

fn geometry_base_recipe(base_recipe: &AssetRecipe) -> AssetRecipe {
    let mut recipe = base_recipe.clone();
    recipe.id = AssetId(0);
    recipe.title.clear();
    recipe
}

fn selected_provider_fingerprints<'a>(
    family_impl: &'a FamilyImplementation,
    style_impl: &'a StyleImplementation,
    selected_providers: &'a BTreeMap<String, ProviderSelection>,
) -> Vec<SelectedProviderFingerprint<'a>> {
    selected_providers
        .iter()
        .map(|(role, selection)| SelectedProviderFingerprint {
            role,
            fragment: &selection.fragment,
            source: match selection.source {
                ProviderSource::FamilyDefault => "family_default",
                ProviderSource::StylePrototype => "style_prototype",
            },
            fragment_contract: find_selected_fragment_for_hash(family_impl, style_impl, selection),
        })
        .collect()
}

fn executable_parameter_bindings<'a>(
    family: &'a AssetFamilySchema,
    bindings: &'a [ParameterBinding],
) -> Vec<&'a ParameterBinding> {
    let executable_slots = family
        .parameter_slots
        .iter()
        .filter(|slot| slot.execution_policy == ParameterExecutionPolicy::RequiredBinding)
        .map(|slot| slot.id.as_str())
        .collect::<BTreeSet<_>>();
    bindings
        .iter()
        .filter(|binding| executable_slots.contains(binding.slot()))
        .collect()
}

fn executable_parameter_values(
    family: &AssetFamilySchema,
    request: &FamilyInstantiationRequest,
) -> BTreeMap<String, FamilyValue> {
    let executable_slots = family
        .parameter_slots
        .iter()
        .filter(|slot| slot.execution_policy == ParameterExecutionPolicy::RequiredBinding)
        .map(|slot| slot.id.as_str())
        .collect::<BTreeSet<_>>();
    request
        .parameters
        .iter()
        .filter(|(slot, _)| executable_slots.contains(slot.as_str()))
        .map(|(slot, value)| (slot.clone(), value.clone()))
        .collect()
}

fn fingerprint_value<T: Serialize>(
    subject: &str,
    value: &T,
) -> Result<serde_json::Value, FamilyCompileError> {
    serde_json::to_value(value).map_err(|error| {
        FamilyCompileError::FingerprintSerializationFailed {
            subject: subject.to_owned(),
            error: error.to_string(),
        }
    })
}

fn fingerprint_error(error: FingerprintError) -> FamilyCompileError {
    match error {
        FingerprintError::Serialization { subject, error } => {
            FamilyCompileError::FingerprintSerializationFailed { subject, error }
        }
        FingerprintError::NonFiniteNumber { subject } => {
            FamilyCompileError::FingerprintSerializationFailed {
                subject,
                error: "canonical fingerprint input contained a non-finite number".to_owned(),
            }
        }
    }
}

fn find_selected_fragment_for_hash<'a>(
    family_impl: &'a FamilyImplementation,
    style_impl: &'a StyleImplementation,
    selection: &ProviderSelection,
) -> Option<&'a RecipeFragment> {
    match selection.source {
        ProviderSource::FamilyDefault => family_impl.fragments.get(&selection.fragment),
        ProviderSource::StylePrototype => style_impl.prototypes.get(&selection.fragment),
    }
}

fn validate_implementations(
    family: &AssetFamilySchema,
    style_kit: &StyleKit,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
) -> Result<(), FamilyCompileError> {
    let mut report = FamilyValidationReport::default();
    if family_impl.schema_version != FAMILY_IMPLEMENTATION_SCHEMA_VERSION {
        push_report_issue(
            &mut report,
            Some("family_implementation.schema_version"),
            "unsupported_family_implementation_schema",
            "Family implementation schema version is not supported.",
        );
    }
    if style_impl.schema_version != STYLE_IMPLEMENTATION_SCHEMA_VERSION {
        push_report_issue(
            &mut report,
            Some("style_implementation.schema_version"),
            "unsupported_style_implementation_schema",
            "Style implementation schema version is not supported.",
        );
    }
    let role_ids = family
        .part_roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<BTreeSet<_>>();
    let roles = family
        .part_roles
        .iter()
        .map(|role| (role.id.as_str(), role))
        .collect::<BTreeMap<_, _>>();
    let slots = family
        .parameter_slots
        .iter()
        .map(|slot| (slot.id.as_str(), slot))
        .collect::<BTreeMap<_, _>>();
    let Some(style_facet) = style_kit.family_facets.get(&family.id) else {
        push_report_issue(
            &mut report,
            Some(format!("style_kit.family_facets.{}", family.id)),
            "missing_family_style_facet",
            "Style kit must declare a family-scoped facet for the selected family.",
        );
        return Err(FamilyCompileError::ImplementationValidationFailed(report));
    };
    let style_prototypes = style_facet
        .part_prototypes
        .iter()
        .map(|prototype| (prototype.id.as_str(), prototype.role.as_str()))
        .collect::<BTreeMap<_, _>>();
    let style_details = style_facet
        .detail_modules
        .iter()
        .map(|module| module.id.as_str())
        .collect::<BTreeSet<_>>();

    for (id, fragment) in &style_impl.prototypes {
        validate_recipe_fragment_exports(
            &mut report,
            &format!("style_implementation.prototypes.{id}"),
            fragment,
        );
        if fragment.id != *id {
            push_report_issue(
                &mut report,
                Some(format!("style_implementation.prototypes.{id}.id")),
                "executable_fragment_id_mismatch",
                "Recipe fragment ID must match its map key.",
            );
        }
        match style_prototypes.get(id.as_str()) {
            Some(role) if *role == fragment.provided_role => {}
            Some(_) => push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.prototypes.{id}.provided_role"
                )),
                "executable_style_prototype_role_mismatch",
                "Executable style prototype role must match the style-kit prototype role.",
            ),
            None => push_report_issue(
                &mut report,
                Some(format!("style_implementation.prototypes.{id}")),
                "undeclared_executable_style_prototype",
                "Executable style prototypes must be declared by the style kit.",
            ),
        }
    }
    for (id, fragment) in &style_impl.detail_modules {
        validate_recipe_fragment_exports(
            &mut report,
            &format!("style_implementation.detail_modules.{id}"),
            fragment,
        );
        if fragment.id != *id {
            push_report_issue(
                &mut report,
                Some(format!("style_implementation.detail_modules.{id}.id")),
                "executable_fragment_id_mismatch",
                "Recipe fragment ID must match its map key.",
            );
        }
        if !style_details.contains(id.as_str()) {
            push_report_issue(
                &mut report,
                Some(format!("style_implementation.detail_modules.{id}")),
                "undeclared_executable_detail_module",
                "Executable detail modules must be declared by the style kit.",
            );
        }
    }

    for (role, fragment_id) in &style_impl.default_role_providers {
        if !role_ids.contains(role.as_str()) {
            push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.default_role_providers.{role}"
                )),
                "unknown_style_default_provider_role",
                "Style default provider role is not declared by the family.",
            );
        }
        if matches!(
            roles.get(role.as_str()).map(|role| role.provision),
            Some(RoleProvision::FamilyDefault | RoleProvision::Derived)
        ) {
            push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.default_role_providers.{role}"
                )),
                "style_default_provider_invalid_role_provision",
                "Style defaults can only provide style-required or style-optional roles.",
            );
        }
        match style_impl.prototypes.get(fragment_id) {
            Some(fragment) if fragment.provided_role == *role => {}
            Some(_) => push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.default_role_providers.{role}"
                )),
                "style_default_provider_role_mismatch",
                "Style default provider fragment role must match its family role.",
            ),
            None => push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.default_role_providers.{role}"
                )),
                "unknown_style_default_provider_fragment",
                "Style default provider fragment is not present in the style implementation.",
            ),
        }
    }
    for role in &family.part_roles {
        if role.required
            && role.provision == RoleProvision::StyleRequired
            && !style_impl.default_role_providers.contains_key(&role.id)
        {
            push_report_issue(
                &mut report,
                Some(format!(
                    "style_implementation.default_role_providers.{}",
                    role.id
                )),
                "missing_required_style_default_provider",
                "Style-required roles must declare an explicit executable default provider.",
            );
        }
    }

    for (role, fragment_id) in &family_impl.default_role_providers {
        if !role_ids.contains(role.as_str()) {
            push_report_issue(
                &mut report,
                Some(format!(
                    "family_implementation.default_role_providers.{role}"
                )),
                "unknown_default_provider_role",
                "Default provider role is not declared by the family.",
            );
        }
        if matches!(
            roles.get(role.as_str()).map(|role| role.provision),
            Some(RoleProvision::StyleRequired | RoleProvision::Derived)
        ) {
            push_report_issue(
                &mut report,
                Some(format!(
                    "family_implementation.default_role_providers.{role}"
                )),
                "family_default_provider_invalid_role_provision",
                "Family defaults can only provide family-default or style-optional roles.",
            );
        }
        match family_impl.fragments.get(fragment_id) {
            Some(fragment) if fragment.provided_role == *role => {}
            Some(_) => push_report_issue(
                &mut report,
                Some(format!(
                    "family_implementation.default_role_providers.{role}"
                )),
                "default_provider_role_mismatch",
                "Default provider fragment role must match its family role.",
            ),
            None => push_report_issue(
                &mut report,
                Some(format!(
                    "family_implementation.default_role_providers.{role}"
                )),
                "unknown_default_provider_fragment",
                "Default provider fragment is not present in the family implementation.",
            ),
        }
    }
    for (id, fragment) in &family_impl.fragments {
        validate_recipe_fragment_exports(
            &mut report,
            &format!("family_implementation.fragments.{id}"),
            fragment,
        );
        if fragment.id != *id {
            push_report_issue(
                &mut report,
                Some(format!("family_implementation.fragments.{id}.id")),
                "executable_fragment_id_mismatch",
                "Recipe fragment ID must match its map key.",
            );
        }
        if !role_ids.contains(fragment.provided_role.as_str()) {
            push_report_issue(
                &mut report,
                Some(format!(
                    "family_implementation.fragments.{id}.provided_role"
                )),
                "unknown_family_fragment_role",
                "Family-owned recipe fragments must provide a declared family role.",
            );
        }
    }

    for (index, binding) in family_impl.parameter_bindings.iter().enumerate() {
        validate_parameter_binding(
            binding,
            index,
            &slots,
            &roles,
            &style_impl.prototypes,
            &mut report,
        );
    }
    validate_parameter_binding_coverage(family, &family_impl.parameter_bindings, &mut report);
    validate_attachment_bindings(family, family_impl, &roles, &mut report);

    if report.is_valid() {
        Ok(())
    } else {
        Err(FamilyCompileError::ImplementationValidationFailed(report))
    }
}

fn validate_attachment_bindings(
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    roles: &BTreeMap<&str, &PartRole>,
    report: &mut FamilyValidationReport,
) {
    let rules = family
        .attachment_rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    for (index, binding) in family_impl.attachment_bindings.iter().enumerate() {
        let subject = format!("family_implementation.attachment_bindings.{index}");
        push_report_issue(
            report,
            Some(subject.clone()),
            "unsupported_fragment_attachment_binding",
            "Fragment attachment bindings are declared but not executable until port remapping is implemented.",
        );
        validate_identifier_report(
            report,
            Some(format!("{subject}.source_port")),
            &binding.source_port,
            "invalid_fragment_attachment_port",
        );
        validate_identifier_report(
            report,
            Some(format!("{subject}.destination_port")),
            &binding.destination_port,
            "invalid_fragment_attachment_port",
        );
        if !roles.contains_key(binding.source_role.as_str()) {
            push_report_issue(
                report,
                Some(format!("{subject}.source_role")),
                "unknown_fragment_attachment_source_role",
                "Fragment attachment bindings must reference declared family roles.",
            );
        }
        if !roles.contains_key(binding.destination_role.as_str()) {
            push_report_issue(
                report,
                Some(format!("{subject}.destination_role")),
                "unknown_fragment_attachment_destination_role",
                "Fragment attachment bindings must reference declared family roles.",
            );
        }
        let Some(rule) = rules.get(binding.family_attachment_rule.as_str()) else {
            push_report_issue(
                report,
                Some(format!("{subject}.family_attachment_rule")),
                "unknown_fragment_attachment_rule",
                "Fragment attachment bindings must implement a declared family attachment rule.",
            );
            continue;
        };
        if rule.from_role != binding.source_role || rule.to_role != binding.destination_role {
            push_report_issue(
                report,
                Some(format!("{subject}.family_attachment_rule")),
                "fragment_attachment_rule_role_mismatch",
                "Fragment attachment binding roles must match the family attachment rule direction.",
            );
        }
        if binding.offset.iter().any(|value| !value.is_finite()) {
            push_report_issue(
                report,
                Some(format!("{subject}.offset")),
                "non_finite_fragment_attachment_offset",
                "Fragment attachment offsets must be finite.",
            );
        }
    }
}

fn validate_recipe_fragment_exports(
    report: &mut FamilyValidationReport,
    subject: &str,
    fragment: &RecipeFragment,
) {
    if fragment.schema_version != RECIPE_FRAGMENT_SCHEMA_VERSION {
        push_report_issue(
            report,
            Some(format!("{subject}.schema_version")),
            "unsupported_recipe_fragment_schema",
            "Recipe fragment schema version is not supported.",
        );
    }
    let exports = &fragment.exports;
    if exports.role_occurrence_roots.is_empty() {
        push_report_issue(
            report,
            Some(format!("{subject}.exports.role_occurrence_roots")),
            "missing_role_occurrence_root",
            "Recipe fragments must export at least one role occurrence root.",
        );
    }
    let instance_ids = fragment
        .recipe
        .instances
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut root_ids = BTreeSet::new();
    for root in &exports.role_occurrence_roots {
        if !root_ids.insert(*root) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.role_occurrence_roots")),
                "duplicate_role_occurrence_root",
                "Role occurrence roots must be unique within one fragment.",
            );
        }
        if !instance_ids.contains(root) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.role_occurrence_roots")),
                "unknown_role_occurrence_root",
                "Role occurrence roots must exist in the fragment recipe.",
            );
        }
    }
    let mut internal_ids = BTreeSet::new();
    for internal in &exports.internal_roots {
        if !internal_ids.insert(*internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.internal_roots")),
                "duplicate_internal_instance",
                "Internal instances must be unique within one fragment.",
            );
        }
        if !instance_ids.contains(internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.internal_roots")),
                "unknown_internal_instance",
                "Internal instances must exist in the fragment recipe.",
            );
        }
        if root_ids.contains(internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.internal_roots")),
                "fragment_export_root_marked_internal",
                "A role occurrence root cannot also be listed as an internal instance.",
            );
        }
    }
    validate_disjoint_fragment_roots(
        report,
        subject,
        "exports.role_occurrence_roots",
        &fragment.recipe,
        &exports.role_occurrence_roots,
    );
    validate_disjoint_fragment_roots(
        report,
        subject,
        "exports.internal_roots",
        &fragment.recipe,
        &exports.internal_roots,
    );
    validate_disjoint_root_sets(
        report,
        subject,
        &fragment.recipe,
        &exports.role_occurrence_roots,
        &exports.internal_roots,
    );
    validate_fragment_socket_ports(report, subject, fragment);
    validate_fragment_surface_ports(report, subject, fragment);
    validate_fragment_port_namespace(report, subject, fragment);
    let mut covered_instances = BTreeSet::new();
    for root in &exports.role_occurrence_roots {
        if instance_ids.contains(root) {
            covered_instances.extend(collect_subtree_instances(&fragment.recipe, *root));
        }
    }
    for internal in &exports.internal_roots {
        if instance_ids.contains(internal) {
            covered_instances.extend(collect_subtree_instances(&fragment.recipe, *internal));
        }
    }
    for instance in instance_ids {
        if !covered_instances.contains(&instance) {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.internal_roots")),
                "unclassified_fragment_instance",
                "Every fragment instance must be under a role occurrence root or explicitly internal.",
            );
        }
    }
    validate_supported_fragment_contract(report, subject, fragment);
}

fn validate_fragment_socket_ports(
    report: &mut FamilyValidationReport,
    subject: &str,
    fragment: &RecipeFragment,
) {
    let mut port_ids = BTreeSet::new();
    for (index, port) in fragment.exports.socket_ports.iter().enumerate() {
        validate_fragment_port_id(
            report,
            format!("{subject}.exports.socket_ports.{index}.id"),
            &port.id,
            &mut port_ids,
            "duplicate_fragment_socket_port",
        );
        validate_identifier_list_report(
            report,
            &format!("{subject}.exports.socket_ports.{index}.compatibility_tags"),
            &port.compatibility_tags,
            "invalid_fragment_socket_port_tag",
        );
        let Some(instance) = fragment.recipe.instances.get(&port.local_occurrence_root) else {
            push_report_issue(
                report,
                Some(format!(
                    "{subject}.exports.socket_ports.{index}.local_occurrence_root"
                )),
                "unknown_fragment_socket_port_occurrence",
                "Socket ports must reference an occurrence inside the fragment recipe.",
            );
            continue;
        };
        let Some(definition) = fragment.recipe.definitions.get(&instance.definition) else {
            push_report_issue(
                report,
                Some(format!(
                    "{subject}.exports.socket_ports.{index}.local_occurrence_root"
                )),
                "fragment_socket_port_external_definition",
                "Socket port occurrence must reference a definition inside the fragment recipe.",
            );
            continue;
        };
        if !definition.sockets.contains_key(&port.local_socket) {
            push_report_issue(
                report,
                Some(format!(
                    "{subject}.exports.socket_ports.{index}.local_socket"
                )),
                "unknown_fragment_socket_port_socket",
                "Socket ports must reference a socket on the local occurrence definition.",
            );
        }
    }
}

fn validate_fragment_surface_ports(
    report: &mut FamilyValidationReport,
    subject: &str,
    fragment: &RecipeFragment,
) {
    let mut port_ids = BTreeSet::new();
    for (index, port) in fragment.exports.surface_ports.iter().enumerate() {
        validate_fragment_port_id(
            report,
            format!("{subject}.exports.surface_ports.{index}.id"),
            &port.id,
            &mut port_ids,
            "duplicate_fragment_surface_port",
        );
        validate_identifier_list_report(
            report,
            &format!("{subject}.exports.surface_ports.{index}.semantic_tags"),
            &port.semantic_tags,
            "invalid_fragment_surface_port_tag",
        );
        let definition_id = match &port.target {
            FragmentSurfaceTarget::Definition(definition) => Some(*definition),
            FragmentSurfaceTarget::Occurrence(instance) => fragment
                .recipe
                .instances
                .get(instance)
                .map(|part| part.definition),
        };
        let Some(definition_id) = definition_id else {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.surface_ports.{index}.target")),
                "unknown_fragment_surface_port_occurrence",
                "Surface ports must reference a definition or occurrence inside the fragment recipe.",
            );
            continue;
        };
        let Some(definition) = fragment.recipe.definitions.get(&definition_id) else {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.surface_ports.{index}.target")),
                "unknown_fragment_surface_port_definition",
                "Surface ports must reference a definition inside the fragment recipe.",
            );
            continue;
        };
        if !definition.regions.contains_key(&port.local_region) {
            push_report_issue(
                report,
                Some(format!(
                    "{subject}.exports.surface_ports.{index}.local_region"
                )),
                "unknown_fragment_surface_port_region",
                "Surface ports must reference a region on the local target definition.",
            );
        }
    }
}

fn validate_fragment_port_namespace(
    report: &mut FamilyValidationReport,
    subject: &str,
    fragment: &RecipeFragment,
) {
    let mut port_ids = BTreeMap::<&str, &'static str>::new();
    for port in &fragment.exports.socket_ports {
        port_ids.insert(port.id.as_str(), "socket");
    }
    for (index, port) in fragment.exports.surface_ports.iter().enumerate() {
        if let Some(previous_kind) = port_ids.insert(port.id.as_str(), "surface") {
            push_report_issue(
                report,
                Some(format!("{subject}.exports.surface_ports.{index}.id")),
                "duplicate_fragment_port_id",
                format!(
                    "Fragment port ID `{}` is already used by a {previous_kind} port.",
                    port.id
                ),
            );
        }
    }
}

fn validate_fragment_port_id(
    report: &mut FamilyValidationReport,
    subject: String,
    id: &str,
    seen: &mut BTreeSet<String>,
    duplicate_code: &'static str,
) {
    validate_non_empty_report(report, Some(subject.clone()), id, "empty_fragment_port_id");
    validate_identifier_report(
        report,
        Some(subject.clone()),
        id,
        "invalid_fragment_port_id",
    );
    if !seen.insert(id.to_owned()) {
        push_report_issue(
            report,
            Some(subject),
            duplicate_code,
            "Fragment port IDs must be unique within one port kind.",
        );
    }
}

fn validate_supported_fragment_contract(
    report: &mut FamilyValidationReport,
    subject: &str,
    fragment: &RecipeFragment,
) {
    if !fragment.recipe.constraints.is_empty()
        || !fragment.recipe.relationships.is_empty()
        || !fragment.recipe.instance_locks.is_empty()
        || !fragment.recipe.subtree_locks.is_empty()
        || !fragment.recipe.topology_locks.is_empty()
        || !fragment.recipe.variation.optional_instances.is_empty()
        || !fragment.recipe.variation.replacement_groups.is_empty()
        || !fragment.recipe.variation.count_ranges.is_empty()
        || !fragment
            .recipe
            .variation
            .parameter_range_overrides
            .is_empty()
        || !fragment.recipe.variation.semantic_cut_groups.is_empty()
    {
        push_report_issue(
            report,
            Some(format!("{subject}.recipe")),
            "unsupported_recipe_fragment_metadata",
            "Constraints, relationships, locks, and variation metadata are not remapped yet.",
        );
    }
    for (definition_id, definition) in &fragment.recipe.definitions {
        if !definition.geometry.operations.is_empty() {
            push_report_issue(
                report,
                Some(format!("{subject}.recipe.definitions.{}", definition_id.0)),
                "unsupported_recipe_fragment_modeling_operations",
                "Modeling operations are not remapped yet.",
            );
        }
        if !definition.sockets.is_empty() {
            push_report_issue(
                report,
                Some(format!(
                    "{subject}.recipe.definitions.{}.sockets",
                    definition_id.0
                )),
                "unsupported_recipe_fragment_sockets",
                "Fragment sockets are not remapped yet.",
            );
        }
    }
    if !fragment.exports.socket_ports.is_empty() {
        push_report_issue(
            report,
            Some(format!("{subject}.exports.socket_ports")),
            "unsupported_fragment_socket_ports",
            "Socket port exports are declared but not executable until socket remapping is implemented.",
        );
    }
    if fragment
        .recipe
        .instances
        .values()
        .any(|instance| instance.generated_by.is_some())
    {
        push_report_issue(
            report,
            Some(format!("{subject}.recipe.instances")),
            "unsupported_recipe_fragment_generated_provenance",
            "Generated-instance provenance is not remapped yet.",
        );
    }
}

fn validate_identifier_list_report(
    report: &mut FamilyValidationReport,
    subject: &str,
    values: &[String],
    code: &'static str,
) {
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        validate_identifier_report(report, Some(format!("{subject}.{index}")), value, code);
        if !seen.insert(value.as_str()) {
            push_report_issue(
                report,
                Some(format!("{subject}.{index}")),
                "duplicate_identifier",
                "Identifier values must be unique.",
            );
        }
    }
}

fn validate_identifier_report(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if !stable_identifier_is_valid(value) {
        push_report_issue(
            report,
            subject,
            code,
            "Stable identifiers must start with a lowercase ASCII letter, end with an alphanumeric character, and use non-repeated lowercase ASCII letters, digits, `_`, `-`, `.`, or `:` separators.",
        );
    }
}

fn validate_non_empty_report(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if value.trim().is_empty() {
        push_report_issue(report, subject, code, "Value cannot be empty.");
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

fn validate_disjoint_fragment_roots(
    report: &mut FamilyValidationReport,
    subject: &str,
    field: &str,
    recipe: &AssetRecipe,
    roots: &[PartInstanceId],
) {
    for (left_index, left) in roots.iter().enumerate() {
        let left_subtree = collect_subtree_instances(recipe, *left)
            .into_iter()
            .collect::<BTreeSet<_>>();
        for right in roots.iter().skip(left_index + 1) {
            let right_subtree = collect_subtree_instances(recipe, *right)
                .into_iter()
                .collect::<BTreeSet<_>>();
            if left_subtree.contains(right) || right_subtree.contains(left) {
                push_report_issue(
                    report,
                    Some(format!("{subject}.{field}")),
                    "nested_fragment_export_root",
                    "Fragment export roots must not be descendants of other export roots.",
                );
            } else if !left_subtree.is_disjoint(&right_subtree) {
                push_report_issue(
                    report,
                    Some(format!("{subject}.{field}")),
                    "overlapping_fragment_export_root",
                    "Fragment export root subtrees must be pairwise disjoint.",
                );
            }
        }
    }
}

fn validate_disjoint_root_sets(
    report: &mut FamilyValidationReport,
    subject: &str,
    recipe: &AssetRecipe,
    occurrence_roots: &[PartInstanceId],
    internal_roots: &[PartInstanceId],
) {
    for occurrence in occurrence_roots {
        let occurrence_subtree = collect_subtree_instances(recipe, *occurrence)
            .into_iter()
            .collect::<BTreeSet<_>>();
        for internal in internal_roots {
            let internal_subtree = collect_subtree_instances(recipe, *internal)
                .into_iter()
                .collect::<BTreeSet<_>>();
            if occurrence_subtree.contains(internal) || internal_subtree.contains(occurrence) {
                push_report_issue(
                    report,
                    Some(format!("{subject}.exports.internal_roots")),
                    "internal_instance_overlaps_occurrence_root",
                    "Internal roots must describe helper subtrees outside exported occurrence roots.",
                );
            } else if !occurrence_subtree.is_disjoint(&internal_subtree) {
                push_report_issue(
                    report,
                    Some(format!("{subject}.exports.internal_roots")),
                    "internal_instance_overlaps_occurrence_root",
                    "Internal root subtrees must not overlap exported occurrence root subtrees.",
                );
            }
        }
    }
}

fn validate_parameter_binding(
    binding: &ParameterBinding,
    index: usize,
    slots: &BTreeMap<&str, &shape_family::FamilyParameterSlot>,
    roles: &BTreeMap<&str, &PartRole>,
    style_prototypes: &BTreeMap<String, RecipeFragment>,
    report: &mut FamilyValidationReport,
) {
    match binding {
        ParameterBinding::Scalar {
            slot,
            role,
            transform,
            ..
        } => {
            let Some(parameter_slot) = slots.get(slot.as_str()) else {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.slot"
                    )),
                    "unknown_parameter_binding_slot",
                    "Parameter binding slot must be declared by the family.",
                );
                return;
            };
            validate_executable_parameter_binding(report, index, parameter_slot);
            validate_binding_role(report, index, role, roles);
            let valid = matches!(
                (&parameter_slot.kind, transform),
                (FamilyParameterKind::Count, ScalarTransform::IntegerCount)
                    | (
                        FamilyParameterKind::Length { .. }
                            | FamilyParameterKind::Ratio
                            | FamilyParameterKind::Angle { .. }
                            | FamilyParameterKind::Custom(_),
                        ScalarTransform::Direct
                            | ScalarTransform::ScaleOffset { .. }
                            | ScalarTransform::Ratio { .. },
                    )
            );
            if !valid {
                push_report_issue(
                    report,
                    Some(format!("family_implementation.parameter_bindings.{index}")),
                    "incompatible_parameter_binding_kind",
                    "Scalar bindings must match the semantic parameter kind.",
                );
            }
            validate_scalar_transform(report, index, *transform);
        }
        ParameterBinding::TogglePartPresence { slot, role } => {
            let Some(parameter_slot) = slots.get(slot.as_str()) else {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.slot"
                    )),
                    "unknown_parameter_binding_slot",
                    "Parameter binding slot must be declared by the family.",
                );
                return;
            };
            validate_executable_parameter_binding(report, index, parameter_slot);
            validate_binding_role(report, index, role, roles);
            if !matches!(parameter_slot.kind, FamilyParameterKind::Toggle) {
                push_report_issue(
                    report,
                    Some(format!("family_implementation.parameter_bindings.{index}")),
                    "incompatible_parameter_binding_kind",
                    "Toggle part-presence bindings require a Toggle parameter slot.",
                );
            }
        }
        ParameterBinding::ChoiceToPrototype {
            slot,
            role,
            choices,
        } => {
            let Some(parameter_slot) = slots.get(slot.as_str()) else {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.slot"
                    )),
                    "unknown_parameter_binding_slot",
                    "Parameter binding slot must be declared by the family.",
                );
                return;
            };
            validate_executable_parameter_binding(report, index, parameter_slot);
            validate_binding_role(report, index, role, roles);
            let FamilyParameterKind::Choice(declared_choices) = &parameter_slot.kind else {
                push_report_issue(
                    report,
                    Some(format!("family_implementation.parameter_bindings.{index}")),
                    "incompatible_parameter_binding_kind",
                    "Choice-to-prototype bindings require a Choice parameter slot.",
                );
                return;
            };
            let declared_choices = declared_choices
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if !matches!(
                roles.get(role.as_str()).map(|role| role.provision),
                Some(RoleProvision::StyleRequired | RoleProvision::FamilyOrStyle)
            ) {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.role"
                    )),
                    "choice_binding_invalid_role_provision",
                    "Choice-to-prototype bindings require a style-provided or style-optional role.",
                );
            }
            for choice in &declared_choices {
                let Some(fragment_id) = choices.get(*choice) else {
                    push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.choices.{choice}"
                        )),
                        "missing_choice_binding_value",
                        "Every declared family choice must map to an executable prototype.",
                    );
                    continue;
                };
                match style_prototypes.get(fragment_id) {
                    Some(fragment) if fragment.provided_role == *role => {}
                    Some(_) => push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.choices.{choice}"
                        )),
                        "choice_binding_role_mismatch",
                        "Choice binding prototype role must match the binding role.",
                    ),
                    None => push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.choices.{choice}"
                        )),
                        "unknown_choice_binding_prototype",
                        "Choice binding prototype must exist in the style implementation.",
                    ),
                }
            }
            for choice in choices.keys() {
                if !declared_choices.contains(choice.as_str()) {
                    push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.choices.{choice}"
                        )),
                        "undeclared_choice_binding_value",
                        "Choice binding values must be declared by the family parameter slot.",
                    );
                }
            }
        }
    }
}

fn validate_executable_parameter_binding(
    report: &mut FamilyValidationReport,
    index: usize,
    slot: &shape_family::FamilyParameterSlot,
) {
    if slot.execution_policy != ParameterExecutionPolicy::RequiredBinding {
        push_report_issue(
            report,
            Some(format!(
                "family_implementation.parameter_bindings.{index}.slot"
            )),
            "non_executable_parameter_binding",
            "Only RequiredBinding family parameters may be consumed by executable geometry bindings.",
        );
    }
}

fn validate_scalar_transform(
    report: &mut FamilyValidationReport,
    index: usize,
    transform: ScalarTransform,
) {
    match transform {
        ScalarTransform::Direct | ScalarTransform::IntegerCount => {}
        ScalarTransform::ScaleOffset { scale, offset } => {
            if !scale.is_finite() || !offset.is_finite() {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.transform"
                    )),
                    "non_finite_scalar_transform",
                    "Scalar transform values must be finite.",
                );
            }
            if scale == 0.0 {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.transform"
                    )),
                    "degenerate_scalar_transform",
                    "Scale-offset bindings must not collapse every input to a constant.",
                );
            }
        }
        ScalarTransform::Ratio { minimum, maximum } => {
            if !minimum.is_finite() || !maximum.is_finite() {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.transform"
                    )),
                    "non_finite_scalar_transform",
                    "Scalar transform values must be finite.",
                );
            }
            if minimum >= maximum {
                push_report_issue(
                    report,
                    Some(format!(
                        "family_implementation.parameter_bindings.{index}.transform"
                    )),
                    "degenerate_scalar_transform",
                    "Ratio bindings must map into a non-empty increasing output range.",
                );
            }
        }
    }
}

fn validate_parameter_binding_coverage(
    family: &AssetFamilySchema,
    bindings: &[ParameterBinding],
    report: &mut FamilyValidationReport,
) {
    let mut bound_slots = BTreeSet::new();
    let mut provider_roles = BTreeMap::<&str, usize>::new();
    let mut presence_roles = BTreeMap::<&str, usize>::new();
    for (index, binding) in bindings.iter().enumerate() {
        match binding {
            ParameterBinding::Scalar { slot, .. } => {
                bound_slots.insert(slot.as_str());
            }
            ParameterBinding::TogglePartPresence { slot, role } => {
                bound_slots.insert(slot.as_str());
                if let Some(previous) = presence_roles.insert(role.as_str(), index) {
                    push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.role"
                        )),
                        "conflicting_presence_binding",
                        format!(
                            "Role `{role}` already has a presence binding at index {previous}."
                        ),
                    );
                }
            }
            ParameterBinding::ChoiceToPrototype { slot, role, .. } => {
                bound_slots.insert(slot.as_str());
                if let Some(previous) = provider_roles.insert(role.as_str(), index) {
                    push_report_issue(
                        report,
                        Some(format!(
                            "family_implementation.parameter_bindings.{index}.role"
                        )),
                        "conflicting_provider_selection_binding",
                        format!(
                            "Role `{role}` already has a provider-selection binding at index {previous}."
                        ),
                    );
                }
            }
        }
    }
    for slot in &family.parameter_slots {
        if slot.execution_policy == ParameterExecutionPolicy::RequiredBinding
            && !bound_slots.contains(slot.id.as_str())
        {
            push_report_issue(
                report,
                Some(format!(
                    "family_implementation.parameter_bindings.{}",
                    slot.id
                )),
                "missing_required_parameter_binding",
                "Required family parameter slots must have at least one executable binding.",
            );
        }
    }
}

fn validate_binding_role(
    report: &mut FamilyValidationReport,
    index: usize,
    role: &str,
    roles: &BTreeMap<&str, &PartRole>,
) {
    if !roles.contains_key(role) {
        push_report_issue(
            report,
            Some(format!(
                "family_implementation.parameter_bindings.{index}.role"
            )),
            "unknown_parameter_binding_role",
            "Parameter binding role must be declared by the family.",
        );
    }
}

fn validate_request(
    family: &AssetFamilySchema,
    request: &FamilyInstantiationRequest,
) -> Result<(), FamilyCompileError> {
    let mut report = FamilyValidationReport::default();
    let slots = family
        .parameter_slots
        .iter()
        .map(|slot| (slot.id.as_str(), slot))
        .collect::<BTreeMap<_, _>>();
    for (key, value) in &request.parameters {
        let Some(slot) = slots.get(key.as_str()) else {
            push_report_issue(
                &mut report,
                Some(format!("request.parameters.{key}")),
                "unknown_request_parameter",
                "Request parameter is not declared by the family.",
            );
            continue;
        };
        validate_request_value(&mut report, slot, value);
    }
    if report.is_valid() {
        Ok(())
    } else {
        Err(FamilyCompileError::RequestValidationFailed(report))
    }
}

fn validate_request_value(
    report: &mut FamilyValidationReport,
    slot: &shape_family::FamilyParameterSlot,
    value: &FamilyValue,
) {
    match (&slot.kind, value) {
        (
            FamilyParameterKind::Length { .. } | FamilyParameterKind::Angle { .. },
            FamilyValue::Scalar(value),
        ) => {
            validate_request_numeric(report, slot, *value);
        }
        (FamilyParameterKind::Ratio, FamilyValue::Scalar(value)) => {
            validate_request_numeric(report, slot, *value);
            if !(0.0..=1.0).contains(value) {
                push_report_issue(
                    report,
                    Some(format!("request.parameters.{}", slot.id)),
                    "request_ratio_out_of_range",
                    "Ratio request values must stay within 0..1.",
                );
            }
        }
        (FamilyParameterKind::Count, FamilyValue::Integer(value)) => {
            validate_request_numeric(report, slot, *value as f32);
        }
        (FamilyParameterKind::Toggle, FamilyValue::Toggle(_)) => {}
        (FamilyParameterKind::Choice(choices), FamilyValue::Choice(value)) => {
            if !choices.iter().any(|choice| choice == value) {
                push_report_issue(
                    report,
                    Some(format!("request.parameters.{}", slot.id)),
                    "request_choice_not_declared",
                    "Choice request values must be declared by the family parameter slot.",
                );
            }
        }
        (FamilyParameterKind::Custom(_), FamilyValue::Scalar(value)) => {
            validate_request_numeric(report, slot, *value);
        }
        (FamilyParameterKind::Custom(_), FamilyValue::Integer(value)) => {
            validate_request_numeric(report, slot, *value as f32);
        }
        _ => push_report_issue(
            report,
            Some(format!("request.parameters.{}", slot.id)),
            "request_parameter_type_mismatch",
            "Request parameter value type must match the family parameter kind.",
        ),
    }
}

fn validate_request_numeric(
    report: &mut FamilyValidationReport,
    slot: &shape_family::FamilyParameterSlot,
    value: f32,
) {
    if !value.is_finite() {
        push_report_issue(
            report,
            Some(format!("request.parameters.{}", slot.id)),
            "request_parameter_non_finite",
            "Request numeric values must be finite.",
        );
        return;
    }
    if let Some(range) = slot.range {
        validate_request_range(report, slot, value, range);
    }
}

fn validate_request_range(
    report: &mut FamilyValidationReport,
    slot: &shape_family::FamilyParameterSlot,
    value: f32,
    range: ParameterRange,
) {
    if value < range.minimum || value > range.maximum {
        push_report_issue(
            report,
            Some(format!("request.parameters.{}", slot.id)),
            "request_parameter_out_of_range",
            "Request parameter value must fall within the family parameter range.",
        );
    }
}

fn resolve_choice_overrides(
    style_impl: &StyleImplementation,
    bindings: &[ParameterBinding],
    request: &FamilyInstantiationRequest,
) -> Result<BTreeMap<String, String>, FamilyCompileError> {
    let mut overrides = BTreeMap::new();
    for binding in bindings {
        let ParameterBinding::ChoiceToPrototype {
            slot,
            role,
            choices,
        } = binding
        else {
            continue;
        };
        let Some(value) = request.parameters.get(slot) else {
            continue;
        };
        let FamilyValue::Choice(choice) = value else {
            return Err(FamilyCompileError::IncompatibleParameterValue { slot: slot.clone() });
        };
        let Some(fragment_id) = choices.get(choice) else {
            return Err(FamilyCompileError::IncompatibleParameterValue { slot: slot.clone() });
        };
        match style_impl.prototypes.get(fragment_id) {
            Some(fragment) if fragment.provided_role == *role => {}
            _ => {
                return Err(FamilyCompileError::UnknownFragment {
                    role: role.clone(),
                    fragment: fragment_id.clone(),
                });
            }
        }
        overrides.insert(role.clone(), fragment_id.clone());
    }
    Ok(overrides)
}

fn resolve_presence_overrides(
    bindings: &[ParameterBinding],
    request: &FamilyInstantiationRequest,
) -> Result<BTreeSet<String>, FamilyCompileError> {
    let mut roles = BTreeSet::new();
    for binding in bindings {
        let ParameterBinding::TogglePartPresence { slot, role } = binding else {
            continue;
        };
        let Some(value) = request.parameters.get(slot) else {
            continue;
        };
        let FamilyValue::Toggle(enabled) = value else {
            return Err(FamilyCompileError::IncompatibleParameterValue { slot: slot.clone() });
        };
        if *enabled {
            roles.insert(role.clone());
        }
    }
    Ok(roles)
}

fn select_role_providers(
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
    choice_overrides: &BTreeMap<String, String>,
    presence_overrides: &BTreeSet<String>,
) -> Result<BTreeMap<String, ProviderSelection>, FamilyCompileError> {
    let mut providers = BTreeMap::new();
    for role in &family.part_roles {
        if !role.required
            && !choice_overrides.contains_key(&role.id)
            && !presence_overrides.contains(&role.id)
        {
            continue;
        }
        if role.provision == RoleProvision::Derived {
            continue;
        }
        let provider = choice_overrides
            .get(&role.id)
            .map(|fragment| ProviderSelection {
                fragment: fragment.clone(),
                source: ProviderSource::StylePrototype,
            })
            .or_else(|| match role.provision {
                RoleProvision::FamilyDefault => family_impl
                    .default_role_providers
                    .get(&role.id)
                    .map(|fragment| ProviderSelection {
                        fragment: fragment.clone(),
                        source: ProviderSource::FamilyDefault,
                    }),
                RoleProvision::StyleRequired => style_impl
                    .default_role_providers
                    .get(&role.id)
                    .map(|fragment| ProviderSelection {
                        fragment: fragment.clone(),
                        source: ProviderSource::StylePrototype,
                    }),
                RoleProvision::FamilyOrStyle => style_impl
                    .default_role_providers
                    .get(&role.id)
                    .map(|fragment| ProviderSelection {
                        fragment: fragment.clone(),
                        source: ProviderSource::StylePrototype,
                    })
                    .or_else(|| {
                        family_impl
                            .default_role_providers
                            .get(&role.id)
                            .map(|fragment| ProviderSelection {
                                fragment: fragment.clone(),
                                source: ProviderSource::FamilyDefault,
                            })
                    }),
                RoleProvision::Derived => None,
            });
        let Some(provider) = provider else {
            return Err(FamilyCompileError::MissingRoleProvider {
                role: role.id.clone(),
            });
        };
        providers.insert(role.id.clone(), provider);
    }
    Ok(providers)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ProviderSource {
    FamilyDefault,
    StylePrototype,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderSelection {
    fragment: String,
    source: ProviderSource,
}

fn find_fragment<'a>(
    family_impl: &'a FamilyImplementation,
    style_impl: &'a StyleImplementation,
    role: &str,
    selection: &ProviderSelection,
) -> Result<&'a RecipeFragment, FamilyCompileError> {
    let fragment = match selection.source {
        ProviderSource::FamilyDefault => family_impl.fragments.get(&selection.fragment),
        ProviderSource::StylePrototype => style_impl.prototypes.get(&selection.fragment),
    };
    fragment
        .filter(|fragment| fragment.provided_role == role)
        .ok_or_else(|| FamilyCompileError::UnknownFragment {
            role: role.to_owned(),
            fragment: selection.fragment.clone(),
        })
}

#[derive(Default)]
struct MergeState {
    scalar_paths: BTreeMap<(String, String), String>,
    role_occurrence_roots: BTreeMap<String, Vec<PartInstanceId>>,
    role_internal_instances: BTreeMap<String, Vec<PartInstanceId>>,
    remap_reports: Vec<remap::FragmentRemapReport>,
}

fn merge_fragment(
    target: &mut AssetRecipe,
    fragment: &RecipeFragment,
    state: &mut MergeState,
) -> Result<(), FamilyCompileError> {
    validate_supported_fragment(fragment)?;
    let fragment_report = validate_asset_recipe(&fragment.recipe);
    if !fragment_report.is_valid() {
        return Err(FamilyCompileError::FragmentValidationFailed {
            fragment: fragment.id.clone(),
            report: fragment_report,
        });
    }

    let prepared_remap = remap::ids::prepare_fragment_id_remap(target, fragment);
    let fragment_remap = &prepared_remap.remap;

    for (old_id, definition) in &fragment.recipe.definitions {
        let mut cloned = definition.clone();
        cloned.id = *fragment_remap
            .definitions
            .get(old_id)
            .ok_or_else(|| unsupported_fragment(&fragment.id, "definition ID was not allocated"))?;
        cloned.regions = remap_regions(&definition.regions, &fragment_remap.regions);
        target.definitions.insert(cloned.id, cloned);
    }

    for (old_id, instance) in &fragment.recipe.instances {
        let mut cloned = instance.clone();
        cloned.id = *fragment_remap
            .instances
            .get(old_id)
            .ok_or_else(|| unsupported_fragment(&fragment.id, "instance ID was not allocated"))?;
        cloned.definition = *fragment_remap
            .definitions
            .get(&instance.definition)
            .ok_or_else(|| {
                unsupported_fragment(&fragment.id, "instance references an unmapped definition")
            })?;
        cloned.parent = instance
            .parent
            .map(|parent| {
                fragment_remap
                    .instances
                    .get(&parent)
                    .copied()
                    .ok_or_else(|| {
                        unsupported_fragment(&fragment.id, "instance references an unmapped parent")
                    })
            })
            .transpose()?;
        target.instances.insert(cloned.id, cloned.clone());
    }
    for root in &fragment.recipe.root_instances {
        let Some(root) = fragment_remap.instances.get(root).copied() else {
            return Err(unsupported_fragment(
                &fragment.id,
                "root instance was not remapped",
            ));
        };
        target.root_instances.push(root);
    }
    let occurrence_roots = remap_fragment_instance_list(
        fragment,
        &fragment.exports.role_occurrence_roots,
        &fragment_remap.instances,
        "role occurrence root",
    )?;
    let internal_instances = remap_fragment_instance_list(
        fragment,
        &fragment.exports.internal_roots,
        &fragment_remap.instances,
        "internal instance",
    )?;
    state
        .role_occurrence_roots
        .entry(fragment.provided_role.clone())
        .or_default()
        .extend(occurrence_roots);
    state
        .role_internal_instances
        .entry(fragment.provided_role.clone())
        .or_default()
        .extend(internal_instances);

    for (old_id, parameter) in &fragment.recipe.parameters {
        let mut cloned = parameter.clone();
        cloned.id = *fragment_remap
            .parameters
            .get(old_id)
            .ok_or_else(|| unsupported_fragment(&fragment.id, "parameter ID was not allocated"))?;
        cloned.path = remap_scalar_path(
            &parameter.path,
            &fragment_remap.definitions,
            &fragment_remap.instances,
        )?;
        let new_parameter_id = cloned.id;
        state.scalar_paths.insert(
            (fragment.provided_role.clone(), parameter.path.clone()),
            cloned.path.clone(),
        );
        target.parameters.insert(new_parameter_id, cloned);
        if fragment.recipe.locks.contains(old_id) {
            target.locks.insert(new_parameter_id);
        }
    }
    state.remap_reports.push(remap::FragmentRemapReport {
        fragment_id: fragment.id.clone(),
        remap: prepared_remap.remap,
        allocated: prepared_remap.allocated,
        warnings: Vec::new(),
    });
    Ok(())
}

fn remap_fragment_instance_list(
    fragment: &RecipeFragment,
    local_instances: &[PartInstanceId],
    instance_map: &BTreeMap<PartInstanceId, PartInstanceId>,
    label: &str,
) -> Result<Vec<PartInstanceId>, FamilyCompileError> {
    local_instances
        .iter()
        .map(|instance| {
            instance_map.get(instance).copied().ok_or_else(|| {
                unsupported_fragment(
                    &fragment.id,
                    &format!("{label} was not remapped from the fragment recipe"),
                )
            })
        })
        .collect()
}

fn validate_supported_fragment(fragment: &RecipeFragment) -> Result<(), FamilyCompileError> {
    if !fragment.recipe.constraints.is_empty()
        || !fragment.recipe.relationships.is_empty()
        || !fragment.recipe.instance_locks.is_empty()
        || !fragment.recipe.subtree_locks.is_empty()
        || !fragment.recipe.topology_locks.is_empty()
        || !fragment.recipe.variation.optional_instances.is_empty()
        || !fragment.recipe.variation.replacement_groups.is_empty()
        || !fragment.recipe.variation.count_ranges.is_empty()
        || !fragment
            .recipe
            .variation
            .parameter_range_overrides
            .is_empty()
        || !fragment.recipe.variation.semantic_cut_groups.is_empty()
    {
        return Err(unsupported_fragment(
            &fragment.id,
            "constraints, relationships, locks, and variation metadata are not remapped yet",
        ));
    }
    for definition in fragment.recipe.definitions.values() {
        if !definition.geometry.operations.is_empty() || !definition.sockets.is_empty() {
            return Err(unsupported_fragment(
                &fragment.id,
                "modeling operations and sockets are not remapped yet",
            ));
        }
    }
    if fragment
        .recipe
        .instances
        .values()
        .any(|instance| instance.generated_by.is_some())
    {
        return Err(unsupported_fragment(
            &fragment.id,
            "generated-instance provenance is not remapped yet",
        ));
    }
    Ok(())
}

fn unsupported_fragment(fragment: &str, reason: &str) -> FamilyCompileError {
    FamilyCompileError::UnsupportedFragment {
        fragment: fragment.to_owned(),
        reason: reason.to_owned(),
    }
}

fn remap_regions(
    regions: &BTreeMap<RegionId, SurfaceRegionSpec>,
    region_map: &BTreeMap<RegionId, RegionId>,
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions
        .iter()
        .map(|(old_id, region)| {
            let new_id = region_map[old_id];
            let mut cloned = region.clone();
            cloned.id = new_id;
            (new_id, cloned)
        })
        .collect()
}

fn remap_scalar_path(
    path: &str,
    definition_map: &BTreeMap<PartDefinitionId, PartDefinitionId>,
    instance_map: &BTreeMap<PartInstanceId, PartInstanceId>,
) -> Result<String, FamilyCompileError> {
    let parts = path.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        ["definition", id, rest @ ..] => {
            let old = PartDefinitionId(parse_id(path, id)?);
            let Some(new) = definition_map.get(&old) else {
                return Err(FamilyCompileError::UnsupportedScalarPath {
                    path: path.to_owned(),
                });
            };
            Ok(format!("definition.{}.{}", new.0, rest.join(".")))
        }
        ["instance", id, rest @ ..] => {
            let old = PartInstanceId(parse_id(path, id)?);
            let Some(new) = instance_map.get(&old) else {
                return Err(FamilyCompileError::UnsupportedScalarPath {
                    path: path.to_owned(),
                });
            };
            Ok(format!("instance.{}.{}", new.0, rest.join(".")))
        }
        _ => Err(FamilyCompileError::UnsupportedScalarPath {
            path: path.to_owned(),
        }),
    }
}

fn parse_id(path: &str, raw: &str) -> Result<u64, FamilyCompileError> {
    raw.parse()
        .map_err(|_| FamilyCompileError::UnsupportedScalarPath {
            path: path.to_owned(),
        })
}

fn apply_parameter_bindings(
    recipe: &mut AssetRecipe,
    bindings: &[ParameterBinding],
    request: &FamilyInstantiationRequest,
    state: &MergeState,
    applications: &mut Vec<ParameterApplication>,
) -> Result<(), FamilyCompileError> {
    for binding in bindings {
        match binding {
            ParameterBinding::Scalar {
                slot,
                role,
                local_path,
                transform,
            } => {
                let Some(value) = request.parameters.get(slot) else {
                    continue;
                };
                let scalar = scalar_value(slot, value, *transform)?;
                let Some(path) = state.scalar_paths.get(&(role.clone(), local_path.clone())) else {
                    return Err(FamilyCompileError::UnresolvedParameterBinding {
                        slot: slot.clone(),
                    });
                };
                shape_asset::set_scalar(recipe, path, scalar).map_err(|_| {
                    FamilyCompileError::UnresolvedParameterBinding { slot: slot.clone() }
                })?;
                applications.push(ParameterApplication {
                    slot: slot.clone(),
                    role: role.clone(),
                    target: path.clone(),
                    value: format!("{scalar:.6}"),
                });
            }
            ParameterBinding::TogglePartPresence { slot, role } => {
                let Some(value) = request.parameters.get(slot) else {
                    continue;
                };
                let FamilyValue::Toggle(enabled) = value else {
                    return Err(FamilyCompileError::IncompatibleParameterValue {
                        slot: slot.clone(),
                    });
                };
                if let Some(roots) = state.role_occurrence_roots.get(role) {
                    let mut instances = roots
                        .iter()
                        .flat_map(|root| collect_subtree_instances(recipe, *root))
                        .collect::<BTreeSet<_>>();
                    if let Some(internal_roots) = state.role_internal_instances.get(role) {
                        instances.extend(
                            internal_roots
                                .iter()
                                .flat_map(|root| collect_subtree_instances(recipe, *root)),
                        );
                    }
                    for instance in instances {
                        if let Some(part) = recipe.instances.get_mut(&instance) {
                            part.enabled = *enabled;
                        }
                    }
                } else if *enabled {
                    return Err(FamilyCompileError::UnresolvedParameterBinding {
                        slot: slot.clone(),
                    });
                }
                applications.push(ParameterApplication {
                    slot: slot.clone(),
                    role: role.clone(),
                    target: "role_presence".to_owned(),
                    value: enabled.to_string(),
                });
            }
            ParameterBinding::ChoiceToPrototype { .. } => {}
        }
    }
    Ok(())
}

fn validate_role_cardinality(
    family: &AssetFamilySchema,
    recipe: &AssetRecipe,
    state: &MergeState,
) -> Result<(), FamilyCompileError> {
    let mut report = FamilyValidationReport::default();
    for role in &family.part_roles {
        if role.provision == RoleProvision::Derived {
            continue;
        }
        let count = state
            .role_occurrence_roots
            .get(&role.id)
            .into_iter()
            .flat_map(|roots| roots.iter())
            .filter(|instance| is_effectively_enabled(recipe, **instance))
            .count() as u32;
        validate_role_count(&mut report, role, count);
    }
    if report.is_valid() {
        Ok(())
    } else {
        Err(FamilyCompileError::RoleValidationFailed(report))
    }
}

fn collect_subtree_instances(recipe: &AssetRecipe, root: PartInstanceId) -> Vec<PartInstanceId> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(instance_id) = stack.pop() {
        if !seen.insert(instance_id) || !recipe.instances.contains_key(&instance_id) {
            continue;
        }
        result.push(instance_id);
        for (child_id, child) in &recipe.instances {
            if child.parent == Some(instance_id) {
                stack.push(*child_id);
            }
        }
    }
    result
}

fn is_effectively_enabled(recipe: &AssetRecipe, instance: PartInstanceId) -> bool {
    let mut current = Some(instance);
    let mut seen = BTreeSet::new();
    while let Some(instance_id) = current {
        if !seen.insert(instance_id) {
            return false;
        }
        let Some(part) = recipe.instances.get(&instance_id) else {
            return false;
        };
        if !part.enabled {
            return false;
        }
        current = part.parent;
    }
    true
}

fn validate_role_count(report: &mut FamilyValidationReport, role: &PartRole, count: u32) {
    let valid = match role.multiplicity {
        RoleMultiplicity::Single => count == 1,
        RoleMultiplicity::Optional => count <= 1 && (!role.required || count == 1),
        RoleMultiplicity::Range { min, max } => (min..=max).contains(&count),
        RoleMultiplicity::Repeated => !role.required || count > 0,
    };
    if !valid {
        push_report_issue(
            report,
            Some(format!("roles.{}", role.id)),
            "family_role_cardinality_unsatisfied",
            format!(
                "Instantiated role `{}` has {count} enabled occurrence(s), outside the family multiplicity contract.",
                role.id
            ),
        );
    }
}

fn scalar_value(
    slot: &str,
    value: &FamilyValue,
    transform: ScalarTransform,
) -> Result<f32, FamilyCompileError> {
    match (value, transform) {
        (FamilyValue::Scalar(value), ScalarTransform::Direct) => Ok(*value),
        (FamilyValue::Scalar(value), ScalarTransform::ScaleOffset { scale, offset }) => {
            Ok(*value * scale + offset)
        }
        (FamilyValue::Scalar(value), ScalarTransform::Ratio { minimum, maximum }) => {
            Ok(minimum + (maximum - minimum) * *value)
        }
        (FamilyValue::Integer(value), ScalarTransform::IntegerCount) => Ok(*value as f32),
        _ => Err(FamilyCompileError::IncompatibleParameterValue {
            slot: slot.to_owned(),
        }),
    }
}

/// Convenience helper for tests and small pack declarations.
#[must_use]
pub fn scalar_parameter(
    id: u64,
    path: impl Into<String>,
    label: impl Into<String>,
    minimum: f32,
    maximum: f32,
    step: f32,
    topology_changing: bool,
) -> ParameterDescriptor {
    let label = label.into();
    ParameterDescriptor {
        id: shape_asset::ParameterId(id),
        path: path.into(),
        label: label.clone(),
        group: "Family".to_owned(),
        minimum,
        maximum,
        step,
        mutation_sigma: step,
        topology_changing,
        beginner_description: format!("Family binding for {label}."),
    }
}

fn push_report_issue(
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
