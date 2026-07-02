#![forbid(unsafe_code)]

//! Serializable, part-aware asset recipe contracts for the explicit modeling lane.
//!
//! These contracts are intentionally separate from the legacy implicit
//! `ShapeDocument` editor. IDs in this crate are semantic: part, operation,
//! region, and socket IDs must remain
//! stable when unrelated scalar parameters change. Generated vertex and face IDs
//! are owned by downstream polygon crates and are deterministic only for a given
//! topology signature. Generated boundary-loop IDs are semantic and stay stable
//! across parameter changes that preserve the feature topology.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize, de};
use thiserror::Error;

pub mod edits;
pub mod parameters;
pub mod patterns;
pub mod property_descriptor;
pub mod relationships;

pub use edits::*;
pub use parameters::*;
pub use patterns::*;
pub use property_descriptor::*;
pub use relationships::*;

/// Current schema version for asset recipes.
pub const ASSET_RECIPE_SCHEMA_VERSION: u32 = 8;
/// Package version for asset-recipe contracts.
pub const SHAPE_ASSET_CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const BOUNDARY_BEVEL_PROFILE_MIN: f32 = 0.05;
const BOUNDARY_BEVEL_PROFILE_MAX: f32 = 8.0;
const CUT_SCALAR_SAFETY_MARGIN: f32 = 0.001;
const SCALAR_RANGE_TOLERANCE: f32 = 1.0e-6;

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
    BoundaryLoopId,
    "Stable semantic identifier for a generated boundary loop."
);
const LEGACY_MISSING_BOUNDARY_LOOP: BoundaryLoopId = BoundaryLoopId(0);
const DEFAULT_RECT_CUT_CORNER_SEGMENTS: u32 = 4;
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
id_type!(
    RelationshipId,
    "Stable semantic identifier for an authored relationship contract."
);
id_type!(
    PatternId,
    "Stable semantic identifier for an authored pattern contract."
);
id_type!(
    SurfaceSlotId,
    "Stable semantic identifier for a future surface slot."
);
id_type!(
    MaterialSlotId,
    "Stable semantic identifier for a future material slot."
);
id_type!(
    CollisionBodyId,
    "Stable semantic identifier for a future collision body."
);
id_type!(
    MotionChannelId,
    "Stable semantic identifier for a future motion channel."
);
id_type!(
    TerrainPatchId,
    "Stable semantic identifier for a future terrain patch."
);
id_type!(
    ExportProfileId,
    "Stable semantic identifier for an export profile shell."
);
id_type!(
    AuthoringOpId,
    "Stable semantic identifier for an authoring operation shell."
);
id_type!(
    ValidationReportId,
    "Stable semantic identifier for a validation report shell."
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
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Locked instances that mutation programs must not edit directly.
    #[serde(default)]
    pub instance_locks: BTreeSet<PartInstanceId>,
    /// Locked instance subtrees. The root and every descendant are protected.
    #[serde(default)]
    pub subtree_locks: BTreeSet<PartInstanceId>,
    /// Locked part definition topology. Shape-preserving value edits remain valid.
    #[serde(default)]
    pub topology_locks: BTreeSet<PartDefinitionId>,
    /// Asset-level constraints.
    pub constraints: Vec<AssetConstraint>,
    /// Authored geometric relationship policies between part instances.
    #[serde(default)]
    pub relationships: Vec<AssetRelationshipPolicy>,
    /// Authored variation hints that do not affect hierarchy or generation semantics.
    #[serde(default)]
    pub variation: AuthoredVariationMetadata,
    /// v8 semantic shells reserved for the canonical Orchard asset lane.
    #[serde(default)]
    pub semantic: AssetRecipeSemanticShells,
    /// Next semantic ID counters.
    pub next_ids: AssetIdCounters,
}

#[derive(Deserialize)]
struct AssetRecipeWire {
    schema_version: u32,
    id: AssetId,
    title: String,
    definitions: BTreeMap<PartDefinitionId, PartDefinitionWire>,
    instances: BTreeMap<PartInstanceId, PartInstance>,
    root_instances: Vec<PartInstanceId>,
    parameters: BTreeMap<ParameterId, ParameterDescriptor>,
    locks: BTreeSet<ParameterId>,
    #[serde(default)]
    instance_locks: BTreeSet<PartInstanceId>,
    #[serde(default)]
    subtree_locks: BTreeSet<PartInstanceId>,
    #[serde(default)]
    topology_locks: BTreeSet<PartDefinitionId>,
    constraints: Vec<AssetConstraint>,
    #[serde(default)]
    relationships: Vec<AssetRelationshipPolicy>,
    #[serde(default)]
    variation: AuthoredVariationMetadata,
    #[serde(default)]
    semantic: AssetRecipeSemanticShells,
    next_ids: AssetIdCounters,
}

#[derive(Deserialize)]
struct PartDefinitionWire {
    id: PartDefinitionId,
    name: String,
    tags: BTreeSet<String>,
    geometry: GeometryRecipeWire,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    local_pivot: Frame3,
    variant_group: Option<String>,
    production_hints: Option<ProductionHints>,
}

#[derive(Deserialize)]
struct GeometryRecipeWire {
    source: GeometrySource,
    operations: Vec<ModelingOperationSpecWire>,
}

impl AssetRecipeWire {
    fn into_recipe(self) -> Result<AssetRecipe, String> {
        let schema_version = self.schema_version;
        let mut definitions = BTreeMap::new();
        for (id, definition) in self.definitions {
            definitions.insert(id, definition.into_definition(schema_version)?);
        }
        let mut recipe = AssetRecipe {
            schema_version: migrated_asset_recipe_schema_version(schema_version),
            id: self.id,
            title: self.title,
            definitions,
            instances: self.instances,
            root_instances: self.root_instances,
            parameters: self.parameters,
            locks: self.locks,
            instance_locks: self.instance_locks,
            subtree_locks: self.subtree_locks,
            topology_locks: self.topology_locks,
            constraints: self.constraints,
            relationships: self.relationships,
            variation: self.variation,
            semantic: self.semantic,
            next_ids: self.next_ids,
        };
        if schema_version < 4 {
            migrate_legacy_cut_boundary_loops(&mut recipe);
        }
        Ok(recipe)
    }
}

impl PartDefinitionWire {
    fn into_definition(self, schema_version: u32) -> Result<PartDefinition, String> {
        let mut operations = Vec::with_capacity(self.geometry.operations.len());
        for operation in self.geometry.operations {
            operations.push(operation.into_operation(schema_version)?);
        }
        Ok(PartDefinition {
            id: self.id,
            name: self.name,
            tags: self.tags,
            geometry: GeometryRecipe {
                source: self.geometry.source,
                operations,
            },
            regions: self.regions,
            sockets: self.sockets,
            local_pivot: self.local_pivot,
            variant_group: self.variant_group,
            production_hints: self.production_hints,
        })
    }
}

impl<'de> Deserialize<'de> for AssetRecipe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AssetRecipeWire::deserialize(deserializer)?
            .into_recipe()
            .map_err(de::Error::custom)
    }
}

fn migrated_asset_recipe_schema_version(schema_version: u32) -> u32 {
    if matches!(schema_version, 1..=7) {
        ASSET_RECIPE_SCHEMA_VERSION
    } else {
        schema_version
    }
}

fn migrate_legacy_cut_boundary_loops(recipe: &mut AssetRecipe) {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .filter_map(|id| (id != LEGACY_MISSING_BOUNDARY_LOOP).then_some(id.0))
        .collect::<BTreeSet<_>>();
    let mut next = recipe
        .next_ids
        .boundary_loop
        .max(used.last().copied().unwrap_or_default().saturating_add(1))
        .max(1);

    for definition in recipe.definitions.values_mut() {
        for operation in &mut definition.geometry.operations {
            match operation {
                ModelingOperationSpec::RecessedPanelCut {
                    entry_loop,
                    floor_loop,
                    ..
                } => {
                    if *entry_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *entry_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                    if *floor_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *floor_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                }
                ModelingOperationSpec::RectangularThroughCut {
                    entry_loop,
                    exit_loop,
                    ..
                }
                | ModelingOperationSpec::CircularThroughCut {
                    entry_loop,
                    exit_loop,
                    ..
                } => {
                    if *entry_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *entry_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                    if *exit_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *exit_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                }
                ModelingOperationSpec::TransformGeometry { .. }
                | ModelingOperationSpec::SetBevelProfile { .. }
                | ModelingOperationSpec::AddPanel { .. }
                | ModelingOperationSpec::AddTrim { .. }
                | ModelingOperationSpec::BevelBoundaryLoop { .. }
                | ModelingOperationSpec::MirrorInstances { .. }
                | ModelingOperationSpec::LinearArray { .. }
                | ModelingOperationSpec::RadialArray { .. }
                | ModelingOperationSpec::ReservedBoolean { .. }
                | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
            }
        }
    }

    recipe.next_ids.boundary_loop = recipe.next_ids.boundary_loop.max(next);
}

fn allocate_migrated_boundary_loop(used: &mut BTreeSet<u64>, next: &mut u64) -> BoundaryLoopId {
    while *next == 0 || used.contains(&*next) {
        *next = next.saturating_add(1);
    }
    let id = *next;
    used.insert(id);
    *next = next.saturating_add(1);
    BoundaryLoopId(id)
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
            instance_locks: BTreeSet::new(),
            subtree_locks: BTreeSet::new(),
            topology_locks: BTreeSet::new(),
            constraints: Vec::new(),
            relationships: Vec::new(),
            variation: AuthoredVariationMetadata::default(),
            semantic: AssetRecipeSemanticShells::default(),
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

    /// Allocate the next boundary loop ID.
    pub fn allocate_boundary_loop_id(&mut self) -> BoundaryLoopId {
        allocate_boundary_loop_id(self)
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

    /// Allocate the next relationship ID.
    pub fn allocate_relationship_id(&mut self) -> RelationshipId {
        let id = RelationshipId(self.next_ids.relationship);
        self.next_ids.relationship = self.next_ids.relationship.saturating_add(1);
        id
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
    /// Next boundary loop ID.
    #[serde(default = "default_id_counter")]
    pub boundary_loop: u64,
    /// Next socket ID.
    pub socket: u64,
    /// Next parameter ID.
    pub parameter: u64,
    /// Next revision ID.
    pub revision: u64,
    /// Next relationship contract ID.
    #[serde(default = "default_id_counter")]
    pub relationship: u64,
    /// Next pattern contract ID.
    #[serde(default = "default_id_counter")]
    pub pattern: u64,
    /// Next surface slot ID.
    #[serde(default = "default_id_counter")]
    pub surface_slot: u64,
    /// Next material slot ID.
    #[serde(default = "default_id_counter")]
    pub material_slot: u64,
    /// Next collision body ID.
    #[serde(default = "default_id_counter")]
    pub collision_body: u64,
    /// Next motion channel ID.
    #[serde(default = "default_id_counter")]
    pub motion_channel: u64,
    /// Next terrain patch ID.
    #[serde(default = "default_id_counter")]
    pub terrain_patch: u64,
    /// Next export profile ID.
    #[serde(default = "default_id_counter")]
    pub export_profile: u64,
    /// Next authoring operation ID.
    #[serde(default = "default_id_counter")]
    pub authoring_op: u64,
    /// Next validation report ID.
    #[serde(default = "default_id_counter")]
    pub validation_report: u64,
}

impl Default for AssetIdCounters {
    fn default() -> Self {
        Self {
            part_definition: 1,
            part_instance: 1,
            operation: 1,
            region: 1,
            boundary_loop: 1,
            socket: 1,
            parameter: 1,
            revision: 1,
            relationship: 1,
            pattern: 1,
            surface_slot: 1,
            material_slot: 1,
            collision_body: 1,
            motion_channel: 1,
            terrain_patch: 1,
            export_profile: 1,
            authoring_op: 1,
            validation_report: 1,
        }
    }
}

fn default_id_counter() -> u64 {
    1
}

/// v8 semantic shells reserved for the canonical Orchard asset lane.
///
/// These fields are serialized contracts only. They do not implement
/// material, collision, motion, terrain, export, or public publishing behavior.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssetRecipeSemanticShells {
    /// Canonical relationship contracts reserved for composition semantics.
    #[serde(default)]
    pub relationships: BTreeMap<RelationshipId, RelationshipContract>,
    /// Canonical pattern contracts reserved for repetition semantics.
    #[serde(default)]
    pub patterns: BTreeMap<PatternId, PatternContract>,
    /// Future primitive-aware surface slots.
    #[serde(default)]
    pub surface_slots: BTreeMap<SurfaceSlotId, SurfaceSlotShell>,
    /// Future material slots. These are not material looks.
    #[serde(default)]
    pub material_slots: BTreeMap<MaterialSlotId, MaterialSlotShell>,
    /// Future collision body declarations. These are not collision output.
    #[serde(default)]
    pub collision_bodies: BTreeMap<CollisionBodyId, CollisionBodyShell>,
    /// Future motion channel declarations. These are not animation output.
    #[serde(default)]
    pub motion_channels: BTreeMap<MotionChannelId, MotionChannelShell>,
    /// Future terrain patch declarations. These are not terrain output.
    #[serde(default)]
    pub terrain_patches: BTreeMap<TerrainPatchId, TerrainPatchShell>,
    /// Export profile shells used by later export reports.
    #[serde(default)]
    pub export_profiles: BTreeMap<ExportProfileId, ExportProfileShell>,
    /// Authoring operation shells used by later replay logs.
    #[serde(default)]
    pub authoring_ops: BTreeMap<AuthoringOpId, AuthoringOpShell>,
    /// Validation report shells used by later proof gates.
    #[serde(default)]
    pub validation_reports: BTreeMap<ValidationReportId, ValidationReportShell>,
    /// Current review/publication boundary for this recipe.
    #[serde(default)]
    pub review_state: ReviewState,
    /// Copy-on-write lineage reserved for later breadcrumbs.
    #[serde(default)]
    pub copy_on_write_lineage: CopyOnWriteLineage,
    /// Effect hashes reserved for later deterministic evidence.
    #[serde(default)]
    pub effect_hashes: EffectHashes,
    /// Capability include/exclude summary reserved for export reports.
    #[serde(default)]
    pub export_includes: ExportIncludes,
}

/// Canonical authored relationship contract shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipContract {
    /// Stable relationship ID.
    pub id: RelationshipId,
    /// Relationship semantic kind.
    pub relationship_type: RelationshipType,
    /// Optional parent endpoint for validation once populated.
    #[serde(default)]
    pub parent: Option<PartInstanceId>,
    /// Optional child endpoint for validation once populated.
    #[serde(default)]
    pub child: Option<PartInstanceId>,
    /// Source parent node reference before concrete instance IDs are assigned.
    #[serde(default)]
    pub parent_node_ref: Option<String>,
    /// Source child node reference before concrete instance IDs are assigned.
    #[serde(default)]
    pub child_node_ref: Option<String>,
    /// Parent anchor ID from an anchor-based composition lane.
    #[serde(default)]
    pub parent_anchor_id: Option<String>,
    /// Child anchor ID from an anchor-based composition lane.
    #[serde(default)]
    pub child_anchor_id: Option<String>,
    /// Product-safe label for future UI/reports.
    #[serde(default)]
    pub label: String,
    /// Optional export profile expected to realize this relationship.
    #[serde(default)]
    pub export_profile: Option<ExportProfileId>,
    /// Placement policy shell.
    #[serde(default)]
    pub placement_policy: PlacementPolicy,
    /// Orientation policy shell.
    #[serde(default)]
    pub orientation_policy: OrientationPolicy,
    /// Scale policy shell.
    #[serde(default)]
    pub scale_policy: ScalePolicy,
    /// Contact policy shell.
    #[serde(default)]
    pub contact_policy: ContactPolicy,
    /// Edit policy shell.
    #[serde(default)]
    pub edit_policy: RelationshipEditPolicy,
    /// Selection policy shell.
    #[serde(default)]
    pub selection_policy: SelectionPolicy,
    /// Reset policy shell.
    #[serde(default)]
    pub reset_policy: ResetPolicy,
    /// Export realization policy shell.
    #[serde(default)]
    pub export_realization: ExportRealizationPolicy,
}

/// Relationship semantic kind reserved for composition work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Child keeps an authored rigid relationship to a parent.
    RigidChild,
    /// Child is mounted on a parent surface.
    SurfaceMounted,
    /// Child is an embedded feature in a parent.
    EmbeddedFeature,
    /// Child is a socketed accessory.
    SocketedAccessory,
    /// Child is attached through a future joint contract.
    JointAttached,
    /// Child has intentional authored offset.
    IntentionalOffset,
    /// Child is a future VFX relationship.
    VfxChild,
    /// Child is produced by a pattern.
    PatternInstance,
    /// Child is a future collision proxy.
    CollisionProxy,
    /// Child is render-only decoration.
    RenderOnlyDecoration,
    /// Relationship may later be baked as a union.
    BakedUnion,
}

/// Canonical authored pattern contract shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternContract {
    /// Stable pattern ID.
    pub id: PatternId,
    /// Pattern semantic kind.
    pub pattern_type: PatternType,
    /// Optional source instance for validation once populated.
    #[serde(default)]
    pub source_instance: Option<PartInstanceId>,
    /// Optional authored count reserved for later evaluation.
    #[serde(default)]
    pub count: Option<u32>,
    /// Product-safe label for future UI/reports.
    #[serde(default)]
    pub label: String,
    /// Count policy shell.
    #[serde(default)]
    pub count_policy: PatternCountPolicy,
    /// Optional density policy shell.
    #[serde(default)]
    pub density_policy: Option<PatternDensityPolicy>,
    /// Export instancing policy shell.
    #[serde(default)]
    pub export_instancing: PatternExportInstancingPolicy,
    /// Linear axis for V0 deterministic evaluation.
    #[serde(default)]
    pub linear_axis: Option<PatternAxis>,
    /// Linear spacing for V0 deterministic evaluation.
    #[serde(default)]
    pub spacing: Option<f32>,
    /// Generated occurrence ID policy.
    #[serde(default)]
    pub generated_id_policy: GeneratedIdPolicy,
}

/// Pattern semantic kind reserved for repetition work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// Linear repeated occurrences.
    Linear,
    /// Radial repeated occurrences.
    Radial,
    /// Grid repeated occurrences.
    Grid,
    /// Mirrored occurrence.
    Mirror,
    /// Occurrences placed along a curve.
    AlongCurve,
    /// Occurrences placed on a surface.
    OnSurface,
    /// Scattered occurrences.
    Scatter,
}

/// Future surface slot shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceSlotShell {
    /// Stable surface slot ID.
    pub id: SurfaceSlotId,
    /// Optional owning definition for validation once populated.
    #[serde(default)]
    pub owner_definition: Option<PartDefinitionId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future material slot shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSlotShell {
    /// Stable material slot ID.
    pub id: MaterialSlotId,
    /// Optional surface slot this material slot would bind to.
    #[serde(default)]
    pub surface_slot: Option<SurfaceSlotId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future collision body shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionBodyShell {
    /// Stable collision body ID.
    pub id: CollisionBodyId,
    /// Optional target instance for validation once populated.
    #[serde(default)]
    pub target_instance: Option<PartInstanceId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future motion channel shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionChannelShell {
    /// Stable motion channel ID.
    pub id: MotionChannelId,
    /// Optional target instance for validation once populated.
    #[serde(default)]
    pub target_instance: Option<PartInstanceId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future terrain patch shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainPatchShell {
    /// Stable terrain patch ID.
    pub id: TerrainPatchId,
    /// Optional root instance for validation once populated.
    #[serde(default)]
    pub root_instance: Option<PartInstanceId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future export profile shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportProfileShell {
    /// Stable export profile ID.
    pub id: ExportProfileId,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
    /// Includes/excludes this profile may later report.
    #[serde(default)]
    pub includes: ExportIncludes,
}

/// Future authoring operation shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringOpShell {
    /// Stable authoring operation ID.
    pub id: AuthoringOpId,
    /// Optional target parameter for validation once populated.
    #[serde(default)]
    pub target_parameter: Option<ParameterId>,
    /// Optional target instance for validation once populated.
    #[serde(default)]
    pub target_instance: Option<PartInstanceId>,
    /// Product-safe label.
    #[serde(default)]
    pub label: String,
}

/// Future validation report shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReportShell {
    /// Stable validation report ID.
    pub id: ValidationReportId,
    /// Optional export profile this report belongs to.
    #[serde(default)]
    pub export_profile: Option<ExportProfileId>,
    /// Product-safe status label.
    #[serde(default)]
    pub status: String,
}

/// Review status shell. Phase A keeps outputs Draft and review-required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewState {
    /// Review tier.
    pub tier: ReviewTier,
    /// Human review is still required.
    pub human_review_required: bool,
    /// Public publishing is not allowed in Phase A.
    pub publish_allowed: bool,
    /// Public catalog visibility is not allowed in Phase A.
    pub public_catalog_visible: bool,
}

impl Default for ReviewState {
    fn default() -> Self {
        Self {
            tier: ReviewTier::Draft,
            human_review_required: true,
            publish_allowed: false,
            public_catalog_visible: false,
        }
    }
}

/// Review tier reserved for future gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTier {
    /// Draft output, not approved.
    Draft,
    /// Explicit review is required.
    ReviewRequired,
    /// Reserved for later evidence-backed review.
    Reviewed,
    /// Reserved for later publishing gates.
    Published,
}

/// Copy-on-write lineage shell reserved for later breadcrumbs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CopyOnWriteLineage {
    /// Optional source asset.
    #[serde(default)]
    pub source_asset: Option<AssetId>,
    /// Optional source revision.
    #[serde(default)]
    pub source_revision: Option<RevisionId>,
}

/// Deterministic effect hashes reserved for later evidence.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EffectHashes {
    /// Named hashes in stable key order.
    #[serde(default)]
    pub hashes: BTreeMap<String, String>,
}

/// Export/proof capability include flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportIncludes {
    /// Geometry was included.
    pub includes_geometry: bool,
    /// UVs were included.
    pub includes_uvs: bool,
    /// Texture files were included.
    pub includes_textures: bool,
    /// Material looks were included.
    pub includes_material_looks: bool,
    /// Collision was included.
    pub includes_collision: bool,
    /// Gameplay metadata was included.
    pub includes_gameplay_metadata: bool,
    /// Rig data was included.
    pub includes_rig: bool,
    /// Skinning was included.
    pub includes_skinning: bool,
    /// Animation was included.
    pub includes_animation: bool,
    /// Terrain collision was included.
    pub includes_terrain_collision: bool,
    /// Godot scene output was included.
    pub includes_godot_scene: bool,
    /// Output is game-ready. Must remain false in Phase A.
    pub game_ready: bool,
    /// Human review is required.
    pub human_review_required: bool,
}

impl Default for ExportIncludes {
    fn default() -> Self {
        Self {
            includes_geometry: false,
            includes_uvs: false,
            includes_textures: false,
            includes_material_looks: false,
            includes_collision: false,
            includes_gameplay_metadata: false,
            includes_rig: false,
            includes_skinning: false,
            includes_animation: false,
            includes_terrain_collision: false,
            includes_godot_scene: false,
            game_ready: false,
            human_review_required: true,
        }
    }
}

/// Non-authoritative variation metadata authored alongside a recipe.
///
/// These hints describe which choices are useful for search or UI tools. They
/// do not generate geometry and do not change the instance hierarchy.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AuthoredVariationMetadata {
    /// Instances that may be omitted by an authoring or search tool.
    pub optional_instances: BTreeSet<PartInstanceId>,
    /// Named groups of interchangeable part definitions.
    pub replacement_groups: BTreeMap<String, ReplacementGroupHint>,
    /// Valid authored count ranges for array operations.
    pub count_ranges: BTreeMap<OperationId, CountRangeHint>,
    /// Parameter-specific authored ranges that override descriptor UI ranges.
    pub parameter_range_overrides: BTreeMap<ParameterId, ParameterRangeOverride>,
    /// Named repeated semantic cuts that should be edited as a group.
    #[serde(default)]
    pub semantic_cut_groups: BTreeMap<String, SemanticCutGroupHint>,
}

/// Replacement group for interchangeable part definitions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReplacementGroupHint {
    /// Definitions that belong to this replacement group.
    pub definitions: BTreeSet<PartDefinitionId>,
}

/// Authored count range for a deterministic array operation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountRangeHint {
    /// Minimum authored count.
    pub minimum: u32,
    /// Maximum authored count.
    pub maximum: u32,
}

/// Authored repeated-cut group for novice-facing controls and search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCutGroupHint {
    /// Human-facing group label.
    pub label: String,
    /// Definition containing every grouped operation.
    pub definition: PartDefinitionId,
    /// Cut operations that participate in stable group order.
    pub operations: Vec<OperationId>,
    /// Author intent for the repeated feature.
    pub role: CutGroupRole,
    /// Optional count range reserved for future add/remove controls.
    #[serde(default)]
    pub count_range: Option<CountRangeHint>,
}

/// Author intent for a semantic cut group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CutGroupRole {
    /// Repeated mounting holes or bolt holes.
    MountHoles,
    /// Repeated ventilation slots.
    Vents,
    /// Repeated recessed panels.
    Recesses,
    /// Project-specific semantic role.
    Custom(String),
}

/// Authored parameter range override.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterRangeOverride {
    /// Minimum authored scalar value.
    pub minimum: f32,
    /// Maximum authored scalar value.
    pub maximum: f32,
    /// Optional UI step override.
    pub step: Option<f32>,
    /// Optional mutation sigma override.
    pub mutation_sigma: Option<f32>,
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

/// Supported planar host face for controlled semantic cuts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlanarCutFace {
    /// Local +X face.
    PositiveX,
    /// Local -X face.
    NegativeX,
    /// Local +Y face.
    PositiveY,
    /// Local -Y face.
    NegativeY,
    /// Local +Z face.
    PositiveZ,
    /// Local -Z face.
    NegativeZ,
}

