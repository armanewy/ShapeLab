//! Deterministic assembly evaluation for part instances, sockets, mirrors, and arrays.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetRecipe, AttachmentMode, Frame3, ModelingOperationSpec, OperationId, PartDefinition,
    PartDefinitionId, PartInstance, PartInstanceId, RegionId, SocketId, SocketSpec, Transform3,
};
use shape_poly::{
    ElementId, FaceMetadata, MeshBounds, PolyError, PolygonMesh, TriangulatedPolygonMesh,
    bounds_from_positions, combine_polygon_meshes, compute_topology_signature,
    triangulate_polygon_mesh,
};
use thiserror::Error;

use crate::{GeneratedPart, GeneratorContext, GeometryGenerator, ModelingError, generate_geometry};

const EPSILON: f32 = 1.0e-6;

/// Deterministic assembly operation plan layered on top of an [`AssetRecipe`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssemblyPlan {
    /// Ordered assembly operations.
    pub operations: Vec<AssemblyOperation>,
}

impl AssemblyPlan {
    /// Build an assembly plan from operation specs that already exist on part definitions.
    ///
    /// The current asset contract stores only a basic subset of assembly fields,
    /// so this maps those specs to non-centered arrays and origin-centered radial
    /// arrays. Callers that need centered arrays or explicit radial centers should
    /// pass a richer `AssemblyPlan` to [`evaluate_assembly_plan_with_generator`].
    #[must_use]
    pub fn from_recipe_operations(recipe: &AssetRecipe) -> Self {
        let mut operations = Vec::new();
        for definition in recipe.definitions.values() {
            let prototypes = recipe
                .instances
                .values()
                .filter(|instance| instance.enabled && instance.definition == definition.id)
                .map(|instance| instance.id)
                .collect::<Vec<_>>();
            if prototypes.is_empty() {
                continue;
            }
            for operation in &definition.geometry.operations {
                match operation {
                    ModelingOperationSpec::MirrorInstances {
                        operation,
                        plane_normal,
                        plane_offset,
                    } => operations.push(AssemblyOperation::Mirror(MirrorOperation {
                        operation: *operation,
                        prototypes: prototypes.clone(),
                        plane: MirrorPlane {
                            normal: *plane_normal,
                            offset: *plane_offset,
                        },
                    })),
                    ModelingOperationSpec::LinearArray {
                        operation,
                        count,
                        offset,
                    } => operations.push(AssemblyOperation::LinearArray(LinearArrayOperation {
                        operation: *operation,
                        prototypes: prototypes.clone(),
                        count: *count,
                        step: Transform3 {
                            translation: *offset,
                            ..Transform3::default()
                        },
                        centered: false,
                    })),
                    ModelingOperationSpec::RadialArray {
                        operation,
                        count,
                        axis,
                        angle_degrees,
                    } => operations.push(AssemblyOperation::RadialArray(RadialArrayOperation {
                        operation: *operation,
                        prototypes: prototypes.clone(),
                        count: *count,
                        center: [0.0, 0.0, 0.0],
                        axis: *axis,
                        angular_span_degrees: *angle_degrees,
                        rotate_instances: true,
                    })),
                    ModelingOperationSpec::TransformGeometry { .. }
                    | ModelingOperationSpec::SetBevelProfile { .. }
                    | ModelingOperationSpec::AddPanel { .. }
                    | ModelingOperationSpec::AddTrim { .. }
                    | ModelingOperationSpec::RecessedPanelCut { .. }
                    | ModelingOperationSpec::RectangularThroughCut { .. }
                    | ModelingOperationSpec::CircularThroughCut { .. }
                    | ModelingOperationSpec::ReservedBoolean { .. }
                    | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
                }
            }
        }
        Self { operations }
    }
}

/// One deterministic assembly operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssemblyOperation {
    /// Mirror prototypes across an explicit plane.
    Mirror(MirrorOperation),
    /// Generate a deterministic linear array from prototypes.
    LinearArray(LinearArrayOperation),
    /// Generate a deterministic radial array from prototypes.
    RadialArray(RadialArrayOperation),
}

/// Plane used by mirror operations.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorPlane {
    /// Plane normal. It is normalized during evaluation.
    pub normal: [f32; 3],
    /// Signed plane offset from the origin along the normal.
    pub offset: f32,
}

/// Mirror prototypes across an explicit plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorOperation {
    /// Stable operation provenance ID.
    pub operation: OperationId,
    /// Prototype instances to mirror, in deterministic operation order.
    pub prototypes: Vec<PartInstanceId>,
    /// Mirror plane.
    pub plane: MirrorPlane,
}

/// Linear array settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearArrayOperation {
    /// Stable operation provenance ID.
    pub operation: OperationId,
    /// Prototype instances to array, in deterministic operation order.
    pub prototypes: Vec<PartInstanceId>,
    /// Total occurrence count including the prototype.
    pub count: u32,
    /// Step transform between adjacent occurrences.
    pub step: Transform3,
    /// When true, generated copy indices are distributed around prototype index zero.
    pub centered: bool,
}

