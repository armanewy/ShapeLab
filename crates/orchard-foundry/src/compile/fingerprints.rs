
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
                foundry_version: ORCHARD_FOUNDRY_CRATE_VERSION,
                family_compile_version: orchard_family_compile_version(),
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

fn orchard_family_compile_version() -> String {
    ORCHARD_FAMILY_COMPILE_CRATE_VERSION.to_owned()
}
