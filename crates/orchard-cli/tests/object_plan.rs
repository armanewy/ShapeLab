#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use orchard_foundry::{
    OBJECT_PLAN_SCHEMA_VERSION, ObjectPlan, ObjectPlanAttachment, ObjectPlanCreatedBy,
    ObjectPlanNode, ObjectPlanProvenance, ObjectPlanReviewTier, ObjectPlanValidationPolicy,
    PrimitiveAttachmentOffsetPolicy, PrimitiveAttachmentOrientationPolicy,
    PrimitiveAttachmentScalePolicy, PrimitiveKind, PrimitivePropertyValue,
    box_primitive_property_schema, flat_panel_primitive_property_schema,
    primitive_default_property_values, sphere_primitive_property_schema,
};

#[test]
fn object_plan_cli_valid_sphere_plan_validates() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("sphere-plan.json");
    write_plan(&plan_path, &one_sphere_plan());

    let output = Command::new(exe)
        .args(["object-plan", "validate"])
        .arg(&plan_path)
        .output()
        .expect("run object-plan validate");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report: orchard_foundry::ObjectPlanValidationReport =
        serde_json::from_slice(&output.stdout).expect("parse validation stdout");
    assert!(report.is_valid());
}

#[test]
fn object_plan_cli_panel_plus_sphere_plan_validates() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("panel-plan.json");
    write_plan(&plan_path, &panel_with_sphere_plan());

    assert!(
        Command::new(exe)
            .args(["object-plan", "validate"])
            .arg(&plan_path)
            .status()
            .expect("run object-plan validate")
            .success()
    );
}

#[test]
fn object_plan_cli_invalid_plan_rejected() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("invalid-plan.json");
    let mut plan = one_sphere_plan();
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));
    write_plan(&plan_path, &plan);

    let output = Command::new(exe)
        .args(["object-plan", "validate"])
        .arg(&plan_path)
        .output()
        .expect("run invalid object-plan validate");

    assert!(!output.status.success());
    let report: orchard_foundry::ObjectPlanValidationReport =
        serde_json::from_slice(&output.stdout).expect("parse validation stdout");
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "invalid_current_property_value"),
        "expected invalid property issue, got {:?}",
        report.issues
    );
}

