#![forbid(unsafe_code)]

//! Implicit scalar field contracts.

use glam::Vec3;
use shape_core::{Aabb, ShapeDocument};
use thiserror::Error;

/// Thread-safe scalar field sampled in world space.
pub trait ScalarField: Send + Sync {
    /// Sample signed distance at a point. Negative values are inside.
    fn sample(&self, point: Vec3) -> f32;

    /// Conservative world-space bounds.
    fn bounds(&self) -> Aabb;
}

/// Immutable compiled field arena.
#[derive(Debug, Clone)]
pub struct CompiledField {
    document: ShapeDocument,
}

impl CompiledField {
    /// Return the source document used by the bootstrap stub.
    #[must_use]
    pub fn document(&self) -> &ShapeDocument {
        &self.document
    }
}

impl ScalarField for CompiledField {
    fn sample(&self, _point: Vec3) -> f32 {
        f32::INFINITY
    }

    fn bounds(&self) -> Aabb {
        Aabb::empty()
    }
}

/// Field compilation and sampling errors.
#[derive(Debug, Error)]
pub enum FieldCompileError {
    /// The source document is invalid.
    #[error("invalid shape document")]
    InvalidDocument,
    /// The requested operation belongs to a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    /// Grid settings are invalid.
    #[error("invalid grid: {0}")]
    InvalidGrid(String),
}

/// Uniform grid sampling specification.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GridSpec {
    /// Sampling bounds.
    pub bounds: Aabb,
    /// Samples along X.
    pub resolution_x: usize,
    /// Samples along Y.
    pub resolution_y: usize,
    /// Samples along Z.
    pub resolution_z: usize,
}

/// Sampled grid values in deterministic X-major, then Y, then Z order.
#[derive(Debug, Clone, PartialEq)]
pub struct GridSamples {
    /// Grid specification.
    pub spec: GridSpec,
    /// Scalar values.
    pub values: Vec<f32>,
}

/// Compile a shape document into a scalar field.
pub fn compile_document(document: &ShapeDocument) -> Result<CompiledField, FieldCompileError> {
    let report = shape_core::validate_document(document);
    if !report.is_valid() {
        return Err(FieldCompileError::InvalidDocument);
    }
    Ok(CompiledField {
        document: document.clone(),
    })
}

/// Sample a field on a uniform grid.
pub fn sample_grid(
    field: &impl ScalarField,
    spec: GridSpec,
) -> Result<GridSamples, FieldCompileError> {
    if spec.resolution_x == 0 || spec.resolution_y == 0 || spec.resolution_z == 0 {
        return Err(FieldCompileError::InvalidGrid(
            "all resolutions must be positive".to_owned(),
        ));
    }
    let count = spec
        .resolution_x
        .checked_mul(spec.resolution_y)
        .and_then(|value| value.checked_mul(spec.resolution_z))
        .ok_or_else(|| FieldCompileError::InvalidGrid("grid is too large".to_owned()))?;
    if count > 16_777_216 {
        return Err(FieldCompileError::InvalidGrid(
            "grid is too large".to_owned(),
        ));
    }
    let mut values = Vec::with_capacity(count);
    let extent = spec.bounds.extent();
    for z in 0..spec.resolution_z {
        for y in 0..spec.resolution_y {
            for x in 0..spec.resolution_x {
                let denom = Vec3::new(
                    spec.resolution_x.saturating_sub(1).max(1) as f32,
                    spec.resolution_y.saturating_sub(1).max(1) as f32,
                    spec.resolution_z.saturating_sub(1).max(1) as f32,
                );
                let t = Vec3::new(x as f32, y as f32, z as f32) / denom;
                values.push(field.sample(spec.bounds.min + extent * t));
            }
        }
    }
    Ok(GridSamples { spec, values })
}