/// Radial array settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialArrayOperation {
    /// Stable operation provenance ID.
    pub operation: OperationId,
    /// Prototype instances to array, in deterministic operation order.
    pub prototypes: Vec<PartInstanceId>,
    /// Total occurrence count including the prototype.
    pub count: u32,
    /// Rotation center in assembly coordinates.
    pub center: [f32; 3],
    /// Rotation axis in assembly coordinates.
    pub axis: [f32; 3],
    /// Angular span covered by the array.
    pub angular_span_degrees: f32,
    /// Rotate instance orientation with the radial placement.
    pub rotate_instances: bool,
}

/// Row-major affine 4x4 transform.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffineTransform3 {
    /// Matrix rows.
    pub matrix: [[f32; 4]; 4],
}

impl Default for AffineTransform3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl AffineTransform3 {
    /// Identity transform.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Build an affine transform matching the public `Transform3` contract.
    #[must_use]
    pub fn from_transform(transform: &Transform3) -> Self {
        let origin = transform.transform_point([0.0, 0.0, 0.0]);
        let x_axis = transform.transform_vector([1.0, 0.0, 0.0]);
        let y_axis = transform.transform_vector([0.0, 1.0, 0.0]);
        let z_axis = transform.transform_vector([0.0, 0.0, 1.0]);
        Self::from_frame(&Frame3 {
            origin,
            x_axis,
            y_axis,
            z_axis,
        })
    }

