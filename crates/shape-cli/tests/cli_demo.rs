#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

use shape_decompiler::v3::bend::{BendParameters, evaluate_bend};

#[test]
fn demo_generates_headless_artifacts() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("demo");

    let status = Command::new(exe)
        .args([
            "demo",
            "--preset",
            "desk-lamp",
            "--seed",
            "7",
            "--mode",
            "explore",
            "--proposal-count",
            "12",
            "--result-count",
            "3",
            "--descriptor-resolution",
            "6",
            "--mesh-resolution",
            "12",
            "--out-dir",
        ])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli demo");

    assert!(status.success());
    for name in [
        "project-before.json",
        "current.obj",
        "current.png",
        "candidate-00.obj",
        "candidate-00.png",
        "contact-sheet.png",
        "project-after.json",
        "accepted.obj",
        "accepted.png",
        "summary.json",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let validate_status = Command::new(exe)
        .arg("validate")
        .arg(out_dir.join("project-after.json"))
        .status()
        .expect("run shape-cli validate");
    assert!(validate_status.success());
}

#[test]
fn model_demo_generates_explicit_asset_artifacts() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("crate");

    let status = Command::new(exe)
        .args(["model-demo", "--asset", "industrial-crate", "--out-dir"])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli model-demo");

    assert!(status.success());
    for name in [
        "recipe.json",
        "asset.obj",
        "provenance.json",
        "validation.json",
        "statistics.json",
        "preview.png",
        "blender_reconstruct.py",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let validation: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("validation.json")).unwrap())
            .unwrap();
    assert_eq!(validation["issues"].as_array().unwrap().len(), 0);
    let statistics: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("statistics.json")).unwrap())
            .unwrap();
    assert!(statistics["triangle_count"].as_u64().unwrap() < 25_000);
    assert_eq!(statistics["used_sdf_or_remeshing"], false);
}

#[test]
fn inspect_and_compile_asset_use_canonical_model_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let out_dir = temp_dir.path().join("lamp");

    let inspect = Command::new(exe)
        .args(["inspect-asset", "explicit-desk-lamp"])
        .output()
        .expect("run shape-cli inspect-asset");
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(stdout.contains("Part tree:"));
    assert!(stdout.contains("Construction timeline:"));
    assert!(stdout.contains("model validation: valid"));
    assert!(stdout.contains("accidental intersections: 0"));

    let status = Command::new(exe)
        .args(["compile-asset", "explicit-desk-lamp", "--out-dir"])
        .arg(&out_dir)
        .status()
        .expect("run shape-cli compile-asset");
    assert!(status.success());
    for name in [
        "asset-manifest.json",
        "recipe.json",
        "provenance.json",
        "validation.json",
        "blender_reconstruct.py",
        "asset.obj",
        "grouped-obj-report.json",
        "statistics.json",
        "model-validation.json",
        "construction-timeline.json",
        "package-verification.json",
        "preview.png",
    ] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }
    assert!(out_dir.join("parts").join("part-001.meshbin").exists());

    let timeline: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("construction-timeline.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(timeline["stages"].as_array().unwrap().len(), 7);
    let model_validation: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("model-validation.json")).unwrap())
            .unwrap();
    assert_eq!(
        model_validation["metrics"]["accidental_intersection_count"],
        0
    );
}

#[test]
fn decompile_generates_lossless_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("package");
    fs::write(
        &source,
        "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
v 0.5 0.5 1
f 1 2 3
f 1 3 4
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
",
    )
    .expect("write source obj");
    fs::write(
        &target,
        "\
v 0.25 0 0
v 1.25 0 0
v 1.25 1 0
v 0.25 1 0
v 0.5 0.5 1.25
f 1 2 3
f 1 3 4
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
",
    )
    .expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli decompile");

    assert!(status.success());
    for name in [
        "manifest.json",
        "verification.json",
        "package-verification.json",
        "inference-diagnostics.json",
        "source.meshbin",
        "target.meshbin",
        "blender_reconstruct.py",
    ] {
        let path = package.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }
    assert!(package.join("residual").join("indices.u32").exists());
    assert!(package.join("residual").join("positions.f32").exists());

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 3);
    assert!(diagnostics["program_hypotheses"].is_array());
    assert!(diagnostics.get("hypotheses").is_none());
    assert!(diagnostics.get("selected_hypothesis_index").is_none());

    let verification: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("verification.json")).unwrap())
            .unwrap();
    assert_eq!(verification["topology_exact"], true);
    assert_eq!(verification["max_euclidean_error"], 0.0);

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(
        manifest["numeric_format"]["affine_evaluation"],
        "float32_stepwise_no_fma"
    );
    let operators = manifest["operators"].as_array().unwrap();
    assert_eq!(operators.len(), 2);
    for operator in operators {
        if let Some(baked_positions_file) = operator["baked_positions_file"].as_str() {
            let path = package.join(baked_positions_file);
            assert!(path.exists(), "{baked_positions_file} should exist");
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "{baked_positions_file} is empty"
            );
        }
    }
    let lossless_stage = package
        .join("operators")
        .join("0001-lossless-correction-positions.f32");
    assert!(lossless_stage.exists());
    assert!(lossless_stage.metadata().expect("metadata").len() > 0);

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile");
    assert!(verify_status.success());
}