/// Edge treatment emitted around generated cut boundaries.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CutEdgeTreatment {
    /// Keep the generated boundary as a hard edge.
    Hard,
    /// Mark the generated boundary as eligible for a later bevel propagation pass.
    BevelEligible,
}

/// How a modeling operation depends on an existing boundary loop.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BoundaryLoopDependencyMode {
    /// The input loop remains live after the operation.
    Reference,
    /// The input loop is replaced by this operation's output loops.
    Consume,
}

/// Boundary-loop lifecycle dependency declared by a modeling operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryLoopDependency {
    /// Existing loop used by the operation.
    pub input: BoundaryLoopId,
    /// Whether the input remains live or is replaced.
    pub mode: BoundaryLoopDependencyMode,
    /// New loops emitted as replacements or related outputs.
    pub outputs: Vec<BoundaryLoopId>,
}

/// Deterministic modeling operation specification.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Analytically create a recessed panel in a supported planar host face.
    RecessedPanelCut {
        /// Stable operation ID.
        operation: OperationId,
        /// Target planar region on the host.
        region: RegionId,
        /// Target local face.
        face: PlanarCutFace,
        /// Center in face-local coordinates.
        center: [f32; 2],
        /// Panel size in face-local coordinates.
        size: [f32; 2],
        /// Recess depth along the inward face normal.
        depth: f32,
        /// Corner radius in the face plane.
        corner_radius: f32,
        /// Rim width between the cut opening and surviving host face.
        rim_width: f32,
        /// Deterministic segment count per rounded corner.
        corner_segments: u32,
        /// Generated boundary loop at the entry cut edge.
        entry_loop: BoundaryLoopId,
        /// Generated boundary loop around the recessed floor edge.
        floor_loop: BoundaryLoopId,
        /// Region assigned to the surviving outer host surface.
        outer_region: RegionId,
        /// Region assigned to the rim/border around the recess.
        rim_region: RegionId,
        /// Region assigned to the cut walls.
        wall_region: RegionId,
        /// Region assigned to the recessed floor.
        floor_region: RegionId,
        /// Edge metadata emitted around the boundary loop.
        edge_treatment: CutEdgeTreatment,
    },
    /// Analytically create a rectangular through-cut in a supported planar host.
    RectangularThroughCut {
        /// Stable operation ID.
        operation: OperationId,
        /// Target planar region on the host.
        region: RegionId,
        /// Target local face.
        face: PlanarCutFace,
        /// Center in face-local coordinates.
        center: [f32; 2],
        /// Opening size in face-local coordinates.
        size: [f32; 2],
        /// Corner radius in the face plane.
        corner_radius: f32,
        /// Rim width between the opening and surviving host face.
        rim_width: f32,
        /// Deterministic segment count per rounded corner.
        corner_segments: u32,
        /// Generated boundary loop at the entry cut edge.
        entry_loop: BoundaryLoopId,
        /// Generated boundary loop at the exit cut edge.
        exit_loop: BoundaryLoopId,
        /// Region assigned to the surviving outer host surface.
        outer_region: RegionId,
        /// Region assigned to the opening rim.
        rim_region: RegionId,
        /// Region assigned to the through-cut walls.
        wall_region: RegionId,
        /// Edge metadata emitted around the boundary loop.
        edge_treatment: CutEdgeTreatment,
    },
    /// Analytically create a circular through-cut in a supported planar host.
    CircularThroughCut {
        /// Stable operation ID.
        operation: OperationId,
        /// Target planar region on the host.
        region: RegionId,
        /// Target local face.
        face: PlanarCutFace,
        /// Center in face-local coordinates.
        center: [f32; 2],
        /// Opening radius.
        radius: f32,
        /// Deterministic radial segment count.
        radial_segments: u32,
        /// Rim width between the circular opening and surviving host face.
        rim_width: f32,
        /// Generated boundary loop at the entry cut edge.
        entry_loop: BoundaryLoopId,
        /// Generated boundary loop at the exit cut edge.
        exit_loop: BoundaryLoopId,
        /// Region assigned to the surviving outer host surface.
        outer_region: RegionId,
        /// Region assigned to the opening rim.
        rim_region: RegionId,
        /// Region assigned to the through-cut walls.
        wall_region: RegionId,
        /// Edge metadata emitted around the boundary loop.
        edge_treatment: CutEdgeTreatment,
    },
    /// Replace one generated boundary loop with a controlled bevel band.
    BevelBoundaryLoop {
        /// Stable operation ID.
        operation: OperationId,
        /// Existing live boundary loop consumed by the bevel.
        target_loop: BoundaryLoopId,
        /// Uniform bevel width.
        width: f32,
        /// Deterministic band segment count.
        segments: u32,
        /// Profile exponent; 1.0 is linear.
        profile: f32,
        /// Region assigned to the generated bevel band.
        bevel_region: RegionId,
        /// Replacement loop on the outer/surface side of the bevel.
        outer_replacement_loop: BoundaryLoopId,
        /// Replacement loop on the inner/wall side of the bevel.
        inner_replacement_loop: BoundaryLoopId,
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
            | Self::RecessedPanelCut { operation, .. }
            | Self::RectangularThroughCut { operation, .. }
            | Self::CircularThroughCut { operation, .. }
            | Self::BevelBoundaryLoop { operation, .. }
            | Self::MirrorInstances { operation, .. }
            | Self::LinearArray { operation, .. }
            | Self::RadialArray { operation, .. }
            | Self::ReservedBoolean { operation, .. }
            | Self::ReservedDeformationProgram { operation, .. } => *operation,
        }
    }

    /// Return the coarse execution phase for this operation.
    #[must_use]
    pub fn phase(&self) -> OperationPhase {
        match self {
            Self::SetBevelProfile { .. } | Self::BevelBoundaryLoop { .. } => {
                OperationPhase::BoundaryTreatment
            }
            Self::AddPanel { .. }
            | Self::AddTrim { .. }
            | Self::RecessedPanelCut { .. }
            | Self::RectangularThroughCut { .. }
            | Self::CircularThroughCut { .. }
            | Self::ReservedBoolean { .. } => OperationPhase::LocalTopology,
            Self::TransformGeometry { .. } | Self::ReservedDeformationProgram { .. } => {
                OperationPhase::LocalTransform
            }
            Self::MirrorInstances { .. } | Self::LinearArray { .. } | Self::RadialArray { .. } => {
                OperationPhase::AssemblyGeneration
            }
        }
    }

    /// Return boundary loops directly authored by this operation.
    #[must_use]
    pub fn direct_boundary_loop_outputs(&self) -> Vec<BoundaryLoopId> {
        match self {
            Self::RecessedPanelCut {
                entry_loop,
                floor_loop,
                ..
            } => vec![*entry_loop, *floor_loop],
            Self::RectangularThroughCut {
                entry_loop,
                exit_loop,
                ..
            }
            | Self::CircularThroughCut {
                entry_loop,
                exit_loop,
                ..
            } => vec![*entry_loop, *exit_loop],
            Self::TransformGeometry { .. }
            | Self::SetBevelProfile { .. }
            | Self::AddPanel { .. }
            | Self::AddTrim { .. }
            | Self::BevelBoundaryLoop { .. }
            | Self::MirrorInstances { .. }
            | Self::LinearArray { .. }
            | Self::RadialArray { .. }
            | Self::ReservedBoolean { .. }
            | Self::ReservedDeformationProgram { .. } => Vec::new(),
        }
    }

    /// Return generated boundary loops directly authored by this operation.
    #[must_use]
    pub fn produced_boundary_loop_ids(&self) -> Vec<BoundaryLoopId> {
        self.direct_boundary_loop_outputs()
    }

    /// Return every boundary loop declared as an operation output.
    #[must_use]
    pub fn all_declared_boundary_loop_outputs(&self) -> Vec<BoundaryLoopId> {
        let mut outputs = Vec::new();
        let mut seen = BTreeSet::new();
        for output in self.direct_boundary_loop_outputs() {
            if seen.insert(output) {
                outputs.push(output);
            }
        }
        for dependency in self.boundary_loop_dependencies() {
            for output in dependency.outputs {
                if seen.insert(output) {
                    outputs.push(output);
                }
            }
        }
        outputs
    }

    /// Return generated boundary loops authored by this operation.
    #[must_use]
    pub fn boundary_loop_ids(&self) -> Vec<BoundaryLoopId> {
        self.all_declared_boundary_loop_outputs()
    }

    /// Return boundary-loop lifecycle dependencies declared by this operation.
    #[must_use]
    pub fn boundary_loop_dependencies(&self) -> Vec<BoundaryLoopDependency> {
        match self {
            Self::BevelBoundaryLoop {
                target_loop,
                outer_replacement_loop,
                inner_replacement_loop,
                ..
            } => vec![BoundaryLoopDependency {
                input: *target_loop,
                mode: BoundaryLoopDependencyMode::Consume,
                outputs: vec![*outer_replacement_loop, *inner_replacement_loop],
            }],
            _ => Vec::new(),
        }
    }

    /// Return operation-emitted region IDs that are not necessarily declared as base regions.
    #[must_use]
    pub fn generated_region_ids(&self) -> Vec<RegionId> {
        match self {
            Self::RecessedPanelCut {
                outer_region,
                rim_region,
                wall_region,
                floor_region,
                ..
            } => vec![*outer_region, *rim_region, *wall_region, *floor_region],
            Self::RectangularThroughCut {
                outer_region,
                rim_region,
                wall_region,
                ..
            }
            | Self::CircularThroughCut {
                outer_region,
                rim_region,
                wall_region,
                ..
            } => vec![*outer_region, *rim_region, *wall_region],
            Self::BevelBoundaryLoop { bevel_region, .. } => vec![*bevel_region],
            Self::TransformGeometry { .. }
            | Self::SetBevelProfile { .. }
            | Self::AddPanel { .. }
            | Self::AddTrim { .. }
            | Self::MirrorInstances { .. }
            | Self::LinearArray { .. }
            | Self::RadialArray { .. }
            | Self::ReservedBoolean { .. }
            | Self::ReservedDeformationProgram { .. } => Vec::new(),
        }
    }
}

/// Inclusive scalar range derived from semantic operation dependencies.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct OperationScalarRange {
    /// Smallest accepted value.
    pub minimum: f32,
    /// Largest accepted value.
    pub maximum: f32,
}

impl OperationScalarRange {
    fn new(minimum: f32, maximum: f32) -> Option<Self> {
        if minimum.is_finite() && maximum.is_finite() && minimum <= maximum {
            Some(Self { minimum, maximum })
        } else {
            None
        }
    }

    /// Return true when `value` lies inside the range, allowing a small
    /// floating-point tolerance for UI and search-generated scalars.
    #[must_use]
    pub fn contains(self, value: f32) -> bool {
        value.is_finite()
            && value + SCALAR_RANGE_TOLERANCE >= self.minimum
            && value - SCALAR_RANGE_TOLERANCE <= self.maximum
    }
}

/// Return a dependency-aware scalar range for an operation field.
///
/// The range is conservative and mirrors the cut generator's hard rejection
/// rules so UI controls, candidate search, and direct edit commands can avoid
/// creating compile-invalid recipes when boundary-loop bevels depend on a cut.
#[must_use]
pub fn feasible_operation_scalar_range(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    field: &str,
) -> Option<OperationScalarRange> {
    let definition_spec = recipe.definitions.get(&definition)?;
    let operation_spec = definition_spec
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)?;
    let host = operation_cut_face(operation_spec)
        .and_then(|face| cut_host_bounds_for_source(&definition_spec.geometry.source, face));
    match operation_spec {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            entry_loop,
            floor_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *floor_loop,
            );
            recessed_cut_scalar_range(
                field,
                host,
                RecessedCutScalars {
                    center: *center,
                    size: *size,
                    depth: *depth,
                    corner_radius: *corner_radius,
                    rim_width: *rim_width,
                },
                bevels,
            )
        }
        ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            corner_radius,
            rim_width,
            entry_loop,
            exit_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *exit_loop,
            );
            rectangular_through_cut_scalar_range(
                field,
                host,
                *center,
                *size,
                *corner_radius,
                *rim_width,
                bevels,
            )
        }
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            rim_width,
            entry_loop,
            exit_loop,
            ..
        } => {
            let bevels = CutLoopBevelWidths::for_loops(
                &definition_spec.geometry.operations,
                *entry_loop,
                *exit_loop,
            );
            circular_through_cut_scalar_range(field, host, *center, *radius, *rim_width, bevels)
        }
        ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            profile,
            ..
        } => match field {
            "bevel_boundary_loop.width" => {
                feasible_boundary_loop_bevel_width_range(recipe, definition, *target_loop)
            }
            "bevel_boundary_loop.profile" => OperationScalarRange::new(
                BOUNDARY_BEVEL_PROFILE_MIN.min(*profile),
                BOUNDARY_BEVEL_PROFILE_MAX.max(*profile),
            ),
            "bevel_boundary_loop.segments" => OperationScalarRange::new(1.0, 128.0),
            _ => None,
        },
        _ => None,
    }
}

/// Return the safe width range for adding or editing a boundary-loop bevel.
#[must_use]
pub fn feasible_boundary_loop_bevel_width_range(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    target_loop: BoundaryLoopId,
) -> Option<OperationScalarRange> {
    let definition_spec = recipe.definitions.get(&definition)?;
    let operations = &definition_spec.geometry.operations;
    for operation in operations {
        let host = operation_cut_face(operation)
            .and_then(|face| cut_host_bounds_for_source(&definition_spec.geometry.source, face));
        match operation {
            ModelingOperationSpec::RecessedPanelCut {
                size,
                depth,
                corner_radius,
                rim_width,
                entry_loop,
                floor_loop,
                ..
            } if *entry_loop == target_loop || *floor_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *floor_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(rect_loop_radius(*size) * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                maximum = maximum.min(*depth - sibling - CUT_SCALAR_SAFETY_MARGIN);
                if *floor_loop == target_loop && *corner_radius > 0.0 {
                    maximum = maximum.min(*corner_radius - CUT_SCALAR_SAFETY_MARGIN);
                }
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness * 0.45);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            ModelingOperationSpec::RectangularThroughCut {
                size,
                rim_width,
                entry_loop,
                exit_loop,
                ..
            } if *entry_loop == target_loop || *exit_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *exit_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(rect_loop_radius(*size) * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness - sibling - CUT_SCALAR_SAFETY_MARGIN);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            ModelingOperationSpec::CircularThroughCut {
                radius,
                rim_width,
                entry_loop,
                exit_loop,
                ..
            } if *entry_loop == target_loop || *exit_loop == target_loop => {
                let bevels = CutLoopBevelWidths::for_loops(operations, *entry_loop, *exit_loop);
                let sibling = if *entry_loop == target_loop {
                    bevels.secondary
                } else {
                    bevels.entry
                };
                let mut maximum = rim_width.abs() - CUT_SCALAR_SAFETY_MARGIN;
                maximum = maximum.min(radius.abs() * 0.5 - CUT_SCALAR_SAFETY_MARGIN);
                if let Some(host) = host {
                    maximum = maximum.min(host.thickness - sibling - CUT_SCALAR_SAFETY_MARGIN);
                }
                return OperationScalarRange::new(0.001, maximum);
            }
            _ => {}
        }
    }
    None
}

/// Return a dependency-aware range for descriptor-backed geometry scalars.
#[must_use]
pub fn feasible_scalar_path_range(
    recipe: &AssetRecipe,
    path: &str,
) -> Option<OperationScalarRange> {
    let parts = path.split('.').collect::<Vec<_>>();
    let ["definition", definition, "geometry", source, rest @ ..] = parts.as_slice() else {
        return None;
    };
    let definition = definition.parse().ok().map(PartDefinitionId)?;
    let definition_spec = recipe.definitions.get(&definition)?;
    match (*source, rest) {
        ("plate", ["thickness"]) => {
            let minimum = minimum_host_thickness_for_dependent_cuts(definition_spec);
            OperationScalarRange::new(minimum.max(0.001), f32::MAX)
        }
        ("rounded_box", ["radius"]) => {
            let GeometrySource::RoundedBox {
                half_extents,
                radius,
            } = definition_spec.geometry.source
            else {
                return None;
            };
            let maximum = rounded_box_radius_max_for_dependent_cuts(definition_spec)
                .min(half_extents[0].min(half_extents[1]).min(half_extents[2]))
                .max(radius);
            OperationScalarRange::new(0.0, maximum)
        }
        ("rounded_box", ["half_extents", component]) => {
            let GeometrySource::RoundedBox { half_extents, .. } = definition_spec.geometry.source
            else {
                return None;
            };
            let axis = axis_index(component)?;
            let minimum = rounded_box_half_extent_min_for_dependent_cuts(definition_spec, axis);
            OperationScalarRange::new(minimum.max(0.001), f32::MAX.max(half_extents[axis]))
        }
        _ => None,
    }
}

#[derive(Debug, Copy, Clone)]
struct CutHostBounds {
    half_size: [f32; 2],
    thickness: f32,
}

#[derive(Debug, Copy, Clone)]
struct CutLoopBevelWidths {
    entry: f32,
    secondary: f32,
}

impl CutLoopBevelWidths {
    fn for_loops(
        operations: &[ModelingOperationSpec],
        entry_loop: BoundaryLoopId,
        secondary_loop: BoundaryLoopId,
    ) -> Self {
        Self {
            entry: boundary_loop_bevel_width(operations, entry_loop),
            secondary: boundary_loop_bevel_width(operations, secondary_loop),
        }
    }

    fn maximum(self) -> f32 {
        self.entry.max(self.secondary)
    }

    fn combined(self) -> f32 {
        self.entry + self.secondary
    }
}

#[derive(Debug, Copy, Clone)]
struct RecessedCutScalars {
    center: [f32; 2],
    size: [f32; 2],
    depth: f32,
    corner_radius: f32,
    rim_width: f32,
}

fn recessed_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    scalars: RecessedCutScalars,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = rect_cut_ranges(host, scalars.center, scalars.size, scalars.rim_width);
    match field {
        "recessed_panel_cut.center.x" => Some(ranges.center_x),
        "recessed_panel_cut.center.y" => Some(ranges.center_y),
        "recessed_panel_cut.size.x" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_x_max.max(scalars.size[0].abs()),
        ),
        "recessed_panel_cut.size.y" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_y_max.max(scalars.size[1].abs()),
        ),
        "recessed_panel_cut.depth" => OperationScalarRange::new(
            (bevels.combined() + CUT_SCALAR_SAFETY_MARGIN).max(0.005),
            host.map_or(f32::MAX, |host| host.thickness * 0.95)
                .max(scalars.depth),
        ),
        "recessed_panel_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(scalars.rim_width),
        ),
        "recessed_panel_cut.corner_radius" => OperationScalarRange::new(
            recessed_corner_radius_min(scalars.corner_radius, bevels.secondary),
            (scalars.size[0].min(scalars.size[1]) * 0.5).max(scalars.corner_radius),
        ),
        "recessed_panel_cut.corner_segments" => OperationScalarRange::new(1.0, 128.0),
        _ => None,
    }
}

fn rectangular_through_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    rim_width: f32,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = rect_cut_ranges(host, center, size, rim_width);
    match field {
        "rectangular_through_cut.center.x" => Some(ranges.center_x),
        "rectangular_through_cut.center.y" => Some(ranges.center_y),
        "rectangular_through_cut.size.x" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_x_max.max(size[0].abs()),
        ),
        "rectangular_through_cut.size.y" => OperationScalarRange::new(
            rect_size_min(bevels.maximum()),
            ranges.size_y_max.max(size[1].abs()),
        ),
        "rectangular_through_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(rim_width),
        ),
        "rectangular_through_cut.corner_radius" => {
            OperationScalarRange::new(0.0, (size[0].min(size[1]) * 0.5).max(corner_radius))
        }
        "rectangular_through_cut.corner_segments" => OperationScalarRange::new(1.0, 128.0),
        _ => None,
    }
}

fn circular_through_cut_scalar_range(
    field: &str,
    host: Option<CutHostBounds>,
    center: [f32; 2],
    radius: f32,
    rim_width: f32,
    bevels: CutLoopBevelWidths,
) -> Option<OperationScalarRange> {
    let ranges = circular_cut_ranges(host, center, radius, rim_width);
    match field {
        "circular_through_cut.center.x" => Some(ranges.center_x),
        "circular_through_cut.center.y" => Some(ranges.center_y),
        "circular_through_cut.radius" => OperationScalarRange::new(
            (bevels.maximum() * 2.0 + CUT_SCALAR_SAFETY_MARGIN).max(0.01),
            ranges.radius_max.max(radius),
        ),
        "circular_through_cut.rim_width" => OperationScalarRange::new(
            (bevels.maximum() + CUT_SCALAR_SAFETY_MARGIN).max(0.001),
            ranges.rim_width_max.max(rim_width),
        ),
        "circular_through_cut.radial_segments" => OperationScalarRange::new(6.0, 128.0),
        _ => None,
    }
}

fn operation_cut_face(operation: &ModelingOperationSpec) -> Option<PlanarCutFace> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut { face, .. }
        | ModelingOperationSpec::RectangularThroughCut { face, .. }
        | ModelingOperationSpec::CircularThroughCut { face, .. } => Some(*face),
        _ => None,
    }
}

fn cut_host_bounds_for_source(
    source: &GeometrySource,
    face: PlanarCutFace,
) -> Option<CutHostBounds> {
    match source {
        GeometrySource::Plate { size, thickness } => match face {
            PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => Some(CutHostBounds {
                half_size: [size[0].abs() * 0.5, size[1].abs() * 0.5],
                thickness: thickness.abs(),
            }),
            _ => None,
        },
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        } => {
            let usable = |axis: usize| (half_extents[axis].abs() - radius.max(0.0)).max(0.0);
            let (u_axis, v_axis, normal_axis) = match face {
                PlanarCutFace::PositiveX | PlanarCutFace::NegativeX => (2, 1, 0),
                PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => (0, 2, 1),
                PlanarCutFace::PositiveZ | PlanarCutFace::NegativeZ => (0, 1, 2),
            };
            Some(CutHostBounds {
                half_size: [usable(u_axis), usable(v_axis)],
                thickness: half_extents[normal_axis].abs() * 2.0,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Copy, Clone)]
struct RectCutRanges {
    center_x: OperationScalarRange,
    center_y: OperationScalarRange,
    size_x_max: f32,
    size_y_max: f32,
    rim_width_max: f32,
}

#[derive(Debug, Copy, Clone)]
struct CircularCutRanges {
    center_x: OperationScalarRange,
    center_y: OperationScalarRange,
    radius_max: f32,
    rim_width_max: f32,
}

fn rect_cut_ranges(
    host: Option<CutHostBounds>,
    center: [f32; 2],
    size: [f32; 2],
    rim_width: f32,
) -> RectCutRanges {
    let Some(host) = host else {
        return RectCutRanges {
            center_x: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            center_y: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            size_x_max: 4.0,
            size_y_max: 4.0,
            rim_width_max: 0.5,
        };
    };
    let half_cut = [size[0].abs() * 0.5, size[1].abs() * 0.5];
    let rim = rim_width.max(0.0);
    let clearance_x = (host.half_size[0] - center[0].abs() - rim).max(0.025);
    let clearance_y = (host.half_size[1] - center[1].abs() - rim).max(0.025);
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - half_cut[0]).max(0.001),
        (host.half_size[1] - center[1].abs() - half_cut[1]).max(0.001),
    ];
    RectCutRanges {
        center_x: ordered_scalar_range(
            -host.half_size[0] + half_cut[0] + rim,
            host.half_size[0] - half_cut[0] - rim,
            center[0],
        ),
        center_y: ordered_scalar_range(
            -host.half_size[1] + half_cut[1] + rim,
            host.half_size[1] - half_cut[1] - rim,
            center[1],
        ),
        size_x_max: (clearance_x * 2.0).max(size[0].abs()).max(0.05),
        size_y_max: (clearance_y * 2.0).max(size[1].abs()).max(0.05),
        rim_width_max: rim_clearance[0].min(rim_clearance[1]).clamp(0.001, 0.5),
    }
}

fn circular_cut_ranges(
    host: Option<CutHostBounds>,
    center: [f32; 2],
    radius: f32,
    rim_width: f32,
) -> CircularCutRanges {
    let Some(host) = host else {
        return CircularCutRanges {
            center_x: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            center_y: OperationScalarRange {
                minimum: -2.0,
                maximum: 2.0,
            },
            radius_max: 2.0,
            rim_width_max: 0.5,
        };
    };
    let cut_radius = radius.abs();
    let rim = rim_width.max(0.0);
    let radius_clearance = [
        (host.half_size[0] - center[0].abs() - rim).max(0.01),
        (host.half_size[1] - center[1].abs() - rim).max(0.01),
    ];
    let rim_clearance = [
        (host.half_size[0] - center[0].abs() - cut_radius).max(0.001),
        (host.half_size[1] - center[1].abs() - cut_radius).max(0.001),
    ];
    CircularCutRanges {
        center_x: ordered_scalar_range(
            -host.half_size[0] + cut_radius + rim,
            host.half_size[0] - cut_radius - rim,
            center[0],
        ),
        center_y: ordered_scalar_range(
            -host.half_size[1] + cut_radius + rim,
            host.half_size[1] - cut_radius - rim,
            center[1],
        ),
        radius_max: radius_clearance[0]
            .min(radius_clearance[1])
            .max(cut_radius)
            .max(0.01),
        rim_width_max: rim_clearance[0].min(rim_clearance[1]).clamp(0.001, 0.5),
    }
}

fn ordered_scalar_range(minimum: f32, maximum: f32, current: f32) -> OperationScalarRange {
    if minimum <= maximum {
        OperationScalarRange { minimum, maximum }
    } else {
        OperationScalarRange {
            minimum: current,
            maximum: current,
        }
    }
}

