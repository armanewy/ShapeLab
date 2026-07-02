use orchard_foundry::{
    AssetRequest, PrimitiveKind, PrototypePackBrief, PrototypePackBriefSummary,
    PrototypePackCapability, PrototypePackCompositionKind, PrototypePackOutputPolicy,
    PrototypePackReviewPolicy, PrototypePackValidationReport, prototype_pack_brief_summary,
    prototype_pack_supported_scope_v0, validate_prototype_pack_brief,
};

#[test]
fn prototype_pack_valid_small_brief_passes() {
    let brief = valid_small_brief();

    let report = validate_prototype_pack_brief(&brief);

    assert_valid(&report);
    assert!(brief.output_policy.draft_only);
    assert!(brief.output_policy.human_review_required);
    assert!(!brief.output_policy.approved);
    assert!(!brief.output_policy.publish_allowed);
}

#[test]
fn prototype_pack_unsupported_capability_blocked() {
    let mut brief = valid_small_brief();
    brief.asset_requests[0]
        .must_have_capabilities
        .push(PrototypePackCapability::Rigging);

    let report = validate_prototype_pack_brief(&brief);

    assert_issue(&report, "prototype_pack_unsupported_capability");
}

#[test]
fn prototype_pack_too_large_desired_count_rejected() {
    let mut brief = valid_small_brief();
    brief.asset_requests[0].desired_count = 25;

    let report = validate_prototype_pack_brief(&brief);

    assert_issue(&report, "prototype_pack_desired_count_out_of_bounds");
}

#[test]
fn prototype_pack_public_publishing_rejected() {
    let mut brief = valid_small_brief();
    brief.output_policy.publish_allowed = true;

    let report = validate_prototype_pack_brief(&brief);

    assert_issue(&report, "prototype_pack_public_publishing_rejected");
}

#[test]
fn prototype_pack_automatic_approval_rejected() {
    let mut brief = valid_small_brief();
    brief.output_policy.approved = true;
    brief.review_policy.automatic_approval_allowed = true;

    let report = validate_prototype_pack_brief(&brief);

    assert_issue(&report, "prototype_pack_auto_approval_rejected");
    assert_issue(&report, "prototype_pack_automatic_approval_rejected");
}

#[test]
fn prototype_pack_unsupported_primitive_and_composition_rejected() {
    let mut brief = valid_small_brief();
    brief.asset_requests[0]
        .allowed_primitives
        .push(PrimitiveKind::CylinderPrimitive);
    brief.asset_requests[0]
        .allowed_compositions
        .push(PrototypePackCompositionKind::Unsupported);

    let report = validate_prototype_pack_brief(&brief);

    assert_issue(&report, "prototype_pack_unsupported_primitive");
    assert_issue(&report, "prototype_pack_unsupported_composition");
}

#[test]
fn prototype_pack_summary_hides_technical_terms() {
    let mut brief = valid_small_brief();
    brief.asset_requests[0]
        .must_have_capabilities
        .push(PrototypePackCapability::MaterialSurface);
    brief.asset_requests[1]
        .must_have_capabilities
        .push(PrototypePackCapability::Animation);

    let summary = prototype_pack_brief_summary(&brief);

    assert_summary_safe(&summary);
    assert_eq!(summary.requested_assets.len(), 2);
    assert_eq!(summary.supported_now.len(), 0);
    assert_eq!(summary.needs_future_capabilities.len(), 2);

    let mut invalid = brief;
    invalid.display_name = "Game-ready marketplace pack".to_owned();
    let report = validate_prototype_pack_brief(&invalid);
    assert_issue(&report, "prototype_pack_user_copy_forbidden_term");
}

#[test]
fn prototype_pack_serde_roundtrip_is_deterministic() {
    let brief = valid_small_brief();

    let first = serde_json::to_string(&brief).expect("brief serializes");
    let decoded = serde_json::from_str::<PrototypePackBrief>(&first).expect("brief decodes");
    let second = serde_json::to_string(&decoded).expect("brief serializes again");

    assert_eq!(first, second);
    assert_eq!(brief, decoded);
}

fn valid_small_brief() -> PrototypePackBrief {
    PrototypePackBrief {
        brief_id: "starter_props".to_owned(),
        display_name: "Starter Props".to_owned(),
        purpose: "Quick draft props for early review.".to_owned(),
        asset_requests: vec![
            AssetRequest {
                request_id: "box_props".to_owned(),
                display_name: "Box props".to_owned(),
                intended_use: "Simple blockout props.".to_owned(),
                allowed_primitives: vec![PrimitiveKind::BoxPrimitive],
                allowed_compositions: Vec::new(),
                desired_count: 3,
                style_hint: Some("Clean shapes with clear silhouettes.".to_owned()),
                must_have_capabilities: vec![
                    PrototypePackCapability::ObjectPlanDraft,
                    PrototypePackCapability::ReviewImage,
                    PrototypePackCapability::GeometryOnlyExport,
                ],
                blocked_capabilities: vec![
                    PrototypePackCapability::PublicCatalogPublishing,
                    PrototypePackCapability::GameReady,
                ],
            },
            AssetRequest {
                request_id: "panel_controls".to_owned(),
                display_name: "Panel controls".to_owned(),
                intended_use: "Simple interactive-looking panels for review.".to_owned(),
                allowed_primitives: vec![
                    PrimitiveKind::FlatPanelPrimitive,
                    PrimitiveKind::SpherePrimitive,
                ],
                allowed_compositions: vec![PrototypePackCompositionKind::PanelWithKnob],
                desired_count: 2,
                style_hint: None,
                must_have_capabilities: vec![PrototypePackCapability::ObjectPlanDraft],
                blocked_capabilities: Vec::new(),
            },
        ],
        supported_primitive_scope: prototype_pack_supported_scope_v0(),
        output_policy: PrototypePackOutputPolicy::default(),
        review_policy: PrototypePackReviewPolicy::default(),
    }
}

fn assert_valid(report: &PrototypePackValidationReport) {
    assert!(report.is_valid(), "expected valid report, got {report:?}");
}

fn assert_issue(report: &PrototypePackValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing issue {expected_code}; got {:?}",
        report.issues
    );
}

fn assert_summary_safe(summary: &PrototypePackBriefSummary) {
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
        "objectplan",
        "runtime llm",
        "public catalog",
        "publish",
        "game-ready",
        "marketplace",
        "rig",
        "animation",
        "uv",
        "material",
    ] {
        assert!(
            !lower.contains(forbidden),
            "summary should not expose {forbidden}: {text}"
        );
    }
}
