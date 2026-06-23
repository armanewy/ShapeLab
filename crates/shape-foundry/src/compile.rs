//! Build stamp, compiled-output, and foundry document compilation contracts.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::{AssetEditProgram, AssetRecipe, AssetValidationReport, validate_asset_recipe};
use shape_compile::{AssetArtifact, CompileError, compile_asset};
use shape_family::{AssetFamilySchema, FamilyRuleExecutionPolicy, RoleProvision};
use shape_family_compile::{
    FamilyCompileError, FamilyImplementation, FamilyInstantiationReport,
    FamilyInstantiationRequest, FamilyValue, SHAPE_FAMILY_COMPILE_CRATE_VERSION,
    StyleImplementation,
    conformance::{
        ConformanceIssue, ConformanceStatus, ConstraintBindingMap, FamilyConformanceReport,
        evaluate_attachment_conformance, evaluate_export_requirements,
        evaluate_geometric_constraints, evaluate_operation_conformance, evaluate_role_conformance,
    },
    identity::{
        ArtifactFingerprint, BuildFingerprint, FingerprintError, GeometryInputFingerprint,
        RecipeFingerprint, fingerprint_serializable,
    },
    instantiate_family,
    remap::ports::SelectedFragmentPorts,
};

use crate::{
    ControlKind, ControlValue, DroppedLocalOverride, FoundryCatalogError, FoundryCatalogResolver,
    FoundryConformanceSummary, FoundryResolvedCatalog, LocalRecipeOverride, LocalRecipeOverrideId,
    OverrideSurvivalPolicy, ResponseCurve, SHAPE_FOUNDRY_CRATE_VERSION,
    catalog::resolve_foundry_catalog, validate_foundry_document,
};

/// Deterministic build stamp emitted after foundry compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryBuildStamp {
    /// Geometry input fingerprint used for the build.
    pub geometry_input_fingerprint: GeometryInputFingerprint,
    /// Build fingerprint including conformance contracts and compiler versions.
    pub build_fingerprint: BuildFingerprint,
    /// Exact generated recipe fingerprint.
    pub recipe_fingerprint: RecipeFingerprint,
    /// Compiled artifact fingerprint.
    pub artifact_fingerprint: ArtifactFingerprint,
    /// Shape Foundry crate version.
    pub foundry_version: String,
    /// Shape Family Compile crate version.
    pub family_compile_version: String,
}

/// Exact generated recipe snapshot persisted beside semantic foundry sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedRecipeSnapshot {
    /// Recipe schema version.
    pub schema_version: u32,
    /// Canonical JSON recipe payload.
    pub canonical_json: String,
    /// Fingerprint of the recipe payload.
    pub recipe_fingerprint: RecipeFingerprint,
}

/// Foundry document compilation options.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCompilationOptions {
    /// Run a pre-override conformance pass after base instantiation.
    pub run_preliminary_conformance: bool,
}

impl Default for FoundryCompilationOptions {
    fn default() -> Self {
        Self {
            run_preliminary_conformance: true,
        }
    }
}

/// Status for one local override during compilation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalOverrideApplicationStatus {
    /// The override was authored against the current base and was applied.
    Applied,
    /// The override was replayed against a changed base and revalidated.
    Revalidated,
    /// The override was explicitly dropped by survival policy.
    Dropped,
}

/// Deterministic report for one local override application decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOverrideApplicationReport {
    /// Override ID.
    pub id: LocalRecipeOverrideId,
    /// Survival policy that made the decision.
    pub survival_policy: OverrideSurvivalPolicy,
    /// Application status.
    pub status: LocalOverrideApplicationStatus,
    /// Deterministic reason code.
    pub reason: String,
    /// Fingerprint the override was authored against.
    pub authored_base_geometry_fingerprint: GeometryInputFingerprint,
    /// Fingerprint of the current base before local overrides.
    pub current_base_geometry_fingerprint: GeometryInputFingerprint,
}

/// Where a provider override was resolved.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderOverrideSource {
    /// The provider ID resolved to a family-owned fragment.
    FamilyFragment,
    /// The provider ID resolved to a style prototype.
    StylePrototype,
}

/// Provider override application report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderOverrideApplicationReport {
    /// Family role.
    pub role: String,
    /// Exact provider ID requested by the document/control.
    pub provider_id: String,
    /// Resolved provider source.
    pub source: ProviderOverrideSource,
}

