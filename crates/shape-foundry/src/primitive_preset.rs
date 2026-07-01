//! Deterministic primitive property presets.
//!
//! Presets are named property-value bundles for approved primitive schemas.
//! They do not generate mesh, expose raw modeling controls, or publish content.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ObjectPlanNode, ObjectPlanReviewTier, PrimitiveKind, PrimitivePropertySchema,
    PrimitivePropertyValidationReport, PrimitivePropertyValue, box_primitive_property_schema,
    flat_panel_primitive_property_schema, sphere_primitive_property_schema,
    validate_primitive_property_values,
};

/// One deterministic primitive property preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePreset {
    /// Stable preset ID.
    pub preset_id: String,
    /// Product-facing preset name.
    pub display_name: String,
    /// Primitive kind this preset configures.
    pub primitive_kind: PrimitiveKind,
    /// Property values keyed by primitive property ID.
    pub property_values: BTreeMap<String, PrimitivePropertyValue>,
    /// Product-safe description of the preset.
    pub user_description: String,
    /// Product-facing use tags.
    pub intended_use_tags: Vec<String>,
    /// Review tier. Built-ins are reviewed; external suggestions start as Draft.
    pub review_tier: ObjectPlanReviewTier,
    /// Source of the preset.
    pub source: PresetSource,
}

/// Source of a primitive preset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PresetSource {
    /// Authored in the product.
    BuiltIn,
    /// Saved locally by a user.
    UserSaved,
    /// Proposed by an ObjectPlan draft.
    ObjectPlanDraft,
    /// Produced by an internal offline tool.
    InternalTool,
}

/// One primitive preset validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePresetValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Primitive preset validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePresetValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PrimitivePresetValidationIssue>,
}

impl PrimitivePresetValidationReport {
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
        self.issues.push(PrimitivePresetValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    fn extend_property_report(&mut self, prefix: &str, nested: PrimitivePropertyValidationReport) {
        for issue in nested.issues {
            self.push(
                format!("{prefix}.{}", issue.subject),
                issue.code,
                issue.message,
            );
        }
    }
}

/// Error returned when building an ObjectPlan node from a preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitivePresetObjectPlanNodeError {
    /// The preset failed validation.
    InvalidPreset(PrimitivePresetValidationReport),
    /// Only reviewed presets may seed ObjectPlan nodes.
    PresetRequiresReview,
}

/// Return the built-in deterministic primitive presets.
#[must_use]
pub fn built_in_primitive_presets() -> Vec<PrimitivePreset> {
    vec![
        box_preset(
            "compact_box",
            "Compact Box",
            [("width", 1.0), ("depth", 0.9), ("height", 0.8)],
            0.08,
            "A small balanced box form.",
            &["box", "compact"],
        ),
        box_preset(
            "wide_box",
            "Wide Box",
            [("width", 3.2), ("depth", 1.2), ("height", 1.0)],
            0.08,
            "A low wide box form.",
            &["box", "wide"],
        ),
        box_preset(
            "tall_box",
            "Tall Box",
            [("width", 1.2), ("depth", 1.0), ("height", 2.6)],
            0.07,
            "A taller upright box form.",
            &["box", "tall"],
        ),
        box_preset(
            "flat_box",
            "Flat Box",
            [("width", 2.4), ("depth", 1.4), ("height", 0.35)],
            0.05,
            "A shallow box form.",
            &["box", "flat"],
        ),
        panel_preset(
            "narrow_panel",
            "Narrow Panel",
            [("width", 0.9), ("height", 2.6), ("thickness", 0.18)],
            0.05,
            "A slim upright panel.",
            &["panel", "narrow"],
        ),
        panel_preset(
            "wide_panel",
            "Wide Panel",
            [("width", 3.0), ("height", 2.0), ("thickness", 0.18)],
            0.05,
            "A broad panel form.",
            &["panel", "wide"],
        ),
        panel_preset(
            "tall_panel",
            "Tall Panel",
            [("width", 1.4), ("height", 4.0), ("thickness", 0.18)],
            0.05,
            "A taller panel form.",
            &["panel", "tall"],
        ),
        panel_preset(
            "short_panel",
            "Short Panel",
            [("width", 1.8), ("height", 1.2), ("thickness", 0.18)],
            0.05,
            "A short panel form.",
            &["panel", "short"],
        ),
        sphere_preset(
            "round_sphere",
            "Round Sphere",
            [("width", 1.0), ("height", 1.0), ("depth", 1.0)],
            [("front_flatten", 0.0), ("back_flatten", 0.0)],
            "A balanced round sphere.",
            &["sphere", "round"],
        ),
        sphere_preset(
            "squashed_sphere",
            "Squashed Sphere",
            [("width", 1.4), ("height", 0.7), ("depth", 1.2)],
            [("front_flatten", 0.0), ("back_flatten", 0.0)],
            "A low rounded form.",
            &["sphere", "squashed"],
        ),
        sphere_preset(
            "flattened_back_sphere",
            "Flattened Back Sphere",
            [("width", 1.1), ("height", 1.1), ("depth", 0.9)],
            [("front_flatten", 0.0), ("back_flatten", 0.45)],
            "A round form with a flat back.",
            &["sphere", "flat-back"],
        ),
        sphere_preset(
            "knob_like_form",
            "Knob-Like Form",
            [("width", 0.55), ("height", 0.55), ("depth", 0.35)],
            [("front_flatten", 0.05), ("back_flatten", 0.25)],
            "A small rounded form for safe attachments.",
            &["sphere", "attachment"],
        ),
    ]
}

