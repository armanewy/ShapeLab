//! Semantic constructive detail features.
//!
//! These builders create explicit polygon topology for common constructive
//! details while keeping the details separate from their host geometry. They do
//! not perform generic mesh booleans or silently fuse host meshes.

use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{FRAC_PI_2, PI};

use serde::{Deserialize, Serialize};
use shape_asset::{
    Frame3, GeometryRecipe, GeometrySource, OperationId, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole,
    Transform3,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, ElementId, FaceMetadata, PolygonFace,
    PolygonMesh, bounds_from_positions, build_adjacency, compute_topology_signature,
    polygon_mesh_from_faces,
};
use thiserror::Error;

use crate::generators::basic::{
    CapMode as BasicCapMode, CylinderParams, FrustumParams, build_cylinder, build_frustum,
};
use crate::{GeneratedPart, GeneratorContext, ModelingError};

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

/// Build repeated ribs.
pub fn build_rib_feature(
    feature: &RibFeature,
    context: &GeneratorContext,
) -> FeatureResult<SemanticFeatureBuild> {
    let placements = span_frames(
        feature.start,
        feature.end,
        feature.count,
        feature.spacing,
        feature.up_hint,
        "rib",
    )?;
    let prototype = build_rib_prototype(feature, context)?;
    let definition = literal_definition(
        context.part_definition,
        "Rib Feature",
        &prototype,
        &["semantic-feature", "rib"],
    );
    let mut definitions = BTreeMap::new();
    definitions.insert(definition.id, definition);
    let mut generated_parts = BTreeMap::new();
    generated_parts.insert(context.part_definition, prototype.clone());
    let instances = feature_instances(
        "Rib",
        context.part_definition,
        context.part_instance,
        feature.operation,
        &placements,
        &["semantic-feature", "rib"],
    );
    let combined_part = if feature.mode == RibBuildMode::CombinedPart {
        Some(combine_feature_instances(
            &prototype,
            &placements,
            feature.operation,
            context.part_definition,
            context.part_instance,
            "ribs_combined",
        )?)
    } else {
        None
    };
    Ok(SemanticFeatureBuild {
        definitions,
        generated_parts,
        instances,
        combined_part,
        provenance: SemanticFeatureProvenance {
            feature: "rib".to_owned(),
            operation: feature.operation,
            definition_ids: vec![context.part_definition],
            instance_ids: placements
                .iter()
                .enumerate()
                .map(|(index, _)| PartInstanceId(context.part_instance.0 + index as u64))
                .collect(),
        },
    })
}

/// Fastener prototype geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FastenerPrototype {
    /// Cylinder prototype.
    Cylinder {
        /// Radius.
        radius: f32,
        /// Height along local Y.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
    /// Frustum prototype.
    Frustum {
        /// Bottom radius.
        bottom_radius: f32,
        /// Top radius.
        top_radius: f32,
        /// Height along local Y.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
}

/// Fastener placement pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FastenerPlacement {
    /// Linear placement between two points.
    Linear {
        /// Start of the span.
        start: [f32; 3],
        /// End of the span.
        end: [f32; 3],
        /// Number of instances.
        count: u32,
        /// Center-to-center spacing behavior.
        spacing: PatternSpacing,
        /// Up hint for instance orientation.
        up_hint: [f32; 3],
    },
    /// Radial placement around an axis.
    Radial {
        /// Pattern center.
        center: [f32; 3],
        /// Radius from center to each instance.
        radius: f32,
        /// Axis of the radial pattern.
        axis: [f32; 3],
        /// Number of instances.
        count: u32,
        /// Starting angle in degrees.
        start_angle_degrees: f32,
        /// Angular spacing in degrees.
        angular_spacing_degrees: f32,
    },
    /// Perimeter placement along a polyline.
    Perimeter {
        /// Ordered perimeter points.
        points: Vec<[f32; 3]>,
        /// Whether the last point connects to the first.
        closed: bool,
        /// Number of instances.
        count: u32,
        /// Center-to-center spacing behavior.
        spacing: PatternSpacing,
        /// Up hint for instance orientation.
        up_hint: [f32; 3],
    },
}

/// Repeated fastener feature with a shared prototype definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FastenerPattern {
    /// Stable operation ID used as face provenance.
    pub operation: OperationId,
    /// Shared prototype geometry.
    pub prototype: FastenerPrototype,
    /// Placement pattern.
    pub placement: FastenerPlacement,
}

