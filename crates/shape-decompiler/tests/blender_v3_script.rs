use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use shape_decompiler::v3::blender::{BlenderAdapterOptions, blender_reconstruction_script_v3};
use tempfile::TempDir;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

const BPY_STUB: &str = r#"
class AttributeValue:
    def __init__(self):
        self.value = 0


class Attribute:
    def __init__(self, size):
        self.data = [AttributeValue() for _ in range(size)]


class Attributes(dict):
    def __init__(self, mesh):
        super().__init__()
        self.mesh = mesh

    def new(self, name, type, domain):
        attribute = Attribute(len(self.mesh.vertices))
        self[name] = attribute
        return attribute


class Vertex:
    def __init__(self, co):
        self.co = tuple(co)


class Polygon:
    def __init__(self, vertices):
        self.vertices = tuple(vertices)


class Mesh:
    def __init__(self, name):
        self.name = name
        self.vertices = []
        self.polygons = []
        self.attributes = Attributes(self)
        self.shape_keys = None

    def from_pydata(self, positions, edges, faces):
        self.vertices = [Vertex(position) for position in positions]
        self.polygons = [Polygon(face) for face in faces]
        self.attributes = Attributes(self)

    def update(self):
        pass


class ShapePoint:
    def __init__(self, co):
        self.co = tuple(co)


class KeyBlock:
    def __init__(self, name, positions):
        self.name = name
        self.data = [ShapePoint(position) for position in positions]
        self.value = 0.0


class ShapeKeys:
    def __init__(self):
        self.key_blocks = []


class Object(dict):
    def __init__(self, name, mesh):
        super().__init__()
        self.name = name
        self.data = mesh
        self.active_shape_key_index = 0
        self.selected = False

    def select_set(self, value):
        self.selected = bool(value)

    def shape_key_add(self, name):
        if self.data.shape_keys is None:
            self.data.shape_keys = ShapeKeys()
        key = KeyBlock(name, [vertex.co for vertex in self.data.vertices])
        self.data.shape_keys.key_blocks.append(key)
        return key


class MeshCollection:
    def new(self, name):
        return Mesh(name)


class ObjectCollection:
    def __init__(self):
        self._objects = {}

    def new(self, name, mesh):
        return Object(name, mesh)

    def get(self, name):
        return self._objects.get(name)

    def remove(self, obj, do_unlink=True):
        self._objects.pop(obj.name, None)


class CollectionObjects:
    def link(self, obj):
        data.objects._objects[obj.name] = obj


class Collection:
    def __init__(self):
        self.objects = CollectionObjects()


class ViewLayerObjects:
    def __init__(self):
        self.active = None


class ViewLayer:
    def __init__(self):
        self.objects = ViewLayerObjects()


class Context:
    def __init__(self):
        self.collection = Collection()
        self.view_layer = ViewLayer()


class Data:
    def __init__(self):
        self.meshes = MeshCollection()
        self.objects = ObjectCollection()


class App:
    version_string = "stub-bpy"


class WmOps:
    def save_as_mainfile(self, filepath):
        with open(filepath, "w", encoding="utf-8") as handle:
            handle.write("stub blend\n")


class Ops:
    def __init__(self):
        self.wm = WmOps()


data = Data()
context = Context()
app = App()
ops = Ops()
"#;

const VERIFY_EXISTING_HARNESS: &str = r#"
from pathlib import Path
from types import SimpleNamespace
import importlib.util

script_path = Path(__file__).with_name("blender_reconstruct_v3.py")
spec = importlib.util.spec_from_file_location("adapter", script_path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

first = adapter.execute(SimpleNamespace(
    manifest="manifest.json",
    report="first.json",
    output_blend="reconstructed.blend",
    no_save=True,
    verify_existing=False,
))
second = adapter.execute(SimpleNamespace(
    manifest="manifest.json",
    report="verify-existing.json",
    output_blend="reconstructed.blend",
    no_save=True,
    verify_existing=True,
))
adapter.write_report(adapter.output_path("verify-existing.json"), second)
assert first["verification_passed"], first
assert second["verification_passed"], second
"#;

#[test]
fn generated_script_parses_with_python() {
    let package = TestPackage::new();
    let script = package.write_script();

    let output = Command::new("python")
        .arg("-m")
        .arg("py_compile")
        .arg(&script)
        .output()
        .expect("python should run");

    assert_success(output);
}

#[test]
fn generated_script_contains_generic_operator_iteration() {
    let script = blender_reconstruction_script_v3(&BlenderAdapterOptions::default());

    assert!(script.contains("for operator_index, operator in enumerate(manifest[\"operators\"]):"));
    assert!(script.contains("kind = operator.get(\"kind\")"));
    assert!(script.contains("elif kind == \"bend\":"));
    assert!(script.contains("elif kind == \"lossless_correction\":"));
}

#[test]
fn empty_explanatory_program_and_verify_existing_run_with_stub_bpy() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_lossless_only_package();
    let harness = package.path().join("verify_existing_harness.py");
    fs::write(&harness, VERIFY_EXISTING_HARNESS).unwrap();

    let output = Command::new("python")
        .arg(&harness)
        .current_dir(package.path())
        .output()
        .expect("python should run");

    assert_success(output);
    let report = read_json(package.path().join("verify-existing.json"));
    assert_eq!(report["verification_passed"], true);
    assert_eq!(report["mode"], "verify_existing_saved_blend");
    assert_eq!(
        report["editable_shape_key"]["shape_key_names"],
        json!(["Basis", "Final lossless correction"])
    );
}

