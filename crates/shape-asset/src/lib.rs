#![forbid(unsafe_code)]

//! Serializable, part-aware asset recipe contracts for the explicit modeling lane.
//!
//! These contracts are intentionally separate from the existing implicit
//! `ShapeDocument` editor and the same-topology deformation decompiler. IDs in
//! this crate are semantic: part, operation, region, and socket IDs must remain
//! stable when unrelated scalar parameters change. Generated vertex and face IDs
//! are owned by downstream polygon crates and are deterministic only for a given
//! topology signature.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current schema version for asset recipes.
pub const ASSET_RECIPE_SCHEMA_VERSION: u32 = 1;

macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Copy,
            Clone,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(pub u64);
    };
}

id_type!(AssetId, "Stable identifier for an asset recipe.");
id_type!(
    PartDefinitionId,
    "Stable semantic identifier for a reusable part definition."
);
id_type!(
    PartInstanceId,
    "Stable semantic identifier for a concrete part instance."
);
id_type!(
    OperationId,
    "Stable semantic identifier for a modeling operation."
);
id_type!(RegionId, "Stable semantic identifier for a surface region.");
id_type!(
    SocketId,
    "Stable semantic identifier for an attachment socket."
);
id_type!(
    ParameterId,
    "Stable semantic identifier for an editable parameter."
);
id_type!(
    RevisionId,
    "Stable identifier for an asset recipe revision."
);

/// Asset-space transform stored as translation, XYZ Euler rotation, and scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3 {
    /// Local translation.
    pub translation: [f32; 3],
    /// XYZ Euler rotation in degrees.
    pub rotation_degrees: [f32; 3],
    /// Per-axis scale.
    pub scale: [f32; 3],
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform3 {
    /// Return the transform as a right-handed 4x4 matrix.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation_degrees[0].to_radians(),
            self.rotation_degrees[1].to_radians(),
            self.rotation_degrees[2].to_radians(),
        );
        Mat4::from_scale_rotation_translation(
            Vec3::from_array(self.scale),
            rotation,
            Vec3::from_array(self.translation),
        )
    }

    /// Transform a point by this transform.
    #[must_use]
    pub fn transform_point(&self, point: [f32; 3]) -> [f32; 3] {
        self.matrix()
            .transform_point3(Vec3::from_array(point))
            .to_array()
    }

    /// Transform a direction vector by this transform.
    #[must_use]
    pub fn transform_vector(&self, vector: [f32; 3]) -> [f32; 3] {
        self.matrix()
            .transform_vector3(Vec3::from_array(vector))
            .to_array()
    }
}

/// A local coordinate frame used for pivots and sockets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3 {
    /// Frame origin.
    pub origin: [f32; 3],
    /// Local X axis.
    pub x_axis: [f32; 3],
    /// Local Y axis.
    pub y_axis: [f32; 3],
    /// Local Z axis.
    pub z_axis: [f32; 3],
}

impl Default for Frame3 {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0],
            z_axis: [0.0, 0.0, 1.0],
        }
    }
}

impl Frame3 {
    /// Return the frame transformed by an asset transform.
    #[must_use]
    pub fn transformed_by(&self, transform: &Transform3) -> Self {
        Self {
            origin: transform.transform_point(self.origin),
            x_axis: transform.transform_vector(self.x_axis),
            y_axis: transform.transform_vector(self.y_axis),
            z_axis: transform.transform_vector(self.z_axis),
        }
    }
}

/// Serializable recipe for one asset and its part hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRecipe {
    /// Schema version for compatibility checks.
    pub schema_version: u32,
    /// Stable asset identifier.
    pub id: AssetId,
    /// Human-facing asset title.
    pub title: String,
    /// Reusable part definitions.
    pub definitions: BTreeMap<PartDefinitionId, PartDefinition>,
    /// Concrete part instances.
    pub instances: BTreeMap<PartInstanceId, PartInstance>,
    /// Root instances in stable display and compile order.
    pub root_instances: Vec<PartInstanceId>,
    /// Editable parameter descriptors.
    pub parameters: BTreeMap<ParameterId, ParameterDescriptor>,
    /// Locked parameters that mutation programs must not edit.
    pub locks: BTreeSet<ParameterId>,
    /// Asset-level constraints.
    pub constraints: Vec<AssetConstraint>,
    /// Next semantic ID counters.
    pub next_ids: AssetIdCounters,
}

impl AssetRecipe {
    /// Create an empty recipe with deterministic ID counters.
    #[must_use]
    pub fn new(id: AssetId, title: impl Into<String>) -> Self {
        Self {
            schema_version: ASSET_RECIPE_SCHEMA_VERSION,
            id,
            title: title.into(),
            definitions: BTreeMap::new(),
            instances: BTreeMap::new(),
            root_instances: Vec::new(),
            parameters: BTreeMap::new(),
            locks: BTreeSet::new(),
            constraints: Vec::new(),
            next_ids: AssetIdCounters::default(),
        }
    }

    /// Allocate the next part definition ID.
    pub fn allocate_part_definition_id(&mut self) -> PartDefinitionId {
        allocate_part_definition_id(self)
    }

    /// Allocate the next part instance ID.
    pub fn allocate_part_instance_id(&mut self) -> PartInstanceId {
        allocate_part_instance_id(self)
    }

    /// Allocate the next operation ID.
    pub fn allocate_operation_id(&mut self) -> OperationId {
        allocate_operation_id(self)
    }

    /// Allocate the next region ID.
    pub fn allocate_region_id(&mut self) -> RegionId {
        allocate_region_id(self)
    }

    /// Allocate the next socket ID.
    pub fn allocate_socket_id(&mut self) -> SocketId {
        allocate_socket_id(self)
    }

    /// Allocate the next parameter ID.
    pub fn allocate_parameter_id(&mut self) -> ParameterId {
        allocate_parameter_id(self)
    }

    /// Allocate the next revision ID.
    pub fn allocate_revision_id(&mut self) -> RevisionId {
        allocate_revision_id(self)
    }
}

/// Next-ID counters stored in an asset recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIdCounters {
    /// Next part definition ID.
    pub part_definition: u64,
    /// Next part instance ID.
    pub part_instance: u64,
    /// Next operation ID.
    pub operation: u64,
    /// Next region ID.
    pub region: u64,
    /// Next socket ID.
    pub socket: u64,
    /// Next parameter ID.
    pub parameter: u64,
    /// Next revision ID.
    pub revision: u64,
}

impl Default for AssetIdCounters {
    fn default() -> Self {
        Self {
            part_definition: 1,
            part_instance: 1,
            operation: 1,
            region: 1,
            socket: 1,
            parameter: 1,
            revision: 1,
        }
    }
}