/// Build a repeated fastener pattern.
pub fn build_fastener_pattern(
    feature: &FastenerPattern,
    context: &GeneratorContext,
) -> FeatureResult<SemanticFeatureBuild> {
    let prototype = build_fastener_prototype(feature, context)?;
    let placements = fastener_frames(&feature.placement)?;
    let definition = literal_definition(
        context.part_definition,
        "Fastener Pattern Prototype",
        &prototype,
        &["semantic-feature", "fastener", "shared-prototype"],
    );
    let mut definitions = BTreeMap::new();
    definitions.insert(definition.id, definition);
    let mut generated_parts = BTreeMap::new();
    generated_parts.insert(context.part_definition, prototype);
    let instances = feature_instances(
        "Fastener",
        context.part_definition,
        context.part_instance,
        feature.operation,
        &placements,
        &["semantic-feature", "fastener"],
    );
    Ok(SemanticFeatureBuild {
        definitions,
        generated_parts,
        instances,
        combined_part: None,
        provenance: SemanticFeatureProvenance {
            feature: "fastener".to_owned(),
            operation: feature.operation,
            definition_ids: vec![context.part_definition],
            instance_ids: placements
                .iter()
                .enumerate()
                .map(|(index, _)| PartInstanceId(context.part_instance.0 + index as u64))
                .collect(),
        },
    })
}

fn build_rib_prototype(
    feature: &RibFeature,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    match &feature.profile {
        RibProfile::Plate {
            width,
            height,
            thickness,
        } => {
            let width = positive(*width, "rib", "width")?;
            let height = positive(*height, "rib", "height")?;
            let thickness = positive(*thickness, "rib", "thickness")?;
            build_extruded_profile_part(
                &[
                    [width * 0.5, height * 0.5],
                    [-width * 0.5, height * 0.5],
                    [-width * 0.5, -height * 0.5],
                    [width * 0.5, -height * 0.5],
                ],
                thickness,
                feature.operation,
                context,
                "rib_plate",
            )
        }
        RibProfile::ExtrudedProfile { profile, depth } => {
            let profile = normalize_profile(profile, "rib")?;
            let depth = positive(*depth, "rib", "depth")?;
            build_extruded_profile_part(&profile, depth, feature.operation, context, "rib_profile")
        }
    }
}

fn build_extruded_profile_part(
    profile: &[[f32; 2]],
    depth: f32,
    operation: OperationId,
    context: &GeneratorContext,
    label: &'static str,
) -> FeatureResult<GeneratedPart> {
    let half_depth = depth * 0.5;
    let mut builder = MeshBuilder::new();
    let back = profile
        .iter()
        .map(|point| [point[0], -half_depth, point[1]])
        .collect::<Vec<_>>();
    let front = profile
        .iter()
        .map(|point| [point[0], half_depth, point[1]])
        .collect::<Vec<_>>();
    let back = builder.add_positions(&back)?;
    let front = builder.add_positions(&front)?;
    add_loop_band(
        &mut builder,
        &back,
        &front,
        FaceStyle {
            operation,
            region: RIB_EDGE_REGION,
            role: SurfaceRole::Side,
            smoothing_group: Some(1),
        },
        true,
    );
    add_cap_face(
        &mut builder,
        &front,
        operation,
        RIB_FRONT_REGION,
        SurfaceRole::PrimarySurface,
        true,
    );
    add_cap_face(
        &mut builder,
        &back,
        operation,
        RIB_BACK_REGION,
        SurfaceRole::PrimarySurface,
        false,
    );
    let mut mesh = builder.finish(operation)?;
    assign_part_context(&mut mesh, context);
    Ok(generated_part(
        mesh,
        feature_regions([
            (
                RIB_FRONT_REGION,
                "rib_front",
                SurfaceRole::PrimarySurface,
                &["rib", "front"][..],
            ),
            (
                RIB_BACK_REGION,
                "rib_back",
                SurfaceRole::PrimarySurface,
                &["rib", "back"][..],
            ),
            (
                RIB_EDGE_REGION,
                "rib_edge",
                SurfaceRole::Side,
                &["rib", "edge"][..],
            ),
        ]),
        BTreeMap::new(),
        format!("{label}:v1:profile={}:depth={depth:.6}", profile.len()),
    ))
}

