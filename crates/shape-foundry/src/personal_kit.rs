//! Personal Kit save view-model and local storage contracts.
//!
//! Personal Kits are local/private save targets. This module defines product
//! copy, validation, and V0 local/private storage. It does not implement
//! public publishing.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    DirectKitDraft, DirectKitSourceKind, DirectKitValidationReport, DirectKitVisibility,
    validate_direct_kit_draft,
};

/// Personal Kit local store schema version.
pub const PERSONAL_KIT_STORE_SCHEMA_VERSION: u32 = 1;
/// Deterministic timestamp used by V0 local storage until project persistence
/// provides authored timestamps.
pub const PERSONAL_KIT_STORAGE_TIMESTAMP_V0: &str = "1970-01-01T00:00:00Z";

/// Source kind a user may save into a Personal Kit.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PersonalKitSourceKind {
    /// The primitive currently being edited.
    CurrentPrimitive,
    /// A review-required ObjectPlan draft.
    ObjectPlanDraft,
    /// A review-required composition draft.
    CompositionDraft,
    /// Unsupported source used to keep validation explicit.
    Unsupported,
}

/// Visibility requested for a Personal Kit save.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PersonalKitVisibility {
    /// Draft save target.
    Draft,
    /// Local/private save target.
    PersonalOnly,
    /// Rejected by V0. Included only so validators can block it explicitly.
    PublicCatalog,
}

/// View model for a future Save as Personal Kit surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitSaveViewModel {
    /// Source kind being saved.
    pub source_kind: PersonalKitSourceKind,
    /// Product-facing display name.
    pub display_name: String,
    /// Editable kit name value.
    pub editable_name: String,
    /// Product-safe summary.
    pub summary: String,
    /// Product-safe warnings.
    pub warnings: Vec<String>,
    /// Whether the future Save action should be enabled.
    pub save_enabled: bool,
    /// Product-safe disabled reason.
    pub disabled_reason: Option<String>,
    /// Resulting local/private visibility.
    pub resulting_visibility: PersonalKitVisibility,
    /// Whether review image evidence exists.
    pub render_evidence_available: bool,
    /// Whether geometry export proof exists.
    pub export_proof_available: bool,
}

/// Command payload for a future Personal Kit save action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitSaveCommand {
    /// Source reference.
    pub source_ref: String,
    /// User-supplied kit name.
    pub kit_name: String,
    /// Requested visibility.
    pub visibility: PersonalKitVisibility,
    /// Include a preview image when available.
    pub include_preview: bool,
    /// Include the source ObjectPlan when applicable.
    pub include_object_plan: bool,
    /// Include a geometry export reference when applicable.
    pub include_export_reference: bool,
}

/// One Personal Kit validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Personal Kit validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PersonalKitValidationIssue>,
}

impl PersonalKitValidationReport {
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
        self.issues.push(PersonalKitValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Manifest for a local/private Personal Kit store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitStoreManifest {
    /// Store schema version.
    pub schema_version: u32,
    /// Saved kit entries.
    pub kits: Vec<PersonalKitManifestEntry>,
}

/// One manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitManifestEntry {
    /// Stable kit ID.
    pub kit_id: String,
    /// Product-facing kit name.
    pub display_name: String,
    /// Draft or PersonalOnly visibility.
    pub visibility: DirectKitVisibility,
    /// Relative path to the stored kit JSON.
    pub kit_path: String,
}

/// Stored local/private kit JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitStoredKit {
    /// Stable kit ID.
    pub kit_id: String,
    /// Product-facing name.
    pub display_name: String,
    /// Direct Kit source kind.
    pub source_kind: DirectKitSourceKind,
    /// Source reference.
    pub source_ref: String,
    /// Direct Kit contract.
    pub direct_kit: DirectKitDraft,
    /// Draft or PersonalOnly visibility.
    pub visibility: DirectKitVisibility,
    /// V0 kits are not novice-visible by default.
    pub novice_visible: bool,
    /// V0 kits are never public catalog-visible.
    pub public_catalog_visible: bool,
    /// Creation timestamp.
    pub created_at: String,
    /// Update timestamp.
    pub updated_at: String,
}

