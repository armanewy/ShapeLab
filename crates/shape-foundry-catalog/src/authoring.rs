//! Typed Foundry Author profile packages.
//!
//! This layer is intentionally a local package format over the existing
//! foundry catalog contracts. It does not add a second compiler path: authored
//! packages become exact catalog entries and then compile through
//! `shape_foundry`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_family::{
    AssetFamilySchema, FamilyValidationReport, StyleKit, validate_asset_family_schema,
    validate_family_style_compatibility, validate_family_style_completeness, validate_style_kit,
};
use shape_family_compile::{FamilyImplementation, StyleImplementation};
use shape_foundry::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_FAMILY_IMPL,
    CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL, ControlValue, CustomizerProfile,
    FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCatalogLock,
    FoundryDocumentId, FoundryValidationReport, SHAPE_FOUNDRY_CRATE_VERSION,
    compile_foundry_document, document_catalog_refs, resolve_foundry_catalog,
    validate_customizer_profile, validate_foundry_document,
};

use crate::{
    FoundryCatalogSerializedEntry, FoundryFixtureCatalog, catalog_entry, roman_bridge, scifi_crate,
    stylized_lamp,
};

/// Current schema version for Foundry Author profile packages.
pub const FOUNDRY_AUTHOR_PROFILE_SCHEMA_VERSION: u32 = 1;

/// Self-contained technical authoring package for one local Foundry profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryAuthorProfilePackage {
    /// Author package schema version.
    pub schema_version: u32,
    /// Stable package ID used as the local catalog namespace.
    pub package_id: String,
    /// Monotonic author package version.
    pub package_version: u32,
    /// Human-facing package name.
    pub display_name: String,
    /// Short author-facing summary.
    pub summary: String,
    /// Document ID to use for the generated Foundry source document.
    pub document_id: String,
    /// Default deterministic seed for the generated document.
    pub seed: u64,
    /// Theme-neutral family contract.
    pub family: AssetFamilySchema,
    /// Style-kit vocabulary and family facets.
    pub style: StyleKit,
    /// Executable family binding.
    pub family_implementation: FamilyImplementation,
    /// Executable style binding and provider vocabulary.
    pub style_implementation: StyleImplementation,
    /// Novice customizer controls and candidate strategies.
    pub customizer_profile: CustomizerProfile,
    /// Initial control state for a new document.
    pub control_state: BTreeMap<String, ControlValue>,
    /// Preview cameras an author expects tooling to render.
    pub preview_cameras: Vec<FoundryAuthorPreviewCamera>,
    /// Pack policies declared by the profile.
    pub pack_policies: Vec<FoundryAuthorPackPolicy>,
}

/// Preview-camera request stored in a profile package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryAuthorPreviewCamera {
    /// Stable camera ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Orbit yaw/pitch/roll in degrees.
    pub orbit_degrees: [f32; 3],
}

/// Pack policy stored in a profile package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryAuthorPackPolicy {
    /// Stable pack policy ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Minimum member count.
    pub minimum_members: u32,
    /// Maximum member count.
    pub maximum_members: u32,
    /// Export profile expected for this pack.
    pub export_profile: String,
    /// Control IDs intended to stay coherent across pack members.
    pub shared_control_ids: Vec<String>,
}

/// One authoring-kit validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryAuthorValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-facing issue message.
    pub message: String,
}

/// Validation and compile-proof summary for an author package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryAuthorValidationReport {
    /// Package ID being validated.
    pub package_id: String,
    /// Validation issues.
    pub issues: Vec<FoundryAuthorValidationIssue>,
    /// Number of primary visible controls.
    pub primary_control_count: u32,
    /// Number of authored candidate strategies.
    pub candidate_strategy_count: u32,
    /// Number of preview cameras.
    pub preview_camera_count: u32,
    /// Number of pack policies.
    pub pack_policy_count: u32,
    /// Number of catalog entries generated by the package.
    pub catalog_entry_count: u32,
    /// Compile build fingerprint when the author package builds cleanly.
    pub build_fingerprint: Option<String>,
    /// Compiled part count when the author package builds cleanly.
    pub compiled_part_count: Option<u64>,
    /// Compiled triangle count when the author package builds cleanly.
    pub triangle_count: Option<u64>,
}