    /// Build an affine transform from a frame whose axes are the transform basis.
    #[must_use]
    pub fn from_frame(frame: &Frame3) -> Self {
        Self {
            matrix: [
                [
                    frame.x_axis[0],
                    frame.y_axis[0],
                    frame.z_axis[0],
                    frame.origin[0],
                ],
                [
                    frame.x_axis[1],
                    frame.y_axis[1],
                    frame.z_axis[1],
                    frame.origin[1],
                ],
                [
                    frame.x_axis[2],
                    frame.y_axis[2],
                    frame.z_axis[2],
                    frame.origin[2],
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Translation transform.
    #[must_use]
    pub fn translation(translation: [f32; 3]) -> Self {
        let mut transform = Self::identity();
        transform.matrix[0][3] = translation[0];
        transform.matrix[1][3] = translation[1];
        transform.matrix[2][3] = translation[2];
        transform
    }

    /// Reflection transform across an explicit plane.
    pub fn reflection(plane: MirrorPlane) -> Result<Self, AssemblyError> {
        let normal = normalize(plane.normal).ok_or_else(|| {
            AssemblyError::InvalidInput("mirror plane normal must be non-zero".to_owned())
        })?;
        if !plane.offset.is_finite() {
            return Err(AssemblyError::InvalidInput(
                "mirror plane offset must be finite".to_owned(),
            ));
        }
        let [nx, ny, nz] = normal;
        let mut matrix = Self::identity().matrix;
        matrix[0][0] = 1.0 - 2.0 * nx * nx;
        matrix[0][1] = -2.0 * nx * ny;
        matrix[0][2] = -2.0 * nx * nz;
        matrix[1][0] = -2.0 * ny * nx;
        matrix[1][1] = 1.0 - 2.0 * ny * ny;
        matrix[1][2] = -2.0 * ny * nz;
        matrix[2][0] = -2.0 * nz * nx;
        matrix[2][1] = -2.0 * nz * ny;
        matrix[2][2] = 1.0 - 2.0 * nz * nz;
        let translation = scale(normal, 2.0 * plane.offset);
        matrix[0][3] = translation[0];
        matrix[1][3] = translation[1];
        matrix[2][3] = translation[2];
        Ok(Self { matrix })
    }

    /// Rotation around an axis and center.
    pub fn rotation_about_axis(
        center: [f32; 3],
        axis: [f32; 3],
        angle_degrees: f32,
    ) -> Result<Self, AssemblyError> {
        if !array_is_finite(center) || !angle_degrees.is_finite() {
            return Err(AssemblyError::InvalidInput(
                "radial array center and angle must be finite".to_owned(),
            ));
        }
        let axis = normalize(axis).ok_or_else(|| {
            AssemblyError::InvalidInput("radial array axis must be non-zero".to_owned())
        })?;
        let angle = angle_degrees.to_radians();
        let (sin, cos) = angle.sin_cos();
        let [x, y, z] = axis;
        let one_minus_cos = 1.0 - cos;
        let rotation = Self {
            matrix: [
                [
                    cos + x * x * one_minus_cos,
                    x * y * one_minus_cos - z * sin,
                    x * z * one_minus_cos + y * sin,
                    0.0,
                ],
                [
                    y * x * one_minus_cos + z * sin,
                    cos + y * y * one_minus_cos,
                    y * z * one_minus_cos - x * sin,
                    0.0,
                ],
                [
                    z * x * one_minus_cos - y * sin,
                    z * y * one_minus_cos + x * sin,
                    cos + z * z * one_minus_cos,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        Ok(Self::translation(center)
            .compose(&rotation)
            .compose(&Self::translation(scale(center, -1.0))))
    }

    /// Compose `self` after `other`; the resulting transform applies `other` first.
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        let mut matrix = [[0.0; 4]; 4];
        for (row, values) in matrix.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                *value = self.matrix[row][0] * other.matrix[0][column]
                    + self.matrix[row][1] * other.matrix[1][column]
                    + self.matrix[row][2] * other.matrix[2][column]
                    + self.matrix[row][3] * other.matrix[3][column];
            }
        }
        Self { matrix }
    }

    /// Transform a point.
    #[must_use]
    pub fn transform_point(&self, point: [f32; 3]) -> [f32; 3] {
        [
            self.matrix[0][0] * point[0]
                + self.matrix[0][1] * point[1]
                + self.matrix[0][2] * point[2]
                + self.matrix[0][3],
            self.matrix[1][0] * point[0]
                + self.matrix[1][1] * point[1]
                + self.matrix[1][2] * point[2]
                + self.matrix[1][3],
            self.matrix[2][0] * point[0]
                + self.matrix[2][1] * point[1]
                + self.matrix[2][2] * point[2]
                + self.matrix[2][3],
        ]
    }

    /// Transform a vector.
    #[must_use]
    pub fn transform_vector(&self, vector: [f32; 3]) -> [f32; 3] {
        [
            self.matrix[0][0] * vector[0]
                + self.matrix[0][1] * vector[1]
                + self.matrix[0][2] * vector[2],
            self.matrix[1][0] * vector[0]
                + self.matrix[1][1] * vector[1]
                + self.matrix[1][2] * vector[2],
            self.matrix[2][0] * vector[0]
                + self.matrix[2][1] * vector[1]
                + self.matrix[2][2] * vector[2],
        ]
    }

    /// Transform a coordinate frame.
    #[must_use]
    pub fn transform_frame(&self, frame: &Frame3) -> Frame3 {
        Frame3 {
            origin: self.transform_point(frame.origin),
            x_axis: self.transform_vector(frame.x_axis),
            y_axis: self.transform_vector(frame.y_axis),
            z_axis: self.transform_vector(frame.z_axis),
        }
    }

    /// Return the affine inverse.
    pub fn inverse(&self) -> Result<Self, AssemblyError> {
        let a = [
            [self.matrix[0][0], self.matrix[0][1], self.matrix[0][2]],
            [self.matrix[1][0], self.matrix[1][1], self.matrix[1][2]],
            [self.matrix[2][0], self.matrix[2][1], self.matrix[2][2]],
        ];
        let inverse = inverse_3x3(a)
            .ok_or_else(|| AssemblyError::InvalidInput("transform is not invertible".to_owned()))?;
        let translation = [self.matrix[0][3], self.matrix[1][3], self.matrix[2][3]];
        let inverse_translation = mul_mat3_vec(inverse, scale(translation, -1.0));
        Ok(Self {
            matrix: [
                [
                    inverse[0][0],
                    inverse[0][1],
                    inverse[0][2],
                    inverse_translation[0],
                ],
                [
                    inverse[1][0],
                    inverse[1][1],
                    inverse[1][2],
                    inverse_translation[1],
                ],
                [
                    inverse[2][0],
                    inverse[2][1],
                    inverse[2][2],
                    inverse_translation[2],
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        })
    }

    /// Return the linear determinant.
    #[must_use]
    pub fn determinant(&self) -> f32 {
        determinant_3x3([
            [self.matrix[0][0], self.matrix[0][1], self.matrix[0][2]],
            [self.matrix[1][0], self.matrix[1][1], self.matrix[1][2]],
            [self.matrix[2][0], self.matrix[2][1], self.matrix[2][2]],
        ])
    }
}

/// Local part compiled exactly once per definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyCompiledPart {
    /// Source part definition.
    pub definition_id: PartDefinitionId,
    /// Local polygon mesh generated for the definition.
    pub local_mesh: PolygonMesh,
    /// Local sockets declared or generated for the definition.
    pub sockets: BTreeMap<SocketId, SocketSpec>,
    /// Local regions declared or generated for the definition.
    pub regions: BTreeMap<RegionId, shape_asset::SurfaceRegionSpec>,
    /// Local bounds.
    pub local_bounds: MeshBounds,
    /// Deterministic generator signature.
    pub generator_signature: String,
}

/// One assembled occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyInstance {
    /// Occurrence instance ID.
    pub instance_id: PartInstanceId,
    /// Referenced part definition.
    pub definition_id: PartDefinitionId,
    /// Source prototype for generated occurrences.
    pub prototype_instance_id: Option<PartInstanceId>,
    /// Operation that generated this occurrence.
    pub generated_by: Option<OperationId>,
    /// Whether this occurrence came directly from the recipe.
    pub source_recipe_instance: bool,
}

/// Provenance for one assembled occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyInstanceProvenance {
    /// Occurrence instance ID.
    pub instance_id: PartInstanceId,
    /// Referenced part definition.
    pub definition_id: PartDefinitionId,
    /// Source prototype for generated occurrences.
    pub prototype_instance_id: Option<PartInstanceId>,
    /// Operation that generated this occurrence.
    pub generated_by: Option<OperationId>,
    /// World-space polygon vertex count.
    pub polygon_vertex_count: u64,
    /// World-space polygon face count.
    pub polygon_face_count: u64,
}

/// Deterministic assembly provenance summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyProvenance {
    /// Definition generation order.
    pub definition_generation_order: Vec<PartDefinitionId>,
    /// Occurrence output order.
    pub instance_order: Vec<PartInstanceId>,
    /// Per-occurrence provenance.
    pub instances: Vec<AssemblyInstanceProvenance>,
}

/// Complete deterministic assembly evaluation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyEvaluation {
    /// Local parts compiled once per definition.
    pub local_parts: BTreeMap<PartDefinitionId, AssemblyCompiledPart>,
    /// Assembled occurrences in deterministic output order.
    pub instances: Vec<AssemblyInstance>,
    /// World transforms by occurrence ID.
    pub world_transforms: BTreeMap<PartInstanceId, AffineTransform3>,
    /// World-space sockets by occurrence ID.
    pub world_sockets: BTreeMap<PartInstanceId, BTreeMap<SocketId, SocketSpec>>,
    /// World-space meshes by occurrence ID.
    pub world_meshes: BTreeMap<PartInstanceId, PolygonMesh>,
    /// Combined world-space polygon mesh.
    pub combined_preview_mesh: PolygonMesh,
    /// Combined triangulated preview mesh.
    pub combined_preview: TriangulatedPolygonMesh,
    /// Per-occurrence world-space bounds.
    pub instance_bounds: BTreeMap<PartInstanceId, MeshBounds>,
    /// Assembly provenance.
    pub provenance: AssemblyProvenance,
}

/// Error type for deterministic assembly evaluation.
#[derive(Debug, Error)]
pub enum AssemblyError {
    /// Requested instance does not exist.
    #[error("unknown part instance {0:?}")]
    UnknownInstance(PartInstanceId),
    /// Requested definition does not exist.
    #[error("unknown part definition {0:?}")]
    UnknownDefinition(PartDefinitionId),
    /// Requested socket does not exist on an instance definition.
    #[error("missing socket {socket:?} on instance {instance:?}")]
    MissingSocket {
        /// Instance whose definition was queried.
        instance: PartInstanceId,
        /// Definition whose socket map was queried.
        definition: PartDefinitionId,
        /// Missing socket.
        socket: SocketId,
    },
    /// Parent or attachment graph contains a cycle.
    #[error("attachment cycle detected at instance {0:?}")]
    AttachmentCycle(PartInstanceId),
    /// Unsupported assembly feature.
    #[error("unsupported assembly feature: {feature}")]
    Unsupported {
        /// Feature label.
        feature: String,
    },
    /// Assembly input is invalid.
    #[error("invalid assembly input: {0}")]
    InvalidInput(String),
    /// Modeling dispatch failed.
    #[error("modeling error: {0}")]
    Modeling(#[from] ModelingError),
    /// Polygon topology helper failed.
    #[error("polygon error: {0}")]
    Polygon(#[from] PolyError),
}

/// Generator adapter that delegates to [`generate_geometry`].
#[derive(Debug, Copy, Clone, Default)]
pub struct DispatchGeometryGenerator;

impl GeometryGenerator for DispatchGeometryGenerator {
    fn generate(
        &self,
        definition: &PartDefinition,
        context: &mut GeneratorContext,
    ) -> Result<GeneratedPart, ModelingError> {
        generate_geometry(definition, context)
    }
}

/// Evaluate an asset recipe using the default generator dispatch and recipe-level assembly ops.
pub fn evaluate_assembly(recipe: &AssetRecipe) -> Result<AssemblyEvaluation, AssemblyError> {
    let generator = DispatchGeometryGenerator;
    evaluate_assembly_with_generator(recipe, &generator)
}

/// Evaluate an asset recipe using an injected generator and recipe-level assembly ops.
pub fn evaluate_assembly_with_generator(
    recipe: &AssetRecipe,
    generator: &impl GeometryGenerator,
) -> Result<AssemblyEvaluation, AssemblyError> {
    let plan = AssemblyPlan::from_recipe_operations(recipe);
    evaluate_assembly_plan_with_generator(recipe, &plan, generator)
}

/// Evaluate an asset recipe using an explicit assembly plan and injected generator.
pub fn evaluate_assembly_plan_with_generator(
    recipe: &AssetRecipe,
    plan: &AssemblyPlan,
    generator: &impl GeometryGenerator,
) -> Result<AssemblyEvaluation, AssemblyError> {
    validate_attachments(recipe)?;
    let enabled_instances = recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
        .map(|instance| instance.id)
        .collect::<BTreeSet<_>>();
    detect_attachment_cycles(recipe, &enabled_instances)?;
    let base_order = ordered_enabled_instances(recipe, &enabled_instances)?;
    let mut state = AssemblyState::new(recipe, generator, &enabled_instances)?;

    for instance_id in base_order {
        let transform = state.resolve_base_transform(instance_id)?;
        let instance = recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        state.push_occurrence(OccurrenceInput {
            instance_id,
            definition_id: instance.definition,
            transform,
            prototype_instance_id: None,
            generated_by: instance.generated_by,
            source_recipe_instance: true,
        })?;
    }

    for operation in &plan.operations {
        state.apply_operation(operation)?;
    }

    state.finish()
}

struct OccurrenceInput {
    instance_id: PartInstanceId,
    definition_id: PartDefinitionId,
    transform: AffineTransform3,
    prototype_instance_id: Option<PartInstanceId>,
    generated_by: Option<OperationId>,
    source_recipe_instance: bool,
}

struct AssemblyState<'a, G> {
    recipe: &'a AssetRecipe,
    local_parts: BTreeMap<PartDefinitionId, AssemblyCompiledPart>,
    instances: Vec<AssemblyInstance>,
    world_transforms: BTreeMap<PartInstanceId, AffineTransform3>,
    world_sockets: BTreeMap<PartInstanceId, BTreeMap<SocketId, SocketSpec>>,
    world_meshes: BTreeMap<PartInstanceId, PolygonMesh>,
    instance_bounds: BTreeMap<PartInstanceId, MeshBounds>,
    base_transforms: BTreeMap<PartInstanceId, AffineTransform3>,
    resolving: BTreeSet<PartInstanceId>,
    next_generated_instance_id: u64,
    definition_generation_order: Vec<PartDefinitionId>,
    _generator: &'a G,
}

impl<'a, G: GeometryGenerator> AssemblyState<'a, G> {
    fn new(
        recipe: &'a AssetRecipe,
        generator: &'a G,
        enabled_instances: &BTreeSet<PartInstanceId>,
    ) -> Result<Self, AssemblyError> {
        let mut definitions = enabled_instances
            .iter()
            .filter_map(|instance_id| recipe.instances.get(instance_id))
            .map(|instance| instance.definition)
            .collect::<BTreeSet<_>>();
        for definition in &definitions {
            if !recipe.definitions.contains_key(definition) {
                return Err(AssemblyError::UnknownDefinition(*definition));
            }
        }

        let mut local_parts = BTreeMap::new();
        let mut definition_generation_order = Vec::new();
        for definition_id in std::mem::take(&mut definitions) {
            let definition = recipe
                .definitions
                .get(&definition_id)
                .ok_or(AssemblyError::UnknownDefinition(definition_id))?;
            let context_instance = enabled_instances
                .iter()
                .filter_map(|instance_id| recipe.instances.get(instance_id))
                .find(|instance| instance.definition == definition_id)
                .map(|instance| instance.id)
                .unwrap_or_default();
            let mut context = GeneratorContext::new(
                definition_id,
                context_instance,
                recipe.next_ids.operation,
                recipe.next_ids.revision,
            );
            let generated = generator.generate(definition, &mut context)?;
            let mut sockets = definition.sockets.clone();
            sockets.extend(generated.sockets);
            let mut regions = definition.regions.clone();
            regions.extend(generated.regions);
            let local_bounds = if generated.local_bounds.is_empty() {
                generated.mesh.bounds
            } else {
                generated.local_bounds
            };
            local_parts.insert(
                definition_id,
                AssemblyCompiledPart {
                    definition_id,
                    local_mesh: generated.mesh,
                    sockets,
                    regions,
                    local_bounds,
                    generator_signature: generated.generator_signature,
                },
            );
            definition_generation_order.push(definition_id);
        }

        let max_instance_id = recipe
            .instances
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or_default();
        let next_generated_instance_id = recipe
            .next_ids
            .part_instance
            .max(max_instance_id.saturating_add(1));

        Ok(Self {
            recipe,
            local_parts,
            instances: Vec::new(),
            world_transforms: BTreeMap::new(),
            world_sockets: BTreeMap::new(),
            world_meshes: BTreeMap::new(),
            instance_bounds: BTreeMap::new(),
            base_transforms: BTreeMap::new(),
            resolving: BTreeSet::new(),
            next_generated_instance_id,
            definition_generation_order,
            _generator: generator,
        })
    }