fn boundary_loop_bevel_width(operations: &[ModelingOperationSpec], target: BoundaryLoopId) -> f32 {
    operations
        .iter()
        .find_map(|operation| match operation {
            ModelingOperationSpec::BevelBoundaryLoop {
                target_loop, width, ..
            } if *target_loop == target => Some(*width),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn rect_size_min(attached_bevel: f32) -> f32 {
    (attached_bevel * 4.0 + CUT_SCALAR_SAFETY_MARGIN).max(0.05)
}

fn rect_loop_radius(size: [f32; 2]) -> f32 {
    size[0].abs().min(size[1].abs()) * 0.25
}

fn recessed_corner_radius_min(current: f32, floor_bevel: f32) -> f32 {
    if current > 0.0 || floor_bevel > 0.0 {
        (floor_bevel + CUT_SCALAR_SAFETY_MARGIN).max(0.0)
    } else {
        0.0
    }
}

fn minimum_host_thickness_for_dependent_cuts(definition: &PartDefinition) -> f32 {
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| match operation {
            ModelingOperationSpec::RecessedPanelCut { depth, .. } => {
                Some(*depth + CUT_SCALAR_SAFETY_MARGIN)
            }
            ModelingOperationSpec::RectangularThroughCut {
                entry_loop,
                exit_loop,
                ..
            }
            | ModelingOperationSpec::CircularThroughCut {
                entry_loop,
                exit_loop,
                ..
            } => {
                let bevels = CutLoopBevelWidths::for_loops(
                    &definition.geometry.operations,
                    *entry_loop,
                    *exit_loop,
                );
                Some(bevels.combined() + CUT_SCALAR_SAFETY_MARGIN)
            }
            _ => None,
        })
        .fold(0.001, f32::max)
}

fn rounded_box_radius_max_for_dependent_cuts(definition: &PartDefinition) -> f32 {
    let GeometrySource::RoundedBox { half_extents, .. } = definition.geometry.source else {
        return f32::MAX;
    };
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| rounded_box_radius_max_for_cut(operation, half_extents))
        .fold(f32::MAX, f32::min)
}

fn rounded_box_radius_max_for_cut(
    operation: &ModelingOperationSpec,
    half_extents: [f32; 3],
) -> Option<f32> {
    let face = operation_cut_face(operation)?;
    let (u_axis, v_axis, _normal_axis) = rounded_box_face_axes(face);
    let required = cut_required_half_size(operation)?;
    Some(
        (half_extents[u_axis].abs() - required[0])
            .min(half_extents[v_axis].abs() - required[1])
            .max(0.0),
    )
}

fn rounded_box_half_extent_min_for_dependent_cuts(definition: &PartDefinition, axis: usize) -> f32 {
    let GeometrySource::RoundedBox { radius, .. } = definition.geometry.source else {
        return 0.001;
    };
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| {
            let face = operation_cut_face(operation)?;
            let (u_axis, v_axis, normal_axis) = rounded_box_face_axes(face);
            let required = cut_required_half_size(operation)?;
            if axis == u_axis {
                Some(radius + required[0])
            } else if axis == v_axis {
                Some(radius + required[1])
            } else if axis == normal_axis {
                Some(normal_half_extent_required(operation))
            } else {
                None
            }
        })
        .fold(radius.max(0.001), f32::max)
}

fn rounded_box_face_axes(face: PlanarCutFace) -> (usize, usize, usize) {
    match face {
        PlanarCutFace::PositiveX | PlanarCutFace::NegativeX => (2, 1, 0),
        PlanarCutFace::PositiveY | PlanarCutFace::NegativeY => (0, 2, 1),
        PlanarCutFace::PositiveZ | PlanarCutFace::NegativeZ => (0, 1, 2),
    }
}

fn cut_required_half_size(operation: &ModelingOperationSpec) -> Option<[f32; 2]> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            center,
            size,
            rim_width,
            ..
        }
        | ModelingOperationSpec::RectangularThroughCut {
            center,
            size,
            rim_width,
            ..
        } => Some([
            center[0].abs() + size[0].abs() * 0.5 + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
            center[1].abs() + size[1].abs() * 0.5 + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
        ]),
        ModelingOperationSpec::CircularThroughCut {
            center,
            radius,
            rim_width,
            ..
        } => Some([
            center[0].abs() + radius.abs() + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
            center[1].abs() + radius.abs() + rim_width.max(0.0) + CUT_SCALAR_SAFETY_MARGIN,
        ]),
        _ => None,
    }
}

fn normal_half_extent_required(operation: &ModelingOperationSpec) -> f32 {
    match operation {
        ModelingOperationSpec::RecessedPanelCut { depth, .. } => {
            (*depth + CUT_SCALAR_SAFETY_MARGIN).max(0.001)
        }
        ModelingOperationSpec::RectangularThroughCut { .. }
        | ModelingOperationSpec::CircularThroughCut { .. } => CUT_SCALAR_SAFETY_MARGIN.max(0.001),
        _ => 0.001,
    }
}

fn axis_index(component: &str) -> Option<usize> {
    match component {
        "x" => Some(0),
        "y" => Some(1),
        "z" => Some(2),
        _ => None,
    }
}

#[derive(Deserialize)]
enum ModelingOperationSpecWire {
    TransformGeometry {
        operation: OperationId,
        transform: Transform3,
    },
    SetBevelProfile {
        operation: OperationId,
        radius: f32,
        segments: u32,
    },
    AddPanel {
        operation: OperationId,
        region: RegionId,
        inset: f32,
        depth: f32,
    },
    AddTrim {
        operation: OperationId,
        region: RegionId,
        width: f32,
        height: f32,
    },
    RecessedPanelCut(RecessedPanelCutWire),
    RectangularThroughCut(RectangularThroughCutWire),
    CircularThroughCut(CircularThroughCutWire),
    BevelBoundaryLoop {
        operation: OperationId,
        target_loop: BoundaryLoopId,
        width: f32,
        segments: u32,
        profile: f32,
        bevel_region: RegionId,
        outer_replacement_loop: BoundaryLoopId,
        inner_replacement_loop: BoundaryLoopId,
    },
    MirrorInstances {
        operation: OperationId,
        plane_normal: [f32; 3],
        plane_offset: f32,
    },
    LinearArray {
        operation: OperationId,
        count: u32,
        offset: [f32; 3],
    },
    RadialArray {
        operation: OperationId,
        count: u32,
        axis: [f32; 3],
        angle_degrees: f32,
    },
    ReservedBoolean {
        operation: OperationId,
        label: String,
    },
    ReservedDeformationProgram {
        operation: OperationId,
        label: String,
    },
}

#[derive(Deserialize)]
struct RecessedPanelCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    size: [f32; 2],
    depth: f32,
    corner_radius: f32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    corner_segments: Option<u32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    floor_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    floor_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}

#[derive(Deserialize)]
struct RectangularThroughCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    corner_segments: Option<u32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    exit_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}

#[derive(Deserialize)]
struct CircularThroughCutWire {
    operation: OperationId,
    region: RegionId,
    face: PlanarCutFace,
    center: [f32; 2],
    radius: f32,
    radial_segments: u32,
    #[serde(default)]
    rim_width: Option<f32>,
    #[serde(default)]
    boundary_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    entry_loop: Option<BoundaryLoopId>,
    #[serde(default)]
    exit_loop: Option<BoundaryLoopId>,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    edge_treatment: CutEdgeTreatment,
}

impl ModelingOperationSpecWire {
    fn into_operation(self, schema_version: u32) -> Result<ModelingOperationSpec, String> {
        Ok(match self {
            Self::TransformGeometry {
                operation,
                transform,
            } => ModelingOperationSpec::TransformGeometry {
                operation,
                transform,
            },
            Self::SetBevelProfile {
                operation,
                radius,
                segments,
            } => ModelingOperationSpec::SetBevelProfile {
                operation,
                radius,
                segments,
            },
            Self::AddPanel {
                operation,
                region,
                inset,
                depth,
            } => ModelingOperationSpec::AddPanel {
                operation,
                region,
                inset,
                depth,
            },
            Self::AddTrim {
                operation,
                region,
                width,
                height,
            } => ModelingOperationSpec::AddTrim {
                operation,
                region,
                width,
                height,
            },
            Self::RecessedPanelCut(wire) => ModelingOperationSpec::RecessedPanelCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                size: wire.size,
                depth: wire.depth,
                corner_radius: wire.corner_radius,
                rim_width: required_or_legacy_rect_rim_width(
                    wire.rim_width,
                    wire.size,
                    "RecessedPanelCut.rim_width",
                    schema_version,
                )?,
                corner_segments: required_or_legacy_corner_segments(
                    wire.corner_segments,
                    "RecessedPanelCut.corner_segments",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "RecessedPanelCut.entry_loop",
                    schema_version,
                )?,
                floor_loop: legacy_or_required_secondary_loop(
                    wire.floor_loop,
                    wire.boundary_loop,
                    "RecessedPanelCut.floor_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                floor_region: wire.floor_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::RectangularThroughCut(wire) => ModelingOperationSpec::RectangularThroughCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                size: wire.size,
                corner_radius: wire.corner_radius,
                rim_width: required_or_legacy_rect_rim_width(
                    wire.rim_width,
                    wire.size,
                    "RectangularThroughCut.rim_width",
                    schema_version,
                )?,
                corner_segments: required_or_legacy_corner_segments(
                    wire.corner_segments,
                    "RectangularThroughCut.corner_segments",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "RectangularThroughCut.entry_loop",
                    schema_version,
                )?,
                exit_loop: legacy_or_required_secondary_loop(
                    wire.exit_loop,
                    wire.boundary_loop,
                    "RectangularThroughCut.exit_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::CircularThroughCut(wire) => ModelingOperationSpec::CircularThroughCut {
                operation: wire.operation,
                region: wire.region,
                face: wire.face,
                center: wire.center,
                radius: wire.radius,
                radial_segments: wire.radial_segments,
                rim_width: required_or_legacy_circular_rim_width(
                    wire.rim_width,
                    wire.radius,
                    "CircularThroughCut.rim_width",
                    schema_version,
                )?,
                entry_loop: required_or_legacy_loop(
                    wire.entry_loop,
                    wire.boundary_loop,
                    "CircularThroughCut.entry_loop",
                    schema_version,
                )?,
                exit_loop: legacy_or_required_secondary_loop(
                    wire.exit_loop,
                    wire.boundary_loop,
                    "CircularThroughCut.exit_loop",
                    schema_version,
                )?,
                outer_region: wire.outer_region,
                rim_region: wire.rim_region,
                wall_region: wire.wall_region,
                edge_treatment: wire.edge_treatment,
            },
            Self::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
            } => ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
            },
            Self::MirrorInstances {
                operation,
                plane_normal,
                plane_offset,
            } => ModelingOperationSpec::MirrorInstances {
                operation,
                plane_normal,
                plane_offset,
            },
            Self::LinearArray {
                operation,
                count,
                offset,
            } => ModelingOperationSpec::LinearArray {
                operation,
                count,
                offset,
            },
            Self::RadialArray {
                operation,
                count,
                axis,
                angle_degrees,
            } => ModelingOperationSpec::RadialArray {
                operation,
                count,
                axis,
                angle_degrees,
            },
            Self::ReservedBoolean { operation, label } => {
                ModelingOperationSpec::ReservedBoolean { operation, label }
            }
            Self::ReservedDeformationProgram { operation, label } => {
                ModelingOperationSpec::ReservedDeformationProgram { operation, label }
            }
        })
    }
}

impl<'de> Deserialize<'de> for ModelingOperationSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ModelingOperationSpecWire::deserialize(deserializer)?
            .into_operation(ASSET_RECIPE_SCHEMA_VERSION)
            .map_err(de::Error::custom)
    }
}

fn required_current_field<T>(current: Option<T>, field: &'static str) -> Result<T, String> {
    current.ok_or_else(|| format!("{field} is missing"))
}

fn legacy_boundary_loop_schema(schema_version: u32) -> bool {
    matches!(schema_version, 1..=3)
}

fn legacy_rim_field_schema(schema_version: u32) -> bool {
    matches!(schema_version, 1..=4)
}

fn required_or_legacy_loop(
    current: Option<BoundaryLoopId>,
    legacy: Option<BoundaryLoopId>,
    field: &'static str,
    schema_version: u32,
) -> Result<BoundaryLoopId, String> {
    if legacy_boundary_loop_schema(schema_version) {
        current
            .or(legacy)
            .ok_or_else(|| format!("{field} is missing"))
    } else {
        required_current_field(current, field)
    }
}

fn legacy_or_required_secondary_loop(
    current: Option<BoundaryLoopId>,
    legacy: Option<BoundaryLoopId>,
    field: &'static str,
    schema_version: u32,
) -> Result<BoundaryLoopId, String> {
    if legacy_boundary_loop_schema(schema_version) {
        if let Some(current) = current {
            Ok(current)
        } else if legacy.is_some() {
            Ok(LEGACY_MISSING_BOUNDARY_LOOP)
        } else {
            Err(format!("{field} is missing"))
        }
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_rect_rim_width(
    current: Option<f32>,
    size: [f32; 2],
    field: &'static str,
    schema_version: u32,
) -> Result<f32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or_else(|| default_rect_cut_rim_width(size)))
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_circular_rim_width(
    current: Option<f32>,
    radius: f32,
    field: &'static str,
    schema_version: u32,
) -> Result<f32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or_else(|| default_circular_cut_rim_width(radius)))
    } else {
        required_current_field(current, field)
    }
}

fn required_or_legacy_corner_segments(
    current: Option<u32>,
    field: &'static str,
    schema_version: u32,
) -> Result<u32, String> {
    if legacy_rim_field_schema(schema_version) {
        Ok(current.unwrap_or(DEFAULT_RECT_CUT_CORNER_SEGMENTS))
    } else {
        required_current_field(current, field)
    }
}

fn default_rect_cut_rim_width(size: [f32; 2]) -> f32 {
    size[0].min(size[1]).max(0.0) * 0.16
}

fn default_circular_cut_rim_width(radius: f32) -> f32 {
    radius.max(0.0) * 2.0 * 0.16
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
    /// Rim or border around an opening or recess.
    Rim,
    /// Wall generated by a cut operation.
    CutWall,
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

/// A semantic endpoint used by authored relationship policies.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AssetPartSelector {
    /// One concrete source recipe instance.
    SpecificInstance {
        /// Selected instance.
        instance: PartInstanceId,
    },
    /// All generated occurrences produced by one modeling operation.
    GeneratedByOperation {
        /// Generator operation.
        operation: OperationId,
    },
    /// A source instance and every generated occurrence that names it as prototype.
    PrototypeAndGeneratedOccurrences {
        /// Source prototype instance.
        prototype: PartInstanceId,
    },
    /// Every occurrence whose part definition carries the tag.
    PartTag {
        /// Required definition tag.
        tag: String,
    },
    /// Every occurrence whose part definition carries the role tag.
    DefinitionRole {
        /// Required definition role.
        role: String,
    },
}

impl AssetPartSelector {
    /// Select one concrete source recipe instance.
    #[must_use]
    pub fn specific(instance: PartInstanceId) -> Self {
        Self::SpecificInstance { instance }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AssetPartSelectorWire {
    Selector(AssetPartSelector),
    LegacyInstance(PartInstanceId),
}

impl From<AssetPartSelectorWire> for AssetPartSelector {
    fn from(value: AssetPartSelectorWire) -> Self {
        match value {
            AssetPartSelectorWire::Selector(selector) => selector,
            AssetPartSelectorWire::LegacyInstance(instance) => {
                AssetPartSelector::specific(instance)
            }
        }
    }
}

/// How relationship selector results should be paired before validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipPairing {
    /// Apply the relationship to every resolved pair.
    #[default]
    AllPairs,
    /// Pair resolved selectors by deterministic occurrence order.
    ByOccurrenceIndex,
    /// Pair occurrences that share the same source prototype lineage.
    ByPrototypeLineage,
    /// Greedily pair each left occurrence with the nearest unpaired right occurrence.
    NearestOneToOne,
    /// Use explicitly authored concrete occurrence pairs.
    Explicit(Vec<(PartInstanceId, PartInstanceId)>),
}

impl RelationshipPairing {
    fn is_all_pairs(&self) -> bool {
        matches!(self, Self::AllPairs)
    }
}

/// Authored geometric relationship policy between semantic part selectors.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum AssetRelationshipPolicy {
    /// This pair is intentionally allowed to overlap.
    MayOverlap {
        /// First selector.
        first: AssetPartSelector,
        /// Second selector.
        second: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
        /// Human-facing reason for the relationship.
        reason: String,
    },
    /// This pair must not intersect.
    MustNotIntersect {
        /// First selector.
        first: AssetPartSelector,
        /// Second selector.
        second: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
    },
    /// This pair must touch or remain within attachment clearance.
    MustTouch {
        /// First selector.
        first: AssetPartSelector,
        /// Second selector.
        second: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
        /// Maximum accepted clearance in world units.
        max_clearance: f32,
    },
    /// The contained part must remain inside the container's authored bounds.
    MustContain {
        /// Containing selector.
        container: AssetPartSelector,
        /// Contained selector.
        contained: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
    },
    /// This pair must maintain at least the authored clearance.
    MinimumClearance {
        /// First selector.
        first: AssetPartSelector,
        /// Second selector.
        second: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
        /// Minimum clearance in world units.
        clearance: f32,
    },
    /// The child selector must remain attached to the parent selector through named sockets.
    SocketAttached {
        /// Parent selector.
        parent: AssetPartSelector,
        /// Child selector.
        child: AssetPartSelector,
        /// Pairing mode for selector results.
        #[serde(default, skip_serializing_if = "RelationshipPairing::is_all_pairs")]
        pairing: RelationshipPairing,
        /// Socket on the parent part.
        parent_socket: SocketId,
        /// Socket on the child part.
        child_socket: SocketId,
        /// Maximum accepted socket-origin distance in world units.
        max_origin_distance: f32,
        /// Maximum accepted axis angle in degrees.
        max_axis_angle_degrees: f32,
        /// Optional maximum accepted mesh clearance in world units.
        max_clearance: Option<f32>,
    },
}

impl AssetRelationshipPolicy {
    /// Create a relationship that allows intentional overlap.
    #[must_use]
    pub fn may_overlap(
        first: PartInstanceId,
        second: PartInstanceId,
        reason: impl Into<String>,
    ) -> Self {
        Self::MayOverlap {
            first: AssetPartSelector::specific(first),
            second: AssetPartSelector::specific(second),
            pairing: RelationshipPairing::AllPairs,
            reason: reason.into(),
        }
    }
}

#[derive(Deserialize)]
enum AssetRelationshipPolicyWire {
    MayOverlap {
        first: AssetPartSelectorWire,
        second: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
        reason: String,
    },
    MustNotIntersect {
        first: AssetPartSelectorWire,
        second: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
    },
    MustTouch {
        first: AssetPartSelectorWire,
        second: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
        max_clearance: f32,
    },
    MustContain {
        container: AssetPartSelectorWire,
        contained: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
    },
    MinimumClearance {
        first: AssetPartSelectorWire,
        second: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
        clearance: f32,
    },
    SocketAttached {
        parent: AssetPartSelectorWire,
        child: AssetPartSelectorWire,
        #[serde(default)]
        pairing: RelationshipPairing,
        parent_socket: Option<SocketId>,
        child_socket: Option<SocketId>,
        socket: Option<SocketId>,
        max_origin_distance: f32,
        max_axis_angle_degrees: f32,
        max_clearance: Option<f32>,
    },
}

impl AssetRelationshipPolicyWire {
    fn into_policy<E>(self) -> Result<AssetRelationshipPolicy, E>
    where
        E: de::Error,
    {
        Ok(match self {
            Self::MayOverlap {
                first,
                second,
                pairing,
                reason,
            } => AssetRelationshipPolicy::MayOverlap {
                first: first.into(),
                second: second.into(),
                pairing,
                reason,
            },
            Self::MustNotIntersect {
                first,
                second,
                pairing,
            } => AssetRelationshipPolicy::MustNotIntersect {
                first: first.into(),
                second: second.into(),
                pairing,
            },
            Self::MustTouch {
                first,
                second,
                pairing,
                max_clearance,
            } => AssetRelationshipPolicy::MustTouch {
                first: first.into(),
                second: second.into(),
                pairing,
                max_clearance,
            },
            Self::MustContain {
                container,
                contained,
                pairing,
            } => AssetRelationshipPolicy::MustContain {
                container: container.into(),
                contained: contained.into(),
                pairing,
            },
            Self::MinimumClearance {
                first,
                second,
                pairing,
                clearance,
            } => AssetRelationshipPolicy::MinimumClearance {
                first: first.into(),
                second: second.into(),
                pairing,
                clearance,
            },
            Self::SocketAttached {
                parent,
                child,
                pairing,
                parent_socket,
                child_socket,
                socket,
                max_origin_distance,
                max_axis_angle_degrees,
                max_clearance,
            } => {
                let parent_socket = parent_socket.or(socket).ok_or_else(|| {
                    E::custom("SocketAttached relationship is missing parent_socket")
                })?;
                let child_socket = child_socket.or(socket).ok_or_else(|| {
                    E::custom("SocketAttached relationship is missing child_socket")
                })?;
                AssetRelationshipPolicy::SocketAttached {
                    parent: parent.into(),
                    child: child.into(),
                    pairing,
                    parent_socket,
                    child_socket,
                    max_origin_distance,
                    max_axis_angle_degrees,
                    max_clearance,
                }
            }
        })
    }
}

impl<'de> Deserialize<'de> for AssetRelationshipPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AssetRelationshipPolicyWire::deserialize(deserializer)?.into_policy()
    }
}

/// Editable generator dimensions and segment counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeneratorDimensionEdit {
    /// Replace rounded-box half extents.
    RoundedBoxHalfExtents([f32; 3]),
    /// Replace rounded-box corner radius.
    RoundedBoxRadius(f32),
    /// Replace cylinder radius.
    CylinderRadius(f32),
    /// Replace cylinder height.
    CylinderHeight(f32),
    /// Replace cylinder radial segment count.
    CylinderRadialSegments(u32),
    /// Replace frustum bottom radius.
    FrustumBottomRadius(f32),
    /// Replace frustum top radius.
    FrustumTopRadius(f32),
    /// Replace frustum height.
    FrustumHeight(f32),
    /// Replace frustum radial segment count.
    FrustumRadialSegments(u32),
    /// Replace plate size.
    PlateSize([f32; 2]),
    /// Replace plate thickness.
    PlateThickness(f32),
    /// Replace lathe segment count.
    LatheSegments(u32),
}

impl GeneratorDimensionEdit {
    /// Return true when this edit can change generated topology.
    #[must_use]
    pub fn topology_changing(&self) -> bool {
        matches!(
            self,
            Self::CylinderRadialSegments(_)
                | Self::FrustumRadialSegments(_)
                | Self::LatheSegments(_)
        )
    }
}

/// Editable spacing fields on array operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArraySpacingEdit {
    /// Replace linear array offset.
    LinearOffset([f32; 3]),
    /// Replace radial array axis.
    RadialAxis([f32; 3]),
    /// Replace radial array total angle in degrees.
    RadialAngleDegrees(f32),
}

