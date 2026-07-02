
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

/// One control marked divergent by an applied local override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergedControlReport {
    /// Control ID.
    pub control_id: String,
    /// Human-facing control label.
    pub label: String,
    /// Family slots linking the override target to this control.
    pub slots: Vec<String>,
}

/// Deterministic report connecting local overrides back to customizer controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOverrideDivergenceReport {
    /// Override ID.
    pub id: LocalRecipeOverrideId,
    /// Override application status.
    pub status: LocalOverrideApplicationStatus,
    /// Authored semantic targets touched by the override.
    pub touched_targets: Vec<TouchedSemanticTarget>,
    /// Controls whose generated semantic targets diverged.
    pub diverged_controls: Vec<DivergedControlReport>,
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
    /// Local override to visible-control divergence reports.
    pub local_override_divergence_reports: Vec<LocalOverrideDivergenceReport>,
    /// Current divergence state for every customizer control.
    pub control_divergence: BTreeMap<String, ControlDivergence>,
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
    compile_foundry_document_inner(document, resolver, options, FinalConformancePolicy::Reject)
}

pub(crate) fn compile_foundry_document_for_pack_report(
    document: &crate::FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
    options: FoundryCompilationOptions,
) -> Result<FoundryCompilationOutput, FoundryCompilationError> {
    compile_foundry_document_inner(document, resolver, options, FinalConformancePolicy::Report)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FinalConformancePolicy {
    Reject,
    Report,
}

fn compile_foundry_document_inner(
    document: &crate::FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
    options: FoundryCompilationOptions,
    final_conformance_policy: FinalConformancePolicy,
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
        if !report.is_accepted() && final_conformance_policy == FinalConformancePolicy::Reject {
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
    annotate_panel_knob_relationship(&mut final_recipe, &catalog.family.id, document);
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
    if !final_conformance.is_accepted()
        && final_conformance_policy == FinalConformancePolicy::Reject
    {
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
    let (control_divergence, local_override_divergence_reports) =
        compute_local_override_divergence_reports(
            &catalog.customizer_profile,
            document,
            &base_instantiation.report,
            &final_recipe,
            &local_override_reports,
        );
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
        local_override_divergence_reports,
        control_divergence,
        dropped_overrides,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderOverrideRequest {
    role: String,
    provider_id: String,
}