#[test]
fn object_plan_cli_render_outputs_are_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("panel-plan.json");
    let first_dir = temp_dir.path().join("run-a");
    let second_dir = temp_dir.path().join("run-b");
    write_plan(&plan_path, &panel_with_sphere_plan());

    for out_dir in [&first_dir, &second_dir] {
        assert!(
            Command::new(exe)
                .args(["object-plan", "render", "--plan"])
                .arg(&plan_path)
                .args(["--out-dir"])
                .arg(out_dir)
                .status()
                .expect("run object-plan render")
                .success()
        );
        for name in [
            "validation-report.json",
            "primitive-summary.json",
            "rendering-report.json",
            "plan-user-summary.md",
        ] {
            assert!(out_dir.join(name).is_file(), "{name} should exist");
        }
        assert!(
            !out_dir.join("contact-sheet.png").exists(),
            "contact sheet should wait for render materialization"
        );
        let rendering: serde_json::Value = serde_json::from_slice(
            &fs::read(out_dir.join("rendering-report.json")).expect("read rendering report"),
        )
        .expect("parse rendering report");
        assert_eq!(rendering["status"], "blocked");
        assert_eq!(rendering["contact_sheet_written"], false);
    }

    for name in [
        "validation-report.json",
        "primitive-summary.json",
        "rendering-report.json",
        "plan-user-summary.md",
    ] {
        let first = fs::read(first_dir.join(name)).expect("read first output");
        let second = fs::read(second_dir.join(name)).expect("read second output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_run_contact_sheet_reports_missing_render_binding() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("panel-plan.json");
    let out_dir = temp_dir.path().join("run-contact-sheet");
    write_plan(&plan_path, &panel_with_sphere_plan());

    let output = Command::new(exe)
        .args(["object-plan", "run", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .arg("--contact-sheet")
        .output()
        .expect("run object-plan contact sheet");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    for name in [
        "validation-report.json",
        "primitive-summary.json",
        "normalized-object-plan.json",
        "renderability-report.json",
        "rendering-report.json",
        "visual-evidence-report.json",
        "plan-user-summary.md",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }
    assert!(
        !out_dir.join("contact-sheet.png").exists(),
        "contact sheet must not be faked without render bindings"
    );

    let renderability: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("renderability-report.json")).expect("read renderability report"),
    )
    .expect("parse renderability report");
    assert_eq!(renderability["renderable"], false);
    assert!(
        renderability["missing_preview_bindings"]
            .as_array()
            .expect("missing bindings array")
            .len()
            >= 2
    );
    assert!(
        renderability["reason"]
            .as_str()
            .expect("reason string")
            .contains("materialization")
    );

    let evidence: serde_json::Value = serde_json::from_slice(
        &fs::read(out_dir.join("visual-evidence-report.json"))
            .expect("read visual evidence report"),
    )
    .expect("parse visual evidence report");
    assert_eq!(evidence["rendered"], false);
    assert_eq!(evidence["preview_count"], 0);
    assert_eq!(evidence["contact_sheet_path"], serde_json::Value::Null);
    assert_eq!(evidence["user_review_required"], true);
    assert_eq!(evidence["approved"], false);
}

#[test]
fn object_plan_cli_invalid_run_does_not_fake_contact_sheet() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("invalid-plan.json");
    let out_dir = temp_dir.path().join("invalid-run");
    let mut plan = one_sphere_plan();
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));
    write_plan(&plan_path, &plan);

    let output = Command::new(exe)
        .args(["object-plan", "run", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .arg("--contact-sheet")
        .output()
        .expect("run invalid object-plan contact sheet");

    assert!(!output.status.success());
    assert!(out_dir.join("validation-report.json").is_file());
    assert!(out_dir.join("renderability-report.json").is_file());
    assert!(
        !out_dir.join("contact-sheet.png").exists(),
        "invalid plans must not emit contact sheets"
    );
}

#[test]
fn object_plan_cli_materialize_valid_box_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("box-plan.json");
    let out_dir = temp_dir.path().join("box-materialized");
    write_plan(&plan_path, &one_box_plan());

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    for name in [
        "materialized-object-draft.json",
        "materialization-report.json",
        "materialized-user-summary.md",
        "normalized-object-plan.json",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }
    let report = read_json(out_dir.join("materialization-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["primitive_count"], 1);
    assert_eq!(report["materialized_primitive_count"], 1);
    assert_eq!(report["attachment_count"], 0);
    assert_eq!(report["materialized_attachment_count"], 0);
    assert_eq!(report["user_review_required"], true);
    assert_eq!(report["publish_allowed"], false);
    assert!(!out_dir.join("unresolved-nodes.json").exists());
    assert!(!out_dir.join("unresolved-attachments.json").exists());
}

#[test]
fn object_plan_cli_materialize_valid_flat_panel_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("flat-panel-plan.json");
    let out_dir = temp_dir.path().join("flat-panel-materialized");
    write_plan(&plan_path, &one_flat_panel_plan());

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("materialization-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["materialized_primitive_count"], 1);
}

#[test]
fn object_plan_cli_materialize_box_render_evidence_writes_real_contact_sheet() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = fixture_path("valid_box_plan.json");
    let out_dir = temp_dir.path().join("box-render-evidence");

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .arg("--render-evidence")
        .output()
        .expect("run object-plan materialize render evidence");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_png(out_dir.join("plan-preview.png"));
    assert_png(out_dir.join("node-previews/box.png"));
    assert_png(out_dir.join("contact-sheet.png"));
    let report = read_json(out_dir.join("render-evidence-report.json"));
    assert_eq!(report["rendered"], true);
    assert_eq!(report["materialized"], true);
    assert_eq!(report["plan_id"], "box_plan");
    assert_eq!(report["preview_count"], 2);
    assert_eq!(report["contact_sheet_path"], "contact-sheet.png");
    assert_eq!(report["user_review_required"], true);
    assert_eq!(report["approved"], false);
    assert_eq!(
        report["unsupported_primitives"].as_array().unwrap().len(),
        0
    );
    assert_eq!(
        report["unsupported_attachments"].as_array().unwrap().len(),
        0
    );
}

#[test]
fn object_plan_cli_materialize_flat_panel_render_evidence_writes_real_contact_sheet() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = fixture_path("valid_flat_panel_plan.json");
    let out_dir = temp_dir.path().join("panel-render-evidence");

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .arg("--render-evidence")
        .output()
        .expect("run object-plan materialize render evidence");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_png(out_dir.join("plan-preview.png"));
    assert_png(out_dir.join("node-previews/panel.png"));
    assert_png(out_dir.join("contact-sheet.png"));
    let report = read_json(out_dir.join("render-evidence-report.json"));
    assert_eq!(report["rendered"], true);
    assert_eq!(report["materialized"], true);
    assert_eq!(report["plan_id"], "flat_panel_plan");
    assert_eq!(report["approved"], false);
}

