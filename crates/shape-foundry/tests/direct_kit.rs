use shape_foundry::{
    DirectKitCreatedFrom, DirectKitDraft, DirectKitEvidenceKind, DirectKitEvidenceRef,
    DirectKitEvidenceStatus, DirectKitPresetRef, DirectKitSourceKind, DirectKitUserSummary,
    DirectKitValidationReport, DirectKitVisibility, ObjectPlanReviewTier, PresetSource,
    PrimitiveKind, direct_kit_property_exposures_for_primitive, direct_kit_user_summary,
    validate_direct_kit_draft,
};

#[test]
fn direct_kit_valid_box_primitive_passes() {
    let kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");

    let report = validate_direct_kit_draft(&kit);

    assert_valid(&report);
    assert!(report.warnings.is_empty());
}

#[test]
fn direct_kit_valid_flat_panel_primitive_passes() {
    let kit = primitive_kit(PrimitiveKind::FlatPanelPrimitive, "flat_panel_primitive");

    let report = validate_direct_kit_draft(&kit);

    assert_valid(&report);
}

#[test]
fn direct_kit_valid_sphere_primitive_passes() {
    let kit = primitive_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive");

    let report = validate_direct_kit_draft(&kit);

    assert_valid(&report);
}

#[test]
fn direct_kit_valid_panel_with_knob_composition_passes() {
    let mut kit = primitive_kit(PrimitiveKind::FlatPanelPrimitive, "panel_with_knob");
    kit.source_kind = DirectKitSourceKind::Composition;
    kit.identity_summary = "This stays a panel with an attached knob-like form.".to_owned();
    kit.included_presets = vec![
        DirectKitPresetRef {
            preset_id: "wide_panel".to_owned(),
            display_name: "Wide Panel".to_owned(),
            source: PresetSource::BuiltIn,
        },
        DirectKitPresetRef {
            preset_id: "knob_like_form".to_owned(),
            display_name: "Knob-Like Form".to_owned(),
            source: PresetSource::BuiltIn,
        },
    ];

    let report = validate_direct_kit_draft(&kit);

    assert_valid(&report);
}

#[test]
fn direct_kit_unknown_property_rejected() {
    let mut kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.changeable_properties[0].property_id = "unknown_width".to_owned();

    let report = validate_direct_kit_draft(&kit);

    assert_error(&report, "direct_kit_unknown_property");
}

#[test]
fn direct_kit_preset_mismatch_rejected() {
    let mut kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.included_presets = vec![DirectKitPresetRef {
        preset_id: "wide_panel".to_owned(),
        display_name: "Wide Panel".to_owned(),
        source: PresetSource::BuiltIn,
    }];

    let report = validate_direct_kit_draft(&kit);

    assert_error(&report, "direct_kit_preset_mismatch");
}

#[test]
fn direct_kit_public_catalog_visibility_rejected() {
    let mut kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.visibility = DirectKitVisibility::PublicCatalog;

    let report = validate_direct_kit_draft(&kit);

    assert_error(&report, "direct_kit_public_catalog_visibility_rejected");
}

#[test]
fn direct_kit_reviewed_visibility_rejected_in_v0() {
    let mut kit = primitive_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive");
    kit.visibility = DirectKitVisibility::Reviewed;
    kit.review_tier = ObjectPlanReviewTier::Reviewed;

    let report = validate_direct_kit_draft(&kit);

    assert_error(&report, "direct_kit_reviewed_visibility_rejected_v0");
    assert_error(&report, "direct_kit_review_tier_rejected_v0");
}

#[test]
fn direct_kit_missing_evidence_warns_without_failing() {
    let mut kit = primitive_kit(PrimitiveKind::FlatPanelPrimitive, "flat_panel_primitive");
    kit.evidence_refs.clear();

    let report = validate_direct_kit_draft(&kit);

    assert_valid(&report);
    assert_warning(&report, "direct_kit_missing_evidence");
}

