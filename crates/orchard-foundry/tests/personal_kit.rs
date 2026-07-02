use orchard_foundry::{
    PersonalKitSaveCommand, PersonalKitSaveViewModel, PersonalKitSourceKind,
    PersonalKitValidationReport, PersonalKitVisibility, personal_kit_save_view_model,
    validate_personal_kit_save_command, validate_personal_kit_save_view_model,
};

#[test]
fn personal_kit_current_primitive_view_model_validates() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::CurrentPrimitive,
        "Compact Box",
        true,
        true,
    );

    let report = validate_personal_kit_save_view_model(&view_model);

    assert_valid(&report);
    assert!(view_model.save_enabled);
    assert_eq!(
        view_model.resulting_visibility,
        PersonalKitVisibility::PersonalOnly
    );
    assert!(view_model.summary.contains("Only visible to you"));
    assert!(view_model.summary.contains("Needs review before sharing"));
}

#[test]
fn personal_kit_object_plan_draft_view_model_validates() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::ObjectPlanDraft,
        "Panel with knob draft",
        true,
        true,
    );

    let report = validate_personal_kit_save_view_model(&view_model);

    assert_valid(&report);
    assert!(view_model.save_enabled);
    assert_eq!(
        view_model.resulting_visibility,
        PersonalKitVisibility::PersonalOnly
    );
}

#[test]
fn personal_kit_public_catalog_visibility_rejected() {
    let command = PersonalKitSaveCommand {
        source_ref: "current-primitive".to_owned(),
        kit_name: "Compact Box".to_owned(),
        visibility: PersonalKitVisibility::PublicCatalog,
        include_preview: true,
        include_object_plan: false,
        include_export_reference: false,
    };

    let report = validate_personal_kit_save_command(&command);

    assert_issue(&report, "personal_kit_public_visibility_rejected");
}

#[test]
fn personal_kit_missing_render_evidence_warning_required() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::CompositionDraft,
        "Panel with knob",
        false,
        true,
    );

    let report = validate_personal_kit_save_view_model(&view_model);

    assert_valid(&report);
    assert!(
        view_model
            .warnings
            .iter()
            .any(|warning| warning == "No review image yet.")
    );

    let mut invalid = view_model;
    invalid.warnings.clear();
    let report = validate_personal_kit_save_view_model(&invalid);
    assert_issue(&report, "personal_kit_missing_render_evidence_warning");
}

#[test]
fn personal_kit_missing_export_proof_warning_required() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::ObjectPlanDraft,
        "Box draft",
        true,
        false,
    );

    let report = validate_personal_kit_save_view_model(&view_model);

    assert_valid(&report);
    assert!(
        view_model
            .warnings
            .iter()
            .any(|warning| warning == "No engine export proof yet.")
    );
}

#[test]
fn personal_kit_user_copy_hides_technical_terms() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::CurrentPrimitive,
        "Compact Box",
        true,
        true,
    );

    assert_user_copy_safe(&view_model);

    let mut invalid = view_model;
    invalid.summary = "Saved provider slot is ready.".to_owned();
    let report = validate_personal_kit_save_view_model(&invalid);
    assert_issue(&report, "personal_kit_user_copy_forbidden_term");
}

#[test]
fn personal_kit_name_required() {
    let command = PersonalKitSaveCommand {
        source_ref: "current-primitive".to_owned(),
        kit_name: " ".to_owned(),
        visibility: PersonalKitVisibility::PersonalOnly,
        include_preview: true,
        include_object_plan: false,
        include_export_reference: false,
    };

    let report = validate_personal_kit_save_command(&command);

    assert_issue(&report, "personal_kit_name_required");
}

#[test]
fn personal_kit_serde_roundtrip_is_deterministic() {
    let view_model = personal_kit_save_view_model(
        PersonalKitSourceKind::ObjectPlanDraft,
        "Panel with knob draft",
        true,
        false,
    );

    let first = serde_json::to_string(&view_model).expect("view model serializes");
    let decoded =
        serde_json::from_str::<PersonalKitSaveViewModel>(&first).expect("view model decodes");
    let second = serde_json::to_string(&decoded).expect("view model serializes again");

    assert_eq!(first, second);
    assert_eq!(view_model, decoded);
}

fn assert_valid(report: &PersonalKitValidationReport) {
    assert!(report.is_valid(), "expected valid report, got {report:?}");
}

fn assert_issue(report: &PersonalKitValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing issue {expected_code}; got {:?}",
        report.issues
    );
}

fn assert_user_copy_safe(view_model: &PersonalKitSaveViewModel) {
    let text = serde_json::to_string(view_model).expect("view model serializes");
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
        "publish",
        "catalog",
        "game-ready",
        "marketplace",
    ] {
        assert!(
            !lower.contains(forbidden),
            "view model user copy should not expose {forbidden}: {text}"
        );
    }
}
