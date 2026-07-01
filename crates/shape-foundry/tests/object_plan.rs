use shape_foundry::{
    MaterializationPolicy, MaterializationStatus, MaterializedObjectNextAction,
    OBJECT_PLAN_SCHEMA_VERSION, ObjectPlan, ObjectPlanAttachment, ObjectPlanCreatedBy,
    ObjectPlanMaterializationOutputMode, ObjectPlanMaterializationRequest, ObjectPlanNode,
    ObjectPlanProvenance, ObjectPlanRepairRisk, ObjectPlanRepairSuggestion, ObjectPlanReviewTier,
    ObjectPlanUserSummary, ObjectPlanValidationPolicy, ObjectPlanValidationReport,
    PrimitiveAttachmentOffsetPolicy, PrimitiveAttachmentOrientationPolicy,
    PrimitiveAttachmentScalePolicy, PrimitiveKind, PrimitivePropertyValue,
    box_primitive_property_schema, flat_panel_primitive_property_schema, materialize_object_plan,
    materialized_object_summary, object_plan_user_summary, primitive_default_property_values,
    sphere_primitive_property_schema, validate_object_plan, validate_object_plan_repair_suggestion,
};

const DRAFT_PROMPT_PACK: &str =
    include_str!("../../../docs/llm_prompt_packs/object_plan_draft_v0.md");
const REPAIR_PROMPT_PACK: &str =
    include_str!("../../../docs/llm_prompt_packs/object_plan_repair_v0.md");
const OBJECT_PLAN_STATUS_DOCS: &[(&str, &str)] = &[
    ("README.md", include_str!("../../../README.md")),
    (
        "docs/CURRENT_PRODUCT_STATUS.md",
        include_str!("../../../docs/CURRENT_PRODUCT_STATUS.md"),
    ),
    (
        "docs/OBJECT_PLAN_V0_INTEGRATION_REPORT.md",
        include_str!("../../../docs/OBJECT_PLAN_V0_INTEGRATION_REPORT.md"),
    ),
    (
        "docs/OBJECT_PLAN_V0_TRUTH_RENDER_BLOCKER_GATE.md",
        include_str!("../../../docs/OBJECT_PLAN_V0_TRUTH_RENDER_BLOCKER_GATE.md"),
    ),
    (
        "docs/OBJECT_PLAN_OFFLINE_RUNNER_CLI.md",
        include_str!("../../../docs/OBJECT_PLAN_OFFLINE_RUNNER_CLI.md"),
    ),
    (
        "docs/OBJECT_PLAN_CONTACT_SHEET_EVIDENCE.md",
        include_str!("../../../docs/OBJECT_PLAN_CONTACT_SHEET_EVIDENCE.md"),
    ),
    (
        "docs/KNOWN_LIMITATIONS.md",
        include_str!("../../../docs/KNOWN_LIMITATIONS.md"),
    ),
];

#[test]
fn object_plan_one_sphere_validates() {
    let plan = one_sphere_plan();

    let report = validate_object_plan(&plan);

    assert_valid(&report);
}

#[test]
fn object_plan_panel_plus_sphere_attachment_validates() {
    let plan = panel_with_sphere_plan();

    let report = validate_object_plan(&plan);

    assert_valid(&report);
    assert_eq!(
        plan.attachments[0].parent_anchor_id,
        "right_side_handle_zone"
    );
    assert_eq!(plan.attachments[0].child_anchor_id, "back_mount_point");
}

#[test]
fn object_plan_materialization_one_box_plan_passes() {
    let plan = one_box_plan();
    let draft = materialize_object_plan(materialization_request(plan.clone()));

    assert_eq!(draft.status, MaterializationStatus::Passed);
    assert_eq!(draft.source_plan_id, plan.plan_id);
    assert_eq!(draft.primitive_instances.len(), 1);
    assert_eq!(
        draft.primitive_instances[0].primitive_kind,
        PrimitiveKind::BoxPrimitive
    );
    assert!(draft.composition_document.attachments.is_empty());
    assert!(draft.unresolved_nodes.is_empty());
    assert!(draft.unresolved_attachments.is_empty());
    assert!(draft.user_review_required);
    assert!(!draft.publish_allowed);
    assert_eq!(draft.review_tier, ObjectPlanReviewTier::Draft);
}