/// Reusable definition of a semantic part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartDefinition {
    /// Stable definition ID.
    pub id: PartDefinitionId,
    /// Human-facing part name.
    pub name: String,
    /// Free-form semantic tags.
    pub tags: BTreeSet<String>,
    /// Source and operation history for local geometry.
    pub geometry: GeometryRecipe,
    /// Declared semantic surface regions.
    pub regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    /// Declared attachment sockets.
    pub sockets: BTreeMap<SocketId, SocketSpec>,
    /// Local pivot frame.
    pub local_pivot: Frame3,
    /// Optional variant group for interchangeable definitions.
    pub variant_group: Option<String>,
    /// Optional production hints for later systems.
    pub production_hints: Option<ProductionHints>,
}

/// Optional non-authoritative production hints.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProductionHints {
    /// Preferred deterministic generator or toolchain label.
    pub preferred_generator: Option<String>,
    /// Free-form key/value hints reserved for later pipelines.
    pub hints: BTreeMap<String, String>,
}

/// One instance of a part definition in an asset hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartInstance {
    /// Stable instance ID.
    pub id: PartInstanceId,
    /// Referenced part definition.
    pub definition: PartDefinitionId,
    /// Human-facing instance name.
    pub name: String,
    /// Optional parent instance.
    pub parent: Option<PartInstanceId>,
    /// Transform relative to the parent or asset root.
    pub local_transform: Transform3,
    /// Optional socket attachment.
    pub attachment: Option<AttachmentSpec>,
    /// Disabled instances remain serializable but are skipped by compilers.
    pub enabled: bool,
    /// Free-form semantic tags.
    pub tags: BTreeSet<String>,
    /// Operation that generated this instance, if any.
    pub generated_by: Option<OperationId>,
}

/// Geometry source plus deterministic local operation history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryRecipe {
    /// Base geometry source.
    pub source: GeometrySource,
    /// Ordered modeling operation specifications.
    pub operations: Vec<ModelingOperationSpec>,
}

/// Base geometry source for an explicit part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeometrySource {
    /// Rounded box described by half extents and corner radius.
    RoundedBox {
        /// Half extents along local X, Y, and Z.
        half_extents: [f32; 3],
        /// Corner radius.
        radius: f32,
    },
    /// Cylinder along local Y.
    Cylinder {
        /// Cylinder radius.
        radius: f32,
        /// Cylinder height.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
    /// Frustum along local Y.
    Frustum {
        /// Bottom radius.
        bottom_radius: f32,
        /// Top radius.
        top_radius: f32,
        /// Frustum height.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
    /// Rectangular plate centered at the local origin.
    Plate {
        /// Size along local X and Z.
        size: [f32; 2],
        /// Plate thickness along local Y.
        thickness: f32,
    },
    /// Sweep profile along a frame path.
    Sweep {
        /// Two-dimensional profile points.
        profile: Vec<[f32; 2]>,
        /// Ordered sweep frames.
        path: Vec<Frame3>,
    },
    /// Lathe profile around local Y.
    Lathe {
        /// Radius/height profile points.
        profile: Vec<[f32; 2]>,
        /// Rotational segment count.
        segments: u32,
    },
    /// Literal polygon source reserved for already-authored explicit topology.
    LiteralMesh {
        /// Vertex positions.
        positions: Vec<[f32; 3]>,
        /// Polygon faces as vertex indices.
        faces: Vec<Vec<u32>>,
    },
    /// Reserved boolean output placeholder.
    ReservedBooleanResult {
        /// Human-readable placeholder label.
        label: String,
    },
}

/// Deterministic modeling operation specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelingOperationSpec {
    /// Apply a local transform to generated geometry.
    TransformGeometry {
        /// Stable operation ID.
        operation: OperationId,
        /// Transform to apply.
        transform: Transform3,
    },
    /// Set a bevel profile for subsequent generation.
    SetBevelProfile {
        /// Stable operation ID.
        operation: OperationId,
        /// Bevel radius.
        radius: f32,
        /// Segment count.
        segments: u32,
    },
    /// Add an inset panel to a region.
    AddPanel {
        /// Stable operation ID.
        operation: OperationId,
        /// Target region.
        region: RegionId,
        /// Panel inset.
        inset: f32,
        /// Panel depth.
        depth: f32,
    },
    /// Add trim to a region boundary.
    AddTrim {
        /// Stable operation ID.
        operation: OperationId,
        /// Target region.
        region: RegionId,
        /// Trim width.
        width: f32,
        /// Trim height.
        height: f32,
    },
    /// Mirror generated instances across a plane.
    MirrorInstances {
        /// Stable operation ID.
        operation: OperationId,
        /// Plane normal.
        plane_normal: [f32; 3],
        /// Plane offset from the origin.
        plane_offset: f32,
    },
    /// Generate a linear array.
    LinearArray {
        /// Stable operation ID.
        operation: OperationId,
        /// Number of copies.
        count: u32,
        /// Offset between copies.
        offset: [f32; 3],
    },
    /// Generate a radial array.
    RadialArray {
        /// Stable operation ID.
        operation: OperationId,
        /// Number of copies.
        count: u32,
        /// Rotation axis.
        axis: [f32; 3],
        /// Total angle in degrees.
        angle_degrees: f32,
    },
    /// Reserved boolean operation that must serialize but not compile yet.
    ReservedBoolean {
        /// Stable operation ID.
        operation: OperationId,
        /// Placeholder label.
        label: String,
    },
    /// Reserved deformation program that must serialize but not compile yet.
    ReservedDeformationProgram {
        /// Stable operation ID.
        operation: OperationId,
        /// Placeholder label.
        label: String,
    },
}

impl ModelingOperationSpec {
    /// Return the semantic operation ID.
    #[must_use]
    pub fn operation_id(&self) -> OperationId {
        match self {
            Self::TransformGeometry { operation, .. }
            | Self::SetBevelProfile { operation, .. }
            | Self::AddPanel { operation, .. }
            | Self::AddTrim { operation, .. }
            | Self::MirrorInstances { operation, .. }
            | Self::LinearArray { operation, .. }
            | Self::RadialArray { operation, .. }
            | Self::ReservedBoolean { operation, .. }
            | Self::ReservedDeformationProgram { operation, .. } => *operation,
        }
    }
}

/// Declared attachment socket on a part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocketSpec {
    /// Stable socket ID.
    pub id: SocketId,
    /// Human-facing socket name.
    pub name: String,
    /// Socket frame in part-local coordinates.
    pub local_frame: Frame3,
    /// Generic socket role.
    pub role: String,
    /// Free-form semantic tags.
    pub tags: BTreeSet<String>,
}

/// Attachment relation for a child instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentSpec {
    /// Parent instance used for the attachment.
    pub parent_instance: PartInstanceId,
    /// Socket on the parent definition.
    pub parent_socket: SocketId,
    /// Socket on the child definition.
    pub child_socket: SocketId,
    /// Offset applied after socket alignment.
    pub local_offset: Transform3,
    /// Attachment mode.
    pub mode: AttachmentMode,
}

/// Attachment implementation mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AttachmentMode {
    /// Keep child and parent as rigid separate parts.
    RigidSeparate,
    /// Reserved future boundary welding mode.
    WeldBoundaryReserved,
}