/// Plane used when mirroring an instance into a new instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorInstanceSpec {
    /// Mirror plane normal.
    pub plane_normal: [f32; 3],
    /// Signed offset from the asset origin along the normal.
    pub plane_offset: f32,
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
    /// Set one scalar field on a modeling operation without a parameter descriptor.
    SetOperationScalar {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to mutate.
        operation: OperationId,
        /// Operation-relative scalar field path, such as `circular_through_cut.radius`.
        field: String,
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
    /// Enable or disable an authored optional part instance.
    SetOptionalPartEnabled {
        /// Target optional instance.
        instance: PartInstanceId,
        /// New enabled state.
        enabled: bool,
    },
    /// Set a generator dimension or segment count.
    SetGeneratorDimension {
        /// Definition containing the generator.
        definition: PartDefinitionId,
        /// Dimension edit.
        dimension: GeneratorDimensionEdit,
    },
    /// Replace a definition's base geometry source.
    ReplaceGeometrySource {
        /// Definition containing the generator.
        definition: PartDefinitionId,
        /// Replacement source.
        source: GeometrySource,
    },
    /// Set radius and/or segment count on a bevel operation.
    SetBevelSettings {
        /// Definition containing the bevel operation.
        definition: PartDefinitionId,
        /// Bevel operation.
        operation: OperationId,
        /// Optional radius replacement.
        radius: Option<f32>,
        /// Optional segment-count replacement.
        segments: Option<u32>,
    },
    /// Replace one sweep profile point.
    SetSweepProfilePoint {
        /// Definition containing the sweep source.
        definition: PartDefinitionId,
        /// Profile point index.
        index: usize,
        /// Replacement profile point.
        point: [f32; 2],
    },
    /// Replace one sweep path frame.
    SetSweepPathFrame {
        /// Definition containing the sweep source.
        definition: PartDefinitionId,
        /// Path frame index.
        index: usize,
        /// Replacement frame.
        frame: Frame3,
    },
    /// Replace one lathe profile point.
    SetLatheProfilePoint {
        /// Definition containing the lathe source.
        definition: PartDefinitionId,
        /// Profile point index.
        index: usize,
        /// Replacement profile point.
        point: [f32; 2],
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
    /// Insert one modeling operation into a definition's ordered local history.
    InsertModelingOperation {
        /// Definition receiving the operation.
        definition: PartDefinitionId,
        /// Insertion index in the ordered operation list.
        index: usize,
        /// Operation payload to insert.
        operation: ModelingOperationSpec,
    },
    /// Remove one modeling operation from a definition's ordered local history.
    RemoveModelingOperation {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to remove.
        operation: OperationId,
        /// How to handle descriptors and authored hints that reference the operation.
        #[serde(default)]
        policy: OperationRemovalPolicy,
    },
    /// Duplicate an authored cut operation with fresh semantic IDs.
    DuplicateCutOperation {
        /// Definition containing the source cut.
        definition: PartDefinitionId,
        /// Source cut operation.
        source: OperationId,
        /// Stable ID for the duplicated operation.
        operation: OperationId,
        /// Entry boundary loop for the duplicate.
        entry_loop: BoundaryLoopId,
        /// Floor or exit boundary loop for the duplicate.
        secondary_loop: BoundaryLoopId,
        /// Rim region for the duplicate.
        rim_region: RegionId,
        /// Wall region for the duplicate.
        wall_region: RegionId,
        /// Floor region for duplicated recessed cuts.
        floor_region: Option<RegionId>,
        /// Offset applied to the duplicate cut center in face-local coordinates.
        center_offset: [f32; 2],
        /// How the duplicated operation joins authored semantic cut groups.
        #[serde(default)]
        group_membership: DuplicateCutGroupMembership,
        /// Dependent boundary-treatment operations to copy with explicit fresh IDs.
        #[serde(default)]
        dependent_bevels: Vec<DuplicateBoundaryBevelSpec>,
    },
    /// Move one modeling operation to a new ordered index.
    MoveModelingOperation {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to move.
        operation: OperationId,
        /// Destination index after removing the operation from its old position.
        new_index: usize,
    },
    /// Replace an instance with another definition from a compatible variant group.
    ReplaceInstanceDefinition {
        /// Instance to retarget.
        instance: PartInstanceId,
        /// Replacement definition.
        definition: PartDefinitionId,
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
    /// Set spacing fields on a deterministic array operation.
    SetArraySpacing {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Array operation.
        operation: OperationId,
        /// New spacing field.
        spacing: ArraySpacingEdit,
    },
    /// Duplicate one leaf instance under the same parent.
    DuplicateInstance {
        /// Source instance.
        source: PartInstanceId,
        /// Stable ID for the duplicated instance.
        instance: PartInstanceId,
        /// Optional replacement name.
        name: Option<String>,
        /// Optional replacement transform.
        transform: Option<Transform3>,
    },
    /// Mirror one leaf instance into a new instance.
    MirrorInstance {
        /// Source instance.
        source: PartInstanceId,
        /// Stable ID for the mirrored instance.
        instance: PartInstanceId,
        /// Mirror plane.
        plane: MirrorInstanceSpec,
        /// Optional replacement name.
        name: Option<String>,
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
    /// Change a part instance lock.
    SetInstanceLock {
        /// Target instance.
        instance: PartInstanceId,
        /// New lock state.
        locked: bool,
    },
    /// Change a subtree lock.
    SetSubtreeLock {
        /// Target subtree root.
        instance: PartInstanceId,
        /// New lock state.
        locked: bool,
    },
    /// Change a definition topology lock.
    SetTopologyLock {
        /// Target definition.
        definition: PartDefinitionId,
        /// New lock state.
        locked: bool,
    },
    /// Request a harmless child order change.
    ReorderChildInstances {
        /// Parent instance, or `None` for roots.
        parent: Option<PartInstanceId>,
        /// Requested child order.
        ordered_children: Vec<PartInstanceId>,
    },
}

/// Explicit policy for removing authored modeling operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OperationRemovalPolicy {
    /// Fail when parameters or variation metadata still reference the operation.
    #[default]
    RejectIfReferenced,
    /// Remove metadata owned by the operation before deleting it.
    CascadeOwnedMetadata,
    /// Remove the operation, owned metadata, and operations depending on its generated loops.
    CascadeDependentOperations,
}

/// Policy for semantic cut-group membership when duplicating a cut operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DuplicateCutGroupMembership {
    /// Add the duplicate to every semantic cut group containing the source operation.
    #[default]
    PreserveSource,
    /// Leave the duplicate outside any semantic cut group.
    Ungrouped,
    /// Add the duplicate to one explicit semantic cut group.
    AddTo(String),
}

/// Explicit remap for duplicating a boundary-loop bevel dependent on a copied cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateBoundaryBevelSpec {
    /// Source bevel operation to copy.
    pub source: OperationId,
    /// Fresh operation ID for the copied bevel.
    pub operation: OperationId,
    /// Fresh bevel-band region for the copied bevel.
    pub bevel_region: RegionId,
    /// Fresh outer replacement loop for the copied bevel.
    pub outer_replacement_loop: BoundaryLoopId,
    /// Fresh inner replacement loop for the copied bevel.
    pub inner_replacement_loop: BoundaryLoopId,
}

/// Coarse execution phase for ordered modeling operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperationPhase {
    /// Controls that configure the base source before local topology is generated.
    SourceConfiguration,
    /// Operations that create or remove local topology.
    LocalTopology,
    /// Operations that consume existing local boundaries or alter boundary treatment.
    BoundaryTreatment,
    /// Operations that move local generated geometry without assembly fan-out.
    LocalTransform,
    /// Operations that generate assembly occurrences such as arrays and mirrors.
    AssemblyGeneration,
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
    /// The scalar cannot be represented by the target field.
    #[error("invalid scalar for {path}: {value} ({reason})")]
    InvalidScalarValue {
        /// Target path.
        path: String,
        /// Supplied value.
        value: f32,
        /// Reason the value cannot be applied.
        reason: &'static str,
    },
    /// The edit attempted to mutate a locked parameter.
    #[error("parameter is locked {0:?}")]
    LockedParameter(ParameterId),
    /// The edit attempted to mutate a locked part instance.
    #[error("part instance is locked {0:?}")]
    LockedInstance(PartInstanceId),
    /// The edit attempted to mutate a part instance inside a locked subtree.
    #[error("part instance {instance:?} is inside locked subtree {root:?}")]
    LockedSubtree {
        /// Locked subtree root.
        root: PartInstanceId,
        /// Mutated instance.
        instance: PartInstanceId,
    },
    /// The edit attempted to change locked topology.
    #[error("topology is locked for definition {0:?}")]
    LockedTopology(PartDefinitionId),
    /// The replacement definition is outside the compatible variant group.
    #[error("incompatible replacement from {from:?} to {to:?}")]
    IncompatibleReplacement {
        /// Existing definition.
        from: PartDefinitionId,
        /// Requested replacement definition.
        to: PartDefinitionId,
    },
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
    validate_locks(recipe, &mut report);
    validate_constraints(recipe, &mut report);
    validate_relationships(recipe, &mut report);
    validate_variation_metadata(recipe, &mut report);
    validate_semantic_shells(recipe, &mut report);
    validate_next_ids(recipe, &mut report);

    report
}

/// Return editable parameters in deterministic order.
#[must_use]
pub fn enumerate_parameters(recipe: &AssetRecipe) -> Vec<ParameterDescriptor> {
    recipe
        .parameters
        .iter()
        .filter(|entry| {
            let (id, parameter) = *entry;
            parameter_is_reflectable(recipe, *id, parameter)
        })
        .map(|(_, parameter)| parameter.clone())
        .collect()
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

/// Allocate the next boundary loop ID.
pub fn allocate_boundary_loop_id(recipe: &mut AssetRecipe) -> BoundaryLoopId {
    let id = BoundaryLoopId(recipe.next_ids.boundary_loop);
    recipe.next_ids.boundary_loop = recipe.next_ids.boundary_loop.saturating_add(1);
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
    let mut seen_operation_ids = BTreeMap::new();
    let mut seen_boundary_loop_ids = BTreeMap::new();
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
        validate_semantic_cut_host_constraints(definition, report);
        let mut boundary_loop_state = BoundaryLoopValidationState::new(&mut seen_boundary_loop_ids);
        for operation in &definition.geometry.operations {
            let operation_id = operation.operation_id();
            if let Some(previous_definition) = seen_operation_ids.insert(operation_id, *id) {
                push_issue(
                    report,
                    Some(format!(
                        "definition.{}.operation.{}",
                        definition.id.0, operation_id.0
                    )),
                    "duplicate_operation_id",
                    format!(
                        "Operation ID is already used by definition {}.",
                        previous_definition.0
                    ),
                );
            }
            let mut local_declared_outputs = BTreeSet::new();
            for (output_index, boundary_loop) in operation
                .direct_boundary_loop_outputs()
                .into_iter()
                .enumerate()
            {
                if !local_declared_outputs.insert(boundary_loop) {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop.{}",
                            definition.id.0, operation_id.0, output_index
                        )),
                        "duplicate_direct_boundary_loop_output",
                        "Direct boundary loop outputs must be distinct.",
                    );
                }
            }
            for dependency in operation.boundary_loop_dependencies() {
                for (output_index, output) in dependency.outputs.iter().copied().enumerate() {
                    if !local_declared_outputs.insert(output) {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "ambiguous_boundary_loop_output_ownership",
                            "Boundary loop outputs must be owned by exactly one direct output or dependency output.",
                        );
                    }
                }
            }
            for boundary_loop in operation.all_declared_boundary_loop_outputs() {
                boundary_loop_state.validate_new(
                    report,
                    Some(format!(
                        "definition.{}.operation.{}.boundary_loop.{}",
                        definition.id.0, operation_id.0, boundary_loop.0
                    )),
                    boundary_loop,
                    *id,
                    operation_id,
                );
            }
            for dependency in operation.boundary_loop_dependencies() {
                if dependency.input == LEGACY_MISSING_BOUNDARY_LOOP {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input",
                            definition.id.0, operation_id.0
                        )),
                        "invalid_boundary_loop_id",
                        "Boundary loop dependency input must be non-zero.",
                    );
                } else if !boundary_loop_state
                    .definition_boundary_loop_ids
                    .contains_key(&dependency.input)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "unknown_boundary_loop_dependency",
                        "Boundary loop dependency input must be produced earlier in the same definition.",
                    );
                } else if !boundary_loop_state
                    .live_boundary_loop_ids
                    .contains_key(&dependency.input)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "consumed_boundary_loop_dependency",
                        "Boundary loop dependency input must still be live.",
                    );
                }
                if dependency.mode == BoundaryLoopDependencyMode::Consume
                    && let Some(previous_operation) = boundary_loop_state
                        .consumed_boundary_loop_ids
                        .insert(dependency.input, operation_id)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "duplicate_boundary_loop_consumption",
                        format!(
                            "Boundary loop is already consumed by operation {}.",
                            previous_operation.0
                        ),
                    );
                }
                if dependency.mode == BoundaryLoopDependencyMode::Consume {
                    boundary_loop_state
                        .live_boundary_loop_ids
                        .remove(&dependency.input);
                }
                let mut local_outputs = BTreeSet::new();
                for (output_index, output) in dependency.outputs.into_iter().enumerate() {
                    if output == dependency.input {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "boundary_loop_dependency_self_output",
                            "Replacement boundary loop output must differ from the dependency input.",
                        );
                    }
                    if !local_outputs.insert(output) {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "duplicate_boundary_loop_dependency_output",
                            "Boundary loop dependency outputs must be distinct.",
                        );
                    }
                }
            }
        }
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

struct BoundaryLoopValidationState<'a> {
    seen_boundary_loop_ids: &'a mut BTreeMap<BoundaryLoopId, (PartDefinitionId, OperationId)>,
    definition_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
    live_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
    consumed_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
}

impl<'a> BoundaryLoopValidationState<'a> {
    fn new(
        seen_boundary_loop_ids: &'a mut BTreeMap<BoundaryLoopId, (PartDefinitionId, OperationId)>,
    ) -> Self {
        Self {
            seen_boundary_loop_ids,
            definition_boundary_loop_ids: BTreeMap::new(),
            live_boundary_loop_ids: BTreeMap::new(),
            consumed_boundary_loop_ids: BTreeMap::new(),
        }
    }

    fn validate_new(
        &mut self,
        report: &mut AssetValidationReport,
        subject: Option<String>,
        boundary_loop: BoundaryLoopId,
        definition: PartDefinitionId,
        operation: OperationId,
    ) {
        if boundary_loop == LEGACY_MISSING_BOUNDARY_LOOP {
            push_issue(
                report,
                subject,
                "invalid_boundary_loop_id",
                "Generated boundary loop IDs must be non-zero.",
            );
            return;
        }
        if let Some((previous_definition, previous_operation)) = self
            .seen_boundary_loop_ids
            .insert(boundary_loop, (definition, operation))
        {
            push_issue(
                report,
                subject,
                "duplicate_boundary_loop_id",
                format!(
                    "Boundary loop ID is already used by definition {} operation {}.",
                    previous_definition.0, previous_operation.0
                ),
            );
            return;
        }
        self.definition_boundary_loop_ids
            .insert(boundary_loop, operation);
        self.live_boundary_loop_ids.insert(boundary_loop, operation);
    }
}

fn validate_instances(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    let mut seen_roots = BTreeSet::new();
    let mut previous_root = None;
    for root in &recipe.root_instances {
        if !seen_roots.insert(*root) {
            push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "duplicate_root_instance",
                "Root instances must not contain duplicates.",
            );
        }
        if let Some(previous_root) = previous_root
            && previous_root > *root
        {
            push_issue(
                report,
                Some("root_instances".to_owned()),
                "unstable_root_order",
                "Root instances must be ordered by semantic instance ID.",
            );
        }
        previous_root = Some(*root);
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
        if instance.parent.is_none() && !seen_roots.contains(id) {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "missing_root_instance",
                "Parentless instances must be listed as roots.",
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
    if attachment.parent_instance == child {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "self_attachment",
            "Instance cannot attach to itself.",
        );
    }
    if child_instance.parent != Some(attachment.parent_instance) {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "attachment_parent_mismatch",
            "Attachment parent must match the instance parent.",
        );
    }
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
        validate_finite(
            report,
            Some(format!("parameter.{}.minimum", id.0)),
            parameter.minimum,
        );
        validate_finite(
            report,
            Some(format!("parameter.{}.maximum", id.0)),
            parameter.maximum,
        );
        validate_positive(
            report,
            Some(format!("parameter.{}.step", id.0)),
            parameter.step,
        );
        validate_non_negative(
            report,
            Some(format!("parameter.{}.mutation_sigma", id.0)),
            parameter.mutation_sigma,
        );
        if parameter.minimum > parameter.maximum {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "invalid_parameter_range",
                "Parameter minimum cannot exceed maximum.",
            );
        }
        match get_scalar(recipe, &parameter.path) {
            Ok(value) => {
                if !value.is_finite() {
                    push_issue(
                        report,
                        Some(format!("parameter.{}", id.0)),
                        "non_finite_parameter_value",
                        "Parameter path resolves to a non-finite scalar.",
                    );
                } else if parameter_range_is_valid(parameter)
                    && (value < parameter.minimum || value > parameter.maximum)
                {
                    push_issue(
                        report,
                        Some(format!("parameter.{}", id.0)),
                        "parameter_value_out_of_range",
                        "Parameter value is outside its descriptor range.",
                    );
                }
            }
            Err(_) => {
                push_issue(
                    report,
                    Some(format!("parameter.{}", id.0)),
                    "unknown_parameter_path",
                    "Parameter path does not resolve to a scalar.",
                );
            }
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

fn parameter_is_reflectable(
    recipe: &AssetRecipe,
    id: ParameterId,
    parameter: &ParameterDescriptor,
) -> bool {
    if parameter.id != id || !parameter_range_is_valid(parameter) {
        return false;
    }
    if !parameters::is_beginner_safe_parameter_path(&parameter.path) {
        return false;
    }
    let Ok(value) = get_scalar(recipe, &parameter.path) else {
        return false;
    };
    value.is_finite() && value >= parameter.minimum && value <= parameter.maximum
}

fn parameter_range_is_valid(parameter: &ParameterDescriptor) -> bool {
    parameter.minimum.is_finite()
        && parameter.maximum.is_finite()
        && parameter.step.is_finite()
        && parameter.step > 0.0
        && parameter.mutation_sigma.is_finite()
        && parameter.mutation_sigma >= 0.0
        && parameter.minimum <= parameter.maximum
}

fn validate_locks(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for instance in &recipe.instance_locks {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("lock.instance.{}", instance.0)),
                "unknown_locked_instance",
                "Locked instance does not exist.",
            );
        }
    }
    for instance in &recipe.subtree_locks {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("lock.subtree.{}", instance.0)),
                "unknown_locked_subtree",
                "Locked subtree root does not exist.",
            );
        }
    }
    for definition in &recipe.topology_locks {
        if !recipe.definitions.contains_key(definition) {
            push_issue(
                report,
                Some(format!("lock.topology.{}", definition.0)),
                "unknown_locked_topology",
                "Locked topology definition does not exist.",
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

fn validate_relationships(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (index, relationship) in recipe.relationships.iter().enumerate() {
        match relationship {
            AssetRelationshipPolicy::MayOverlap {
                first,
                second,
                pairing,
                reason,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                if reason.trim().is_empty() {
                    push_issue(
                        report,
                        Some(format!("relationship.{index}.reason")),
                        "empty_relationship_reason",
                        "MayOverlap relationships must explain why overlap is intentional.",
                    );
                }
            }
            AssetRelationshipPolicy::MustNotIntersect {
                first,
                second,
                pairing,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
            }
            AssetRelationshipPolicy::MustTouch {
                first,
                second,
                pairing,
                max_clearance,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_clearance")),
                    *max_clearance,
                );
            }
            AssetRelationshipPolicy::MustContain {
                container,
                contained,
                pairing,
            } => {
                validate_relationship_pair(recipe, report, index, container, contained);
                validate_relationship_pairing(recipe, report, index, pairing);
            }
            AssetRelationshipPolicy::MinimumClearance {
                first,
                second,
                pairing,
                clearance,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.clearance")),
                    *clearance,
                );
            }
            AssetRelationshipPolicy::SocketAttached {
                parent,
                child,
                pairing,
                parent_socket,
                child_socket,
                max_origin_distance,
                max_axis_angle_degrees,
                max_clearance,
            } => {
                validate_relationship_pair(recipe, report, index, parent, child);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_origin_distance")),
                    *max_origin_distance,
                );
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_axis_angle_degrees")),
                    *max_axis_angle_degrees,
                );
                if let Some(clearance) = max_clearance {
                    validate_non_negative(
                        report,
                        Some(format!("relationship.{index}.max_clearance")),
                        *clearance,
                    );
                }
                validate_relationship_socket(
                    recipe,
                    report,
                    index,
                    parent,
                    child,
                    *parent_socket,
                    *child_socket,
                );
            }
        }
    }
}

fn validate_relationship_pair(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    first: &AssetPartSelector,
    second: &AssetPartSelector,
) {
    if first == second {
        push_issue(
            report,
            Some(format!("relationship.{index}")),
            "self_relationship",
            "Relationship endpoints must be different instances.",
        );
    }
    validate_part_selector(recipe, report, format!("relationship.{index}.first"), first);
    validate_part_selector(
        recipe,
        report,
        format!("relationship.{index}.second"),
        second,
    );
}

fn validate_relationship_pairing(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    pairing: &RelationshipPairing,
) {
    let RelationshipPairing::Explicit(pairs) = pairing else {
        return;
    };
    if pairs.is_empty() {
        push_issue(
            report,
            Some(format!("relationship.{index}.pairing")),
            "empty_relationship_pairing",
            "Explicit relationship pairing must include at least one pair.",
        );
    }
    for (pair_index, (first, second)) in pairs.iter().enumerate() {
        if first == second {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}")),
                "self_relationship_pairing",
                "Explicit relationship pairing endpoints must be different instances.",
            );
        }
        if !recipe.instances.contains_key(first) {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}.first")),
                "unknown_relationship_pairing_instance",
                "Explicit relationship pairing references an unknown first instance.",
            );
        }
        if !recipe.instances.contains_key(second) {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}.second")),
                "unknown_relationship_pairing_instance",
                "Explicit relationship pairing references an unknown second instance.",
            );
        }
    }
}

fn validate_relationship_socket(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    parent: &AssetPartSelector,
    child: &AssetPartSelector,
    parent_socket: SocketId,
    child_socket: SocketId,
) {
    let parent_definitions = selector_definitions(recipe, parent);
    let child_definitions = selector_definitions(recipe, child);
    if parent_definitions
        .iter()
        .any(|definition| !definition.sockets.contains_key(&parent_socket))
    {
        push_issue(
            report,
            Some(format!("relationship.{index}.parent_socket")),
            "unknown_relationship_parent_socket",
            "SocketAttached relationship references a missing parent socket.",
        );
    }
    if child_definitions
        .iter()
        .any(|definition| !definition.sockets.contains_key(&child_socket))
    {
        push_issue(
            report,
            Some(format!("relationship.{index}.child_socket")),
            "unknown_relationship_child_socket",
            "SocketAttached relationship references a missing child socket.",
        );
    }
}

fn validate_part_selector(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    selector: &AssetPartSelector,
) {
    match selector {
        AssetPartSelector::SpecificInstance { instance } => {
            if !recipe.instances.contains_key(instance) {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_instance",
                    "Relationship selector references an unknown instance.",
                );
            }
        }
        AssetPartSelector::GeneratedByOperation { operation } => {
            let exists = recipe
                .definitions
                .values()
                .flat_map(|definition| &definition.geometry.operations)
                .any(|candidate| candidate.operation_id() == *operation);
            if !exists {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_operation",
                    "Relationship selector references an unknown generator operation.",
                );
            }
        }
        AssetPartSelector::PrototypeAndGeneratedOccurrences { prototype } => {
            if !recipe.instances.contains_key(prototype) {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_instance",
                    "Relationship selector references an unknown prototype instance.",
                );
            }
        }
        AssetPartSelector::PartTag { tag } => {
            validate_selector_tag(recipe, report, subject, tag, "part tag");
        }
        AssetPartSelector::DefinitionRole { role } => {
            validate_selector_tag(recipe, report, subject, role, "definition role");
        }
    }
}

fn validate_selector_tag(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    tag: &str,
    label: &'static str,
) {
    if tag.trim().is_empty() {
        push_issue(
            report,
            Some(subject),
            "empty_relationship_selector",
            format!("Relationship selector {label} cannot be empty."),
        );
        return;
    }
    if !recipe
        .definitions
        .values()
        .any(|definition| definition.tags.contains(tag))
    {
        push_issue(
            report,
            Some(subject),
            "unknown_relationship_selector",
            format!("Relationship selector references an unknown {label}."),
        );
    }
}

fn selector_definitions<'a>(
    recipe: &'a AssetRecipe,
    selector: &AssetPartSelector,
) -> Vec<&'a PartDefinition> {
    match selector {
        AssetPartSelector::SpecificInstance { instance }
        | AssetPartSelector::PrototypeAndGeneratedOccurrences {
            prototype: instance,
        } => recipe
            .instances
            .get(instance)
            .and_then(|instance| recipe.definitions.get(&instance.definition))
            .into_iter()
            .collect(),
        AssetPartSelector::GeneratedByOperation { operation } => recipe
            .definitions
            .values()
            .filter(|definition| {
                definition
                    .geometry
                    .operations
                    .iter()
                    .any(|candidate| candidate.operation_id() == *operation)
            })
            .collect(),
        AssetPartSelector::PartTag { tag } | AssetPartSelector::DefinitionRole { role: tag } => {
            recipe
                .definitions
                .values()
                .filter(|definition| definition.tags.contains(tag))
                .collect()
        }
    }
}

fn validate_variation_metadata(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for instance in &recipe.variation.optional_instances {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("variation.optional_instance.{}", instance.0)),
                "unknown_optional_instance",
                "Optional instance hint references an unknown instance.",
            );
        }
    }

    for (group, hint) in &recipe.variation.replacement_groups {
        if group.trim().is_empty() {
            push_issue(
                report,
                Some("variation.replacement_group".to_owned()),
                "empty_replacement_group",
                "Replacement group names cannot be empty.",
            );
        }
        if hint.definitions.is_empty() {
            push_issue(
                report,
                Some(format!("variation.replacement_group.{group}")),
                "empty_replacement_group_definitions",
                "Replacement groups must reference at least one definition.",
            );
        }
        for definition in &hint.definitions {
            if !recipe.definitions.contains_key(definition) {
                push_issue(
                    report,
                    Some(format!(
                        "variation.replacement_group.{group}.definition.{}",
                        definition.0
                    )),
                    "unknown_replacement_definition",
                    "Replacement group references an unknown definition.",
                );
            }
        }
    }

    for (operation, range) in &recipe.variation.count_ranges {
        if range.minimum > range.maximum {
            push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "invalid_count_range",
                "Count range minimum cannot exceed maximum.",
            );
        }
        if range.minimum == 0 {
            push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "count_range_too_small",
                "Array count ranges must start at one or greater.",
            );
        }
        match operation_by_id(recipe, *operation) {
            Some(operation_spec) if operation_is_array(operation_spec) => {}
            Some(_) => push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "invalid_count_range_operation",
                "Count range hint must target an array operation.",
            ),
            None => push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "unknown_count_range_operation",
                "Count range hint references an unknown operation.",
            ),
        }
    }

    for (parameter, range) in &recipe.variation.parameter_range_overrides {
        if !recipe.parameters.contains_key(parameter) {
            push_issue(
                report,
                Some(format!("variation.parameter_range.{}", parameter.0)),
                "unknown_parameter_range_override",
                "Parameter range override references an unknown parameter.",
            );
        }
        validate_finite(
            report,
            Some(format!("variation.parameter_range.{}.minimum", parameter.0)),
            range.minimum,
        );
        validate_finite(
            report,
            Some(format!("variation.parameter_range.{}.maximum", parameter.0)),
            range.maximum,
        );
        if range.minimum > range.maximum {
            push_issue(
                report,
                Some(format!("variation.parameter_range.{}", parameter.0)),
                "invalid_parameter_range_override",
                "Parameter range override minimum cannot exceed maximum.",
            );
        }
        if let Some(step) = range.step {
            validate_positive(
                report,
                Some(format!("variation.parameter_range.{}.step", parameter.0)),
                step,
            );
        }
        if let Some(mutation_sigma) = range.mutation_sigma {
            validate_non_negative(
                report,
                Some(format!(
                    "variation.parameter_range.{}.mutation_sigma",
                    parameter.0
                )),
                mutation_sigma,
            );
        }
    }

    for (group, hint) in &recipe.variation.semantic_cut_groups {
        if group.trim().is_empty() {
            push_issue(
                report,
                Some("variation.semantic_cut_group".to_owned()),
                "empty_semantic_cut_group",
                "Semantic cut group IDs cannot be empty.",
            );
        }
        if hint.label.trim().is_empty() {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.label")),
                "empty_semantic_cut_group_label",
                "Semantic cut groups must have a non-empty label.",
            );
        }
        if hint.operations.is_empty() {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.operations")),
                "empty_semantic_cut_group_operations",
                "Semantic cut groups must reference at least one cut operation.",
            );
        }
        let Some(definition) = recipe.definitions.get(&hint.definition) else {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.definition")),
                "unknown_semantic_cut_group_definition",
                "Semantic cut group references an unknown definition.",
            );
            continue;
        };
        let mut seen_operations = BTreeSet::new();
        for operation in &hint.operations {
            if !seen_operations.insert(*operation) {
                push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "duplicate_semantic_cut_group_operation",
                    "Semantic cut groups cannot list the same operation more than once.",
                );
            }
            match definition
                .geometry
                .operations
                .iter()
                .find(|candidate| candidate.operation_id() == *operation)
            {
                Some(operation_spec) if operation_is_cut(operation_spec) => {
                    if !cut_group_role_accepts_operation(&hint.role, operation_spec) {
                        push_issue(
                            report,
                            Some(format!(
                                "variation.semantic_cut_group.{group}.operation.{}",
                                operation.0
                            )),
                            "semantic_cut_group_role_mismatch",
                            "Semantic cut group role must match each member cut family.",
                        );
                    }
                }
                Some(_) => push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "invalid_semantic_cut_group_operation",
                    "Semantic cut groups must reference cut operations.",
                ),
                None => push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "unknown_semantic_cut_group_operation",
                    "Semantic cut group references an unknown operation on its definition.",
                ),
            }
        }
        if let Some(range) = hint.count_range {
            if range.minimum > range.maximum {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "invalid_semantic_cut_group_count_range",
                    "Semantic cut group count range minimum cannot exceed maximum.",
                );
            }
            if range.minimum == 0 {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "semantic_cut_group_count_range_too_small",
                    "Semantic cut group count ranges must start at one or greater.",
                );
            }
            let count = hint.operations.len() as u32;
            if count < range.minimum || count > range.maximum {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "semantic_cut_group_count_out_of_range",
                    "Semantic cut group member count must fit the authored count range.",
                );
            }
        }
    }
}