    fn resolve_base_transform(
        &mut self,
        instance_id: PartInstanceId,
    ) -> Result<AffineTransform3, AssemblyError> {
        if let Some(transform) = self.base_transforms.get(&instance_id) {
            return Ok(*transform);
        }
        if !self.resolving.insert(instance_id) {
            return Err(AssemblyError::AttachmentCycle(instance_id));
        }

        let instance = self
            .recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        let transform = if let Some(attachment) = &instance.attachment {
            if attachment.mode != AttachmentMode::RigidSeparate {
                return Err(AssemblyError::Unsupported {
                    feature: "WeldBoundaryReserved".to_owned(),
                });
            }
            let parent_transform = self.resolve_base_transform(attachment.parent_instance)?;
            let parent = self
                .recipe
                .instances
                .get(&attachment.parent_instance)
                .ok_or(AssemblyError::UnknownInstance(attachment.parent_instance))?;
            let parent_socket =
                self.socket(parent.id, parent.definition, attachment.parent_socket)?;
            let child_socket =
                self.socket(instance.id, instance.definition, attachment.child_socket)?;
            parent_transform
                .compose(&AffineTransform3::from_frame(&parent_socket.local_frame))
                .compose(&AffineTransform3::from_transform(&attachment.local_offset))
                .compose(&AffineTransform3::from_frame(&child_socket.local_frame).inverse()?)
        } else if let Some(parent_id) = instance.parent {
            let parent_transform = self.resolve_base_transform(parent_id)?;
            parent_transform.compose(&AffineTransform3::from_transform(&instance.local_transform))
        } else {
            AffineTransform3::from_transform(&instance.local_transform)
        };

        self.resolving.remove(&instance_id);
        self.base_transforms.insert(instance_id, transform);
        Ok(transform)
    }