/// One Personal Kit store validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitStoreValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Personal Kit store validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalKitStoreValidationReport {
    /// Errors that make the store invalid.
    pub errors: Vec<PersonalKitStoreValidationIssue>,
    /// Warnings that should be reviewed.
    pub warnings: Vec<PersonalKitStoreValidationIssue>,
}

impl PersonalKitStoreValidationReport {
    /// Return true when no errors were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    fn error(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.errors.push(PersonalKitStoreValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    fn warning(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.warnings.push(PersonalKitStoreValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Storage error for Personal Kit local persistence.
#[derive(Debug)]
pub enum PersonalKitStorageError {
    /// I/O failure.
    Io(std::io::Error),
    /// JSON failure.
    Json(serde_json::Error),
    /// Validation failure.
    Validation(PersonalKitStoreValidationReport),
}

impl fmt::Display for PersonalKitStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Validation(report) => {
                write!(
                    formatter,
                    "Personal Kit store validation failed with {} error(s)",
                    report.errors.len()
                )
            }
        }
    }
}

impl std::error::Error for PersonalKitStorageError {}

impl From<std::io::Error> for PersonalKitStorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for PersonalKitStorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Build product-safe default copy for a future Personal Kit save surface.
#[must_use]
pub fn personal_kit_save_view_model(
    source_kind: PersonalKitSourceKind,
    display_name: impl Into<String>,
    render_evidence_available: bool,
    export_proof_available: bool,
) -> PersonalKitSaveViewModel {
    let display_name = display_name.into();
    let mut warnings = Vec::new();
    if !render_evidence_available {
        warnings.push("No review image yet.".to_owned());
    }
    if !export_proof_available {
        warnings.push("No engine export proof yet.".to_owned());
    }
    PersonalKitSaveViewModel {
        source_kind,
        display_name: display_name.clone(),
        editable_name: display_name,
        summary: "Only visible to you. Needs review before sharing.".to_owned(),
        warnings,
        save_enabled: source_kind != PersonalKitSourceKind::Unsupported,
        disabled_reason: (source_kind == PersonalKitSourceKind::Unsupported)
            .then(|| "This source cannot be saved yet.".to_owned()),
        resulting_visibility: PersonalKitVisibility::PersonalOnly,
        render_evidence_available,
        export_proof_available,
    }
}

/// Validate a Personal Kit save view model.
#[must_use]
pub fn validate_personal_kit_save_view_model(
    view_model: &PersonalKitSaveViewModel,
) -> PersonalKitValidationReport {
    let mut report = PersonalKitValidationReport::default();

    if view_model.source_kind == PersonalKitSourceKind::Unsupported {
        report.push(
            "source_kind",
            "personal_kit_unsupported_source",
            "This source cannot be saved as a Personal Kit.",
        );
    }
    if view_model.editable_name.trim().is_empty() {
        report.push(
            "editable_name",
            "personal_kit_name_required",
            "Personal Kit name is required.",
        );
    }
    if view_model.resulting_visibility == PersonalKitVisibility::PublicCatalog {
        report.push(
            "resulting_visibility",
            "personal_kit_public_visibility_rejected",
            "Personal Kits cannot use public visibility.",
        );
    }
    if !view_model.render_evidence_available
        && !view_model
            .warnings
            .iter()
            .any(|warning| warning == "No review image yet.")
    {
        report.push(
            "warnings",
            "personal_kit_missing_render_evidence_warning",
            "Missing review images must be shown as a warning.",
        );
    }
    if !view_model.export_proof_available
        && !view_model
            .warnings
            .iter()
            .any(|warning| warning == "No engine export proof yet.")
    {
        report.push(
            "warnings",
            "personal_kit_missing_export_proof_warning",
            "Missing export proof must be shown as a warning.",
        );
    }
    validate_user_copy(
        &[
            view_model.display_name.as_str(),
            view_model.editable_name.as_str(),
            view_model.summary.as_str(),
            view_model.disabled_reason.as_deref().unwrap_or_default(),
        ],
        "view_model",
        &mut report,
    );
    for warning in &view_model.warnings {
        validate_user_copy(&[warning.as_str()], "warnings", &mut report);
    }

    report
}