#[test]
fn object_plan_cli_materialize_unsupported_render_evidence_blocks_without_contact_sheet() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = fixture_path("invalid_unknown_primitive_plan.json");
    let out_dir = temp_dir.path().join("unsupported-render-evidence");

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .arg("--render-evidence")
        .output()
        .expect("run unsupported object-plan materialize render evidence");

    assert!(!output.status.success());
    assert!(out_dir.join("materialization-report.json").is_file());
    assert!(out_dir.join("render-evidence-report.json").is_file());
    assert!(!out_dir.join("contact-sheet.png").exists());
    assert!(!out_dir.join("plan-preview.png").exists());
    assert!(!out_dir.join("node-previews").exists());
    let report = read_json(out_dir.join("render-evidence-report.json"));
    assert_eq!(report["rendered"], false);
    assert_eq!(report["materialized"], false);
    assert_eq!(report["approved"], false);
    assert_eq!(report["user_review_required"], true);
    assert!(
        !report["unsupported_primitives"]
            .as_array()
            .expect("unsupported primitives")
            .is_empty()
    );
    assert!(
        report["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning.as_str().unwrap_or_default().contains("blocked"))
    );
}

#[test]
fn object_plan_cli_materialize_valid_panel_knob_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("panel-knob-plan.json");
    let out_dir = temp_dir.path().join("panel-knob-materialized");
    write_plan(&plan_path, &panel_with_sphere_plan());

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("materialization-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["primitive_count"], 2);
    assert_eq!(report["materialized_primitive_count"], 2);
    assert_eq!(report["attachment_count"], 1);
    assert_eq!(report["materialized_attachment_count"], 1);
    assert_eq!(report["publish_allowed"], false);
}

#[test]
fn object_plan_cli_export_geometry_valid_box_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("box-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("valid_box_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    for name in [
        "asset.glb",
        "geometry-export-report.json",
        "geometry-export-user-summary.md",
        "normalized-object-plan.json",
        "materialization-report.json",
        "render-evidence-report.json",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }
    assert_glb(out_dir.join("asset.glb"));
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["output_files"][0], "asset.glb");
    assert_eq!(report["source_plan_id"], "box_plan");
    assert_eq!(report["primitive_count"], 1);
    assert_eq!(report["mesh_count"], 1);
    assert!(
        report["triangle_count"].as_u64().expect("triangle count") > 0,
        "triangle count should be positive"
    );
    assert_geometry_export_report_excludes_non_geometry_features(&report);
    assert_eq!(
        report["relationship_realizations"]
            .as_array()
            .expect("relationship realizations")
            .len(),
        0
    );
    let summary = fs::read_to_string(out_dir.join("geometry-export-user-summary.md"))
        .expect("read geometry summary");
    assert!(summary.contains("Geometry-only GLB exported."));
    assert!(summary.contains("No textures, collision, rigging, or animation are included."));
    assert!(summary.contains("Godot import proof is required before calling this Godot-ready."));
    assert!(
        !summary.to_ascii_lowercase().contains("game-ready"),
        "summary must not claim game-ready status"
    );
}

#[test]
fn object_plan_cli_export_geometry_valid_flat_panel_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("panel-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("valid_flat_panel_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_glb(out_dir.join("asset.glb"));
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["source_plan_id"], "flat_panel_plan");
    assert_geometry_export_report_excludes_non_geometry_features(&report);
}

#[test]
fn object_plan_cli_export_geometry_valid_sphere_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("sphere-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("valid_sphere_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_glb(out_dir.join("asset.glb"));
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["source_plan_id"], "round_knob_plan");
    assert_geometry_export_report_excludes_non_geometry_features(&report);
}