/// Complete foundry document compilation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCompilationOutput {
    /// Document with build stamp and exact lock populated for this build.
    pub document: crate::FoundryAssetDocument,
    /// Resolved catalog inputs.
    pub catalog: FoundryResolvedCatalog,
    /// Effective family request sent to the family compiler.
    pub family_request: FamilyInstantiationRequest,
    /// Base instantiated recipe before local overrides.
    pub base_recipe: AssetRecipe,
    /// Fingerprint of the base recipe before local overrides.
    pub base_geometry_fingerprint: GeometryInputFingerprint,
    /// Final recipe after local overrides.
    pub recipe: AssetRecipe,
    /// Final compiled artifact.
    pub artifact: AssetArtifact,
    /// Optional pre-override conformance report.
    pub preliminary_conformance: Option<FamilyConformanceReport>,
    /// Authoritative final conformance report.
    pub final_conformance: FamilyConformanceReport,
    /// Summary derived from the final conformance report.
    pub conformance_summary: FoundryConformanceSummary,
    /// Build stamp emitted for the final output.
    pub build_stamp: FoundryBuildStamp,
    /// Exact generated recipe snapshot.
    pub recipe_snapshot: GeneratedRecipeSnapshot,
    /// Provider override application reports.
    pub provider_override_reports: Vec<ProviderOverrideApplicationReport>,
    /// Local override application reports.
    pub local_override_reports: Vec<LocalOverrideApplicationReport>,
    /// Local override rows explicitly dropped by policy.
    pub dropped_overrides: Vec<DroppedLocalOverride>,
}

/// Foundry document compilation failure.
#[derive(Debug)]
pub enum FoundryCompilationError {
    /// The semantic document contract is invalid.
    DocumentValidationFailed(crate::FoundryValidationReport),
    /// Catalog resolution or lock verification failed.
    Catalog(FoundryCatalogError),
    /// Control state references a profile control that is not available.
    UnknownControl {
        /// Control ID.
        control_id: String,
    },
    /// Control value kind does not match the profile control kind.
    ControlValueKindMismatch {
        /// Control ID.
        control_id: String,
    },
    /// Control value is outside the resolved feasible domain.
    ControlValueUnavailable {
        /// Control ID.
        control_id: String,
    },
    /// A response curve could not be evaluated for a control binding.
    InvalidResponseCurve {
        /// Control ID.
        control_id: String,
        /// Family slot.
        slot: String,
    },
    /// Integer controls must not be negative when converted to family counts.
    NegativeIntegerControl {
        /// Control ID.
        control_id: String,
        /// Supplied value.
        value: i64,
    },
    /// A provider gallery selected an option not present in the profile.
    UnknownProviderOption {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
    },
    /// A document and provider-control selection conflict.
    ProviderOverrideConflict {
        /// Family role.
        role: String,
        /// First provider ID.
        first: String,
        /// Second provider ID.
        second: String,
    },
    /// A provider override did not name an available provider for the role.
    ProviderOverrideUnavailable {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
    },
    /// A provider ID exists in both family and style provider sets for the same role.
    ProviderOverrideAmbiguous {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
    },
    /// A provider override resolves to a provider source that the role cannot use.
    ProviderOverrideInvalidProvision {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
    },
    /// Family compiler rejected the request or implementation set.
    FamilyCompile(FamilyCompileError),
    /// A pinned or revalidated local override could not be applied.
    LocalOverrideRejected {
        /// Override ID.
        override_id: LocalRecipeOverrideId,
        /// Deterministic reason.
        reason: String,
    },
    /// Final recipe validation failed after local overrides.
    AssetValidationFailed(AssetValidationReport),
    /// Final asset compilation failed.
    Compile(CompileError),
    /// Final conformance rejected the build.
    FinalConformanceRejected(FamilyConformanceReport),
    /// Deterministic fingerprinting failed.
    Fingerprint {
        /// Fingerprinted subject.
        subject: String,
        /// Error text.
        error: String,
    },
    /// Deterministic JSON generation failed.
    Json {
        /// Subject being serialized.
        subject: String,
        /// Error text.
        error: String,
    },
}