/// Declared surface region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceRegionSpec {
    /// Stable region ID.
    pub id: RegionId,
    /// Human-facing region name.
    pub name: String,
    /// Generic region role.
    pub role: SurfaceRole,
    /// Free-form semantic tags.
    pub tags: BTreeSet<String>,
}

/// Generic role for a semantic surface region.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SurfaceRole {
    /// Primary visible surface.
    PrimarySurface,
    /// Cap surface.
    Cap,
    /// Side surface.
    Side,
    /// Bevel band surface.
    BevelBand,
    /// Panel surface.
    Panel,
    /// Trim surface.
    Trim,
    /// Attachment surface.
    Attachment,
    /// Interior surface.
    Interior,
    /// Detail surface.
    Detail,
    /// Custom generic role.
    Custom(String),
}

/// Editable scalar parameter descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    /// Stable parameter ID.
    pub id: ParameterId,
    /// Canonical scalar path.
    pub path: String,
    /// Human-facing label.
    pub label: String,
    /// Human-facing parameter group.
    pub group: String,
    /// Minimum permitted scalar.
    pub minimum: f32,
    /// Maximum permitted scalar.
    pub maximum: f32,
    /// Suggested UI step.
    pub step: f32,
    /// Standard deviation for deterministic mutation.
    pub mutation_sigma: f32,
    /// Whether changing this parameter can change topology.
    pub topology_changing: bool,
    /// Beginner-facing explanation.
    pub beginner_description: String,
}

/// Asset-level constraint placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetConstraint {
    /// Require an instance to remain present.
    RequireInstance {
        /// Required instance.
        instance: PartInstanceId,
    },
    /// Prevent two semantic tags from coexisting.
    MutuallyExclusiveTags {
        /// First tag.
        first: String,
        /// Second tag.
        second: String,
    },
    /// Reserved custom constraint identified by stable code.
    Custom {
        /// Stable constraint code.
        code: String,
        /// Human-readable message.
        message: String,
    },
}

/// Structural or scalar asset edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetEdit {
    /// Set an editable scalar parameter.
    SetScalar {
        /// Target parameter.
        parameter: ParameterId,
        /// New scalar value.
        value: f32,
    },
    /// Replace an instance transform.
    SetTransform {
        /// Target instance.
        instance: PartInstanceId,
        /// New transform.
        transform: Transform3,
    },
    /// Enable or disable an instance.
    SetEnabled {
        /// Target instance.
        instance: PartInstanceId,
        /// New enabled state.
        enabled: bool,
    },
    /// Add a new instance.
    AddInstance {
        /// Instance to add.
        instance: PartInstance,
    },
    /// Remove an existing instance.
    RemoveInstance {
        /// Instance to remove.
        instance: PartInstanceId,
    },
    /// Replace or insert a part definition.
    ReplaceDefinition {
        /// New definition payload.
        definition: PartDefinition,
    },
    /// Set the count on a deterministic array operation.
    SetArrayCount {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Array operation.
        operation: OperationId,
        /// New count.
        count: u32,
    },
    /// Attach an instance to a parent socket.
    Attach {
        /// Target child instance.
        instance: PartInstanceId,
        /// Attachment specification.
        attachment: AttachmentSpec,
    },
    /// Remove an instance attachment.
    Detach {
        /// Target instance.
        instance: PartInstanceId,
    },
    /// Change a parameter lock.
    SetLock {
        /// Target parameter.
        parameter: ParameterId,
        /// New lock state.
        locked: bool,
    },
}

/// Ordered edit program with a deterministic seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEditProgram {
    /// Human-facing edit label.
    pub label: String,
    /// Deterministic seed associated with this edit program.
    pub seed: u64,
    /// Ordered edit operations.
    pub operations: Vec<AssetEdit>,
}

/// Validation issue emitted for asset recipes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation report for an asset recipe.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetValidationReport {
    /// Discovered issues.
    pub issues: Vec<AssetValidationIssue>,
}

