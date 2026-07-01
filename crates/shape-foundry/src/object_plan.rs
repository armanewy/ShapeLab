//! Structured ObjectPlan contracts for offline primitive planning.
//!
//! ObjectPlans are bounded primitive and composition descriptions. They are
//! intentionally closed to raw mesh payloads, arbitrary transforms, and public
//! publishing shortcuts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    PrimitiveAttachment, PrimitiveAttachmentOffsetPolicy, PrimitiveAttachmentOrientationPolicy,
    PrimitiveAttachmentScalePolicy, PrimitiveCompositionDocument,
    PrimitiveCompositionValidationReport, PrimitiveKind, PrimitiveNode, PrimitiveNodeVisibility,
    PrimitivePropertySchema, PrimitivePropertyValidationReport, PrimitivePropertyValue,
    box_primitive_property_schema, flat_panel_primitive_property_schema,
    primitive_anchor_definitions, sphere_primitive_property_schema,
    validate_primitive_composition_document, validate_primitive_property_values,
};

/// Current schema version for ObjectPlan contracts.
pub const OBJECT_PLAN_SCHEMA_VERSION: u32 = 1;

/// A structured offline object plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlan {
    /// Schema version.
    pub schema_version: u32,
    /// Stable plan ID.
    pub plan_id: String,
    /// Product-facing plan name.
    pub display_name: String,
    /// Product-safe intent summary.
    pub intent_summary: String,
    /// Primitive nodes in the plan.
    pub nodes: Vec<ObjectPlanNode>,
    /// Safe-anchor attachments between nodes.
    pub attachments: Vec<ObjectPlanAttachment>,
    /// Validation requirements for this plan.
    pub validation_policy: ObjectPlanValidationPolicy,
    /// Review tier. Missing values deserialize as Draft.
    #[serde(default)]
    pub review_tier: ObjectPlanReviewTier,
    /// Plan source metadata.
    pub provenance: ObjectPlanProvenance,
}

/// One primitive node in an ObjectPlan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanNode {
    /// Stable node ID.
    pub node_id: String,
    /// Primitive kind.
    pub primitive_kind: PrimitiveKind,
    /// Product-facing node name.
    pub display_name: String,
    /// Primitive property values keyed by property ID.
    pub property_values: BTreeMap<String, PrimitivePropertyValue>,
    /// Product-safe role hint for summaries and review.
    pub role_hint: String,
    /// Whether review tools should avoid changing this node.
    pub locked: bool,
}

/// One safe-anchor attachment in an ObjectPlan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanAttachment {
    /// Stable attachment ID.
    pub attachment_id: String,
    /// Parent node ID.
    pub parent_node_id: String,
    /// Parent anchor ID.
    pub parent_anchor_id: String,
    /// Child node ID.
    pub child_node_id: String,
    /// Child anchor ID.
    pub child_anchor_id: String,
    /// Bounded offset policy.
    pub offset: PrimitiveAttachmentOffsetPolicy,
    /// Derived orientation policy.
    pub orientation_policy: PrimitiveAttachmentOrientationPolicy,
    /// Scale policy.
    pub scale_policy: PrimitiveAttachmentScalePolicy,
}

/// Validation policy for ObjectPlan intake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanValidationPolicy {
    /// Primitive property schema validation must remain enabled.
    pub require_primitive_schema_validation: bool,
    /// Safe-anchor validation must remain enabled.
    pub require_anchor_validation: bool,
    /// Public catalog publishing is not allowed by ObjectPlan intake.
    pub allow_public_catalog_publish: bool,
}

impl Default for ObjectPlanValidationPolicy {
    fn default() -> Self {
        Self {
            require_primitive_schema_validation: true,
            require_anchor_validation: true,
            allow_public_catalog_publish: false,
        }
    }
}

/// Review tier for an ObjectPlan.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ObjectPlanReviewTier {
    /// Draft plan that needs review before becoming a local asset.
    #[default]
    Draft,
    /// Personal local plan accepted by a user.
    Personal,
    /// Reviewed local plan.
    Reviewed,
}

/// Source metadata for an ObjectPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanProvenance {
    /// Source type.
    pub created_by: ObjectPlanCreatedBy,
    /// Optional hash of an offline prompt or tool request.
    pub source_prompt_hash: Option<String>,
    /// Product-safe seed references.
    pub source_seed_refs: Vec<String>,
    /// Creation timestamp supplied by the offline tool.
    pub created_at: String,
}