/// Compile a foundry asset document using default compilation options.
pub fn compile_foundry_document(
    document: &crate::FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryCompilationOutput, FoundryCompilationError> {
    compile_foundry_document_with_options(document, resolver, FoundryCompilationOptions::default())
}

/// Compile a foundry asset document.
pub fn compile_foundry_document_with_options(
    document: &crate::FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
    options: FoundryCompilationOptions,
) -> Result<FoundryCompilationOutput, FoundryCompilationError> {
    let document_validation = validate_foundry_document(document);
    if !document_validation.is_valid() {
        return Err(FoundryCompilationError::DocumentValidationFailed(
            document_validation,
        ));
    }

    let catalog =
        resolve_foundry_catalog(document, resolver).map_err(FoundryCompilationError::Catalog)?;
    let (family_request, provider_requests) =
        evaluate_effective_family_request(document, &catalog)?;
    let mut family_implementation = catalog.family_implementation.clone();
    let mut style_implementation = catalog.style_implementation.clone();
    let provider_override_reports = apply_provider_overrides(
        &catalog.family,
        &mut family_implementation,
        &mut style_implementation,
        &provider_requests,
    )?;

    let base_instantiation = instantiate_family(
        &catalog.family,
        &catalog.style_kit,
        &family_implementation,
        &style_implementation,
        &family_request,
    )
    .map_err(FoundryCompilationError::FamilyCompile)?;
    let base_geometry_fingerprint = base_geometry_fingerprint(&base_instantiation.recipe)?;

    let preliminary_conformance = if options.run_preliminary_conformance {
        let report = evaluate_foundry_conformance(
            &catalog.family,
            &catalog.style_kit,
            &family_implementation,
            &style_implementation,
            &base_instantiation.report,
            &base_instantiation.recipe,
            &base_instantiation.artifact,
        );
        if !report.is_accepted() {
            return Err(FoundryCompilationError::FinalConformanceRejected(report));
        }
        Some(report)
    } else {
        None
    };

    let mut final_recipe = base_instantiation.recipe.clone();
    let local_override_reports = apply_local_recipe_overrides(
        &mut final_recipe,
        &document.local_recipe_overrides,
        base_geometry_fingerprint,
    )?;
    let final_recipe_validation = validate_asset_recipe(&final_recipe);
    if !final_recipe_validation.is_valid() {
        return Err(FoundryCompilationError::AssetValidationFailed(
            final_recipe_validation,
        ));
    }

    let artifact = compile_asset(&final_recipe).map_err(FoundryCompilationError::Compile)?;
    let final_conformance = evaluate_foundry_conformance(
        &catalog.family,
        &catalog.style_kit,
        &family_implementation,
        &style_implementation,
        &base_instantiation.report,
        &final_recipe,
        &artifact,
    );
    if !final_conformance.is_accepted() {
        return Err(FoundryCompilationError::FinalConformanceRejected(
            final_conformance,
        ));
    }

    let recipe_snapshot = generated_recipe_snapshot(&final_recipe)?;
    let artifact_fingerprint = artifact_fingerprint(&artifact)?;
    let final_geometry_input_fingerprint = final_geometry_input_fingerprint(
        base_geometry_fingerprint,
        &recipe_snapshot,
        &provider_override_reports,
        &local_override_reports,
    )?;
    let build_fingerprint = build_fingerprint(
        final_geometry_input_fingerprint,
        recipe_snapshot.recipe_fingerprint,
        artifact_fingerprint,
        &final_conformance,
        &catalog,
    )?;
    let build_stamp = FoundryBuildStamp {
        geometry_input_fingerprint: final_geometry_input_fingerprint,
        build_fingerprint,
        recipe_fingerprint: recipe_snapshot.recipe_fingerprint,
        artifact_fingerprint,
        foundry_version: SHAPE_FOUNDRY_CRATE_VERSION.to_owned(),
        family_compile_version: shape_family_compile_version(),
    };
    let conformance_summary = summarize_conformance(&final_conformance);
    let dropped_overrides = local_override_reports
        .iter()
        .filter(|report| report.status == LocalOverrideApplicationStatus::Dropped)
        .map(|report| DroppedLocalOverride {
            id: report.id.clone(),
            reason: report.reason.clone(),
        })
        .collect();
    let mut stamped_document = document.clone();
    stamped_document.catalog_lock = Some(catalog.catalog_lock.clone());
    stamped_document.build_stamp = Some(build_stamp.clone());

    Ok(FoundryCompilationOutput {
        document: stamped_document,
        catalog,
        family_request,
        base_recipe: base_instantiation.recipe,
        base_geometry_fingerprint,
        recipe: final_recipe,
        artifact,
        preliminary_conformance,
        final_conformance,
        conformance_summary,
        build_stamp,
        recipe_snapshot,
        provider_override_reports,
        local_override_reports,
        dropped_overrides,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderOverrideRequest {
    role: String,
    provider_id: String,
}

fn evaluate_effective_family_request(
    document: &crate::FoundryAssetDocument,
    catalog: &FoundryResolvedCatalog,
) -> Result<(FamilyInstantiationRequest, Vec<ProviderOverrideRequest>), FoundryCompilationError> {
    let controls = catalog
        .customizer_profile
        .controls
        .iter()
        .map(|control| (control.id.as_str(), control))
        .collect::<BTreeMap<_, _>>();
    let mut parameters = BTreeMap::new();
    let mut provider_requests = document
        .provider_overrides
        .values()
        .map(|override_row| ProviderOverrideRequest {
            role: override_row.role.clone(),
            provider_id: override_row.provider_ref.stable_id.clone(),
        })
        .collect::<Vec<_>>();

    for (control_id, value) in &document.control_state {
        let Some(control) = controls.get(control_id.as_str()) else {
            return Err(FoundryCompilationError::UnknownControl {
                control_id: control_id.clone(),
            });
        };
        ensure_control_value_available(control_id, control, value)?;
        match (&control.kind, value) {
            (
                ControlKind::ProviderGallery { role, options },
                ControlValue::Provider(provider_id),
            ) => {
                if !options
                    .iter()
                    .any(|option| option.provider_id == *provider_id)
                {
                    return Err(FoundryCompilationError::UnknownProviderOption {
                        role: role.clone(),
                        provider_id: provider_id.clone(),
                    });
                }
                push_provider_request(
                    &mut provider_requests,
                    ProviderOverrideRequest {
                        role: role.clone(),
                        provider_id: provider_id.clone(),
                    },
                )?;
            }
            _ => {
                for binding in &control.bindings {
                    let family_value =
                        family_value_from_control(control_id, &control.kind, value, binding)?;
                    parameters.insert(binding.slot.clone(), family_value);
                }
            }
        }
    }

    Ok((
        FamilyInstantiationRequest {
            family_id: catalog.family.id.clone(),
            style_kit_id: catalog.style_kit.id.clone(),
            parameters,
            seed: document.seed,
        },
        provider_requests,
    ))
}

fn push_provider_request(
    requests: &mut Vec<ProviderOverrideRequest>,
    request: ProviderOverrideRequest,
) -> Result<(), FoundryCompilationError> {
    if let Some(existing) = requests
        .iter()
        .find(|existing| existing.role == request.role)
    {
        if existing.provider_id != request.provider_id {
            return Err(FoundryCompilationError::ProviderOverrideConflict {
                role: request.role,
                first: existing.provider_id.clone(),
                second: request.provider_id,
            });
        }
        return Ok(());
    }
    requests.push(request);
    Ok(())
}

fn ensure_control_value_available(
    control_id: &str,
    control: &crate::CustomizerControl,
    value: &ControlValue,
) -> Result<(), FoundryCompilationError> {
    if !control_value_matches_kind(&control.kind, value) {
        return Err(FoundryCompilationError::ControlValueKindMismatch {
            control_id: control_id.to_owned(),
        });
    }
    if control
        .domain
        .discrete_values
        .iter()
        .any(|allowed| allowed == value)
    {
        return Ok(());
    }
    if let ControlValue::Scalar(value) = value
        && control
            .domain
            .continuous_intervals
            .iter()
            .any(|interval| *value >= interval.minimum && *value <= interval.maximum)
    {
        return Ok(());
    }
    if control.domain.discrete_values.is_empty() && control.domain.continuous_intervals.is_empty() {
        return Ok(());
    }
    Err(FoundryCompilationError::ControlValueUnavailable {
        control_id: control_id.to_owned(),
    })
}

fn control_value_matches_kind(kind: &ControlKind, value: &ControlValue) -> bool {
    matches!(
        (kind, value),
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(_))
            | (ControlKind::IntegerStepper { .. }, ControlValue::Integer(_))
            | (ControlKind::Toggle { .. }, ControlValue::Toggle(_))
            | (ControlKind::ChoiceGallery { .. }, ControlValue::Choice(_))
            | (
                ControlKind::ProviderGallery { .. },
                ControlValue::Provider(_)
            )
    )
}

fn family_value_from_control(
    control_id: &str,
    kind: &ControlKind,
    value: &ControlValue,
    binding: &crate::ControlSlotBinding,
) -> Result<FamilyValue, FoundryCompilationError> {
    match (kind, value) {
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(value)) => {
            Ok(FamilyValue::Scalar(apply_response_curve(
                control_id,
                &binding.slot,
                *value,
                &binding.response,
            )?))
        }
        (ControlKind::IntegerStepper { .. }, ControlValue::Integer(value)) => {
            let value = u32::try_from(*value).map_err(|_| {
                FoundryCompilationError::NegativeIntegerControl {
                    control_id: control_id.to_owned(),
                    value: *value,
                }
            })?;
            Ok(FamilyValue::Integer(value))
        }
        (ControlKind::Toggle { .. }, ControlValue::Toggle(value)) => {
            Ok(FamilyValue::Toggle(*value))
        }
        (ControlKind::ChoiceGallery { .. }, ControlValue::Choice(value)) => {
            Ok(FamilyValue::Choice(value.clone()))
        }
        _ => Err(FoundryCompilationError::ControlValueKindMismatch {
            control_id: control_id.to_owned(),
        }),
    }
}

fn apply_response_curve(
    control_id: &str,
    slot: &str,
    value: f32,
    response: &ResponseCurve,
) -> Result<f32, FoundryCompilationError> {
    match response {
        ResponseCurve::Linear => Ok(value),
        ResponseCurve::Piecewise { points, .. } => {
            if points.is_empty() {
                return Err(FoundryCompilationError::InvalidResponseCurve {
                    control_id: control_id.to_owned(),
                    slot: slot.to_owned(),
                });
            }
            if points.len() == 1 {
                return (points[0][0] == value)
                    .then_some(points[0][1])
                    .ok_or_else(|| FoundryCompilationError::InvalidResponseCurve {
                        control_id: control_id.to_owned(),
                        slot: slot.to_owned(),
                    });
            }
            for pair in points.windows(2) {
                let [left, right] = pair else {
                    continue;
                };
                let [x0, y0] = *left;
                let [x1, y1] = *right;
                if value >= x0 && value <= x1 && x1 != x0 {
                    let t = (value - x0) / (x1 - x0);
                    return Ok(y0 + (y1 - y0) * t);
                }
            }
            Err(FoundryCompilationError::InvalidResponseCurve {
                control_id: control_id.to_owned(),
                slot: slot.to_owned(),
            })
        }
    }
}

fn apply_provider_overrides(
    family: &AssetFamilySchema,
    family_implementation: &mut FamilyImplementation,
    style_implementation: &mut StyleImplementation,
    requests: &[ProviderOverrideRequest],
) -> Result<Vec<ProviderOverrideApplicationReport>, FoundryCompilationError> {
    let roles = family
        .part_roles
        .iter()
        .map(|role| (role.id.as_str(), role))
        .collect::<BTreeMap<_, _>>();
    let mut reports = Vec::with_capacity(requests.len());
    for request in requests {
        let Some(role) = roles.get(request.role.as_str()) else {
            return Err(FoundryCompilationError::ProviderOverrideUnavailable {
                role: request.role.clone(),
                provider_id: request.provider_id.clone(),
            });
        };
        let family_match = family_implementation
            .fragments
            .get(&request.provider_id)
            .is_some_and(|fragment| fragment.provided_role == request.role);
        let style_match = style_implementation
            .prototypes
            .get(&request.provider_id)
            .is_some_and(|fragment| fragment.provided_role == request.role);
        match (family_match, style_match) {
            (false, false) => {
                return Err(FoundryCompilationError::ProviderOverrideUnavailable {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                });
            }
            (true, true) => {
                return Err(FoundryCompilationError::ProviderOverrideAmbiguous {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                });
            }
            (true, false) => {
                if matches!(
                    role.provision,
                    RoleProvision::StyleRequired | RoleProvision::Derived
                ) {
                    return Err(FoundryCompilationError::ProviderOverrideInvalidProvision {
                        role: request.role.clone(),
                        provider_id: request.provider_id.clone(),
                    });
                }
                family_implementation
                    .default_role_providers
                    .insert(request.role.clone(), request.provider_id.clone());
                if role.provision == RoleProvision::FamilyOrStyle {
                    style_implementation
                        .default_role_providers
                        .remove(&request.role);
                }
                reports.push(ProviderOverrideApplicationReport {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                    source: ProviderOverrideSource::FamilyFragment,
                });
            }
            (false, true) => {
                if matches!(
                    role.provision,
                    RoleProvision::FamilyDefault | RoleProvision::Derived
                ) {
                    return Err(FoundryCompilationError::ProviderOverrideInvalidProvision {
                        role: request.role.clone(),
                        provider_id: request.provider_id.clone(),
                    });
                }
                style_implementation
                    .default_role_providers
                    .insert(request.role.clone(), request.provider_id.clone());
                reports.push(ProviderOverrideApplicationReport {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                    source: ProviderOverrideSource::StylePrototype,
                });
            }
        }
    }
    Ok(reports)
}

fn apply_local_recipe_overrides(
    recipe: &mut AssetRecipe,
    overrides: &[LocalRecipeOverride],
    current_base_geometry_fingerprint: GeometryInputFingerprint,
) -> Result<Vec<LocalOverrideApplicationReport>, FoundryCompilationError> {
    let mut reports = Vec::with_capacity(overrides.len());
    for override_row in overrides {
        let base_matches =
            override_row.base_geometry_fingerprint == current_base_geometry_fingerprint;
        match (base_matches, override_row.survival_policy) {
            (true, _) => {
                apply_local_edit_program(recipe, &override_row.id, &override_row.edit_program)?;
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Applied,
                    reason: "base_geometry_unchanged".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
            (false, OverrideSurvivalPolicy::Pinned) => {
                return Err(FoundryCompilationError::LocalOverrideRejected {
                    override_id: override_row.id.clone(),
                    reason: "pinned_base_geometry_changed".to_owned(),
                });
            }
            (false, OverrideSurvivalPolicy::DropOnStyleChange) => {
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Dropped,
                    reason: "base_geometry_changed".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
            (false, OverrideSurvivalPolicy::Revalidate) => {
                apply_local_edit_program(recipe, &override_row.id, &override_row.edit_program)?;
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Revalidated,
                    reason: "base_geometry_changed_revalidated".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
        }
    }
    Ok(reports)
}

fn apply_local_edit_program(
    recipe: &mut AssetRecipe,
    override_id: &LocalRecipeOverrideId,
    program: &AssetEditProgram,
) -> Result<(), FoundryCompilationError> {
    match shape_asset::apply_edit_program(recipe, program) {
        Ok(edited) => {
            *recipe = edited;
            Ok(())
        }
        Err(error) => Err(FoundryCompilationError::LocalOverrideRejected {
            override_id: override_id.clone(),
            reason: error.to_string(),
        }),
    }
}

fn evaluate_foundry_conformance(
    family: &AssetFamilySchema,
    style_kit: &shape_family::StyleKit,
    family_implementation: &FamilyImplementation,
    style_implementation: &StyleImplementation,
    instantiation_report: &FamilyInstantiationReport,
    recipe: &AssetRecipe,
    artifact: &AssetArtifact,
) -> FamilyConformanceReport {
    let selected_fragments = selected_fragment_ports(
        family_implementation,
        style_implementation,
        instantiation_report,
    );
    let mut report = FamilyConformanceReport {
        family_id: family.id.clone(),
        style_kit_id: style_kit.id.clone(),
        ..FamilyConformanceReport::default()
    };
    report.roles = evaluate_role_conformance(family, recipe, &selected_fragments);
    report.attachments =
        evaluate_attachment_conformance(family, family_implementation, recipe, &selected_fragments);
    report.constraints = evaluate_geometric_constraints(
        &family.constraints,
        &ConstraintBindingMap::new(),
        recipe,
        Some(artifact),
    );
    report.operations = evaluate_operation_conformance(family, recipe);
    report.exports =
        evaluate_export_requirements(&family.export_requirements, recipe, Some(artifact));
    for issue in artifact.validation_report.issues.iter() {
        report.issues.push(ConformanceIssue {
            subject: issue
                .subject
                .clone()
                .unwrap_or_else(|| "artifact".to_owned()),
            code: issue.code.clone(),
            message: issue.message.clone(),
            policy: FamilyRuleExecutionPolicy::Required,
            status: ConformanceStatus::Failed,
        });
    }
    append_rejecting_conformance_issues(&mut report);
    report
}

fn selected_fragment_ports<'a>(
    family_implementation: &'a FamilyImplementation,
    style_implementation: &'a StyleImplementation,
    instantiation_report: &'a FamilyInstantiationReport,
) -> Vec<SelectedFragmentPorts<'a>> {
    let remaps = instantiation_report
        .fragment_remaps
        .iter()
        .map(|report| (report.fragment_id.as_str(), &report.remap))
        .collect::<BTreeMap<_, _>>();
    instantiation_report
        .selected_providers
        .iter()
        .filter_map(|(role, fragment_id)| {
            let fragment = family_implementation
                .fragments
                .get(fragment_id)
                .or_else(|| style_implementation.prototypes.get(fragment_id))?;
            let remap = remaps.get(fragment_id.as_str()).copied()?;
            Some(SelectedFragmentPorts {
                role: role.as_str(),
                fragment,
                remap,
            })
        })
        .collect()
}

fn append_rejecting_conformance_issues(report: &mut FamilyConformanceReport) {
    for role in &report.roles {
        if role.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("roles.{}", role.role),
                code: "role_conformance_failed".to_owned(),
                message: "Final recipe does not satisfy required family role cardinality."
                    .to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: role.status,
            });
        }
    }
    for attachment in &report.attachments {
        if attachment.policy == FamilyRuleExecutionPolicy::Required
            && attachment.status.rejects_required()
        {
            report.issues.push(ConformanceIssue {
                subject: format!("attachments.{}", attachment.rule_id),
                code: "attachment_conformance_failed".to_owned(),
                message: "Final recipe does not satisfy required family attachment rules."
                    .to_owned(),
                policy: attachment.policy,
                status: attachment.status,
            });
        }
    }
    for constraint in &report.constraints {
        if constraint.policy == FamilyRuleExecutionPolicy::Required
            && constraint.status.rejects_required()
        {
            report.issues.push(ConformanceIssue {
                subject: format!("constraints.{}", constraint.constraint_id),
                code: "constraint_conformance_failed".to_owned(),
                message: "Final artifact does not satisfy required geometric constraints."
                    .to_owned(),
                policy: constraint.policy,
                status: constraint.status,
            });
        }
    }
    for operation in &report.operations {
        if operation.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("operations.{:?}", operation.operation),
                code: "operation_conformance_failed".to_owned(),
                message: "Final recipe contains a forbidden or invalid operation class.".to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: operation.status,
            });
        }
    }
    for export in &report.exports {
        if export.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("exports.{}", export.profile),
                code: "export_conformance_failed".to_owned(),
                message: "Final artifact does not satisfy export requirements.".to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: export.status,
            });
        }
    }
}

