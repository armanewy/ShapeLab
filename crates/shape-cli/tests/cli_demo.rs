#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

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

    let verify_status = Command::new(exe)
        .arg("verify-decompile")
        .arg(&package)
        .status()
        .expect("run shape-cli verify-decompile");
    assert!(verify_status.success());
}