    fn socket(
        &self,
        instance: PartInstanceId,
        definition: PartDefinitionId,
        socket: SocketId,
    ) -> Result<&SocketSpec, AssemblyError> {
        self.local_parts
            .get(&definition)
            .ok_or(AssemblyError::UnknownDefinition(definition))?
            .sockets
            .get(&socket)
            .ok_or(AssemblyError::MissingSocket {
                instance,
                definition,
                socket,
            })
    }

    fn push_occurrence(&mut self, input: OccurrenceInput) -> Result<(), AssemblyError> {
        let local_part = self
            .local_parts
            .get(&input.definition_id)
            .ok_or(AssemblyError::UnknownDefinition(input.definition_id))?;
        let world_mesh = transform_mesh_for_instance(
            &local_part.local_mesh,
            input.definition_id,
            input.instance_id,
            input.generated_by,
            &input.transform,
        )?;
        let sockets = transform_sockets(&local_part.sockets, &input.transform);
        self.instance_bounds
            .insert(input.instance_id, world_mesh.bounds);
        self.world_transforms
            .insert(input.instance_id, input.transform);
        self.world_sockets.insert(input.instance_id, sockets);
        self.world_meshes.insert(input.instance_id, world_mesh);
        self.instances.push(AssemblyInstance {
            instance_id: input.instance_id,
            definition_id: input.definition_id,
            prototype_instance_id: input.prototype_instance_id,
            generated_by: input.generated_by,
            source_recipe_instance: input.source_recipe_instance,
        });
        Ok(())
    }

