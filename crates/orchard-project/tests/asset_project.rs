use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use orchard_asset::{
    AssetEdit, AssetEditProgram, AssetId, AssetRecipe, Frame3, GeneratorDimensionEdit,
    GeometryRecipe, GeometrySource, ParameterDescriptor, ParameterId, PartDefinition,
    PartDefinitionId, PartInstance, PartInstanceId, RegionId, RevisionId, SurfaceRegionSpec,
    SurfaceRole, Transform3, definition_scalar_path,
};
use orchard_project::asset::{
    ASSET_PROJECT_KIND, ASSET_PROJECT_SCHEMA_VERSION, AssetProject, AssetProjectError,
    AssetProjectFile, autosave_snapshot_path, ensure_asset_project_path,
};

fn tempdir(name: &str) -> TestTempDir {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("object-orchard-asset-project-{name}-{nonce}"));
    fs::create_dir(&path).unwrap();
    TestTempDir { path }
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn recipe() -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(77), "Asset Project Fixture");
    recipe.definitions.insert(
        PartDefinitionId(1),
        plate_definition(1, "body definition", [1.0, 1.0], 0.12),
    );
    recipe.instances.insert(
        PartInstanceId(1),
        PartInstance {
            id: PartInstanceId(1),
            definition: PartDefinitionId(1),
            name: "Body Plate".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        },
    );
    recipe.root_instances.push(PartInstanceId(1));
    recipe.parameters.insert(
        ParameterId(1),
        ParameterDescriptor {
            id: ParameterId(1),
            path: definition_scalar_path(PartDefinitionId(1), "geometry.plate.thickness"),
            label: "Thickness".to_owned(),
            group: "Form".to_owned(),
            minimum: 0.01,
            maximum: 1.0,
            step: 0.01,
            mutation_sigma: 0.05,
            topology_changing: false,
            beginner_description: "Plate thickness".to_owned(),
        },
    );
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.parameter = 2;
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

fn thickness_edit(label: &str, thickness: f32) -> AssetEditProgram {
    AssetEditProgram {
        label: label.to_owned(),
        seed: 11,
        operations: vec![AssetEdit::SetGeneratorDimension {
            definition: PartDefinitionId(1),
            dimension: GeneratorDimensionEdit::PlateThickness(thickness),
        }],
    }
}

#[test]
fn new_project_from_template_stores_complete_root_revision() {
    let project = AssetProject::from_template(recipe()).unwrap();

    assert_eq!(project.project_kind, ASSET_PROJECT_KIND);
    assert_eq!(project.schema_version, ASSET_PROJECT_SCHEMA_VERSION);
    assert_eq!(project.current_revision, RevisionId(0));
    assert_eq!(project.next_revision_id, 1);
    assert!(!project.can_undo());
    assert_eq!(
        project.current().unwrap().recipe.title,
        "Asset Project Fixture"
    );
    assert!(project.current().unwrap().compiled_artifact_hash != 0);
    assert!(project.current().unwrap().validation.recipe_valid);
    assert!(
        project
            .current()
            .unwrap()
            .validation
            .compiled_artifact_valid
    );
}

#[test]
fn branching_after_undo_preserves_sibling_revisions() {
    let mut project = AssetProject::from_template(recipe()).unwrap();
    let first = project
        .accept_candidate(thickness_edit("thin", 0.2))
        .unwrap();

    project.undo().unwrap();
    let second = project
        .accept_candidate(thickness_edit("thick", 0.3))
        .unwrap();

    assert_eq!(first, RevisionId(1));
    assert_eq!(second, RevisionId(2));
    assert_eq!(
        project.children_of(RevisionId(0)),
        vec![RevisionId(1), RevisionId(2)]
    );
    assert_eq!(project.next_revision_id, 3);
    project.switch_to(first).unwrap();
    assert_eq!(
        project.revision_path_to_root().unwrap(),
        vec![RevisionId(1), RevisionId(0)]
    );
    project.switch_to(second).unwrap();
    assert_eq!(project.current().unwrap().label, "thick");
}

#[test]
fn save_load_preserves_revision_graph_and_current_revision() {
    let temp = tempdir("roundtrip");
    let path = temp.path().join("fixture.object-orchard-asset.json");
    let mut project = AssetProject::from_template(recipe()).unwrap();
    project
        .accept_candidate(thickness_edit("thin", 0.2))
        .unwrap();
    project.undo().unwrap();
    let current = project
        .accept_candidate(thickness_edit("thick", 0.3))
        .unwrap();

    project.save_json(&path).unwrap();
    let loaded = AssetProject::load_json(&path).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.current_revision, current);
    assert_eq!(
        loaded.children_of(RevisionId(0)),
        vec![RevisionId(1), RevisionId(2)]
    );
}

#[test]
fn malformed_legacy_and_future_project_files_are_rejected() {
    let temp = tempdir("bad-files");
    let malformed = temp.path().join("malformed.object-orchard-asset.json");
    fs::write(&malformed, b"{not valid json").unwrap();
    assert!(matches!(
        AssetProject::load_json(&malformed),
        Err(AssetProjectError::JsonAtPath { .. })
    ));

    let legacy = temp.path().join("legacy.object-orchard-asset.json");
    fs::write(
        &legacy,
        br#"{"schema_version":1,"title":"legacy","current_revision":0,"next_revision_id":1,"revisions":{}}"#,
    )
    .unwrap();
    assert!(matches!(
        AssetProject::load_json(&legacy),
        Err(AssetProjectError::UnsupportedProjectKind { .. })
    ));

    let future = temp.path().join("future.object-orchard-asset.json");
    fs::write(
        &future,
        br#"{"project_kind":"object-orchard.asset-project","schema_version":2,"future_payload":true}"#,
    )
    .unwrap();
    assert!(matches!(
        AssetProject::load_json(&future),
        Err(AssetProjectError::FutureSchemaVersion {
            found: 2,
            supported: 1
        })
    ));
}

