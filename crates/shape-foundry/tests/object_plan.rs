use shape_foundry::{
    OBJECT_PLAN_SCHEMA_VERSION, ObjectPlan, ObjectPlanAttachment, ObjectPlanCreatedBy,
    ObjectPlanNode, ObjectPlanProvenance, ObjectPlanReviewTier, ObjectPlanUserSummary,
    ObjectPlanValidationPolicy, ObjectPlanValidationReport, PrimitiveAttachmentOffsetPolicy,
    PrimitiveAttachmentOrientationPolicy, PrimitiveAttachmentScalePolicy, PrimitiveKind,
    PrimitivePropertyValue, flat_panel_primitive_property_schema, object_plan_user_summary,
    primitive_default_property_values, sphere_primitive_property_schema, validate_object_plan,
};

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