impl AssetValidationReport {
    /// Return true when the recipe is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Error type for asset recipe helpers.
#[derive(Debug, Error)]
pub enum AssetError {
    /// The requested part definition does not exist.
    #[error("unknown part definition {0:?}")]
    UnknownDefinition(PartDefinitionId),
    /// The requested part instance does not exist.
    #[error("unknown part instance {0:?}")]
    UnknownInstance(PartInstanceId),
    /// The requested parameter does not exist.
    #[error("unknown parameter {0:?}")]
    UnknownParameter(ParameterId),
    /// The requested scalar path does not exist.
    #[error("unknown scalar path {0}")]
    UnknownScalarPath(String),
    /// A non-finite scalar was supplied.
    #[error("non-finite scalar for {path}: {value}")]
    NonFiniteScalar {
        /// Target path.
        path: String,
        /// Supplied value.
        value: f32,
    },
    /// The edit attempted to mutate a locked parameter.
    #[error("parameter is locked {0:?}")]
    LockedParameter(ParameterId),
    /// An edit cannot be applied to the target.
    #[error("unsupported edit: {0}")]
    UnsupportedEdit(String),
    /// The edited recipe failed validation.
    #[error("asset recipe validation failed")]
    ValidationFailed(AssetValidationReport),
}

/// Build a canonical scalar path for a part definition.
#[must_use]
pub fn definition_scalar_path(definition: PartDefinitionId, key: impl AsRef<str>) -> String {
    format!("definition.{}.{}", definition.0, key.as_ref())
}

/// Build a canonical scalar path for a part instance.
#[must_use]
pub fn instance_scalar_path(instance: PartInstanceId, key: impl AsRef<str>) -> String {
    format!("instance.{}.{}", instance.0, key.as_ref())
}

/// Validate an asset recipe and collect all discoverable issues.
#[must_use]
pub fn validate_asset_recipe(recipe: &AssetRecipe) -> AssetValidationReport {
    let mut report = AssetValidationReport::default();

    if recipe.schema_version != ASSET_RECIPE_SCHEMA_VERSION {
        push_issue(
            &mut report,
            None,
            "unsupported_schema_version",
            "Asset recipe schema version is not supported.",
        );
    }
    if recipe.title.trim().is_empty() {
        push_issue(
            &mut report,
            None,
            "empty_title",
            "Asset recipe title cannot be empty.",
        );
    }

    validate_definitions(recipe, &mut report);
    validate_instances(recipe, &mut report);
    validate_parameters(recipe, &mut report);
    validate_constraints(recipe, &mut report);
    validate_next_ids(recipe, &mut report);

    report
}

/// Return editable parameters in deterministic order.
#[must_use]
pub fn enumerate_parameters(recipe: &AssetRecipe) -> Vec<ParameterDescriptor> {
    recipe.parameters.values().cloned().collect()
}

/// Read a scalar parameter by canonical path.
pub fn get_scalar(recipe: &AssetRecipe, path: impl AsRef<str>) -> Result<f32, AssetError> {
    let path = path.as_ref();
    let parts = path.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        ["definition", id, rest @ ..] => {
            let definition_id = parse_id(id, path).map(PartDefinitionId)?;
            let definition = recipe
                .definitions
                .get(&definition_id)
                .ok_or(AssetError::UnknownDefinition(definition_id))?;
            get_definition_scalar(definition, rest, path)
        }
        ["instance", id, rest @ ..] => {
            let instance_id = parse_id(id, path).map(PartInstanceId)?;
            let instance = recipe
                .instances
                .get(&instance_id)
                .ok_or(AssetError::UnknownInstance(instance_id))?;
            get_transform_scalar(&instance.local_transform, rest, path)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

/// Set a scalar parameter by canonical path.
pub fn set_scalar(
    recipe: &mut AssetRecipe,
    path: impl AsRef<str>,
    value: f32,
) -> Result<(), AssetError> {
    let path = path.as_ref();
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar {
            path: path.to_owned(),
            value,
        });
    }

    let mut clone = recipe.clone();
    set_scalar_in_place(&mut clone, path, value)?;
    *recipe = clone;
    Ok(())
}

/// Apply an edit program atomically.
pub fn apply_edit_program(
    recipe: &AssetRecipe,
    program: &AssetEditProgram,
) -> Result<AssetRecipe, AssetError> {
    let mut clone = recipe.clone();
    for operation in &program.operations {
        apply_edit(&mut clone, operation)?;
    }
    let report = validate_asset_recipe(&clone);
    if report.is_valid() {
        Ok(clone)
    } else {
        Err(AssetError::ValidationFailed(report))
    }
}

/// Allocate the next part definition ID.
pub fn allocate_part_definition_id(recipe: &mut AssetRecipe) -> PartDefinitionId {
    let id = PartDefinitionId(recipe.next_ids.part_definition);
    recipe.next_ids.part_definition = recipe.next_ids.part_definition.saturating_add(1);
    id
}

/// Allocate the next part instance ID.
pub fn allocate_part_instance_id(recipe: &mut AssetRecipe) -> PartInstanceId {
    let id = PartInstanceId(recipe.next_ids.part_instance);
    recipe.next_ids.part_instance = recipe.next_ids.part_instance.saturating_add(1);
    id
}

/// Allocate the next operation ID.
pub fn allocate_operation_id(recipe: &mut AssetRecipe) -> OperationId {
    let id = OperationId(recipe.next_ids.operation);
    recipe.next_ids.operation = recipe.next_ids.operation.saturating_add(1);
    id
}

/// Allocate the next region ID.
pub fn allocate_region_id(recipe: &mut AssetRecipe) -> RegionId {
    let id = RegionId(recipe.next_ids.region);
    recipe.next_ids.region = recipe.next_ids.region.saturating_add(1);
    id
}

/// Allocate the next socket ID.
pub fn allocate_socket_id(recipe: &mut AssetRecipe) -> SocketId {
    let id = SocketId(recipe.next_ids.socket);
    recipe.next_ids.socket = recipe.next_ids.socket.saturating_add(1);
    id
}

/// Allocate the next parameter ID.
pub fn allocate_parameter_id(recipe: &mut AssetRecipe) -> ParameterId {
    let id = ParameterId(recipe.next_ids.parameter);
    recipe.next_ids.parameter = recipe.next_ids.parameter.saturating_add(1);
    id
}

/// Allocate the next revision ID.
pub fn allocate_revision_id(recipe: &mut AssetRecipe) -> RevisionId {
    let id = RevisionId(recipe.next_ids.revision);
    recipe.next_ids.revision = recipe.next_ids.revision.saturating_add(1);
    id
}

/// Return descendants of an instance in stable order.
pub fn descendants_of(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
) -> Result<Vec<PartInstanceId>, AssetError> {
    if !recipe.instances.contains_key(&instance) {
        return Err(AssetError::UnknownInstance(instance));
    }
    let mut result = BTreeSet::new();
    collect_descendants(recipe, instance, &mut result);
    result.remove(&instance);
    Ok(result.into_iter().collect())
}

/// Return instances that reference a definition in stable order.
#[must_use]
pub fn instances_of_definition(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
) -> Vec<PartInstanceId> {
    recipe
        .instances
        .values()
        .filter(|instance| instance.definition == definition)
        .map(|instance| instance.id)
        .collect()
}

fn validate_definitions(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (id, definition) in &recipe.definitions {
        if definition.id != *id {
            push_issue(
                report,
                Some(format!("definition.{}", id.0)),
                "definition_id_mismatch",
                "Part definition map key and payload ID differ.",
            );
        }
        if definition.name.trim().is_empty() {
            push_issue(
                report,
                Some(format!("definition.{}", id.0)),
                "empty_definition_name",
                "Part definition name cannot be empty.",
            );
        }
        validate_geometry_source(*id, &definition.geometry.source, report);
        validate_operations(definition, report);
        for (region_id, region) in &definition.regions {
            if region.id != *region_id {
                push_issue(
                    report,
                    Some(format!("definition.{}.region.{}", id.0, region_id.0)),
                    "region_id_mismatch",
                    "Region map key and payload ID differ.",
                );
            }
        }
        for (socket_id, socket) in &definition.sockets {
            if socket.id != *socket_id {
                push_issue(
                    report,
                    Some(format!("definition.{}.socket.{}", id.0, socket_id.0)),
                    "socket_id_mismatch",
                    "Socket map key and payload ID differ.",
                );
            }
            validate_frame(
                report,
                Some(format!("definition.{}.socket.{}", id.0, socket_id.0)),
                &socket.local_frame,
            );
        }
        validate_frame(
            report,
            Some(format!("definition.{}.pivot", id.0)),
            &definition.local_pivot,
        );
    }
}

fn validate_instances(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for root in &recipe.root_instances {
        match recipe.instances.get(root) {
            Some(instance) if instance.parent.is_some() => push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "root_has_parent",
                "Root instances cannot also declare a parent.",
            ),
            Some(_) => {}
            None => push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "unknown_root_instance",
                "Root instance does not exist.",
            ),
        }
    }

    for (id, instance) in &recipe.instances {
        if instance.id != *id {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "instance_id_mismatch",
                "Instance map key and payload ID differ.",
            );
        }
        if !recipe.definitions.contains_key(&instance.definition) {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "unknown_instance_definition",
                "Instance references an unknown definition.",
            );
        }
        if let Some(parent) = instance.parent {
            if parent == *id {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "self_parent",
                    "Instance cannot parent itself.",
                );
            } else if !recipe.instances.contains_key(&parent) {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "unknown_parent_instance",
                    "Instance references an unknown parent.",
                );
            }
        }
        validate_transform(
            report,
            Some(format!("instance.{}.transform", id.0)),
            &instance.local_transform,
        );
        if let Some(attachment) = &instance.attachment {
            validate_attachment(recipe, *id, attachment, report);
        }
    }
    validate_parent_cycles(recipe, report);
}