#[test]
fn save_as_validates_asset_project_suffix_and_updates_dirty_marker() {
    let temp = tempdir("dirty");
    let bad_path = temp.path().join("fixture.object-orchard.json");
    let good_path = temp.path().join("fixture.object-orchard-asset.json");
    let mut file = AssetProjectFile::new_from_template("Fixture", recipe()).unwrap();

    assert!(file.is_dirty());
    assert!(matches!(
        file.save_as(&bad_path),
        Err(AssetProjectError::InvalidProjectPath { .. })
    ));
    file.save_as(&good_path).unwrap();
    assert!(!file.is_dirty());
    file.accept_candidate(thickness_edit("thin", 0.2)).unwrap();
    assert!(file.is_dirty());
    file.save().unwrap();
    assert!(!file.is_dirty());
    assert_eq!(file.path.as_deref(), Some(good_path.as_path()));
}

#[test]
fn atomic_save_failure_preserves_existing_project_file() {
    let temp = tempdir("atomic");
    let path = temp.path().join("fixture.object-orchard-asset.json");
    let original = AssetProject::from_template(recipe()).unwrap();
    original.save_json(&path).unwrap();
    let original_bytes = fs::read(&path).unwrap();

    let error = orchard_project::asset::test_support::atomic_replace_for_test(&path, |file| {
        file.write_all(b"{\"project_kind\":\"object-orchard.asset-project\"")?;
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "simulated interrupted write",
        ))
    })
    .unwrap_err();

    assert!(matches!(error, AssetProjectError::PathIo { .. }));
    assert_eq!(fs::read(&path).unwrap(), original_bytes);
    assert_eq!(AssetProject::load_json(&path).unwrap(), original);
}

#[test]
fn autosave_recovery_snapshot_is_loadable_and_separate_from_project_path() {
    let temp = tempdir("autosave");
    let path = temp.path().join("fixture.object-orchard-asset.json");
    let autosave = autosave_snapshot_path(&path);
    let mut file = AssetProjectFile::new_from_template("Fixture", recipe()).unwrap();
    file.accept_candidate(thickness_edit("thin", 0.2)).unwrap();

    let snapshot = file.save_autosave_snapshot(&autosave).unwrap();
    let recovered = AssetProject::load_json(&snapshot.path).unwrap();

    assert_ne!(snapshot.path, path);
    assert_eq!(snapshot.marker, file.project.persistence_marker());
    assert_eq!(recovered, file.project);
}

#[test]
fn export_current_obj_failure_does_not_clear_dirty_state() {
    let temp = tempdir("export-failure");
    let project_path = temp.path().join("fixture.object-orchard-asset.json");
    let obj_path = temp.path().join("missing").join("out.obj");
    let mut file = AssetProjectFile::new_from_template("Fixture", recipe()).unwrap();

    file.save_as(&project_path).unwrap();
    file.accept_candidate(thickness_edit("thin", 0.2)).unwrap();
    assert!(file.is_dirty());

    let error = file.export_current_obj(&obj_path).unwrap_err();

    assert!(matches!(error, AssetProjectError::PathIo { .. }));
    assert!(file.is_dirty());
}

#[test]
fn export_current_model_package_and_obj_write_current_revision() {
    let temp = tempdir("export");
    let mut file = AssetProjectFile::new_from_template("Fixture", recipe()).unwrap();
    file.accept_candidate(thickness_edit("thin", 0.2)).unwrap();

    let package = file
        .export_current_model_package(temp.path().join("package"))
        .unwrap();
    let report = file
        .export_current_obj(temp.path().join("out.obj"))
        .unwrap();

    assert!(package.manifest.exists());
    assert!(package.recipe.exists());
    assert!(temp.path().join("out.obj").exists());
    assert_eq!(report.object_count, 1);
    assert!(file.is_dirty());
}

#[test]
fn current_revision_switch_is_dirty_and_survives_reload() {
    let temp = tempdir("switch");
    let path = temp.path().join("fixture.object-orchard-asset.json");
    let mut file = AssetProjectFile::new_from_template("Fixture", recipe()).unwrap();
    let first = file.accept_candidate(thickness_edit("thin", 0.2)).unwrap();
    file.undo().unwrap();
    let second = file.accept_candidate(thickness_edit("thick", 0.3)).unwrap();
    file.save_as(&path).unwrap();

    file.switch_to(first).unwrap();
    assert!(file.is_dirty());
    file.save().unwrap();
    let loaded = AssetProjectFile::load(&path).unwrap();

    assert_eq!(second, RevisionId(2));
    assert_eq!(loaded.project.current_revision, first);
    assert!(!loaded.is_dirty());
}

#[test]
fn asset_project_suffix_helper_rejects_legacy_project_names() {
    assert!(ensure_asset_project_path("asset.object-orchard-asset.json").is_ok());
    assert!(matches!(
        ensure_asset_project_path("legacy.object-orchard.json"),
        Err(AssetProjectError::InvalidProjectPath { .. })
    ));
}
