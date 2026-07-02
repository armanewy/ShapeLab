
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

/// Request to turn a validated ObjectPlan into an internal draft graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPlanMaterializationRequest {
    /// Source ObjectPlan.
    pub plan: ObjectPlan,
    /// Materialization safety policy.
    pub materialization_policy: MaterializationPolicy,
    /// Product-safe preview target, such as "clay-review".
    pub target_preview_profile: String,
    /// Output mode for the draft.
    pub output_mode: ObjectPlanMaterializationOutputMode,
}

/// Safety policy for ObjectPlan materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializationPolicy {
    /// Validation must pass before any draft can be materialized.
    pub require_valid_plan: bool,
    /// Primitive kinds must be supported by this materializer.
    pub require_supported_primitives: bool,
    /// Attachments must be supported by this materializer.
    pub require_supported_attachments: bool,
    /// Keep product-facing node labels in draft instances.
    pub preserve_node_labels: bool,
    /// Catalog publishing is forbidden from materialization.
    pub forbid_catalog_publish: bool,
}

impl Default for MaterializationPolicy {
    fn default() -> Self {
        Self {
            require_valid_plan: true,
            require_supported_primitives: true,
            require_supported_attachments: true,
            preserve_node_labels: true,
            forbid_catalog_publish: true,
        }
    }
}

/// Output mode for a materialized ObjectPlan.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ObjectPlanMaterializationOutputMode {
    /// Internal draft for review evidence.
    DraftReview,
}

/// Materialization status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MaterializationStatus {
    /// All supported nodes and attachments materialized.
    Passed,
    /// Some supported content materialized and unresolved content was reported.
    Partial,
    /// Materialization failed.
    Failed,
}

/// Materialized primitive instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedPrimitiveInstance {
    /// Source node ID.
    pub node_id: String,
    /// Supported primitive kind.
    pub primitive_kind: PrimitiveKind,
    /// Product-facing label.
    pub display_name: String,
    /// Validated property values.
    pub property_values: BTreeMap<String, PrimitivePropertyValue>,
    /// Whether review tools should avoid changing this instance.
    pub locked: bool,
}

/// Unresolved primitive node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnresolvedObjectPlanNode {
    /// Source node ID.
    pub node_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Reason the node was not materialized.
    pub reason: String,
}

/// Unresolved attachment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnresolvedObjectPlanAttachment {
    /// Source attachment ID.
    pub attachment_id: String,
    /// Parent node ID.
    pub parent_node_id: String,
    /// Child node ID.
    pub child_node_id: String,
    /// Reason the attachment was not materialized.
    pub reason: String,
}

/// Materialized draft graph produced from an ObjectPlan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedObjectDraft {
    /// Stable draft ID.
    pub draft_id: String,
    /// Source ObjectPlan ID.
    pub source_plan_id: String,
    /// Materialization status.
    pub status: MaterializationStatus,
    /// Supported primitive instances.
    pub primitive_instances: Vec<MaterializedPrimitiveInstance>,
    /// Supported composition document.
    pub composition_document: PrimitiveCompositionDocument,
    /// Relationship contracts derived from supported safe-anchor attachments.
    #[serde(default)]
    pub relationship_contracts: Vec<RelationshipContract>,
    /// Nodes that could not be materialized.
    pub unresolved_nodes: Vec<UnresolvedObjectPlanNode>,
    /// Attachments that could not be materialized.
    pub unresolved_attachments: Vec<UnresolvedObjectPlanAttachment>,
    /// Validation report used for this draft.
    pub validation_report: ObjectPlanValidationReport,
    /// Review tier for the draft.
    pub review_tier: ObjectPlanReviewTier,
    /// All materialized drafts require review.
    pub user_review_required: bool,
    /// Materialization never grants publishing rights.
    pub publish_allowed: bool,
}

/// Product-safe materialization summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterializedObjectSummary {
    /// Product-facing source plan label.
    pub source_plan_label: String,
    /// Supported primitive count.
    pub supported_primitive_count: usize,
    /// Unresolved primitive count.
    pub unresolved_primitive_count: usize,
    /// Supported attachment count.
    pub supported_attachment_count: usize,
    /// Unresolved attachment count.
    pub unresolved_attachment_count: usize,
    /// Human review is required.
    pub user_review_required: bool,
    /// Suggested next action.
    pub next_action: MaterializedObjectNextAction,
}

/// Product-safe next action for a materialized draft.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MaterializedObjectNextAction {
    /// Review rendered evidence.
    Review,
    /// Simplify unsupported structure.
    Simplify,
    /// Regenerate from the source request.
    Regenerate,
    /// Stop until the blocker is resolved.
    Blocked,
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