fn validate_semantic_shells(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (id, relationship) in &recipe.semantic.relationships {
        validate_shell_id(
            report,
            format!("semantic.relationships.{}", id.0),
            id.0,
            relationship.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.relationships.{}.parent", id.0),
            relationship.parent,
            "unknown_semantic_relationship_parent",
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.relationships.{}.child", id.0),
            relationship.child,
            "unknown_semantic_relationship_child",
        );
        validate_optional_export_profile(
            recipe,
            report,
            format!("semantic.relationships.{}.export_profile", id.0),
            relationship.export_profile,
            "unknown_semantic_relationship_export_profile",
        );
        validate_relationship_contract_policy(report, *id, relationship);
    }
    validate_semantic_relationship_cycles(recipe, report);

    for (id, pattern) in &recipe.semantic.patterns {
        validate_shell_id(
            report,
            format!("semantic.patterns.{}", id.0),
            id.0,
            pattern.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.patterns.{}.source_instance", id.0),
            pattern.source_instance,
            "unknown_semantic_pattern_source",
        );
        if let Some(count) = pattern.count
            && !(1..=10_000).contains(&count)
        {
            push_issue(
                report,
                Some(format!("semantic.patterns.{}.count", id.0)),
                "invalid_semantic_pattern_count",
                "Pattern count must be between 1 and 10000.",
            );
        }
        validate_pattern_contract_policy(report, *id, pattern);
    }

    for (id, slot) in &recipe.semantic.surface_slots {
        validate_shell_id(
            report,
            format!("semantic.surface_slots.{}", id.0),
            id.0,
            slot.id.0,
        );
        validate_optional_definition(
            recipe,
            report,
            format!("semantic.surface_slots.{}.owner_definition", id.0),
            slot.owner_definition,
            "unknown_semantic_surface_owner",
        );
    }

    for (id, slot) in &recipe.semantic.material_slots {
        validate_shell_id(
            report,
            format!("semantic.material_slots.{}", id.0),
            id.0,
            slot.id.0,
        );
        if let Some(surface_slot) = slot.surface_slot
            && !recipe.semantic.surface_slots.contains_key(&surface_slot)
        {
            push_issue(
                report,
                Some(format!("semantic.material_slots.{}.surface_slot", id.0)),
                "unknown_semantic_material_surface_slot",
                "Material slot references an unknown surface slot.",
            );
        }
    }

    for (id, body) in &recipe.semantic.collision_bodies {
        validate_shell_id(
            report,
            format!("semantic.collision_bodies.{}", id.0),
            id.0,
            body.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.collision_bodies.{}.target_instance", id.0),
            body.target_instance,
            "unknown_semantic_collision_target",
        );
    }

    for (id, channel) in &recipe.semantic.motion_channels {
        validate_shell_id(
            report,
            format!("semantic.motion_channels.{}", id.0),
            id.0,
            channel.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.motion_channels.{}.target_instance", id.0),
            channel.target_instance,
            "unknown_semantic_motion_target",
        );
    }

    for (id, patch) in &recipe.semantic.terrain_patches {
        validate_shell_id(
            report,
            format!("semantic.terrain_patches.{}", id.0),
            id.0,
            patch.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.terrain_patches.{}.root_instance", id.0),
            patch.root_instance,
            "unknown_semantic_terrain_root",
        );
    }

    for (id, profile) in &recipe.semantic.export_profiles {
        validate_shell_id(
            report,
            format!("semantic.export_profiles.{}", id.0),
            id.0,
            profile.id.0,
        );
        validate_export_includes(
            report,
            Some(format!("semantic.export_profiles.{}.includes", id.0)),
            &profile.includes,
        );
    }

    for (id, op) in &recipe.semantic.authoring_ops {
        validate_shell_id(
            report,
            format!("semantic.authoring_ops.{}", id.0),
            id.0,
            op.id.0,
        );
        if let Some(parameter) = op.target_parameter
            && !recipe.parameters.contains_key(&parameter)
        {
            push_issue(
                report,
                Some(format!("semantic.authoring_ops.{}.target_parameter", id.0)),
                "unknown_semantic_authoring_parameter",
                "Authoring op references an unknown parameter.",
            );
        }
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.authoring_ops.{}.target_instance", id.0),
            op.target_instance,
            "unknown_semantic_authoring_instance",
        );
    }

    for (id, validation) in &recipe.semantic.validation_reports {
        validate_shell_id(
            report,
            format!("semantic.validation_reports.{}", id.0),
            id.0,
            validation.id.0,
        );
        validate_optional_export_profile(
            recipe,
            report,
            format!("semantic.validation_reports.{}.export_profile", id.0),
            validation.export_profile,
            "unknown_semantic_validation_export_profile",
        );
    }

    validate_review_state(report, &recipe.semantic.review_state);
    validate_export_includes(
        report,
        Some("semantic.export_includes".to_owned()),
        &recipe.semantic.export_includes,
    );
}

fn validate_relationship_contract_policy(
    report: &mut AssetValidationReport,
    id: RelationshipId,
    relationship: &RelationshipContract,
) {
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.parent_node_ref", id.0)),
        relationship.parent_node_ref.as_deref(),
        "invalid_semantic_relationship_parent_node_ref",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.child_node_ref", id.0)),
        relationship.child_node_ref.as_deref(),
        "invalid_semantic_relationship_child_node_ref",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.parent_anchor_id", id.0)),
        relationship.parent_anchor_id.as_deref(),
        "invalid_semantic_relationship_parent_anchor",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.child_anchor_id", id.0)),
        relationship.child_anchor_id.as_deref(),
        "invalid_semantic_relationship_child_anchor",
    );

    match &relationship.placement_policy.position_rule {
        PositionRule::FixedOffsetFromEdge { edge, offset } => {
            if edge.trim().is_empty() {
                push_issue(
                    report,
                    Some(format!("semantic.relationships.{}.placement.edge", id.0)),
                    "empty_relationship_edge",
                    "Fixed edge placement must name an edge.",
                );
            }
            validate_finite_array(
                report,
                Some(format!("semantic.relationships.{}.placement.offset", id.0)),
                offset,
            );
        }
        PositionRule::ProportionalUv { u, v } => {
            validate_range(
                report,
                Some(format!("semantic.relationships.{}.placement.u", id.0)),
                *u,
                0.0,
                1.0,
            );
            validate_range(
                report,
                Some(format!("semantic.relationships.{}.placement.v", id.0)),
                *v,
                0.0,
                1.0,
            );
        }
        PositionRule::CenteredInZone { zone } => {
            if zone.trim().is_empty() {
                push_issue(
                    report,
                    Some(format!("semantic.relationships.{}.placement.zone", id.0)),
                    "empty_relationship_zone",
                    "Centered placement must name a zone.",
                );
            }
        }
        PositionRule::PreserveCurrentOnDetach => {}
    }

    if let OrientationPolicy::AlignToSurfaceNormal { max_angle_degrees } =
        relationship.orientation_policy
    {
        validate_range(
            report,
            Some(format!(
                "semantic.relationships.{}.orientation.max_angle_degrees",
                id.0
            )),
            max_angle_degrees,
            0.0,
            180.0,
        );
    }

    if let ScalePolicy::ClampToRange { minimum, maximum } = relationship.scale_policy {
        validate_positive(
            report,
            Some(format!("semantic.relationships.{}.scale.minimum", id.0)),
            minimum,
        );
        validate_positive(
            report,
            Some(format!("semantic.relationships.{}.scale.maximum", id.0)),
            maximum,
        );
        if minimum > maximum {
            push_issue(
                report,
                Some(format!("semantic.relationships.{}.scale", id.0)),
                "invalid_relationship_scale_range",
                "Scale policy minimum must be less than or equal to maximum.",
            );
        }
    }

    match relationship.contact_policy {
        ContactPolicy::SurfaceContact { clearance }
        | ContactPolicy::IntentionalGap { clearance } => validate_non_negative(
            report,
            Some(format!("semantic.relationships.{}.contact.clearance", id.0)),
            clearance,
        ),
        ContactPolicy::NotChecked | ContactPolicy::IntentionalOverlap => {}
    }
}

fn validate_semantic_relationship_cycles(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    let mut graph: BTreeMap<PartInstanceId, Vec<PartInstanceId>> = BTreeMap::new();
    for relationship in recipe.semantic.relationships.values() {
        if let (Some(parent), Some(child)) = (relationship.parent, relationship.child) {
            graph.entry(parent).or_default().push(child);
        }
    }

    for (parent, children) in &graph {
        for child in children {
            let mut visited = BTreeSet::new();
            if *parent == *child || relationship_path_exists(&graph, *child, *parent, &mut visited)
            {
                push_issue(
                    report,
                    Some("semantic.relationships".to_owned()),
                    "semantic_relationship_cycle",
                    "Relationship contracts must not form cycles.",
                );
                return;
            }
        }
    }
}

fn relationship_path_exists(
    graph: &BTreeMap<PartInstanceId, Vec<PartInstanceId>>,
    current: PartInstanceId,
    target: PartInstanceId,
    visited: &mut BTreeSet<PartInstanceId>,
) -> bool {
    if !visited.insert(current) {
        return false;
    }
    graph.get(&current).is_some_and(|children| {
        children.iter().any(|child| {
            *child == target || relationship_path_exists(graph, *child, target, visited)
        })
    })
}

fn validate_pattern_contract_policy(
    report: &mut AssetValidationReport,
    id: PatternId,
    pattern: &PatternContract,
) {
    match pattern.count_policy {
        PatternCountPolicy::Unspecified => {}
        PatternCountPolicy::Exact(count) => validate_pattern_count(report, id, count),
        PatternCountPolicy::Range { minimum, maximum } => {
            validate_pattern_count(report, id, minimum);
            validate_pattern_count(report, id, maximum);
            if minimum > maximum {
                push_issue(
                    report,
                    Some(format!("semantic.patterns.{}.count_policy", id.0)),
                    "invalid_semantic_pattern_count_range",
                    "Pattern count range minimum must be less than or equal to maximum.",
                );
            }
        }
    }

    if let Some(density) = pattern.density_policy {
        match density {
            PatternDensityPolicy::Exact(value) => validate_non_negative(
                report,
                Some(format!("semantic.patterns.{}.density", id.0)),
                value,
            ),
            PatternDensityPolicy::Range { minimum, maximum } => {
                validate_non_negative(
                    report,
                    Some(format!("semantic.patterns.{}.density.minimum", id.0)),
                    minimum,
                );
                validate_non_negative(
                    report,
                    Some(format!("semantic.patterns.{}.density.maximum", id.0)),
                    maximum,
                );
                if minimum > maximum {
                    push_issue(
                        report,
                        Some(format!("semantic.patterns.{}.density", id.0)),
                        "invalid_semantic_pattern_density_range",
                        "Pattern density range minimum must be less than or equal to maximum.",
                    );
                }
            }
        }
    }

    if let Some(spacing) = pattern.spacing {
        validate_non_negative(
            report,
            Some(format!("semantic.patterns.{}.spacing", id.0)),
            spacing,
        );
    }
}

fn validate_pattern_count(report: &mut AssetValidationReport, id: PatternId, count: u32) {
    if !(1..=10_000).contains(&count) {
        push_issue(
            report,
            Some(format!("semantic.patterns.{}.count_policy", id.0)),
            "invalid_semantic_pattern_count",
            "Pattern count must be between 1 and 10000.",
        );
    }
}

fn validate_optional_semantic_identifier(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: Option<&str>,
    code: &'static str,
) {
    let Some(value) = value else {
        return;
    };
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        push_issue(
            report,
            subject,
            code,
            "Semantic relationship references must use stable lowercase identifiers.",
        );
    }
}

fn validate_shell_id(
    report: &mut AssetValidationReport,
    subject: String,
    map_id: u64,
    payload_id: u64,
) {
    if payload_id == 0 {
        push_issue(
            report,
            Some(subject.clone()),
            "zero_semantic_shell_id",
            "Semantic shell IDs must be non-zero.",
        );
    }
    if map_id != payload_id {
        push_issue(
            report,
            Some(subject),
            "semantic_shell_id_mismatch",
            "Semantic shell map key and payload ID differ.",
        );
    }
}

fn validate_optional_instance(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    instance: Option<PartInstanceId>,
    code: &'static str,
) {
    if let Some(instance) = instance
        && !recipe.instances.contains_key(&instance)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown instance.",
        );
    }
}

fn validate_optional_definition(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    definition: Option<PartDefinitionId>,
    code: &'static str,
) {
    if let Some(definition) = definition
        && !recipe.definitions.contains_key(&definition)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown definition.",
        );
    }
}

fn validate_optional_export_profile(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    export_profile: Option<ExportProfileId>,
    code: &'static str,
) {
    if let Some(export_profile) = export_profile
        && !recipe
            .semantic
            .export_profiles
            .contains_key(&export_profile)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown export profile.",
        );
    }
}

fn validate_review_state(report: &mut AssetValidationReport, review: &ReviewState) {
    if matches!(review.tier, ReviewTier::Reviewed | ReviewTier::Published) {
        push_issue(
            report,
            Some("semantic.review_state.tier".to_owned()),
            "unsupported_semantic_review_tier",
            "Phase A semantic shells cannot mark assets reviewed or published.",
        );
    }
    if !review.human_review_required {
        push_issue(
            report,
            Some("semantic.review_state.human_review_required".to_owned()),
            "semantic_human_review_required_false",
            "Phase A semantic shells must keep human review required.",
        );
    }
    if review.publish_allowed {
        push_issue(
            report,
            Some("semantic.review_state.publish_allowed".to_owned()),
            "semantic_publish_allowed",
            "Phase A semantic shells must not allow publishing.",
        );
    }
    if review.public_catalog_visible {
        push_issue(
            report,
            Some("semantic.review_state.public_catalog_visible".to_owned()),
            "semantic_public_catalog_visible",
            "Phase A semantic shells must not make assets public catalog visible.",
        );
    }
}

fn validate_export_includes(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    includes: &ExportIncludes,
) {
    for (enabled, suffix) in [
        (includes.includes_uvs, "includes_uvs"),
        (includes.includes_textures, "includes_textures"),
        (includes.includes_material_looks, "includes_material_looks"),
        (includes.includes_collision, "includes_collision"),
        (
            includes.includes_gameplay_metadata,
            "includes_gameplay_metadata",
        ),
        (includes.includes_rig, "includes_rig"),
        (includes.includes_skinning, "includes_skinning"),
        (includes.includes_animation, "includes_animation"),
        (
            includes.includes_terrain_collision,
            "includes_terrain_collision",
        ),
        (includes.includes_godot_scene, "includes_godot_scene"),
    ] {
        if enabled {
            push_issue(
                report,
                append_subject(subject.clone(), suffix),
                "unsupported_semantic_export_include",
                "Phase A semantic shells cannot claim unsupported export includes.",
            );
        }
    }
    if includes.game_ready {
        push_issue(
            report,
            append_subject(subject.clone(), "game_ready"),
            "semantic_game_ready_claim",
            "Phase A semantic shells must keep game_ready false.",
        );
    }
    if !includes.human_review_required {
        push_issue(
            report,
            append_subject(subject, "human_review_required"),
            "semantic_export_review_required_false",
            "Phase A semantic shells must keep export review required.",
        );
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
        "boundary_loop",
        recipe.next_ids.boundary_loop,
        max_boundary_loop_id(recipe),
    );
    validate_counter(
        report,
        "socket",
        recipe.next_ids.socket,
        max_socket_id(recipe),
    );
    validate_counter(
        report,
        "relationship",
        recipe.next_ids.relationship,
        recipe.semantic.relationships.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "pattern",
        recipe.next_ids.pattern,
        recipe.semantic.patterns.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "surface_slot",
        recipe.next_ids.surface_slot,
        recipe.semantic.surface_slots.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "material_slot",
        recipe.next_ids.material_slot,
        recipe.semantic.material_slots.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "collision_body",
        recipe.next_ids.collision_body,
        recipe.semantic.collision_bodies.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "motion_channel",
        recipe.next_ids.motion_channel,
        recipe.semantic.motion_channels.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "terrain_patch",
        recipe.next_ids.terrain_patch,
        recipe.semantic.terrain_patches.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "export_profile",
        recipe.next_ids.export_profile,
        recipe.semantic.export_profiles.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "authoring_op",
        recipe.next_ids.authoring_op,
        recipe.semantic.authoring_ops.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "validation_report",
        recipe.next_ids.validation_report,
        recipe
            .semantic
            .validation_reports
            .keys()
            .map(|id| id.0)
            .max(),
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
    let declared = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.regions.keys().map(|id| id.0));
    let generated = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::generated_region_ids)
        .map(|id| id.0);
    declared.chain(generated).max()
}

fn max_boundary_loop_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .map(|id| id.0)
        .max()
}

fn max_socket_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.sockets.keys().map(|id| id.0))
        .max()
}

fn operation_by_id(recipe: &AssetRecipe, operation: OperationId) -> Option<&ModelingOperationSpec> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .find(|candidate| candidate.operation_id() == operation)
}

fn operation_is_array(operation: &ModelingOperationSpec) -> bool {
    matches!(
        operation,
        ModelingOperationSpec::LinearArray { .. } | ModelingOperationSpec::RadialArray { .. }
    )
}

fn operation_is_cut(operation: &ModelingOperationSpec) -> bool {
    matches!(
        operation,
        ModelingOperationSpec::RecessedPanelCut { .. }
            | ModelingOperationSpec::RectangularThroughCut { .. }
            | ModelingOperationSpec::CircularThroughCut { .. }
    )
}

fn cut_group_role_accepts_operation(
    role: &CutGroupRole,
    operation: &ModelingOperationSpec,
) -> bool {
    match role {
        CutGroupRole::MountHoles => {
            matches!(operation, ModelingOperationSpec::CircularThroughCut { .. })
        }
        CutGroupRole::Vents => {
            matches!(
                operation,
                ModelingOperationSpec::RectangularThroughCut { .. }
            )
        }
        CutGroupRole::Recesses => {
            matches!(operation, ModelingOperationSpec::RecessedPanelCut { .. })
        }
        CutGroupRole::Custom(_) => true,
    }
}

fn validate_semantic_cut_host_constraints(
    definition: &PartDefinition,
    report: &mut AssetValidationReport,
) {
    let mut rounded_box_cut_faces = BTreeSet::new();
    for operation in &definition.geometry.operations {
        let Some(face) = operation_cut_face(operation) else {
            continue;
        };
        let operation_id = operation.operation_id();
        match &definition.geometry.source {
            GeometrySource::Plate { .. } => {
                if !matches!(face, PlanarCutFace::PositiveY | PlanarCutFace::NegativeY) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "cut.face"),
                        "unsupported_plate_cut_face",
                        "Plate semantic cuts currently target only local +/-Y planar faces.",
                    );
                }
            }
            GeometrySource::RoundedBox { .. } => {
                rounded_box_cut_faces.insert(face);
            }
            _ => {
                push_issue(
                    report,
                    operation_subject(definition.id, operation_id, "cut.host"),
                    "unsupported_semantic_cut_host",
                    "Semantic cuts currently target only Plate or RoundedBox geometry sources.",
                );
            }
        }
    }

    if rounded_box_cut_faces.len() > 1 {
        push_issue(
            report,
            definition_subject(definition.id, "rounded_box.semantic_cuts"),
            "unsupported_rounded_box_cut_face_set",
            "RoundedBox semantic cuts currently support one selected primary face per definition.",
        );
    }
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
            for point in profile {
                if !array_is_finite(point) {
                    push_issue(
                        report,
                        definition_subject(definition, "sweep.profile"),
                        "non_finite",
                        "Sweep profile points must be finite.",
                    );
                }
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
            for point in profile {
                if !array_is_finite(point) {
                    push_issue(
                        report,
                        definition_subject(definition, "lathe.profile"),
                        "non_finite",
                        "Lathe profile points must be finite.",
                    );
                }
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
            for face in faces {
                if face.len() < 3 {
                    push_issue(
                        report,
                        definition_subject(definition, "literal_mesh.faces"),
                        "invalid_literal_face",
                        "Literal mesh faces must contain at least three vertices.",
                    );
                }
                for index in face {
                    if (*index as usize) >= positions.len() {
                        push_issue(
                            report,
                            definition_subject(definition, "literal_mesh.faces"),
                            "literal_face_index_out_of_bounds",
                            "Literal mesh face indices must reference positions.",
                        );
                    }
                }
            }
        }
        GeometrySource::ReservedBooleanResult { .. } => {}
    }
}

fn validate_operations(definition: &PartDefinition, report: &mut AssetValidationReport) {
    let mut seen = BTreeSet::new();
    let mut previous_phase: Option<(OperationId, OperationPhase)> = None;
    for operation in &definition.geometry.operations {
        let operation_id = operation.operation_id();
        let phase = operation.phase();
        if let Some((previous_operation, previous_phase)) = previous_phase
            && previous_phase > phase
        {
            push_issue(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                "invalid_operation_phase_order",
                format!(
                    "Operation phase {:?} cannot follow operation {} phase {:?}.",
                    phase, previous_operation.0, previous_phase
                ),
            );
        }
        previous_phase = Some((operation_id, phase));
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
            ModelingOperationSpec::RecessedPanelCut {
                region,
                center,
                size,
                depth,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                floor_loop,
                outer_region,
                rim_region,
                wall_region,
                floor_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    *outer_region,
                    &[*rim_region, *wall_region, *floor_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    *entry_loop,
                    *floor_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    center,
                    report,
                );
                validate_cut_size(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    size,
                    report,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "recessed_panel_cut.depth"),
                    *depth,
                );
                validate_non_negative(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "recessed_panel_cut.corner_radius",
                    ),
                    *corner_radius,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "recessed_panel_cut.rim_width"),
                    *rim_width,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "recessed_panel_cut.corner_segments",
                    ),
                    *corner_segments,
                    1,
                );
                validate_rect_cut_corner_radius(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    size,
                    *corner_radius,
                    report,
                );
            }
            ModelingOperationSpec::RectangularThroughCut {
                region,
                center,
                size,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                exit_loop,
                outer_region,
                rim_region,
                wall_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    *outer_region,
                    &[*rim_region, *wall_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    *entry_loop,
                    *exit_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    center,
                    report,
                );
                validate_cut_size(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    size,
                    report,
                );
                validate_non_negative(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.corner_radius",
                    ),
                    *corner_radius,
                );
                validate_positive(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.rim_width",
                    ),
                    *rim_width,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.corner_segments",
                    ),
                    *corner_segments,
                    1,
                );
                validate_rect_cut_corner_radius(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    size,
                    *corner_radius,
                    report,
                );
            }
            ModelingOperationSpec::CircularThroughCut {
                region,
                center,
                radius,
                radial_segments,
                rim_width,
                entry_loop,
                exit_loop,
                outer_region,
                rim_region,
                wall_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    *outer_region,
                    &[*rim_region, *wall_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    *entry_loop,
                    *exit_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    center,
                    report,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "circular_through_cut.radius"),
                    *radius,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "circular_through_cut.radial_segments",
                    ),
                    *radial_segments,
                    6,
                );
                validate_positive(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "circular_through_cut.rim_width",
                    ),
                    *rim_width,
                );
            }
            ModelingOperationSpec::BevelBoundaryLoop {
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
                ..
            } => {
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.width"),
                    *width,
                );
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.segments"),
                    *segments,
                    1,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.profile"),
                    *profile,
                );
                validate_range(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.profile"),
                    *profile,
                    BOUNDARY_BEVEL_PROFILE_MIN,
                    BOUNDARY_BEVEL_PROFILE_MAX,
                );
                if *bevel_region == RegionId(0) {
                    push_issue(
                        report,
                        operation_subject(
                            definition.id,
                            operation_id,
                            "bevel_boundary_loop.bevel_region",
                        ),
                        "invalid_region_id",
                        "Generated bevel region IDs must be non-zero.",
                    );
                }
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "bevel_boundary_loop.replacement",
                    *outer_replacement_loop,
                    *inner_replacement_loop,
                    report,
                );
                if *target_loop == *outer_replacement_loop
                    || *target_loop == *inner_replacement_loop
                {
                    push_issue(
                        report,
                        operation_subject(
                            definition.id,
                            operation_id,
                            "bevel_boundary_loop.target_loop",
                        ),
                        "boundary_loop_dependency_self_output",
                        "Bevel replacement loops must differ from the consumed target loop.",
                    );
                }
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