#[test]
fn object_plan_materialization_flat_panel_plan_passes() {
    let plan = one_flat_panel_plan();
    let draft = materialize_object_plan(materialization_request(plan));

    assert_eq!(draft.status, MaterializationStatus::Passed);
    assert_eq!(draft.primitive_instances.len(), 1);
    assert_eq!(
        draft.primitive_instances[0].primitive_kind,
        PrimitiveKind::FlatPanelPrimitive
    );
}

#[test]
fn object_plan_materialization_sphere_plan_passes() {
    let plan = one_sphere_plan();
    let draft = materialize_object_plan(materialization_request(plan));

    assert_eq!(draft.status, MaterializationStatus::Passed);
    assert_eq!(draft.primitive_instances.len(), 1);
    assert_eq!(
        draft.primitive_instances[0].primitive_kind,
        PrimitiveKind::SpherePrimitive
    );
}

#[test]
fn object_plan_materialization_panel_plus_sphere_attachment_passes() {
    let plan = panel_with_sphere_plan();
    let draft = materialize_object_plan(materialization_request(plan));

    assert_eq!(draft.status, MaterializationStatus::Passed);
    assert_eq!(draft.primitive_instances.len(), 2);
    assert_eq!(draft.composition_document.attachments.len(), 1);
    assert_eq!(
        draft.composition_document.attachments[0].attachment_id,
        "panel_knob_attachment"
    );
    assert!(draft.unresolved_attachments.is_empty());
}

#[test]
fn object_plan_materialization_invalid_plan_fails() {
    let mut plan = one_box_plan();
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));

    let draft = materialize_object_plan(materialization_request(plan));

    assert_eq!(draft.status, MaterializationStatus::Failed);
    assert!(draft.primitive_instances.is_empty());
    assert_eq!(draft.unresolved_nodes.len(), 1);
    assert!(
        draft
            .validation_report
            .issues
            .iter()
            .any(|issue| { issue.code == "invalid_current_property_value" })
    );
}

#[test]
fn object_plan_materialization_raw_mesh_payload_fails_decode() {
    let plan = one_box_plan();
    let mut value = serde_json::to_value(&plan).expect("plan serializes");
    value.as_object_mut().expect("plan is object").insert(
        "raw_mesh_payload".to_owned(),
        serde_json::json!({"vertices": [[0, 0, 0]]}),
    );

    assert!(serde_json::from_value::<ObjectPlan>(value).is_err());
}

#[test]
fn object_plan_materialization_public_publish_request_fails() {
    let mut plan = one_box_plan();
    plan.validation_policy.allow_public_catalog_publish = true;

    let draft = materialize_object_plan(materialization_request(plan));

    assert_eq!(draft.status, MaterializationStatus::Failed);
    assert!(!draft.publish_allowed);
    assert!(
        draft
            .validation_report
            .issues
            .iter()
            .any(|issue| { issue.code == "public_catalog_publish_rejected" })
    );
}

#[test]
fn object_plan_materialization_policy_cannot_enable_publish() {
    let plan = one_box_plan();
    let mut request = materialization_request(plan);
    request.materialization_policy.forbid_catalog_publish = false;

    let draft = materialize_object_plan(request);

    assert_eq!(draft.status, MaterializationStatus::Failed);
    assert!(!draft.publish_allowed);
    assert!(
        draft
            .validation_report
            .issues
            .iter()
            .any(|issue| { issue.code == "materialization_catalog_publish_forbidden" })
    );
}

