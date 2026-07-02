//! Direct Kit contracts.
//!
//! A Direct Kit is a local reusable Draft or PersonalOnly kit built from
//! supported primitive schemas, safe composition, and review evidence. This
//! module defines contracts and validation only.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ObjectPlanReviewTier, PresetSource, PrimitiveKind, PrimitiveProperty, PrimitivePropertyDomain,
    PrimitivePropertySchema, PrimitivePropertyValue, box_primitive_property_schema,
    built_in_primitive_preset, flat_panel_primitive_property_schema,
    primitive_default_property_values, sphere_primitive_property_schema,
    validate_primitive_property_values,
};

/// Local reusable kit draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitDraft {
    /// Stable normalized kit ID.
    pub kit_id: String,
    /// Product-facing name.
    pub display_name: String,
    /// Source kind.
    pub source_kind: DirectKitSourceKind,
    /// Source reference, such as a primitive ID or ObjectPlan path.
    pub source_ref: String,
    /// Product-safe identity summary.
    pub identity_summary: String,
    /// Properties exposed as user-changeable controls.
    pub changeable_properties: Vec<DirectKitPropertyExposure>,
    /// Properties fixed by this kit.
    pub locked_properties: Vec<DirectKitPropertyExposure>,
    /// Optional deterministic presets.
    pub included_presets: Vec<DirectKitPresetRef>,
    /// Evidence references.
    pub evidence_refs: Vec<DirectKitEvidenceRef>,
    /// Review tier. V0 must remain Draft or Personal.
    pub review_tier: ObjectPlanReviewTier,
    /// Local visibility.
    pub visibility: DirectKitVisibility,
    /// Product-safe creation source.
    pub created_from: DirectKitCreatedFrom,
}

/// Supported Direct Kit source kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DirectKitSourceKind {
    /// One primitive schema.
    Primitive,
    /// Supported ObjectPlan Draft.
    ObjectPlan,
    /// Supported safe-anchor composition.
    Composition,
    /// Explicitly rejected placeholder.
    Unsupported,
}

/// Direct Kit visibility.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DirectKitVisibility {
    /// Draft local kit.
    Draft,
    /// Personal/private local kit.
    PersonalOnly,
    /// Future reviewed visibility; rejected in V0.
    Reviewed,
    /// Future showcase visibility; rejected in V0.
    Showcase,
    /// Explicitly rejected public catalog visibility.
    PublicCatalog,
}

/// One property exposed or locked by a Direct Kit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitPropertyExposure {
    /// Primitive property ID.
    pub property_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Current value.
    pub current_value: PrimitivePropertyValue,
    /// Schema default value.
    pub default_value: PrimitivePropertyValue,
    /// Bounded schema domain.
    pub domain: PrimitivePropertyDomain,
    /// Product-facing description.
    pub user_description: String,
    /// Whether this must be present for the kit.
    pub required: bool,
    /// Whether this belongs behind advanced UI.
    pub advanced: bool,
}

/// Deterministic preset reference included in a Direct Kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitPresetRef {
    /// Stable preset ID.
    pub preset_id: String,
    /// Product-facing preset name.
    pub display_name: String,
    /// Preset source.
    pub source: PresetSource,
}

/// Evidence kind referenced by a Direct Kit.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DirectKitEvidenceKind {
    /// Property endpoint sheet.
    PropertyEndpointSheet,
    /// Deterministic preset contact sheet.
    PresetContactSheet,
    /// ObjectPlan render evidence.
    ObjectPlanRenderEvidence,
    /// Geometry export report.
    ExportReport,
}

/// Evidence status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DirectKitEvidenceStatus {
    /// Evidence passed.
    Passed,
    /// Evidence exists with warnings.
    Warnings,
    /// Evidence was blocked honestly.
    Blocked,
    /// Evidence failed.
    Failed,
    /// Evidence is missing.
    Missing,
}

/// Evidence reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitEvidenceRef {
    /// Evidence kind.
    pub evidence_kind: DirectKitEvidenceKind,
    /// Relative evidence path.
    pub path: String,
    /// Evidence status.
    pub status: DirectKitEvidenceStatus,
    /// Human review remains required.
    pub human_review_required: bool,
}