fn build_fastener_prototype(
    feature: &FastenerPattern,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    let mut part = match feature.prototype {
        FastenerPrototype::Cylinder {
            radius,
            height,
            radial_segments,
        } => {
            let params = CylinderParams {
                radius,
                half_height: height * 0.5,
                radial_segments,
                height_segments: 1,
                cap_mode: BasicCapMode::Both,
                top_bevel_radius: 0.0,
                bottom_bevel_radius: 0.0,
                bevel_segments: 0,
            };
            build_cylinder(&params, context)?
        }
        FastenerPrototype::Frustum {
            bottom_radius,
            top_radius,
            height,
            radial_segments,
        } => {
            let params = FrustumParams {
                bottom_radius,
                top_radius,
                half_height: height * 0.5,
                radial_segments,
                height_segments: 1,
                cap_mode: BasicCapMode::Both,
                top_bevel_radius: 0.0,
                bottom_bevel_radius: 0.0,
                bevel_segments: 0,
            };
            build_frustum(&params, context)?
        }
    };
    assign_operation(&mut part.mesh, feature.operation)?;
    part.generator_signature = format!(
        "fastener_pattern:v1:prototype={:?}:operation={}",
        feature.prototype, feature.operation.0
    );
    assign_part_context(&mut part.mesh, context);
    Ok(part)
}

fn fastener_frames(placement: &FastenerPlacement) -> FeatureResult<Vec<Frame3>> {
    match placement {
        FastenerPlacement::Linear {
            start,
            end,
            count,
            spacing,
            up_hint,
        } => span_frames(*start, *end, *count, *spacing, *up_hint, "fastener"),
        FastenerPlacement::Radial {
            center,
            radius,
            axis,
            count,
            start_angle_degrees,
            angular_spacing_degrees,
        } => radial_frames(
            *center,
            *radius,
            *axis,
            *count,
            *start_angle_degrees,
            *angular_spacing_degrees,
        ),
        FastenerPlacement::Perimeter {
            points,
            closed,
            count,
            spacing,
            up_hint,
        } => perimeter_frames(points, *closed, *count, *spacing, *up_hint),
    }
}

fn span_frames(
    start: [f32; 3],
    end: [f32; 3],
    count: u32,
    spacing: PatternSpacing,
    up_hint: [f32; 3],
    feature: &'static str,
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation(feature, "count must be at least one");
    }
    let start = Vec3::from_array(start, feature)?;
    let end = Vec3::from_array(end, feature)?;
    let span = end.sub(start);
    let length = span.length();
    if length <= EPSILON {
        return validation(feature, "span endpoints must not collapse");
    }
    let y = span.scale(1.0 / length);
    let up = Vec3::from_array(up_hint, feature)?.normalized(feature, "up hint must be non-zero")?;
    if y.dot(up).abs() > 0.999 {
        return validation(feature, "up hint must not be parallel to the span");
    }
    let z = up
        .sub(y.scale(up.dot(y)))
        .normalized(feature, "frame is degenerate")?;
    let x = y.cross(z).normalized(feature, "frame is degenerate")?;
    let spacing_mode = spacing;
    let spacing = match spacing_mode {
        PatternSpacing::Fit => {
            if count <= 1 {
                0.0
            } else {
                length / (count - 1) as f32
            }
        }
        PatternSpacing::Fixed(value) => {
            let value = positive(value, feature, "spacing")?;
            let occupied = value * count.saturating_sub(1) as f32;
            if occupied > length + EPSILON {
                return validation(feature, "fixed spacing does not fit between endpoints");
            }
            value
        }
    };
    let start_offset = if count <= 1 {
        length * 0.5
    } else {
        match spacing_mode {
            PatternSpacing::Fit => 0.0,
            PatternSpacing::Fixed(_) => (length - spacing * count.saturating_sub(1) as f32) * 0.5,
        }
    };
    Ok((0..count)
        .map(|index| {
            let origin = start.add(y.scale(start_offset + spacing * index as f32));
            Frame3 {
                origin: origin.to_array(),
                x_axis: x.to_array(),
                y_axis: y.to_array(),
                z_axis: z.to_array(),
            }
        })
        .collect())
}

fn radial_frames(
    center: [f32; 3],
    radius: f32,
    axis: [f32; 3],
    count: u32,
    start_angle_degrees: f32,
    angular_spacing_degrees: f32,
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation("fastener", "count must be at least one");
    }
    let radius = positive(radius, "fastener", "radius")?;
    let center = Vec3::from_array(center, "fastener")?;
    let y = Vec3::from_array(axis, "fastener")?.normalized("fastener", "axis must be non-zero")?;
    let reference = perpendicular(y);
    let z0 = y
        .cross(reference)
        .normalized("fastener", "radial frame is degenerate")?;
    let x0 = y
        .cross(z0)
        .normalized("fastener", "radial frame is degenerate")?;
    let mut frames = Vec::with_capacity(count as usize);
    for index in 0..count {
        let angle = (start_angle_degrees + angular_spacing_degrees * index as f32).to_radians();
        if !angle.is_finite() {
            return validation("fastener", "radial angle must be finite");
        }
        let radial = x0.scale(angle.cos()).add(z0.scale(angle.sin()));
        let tangent = y
            .cross(radial)
            .normalized("fastener", "radial tangent is degenerate")?;
        frames.push(Frame3 {
            origin: center.add(radial.scale(radius)).to_array(),
            x_axis: tangent.to_array(),
            y_axis: y.to_array(),
            z_axis: radial.to_array(),
        });
    }
    Ok(frames)
}

