use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use shape_decompiler::v3::bend::{BendParameters, evaluate_bend};
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
fn python_bend_evaluator_matches_known_points() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();

    let output = package.run_harness(
        "known_points.py",
        r#"
import importlib.util
import math
import struct
from pathlib import Path

script_path = Path(__file__).with_name("blender_reconstruct_v3.py")
spec = importlib.util.spec_from_file_location("adapter", script_path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]

def assert_close(actual, expected, epsilon=1.0e-6):
    assert len(actual) == len(expected), (actual, expected)
    for left, right in zip(actual, expected):
        assert abs(left - right) <= epsilon, (actual, expected)

angle = f32(math.pi / 2.0)
evaluated = adapter.evaluate_bend({
    "origin": [0.0, 0.0, 0.0],
    "longitudinal_axis": [1.0, 0.0, 0.0],
    "bend_direction": [0.0, 1.0, 0.0],
    "angle_radians": angle,
    "interval_start": 0.0,
    "interval_end": 1.0,
}, [(1.0, 0.0, 0.0)])[0]
expected = (math.sin(angle) / angle, (1.0 - math.cos(angle)) / angle, 0.0)
assert_close(evaluated, expected)

small_angle = f32(1.0e-5)
small = adapter.evaluate_bend({
    "origin": [0.0, 0.0, 0.0],
    "longitudinal_axis": [0.0, 1.0, 0.0],
    "bend_direction": [1.0, 0.0, 0.0],
    "angle_radians": small_angle,
    "interval_start": 0.0,
    "interval_end": 1.0,
}, [(0.0, 0.5, 0.0)])[0]
phi = small_angle * 0.5
phi_squared = phi * phi
expected_small = (
    small_angle * 0.5 * 0.5 * (
        0.5
        - phi_squared / 24.0
        + phi_squared * phi_squared / 720.0
        - phi_squared * phi_squared * phi_squared / 40320.0
    ),
    0.5 * (
        1.0
        - phi_squared / 6.0
        + phi_squared * phi_squared / 120.0
        - phi_squared * phi_squared * phi_squared / 5040.0
    ),
    0.0,
)
assert_close(small, expected_small)
"#,
    );

    assert_success(output);
}

#[test]
fn python_bend_zero_angle_has_exact_identity_path() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();

    let output = package.run_harness(
        "zero_angle.py",
        r#"
import importlib.util
from pathlib import Path

script_path = Path(__file__).with_name("blender_reconstruct_v3.py")
spec = importlib.util.spec_from_file_location("adapter", script_path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

source = [(1.25, -2.5, 3.75), (-0.0, 8.0, -9.0)]
evaluated = adapter.evaluate_bend({
    "origin": [0.0, 0.0, 0.0],
    "longitudinal_axis": [0.0, 1.0, 0.0],
    "bend_direction": [1.0, 0.0, 0.0],
    "angle_radians": 5.0e-8,
    "interval_start": 0.0,
    "interval_end": 1.0,
}, source)
assert evaluated == source, evaluated
"#,
    );

    assert_success(output);
}

#[test]
fn python_bend_evaluator_supports_translated_origin() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();

    let output = package.run_harness(
        "translated_origin.py",
        r#"
import importlib.util
import math
import struct
from pathlib import Path

script_path = Path(__file__).with_name("blender_reconstruct_v3.py")
spec = importlib.util.spec_from_file_location("adapter", script_path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]

def assert_close(actual, expected, epsilon=1.0e-6):
    assert len(actual) == len(expected), (actual, expected)
    for left, right in zip(actual, expected):
        assert abs(left - right) <= epsilon, (actual, expected)

angle = f32(math.pi / 2.0)
evaluated = adapter.evaluate_bend({
    "origin": [10.0, 20.0, 30.0],
    "longitudinal_axis": [0.0, 1.0, 0.0],
    "bend_direction": [1.0, 0.0, 0.0],
    "angle_radians": angle,
    "interval_start": 0.0,
    "interval_end": 1.0,
}, [(10.0, 21.0, 30.0)])[0]
expected = (
    10.0 + (1.0 - math.cos(angle)) / angle,
    20.0 + math.sin(angle) / angle,
    30.0,
)
assert_close(evaluated, expected)
"#,
    );

    assert_success(output);
}

#[test]
fn python_bend_rejects_malformed_parameters() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();

    let output = package.run_harness(
        "malformed.py",
        r#"
import importlib.util
import math
from pathlib import Path

script_path = Path(__file__).with_name("blender_reconstruct_v3.py")
spec = importlib.util.spec_from_file_location("adapter", script_path)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

def valid():
    return {
        "origin": [0.0, 0.0, 0.0],
        "longitudinal_axis": [0.0, 1.0, 0.0],
        "bend_direction": [1.0, 0.0, 0.0],
        "angle_radians": 0.5,
        "interval_start": 0.0,
        "interval_end": 1.0,
    }

def assert_rejects(mutator, expected):
    parameters = valid()
    mutator(parameters)
    try:
        adapter.validate_bend_parameters(parameters)
    except ValueError as error:
        assert expected in str(error), str(error)
    else:
        raise AssertionError(parameters)

assert_rejects(lambda p: p.update({"longitudinal_axis": [0.0, 0.0, 0.0]}), "longitudinal_axis")
assert_rejects(lambda p: p.update({"bend_direction": [0.0, 2.0, 0.0]}), "bend_direction")
assert_rejects(lambda p: p.update({"interval_end": 0.0}), "interval_end")
assert_rejects(lambda p: p.update({"angle_radians": 3.5}), "angle magnitude")
assert_rejects(lambda p: p.update({"angle_radians": math.nan}), "angle")
"#,
    );

    assert_success(output);
}