/// Product-safe creation source.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DirectKitCreatedFrom {
    /// Current primitive editor state.
    CurrentPrimitive,
    /// Internal ObjectPlan Draft review state.
    ObjectPlanDraft,
    /// Safe composition editor state.
    CompositionDraft,
    /// Offline/internal tool output.
    InternalTool,
}

/// Product-facing Direct Kit summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitUserSummary {
    /// Short title.
    pub title: String,
    /// What this kit is.
    pub what_this_is: String,
    /// What can change.
    pub can_change: Vec<String>,
    /// What stays fixed.
    pub stays_fixed: Vec<String>,
    /// Evidence lines.
    pub evidence: Vec<String>,
    /// Personal-use status.
    pub personal_use: String,
    /// Review status.
    pub review_status: String,
}

/// One Direct Kit validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Direct Kit validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectKitValidationReport {
    /// Errors that prevent the kit from being valid.
    pub errors: Vec<DirectKitValidationIssue>,
    /// Warnings that require review but can still allow a Draft.
    pub warnings: Vec<DirectKitValidationIssue>,
}

impl DirectKitValidationReport {
    /// Return true when no validation errors were found.
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
        self.errors.push(DirectKitValidationIssue {
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
        self.warnings.push(DirectKitValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return property exposures for one primitive schema.
#[must_use]
pub fn direct_kit_property_exposures_for_primitive(
    primitive_kind: PrimitiveKind,
) -> Vec<DirectKitPropertyExposure> {
    let schema = primitive_schema(primitive_kind);
    schema
        .properties
        .iter()
        .map(direct_kit_property_exposure)
        .collect()
}

/// Build a product-facing summary for a Direct Kit.
#[must_use]
pub fn direct_kit_user_summary(draft: &DirectKitDraft) -> DirectKitUserSummary {
    DirectKitUserSummary {
        title: draft.display_name.clone(),
        what_this_is: draft.identity_summary.clone(),
        can_change: draft
            .changeable_properties
            .iter()
            .map(|property| format!("{} can be adjusted.", property.display_name))
            .collect(),
        stays_fixed: draft
            .locked_properties
            .iter()
            .map(|property| format!("{} stays fixed.", property.display_name))
            .collect(),
        evidence: evidence_summary_lines(draft),
        personal_use: match draft.visibility {
            DirectKitVisibility::Draft => "This kit is a Draft.".to_owned(),
            DirectKitVisibility::PersonalOnly => "This kit can be used personally.".to_owned(),
            DirectKitVisibility::Reviewed
            | DirectKitVisibility::Showcase
            | DirectKitVisibility::PublicCatalog => {
                "This kit needs review before sharing.".to_owned()
            }
        },
        review_status: "Needs review before sharing.".to_owned(),
    }
}

/// Validate a Direct Kit draft.
#[must_use]
pub fn validate_direct_kit_draft(draft: &DirectKitDraft) -> DirectKitValidationReport {
    let mut report = DirectKitValidationReport::default();

    validate_identifier(&draft.kit_id, "kit_id", &mut report);
    validate_user_copy(&draft.display_name, "display_name", &mut report);
    validate_user_copy(&draft.identity_summary, "identity_summary", &mut report);
    validate_user_copy(&draft.source_ref, "source_ref", &mut report);
    if draft.source_ref.trim().is_empty() {
        report.error(
            "source_ref",
            "direct_kit_source_ref_required",
            "Direct Kit source reference is required.",
        );
    }
    if draft.source_kind == DirectKitSourceKind::Unsupported {
        report.error(
            "source_kind",
            "direct_kit_unsupported_source_kind",
            "Direct Kit source kind is not supported in V0.",
        );
    }
    validate_visibility(draft, &mut report);

    let allowed_kinds = allowed_primitive_kinds(draft);
    if allowed_kinds.is_empty() {
        report.error(
            "source_ref",
            "direct_kit_unknown_source",
            "Direct Kit source must reference a supported primitive, composition, or ObjectPlan Draft.",
        );
    }

    validate_properties(
        &draft.changeable_properties,
        "changeable_properties",
        &allowed_kinds,
        &mut report,
    );
    validate_properties(
        &draft.locked_properties,
        "locked_properties",
        &allowed_kinds,
        &mut report,
    );
    validate_no_duplicate_property_role(draft, &mut report);
    validate_presets(draft, &allowed_kinds, &mut report);
    validate_evidence(draft, &mut report);

    let summary = direct_kit_user_summary(draft);
    validate_user_copy(&summary.title, "summary.title", &mut report);
    validate_user_copy(&summary.what_this_is, "summary.what_this_is", &mut report);
    validate_user_copy(&summary.personal_use, "summary.personal_use", &mut report);
    validate_user_copy(&summary.review_status, "summary.review_status", &mut report);
    for (index, line) in summary.can_change.iter().enumerate() {
        validate_user_copy(line, format!("summary.can_change.{index}"), &mut report);
    }
    for (index, line) in summary.stays_fixed.iter().enumerate() {
        validate_user_copy(line, format!("summary.stays_fixed.{index}"), &mut report);
    }
    for (index, line) in summary.evidence.iter().enumerate() {
        validate_user_copy(line, format!("summary.evidence.{index}"), &mut report);
    }

    report
}

fn direct_kit_property_exposure(property: &PrimitiveProperty) -> DirectKitPropertyExposure {
    DirectKitPropertyExposure {
        property_id: property.property_id.clone(),
        display_name: property.display_name.clone(),
        current_value: property.default_value.clone(),
        default_value: property.default_value.clone(),
        domain: property.domain.clone(),
        user_description: property.user_facing_description.clone(),
        required: true,
        advanced: property.advanced,
    }
}

fn primitive_schema(primitive_kind: PrimitiveKind) -> PrimitivePropertySchema {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => box_primitive_property_schema(),
        PrimitiveKind::FlatPanelPrimitive => flat_panel_primitive_property_schema(),
        PrimitiveKind::SpherePrimitive => sphere_primitive_property_schema(),
        PrimitiveKind::CylinderPrimitive => PrimitivePropertySchema {
            schema_version: 0,
            primitive_kind,
            display_name: String::new(),
            identity_summary: String::new(),
            properties: Vec::new(),
            constraints: Vec::new(),
            preview_policy: crate::PrimitivePreviewPolicy {
                preserve_previous_valid_preview: false,
                continuous_preview_allowed: false,
                user_facing_description: String::new(),
            },
            export_policy: crate::PrimitiveExportPolicy {
                export_current_primitive: false,
                user_facing_description: String::new(),
                limitations: Vec::new(),
            },
        },
    }
}

fn allowed_primitive_kinds(draft: &DirectKitDraft) -> Vec<PrimitiveKind> {
    match draft.source_kind {
        DirectKitSourceKind::Primitive => primitive_kind_from_source_ref(&draft.source_ref)
            .into_iter()
            .collect(),
        DirectKitSourceKind::ObjectPlan => vec![
            PrimitiveKind::BoxPrimitive,
            PrimitiveKind::FlatPanelPrimitive,
            PrimitiveKind::SpherePrimitive,
        ],
        DirectKitSourceKind::Composition => {
            if source_ref_is_panel_with_knob(&draft.source_ref) {
                vec![
                    PrimitiveKind::FlatPanelPrimitive,
                    PrimitiveKind::SpherePrimitive,
                ]
            } else {
                Vec::new()
            }
        }
        DirectKitSourceKind::Unsupported => Vec::new(),
    }
}

fn primitive_kind_from_source_ref(source_ref: &str) -> Option<PrimitiveKind> {
    let lower = source_ref.to_ascii_lowercase();
    if lower.contains("flat_panel") || lower.contains("flat-panel") || lower.contains("panel") {
        Some(PrimitiveKind::FlatPanelPrimitive)
    } else if lower.contains("sphere") || lower.contains("round") {
        Some(PrimitiveKind::SpherePrimitive)
    } else if lower.contains("box") {
        Some(PrimitiveKind::BoxPrimitive)
    } else {
        None
    }
}

fn source_ref_is_panel_with_knob(source_ref: &str) -> bool {
    let lower = source_ref.to_ascii_lowercase();
    lower.contains("panel_with_knob") || lower.contains("panel-with-knob")
}

fn validate_properties(
    properties: &[DirectKitPropertyExposure],
    subject: &str,
    allowed_kinds: &[PrimitiveKind],
    report: &mut DirectKitValidationReport,
) {
    let mut seen = BTreeSet::new();
    for (index, property) in properties.iter().enumerate() {
        let property_subject = format!("{subject}.{index}");
        validate_identifier(
            &property.property_id,
            format!("{property_subject}.property_id"),
            report,
        );
        validate_user_copy(
            &property.display_name,
            format!("{property_subject}.display_name"),
            report,
        );
        validate_user_copy(
            &property.user_description,
            format!("{property_subject}.user_description"),
            report,
        );

        if !seen.insert(property.property_id.clone()) {
            report.error(
                format!("{property_subject}.property_id"),
                "direct_kit_duplicate_property",
                "Direct Kit properties must not be duplicated within one role.",
            );
        }

        let schema_property = allowed_kinds
            .iter()
            .find_map(|primitive_kind| schema_property(*primitive_kind, &property.property_id));
        let Some((primitive_kind, schema_property)) = schema_property else {
            report.error(
                format!("{property_subject}.property_id"),
                "direct_kit_unknown_property",
                "Direct Kit property does not exist in the supported primitive schema.",
            );
            continue;
        };

        if property.domain != schema_property.domain {
            report.error(
                format!("{property_subject}.domain"),
                "direct_kit_property_domain_mismatch",
                "Direct Kit property domain must match the primitive schema.",
            );
        }
        if property.default_value != schema_property.default_value {
            report.error(
                format!("{property_subject}.default_value"),
                "direct_kit_property_default_mismatch",
                "Direct Kit property default must match the primitive schema.",
            );
        }
        validate_property_value(
            primitive_kind,
            &property.property_id,
            &property.current_value,
            format!("{property_subject}.current_value"),
            report,
        );
        validate_property_value(
            primitive_kind,
            &property.property_id,
            &property.default_value,
            format!("{property_subject}.default_value"),
            report,
        );
    }
}

fn schema_property(
    primitive_kind: PrimitiveKind,
    property_id: &str,
) -> Option<(PrimitiveKind, PrimitiveProperty)> {
    primitive_schema(primitive_kind)
        .properties
        .into_iter()
        .find(|property| property.property_id == property_id)
        .map(|property| (primitive_kind, property))
}

fn validate_property_value(
    primitive_kind: PrimitiveKind,
    property_id: &str,
    value: &PrimitivePropertyValue,
    subject: String,
    report: &mut DirectKitValidationReport,
) {
    let schema = primitive_schema(primitive_kind);
    let mut values = primitive_default_property_values(&schema);
    values.insert(property_id.to_owned(), value.clone());
    let nested = validate_primitive_property_values(&schema, &values);
    if !nested.is_valid() {
        report.error(
            subject,
            "direct_kit_property_value_invalid",
            "Direct Kit property value must stay inside its primitive schema domain.",
        );
    }
}

fn validate_no_duplicate_property_role(
    draft: &DirectKitDraft,
    report: &mut DirectKitValidationReport,
) {
    let changeable: BTreeSet<&str> = draft
        .changeable_properties
        .iter()
        .map(|property| property.property_id.as_str())
        .collect();
    for property in &draft.locked_properties {
        if changeable.contains(property.property_id.as_str()) {
            report.error(
                "locked_properties",
                "direct_kit_property_in_multiple_roles",
                "A Direct Kit property cannot be both changeable and locked.",
            );
        }
    }
}

fn validate_presets(
    draft: &DirectKitDraft,
    allowed_kinds: &[PrimitiveKind],
    report: &mut DirectKitValidationReport,
) {
    for (index, preset_ref) in draft.included_presets.iter().enumerate() {
        let subject = format!("included_presets.{index}");
        validate_identifier(
            &preset_ref.preset_id,
            format!("{subject}.preset_id"),
            report,
        );
        validate_user_copy(
            &preset_ref.display_name,
            format!("{subject}.display_name"),
            report,
        );
        if preset_ref.source == PresetSource::BuiltIn {
            let Some(preset) = built_in_primitive_preset(&preset_ref.preset_id) else {
                report.error(
                    format!("{subject}.preset_id"),
                    "direct_kit_unknown_preset",
                    "Built-in Direct Kit preset must exist.",
                );
                continue;
            };
            if !allowed_kinds.contains(&preset.primitive_kind) {
                report.error(
                    format!("{subject}.preset_id"),
                    "direct_kit_preset_mismatch",
                    "Direct Kit preset must match the source primitive or composition.",
                );
            }
        }
    }
}

fn validate_evidence(draft: &DirectKitDraft, report: &mut DirectKitValidationReport) {
    if draft.evidence_refs.is_empty() {
        report.warning(
            "evidence_refs",
            "direct_kit_missing_evidence",
            "Direct Kit is Draft-valid, but needs evidence before review.",
        );
    }
    for (index, evidence) in draft.evidence_refs.iter().enumerate() {
        let subject = format!("evidence_refs.{index}");
        if evidence.path.trim().is_empty() {
            report.warning(
                format!("{subject}.path"),
                "direct_kit_evidence_path_missing",
                "Direct Kit evidence path is missing.",
            );
        }
        if evidence.path.starts_with('/') || evidence.path.contains(":\\") {
            report.error(
                format!("{subject}.path"),
                "direct_kit_evidence_absolute_path",
                "Direct Kit evidence paths must be relative.",
            );
        }
        if !evidence.human_review_required {
            report.error(
                format!("{subject}.human_review_required"),
                "direct_kit_evidence_review_required",
                "Direct Kit evidence must remain review-required.",
            );
        }
        if matches!(
            evidence.status,
            DirectKitEvidenceStatus::Blocked
                | DirectKitEvidenceStatus::Failed
                | DirectKitEvidenceStatus::Missing
        ) {
            report.warning(
                format!("{subject}.status"),
                "direct_kit_evidence_not_passed",
                "Direct Kit evidence is incomplete or blocked.",
            );
        }
    }
}

fn validate_visibility(draft: &DirectKitDraft, report: &mut DirectKitValidationReport) {
    match draft.visibility {
        DirectKitVisibility::Draft | DirectKitVisibility::PersonalOnly => {}
        DirectKitVisibility::Reviewed => report.error(
            "visibility",
            "direct_kit_reviewed_visibility_rejected_v0",
            "Direct Kit V0 cannot use Reviewed visibility.",
        ),
        DirectKitVisibility::Showcase => report.error(
            "visibility",
            "direct_kit_showcase_visibility_rejected_v0",
            "Direct Kit V0 cannot use Showcase visibility.",
        ),
        DirectKitVisibility::PublicCatalog => report.error(
            "visibility",
            "direct_kit_public_catalog_visibility_rejected",
            "Direct Kit V0 cannot use public catalog visibility.",
        ),
    }

    if draft.review_tier != ObjectPlanReviewTier::Draft
        && draft.review_tier != ObjectPlanReviewTier::Personal
    {
        report.error(
            "review_tier",
            "direct_kit_review_tier_rejected_v0",
            "Direct Kit V0 must remain Draft or Personal.",
        );
    }
}

fn evidence_summary_lines(draft: &DirectKitDraft) -> Vec<String> {
    if draft.evidence_refs.is_empty() {
        return vec!["No test evidence has been added yet.".to_owned()];
    }
    draft
        .evidence_refs
        .iter()
        .map(|evidence| match evidence.evidence_kind {
            DirectKitEvidenceKind::PropertyEndpointSheet => {
                "Property endpoint checks are linked.".to_owned()
            }
            DirectKitEvidenceKind::PresetContactSheet => {
                "Preset review images are linked.".to_owned()
            }
            DirectKitEvidenceKind::ObjectPlanRenderEvidence => {
                "Draft review images are linked.".to_owned()
            }
            DirectKitEvidenceKind::ExportReport => "Geometry export report is linked.".to_owned(),
        })
        .collect()
}

fn validate_identifier(
    value: &str,
    subject: impl Into<String>,
    report: &mut DirectKitValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.error(
            subject,
            "direct_kit_identifier_required",
            "Direct Kit identifiers are required.",
        );
        return;
    }
    if !value.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    }) {
        report.error(
            subject,
            "direct_kit_identifier_not_normalized",
            "Direct Kit identifiers must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_user_copy(
    value: &str,
    subject: impl Into<String>,
    report: &mut DirectKitValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.error(
            subject.clone(),
            "direct_kit_user_copy_required",
            "Direct Kit user-facing copy is required.",
        );
    }

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
        "mesh payload",
    ] {
        if lower.contains(forbidden) {
            report.error(
                subject.clone(),
                "direct_kit_user_copy_forbidden_term",
                "Direct Kit user-facing copy must not expose internal technical terms.",
            );
        }
    }
}