#[test]
fn object_plan_cli_export_geometry_valid_panel_knob_plan() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("panel-knob-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("valid_panel_knob_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_glb(out_dir.join("asset.glb"));
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["source_plan_id"], "panel_with_knob_plan");
    assert_eq!(report["primitive_count"], 2);
    assert_geometry_export_report_excludes_non_geometry_features(&report);
    let relationship_realizations = report["relationship_realizations"]
        .as_array()
        .expect("relationship realizations");
    assert_eq!(relationship_realizations.len(), 1);
    let realization = &relationship_realizations[0];
    assert_eq!(realization["relationship_id"], 1);
    assert_eq!(realization["relationship_type"], "SurfaceMounted");
    assert_eq!(realization["realization_policy"], "PreserveSemanticSidecar");
    assert_eq!(realization["output_node"], serde_json::Value::Null);
    assert_eq!(realization["output_mesh"], "asset.glb#mesh0");
    assert_eq!(realization["child_output"], "CombinedMesh");
    assert_eq!(realization["baked"], false);
    assert_eq!(realization["semantics_preserved_in_sidecar"], true);
    let summary = fs::read_to_string(out_dir.join("geometry-export-user-summary.md"))
        .expect("read geometry summary");
    assert!(summary.contains("child included in combined mesh"));
    assert!(summary.contains("baked: false"));
}

