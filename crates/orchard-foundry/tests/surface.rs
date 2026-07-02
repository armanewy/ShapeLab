use orchard_foundry::{
    MaterialSlotPolicy, PrimitiveKind, PrimitiveSurfaceCapability,
    PrimitiveSurfaceValidationReport, UvPolicy, panel_with_knob_surface_capability,
    primitive_surface_capabilities_v0, primitive_surface_capability, primitive_surface_ui_enabled,
    validate_primitive_surface_capability,
};

#[test]
fn primitive_surface_policies_validate() {
    let policies = primitive_surface_capabilities_v0();

    assert_eq!(policies.len(), 4);
    for policy in &policies {
        assert_valid(&validate_primitive_surface_capability(policy));
        assert!(policy.review_required);
    }
}

#[test]
fn primitive_surface_initial_policies_are_future_candidates_only() {
    let box_policy = primitive_surface_capability(PrimitiveKind::BoxPrimitive);
    assert_eq!(box_policy.uv_policy, UvPolicy::BoxProjection);

    let panel_policy = primitive_surface_capability(PrimitiveKind::FlatPanelPrimitive);
    assert_eq!(panel_policy.uv_policy, UvPolicy::PlanarProjection);

    let sphere_policy = primitive_surface_capability(PrimitiveKind::SpherePrimitive);
    assert_eq!(sphere_policy.uv_policy, UvPolicy::SphericalProjection);

    let panel_knob_policy = panel_with_knob_surface_capability();
    assert_eq!(
        panel_knob_policy.uv_policy,
        UvPolicy::PerNodePrimitivePolicy
    );
}

#[test]
fn primitive_surface_policy_does_not_enable_ui() {
    for policy in primitive_surface_capabilities_v0() {
        assert!(!policy.supported);
        assert!(!primitive_surface_ui_enabled(&policy));
        assert!(!policy.blocked_reasons.is_empty());
    }

    let mut invalid = primitive_surface_capability(PrimitiveKind::BoxPrimitive);
    invalid.supported = true;
    let report = validate_primitive_surface_capability(&invalid);
    assert_issue(&report, "primitive_surface_ui_disabled_v0");
}

#[test]
fn primitive_surface_policy_emits_no_texture_path() {
    for policy in primitive_surface_capabilities_v0() {
        let serialized = serde_json::to_string(&policy).expect("policy serializes");
        for forbidden in [".png", ".jpg", ".jpeg", ".webp", ".ktx", "texture path"] {
            assert!(
                !serialized.to_ascii_lowercase().contains(forbidden),
                "surface policy should not emit {forbidden}: {serialized}"
            );
        }
    }
}

#[test]
fn primitive_surface_policy_has_no_material_editor_or_game_ready_claim() {
    for policy in primitive_surface_capabilities_v0() {
        let serialized = serde_json::to_string(&policy).expect("policy serializes");
        let lower = serialized.to_ascii_lowercase();
        assert!(!lower.contains("material editor"));
        assert!(!lower.contains("game-ready"));
    }

    let mut invalid = primitive_surface_capability(PrimitiveKind::BoxPrimitive);
    invalid
        .blocked_reasons
        .push("Material editor is ready.".to_owned());
    let report = validate_primitive_surface_capability(&invalid);
    assert_issue(&report, "primitive_surface_forbidden_claim");
}

#[test]
fn primitive_surface_policy_keeps_neutral_clay_only() {
    let mut invalid = primitive_surface_capability(PrimitiveKind::SpherePrimitive);
    invalid.material_slot_policy = MaterialSlotPolicy::SingleMaterialSlot;

    let report = validate_primitive_surface_capability(&invalid);

    assert_issue(&report, "primitive_surface_material_slots_disabled_v0");
}

#[test]
fn primitive_surface_serde_roundtrip_is_deterministic() {
    let policy = panel_with_knob_surface_capability();

    let first = serde_json::to_string(&policy).expect("policy serializes");
    let decoded =
        serde_json::from_str::<PrimitiveSurfaceCapability>(&first).expect("policy decodes");
    let second = serde_json::to_string(&decoded).expect("policy serializes again");

    assert_eq!(first, second);
    assert_eq!(policy, decoded);
}

fn assert_valid(report: &PrimitiveSurfaceValidationReport) {
    assert!(report.is_valid(), "expected valid report, got {report:?}");
}

fn assert_issue(report: &PrimitiveSurfaceValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing issue {expected_code}; got {:?}",
        report.issues
    );
}