fn summarize_conformance(report: &FamilyConformanceReport) -> FoundryConformanceSummary {
    let required_issue_failures = report
        .issues
        .iter()
        .filter(|issue| {
            issue.policy == FamilyRuleExecutionPolicy::Required && issue.status.rejects_required()
        })
        .count();
    let role_failures = report
        .roles
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let attachment_failures = report
        .attachments
        .iter()
        .filter(|row| {
            row.policy == FamilyRuleExecutionPolicy::Required && row.status.rejects_required()
        })
        .count();
    let constraint_failures = report
        .constraints
        .iter()
        .filter(|row| {
            row.policy == FamilyRuleExecutionPolicy::Required && row.status.rejects_required()
        })
        .count();
    let operation_failures = report
        .operations
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let export_failures = report
        .exports
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let runtime_deferred_count = report
        .issues
        .iter()
        .filter(|issue| issue.status == ConformanceStatus::Deferred)
        .count();
    let advisory_issue_count = report
        .issues
        .iter()
        .filter(|issue| issue.policy != FamilyRuleExecutionPolicy::Required)
        .count();
    FoundryConformanceSummary {
        accepted: report.is_accepted(),
        required_failure_count: required_issue_failures
            + role_failures
            + attachment_failures
            + constraint_failures
            + operation_failures
            + export_failures,
        advisory_issue_count,
        runtime_deferred_count,
    }
}

