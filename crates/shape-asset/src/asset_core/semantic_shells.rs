
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