#[test]
fn object_plan_materialization_summary_is_product_safe() {
    let plan = panel_with_sphere_plan();
    let draft = materialize_object_plan(materialization_request(plan.clone()));
    let summary = materialized_object_summary(&plan, &draft);

    assert_eq!(summary.source_plan_label, "Panel with knob");
    assert_eq!(summary.supported_primitive_count, 2);
    assert_eq!(summary.supported_attachment_count, 1);
    assert!(summary.user_review_required);
    assert_eq!(summary.next_action, MaterializedObjectNextAction::Review);
    assert_materialized_summary_product_safe(&summary);
}

#[test]
fn object_plan_materialization_serde_roundtrip_is_deterministic() {
    let request = materialization_request(panel_with_sphere_plan());
    let draft = materialize_object_plan(request);

    let first = serde_json::to_string(&draft).expect("draft serializes");
    let decoded = serde_json::from_str(&first).expect("draft decodes");
    let second = serde_json::to_string(&decoded).expect("draft serializes again");

    assert_eq!(first, second);
    assert_eq!(draft, decoded);
}

#[test]
fn object_plan_unknown_primitive_rejected() {
    let mut plan = one_sphere_plan();
    plan.nodes[0].primitive_kind = PrimitiveKind::CylinderPrimitive;

    let report = validate_object_plan(&plan);

    assert_issue(&report, "unsupported_object_plan_primitive_kind");
}

#[test]
fn object_plan_unknown_property_rejected() {
    let mut plan = one_sphere_plan();
    plan.nodes[0]
        .property_values
        .insert("raw_radius".to_owned(), PrimitivePropertyValue::Length(0.5));

    let report = validate_object_plan(&plan);

    assert_issue(&report, "unknown_current_property_value");
}

#[test]
fn object_plan_property_out_of_bounds_rejected() {
    let mut plan = one_sphere_plan();
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));

    let report = validate_object_plan(&plan);

    assert_issue(&report, "invalid_current_property_value");
}

#[test]
fn object_plan_incompatible_anchor_rejected() {
    let mut plan = panel_with_sphere_plan();
    plan.attachments[0].parent_anchor_id = "hinge_edge_zone".to_owned();

    let report = validate_object_plan(&plan);

    assert_issue(&report, "incompatible_attachment_anchor");
}

#[test]
fn object_plan_raw_mesh_payload_rejected() {
    let plan = one_sphere_plan();
    let mut value = serde_json::to_value(&plan).expect("plan serializes");
    value.as_object_mut().expect("plan is object").insert(
        "raw_mesh_payload".to_owned(),
        serde_json::json!({"vertices": []}),
    );

    let decoded = serde_json::from_value::<ObjectPlan>(value);

    assert!(
        decoded.is_err(),
        "ObjectPlan must reject raw mesh payload fields"
    );
}

#[test]
fn object_plan_arbitrary_matrix_payload_rejected() {
    let plan = panel_with_sphere_plan();
    let mut value = serde_json::to_value(&plan).expect("plan serializes");
    value["attachments"][0]
        .as_object_mut()
        .expect("attachment is object")
        .insert(
            "raw_matrix".to_owned(),
            serde_json::json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]),
        );

    let decoded = serde_json::from_value::<ObjectPlan>(value);

    assert!(
        decoded.is_err(),
        "ObjectPlan attachments must reject arbitrary matrix payload fields"
    );
}

#[test]
fn object_plan_public_catalog_publish_rejected() {
    let mut plan = one_sphere_plan();
    plan.validation_policy.allow_public_catalog_publish = true;

    let report = validate_object_plan(&plan);

    assert_issue(&report, "public_catalog_publish_rejected");
}

#[test]
fn object_plan_llm_draft_provenance_does_not_bypass_validation() {
    let mut plan = one_sphere_plan();
    plan.provenance.created_by = ObjectPlanCreatedBy::LlmDraft;
    plan.review_tier = ObjectPlanReviewTier::Personal;
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));

    let report = validate_object_plan(&plan);

    assert_issue(&report, "invalid_current_property_value");
    assert_issue(&report, "llm_draft_must_remain_draft");
}