    fn apply_operation(&mut self, operation: &AssemblyOperation) -> Result<(), AssemblyError> {
        match operation {
            AssemblyOperation::Mirror(operation) => self.apply_mirror(operation),
            AssemblyOperation::LinearArray(operation) => self.apply_linear_array(operation),
            AssemblyOperation::RadialArray(operation) => self.apply_radial_array(operation),
        }
    }

    fn apply_mirror(&mut self, operation: &MirrorOperation) -> Result<(), AssemblyError> {
        let reflection = AffineTransform3::reflection(operation.plane)?;
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            let instance_id = self.allocate_generated_instance_id();
            let transform = reflection.compose(&prototype_transform);
            self.push_occurrence(OccurrenceInput {
                instance_id,
                definition_id,
                transform,
                prototype_instance_id: Some(*prototype),
                generated_by: Some(operation.operation),
                source_recipe_instance: false,
            })?;
        }
        Ok(())
    }

    fn apply_linear_array(
        &mut self,
        operation: &LinearArrayOperation,
    ) -> Result<(), AssemblyError> {
        if operation.count == 0 {
            return Err(AssemblyError::InvalidInput(
                "linear array count must be at least one".to_owned(),
            ));
        }
        let step = AffineTransform3::from_transform(&operation.step);
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            for index in linear_generated_indices(operation.count, operation.centered) {
                let instance_id = self.allocate_generated_instance_id();
                let step_transform = transform_power(&step, index)?;
                let transform = prototype_transform.compose(&step_transform);
                self.push_occurrence(OccurrenceInput {
                    instance_id,
                    definition_id,
                    transform,
                    prototype_instance_id: Some(*prototype),
                    generated_by: Some(operation.operation),
                    source_recipe_instance: false,
                })?;
            }
        }
        Ok(())
    }

    fn apply_radial_array(
        &mut self,
        operation: &RadialArrayOperation,
    ) -> Result<(), AssemblyError> {
        if operation.count == 0 {
            return Err(AssemblyError::InvalidInput(
                "radial array count must be at least one".to_owned(),
            ));
        }
        let denominator = operation.count.saturating_sub(1).max(1) as f32;
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            for index in 1..operation.count {
                let angle = operation.angular_span_degrees * index as f32 / denominator;
                let radial =
                    AffineTransform3::rotation_about_axis(operation.center, operation.axis, angle)?;
                let transform = if operation.rotate_instances {
                    radial.compose(&prototype_transform)
                } else {
                    let origin = prototype_transform.transform_point([0.0, 0.0, 0.0]);
                    let rotated_origin = radial.transform_point(origin);
                    AffineTransform3::translation(sub(rotated_origin, origin))
                        .compose(&prototype_transform)
                };
                let instance_id = self.allocate_generated_instance_id();
                self.push_occurrence(OccurrenceInput {
                    instance_id,
                    definition_id,
                    transform,
                    prototype_instance_id: Some(*prototype),
                    generated_by: Some(operation.operation),
                    source_recipe_instance: false,
                })?;
            }
        }
        Ok(())
    }

    fn prototype(
        &self,
        instance_id: PartInstanceId,
    ) -> Result<(PartDefinitionId, AffineTransform3), AssemblyError> {
        let instance = self
            .recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        if !instance.enabled {
            return Err(AssemblyError::InvalidInput(format!(
                "prototype instance {} is disabled",
                instance_id.0
            )));
        }
        let transform = self
            .world_transforms
            .get(&instance_id)
            .copied()
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        Ok((instance.definition, transform))
    }

    fn allocate_generated_instance_id(&mut self) -> PartInstanceId {
        let id = PartInstanceId(self.next_generated_instance_id);
        self.next_generated_instance_id = self.next_generated_instance_id.saturating_add(1);
        id
    }

    fn finish(self) -> Result<AssemblyEvaluation, AssemblyError> {
        let mut ordered_meshes = Vec::new();
        let mut next_preview_vertex_id = 0;
        let mut next_preview_face_id = 0;
        for instance in &self.instances {
            if let Some(mesh) = self.world_meshes.get(&instance.instance_id) {
                let mut mesh = mesh.clone();
                remap_preview_element_ids(
                    &mut mesh,
                    &mut next_preview_vertex_id,
                    &mut next_preview_face_id,
                )?;
                ordered_meshes.push(mesh);
            }
        }
        let combined_preview_mesh = combine_polygon_meshes(&ordered_meshes)?;
        let combined_preview = triangulate_polygon_mesh(&combined_preview_mesh)?;
        let provenance = build_provenance(&self);
        Ok(AssemblyEvaluation {
            local_parts: self.local_parts,
            instances: self.instances,
            world_transforms: self.world_transforms,
            world_sockets: self.world_sockets,
            world_meshes: self.world_meshes,
            combined_preview_mesh,
            combined_preview,
            instance_bounds: self.instance_bounds,
            provenance,
        })
    }
}