fn perimeter_frames(
    points: &[[f32; 3]],
    closed: bool,
    count: u32,
    spacing: PatternSpacing,
    up_hint: [f32; 3],
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation("fastener", "count must be at least one");
    }
    let samples = sample_polyline(points, closed, count, spacing, "fastener")?;
    let up = Vec3::from_array(up_hint, "fastener")?
        .normalized("fastener", "up hint must be non-zero")?;
    samples
        .into_iter()
        .map(|sample| {
            let y = sample
                .tangent
                .normalized("fastener", "perimeter tangent must be non-zero")?;
            if y.dot(up).abs() > 0.999 {
                return validation(
                    "fastener",
                    "up hint must not be parallel to perimeter tangent",
                );
            }
            let z = up
                .sub(y.scale(up.dot(y)))
                .normalized("fastener", "perimeter frame is degenerate")?;
            let x = y
                .cross(z)
                .normalized("fastener", "perimeter frame is degenerate")?;
            Ok(Frame3 {
                origin: sample.position.to_array(),
                x_axis: x.to_array(),
                y_axis: y.to_array(),
                z_axis: z.to_array(),
            })
        })
        .collect()
}

fn sample_polyline(
    points: &[[f32; 3]],
    closed: bool,
    count: u32,
    spacing: PatternSpacing,
    feature: &'static str,
) -> FeatureResult<Vec<PathSample>> {
    let min_points = if closed { 3 } else { 2 };
    if points.len() < min_points {
        return validation(feature, "path has too few points");
    }
    let points = points
        .iter()
        .map(|point| Vec3::from_array(*point, feature))
        .collect::<FeatureResult<Vec<_>>>()?;
    let segment_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut segments = Vec::with_capacity(segment_count);
    let mut total_length = 0.0;
    for index in 0..segment_count {
        let next = (index + 1) % points.len();
        let delta = points[next].sub(points[index]);
        let length = delta.length();
        if length <= EPSILON {
            return validation(feature, "path contains a collapsed segment");
        }
        segments.push((index, next, length));
        total_length += length;
    }
    let step = match spacing {
        PatternSpacing::Fit => {
            if count <= 1 {
                0.0
            } else if closed {
                total_length / count as f32
            } else {
                total_length / (count - 1) as f32
            }
        }
        PatternSpacing::Fixed(value) => {
            let value = positive(value, feature, "spacing")?;
            let occupied = value * count.saturating_sub(1) as f32;
            if !closed && occupied > total_length + EPSILON {
                return validation(feature, "fixed spacing does not fit along path");
            }
            value
        }
    };
    let start_offset = if count <= 1 {
        total_length * 0.5
    } else if closed || matches!(spacing, PatternSpacing::Fit) {
        0.0
    } else {
        (total_length - step * count.saturating_sub(1) as f32) * 0.5
    };
    let mut samples = Vec::with_capacity(count as usize);
    for index in 0..count {
        let mut distance = start_offset + step * index as f32;
        if closed {
            distance %= total_length;
        }
        let mut cursor = 0.0;
        for (from, to, length) in &segments {
            if distance <= cursor + *length + EPSILON {
                let t = ((distance - cursor) / *length).clamp(0.0, 1.0);
                let tangent = points[*to].sub(points[*from]);
                samples.push(PathSample {
                    position: points[*from].add(tangent.scale(t)),
                    tangent,
                });
                break;
            }
            cursor += *length;
        }
    }
    Ok(samples)
}

fn feature_instances(
    name_prefix: &str,
    definition: PartDefinitionId,
    first_instance: PartInstanceId,
    operation: OperationId,
    frames: &[Frame3],
    tags: &[&str],
) -> BTreeMap<PartInstanceId, SemanticFeatureInstance> {
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let id = PartInstanceId(first_instance.0 + index as u64);
            let part_instance = PartInstance {
                id,
                definition,
                name: format!("{name_prefix} {}", index + 1),
                parent: None,
                local_transform: Transform3 {
                    translation: frame.origin,
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: string_set(tags),
                generated_by: Some(operation),
            };
            (
                id,
                SemanticFeatureInstance {
                    part_instance,
                    frame: frame.clone(),
                },
            )
        })
        .collect()
}