fn validate_attachment(
    recipe: &AssetRecipe,
    child: PartInstanceId,
    attachment: &AttachmentSpec,
    report: &mut AssetValidationReport,
) {
    let Some(parent) = recipe.instances.get(&attachment.parent_instance) else {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_attachment_parent",
            "Attachment parent instance does not exist.",
        );
        return;
    };
    let Some(child_instance) = recipe.instances.get(&child) else {
        return;
    };
    let Some(parent_definition) = recipe.definitions.get(&parent.definition) else {
        return;
    };
    let Some(child_definition) = recipe.definitions.get(&child_instance.definition) else {
        return;
    };
    if !parent_definition
        .sockets
        .contains_key(&attachment.parent_socket)
    {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_parent_socket",
            "Attachment parent socket does not exist on the parent definition.",
        );
    }
    if !child_definition
        .sockets
        .contains_key(&attachment.child_socket)
    {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_child_socket",
            "Attachment child socket does not exist on the child definition.",
        );
    }
    validate_transform(
        report,
        Some(format!("instance.{}.attachment_offset", child.0)),
        &attachment.local_offset,
    );
}

fn validate_parameters(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (id, parameter) in &recipe.parameters {
        if parameter.id != *id {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "parameter_id_mismatch",
                "Parameter map key and payload ID differ.",
            );
        }
        for (label, value) in [
            ("minimum", parameter.minimum),
            ("maximum", parameter.maximum),
            ("step", parameter.step),
            ("mutation_sigma", parameter.mutation_sigma),
        ] {
            if !value.is_finite() {
                push_issue(
                    report,
                    Some(format!("parameter.{}.{}", id.0, label)),
                    "non_finite_parameter_bound",
                    "Parameter bounds must be finite.",
                );
            }
        }
        if parameter.minimum > parameter.maximum {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "invalid_parameter_range",
                "Parameter minimum cannot exceed maximum.",
            );
        }
        if get_scalar(recipe, &parameter.path).is_err() {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "unknown_parameter_path",
                "Parameter path does not resolve to a scalar.",
            );
        }
    }
    for lock in &recipe.locks {
        if !recipe.parameters.contains_key(lock) {
            push_issue(
                report,
                Some(format!("parameter.{}", lock.0)),
                "unknown_locked_parameter",
                "Locked parameter does not exist.",
            );
        }
    }
}

fn validate_constraints(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for constraint in &recipe.constraints {
        if let AssetConstraint::RequireInstance { instance } = constraint
            && !recipe.instances.contains_key(instance)
        {
            push_issue(
                report,
                Some(format!("constraint.instance.{}", instance.0)),
                "unknown_required_instance",
                "Constraint references an unknown instance.",
            );
        }
    }
}

fn validate_next_ids(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    validate_counter(
        report,
        "part_definition",
        recipe.next_ids.part_definition,
        recipe.definitions.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "part_instance",
        recipe.next_ids.part_instance,
        recipe.instances.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "parameter",
        recipe.next_ids.parameter,
        recipe.parameters.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "operation",
        recipe.next_ids.operation,
        max_operation_id(recipe),
    );
    validate_counter(
        report,
        "region",
        recipe.next_ids.region,
        max_region_id(recipe),
    );
    validate_counter(
        report,
        "socket",
        recipe.next_ids.socket,
        max_socket_id(recipe),
    );
}

fn validate_counter(
    report: &mut AssetValidationReport,
    name: &'static str,
    next: u64,
    max_existing: Option<u64>,
) {
    if let Some(max_existing) = max_existing
        && next <= max_existing
    {
        push_issue(
            report,
            Some(format!("next_ids.{name}")),
            "next_id_not_fresh",
            "Next ID counter would reallocate an existing semantic ID.",
        );
    }
}

fn max_operation_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .map(|operation| operation.operation_id().0)
        .max()
}

fn max_region_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.regions.keys().map(|id| id.0))
        .max()
}

fn max_socket_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.sockets.keys().map(|id| id.0))
        .max()
}

fn validate_geometry_source(
    definition: PartDefinitionId,
    source: &GeometrySource,
    report: &mut AssetValidationReport,
) {
    match source {
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        } => {
            validate_positive_array(
                report,
                Some(format!(
                    "definition.{}.rounded_box.half_extents",
                    definition.0
                )),
                half_extents,
            );
            validate_non_negative(
                report,
                Some(format!("definition.{}.rounded_box.radius", definition.0)),
                *radius,
            );
        }
        GeometrySource::Cylinder {
            radius,
            height,
            radial_segments,
        } => {
            validate_positive(
                report,
                definition_subject(definition, "cylinder.radius"),
                *radius,
            );
            validate_positive(
                report,
                definition_subject(definition, "cylinder.height"),
                *height,
            );
            validate_count(
                report,
                definition_subject(definition, "cylinder.radial_segments"),
                *radial_segments,
                3,
            );
        }
        GeometrySource::Frustum {
            bottom_radius,
            top_radius,
            height,
            radial_segments,
        } => {
            validate_non_negative(
                report,
                definition_subject(definition, "frustum.bottom_radius"),
                *bottom_radius,
            );
            validate_non_negative(
                report,
                definition_subject(definition, "frustum.top_radius"),
                *top_radius,
            );
            validate_positive(
                report,
                definition_subject(definition, "frustum.height"),
                *height,
            );
            validate_count(
                report,
                definition_subject(definition, "frustum.radial_segments"),
                *radial_segments,
                3,
            );
        }
        GeometrySource::Plate { size, thickness } => {
            validate_positive_array(report, definition_subject(definition, "plate.size"), size);
            validate_positive(
                report,
                definition_subject(definition, "plate.thickness"),
                *thickness,
            );
        }
        GeometrySource::Sweep { profile, path } => {
            if profile.len() < 2 || path.len() < 2 {
                push_issue(
                    report,
                    definition_subject(definition, "sweep"),
                    "insufficient_sweep_data",
                    "Sweep requires at least two profile points and two path frames.",
                );
            }
            for frame in path {
                validate_frame(report, definition_subject(definition, "sweep.path"), frame);
            }
        }
        GeometrySource::Lathe { profile, segments } => {
            if profile.len() < 2 {
                push_issue(
                    report,
                    definition_subject(definition, "lathe.profile"),
                    "insufficient_lathe_profile",
                    "Lathe requires at least two profile points.",
                );
            }
            validate_count(
                report,
                definition_subject(definition, "lathe.segments"),
                *segments,
                3,
            );
        }
        GeometrySource::LiteralMesh { positions, faces } => {
            if positions.is_empty() || faces.is_empty() {
                push_issue(
                    report,
                    definition_subject(definition, "literal_mesh"),
                    "empty_literal_mesh",
                    "Literal mesh must contain positions and faces.",
                );
            }
            for position in positions {
                if !array_is_finite(position) {
                    push_issue(
                        report,
                        definition_subject(definition, "literal_mesh.positions"),
                        "non_finite_literal_position",
                        "Literal mesh positions must be finite.",
                    );
                }
            }
        }
        GeometrySource::ReservedBooleanResult { .. } => {}
    }
}