#[test]
fn affine_stage_path_runs_with_stub_bpy() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_affine_package();

    let report = package.run_adapter_success();

    assert_eq!(report["verification_passed"], true);
    assert_eq!(report["stage_semantic_reports"][0]["kind"], "affine");
    assert_eq!(
        report["editable_shape_key"]["shape_key_names"],
        json!(["Basis", "Translation", "Final lossless correction"])
    );
    assert_eq!(
        report["editable_shape_key"]["preceding_shape_key_values_zero"],
        true
    );
}

#[test]
fn bend_stage_path_runs_with_stub_bpy() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_bend_package();

    let report = package.run_adapter_success();

    assert_eq!(report["verification_passed"], true);
    assert_eq!(report["stage_semantic_reports"][0]["kind"], "bend");
    assert_eq!(
        report["stage_semantic_reports"][0]["semantic_report"]["passed"],
        true
    );
}

#[test]
fn lossless_stage_path_applies_absolute_residual_positions() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_lossless_only_package();

    let report = package.run_adapter_success();

    assert_eq!(report["verification_passed"], true);
    assert_eq!(
        report["stage_semantic_reports"][0]["kind"],
        "lossless_correction"
    );
    assert_eq!(report["package_replay"]["residual_vertex_count"], 1);
    assert_eq!(report["positions_bit_exact"], true);
}

#[test]
fn script_rejects_unknown_operator_kind() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_unknown_operator_package();

    let output = Command::new("python")
        .arg(package.script_path())
        .arg("--")
        .arg("--no-save")
        .arg("--report")
        .arg("unknown-report.json")
        .current_dir(package.path())
        .output()
        .expect("python should run");

    assert!(
        !output.status.success(),
        "unknown operator unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read_json(package.path().join("unknown-report.json"));
    assert_eq!(report["verification_passed"], false);
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("unsupported operator kind")
    );
}

#[test]
fn existing_schema_two_blender_generator_source_remains_schema_two() {
    let lib_rs = include_str!("../src/lib.rs");

    assert!(lib_rs.contains("fn blender_reconstruction_script() -> &'static str"));
    assert!(lib_rs.contains("SUPPORTED_SCHEMA_VERSION = 2"));
    assert!(lib_rs.contains("ShapeLab_Reconstructed_Baked"));
    assert!(!lib_rs.contains("ShapeLab_V3_Reconstructed_Baked"));
}

struct TestPackage {
    dir: TempDir,
}

impl TestPackage {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn script_path(&self) -> PathBuf {
        self.path().join("blender_reconstruct_v3.py")
    }

    fn write_script(&self) -> PathBuf {
        let script = blender_reconstruction_script_v3(&BlenderAdapterOptions::default());
        let path = self.script_path();
        fs::write(&path, script).unwrap();
        path
    }

    fn write_bpy_stub(&self) {
        fs::write(self.path().join("bpy.py"), BPY_STUB).unwrap();
    }

    fn run_adapter_success(&self) -> Value {
        let output = Command::new("python")
            .arg(self.script_path())
            .arg("--")
            .arg("--no-save")
            .arg("--report")
            .arg("report.json")
            .current_dir(self.path())
            .output()
            .expect("python should run");
        assert_success(output);
        read_json(self.path().join("report.json"))
    }

