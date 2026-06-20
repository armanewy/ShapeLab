#![forbid(unsafe_code)]

//! Uniform-grid mesh generation and OBJ import/export.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use glam::Vec3;
use shape_core::Aabb;
use shape_field::ScalarField;
use thiserror::Error;

const MAX_GRID_SAMPLES: usize = 16_777_216;
const MIN_GRADIENT_STEP: f32 = 1.0e-4;
const MIN_NORMAL_LENGTH: f32 = 1.0e-6;
const MIN_TRIANGLE_AREA_NORMAL: f32 = 1.0e-8;
const OBJ_TEMP_PREFIX: &str = ".shape-lab-obj-";
const TEMP_FILE_SUFFIX: &str = ".tmp";
const OBSOLETE_TEMP_MIN_AGE: Duration = Duration::from_secs(60 * 60);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

// Cube corner numbering is deterministic and shared by all voxels:
// 0=(0,0,0), 1=(1,0,0), 2=(1,1,0), 3=(0,1,0),
// 4=(0,0,1), 5=(1,0,1), 6=(1,1,1), 7=(0,1,1).
const CUBE_CORNERS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];

// Each cube is split along the 0-to-6 diagonal into six tetrahedra. This keeps
// the decomposition small and deterministic without a marching-cubes table.
const TETRAHEDRA: [[usize; 4]; 6] = [
    [0, 5, 1, 6],
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
];

/// Triangle mesh with indexed positions and normals.
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Mesh bounds.
    pub bounds: Aabb,
}

/// Meshing settings.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MeshSettings {
    /// Uniform voxel resolution per axis. The sampler evaluates
    /// `(resolution + 1)^3` grid points.
    pub resolution: usize,
    /// Fraction of the largest field-bounds extent added on each side.
    pub padding_fraction: f32,
    /// Surface iso-value.
    pub iso_value: f32,
}

impl Default for MeshSettings {
    fn default() -> Self {
        Self {
            resolution: 32,
            padding_fraction: 0.08,
            iso_value: 0.0,
        }
    }
}

/// Mesh generation and export errors.
#[derive(Debug, Error)]
pub enum MeshError {
    /// Mesh settings are invalid.
    #[error("invalid mesh settings: {0}")]
    InvalidSettings(String),
    /// Field bounds cannot be meshed.
    #[error("invalid field bounds: {0}")]
    InvalidBounds(String),
    /// The field returned a non-finite value.
    #[error("field returned non-finite sample {value} at {point:?}")]
    NonFiniteSample { point: [f32; 3], value: f32 },
    /// The requested grid or mesh is too large for the MVP implementation.
    #[error("mesh request is too large: {0}")]
    TooLarge(String),
    /// Mesh indices cannot fit in OBJ-compatible u32 storage.
    #[error("mesh index count exceeds u32")]
    IndexOverflow,
    /// A provided mesh is internally inconsistent.
    #[error("invalid mesh: {0}")]
    InvalidMesh(String),
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// I/O failed for a specific path.
    #[error("io error for {path}: {source}")]
    PathIo {
        /// Output path.
        path: PathBuf,
        /// Source error.
        #[source]
        source: std::io::Error,
    },
}

/// Mesh an implicit scalar field with uniform-grid marching tetrahedra.
pub fn mesh_field(
    field: &impl ScalarField,
    settings: MeshSettings,
) -> Result<TriangleMesh, MeshError> {
    validate_settings(settings)?;

    let bounds = padded_bounds(field.bounds(), settings.padding_fraction)?;
    let resolution = settings.resolution;
    let side = resolution
        .checked_add(1)
        .ok_or_else(|| MeshError::TooLarge("resolution overflow".to_owned()))?;
    let sample_count = checked_grid_sample_count(side)?;
    let extent = bounds.extent();

    let mut samples = Vec::with_capacity(sample_count);
    for z in 0..side {
        for y in 0..side {
            for x in 0..side {
                let point = grid_point(bounds, extent, resolution, x, y, z);
                samples.push(sample_checked(field, point)?);
            }
        }
    }

    let gradient_step = gradient_step(bounds, resolution);
    let mut builder = MeshBuilder::default();
    for z in 0..resolution {
        for y in 0..resolution {
            for x in 0..resolution {
                let mut cube = [GridCorner::default(); 8];
                for (corner_index, offset) in CUBE_CORNERS.iter().enumerate() {
                    let gx = x + offset[0];
                    let gy = y + offset[1];
                    let gz = z + offset[2];
                    cube[corner_index] = GridCorner {
                        position: grid_point(bounds, extent, resolution, gx, gy, gz),
                        value: samples[grid_index(side, gx, gy, gz)],
                    };
                }
                for tetrahedron in TETRAHEDRA {
                    polygonize_tetrahedron(
                        field,
                        settings.iso_value,
                        gradient_step,
                        tetrahedron.map(|index| cube[index]),
                        &mut builder,
                    )?;
                }
            }
        }
    }

    Ok(builder.finish())
}

