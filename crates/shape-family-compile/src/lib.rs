#![forbid(unsafe_code)]

//! Executable bindings from asset-family/style-kit contracts to concrete recipes.
//!
//! This crate deliberately keeps the first compiler small: recipe fragments are
//! deterministic Shape Lab recipes, parameter bindings are simple scalar or
//! presence/prototype choices, and unsupported fragment features are rejected
//! instead of implicitly remapped.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetId, AssetRecipe, AssetValidationReport, ParameterDescriptor, PartDefinitionId,
    PartInstanceId, RegionId, SurfaceRegionSpec, validate_asset_recipe,
};
use shape_compile::{AssetArtifact, CompileError, CompileValidationReport, compile_asset};
use shape_family::{
    AssetFamilySchema, FamilyDefaultValue, FamilyParameterKind, FamilyValidationIssue,
    FamilyValidationReport, ParameterRange, PartRole, RoleMultiplicity, RoleProvision, StyleKit,
    validate_family_style_compatibility, validate_family_style_completeness,
};
use thiserror::Error;

/// Current schema version for executable family implementations.
pub const FAMILY_IMPLEMENTATION_SCHEMA_VERSION: u32 = 1;

/// Current schema version for executable style implementations.
pub const STYLE_IMPLEMENTATION_SCHEMA_VERSION: u32 = 1;

/// Current schema version for executable recipe fragments.
pub const RECIPE_FRAGMENT_SCHEMA_VERSION: u32 = 1;

/// Executable family binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyImplementation {
    /// Executable family implementation schema version.
    pub schema_version: u32,
    /// Family ID implemented by this binding.
    pub family_id: String,
    /// Base recipe used as the deterministic merge target.
    pub base_recipe: AssetRecipe,
    /// Optional role-level binding metadata.
    pub role_bindings: BTreeMap<String, RoleBinding>,
    /// Parameter bindings applied after fragments are merged.
    pub parameter_bindings: Vec<ParameterBinding>,
    /// Placeholder variant binding keys reserved for later search integration.
    pub variant_bindings: BTreeMap<String, VariantBinding>,
    /// Family-owned default providers keyed by role ID.
    pub default_role_providers: BTreeMap<String, String>,
    /// Family-owned recipe fragments keyed by fragment ID.
    pub fragments: BTreeMap<String, RecipeFragment>,
}

/// Role binding metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleBinding {
    /// Family role ID.
    pub role: String,
    /// Whether the role may be omitted when not required.
    pub optional: bool,
}

/// Variant binding placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantBinding {
    /// Variant rule ID.
    pub rule: String,
    /// Human-readable binding note.
    pub note: String,
}

/// Executable style binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleImplementation {
    /// Executable style implementation schema version.
    pub schema_version: u32,
    /// Style kit ID implemented by this binding.
    pub style_kit_id: String,
    /// Explicit default style providers keyed by family role ID.
    pub default_role_providers: BTreeMap<String, String>,
    /// Style prototypes keyed by prototype ID.
    pub prototypes: BTreeMap<String, RecipeFragment>,
    /// Detail fragments keyed by detail module ID.
    pub detail_modules: BTreeMap<String, RecipeFragment>,
}

