#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use shape_foundry::{
    OBJECT_PLAN_SCHEMA_VERSION, ObjectPlan, ObjectPlanAttachment, ObjectPlanCreatedBy,
    ObjectPlanNode, ObjectPlanProvenance, ObjectPlanReviewTier, ObjectPlanValidationPolicy,
    PrimitiveAttachmentOffsetPolicy, PrimitiveAttachmentOrientationPolicy,
    PrimitiveAttachmentScalePolicy, PrimitiveKind, PrimitivePropertyValue,
    flat_panel_primitive_property_schema, primitive_default_property_values,
    sphere_primitive_property_schema,
};

#[test]
fn object_plan_cli_valid_sphere_plan_validates() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let plan_path = temp_dir.path().join("sphere-plan.json");
    write_plan(&plan_path, &one_sphere_plan());

    let output = Command::new(exe)
        .args(["object-plan", "validate"])
        .arg(&plan_path)
        .output()
        .expect("run object-plan validate");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report: shape_foundry::ObjectPlanValidationReport =
        serde_json::from_slice(&output.stdout).expect("parse validation stdout");
    assert!(report.is_valid());
}

#[test]
fn object_plan_cli_panel_plus_sphere_plan_validates() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
    let report: shape_foundry::ObjectPlanValidationReport =
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
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
fn object_plan_cli_batch_run_directory_reports_mixed_results() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
        "keep-regenerate-simplify.md",
        "batch-user-summary.md",
    ] {
        assert!(out_dir.join(name).is_file(), "{name} should exist");
    }
    assert!(
        !out_dir.join("batch-contact-sheet.png").exists(),
        "batch contact sheet must not be faked"
    );

    let report = read_json(out_dir.join("batch-validation-report.json"));
    assert_eq!(report["total_plans"], 2);
    assert_eq!(report["passed_validation"], 1);
    assert_eq!(report["failed_validation"], 1);
    assert_eq!(report["human_review_required"], true);
    assert_eq!(report["approved"], false);
    assert_eq!(report["rendered"], 0);
    assert_eq!(report["plans"].as_array().expect("plans").len(), 2);
}

#[test]
fn object_plan_cli_batch_run_json_uses_relative_plan_refs() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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
    assert_no_absolute_output_paths(&out_a, temp_dir.path());

    for name in [
        "batch-validation-report.json",
        "keep-regenerate-simplify.md",
        "batch-user-summary.md",
    ] {
        let first = fs::read(out_a.join(name)).expect("read first batch output");
        let second = fs::read(out_b.join(name)).expect("read second batch output");
        assert_eq!(first, second, "{name} should be deterministic");
    }
}

#[test]
fn object_plan_cli_has_no_llm_runtime_dependency() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let forbidden = ["openai", "reqwest", "ureq", "llm", "async-openai"];
    for term in forbidden {
        assert!(
            !manifest.to_ascii_lowercase().contains(term),
            "shape-cli must not add runtime LLM dependency {term}"
        );
    }
}

#[test]
fn object_plan_cli_rejects_raw_mesh_payload() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
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

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
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