impl FoundryAuthorValidationReport {
    /// Return true when no issues were discovered.
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
        self.issues.push(FoundryAuthorValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

impl FoundryAuthorProfilePackage {
    /// Convert this author package into the exact catalog layout consumed by Foundry builds.
    #[must_use]
    pub fn to_fixture_catalog(&self) -> FoundryFixtureCatalog {
        let entries = [
            catalog_entry(
                &format!("{}-family", self.package_id),
                self.family.schema_version,
                &format!("{} family", self.family.display_name),
                vec!["family".to_owned(), "author-package".to_owned()],
                &self.family,
            ),
            catalog_entry(
                &format!("{}-style", self.package_id),
                self.style.schema_version,
                &format!("{} style", self.style.display_name),
                vec!["style".to_owned(), "author-package".to_owned()],
                &self.style,
            ),
            catalog_entry(
                &format!("{}-family-impl", self.package_id),
                self.family_implementation.schema_version,
                "Family implementation",
                vec!["implementation".to_owned(), "author-package".to_owned()],
                &self.family_implementation,
            ),
            catalog_entry(
                &format!("{}-style-impl", self.package_id),
                self.style_implementation.schema_version,
                "Style implementation",
                vec!["implementation".to_owned(), "author-package".to_owned()],
                &self.style_implementation,
            ),
            catalog_entry(
                &format!("{}-profile", self.package_id),
                self.customizer_profile.schema_version,
                "Customizer profile",
                vec!["profile".to_owned(), "author-package".to_owned()],
                &self.customizer_profile,
            ),
        ];
        catalog_from_author_entries(self, entries)
    }
}

/// Return a built-in fixture as a typed author profile template.
#[must_use]
pub fn author_profile_template(template: &str) -> Option<FoundryAuthorProfilePackage> {
    let fixture = match template {
        "roman-bridge" | "bridge" => roman_bridge::fixture_catalog(),
        "sci-fi-crate" | "scifi-crate" | "crate" => scifi_crate::fixture_catalog(),
        "stylized-lamp" | "lamp" => stylized_lamp::fixture_catalog(),
        _ => return None,
    };
    Some(author_profile_from_fixture(&fixture))
}

/// Build a typed author package from an existing exact fixture catalog.
#[must_use]
pub fn author_profile_from_fixture(fixture: &FoundryFixtureCatalog) -> FoundryAuthorProfilePackage {
    let catalog = resolve_foundry_catalog(&fixture.document, fixture)
        .expect("built-in foundry fixture catalog should resolve");
    let shared_control_ids = catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary)
        .map(|control| control.id.clone())
        .collect();
    FoundryAuthorProfilePackage {
        schema_version: FOUNDRY_AUTHOR_PROFILE_SCHEMA_VERSION,
        package_id: fixture.slug.clone(),
        package_version: 1,
        display_name: format!(
            "{} / {}",
            catalog.family.display_name, catalog.style_kit.display_name
        ),
        summary: format!(
            "Local Foundry Author package for {} using {}.",
            catalog.family.display_name, catalog.style_kit.display_name
        ),
        document_id: fixture.document.document_id.0.clone(),
        seed: fixture.document.seed,
        family: catalog.family,
        style: catalog.style_kit,
        family_implementation: catalog.family_implementation,
        style_implementation: catalog.style_implementation,
        customizer_profile: catalog.customizer_profile,
        control_state: fixture.document.control_state.clone(),
        preview_cameras: vec![FoundryAuthorPreviewCamera {
            id: "default".to_owned(),
            label: "Default".to_owned(),
            width: 512,
            height: 512,
            orbit_degrees: [35.0, 25.0, 0.0],
        }],
        pack_policies: vec![FoundryAuthorPackPolicy {
            id: "three-member-pack".to_owned(),
            label: "Three Member Pack".to_owned(),
            minimum_members: 1,
            maximum_members: 3,
            export_profile: "canonical-model-package".to_owned(),
            shared_control_ids,
        }],
    }
}

/// Validate a Foundry Author package and compile it as proof.
#[must_use]
pub fn validate_author_profile_package(
    package: &FoundryAuthorProfilePackage,
) -> FoundryAuthorValidationReport {
    let mut report = FoundryAuthorValidationReport {
        package_id: package.package_id.clone(),
        issues: Vec::new(),
        primary_control_count: package
            .customizer_profile
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count() as u32,
        candidate_strategy_count: package.customizer_profile.candidate_strategies.len() as u32,
        preview_camera_count: package.preview_cameras.len() as u32,
        pack_policy_count: package.pack_policies.len() as u32,
        catalog_entry_count: 0,
        build_fingerprint: None,
        compiled_part_count: None,
        triangle_count: None,
    };

    validate_package_metadata(package, &mut report);
    extend_family_report(
        &mut report,
        "family",
        validate_asset_family_schema(&package.family),
    );
    extend_family_report(&mut report, "style", validate_style_kit(&package.style));
    extend_family_report(
        &mut report,
        "family_style",
        validate_family_style_compatibility(&package.family, &package.style),
    );
    extend_family_report(
        &mut report,
        "family_style",
        validate_family_style_completeness(&package.family, &package.style),
    );
    extend_foundry_report(
        &mut report,
        "customizer_profile",
        validate_customizer_profile(&package.customizer_profile),
    );
    validate_author_customizer_basics(package, &mut report);
    validate_cross_references(package, &mut report);

    if !report.is_valid() {
        return report;
    }

    let fixture = package.to_fixture_catalog();
    report.catalog_entry_count = fixture.entries.len() as u32;
    extend_foundry_report(
        &mut report,
        "document",
        validate_foundry_document(&fixture.document),
    );
    if !report.is_valid() {
        return report;
    }
    match compile_foundry_document(&fixture.document, &fixture) {
        Ok(output) => {
            if !output.final_conformance.is_accepted() {
                report.push(
                    "compile.conformance",
                    "author_package_conformance_failed",
                    "Author package compiled but did not pass final family conformance.",
                );
            }
            if !output.artifact.validation_report.is_valid() {
                report.push(
                    "compile.artifact",
                    "author_package_artifact_invalid",
                    "Author package compiled but the artifact validation report is invalid.",
                );
            }
            let model_config = validation_config_from_recipe_with_limits(
                &output.recipe,
                &output.artifact,
                ValidationLimits::default(),
            );
            let model_report = validate_model(&output.artifact, &model_config);
            if !model_report.is_valid() {
                report.push(
                    "compile.model_validation",
                    "author_package_model_validation_failed",
                    format!(
                        "Author package compiled but model validation reported {} issue(s).",
                        model_report.issues.len()
                    ),
                );
            }
            report.build_fingerprint = Some(output.build_stamp.build_fingerprint.0.to_hex());
            report.compiled_part_count = Some(output.artifact.statistics.part_count);
            report.triangle_count = Some(output.artifact.statistics.triangle_count);
        }
        Err(error) => report.push(
            "compile",
            "author_package_compile_failed",
            format!("Author package failed to compile: {error:#?}"),
        ),
    }
    report
}

fn catalog_from_author_entries(
    package: &FoundryAuthorProfilePackage,
    entries: [FoundryCatalogSerializedEntry; 5],
) -> FoundryFixtureCatalog {
    let entries = entries
        .into_iter()
        .map(|entry| (entry.content_ref.stable_id.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut document = FoundryAssetDocument {
        schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
        document_id: FoundryDocumentId(package.document_id.clone()),
        family_content_ref: entries[&format!("{}-family", package.package_id)]
            .content_ref
            .clone(),
        style_content_ref: entries[&format!("{}-style", package.package_id)]
            .content_ref
            .clone(),
        family_implementation_ref: entries[&format!("{}-family-impl", package.package_id)]
            .content_ref
            .clone(),
        style_implementation_ref: entries[&format!("{}-style-impl", package.package_id)]
            .content_ref
            .clone(),
        customizer_profile_ref: entries[&format!("{}-profile", package.package_id)]
            .content_ref
            .clone(),
        control_state: package.control_state.clone(),
        provider_overrides: BTreeMap::new(),
        foundry_locks: Vec::new(),
        local_recipe_overrides: Vec::new(),
        seed: package.seed,
        catalog_lock: None,
        build_stamp: None,
    };
    document.catalog_lock = Some(FoundryCatalogLock {
        exact_refs: document_catalog_refs(&document),
        embedded_snapshots: Vec::new(),
        compiler_version: SHAPE_FOUNDRY_CRATE_VERSION.to_owned(),
        catalog_version: package.package_version,
    });
    debug_assert!(
        [
            CATALOG_LOCK_KEY_FAMILY,
            CATALOG_LOCK_KEY_STYLE,
            CATALOG_LOCK_KEY_FAMILY_IMPL,
            CATALOG_LOCK_KEY_STYLE_IMPL,
            CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE,
        ]
        .iter()
        .all(|key| document
            .catalog_lock
            .as_ref()
            .unwrap()
            .exact_refs
            .contains_key(*key))
    );
    FoundryFixtureCatalog {
        slug: package.package_id.clone(),
        catalog_version: package.package_version,
        document,
        entries,
    }
}

fn validate_package_metadata(
    package: &FoundryAuthorProfilePackage,
    report: &mut FoundryAuthorValidationReport,
) {
    if package.schema_version != FOUNDRY_AUTHOR_PROFILE_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_author_profile_schema",
            "Foundry Author profile schema version is not supported.",
        );
    }
    if package.package_version == 0 {
        report.push(
            "package_version",
            "invalid_author_package_version",
            "Author package version must be greater than zero.",
        );
    }
    validate_identifier(report, "package_id", &package.package_id);
    validate_identifier(report, "document_id", &package.document_id);
    validate_text(report, "display_name", &package.display_name);
    validate_text(report, "summary", &package.summary);
    validate_preview_cameras(package, report);
    validate_pack_policies(package, report);
}

fn validate_cross_references(
    package: &FoundryAuthorProfilePackage,
    report: &mut FoundryAuthorValidationReport,
) {
    if package.family_implementation.family_id != package.family.id {
        report.push(
            "family_implementation.family_id",
            "author_family_impl_mismatch",
            "Family implementation must target the package family.",
        );
    }
    if package.style_implementation.family_id != package.family.id {
        report.push(
            "style_implementation.family_id",
            "author_style_impl_family_mismatch",
            "Style implementation must target the package family.",
        );
    }
    if package.style_implementation.style_kit_id != package.style.id {
        report.push(
            "style_implementation.style_kit_id",
            "author_style_impl_mismatch",
            "Style implementation must target the package style kit.",
        );
    }
    if package.customizer_profile.family_id != package.family.id {
        report.push(
            "customizer_profile.family_id",
            "author_profile_family_mismatch",
            "Customizer profile must target the package family.",
        );
    }
    if package.customizer_profile.style_id.as_ref() != Some(&package.style.id) {
        report.push(
            "customizer_profile.style_id",
            "author_profile_style_mismatch",
            "Customizer profile must target the package style kit.",
        );
    }

    let controls = package
        .customizer_profile
        .controls
        .iter()
        .map(|control| control.id.as_str())
        .collect::<BTreeSet<_>>();
    if controls.is_empty() {
        report.push(
            "customizer_profile.controls",
            "missing_author_controls",
            "Author packages must expose at least one customizer control.",
        );
    }
    for control_id in package.control_state.keys() {
        if !controls.contains(control_id.as_str()) {
            report.push(
                format!("control_state.{control_id}"),
                "author_unknown_control_state",
                "Initial control state references an unknown customizer control.",
            );
        }
    }
    if package.customizer_profile.candidate_strategies.is_empty() {
        report.push(
            "customizer_profile.candidate_strategies",
            "missing_author_candidate_strategy",
            "Author packages must define at least one candidate strategy.",
        );
    }
}

fn validate_author_customizer_basics(
    package: &FoundryAuthorProfilePackage,
    report: &mut FoundryAuthorValidationReport,
) {
    if package.customizer_profile.maximum_primary_controls == 0 {
        report.push(
            "customizer_profile.maximum_primary_controls",
            "invalid_author_primary_control_limit",
            "Customizer profile must allow at least one primary control.",
        );
    }
    let mut control_ids = BTreeSet::new();
    for (index, control) in package.customizer_profile.controls.iter().enumerate() {
        validate_identifier(
            report,
            format!("customizer_profile.controls.{index}.id"),
            &control.id,
        );
        validate_text(
            report,
            format!("customizer_profile.controls.{index}.label"),
            &control.label,
        );
        if !control_ids.insert(control.id.as_str()) {
            report.push(
                format!("customizer_profile.controls.{index}.id"),
                "duplicate_author_control",
                "Customizer control IDs must be unique.",
            );
        }
    }
    if report.primary_control_count > package.customizer_profile.maximum_primary_controls {
        report.push(
            "customizer_profile.controls",
            "too_many_author_primary_controls",
            "Customizer profile exceeds its maximum primary control count.",
        );
    }
    for (index, strategy) in package
        .customizer_profile
        .candidate_strategies
        .iter()
        .enumerate()
    {
        validate_identifier(
            report,
            format!("customizer_profile.candidate_strategies.{index}.id"),
            &strategy.id,
        );
        validate_text(
            report,
            format!("customizer_profile.candidate_strategies.{index}.label"),
            &strategy.label,
        );
        for (control_index, control_id) in strategy.control_ids.iter().enumerate() {
            if !control_ids.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "customizer_profile.candidate_strategies.{index}.control_ids.{control_index}"
                    ),
                    "unknown_author_strategy_control",
                    "Candidate strategy references an unknown customizer control.",
                );
            }
        }
    }
}