/// Concrete fragment that can provide a family role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeFragment {
    /// Executable fragment schema version.
    pub schema_version: u32,
    /// Fragment ID.
    pub id: String,
    /// Family role provided by this fragment.
    pub provided_role: String,
    /// Root instances that count as externally visible role occurrences.
    pub role_occurrence_roots: Vec<PartInstanceId>,
    /// Internal instances that do not count toward role cardinality.
    pub internal_instances: Vec<PartInstanceId>,
    /// Source recipe whose roots will be merged into the target recipe.
    pub recipe: AssetRecipe,
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
    /// Stable source recipe hash from the compiled artifact.
    pub source_recipe_hash: u64,
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
    recipe.id = AssetId(derived_asset_id(
        family,
        style_kit,
        family_impl,
        style_impl,
        &effective_request,
        &selected_providers,
    ));
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
    let artifact = compile_asset(&recipe)?;
    if !artifact.validation_report.is_valid() {
        return Err(FamilyCompileError::CompileValidationFailed(
            artifact.validation_report.clone(),
        ));
    }

    let report = FamilyInstantiationReport {
        family_id: family.id.clone(),
        style_kit_id: style_kit.id.clone(),
        selected_providers: selected_providers
            .iter()
            .map(|(role, selection)| (role.clone(), selection.fragment.clone()))
            .collect(),
        parameter_applications,
        source_recipe_hash: artifact.source_recipe_hash,
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

fn derived_asset_id(
    family: &AssetFamilySchema,
    style_kit: &StyleKit,
    family_impl: &FamilyImplementation,
    style_impl: &StyleImplementation,
    request: &FamilyInstantiationRequest,
    selected_providers: &BTreeMap<String, ProviderSelection>,
) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_str(&mut hash, "shape-lab.family-instantiation.v1");
    hash_str(&mut hash, &family.id);
    hash_u64(&mut hash, u64::from(family.schema_version));
    hash_str(&mut hash, &style_kit.id);
    hash_u64(&mut hash, u64::from(style_kit.schema_version));
    hash_str(&mut hash, &family_impl.family_id);
    hash_u64(&mut hash, u64::from(family_impl.schema_version));
    hash_str(&mut hash, &style_impl.style_kit_id);
    hash_u64(&mut hash, u64::from(style_impl.schema_version));
    hash_u64(&mut hash, request.seed);
    for (slot, value) in &request.parameters {
        hash_str(&mut hash, slot);
        hash_family_value(&mut hash, value);
    }
    for (role, selection) in selected_providers {
        hash_str(&mut hash, role);
        hash_str(&mut hash, &selection.fragment);
        hash_u64(
            &mut hash,
            match selection.source {
                ProviderSource::FamilyDefault => 1,
                ProviderSource::StylePrototype => 2,
            },
        );
        if let Some(fragment) = find_selected_fragment_for_hash(family_impl, style_impl, selection)
        {
            hash_u64(&mut hash, u64::from(fragment.schema_version));
        }
    }
    if hash == 0 { 1 } else { hash }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_u8(hash: &mut u64, value: u8) {
    *hash ^= u64::from(value);
    *hash = hash.wrapping_mul(FNV_PRIME);
}

fn hash_u64(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        hash_u8(hash, byte);
    }
}

fn hash_str(hash: &mut u64, value: &str) {
    hash_u64(hash, value.len() as u64);
    for byte in value.as_bytes() {
        hash_u8(hash, *byte);
    }
}