fn validate_cut_center(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    center: &[f32; 2],
    report: &mut AssetValidationReport,
) {
    for (component, value) in ["x", "y"].into_iter().zip(center) {
        validate_finite(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.center.{component}"),
            ),
            *value,
        );
    }
}

fn validate_cut_size(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    size: &[f32; 2],
    report: &mut AssetValidationReport,
) {
    for (component, value) in ["x", "y"].into_iter().zip(size) {
        validate_positive(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.size.{component}"),
            ),
            *value,
        );
    }
}

fn validate_cut_generated_regions(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    outer_region: RegionId,
    generated_regions: &[RegionId],
    report: &mut AssetValidationReport,
) {
    let mut seen = BTreeSet::new();
    for region in generated_regions {
        if *region == outer_region {
            push_issue(
                report,
                operation_subject(
                    definition,
                    operation,
                    format!("{operation_kind}.generated_region"),
                ),
                "cut_region_collision",
                "Generated cut detail regions must not reuse the surviving outer host region.",
            );
        }
        if !seen.insert(*region) {
            push_issue(
                report,
                operation_subject(
                    definition,
                    operation,
                    format!("{operation_kind}.generated_region"),
                ),
                "duplicate_cut_generated_region",
                "Generated cut detail regions must be distinct.",
            );
        }
    }
}

fn validate_cut_loop_pair(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    first: BoundaryLoopId,
    second: BoundaryLoopId,
    report: &mut AssetValidationReport,
) {
    if first == second {
        push_issue(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.boundary_loop"),
            ),
            "duplicate_cut_boundary_loop",
            "Each physical cut boundary loop must have a distinct semantic ID.",
        );
    }
}

fn validate_rect_cut_corner_radius(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    size: &[f32; 2],
    corner_radius: f32,
    report: &mut AssetValidationReport,
) {
    if size
        .iter()
        .copied()
        .all(|value| value.is_finite() && value > 0.0)
        && corner_radius.is_finite()
        && corner_radius > size[0].min(size[1]) * 0.5
    {
        push_issue(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.corner_radius"),
            ),
            "cut_corner_radius_too_large",
            "Cut corner radius must not exceed half the smaller cut dimension.",
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
    validate_finite_array(
        report,
        append_subject(subject.clone(), "scale"),
        &transform.scale,
    );
    if transform
        .scale
        .iter()
        .copied()
        .any(|value| value.is_finite() && value == 0.0)
    {
        push_issue(
            report,
            append_subject(subject, "scale"),
            "zero_scale",
            "Transform scale axes must be non-zero.",
        );
    }
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

fn validate_range(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: f32,
    minimum: f32,
    maximum: f32,
) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && (value < minimum || value > maximum) {
        push_issue(
            report,
            subject,
            "value_out_of_range",
            format!("Value must be between {minimum:.3} and {maximum:.3}."),
        );
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
        (GeometrySource::Sweep { profile, .. }, "sweep", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            component_value(point, component, path)
        }
        (
            GeometrySource::Sweep { path: frames, .. },
            "sweep",
            ["path", index, frame_field, component],
        ) => {
            let index = parse_index(index, path)?;
            let frame = frames
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            get_frame_scalar(frame, frame_field, component, path)
        }
        (GeometrySource::Lathe { profile, .. }, "lathe", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            component_value(point, component, path)
        }
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
        (
            ModelingOperationSpec::RecessedPanelCut { size, .. },
            ["recessed_panel_cut", "size", component],
        ) => component_value(size, component, path),
        (
            ModelingOperationSpec::RecessedPanelCut { center, .. },
            ["recessed_panel_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::RecessedPanelCut { depth, .. },
            ["recessed_panel_cut", "depth"],
        ) => Ok(*depth),
        (
            ModelingOperationSpec::RecessedPanelCut { corner_radius, .. },
            ["recessed_panel_cut", "corner_radius"],
        ) => Ok(*corner_radius),
        (
            ModelingOperationSpec::RecessedPanelCut { rim_width, .. },
            ["recessed_panel_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::RecessedPanelCut {
                corner_segments, ..
            },
            ["recessed_panel_cut", "corner_segments"],
        ) => Ok(*corner_segments as f32),
        (
            ModelingOperationSpec::RectangularThroughCut { size, .. },
            ["rectangular_through_cut", "size", component],
        ) => component_value(size, component, path),
        (
            ModelingOperationSpec::RectangularThroughCut { center, .. },
            ["rectangular_through_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::RectangularThroughCut { corner_radius, .. },
            ["rectangular_through_cut", "corner_radius"],
        ) => Ok(*corner_radius),
        (
            ModelingOperationSpec::RectangularThroughCut { rim_width, .. },
            ["rectangular_through_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::RectangularThroughCut {
                corner_segments, ..
            },
            ["rectangular_through_cut", "corner_segments"],
        ) => Ok(*corner_segments as f32),
        (
            ModelingOperationSpec::CircularThroughCut { center, .. },
            ["circular_through_cut", "center", component],
        ) => component_value(center, component, path),
        (
            ModelingOperationSpec::CircularThroughCut { radius, .. },
            ["circular_through_cut", "radius"],
        ) => Ok(*radius),
        (
            ModelingOperationSpec::CircularThroughCut {
                radial_segments, ..
            },
            ["circular_through_cut", "radial_segments"],
        ) => Ok(*radial_segments as f32),
        (
            ModelingOperationSpec::CircularThroughCut { rim_width, .. },
            ["circular_through_cut", "rim_width"],
        ) => Ok(*rim_width),
        (
            ModelingOperationSpec::BevelBoundaryLoop { width, .. },
            ["bevel_boundary_loop", "width"],
        ) => Ok(*width),
        (
            ModelingOperationSpec::BevelBoundaryLoop { segments, .. },
            ["bevel_boundary_loop", "segments"],
        ) => Ok(*segments as f32),
        (
            ModelingOperationSpec::BevelBoundaryLoop { profile, .. },
            ["bevel_boundary_loop", "profile"],
        ) => Ok(*profile),
        (ModelingOperationSpec::LinearArray { count, .. }, ["linear_array", "count"]) => {
            Ok(*count as f32)
        }
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ["linear_array", "offset", component],
        ) => component_value(offset, component, path),
        (ModelingOperationSpec::RadialArray { count, .. }, ["radial_array", "count"]) => {
            Ok(*count as f32)
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ["radial_array", "axis", component]) => {
            component_value(axis, component, path)
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ["radial_array", "angle_degrees"],
        ) => Ok(*angle_degrees),
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

fn get_frame_scalar(
    frame: &Frame3,
    frame_field: &str,
    component: &str,
    path: &str,
) -> Result<f32, AssetError> {
    match frame_field {
        "origin" => component_value(&frame.origin, component, path),
        "x_axis" => component_value(&frame.x_axis, component, path),
        "y_axis" => component_value(&frame.y_axis, component, path),
        "z_axis" => component_value(&frame.z_axis, component, path),
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
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            "cylinder",
            ["radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
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
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            "frustum",
            ["radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (GeometrySource::Plate { size, .. }, "plate", ["size", component]) => {
            set_component_value(size, component, path, value)
        }
        (GeometrySource::Plate { thickness, .. }, "plate", ["thickness"]) => {
            *thickness = value;
            Ok(())
        }
        (GeometrySource::Sweep { profile, .. }, "sweep", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_component_value(point, component, path, value)
        }
        (
            GeometrySource::Sweep { path: frames, .. },
            "sweep",
            ["path", index, frame_field, component],
        ) => {
            let index = parse_index(index, path)?;
            let frame = frames
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_frame_scalar(frame, frame_field, component, path, value)
        }
        (GeometrySource::Lathe { profile, .. }, "lathe", ["profile", index, component]) => {
            let index = parse_index(index, path)?;
            let point = profile
                .get_mut(index)
                .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))?;
            set_component_value(point, component, path, value)
        }
        (GeometrySource::Lathe { segments, .. }, "lathe", ["segments"]) => {
            *segments = scalar_to_u32(path, value)?;
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
        (ModelingOperationSpec::SetBevelProfile { segments, .. }, ["bevel", "segments"]) => {
            *segments = scalar_to_u32(path, value)?;
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
        (
            ModelingOperationSpec::RecessedPanelCut { size, .. },
            ["recessed_panel_cut", "size", component],
        ) => set_component_value(size, component, path, value),
        (
            ModelingOperationSpec::RecessedPanelCut { center, .. },
            ["recessed_panel_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::RecessedPanelCut { depth, .. },
            ["recessed_panel_cut", "depth"],
        ) => {
            *depth = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut { corner_radius, .. },
            ["recessed_panel_cut", "corner_radius"],
        ) => {
            *corner_radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut { rim_width, .. },
            ["recessed_panel_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RecessedPanelCut {
                corner_segments, ..
            },
            ["recessed_panel_cut", "corner_segments"],
        ) => {
            *corner_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut { size, .. },
            ["rectangular_through_cut", "size", component],
        ) => set_component_value(size, component, path, value),
        (
            ModelingOperationSpec::RectangularThroughCut { center, .. },
            ["rectangular_through_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::RectangularThroughCut { corner_radius, .. },
            ["rectangular_through_cut", "corner_radius"],
        ) => {
            *corner_radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut { rim_width, .. },
            ["rectangular_through_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::RectangularThroughCut {
                corner_segments, ..
            },
            ["rectangular_through_cut", "corner_segments"],
        ) => {
            *corner_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut { center, .. },
            ["circular_through_cut", "center", component],
        ) => set_component_value(center, component, path, value),
        (
            ModelingOperationSpec::CircularThroughCut { radius, .. },
            ["circular_through_cut", "radius"],
        ) => {
            *radius = value;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut {
                radial_segments, ..
            },
            ["circular_through_cut", "radial_segments"],
        ) => {
            *radial_segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::CircularThroughCut { rim_width, .. },
            ["circular_through_cut", "rim_width"],
        ) => {
            *rim_width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { width, .. },
            ["bevel_boundary_loop", "width"],
        ) => {
            *width = value;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { segments, .. },
            ["bevel_boundary_loop", "segments"],
        ) => {
            *segments = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::BevelBoundaryLoop { profile, .. },
            ["bevel_boundary_loop", "profile"],
        ) => {
            *profile = value;
            Ok(())
        }
        (ModelingOperationSpec::LinearArray { count, .. }, ["linear_array", "count"]) => {
            *count = scalar_to_u32(path, value)?;
            Ok(())
        }
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ["linear_array", "offset", component],
        ) => set_component_value(offset, component, path, value),
        (ModelingOperationSpec::RadialArray { count, .. }, ["radial_array", "count"]) => {
            *count = scalar_to_u32(path, value)?;
            Ok(())
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ["radial_array", "axis", component]) => {
            set_component_value(axis, component, path, value)
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ["radial_array", "angle_degrees"],
        ) => {
            *angle_degrees = value;
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

fn set_frame_scalar(
    frame: &mut Frame3,
    frame_field: &str,
    component: &str,
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    match frame_field {
        "origin" => set_component_value(&mut frame.origin, component, path, value),
        "x_axis" => set_component_value(&mut frame.x_axis, component, path, value),
        "y_axis" => set_component_value(&mut frame.y_axis, component, path, value),
        "z_axis" => set_component_value(&mut frame.z_axis, component, path, value),
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
            if !parameter_range_is_valid(descriptor) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "invalid parameter descriptor {parameter:?}"
                )));
            }
            if *value < descriptor.minimum || *value > descriptor.maximum {
                return Err(AssetError::InvalidScalarValue {
                    path: descriptor.path.clone(),
                    value: *value,
                    reason: "value is outside the parameter range",
                });
            }
            if let Some(definition) = edits::definition_id_from_scalar_path(&descriptor.path)
                && descriptor.topology_changing
            {
                edits::ensure_topology_editable(recipe, definition)?;
            }
            if let Some(instance) = edits::instance_id_from_scalar_path(&descriptor.path) {
                edits::ensure_instance_editable(recipe, instance)?;
            }
            let path = descriptor.path.clone();
            set_scalar(recipe, path, *value)
        }
        AssetEdit::SetOperationScalar {
            definition,
            operation,
            field,
            value,
        } => set_modeling_operation_scalar(recipe, *definition, *operation, field, *value),
        AssetEdit::SetTransform {
            instance,
            transform,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.local_transform = transform.clone();
            Ok(())
        }
        AssetEdit::SetEnabled { instance, enabled } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.enabled = *enabled;
            Ok(())
        }
        AssetEdit::SetOptionalPartEnabled { instance, enabled } => {
            if !recipe.variation.optional_instances.contains(instance) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "instance {instance:?} is not optional"
                )));
            }
            edits::ensure_instance_editable(recipe, *instance)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.enabled = *enabled;
            Ok(())
        }
        AssetEdit::SetGeneratorDimension {
            definition,
            dimension,
        } => set_generator_dimension(recipe, *definition, dimension),
        AssetEdit::ReplaceGeometrySource { definition, source } => {
            edits::ensure_topology_editable(recipe, *definition)?;
            let target = recipe
                .definitions
                .get_mut(definition)
                .ok_or(AssetError::UnknownDefinition(*definition))?;
            target.geometry.source = source.clone();
            Ok(())
        }
        AssetEdit::SetBevelSettings {
            definition,
            operation,
            radius,
            segments,
        } => set_bevel_settings(recipe, *definition, *operation, *radius, *segments),
        AssetEdit::SetSweepProfilePoint {
            definition,
            index,
            point,
        } => set_sweep_profile_point(recipe, *definition, *index, *point),
        AssetEdit::SetSweepPathFrame {
            definition,
            index,
            frame,
        } => set_sweep_path_frame(recipe, *definition, *index, frame),
        AssetEdit::SetLatheProfilePoint {
            definition,
            index,
            point,
        } => set_lathe_profile_point(recipe, *definition, *index, *point),
        AssetEdit::AddInstance { instance } => {
            if recipe.instances.contains_key(&instance.id) {
                return Err(AssetError::UnsupportedEdit(format!(
                    "duplicate instance {:?}",
                    instance.id
                )));
            }
            if let Some(parent) = instance.parent {
                edits::ensure_instance_editable(recipe, parent)?;
            }
            recipe.instances.insert(instance.id, instance.clone());
            if instance.parent.is_none() {
                insert_root_instance(recipe, instance.id);
            }
            bump_next_ids_for_instance(recipe, instance);
            Ok(())
        }
        AssetEdit::RemoveInstance { instance } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let descendants = descendants_of(recipe, *instance)?;
            if !descendants.is_empty() {
                return Err(AssetError::UnsupportedEdit(format!(
                    "cannot remove {instance:?} while descendants exist"
                )));
            }
            recipe
                .instances
                .remove(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            recipe.root_instances.retain(|root| root != instance);
            recipe.variation.optional_instances.remove(instance);
            Ok(())
        }
        AssetEdit::ReplaceDefinition { definition } => {
            if let Some(existing) = recipe.definitions.get(&definition.id)
                && edits::definition_topology_signature(existing)
                    != edits::definition_topology_signature(definition)
            {
                edits::ensure_topology_editable(recipe, definition.id)?;
            }
            recipe.definitions.insert(definition.id, definition.clone());
            bump_next_ids_for_definition(recipe, definition);
            Ok(())
        }
        AssetEdit::InsertModelingOperation {
            definition,
            index,
            operation,
        } => insert_modeling_operation(recipe, *definition, *index, operation),
        AssetEdit::RemoveModelingOperation {
            definition,
            operation,
            policy,
        } => remove_modeling_operation(recipe, *definition, *operation, *policy),
        AssetEdit::DuplicateCutOperation {
            definition,
            source,
            operation,
            entry_loop,
            secondary_loop,
            rim_region,
            wall_region,
            floor_region,
            center_offset,
            group_membership,
            dependent_bevels,
        } => duplicate_cut_operation(
            recipe,
            *definition,
            *source,
            DuplicateCutSpec {
                operation: *operation,
                entry_loop: *entry_loop,
                secondary_loop: *secondary_loop,
                rim_region: *rim_region,
                wall_region: *wall_region,
                floor_region: *floor_region,
                center_offset: *center_offset,
                group_membership: group_membership.clone(),
                dependent_bevels: dependent_bevels.clone(),
            },
        ),
        AssetEdit::MoveModelingOperation {
            definition,
            operation,
            new_index,
        } => move_modeling_operation(recipe, *definition, *operation, *new_index),
        AssetEdit::ReplaceInstanceDefinition {
            instance,
            definition,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let current_definition = recipe
                .instances
                .get(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?
                .definition;
            edits::ensure_topology_editable(recipe, current_definition)?;
            edits::ensure_compatible_replacement(recipe, current_definition, *definition)?;
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.definition = *definition;
            Ok(())
        }
        AssetEdit::SetArrayCount {
            definition,
            operation,
            count,
        } => set_array_count(recipe, *definition, *operation, *count),
        AssetEdit::SetArraySpacing {
            definition,
            operation,
            spacing,
        } => set_array_spacing(recipe, *definition, *operation, spacing),
        AssetEdit::DuplicateInstance {
            source,
            instance,
            name,
            transform,
        } => duplicate_instance(
            recipe,
            *source,
            *instance,
            name.as_deref(),
            transform.as_ref(),
        ),
        AssetEdit::MirrorInstance {
            source,
            instance,
            plane,
            name,
        } => mirror_instance(recipe, *source, *instance, plane, name.as_deref()),
        AssetEdit::Attach {
            instance,
            attachment,
        } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            if recipe.instances.contains_key(&attachment.parent_instance) {
                edits::ensure_instance_editable(recipe, attachment.parent_instance)?;
            }
            let target = recipe
                .instances
                .get_mut(instance)
                .ok_or(AssetError::UnknownInstance(*instance))?;
            target.attachment = Some(attachment.clone());
            target.parent = Some(attachment.parent_instance);
            recipe.root_instances.retain(|root| root != instance);
            Ok(())
        }
        AssetEdit::Detach { instance } => {
            edits::ensure_instance_editable(recipe, *instance)?;
            let previous_parent = recipe
                .instances
                .get(instance)
                .and_then(|target| target.parent);
            if let Some(parent) = previous_parent {
                edits::ensure_instance_editable(recipe, parent)?;
            }
            {
                let target = recipe
                    .instances
                    .get_mut(instance)
                    .ok_or(AssetError::UnknownInstance(*instance))?;
                target.attachment = None;
                target.parent = None;
            }
            insert_root_instance(recipe, *instance);
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
        AssetEdit::SetInstanceLock { instance, locked } => {
            if !recipe.instances.contains_key(instance) {
                return Err(AssetError::UnknownInstance(*instance));
            }
            if *locked {
                recipe.instance_locks.insert(*instance);
            } else {
                recipe.instance_locks.remove(instance);
            }
            Ok(())
        }
        AssetEdit::SetSubtreeLock { instance, locked } => {
            if !recipe.instances.contains_key(instance) {
                return Err(AssetError::UnknownInstance(*instance));
            }
            if *locked {
                recipe.subtree_locks.insert(*instance);
            } else {
                recipe.subtree_locks.remove(instance);
            }
            Ok(())
        }
        AssetEdit::SetTopologyLock { definition, locked } => {
            if !recipe.definitions.contains_key(definition) {
                return Err(AssetError::UnknownDefinition(*definition));
            }
            if *locked {
                recipe.topology_locks.insert(*definition);
            } else {
                recipe.topology_locks.remove(definition);
            }
            Ok(())
        }
        AssetEdit::ReorderChildInstances {
            parent,
            ordered_children,
        } => reorder_child_instances(recipe, *parent, ordered_children),
    }
}

fn set_generator_dimension(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    dimension: &GeneratorDimensionEdit,
) -> Result<(), AssetError> {
    if dimension.topology_changing() {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    match (&mut definition_ref.geometry.source, dimension) {
        (
            GeometrySource::RoundedBox { half_extents, .. },
            GeneratorDimensionEdit::RoundedBoxHalfExtents(value),
        ) => {
            *half_extents = *value;
            Ok(())
        }
        (
            GeometrySource::RoundedBox { radius, .. },
            GeneratorDimensionEdit::RoundedBoxRadius(value),
        ) => {
            *radius = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder { radius, .. },
            GeneratorDimensionEdit::CylinderRadius(value),
        ) => {
            *radius = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder { height, .. },
            GeneratorDimensionEdit::CylinderHeight(value),
        ) => {
            *height = *value;
            Ok(())
        }
        (
            GeometrySource::Cylinder {
                radial_segments, ..
            },
            GeneratorDimensionEdit::CylinderRadialSegments(value),
        ) => {
            *radial_segments = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum { bottom_radius, .. },
            GeneratorDimensionEdit::FrustumBottomRadius(value),
        ) => {
            *bottom_radius = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum { top_radius, .. },
            GeneratorDimensionEdit::FrustumTopRadius(value),
        ) => {
            *top_radius = *value;
            Ok(())
        }
        (GeometrySource::Frustum { height, .. }, GeneratorDimensionEdit::FrustumHeight(value)) => {
            *height = *value;
            Ok(())
        }
        (
            GeometrySource::Frustum {
                radial_segments, ..
            },
            GeneratorDimensionEdit::FrustumRadialSegments(value),
        ) => {
            *radial_segments = *value;
            Ok(())
        }
        (GeometrySource::Plate { size, .. }, GeneratorDimensionEdit::PlateSize(value)) => {
            *size = *value;
            Ok(())
        }
        (
            GeometrySource::Plate { thickness, .. },
            GeneratorDimensionEdit::PlateThickness(value),
        ) => {
            *thickness = *value;
            Ok(())
        }
        (GeometrySource::Lathe { segments, .. }, GeneratorDimensionEdit::LatheSegments(value)) => {
            *segments = *value;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "dimension edit does not match definition {definition:?}"
        ))),
    }
}

fn set_bevel_settings(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    radius: Option<f32>,
    segments: Option<u32>,
) -> Result<(), AssetError> {
    if segments.is_some() {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_ref = definition_ref
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown bevel operation {operation:?}"))
        })?;
    let ModelingOperationSpec::SetBevelProfile {
        radius: target_radius,
        segments: target_segments,
        ..
    } = operation_ref
    else {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation {operation:?} is not a bevel"
        )));
    };
    if let Some(radius) = radius {
        *target_radius = radius;
    }
    if let Some(segments) = segments {
        *target_segments = segments;
    }
    Ok(())
}

fn set_sweep_profile_point(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    point: [f32; 2],
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Sweep { profile, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a sweep"
        )));
    };
    let target = profile.get_mut(index).ok_or_else(|| {
        AssetError::UnsupportedEdit(format!("unknown sweep profile index {index}"))
    })?;
    *target = point;
    Ok(())
}

fn set_sweep_path_frame(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    frame: &Frame3,
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Sweep { path, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a sweep"
        )));
    };
    let target = path
        .get_mut(index)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown sweep path index {index}")))?;
    *target = frame.clone();
    Ok(())
}

fn set_lathe_profile_point(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    point: [f32; 2],
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let GeometrySource::Lathe { profile, .. } = &mut definition_ref.geometry.source else {
        return Err(AssetError::UnsupportedEdit(format!(
            "definition {definition:?} is not a lathe"
        )));
    };
    let target = profile.get_mut(index).ok_or_else(|| {
        AssetError::UnsupportedEdit(format!("unknown lathe profile index {index}"))
    })?;
    *target = point;
    Ok(())
}

fn set_array_count(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    count: u32,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    if let Some(range) = recipe.variation.count_ranges.get(&operation)
        && (count < range.minimum || count > range.maximum)
    {
        return Err(AssetError::InvalidScalarValue {
            path: format!(
                "definition.{}.operation.{}.array.count",
                definition.0, operation.0
            ),
            value: count as f32,
            reason: "count is outside the authored array range",
        });
    }
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

fn set_modeling_operation_scalar(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    field: &str,
    value: f32,
) -> Result<(), AssetError> {
    let path = definition_scalar_path(definition, format!("operation.{}.{}", operation.0, field));
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar { path, value });
    }
    let definition_spec = recipe
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_spec = definition_spec
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| AssetError::UnknownScalarPath(path.clone()))?;
    let field_parts = field.split('.').collect::<Vec<_>>();
    get_operation_scalar(operation_spec, &field_parts, &path)?;
    let before_signature = edits::definition_topology_signature(definition_spec);
    let mut edited = recipe.clone();
    set_scalar_in_place(&mut edited, &path, value)?;
    let after_definition = edited
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if edits::definition_topology_signature(after_definition) != before_signature {
        edits::ensure_topology_editable(recipe, definition)?;
    }
    *recipe = edited;
    Ok(())
}

fn set_array_spacing(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    spacing: &ArraySpacingEdit,
) -> Result<(), AssetError> {
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let operation_ref = definition_ref
        .geometry
        .operations
        .iter_mut()
        .find(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| {
            AssetError::UnsupportedEdit(format!("unknown array operation {operation:?}"))
        })?;
    match (operation_ref, spacing) {
        (
            ModelingOperationSpec::LinearArray { offset, .. },
            ArraySpacingEdit::LinearOffset(value),
        ) => {
            *offset = *value;
            Ok(())
        }
        (ModelingOperationSpec::RadialArray { axis, .. }, ArraySpacingEdit::RadialAxis(value)) => {
            *axis = *value;
            Ok(())
        }
        (
            ModelingOperationSpec::RadialArray { angle_degrees, .. },
            ArraySpacingEdit::RadialAngleDegrees(value),
        ) => {
            *angle_degrees = *value;
            Ok(())
        }
        _ => Err(AssetError::UnsupportedEdit(format!(
            "spacing edit does not match array operation {operation:?}"
        ))),
    }
}