#[test]
fn object_plan_review_tier_defaults_to_draft() {
    let mut value = serde_json::to_value(one_sphere_plan()).expect("plan serializes");
    value
        .as_object_mut()
        .expect("plan is object")
        .remove("review_tier");

    let decoded = serde_json::from_value::<ObjectPlan>(value).expect("plan decodes");

    assert_eq!(decoded.review_tier, ObjectPlanReviewTier::Draft);
    assert_valid(&validate_object_plan(&decoded));
}

#[test]
fn object_plan_user_summary_is_product_safe() {
    let plan = panel_with_sphere_plan();

    let summary = object_plan_user_summary(&plan);

    assert!(
        summary
            .primitives_used
            .iter()
            .any(|item| item.contains("Sphere Primitive"))
    );
    assert!(
        summary
            .adjustable_properties
            .iter()
            .any(|item| item.contains("Width"))
    );
    assert!(
        summary
            .attachments
            .iter()
            .any(|item| item.contains("attaches to Panel"))
    );
    assert!(summary.review_summary.contains("Draft"));
    assert_summary_product_safe(&summary);
}

#[test]
fn object_plan_absolute_paths_are_rejected() {
    let mut plan = one_sphere_plan();
    plan.provenance
        .source_seed_refs
        .push("/Users/arman/private-seed".to_owned());

    let report = validate_object_plan(&plan);

    assert_issue(&report, "invalid_source_seed_ref");
}

#[test]
fn object_plan_serde_roundtrip_is_deterministic() {
    let plan = panel_with_sphere_plan();

    let first = serde_json::to_string(&plan).expect("plan serializes");
    let decoded = serde_json::from_str::<ObjectPlan>(&first).expect("plan decodes");
    let second = serde_json::to_string(&decoded).expect("plan serializes again");

    assert_eq!(first, second);
    assert_eq!(plan, decoded);
}

#[test]
fn object_plan_prompt_pack_does_not_mention_runtime_integration() {
    for prompt_pack in [DRAFT_PROMPT_PACK, REPAIR_PROMPT_PACK] {
        let lower = prompt_pack.to_ascii_lowercase();
        assert!(!lower.contains("runtime integration"));
        assert!(!lower.contains("runtime llm"));
    }
}

#[test]
fn object_plan_prompt_pack_rejects_raw_mesh_output() {
    for prompt_pack in [DRAFT_PROMPT_PACK, REPAIR_PROMPT_PACK] {
        let lower = prompt_pack.to_ascii_lowercase();
        assert!(lower.contains("do not generate raw mesh"));
        assert!(lower.contains("mesh payload"));
    }
}

#[test]
fn object_plan_prompt_pack_blocks_unsupported_capabilities() {
    for prompt_pack in [DRAFT_PROMPT_PACK, REPAIR_PROMPT_PACK] {
        let lower = prompt_pack.to_ascii_lowercase();
        assert!(lower.contains("status: \"blocked\""));
        assert!(lower.contains("unsupported"));
        assert!(lower.contains("uv"));
        assert!(lower.contains("rigging"));
        assert!(lower.contains("animation"));
    }
}

#[test]
fn object_plan_repair_suggestion_requires_human_review() {
    let mut suggestion = ObjectPlanRepairSuggestion {
        finding_id: "finding_001".to_owned(),
        summary: "Width is outside the approved range.".to_owned(),
        suggested_change: "Set Width inside the approved range.".to_owned(),
        target_node_id: Some("knob".to_owned()),
        target_property_id: Some("width".to_owned()),
        target_attachment_id: None,
        risk: ObjectPlanRepairRisk::Low,
        requires_human_review: true,
    };

    assert!(validate_object_plan_repair_suggestion(&suggestion).is_valid());

    suggestion.requires_human_review = false;
    let report = validate_object_plan_repair_suggestion(&suggestion);

    assert_issue(&report, "repair_requires_human_review");
}

#[test]
fn object_plan_status_docs_do_not_overclaim_asset_generation() {
    for (path, doc) in OBJECT_PLAN_STATUS_DOCS {
        let lower = doc.to_ascii_lowercase();
        for forbidden in [
            "objectplan generates assets",
            "objectplan can generate assets",
            "objectplan v0 generates assets",
            "objectplan produces reusable prototype geometry",
            "objectplan produces visible generated assets",
            "objectplan renders every supported plan",
        ] {
            assert!(
                !lower.contains(forbidden),
                "{path} must not overclaim ObjectPlan generation/rendering: {forbidden}"
            );
        }
    }
}