fn validate_attachments(recipe: &AssetRecipe) -> Result<(), AssemblyError> {
    for instance in recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
    {
        if let Some(attachment) = &instance.attachment {
            if attachment.mode == AttachmentMode::WeldBoundaryReserved {
                return Err(AssemblyError::Unsupported {
                    feature: "WeldBoundaryReserved".to_owned(),
                });
            }
            let parent = recipe
                .instances
                .get(&attachment.parent_instance)
                .ok_or(AssemblyError::UnknownInstance(attachment.parent_instance))?;
            if !parent.enabled {
                return Err(AssemblyError::InvalidInput(format!(
                    "attachment parent {} is disabled",
                    parent.id.0
                )));
            }
        }
    }
    Ok(())
}

fn detect_attachment_cycles(
    recipe: &AssetRecipe,
    enabled_instances: &BTreeSet<PartInstanceId>,
) -> Result<(), AssemblyError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for instance_id in enabled_instances {
        detect_cycle_from(
            recipe,
            *instance_id,
            enabled_instances,
            &mut visiting,
            &mut visited,
        )?;
    }
    Ok(())
}

fn detect_cycle_from(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
    enabled_instances: &BTreeSet<PartInstanceId>,
    visiting: &mut BTreeSet<PartInstanceId>,
    visited: &mut BTreeSet<PartInstanceId>,
) -> Result<(), AssemblyError> {
    if visited.contains(&instance_id) {
        return Ok(());
    }
    if !visiting.insert(instance_id) {
        return Err(AssemblyError::AttachmentCycle(instance_id));
    }
    let instance = recipe
        .instances
        .get(&instance_id)
        .ok_or(AssemblyError::UnknownInstance(instance_id))?;
    if let Some(parent) = parent_relation(instance) {
        if enabled_instances.contains(&parent) {
            detect_cycle_from(recipe, parent, enabled_instances, visiting, visited)?;
        } else {
            return Err(AssemblyError::UnknownInstance(parent));
        }
    }
    visiting.remove(&instance_id);
    visited.insert(instance_id);
    Ok(())
}

fn ordered_enabled_instances(
    recipe: &AssetRecipe,
    enabled_instances: &BTreeSet<PartInstanceId>,
) -> Result<Vec<PartInstanceId>, AssemblyError> {
    let mut children = BTreeMap::<PartInstanceId, Vec<PartInstanceId>>::new();
    for instance_id in enabled_instances {
        let instance = recipe
            .instances
            .get(instance_id)
            .ok_or(AssemblyError::UnknownInstance(*instance_id))?;
        if let Some(parent) = parent_relation(instance) {
            children.entry(parent).or_default().push(*instance_id);
        }
    }
    for child_ids in children.values_mut() {
        child_ids.sort();
    }

    let mut order = Vec::new();
    let mut visited = BTreeSet::new();
    for root in &recipe.root_instances {
        if enabled_instances.contains(root) {
            visit_instance_order(*root, &children, &mut visited, &mut order);
        }
    }
    for instance_id in enabled_instances {
        visit_instance_order(*instance_id, &children, &mut visited, &mut order);
    }
    Ok(order)
}

fn visit_instance_order(
    instance_id: PartInstanceId,
    children: &BTreeMap<PartInstanceId, Vec<PartInstanceId>>,
    visited: &mut BTreeSet<PartInstanceId>,
    order: &mut Vec<PartInstanceId>,
) {
    if !visited.insert(instance_id) {
        return;
    }
    order.push(instance_id);
    if let Some(child_ids) = children.get(&instance_id) {
        for child in child_ids {
            visit_instance_order(*child, children, visited, order);
        }
    }
}

fn parent_relation(instance: &PartInstance) -> Option<PartInstanceId> {
    instance
        .attachment
        .as_ref()
        .map(|attachment| attachment.parent_instance)
        .or(instance.parent)
}