#[test]
fn semantic_mismatch_against_tampered_baked_stage_fails() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_bend_package(BendPackageOptions {
        baked_x_delta: 0.25,
        absolute_epsilon: 1.0e-7,
    });

    let output = package.run_adapter();

    assert!(
        !output.status.success(),
        "tampered package unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read_json(package.path().join("report.json"));
    assert_eq!(report["verification_passed"], false);
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("bend semantic evaluation does not satisfy")
    );
}

#[test]
fn shape_key_creation_uses_baked_data_after_semantic_check() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_bend_package(BendPackageOptions {
        baked_x_delta: 0.0005,
        absolute_epsilon: 0.01,
    });

    let report = package.run_adapter_success();

    assert_eq!(report["verification_passed"], true);
    assert!(
        report["stage_semantic_reports"][0]["semantic_report"]["max_component_error"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        report["editable_shape_key"]["stage_results"][1]["expected_name"],
        "Bend"
    );
    assert_eq!(
        report["editable_shape_key"]["stage_results"][1]["positions_bit_exact"],
        true
    );
    assert_eq!(report["positions_bit_exact"], true);
}

#[test]
fn bend_stage_replays_with_stub_bpy_and_reopen_verifier_fields() {
    let package = TestPackage::new();
    package.write_script();
    package.write_bpy_stub();
    package.write_bend_package(BendPackageOptions {
        baked_x_delta: 0.0,
        absolute_epsilon: 1.0e-5,
    });

    let report = package.run_verify_existing_success();

    assert_eq!(report["verification_passed"], true);
    assert_eq!(report["mode"], "verify_existing_saved_blend");
    assert_eq!(report["stage_semantic_reports"][0]["kind"], "bend");
    assert_eq!(
        report["editable_shape_key"]["shape_key_names"],
        json!(["Basis", "Bend", "Final lossless correction"])
    );
    assert_eq!(report["editable_shape_key"]["shape_key_names_exact"], true);
    assert_eq!(report["editable_shape_key"]["stage_positions_exact"], true);
    assert_eq!(report["editable_shape_key"]["final_shape_key_value"], 1.0);
    assert_eq!(report["editable_shape_key"]["vertex_ids_exact"], true);
    assert_eq!(report["topology_exact"], true);
    assert_eq!(report["positions_bit_exact"], true);
}

#[test]
fn existing_schema_two_blender_generator_source_remains_schema_two() {
    let lib_rs = include_str!("../src/lib.rs");

    assert!(lib_rs.contains("fn blender_reconstruction_script() -> &'static str"));
    assert!(lib_rs.contains("SUPPORTED_SCHEMA_VERSION = 2"));
    assert!(lib_rs.contains("ShapeLab_Reconstructed_Baked"));
    assert!(!lib_rs.contains("ShapeLab_V3_Reconstructed_Baked"));
}

#[derive(Debug, Copy, Clone)]
struct BendPackageOptions {
    baked_x_delta: f32,
    absolute_epsilon: f64,
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

    fn run_harness(&self, name: &str, source: &str) -> Output {
        let path = self.path().join(name);
        fs::write(&path, source).unwrap();
        Command::new("python")
            .arg(path)
            .current_dir(self.path())
            .output()
            .expect("python should run")
    }

    fn run_adapter(&self) -> Output {
        Command::new("python")
            .arg(self.script_path())
            .arg("--")
            .arg("--no-save")
            .arg("--report")
            .arg("report.json")
            .current_dir(self.path())
            .output()
            .expect("python should run")
    }

    fn run_adapter_success(&self) -> Value {
        let output = self.run_adapter();
        assert_success(output);
        read_json(self.path().join("report.json"))
    }

    fn run_verify_existing_success(&self) -> Value {
        let output = self.run_harness(
            "verify_existing.py",
            r#"
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
"#,
        );
        assert_success(output);
        read_json(self.path().join("verify-existing.json"))
    }

    fn write_bend_package(&self, options: BendPackageOptions) {
        let source = vec![[0.0, 0.5, 0.0], [0.25, 0.25, 0.5], [0.1, 1.25, -0.2]];
        let parameters = BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [0.0, 1.0, 0.0],
            bend_direction: [1.0, 0.0, 0.0],
            angle_radians: 0.5,
            interval_start: 0.0,
            interval_end: 1.0,
        };
        let mut bend_stage = evaluate_bend(&parameters, &source).unwrap();
        for position in &mut bend_stage {
            position[0] += options.baked_x_delta;
        }
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
                        options.absolute_epsilon,
                    ),
                    "parameters": {
                        "origin": parameters.origin,
                        "longitudinal_axis": parameters.longitudinal_axis,
                        "bend_direction": parameters.bend_direction,
                        "angle_radians": parameters.angle_radians,
                        "interval_start": parameters.interval_start,
                        "interval_end": parameters.interval_end
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
