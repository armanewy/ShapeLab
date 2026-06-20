#![forbid(unsafe_code)]

//! Uniform-grid mesh generation and OBJ export.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use glam::Vec3;
use shape_core::Aabb;
use shape_field::ScalarField;
use thiserror::Error;

const MAX_GRID_SAMPLES: usize = 16_777_216;
const MIN_GRADIENT_STEP: f32 = 1.0e-4;
const MIN_NORMAL_LENGTH: f32 = 1.0e-6;
const MIN_TRIANGLE_AREA_NORMAL: f32 = 1.0e-8;

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
    #[error("io error while writing {path}: {source}")]
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
    let file = File::create(path).map_err(|source| MeshError::PathIo {
        path: path.to_path_buf(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    write_obj(mesh, &mut writer).map_err(|error| attach_path(error, path))?;
    writer.flush().map_err(|source| MeshError::PathIo {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Return OBJ text as a string for callers that need an in-memory export.
pub fn write_obj_to_string(mesh: &TriangleMesh) -> Result<String, MeshError> {
    let mut bytes = Vec::new();
    write_obj(mesh, &mut bytes)?;
    String::from_utf8(bytes).map_err(|error| MeshError::InvalidMesh(error.to_string()))
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

fn attach_path(error: MeshError, path: &Path) -> MeshError {
    match error {
        MeshError::Io(source) => MeshError::PathIo {
            path: path.to_path_buf(),
            source,
        },
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