#[derive(Debug, Clone)]
struct DuplicateCutSpec {
    operation: OperationId,
    entry_loop: BoundaryLoopId,
    secondary_loop: BoundaryLoopId,
    rim_region: RegionId,
    wall_region: RegionId,
    floor_region: Option<RegionId>,
    center_offset: [f32; 2],
    group_membership: DuplicateCutGroupMembership,
    dependent_bevels: Vec<DuplicateBoundaryBevelSpec>,
}

fn insert_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    index: usize,
    operation: &ModelingOperationSpec,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    if operation_id_exists(recipe, operation.operation_id()) {
        return Err(AssetError::UnsupportedEdit(format!(
            "duplicate operation {:?}",
            operation.operation_id()
        )));
    }
    ensure_new_boundary_loops_available(recipe, operation.boundary_loop_ids())?;
    ensure_new_generated_regions_available(recipe, operation_detail_region_ids(operation))?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if index > definition_ref.geometry.operations.len() {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation insertion index {index} is out of bounds"
        )));
    }
    definition_ref
        .geometry
        .operations
        .insert(index, operation.clone());
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.remove(index);
    })?;
    bump_next_ids_for_operation(recipe, operation);
    Ok(())
}

fn remove_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    policy: OperationRemovalPolicy,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let dependents = dependent_operation_closure(recipe, definition, operation)?;
    let dependent_only = dependents
        .iter()
        .copied()
        .filter(|dependent| *dependent != operation)
        .collect::<Vec<_>>();
    let references = operation_metadata_references(recipe, definition, operation);
    match policy {
        OperationRemovalPolicy::RejectIfReferenced if !references.is_empty() => {
            return Err(AssetError::UnsupportedEdit(format!(
                "operation {:?} is still referenced by {}",
                operation,
                references.join(", ")
            )));
        }
        OperationRemovalPolicy::CascadeOwnedMetadata => {
            if !dependent_only.is_empty() {
                return Err(AssetError::UnsupportedEdit(format!(
                    "operation {:?} has dependent operation(s) {}",
                    operation,
                    operation_id_list(&dependent_only)
                )));
            }
            cascade_operation_metadata(recipe, definition, operation);
        }
        OperationRemovalPolicy::CascadeDependentOperations => {
            for dependent in &dependents {
                cascade_operation_metadata(recipe, definition, *dependent);
            }
        }
        OperationRemovalPolicy::RejectIfReferenced => {}
    }
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    definition_ref
        .geometry
        .operations
        .retain(|candidate| !dependents.contains(&candidate.operation_id()));
    Ok(())
}

fn duplicate_cut_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    source: OperationId,
    spec: DuplicateCutSpec,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let group_membership = spec.group_membership.clone();
    let (duplicate, dependent_duplicates) = {
        let definition_ref = recipe
            .definitions
            .get(&definition)
            .ok_or(AssetError::UnknownDefinition(definition))?;
        let source_operation = definition_ref
            .geometry
            .operations
            .iter()
            .find(|candidate| candidate.operation_id() == source)
            .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {source:?}")))?;
        let duplicate = remap_cut_operation(source_operation, spec.clone())?;
        let dependent_duplicates = remap_dependent_boundary_bevels(
            &definition_ref.geometry.operations,
            source_operation,
            &duplicate,
            &spec.dependent_bevels,
        )?;
        (duplicate, dependent_duplicates)
    };
    let mut operation_ids = vec![duplicate.operation_id()];
    operation_ids.extend(
        dependent_duplicates
            .iter()
            .map(ModelingOperationSpec::operation_id),
    );
    ensure_new_operation_ids_available(recipe, operation_ids)?;
    let mut boundary_loop_ids = duplicate.boundary_loop_ids();
    boundary_loop_ids.extend(
        dependent_duplicates
            .iter()
            .flat_map(ModelingOperationSpec::boundary_loop_ids),
    );
    ensure_new_boundary_loops_available(recipe, boundary_loop_ids)?;
    let mut generated_regions = operation_detail_region_ids(&duplicate);
    generated_regions.extend(
        dependent_duplicates
            .iter()
            .flat_map(operation_detail_region_ids),
    );
    ensure_new_generated_regions_available(recipe, generated_regions)?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|candidate| candidate.operation_id() == source)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {source:?}")))?
        + 1;
    definition_ref
        .geometry
        .operations
        .insert(index, duplicate.clone());
    let dependent_insert_index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|operation| operation.phase() > OperationPhase::BoundaryTreatment)
        .unwrap_or(definition_ref.geometry.operations.len());
    for (offset, dependent) in dependent_duplicates.iter().enumerate() {
        definition_ref
            .geometry
            .operations
            .insert(dependent_insert_index + offset, dependent.clone());
    }
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.retain(|operation| {
            operation.operation_id() != duplicate.operation_id()
                && !dependent_duplicates
                    .iter()
                    .any(|dependent| dependent.operation_id() == operation.operation_id())
        });
    })?;
    bump_next_ids_for_operation(recipe, &duplicate);
    for dependent in &dependent_duplicates {
        bump_next_ids_for_operation(recipe, dependent);
    }
    apply_duplicate_cut_group_membership(
        recipe,
        definition,
        source,
        duplicate.operation_id(),
        &group_membership,
    )?;
    Ok(())
}

fn apply_duplicate_cut_group_membership(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    source: OperationId,
    duplicate: OperationId,
    membership: &DuplicateCutGroupMembership,
) -> Result<(), AssetError> {
    match membership {
        DuplicateCutGroupMembership::PreserveSource => {
            for group in recipe.variation.semantic_cut_groups.values_mut() {
                if group.definition == definition
                    && group.operations.contains(&source)
                    && !group.operations.contains(&duplicate)
                {
                    group.operations.push(duplicate);
                }
            }
            Ok(())
        }
        DuplicateCutGroupMembership::Ungrouped => Ok(()),
        DuplicateCutGroupMembership::AddTo(group_id) => {
            let group = recipe
                .variation
                .semantic_cut_groups
                .get_mut(group_id)
                .ok_or_else(|| {
                    AssetError::UnsupportedEdit(format!("unknown semantic cut group {group_id}"))
                })?;
            if group.definition != definition {
                return Err(AssetError::UnsupportedEdit(format!(
                    "semantic cut group {group_id} belongs to definition {:?}",
                    group.definition
                )));
            }
            if !group.operations.contains(&duplicate) {
                group.operations.push(duplicate);
            }
            Ok(())
        }
    }
}

fn move_modeling_operation(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
    new_index: usize,
) -> Result<(), AssetError> {
    edits::ensure_topology_editable(recipe, definition)?;
    let definition_ref = recipe
        .definitions
        .get_mut(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    let old_index = definition_ref
        .geometry
        .operations
        .iter()
        .position(|candidate| candidate.operation_id() == operation)
        .ok_or_else(|| AssetError::UnsupportedEdit(format!("unknown operation {operation:?}")))?;
    if new_index >= definition_ref.geometry.operations.len() {
        return Err(AssetError::UnsupportedEdit(format!(
            "operation move index {new_index} is out of bounds"
        )));
    }
    let operation = definition_ref.geometry.operations.remove(old_index);
    definition_ref
        .geometry
        .operations
        .insert(new_index, operation.clone());
    ensure_operation_phase_order(&definition_ref.geometry.operations).inspect_err(|_| {
        definition_ref.geometry.operations.remove(new_index);
        definition_ref
            .geometry
            .operations
            .insert(old_index, operation);
    })?;
    Ok(())
}

fn ensure_operation_phase_order(operations: &[ModelingOperationSpec]) -> Result<(), AssetError> {
    for pair in operations.windows(2) {
        let previous = pair[0].phase();
        let next = pair[1].phase();
        if previous > next {
            return Err(AssetError::UnsupportedEdit(format!(
                "operation phase order violation: {:?} cannot appear before {:?}",
                pair[1].operation_id(),
                pair[0].operation_id()
            )));
        }
    }
    Ok(())
}

fn operation_metadata_references(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Vec<String> {
    let owned_parameters = operation_parameter_ids(recipe, definition, operation);
    let mut references = Vec::new();
    references.extend(
        owned_parameters
            .iter()
            .map(|parameter| format!("parameter.{}", parameter.0)),
    );
    references.extend(
        owned_parameters
            .iter()
            .filter(|parameter| recipe.locks.contains(parameter))
            .map(|parameter| format!("lock.{}", parameter.0)),
    );
    if recipe.variation.count_ranges.contains_key(&operation) {
        references.push(format!("variation.count_range.{}", operation.0));
    }
    if let Ok(dependents) = dependent_operation_closure(recipe, definition, operation) {
        references.extend(
            dependents
                .into_iter()
                .filter(|dependent| *dependent != operation)
                .map(|dependent| format!("operation.{}.dependency", dependent.0)),
        );
    }
    references.extend(
        recipe
            .variation
            .semantic_cut_groups
            .iter()
            .filter(|(_, group)| {
                group.definition == definition && group.operations.contains(&operation)
            })
            .map(|(group, _)| format!("variation.semantic_cut_group.{group}")),
    );
    references.extend(
        owned_parameters
            .iter()
            .filter(|parameter| {
                recipe
                    .variation
                    .parameter_range_overrides
                    .contains_key(parameter)
            })
            .map(|parameter| format!("variation.parameter_range.{}", parameter.0)),
    );
    references
}

fn dependent_operation_closure(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Result<BTreeSet<OperationId>, AssetError> {
    let definition_ref = recipe
        .definitions
        .get(&definition)
        .ok_or(AssetError::UnknownDefinition(definition))?;
    if !definition_ref
        .geometry
        .operations
        .iter()
        .any(|candidate| candidate.operation_id() == operation)
    {
        return Err(AssetError::UnsupportedEdit(format!(
            "unknown operation {operation:?}"
        )));
    }
    let mut removal = BTreeSet::from([operation]);
    loop {
        let produced_loops = definition_ref
            .geometry
            .operations
            .iter()
            .filter(|candidate| removal.contains(&candidate.operation_id()))
            .flat_map(ModelingOperationSpec::all_declared_boundary_loop_outputs)
            .collect::<BTreeSet<_>>();
        let before = removal.len();
        for candidate in &definition_ref.geometry.operations {
            if removal.contains(&candidate.operation_id()) {
                continue;
            }
            if candidate
                .boundary_loop_dependencies()
                .iter()
                .any(|dependency| produced_loops.contains(&dependency.input))
            {
                removal.insert(candidate.operation_id());
            }
        }
        if removal.len() == before {
            return Ok(removal);
        }
    }
}

fn operation_id_list(operations: &[OperationId]) -> String {
    operations
        .iter()
        .map(|operation| operation.0.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn cascade_operation_metadata(
    recipe: &mut AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) {
    let owned_parameters = operation_parameter_ids(recipe, definition, operation);
    for parameter in &owned_parameters {
        recipe.parameters.remove(parameter);
        recipe.locks.remove(parameter);
        recipe.variation.parameter_range_overrides.remove(parameter);
    }
    recipe.variation.count_ranges.remove(&operation);
    recipe.variation.semantic_cut_groups.retain(|_, group| {
        if group.definition != definition {
            return true;
        }
        group.operations.retain(|candidate| *candidate != operation);
        !group.operations.is_empty()
    });
}

fn operation_parameter_ids(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Vec<ParameterId> {
    let prefix = format!("definition.{}.operation.{}.", definition.0, operation.0);
    recipe
        .parameters
        .iter()
        .filter_map(|(id, descriptor)| descriptor.path.starts_with(&prefix).then_some(*id))
        .collect()
}

fn remap_cut_operation(
    operation: &ModelingOperationSpec,
    spec: DuplicateCutSpec,
) -> Result<ModelingOperationSpec, AssetError> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            region,
            face,
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            outer_region,
            edge_treatment,
            ..
        } => {
            let floor_region = spec.floor_region.ok_or_else(|| {
                AssetError::UnsupportedEdit(
                    "duplicated recessed cuts require a floor region".to_owned(),
                )
            })?;
            Ok(ModelingOperationSpec::RecessedPanelCut {
                operation: spec.operation,
                region: *region,
                face: *face,
                center: [
                    center[0] + spec.center_offset[0],
                    center[1] + spec.center_offset[1],
                ],
                size: *size,
                depth: *depth,
                corner_radius: *corner_radius,
                rim_width: *rim_width,
                corner_segments: *corner_segments,
                entry_loop: spec.entry_loop,
                floor_loop: spec.secondary_loop,
                outer_region: *outer_region,
                rim_region: spec.rim_region,
                wall_region: spec.wall_region,
                floor_region,
                edge_treatment: *edge_treatment,
            })
        }
        ModelingOperationSpec::RectangularThroughCut {
            region,
            face,
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            outer_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::RectangularThroughCut {
            operation: spec.operation,
            region: *region,
            face: *face,
            center: [
                center[0] + spec.center_offset[0],
                center[1] + spec.center_offset[1],
            ],
            size: *size,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: spec.entry_loop,
            exit_loop: spec.secondary_loop,
            outer_region: *outer_region,
            rim_region: spec.rim_region,
            wall_region: spec.wall_region,
            edge_treatment: *edge_treatment,
        }),
        ModelingOperationSpec::CircularThroughCut {
            region,
            face,
            center,
            radius,
            radial_segments,
            rim_width,
            outer_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::CircularThroughCut {
            operation: spec.operation,
            region: *region,
            face: *face,
            center: [
                center[0] + spec.center_offset[0],
                center[1] + spec.center_offset[1],
            ],
            radius: *radius,
            radial_segments: *radial_segments,
            rim_width: *rim_width,
            entry_loop: spec.entry_loop,
            exit_loop: spec.secondary_loop,
            outer_region: *outer_region,
            rim_region: spec.rim_region,
            wall_region: spec.wall_region,
            edge_treatment: *edge_treatment,
        }),
        _ => Err(AssetError::UnsupportedEdit(format!(
            "operation {:?} is not a cut",
            operation.operation_id()
        ))),
    }
}

fn remap_dependent_boundary_bevels(
    operations: &[ModelingOperationSpec],
    source_cut: &ModelingOperationSpec,
    duplicate_cut: &ModelingOperationSpec,
    specs: &[DuplicateBoundaryBevelSpec],
) -> Result<Vec<ModelingOperationSpec>, AssetError> {
    let mut copied = Vec::with_capacity(specs.len());
    let mut seen_sources = BTreeSet::new();
    for spec in specs {
        if !seen_sources.insert(spec.source) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate dependent bevel source {:?}",
                spec.source
            )));
        }
        let source_bevel = operations
            .iter()
            .find(|operation| operation.operation_id() == spec.source)
            .ok_or_else(|| {
                AssetError::UnsupportedEdit(format!(
                    "unknown dependent bevel operation {:?}",
                    spec.source
                ))
            })?;
        let ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            width,
            segments,
            profile,
            ..
        } = source_bevel
        else {
            return Err(AssetError::UnsupportedEdit(format!(
                "dependent operation {:?} is not a boundary-loop bevel",
                spec.source
            )));
        };
        let Some(remapped_target_loop) =
            remapped_cut_boundary_loop(source_cut, duplicate_cut, *target_loop)
        else {
            return Err(AssetError::UnsupportedEdit(format!(
                "dependent bevel {:?} does not target a loop produced by {:?}",
                spec.source,
                source_cut.operation_id()
            )));
        };
        copied.push(ModelingOperationSpec::BevelBoundaryLoop {
            operation: spec.operation,
            target_loop: remapped_target_loop,
            width: *width,
            segments: *segments,
            profile: *profile,
            bevel_region: spec.bevel_region,
            outer_replacement_loop: spec.outer_replacement_loop,
            inner_replacement_loop: spec.inner_replacement_loop,
        });
    }
    Ok(copied)
}

fn remapped_cut_boundary_loop(
    source_cut: &ModelingOperationSpec,
    duplicate_cut: &ModelingOperationSpec,
    source_loop: BoundaryLoopId,
) -> Option<BoundaryLoopId> {
    let source_loops = source_cut.direct_boundary_loop_outputs();
    let duplicate_loops = duplicate_cut.direct_boundary_loop_outputs();
    source_loops
        .iter()
        .position(|candidate| *candidate == source_loop)
        .and_then(|index| duplicate_loops.get(index).copied())
}

fn operation_id_exists(recipe: &AssetRecipe, operation: OperationId) -> bool {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .any(|candidate| candidate.operation_id() == operation)
}

fn ensure_new_operation_ids_available(
    recipe: &AssetRecipe,
    operations: Vec<OperationId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .map(ModelingOperationSpec::operation_id)
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for operation in operations {
        if !local.insert(operation) || !used.insert(operation) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate operation {operation:?}"
            )));
        }
    }
    Ok(())
}

fn ensure_new_boundary_loops_available(
    recipe: &AssetRecipe,
    boundary_loops: Vec<BoundaryLoopId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for boundary_loop in boundary_loops {
        if boundary_loop == LEGACY_MISSING_BOUNDARY_LOOP
            || !local.insert(boundary_loop)
            || !used.insert(boundary_loop)
        {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate boundary loop {boundary_loop:?}"
            )));
        }
    }
    Ok(())
}

fn ensure_new_generated_regions_available(
    recipe: &AssetRecipe,
    regions: Vec<RegionId>,
) -> Result<(), AssetError> {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| {
            definition.regions.keys().copied().chain(
                definition
                    .geometry
                    .operations
                    .iter()
                    .flat_map(ModelingOperationSpec::generated_region_ids),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut local = BTreeSet::new();
    for region in regions {
        if !local.insert(region) || !used.insert(region) {
            return Err(AssetError::UnsupportedEdit(format!(
                "duplicate generated region {region:?}"
            )));
        }
    }
    Ok(())
}

fn operation_detail_region_ids(operation: &ModelingOperationSpec) -> Vec<RegionId> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut {
            rim_region,
            wall_region,
            floor_region,
            ..
        } => vec![*rim_region, *wall_region, *floor_region],
        ModelingOperationSpec::RectangularThroughCut {
            rim_region,
            wall_region,
            ..
        }
        | ModelingOperationSpec::CircularThroughCut {
            rim_region,
            wall_region,
            ..
        } => vec![*rim_region, *wall_region],
        ModelingOperationSpec::BevelBoundaryLoop { bevel_region, .. } => vec![*bevel_region],
        _ => Vec::new(),
    }
}

fn duplicate_instance(
    recipe: &mut AssetRecipe,
    source: PartInstanceId,
    instance: PartInstanceId,
    name: Option<&str>,
    transform: Option<&Transform3>,
) -> Result<(), AssetError> {
    edits::ensure_instance_editable(recipe, source)?;
    if recipe.instances.contains_key(&instance) {
        return Err(AssetError::UnsupportedEdit(format!(
            "duplicate instance {instance:?}"
        )));
    }
    let mut duplicate = recipe
        .instances
        .get(&source)
        .ok_or(AssetError::UnknownInstance(source))?
        .clone();
    if let Some(parent) = duplicate.parent {
        edits::ensure_instance_editable(recipe, parent)?;
    }
    duplicate.id = instance;
    duplicate.name = name
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} copy", duplicate.name));
    if let Some(transform) = transform {
        duplicate.local_transform = transform.clone();
    }
    recipe.instances.insert(instance, duplicate.clone());
    if duplicate.parent.is_none() {
        insert_root_instance(recipe, instance);
    }
    recipe.variation.optional_instances.remove(&instance);
    bump_next_ids_for_instance(recipe, &duplicate);
    Ok(())
}

fn mirror_instance(
    recipe: &mut AssetRecipe,
    source: PartInstanceId,
    instance: PartInstanceId,
    plane: &MirrorInstanceSpec,
    name: Option<&str>,
) -> Result<(), AssetError> {
    if !array_is_finite(&plane.plane_normal) || !plane.plane_offset.is_finite() {
        return Err(AssetError::UnsupportedEdit(
            "mirror plane must be finite".to_owned(),
        ));
    }
    let normal = Vec3::from_array(plane.plane_normal);
    let length = normal.length();
    if length == 0.0 {
        return Err(AssetError::UnsupportedEdit(
            "mirror plane normal must be non-zero".to_owned(),
        ));
    }
    duplicate_instance(recipe, source, instance, name, None)?;
    let target = recipe
        .instances
        .get_mut(&instance)
        .ok_or(AssetError::UnknownInstance(instance))?;
    let unit_normal = normal / length;
    let point = Vec3::from_array(target.local_transform.translation);
    let distance = point.dot(unit_normal) - plane.plane_offset;
    target.local_transform.translation = (point - 2.0 * distance * unit_normal).to_array();
    Ok(())
}

fn reorder_child_instances(
    recipe: &mut AssetRecipe,
    parent: Option<PartInstanceId>,
    ordered_children: &[PartInstanceId],
) -> Result<(), AssetError> {
    let actual_children = match parent {
        Some(parent) => {
            edits::ensure_instance_editable(recipe, parent)?;
            if !recipe.instances.contains_key(&parent) {
                return Err(AssetError::UnknownInstance(parent));
            }
            recipe
                .instances
                .values()
                .filter(|candidate| candidate.parent == Some(parent))
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>()
        }
        None => recipe.root_instances.clone(),
    };
    let requested = ordered_children.iter().copied().collect::<BTreeSet<_>>();
    let actual = actual_children.iter().copied().collect::<BTreeSet<_>>();
    if requested != actual {
        return Err(AssetError::UnsupportedEdit(
            "reorder must contain exactly the current children".to_owned(),
        ));
    }
    if parent.is_none() {
        let mut roots = ordered_children.to_vec();
        roots.sort_unstable();
        recipe.root_instances = roots;
    }
    Ok(())
}

fn insert_root_instance(recipe: &mut AssetRecipe, instance: PartInstanceId) {
    if !recipe.root_instances.contains(&instance) {
        recipe.root_instances.push(instance);
        recipe.root_instances.sort_unstable();
    }
}

fn bump_next_ids_for_instance(recipe: &mut AssetRecipe, instance: &PartInstance) {
    bump_counter(&mut recipe.next_ids.part_instance, instance.id.0);
}

fn bump_next_ids_for_definition(recipe: &mut AssetRecipe, definition: &PartDefinition) {
    bump_counter(&mut recipe.next_ids.part_definition, definition.id.0);
    for operation in &definition.geometry.operations {
        bump_next_ids_for_operation(recipe, operation);
    }
    for region in definition.regions.keys() {
        bump_counter(&mut recipe.next_ids.region, region.0);
    }
    for socket in definition.sockets.keys() {
        bump_counter(&mut recipe.next_ids.socket, socket.0);
    }
}

fn bump_next_ids_for_operation(recipe: &mut AssetRecipe, operation: &ModelingOperationSpec) {
    bump_counter(&mut recipe.next_ids.operation, operation.operation_id().0);
    for boundary_loop in operation.boundary_loop_ids() {
        bump_counter(&mut recipe.next_ids.boundary_loop, boundary_loop.0);
    }
    for region in operation.generated_region_ids() {
        bump_counter(&mut recipe.next_ids.region, region.0);
    }
}

fn bump_counter(counter: &mut u64, used: u64) {
    *counter = (*counter).max(used.saturating_add(1));
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

fn scalar_to_u32(path: &str, value: f32) -> Result<u32, AssetError> {
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar {
            path: path.to_owned(),
            value,
        });
    }
    if value < 0.0 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value must not be negative",
        });
    }
    if value.fract() != 0.0 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value must be an integer",
        });
    }
    if value > u32::MAX as f32 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value exceeds u32 range",
        });
    }
    Ok(value as u32)
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

