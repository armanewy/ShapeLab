#![forbid(unsafe_code)]

//! Runtime-neutral game asset metadata layered on top of Shape Lab recipes.
//!
//! The contracts in this crate describe how an authored Shape Lab asset can be
//! placed, snapped, traversed, collision-proxied, budgeted, and previewed by a
//! game runtime. They intentionally avoid gameplay-balance values such as cost,
//! labor, damage, movement bonuses, or AI behavior.

pub mod export;
pub mod readability;
pub mod validation;

use serde::{Deserialize, Serialize};
use shape_asset::{AssetRecipe, Frame3};

pub use validation::{
    GameAssetValidationIssue, GameAssetValidationReport, validate_construction_profile,
    validate_game_asset_definition, validate_game_asset_pack, validate_logical_footprint,
    validate_readability_profile, validate_snap_anchors, validate_triangle_budget,
    validate_walkable_surfaces,
};

/// Current schema version for game asset packs.
pub const GAME_ASSET_PACK_SCHEMA_VERSION: u32 = 1;

/// A deterministic bundle of runtime-neutral game asset definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAssetPack {
    /// Game asset pack schema version.
    pub schema_version: u32,
    /// Stable pack identifier.
    pub id: String,
    /// Human-facing pack title.
    pub title: String,
    /// Authored game asset definitions in deterministic runtime-key order.
    pub assets: Vec<GameAssetDefinition>,
    /// Export profile used by downstream tooling.
    pub export_profile: ExportProfile,
    /// Source repository revision or authored provenance string.
    pub source_revision: String,
}

/// One Shape Lab recipe plus game-runtime metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAssetDefinition {
    /// Stable asset definition identifier.
    pub id: String,
    /// Human-facing asset name.
    pub display_name: String,
    /// Generic asset family name.
    pub family: String,
    /// Source Shape Lab recipe.
    pub source_recipe: AssetRecipe,
    /// Placement, traversal, collision, and semantic labels.
    pub module_semantics: ModuleSemantics,
    /// Authored construction phase contract.
    pub construction_profile: ConstructionProfile,
    /// Fixed-camera readability contract.
    pub readability_profile: ReadabilityProfile,
    /// Triangle budgets for preview, game export, and repeated instances.
    pub budgets: TriangleBudget,
    /// Additional semantic tags.
    pub tags: Vec<String>,
}

/// Export profile for a game asset pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportProfile {
    /// Stable export profile key.
    pub id: String,
    /// Whether canonical model packages should be emitted.
    pub emit_model_packages: bool,
    /// Whether fixed-camera preview artifacts should be emitted.
    pub emit_previews: bool,
}

impl ExportProfile {
    /// Deterministic default profile for internal dogfooding packs.
    #[must_use]
    pub fn internal_dogfood() -> Self {
        Self {
            id: "internal-dogfood".to_owned(),
            emit_model_packages: true,
            emit_previews: true,
        }
    }
}

/// Runtime-neutral placement and semantic metadata for a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleSemantics {
    /// Runtime module key used by a game catalog.
    pub runtime_key: String,
    /// Integer footprint and vertical layer bounds.
    pub logical_footprint: LogicalFootprint,
    /// Rotation symmetry contract.
    pub rotation_symmetry: RotationSymmetry,
    /// Whether this module is intended to be instanced repeatedly.
    pub instanceable: bool,
    /// Semantic anchors used for snapping and runtime attachment.
    pub snap_anchors: Vec<SnapAnchor>,
    /// Surfaces that can support other modules or pieces.
    pub support_surfaces: Vec<SupportSurface>,
    /// Walkable or traversable authored surfaces.
    pub walkable_surfaces: Vec<WalkableSurface>,
    /// Explicit traversal links between anchors.
    pub traversal_links: Vec<TraversalLink>,
    /// Simple collision proxies.
    pub collision_proxies: Vec<CollisionProxy>,
    /// Semantic gameplay labels only; no balance values.
    pub gameplay_tags: Vec<GameplayTag>,
}