#[test]
fn object_plan_cli_export_geometry_texture_request_blocked() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("texture-request-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("invalid_texture_request_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(!output.status.success());
    assert!(!out_dir.join("asset.glb").exists());
    assert!(!out_dir.join("materialization-report.json").exists());
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Blocked");
    assert_eq!(report["game_ready"], false);
    assert!(
        report["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker.as_str().unwrap_or_default().contains("Texture"))
    );
}

#[test]
fn object_plan_cli_export_geometry_game_ready_request_blocked() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("game-ready-request-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("invalid_game_ready_request_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(!output.status.success());
    assert!(!out_dir.join("asset.glb").exists());
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Blocked");
    assert_eq!(report["game_ready"], false);
    assert!(
        report["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker.as_str().unwrap_or_default().contains("Game-ready"))
    );
}

#[test]
fn object_plan_cli_export_geometry_unsupported_unresolved_plan_blocked() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("unsupported-geometry-export");

    let output = Command::new(exe)
        .args(["object-plan", "export-geometry", "--plan"])
        .arg(fixture_path("invalid_unknown_primitive_plan.json"))
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--format", "glb"])
        .output()
        .expect("run object-plan export geometry");

    assert!(!output.status.success());
    assert!(!out_dir.join("asset.glb").exists());
    assert!(out_dir.join("materialization-report.json").is_file());
    let report = read_json(out_dir.join("geometry-export-report.json"));
    assert_eq!(report["status"], "Blocked");
    assert_geometry_export_report_excludes_non_geometry_features(&report);
    assert!(!report["blockers"].as_array().expect("blockers").is_empty());
}

#[test]
fn object_plan_cli_export_geometry_outputs_are_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let first_dir = temp_dir.path().join("geometry-export-a");
    let second_dir = temp_dir.path().join("geometry-export-b");

    for out_dir in [&first_dir, &second_dir] {
        assert!(
            Command::new(exe)
                .args(["object-plan", "export-geometry", "--plan"])
                .arg(fixture_path("valid_box_plan.json"))
                .args(["--out-dir"])
                .arg(out_dir)
                .args(["--format", "glb"])
                .status()
                .expect("run object-plan export geometry")
                .success()
        );
    }

    for name in [
        "asset.glb",
        "geometry-export-report.json",
        "geometry-export-user-summary.md",
        "normalized-object-plan.json",
        "materialization-report.json",
        "render-evidence-report.json",
    ] {
        let first = fs::read(first_dir.join(name)).expect("read first output");
        let second = fs::read(second_dir.join(name)).expect("read second output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_materialize_raw_mesh_payload_fails() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("raw-plan.json");
    let out_dir = temp_dir.path().join("raw-materialized");
    let mut value = serde_json::to_value(one_box_plan()).expect("plan value");
    value.as_object_mut().expect("plan object").insert(
        "raw_mesh_payload".to_owned(),
        serde_json::json!({"vertices": [[0, 0, 0]]}),
    );
    fs::write(
        &plan_path,
        serde_json::to_vec_pretty(&value).expect("serialize raw plan"),
    )
    .expect("write raw plan");

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(!output.status.success());
    assert!(!out_dir.join("materialization-report.json").exists());
}

#[test]
fn object_plan_cli_materialize_public_publish_plan_fails() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("publish-plan.json");
    let out_dir = temp_dir.path().join("publish-materialized");
    let mut plan = one_box_plan();
    plan.validation_policy.allow_public_catalog_publish = true;
    write_plan(&plan_path, &plan);

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(!output.status.success());
    let report = read_json(out_dir.join("materialization-report.json"));
    assert_eq!(report["status"], "Failed");
    assert_eq!(report["publish_allowed"], false);
    assert_eq!(report["user_review_required"], true);
}

#[test]
fn object_plan_cli_materialize_invalid_attachment_plan_fails() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("invalid-attachment-plan.json");
    let out_dir = temp_dir.path().join("invalid-attachment-materialized");
    let mut plan = panel_with_sphere_plan();
    plan.attachments[0].parent_anchor_id = "hinge_edge_zone".to_owned();
    write_plan(&plan_path, &plan);

    let output = Command::new(exe)
        .args(["object-plan", "materialize", "--plan"])
        .arg(&plan_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan materialize");

    assert!(!output.status.success());
    let report = read_json(out_dir.join("materialization-report.json"));
    assert_eq!(report["status"], "Failed");
    assert_eq!(
        report["unresolved_attachments"]
            .as_array()
            .expect("attachments")
            .len(),
        1
    );
    assert!(out_dir.join("unresolved-attachments.json").is_file());
}

#[test]
fn object_plan_cli_materialize_outputs_are_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("box-plan.json");
    let first_dir = temp_dir.path().join("materialized-a");
    let second_dir = temp_dir.path().join("materialized-b");
    write_plan(&plan_path, &one_box_plan());

    for out_dir in [&first_dir, &second_dir] {
        assert!(
            Command::new(exe)
                .args(["object-plan", "materialize", "--plan"])
                .arg(&plan_path)
                .args(["--out-dir"])
                .arg(out_dir)
                .status()
                .expect("run object-plan materialize")
                .success()
        );
    }

    for name in [
        "materialized-object-draft.json",
        "materialization-report.json",
        "materialized-user-summary.md",
        "normalized-object-plan.json",
    ] {
        let first = fs::read(first_dir.join(name)).expect("read first output");
        let second = fs::read(second_dir.join(name)).expect("read second output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_materialize_render_evidence_outputs_are_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("box-plan.json");
    let first_dir = temp_dir.path().join("render-evidence-a");
    let second_dir = temp_dir.path().join("render-evidence-b");
    write_plan(&plan_path, &one_box_plan());

    for out_dir in [&first_dir, &second_dir] {
        assert!(
            Command::new(exe)
                .args(["object-plan", "materialize", "--plan"])
                .arg(&plan_path)
                .args(["--out-dir"])
                .arg(out_dir)
                .arg("--render-evidence")
                .status()
                .expect("run object-plan materialize render evidence")
                .success()
        );
    }

    for name in [
        "render-evidence-report.json",
        "plan-preview.png",
        "node-previews/box.png",
        "contact-sheet.png",
    ] {
        let first = fs::read(first_dir.join(name)).expect("read first output");
        let second = fs::read(second_dir.join(name)).expect("read second output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_batch_run_directory_reports_mixed_results() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plans_dir = temp_dir.path().join("plans");
    fs::create_dir_all(&plans_dir).expect("create plans dir");
    write_plan(&plans_dir.join("a-valid.json"), &one_sphere_plan());
    write_plan(&plans_dir.join("b-invalid.json"), &invalid_width_plan());
    let out_dir = temp_dir.path().join("batch-out");

    let output = Command::new(exe)
        .args(["object-plan", "batch-run", "--input"])
        .arg(&plans_dir)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan batch");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    for name in [
        "batch-validation-report.json",
        "batch-materialization-report.json",
        "batch-render-evidence-report.json",
        "batch-contact-sheet.png",
        "keep-regenerate-simplify.md",
        "batch-user-summary.md",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }
    assert_png(out_dir.join("batch-contact-sheet.png"));

    let report = read_json(out_dir.join("batch-validation-report.json"));
    assert_eq!(report["total_plans"], 2);
    assert_eq!(report["passed_validation"], 1);
    assert_eq!(report["failed_validation"], 1);
    assert_eq!(report["human_review_required"], true);
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
    assert_eq!(report["rendered"], 1);
    assert_eq!(report["plans"].as_array().expect("plans").len(), 2);
    assert!(
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .any(|plan| plan["recommendation"] == "Keep")
    );
    assert!(
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .any(|plan| plan["recommendation"] == "Blocked")
    );
    let materialization = read_json(out_dir.join("batch-materialization-report.json"));
    assert_eq!(materialization["passed"], 1);
    assert_eq!(materialization["failed"], 1);
    assert_eq!(materialization["publish_allowed"], false);
    let render = read_json(out_dir.join("batch-render-evidence-report.json"));
    assert_eq!(render["rendered"], 1);
    assert_eq!(render["blocked"], 1);
    assert_eq!(render["contact_sheet_path"], "batch-contact-sheet.png");
    assert_eq!(render["approved"], false);
    assert_eq!(render["publish_allowed"], false);
}

#[test]
fn object_plan_cli_batch_run_json_uses_relative_plan_refs() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plans_dir = temp_dir.path().join("plans");
    fs::create_dir_all(&plans_dir).expect("create plans dir");
    write_plan(&plans_dir.join("sphere.json"), &one_sphere_plan());
    write_plan(&plans_dir.join("panel.json"), &panel_with_sphere_plan());
    let batch_path = temp_dir.path().join("batch.json");
    fs::write(
        &batch_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "batch_id": "basic_batch",
            "display_name": "Basic Batch",
            "plans": ["plans/sphere.json", "plans/panel.json"],
            "review_policy": {"human_review_required": false},
            "output_policy": {"contact_sheet": true}
        }))
        .expect("batch json"),
    )
    .expect("write batch json");
    let out_a = temp_dir.path().join("batch-a");
    let out_b = temp_dir.path().join("batch-b");

    for out_dir in [&out_a, &out_b] {
        assert!(
            Command::new(exe)
                .args(["object-plan", "batch-run", "--input"])
                .arg(&batch_path)
                .args(["--out-dir"])
                .arg(out_dir)
                .status()
                .expect("run object-plan batch json")
                .success()
        );
    }

    let report = read_json(out_a.join("batch-validation-report.json"));
    assert_eq!(report["batch_id"], "basic_batch");
    assert_eq!(report["display_name"], "Basic Batch");
    assert_eq!(report["total_plans"], 2);
    assert_eq!(report["human_review_required"], true);
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
    assert_eq!(report["rendered"], 2);
    assert_png(out_a.join("batch-contact-sheet.png"));
    assert_no_absolute_output_paths(&out_a, temp_dir.path());

    for name in [
        "batch-validation-report.json",
        "batch-materialization-report.json",
        "batch-render-evidence-report.json",
        "keep-regenerate-simplify.md",
        "batch-user-summary.md",
        "batch-contact-sheet.png",
    ] {
        let first = fs::read(out_a.join(name)).expect("read first batch output");
        let second = fs::read(out_b.join(name)).expect("read second batch output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_batch_run_unsupported_plan_does_not_crash() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plans_dir = temp_dir.path().join("plans");
    fs::create_dir_all(&plans_dir).expect("create plans dir");
    fs::copy(
        fixture_path("valid_box_plan.json"),
        plans_dir.join("valid-box.json"),
    )
    .expect("copy valid box");
    fs::copy(
        fixture_path("invalid_unknown_primitive_plan.json"),
        plans_dir.join("unsupported.json"),
    )
    .expect("copy unsupported plan");
    let out_dir = temp_dir.path().join("batch-out");

    let output = Command::new(exe)
        .args(["object-plan", "batch-run", "--input"])
        .arg(&plans_dir)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run object-plan batch");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("batch-validation-report.json"));
    assert_eq!(report["total_plans"], 2);
    assert_eq!(report["rendered"], 1);
    assert!(
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .any(|plan| plan["recommendation"] == "Blocked")
    );
    let render = read_json(out_dir.join("batch-render-evidence-report.json"));
    assert_eq!(render["rendered"], 1);
    assert_eq!(render["blocked"], 1);
    assert_png(out_dir.join("batch-contact-sheet.png"));
}

#[test]
fn object_plan_cli_has_no_llm_runtime_dependency() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let forbidden = ["openai", "reqwest", "ureq", "llm", "async-openai"];
    for term in forbidden {
        assert!(
            !manifest.to_ascii_lowercase().contains(term),
            "orchard-cli must not add runtime LLM dependency {term}"
        );
    }
}

