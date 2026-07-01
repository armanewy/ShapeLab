//! Personal Kit save view-model contracts.
//!
//! Personal Kits are local/private save targets. This module defines product
//! copy and validation only; it does not implement file storage or publishing.

use serde::{Deserialize, Serialize};

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