/// Write a mesh as OBJ text to any writer.
pub fn write_obj(mesh: &TriangleMesh, mut writer: impl Write) -> Result<(), MeshError> {
    validate_mesh(mesh)?;

    writeln!(writer, "# Shape Lab generated OBJ")?;
    writeln!(writer, "# format wavefront-obj")?;
    writeln!(writer, "# vertex_count {}", mesh.positions.len())?;
    writeln!(writer, "# normal_count {}", mesh.normals.len())?;
    writeln!(writer, "# triangle_count {}", mesh.indices.len() / 3)?;
    if mesh.bounds.is_empty() {
        writeln!(writer, "# bounds empty")?;
    } else {
        writeln!(
            writer,
            "# bounds min {:.9} {:.9} {:.9} max {:.9} {:.9} {:.9}",
            mesh.bounds.min.x,
            mesh.bounds.min.y,
            mesh.bounds.min.z,
            mesh.bounds.max.x,
            mesh.bounds.max.y,
            mesh.bounds.max.z
        )?;
    }
    writeln!(
        writer,
        "# vertices {} triangles {}",
        mesh.positions.len(),
        mesh.indices.len() / 3
    )?;
    for position in &mesh.positions {
        writeln!(
            writer,
            "v {:.9} {:.9} {:.9}",
            position[0], position[1], position[2]
        )?;
    }
    for normal in &mesh.normals {
        writeln!(
            writer,
            "vn {:.9} {:.9} {:.9}",
            normal[0], normal[1], normal[2]
        )?;
    }
    for triangle in mesh.indices.chunks_exact(3) {
        let a = obj_index(triangle[0])?;
        let b = obj_index(triangle[1])?;
        let c = obj_index(triangle[2])?;
        writeln!(writer, "f {a}//{a} {b}//{b} {c}//{c}")?;
    }
    Ok(())
}

/// Write OBJ text to a path.
pub fn write_obj_to_path(mesh: &TriangleMesh, path: impl AsRef<Path>) -> Result<(), MeshError> {
    let path = path.as_ref();
    atomic_obj_replace(path, |file| {
        let mut writer = BufWriter::new(file);
        write_obj(mesh, &mut writer)?;
        writer.flush()?;
        Ok(())
    })
}

/// Return OBJ text as a string for callers that need an in-memory export.
pub fn write_obj_to_string(mesh: &TriangleMesh) -> Result<String, MeshError> {
    let mut bytes = Vec::new();
    write_obj(mesh, &mut bytes)?;
    String::from_utf8(bytes).map_err(|error| MeshError::InvalidMesh(error.to_string()))
}

/// Read a triangle OBJ mesh from a path.
pub fn read_obj_from_path(path: impl AsRef<Path>) -> Result<TriangleMesh, MeshError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| path_io(path, source))?;
    read_obj(BufReader::new(file))
}

/// Read a triangle OBJ mesh from any buffered reader.
///
/// The parser accepts vertex records (`v`) and triangular faces (`f`) using
/// `v`, `v/vt`, `v//vn`, or `v/vt/vn` face elements. Normals are recomputed
/// into the mesh's one-normal-per-position representation.
pub fn read_obj(reader: impl BufRead) -> Result<TriangleMesh, MeshError> {
    let mut positions = Vec::new();
    let mut indices = Vec::new();

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line_result?;
        let Some(trimmed) = line.split('#').next().map(str::trim) else {
            continue;
        };
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };

        match kind {
            "v" => {
                let x = parse_obj_f32(parts.next(), line_number, "x")?;
                let y = parse_obj_f32(parts.next(), line_number, "y")?;
                let z = parse_obj_f32(parts.next(), line_number, "z")?;
                if parts.next().is_some() {
                    return Err(obj_parse_error(
                        line_number,
                        "vertex records must contain exactly three coordinates",
                    ));
                }
                positions.push([x, y, z]);
            }
            "f" => {
                let face = parts.collect::<Vec<_>>();
                if face.len() != 3 {
                    return Err(obj_parse_error(
                        line_number,
                        "only triangular faces are supported",
                    ));
                }
                for element in face {
                    indices.push(parse_obj_vertex_index(
                        element,
                        positions.len(),
                        line_number,
                    )?);
                }
            }
            "vn" | "vt" | "o" | "g" | "s" | "mtllib" | "usemtl" => {}
            _ => {}
        }
    }

    mesh_from_obj_data(positions, indices)
}

