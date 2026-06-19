#![forbid(unsafe_code)]

//! Mesh generation and OBJ export contracts.

use std::fmt::Write as _;
use std::path::Path;

use shape_core::Aabb;
use shape_field::ScalarField;
use thiserror::Error;

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
    /// Uniform grid resolution.
    pub resolution: usize,
    /// Fractional padding around field bounds.
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
    /// The requested operation belongs to a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Mesh an implicit scalar field.
pub fn mesh_field(
    _field: &impl ScalarField,
    _settings: MeshSettings,
) -> Result<TriangleMesh, MeshError> {
    Err(MeshError::NotImplemented("marching tetrahedra"))
}

/// Write a mesh as OBJ text.
pub fn write_obj(mesh: &TriangleMesh) -> Result<String, MeshError> {
    let mut output = String::new();
    output.push_str("# Shape Lab OBJ export\n");
    for position in &mesh.positions {
        writeln!(output, "v {} {} {}", position[0], position[1], position[2])
            .map_err(|_| MeshError::NotImplemented("string formatting"))?;
    }
    for normal in &mesh.normals {
        writeln!(output, "vn {} {} {}", normal[0], normal[1], normal[2])
            .map_err(|_| MeshError::NotImplemented("string formatting"))?;
    }
    for triangle in mesh.indices.chunks(3) {
        if triangle.len() == 3 {
            writeln!(
                output,
                "f {0}//{0} {1}//{1} {2}//{2}",
                triangle[0] + 1,
                triangle[1] + 1,
                triangle[2] + 1
            )
            .map_err(|_| MeshError::NotImplemented("string formatting"))?;
        }
    }
    Ok(output)
}

/// Write OBJ text to a path.
pub fn write_obj_to_path(mesh: &TriangleMesh, path: impl AsRef<Path>) -> Result<(), MeshError> {
    std::fs::write(path, write_obj(mesh)?)?;
    Ok(())
}