fn combine_feature_instances(
    prototype: &GeneratedPart,
    frames: &[Frame3],
    operation: OperationId,
    definition: PartDefinitionId,
    instance: PartInstanceId,
    label: &'static str,
) -> FeatureResult<GeneratedPart> {
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    let mut metadata = Vec::new();
    for frame in frames {
        let basis = basis_from_frame(frame, label)?;
        let vertex_offset = u32::try_from(positions.len()).map_err(|_| {
            FeatureError::Modeling(ModelingError::InvalidInput(
                "combined feature exceeded u32 index range".to_owned(),
            ))
        })?;
        positions.extend(
            prototype
                .mesh
                .positions
                .iter()
                .map(|position| basis.transform_local(*position)),
        );
        for face in &prototype.mesh.faces {
            faces.push(
                face.vertices
                    .iter()
                    .map(|vertex| vertex + vertex_offset)
                    .collect::<Vec<_>>(),
            );
        }
        metadata.extend(
            prototype
                .mesh
                .face_metadata
                .iter()
                .cloned()
                .map(|mut item| {
                    item.part_definition = Some(definition);
                    item.part_instance = Some(instance);
                    item.operation = Some(operation);
                    item
                }),
        );
    }
    let mut mesh =
        polygon_mesh_from_faces(positions, faces, metadata).map_err(ModelingError::from)?;
    remap_mesh_ids(&mut mesh);
    mesh.edge_metadata = semantic_edge_metadata(&mesh, operation)?;
    Ok(generated_part(
        mesh,
        prototype.regions.clone(),
        BTreeMap::new(),
        format!("{label}:v1:instances={}", frames.len()),
    ))
}

fn literal_definition(
    id: PartDefinitionId,
    name: &str,
    generated: &GeneratedPart,
    tags: &[&str],
) -> PartDefinition {
    PartDefinition {
        id,
        name: name.to_owned(),
        tags: string_set(tags),
        geometry: GeometryRecipe {
            source: GeometrySource::LiteralMesh {
                positions: generated.mesh.positions.clone(),
                faces: generated
                    .mesh
                    .faces
                    .iter()
                    .map(|face| face.vertices.clone())
                    .collect(),
            },
            operations: Vec::new(),
        },
        regions: generated.regions.clone(),
        sockets: generated.sockets.clone(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn assign_operation(mesh: &mut PolygonMesh, operation: OperationId) -> FeatureResult<()> {
    for metadata in &mut mesh.face_metadata {
        metadata.operation = Some(operation);
    }
    mesh.edge_metadata = semantic_edge_metadata(mesh, operation)?;
    Ok(())
}

fn assign_part_context(mesh: &mut PolygonMesh, context: &GeneratorContext) {
    for metadata in &mut mesh.face_metadata {
        metadata.part_definition = Some(context.part_definition);
        metadata.part_instance = Some(context.part_instance);
    }
}

fn resolve_planar_frame(
    host: &FeatureHost,
    target: &PlanarHost,
    feature: &'static str,
) -> FeatureResult<Frame3> {
    match target {
        PlanarHost::Socket(socket) => host
            .sockets
            .get(socket)
            .map(|socket| socket.local_frame.clone())
            .ok_or_else(|| FeatureError::Validation {
                feature,
                message: format!("host socket {} does not exist", socket.0),
            }),
        PlanarHost::SocketName(name) => host
            .sockets
            .values()
            .find(|socket| socket.name == *name)
            .map(|socket| socket.local_frame.clone())
            .ok_or_else(|| FeatureError::Validation {
                feature,
                message: format!("host socket {name} does not exist"),
            }),
        PlanarHost::SurfaceRegion(region) => resolve_region_frame(host, *region, feature),
        PlanarHost::SurfaceRegionName(name) => {
            let region = host
                .regions
                .values()
                .find(|region| region.name == *name)
                .map(|region| region.id)
                .ok_or_else(|| FeatureError::Validation {
                    feature,
                    message: format!("host region {name} does not exist"),
                })?;
            resolve_region_frame(host, region, feature)
        }
    }
}

fn resolve_region_frame(
    host: &FeatureHost,
    region: RegionId,
    feature: &'static str,
) -> FeatureResult<Frame3> {
    if !host.regions.contains_key(&region) {
        return validation(feature, &format!("host region {} does not exist", region.0));
    }
    host.region_frames
        .get(&region)
        .cloned()
        .ok_or_else(|| FeatureError::Validation {
            feature,
            message: format!("host region {} has no planar frame", region.0),
        })
}

fn generated_part(
    mesh: PolygonMesh,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    generator_signature: String,
) -> GeneratedPart {
    GeneratedPart {
        local_bounds: mesh.bounds,
        mesh,
        sockets,
        regions,
        generator_signature,
    }
}

fn feature_regions<const N: usize>(
    specs: [(RegionId, &str, SurfaceRole, &[&str]); N],
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    feature_regions_from_vec(specs.into_iter().collect())
}

fn feature_regions_from_vec(
    specs: Vec<(RegionId, &str, SurfaceRole, &[&str])>,
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    specs
        .into_iter()
        .map(|(id, name, role, tags)| {
            (
                id,
                SurfaceRegionSpec {
                    id,
                    name: name.to_owned(),
                    role,
                    tags: string_set(tags),
                },
            )
        })
        .collect()
}

fn string_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    vertex_ids: Vec<ElementId>,
    faces: Vec<PolygonFace>,
    face_metadata: Vec<FaceMetadata>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            vertex_ids: Vec::new(),
            faces: Vec::new(),
            face_metadata: Vec::new(),
        }
    }

    fn add_ring(
        &mut self,
        points: &[[f32; 2]],
        z: f32,
        basis: &FrameBasis,
    ) -> FeatureResult<Vec<u32>> {
        let positions = points
            .iter()
            .map(|point| basis.transform_local([point[0], point[1], z]))
            .collect::<Vec<_>>();
        self.add_positions(&positions)
    }

    fn add_positions(&mut self, positions: &[[f32; 3]]) -> FeatureResult<Vec<u32>> {
        positions
            .iter()
            .copied()
            .map(|position| self.add_position(position))
            .collect()
    }

    fn add_position(&mut self, position: [f32; 3]) -> FeatureResult<u32> {
        if position.iter().any(|component| !component.is_finite()) {
            return validation("feature", "generated non-finite vertex position");
        }
        let index = u32::try_from(self.positions.len()).map_err(|_| {
            FeatureError::Modeling(ModelingError::InvalidInput(
                "generated mesh exceeded u32 index range".to_owned(),
            ))
        })?;
        self.positions.push(position);
        self.vertex_ids.push(ElementId(u64::from(index)));
        Ok(index)
    }

    fn add_face(
        &mut self,
        vertices: Vec<u32>,
        operation: OperationId,
        region: RegionId,
        role: SurfaceRole,
        smoothing_group: Option<u32>,
    ) {
        if vertices.len() < 3 || has_duplicate_indices(&vertices) {
            return;
        }
        let id = ElementId(self.faces.len() as u64);
        self.faces.push(PolygonFace { id, vertices });
        self.face_metadata.push(FaceMetadata {
            part_definition: None,
            part_instance: None,
            region: Some(region),
            operation: Some(operation),
            smoothing_group,
            surface_role: Some(role),
        });
    }

    fn finish(self, operation: OperationId) -> FeatureResult<PolygonMesh> {
        let bounds = bounds_from_positions(&self.positions).map_err(ModelingError::from)?;
        let mut mesh = PolygonMesh {
            positions: self.positions,
            vertex_ids: self.vertex_ids,
            faces: self.faces,
            face_metadata: self.face_metadata,
            edge_metadata: BTreeMap::new(),
            topology_signature: 0,
            bounds,
        };
        mesh.topology_signature = compute_topology_signature(&mesh.positions, &mesh.faces);
        mesh.edge_metadata = semantic_edge_metadata(&mesh, operation)?;
        Ok(mesh)
    }
}