fn parse_obj_f32(
    token: Option<&str>,
    line_number: usize,
    field: &'static str,
) -> Result<f32, MeshError> {
    let token = token.ok_or_else(|| obj_parse_error(line_number, format!("missing {field}")))?;
    let value = token.parse::<f32>().map_err(|_| {
        obj_parse_error(
            line_number,
            format!("invalid floating-point {field} coordinate"),
        )
    })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(obj_parse_error(
            line_number,
            format!("{field} coordinate must be finite"),
        ))
    }
}

fn parse_obj_vertex_index(
    element: &str,
    position_count: usize,
    line_number: usize,
) -> Result<u32, MeshError> {
    let components = element.split('/').collect::<Vec<_>>();
    let raw_index = match components.as_slice() {
        [position] if !position.is_empty() => *position,
        [position, texture] if !position.is_empty() && !texture.is_empty() => {
            parse_obj_aux_index(texture, line_number, "texture")?;
            *position
        }
        [position, "", normal] if !position.is_empty() && !normal.is_empty() => {
            parse_obj_aux_index(normal, line_number, "normal")?;
            *position
        }
        [position, texture, normal]
            if !position.is_empty() && !texture.is_empty() && !normal.is_empty() =>
        {
            parse_obj_aux_index(texture, line_number, "texture")?;
            parse_obj_aux_index(normal, line_number, "normal")?;
            *position
        }
        _ => {
            return Err(obj_parse_error(
                line_number,
                "face vertex must use v, v/vt, v//vn, or v/vt/vn",
            ));
        }
    };
    let parsed = raw_index
        .parse::<i64>()
        .map_err(|_| obj_parse_error(line_number, "invalid face vertex position index"))?;
    if parsed == 0 {
        return Err(obj_parse_error(
            line_number,
            "OBJ position indices are one-based; zero is invalid",
        ));
    }

    let zero_based = if parsed > 0 {
        parsed - 1
    } else {
        i64::try_from(position_count).map_err(|_| MeshError::IndexOverflow)? + parsed
    };
    if zero_based < 0 || zero_based >= i64::try_from(position_count).unwrap_or(i64::MAX) {
        return Err(obj_parse_error(
            line_number,
            "face vertex position index is out of range",
        ));
    }
    u32::try_from(zero_based).map_err(|_| MeshError::IndexOverflow)
}

fn parse_obj_aux_index(
    raw_index: &str,
    line_number: usize,
    label: &'static str,
) -> Result<(), MeshError> {
    let parsed = raw_index
        .parse::<i64>()
        .map_err(|_| obj_parse_error(line_number, format!("invalid face vertex {label} index")))?;
    if parsed == 0 {
        Err(obj_parse_error(
            line_number,
            format!("OBJ {label} indices are one-based; zero is invalid"),
        ))
    } else {
        Ok(())
    }
}

fn mesh_from_obj_data(
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
) -> Result<TriangleMesh, MeshError> {
    if positions.is_empty() {
        return Err(MeshError::InvalidMesh(
            "OBJ must contain at least one vertex".to_owned(),
        ));
    }
    if !indices.len().is_multiple_of(3) {
        return Err(MeshError::InvalidMesh(
            "OBJ index count must be divisible by three".to_owned(),
        ));
    }
    let normals = computed_vertex_normals(&positions, &indices)?;
    let bounds = bounds_from_positions(&positions)?;
    let mesh = TriangleMesh {
        positions,
        normals,
        indices,
        bounds,
    };
    validate_mesh(&mesh)?;
    Ok(mesh)
}

fn computed_vertex_normals(
    positions: &[[f32; 3]],
    indices: &[u32],
) -> Result<Vec<[f32; 3]>, MeshError> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let a = vertex_position(positions, triangle[0])?;
        let b = vertex_position(positions, triangle[1])?;
        let c = vertex_position(positions, triangle[2])?;
        if let Some(face_normal) = normalize_or_none((b - a).cross(c - a)) {
            normals[triangle[0] as usize] += face_normal;
            normals[triangle[1] as usize] += face_normal;
            normals[triangle[2] as usize] += face_normal;
        }
    }
    Ok(normals
        .into_iter()
        .map(|normal| normalize_or_none(normal).unwrap_or(Vec3::Y).to_array())
        .collect())
}

fn vertex_position(positions: &[[f32; 3]], index: u32) -> Result<Vec3, MeshError> {
    positions
        .get(index as usize)
        .copied()
        .map(Vec3::from_array)
        .ok_or_else(|| MeshError::InvalidMesh("face index is out of range".to_owned()))
}

fn bounds_from_positions(positions: &[[f32; 3]]) -> Result<Aabb, MeshError> {
    let mut bounds = Aabb::empty();
    for position in positions {
        if !array_is_finite(*position) {
            return Err(MeshError::InvalidMesh(
                "all OBJ positions must be finite".to_owned(),
            ));
        }
        let point = Vec3::from_array(*position);
        bounds = bounds.union(&Aabb {
            min: point,
            max: point,
        });
    }
    Ok(bounds)
}

