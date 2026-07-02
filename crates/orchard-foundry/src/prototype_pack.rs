//! Prototype Pack brief contracts.
//!
//! These contracts describe a future batch asset-creation brief. They do not
//! implement generation, runtime LLM integration, approval, or publishing.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::PrimitiveKind;

/// Maximum number of draft options one asset request may ask for in V0.
pub const PROTOTYPE_PACK_BRIEF_MAX_DESIRED_COUNT: u32 = 24;

/// Future Prototype Pack brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackBrief {
    /// Stable brief ID.
    pub brief_id: String,
    /// Product-facing brief name.
    pub display_name: String,
    /// Product-facing purpose.
    pub purpose: String,
    /// Requested draft assets.
    pub asset_requests: Vec<AssetRequest>,
    /// Primitive and composition scope allowed for this brief.
    pub supported_primitive_scope: SupportedPrimitiveScope,
    /// Draft-only output policy.
    pub output_policy: PrototypePackOutputPolicy,
    /// Review policy for generated drafts.
    pub review_policy: PrototypePackReviewPolicy,
}

/// One requested draft asset in a future Prototype Pack brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRequest {
    /// Stable request ID.
    pub request_id: String,
    /// Product-facing asset name.
    pub display_name: String,
    /// Product-facing intended use.
    pub intended_use: String,
    /// Primitive kinds allowed for this request.
    pub allowed_primitives: Vec<PrimitiveKind>,
    /// Composition kinds allowed for this request.
    pub allowed_compositions: Vec<PrototypePackCompositionKind>,
    /// Number of draft options requested.
    pub desired_count: u32,
    /// Optional product-facing style note.
    pub style_hint: Option<String>,
    /// Capabilities required by this request.
    pub must_have_capabilities: Vec<PrototypePackCapability>,
    /// Capabilities explicitly blocked by this request.
    pub blocked_capabilities: Vec<PrototypePackCapability>,
}

/// Primitive and composition scope for a brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedPrimitiveScope {
    /// Primitive kinds supported by this brief.
    pub primitive_kinds: Vec<PrimitiveKind>,
    /// Composition kinds supported by this brief.
    pub composition_kinds: Vec<PrototypePackCompositionKind>,
}

/// Supported composition labels for Prototype Pack briefs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrototypePackCompositionKind {
    /// Supported safe-anchor composition.
    PanelWithKnob,
    /// Rejected placeholder for tests and future expansion.
    Unsupported,
}

/// Capability labels a brief may request or block.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrototypePackCapability {
    /// Draft ObjectPlan output.
    ObjectPlanDraft,
    /// Review image/contact sheet evidence.
    ReviewImage,
    /// Geometry-only GLB export.
    GeometryOnlyExport,
    /// Future visual surface work.
    MaterialSurface,
    /// Future UV/texturing work.
    UvTexturing,
    /// Future collision/gameplay metadata.
    Collision,
    /// Future rigging work.
    Rigging,
    /// Future animation work.
    Animation,
    /// Rejected game-ready status.
    GameReady,
    /// Rejected public catalog publishing.
    PublicCatalogPublishing,
    /// Rejected runtime LLM integration.
    RuntimeLlm,
}

/// Draft-only output policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackOutputPolicy {
    /// Outputs must remain Draft.
    pub draft_only: bool,
    /// Human review remains required.
    pub human_review_required: bool,
    /// V0 cannot approve assets automatically.
    pub approved: bool,
    /// V0 cannot publish assets.
    pub publish_allowed: bool,
}

impl Default for PrototypePackOutputPolicy {
    fn default() -> Self {
        Self {
            draft_only: true,
            human_review_required: true,
            approved: false,
            publish_allowed: false,
        }
    }
}

/// Review policy for future Prototype Pack drafts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackReviewPolicy {
    /// Human review remains required.
    pub human_review_required: bool,
    /// V0 forbids automatic approval.
    pub automatic_approval_allowed: bool,
}

