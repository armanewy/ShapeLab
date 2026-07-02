#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use orchard_foundry::{
    AssetRequest, ObjectPlan, PrimitiveKind, PrototypePackBrief, PrototypePackCapability,
    PrototypePackCompositionKind, PrototypePackOutputPolicy, PrototypePackReviewPolicy,
    SupportedPrimitiveScope, prototype_pack_supported_scope_v0, validate_object_plan,
};

#[test]
fn prototype_pack_cli_supported_brief_generates_object_plans() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("prototype-pack");

    let output = Command::new(exe)
        .args(["prototype-pack", "plan", "--brief"])
        .arg(workspace_fixture())
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run prototype-pack plan");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("prototype-pack-plan-report.json"));
    assert_eq!(report["status"], "Passed");
    assert_eq!(report["generated_plan_count"], 4);
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
    assert_eq!(report["runtime_llm_used"], false);
    assert_eq!(report["public_catalog_publishing"], false);
    assert_eq!(report["game_ready"], false);
    assert!(out_dir.join("object-plan-batch.json").exists());

    for entry in fs::read_dir(out_dir.join("object-plans")).expect("read object-plans") {
        let path = entry.expect("dir entry").path();
        let plan: ObjectPlan =
            serde_json::from_slice(&fs::read(&path).expect("read plan")).expect("parse plan");
        assert!(
            validate_object_plan(&plan).is_valid(),
            "invalid plan {path:?}"
        );
        assert_eq!(
            plan.review_tier,
            orchard_foundry::ObjectPlanReviewTier::Draft
        );
    }
}

#[test]
fn prototype_pack_cli_unsupported_request_is_blocked() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let brief_path = temp_dir.path().join("unsupported-brief.json");
    let out_dir = temp_dir.path().join("out");
    write_brief(&brief_path, unsupported_brief());

    let output = Command::new(exe)
        .args(["prototype-pack", "plan", "--brief"])
        .arg(&brief_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run prototype-pack plan");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("prototype-pack-plan-report.json"));
    assert_eq!(report["status"], "Blocked");
    assert_eq!(report["generated_plan_count"], 0);
    assert_eq!(report["blocked_request_count"], 1);
    assert_eq!(report["request_reports"][0]["status"], "Blocked");
    assert_eq!(report["approved"], false);
    assert_eq!(report["publish_allowed"], false);
}

#[test]
fn prototype_pack_cli_outputs_are_draft_only_and_not_public() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let brief_path = temp_dir.path().join("brief.json");
    let out_dir = temp_dir.path().join("out");
    write_brief(&brief_path, supported_single_brief());

    let output = Command::new(exe)
        .args(["prototype-pack", "plan", "--brief"])
        .arg(&brief_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .output()
        .expect("run prototype-pack plan");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report =
        fs::read_to_string(out_dir.join("prototype-pack-plan-report.json")).expect("read report");
    let lower = report.to_ascii_lowercase();
    assert!(!lower.contains("\"approved\": true"));
    assert!(!lower.contains("\"publish_allowed\": true"));
    assert!(!lower.contains("\"runtime_llm_used\": true"));
    assert!(!lower.contains("\"public_catalog_publishing\": true"));
    assert!(!lower.contains("\"game_ready\": true"));
}

#[test]
fn prototype_pack_cli_output_is_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let first = temp_dir.path().join("first");
    let second = temp_dir.path().join("second");

    for out_dir in [&first, &second] {
        assert!(
            Command::new(exe)
                .args(["prototype-pack", "plan", "--brief"])
                .arg(workspace_fixture())
                .args(["--out-dir"])
                .arg(out_dir)
                .status()
                .expect("run prototype-pack plan")
                .success()
        );
    }

    let first_report =
        fs::read(first.join("prototype-pack-plan-report.json")).expect("read first report");
    let second_report =
        fs::read(second.join("prototype-pack-plan-report.json")).expect("read second report");
    assert_eq!(first_report, second_report);
}

fn supported_single_brief() -> PrototypePackBrief {
    PrototypePackBrief {
        brief_id: "single_box_brief".to_owned(),
        display_name: "Single Box Brief".to_owned(),
        purpose: "Draft one supported box primitive.".to_owned(),
        asset_requests: vec![AssetRequest {
            request_id: "single_box".to_owned(),
            display_name: "Single Box".to_owned(),
            intended_use: "Box primitive for review.".to_owned(),
            allowed_primitives: vec![PrimitiveKind::BoxPrimitive],
            allowed_compositions: Vec::new(),
            desired_count: 1,
            style_hint: None,
            must_have_capabilities: vec![PrototypePackCapability::ObjectPlanDraft],
            blocked_capabilities: vec![PrototypePackCapability::PublicCatalogPublishing],
        }],
        supported_primitive_scope: prototype_pack_supported_scope_v0(),
        output_policy: PrototypePackOutputPolicy::default(),
        review_policy: PrototypePackReviewPolicy::default(),
    }
}

fn unsupported_brief() -> PrototypePackBrief {
    PrototypePackBrief {
        brief_id: "unsupported_brief".to_owned(),
        display_name: "Unsupported Brief".to_owned(),
        purpose: "Request one unsupported primitive.".to_owned(),
        asset_requests: vec![AssetRequest {
            request_id: "unsupported_cylinder".to_owned(),
            display_name: "Unsupported Cylinder".to_owned(),
            intended_use: "Unsupported primitive for review.".to_owned(),
            allowed_primitives: vec![PrimitiveKind::CylinderPrimitive],
            allowed_compositions: Vec::new(),
            desired_count: 1,
            style_hint: None,
            must_have_capabilities: vec![PrototypePackCapability::ObjectPlanDraft],
            blocked_capabilities: Vec::new(),
        }],
        supported_primitive_scope: SupportedPrimitiveScope {
            primitive_kinds: vec![PrimitiveKind::BoxPrimitive],
            composition_kinds: vec![PrototypePackCompositionKind::PanelWithKnob],
        },
        output_policy: PrototypePackOutputPolicy::default(),
        review_policy: PrototypePackReviewPolicy::default(),
    }
}

fn workspace_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("fixtures/prototype-pack/simple-room-primitives-v0.json")
}

fn write_brief(path: &std::path::Path, brief: PrototypePackBrief) {
    fs::write(
        path,
        serde_json::to_string_pretty(&brief).expect("brief serializes"),
    )
    .expect("write brief");
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