    fn write_lossless_only_package(&self) {
        let source = vec![[0.0, 0.0, 0.0]];
        let target = vec![[1.0, 0.0, 0.0]];
        let indices = Vec::new();
        self.write_common_meshes(&source, &target, &indices);
        self.write_positions("operators/0000-lossless.f32", &target);
        self.write_u32s("residual/indices.u32", &[0]);
        self.write_positions("residual/positions.f32", &target);
        self.write_manifest(
            &source,
            &target,
            &indices,
            vec![json!({
                "kind": "lossless_correction",
                "stage": stage_json(
                    0,
                    "op-0000-lossless",
                    "Final lossless correction",
                    "operators/0000-lossless.f32",
                    "bit_exact",
                    0.0,
                ),
                "correction": {
                    "residual_index_file": "residual/indices.u32",
                    "residual_position_file": "residual/positions.f32",
                    "corrected_vertex_count": 1
                }
            })],
        );
    }

    fn write_affine_package(&self) {
        let source = vec![[1.0, 2.0, 3.0]];
        let affine_stage = vec![[2.0, 3.0, 4.0]];
        let indices = Vec::new();
        self.write_common_meshes(&source, &affine_stage, &indices);
        self.write_positions("operators/0000-affine.f32", &affine_stage);
        self.write_positions("operators/0001-lossless.f32", &affine_stage);
        self.write_u32s("residual/indices.u32", &[]);
        self.write_positions("residual/positions.f32", &[]);
        self.write_manifest(
            &source,
            &affine_stage,
            &indices,
            vec![
                json!({
                    "kind": "affine",
                    "stage": stage_json(
                        0,
                        "op-0000-translation",
                        "Translation",
                        "operators/0000-affine.f32",
                        "bit_exact",
                        0.0,
                    ),
                    "operator": {
                        "semantic_family": "translation",
                        "matrix_row_major_4x4": [
                            1.0, 0.0, 0.0, 1.0,
                            0.0, 1.0, 0.0, 1.0,
                            0.0, 0.0, 1.0, 1.0,
                            0.0, 0.0, 0.0, 1.0
                        ],
                        "translation": [1.0, 1.0, 1.0]
                    }
                }),
                json!({
                    "kind": "lossless_correction",
                    "stage": stage_json(
                        1,
                        "op-0001-lossless",
                        "Final lossless correction",
                        "operators/0001-lossless.f32",
                        "bit_exact",
                        0.0,
                    ),
                    "correction": {
                        "residual_index_file": "residual/indices.u32",
                        "residual_position_file": "residual/positions.f32",
                        "corrected_vertex_count": 0
                    }
                }),
            ],
        );
    }

    fn write_bend_package(&self) {
        let source = vec![[0.0, 0.5, 0.0]];
        let bend_stage = vec![bend_expected_position(source[0])];
        let indices = Vec::new();
        self.write_common_meshes(&source, &bend_stage, &indices);
        self.write_positions("operators/0000-bend.f32", &bend_stage);
        self.write_positions("operators/0001-lossless.f32", &bend_stage);
        self.write_u32s("residual/indices.u32", &[]);
        self.write_positions("residual/positions.f32", &[]);
        self.write_manifest(
            &source,
            &bend_stage,
            &indices,
            vec![
                json!({
                    "kind": "bend",
                    "stage": stage_json(
                        0,
                        "op-0000-bend",
                        "Bend",
                        "operators/0000-bend.f32",
                        "tolerance",
                        0.00001,
                    ),
                    "parameters": {
                        "origin": [0.0, 0.0, 0.0],
                        "longitudinal_axis": [0.0, 1.0, 0.0],
                        "bend_direction": [1.0, 0.0, 0.0],
                        "angle_radians": 0.5,
                        "interval_start": 0.0,
                        "interval_end": 1.0
                    }
                }),
                json!({
                    "kind": "lossless_correction",
                    "stage": stage_json(
                        1,
                        "op-0001-lossless",
                        "Final lossless correction",
                        "operators/0001-lossless.f32",
                        "bit_exact",
                        0.0,
                    ),
                    "correction": {
                        "residual_index_file": "residual/indices.u32",
                        "residual_position_file": "residual/positions.f32",
                        "corrected_vertex_count": 0
                    }
                }),
            ],
        );
    }

    fn write_unknown_operator_package(&self) {
        let source = vec![[0.0, 0.0, 0.0]];
        let target = source.clone();
        let indices = Vec::new();
        self.write_manifest(
            &source,
            &target,
            &indices,
            vec![
                json!({
                    "kind": "twist",
                    "stage": stage_json(
                        0,
                        "op-0000-twist",
                        "Twist",
                        "operators/0000-twist.f32",
                        "tolerance",
                        0.001,
                    )
                }),
                json!({
                    "kind": "lossless_correction",
                    "stage": stage_json(
                        1,
                        "op-0001-lossless",
                        "Final lossless correction",
                        "operators/0001-lossless.f32",
                        "bit_exact",
                        0.0,
                    ),
                    "correction": {
                        "residual_index_file": "residual/indices.u32",
                        "residual_position_file": "residual/positions.f32",
                        "corrected_vertex_count": 0
                    }
                }),
            ],
        );
    }