fn validate_preview_cameras(
    package: &FoundryAuthorProfilePackage,
    report: &mut FoundryAuthorValidationReport,
) {
    if package.preview_cameras.is_empty() {
        report.push(
            "preview_cameras",
            "missing_author_preview_camera",
            "Author packages must declare at least one preview camera.",
        );
    }
    let mut ids = BTreeSet::new();
    for (index, camera) in package.preview_cameras.iter().enumerate() {
        validate_identifier(report, format!("preview_cameras.{index}.id"), &camera.id);
        validate_text(
            report,
            format!("preview_cameras.{index}.label"),
            &camera.label,
        );
        if !ids.insert(camera.id.as_str()) {
            report.push(
                format!("preview_cameras.{index}.id"),
                "duplicate_author_preview_camera",
                "Preview camera IDs must be unique.",
            );
        }
        if camera.width == 0 || camera.height == 0 || camera.width > 4096 || camera.height > 4096 {
            report.push(
                format!("preview_cameras.{index}"),
                "invalid_author_preview_camera_size",
                "Preview camera dimensions must be in the inclusive range 1..=4096.",
            );
        }
        if !camera.orbit_degrees.iter().all(|value| value.is_finite()) {
            report.push(
                format!("preview_cameras.{index}.orbit_degrees"),
                "non_finite_author_preview_camera",
                "Preview camera orbit values must be finite.",
            );
        }
    }
}

