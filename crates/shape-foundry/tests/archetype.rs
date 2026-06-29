#![forbid(unsafe_code)]

use shape_foundry::{CARGO_CASE_ARCHETYPE_ID, cargo_case_archetype, validate_foundry_archetype};

#[test]
fn cargo_case_archetype_validates() {
    let archetype = cargo_case_archetype();
    let report = validate_foundry_archetype(&archetype);

    assert!(
        report.is_valid(),
        "Cargo Case archetype should validate: {:?}",
        report.issues
    );
    assert_eq!(archetype.archetype_id, CARGO_CASE_ARCHETYPE_ID);
    assert_eq!(archetype.role_templates.len(), 6);
    assert_eq!(archetype.optional_role_templates.len(), 9);
    assert_eq!(archetype.control_axis_templates.len(), 7);
    assert_eq!(archetype.candidate_strategy_templates.len(), 6);
    assert!(!archetype.publish_allowed);
    assert!(!archetype.novice_visible);
    assert!(archetype.geometry_payload.is_none());
    assert!(archetype.raw_vertex_payload.is_none());
}

#[test]
fn archetype_invalid_provider_slot_role_fails() {
    let mut archetype = cargo_case_archetype();
    archetype.provider_slot_templates[0].target_role_id = "missing_role".to_owned();

    assert_issue(&archetype, "unknown_provider_slot_role");
}

#[test]
fn archetype_invalid_control_role_fails() {
    let mut archetype = cargo_case_archetype();
    archetype.control_axis_templates[0]
        .owns_role_ids
        .push("missing_role".to_owned());

    assert_issue(&archetype, "unknown_control_role");
}

#[test]
fn archetype_invalid_candidate_strategy_reference_fails() {
    let mut archetype = cargo_case_archetype();
    archetype.candidate_strategy_templates[0]
        .intended_changed_controls
        .push("missing_control".to_owned());
    archetype.candidate_strategy_templates[0]
        .intended_changed_roles
        .push("missing_role".to_owned());

    assert_issue(&archetype, "unknown_candidate_strategy_control");
    assert_issue(&archetype, "unknown_candidate_strategy_role");
}

#[test]
fn archetype_empty_quality_gates_fail() {
    let mut archetype = cargo_case_archetype();
    archetype.quality_gate_templates.clear();

    assert_issue(&archetype, "empty_quality_gates");
}

#[test]
fn archetype_geometry_payload_is_rejected() {
    let mut archetype = cargo_case_archetype();
    archetype.geometry_payload = Some("mesh bytes would be rejected".to_owned());
    archetype.raw_vertex_payload = Some("[[0,0,0],[1,0,0],[0,1,0]]".to_owned());

    assert_issue(&archetype, "geometry_payload_rejected");
    assert_issue(&archetype, "raw_vertex_payload_rejected");
}

#[test]
fn archetype_novice_catalog_visibility_is_rejected() {
    let mut archetype = cargo_case_archetype();
    archetype.publish_allowed = true;
    archetype.novice_visible = true;

    assert_issue(&archetype, "archetype_publish_forbidden");
    assert_issue(&archetype, "archetype_novice_visibility_forbidden");
}

#[test]
fn archetype_serde_roundtrip_is_deterministic() {
    let archetype = cargo_case_archetype();
    let first_json = serde_json::to_string_pretty(&archetype).expect("serialize archetype");
    let decoded: shape_foundry::FoundryArchetype =
        serde_json::from_str(&first_json).expect("deserialize archetype");
    let second_json = serde_json::to_string_pretty(&decoded).expect("serialize archetype again");

    assert_eq!(decoded, archetype);
    assert_eq!(second_json, first_json);
}

fn assert_issue(archetype: &shape_foundry::FoundryArchetype, expected_code: &str) {
    let report = validate_foundry_archetype(archetype);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "expected issue {expected_code}, got {:?}",
        report.issues
    );
}
