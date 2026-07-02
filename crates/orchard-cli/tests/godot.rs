#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

#[test]
fn godot_geometry_import_missing_binary_returns_blocked_report() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let glb_path = temp_dir.path().join("asset.glb");
    fs::write(&glb_path, b"glTF dummy geometry-only payload").expect("write dummy glb");
    let out_dir = temp_dir.path().join("godot-proof");
    let missing_godot = temp_dir.path().join("missing-godot");

    let output = Command::new(exe)
        .args(["godot-proof", "geometry-import", "--glb"])
        .arg(&glb_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--godot-bin"])
        .arg(&missing_godot)
        .output()
        .expect("run godot proof");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = read_json(out_dir.join("godot-import-proof-report.json"));
    assert_eq!(report["status"], "Blocked");
    assert_eq!(report["godot_available"], false);
    assert_eq!(report["godot_version"], serde_json::Value::Null);
    assert_eq!(report["source_glb"], "asset.glb");
    assert_godot_report_excludes_later_features(&report);
    assert!(
        report["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker
                .as_str()
                .unwrap_or_default()
                .contains("Godot binary"))
    );
    assert!(!out_dir.join("godot-project").exists());
    assert!(!out_dir.join("imported-asset-report.json").exists());
}

#[test]
fn godot_geometry_import_invalid_glb_path_fails_cleanly() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let glb_path = temp_dir.path().join("missing.glb");
    let out_dir = temp_dir.path().join("godot-proof");
    let missing_godot = temp_dir.path().join("missing-godot");

    let output = Command::new(exe)
        .args(["godot-proof", "geometry-import", "--glb"])
        .arg(&glb_path)
        .args(["--out-dir"])
        .arg(&out_dir)
        .args(["--godot-bin"])
        .arg(&missing_godot)
        .output()
        .expect("run godot proof");

    assert!(!output.status.success());
    let report = read_json(out_dir.join("godot-import-proof-report.json"));
    assert_eq!(report["status"], "Failed");
    assert_eq!(report["source_glb"], "missing.glb");
    assert_godot_report_excludes_later_features(&report);
    assert!(
        report["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker.as_str().unwrap_or_default().contains("Source GLB"))
    );
}

#[test]
fn godot_geometry_import_missing_binary_output_is_deterministic() {
    let exe = env!("CARGO_BIN_EXE_orchard-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let glb_path = temp_dir.path().join("asset.glb");
    fs::write(&glb_path, b"glTF dummy geometry-only payload").expect("write dummy glb");
    let missing_godot = temp_dir.path().join("missing-godot");
    let first_dir = temp_dir.path().join("proof-a");
    let second_dir = temp_dir.path().join("proof-b");

    for out_dir in [&first_dir, &second_dir] {
        assert!(
            Command::new(exe)
                .args(["godot-proof", "geometry-import", "--glb"])
                .arg(&glb_path)
                .args(["--out-dir"])
                .arg(out_dir)
                .args(["--godot-bin"])
                .arg(&missing_godot)
                .status()
                .expect("run godot proof")
                .success()
        );
    }

    let first =
        fs::read(first_dir.join("godot-import-proof-report.json")).expect("read first report");
    let second =
        fs::read(second_dir.join("godot-import-proof-report.json")).expect("read second report");
    assert_eq!(first, second);
}

fn read_json(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_godot_report_excludes_later_features(report: &serde_json::Value) {
    assert_eq!(report["mesh_imported"], false);
    assert_eq!(report["hierarchy_checked"], false);
    assert_eq!(report["imported_node_count"], 0);
    assert_eq!(report["imported_mesh_count"], 0);
    assert_eq!(report["relationship_hierarchy_preserved"], false);
    assert_eq!(report["material_imported"], false);
    assert_eq!(report["collision_imported"], false);
    assert_eq!(report["rig_imported"], false);
    assert_eq!(report["animation_imported"], false);
    assert_eq!(report["game_ready"], false);
    assert!(
        report["hierarchy_notes"]
            .as_array()
            .expect("hierarchy notes")
            .iter()
            .any(|note| note
                .as_str()
                .unwrap_or_default()
                .contains("hierarchy inspection"))
    );
}