fn add_loop_band(
    builder: &mut MeshBuilder,
    first: &[u32],
    second: &[u32],
    style: FaceStyle,
    forward: bool,
) {
    for index in 0..first.len() {
        let next = (index + 1) % first.len();
        let mut face = vec![first[index], first[next], second[next], second[index]];
        if !forward {
            face.reverse();
        }
        builder.add_face(
            face,
            style.operation,
            style.region,
            style.role.clone(),
            style.smoothing_group,
        );
    }
}

#[derive(Clone)]
struct FaceStyle {
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
    smoothing_group: Option<u32>,
}

fn add_cap_face(
    builder: &mut MeshBuilder,
    ring: &[u32],
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
    forward: bool,
) {
    let mut face = ring.to_vec();
    if !forward {
        face.reverse();
    }
    builder.add_face(face, operation, region, role, None);
}

fn semantic_edge_metadata(
    mesh: &PolygonMesh,
    operation: OperationId,
) -> Result<BTreeMap<EdgeKey, EdgeMetadata>, ModelingError> {
    let adjacency = build_adjacency(mesh)?;
    let mut metadata = BTreeMap::new();
    for (edge, faces) in adjacency.edge_faces {
        let transition = region_transition(mesh, &faces);
        let open = faces.len() == 1;
        let region_changes = transition.is_some();
        let boundary_role = if open {
            BoundaryRole::OpenBoundary
        } else if region_changes {
            BoundaryRole::Feature
        } else {
            BoundaryRole::Smooth
        };
        let classification = if open || region_changes {
            EdgeClassification::Hard
        } else {
            EdgeClassification::Smooth
        };
        metadata.insert(
            edge,
            EdgeMetadata {
                boundary_role,
                classification,
                seam_candidate: open,
                bevel_eligible: false,
                operation: Some(operation),
                region_transition: transition,
                boundary_loop: None,
            },
        );
    }
    Ok(metadata)
}