fn hash_family_value(hash: &mut u64, value: &FamilyValue) {
    match value {
        FamilyValue::Scalar(value) => {
            hash_str(hash, "scalar");
            hash_u64(hash, u64::from(value.to_bits()));
        }
        FamilyValue::Integer(value) => {
            hash_str(hash, "integer");
            hash_u64(hash, u64::from(*value));
        }
        FamilyValue::Toggle(value) => {
            hash_str(hash, "toggle");
            hash_u64(hash, if *value { 1 } else { 0 });
        }
        FamilyValue::Choice(value) => {
            hash_str(hash, "choice");
            hash_str(hash, value);
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
    let style_prototypes = style_kit
        .part_prototypes
        .iter()
        .map(|prototype| (prototype.id.as_str(), prototype.role.as_str()))
        .collect::<BTreeMap<_, _>>();
    let style_details = style_kit
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

    if report.is_valid() {
        Ok(())
    } else {
        Err(FamilyCompileError::ImplementationValidationFailed(report))
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
    if fragment.role_occurrence_roots.is_empty() {
        push_report_issue(
            report,
            Some(format!("{subject}.role_occurrence_roots")),
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
    for root in &fragment.role_occurrence_roots {
        if !root_ids.insert(*root) {
            push_report_issue(
                report,
                Some(format!("{subject}.role_occurrence_roots")),
                "duplicate_role_occurrence_root",
                "Role occurrence roots must be unique within one fragment.",
            );
        }
        if !instance_ids.contains(root) {
            push_report_issue(
                report,
                Some(format!("{subject}.role_occurrence_roots")),
                "unknown_role_occurrence_root",
                "Role occurrence roots must exist in the fragment recipe.",
            );
        }
    }
    let mut internal_ids = BTreeSet::new();
    for internal in &fragment.internal_instances {
        if !internal_ids.insert(*internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.internal_instances")),
                "duplicate_internal_instance",
                "Internal instances must be unique within one fragment.",
            );
        }
        if !instance_ids.contains(internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.internal_instances")),
                "unknown_internal_instance",
                "Internal instances must exist in the fragment recipe.",
            );
        }
        if root_ids.contains(internal) {
            push_report_issue(
                report,
                Some(format!("{subject}.internal_instances")),
                "fragment_export_root_marked_internal",
                "A role occurrence root cannot also be listed as an internal instance.",
            );
        }
    }
    let mut covered_instances = BTreeSet::new();
    for root in root_ids {
        if instance_ids.contains(&root) {
            covered_instances.extend(collect_subtree_instances(&fragment.recipe, root));
        }
    }
    covered_instances.extend(internal_ids);
    for instance in instance_ids {
        if !covered_instances.contains(&instance) {
            push_report_issue(
                report,
                Some(format!("{subject}.internal_instances")),
                "unclassified_fragment_instance",
                "Every fragment instance must be under a role occurrence root or explicitly internal.",
            );
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

    let mut definition_map = BTreeMap::new();
    let mut instance_map = BTreeMap::new();
    let mut region_map = BTreeMap::new();

    for definition_id in fragment.recipe.definitions.keys() {
        definition_map.insert(*definition_id, target.allocate_part_definition_id());
    }
    for instance_id in fragment.recipe.instances.keys() {
        instance_map.insert(*instance_id, target.allocate_part_instance_id());
    }
    for definition in fragment.recipe.definitions.values() {
        for region_id in definition.regions.keys() {
            region_map.insert(*region_id, target.allocate_region_id());
        }
    }

    for (old_id, definition) in &fragment.recipe.definitions {
        let mut cloned = definition.clone();
        cloned.id = *definition_map
            .get(old_id)
            .ok_or_else(|| unsupported_fragment(&fragment.id, "definition ID was not allocated"))?;
        cloned.regions = remap_regions(&definition.regions, &region_map);
        target.definitions.insert(cloned.id, cloned);
    }

    for (old_id, instance) in &fragment.recipe.instances {
        let mut cloned = instance.clone();
        cloned.id = *instance_map
            .get(old_id)
            .ok_or_else(|| unsupported_fragment(&fragment.id, "instance ID was not allocated"))?;
        cloned.definition = *definition_map.get(&instance.definition).ok_or_else(|| {
            unsupported_fragment(&fragment.id, "instance references an unmapped definition")
        })?;
        cloned.parent = instance
            .parent
            .map(|parent| {
                instance_map.get(&parent).copied().ok_or_else(|| {
                    unsupported_fragment(&fragment.id, "instance references an unmapped parent")
                })
            })
            .transpose()?;
        target.instances.insert(cloned.id, cloned.clone());
    }
    for root in &fragment.recipe.root_instances {
        let Some(root) = instance_map.get(root).copied() else {
            return Err(unsupported_fragment(
                &fragment.id,
                "root instance was not remapped",
            ));
        };
        target.root_instances.push(root);
    }
    let occurrence_roots = remap_fragment_instance_list(
        fragment,
        &fragment.role_occurrence_roots,
        &instance_map,
        "role occurrence root",
    )?;
    let internal_instances = remap_fragment_instance_list(
        fragment,
        &fragment.internal_instances,
        &instance_map,
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
        cloned.id = target.allocate_parameter_id();
        cloned.path = remap_scalar_path(&parameter.path, &definition_map, &instance_map)?;
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
                    let instances = roots
                        .iter()
                        .flat_map(|root| collect_subtree_instances(recipe, *root))
                        .collect::<Vec<_>>();
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
            .filter(|instance| {
                recipe
                    .instances
                    .get(instance)
                    .is_some_and(|part| part.enabled)
            })
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