/// Validate a future Personal Kit save command.
#[must_use]
pub fn validate_personal_kit_save_command(
    command: &PersonalKitSaveCommand,
) -> PersonalKitValidationReport {
    let mut report = PersonalKitValidationReport::default();

    if command.source_ref.trim().is_empty() {
        report.push(
            "source_ref",
            "personal_kit_source_ref_required",
            "Personal Kit source reference is required.",
        );
    }
    if command.kit_name.trim().is_empty() {
        report.push(
            "kit_name",
            "personal_kit_name_required",
            "Personal Kit name is required.",
        );
    }
    if command.visibility == PersonalKitVisibility::PublicCatalog {
        report.push(
            "visibility",
            "personal_kit_public_visibility_rejected",
            "Personal Kits cannot use public visibility.",
        );
    }
    validate_user_copy(&[command.kit_name.as_str()], "kit_name", &mut report);

    report
}

/// Save a Direct Kit into a local/private Personal Kit store.
pub fn save_direct_kit(
    base_dir: impl AsRef<Path>,
    kit: &DirectKitDraft,
) -> Result<PersonalKitStoredKit, PersonalKitStorageError> {
    let stored = PersonalKitStoredKit {
        kit_id: kit.kit_id.clone(),
        display_name: kit.display_name.clone(),
        source_kind: kit.source_kind,
        source_ref: kit.source_ref.clone(),
        direct_kit: kit.clone(),
        visibility: kit.visibility,
        novice_visible: false,
        public_catalog_visible: false,
        created_at: PERSONAL_KIT_STORAGE_TIMESTAMP_V0.to_owned(),
        updated_at: PERSONAL_KIT_STORAGE_TIMESTAMP_V0.to_owned(),
    };

    let report = validate_personal_kit_stored_kit(&stored);
    if !report.is_valid() {
        return Err(PersonalKitStorageError::Validation(report));
    }

    let root = personal_kit_store_root(base_dir.as_ref());
    let kit_dir = root.join("kits").join(&kit.kit_id);
    fs::create_dir_all(&kit_dir)?;
    write_json_file(kit_dir.join("kit.json"), &stored)?;
    write_manifest(&root)?;
    Ok(stored)
}