/// Allowed ObjectPlan creators.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ObjectPlanCreatedBy {
    /// Human-authored plan.
    Human,
    /// Internal offline tool.
    InternalTool,
    /// Offline LLM draft.
    LlmDraft,
}

/// Offline repair suggestion for an ObjectPlan validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanRepairSuggestion {
    /// Stable finding ID from a validator or review report.
    pub finding_id: String,
    /// Product-safe repair summary.
    pub summary: String,
    /// Product-safe suggested change.
    pub suggested_change: String,
    /// Optional target node ID.
    pub target_node_id: Option<String>,
    /// Optional target property ID.
    pub target_property_id: Option<String>,
    /// Optional target attachment ID.
    pub target_attachment_id: Option<String>,
    /// Risk level for the suggested change.
    pub risk: ObjectPlanRepairRisk,
    /// All repair suggestions require a human review step.
    pub requires_human_review: bool,
}

/// Risk level for an ObjectPlan repair suggestion.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ObjectPlanRepairRisk {
    /// Narrow value adjustment inside an approved schema.
    Low,
    /// Structural adjustment to an approved node or attachment.
    Medium,
    /// Change may alter object intent and needs careful review.
    High,
    /// Unsupported request should remain blocked.
    Blocked,
}

/// Product-safe summary of an ObjectPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanUserSummary {
    /// Product-facing plan name.
    pub display_name: String,
    /// Product-safe intent summary.
    pub intent_summary: String,
    /// Primitive descriptions.
    pub primitives_used: Vec<String>,
    /// Adjustable property descriptions.
    pub adjustable_properties: Vec<String>,
    /// Attachment descriptions.
    pub attachments: Vec<String>,
    /// Draft/personal/reviewed summary.
    pub review_summary: String,
}

/// One ObjectPlan validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// ObjectPlan validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<ObjectPlanValidationIssue>,
}

impl ObjectPlanValidationReport {
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
        self.issues.push(ObjectPlanValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Validate an ObjectPlan before an offline tool can render or save it.
#[must_use]
pub fn validate_object_plan(plan: &ObjectPlan) -> ObjectPlanValidationReport {
    let mut report = ObjectPlanValidationReport::default();

    if plan.schema_version != OBJECT_PLAN_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_object_plan_schema",
            "ObjectPlan schema version is not supported.",
        );
    }
    validate_identifier(
        &mut report,
        "plan_id",
        &plan.plan_id,
        "invalid_object_plan_id",
    );
    validate_product_text(
        &mut report,
        "display_name",
        &plan.display_name,
        "invalid_object_plan_display_name",
    );
    validate_product_text(
        &mut report,
        "intent_summary",
        &plan.intent_summary,
        "invalid_object_plan_intent_summary",
    );
    validate_policy(&mut report, &plan.validation_policy);
    validate_provenance(&mut report, &plan.provenance, plan.review_tier);

    if plan.nodes.is_empty() {
        report.push(
            "nodes",
            "missing_object_plan_nodes",
            "ObjectPlans must contain at least one primitive node.",
        );
    }

    let mut node_ids = BTreeSet::new();
    for (index, node) in plan.nodes.iter().enumerate() {
        validate_node(&mut report, index, node);
        if !node_ids.insert(node.node_id.as_str()) {
            report.push(
                format!("nodes.{index}.node_id"),
                "duplicate_object_plan_node",
                "ObjectPlan node IDs must be unique.",
            );
        }
    }

    extend_composition_report(
        &mut report,
        validate_primitive_composition_document(&object_plan_composition_document(plan)),
    );

    report
}