fn region_transition(mesh: &PolygonMesh, face_indices: &[usize]) -> Option<(RegionId, RegionId)> {
    if face_indices.len() != 2 {
        return None;
    }
    let first = mesh.face_metadata.get(face_indices[0])?.region?;
    let second = mesh.face_metadata.get(face_indices[1])?.region?;
    if first == second {
        None
    } else if first < second {
        Some((first, second))
    } else {
        Some((second, first))
    }
}

fn remap_mesh_ids(mesh: &mut PolygonMesh) {
    for (index, vertex_id) in mesh.vertex_ids.iter_mut().enumerate() {
        *vertex_id = ElementId(index as u64);
    }
    for (index, face) in mesh.faces.iter_mut().enumerate() {
        face.id = ElementId(index as u64);
    }
}

fn normalize_profile(profile: &[[f32; 2]], feature: &'static str) -> FeatureResult<Vec<[f32; 2]>> {
    if profile.len() < 3 {
        return validation(feature, "profile requires at least three points");
    }
    let mut normalized = profile.to_vec();
    if points2_close(
        normalized[0],
        *normalized.last().expect("profile is not empty"),
    ) {
        normalized.pop();
    }
    if normalized.len() < 3 {
        return validation(feature, "profile requires at least three unique points");
    }
    for point in &normalized {
        if !point[0].is_finite() || !point[1].is_finite() {
            return validation(feature, "profile points must be finite");
        }
    }
    if signed_area(&normalized).abs() <= EPSILON {
        return validation(feature, "profile area must be non-zero");
    }
    Ok(normalized)
}

fn offset_points(
    points: &[[f32; 3]],
    offset: [f32; 3],
    feature: &'static str,
) -> FeatureResult<Vec<[f32; 3]>> {
    if offset.iter().any(|component| !component.is_finite()) {
        return validation(feature, "offset must be finite");
    }
    points
        .iter()
        .map(|point| {
            if point.iter().any(|component| !component.is_finite()) {
                validation(feature, "path points must be finite")
            } else {
                Ok([
                    point[0] + offset[0],
                    point[1] + offset[1],
                    point[2] + offset[2],
                ])
            }
        })
        .collect()
}

fn path_frames(
    points: &[[f32; 3]],
    up_hint: [f32; 3],
    roll_degrees: f32,
    closed: bool,
    feature: &'static str,
) -> FeatureResult<Vec<FrameBasis>> {
    let min_points = if closed { 3 } else { 2 };
    if points.len() < min_points {
        return validation(feature, "path has too few points");
    }
    let points = points
        .iter()
        .map(|point| Vec3::from_array(*point, feature))
        .collect::<FeatureResult<Vec<_>>>()?;
    let up = Vec3::from_array(up_hint, feature)?.normalized(feature, "up hint must be non-zero")?;
    let mut frames = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let tangent = path_tangent(&points, index, closed, feature)?;
        if tangent.dot(up).abs() > 0.999 {
            return validation(feature, "up hint must not be parallel to the path tangent");
        }
        let z = up
            .sub(tangent.scale(up.dot(tangent)))
            .normalized(feature, "path frame is degenerate")?;
        let x = tangent
            .cross(z)
            .normalized(feature, "path frame is degenerate")?;
        frames.push(
            FrameBasis {
                origin: points[index],
                x,
                y: tangent,
                z,
            }
            .rolled(roll_degrees),
        );
    }
    Ok(frames)
}

fn path_tangent(
    points: &[Vec3],
    index: usize,
    closed: bool,
    feature: &'static str,
) -> FeatureResult<Vec3> {
    let raw = if closed {
        let previous = points[(index + points.len() - 1) % points.len()];
        let next = points[(index + 1) % points.len()];
        next.sub(previous)
    } else if index == 0 {
        points[1].sub(points[0])
    } else if index + 1 == points.len() {
        points[index].sub(points[index - 1])
    } else {
        points[index + 1].sub(points[index - 1])
    };
    raw.normalized(feature, "path contains a zero-length tangent")
}