/// Return one built-in primitive preset by ID.
#[must_use]
pub fn built_in_primitive_preset(preset_id: &str) -> Option<PrimitivePreset> {
    built_in_primitive_presets()
        .into_iter()
        .find(|preset| preset.preset_id == preset_id)
}

/// Validate one primitive preset.
#[must_use]
pub fn validate_primitive_preset(preset: &PrimitivePreset) -> PrimitivePresetValidationReport {
    let mut report = PrimitivePresetValidationReport::default();
    validate_identifier(
        &mut report,
        "preset_id",
        &preset.preset_id,
        "invalid_preset_id",
    );
    validate_product_copy(
        &mut report,
        "display_name",
        &preset.display_name,
        "invalid_preset_display_name",
    );
    validate_product_copy(
        &mut report,
        "user_description",
        &preset.user_description,
        "invalid_preset_user_description",
    );
    if preset.intended_use_tags.is_empty() {
        report.push(
            "intended_use_tags",
            "missing_preset_tags",
            "Primitive presets must include at least one product-facing use tag.",
        );
    }
    for (index, tag) in preset.intended_use_tags.iter().enumerate() {
        validate_identifier(
            &mut report,
            format!("intended_use_tags.{index}"),
            tag,
            "invalid_preset_tag",
        );
    }

    let Some(schema) = primitive_property_schema_for_kind(preset.primitive_kind) else {
        report.push(
            "primitive_kind",
            "unsupported_preset_primitive_kind",
            "Primitive preset references an unsupported primitive kind.",
        );
        return report;
    };
    report.extend_property_report(
        "property_values",
        validate_primitive_property_values(&schema, &preset.property_values),
    );

    report
}

/// Return false for every preset; presets never publish to a public catalog.
#[must_use]
pub const fn primitive_preset_public_catalog_publish_allowed(_preset: &PrimitivePreset) -> bool {
    false
}

/// Build an ObjectPlan node from a reviewed preset.
pub fn object_plan_node_from_reviewed_preset(
    preset: &PrimitivePreset,
    node_id: impl Into<String>,
    role_hint: impl Into<String>,
    locked: bool,
) -> Result<ObjectPlanNode, PrimitivePresetObjectPlanNodeError> {
    let report = validate_primitive_preset(preset);
    if !report.is_valid() {
        return Err(PrimitivePresetObjectPlanNodeError::InvalidPreset(report));
    }
    if preset.review_tier != ObjectPlanReviewTier::Reviewed {
        return Err(PrimitivePresetObjectPlanNodeError::PresetRequiresReview);
    }

    Ok(ObjectPlanNode {
        node_id: node_id.into(),
        primitive_kind: preset.primitive_kind,
        display_name: preset.display_name.clone(),
        property_values: preset.property_values.clone(),
        role_hint: role_hint.into(),
        locked,
    })
}