/// Integer logical footprint in grid cells and vertical layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalFootprint {
    /// Inclusive 2D cell bounds.
    pub cell_bounds: CellBounds,
    /// Inclusive vertical layer bounds.
    pub vertical_layers: LayerBounds,
    /// Authored origin cell.
    pub origin_cell: [i32; 2],
    /// Permitted placed rotations.
    pub permitted_rotations: Vec<GridRotation>,
}

/// Inclusive grid-cell bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellBounds {
    /// Minimum inclusive cell coordinate.
    pub min: [i32; 2],
    /// Maximum inclusive cell coordinate.
    pub max: [i32; 2],
}

/// Inclusive vertical layer bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerBounds {
    /// Minimum inclusive layer.
    pub min: i32,
    /// Maximum inclusive layer.
    pub max: i32,
}

/// Discrete placement rotation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GridRotation {
    /// No rotation.
    R0,
    /// 90 degrees clockwise.
    R90,
    /// 180 degrees.
    R180,
    /// 270 degrees clockwise.
    R270,
}

/// Rotation symmetry class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationSymmetry {
    /// No symmetry; all authored rotations may differ.
    None,
    /// Half-turn symmetry.
    TwoWay,
    /// Quarter-turn symmetry.
    FourWay,
}

/// A semantic snap or attachment anchor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapAnchor {
    /// Stable semantic anchor ID.
    pub id: String,
    /// Anchor role.
    pub role: SnapAnchorRole,
    /// Local frame for placement and orientation.
    pub local_frame: Frame3,
    /// Compatibility tags matched by game/runtime importers.
    pub compatibility_tags: Vec<String>,
    /// Whether this anchor requires or supplies support.
    pub relationship: SnapRelationship,
}

/// Runtime-neutral snap-anchor role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapAnchorRole {
    /// Linear continuation endpoint.
    Continuation,
    /// Support receiver or provider.
    Support,
    /// Entry point.
    Entry,
    /// Exit point.
    Exit,
    /// Brace connection.
    Brace,
    /// Center or pivot marker.
    Center,
    /// Pack-authored custom role.
    Custom(String),
}

/// Support relationship declared by a snap anchor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapRelationship {
    /// Optional snap point.
    Optional,
    /// This module requires support here.
    Required,
    /// This module provides support here.
    Supporting,
}

/// A surface that can support other assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportSurface {
    /// Stable semantic surface ID.
    pub id: String,
    /// Local surface shape.
    pub shape: SurfaceShape,
    /// Support role.
    pub support_role: SupportRole,
    /// Maximum supported layer hint.
    pub maximum_supported_layer_hint: Option<i32>,
}

/// A walkable or traversable local surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalkableSurface {
    /// Stable semantic surface ID.
    pub id: String,
    /// Local 2D polygon in module grid units.
    pub polygon: Vec<[f32; 2]>,
    /// Surface elevation in module units.
    pub elevation: f32,
    /// Traversal role.
    pub traversal_role: TraversalRole,
    /// Optional entry and exit anchor IDs.
    pub entry_exit_anchors: Vec<String>,
}

/// Local support surface shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceShape {
    /// Axis-aligned local rectangle.
    Rectangle {
        /// Rectangle center.
        center: [f32; 2],
        /// Rectangle size.
        size: [f32; 2],
    },
    /// Local polygon.
    Polygon {
        /// Polygon vertices.
        points: Vec<[f32; 2]>,
    },
}

/// Support surface role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportRole {
    /// Ground or foundation support.
    Foundation,
    /// Bridge or deck support.
    DeckSupport,
    /// Elevated platform support.
    ElevatedPlatform,
    /// Temporary work scaffold support.
    Scaffold,
    /// Pack-authored custom role.
    Custom(String),
}

/// Traversal surface role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalRole {
    /// Ground-level walkable surface.
    Ground,
    /// Road surface.
    Road,
    /// Bridge deck.
    BridgeDeck,
    /// Inclined ramp.
    Ramp,
    /// Elevated platform.
    Platform,
    /// Pack-authored custom role.
    Custom(String),
}

