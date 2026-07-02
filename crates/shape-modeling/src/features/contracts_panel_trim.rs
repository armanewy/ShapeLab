
const EPSILON: f32 = 1.0e-5;

/// Panel front face region.
pub const PANEL_FRONT_REGION: RegionId = RegionId(1);
/// Panel border face region.
pub const PANEL_BORDER_REGION: RegionId = RegionId(2);
/// Panel side wall region.
pub const PANEL_SIDE_REGION: RegionId = RegionId(3);
/// Panel back face region.
pub const PANEL_BACK_REGION: RegionId = RegionId(4);

/// Trim body side region.
pub const TRIM_BODY_REGION: RegionId = RegionId(1);
/// Trim start cap region.
pub const TRIM_START_CAP_REGION: RegionId = RegionId(2);
/// Trim end cap region.
pub const TRIM_END_CAP_REGION: RegionId = RegionId(3);

/// Rib front face region.
pub const RIB_FRONT_REGION: RegionId = RegionId(1);
/// Rib back face region.
pub const RIB_BACK_REGION: RegionId = RegionId(2);
/// Rib edge wall region.
pub const RIB_EDGE_REGION: RegionId = RegionId(3);

/// Error type for semantic feature construction.
#[derive(Debug, Error)]
pub enum FeatureError {
    /// Feature input could not be placed or validated.
    #[error("feature validation failed for {feature}: {message}")]
    Validation {
        /// Feature family.
        feature: &'static str,
        /// Validation message.
        message: String,
    },
    /// Lower-level explicit modeling error.
    #[error(transparent)]
    Modeling(#[from] ModelingError),
}

type FeatureResult<T> = Result<T, FeatureError>;

/// Host data needed to resolve semantic feature placement.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureHost {
    /// Available sockets on the host part.
    pub sockets: BTreeMap<SocketId, SocketSpec>,
    /// Available semantic surface regions on the host part.
    pub regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    /// Author-provided planar frames for surface regions.
    pub region_frames: BTreeMap<RegionId, Frame3>,
}

impl FeatureHost {
    /// Build host data from a part definition.
    #[must_use]
    pub fn from_part_definition(definition: &PartDefinition) -> Self {
        Self {
            sockets: definition.sockets.clone(),
            regions: definition.regions.clone(),
            region_frames: BTreeMap::new(),
        }
    }

    /// Build host data from generated part metadata.
    #[must_use]
    pub fn from_generated_part(part: &GeneratedPart) -> Self {
        Self {
            sockets: part.sockets.clone(),
            regions: part.regions.clone(),
            region_frames: BTreeMap::new(),
        }
    }

    /// Add a planar frame for a semantic surface region.
    #[must_use]
    pub fn with_region_frame(mut self, region: RegionId, frame: Frame3) -> Self {
        self.region_frames.insert(region, frame);
        self
    }
}

/// Target for a planar feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanarHost {
    /// Attach to a socket by stable ID.
    Socket(SocketId),
    /// Attach to a socket by name.
    SocketName(String),
    /// Attach to a surface region by stable ID.
    SurfaceRegion(RegionId),
    /// Attach to a surface region by name.
    SurfaceRegionName(String),
}

/// Visual panel mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelVisualMode {
    /// Panel extends outward along the host frame normal.
    Raised,
    /// Panel extends inward along the host frame normal.
    Recessed,
}

/// Semantic panel feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelFeature {
    /// Stable operation ID used as face provenance.
    pub operation: OperationId,
    /// Host socket or planar region.
    pub host: PlanarHost,
    /// Panel width along host local X.
    pub width: f32,
    /// Panel height along host local Y.
    pub height: f32,
    /// Panel visual depth along host local Z.
    pub depth: f32,
    /// Corner radius in the panel plane.
    pub corner_radius: f32,
    /// Border width between front and outer perimeter.
    pub border_width: f32,
    /// Raised or recessed visual mode.
    pub mode: PanelVisualMode,
}

