use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use orchard_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, SurfaceRegionSpec, SurfaceRole, Transform3,
};
use orchard_compile::export::{
    ASSET_MANIFEST_FILE, CanonicalPartMesh, DCC_ADAPTER_MANIFEST_FILE, DCC_REBUILD_SCRIPT_FILE,
    DCC_VERIFICATION_FILE, DccAdapterOptions, DccVariantControl, ExportError, PROVENANCE_FILE,
    RECIPE_FILE, VALIDATION_FILE, encode_part_meshbin, export_counts, ordered_parts,
    read_model_package, read_part_meshbin, verify_model_package, write_grouped_obj_export,
    write_model_package, write_model_package_with_dcc_options,
};
use orchard_compile::{AssetArtifact, compile_asset};
use serde_json::Value;

fn multipart_recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(77), "Multipart Export Fixture");
    recipe.definitions.insert(
        PartDefinitionId(1),
        plate_definition(1, "base definition", [1.0, 1.0], 0.12),
    );
    recipe.definitions.insert(
        PartDefinitionId(2),
        plate_definition(2, "lid definition", [0.5, 0.75], 0.08),
    );
    recipe.instances.insert(
        PartInstanceId(1),
        PartInstance {
            id: PartInstanceId(1),
            definition: PartDefinitionId(1),
            name: "Base Plate".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.instances.insert(
        PartInstanceId(2),
        PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(2),
            name: "Lid Plate".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3 {
                translation: [1.75, 0.25, 0.0],
                ..Transform3::default()
            },
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.next_ids.part_definition = 3;
    recipe.next_ids.part_instance = 3;
    recipe.next_ids.region = 5;
    recipe
}

fn plate_definition(id: u64, name: &str, size: [f32; 2], thickness: f32) -> PartDefinition {
    PartDefinition {
        id: PartDefinitionId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::Plate { size, thickness },
            operations: Vec::new(),
        },
        regions: plate_regions(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn plate_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    [
        (RegionId(1), "front", SurfaceRole::PrimarySurface),
        (RegionId(2), "back", SurfaceRole::PrimarySurface),
        (RegionId(3), "side", SurfaceRole::Side),
        (RegionId(4), "bevel", SurfaceRole::BevelBand),
    ]
    .into_iter()
    .map(|(id, name, role)| {
        (
            id,
            SurfaceRegionSpec {
                id,
                name: name.to_owned(),
                role,
                tags: BTreeSet::new(),
            },
        )
    })
    .collect()
}

fn compiled_fixture() -> (AssetRecipe, AssetArtifact) {
    let recipe = multipart_recipe();
    let artifact = compile_asset(&recipe).expect("fixture should compile");
    assert!(artifact.validation_report.is_valid());
    assert_eq!(artifact.compiled_parts.len(), 2);
    (recipe, artifact)
}

#[test]
fn multipart_obj_is_grouped_deterministic_and_material_free() {
    let (recipe, artifact) = compiled_fixture();

    let first = write_grouped_obj_export(&artifact, Some(&recipe)).expect("obj export");
    let second = write_grouped_obj_export(&artifact, Some(&recipe)).expect("obj export");

    assert_eq!(first.obj, second.obj);
    assert_eq!(first.report.object_count, 2);
    assert_eq!(line_count(&first.obj, "o "), 2);
    assert_eq!(line_count(&first.obj, "g "), 2);
    assert!(first.obj.contains("o part_001_base_plate"));
    assert!(first.obj.contains("o part_002_lid_plate"));
    assert!(first.obj.contains("vn "));
    assert!(first.obj.contains("# region 1 front"));
    assert!(first.obj.contains("# total_counts objects=2"));
    assert!(!first.obj.contains("mtllib"));
    assert!(!first.obj.contains("usemtl"));
    assert_eq!(first.report.face_count, line_count(&first.obj, "f ") as u64);
    assert!(
        first
            .provenance_json
            .contains("part_region_operation_mappings")
    );
}

#[test]
fn model_package_bytes_are_deterministic() {
    let (recipe, artifact) = compiled_fixture();
    let first_dir = temp_dir("deterministic-a");
    let second_dir = temp_dir("deterministic-b");

    write_model_package(&recipe, &artifact, &first_dir).expect("first package");
    write_model_package(&recipe, &artifact, &second_dir).expect("second package");

    assert_eq!(collect_files(&first_dir), collect_files(&second_dir));
}

#[test]
fn binary_package_round_trips_canonical_part_meshes() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("round-trip");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");

    let package = read_model_package(&package_dir).expect("package should verify");
    let verification = verify_model_package(&package_dir).expect("verification");

    assert_eq!(package.manifest.counts, export_counts(&artifact));
    assert_eq!(verification.counts, export_counts(&artifact));
    assert!(verification.checksums_match);
    assert!(verification.topology_matches_manifest);
    assert!(verification.finite_numeric_payloads);
    assert_eq!(package.parts.len(), 2);

    for (expected_part, decoded_part) in ordered_parts(&artifact).into_iter().zip(&package.parts) {
        let expected =
            CanonicalPartMesh::from_compiled_part(expected_part).expect("canonical part");
        assert_eq!(
            encode_part_meshbin(&expected).expect("expected bytes"),
            encode_part_meshbin(decoded_part).expect("decoded bytes")
        );
    }

    let direct_part = read_part_meshbin(&paths.parts[0]).expect("direct part read");
    assert_eq!(
        encode_part_meshbin(&direct_part).expect("direct bytes"),
        encode_part_meshbin(&package.parts[0]).expect("package bytes")
    );
}

#[test]
fn region_provenance_and_validation_sidecars_are_preserved() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("sidecars");

    write_model_package(&recipe, &artifact, &package_dir).expect("package");
    let package = read_model_package(&package_dir).expect("package should verify");

    assert_eq!(package.recipe.title, recipe.title);
    assert_eq!(
        package.validation.source_recipe_hash,
        artifact.source_recipe_hash
    );
    assert_eq!(package.validation.counts, export_counts(&artifact));
    assert!(package.validation.compile_issues.is_empty());
    assert!(package.validation.model_issues.is_empty());
    assert!(
        package
            .provenance
            .part_region_operation_mappings
            .iter()
            .any(|mapping| mapping.region == Some(RegionId(1)))
    );
    assert!(package.manifest.parts.iter().any(|part| {
        part.regions
            .iter()
            .any(|region| region.id == 1 && region.name == "front")
    }));
    assert!(
        package
            .manifest
            .parts
            .iter()
            .any(|part| { part.parent_instance_id == Some(1) && part.instance_id == 2 })
    );
    assert!(package_dir.join(RECIPE_FILE).exists());
    assert!(package_dir.join(PROVENANCE_FILE).exists());
    assert!(package_dir.join(VALIDATION_FILE).exists());
    assert!(package_dir.join(DCC_ADAPTER_MANIFEST_FILE).exists());
    assert!(package_dir.join(DCC_REBUILD_SCRIPT_FILE).exists());
    assert!(package_dir.join(DCC_VERIFICATION_FILE).exists());
}

#[test]
fn dcc_adapter_sidecars_project_package_outward_not_source_of_truth() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("dcc-sidecars");

    write_model_package(&recipe, &artifact, &package_dir).expect("package");
    let package = read_model_package(&package_dir).expect("package should verify");

    assert_eq!(
        package.dcc_adapter.source_recipe_hash,
        artifact.source_recipe_hash
    );
    assert!(
        !package
            .dcc_adapter
            .source_of_truth
            .dcc_scene_is_source_of_truth
    );
    assert!(
        !package
            .dcc_adapter
            .source_of_truth
            .external_scene_import_supported
    );
    assert_eq!(
        package.dcc_adapter.files.asset_manifest,
        ASSET_MANIFEST_FILE
    );
    assert_eq!(package.dcc_adapter.semantic_parts.len(), 2);
    assert!(package.dcc_adapter.semantic_parts.iter().any(|part| {
        part.part_id == "part-001"
            && part.object_name == "part_001_base_plate"
            && part
                .metadata
                .iter()
                .any(|field| field.key == "shape_lab_instance_id" && field.value == "1")
    }));
    assert!(package.dcc_adapter.collections.iter().any(|collection| {
        collection.id == "shape_lab_asset"
            && collection.part_ids == vec!["part-001".to_owned(), "part-002".to_owned()]
    }));
    assert!(package.dcc_verification.canonical_package_verified);
    assert!(!package.dcc_verification.dcc_scene_is_source_of_truth);
    assert!(!package.dcc_verification.external_scene_import_supported);
    assert_eq!(package.dcc_verification.semantic_part_count, 2);
}