fn obj_parse_error(line_number: usize, message: impl Into<String>) -> MeshError {
    MeshError::InvalidMesh(format!("OBJ line {line_number}: {}", message.into()))
}

#[derive(Debug, Copy, Clone)]
struct GridCorner {
    position: Vec3,
    value: f32,
}

impl Default for GridCorner {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            value: 0.0,
        }
    }
}

#[derive(Debug)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    bounds: Aabb,
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::empty(),
        }
    }
}

impl MeshBuilder {
    fn push_triangle(&mut self, positions: [Vec3; 3], normals: [Vec3; 3]) -> Result<(), MeshError> {
        let base = u32::try_from(self.positions.len()).map_err(|_| MeshError::IndexOverflow)?;
        base.checked_add(2).ok_or(MeshError::IndexOverflow)?;

        for (position, normal) in positions.into_iter().zip(normals) {
            if !vec3_is_finite(position) || !vec3_is_finite(normal) {
                return Err(MeshError::InvalidMesh(
                    "attempted to emit non-finite vertex data".to_owned(),
                ));
            }
            self.positions.push(position.to_array());
            self.normals.push(normal.to_array());
            self.bounds = self.bounds.union(&Aabb {
                min: position,
                max: position,
            });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
        Ok(())
    }

    fn finish(self) -> TriangleMesh {
        TriangleMesh {
            positions: self.positions,
            normals: self.normals,
            indices: self.indices,
            bounds: self.bounds,
        }
    }
}

fn validate_settings(settings: MeshSettings) -> Result<(), MeshError> {
    if settings.resolution == 0 {
        return Err(MeshError::InvalidSettings(
            "resolution must be greater than zero".to_owned(),
        ));
    }
    if !settings.padding_fraction.is_finite() || settings.padding_fraction < 0.0 {
        return Err(MeshError::InvalidSettings(
            "padding_fraction must be finite and non-negative".to_owned(),
        ));
    }
    if !settings.iso_value.is_finite() {
        return Err(MeshError::InvalidSettings(
            "iso_value must be finite".to_owned(),
        ));
    }
    let side = settings
        .resolution
        .checked_add(1)
        .ok_or_else(|| MeshError::TooLarge("resolution overflow".to_owned()))?;
    checked_grid_sample_count(side)?;
    Ok(())
}

fn checked_grid_sample_count(side: usize) -> Result<usize, MeshError> {
    let count = side
        .checked_mul(side)
        .and_then(|value| value.checked_mul(side))
        .ok_or_else(|| MeshError::TooLarge("grid sample count overflow".to_owned()))?;
    if count > MAX_GRID_SAMPLES {
        return Err(MeshError::TooLarge(format!(
            "grid has {count} samples; limit is {MAX_GRID_SAMPLES}"
        )));
    }
    Ok(count)
}

fn padded_bounds(bounds: Aabb, padding_fraction: f32) -> Result<Aabb, MeshError> {
    if bounds.is_empty() {
        return Err(MeshError::InvalidBounds(
            "field bounds are empty".to_owned(),
        ));
    }
    if !vec3_is_finite(bounds.min) || !vec3_is_finite(bounds.max) {
        return Err(MeshError::InvalidBounds(
            "field bounds must be finite".to_owned(),
        ));
    }
    let extent = bounds.extent();
    if !vec3_is_finite(extent) || extent.cmple(Vec3::ZERO).any() {
        return Err(MeshError::InvalidBounds(
            "field bounds must have positive finite extent on all axes".to_owned(),
        ));
    }
    let padding = extent.max_element() * padding_fraction;
    let padded = bounds.expanded(padding);
    if !vec3_is_finite(padded.min) || !vec3_is_finite(padded.max) {
        return Err(MeshError::InvalidBounds(
            "padded field bounds must be finite".to_owned(),
        ));
    }
    Ok(padded)
}

fn grid_index(side: usize, x: usize, y: usize, z: usize) -> usize {
    x + y * side + z * side * side
}

fn grid_point(bounds: Aabb, extent: Vec3, resolution: usize, x: usize, y: usize, z: usize) -> Vec3 {
    let denom = resolution as f32;
    bounds.min
        + Vec3::new(
            extent.x * (x as f32 / denom),
            extent.y * (y as f32 / denom),
            extent.z * (z as f32 / denom),
        )
}

fn gradient_step(bounds: Aabb, resolution: usize) -> f32 {
    let cell = bounds.extent().min_element() / resolution as f32;
    (cell * 0.5).max(MIN_GRADIENT_STEP)
}

fn polygonize_tetrahedron(
    field: &impl ScalarField,
    iso_value: f32,
    gradient_step: f32,
    corners: [GridCorner; 4],
    builder: &mut MeshBuilder,
) -> Result<(), MeshError> {
    let mut inside = [0_usize; 4];
    let mut outside = [0_usize; 4];
    let mut inside_len = 0;
    let mut outside_len = 0;

    for (index, corner) in corners.iter().enumerate() {
        if corner.value <= iso_value {
            inside[inside_len] = index;
            inside_len += 1;
        } else {
            outside[outside_len] = index;
            outside_len += 1;
        }
    }

    match inside_len {
        0 | 4 => Ok(()),
        1 => {
            let i = inside[0];
            emit_triangle(
                field,
                [
                    interpolate(corners[i], corners[outside[0]], iso_value),
                    interpolate(corners[i], corners[outside[1]], iso_value),
                    interpolate(corners[i], corners[outside[2]], iso_value),
                ],
                gradient_step,
                builder,
            )
        }
        2 => {
            let i0 = inside[0];
            let i1 = inside[1];
            let o0 = outside[0];
            let o1 = outside[1];
            let p00 = interpolate(corners[i0], corners[o0], iso_value);
            let p01 = interpolate(corners[i0], corners[o1], iso_value);
            let p10 = interpolate(corners[i1], corners[o0], iso_value);
            let p11 = interpolate(corners[i1], corners[o1], iso_value);
            emit_triangle(field, [p00, p01, p10], gradient_step, builder)?;
            emit_triangle(field, [p10, p01, p11], gradient_step, builder)
        }
        3 => {
            let o = outside[0];
            emit_triangle(
                field,
                [
                    interpolate(corners[o], corners[inside[0]], iso_value),
                    interpolate(corners[o], corners[inside[1]], iso_value),
                    interpolate(corners[o], corners[inside[2]], iso_value),
                ],
                gradient_step,
                builder,
            )
        }
        _ => Ok(()),
    }
}

fn interpolate(a: GridCorner, b: GridCorner, iso_value: f32) -> Vec3 {
    let denominator = b.value - a.value;
    if denominator.abs() <= f32::EPSILON {
        return (a.position + b.position) * 0.5;
    }
    let t = ((iso_value - a.value) / denominator).clamp(0.0, 1.0);
    a.position + (b.position - a.position) * t
}

fn emit_triangle(
    field: &impl ScalarField,
    mut positions: [Vec3; 3],
    gradient_step: f32,
    builder: &mut MeshBuilder,
) -> Result<(), MeshError> {
    if positions.iter().any(|position| !vec3_is_finite(*position)) {
        return Err(MeshError::InvalidMesh(
            "attempted to emit a non-finite position".to_owned(),
        ));
    }

    let face = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
    let Some(face_normal) = normalize_or_none(face) else {
        return Ok(());
    };
    let centroid = (positions[0] + positions[1] + positions[2]) / 3.0;
    let outward = normalized_gradient(field, centroid, gradient_step)?.unwrap_or(face_normal);
    let final_face_normal = if face_normal.dot(outward) < 0.0 {
        positions.swap(1, 2);
        -face_normal
    } else {
        face_normal
    };

    let mut normals = [Vec3::ZERO; 3];
    for (normal, position) in normals.iter_mut().zip(positions) {
        *normal = normalized_gradient(field, position, gradient_step)?.unwrap_or(final_face_normal);
    }
    builder.push_triangle(positions, normals)
}

fn normalized_gradient(
    field: &impl ScalarField,
    point: Vec3,
    step: f32,
) -> Result<Option<Vec3>, MeshError> {
    let dx = sample_checked(field, point + Vec3::X * step)?
        - sample_checked(field, point - Vec3::X * step)?;
    let dy = sample_checked(field, point + Vec3::Y * step)?
        - sample_checked(field, point - Vec3::Y * step)?;
    let dz = sample_checked(field, point + Vec3::Z * step)?
        - sample_checked(field, point - Vec3::Z * step)?;
    Ok(normalize_or_none(Vec3::new(dx, dy, dz)))
}

fn normalize_or_none(vector: Vec3) -> Option<Vec3> {
    if !vec3_is_finite(vector) {
        return None;
    }
    let length = vector.length();
    if !length.is_finite()
        || length <= MIN_NORMAL_LENGTH
        || vector.length_squared() <= MIN_TRIANGLE_AREA_NORMAL
    {
        None
    } else {
        Some(vector / length)
    }
}

fn sample_checked(field: &impl ScalarField, point: Vec3) -> Result<f32, MeshError> {
    let value = field.sample(point);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MeshError::NonFiniteSample {
            point: point.to_array(),
            value,
        })
    }
}