#[test]
fn direct_kit_user_summary_hides_technical_terms() {
    let kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    let summary = direct_kit_user_summary(&kit);

    assert_summary_safe(&summary);

    let mut invalid = kit;
    invalid.identity_summary = "This exposes provider slot topology.".to_owned();
    let report = validate_direct_kit_draft(&invalid);
    assert_error(&report, "direct_kit_user_copy_forbidden_term");
}

#[test]
fn direct_kit_serde_roundtrip_is_deterministic() {
    let kit = primitive_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive");

    let first = serde_json::to_string(&kit).expect("kit serializes");
    let decoded = serde_json::from_str::<DirectKitDraft>(&first).expect("kit decodes");
    let second = serde_json::to_string(&decoded).expect("kit serializes again");

    assert_eq!(first, second);
    assert_eq!(kit, decoded);
}

fn primitive_kit(primitive_kind: PrimitiveKind, source_ref: &str) -> DirectKitDraft {
    let mut exposures = direct_kit_property_exposures_for_primitive(primitive_kind);
    let locked_properties = exposures.split_off(1);
    let included_presets = match primitive_kind {
        PrimitiveKind::BoxPrimitive => vec![DirectKitPresetRef {
            preset_id: "compact_box".to_owned(),
            display_name: "Compact Box".to_owned(),
            source: PresetSource::BuiltIn,
        }],
        PrimitiveKind::FlatPanelPrimitive => vec![DirectKitPresetRef {
            preset_id: "wide_panel".to_owned(),
            display_name: "Wide Panel".to_owned(),
            source: PresetSource::BuiltIn,
        }],
        PrimitiveKind::SpherePrimitive => vec![DirectKitPresetRef {
            preset_id: "round_sphere".to_owned(),
            display_name: "Round Sphere".to_owned(),
            source: PresetSource::BuiltIn,
        }],
        PrimitiveKind::CylinderPrimitive => Vec::new(),
    };

    DirectKitDraft {
        kit_id: format!("{source_ref}_kit"),
        display_name: "Reusable Shape Kit".to_owned(),
        source_kind: DirectKitSourceKind::Primitive,
        source_ref: source_ref.to_owned(),
        identity_summary: identity_summary(primitive_kind).to_owned(),
        changeable_properties: exposures,
        locked_properties,
        included_presets,
        evidence_refs: vec![DirectKitEvidenceRef {
            evidence_kind: DirectKitEvidenceKind::PropertyEndpointSheet,
            path: "evidence/property-endpoints.json".to_owned(),
            status: DirectKitEvidenceStatus::Passed,
            human_review_required: true,
        }],
        review_tier: ObjectPlanReviewTier::Draft,
        visibility: DirectKitVisibility::Draft,
        created_from: DirectKitCreatedFrom::CurrentPrimitive,
    }
}

fn identity_summary(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "This stays a box-like primitive.",
        PrimitiveKind::FlatPanelPrimitive => "This stays a flat panel.",
        PrimitiveKind::SpherePrimitive => "This stays a round primitive.",
        PrimitiveKind::CylinderPrimitive => "This primitive is not active.",
    }
}

fn assert_valid(report: &DirectKitValidationReport) {
    assert!(report.is_valid(), "expected valid report, got {report:?}");
}

fn assert_error(report: &DirectKitValidationReport, expected_code: &str) {
    assert!(
        report
            .errors
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing error {expected_code}; got {:?}",
        report.errors
    );
}

fn assert_warning(report: &DirectKitValidationReport, expected_code: &str) {
    assert!(
        report
            .warnings
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing warning {expected_code}; got {:?}",
        report.warnings
    );
}

fn assert_summary_safe(summary: &DirectKitUserSummary) {
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
        "mesh payload",
    ] {
        assert!(
            !lower.contains(forbidden),
            "summary should not expose {forbidden}: {text}"
        );
    }
    assert!(!lower.contains("generated variation"));
    assert!(!lower.contains("public catalog"));
}