fn basis_from_frame(frame: &Frame3, feature: &'static str) -> FeatureResult<FrameBasis> {
    let origin = Vec3::from_array(frame.origin, feature)?;
    let x = Vec3::from_array(frame.x_axis, feature)?.normalized(feature, "frame x axis is zero")?;
    let y = Vec3::from_array(frame.y_axis, feature)?.normalized(feature, "frame y axis is zero")?;
    let z = Vec3::from_array(frame.z_axis, feature)?.normalized(feature, "frame z axis is zero")?;
    if x.dot(y).abs() > 1.0e-3 || y.dot(z).abs() > 1.0e-3 || z.dot(x).abs() > 1.0e-3 {
        return validation(feature, "frame axes must be orthogonal");
    }
    if x.cross(y).dot(z) < 0.99 {
        return validation(feature, "frame axes must be right-handed");
    }
    Ok(FrameBasis { origin, x, y, z })
}

#[derive(Debug, Copy, Clone)]
struct FrameBasis {
    origin: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl FrameBasis {
    fn transform_local(self, point: [f32; 3]) -> [f32; 3] {
        self.origin
            .add(self.x.scale(point[0]))
            .add(self.y.scale(point[1]))
            .add(self.z.scale(point[2]))
            .to_array()
    }

    fn rolled(self, roll_degrees: f32) -> Self {
        if roll_degrees.abs() <= EPSILON {
            return self;
        }
        let angle = roll_degrees.to_radians();
        Self {
            origin: self.origin,
            x: self.x.rotate_about(self.y, angle),
            y: self.y,
            z: self.z.rotate_about(self.y, angle),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct PathSample {
    position: Vec3,
    tangent: Vec3,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn from_array(value: [f32; 3], feature: &'static str) -> FeatureResult<Self> {
        if value.iter().any(|component| !component.is_finite()) {
            return validation(feature, "3D vectors must be finite");
        }
        Ok(Self {
            x: value[0],
            y: value[1],
            z: value[2],
        })
    }

    fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn scale(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self, feature: &'static str, message: &str) -> FeatureResult<Self> {
        let length = self.length();
        if !length.is_finite() || length <= EPSILON {
            return validation(feature, message);
        }
        Ok(self.scale(1.0 / length))
    }

    fn rotate_about(self, axis: Self, angle: f32) -> Self {
        let sin = angle.sin();
        let cos = angle.cos();
        self.scale(cos)
            .add(axis.cross(self).scale(sin))
            .add(axis.scale(axis.dot(self) * (1.0 - cos)))
    }
}

fn rounded_rect_points(
    half_x: f32,
    half_y: f32,
    radius: f32,
    corner_segments: u32,
) -> Vec<[f32; 2]> {
    if radius <= EPSILON {
        return vec![
            [half_x, half_y],
            [-half_x, half_y],
            [-half_x, -half_y],
            [half_x, -half_y],
        ];
    }
    let centers = [
        [half_x - radius, half_y - radius],
        [-half_x + radius, half_y - radius],
        [-half_x + radius, -half_y + radius],
        [half_x - radius, -half_y + radius],
    ];
    let starts = [0.0, FRAC_PI_2, PI, PI + FRAC_PI_2];
    let mut points = Vec::new();
    for (center, start) in centers.into_iter().zip(starts) {
        for index in 0..=corner_segments {
            let t = index as f32 / corner_segments as f32;
            let angle = start + t * FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            points.push([center[0] + radius * cos, center[1] + radius * sin]);
        }
    }
    points
}

fn perpendicular(axis: Vec3) -> Vec3 {
    let candidate = if axis.x.abs() < 0.9 {
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    };
    axis.cross(candidate)
        .normalized("fastener", "radial reference is degenerate")
        .expect("candidate should not be parallel")
}

fn positive(value: f32, feature: &'static str, label: &str) -> FeatureResult<f32> {
    if value.is_finite() && value > EPSILON {
        Ok(value)
    } else {
        validation(feature, &format!("{label} must be finite and positive"))
    }
}

fn non_negative(value: f32, feature: &'static str, label: &str) -> FeatureResult<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        validation(feature, &format!("{label} must be finite and non-negative"))
    }
}

fn validation<T>(feature: &'static str, message: &str) -> FeatureResult<T> {
    Err(FeatureError::Validation {
        feature,
        message: message.to_owned(),
    })
}

fn points2_close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() <= EPSILON && (a[1] - b[1]).abs() <= EPSILON
}

fn signed_area(profile: &[[f32; 2]]) -> f32 {
    let mut area = 0.0;
    for index in 0..profile.len() {
        let current = profile[index];
        let next = profile[(index + 1) % profile.len()];
        area += current[0] * next[1] - next[0] * current[1];
    }
    area * 0.5
}

fn has_duplicate_indices(vertices: &[u32]) -> bool {
    let mut seen = BTreeSet::new();
    vertices.iter().any(|vertex| !seen.insert(*vertex))
}