fn validate_mesh(mesh: &TriangleMesh) -> Result<(), MeshError> {
    if mesh.positions.len() != mesh.normals.len() {
        return Err(MeshError::InvalidMesh(
            "position and normal counts must match".to_owned(),
        ));
    }
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(MeshError::InvalidMesh(
            "index count must be divisible by three".to_owned(),
        ));
    }
    if !mesh.bounds.is_empty()
        && (!vec3_is_finite(mesh.bounds.min) || !vec3_is_finite(mesh.bounds.max))
    {
        return Err(MeshError::InvalidMesh(
            "mesh bounds must be finite or empty".to_owned(),
        ));
    }
    for position in &mesh.positions {
        if !array_is_finite(*position) {
            return Err(MeshError::InvalidMesh(
                "all positions must be finite".to_owned(),
            ));
        }
    }
    for normal in &mesh.normals {
        if !array_is_finite(*normal) {
            return Err(MeshError::InvalidMesh(
                "all normals must be finite".to_owned(),
            ));
        }
    }
    for index in &mesh.indices {
        if *index as usize >= mesh.positions.len() || *index as usize >= mesh.normals.len() {
            return Err(MeshError::InvalidMesh(
                "all indices must reference existing vertices and normals".to_owned(),
            ));
        }
    }
    Ok(())
}