fn parse_index(raw: &str, path: &str) -> Result<usize, AssetError> {
    raw.parse::<usize>()
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
    suffix: impl AsRef<str>,
) -> Option<String> {
    let suffix = suffix.as_ref();
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

    fn socket(id: SocketId, name: &str) -> SocketSpec {
        SocketSpec {
            id,
            name: name.to_owned(),
            local_frame: Frame3::default(),
            role: "mount".to_owned(),
            tags: BTreeSet::new(),
        }
    }

    fn multipart_recipe() -> AssetRecipe {
        let mut recipe = test_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("body definition should exist")
            .sockets
            .insert(SocketId(1), socket(SocketId(1), "body_mount"));

        let mut wheel_sockets = BTreeMap::new();
        wheel_sockets.insert(SocketId(2), socket(SocketId(2), "wheel_mount"));
        let wheel_definition = PartDefinition {
            id: PartDefinitionId(2),
            name: "Wheel".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source: GeometrySource::Cylinder {
                    radius: 0.25,
                    height: 0.2,
                    radial_segments: 16,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: wheel_sockets,
            local_pivot: Frame3::default(),
            variant_group: Some("wheel".to_owned()),
            production_hints: None,
        };
        let wheel_instance = PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(2),
            name: "Wheel L".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3::default(),
            attachment: Some(AttachmentSpec {
                parent_instance: PartInstanceId(1),
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                local_offset: Transform3::default(),
                mode: AttachmentMode::RigidSeparate,
            }),
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };

        recipe
            .definitions
            .insert(wheel_definition.id, wheel_definition);
        recipe.instances.insert(wheel_instance.id, wheel_instance);
        recipe.next_ids.part_definition = 3;
        recipe.next_ids.part_instance = 3;
        recipe.next_ids.socket = 3;
        recipe
            .variation
            .optional_instances
            .insert(PartInstanceId(2));
        recipe.variation.replacement_groups.insert(
            "wheel".to_owned(),
            ReplacementGroupHint {
                definitions: BTreeSet::from([PartDefinitionId(2)]),
            },
        );
        recipe.variation.count_ranges.insert(
            OperationId(1),
            CountRangeHint {
                minimum: 1,
                maximum: 6,
            },
        );
        recipe.variation.parameter_range_overrides.insert(
            ParameterId(1),
            ParameterRangeOverride {
                minimum: 0.0,
                maximum: 0.5,
                step: Some(0.01),
                mutation_sigma: None,
            },
        );
        recipe
    }

    fn issue_codes(report: &AssetValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    fn relationship_contract(
        id: RelationshipId,
        parent: PartInstanceId,
        child: PartInstanceId,
    ) -> RelationshipContract {
        RelationshipContract {
            id,
            relationship_type: RelationshipType::SurfaceMounted,
            parent: Some(parent),
            child: Some(child),
            parent_node_ref: None,
            child_node_ref: None,
            parent_anchor_id: None,
            child_anchor_id: None,
            label: "mounted child".to_owned(),
            export_profile: None,
            placement_policy: PlacementPolicy::default(),
            orientation_policy: OrientationPolicy::default(),
            scale_policy: ScalePolicy::default(),
            contact_policy: ContactPolicy::default(),
            edit_policy: RelationshipEditPolicy::default(),
            selection_policy: SelectionPolicy::default(),
            reset_policy: ResetPolicy::default(),
            export_realization: ExportRealizationPolicy::default(),
        }
    }

    fn linear_pattern_contract(id: PatternId, source_instance: PartInstanceId) -> PatternContract {
        PatternContract {
            id,
            pattern_type: PatternType::Linear,
            source_instance: Some(source_instance),
            count: Some(3),
            label: "repeat detail".to_owned(),
            count_policy: PatternCountPolicy::Exact(3),
            density_policy: None,
            export_instancing: PatternExportInstancingPolicy::default(),
            linear_axis: Some(PatternAxis::X),
            spacing: Some(0.25),
            generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
        }
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
    fn asset_recipe_v8_empty_semantic_shells_validate() {
        let recipe = AssetRecipe::new(AssetId(9), "V8 semantic shells");

        assert_eq!(recipe.schema_version, 8);
        assert_eq!(recipe.semantic.review_state, ReviewState::default());
        assert_eq!(recipe.semantic.export_includes, ExportIncludes::default());
        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn schema_seven_recipe_migrates_to_v8_empty_semantic_shells() {
        let recipe = test_recipe();
        let mut value = serde_json::to_value(&recipe).expect("recipe should serialize");
        value["schema_version"] = serde_json::json!(7);
        value.as_object_mut().unwrap().remove("semantic");
        for key in [
            "relationship",
            "pattern",
            "surface_slot",
            "material_slot",
            "collision_body",
            "motion_channel",
            "terrain_patch",
            "export_profile",
            "authoring_op",
            "validation_report",
        ] {
            value["next_ids"].as_object_mut().unwrap().remove(key);
        }

        let migrated: AssetRecipe =
            serde_json::from_value(value).expect("schema 7 recipe should migrate");

        assert_eq!(migrated.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
        assert_eq!(migrated.semantic, AssetRecipeSemanticShells::default());
        assert!(validate_asset_recipe(&migrated).is_valid());
    }

    #[test]
    fn asset_recipe_v8_semantic_shells_round_trip_deterministically() {
        let mut recipe = multipart_recipe();
        recipe.semantic.relationships.insert(
            RelationshipId(1),
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2)),
        );
        recipe.semantic.patterns.insert(
            PatternId(1),
            linear_pattern_contract(PatternId(1), PartInstanceId(2)),
        );
        recipe.next_ids.relationship = 2;
        recipe.next_ids.pattern = 2;

        let first = serde_json::to_string(&recipe).expect("recipe serializes");
        let round_tripped: AssetRecipe = serde_json::from_str(&first).expect("recipe parses");
        let second = serde_json::to_string(&round_tripped).expect("recipe serializes");

        assert_eq!(first, second);
        assert_eq!(recipe, round_tripped);
        assert!(validate_asset_recipe(&round_tripped).is_valid());
    }

    #[test]
    fn semantic_shell_validation_rejects_unknown_references() {
        let mut recipe = test_recipe();
        recipe.semantic.relationships.insert(
            RelationshipId(1),
            RelationshipContract {
                parent: Some(PartInstanceId(404)),
                export_profile: Some(ExportProfileId(404)),
                ..relationship_contract(RelationshipId(1), PartInstanceId(404), PartInstanceId(1))
            },
        );
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                source_instance: Some(PartInstanceId(405)),
                count: Some(0),
                count_policy: PatternCountPolicy::Exact(0),
                ..linear_pattern_contract(PatternId(1), PartInstanceId(405))
            },
        );
        recipe.semantic.material_slots.insert(
            MaterialSlotId(1),
            MaterialSlotShell {
                id: MaterialSlotId(1),
                surface_slot: Some(SurfaceSlotId(404)),
                label: String::new(),
            },
        );
        recipe.semantic.authoring_ops.insert(
            AuthoringOpId(1),
            AuthoringOpShell {
                id: AuthoringOpId(1),
                target_parameter: Some(ParameterId(404)),
                target_instance: Some(PartInstanceId(406)),
                label: String::new(),
            },
        );
        recipe.next_ids.relationship = 2;
        recipe.next_ids.pattern = 2;
        recipe.next_ids.material_slot = 2;
        recipe.next_ids.authoring_op = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unknown_semantic_relationship_parent"));
        assert!(codes.contains("unknown_semantic_relationship_export_profile"));
        assert!(codes.contains("unknown_semantic_pattern_source"));
        assert!(codes.contains("invalid_semantic_pattern_count"));
        assert!(codes.contains("unknown_semantic_material_surface_slot"));
        assert!(codes.contains("unknown_semantic_authoring_parameter"));
        assert!(codes.contains("unknown_semantic_authoring_instance"));
    }

    #[test]
    fn semantic_shell_validation_rejects_product_claims() {
        let mut recipe = test_recipe();
        recipe.semantic.review_state = ReviewState {
            tier: ReviewTier::Published,
            human_review_required: false,
            publish_allowed: true,
            public_catalog_visible: true,
        };
        recipe.semantic.export_profiles.insert(
            ExportProfileId(1),
            ExportProfileShell {
                id: ExportProfileId(1),
                label: "invalid export".to_owned(),
                includes: ExportIncludes {
                    includes_geometry: true,
                    includes_textures: true,
                    includes_collision: true,
                    game_ready: true,
                    ..ExportIncludes::default()
                },
            },
        );
        recipe.next_ids.export_profile = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unsupported_semantic_review_tier"));
        assert!(codes.contains("semantic_human_review_required_false"));
        assert!(codes.contains("semantic_publish_allowed"));
        assert!(codes.contains("semantic_public_catalog_visible"));
        assert!(codes.contains("unsupported_semantic_export_include"));
        assert!(codes.contains("semantic_game_ready_claim"));
    }

    #[test]
    fn relationship_contract_accepts_fixed_and_proportional_placement() {
        let mut recipe = multipart_recipe();
        let mut fixed =
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2));
        fixed.parent_node_ref = Some("panel".to_owned());
        fixed.child_node_ref = Some("knob".to_owned());
        fixed.parent_anchor_id = Some("front_handle_zone".to_owned());
        fixed.child_anchor_id = Some("back_mount_point".to_owned());
        fixed.placement_policy = PlacementPolicy {
            position_rule: PositionRule::FixedOffsetFromEdge {
                edge: "right".to_owned(),
                offset: [0.1, 0.0, 0.0],
            },
        };
        fixed.contact_policy = ContactPolicy::SurfaceContact { clearance: 0.0 };
        let mut proportional =
            relationship_contract(RelationshipId(2), PartInstanceId(1), PartInstanceId(2));
        proportional.placement_policy = PlacementPolicy {
            position_rule: PositionRule::ProportionalUv { u: 0.5, v: 0.25 },
        };
        proportional.scale_policy = ScalePolicy::ClampToRange {
            minimum: 0.5,
            maximum: 2.0,
        };
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(1), fixed);
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(2), proportional);
        recipe.next_ids.relationship = 3;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn relationship_contract_rejects_cycles_and_invalid_domains() {
        let mut recipe = multipart_recipe();
        let mut first =
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2));
        first.parent_anchor_id = Some("Front Handle Zone".to_owned());
        first.placement_policy = PlacementPolicy {
            position_rule: PositionRule::ProportionalUv { u: 2.0, v: 0.5 },
        };
        first.scale_policy = ScalePolicy::ClampToRange {
            minimum: 2.0,
            maximum: 1.0,
        };
        first.contact_policy = ContactPolicy::IntentionalGap { clearance: -0.1 };
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(1), first);
        recipe.semantic.relationships.insert(
            RelationshipId(2),
            relationship_contract(RelationshipId(2), PartInstanceId(2), PartInstanceId(1)),
        );
        recipe.next_ids.relationship = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("value_out_of_range"));
        assert!(codes.contains("invalid_relationship_scale_range"));
        assert!(codes.contains("negative_value"));
        assert!(codes.contains("semantic_relationship_cycle"));
        assert!(codes.contains("invalid_semantic_relationship_parent_anchor"));
    }

    #[test]
    fn pattern_contract_accepts_valid_linear_pattern() {
        let mut recipe = multipart_recipe();
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                count_policy: PatternCountPolicy::Range {
                    minimum: 2,
                    maximum: 6,
                },
                density_policy: Some(PatternDensityPolicy::Range {
                    minimum: 0.0,
                    maximum: 1.0,
                }),
                export_instancing: PatternExportInstancingPolicy::Disabled,
                ..linear_pattern_contract(PatternId(1), PartInstanceId(2))
            },
        );
        recipe.next_ids.pattern = 2;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn pattern_contract_rejects_invalid_count_and_density() {
        let mut recipe = multipart_recipe();
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                count_policy: PatternCountPolicy::Range {
                    minimum: 0,
                    maximum: 20_000,
                },
                density_policy: Some(PatternDensityPolicy::Range {
                    minimum: 3.0,
                    maximum: 1.0,
                }),
                ..linear_pattern_contract(PatternId(1), PartInstanceId(2))
            },
        );
        recipe.next_ids.pattern = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("invalid_semantic_pattern_count"));
        assert!(codes.contains("invalid_semantic_pattern_density_range"));
    }

    #[test]
    fn schema_one_relationships_migrate_to_schema_two() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: Some(0.001),
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::specific(PartInstanceId(1)),
                second: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                reason: "legacy authored contact".to_owned(),
            });
        let mut value = serde_json::to_value(&recipe).expect("recipe should serialize");
        value["schema_version"] = serde_json::json!(1);
        value["relationships"] = serde_json::json!([
            {
                "SocketAttached": {
                    "parent": 1,
                    "child": 2,
                    "socket": 7,
                    "max_origin_distance": 0.001,
                    "max_axis_angle_degrees": 1.0,
                    "max_clearance": 0.001
                }
            },
            {
                "MayOverlap": {
                    "first": 1,
                    "second": 2,
                    "reason": "legacy authored contact"
                }
            }
        ]);

        let migrated: AssetRecipe =
            serde_json::from_value(value).expect("schema one recipe should migrate");

        assert_eq!(migrated.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
        assert!(matches!(
            &migrated.relationships[0],
            AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(1)
                },
                child: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(2)
                },
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(7),
                child_socket: SocketId(7),
                ..
            }
        ));
        assert!(matches!(
            &migrated.relationships[1],
            AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(1)
                },
                second: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(2)
                },
                pairing: RelationshipPairing::AllPairs,
                ..
            }
        ));
        let saved = serde_json::to_value(&migrated).expect("migrated recipe should serialize");
        assert_eq!(
            saved["schema_version"],
            serde_json::json!(ASSET_RECIPE_SCHEMA_VERSION)
        );
        assert!(
            saved["relationships"][0]["SocketAttached"]
                .get("socket")
                .is_none()
        );
    }

    #[test]
    fn validation_accepts_minimal_valid_recipe() {
        let recipe = test_recipe();

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_rejects_invalid_semantic_cut_groups() {
        let mut recipe = test_recipe();
        recipe.variation.semantic_cut_groups.insert(
            "body_rows".to_owned(),
            SemanticCutGroupHint {
                label: String::new(),
                definition: PartDefinitionId(1),
                operations: vec![OperationId(1), OperationId(1), OperationId(99)],
                role: CutGroupRole::Vents,
                count_range: Some(CountRangeHint {
                    minimum: 0,
                    maximum: 1,
                }),
            },
        );

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("empty_semantic_cut_group_label"));
        assert!(codes.contains("duplicate_semantic_cut_group_operation"));
        assert!(codes.contains("invalid_semantic_cut_group_operation"));
        assert!(codes.contains("unknown_semantic_cut_group_operation"));
        assert!(codes.contains("semantic_cut_group_count_range_too_small"));
    }

    #[test]
    fn validation_rejects_phase_inverted_loaded_operations() {
        let mut recipe = test_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist")
            .geometry
            .operations
            .push(ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(2),
                radius: 0.02,
                segments: 1,
            });
        recipe.next_ids.operation = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("invalid_operation_phase_order"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_unsupported_semantic_cut_hosts() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Cylinder {
            radius: 0.5,
            height: 1.0,
            radial_segments: 16,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(2),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::Hard,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("unsupported_semantic_cut_host"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_multi_face_rounded_box_cut_sets() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::Hard,
            },
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(3),
                region: RegionId(1),
                face: PlanarCutFace::PositiveZ,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(3),
                exit_loop: BoundaryLoopId(4),
                outer_region: RegionId(1),
                rim_region: RegionId(4),
                wall_region: RegionId(5),
                edge_treatment: CutEdgeTreatment::Hard,
            },
        ];
        recipe.next_ids.operation = 4;
        recipe.next_ids.region = 6;
        recipe.next_ids.boundary_loop = 5;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("unsupported_rounded_box_cut_face_set"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_duplicate_direct_boundary_loop_outputs() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(1),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::Hard,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 2;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("duplicate_direct_boundary_loop_output"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_out_of_range_boundary_bevel_profile() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::BevelEligible,
            },
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(3),
                target_loop: BoundaryLoopId(1),
                width: 0.01,
                segments: 2,
                profile: 100.0,
                bevel_region: RegionId(4),
                outer_replacement_loop: BoundaryLoopId(3),
                inner_replacement_loop: BoundaryLoopId(4),
            },
        ];
        recipe.next_ids.operation = 4;
        recipe.next_ids.region = 5;
        recipe.next_ids.boundary_loop = 5;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("value_out_of_range"),
            "{report:?}"
        );
    }

    #[test]
    fn inserting_boundary_bevel_rejects_reused_generated_region() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(2),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let result = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "reuse bevel region".to_owned(),
                seed: 9,
                operations: vec![AssetEdit::InsertModelingOperation {
                    definition: PartDefinitionId(1),
                    index: 1,
                    operation: ModelingOperationSpec::BevelBoundaryLoop {
                        operation: OperationId(3),
                        target_loop: BoundaryLoopId(1),
                        width: 0.01,
                        segments: 2,
                        profile: 1.0,
                        bevel_region: RegionId(3),
                        outer_replacement_loop: BoundaryLoopId(3),
                        inner_replacement_loop: BoundaryLoopId(4),
                    },
                }],
            },
        );

        assert!(
            matches!(result, Err(AssetError::UnsupportedEdit(ref message)) if message.contains("duplicate generated region")),
            "{result:?}"
        );
    }

    #[test]
    fn validation_rejects_cut_group_role_and_count_mismatch() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition
            .geometry
            .operations
            .push(ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::Hard,
            });
        recipe.variation.semantic_cut_groups.insert(
            "vents".to_owned(),
            SemanticCutGroupHint {
                label: "Vents".to_owned(),
                definition: PartDefinitionId(1),
                operations: vec![OperationId(2)],
                role: CutGroupRole::Vents,
                count_range: Some(CountRangeHint {
                    minimum: 2,
                    maximum: 4,
                }),
            },
        );
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("semantic_cut_group_role_mismatch"));
        assert!(codes.contains("semantic_cut_group_count_out_of_range"));
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

    #[test]
    fn validation_accepts_valid_multipart_assembly() {
        let recipe = multipart_recipe();

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_accepts_relationship_selectors_and_separate_sockets() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: Some(0.001),
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::PrototypeAndGeneratedOccurrences {
                    prototype: PartInstanceId(1),
                },
                second: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(1),
                },
                pairing: RelationshipPairing::AllPairs,
                reason: "arrayed prototype contacts are authored".to_owned(),
            });
        recipe
            .definitions
            .get_mut(&PartDefinitionId(2))
            .expect("wheel definition should exist")
            .tags
            .insert("support".to_owned());
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MustTouch {
                first: AssetPartSelector::DefinitionRole {
                    role: "support".to_owned(),
                },
                second: AssetPartSelector::specific(PartInstanceId(1)),
                pairing: RelationshipPairing::AllPairs,
                max_clearance: 0.02,
            });

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_reports_unknown_relationship_selectors_and_sockets() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(99),
                child_socket: SocketId(98),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: None,
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MinimumClearance {
                first: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(99),
                },
                second: AssetPartSelector::PartTag {
                    tag: "missing".to_owned(),
                },
                pairing: RelationshipPairing::AllPairs,
                clearance: 0.01,
            });

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unknown_relationship_parent_socket"));
        assert!(codes.contains("unknown_relationship_child_socket"));
        assert!(codes.contains("unknown_relationship_operation"));
        assert!(codes.contains("unknown_relationship_selector"));
    }

    #[test]
    fn validation_accepts_reused_part_definition() {
        let mut recipe = test_recipe();
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(1),
                name: "Body copy".to_owned(),
                parent: Some(PartInstanceId(1)),
                local_transform: Transform3::default(),
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.next_ids.part_instance = 3;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_reports_hierarchy_cycle() {
        let mut recipe = test_recipe();
        recipe.root_instances.clear();
        recipe
            .instances
            .get_mut(&PartInstanceId(1))
            .expect("root should exist")
            .parent = Some(PartInstanceId(2));
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(1),
                name: "Cycle".to_owned(),
                parent: Some(PartInstanceId(1)),
                local_transform: Transform3::default(),
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.next_ids.part_instance = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(issue_codes(&report).contains("parent_cycle"));
    }

    #[test]
    fn validation_reports_dangling_socket() {
        let mut recipe = multipart_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("body definition should exist")
            .sockets
            .remove(&SocketId(1));

        let report = validate_asset_recipe(&recipe);

        assert!(issue_codes(&report).contains("unknown_parent_socket"));
    }

    #[test]
    fn validation_reports_invalid_attachment() {
        let mut recipe = multipart_recipe();
        let wheel = recipe
            .instances
            .get_mut(&PartInstanceId(2))
            .expect("wheel instance should exist");
        wheel.parent = None;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("attachment_parent_mismatch"));
        assert!(codes.contains("missing_root_instance"));
    }

    #[test]
    fn scalar_get_set_round_trip() {
        let mut recipe = test_recipe();
        let path = definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius");

        set_scalar(&mut recipe, &path, 0.4).expect("scalar edit should apply");

        assert_eq!(
            get_scalar(&recipe, &path).expect("scalar should exist"),
            0.4
        );
    }

    #[test]
    fn failed_validation_keeps_edit_program_atomic() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "bad transform".to_owned(),
            seed: 3,
            operations: vec![
                AssetEdit::SetScalar {
                    parameter: ParameterId(1),
                    value: 0.2,
                },
                AssetEdit::SetTransform {
                    instance: PartInstanceId(1),
                    transform: Transform3 {
                        scale: [0.0, 1.0, 1.0],
                        ..Transform3::default()
                    },
                },
            ],
        };

        assert!(matches!(
            apply_edit_program(&recipe, &program),
            Err(AssetError::ValidationFailed(report))
                if issue_codes(&report).contains("zero_scale")
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
    fn add_and_remove_instance_updates_roots_deterministically() {
        let recipe = test_recipe();
        let added = PartInstance {
            id: PartInstanceId(3),
            definition: PartDefinitionId(1),
            name: "Accessory".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "add".to_owned(),
                seed: 4,
                operations: vec![AssetEdit::AddInstance { instance: added }],
            },
        )
        .expect("add should apply");

        assert_eq!(
            edited.root_instances,
            vec![PartInstanceId(1), PartInstanceId(3)]
        );
        assert_eq!(edited.next_ids.part_instance, 4);

        let removed = apply_edit_program(
            &edited,
            &AssetEditProgram {
                label: "remove".to_owned(),
                seed: 5,
                operations: vec![AssetEdit::RemoveInstance {
                    instance: PartInstanceId(3),
                }],
            },
        )
        .expect("remove should apply");

        assert_eq!(removed.root_instances, vec![PartInstanceId(1)]);
        assert!(!removed.instances.contains_key(&PartInstanceId(3)));
    }

    #[test]
    fn removing_instance_with_descendants_is_rejected() {
        let recipe = multipart_recipe();

        assert!(matches!(
            apply_edit_program(
                &recipe,
                &AssetEditProgram {
                    label: "remove parent".to_owned(),
                    seed: 6,
                    operations: vec![AssetEdit::RemoveInstance {
                        instance: PartInstanceId(1),
                    }],
                },
            ),
            Err(AssetError::UnsupportedEdit(message))
                if message.contains("descendants")
        ));
    }

    #[test]
    fn replace_definition_preserves_instances() {
        let recipe = test_recipe();
        let mut replacement = recipe
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist")
            .clone();
        replacement.name = "Replacement Body".to_owned();
        if let GeometrySource::RoundedBox { radius, .. } = &mut replacement.geometry.source {
            *radius = 0.2;
        }

        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "replace".to_owned(),
                seed: 8,
                operations: vec![AssetEdit::ReplaceDefinition {
                    definition: replacement,
                }],
            },
        )
        .expect("replace should apply");

        assert_eq!(
            edited
                .instances
                .get(&PartInstanceId(1))
                .expect("instance should exist")
                .definition,
            PartDefinitionId(1)
        );
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
    fn locked_parameters_remain_inspectable() {
        let mut recipe = test_recipe();
        recipe.locks.insert(ParameterId(1));

        assert_eq!(enumerate_parameters(&recipe).len(), 1);
        assert_eq!(
            get_scalar(
                &recipe,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("locked parameter should remain readable"),
            0.1
        );
    }

    #[test]
    fn deterministic_serialization_orders_semantic_ids() {
        let mut recipe = test_recipe();
        let mut second = recipe
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist")
            .clone();
        second.id = PartDefinitionId(2);
        second.name = "Second".to_owned();
        recipe.definitions.insert(second.id, second);
        recipe.next_ids.part_definition = 3;

        let json = serde_json::to_string(&recipe).expect("recipe should serialize");
        let first_position = json.find("\"1\"").expect("id 1 key should serialize");
        let second_position = json.find("\"2\"").expect("id 2 key should serialize");

        assert!(first_position < second_position);
        assert_eq!(
            json,
            serde_json::to_string(&recipe).expect("recipe should serialize deterministically")
        );
    }

    #[test]
    fn unrelated_parameter_edit_preserves_semantic_ids() {
        let recipe = multipart_recipe();
        let definition_ids = recipe.definitions.keys().copied().collect::<Vec<_>>();
        let instance_ids = recipe.instances.keys().copied().collect::<Vec<_>>();
        let operation_ids = recipe.definitions[&PartDefinitionId(1)]
            .geometry
            .operations
            .iter()
            .map(ModelingOperationSpec::operation_id)
            .collect::<Vec<_>>();
        let next_ids = recipe.next_ids.clone();

        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "radius".to_owned(),
                seed: 9,
                operations: vec![AssetEdit::SetScalar {
                    parameter: ParameterId(1),
                    value: 0.2,
                }],
            },
        )
        .expect("edit should apply");

        assert_eq!(
            edited.definitions.keys().copied().collect::<Vec<_>>(),
            definition_ids
        );
        assert_eq!(
            edited.instances.keys().copied().collect::<Vec<_>>(),
            instance_ids
        );
        assert_eq!(
            edited.definitions[&PartDefinitionId(1)]
                .geometry
                .operations
                .iter()
                .map(ModelingOperationSpec::operation_id)
                .collect::<Vec<_>>(),
            operation_ids
        );
        assert_eq!(edited.next_ids, next_ids);
    }

    #[test]
    fn validation_reports_multiple_issues() {
        let mut recipe = test_recipe();
        recipe.root_instances.push(PartInstanceId(1));
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(99),
                name: "Invalid".to_owned(),
                parent: Some(PartInstanceId(42)),
                local_transform: Transform3 {
                    translation: [f32::NAN, 0.0, 0.0],
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.parameters.insert(
            ParameterId(2),
            ParameterDescriptor {
                id: ParameterId(2),
                path: "definition.1.geometry.rounded_box.nope".to_owned(),
                label: "Bad".to_owned(),
                group: "Bad".to_owned(),
                minimum: 1.0,
                maximum: 0.0,
                step: 0.0,
                mutation_sigma: -1.0,
                topology_changing: false,
                beginner_description: "Bad".to_owned(),
            },
        );
        recipe
            .variation
            .optional_instances
            .insert(PartInstanceId(404));
        recipe.next_ids.part_instance = 3;
        recipe.next_ids.parameter = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(report.issues.len() >= 7);
        assert!(codes.contains("duplicate_root_instance"));
        assert!(codes.contains("unknown_instance_definition"));
        assert!(codes.contains("unknown_parent_instance"));
        assert!(codes.contains("non_finite"));
        assert!(codes.contains("invalid_parameter_range"));
        assert!(codes.contains("unknown_parameter_path"));
        assert!(codes.contains("unknown_optional_instance"));
    }

    #[test]
    fn parameter_reflection_filters_invalid_descriptors_but_not_locks() {
        let mut recipe = test_recipe();
        recipe.parameters.insert(
            ParameterId(2),
            ParameterDescriptor {
                id: ParameterId(2),
                path: "definition.1.geometry.rounded_box.nope".to_owned(),
                label: "Invalid".to_owned(),
                group: "Form".to_owned(),
                minimum: 0.0,
                maximum: 1.0,
                step: 0.01,
                mutation_sigma: 0.05,
                topology_changing: false,
                beginner_description: "Invalid".to_owned(),
            },
        );
        recipe.locks.insert(ParameterId(1));
        recipe.next_ids.parameter = 3;

        let reflected = enumerate_parameters(&recipe);

        assert_eq!(
            reflected
                .iter()
                .map(|parameter| parameter.id)
                .collect::<Vec<_>>(),
            vec![ParameterId(1)]
        );
    }
}