/// Build a semantic panel as separate explicit geometry.
pub fn build_panel_feature(
    feature: &PanelFeature,
    host: &FeatureHost,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    let frame = resolve_planar_frame(host, &feature.host, "panel")?;
    let basis = basis_from_frame(&frame, "panel")?;
    let width = positive(feature.width, "panel", "width")?;
    let height = positive(feature.height, "panel", "height")?;
    let depth = positive(feature.depth, "panel", "depth")?;
    let border_width = positive(feature.border_width, "panel", "border width")?;
    let corner_radius = non_negative(feature.corner_radius, "panel", "corner radius")?;
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    if border_width >= half_width.min(half_height) - EPSILON {
        return validation(
            "panel",
            "border width must leave a positive front face inside the panel",
        );
    }
    let outer_radius = corner_radius.min(half_width.min(half_height));
    let inner_half_width = half_width - border_width;
    let inner_half_height = half_height - border_width;
    let inner_radius = outer_radius.min(inner_half_width.min(inner_half_height));
    let corner_segments = if outer_radius > EPSILON { 6 } else { 1 };
    let outer = rounded_rect_points(half_width, half_height, outer_radius, corner_segments);
    let inner = rounded_rect_points(
        inner_half_width,
        inner_half_height,
        inner_radius,
        corner_segments,
    );
    let front_z = match feature.mode {
        PanelVisualMode::Raised => depth,
        PanelVisualMode::Recessed => -depth,
    };
    let mut builder = MeshBuilder::new();
    let outer_back = builder.add_ring(&outer, 0.0, &basis)?;
    let outer_front = builder.add_ring(&outer, front_z, &basis)?;
    let inner_front = builder.add_ring(&inner, front_z, &basis)?;

    add_loop_band(
        &mut builder,
        &outer_back,
        &outer_front,
        FaceStyle {
            operation: feature.operation,
            region: PANEL_SIDE_REGION,
            role: SurfaceRole::Side,
            smoothing_group: Some(1),
        },
        front_z >= 0.0,
    );
    add_loop_band(
        &mut builder,
        &outer_front,
        &inner_front,
        FaceStyle {
            operation: feature.operation,
            region: PANEL_BORDER_REGION,
            role: SurfaceRole::Panel,
            smoothing_group: None,
        },
        front_z >= 0.0,
    );
    add_cap_face(
        &mut builder,
        &inner_front,
        feature.operation,
        PANEL_FRONT_REGION,
        SurfaceRole::Panel,
        front_z >= 0.0,
    );
    add_cap_face(
        &mut builder,
        &outer_back,
        feature.operation,
        PANEL_BACK_REGION,
        SurfaceRole::Interior,
        front_z < 0.0,
    );

    let mut mesh = builder.finish(feature.operation)?;
    assign_part_context(&mut mesh, context);
    Ok(generated_part(
        mesh,
        feature_regions([
            (
                PANEL_FRONT_REGION,
                "panel_front",
                SurfaceRole::Panel,
                &["panel", "front"][..],
            ),
            (
                PANEL_BORDER_REGION,
                "panel_border",
                SurfaceRole::Panel,
                &["panel", "border"][..],
            ),
            (
                PANEL_SIDE_REGION,
                "panel_side",
                SurfaceRole::Side,
                &["panel", "side"][..],
            ),
            (
                PANEL_BACK_REGION,
                "panel_back",
                SurfaceRole::Interior,
                &["panel", "back"][..],
            ),
        ]),
        BTreeMap::new(),
        format!(
            "panel:v1:host={:?}:w={:.6}:h={:.6}:d={:.6}:r={:.6}:bw={:.6}:mode={:?}",
            feature.host, width, height, depth, outer_radius, border_width, feature.mode
        ),
    ))
}

/// Path source for trim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrimPath {
    /// Ordered points from a declared edge loop. The path is treated as closed.
    EdgeLoop { points: Vec<[f32; 3]> },
    /// Ordered authored path points.
    AuthoredPath {
        /// Path samples.
        points: Vec<[f32; 3]>,
        /// Whether the last point connects to the first.
        closed: bool,
    },
}

impl TrimPath {
    fn points(&self) -> &[[f32; 3]] {
        match self {
            Self::EdgeLoop { points } | Self::AuthoredPath { points, .. } => points,
        }
    }

    fn closed(&self) -> bool {
        match self {
            Self::EdgeLoop { .. } => true,
            Self::AuthoredPath { closed, .. } => *closed,
        }
    }
}

/// Semantic trim feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimFeature {
    /// Stable operation ID used as face provenance.
    pub operation: OperationId,
    /// Declared edge loop or authored path.
    pub path: TrimPath,
    /// Closed two-dimensional profile points swept along the path.
    pub profile: Vec<[f32; 2]>,
    /// World/local offset applied to path points before sweeping.
    pub offset: [f32; 3],
    /// Offset applied to profile coordinates before frame placement.
    pub profile_offset: [f32; 2],
    /// Up hint used to orient the trim cross-section.
    pub up_hint: [f32; 3],
    /// Roll applied to the trim cross-section in degrees.
    pub roll_degrees: f32,
    /// Generate a start cap for open paths.
    pub start_cap: bool,
    /// Generate an end cap for open paths.
    pub end_cap: bool,
}