fn box_preset(
    preset_id: &str,
    display_name: &str,
    lengths: [(&str, f32); 3],
    edge_softness: f32,
    user_description: &str,
    intended_use_tags: &[&str],
) -> PrimitivePreset {
    let mut property_values = length_values(lengths);
    property_values.insert(
        "edge_softness".to_owned(),
        PrimitivePropertyValue::Ratio(edge_softness),
    );
    preset(
        preset_id,
        display_name,
        PrimitiveKind::BoxPrimitive,
        property_values,
        user_description,
        intended_use_tags,
    )
}

fn panel_preset(
    preset_id: &str,
    display_name: &str,
    lengths: [(&str, f32); 3],
    edge_softness: f32,
    user_description: &str,
    intended_use_tags: &[&str],
) -> PrimitivePreset {
    let mut property_values = length_values(lengths);
    property_values.insert(
        "edge_softness".to_owned(),
        PrimitivePropertyValue::Ratio(edge_softness),
    );
    preset(
        preset_id,
        display_name,
        PrimitiveKind::FlatPanelPrimitive,
        property_values,
        user_description,
        intended_use_tags,
    )
}

fn sphere_preset(
    preset_id: &str,
    display_name: &str,
    lengths: [(&str, f32); 3],
    flattening: [(&str, f32); 2],
    user_description: &str,
    intended_use_tags: &[&str],
) -> PrimitivePreset {
    let mut property_values = length_values(lengths);
    for (property_id, value) in flattening {
        property_values.insert(property_id.to_owned(), PrimitivePropertyValue::Ratio(value));
    }
    preset(
        preset_id,
        display_name,
        PrimitiveKind::SpherePrimitive,
        property_values,
        user_description,
        intended_use_tags,
    )
}

fn preset(
    preset_id: &str,
    display_name: &str,
    primitive_kind: PrimitiveKind,
    property_values: BTreeMap<String, PrimitivePropertyValue>,
    user_description: &str,
    intended_use_tags: &[&str],
) -> PrimitivePreset {
    PrimitivePreset {
        preset_id: preset_id.to_owned(),
        display_name: display_name.to_owned(),
        primitive_kind,
        property_values,
        user_description: user_description.to_owned(),
        intended_use_tags: intended_use_tags
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect(),
        review_tier: ObjectPlanReviewTier::Reviewed,
        source: PresetSource::BuiltIn,
    }
}

fn length_values(values: [(&str, f32); 3]) -> BTreeMap<String, PrimitivePropertyValue> {
    values
        .into_iter()
        .map(|(property_id, value)| {
            (
                property_id.to_owned(),
                PrimitivePropertyValue::Length(value),
            )
        })
        .collect()
}

fn primitive_property_schema_for_kind(
    primitive_kind: PrimitiveKind,
) -> Option<PrimitivePropertySchema> {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => Some(box_primitive_property_schema()),
        PrimitiveKind::FlatPanelPrimitive => Some(flat_panel_primitive_property_schema()),
        PrimitiveKind::SpherePrimitive => Some(sphere_primitive_property_schema()),
        PrimitiveKind::CylinderPrimitive => None,
    }
}

fn validate_identifier(
    report: &mut PrimitivePresetValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
        || contains_internal_term(value)
    {
        report.push(
            subject,
            code,
            "Primitive preset IDs and tags must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_copy(
    report: &mut PrimitivePresetValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blocked_capability_term(trimmed)
        || trimmed.contains("::")
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        report.push(subject, code, "Primitive preset copy must be product-safe.");
    }
}

fn contains_internal_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "raw transform",
        "mesh payload",
        "conformance",
        "artifact",
        "recipe",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn contains_blocked_capability_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "uv",
        "texture",
        "rigging",
        "animation",
        "game-ready",
        "vertex",
        "vertices",
        "face edit",
        "boolean",
        "mesh edit",
        "door knob",
    ]
    .iter()
    .any(|term| lower.contains(term))
}