fn base_geometry_fingerprint(
    recipe: &AssetRecipe,
) -> Result<GeometryInputFingerprint, FoundryCompilationError> {
    Ok(GeometryInputFingerprint(
        fingerprint_serializable("shape-lab.foundry-base-geometry.v1", "base_recipe", recipe)
            .map_err(foundry_fingerprint_error)?,
    ))
}

fn final_geometry_input_fingerprint(
    base_geometry_fingerprint: GeometryInputFingerprint,
    recipe_snapshot: &GeneratedRecipeSnapshot,
    provider_reports: &[ProviderOverrideApplicationReport],
    local_override_reports: &[LocalOverrideApplicationReport],
) -> Result<GeometryInputFingerprint, FoundryCompilationError> {
    #[derive(Serialize)]
    struct Payload<'a> {
        base_geometry_fingerprint: GeometryInputFingerprint,
        recipe_fingerprint: RecipeFingerprint,
        provider_reports: &'a [ProviderOverrideApplicationReport],
        local_override_reports: &'a [LocalOverrideApplicationReport],
    }
    Ok(GeometryInputFingerprint(
        fingerprint_serializable(
            "shape-lab.foundry-final-geometry-input.v1",
            "final_geometry_input",
            &Payload {
                base_geometry_fingerprint,
                recipe_fingerprint: recipe_snapshot.recipe_fingerprint,
                provider_reports,
                local_override_reports,
            },
        )
        .map_err(foundry_fingerprint_error)?,
    ))
}