fn validate_operations(definition: &PartDefinition, report: &mut AssetValidationReport) {
    let mut seen = BTreeSet::new();
    for operation in &definition.geometry.operations {
        let operation_id = operation.operation_id();
        if !seen.insert(operation_id) {
            push_issue(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                "duplicate_operation_id",
                "Operation IDs must be unique within a definition.",
            );
        }
        match operation {
            ModelingOperationSpec::TransformGeometry { transform, .. } => validate_transform(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                transform,
            ),
            ModelingOperationSpec::SetBevelProfile {
                radius, segments, ..
            } => {
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "bevel.radius"),
                    *radius,
                );
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "bevel.segments"),
                    *segments,
                    1,
                );
            }
            ModelingOperationSpec::AddPanel {
                region,
                inset,
                depth,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "panel.inset"),
                    *inset,
                );
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "panel.depth"),
                    *depth,
                );
            }
            ModelingOperationSpec::AddTrim {
                region,
                width,
                height,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "trim.width"),
                    *width,
                );
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "trim.height"),
                    *height,
                );
            }
            ModelingOperationSpec::MirrorInstances {
                plane_normal,
                plane_offset,
                ..
            } => {
                if !array_is_finite(plane_normal) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "mirror.plane_normal"),
                        "non_finite",
                        "Mirror plane normal must be finite.",
                    );
                }
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "mirror.plane_offset"),
                    *plane_offset,
                );
            }
            ModelingOperationSpec::LinearArray { count, offset, .. } => {
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "linear_array.count"),
                    *count,
                    1,
                );
                if !array_is_finite(offset) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "linear_array.offset"),
                        "non_finite",
                        "Linear array offset must be finite.",
                    );
                }
            }
            ModelingOperationSpec::RadialArray {
                count,
                axis,
                angle_degrees,
                ..
            } => {
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "radial_array.count"),
                    *count,
                    1,
                );
                if !array_is_finite(axis) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "radial_array.axis"),
                        "non_finite",
                        "Radial array axis must be finite.",
                    );
                }
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "radial_array.angle_degrees"),
                    *angle_degrees,
                );
            }
            ModelingOperationSpec::ReservedBoolean { .. }
            | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
        }
    }
}

fn validate_region_reference(
    definition: &PartDefinition,
    region: RegionId,
    operation: OperationId,
    report: &mut AssetValidationReport,
) {
    if !definition.regions.contains_key(&region) {
        push_issue(
            report,
            operation_subject(definition.id, operation, "region"),
            "unknown_operation_region",
            "Operation references an unknown region.",
        );
    }
}

fn validate_parent_cycles(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for id in recipe.instances.keys() {
        let mut seen = BTreeSet::new();
        let mut cursor = Some(*id);
        while let Some(current) = cursor {
            if !seen.insert(current) {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "parent_cycle",
                    "Instance parent chain contains a cycle.",
                );
                break;
            }
            cursor = recipe
                .instances
                .get(&current)
                .and_then(|instance| instance.parent);
        }
    }
}

fn validate_transform(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    transform: &Transform3,
) {
    validate_finite_array(
        report,
        append_subject(subject.clone(), "translation"),
        &transform.translation,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "rotation_degrees"),
        &transform.rotation_degrees,
    );
    validate_finite_array(report, append_subject(subject, "scale"), &transform.scale);
}

fn validate_frame(report: &mut AssetValidationReport, subject: Option<String>, frame: &Frame3) {
    validate_finite_array(
        report,
        append_subject(subject.clone(), "origin"),
        &frame.origin,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "x_axis"),
        &frame.x_axis,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "y_axis"),
        &frame.y_axis,
    );
    validate_finite_array(report, append_subject(subject, "z_axis"), &frame.z_axis);
}

fn validate_finite_array(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    values: &[f32],
) {
    if !values.iter().copied().all(f32::is_finite) {
        push_issue(
            report,
            subject,
            "non_finite",
            "All numeric components must be finite.",
        );
    }
}

fn validate_positive_array(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    values: &[f32],
) {
    for value in values {
        validate_positive(report, subject.clone(), *value);
    }
}

fn validate_positive(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && value <= 0.0 {
        push_issue(
            report,
            subject,
            "not_positive",
            "Value must be greater than zero.",
        );
    }
}

fn validate_non_negative(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && value < 0.0 {
        push_issue(
            report,
            subject,
            "negative_value",
            "Value must not be negative.",
        );
    }
}

fn validate_finite(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    if !value.is_finite() {
        push_issue(report, subject, "non_finite", "Value must be finite.");
    }
}

fn validate_count(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: u32,
    minimum: u32,
) {
    if value < minimum {
        push_issue(
            report,
            subject,
            "count_too_small",
            format!("Count must be at least {minimum}."),
        );
    }
}