#[test]
fn decompile_generates_schema_three_residual_only_package() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("package-v3");
    let source_obj = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3
f 1 3 4
";
    fs::write(&source, source_obj).expect("write source obj");
    fs::write(&target, source_obj).expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--package-schema")
        .arg("3")
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli schema-3 decompile");

    assert!(status.success());
    for name in [
        "manifest.json",
        "package-verification.json",
        "inference-diagnostics.json",
        "source.meshbin",
        "target.meshbin",
        "blender_reconstruct.py",
    ] {
        let path = package.join(name);
        assert!(path.exists(), "{name} should exist");
        assert!(
            path.metadata().expect("metadata").len() > 0,
            "{name} is empty"
        );
    }

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 3);
    let operators = manifest["operators"].as_array().unwrap();
    assert_eq!(operators.len(), 1);
    assert_eq!(operators[0]["kind"], "lossless_correction");
    assert_eq!(
        operators[0]["stage"]["baked_positions_file"],
        "operators/0000-lossless-correction-positions.f32"
    );

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 4);
    assert_eq!(diagnostics["package_schema_version"], 3);
    assert!(
        diagnostics["program_hypotheses"][0]["operators"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile for schema 3");
    assert!(verify_status.success());
}

#[test]
fn enable_bend_requires_schema_three() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
    fs::write(&source, obj).expect("write source obj");
    fs::write(&target, obj).expect("write target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--enable-bend")
        .arg("--out-dir")
        .arg(temp_dir.path().join("package"))
        .status()
        .expect("run shape-cli invalid bend decompile");

    assert!(!status.success());
}

#[test]
fn decompile_schema_three_enable_bend_writes_bend_program() {
    let exe = env!("CARGO_BIN_EXE_shape-cli");
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let source = temp_dir.path().join("source.obj");
    let target = temp_dir.path().join("target.obj");
    let package = temp_dir.path().join("bend-package");
    let (source_obj, target_obj) = bend_pair_objs();
    fs::write(&source, source_obj).expect("write bend source obj");
    fs::write(&target, target_obj).expect("write bend target obj");

    let status = Command::new(exe)
        .arg("decompile")
        .arg(&source)
        .arg(&target)
        .arg("--package-schema")
        .arg("3")
        .arg("--enable-bend")
        .arg("--verbose")
        .arg("--out-dir")
        .arg(&package)
        .status()
        .expect("run shape-cli bend decompile");

    assert!(status.success());
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(package.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 3);
    assert!(
        manifest["operators"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operator| operator["kind"] == "bend")
    );

    let diagnostics: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(package.join("inference-diagnostics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(diagnostics["diagnostics_schema_version"], 4);
    assert!(diagnostics["program_hypotheses"].as_array().unwrap().len() > 1);
    assert!(diagnostics["timing_by_phase_ms"]["program_scoring_ms"].is_number());
    let selected = diagnostics["selected_program_hypothesis_index"]
        .as_u64()
        .unwrap() as usize;
    let selected_hypothesis = &diagnostics["program_hypotheses"].as_array().unwrap()[selected];
    assert!(
        selected_hypothesis["operators"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operator| operator["family"] == "bend")
    );

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile for bend package");
    assert!(verify_status.success());
}

fn bend_pair_objs() -> (String, String) {
    let mut source_positions = Vec::new();
    for index in 0..=10 {
        let x = index as f32 / 10.0;
        source_positions.extend([
            [x, -0.10, -0.05],
            [x, 0.10, -0.05],
            [x, 0.10, 0.05],
            [x, -0.10, 0.05],
        ]);
    }
    let target_positions = evaluate_bend(
        &BendParameters {
            origin: [0.5, 0.0, 0.0],
            longitudinal_axis: [1.0, 0.0, 0.0],
            bend_direction: [0.0, 1.0, 0.0],
            angle_radians: 45.0_f32.to_radians(),
            interval_start: -0.5,
            interval_end: 0.5,
        },
        &source_positions,
    )
    .unwrap();
    let faces = beam_faces(11);
    (
        obj_from_positions(&source_positions, &faces),
        obj_from_positions(&target_positions, &faces),
    )
}

fn beam_faces(station_count: usize) -> Vec<[usize; 3]> {
    let mut faces = Vec::new();
    for ring in 0..station_count - 1 {
        let current = ring * 4 + 1;
        let next = current + 4;
        for corner in 0..4 {
            let a = current + corner;
            let b = current + (corner + 1) % 4;
            let c = next + (corner + 1) % 4;
            let d = next + corner;
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }
    let last = (station_count - 1) * 4 + 1;
    faces.push([1, 2, 3]);
    faces.push([1, 3, 4]);
    faces.push([last, last + 2, last + 1]);
    faces.push([last, last + 3, last + 2]);
    faces
}

fn obj_from_positions(positions: &[[f32; 3]], faces: &[[usize; 3]]) -> String {
    let mut obj = String::new();
    for position in positions {
        obj.push_str(&format!(
            "v {:.9} {:.9} {:.9}\n",
            position[0], position[1], position[2]
        ));
    }
    for [a, b, c] in faces {
        obj.push_str(&format!("f {a} {b} {c}\n"));
    }
    obj
}
