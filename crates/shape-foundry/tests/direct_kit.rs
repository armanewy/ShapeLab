use shape_foundry::{
    DirectKitCreatedFrom, DirectKitDraft, DirectKitEvidenceKind, DirectKitEvidenceRef,
    DirectKitEvidenceStatus, DirectKitPresetRef, DirectKitSourceKind, DirectKitUserSummary,
    DirectKitValidationReport, DirectKitVisibility, KitCapabilityAvailability, KitCapabilityCard,
    KitCapabilitySourceKind, KitCapabilityValidationReport, ObjectPlanReviewTier,
    PersonalKitStoreValidationReport, PresetSource, PrimitiveKind,
    direct_kit_property_exposures_for_primitive, direct_kit_user_summary,
    kit_capability_cards_for_panel_with_knob, kit_capability_cards_for_primitive,
    list_personal_kits, load_direct_kit, personal_kit_store_root, save_direct_kit,
    validate_direct_kit_draft, validate_kit_capability_card, validate_kit_capability_cards,
    validate_personal_kit_store,
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

#[test]
fn direct_kit_box_capability_cards_include_dimensions_and_edge_softness() {
    let cards = kit_capability_cards_for_primitive(PrimitiveKind::BoxPrimitive, false);

    assert_valid_cards(&cards);
    assert_card(&cards, "Width");
    assert_card(&cards, "Depth");
    assert_card(&cards, "Height");
    assert_card(&cards, "Edge Softness");
}

#[test]
fn direct_kit_flat_panel_capability_cards_include_dimensions_thickness_and_edge_softness() {
    let cards = kit_capability_cards_for_primitive(PrimitiveKind::FlatPanelPrimitive, false);

    assert_valid_cards(&cards);
    assert_card(&cards, "Width");
    assert_card(&cards, "Height");
    assert_card(&cards, "Thickness");
    assert_card(&cards, "Edge Softness");
}

#[test]
fn direct_kit_sphere_capability_cards_include_dimensions_and_flattening() {
    let cards = kit_capability_cards_for_primitive(PrimitiveKind::SpherePrimitive, false);

    assert_valid_cards(&cards);
    assert_card(&cards, "Width");
    assert_card(&cards, "Height");
    assert_card(&cards, "Depth");
    assert_card(&cards, "Front Flatten");
    assert_card(&cards, "Back Flatten");
}

#[test]
fn direct_kit_panel_with_knob_capability_cards_include_panel_and_knob_controls() {
    let cards = kit_capability_cards_for_panel_with_knob(false);

    assert_valid_cards(&cards);
    assert_card(&cards, "Panel Width");
    assert_card(&cards, "Knob Width");
    assert_card(&cards, "Knob Position");
    assert!(
        cards
            .iter()
            .any(|card| card.source_kind == KitCapabilitySourceKind::CompositionOffset)
    );
}

#[test]
fn direct_kit_material_look_card_is_later_when_surface_evidence_missing() {
    let cards = kit_capability_cards_for_primitive(PrimitiveKind::BoxPrimitive, false);
    let card = cards
        .iter()
        .find(|card| card.display_name == "Material Look")
        .expect("material look card exists");

    assert_eq!(card.availability, KitCapabilityAvailability::Later);
    assert!(card.reason.blocked_reason.is_some());
    assert!(!card.reason.requirements_satisfied);
}

#[test]
fn direct_kit_capability_cards_do_not_use_generated_variation_or_technical_copy() {
    let mut cards = kit_capability_cards_for_panel_with_knob(false);
    cards.extend(kit_capability_cards_for_primitive(
        PrimitiveKind::SpherePrimitive,
        false,
    ));

    assert_valid_cards(&cards);
    for card in &cards {
        assert_card_copy_safe(card);
    }

    let mut invalid = cards[0].clone();
    invalid.description = "Generated variation candidate from provider slot.".to_owned();
    let report = validate_kit_capability_card(&invalid);
    assert_card_issue(&report, "kit_capability_user_copy_forbidden_term");
}

#[test]
fn direct_kit_capability_unknown_property_mapping_rejected() {
    let mut card = kit_capability_cards_for_primitive(PrimitiveKind::BoxPrimitive, false)
        .into_iter()
        .find(|card| card.display_name == "Width")
        .expect("width card exists");
    card.maps_to = "unknown_width".to_owned();

    let report = validate_kit_capability_card(&card);

    assert_card_issue(&report, "kit_capability_unknown_mapping");
}

#[test]
fn direct_kit_capability_serde_roundtrip_is_deterministic() {
    let cards = kit_capability_cards_for_primitive(PrimitiveKind::FlatPanelPrimitive, false);

    let first = serde_json::to_string(&cards).expect("cards serialize");
    let decoded = serde_json::from_str::<Vec<KitCapabilityCard>>(&first).expect("cards decode");
    let second = serde_json::to_string(&decoded).expect("cards serialize again");

    assert_eq!(first, second);
    assert_eq!(cards, decoded);
}

#[test]
fn direct_kit_personal_storage_saves_valid_draft() {
    let store = test_store_dir("saves_valid_draft");
    let kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");

    let stored = save_direct_kit(&store, &kit).expect("save direct kit");

    assert_eq!(stored.visibility, DirectKitVisibility::Draft);
    assert!(!stored.novice_visible);
    assert!(!stored.public_catalog_visible);
    assert!(
        personal_kit_store_root(&store)
            .join("manifest.json")
            .exists()
    );
    assert!(
        personal_kit_store_root(&store)
            .join("kits")
            .join("box_primitive_kit")
            .join("kit.json")
            .exists()
    );
    assert_store_valid(&validate_personal_kit_store(&store));
}

#[test]
fn direct_kit_personal_storage_saves_valid_personal_only() {
    let store = test_store_dir("saves_valid_personal_only");
    let mut kit = primitive_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive");
    kit.visibility = DirectKitVisibility::PersonalOnly;
    kit.review_tier = ObjectPlanReviewTier::Personal;

    let stored = save_direct_kit(&store, &kit).expect("save direct kit");

    assert_eq!(stored.visibility, DirectKitVisibility::PersonalOnly);
    assert!(!stored.public_catalog_visible);
    assert_store_valid(&validate_personal_kit_store(&store));
}

#[test]
fn direct_kit_personal_storage_list_and_load_roundtrip() {
    let store = test_store_dir("list_and_load_roundtrip");
    let kit = primitive_kit(PrimitiveKind::FlatPanelPrimitive, "flat_panel_primitive");
    save_direct_kit(&store, &kit).expect("save direct kit");

    let manifest = list_personal_kits(&store).expect("list personal kits");
    let loaded = load_direct_kit(&store, "flat_panel_primitive_kit").expect("load direct kit");

    assert_eq!(manifest.kits.len(), 1);
    assert_eq!(
        manifest.kits[0].kit_path,
        "kits/flat_panel_primitive_kit/kit.json"
    );
    assert_eq!(loaded.direct_kit, kit);
}

#[test]
fn direct_kit_personal_storage_rejects_invalid_public_visibility() {
    let store = test_store_dir("rejects_invalid_public_visibility");
    let mut kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.visibility = DirectKitVisibility::PublicCatalog;

    let error = save_direct_kit(&store, &kit).expect_err("public visibility rejected");
    let text = error.to_string();

    assert!(text.contains("validation failed"));
}

#[test]
fn direct_kit_personal_storage_rejects_absolute_paths() {
    let store = test_store_dir("rejects_absolute_paths");
    let mut kit = primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive");
    kit.evidence_refs[0].path = "/tmp/evidence.json".to_owned();

    let error = save_direct_kit(&store, &kit).expect_err("absolute path rejected");
    let text = error.to_string();

    assert!(text.contains("validation failed"));
}

#[test]
fn direct_kit_personal_storage_manifest_is_deterministic() {
    let first = test_store_dir("manifest_deterministic_first");
    let second = test_store_dir("manifest_deterministic_second");
    for store in [&first, &second] {
        save_direct_kit(
            store,
            &primitive_kit(PrimitiveKind::BoxPrimitive, "box_primitive"),
        )
        .expect("save box kit");
        save_direct_kit(
            store,
            &primitive_kit(PrimitiveKind::SpherePrimitive, "sphere_primitive"),
        )
        .expect("save sphere kit");
    }

    let first_manifest =
        std::fs::read(personal_kit_store_root(&first).join("manifest.json")).expect("read first");
    let second_manifest =
        std::fs::read(personal_kit_store_root(&second).join("manifest.json")).expect("read second");

    assert_eq!(first_manifest, second_manifest);
    assert!(
        !String::from_utf8_lossy(&first_manifest)
            .to_ascii_lowercase()
            .contains("public_catalog_visible\": true")
    );
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

fn assert_valid_cards(cards: &[KitCapabilityCard]) {
    let report = validate_kit_capability_cards(cards);
    assert!(
        report.is_valid(),
        "expected valid capability cards, got {report:?}"
    );
}

fn assert_card(cards: &[KitCapabilityCard], display_name: &str) {
    assert!(
        cards.iter().any(|card| card.display_name == display_name),
        "missing capability card {display_name}; got {:?}",
        cards
            .iter()
            .map(|card| card.display_name.as_str())
            .collect::<Vec<_>>()
    );
}

fn assert_card_issue(report: &KitCapabilityValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing capability issue {expected_code}; got {:?}",
        report.issues
    );
}

fn assert_card_copy_safe(card: &KitCapabilityCard) {
    let text = format!(
        "{}\n{}\n{}\n{}\n{}",
        card.display_name,
        card.description,
        card.reason.plain_language_reason,
        card.reason.suggested_next_action,
        card.reason.blocked_reason.as_deref().unwrap_or_default()
    );
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
        "generated variation",
        "candidate",
        "runtime llm",
        "public catalog",
        "publish",
        "uv",
        "rigging",
        "animation",
        "game-ready",
    ] {
        assert!(
            !lower.contains(forbidden),
            "card should not expose {forbidden}: {text}"
        );
    }
}

fn assert_store_valid(report: &PersonalKitStoreValidationReport) {
    assert!(report.is_valid(), "expected valid store, got {report:?}");
}

fn test_store_dir(name: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("target")
        .join("shape-foundry-direct-kit-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test store dir");
    path
}