fn get_definition_scalar(
    definition: &PartDefinition,
    rest: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match rest {
        ["geometry", source_kind, field @ ..] => {
            get_geometry_source_scalar(&definition.geometry.source, source_kind, field, path)
        }
        ["operation", operation_id, field @ ..] => {
            let operation_id = parse_id(operation_id, path).map(OperationId)?;
            let operation = definition
                .geometry
                .operations
                .iter()
                .find(|operation| operation.operation_id() == operation_id)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            get_operation_scalar(operation, field, path)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_geometry_source_scalar(
    source: &GeometrySource,
    source_kind: &str,
    field: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match (source, source_kind, field) {
        (
            GeometrySource::RoundedBox {
                half_extents,
                radius: _,
            },
            "rounded_box",
            ["half_extents", component],
        ) => component_value(half_extents, component, path),
        (GeometrySource::RoundedBox { radius, .. }, "rounded_box", ["radius"]) => Ok(*radius),
        (GeometrySource::Cylinder { radius, .. }, "cylinder", ["radius"]) => Ok(*radius),
        (GeometrySource::Cylinder { height, .. }, "cylinder", ["height"]) => Ok(*height),
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            "cylinder",
            ["radial_segments"],
        ) => Ok(*radial_segments as f32),
        (GeometrySource::Frustum { bottom_radius, .. }, "frustum", ["bottom_radius"]) => {
            Ok(*bottom_radius)
        }
        (GeometrySource::Frustum { top_radius, .. }, "frustum", ["top_radius"]) => Ok(*top_radius),
        (GeometrySource::Frustum { height, .. }, "frustum", ["height"]) => Ok(*height),
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            "frustum",
            ["radial_segments"],
        ) => Ok(*radial_segments as f32),
        (GeometrySource::Plate { size, .. }, "plate", ["size", component]) => {
            component_value(size, component, path)
        }
        (GeometrySource::Plate { thickness, .. }, "plate", ["thickness"]) => Ok(*thickness),
        (GeometrySource::Lathe { segments, .. }, "lathe", ["segments"]) => Ok(*segments as f32),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_operation_scalar(
    operation: &ModelingOperationSpec,
    field: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match (operation, field) {
        (ModelingOperationSpec::SetBevelProfile { radius, .. }, ["bevel", "radius"]) => Ok(*radius),
        (ModelingOperationSpec::SetBevelProfile { segments, .. }, ["bevel", "segments"]) => {
            Ok(*segments as f32)
        }
        (ModelingOperationSpec::AddPanel { inset, .. }, ["panel", "inset"]) => Ok(*inset),
        (ModelingOperationSpec::AddPanel { depth, .. }, ["panel", "depth"]) => Ok(*depth),
        (ModelingOperationSpec::AddTrim { width, .. }, ["trim", "width"]) => Ok(*width),
        (ModelingOperationSpec::AddTrim { height, .. }, ["trim", "height"]) => Ok(*height),
        (ModelingOperationSpec::LinearArray { count, .. }, ["linear_array", "count"]) => {
            Ok(*count as f32)
        }
        (ModelingOperationSpec::RadialArray { count, .. }, ["radial_array", "count"]) => {
            Ok(*count as f32)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn get_transform_scalar(
    transform: &Transform3,
    rest: &[&str],
    path: &str,
) -> Result<f32, AssetError> {
    match rest {
        ["transform", "translation", component] => {
            component_value(&transform.translation, component, path)
        }
        ["transform", "rotation_degrees", component] => {
            component_value(&transform.rotation_degrees, component, path)
        }
        ["transform", "scale", component] => component_value(&transform.scale, component, path),
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_scalar_in_place(recipe: &mut AssetRecipe, path: &str, value: f32) -> Result<(), AssetError> {
    let parts = path.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        ["definition", id, rest @ ..] => {
            let definition_id = parse_id(id, path).map(PartDefinitionId)?;
            let definition = recipe
                .definitions
                .get_mut(&definition_id)
                .ok_or(AssetError::UnknownDefinition(definition_id))?;
            set_definition_scalar(definition, rest, path, value)
        }
        ["instance", id, rest @ ..] => {
            let instance_id = parse_id(id, path).map(PartInstanceId)?;
            let instance = recipe
                .instances
                .get_mut(&instance_id)
                .ok_or(AssetError::UnknownInstance(instance_id))?;
            set_transform_scalar(&mut instance.local_transform, rest, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_definition_scalar(
    definition: &mut PartDefinition,
    rest: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match rest {
        ["geometry", source_kind, field @ ..] => set_geometry_source_scalar(
            &mut definition.geometry.source,
            source_kind,
            field,
            path,
            value,
        ),
        ["operation", operation_id, field @ ..] => {
            let operation_id = parse_id(operation_id, path).map(OperationId)?;
            let operation = definition
                .geometry
                .operations
                .iter_mut()
                .find(|operation| operation.operation_id() == operation_id)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_operation_scalar(operation, field, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_geometry_source_scalar(
    source: &mut GeometrySource,
    source_kind: &str,
    field: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match (source, source_kind, field) {
        (
            GeometrySource::RoundedBox { half_extents, .. },
            "rounded_box",
            ["half_extents", component],
        ) => set_component_value(half_extents, component, path, value),
        (GeometrySource::RoundedBox { radius, .. }, "rounded_box", ["radius"]) => {
            *radius = value;
            Ok(())
        }
        (GeometrySource::Cylinder { radius, .. }, "cylinder", ["radius"]) => {
            *radius = value;
            Ok(())
        }
        (GeometrySource::Cylinder { height, .. }, "cylinder", ["height"]) => {
            *height = value;
            Ok(())
        }
        (GeometrySource::Frustum { bottom_radius, .. }, "frustum", ["bottom_radius"]) => {
            *bottom_radius = value;
            Ok(())
        }
        (GeometrySource::Frustum { top_radius, .. }, "frustum", ["top_radius"]) => {
            *top_radius = value;
            Ok(())
        }
        (GeometrySource::Frustum { height, .. }, "frustum", ["height"]) => {
            *height = value;
            Ok(())
        }
        (GeometrySource::Plate { size, .. }, "plate", ["size", component]) => {
            set_component_value(size, component, path, value)
        }
        (GeometrySource::Plate { thickness, .. }, "plate", ["thickness"]) => {
            *thickness = value;
            Ok(())
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_operation_scalar(
    operation: &mut ModelingOperationSpec,
    field: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match (operation, field) {
        (ModelingOperationSpec::SetBevelProfile { radius, .. }, ["bevel", "radius"]) => {
            *radius = value;
            Ok(())
        }
        (ModelingOperationSpec::AddPanel { inset, .. }, ["panel", "inset"]) => {
            *inset = value;
            Ok(())
        }
        (ModelingOperationSpec::AddPanel { depth, .. }, ["panel", "depth"]) => {
            *depth = value;
            Ok(())
        }
        (ModelingOperationSpec::AddTrim { width, .. }, ["trim", "width"]) => {
            *width = value;
            Ok(())
        }
        (ModelingOperationSpec::AddTrim { height, .. }, ["trim", "height"]) => {
            *height = value;
            Ok(())
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn set_transform_scalar(
    transform: &mut Transform3,
    rest: &[&str],
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match rest {
        ["transform", "translation", component] => {
            set_component_value(&mut transform.translation, component, path, value)
        }
        ["transform", "rotation_degrees", component] => {
            set_component_value(&mut transform.rotation_degrees, component, path, value)
        }
        ["transform", "scale", component] => {
            set_component_value(&mut transform.scale, component, path, value)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

fn apply_edit(recipe: &mut AssetRecipe, edit: &AssetEdit) -> Result<(), AssetError> {
    match edit {
        AssetEdit::SetScalar { parameter, value } => {
            if recipe.locks.contains(parameter) {
                return Err(AssetError::LockedParameter(*parameter));
            }
            let descriptor = recipe
                .parameters
                .get(parameter)
                .ok_or(AssetError::UnknownParameter(*parameter))?;
            let path = descriptor.path.clone();
            set_scalar(recipe, path, *value)
        }
        AssetEdit::SetTransform {
            instance,
            transform,
        } => {
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.local_transform = transform.clone();
            Ok(())
        }
        AssetEdit::SetEnabled { instance, enabled } => {
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.enabled = *enabled;
            Ok(())
        }
        AssetEdit::AddInstance { instance } => {
            if recipe.instances.contains_key(&instance.id) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "duplicate instance {:?}",
                    instance.id
                )));
            }
            recipe.instances.insert(instance.id, instance.clone());
            Ok(())
        }
        AssetEdit::RemoveInstance { instance } => {
            recipe
                .instances
                .remove(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            recipe.root_instances.retain(|root| root != instance);
            Ok(())
        }
        AssetEdit::ReplaceDefinition { definition } => {
            recipe.definitions.insert(definition.id, definition.clone());
            Ok(())
        }
        AssetEdit::SetArrayCount {
            definition,
            operation,
            count,
        } => set_array_count(recipe, *definition, *operation, *count),
        AssetEdit::Attach {
            instance,
            attachment,
        } => {
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.attachment = Some(attachment.clone());
            target.parent = Some(attachment.parent_instance);
            Ok(())
        }
        AssetEdit::Detach { instance } => {
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.attachment = None;
            Ok(())
        }
        AssetEdit::SetLock { parameter, locked } => {
            if !recipe.parameters.contains_key(parameter) {
                return Err(AssetError::UnknownParameter(*parameter));
            }
            if *locked {
                recipe.locks.insert(*parameter);
            } else {
                recipe.locks.remove(parameter);
            }
            Ok(())
        }
    }
}

fn set_array_count(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    count: u32,
) -> Result<(), AssetError> {
    let definition = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_spec = definition
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown array operation {operation:?}"))
        })?;
    match operation_spec {
        ModelingOperationSpec::LinearArray { count: target, .. }
        | ModelingOperationSpec::RadialArray { count: target, .. } => {
            *target = count;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "operation {operation:?} is not an array"
        ))),
    }
}

fn collect_descendants(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
    result: &mut BTreeSet<PartInstanceId>,
) {
    for child in recipe
        .instances
        .values()
        .filter(|candidate| candidate.parent == Some(instance))
    {
        if result.insert(child.id) {
            collect_descendants(recipe, child.id, result);
        }
    }
}

fn component_value(values: &[f32], component: &str, path: &str) -> Result<f32, AssetError> {
    let index = component_index(component, values.len(), path)?;
    values
        .get(index)
        .copied()
        .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))
}

fn set_component_value(
    values: &mut [f32],
    component: &str,
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    let index = component_index(component, values.len(), path)?;
    let Some(target) = values.get_mut(index) else {
        return Err(AssetError::UnknownScalarPath(path.to_owned()));
    };
    *target = value;
    Ok(())
}

fn component_index(component: &str, length: usize, path: &str) -> Result<usize, AssetError> {
    let index = match component {
        "x" => 0,
        "y" => 1,
        "z" => 2,
        _ => return Err(AssetError::UnknownScalarPath(path.to_owned())),
    };
    if index < length {
        Ok(index)
    } else {
        Err(AssetError::UnknownScalarPath(path.to_owned()))
    }
}

fn parse_id(raw: &str, path: &str) -> Result<u64, AssetError> {
    raw.parse::<u64>()
        .map_err(|_| AssetError::UnknownScalarPath(path.to_owned()))
}

fn append_subject(subject: Option<String>, suffix: &'static str) -> Option<String> {
    subject.map(|subject| format!("{subject}.{suffix}"))
}

fn definition_subject(definition: PartDefinitionId, suffix: &'static str) -> Option<String> {
    Some(format!("definition.{}.{suffix}", definition.0))
}

fn operation_subject(
    definition: PartDefinitionId,
    operation: OperationId,
    suffix: &'static str,
) -> Option<String> {
    Some(format!(
        "definition.{}.operation.{}.{suffix}",
        definition.0, operation.0
    ))
}

fn array_is_finite(values: &[f32]) -> bool {
    values.iter().copied().all(f32::is_finite)
}

fn push_issue(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    code: &'static str,
    message: impl Into<String>,
) {
    report.issues.push(AssetValidationIssue {
        subject,
        code: code.to_owned(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_recipe() -> AssetRecipe {
        let definition_id = PartDefinitionId(1);
        let instance_id = PartInstanceId(1);
        let parameter_id = ParameterId(1);
        let source = GeometrySource::RoundedBox {
            half_extents: [1.0, 0.5, 0.25],
            radius: 0.1,
        };
        let definition = PartDefinition {
            id: definition_id,
            name: "Body".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source,
                operations: vec![ModelingOperationSpec::LinearArray {
                    operation: OperationId(1),
                    count: 2,
                    offset: [1.0, 0.0, 0.0],
                }],
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        };
        let instance = PartInstance {
            id: instance_id,
            definition: definition_id,
            name: "Body".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let descriptor = ParameterDescriptor {
            id: parameter_id,
            path: definition_scalar_path(definition_id, "geometry.rounded_box.radius"),
            label: "Radius".to_owned(),
            group: "Form".to_owned(),
            minimum: 0.0,
            maximum: 1.0,
            step: 0.01,
            mutation_sigma: 0.05,
            topology_changing: false,
            beginner_description: "Corner radius".to_owned(),
        };
        let mut recipe = AssetRecipe::new(AssetId(1), "Contract");
        recipe.definitions.insert(definition_id, definition);
        recipe.instances.insert(instance_id, instance);
        recipe.root_instances.push(instance_id);
        recipe.parameters.insert(parameter_id, descriptor);
        recipe.next_ids.part_definition = 2;
        recipe.next_ids.part_instance = 2;
        recipe.next_ids.operation = 2;
        recipe.next_ids.parameter = 2;
        recipe
    }

    #[test]
    fn serde_json_round_trip_preserves_ordered_recipe() {
        let recipe = test_recipe();

        let json = serde_json::to_string(&recipe).expect("recipe should serialize");
        let round_tripped: AssetRecipe =
            serde_json::from_str(&json).expect("recipe should deserialize");

        assert_eq!(recipe, round_tripped);
    }

    #[test]
    fn validation_accepts_minimal_valid_recipe() {
        let recipe = test_recipe();

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn set_scalar_uses_descriptor_path() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "resize".to_owned(),
            seed: 7,
            operations: vec![AssetEdit::SetScalar {
                parameter: ParameterId(1),
                value: 0.2,
            }],
        };

        let edited = apply_edit_program(&recipe, &program).expect("edit should apply");

        assert_eq!(
            get_scalar(
                &edited,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.2
        );
    }

    #[test]
    fn locked_parameter_rejects_edit_atomically() {
        let mut recipe = test_recipe();
        recipe.locks.insert(ParameterId(1));
        let program = AssetEditProgram {
            label: "locked".to_owned(),
            seed: 1,
            operations: vec![AssetEdit::SetScalar {
                parameter: ParameterId(1),
                value: 0.3,
            }],
        };

        assert!(matches!(
            apply_edit_program(&recipe, &program),
            Err(AssetError::LockedParameter(ParameterId(1)))
        ));
        assert_eq!(
            get_scalar(
                &recipe,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.1
        );
    }

    #[test]
    fn descendants_and_definition_instances_are_stable() {
        let mut recipe = test_recipe();
        let child = PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(1),
            name: "Child".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        recipe.instances.insert(child.id, child);
        recipe.next_ids.part_instance = 3;

        assert_eq!(
            descendants_of(&recipe, PartInstanceId(1)).expect("root should exist"),
            vec![PartInstanceId(2)]
        );
        assert_eq!(
            instances_of_definition(&recipe, PartDefinitionId(1)),
            vec![PartInstanceId(1), PartInstanceId(2)]
        );
    }

    #[test]
    fn set_array_count_targets_array_operations() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "array count".to_owned(),
            seed: 2,
            operations: vec![AssetEdit::SetArrayCount {
                definition: PartDefinitionId(1),
                operation: OperationId(1),
                count: 4,
            }],
        };

        let edited = apply_edit_program(&recipe, &program).expect("array edit should apply");

        let definition = edited
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist");
        assert!(matches!(
            definition.geometry.operations.as_slice(),
            [ModelingOperationSpec::LinearArray { count: 4, .. }]
        ));
    }
}