impl Default for PrototypePackReviewPolicy {
    fn default() -> Self {
        Self {
            human_review_required: true,
            automatic_approval_allowed: false,
        }
    }
}

/// Product-facing summary for a future Prototype Pack brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackBriefSummary {
    /// Product-facing title.
    pub title: String,
    /// Requested draft assets.
    pub requested_assets: Vec<String>,
    /// Requests supported by current Draft infrastructure.
    pub supported_now: Vec<String>,
    /// Requests needing future capability work.
    pub needs_future_capabilities: Vec<String>,
    /// Draft-only review notice.
    pub draft_notice: String,
}

/// One Prototype Pack brief validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Validation report for Prototype Pack brief contracts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePackValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PrototypePackValidationIssue>,
}

impl PrototypePackValidationReport {
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
        self.issues.push(PrototypePackValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return the default V0 primitive and composition scope.
#[must_use]
pub fn prototype_pack_supported_scope_v0() -> SupportedPrimitiveScope {
    SupportedPrimitiveScope {
        primitive_kinds: vec![
            PrimitiveKind::BoxPrimitive,
            PrimitiveKind::FlatPanelPrimitive,
            PrimitiveKind::SpherePrimitive,
        ],
        composition_kinds: vec![PrototypePackCompositionKind::PanelWithKnob],
    }
}

/// Return true when a capability is currently supported for draft briefs.
#[must_use]
pub const fn prototype_pack_capability_supported_now(capability: PrototypePackCapability) -> bool {
    matches!(
        capability,
        PrototypePackCapability::ObjectPlanDraft
            | PrototypePackCapability::ReviewImage
            | PrototypePackCapability::GeometryOnlyExport
    )
}

/// Validate a future Prototype Pack brief.
#[must_use]
pub fn validate_prototype_pack_brief(brief: &PrototypePackBrief) -> PrototypePackValidationReport {
    let mut report = PrototypePackValidationReport::default();

    validate_identifier(&brief.brief_id, "brief_id", &mut report);
    validate_user_copy(&brief.display_name, "display_name", &mut report);
    validate_user_copy(&brief.purpose, "purpose", &mut report);
    validate_scope(&brief.supported_primitive_scope, &mut report);
    validate_output_policy(&brief.output_policy, &mut report);
    validate_review_policy(&brief.review_policy, &mut report);

    if brief.asset_requests.is_empty() {
        report.push(
            "asset_requests",
            "prototype_pack_asset_requests_required",
            "Prototype Pack briefs must request at least one draft asset.",
        );
    }

    let mut request_ids = BTreeSet::new();
    for (index, request) in brief.asset_requests.iter().enumerate() {
        validate_asset_request(
            request,
            index,
            &brief.supported_primitive_scope,
            &mut request_ids,
            &mut report,
        );
    }

    report
}

/// Build a product-safe summary for a Prototype Pack brief.
#[must_use]
pub fn prototype_pack_brief_summary(brief: &PrototypePackBrief) -> PrototypePackBriefSummary {
    let mut requested_assets = Vec::new();
    let mut supported_now = Vec::new();
    let mut needs_future_capabilities = Vec::new();

    for request in &brief.asset_requests {
        requested_assets.push(format!(
            "{}: {} draft option{}",
            request.display_name,
            request.desired_count,
            if request.desired_count == 1 { "" } else { "s" }
        ));

        let future_needs: Vec<String> = request
            .must_have_capabilities
            .iter()
            .copied()
            .filter(|capability| !prototype_pack_capability_supported_now(*capability))
            .map(future_capability_label)
            .collect();

        if future_needs.is_empty() {
            supported_now.push(request.display_name.clone());
        } else {
            needs_future_capabilities.push(format!(
                "{} needs {}.",
                request.display_name,
                join_product_list(&future_needs)
            ));
        }
    }

    PrototypePackBriefSummary {
        title: brief.display_name.clone(),
        requested_assets,
        supported_now,
        needs_future_capabilities,
        draft_notice: "Draft options need review before sharing.".to_owned(),
    }
}

fn validate_asset_request(
    request: &AssetRequest,
    index: usize,
    scope: &SupportedPrimitiveScope,
    request_ids: &mut BTreeSet<String>,
    report: &mut PrototypePackValidationReport,
) {
    let prefix = format!("asset_requests.{index}");

    validate_identifier(&request.request_id, format!("{prefix}.request_id"), report);
    if !request_ids.insert(request.request_id.clone()) {
        report.push(
            format!("{prefix}.request_id"),
            "prototype_pack_duplicate_request_id",
            "Prototype Pack asset request IDs must be unique.",
        );
    }
    validate_user_copy(
        &request.display_name,
        format!("{prefix}.display_name"),
        report,
    );
    validate_user_copy(
        &request.intended_use,
        format!("{prefix}.intended_use"),
        report,
    );
    if let Some(style_hint) = &request.style_hint {
        validate_user_copy(style_hint, format!("{prefix}.style_hint"), report);
    }

    if request.allowed_primitives.is_empty() && request.allowed_compositions.is_empty() {
        report.push(
            format!("{prefix}.allowed_primitives"),
            "prototype_pack_request_scope_required",
            "Asset requests must allow at least one primitive or composition.",
        );
    }
    for primitive_kind in &request.allowed_primitives {
        if !supported_primitive_now(*primitive_kind) {
            report.push(
                format!("{prefix}.allowed_primitives"),
                "prototype_pack_unsupported_primitive",
                "Asset request uses a primitive outside the current supported scope.",
            );
        }
        if !scope.primitive_kinds.contains(primitive_kind) {
            report.push(
                format!("{prefix}.allowed_primitives"),
                "prototype_pack_primitive_outside_brief_scope",
                "Asset request primitive is not listed in the brief scope.",
            );
        }
    }
    for composition_kind in &request.allowed_compositions {
        if !supported_composition_now(*composition_kind) {
            report.push(
                format!("{prefix}.allowed_compositions"),
                "prototype_pack_unsupported_composition",
                "Asset request uses a composition outside the current supported scope.",
            );
        }
        if !scope.composition_kinds.contains(composition_kind) {
            report.push(
                format!("{prefix}.allowed_compositions"),
                "prototype_pack_composition_outside_brief_scope",
                "Asset request composition is not listed in the brief scope.",
            );
        }
    }

    if request.desired_count == 0 || request.desired_count > PROTOTYPE_PACK_BRIEF_MAX_DESIRED_COUNT
    {
        report.push(
            format!("{prefix}.desired_count"),
            "prototype_pack_desired_count_out_of_bounds",
            "Asset request desired_count must stay within the V0 bound.",
        );
    }

    for capability in &request.must_have_capabilities {
        if !prototype_pack_capability_supported_now(*capability) {
            report.push(
                format!("{prefix}.must_have_capabilities"),
                "prototype_pack_unsupported_capability",
                "Asset request requires a capability outside Prototype Pack Brief V0.",
            );
        }
    }
}

fn validate_scope(scope: &SupportedPrimitiveScope, report: &mut PrototypePackValidationReport) {
    if scope.primitive_kinds.is_empty() && scope.composition_kinds.is_empty() {
        report.push(
            "supported_primitive_scope",
            "prototype_pack_scope_required",
            "Prototype Pack briefs must include a primitive or composition scope.",
        );
    }
    for primitive_kind in &scope.primitive_kinds {
        if !supported_primitive_now(*primitive_kind) {
            report.push(
                "supported_primitive_scope.primitive_kinds",
                "prototype_pack_unsupported_scope_primitive",
                "Prototype Pack Brief V0 only supports current primitive kinds.",
            );
        }
    }
    for composition_kind in &scope.composition_kinds {
        if !supported_composition_now(*composition_kind) {
            report.push(
                "supported_primitive_scope.composition_kinds",
                "prototype_pack_unsupported_scope_composition",
                "Prototype Pack Brief V0 only supports current composition kinds.",
            );
        }
    }
}

fn validate_output_policy(
    policy: &PrototypePackOutputPolicy,
    report: &mut PrototypePackValidationReport,
) {
    if !policy.draft_only {
        report.push(
            "output_policy.draft_only",
            "prototype_pack_draft_only_required",
            "Prototype Pack Brief V0 outputs must remain Draft.",
        );
    }
    if !policy.human_review_required {
        report.push(
            "output_policy.human_review_required",
            "prototype_pack_human_review_required",
            "Prototype Pack Brief V0 requires human review.",
        );
    }
    if policy.approved {
        report.push(
            "output_policy.approved",
            "prototype_pack_auto_approval_rejected",
            "Prototype Pack Brief V0 cannot approve assets automatically.",
        );
    }
    if policy.publish_allowed {
        report.push(
            "output_policy.publish_allowed",
            "prototype_pack_public_publishing_rejected",
            "Prototype Pack Brief V0 cannot publish assets.",
        );
    }
}

fn validate_review_policy(
    policy: &PrototypePackReviewPolicy,
    report: &mut PrototypePackValidationReport,
) {
    if !policy.human_review_required {
        report.push(
            "review_policy.human_review_required",
            "prototype_pack_review_required",
            "Prototype Pack Brief V0 requires review.",
        );
    }
    if policy.automatic_approval_allowed {
        report.push(
            "review_policy.automatic_approval_allowed",
            "prototype_pack_automatic_approval_rejected",
            "Prototype Pack Brief V0 cannot approve assets automatically.",
        );
    }
}

fn validate_identifier(
    value: &str,
    subject: impl Into<String>,
    report: &mut PrototypePackValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.push(
            subject,
            "prototype_pack_identifier_required",
            "Prototype Pack identifiers are required.",
        );
        return;
    }
    if !value.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    }) {
        report.push(
            subject,
            "prototype_pack_identifier_not_product_safe",
            "Prototype Pack identifiers must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_user_copy(
    value: &str,
    subject: impl Into<String>,
    report: &mut PrototypePackValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.push(
            subject.clone(),
            "prototype_pack_copy_required",
            "Prototype Pack user copy is required.",
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
        "game-ready",
        "marketplace",
        "public catalog",
        "runtime llm",
        "publish",
    ] {
        if lower.contains(forbidden) {
            report.push(
                subject.clone(),
                "prototype_pack_user_copy_forbidden_term",
                "Prototype Pack user copy must stay product-safe.",
            );
        }
    }
}

const fn supported_primitive_now(primitive_kind: PrimitiveKind) -> bool {
    matches!(
        primitive_kind,
        PrimitiveKind::BoxPrimitive
            | PrimitiveKind::FlatPanelPrimitive
            | PrimitiveKind::SpherePrimitive
    )
}

const fn supported_composition_now(composition_kind: PrototypePackCompositionKind) -> bool {
    matches!(
        composition_kind,
        PrototypePackCompositionKind::PanelWithKnob
    )
}

fn future_capability_label(capability: PrototypePackCapability) -> String {
    match capability {
        PrototypePackCapability::ObjectPlanDraft
        | PrototypePackCapability::ReviewImage
        | PrototypePackCapability::GeometryOnlyExport => "current draft support".to_owned(),
        PrototypePackCapability::MaterialSurface | PrototypePackCapability::UvTexturing => {
            "future visual finishing".to_owned()
        }
        PrototypePackCapability::Collision => "future engine behavior".to_owned(),
        PrototypePackCapability::Rigging | PrototypePackCapability::Animation => {
            "future movement setup".to_owned()
        }
        PrototypePackCapability::GameReady => "future play package review".to_owned(),
        PrototypePackCapability::PublicCatalogPublishing => "future sharing review".to_owned(),
        PrototypePackCapability::RuntimeLlm => "future request workflow".to_owned(),
    }
}

fn join_product_list(values: &[String]) -> String {
    match values {
        [] => "no future work".to_owned(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut joined = values[..values.len() - 1].join(", ");
            joined.push_str(", and ");
            joined.push_str(&values[values.len() - 1]);
            joined
        }
    }
}