fn obj_index(index: u32) -> Result<u32, MeshError> {
    index.checked_add(1).ok_or(MeshError::IndexOverflow)
}

fn atomic_obj_replace(
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> Result<(), MeshError>,
) -> Result<(), MeshError> {
    cleanup_obsolete_obj_temp_files(path);

    let mut temp = TempSibling::create(path)?;

    write_temp(temp.file_mut()).map_err(|error| attach_path(error, path))?;
    temp.file_mut()
        .sync_all()
        .map_err(|source| path_io(path, source))?;
    temp.persist(path).map_err(|source| path_io(path, source))?;

    cleanup_obsolete_obj_temp_files(path);
    Ok(())
}

struct TempSibling {
    path: PathBuf,
    file: Option<File>,
    persisted: bool,
}

impl TempSibling {
    fn create(target: &Path) -> Result<Self, MeshError> {
        let parent = sibling_directory(target);
        let prefix = obj_temp_prefix(target);
        let process_id = process::id();

        for _ in 0..100 {
            let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("{prefix}{process_id}-{counter}{TEMP_FILE_SUFFIX}"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        persisted: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(path_io(target, error)),
            }
        }

        Err(path_io(
            target,
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not allocate a unique temporary filename",
            ),
        ))
    }

    fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("temporary file handle must remain open until persist")
    }

    fn persist(mut self, target: &Path) -> std::io::Result<()> {
        drop(self.file.take());
        fs::rename(&self.path, target)?;
        self.persisted = true;
        Ok(())
    }
}