fn transform_mesh_for_instance(
    mesh: &PolygonMesh,
    definition_id: PartDefinitionId,
    instance_id: PartInstanceId,
    generated_by: Option<OperationId>,
    transform: &AffineTransform3,
) -> Result<PolygonMesh, AssemblyError> {
    let mut transformed = mesh.clone();
    transformed.positions = mesh
        .positions
        .iter()
        .map(|position| transform.transform_point(*position))
        .collect();
    if transform.determinant() < 0.0 {
        for face in &mut transformed.faces {
            face.vertices.reverse();
        }
    }
    for metadata in &mut transformed.face_metadata {
        fill_metadata(metadata, definition_id, instance_id, generated_by);
    }
    transformed.bounds = bounds_from_positions(&transformed.positions)?;
    transformed.topology_signature =
        compute_topology_signature(&transformed.positions, &transformed.faces);
    Ok(transformed)
}

fn remap_preview_element_ids(
    mesh: &mut PolygonMesh,
    next_vertex_id: &mut u64,
    next_face_id: &mut u64,
) -> Result<(), AssemblyError> {
    for vertex_id in &mut mesh.vertex_ids {
        *vertex_id = ElementId(*next_vertex_id);
        *next_vertex_id = next_vertex_id.checked_add(1).ok_or_else(|| {
            AssemblyError::InvalidInput("combined preview vertex ElementId overflow".to_owned())
        })?;
    }
    for face in &mut mesh.faces {
        face.id = ElementId(*next_face_id);
        *next_face_id = next_face_id.checked_add(1).ok_or_else(|| {
            AssemblyError::InvalidInput("combined preview face ElementId overflow".to_owned())
        })?;
    }
    Ok(())
}

fn fill_metadata(
    metadata: &mut FaceMetadata,
    definition_id: PartDefinitionId,
    instance_id: PartInstanceId,
    generated_by: Option<OperationId>,
) {
    metadata.part_definition = Some(definition_id);
    metadata.part_instance = Some(instance_id);
    metadata.operation = metadata.operation.or(generated_by);
}

fn transform_sockets(
    sockets: &BTreeMap<SocketId, SocketSpec>,
    transform: &AffineTransform3,
) -> BTreeMap<SocketId, SocketSpec> {
    sockets
        .iter()
        .map(|(socket_id, socket)| {
            let mut socket = socket.clone();
            socket.local_frame = transform.transform_frame(&socket.local_frame);
            (*socket_id, socket)
        })
        .collect()
}

fn build_provenance<G>(state: &AssemblyState<'_, G>) -> AssemblyProvenance {
    let instances = state
        .instances
        .iter()
        .filter_map(|instance| {
            state
                .world_meshes
                .get(&instance.instance_id)
                .map(|mesh| AssemblyInstanceProvenance {
                    instance_id: instance.instance_id,
                    definition_id: instance.definition_id,
                    prototype_instance_id: instance.prototype_instance_id,
                    generated_by: instance.generated_by,
                    polygon_vertex_count: mesh.positions.len() as u64,
                    polygon_face_count: mesh.faces.len() as u64,
                })
        })
        .collect::<Vec<_>>();
    AssemblyProvenance {
        definition_generation_order: state.definition_generation_order.clone(),
        instance_order: state
            .instances
            .iter()
            .map(|instance| instance.instance_id)
            .collect(),
        instances,
    }
}

fn linear_generated_indices(count: u32, centered: bool) -> Vec<i32> {
    if count <= 1 {
        return Vec::new();
    }
    if centered {
        let center = (count / 2) as i32;
        (0..count)
            .map(|index| index as i32 - center)
            .filter(|index| *index != 0)
            .collect()
    } else {
        (1..count).map(|index| index as i32).collect()
    }
}

fn transform_power(
    transform: &AffineTransform3,
    exponent: i32,
) -> Result<AffineTransform3, AssemblyError> {
    if exponent == 0 {
        return Ok(AffineTransform3::identity());
    }
    if exponent < 0 {
        return transform_power(&transform.inverse()?, exponent.saturating_abs());
    }
    let mut result = AffineTransform3::identity();
    for _ in 0..exponent {
        result = result.compose(transform);
    }
    Ok(result)
}

fn determinant_3x3(matrix: [[f32; 3]; 3]) -> f32 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn inverse_3x3(matrix: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let determinant = determinant_3x3(matrix);
    if !determinant.is_finite() || determinant.abs() <= EPSILON {
        return None;
    }
    let inv_det = 1.0 / determinant;
    Some([
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) * inv_det,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) * inv_det,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) * inv_det,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) * inv_det,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) * inv_det,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) * inv_det,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) * inv_det,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) * inv_det,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) * inv_det,
        ],
    ])
}

fn mul_mat3_vec(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    if !array_is_finite(vector) {
        return None;
    }
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= EPSILON {
        None
    } else {
        Some([vector[0] / length, vector[1] / length, vector[2] / length])
    }
}

fn array_is_finite(values: [f32; 3]) -> bool {
    values.iter().copied().all(f32::is_finite)
}

fn scale(vector: [f32; 3], scale: f32) -> [f32; 3] {
    [vector[0] * scale, vector[1] * scale, vector[2] * scale]
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}