/// Build a semantic trim sweep.
pub fn build_trim_feature(
    feature: &TrimFeature,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    let profile = normalize_profile(&feature.profile, "trim")?;
    let points = offset_points(feature.path.points(), feature.offset, "trim")?;
    let closed = feature.path.closed();
    let frames = path_frames(
        &points,
        feature.up_hint,
        feature.roll_degrees,
        closed,
        "trim",
    )?;
    let profile_count = profile.len();
    let ring_count = frames.len();
    let mut builder = MeshBuilder::new();
    let mut rings = Vec::with_capacity(ring_count);
    for frame in &frames {
        let ring = profile
            .iter()
            .map(|point| {
                frame.transform_local([
                    point[0] + feature.profile_offset[0],
                    0.0,
                    point[1] + feature.profile_offset[1],
                ])
            })
            .collect::<Vec<_>>();
        rings.push(builder.add_positions(&ring)?);
    }
    let segment_count = if closed { ring_count } else { ring_count - 1 };
    for segment in 0..segment_count {
        let next_ring = (segment + 1) % ring_count;
        for profile_index in 0..profile_count {
            let next_profile = (profile_index + 1) % profile_count;
            builder.add_face(
                vec![
                    rings[segment][profile_index],
                    rings[next_ring][profile_index],
                    rings[next_ring][next_profile],
                    rings[segment][next_profile],
                ],
                feature.operation,
                TRIM_BODY_REGION,
                SurfaceRole::Trim,
                Some(1),
            );
        }
    }
    if !closed && feature.start_cap {
        builder.add_face(
            rings[0].clone(),
            feature.operation,
            TRIM_START_CAP_REGION,
            SurfaceRole::Cap,
            None,
        );
    }
    if !closed && feature.end_cap {
        let mut cap = rings[ring_count - 1].clone();
        cap.reverse();
        builder.add_face(
            cap,
            feature.operation,
            TRIM_END_CAP_REGION,
            SurfaceRole::Cap,
            None,
        );
    }

    let mut region_specs = vec![(
        TRIM_BODY_REGION,
        "trim_body",
        SurfaceRole::Trim,
        &["trim", "body"][..],
    )];
    if !closed && feature.start_cap {
        region_specs.push((
            TRIM_START_CAP_REGION,
            "trim_start_cap",
            SurfaceRole::Cap,
            &["trim", "cap", "start"][..],
        ));
    }
    if !closed && feature.end_cap {
        region_specs.push((
            TRIM_END_CAP_REGION,
            "trim_end_cap",
            SurfaceRole::Cap,
            &["trim", "cap", "end"][..],
        ));
    }

    let mut mesh = builder.finish(feature.operation)?;
    assign_part_context(&mut mesh, context);
    Ok(generated_part(
        mesh,
        feature_regions_from_vec(region_specs),
        BTreeMap::new(),
        format!(
            "trim:v1:profile={}:rings={}:closed={}:offset={:.6},{:.6},{:.6}:roll={:.6}:caps={},{}",
            profile_count,
            ring_count,
            closed,
            feature.offset[0],
            feature.offset[1],
            feature.offset[2],
            feature.roll_degrees,
            feature.start_cap,
            feature.end_cap
        ),
    ))
}

/// Repetition spacing for ribs and linear/perimeter fastener patterns.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternSpacing {
    /// Distribute the count across the full path span.
    Fit,
    /// Use explicit center-to-center spacing and center the sequence in the span.
    Fixed(f32),
}

/// Rib output mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RibBuildMode {
    /// One shared rib definition with repeated semantic instances.
    SeparateInstances,
    /// One combined generated part containing all rib meshes.
    CombinedPart,
}

/// Cross-section for rib generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RibProfile {
    /// Rectangular plate rib.
    Plate {
        /// Width along local X.
        width: f32,
        /// Height along local Z.
        height: f32,
        /// Thickness along local Y.
        thickness: f32,
    },
    /// Closed two-dimensional profile extruded along local Y.
    ExtrudedProfile {
        /// Closed profile in local X/Z coordinates.
        profile: Vec<[f32; 2]>,
        /// Extrusion depth along local Y.
        depth: f32,
    },
}

/// Repeated rib feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RibFeature {
    /// Stable operation ID used as face provenance.
    pub operation: OperationId,
    /// Start point for the rib span.
    pub start: [f32; 3],
    /// End point for the rib span.
    pub end: [f32; 3],
    /// Number of ribs to create.
    pub count: u32,
    /// Center-to-center spacing behavior.
    pub spacing: PatternSpacing,
    /// Up hint for rib orientation.
    pub up_hint: [f32; 3],
    /// Rib profile.
    pub profile: RibProfile,
    /// Instance or combined-part output mode.
    pub mode: RibBuildMode,
}

/// Shared semantic instance payload for repeated features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticFeatureInstance {
    /// Serializable part instance.
    pub part_instance: PartInstance,
    /// Full semantic placement frame for tools that need orientation.
    pub frame: Frame3,
}

/// Provenance summary for a generated feature output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFeatureProvenance {
    /// Feature family.
    pub feature: String,
    /// Operation that generated the output.
    pub operation: OperationId,
    /// Shared definition IDs.
    pub definition_ids: Vec<PartDefinitionId>,
    /// Generated instance IDs.
    pub instance_ids: Vec<PartInstanceId>,
}

/// Repeated feature output containing definitions, generated parts, and instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticFeatureBuild {
    /// Shared semantic definitions.
    pub definitions: BTreeMap<PartDefinitionId, PartDefinition>,
    /// Generated explicit geometry by definition ID.
    pub generated_parts: BTreeMap<PartDefinitionId, GeneratedPart>,
    /// Semantic instances.
    pub instances: BTreeMap<PartInstanceId, SemanticFeatureInstance>,
    /// Combined generated part for combined-part mode.
    pub combined_part: Option<GeneratedPart>,
    /// Provenance summary.
    pub provenance: SemanticFeatureProvenance,
}