#[test]
fn object_plan_status_docs_do_not_claim_auto_approval_or_runtime_llm() {
    for (path, doc) in OBJECT_PLAN_STATUS_DOCS {
        let lower = doc.to_ascii_lowercase();
        for forbidden in [
            "approved: true",
            "plans are approved automatically",
            "objectplans are approved automatically",
            "runtime llm support",
            "runtime llm is supported",
            "the app calls llms at runtime",
            "public catalog publishing is enabled",
        ] {
            assert!(
                !lower.contains(forbidden),
                "{path} must not claim approval, runtime LLM, or public publishing: {forbidden}"
            );
        }
    }
}

#[test]
fn object_plan_status_docs_keep_surface_rig_animation_blocked() {
    let combined = OBJECT_PLAN_STATUS_DOCS
        .iter()
        .map(|(_, doc)| *doc)
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    for required in [
        "material/surface",
        "uv/texturing",
        "rigging",
        "animation",
        "game-ready",
    ] {
        assert!(
            combined.contains(required),
            "ObjectPlan status docs must keep {required} blocked or caveated"
        );
    }
}

#[test]
fn object_plan_render_blocked_state_is_documented_as_valid_but_incomplete() {
    let truth_doc = include_str!("../../../docs/OBJECT_PLAN_V0_TRUTH_RENDER_BLOCKER_GATE.md")
        .to_ascii_lowercase();
    assert!(truth_doc.contains("render-blocked"));
    assert!(truth_doc.contains("valid for objectplan v0"));
    assert!(truth_doc.contains("incomplete"));
    assert!(truth_doc.contains("next milestone"));
}

fn one_sphere_plan() -> ObjectPlan {
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: "round_knob_plan".to_owned(),
        display_name: "Round knob-like form".to_owned(),
        intent_summary: "One rounded primitive with bounded dimensions and flattening.".to_owned(),
        nodes: vec![sphere_node()],
        attachments: Vec::new(),
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: ObjectPlanProvenance {
            created_by: ObjectPlanCreatedBy::Human,
            source_prompt_hash: Some("abc123".to_owned()),
            source_seed_refs: vec!["seed_round_form".to_owned()],
            created_at: "2026-06-30T00:00:00Z".to_owned(),
        },
    }
}

fn one_box_plan() -> ObjectPlan {
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: "box_plan".to_owned(),
        display_name: "Box plan".to_owned(),
        intent_summary: "One editable box primitive with bounded dimensions.".to_owned(),
        nodes: vec![box_node()],
        attachments: Vec::new(),
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: ObjectPlanProvenance {
            created_by: ObjectPlanCreatedBy::Human,
            source_prompt_hash: Some("box123".to_owned()),
            source_seed_refs: vec!["seed_box".to_owned()],
            created_at: "2026-07-01T00:00:00Z".to_owned(),
        },
    }
}

fn one_flat_panel_plan() -> ObjectPlan {
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: "flat_panel_plan".to_owned(),
        display_name: "Flat panel plan".to_owned(),
        intent_summary: "One editable flat panel primitive with bounded dimensions.".to_owned(),
        nodes: vec![panel_node()],
        attachments: Vec::new(),
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: ObjectPlanProvenance {
            created_by: ObjectPlanCreatedBy::Human,
            source_prompt_hash: Some("panel123".to_owned()),
            source_seed_refs: vec!["seed_panel".to_owned()],
            created_at: "2026-07-01T00:00:00Z".to_owned(),
        },
    }
}