fn generated_recipe_snapshot(
    recipe: &AssetRecipe,
) -> Result<GeneratedRecipeSnapshot, FoundryCompilationError> {
    GeneratedRecipeSnapshot::from_recipe(recipe).map_err(|error| match error {
        crate::FoundryRecipeSnapshotError::Serialization { subject, error } => {
            FoundryCompilationError::Json { subject, error }
        }
        crate::FoundryRecipeSnapshotError::NonFiniteNumber { subject } => {
            FoundryCompilationError::Fingerprint {
                subject,
                error: "canonical recipe snapshot contained a non-finite number".to_owned(),
            }
        }
    })
}

fn artifact_fingerprint(
    artifact: &AssetArtifact,
) -> Result<ArtifactFingerprint, FoundryCompilationError> {
    Ok(ArtifactFingerprint(
        fingerprint_serializable("shape-lab.artifact.v1", "compiled_artifact", artifact)
            .map_err(foundry_fingerprint_error)?,
    ))
}

fn build_fingerprint(
    geometry_input_fingerprint: GeometryInputFingerprint,
    recipe_fingerprint: RecipeFingerprint,
    artifact_fingerprint: ArtifactFingerprint,
    final_conformance: &FamilyConformanceReport,
    catalog: &FoundryResolvedCatalog,
) -> Result<BuildFingerprint, FoundryCompilationError> {
    #[derive(Serialize)]
    struct Payload<'a> {
        geometry_input_fingerprint: GeometryInputFingerprint,
        recipe_fingerprint: RecipeFingerprint,
        artifact_fingerprint: ArtifactFingerprint,
        conformance_accepted: bool,
        foundry_version: &'a str,
        family_compile_version: String,
        catalog_lock: &'a crate::FoundryCatalogLock,
    }
    Ok(BuildFingerprint(
        fingerprint_serializable(
            "shape-lab.foundry-build.v1",
            "foundry_build",
            &Payload {
                geometry_input_fingerprint,
                recipe_fingerprint,
                artifact_fingerprint,
                conformance_accepted: final_conformance.is_accepted(),
                foundry_version: SHAPE_FOUNDRY_CRATE_VERSION,
                family_compile_version: shape_family_compile_version(),
                catalog_lock: &catalog.catalog_lock,
            },
        )
        .map_err(foundry_fingerprint_error)?,
    ))
}

fn foundry_fingerprint_error(error: FingerprintError) -> FoundryCompilationError {
    match error {
        FingerprintError::Serialization { subject, error } => {
            FoundryCompilationError::Fingerprint { subject, error }
        }
        FingerprintError::NonFiniteNumber { subject } => FoundryCompilationError::Fingerprint {
            subject,
            error: "canonical fingerprint input contained a non-finite number".to_owned(),
        },
    }
}

fn shape_family_compile_version() -> String {
    SHAPE_FAMILY_COMPILE_CRATE_VERSION.to_owned()
}