/// Load one Direct Kit from a local/private Personal Kit store.
pub fn load_direct_kit(
    base_dir: impl AsRef<Path>,
    kit_id: &str,
) -> Result<PersonalKitStoredKit, PersonalKitStorageError> {
    let path = personal_kit_store_root(base_dir.as_ref())
        .join("kits")
        .join(kit_id)
        .join("kit.json");
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// List a local/private Personal Kit store.
pub fn list_personal_kits(
    base_dir: impl AsRef<Path>,
) -> Result<PersonalKitStoreManifest, PersonalKitStorageError> {
    let root = personal_kit_store_root(base_dir.as_ref());
    let path = root.join("manifest.json");
    if !path.exists() {
        return Ok(PersonalKitStoreManifest {
            schema_version: PERSONAL_KIT_STORE_SCHEMA_VERSION,
            kits: Vec::new(),
        });
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Validate a local/private Personal Kit store.
#[must_use]
pub fn validate_personal_kit_store(base_dir: impl AsRef<Path>) -> PersonalKitStoreValidationReport {
    let base_dir = base_dir.as_ref();
    let mut report = PersonalKitStoreValidationReport::default();
    let manifest = match list_personal_kits(base_dir) {
        Ok(manifest) => manifest,
        Err(error) => {
            report.error(
                "manifest",
                "personal_kit_manifest_unreadable",
                format!("Personal Kit manifest could not be read: {error}"),
            );
            return report;
        }
    };

    if manifest.schema_version != PERSONAL_KIT_STORE_SCHEMA_VERSION {
        report.error(
            "manifest.schema_version",
            "personal_kit_store_schema_unsupported",
            "Personal Kit store schema version is unsupported.",
        );
    }
    let mut seen = std::collections::BTreeSet::new();
    for (index, entry) in manifest.kits.iter().enumerate() {
        let subject = format!("manifest.kits.{index}");
        if !seen.insert(entry.kit_id.clone()) {
            report.error(
                format!("{subject}.kit_id"),
                "personal_kit_duplicate_manifest_entry",
                "Personal Kit manifest entries must be unique.",
            );
        }
        if path_is_absolute_like(&entry.kit_path) {
            report.error(
                format!("{subject}.kit_path"),
                "personal_kit_absolute_path_rejected",
                "Personal Kit manifest paths must be relative.",
            );
        }
        match load_direct_kit(base_dir, &entry.kit_id) {
            Ok(stored) => {
                merge_store_report(
                    &mut report,
                    &format!("kits.{}", entry.kit_id),
                    validate_personal_kit_stored_kit(&stored),
                );
            }
            Err(error) => report.error(
                format!("kits.{}", entry.kit_id),
                "personal_kit_load_failed",
                format!("Personal Kit could not be loaded: {error}"),
            ),
        }
    }

    report
}

/// Return the on-disk store root for a base directory.
#[must_use]
pub fn personal_kit_store_root(base_dir: &Path) -> PathBuf {
    base_dir.join("personal-kits")
}

/// Validate one stored local/private kit.
#[must_use]
pub fn validate_personal_kit_stored_kit(
    stored: &PersonalKitStoredKit,
) -> PersonalKitStoreValidationReport {
    let mut report = PersonalKitStoreValidationReport::default();
    merge_direct_kit_report(
        &mut report,
        "direct_kit",
        validate_direct_kit_draft(&stored.direct_kit),
    );
    if stored.kit_id != stored.direct_kit.kit_id {
        report.error(
            "kit_id",
            "personal_kit_id_mismatch",
            "Stored kit ID must match the Direct Kit ID.",
        );
    }
    if stored.display_name != stored.direct_kit.display_name {
        report.error(
            "display_name",
            "personal_kit_display_name_mismatch",
            "Stored kit display name must match the Direct Kit display name.",
        );
    }
    if stored.source_kind != stored.direct_kit.source_kind {
        report.error(
            "source_kind",
            "personal_kit_source_kind_mismatch",
            "Stored kit source kind must match the Direct Kit source kind.",
        );
    }
    if stored.source_ref != stored.direct_kit.source_ref {
        report.error(
            "source_ref",
            "personal_kit_source_ref_mismatch",
            "Stored kit source reference must match the Direct Kit source reference.",
        );
    }
    if stored.source_ref.trim().is_empty() || path_is_absolute_like(&stored.source_ref) {
        report.error(
            "source_ref",
            "personal_kit_source_ref_invalid",
            "Stored kit source reference must be present and relative/product-safe.",
        );
    }
    match stored.visibility {
        DirectKitVisibility::Draft | DirectKitVisibility::PersonalOnly => {}
        DirectKitVisibility::Reviewed | DirectKitVisibility::Showcase => report.error(
            "visibility",
            "personal_kit_future_visibility_rejected",
            "Personal Kit V0 stores only Draft or PersonalOnly kits.",
        ),
        DirectKitVisibility::PublicCatalog => report.error(
            "visibility",
            "personal_kit_public_visibility_rejected",
            "Personal Kit V0 cannot store public catalog visibility.",
        ),
    }
    if stored.visibility != stored.direct_kit.visibility {
        report.error(
            "visibility",
            "personal_kit_visibility_mismatch",
            "Stored kit visibility must match Direct Kit visibility.",
        );
    }
    if stored.novice_visible {
        report.error(
            "novice_visible",
            "personal_kit_novice_visible_rejected",
            "Personal Kit V0 storage is not novice-visible by default.",
        );
    }
    if stored.public_catalog_visible {
        report.error(
            "public_catalog_visible",
            "personal_kit_public_catalog_visible_rejected",
            "Personal Kit V0 storage cannot be public catalog-visible.",
        );
    }
    if stored.created_at.trim().is_empty() || stored.updated_at.trim().is_empty() {
        report.error(
            "timestamps",
            "personal_kit_timestamps_required",
            "Stored kits require created_at and updated_at timestamps.",
        );
    }
    if serialized_contains_absolute_path(stored) {
        report.error(
            "stored_kit",
            "personal_kit_absolute_path_rejected",
            "Stored kit JSON must not contain absolute paths.",
        );
    }
    if serialized_contains_forbidden_claim(stored) {
        report.error(
            "stored_kit",
            "personal_kit_forbidden_claim",
            "Stored kit JSON must not claim raw mesh payloads or game-ready status.",
        );
    }
    if !stored.direct_kit.evidence_refs.is_empty()
        && stored
            .direct_kit
            .evidence_refs
            .iter()
            .any(|evidence| path_is_absolute_like(&evidence.path))
    {
        report.error(
            "direct_kit.evidence_refs",
            "personal_kit_absolute_path_rejected",
            "Evidence paths must remain relative.",
        );
    }

    report
}

fn validate_user_copy(values: &[&str], subject: &str, report: &mut PersonalKitValidationReport) {
    for (index, value) in values.iter().enumerate() {
        let lower = value.to_ascii_lowercase();
        for forbidden in [
            "kernel",
            "module",
            "provider",
            "slot",
            "topology",
            "fingerprint",
            "conformance",
            "artifact",
            "raw transform",
            "publish",
            "catalog",
            "game-ready",
            "marketplace",
            "material",
            "rig",
            "animation",
        ] {
            if lower.contains(forbidden) {
                report.push(
                    format!("{subject}.{index}"),
                    "personal_kit_user_copy_forbidden_term",
                    "Personal Kit user copy must stay local/private and product-safe.",
                );
            }
        }
    }
}

fn write_manifest(root: &Path) -> Result<(), PersonalKitStorageError> {
    fs::create_dir_all(root)?;
    let kits_dir = root.join("kits");
    fs::create_dir_all(&kits_dir)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&kits_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let kit_path = entry.path().join("kit.json");
        if !kit_path.exists() {
            continue;
        }
        let bytes = fs::read(&kit_path)?;
        let stored: PersonalKitStoredKit = serde_json::from_slice(&bytes)?;
        entries.push(PersonalKitManifestEntry {
            kit_id: stored.kit_id.clone(),
            display_name: stored.display_name.clone(),
            visibility: stored.visibility,
            kit_path: format!("kits/{}/kit.json", stored.kit_id),
        });
    }
    entries.sort_by(|left, right| left.kit_id.cmp(&right.kit_id));
    write_json_file(
        root.join("manifest.json"),
        &PersonalKitStoreManifest {
            schema_version: PERSONAL_KIT_STORE_SCHEMA_VERSION,
            kits: entries,
        },
    )
}

fn write_json_file(
    path: impl AsRef<Path>,
    value: &impl Serialize,
) -> Result<(), PersonalKitStorageError> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn merge_store_report(
    report: &mut PersonalKitStoreValidationReport,
    prefix: &str,
    nested: PersonalKitStoreValidationReport,
) {
    for issue in nested.errors {
        report.error(
            format!("{prefix}.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
    for issue in nested.warnings {
        report.warning(
            format!("{prefix}.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn merge_direct_kit_report(
    report: &mut PersonalKitStoreValidationReport,
    prefix: &str,
    nested: DirectKitValidationReport,
) {
    for issue in nested.errors {
        report.error(
            format!("{prefix}.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
    for issue in nested.warnings {
        report.warning(
            format!("{prefix}.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn path_is_absolute_like(value: &str) -> bool {
    value.starts_with('/') || value.contains(":\\")
}

fn serialized_contains_absolute_path(value: &impl Serialize) -> bool {
    let Ok(text) = serde_json::to_string(value) else {
        return true;
    };
    text.contains("\"/") || text.contains(":\\\\")
}

fn serialized_contains_forbidden_claim(value: &impl Serialize) -> bool {
    let Ok(text) = serde_json::to_string(value) else {
        return true;
    };
    let lower = text.to_ascii_lowercase();
    lower.contains("raw mesh payload") || lower.contains("game-ready")
}
