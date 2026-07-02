
const EPSILON: f32 = 1.0e-6;

/// Evaluate a deterministic pattern contract in the assembly layer.
///
/// V0 delegates to the canonical pattern evaluator and does not expose any
/// product UI or export instancing claim.
pub fn evaluate_pattern_contract(
    pattern: &PatternContract,
) -> Result<PatternEvaluation, PatternEvaluationError> {
    shape_asset::evaluate_linear_pattern_contract(pattern)
}

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
                    | ModelingOperationSpec::BevelBoundaryLoop { .. }
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