fn panel_with_sphere_plan() -> ObjectPlan {
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: "panel_with_knob_plan".to_owned(),
        display_name: "Panel with knob".to_owned(),
        intent_summary: "A flat panel with one rounded form attached by a safe anchor.".to_owned(),
        nodes: vec![panel_node(), sphere_node()],
        attachments: vec![ObjectPlanAttachment {
            attachment_id: "panel_knob_attachment".to_owned(),
            parent_node_id: "panel".to_owned(),
            parent_anchor_id: "right_side_handle_zone".to_owned(),
            child_node_id: "knob".to_owned(),
            child_anchor_id: "back_mount_point".to_owned(),
            offset: PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
                x: 0.25,
                y: 0.0,
                minimum_x: -0.6,
                maximum_x: 0.6,
                minimum_y: -0.5,
                maximum_y: 0.5,
            },
            orientation_policy: PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal,
            scale_policy: PrimitiveAttachmentScalePolicy::KeepChildScale,
        }],
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: ObjectPlanProvenance {
            created_by: ObjectPlanCreatedBy::InternalTool,
            source_prompt_hash: Some("def456".to_owned()),
            source_seed_refs: vec!["seed_panel_knob".to_owned()],
            created_at: "2026-06-30T00:00:00Z".to_owned(),
        },
    }
}

fn panel_node() -> ObjectPlanNode {
    let schema = flat_panel_primitive_property_schema();
    ObjectPlanNode {
        node_id: "panel".to_owned(),
        primitive_kind: PrimitiveKind::FlatPanelPrimitive,
        display_name: "Panel".to_owned(),
        property_values: primitive_default_property_values(&schema),
        role_hint: "Base panel".to_owned(),
        locked: false,
    }
}

fn box_node() -> ObjectPlanNode {
    let schema = box_primitive_property_schema();
    ObjectPlanNode {
        node_id: "box".to_owned(),
        primitive_kind: PrimitiveKind::BoxPrimitive,
        display_name: "Box".to_owned(),
        property_values: primitive_default_property_values(&schema),
        role_hint: "Simple box body".to_owned(),
        locked: false,
    }
}

fn sphere_node() -> ObjectPlanNode {
    let schema = sphere_primitive_property_schema();
    ObjectPlanNode {
        node_id: "knob".to_owned(),
        primitive_kind: PrimitiveKind::SpherePrimitive,
        display_name: "Knob-like form".to_owned(),
        property_values: primitive_default_property_values(&schema),
        role_hint: "Rounded attached form".to_owned(),
        locked: false,
    }
}

fn materialization_request(plan: ObjectPlan) -> ObjectPlanMaterializationRequest {
    ObjectPlanMaterializationRequest {
        plan,
        materialization_policy: MaterializationPolicy::default(),
        target_preview_profile: "clay-review".to_owned(),
        output_mode: ObjectPlanMaterializationOutputMode::DraftReview,
    }
}

fn assert_valid(report: &ObjectPlanValidationReport) {
    assert!(
        report.is_valid(),
        "expected valid ObjectPlan, got {:?}",
        report.issues
    );
}

fn assert_issue(report: &ObjectPlanValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "expected issue {expected_code}, got {:?}",
        report.issues
    );
}

fn assert_summary_product_safe(summary: &ObjectPlanUserSummary) {
    let all_text = std::iter::once(summary.display_name.as_str())
        .chain(std::iter::once(summary.intent_summary.as_str()))
        .chain(summary.primitives_used.iter().map(String::as_str))
        .chain(summary.adjustable_properties.iter().map(String::as_str))
        .chain(summary.attachments.iter().map(String::as_str))
        .chain(std::iter::once(summary.review_summary.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    for forbidden in [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "raw transform",
        "mesh payload",
    ] {
        assert!(
            !all_text.to_ascii_lowercase().contains(forbidden),
            "summary must not expose {forbidden}: {all_text}"
        );
    }
}

fn assert_materialized_summary_product_safe(summary: &shape_foundry::MaterializedObjectSummary) {
    let text = serde_json::to_string(summary).expect("summary serializes");
    let lower = text.to_ascii_lowercase();
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
    ] {
        assert!(
            !lower.contains(forbidden),
            "materialized summary exposed internal term {forbidden}: {text}"
        );
    }
}