/// Traversal link between two anchors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalLink {
    /// Source anchor ID.
    pub from_anchor: String,
    /// Destination anchor ID.
    pub to_anchor: String,
    /// Traversal link kind.
    pub kind: TraversalLinkKind,
}

/// Supported traversal link kinds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalLinkKind {
    /// Inclined ramp movement.
    Ramp,
    /// Ladder-like vertical movement.
    Ladder,
    /// Short step.
    Step,
    /// Bridge-to-bridge connection.
    BridgeConnection,
}

/// Simple collision proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollisionProxy {
    /// Axis-aligned or oriented box proxy.
    Box {
        /// Center point.
        center: [f32; 3],
        /// Half extents.
        half_extents: [f32; 3],
    },
    /// Capsule proxy.
    Capsule {
        /// Capsule endpoint A.
        a: [f32; 3],
        /// Capsule endpoint B.
        b: [f32; 3],
        /// Capsule radius.
        radius: f32,
    },
    /// Cylinder proxy.
    Cylinder {
        /// Center point.
        center: [f32; 3],
        /// Cylinder radius.
        radius: f32,
        /// Cylinder height.
        height: f32,
    },
    /// Reserved future convex hull proxy.
    ConvexHullReserved {
        /// Human-facing reason or source label.
        reason: String,
    },
}

/// Semantic gameplay labels. Shape Lab does not simulate these labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GameplayTag {
    /// Walkable surface.
    Walkable,
    /// Blocks movement.
    BlocksMovement,
    /// Provides support.
    ProvidesSupport,
    /// Road surface.
    RoadSurface,
    /// Cover source.
    CoverSource,
    /// Elevated platform.
    ElevatedPlatform,
    /// Concealment signature.
    ConcealmentSignature,
    /// Decoy signature.
    DecoySignature,
    /// Pack-authored custom tag.
    Custom(String),
}

/// Authored construction phase contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionProfile {
    /// Ordered construction phases.
    pub phases: Vec<ConstructionPhase>,
    /// Optional damaged-state phase ID.
    pub optional_damaged_state: Option<String>,
    /// Final complete phase ID.
    pub final_phase: String,
    /// Visibility monotonicity policy.
    pub monotonic_visibility_policy: MonotonicVisibilityPolicy,
}

/// One construction phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionPhase {
    /// Stable phase ID.
    pub id: String,
    /// Human-facing phase label.
    pub label: String,
    /// Progress threshold from 0 to 1.
    pub progress_threshold: f32,
    /// Visible semantic part tags in this phase.
    pub visible_part_tags: Vec<String>,
    /// Optional required predecessor phase ID.
    pub required_predecessor: Option<String>,
}

/// Phase visibility policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonotonicVisibilityPolicy {
    /// Visible tags may only be added as construction progresses.
    Strict,
    /// Some tags may disappear when explicitly listed as temporary.
    AllowTemporaryHidden { tags: Vec<String> },
}

/// Fixed-camera readability contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadabilityProfile {
    /// Required fixed camera profiles.
    pub fixed_camera_profiles: Vec<FixedCameraProfile>,
    /// Minimum recognizable size in pixels.
    pub minimum_recognizable_pixel_size: u32,
    /// Relative silhouette importance from 0 to 1.
    pub silhouette_importance: f32,
    /// Maximum accepted hidden-area fraction from 0 to 1.
    pub maximum_hidden_area_fraction: f32,
    /// Required orientation coverage.
    pub orientation_coverage: Vec<GridRotation>,
}

/// Fixed camera profile key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FixedCameraProfile {
    /// Generic oblique strategy or isometric-style camera.
    Oblique,
    /// Generic top-down strategy camera.
    Top,
    /// Generic lower oblique inspection camera.
    LowOblique,
    /// Pack-authored custom camera profile.
    Custom(String),
}

/// Triangle budget for a game asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriangleBudget {
    /// Maximum preview triangles.
    pub preview_maximum: u32,
    /// Maximum game-export triangles.
    pub game_maximum: u32,
    /// Maximum triangles for highly repeated instances.
    pub repeated_instance_maximum: u32,
}
