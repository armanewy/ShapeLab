use shape_foundry::{
    GeometryExportFormat, GeometryExportPolicy, GeometryExportReport, GeometryExportRequest,
    GeometryExportSourceKind, GeometryExportStatus, GeometryExportValidationReport,
    geometry_export_user_summary, validate_geometry_export_policy, validate_geometry_export_report,
    validate_geometry_export_request,
};

#[test]
fn geometry_export_policy_validates_for_geometry_only_glb() {
    let report = validate_geometry_export_policy(&GeometryExportPolicy::default());

    assert_valid(&report);
}

#[test]
fn geometry_export_request_validates_for_v0_scope() {
    let request = GeometryExportRequest {
        source_kind: GeometryExportSourceKind::MaterializedObjectDraft,
        source_ref: "target/object-plan-materialized/box/materialized-object-draft.json".to_owned(),
        export_format: GeometryExportFormat::Glb,
        export_policy: GeometryExportPolicy::default(),
        output_name: "asset".to_owned(),
        output_dir: "target/object-plan-geometry-export/box".to_owned(),
    };

    let report = validate_geometry_export_request(&request);

    assert_valid(&report);
}

#[test]
fn geometry_export_policy_blocks_texture_request() {
    let policy = GeometryExportPolicy {
        forbid_textures: false,
        ..Default::default()
    };

    let report = validate_geometry_export_policy(&policy);

    assert_issue(&report, "geometry_export_textures_forbidden");
}

#[test]
fn geometry_export_policy_blocks_rigging_request() {
    let policy = GeometryExportPolicy {
        forbid_rigging: false,
        ..Default::default()
    };

    let report = validate_geometry_export_policy(&policy);

    assert_issue(&report, "geometry_export_rigging_forbidden");
}

#[test]
fn geometry_export_policy_blocks_animation_request() {
    let policy = GeometryExportPolicy {
        forbid_animation: false,
        ..Default::default()
    };

    let report = validate_geometry_export_policy(&policy);

    assert_issue(&report, "geometry_export_animation_forbidden");
}

#[test]
fn geometry_export_policy_blocks_game_ready_claim() {
    let policy = GeometryExportPolicy {
        forbid_game_ready_claims: false,
        ..Default::default()
    };

    let report = validate_geometry_export_policy(&policy);

    assert_issue(&report, "geometry_export_game_ready_claims_forbidden");
}

#[test]
fn geometry_export_report_with_game_ready_true_is_invalid_in_v0() {
    let mut export_report = passed_report();
    export_report.game_ready = true;

    let report = validate_geometry_export_report(&export_report);

    assert_issue(&report, "geometry_export_game_ready_claims_forbidden");
}

#[test]
fn geometry_export_report_blocks_non_geometry_features() {
    let mut export_report = passed_report();
    export_report.includes_uvs = true;
    export_report.includes_textures = true;
    export_report.includes_material_looks = true;
    export_report.includes_collision = true;
    export_report.includes_rig = true;
    export_report.includes_animation = true;

    let report = validate_geometry_export_report(&export_report);

    assert_issue(&report, "geometry_export_uv_claims_forbidden");
    assert_issue(&report, "geometry_export_textures_forbidden");
    assert_issue(&report, "geometry_export_material_looks_forbidden");
    assert_issue(&report, "geometry_export_collision_claims_forbidden");
    assert_issue(&report, "geometry_export_rigging_forbidden");
    assert_issue(&report, "geometry_export_animation_forbidden");
}

#[test]
fn geometry_export_user_summary_is_product_safe() {
    let summary = geometry_export_user_summary(&passed_report());

    assert_eq!(summary.title, "Geometry export complete");
    assert!(
        summary
            .lines
            .iter()
            .any(|line| line == "Geometry-only GLB exported.")
    );
    assert!(
        summary
            .lines
            .iter()
            .any(|line| line == "No textures, collision, rigging, or animation are included.")
    );
    assert!(
        summary
            .lines
            .iter()
            .any(|line| line == "Godot import proof is required before calling this Godot-ready.")
    );
    assert_product_safe_summary(&summary.lines.join(" "));
}

#[test]
fn geometry_export_serde_roundtrip_is_deterministic() {
    let export_report = passed_report();

    let first = serde_json::to_string(&export_report).expect("export report serializes");
    let decoded = serde_json::from_str::<GeometryExportReport>(&first).expect("report decodes");
    let second = serde_json::to_string(&decoded).expect("export report serializes again");

    assert_eq!(first, second);
    assert_eq!(export_report, decoded);
}

fn passed_report() -> GeometryExportReport {
    GeometryExportReport {
        status: GeometryExportStatus::Passed,
        output_files: vec!["asset.glb".to_owned()],
        source_plan_id: Some("box_plan".to_owned()),
        primitive_count: 1,
        mesh_count: 1,
        triangle_count: 12,
        warning_count: 0,
        blockers: Vec::new(),
        includes_uvs: false,
        includes_textures: false,
        includes_material_looks: false,
        includes_collision: false,
        includes_rig: false,
        includes_animation: false,
        game_ready: false,
        human_review_required: true,
    }
}

fn assert_valid(report: &GeometryExportValidationReport) {
    assert!(report.is_valid(), "expected valid report, got {report:?}");
}

fn assert_issue(report: &GeometryExportValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "missing issue {expected_code}; got {:?}",
        report.issues
    );
}

fn assert_product_safe_summary(text: &str) {
    let lower = text.to_ascii_lowercase();
    for forbidden in ["textured", "rigged", "animated"] {
        assert!(
            !lower.contains(forbidden),
            "summary should not claim {forbidden}: {text}"
        );
    }
    assert!(
        !lower.contains("game-ready"),
        "summary should not claim game-ready: {text}"
    );
}