/// Build a product-safe summary for review UI or offline reports.
#[must_use]
pub fn object_plan_user_summary(plan: &ObjectPlan) -> ObjectPlanUserSummary {
    let nodes = plan
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let primitives_used = plan
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{} as {}",
                primitive_display_name(node.primitive_kind),
                node.display_name
            )
        })
        .collect::<Vec<_>>();
    let adjustable_properties = plan
        .nodes
        .iter()
        .map(|node| {
            let properties = primitive_property_schema_for_kind(node.primitive_kind)
                .map(|schema| {
                    schema
                        .properties
                        .iter()
                        .map(|property| property.display_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "No approved properties".to_owned());
            format!("{}: {}", node.display_name, properties)
        })
        .collect::<Vec<_>>();
    let attachments = plan
        .attachments
        .iter()
        .map(|attachment| attachment_summary(attachment, &nodes))
        .collect::<Vec<_>>();

    ObjectPlanUserSummary {
        display_name: plan.display_name.clone(),
        intent_summary: plan.intent_summary.clone(),
        primitives_used,
        adjustable_properties,
        attachments,
        review_summary: match plan.review_tier {
            ObjectPlanReviewTier::Draft => "Draft plan. Review before keeping.".to_owned(),
            ObjectPlanReviewTier::Personal => "Personal plan kept for local use.".to_owned(),
            ObjectPlanReviewTier::Reviewed => "Reviewed plan.".to_owned(),
        },
    }
}

/// Validate an offline ObjectPlan repair suggestion.
#[must_use]
pub fn validate_object_plan_repair_suggestion(
    suggestion: &ObjectPlanRepairSuggestion,
) -> ObjectPlanValidationReport {
    let mut report = ObjectPlanValidationReport::default();
    validate_identifier(
        &mut report,
        "finding_id",
        &suggestion.finding_id,
        "invalid_repair_finding_id",
    );
    validate_product_text(
        &mut report,
        "summary",
        &suggestion.summary,
        "invalid_repair_summary",
    );
    validate_product_text(
        &mut report,
        "suggested_change",
        &suggestion.suggested_change,
        "invalid_repair_suggested_change",
    );
    if let Some(node_id) = &suggestion.target_node_id {
        validate_identifier(
            &mut report,
            "target_node_id",
            node_id,
            "invalid_repair_target_node_id",
        );
    }
    if let Some(property_id) = &suggestion.target_property_id {
        validate_identifier(
            &mut report,
            "target_property_id",
            property_id,
            "invalid_repair_target_property_id",
        );
    }
    if let Some(attachment_id) = &suggestion.target_attachment_id {
        validate_identifier(
            &mut report,
            "target_attachment_id",
            attachment_id,
            "invalid_repair_target_attachment_id",
        );
    }
    if !suggestion.requires_human_review {
        report.push(
            "requires_human_review",
            "repair_requires_human_review",
            "ObjectPlan repair suggestions require human review.",
        );
    }
    report
}

fn validate_policy(report: &mut ObjectPlanValidationReport, policy: &ObjectPlanValidationPolicy) {
    if !policy.require_primitive_schema_validation {
        report.push(
            "validation_policy.require_primitive_schema_validation",
            "primitive_schema_validation_required",
            "ObjectPlan intake cannot bypass primitive schema validation.",
        );
    }
    if !policy.require_anchor_validation {
        report.push(
            "validation_policy.require_anchor_validation",
            "anchor_validation_required",
            "ObjectPlan intake cannot bypass safe-anchor validation.",
        );
    }
    if policy.allow_public_catalog_publish {
        report.push(
            "validation_policy.allow_public_catalog_publish",
            "public_catalog_publish_rejected",
            "ObjectPlan intake cannot publish directly to the public catalog.",
        );
    }
}

fn validate_provenance(
    report: &mut ObjectPlanValidationReport,
    provenance: &ObjectPlanProvenance,
    review_tier: ObjectPlanReviewTier,
) {
    if let Some(hash) = &provenance.source_prompt_hash {
        validate_reference_text(
            report,
            "provenance.source_prompt_hash",
            hash,
            "invalid_source_prompt_hash",
        );
    }
    for (index, seed_ref) in provenance.source_seed_refs.iter().enumerate() {
        validate_reference_text(
            report,
            format!("provenance.source_seed_refs.{index}"),
            seed_ref,
            "invalid_source_seed_ref",
        );
    }
    validate_reference_text(
        report,
        "provenance.created_at",
        &provenance.created_at,
        "invalid_object_plan_created_at",
    );
    if provenance.created_by == ObjectPlanCreatedBy::LlmDraft
        && review_tier != ObjectPlanReviewTier::Draft
    {
        report.push(
            "review_tier",
            "llm_draft_must_remain_draft",
            "Offline LLM ObjectPlans remain Draft until reviewed.",
        );
    }
}

fn validate_node(report: &mut ObjectPlanValidationReport, index: usize, node: &ObjectPlanNode) {
    let subject = format!("nodes.{index}");
    validate_identifier(
        report,
        format!("{subject}.node_id"),
        &node.node_id,
        "invalid_object_plan_node_id",
    );
    validate_product_text(
        report,
        format!("{subject}.display_name"),
        &node.display_name,
        "invalid_object_plan_node_display_name",
    );
    validate_product_text(
        report,
        format!("{subject}.role_hint"),
        &node.role_hint,
        "invalid_object_plan_node_role_hint",
    );

    let Some(schema) = primitive_property_schema_for_kind(node.primitive_kind) else {
        report.push(
            format!("{subject}.primitive_kind"),
            "unsupported_object_plan_primitive_kind",
            "ObjectPlan node primitive kind is not supported.",
        );
        return;
    };
    extend_property_report(
        report,
        &subject,
        validate_primitive_property_values(&schema, &node.property_values),
    );
}

fn object_plan_composition_document(plan: &ObjectPlan) -> PrimitiveCompositionDocument {
    PrimitiveCompositionDocument {
        schema_version: crate::PRIMITIVE_COMPOSITION_SCHEMA_VERSION,
        document_id: plan.plan_id.clone(),
        nodes: plan
            .nodes
            .iter()
            .map(|node| PrimitiveNode {
                node_id: node.node_id.clone(),
                primitive_kind: node.primitive_kind,
                property_values: node.property_values.clone(),
                local_label: node.display_name.clone(),
                visibility: PrimitiveNodeVisibility::Visible,
            })
            .collect(),
        attachments: plan
            .attachments
            .iter()
            .map(|attachment| PrimitiveAttachment {
                attachment_id: attachment.attachment_id.clone(),
                parent_node_id: attachment.parent_node_id.clone(),
                parent_anchor_id: attachment.parent_anchor_id.clone(),
                child_node_id: attachment.child_node_id.clone(),
                child_anchor_id: attachment.child_anchor_id.clone(),
                offset_policy: attachment.offset.clone(),
                orientation_policy: attachment.orientation_policy,
                scale_policy: attachment.scale_policy,
            })
            .collect(),
        root_node_id: plan
            .nodes
            .first()
            .map(|node| node.node_id.clone())
            .unwrap_or_default(),
    }
}

fn extend_property_report(
    report: &mut ObjectPlanValidationReport,
    subject: &str,
    nested: PrimitivePropertyValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("{subject}.property_values.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn extend_composition_report(
    report: &mut ObjectPlanValidationReport,
    nested: PrimitiveCompositionValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("composition.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn attachment_summary(
    attachment: &ObjectPlanAttachment,
    nodes: &BTreeMap<&str, &ObjectPlanNode>,
) -> String {
    let parent = nodes
        .get(attachment.parent_node_id.as_str())
        .map(|node| node.display_name.as_str())
        .unwrap_or("Parent primitive");
    let child = nodes
        .get(attachment.child_node_id.as_str())
        .map(|node| node.display_name.as_str())
        .unwrap_or("Child primitive");
    let anchor = nodes
        .get(attachment.parent_node_id.as_str())
        .and_then(|node| {
            parent_anchor_display_name(node.primitive_kind, &attachment.parent_anchor_id)
        })
        .unwrap_or_else(|| "approved anchor".to_owned());
    format!("{child} attaches to {parent} at {anchor}.")
}

fn parent_anchor_display_name(primitive_kind: PrimitiveKind, anchor_id: &str) -> Option<String> {
    primitive_anchor_definitions(primitive_kind, "summary")
        .into_iter()
        .find(|anchor| anchor.anchor_id == anchor_id)
        .map(|anchor| anchor.display_name)
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

fn primitive_display_name(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "Box Primitive",
        PrimitiveKind::FlatPanelPrimitive => "Flat Panel Primitive",
        PrimitiveKind::SpherePrimitive => "Sphere Primitive",
        PrimitiveKind::CylinderPrimitive => "Unsupported primitive",
    }
}

fn validate_identifier(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
        || contains_internal_term(value)
        || looks_like_path(value)
    {
        report.push(
            subject,
            code,
            "ObjectPlan IDs must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_text(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || looks_like_path(trimmed)
        || trimmed.contains("::")
    {
        report.push(
            subject,
            code,
            "ObjectPlan product-facing text must be product-safe.",
        );
    }
}

fn validate_reference_text(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || looks_like_path(trimmed)
        || trimmed.contains("::")
    {
        report.push(
            subject,
            code,
            "ObjectPlan source references must be product-safe and local-path free.",
        );
    }
}

fn contains_internal_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "kernel",
        "module",
        "provider",
        "slot",
        "fingerprint",
        "operation id",
        "scalar path",
        "raw transform",
        "matrix",
        "mesh payload",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn contains_blender_like_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "vertex", "vertices", "face", "faces", "loop", "loops", "cage", "boolean", "sculpt",
        "topology", "mesh", "gizmo", "blender",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.contains("~/")
}