#[test]
fn object_plan_cli_rejects_raw_mesh_payload() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("raw-plan.json");
    let mut value = serde_json::to_value(one_sphere_plan()).expect("plan value");
    value.as_object_mut().expect("plan object").insert(
        "raw_mesh_payload".to_owned(),
        serde_json::json!({"vertices": [[0, 0, 0]]}),
    );
    fs::write(
        &plan_path,
        serde_json::to_vec_pretty(&value).expect("serialize raw plan"),
    )
    .expect("write raw plan");

    assert!(
        !Command::new(exe)
            .args(["object-plan", "validate"])
            .arg(&plan_path)
            .status()
            .expect("run object-plan validate")
            .success()
    );
}

fn write_plan(path: &std::path::Path, plan: &ObjectPlan) {
    fs::write(
        path,
        serde_json::to_vec_pretty(plan).expect("serialize plan"),
    )
    .expect("write plan");
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

fn invalid_width_plan() -> ObjectPlan {
    let mut plan = one_sphere_plan();
    plan.nodes[0]
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));
    plan
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
            parent_anchor_id: "front_handle_zone".to_owned(),
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

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/object-plan")
        .join(name)
}

fn assert_png(path: impl AsRef<std::path::Path>) {
    let bytes = fs::read(path.as_ref()).expect("read png");
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} should be a PNG",
        path.as_ref().display()
    );
    assert!(
        bytes.len() > 128,
        "{} should not be empty",
        path.as_ref().display()
    );
}

