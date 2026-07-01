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