fn validate_pack_policies(
    package: &FoundryAuthorProfilePackage,
    report: &mut FoundryAuthorValidationReport,
) {
    if package.pack_policies.is_empty() {
        report.push(
            "pack_policies",
            "missing_author_pack_policy",
            "Author packages must declare at least one pack policy.",
        );
    }
    let controls = package
        .customizer_profile
        .controls
        .iter()
        .map(|control| control.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for (index, policy) in package.pack_policies.iter().enumerate() {
        validate_identifier(report, format!("pack_policies.{index}.id"), &policy.id);
        validate_text(
            report,
            format!("pack_policies.{index}.label"),
            &policy.label,
        );
        validate_identifier(
            report,
            format!("pack_policies.{index}.export_profile"),
            &policy.export_profile,
        );
        if !ids.insert(policy.id.as_str()) {
            report.push(
                format!("pack_policies.{index}.id"),
                "duplicate_author_pack_policy",
                "Pack policy IDs must be unique.",
            );
        }
        if policy.minimum_members == 0 || policy.minimum_members > policy.maximum_members {
            report.push(
                format!("pack_policies.{index}"),
                "invalid_author_pack_member_range",
                "Pack policy member range must be non-empty and ordered.",
            );
        }
        for (control_index, control_id) in policy.shared_control_ids.iter().enumerate() {
            if !controls.contains(control_id.as_str()) {
                report.push(
                    format!("pack_policies.{index}.shared_control_ids.{control_index}"),
                    "unknown_author_pack_shared_control",
                    "Pack policy references an unknown shared control.",
                );
            }
        }
    }
}

fn extend_family_report(
    report: &mut FoundryAuthorValidationReport,
    prefix: &str,
    nested: FamilyValidationReport,
) {
    for issue in nested.issues {
        report.push(
            issue
                .subject
                .map(|subject| format!("{prefix}.{subject}"))
                .unwrap_or_else(|| prefix.to_owned()),
            issue.code,
            issue.message,
        );
    }
}

fn extend_foundry_report(
    report: &mut FoundryAuthorValidationReport,
    prefix: &str,
    nested: FoundryValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("{prefix}.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn validate_identifier(
    report: &mut FoundryAuthorValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    let subject = subject.into();
    if value.is_empty() {
        report.push(
            subject,
            "empty_author_identifier",
            "Identifier must not be empty.",
        );
        return;
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        report.push(
            subject,
            "invalid_author_identifier",
            "Identifier must contain only ASCII letters, digits, dashes, underscores, or dots.",
        );
    }
}

fn validate_text(
    report: &mut FoundryAuthorValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.trim().is_empty() {
        report.push(
            subject,
            "empty_author_text",
            "Text field must not be empty.",
        );
    }
}