impl Drop for TempSibling {
    fn drop(&mut self) {
        if !self.persisted {
            drop(self.file.take());
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn cleanup_obsolete_obj_temp_files(path: &Path) {
    let prefix = obj_temp_prefix(path);
    let Ok(entries) = fs::read_dir(sibling_directory(path)) else {
        return;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if !file_name.starts_with(&prefix) || !file_name.ends_with(TEMP_FILE_SUFFIX) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_file() && obsolete_temp_metadata(&metadata) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn obsolete_temp_metadata(metadata: &fs::Metadata) -> bool {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= OBSOLETE_TEMP_MIN_AGE)
}

fn sibling_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn obj_temp_prefix(path: &Path) -> String {
    format!("{OBJ_TEMP_PREFIX}{}-", path_file_fragment(path))
}

fn path_file_fragment(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(safe_file_fragment)
        .filter(|fragment| !fragment.is_empty())
        .unwrap_or_else(|| "untitled".to_owned())
}

fn safe_file_fragment(value: &str) -> String {
    let mut fragment = String::new();
    let mut pending_separator = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !fragment.is_empty() {
                fragment.push('-');
            }
            fragment.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !fragment.is_empty() {
            pending_separator = true;
        }

        if fragment.len() >= 48 {
            break;
        }
    }

    fragment.trim_matches('-').to_owned()
}

fn path_io(path: &Path, source: std::io::Error) -> MeshError {
    MeshError::PathIo {
        path: path.to_path_buf(),
        source,
    }
}

fn attach_path(error: MeshError, path: &Path) -> MeshError {
    match error {
        MeshError::Io(source) => path_io(path, source),
        other => other,
    }
}

fn vec3_is_finite(vector: Vec3) -> bool {
    vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
}

fn array_is_finite(array: [f32; 3]) -> bool {
    array[0].is_finite() && array[1].is_finite() && array[2].is_finite()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[derive(Debug, Copy, Clone)]
    struct SphereField {
        radius: f32,
    }

    impl ScalarField for SphereField {
        fn sample(&self, point: Vec3) -> f32 {
            point.length() - self.radius
        }

        fn bounds(&self) -> Aabb {
            Aabb {
                min: Vec3::splat(-self.radius),
                max: Vec3::splat(self.radius),
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    struct ConstantField {
        value: f32,
        bounds: Aabb,
    }

    impl ScalarField for ConstantField {
        fn sample(&self, _point: Vec3) -> f32 {
            self.value
        }

        fn bounds(&self) -> Aabb {
            self.bounds
        }
    }

    #[derive(Debug, Copy, Clone)]
    struct NonFiniteField;

    impl ScalarField for NonFiniteField {
        fn sample(&self, _point: Vec3) -> f32 {
            f32::NAN
        }

        fn bounds(&self) -> Aabb {
            unit_bounds()
        }
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

    fn tempdir() -> TestTempDir {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("shape-lab-mesh-tests-{nonce}"));
        fs::create_dir(&path).unwrap();
        TestTempDir { path }
    }

    #[test]
    fn sphere_produces_nonempty_mesh() {
        let mesh = sphere_mesh(18);

        assert!(!mesh.positions.is_empty());
        assert!(!mesh.normals.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert_eq!(mesh.positions.len(), mesh.normals.len());
    }

    #[test]
    fn all_indices_are_in_range() {
        let mesh = sphere_mesh(16);

        for index in &mesh.indices {
            assert!((*index as usize) < mesh.positions.len());
            assert!((*index as usize) < mesh.normals.len());
        }
    }

    #[test]
    fn positions_and_normals_are_finite() {
        let mesh = sphere_mesh(16);

        assert!(mesh.positions.iter().copied().all(array_is_finite));
        assert!(mesh.normals.iter().copied().all(array_is_finite));
    }

    #[test]
    fn sphere_normals_generally_face_outward() {
        let mesh = sphere_mesh(18);
        let outward_count = mesh
            .positions
            .iter()
            .zip(&mesh.normals)
            .filter(|(position, normal)| {
                let position = Vec3::from_array(**position);
                let normal = Vec3::from_array(**normal);
                position.normalize_or_zero().dot(normal) > 0.7
            })
            .count();

        assert!(outward_count * 10 > mesh.positions.len() * 9);
    }

    #[test]
    fn field_with_no_crossing_returns_empty_mesh() {
        let field = ConstantField {
            value: 1.0,
            bounds: unit_bounds(),
        };
        let mesh = mesh_field(
            &field,
            MeshSettings {
                resolution: 8,
                padding_fraction: 0.0,
                iso_value: 0.0,
            },
        )
        .expect("constant positive field should mesh as empty");

        assert!(mesh.positions.is_empty());
        assert!(mesh.normals.is_empty());
        assert!(mesh.indices.is_empty());
        assert!(mesh.bounds.is_empty());
    }

    #[test]
    fn higher_resolution_produces_more_detail() {
        let low = sphere_mesh(5);
        let high = sphere_mesh(14);

        assert!(high.indices.len() > low.indices.len());
        assert!(high.positions.len() > low.positions.len());
    }

    #[test]
    fn obj_output_has_vertices_normals_and_faces() {
        let mesh = sphere_mesh(8);
        let obj = obj_string(&mesh);

        assert!(obj.starts_with("# Shape Lab generated OBJ\n"));
        assert!(obj.lines().any(|line| line == "# format wavefront-obj"));
        assert!(obj.lines().any(|line| line.starts_with("# vertex_count ")));
        assert!(obj.lines().any(|line| line.starts_with("# normal_count ")));
        assert!(
            obj.lines()
                .any(|line| line.starts_with("# triangle_count "))
        );
        assert!(obj.lines().any(|line| line.starts_with("# bounds min ")));
        assert!(obj.lines().any(|line| line.starts_with("v ")));
        assert!(obj.lines().any(|line| line.starts_with("vn ")));
        assert!(obj.lines().any(|line| line.starts_with("f ")));
        assert!(obj.lines().any(|line| line.contains("//")));
    }

    #[test]
    fn invalid_settings_fail_without_panic() {
        let field = SphereField { radius: 1.0 };

        let invalid_resolution = mesh_field(
            &field,
            MeshSettings {
                resolution: 0,
                ..MeshSettings::default()
            },
        );
        assert!(matches!(
            invalid_resolution,
            Err(MeshError::InvalidSettings(_))
        ));

        let invalid_padding = mesh_field(
            &field,
            MeshSettings {
                padding_fraction: -0.1,
                ..MeshSettings::default()
            },
        );
        assert!(matches!(
            invalid_padding,
            Err(MeshError::InvalidSettings(_))
        ));

        let invalid_iso = mesh_field(
            &field,
            MeshSettings {
                iso_value: f32::NAN,
                ..MeshSettings::default()
            },
        );
        assert!(matches!(invalid_iso, Err(MeshError::InvalidSettings(_))));

        assert!(matches!(
            mesh_field(&NonFiniteField, MeshSettings::default()),
            Err(MeshError::NonFiniteSample { .. })
        ));
    }

    #[test]
    fn empty_bounds_fail_without_panic() {
        let field = ConstantField {
            value: 1.0,
            bounds: Aabb::empty(),
        };

        assert!(matches!(
            mesh_field(&field, MeshSettings::default()),
            Err(MeshError::InvalidBounds(_))
        ));
    }

    #[test]
    fn deterministic_settings_produce_byte_identical_obj() {
        let first = sphere_mesh(12);
        let second = sphere_mesh(12);

        assert_eq!(obj_string(&first), obj_string(&second));
    }

    #[test]
    fn obj_round_trips_exported_triangle_mesh() {
        let mesh = triangle_mesh();
        let obj = obj_string(&mesh);

        let imported = read_obj(Cursor::new(obj)).unwrap();

        assert_eq!(imported.positions, mesh.positions);
        assert_eq!(imported.indices, mesh.indices);
        assert_eq!(imported.bounds, mesh.bounds);
        assert_eq!(imported.normals.len(), imported.positions.len());
    }

    #[test]
    fn obj_reader_accepts_common_face_index_forms() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
f 1/1/1 2//1 -1
";

        let imported = read_obj(Cursor::new(obj)).unwrap();

        assert_eq!(imported.positions.len(), 3);
        assert_eq!(imported.indices, vec![0, 1, 2]);
    }

    #[test]
    fn obj_reader_rejects_non_triangular_faces() {
        let obj = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";

        let error = read_obj(Cursor::new(obj)).unwrap_err();

        assert!(matches!(error, MeshError::InvalidMesh(_)));
        assert!(error.to_string().contains("triangular"));
    }

    #[test]
    fn obj_reader_rejects_malformed_face_vertex_tokens() {
        for token in ["1/", "1/2/", "1//", "/1/2", "1/0/2"] {
            let obj = format!(
                "\
v 0 0 0
v 1 0 0
v 0 1 0
f {token} 2 3
"
            );

            let error = read_obj(Cursor::new(obj)).unwrap_err();

            assert!(matches!(error, MeshError::InvalidMesh(_)));
        }
    }

    #[test]
    fn obj_export_atomically_replaces_existing_file() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("mesh.obj");
        fs::write(&path, "old obj\n").unwrap();
        let mesh = sphere_mesh(6);

        write_obj_to_path(&mesh, &path).unwrap();
        let saved = fs::read_to_string(&path).unwrap();

        assert!(saved.starts_with("# Shape Lab generated OBJ\n"));
        assert_ne!(saved, "old obj\n");
    }

    #[test]
    fn interrupted_obj_temp_write_preserves_existing_file() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("mesh.obj");
        fs::write(&path, "valid old obj\n").unwrap();

        let error = atomic_obj_replace(&path, |file| {
            file.write_all(b"# partial obj\n")?;
            Err(MeshError::Io(io::Error::new(
                io::ErrorKind::Interrupted,
                "simulated interrupted write",
            )))
        })
        .unwrap_err();

        assert!(matches!(error, MeshError::PathIo { .. }));
        assert_eq!(fs::read_to_string(&path).unwrap(), "valid old obj\n");
    }

    #[test]
    fn invalid_mesh_export_preserves_existing_file() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("mesh.obj");
        fs::write(&path, "valid old obj\n").unwrap();
        let mesh = TriangleMesh {
            positions: vec![[0.0, 0.0, 0.0]],
            normals: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::empty(),
        };

        let error = write_obj_to_path(&mesh, &path).unwrap_err();

        assert!(matches!(error, MeshError::InvalidMesh(_)));
        assert_eq!(fs::read_to_string(&path).unwrap(), "valid old obj\n");
    }

    #[test]
    fn obj_temp_prefix_is_scoped_to_target_name() {
        let temp_dir = tempdir();
        let path = temp_dir.path().join("Unsafe Mesh Name!.obj");

        assert_eq!(
            obj_temp_prefix(&path),
            ".shape-lab-obj-unsafe-mesh-name-obj-"
        );
    }

    fn sphere_mesh(resolution: usize) -> TriangleMesh {
        mesh_field(
            &SphereField { radius: 1.0 },
            MeshSettings {
                resolution,
                padding_fraction: 0.2,
                iso_value: 0.0,
            },
        )
        .expect("sphere mesh should be generated")
    }

    fn triangle_mesh() -> TriangleMesh {
        TriangleMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::ZERO,
                max: Vec3::new(1.0, 1.0, 0.0),
            },
        }
    }

    fn obj_string(mesh: &TriangleMesh) -> String {
        write_obj_to_string(mesh).expect("OBJ should be valid UTF-8")
    }

    fn unit_bounds() -> Aabb {
        Aabb {
            min: Vec3::splat(-1.0),
            max: Vec3::splat(1.0),
        }
    }
}