#[test]
fn legacy_schema_one_package_without_dcc_sidecars_still_reads() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("legacy-no-dcc");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");
    fs::remove_file(&paths.dcc_adapter).expect("remove dcc adapter");
    fs::remove_file(&paths.dcc_rebuild).expect("remove dcc rebuild");
    fs::remove_file(&paths.dcc_verification).expect("remove dcc verification");
    strip_dcc_manifest_file_fields(&paths.manifest);

    let package = read_model_package(&package_dir).expect("legacy package should verify");

    assert_eq!(
        package.dcc_adapter.source_recipe_hash,
        artifact.source_recipe_hash
    );
    assert_eq!(package.dcc_adapter.semantic_parts.len(), 2);
    assert!(package.dcc_adapter.variant_controls.is_empty());
    assert!(package.dcc_verification.canonical_package_verified);
}

#[test]
fn declared_dcc_sidecars_are_required_and_bound_to_manifest() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("dcc-integrity");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");

    fs::remove_file(&paths.dcc_adapter).expect("remove declared dcc adapter");
    assert!(read_model_package(&package_dir).is_err());

    write_model_package(&recipe, &artifact, &package_dir).expect("rewrite package");
    let mut adapter: Value =
        serde_json::from_slice(&fs::read(&paths.dcc_adapter).expect("read dcc adapter"))
            .expect("adapter json");
    adapter["semantic_parts"][0]["object_name"] = Value::String("tampered".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");
    let error = read_model_package(&package_dir).expect_err("tampered adapter should fail");
    match error {
        ExportError::InvalidPackage { message, .. } => {
            assert!(message.contains("DCC adapter projection"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn dcc_adapter_file_targets_must_remain_canonical() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("dcc-file-targets");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");
    let mut adapter: Value =
        serde_json::from_slice(&fs::read(&paths.dcc_adapter).expect("read dcc adapter"))
            .expect("adapter json");
    adapter["files"]["asset_manifest"] = Value::String("alternate-manifest.json".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");

    let error = read_model_package(&package_dir).expect_err("redirected adapter should fail");
    match error {
        ExportError::InvalidPackage { message, .. } => {
            assert!(message.contains("canonical package sidecar paths"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn dcc_variant_controls_are_metadata_only() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("dcc-controls");
    let options = DccAdapterOptions {
        variant_controls: vec![DccVariantControl {
            id: "span_length".to_owned(),
            label: "Span Length".to_owned(),
            value: "wide".to_owned(),
            locked: true,
        }],
    };

    let paths = write_model_package_with_dcc_options(&recipe, &artifact, &package_dir, &options)
        .expect("package");
    let package = read_model_package(&package_dir).expect("package should verify");

    assert_eq!(
        package.dcc_adapter.variant_controls,
        options.variant_controls
    );
    assert_eq!(package.dcc_verification.variant_control_count, 1);
    assert_eq!(package.recipe, recipe);

    let mut adapter: Value =
        serde_json::from_slice(&fs::read(&paths.dcc_adapter).expect("read dcc adapter"))
            .expect("adapter json");
    adapter["variant_controls"][0]["label"] = Value::String("Tampered Label".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");

    let error = read_model_package(&package_dir).expect_err("tampered controls should fail");
    match error {
        ExportError::InvalidPackage { message, .. } => {
            assert!(message.contains("DCC verification report"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn dcc_rebuild_script_has_valid_syntax_and_emits_projection_report() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("dcc-rebuild");
    let out_dir = temp_dir("dcc-rebuild-out");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");
    let python = python_command();
    let original_adapter = fs::read(&paths.dcc_adapter).expect("read original dcc adapter");

    let compile = Command::new(&python.0)
        .args(&python.1)
        .arg("-m")
        .arg("py_compile")
        .arg(&paths.dcc_rebuild)
        .output()
        .expect("run python py_compile");
    assert!(
        compile.status.success(),
        "py_compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&paths.dcc_rebuild)
        .arg("--package-dir")
        .arg(&package_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run dcc rebuild script");
    assert!(
        output.status.success(),
        "script failed stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"semantic_parts\": 2"));
    let report =
        fs::read_to_string(out_dir.join("dcc_projection_report.json")).expect("projection report");
    assert!(report.contains("\"dcc_scene_is_source_of_truth\": false"));
    assert!(report.contains("\"external_scene_import_supported\": false"));

    let mut adapter: Value = serde_json::from_slice(&original_adapter).expect("adapter json");
    adapter["semantic_parts"][0]["object_name"] = Value::String("tampered".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");
    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&paths.dcc_rebuild)
        .arg("--package-dir")
        .arg(&package_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run tampered dcc rebuild script");
    assert!(
        !output.status.success(),
        "semantic tamper unexpectedly passed stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&paths.dcc_adapter, &original_adapter).expect("restore dcc adapter");
    let mut adapter: Value = serde_json::from_slice(&original_adapter).expect("adapter json");
    adapter["collections"][0]["part_ids"][0] = Value::String("part-999".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");
    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&paths.dcc_rebuild)
        .arg("--package-dir")
        .arg(&package_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run tampered dcc rebuild script");
    assert!(
        !output.status.success(),
        "collection tamper unexpectedly passed stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(&paths.dcc_adapter, &original_adapter).expect("restore dcc adapter");
    let mut adapter: Value = serde_json::from_slice(&original_adapter).expect("adapter json");
    adapter["files"]["asset_manifest"] = Value::String("alternate-manifest.json".to_owned());
    fs::write(
        &paths.dcc_adapter,
        serde_json::to_string_pretty(&adapter).expect("adapter encode"),
    )
    .expect("write tampered adapter");
    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&paths.dcc_rebuild)
        .arg("--package-dir")
        .arg(&package_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run tampered dcc rebuild script");
    assert!(
        !output.status.success(),
        "tampered script unexpectedly passed stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupted_part_payload_is_rejected() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("corrupt");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");

    fs::write(&paths.parts[0], b"not a meshbin").expect("corrupt meshbin");

    assert!(matches!(
        read_model_package(&package_dir),
        Err(ExportError::InvalidPackage { .. })
    ));
}

#[test]
fn unsafe_package_paths_are_rejected() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("unsafe");
    write_model_package(&recipe, &artifact, &package_dir).expect("package");

    let manifest_path = package_dir.join(ASSET_MANIFEST_FILE);
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
            .expect("manifest json");
    manifest["parts"][0]["mesh"] = Value::String("../escape.meshbin".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest encode"),
    )
    .expect("write manifest");

    let error = read_model_package(&package_dir).expect_err("unsafe path should fail");
    match error {
        ExportError::InvalidPackage { message, .. } => {
            assert!(message.contains("unsafe package-relative path"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn blender_script_has_valid_syntax_and_runs_with_stub_bpy() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("blender");
    let out_dir = temp_dir("blender-out");
    let paths = write_model_package(&recipe, &artifact, &package_dir).expect("package");
    let python = python_command();

    let compile = Command::new(&python.0)
        .args(&python.1)
        .arg("-m")
        .arg("py_compile")
        .arg(&paths.blender_reconstruct)
        .output()
        .expect("run python py_compile");
    assert!(
        compile.status.success(),
        "py_compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let output = Command::new(&python.0)
        .args(&python.1)
        .arg(&paths.blender_reconstruct)
        .arg("--package-dir")
        .arg(&package_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--verify-reopen")
        .output()
        .expect("run blender reconstruct script");
    assert!(
        output.status.success(),
        "script failed stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"objects\": 2"));
    assert!(out_dir.join("reconstructed.blend").exists());
}

#[test]
fn schema_two_decompiler_package_names_are_not_written() {
    let (recipe, artifact) = compiled_fixture();
    let package_dir = temp_dir("schema-two-isolation");

    write_model_package(&recipe, &artifact, &package_dir).expect("package");

    assert!(package_dir.join(ASSET_MANIFEST_FILE).exists());
    assert!(!package_dir.join("manifest.json").exists());
    assert!(!package_dir.join("source.meshbin").exists());
    assert!(!package_dir.join("target.meshbin").exists());
    assert!(!package_dir.join("operators").exists());
    assert!(!package_dir.join("residual").exists());
}

fn line_count(text: &str, prefix: &str) -> usize {
    text.lines().filter(|line| line.starts_with(prefix)).count()
}

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "shape-lab-model-export-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn collect_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    collect_files_inner(root, root, &mut files);
    files
}

fn collect_files_inner(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    let mut entries = fs::read_dir(current)
        .expect("read dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files_inner(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("relative")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, fs::read(path).expect("read file"));
        }
    }
}

fn strip_dcc_manifest_file_fields(manifest_path: &Path) {
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(manifest_path).expect("read manifest"))
            .expect("manifest json");
    let files = manifest["files"]
        .as_object_mut()
        .expect("manifest files object");
    files.remove("dcc_adapter");
    files.remove("dcc_rebuild");
    files.remove("dcc_verification");
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest encode"),
    )
    .expect("write legacy manifest");
}

fn python_command() -> (String, Vec<String>) {
    if let Some(value) = std::env::var_os("PYTHON") {
        let command = value.to_string_lossy().to_string();
        if command_works(&command, &[]) {
            return (command, Vec::new());
        }
    }
    for command in ["python", "python3"] {
        if command_works(command, &[]) {
            return (command.to_owned(), Vec::new());
        }
    }
    if command_works("py", &["-3"]) {
        return ("py".to_owned(), vec!["-3".to_owned()]);
    }
    panic!("Python is required for Blender script syntax tests");
}

fn command_works(command: &str, prefix_args: &[&str]) -> bool {
    Command::new(command)
        .args(prefix_args)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}