fn assert_glb(path: impl AsRef<std::path::Path>) {
    let bytes = fs::read(path.as_ref()).expect("read glb");
    assert!(
        bytes.len() > 64,
        "{} should not be empty",
        path.as_ref().display()
    );
    assert_eq!(&bytes[0..4], b"glTF", "GLB magic should be present");
    assert_eq!(read_le_u32(&bytes, 4), 2, "GLB version should be 2");
    assert_eq!(
        read_le_u32(&bytes, 8) as usize,
        bytes.len(),
        "GLB length should match file length"
    );
    let json_length = read_le_u32(&bytes, 12) as usize;
    assert_eq!(&bytes[16..20], b"JSON", "first chunk should be JSON");
    let json_start = 20;
    let json_end = json_start + json_length;
    let json_text = std::str::from_utf8(&bytes[json_start..json_end])
        .expect("GLB JSON is UTF-8")
        .trim_end();
    let json: serde_json::Value = serde_json::from_str(json_text).expect("parse GLB JSON");
    assert_eq!(json["asset"]["version"], "2.0");
    assert!(json.get("images").is_none(), "GLB must not include images");
    assert!(
        json.get("textures").is_none(),
        "GLB must not include textures"
    );
    assert!(json.get("skins").is_none(), "GLB must not include skins");
    assert!(
        json.get("animations").is_none(),
        "GLB must not include animations"
    );
    let accessors = json["accessors"].as_array().expect("accessors");
    assert_eq!(accessors.len(), 3);
    let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
    assert!(attributes.get("POSITION").is_some());
    assert!(attributes.get("NORMAL").is_some());
    assert!(
        attributes.get("TEXCOORD_0").is_none(),
        "geometry-only GLB must not include UV claims"
    );
    let bin_header = json_end;
    assert_eq!(&bytes[bin_header + 4..bin_header + 8], b"BIN\0");
}

fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("read u32 bytes"),
    )
}

fn assert_geometry_export_report_excludes_non_geometry_features(report: &serde_json::Value) {
    assert_eq!(report["includes_uvs"], false);
    assert_eq!(report["includes_textures"], false);
    assert_eq!(report["includes_material_looks"], false);
    assert_eq!(report["includes_collision"], false);
    assert_eq!(report["includes_rig"], false);
    assert_eq!(report["includes_animation"], false);
    assert_eq!(report["game_ready"], false);
    assert_eq!(report["human_review_required"], true);
}

fn assert_no_absolute_output_paths(out_dir: &std::path::Path, temp_dir: &std::path::Path) {
    let needle = temp_dir.to_string_lossy();
    let mut stack = vec![out_dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(path).expect("read output dir") {
                stack.push(entry.expect("dir entry").path());
            }
            continue;
        }
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "json" | "md"))
        {
            let text = fs::read_to_string(&path).expect("read output text");
            assert!(
                !text.contains(needle.as_ref()),
                "{} should not persist absolute temp paths",
                path.display()
            );
        }
    }
}