    fn write_common_meshes(&self, source: &[[f32; 3]], target: &[[f32; 3]], indices: &[u32]) {
        write_meshbin(&self.path().join("source.meshbin"), source, indices);
        write_meshbin(&self.path().join("target.meshbin"), target, indices);
    }

    fn write_positions(&self, path: &str, positions: &[[f32; 3]]) {
        let path = self.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_positions(&path, positions);
    }

    fn write_u32s(&self, path: &str, values: &[u32]) {
        let path = self.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();
        for value in values {
            file.write_all(&value.to_le_bytes()).unwrap();
        }
    }

    fn write_manifest(
        &self,
        source: &[[f32; 3]],
        target: &[[f32; 3]],
        indices: &[u32],
        operators: Vec<Value>,
    ) {
        let manifest = json!({
            "schema_version": 3,
            "coordinate_system": "right-handed-y-up",
            "numeric_format": {
                "scalar": "float32",
                "endian": "little",
                "affine_evaluation": "float32_stepwise_no_fma"
            },
            "source": {
                "path": "source.meshbin",
                "vertex_count": source.len(),
                "triangle_count": indices.len() / 3
            },
            "target": {
                "path": "target.meshbin",
                "vertex_count": target.len(),
                "triangle_count": indices.len() / 3
            },
            "topology": {
                "vertex_count": source.len(),
                "triangle_count": indices.len() / 3,
                "index_count": indices.len(),
                "hash": topology_hash(source.len(), indices)
            },
            "operators": operators
        });
        fs::write(
            self.path().join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }
}

fn stage_json(
    stage_index: usize,
    operator_id: &str,
    label: &str,
    baked_positions_file: &str,
    mode: &str,
    absolute_epsilon: f64,
) -> Value {
    json!({
        "stage_index": stage_index,
        "operator_id": operator_id,
        "label": label,
        "baked_positions_file": baked_positions_file,
        "semantic_verification_policy": {
            "mode": mode,
            "absolute_epsilon": absolute_epsilon,
            "relative_epsilon": 0.0,
            "ulp_multiplier": 0.0
        },
        "semantic_verification_report": {
            "max_component_error": 0.0,
            "max_euclidean_error": 0.0,
            "mean_euclidean_error": 0.0,
            "rms_euclidean_error": 0.0,
            "outside_tolerance": 0,
            "passed": true
        }
    })
}

fn bend_expected_position(position: [f32; 3]) -> [f32; 3] {
    let angle = 0.5_f32;
    let interval_start = 0.0_f32;
    let interval_length = 1.0_f32;
    let radius = interval_length / angle;
    let theta = angle * ((position[1] - interval_start) / interval_length);
    [
        radius * (1.0 - theta.cos()) + position[0] * theta.cos(),
        interval_start + radius * theta.sin() - position[0] * theta.sin(),
        position[2],
    ]
}

fn write_meshbin(path: &Path, positions: &[[f32; 3]], indices: &[u32]) {
    let mut file = File::create(path).unwrap();
    file.write_all(b"SLMBIN01").unwrap();
    file.write_all(&(positions.len() as u64).to_le_bytes())
        .unwrap();
    file.write_all(&(indices.len() as u64).to_le_bytes())
        .unwrap();
    for position in positions {
        for component in position {
            file.write_all(&component.to_le_bytes()).unwrap();
        }
    }
    for index in indices {
        file.write_all(&index.to_le_bytes()).unwrap();
    }
}

fn write_positions(path: &Path, positions: &[[f32; 3]]) {
    let mut file = File::create(path).unwrap();
    for position in positions {
        for component in position {
            file.write_all(&component.to_le_bytes()).unwrap();
        }
    }
}

fn topology_hash(vertex_count: usize, indices: &[u32]) -> String {
    let mut value = FNV_OFFSET;
    value = fnv1a_update(value, &(vertex_count as u64).to_le_bytes());
    value = fnv1a_update(value, &(indices.len() as u64).to_le_bytes());
    for index in indices {
        value = fnv1a_update(value, &index.to_le_bytes());
    }
    format!("fnv1a64:{value:016x}")
}

fn fnv1a_update(mut value: u64, payload: &[u8]) -> u64 {
    for byte in payload {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    value
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
