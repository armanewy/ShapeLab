
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
