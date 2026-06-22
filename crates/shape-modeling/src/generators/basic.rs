use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{FRAC_PI_2, PI};

use shape_asset::{
    BoundaryLoopId, CutEdgeTreatment, Frame3, GeometrySource, ModelingOperationSpec, OperationId,
    PartDefinition, PlanarCutFace, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, ElementId, FaceMetadata, PolygonFace,
    PolygonMesh, bounds_from_positions, compute_topology_signature,
};

use crate::{GeneratedPart, GeneratorContext, ModelingError};

const EPSILON: f32 = 1.0e-6;

const ROUNDED_PRIMARY_REGION: RegionId = RegionId(1);
const ROUNDED_BEVEL_REGION: RegionId = RegionId(2);
const ROUNDED_CORNER_REGION: RegionId = RegionId(3);

const CYLINDER_SIDE_REGION: RegionId = RegionId(1);
const CYLINDER_TOP_CAP_REGION: RegionId = RegionId(2);
const CYLINDER_BOTTOM_CAP_REGION: RegionId = RegionId(3);
const CYLINDER_TOP_BEVEL_REGION: RegionId = RegionId(4);
const CYLINDER_BOTTOM_BEVEL_REGION: RegionId = RegionId(5);

const PLATE_FRONT_REGION: RegionId = RegionId(1);
const PLATE_BACK_REGION: RegionId = RegionId(2);
const PLATE_SIDE_REGION: RegionId = RegionId(3);
const PLATE_BEVEL_REGION: RegionId = RegionId(4);
const BOUNDARY_BEVEL_PROFILE_MIN: f32 = 0.05;
const BOUNDARY_BEVEL_PROFILE_MAX: f32 = 8.0;

const SOCKET_TOP: SocketId = SocketId(1);
const SOCKET_BOTTOM: SocketId = SocketId(2);
const SOCKET_AXIS: SocketId = SocketId(3);

type SocketTemplate = (
    SocketId,
    &'static str,
    [f32; 3],
    [f32; 3],
    [f32; 3],
    [f32; 3],
);

/// Six-sided rounded-box face inclusion mask.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FaceMask {
    /// Positive X face.
    pub positive_x: bool,
    /// Negative X face.
    pub negative_x: bool,
    /// Positive Y face.
    pub positive_y: bool,
    /// Negative Y face.
    pub negative_y: bool,
    /// Positive Z face.
    pub positive_z: bool,
    /// Negative Z face.
    pub negative_z: bool,
}

impl FaceMask {
    /// Return a closed six-face mask.
    #[must_use]
    pub fn all() -> Self {
        Self {
            positive_x: true,
            negative_x: true,
            positive_y: true,
            negative_y: true,
            positive_z: true,
            negative_z: true,
        }
    }

    /// Return true when the given side is enabled.
    #[must_use]
    fn includes(self, side: FaceSide) -> bool {
        match side {
            FaceSide::PositiveX => self.positive_x,
            FaceSide::NegativeX => self.negative_x,
            FaceSide::PositiveY => self.positive_y,
            FaceSide::NegativeY => self.negative_y,
            FaceSide::PositiveZ => self.positive_z,
            FaceSide::NegativeZ => self.negative_z,
        }
    }
}

impl Default for FaceMask {
    fn default() -> Self {
        Self::all()
    }
}

/// Cap inclusion mode for cylinders and frusta.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CapMode {
    /// No caps.
    None,
    /// Top cap only.
    Top,
    /// Bottom cap only.
    Bottom,
    /// Top and bottom caps.
    Both,
}

impl CapMode {
    #[must_use]
    fn has_top(self) -> bool {
        matches!(self, Self::Top | Self::Both)
    }

    #[must_use]
    fn has_bottom(self) -> bool {
        matches!(self, Self::Bottom | Self::Both)
    }
}

/// Explicit rounded-box generator parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundedBoxParams {
    /// Half extents along local X, Y, and Z.
    pub half_extents: [f32; 3],
    /// Requested bevel radius. Clamped to valid half extents.
    pub bevel_radius: f32,
    /// Number of samples across bevel bands.
    pub bevel_segments: u32,
    /// Number of subdivisions across each primary face axis.
    pub face_subdivisions: u32,
    /// Closed/open side mask.
    pub face_mask: FaceMask,
}

/// Explicit cylinder generator parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct CylinderParams {
    /// Cylinder radius.
    pub radius: f32,
    /// Half height along local Y.
    pub half_height: f32,
    /// Radial segment count.
    pub radial_segments: u32,
    /// Side height segment count.
    pub height_segments: u32,
    /// Cap inclusion mode.
    pub cap_mode: CapMode,
    /// Top bevel radius.
    pub top_bevel_radius: f32,
    /// Bottom bevel radius.
    pub bottom_bevel_radius: f32,
    /// Bevel segment count.
    pub bevel_segments: u32,
}

/// Explicit frustum generator parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct FrustumParams {
    /// Bottom radius.
    pub bottom_radius: f32,
    /// Top radius.
    pub top_radius: f32,
    /// Half height along local Y.
    pub half_height: f32,
    /// Radial segment count.
    pub radial_segments: u32,
    /// Side height segment count.
    pub height_segments: u32,
    /// Cap inclusion mode.
    pub cap_mode: CapMode,
    /// Top bevel radius.
    pub top_bevel_radius: f32,
    /// Bottom bevel radius.
    pub bottom_bevel_radius: f32,
    /// Bevel segment count.
    pub bevel_segments: u32,
}

/// Explicit rectangular plate generator parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateParams {
    /// Width along local X.
    pub width: f32,
    /// Height along local Z.
    pub height: f32,
    /// Thickness along local Y.
    pub thickness: f32,
    /// Rounded corner radius in the X/Z plane.
    pub corner_radius: f32,
    /// Corner arc segment count.
    pub corner_segments: u32,
    /// Symmetric front/back bevel amount.
    pub front_back_bevel: f32,
}

/// Generate a rounded box from a `PartDefinition` using schema-1 fields.
pub fn generate_rounded_box(
    definition: &PartDefinition,
    context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let GeometrySource::RoundedBox {
        half_extents,
        radius,
    } = definition.geometry.source
    else {
        return Err(ModelingError::InvalidInput(
            "rounded-box generator received a different geometry source".to_owned(),
        ));
    };
    let (bevel_radius, bevel_segments) = bevel_profile(definition, radius, 3);
    let params = RoundedBoxParams {
        half_extents,
        bevel_radius,
        bevel_segments,
        face_subdivisions: 1,
        face_mask: FaceMask::all(),
    };
    build_rounded_box(&params, context)
}

/// Generate a cylinder from a `PartDefinition` using schema-1 fields.
pub fn generate_cylinder(
    definition: &PartDefinition,
    context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let GeometrySource::Cylinder {
        radius,
        height,
        radial_segments,
    } = definition.geometry.source
    else {
        return Err(ModelingError::InvalidInput(
            "cylinder generator received a different geometry source".to_owned(),
        ));
    };
    let (bevel_radius, bevel_segments) = bevel_profile(definition, 0.0, 0);
    let params = CylinderParams {
        radius,
        half_height: height * 0.5,
        radial_segments,
        height_segments: 1,
        cap_mode: CapMode::Both,
        top_bevel_radius: bevel_radius,
        bottom_bevel_radius: bevel_radius,
        bevel_segments,
    };
    build_cylinder(&params, context)
}

/// Generate a frustum from a `PartDefinition` using schema-1 fields.
pub fn generate_frustum(
    definition: &PartDefinition,
    context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let GeometrySource::Frustum {
        bottom_radius,
        top_radius,
        height,
        radial_segments,
    } = definition.geometry.source
    else {
        return Err(ModelingError::InvalidInput(
            "frustum generator received a different geometry source".to_owned(),
        ));
    };
    let (bevel_radius, bevel_segments) = bevel_profile(definition, 0.0, 0);
    let params = FrustumParams {
        bottom_radius,
        top_radius,
        half_height: height * 0.5,
        radial_segments,
        height_segments: 1,
        cap_mode: CapMode::Both,
        top_bevel_radius: bevel_radius,
        bottom_bevel_radius: bevel_radius,
        bevel_segments,
    };
    build_frustum(&params, context)
}

/// Generate a plate from a `PartDefinition` using schema-1 fields.
pub fn generate_plate(
    definition: &PartDefinition,
    context: &mut GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let GeometrySource::Plate { size, thickness } = definition.geometry.source else {
        return Err(ModelingError::InvalidInput(
            "plate generator received a different geometry source".to_owned(),
        ));
    };
    let cut_operations = cut_operations(definition);
    let boundary_loop_bevels = boundary_loop_bevel_operations(definition)?;
    if let Some(operation) = cut_operations.first().copied() {
        let (bevel_radius, _) = bevel_profile(definition, 0.0, 0);
        if bevel_radius > EPSILON {
            return Err(ModelingError::UnsupportedOperation {
                operation: operation.operation_id(),
                reason: "plate cuts do not yet combine with bevel profiles".to_owned(),
            });
        }
        if cut_operations.len() > 1 {
            return build_multi_cut_plate(
                size,
                thickness,
                &cut_operations,
                &boundary_loop_bevels,
                context,
            );
        }
        return build_cut_plate(size, thickness, operation, &boundary_loop_bevels, context);
    }
    if let Some(bevel) = boundary_loop_bevels.first() {
        return Err(ModelingError::UnsupportedOperation {
            operation: bevel.operation,
            reason:
                "BevelBoundaryLoop requires a supported cut operation in the same Plate definition"
                    .to_owned(),
        });
    }
    let (bevel_radius, bevel_segments) = bevel_profile(definition, 0.0, 0);
    let params = PlateParams {
        width: size[0],
        height: size[1],
        thickness,
        corner_radius: 0.0,
        corner_segments: bevel_segments.max(1),
        front_back_bevel: bevel_radius,
    };
    build_plate(&params, context)
}

/// Build a rounded-box mesh with explicit topology controls.
pub fn build_rounded_box(
    params: &RoundedBoxParams,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let half = positive_triplet(params.half_extents, "rounded_box.half_extents")?;
    let requested_radius = finite_non_negative(params.bevel_radius, "rounded_box.bevel_radius")?;
    let radius = requested_radius.min(half[0].min(half[1]).min(half[2]));
    let bevel_segments = if radius > EPSILON {
        params.bevel_segments.max(1)
    } else {
        0
    };
    let face_subdivisions = params.face_subdivisions.max(1);
    let inner = [
        (half[0] - radius).max(0.0),
        (half[1] - radius).max(0.0),
        (half[2] - radius).max(0.0),
    ];
    let axis_samples = [
        axis_samples(half[0], inner[0], radius, bevel_segments, face_subdivisions),
        axis_samples(half[1], inner[1], radius, bevel_segments, face_subdivisions),
        axis_samples(half[2], inner[2], radius, bevel_segments, face_subdivisions),
    ];
    let mut builder = MeshBuilder::new();

    for side in FaceSide::ALL {
        if !params.face_mask.includes(side) {
            continue;
        }
        let [u_axis, v_axis] = side.tangent_axes();
        let u_samples = &axis_samples[u_axis];
        let v_samples = &axis_samples[v_axis];
        for u in 0..u_samples.len() - 1 {
            for v in 0..v_samples.len() - 1 {
                let corners = [
                    rounded_box_position(side, u_samples[u], v_samples[v], half, inner, radius),
                    rounded_box_position(side, u_samples[u + 1], v_samples[v], half, inner, radius),
                    rounded_box_position(
                        side,
                        u_samples[u + 1],
                        v_samples[v + 1],
                        half,
                        inner,
                        radius,
                    ),
                    rounded_box_position(side, u_samples[u], v_samples[v + 1], half, inner, radius),
                ];
                let vertices = builder.add_vertices(&corners)?;
                let region = rounded_box_region(
                    [u_axis, v_axis],
                    [
                        (u_samples[u] + u_samples[u + 1]) * 0.5,
                        (v_samples[v] + v_samples[v + 1]) * 0.5,
                    ],
                    inner,
                    radius,
                );
                builder.add_face(vertices, rounded_box_metadata(context, region));
            }
        }
    }

    let mesh = builder.finish()?;
    let regions = rounded_box_regions();
    let sockets = rounded_box_sockets(half);
    Ok(part(
        mesh,
        regions,
        sockets,
        format!(
            "rounded_box:h={:.6},{:.6},{:.6}:r={:.6}:bs={}:fs={}:mask={:?}",
            half[0], half[1], half[2], radius, bevel_segments, face_subdivisions, params.face_mask
        ),
    ))
}

/// Build a cylinder mesh with explicit topology controls.
pub fn build_cylinder(
    params: &CylinderParams,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let frustum = FrustumParams {
        bottom_radius: params.radius,
        top_radius: params.radius,
        half_height: params.half_height,
        radial_segments: params.radial_segments,
        height_segments: params.height_segments,
        cap_mode: params.cap_mode,
        top_bevel_radius: params.top_bevel_radius,
        bottom_bevel_radius: params.bottom_bevel_radius,
        bevel_segments: params.bevel_segments,
    };
    build_frustum_like(&frustum, context, "cylinder")
}

/// Build a frustum mesh with explicit topology controls.
pub fn build_frustum(
    params: &FrustumParams,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    build_frustum_like(params, context, "frustum")
}

/// Build a rounded rectangular plate mesh with explicit topology controls.
pub fn build_plate(
    params: &PlateParams,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let width = finite_positive(params.width, "plate.width")?;
    let height = finite_positive(params.height, "plate.height")?;
    let thickness = finite_positive(params.thickness, "plate.thickness")?;
    let half_x = width * 0.5;
    let half_z = height * 0.5;
    let half_y = thickness * 0.5;
    let corner_radius =
        finite_non_negative(params.corner_radius, "plate.corner_radius")?.min(half_x.min(half_z));
    let mut bevel =
        finite_non_negative(params.front_back_bevel, "plate.front_back_bevel")?.min(half_y);
    if corner_radius > EPSILON {
        bevel = bevel.min(corner_radius * 0.5);
    }
    bevel = bevel.min(half_x * 0.5).min(half_z * 0.5);
    let corner_segments = if corner_radius > EPSILON {
        params.corner_segments.max(1)
    } else {
        1
    };

    let outer = rounded_rect_points(half_x, half_z, corner_radius, corner_segments);
    let inner_half_x = (half_x - bevel).max(EPSILON);
    let inner_half_z = (half_z - bevel).max(EPSILON);
    let inner_radius = if corner_radius > EPSILON {
        (corner_radius - bevel).max(EPSILON)
    } else {
        0.0
    };
    let inner = rounded_rect_points(inner_half_x, inner_half_z, inner_radius, corner_segments);
    let mut builder = MeshBuilder::new();

    let back_face = if bevel > EPSILON { &inner } else { &outer };
    let front_face = if bevel > EPSILON { &inner } else { &outer };
    let back_face_ring = builder.add_plate_ring(-half_y, back_face)?;
    let front_face_ring = if bevel > EPSILON {
        let back_outer_ring = builder.add_plate_ring(-half_y + bevel, &outer)?;
        let front_outer_ring = builder.add_plate_ring(half_y - bevel, &outer)?;
        let front_inner_ring = builder.add_plate_ring(half_y, front_face)?;
        add_plate_band(
            &mut builder,
            &back_face_ring,
            &back_outer_ring,
            context,
            PLATE_BEVEL_REGION,
        );
        add_plate_band(
            &mut builder,
            &back_outer_ring,
            &front_outer_ring,
            context,
            PLATE_SIDE_REGION,
        );
        add_plate_band(
            &mut builder,
            &front_outer_ring,
            &front_inner_ring,
            context,
            PLATE_BEVEL_REGION,
        );
        front_inner_ring
    } else {
        let front_ring = builder.add_plate_ring(half_y, front_face)?;
        add_plate_band(
            &mut builder,
            &back_face_ring,
            &front_ring,
            context,
            PLATE_SIDE_REGION,
        );
        front_ring
    };
    let mut front_cap = front_face_ring.clone();
    front_cap.reverse();
    builder.add_face(front_cap, plate_metadata(context, PLATE_FRONT_REGION));
    builder.add_face(back_face_ring, plate_metadata(context, PLATE_BACK_REGION));

    let mesh = builder.finish()?;
    let regions = plate_regions();
    let sockets = plate_sockets(half_y);
    Ok(part(
        mesh,
        regions,
        sockets,
        format!(
            "plate:w={:.6}:h={:.6}:t={:.6}:r={:.6}:cs={}:b={:.6}",
            width, height, thickness, corner_radius, corner_segments, bevel
        ),
    ))
}

fn cut_operations(definition: &PartDefinition) -> Vec<&ModelingOperationSpec> {
    definition
        .geometry
        .operations
        .iter()
        .filter(|operation| {
            matches!(
                operation,
                ModelingOperationSpec::RecessedPanelCut { .. }
                    | ModelingOperationSpec::RectangularThroughCut { .. }
                    | ModelingOperationSpec::CircularThroughCut { .. }
            )
        })
        .collect()
}

fn boundary_loop_bevel_operations(
    definition: &PartDefinition,
) -> Result<Vec<BoundaryLoopBevelPlan>, ModelingError> {
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|operation| match operation {
            ModelingOperationSpec::BevelBoundaryLoop { .. } => {
                Some(BoundaryLoopBevelPlan::from_operation(operation))
            }
            _ => None,
        })
        .collect()
}

fn build_cut_plate(
    size: [f32; 2],
    thickness: f32,
    operation: &ModelingOperationSpec,
    bevels: &[BoundaryLoopBevelPlan],
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let width = finite_positive(size[0], "plate.width")?;
    let height = finite_positive(size[1], "plate.height")?;
    let thickness = finite_positive(thickness, "plate.thickness")?;
    let half_x = width * 0.5;
    let half_z = height * 0.5;
    let half_y = thickness * 0.5;
    let mut cut = PlateCutPlan::from_operation(operation, half_x, half_z, thickness)?;
    apply_boundary_loop_bevels(std::slice::from_mut(&mut cut), bevels, thickness)?;
    let face_sign = planar_plate_face_sign(cut.face, cut.operation)?;
    let (entry_region, opposite_region) = plate_cut_face_regions(cut.face, cut.operation)?;
    if cut.target_region != entry_region || cut.outer_region != entry_region {
        return Err(ModelingError::InvalidInput(
            "cut target region and outer region must match the selected plate face".to_owned(),
        ));
    }
    let outside_y = face_sign * half_y;
    let opposite_y = -outside_y;
    let opposite_normal = [0.0, -face_sign, 0.0];
    let outside_normal = [0.0, face_sign, 0.0];
    let host_points = rect_points(-half_x, half_x, -half_z, half_z);
    let frame_ring = cut.frame_points.clone();

    let mut builder = MeshBuilder::new();
    let outside_host = builder.add_plate_ring(outside_y, &host_points)?;
    let opposite_host = builder.add_plate_ring(opposite_y, &host_points)?;
    add_plate_shell_sides(
        &mut builder,
        &opposite_host,
        &outside_host,
        &host_points,
        context,
        Some(cut.operation),
    );

    match cut.kind {
        PlateCutKind::Recessed {
            depth,
            floor_region,
        } => {
            let floor_y = outside_y - face_sign * depth;
            let outside_frame_ring = builder.add_plate_ring(outside_y, &frame_ring)?;
            let outside_rim_ring = if cut.has_host_surface_band {
                builder.add_plate_ring(outside_y, &cut.rim_points)?
            } else {
                outside_frame_ring.clone()
            };
            let entry_surface_points = cut
                .entry_bevel
                .as_ref()
                .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, bevel.width))
                .unwrap_or_else(|| cut.inner_points.clone());
            let (outside_inner, wall_top_ring) = if let Some(bevel) = &cut.entry_bevel {
                add_boundary_loop_bevel_band(
                    &mut builder,
                    outside_y,
                    &entry_surface_points,
                    outside_y - face_sign * bevel.width,
                    &cut.inner_points,
                    outside_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_plate_ring(outside_y, &cut.inner_points)?;
                (ring.clone(), ring)
            };
            let floor_surface_points = cut
                .secondary_bevel
                .as_ref()
                .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, -bevel.width))
                .unwrap_or_else(|| cut.inner_points.clone());
            let (wall_bottom_ring, floor_inner) = if let Some(bevel) = &cut.secondary_bevel {
                add_boundary_loop_bevel_band(
                    &mut builder,
                    floor_y + face_sign * bevel.width,
                    &cut.inner_points,
                    floor_y,
                    &floor_surface_points,
                    outside_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_plate_ring(floor_y, &cut.inner_points)?;
                (ring.clone(), ring)
            };

            add_host_to_ring_cap(
                &mut builder,
                &outside_host,
                &host_points,
                &outside_frame_ring,
                &frame_ring,
                cut.frame,
                outside_normal,
                context,
                cut.operation,
                cut.outer_region,
                SurfaceRole::PrimarySurface,
            );
            if cut.has_host_surface_band {
                add_matched_ring_band(
                    &mut builder,
                    &outside_frame_ring,
                    &outside_rim_ring,
                    &frame_ring,
                    &cut.rim_points,
                    outside_normal,
                    context,
                    cut.operation,
                    cut.outer_region,
                    SurfaceRole::PrimarySurface,
                );
            }
            add_matched_ring_band(
                &mut builder,
                &outside_rim_ring,
                &outside_inner,
                &cut.rim_points,
                &entry_surface_points,
                outside_normal,
                context,
                cut.operation,
                cut.rim_region,
                SurfaceRole::Rim,
            );
            add_cut_wall_band(
                &mut builder,
                &wall_top_ring,
                &wall_bottom_ring,
                &cut.inner_points,
                cut.center,
                context,
                cut.operation,
                cut.wall_region,
            );
            add_cap_oriented(
                &mut builder,
                floor_inner.clone(),
                outside_normal,
                cut_metadata(
                    context,
                    floor_region,
                    SurfaceRole::Interior,
                    cut.operation,
                    None,
                ),
            );
            add_cap_oriented(
                &mut builder,
                opposite_host,
                opposite_normal,
                cut_metadata(
                    context,
                    opposite_region,
                    SurfaceRole::PrimarySurface,
                    cut.operation,
                    None,
                ),
            );

            let mut mesh = builder.finish()?;
            mark_cut_or_bevel_boundary_loop(
                &mut mesh,
                &outside_inner,
                &wall_top_ring,
                cut.operation,
                cut.entry_loop,
                cut.edge_treatment,
                cut.entry_bevel.as_ref(),
            );
            mark_cut_or_bevel_boundary_loop(
                &mut mesh,
                &wall_bottom_ring,
                &floor_inner,
                cut.operation,
                cut.secondary_loop,
                cut.edge_treatment,
                cut.secondary_bevel.as_ref(),
            );
            let mut regions = plate_regions();
            insert_cut_region(&mut regions, cut.rim_region, "cut_rim", SurfaceRole::Rim);
            insert_cut_region(
                &mut regions,
                cut.wall_region,
                "cut_wall",
                SurfaceRole::CutWall,
            );
            insert_cut_region(
                &mut regions,
                floor_region,
                "recess_floor",
                SurfaceRole::Interior,
            );
            insert_boundary_bevel_region(&mut regions, cut.entry_bevel.as_ref());
            insert_boundary_bevel_region(&mut regions, cut.secondary_bevel.as_ref());
            Ok(part(
                mesh,
                regions,
                plate_sockets(half_y),
                format!(
                    "plate_cut:recessed:w={:.6}:h={:.6}:t={:.6}:op={}:face={:?}:cx={:.6}:cz={:.6}:n={}:depth={:.6}:rim={:.6}:cs={}:frame={:.6},{:.6},{:.6},{:.6}",
                    width,
                    height,
                    thickness,
                    cut.operation.0,
                    cut.face,
                    cut.center[0],
                    cut.center[1],
                    cut.inner_points.len(),
                    depth,
                    cut.rim_width,
                    cut.corner_segments,
                    cut.frame.min_x,
                    cut.frame.max_x,
                    cut.frame.min_z,
                    cut.frame.max_z
                ),
            ))
        }
        PlateCutKind::Through => {
            let outside_frame_ring = builder.add_plate_ring(outside_y, &frame_ring)?;
            let outside_rim_ring = if cut.has_host_surface_band {
                builder.add_plate_ring(outside_y, &cut.rim_points)?
            } else {
                outside_frame_ring.clone()
            };
            let opposite_frame_ring = builder.add_plate_ring(opposite_y, &frame_ring)?;
            let opposite_rim_ring = if cut.has_host_surface_band {
                builder.add_plate_ring(opposite_y, &cut.rim_points)?
            } else {
                opposite_frame_ring.clone()
            };
            let entry_surface_points = cut
                .entry_bevel
                .as_ref()
                .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, bevel.width))
                .unwrap_or_else(|| cut.inner_points.clone());
            let (outside_inner, wall_front_ring) = if let Some(bevel) = &cut.entry_bevel {
                add_boundary_loop_bevel_band(
                    &mut builder,
                    outside_y,
                    &entry_surface_points,
                    outside_y - face_sign * bevel.width,
                    &cut.inner_points,
                    outside_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_plate_ring(outside_y, &cut.inner_points)?;
                (ring.clone(), ring)
            };
            let exit_surface_points = cut
                .secondary_bevel
                .as_ref()
                .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, bevel.width))
                .unwrap_or_else(|| cut.inner_points.clone());
            let (opposite_inner, wall_back_ring) = if let Some(bevel) = &cut.secondary_bevel {
                add_boundary_loop_bevel_band(
                    &mut builder,
                    opposite_y,
                    &exit_surface_points,
                    opposite_y + face_sign * bevel.width,
                    &cut.inner_points,
                    opposite_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_plate_ring(opposite_y, &cut.inner_points)?;
                (ring.clone(), ring)
            };

            add_host_to_ring_cap(
                &mut builder,
                &outside_host,
                &host_points,
                &outside_frame_ring,
                &frame_ring,
                cut.frame,
                outside_normal,
                context,
                cut.operation,
                cut.outer_region,
                SurfaceRole::PrimarySurface,
            );
            if cut.has_host_surface_band {
                add_matched_ring_band(
                    &mut builder,
                    &outside_frame_ring,
                    &outside_rim_ring,
                    &frame_ring,
                    &cut.rim_points,
                    outside_normal,
                    context,
                    cut.operation,
                    cut.outer_region,
                    SurfaceRole::PrimarySurface,
                );
            }
            add_matched_ring_band(
                &mut builder,
                &outside_rim_ring,
                &outside_inner,
                &cut.rim_points,
                &entry_surface_points,
                outside_normal,
                context,
                cut.operation,
                cut.rim_region,
                SurfaceRole::Rim,
            );
            add_host_to_ring_cap(
                &mut builder,
                &opposite_host,
                &host_points,
                &opposite_frame_ring,
                &frame_ring,
                cut.frame,
                opposite_normal,
                context,
                cut.operation,
                opposite_region,
                SurfaceRole::PrimarySurface,
            );
            if cut.has_host_surface_band {
                add_matched_ring_band(
                    &mut builder,
                    &opposite_frame_ring,
                    &opposite_rim_ring,
                    &frame_ring,
                    &cut.rim_points,
                    opposite_normal,
                    context,
                    cut.operation,
                    opposite_region,
                    SurfaceRole::PrimarySurface,
                );
            }
            add_matched_ring_band(
                &mut builder,
                &opposite_rim_ring,
                &opposite_inner,
                &cut.rim_points,
                &exit_surface_points,
                opposite_normal,
                context,
                cut.operation,
                cut.rim_region,
                SurfaceRole::Rim,
            );
            add_cut_wall_band(
                &mut builder,
                &wall_front_ring,
                &wall_back_ring,
                &cut.inner_points,
                cut.center,
                context,
                cut.operation,
                cut.wall_region,
            );

            let mut mesh = builder.finish()?;
            mark_cut_or_bevel_boundary_loop(
                &mut mesh,
                &outside_inner,
                &wall_front_ring,
                cut.operation,
                cut.entry_loop,
                cut.edge_treatment,
                cut.entry_bevel.as_ref(),
            );
            mark_cut_or_bevel_boundary_loop(
                &mut mesh,
                &opposite_inner,
                &wall_back_ring,
                cut.operation,
                cut.secondary_loop,
                cut.edge_treatment,
                cut.secondary_bevel.as_ref(),
            );
            let mut regions = plate_regions();
            insert_cut_region(&mut regions, cut.rim_region, "cut_rim", SurfaceRole::Rim);
            insert_cut_region(
                &mut regions,
                cut.wall_region,
                "cut_wall",
                SurfaceRole::CutWall,
            );
            insert_boundary_bevel_region(&mut regions, cut.entry_bevel.as_ref());
            insert_boundary_bevel_region(&mut regions, cut.secondary_bevel.as_ref());
            Ok(part(
                mesh,
                regions,
                plate_sockets(half_y),
                format!(
                    "plate_cut:through:w={:.6}:h={:.6}:t={:.6}:op={}:face={:?}:cx={:.6}:cz={:.6}:n={}:rim={:.6}:cs={}:frame={:.6},{:.6},{:.6},{:.6}",
                    width,
                    height,
                    thickness,
                    cut.operation.0,
                    cut.face,
                    cut.center[0],
                    cut.center[1],
                    cut.inner_points.len(),
                    cut.rim_width,
                    cut.corner_segments,
                    cut.frame.min_x,
                    cut.frame.max_x,
                    cut.frame.min_z,
                    cut.frame.max_z
                ),
            ))
        }
    }
}

fn build_multi_cut_plate(
    size: [f32; 2],
    thickness: f32,
    operations: &[&ModelingOperationSpec],
    bevels: &[BoundaryLoopBevelPlan],
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let width = finite_positive(size[0], "plate.width")?;
    let height = finite_positive(size[1], "plate.height")?;
    let thickness = finite_positive(thickness, "plate.thickness")?;
    let half_x = width * 0.5;
    let half_z = height * 0.5;
    let half_y = thickness * 0.5;
    let mut cuts = operations
        .iter()
        .map(|operation| PlateCutPlan::from_operation(operation, half_x, half_z, thickness))
        .collect::<Result<Vec<_>, _>>()?;
    apply_boundary_loop_bevels(&mut cuts, bevels, thickness)?;

    let first = cuts.first().ok_or_else(|| {
        ModelingError::InvalidInput(
            "multi-cut plate generation requires at least one cut".to_owned(),
        )
    })?;
    let face = first.face;
    let face_sign = planar_plate_face_sign(face, first.operation)?;
    let (entry_region, opposite_region) = plate_cut_face_regions(face, first.operation)?;
    for cut in &cuts {
        if cut.face != face {
            return Err(ModelingError::UnsupportedOperation {
                operation: cut.operation,
                reason: "multi-cut plate composition currently supports one target face per part"
                    .to_owned(),
            });
        }
        if cut.target_region != entry_region || cut.outer_region != entry_region {
            return Err(ModelingError::InvalidInput(
                "cut target region and outer region must match the selected plate face".to_owned(),
            ));
        }
    }
    validate_cut_frame_clearance(&cuts)?;

    let xs = plate_cut_axis_samples(half_x, &cuts, 0);
    let zs = plate_cut_axis_samples(half_z, &cuts, 1);
    let outside_y = face_sign * half_y;
    let opposite_y = -outside_y;
    let opposite_normal = [0.0, -face_sign, 0.0];
    let outside_normal = [0.0, face_sign, 0.0];
    let host_frame = Rect2 {
        min_x: -half_x,
        max_x: half_x,
        min_z: -half_z,
        max_z: half_z,
    };
    let host_points = frame_boundary_points(host_frame, &xs, &zs);

    let mut builder = MeshBuilder::new();
    let outside_host = builder.add_plate_ring(outside_y, &host_points)?;
    let opposite_host = builder.add_plate_ring(opposite_y, &host_points)?;
    add_plate_shell_sides(
        &mut builder,
        &opposite_host,
        &outside_host,
        &host_points,
        context,
        None,
    );

    add_plate_grid_face_with_holes(
        &mut builder,
        outside_y,
        &xs,
        &zs,
        &cuts,
        outside_normal,
        context,
        entry_region,
    )?;
    let through_cuts = cuts
        .iter()
        .filter(|cut| matches!(cut.kind, PlateCutKind::Through))
        .cloned()
        .collect::<Vec<_>>();
    if through_cuts.is_empty() {
        add_cap_oriented(
            &mut builder,
            opposite_host,
            opposite_normal,
            plate_metadata(context, opposite_region),
        );
    } else {
        add_plate_grid_face_with_holes(
            &mut builder,
            opposite_y,
            &xs,
            &zs,
            &through_cuts,
            opposite_normal,
            context,
            opposite_region,
        )?;
    }

    let mut boundary_marks = Vec::new();
    let mut regions = plate_regions();
    let mut entry_wall_rings: BTreeMap<OperationId, Vec<u32>> = BTreeMap::new();
    let mut secondary_wall_rings: BTreeMap<OperationId, Vec<u32>> = BTreeMap::new();
    for cut in &cuts {
        let (entry_surface_ring, entry_wall_ring) = add_cut_features_for_face(
            &mut builder,
            cut,
            outside_y,
            outside_normal,
            &xs,
            &zs,
            entry_region,
            context,
            cut.entry_bevel.as_ref(),
        )?;
        push_cut_or_bevel_boundary_marks(
            &mut boundary_marks,
            entry_surface_ring.clone(),
            entry_wall_ring.clone(),
            cut.operation,
            cut.entry_loop,
            cut.edge_treatment,
            cut.entry_bevel.as_ref(),
        );
        entry_wall_rings.insert(cut.operation, entry_wall_ring);
        if matches!(cut.kind, PlateCutKind::Through) {
            let (secondary_surface_ring, secondary_wall_ring) = add_cut_features_for_face(
                &mut builder,
                cut,
                opposite_y,
                opposite_normal,
                &xs,
                &zs,
                opposite_region,
                context,
                cut.secondary_bevel.as_ref(),
            )?;
            push_cut_or_bevel_boundary_marks(
                &mut boundary_marks,
                secondary_surface_ring.clone(),
                secondary_wall_ring.clone(),
                cut.operation,
                cut.secondary_loop,
                cut.edge_treatment,
                cut.secondary_bevel.as_ref(),
            );
            secondary_wall_rings.insert(cut.operation, secondary_wall_ring);
        }
        if let PlateCutKind::Recessed {
            depth,
            floor_region,
        } = cut.kind
        {
            let floor_y = outside_y - face_sign * depth;
            let outside_inner = entry_wall_rings
                .get(&cut.operation)
                .cloned()
                .expect("entry wall ring should be generated before recessed wall");
            let floor_surface_points = cut
                .secondary_bevel
                .as_ref()
                .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, -bevel.width))
                .unwrap_or_else(|| cut.inner_points.clone());
            let (wall_bottom_ring, floor_inner) = if let Some(bevel) = &cut.secondary_bevel {
                add_boundary_loop_bevel_band(
                    &mut builder,
                    floor_y + face_sign * bevel.width,
                    &cut.inner_points,
                    floor_y,
                    &floor_surface_points,
                    outside_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_plate_ring(floor_y, &cut.inner_points)?;
                (ring.clone(), ring)
            };
            add_cut_wall_band(
                &mut builder,
                &outside_inner,
                &wall_bottom_ring,
                &cut.inner_points,
                cut.center,
                context,
                cut.operation,
                cut.wall_region,
            );
            add_cap_oriented(
                &mut builder,
                floor_inner.clone(),
                outside_normal,
                cut_metadata(
                    context,
                    floor_region,
                    SurfaceRole::Interior,
                    cut.operation,
                    None,
                ),
            );
            push_cut_or_bevel_boundary_marks(
                &mut boundary_marks,
                wall_bottom_ring,
                floor_inner,
                cut.operation,
                cut.secondary_loop,
                cut.edge_treatment,
                cut.secondary_bevel.as_ref(),
            );
            insert_cut_region(
                &mut regions,
                floor_region,
                "recess_floor",
                SurfaceRole::Interior,
            );
        }
        insert_cut_region(&mut regions, cut.rim_region, "cut_rim", SurfaceRole::Rim);
        insert_cut_region(
            &mut regions,
            cut.wall_region,
            "cut_wall",
            SurfaceRole::CutWall,
        );
        insert_boundary_bevel_region(&mut regions, cut.entry_bevel.as_ref());
        insert_boundary_bevel_region(&mut regions, cut.secondary_bevel.as_ref());
    }

    for cut in cuts
        .iter()
        .filter(|cut| matches!(cut.kind, PlateCutKind::Through))
    {
        let outside_inner = entry_wall_rings
            .get(&cut.operation)
            .expect("entry wall ring should be generated before through wall");
        let opposite_inner = secondary_wall_rings
            .get(&cut.operation)
            .expect("secondary wall ring should be generated before through wall");
        add_cut_wall_band(
            &mut builder,
            outside_inner,
            opposite_inner,
            &cut.inner_points,
            cut.center,
            context,
            cut.operation,
            cut.wall_region,
        );
    }

    let mut mesh = builder.finish()?;
    for mark in boundary_marks {
        mark_boundary_loop(
            &mut mesh,
            &mark.ring,
            mark.operation,
            mark.boundary_loop,
            mark.treatment,
        );
    }

    Ok(part(
        mesh,
        regions,
        plate_sockets(half_y),
        format!(
            "plate_multi_cut:w={:.6}:h={:.6}:t={:.6}:face={:?}:cuts={}",
            width,
            height,
            thickness,
            face,
            cuts.iter()
                .map(|cut| cut.operation.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    ))
}

fn build_frustum_like(
    params: &FrustumParams,
    context: &GeneratorContext,
    label: &'static str,
) -> Result<GeneratedPart, ModelingError> {
    let bottom_radius = finite_non_negative(params.bottom_radius, "frustum.bottom_radius")?;
    let top_radius = finite_non_negative(params.top_radius, "frustum.top_radius")?;
    if bottom_radius <= EPSILON && top_radius <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "frustum requires at least one positive radius".to_owned(),
        ));
    }
    let half_height = finite_positive(params.half_height, "frustum.half_height")?;
    let radial_segments = params.radial_segments.max(3);
    let height_segments = params.height_segments.max(1);
    let requested_top = finite_non_negative(params.top_bevel_radius, "frustum.top_bevel_radius")?;
    let requested_bottom =
        finite_non_negative(params.bottom_bevel_radius, "frustum.bottom_bevel_radius")?;
    let (bottom_bevel, top_bevel) = clamp_frustum_bevels(
        requested_bottom,
        requested_top,
        bottom_radius,
        top_radius,
        half_height * 2.0,
    );
    let bevel_segments = if bottom_bevel > EPSILON || top_bevel > EPSILON {
        params.bevel_segments.max(1)
    } else {
        0
    };
    let mut builder = MeshBuilder::new();
    let rings = frustum_rings(
        &mut builder,
        FrustumRingPlan {
            bottom_radius,
            top_radius,
            half_height,
            radial_segments,
            height_segments,
            bottom_bevel,
            top_bevel,
            bevel_segments,
        },
    )?;

    for pair in rings.windows(2) {
        let region = pair[1].incoming_region;
        add_ring_band(&mut builder, &pair[0], &pair[1], context, region);
    }
    if params.cap_mode.has_bottom() {
        add_cap(&mut builder, &rings[0], false, context);
    }
    if params.cap_mode.has_top() {
        let last = rings
            .last()
            .expect("frustum ring generation should always produce a ring");
        add_cap(&mut builder, last, true, context);
    }

    let mesh = builder.finish()?;
    let regions = cylinder_regions();
    let sockets = cylinder_sockets(half_height);
    Ok(part(
        mesh,
        regions,
        sockets,
        format!(
            "{label}:br={:.6}:tr={:.6}:hh={:.6}:rs={}:hs={}:cap={:?}:tb={:.6}:bb={:.6}:bs={}",
            bottom_radius,
            top_radius,
            half_height,
            radial_segments,
            height_segments,
            params.cap_mode,
            top_bevel,
            bottom_bevel,
            bevel_segments
        ),
    ))
}

#[derive(Debug, Copy, Clone)]
struct FrustumRingPlan {
    bottom_radius: f32,
    top_radius: f32,
    half_height: f32,
    radial_segments: u32,
    height_segments: u32,
    bottom_bevel: f32,
    top_bevel: f32,
    bevel_segments: u32,
}

fn frustum_rings(
    builder: &mut MeshBuilder,
    plan: FrustumRingPlan,
) -> Result<Vec<Ring>, ModelingError> {
    let bottom_y = -plan.half_height;
    let top_y = plan.half_height;
    let bottom_cap_radius = (plan.bottom_radius - plan.bottom_bevel).max(0.0);
    let top_cap_radius = (plan.top_radius - plan.top_bevel).max(0.0);
    let bottom_side_y = bottom_y + plan.bottom_bevel;
    let top_side_y = top_y - plan.top_bevel;
    let mut rings = Vec::new();

    if plan.bottom_bevel > EPSILON {
        for index in 0..=plan.bevel_segments {
            let t = index as f32 / plan.bevel_segments as f32;
            let angle = t * FRAC_PI_2;
            let radius = bottom_cap_radius + plan.bottom_bevel * (1.0 - angle.cos());
            let y = bottom_y + plan.bottom_bevel * angle.sin();
            rings.push(builder.add_ring(
                y,
                radius,
                plan.radial_segments,
                CYLINDER_BOTTOM_BEVEL_REGION,
            )?);
        }
    } else {
        rings.push(builder.add_ring(
            bottom_y,
            plan.bottom_radius,
            plan.radial_segments,
            CYLINDER_SIDE_REGION,
        )?);
    }

    for index in 1..=plan.height_segments {
        let t = index as f32 / plan.height_segments as f32;
        let y = lerp(bottom_side_y, top_side_y, t);
        let radius = lerp(plan.bottom_radius, plan.top_radius, t);
        if index == plan.height_segments && plan.top_bevel > EPSILON {
            continue;
        }
        rings.push(builder.add_ring(y, radius, plan.radial_segments, CYLINDER_SIDE_REGION)?);
    }

    if plan.top_bevel > EPSILON {
        if rings
            .last()
            .is_none_or(|ring| (ring.y - top_side_y).abs() > EPSILON)
        {
            rings.push(builder.add_ring(
                top_side_y,
                plan.top_radius,
                plan.radial_segments,
                CYLINDER_SIDE_REGION,
            )?);
        }
        for index in 1..=plan.bevel_segments {
            let t = index as f32 / plan.bevel_segments as f32;
            let angle = t * FRAC_PI_2;
            let radius = top_cap_radius + plan.top_bevel * angle.cos();
            let y = top_y - plan.top_bevel * (1.0 - angle.sin());
            rings.push(builder.add_ring(
                y,
                radius,
                plan.radial_segments,
                CYLINDER_TOP_BEVEL_REGION,
            )?);
        }
    }
    Ok(rings)
}

fn add_ring_band(
    builder: &mut MeshBuilder,
    lower: &Ring,
    upper: &Ring,
    context: &GeneratorContext,
    region: RegionId,
) {
    match (&lower.vertices, &upper.vertices) {
        (RingVertices::Circle(lower_vertices), RingVertices::Circle(upper_vertices)) => {
            for index in 0..lower_vertices.len() {
                let next = (index + 1) % lower_vertices.len();
                builder.add_face(
                    vec![
                        lower_vertices[index],
                        upper_vertices[index],
                        upper_vertices[next],
                        lower_vertices[next],
                    ],
                    cylinder_metadata(context, region),
                );
            }
        }
        (RingVertices::Apex(apex), RingVertices::Circle(upper_vertices)) => {
            for index in 0..upper_vertices.len() {
                let next = (index + 1) % upper_vertices.len();
                builder.add_face(
                    vec![*apex, upper_vertices[index], upper_vertices[next]],
                    cylinder_metadata(context, region),
                );
            }
        }
        (RingVertices::Circle(lower_vertices), RingVertices::Apex(apex)) => {
            for index in 0..lower_vertices.len() {
                let next = (index + 1) % lower_vertices.len();
                builder.add_face(
                    vec![lower_vertices[index], *apex, lower_vertices[next]],
                    cylinder_metadata(context, region),
                );
            }
        }
        (RingVertices::Apex(_), RingVertices::Apex(_)) => {}
    }
}

fn add_cap(builder: &mut MeshBuilder, ring: &Ring, top: bool, context: &GeneratorContext) {
    let RingVertices::Circle(vertices) = &ring.vertices else {
        return;
    };
    let mut face = vertices.clone();
    let region = if top {
        face.reverse();
        CYLINDER_TOP_CAP_REGION
    } else {
        CYLINDER_BOTTOM_CAP_REGION
    };
    builder.add_face(face, cylinder_metadata(context, region));
}

fn add_plate_band(
    builder: &mut MeshBuilder,
    lower: &[u32],
    upper: &[u32],
    context: &GeneratorContext,
    region: RegionId,
) {
    for index in 0..lower.len() {
        let next = (index + 1) % lower.len();
        builder.add_face(
            vec![lower[index], upper[index], upper[next], lower[next]],
            plate_metadata(context, region),
        );
    }
}

#[derive(Debug, Copy, Clone)]
enum PlateCutKind {
    Recessed { depth: f32, floor_region: RegionId },
    Through,
}

#[derive(Debug, Clone)]
struct PlateCutPlan {
    kind: PlateCutKind,
    operation: OperationId,
    face: PlanarCutFace,
    center: [f32; 2],
    inner_points: Vec<[f32; 2]>,
    rim_points: Vec<[f32; 2]>,
    frame_points: Vec<[f32; 2]>,
    frame: Rect2,
    has_host_surface_band: bool,
    rim_width: f32,
    corner_segments: u32,
    target_region: RegionId,
    entry_loop: BoundaryLoopId,
    secondary_loop: BoundaryLoopId,
    outer_region: RegionId,
    rim_region: RegionId,
    wall_region: RegionId,
    edge_treatment: CutEdgeTreatment,
    entry_bevel: Option<BoundaryLoopBevelPlan>,
    secondary_bevel: Option<BoundaryLoopBevelPlan>,
}

struct BoundaryLoopMark {
    ring: Vec<u32>,
    operation: OperationId,
    boundary_loop: BoundaryLoopId,
    treatment: CutEdgeTreatment,
}

#[derive(Debug, Clone)]
struct BoundaryLoopBevelPlan {
    operation: OperationId,
    target_loop: BoundaryLoopId,
    width: f32,
    segments: u32,
    profile: f32,
    bevel_region: RegionId,
    outer_replacement_loop: BoundaryLoopId,
    inner_replacement_loop: BoundaryLoopId,
}

impl BoundaryLoopBevelPlan {
    fn from_operation(operation: &ModelingOperationSpec) -> Result<Self, ModelingError> {
        let ModelingOperationSpec::BevelBoundaryLoop {
            operation,
            target_loop,
            width,
            segments,
            profile,
            bevel_region,
            outer_replacement_loop,
            inner_replacement_loop,
        } = operation
        else {
            return Err(ModelingError::InvalidInput(
                "expected BevelBoundaryLoop operation".to_owned(),
            ));
        };
        Ok(Self {
            operation: *operation,
            target_loop: *target_loop,
            width: finite_positive(*width, "bevel_boundary_loop.width")?,
            segments: (*segments).max(1),
            profile: boundary_bevel_profile(*profile)?,
            bevel_region: *bevel_region,
            outer_replacement_loop: *outer_replacement_loop,
            inner_replacement_loop: *inner_replacement_loop,
        })
    }
}

impl PlateCutPlan {
    fn from_operation(
        operation: &ModelingOperationSpec,
        half_x: f32,
        half_z: f32,
        thickness: f32,
    ) -> Result<Self, ModelingError> {
        match operation {
            ModelingOperationSpec::RecessedPanelCut {
                operation,
                face,
                center,
                size,
                depth,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                floor_loop,
                region,
                outer_region,
                rim_region,
                wall_region,
                floor_region,
                edge_treatment,
                ..
            } => {
                let depth = finite_positive(*depth, "recessed_panel_cut.depth")?;
                if depth >= thickness - EPSILON {
                    return Err(ModelingError::InvalidInput(
                        "recessed panel depth must leave material behind the cut".to_owned(),
                    ));
                }
                let rim_width = finite_positive(*rim_width, "recessed_panel_cut.rim_width")?;
                let corner_segments = (*corner_segments).max(1);
                let inner_points = rounded_cut_points(
                    *center,
                    *size,
                    *corner_radius,
                    corner_segments,
                    CutPointCount::RoundedRect,
                )?;
                let frame = cut_frame_rect(*center, &inner_points, half_x, half_z, rim_width)?;
                let frame_points =
                    rounded_frame_points(*center, *size, *corner_radius, corner_segments, frame)?;
                Ok(Self {
                    kind: PlateCutKind::Recessed {
                        depth,
                        floor_region: *floor_region,
                    },
                    operation: *operation,
                    face: *face,
                    center: *center,
                    inner_points,
                    rim_points: frame_points.clone(),
                    frame_points,
                    frame,
                    has_host_surface_band: false,
                    rim_width,
                    corner_segments,
                    target_region: *region,
                    entry_loop: *entry_loop,
                    secondary_loop: *floor_loop,
                    outer_region: *outer_region,
                    rim_region: *rim_region,
                    wall_region: *wall_region,
                    edge_treatment: *edge_treatment,
                    entry_bevel: None,
                    secondary_bevel: None,
                })
            }
            ModelingOperationSpec::RectangularThroughCut {
                operation,
                face,
                center,
                size,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                exit_loop,
                region,
                outer_region,
                rim_region,
                wall_region,
                edge_treatment,
                ..
            } => {
                let rim_width = finite_positive(*rim_width, "rectangular_through_cut.rim_width")?;
                let corner_segments = (*corner_segments).max(1);
                let inner_points = rounded_cut_points(
                    *center,
                    *size,
                    *corner_radius,
                    corner_segments,
                    CutPointCount::RoundedRect,
                )?;
                let frame = cut_frame_rect(*center, &inner_points, half_x, half_z, rim_width)?;
                let frame_points =
                    rounded_frame_points(*center, *size, *corner_radius, corner_segments, frame)?;
                Ok(Self {
                    kind: PlateCutKind::Through,
                    operation: *operation,
                    face: *face,
                    center: *center,
                    inner_points,
                    rim_points: frame_points.clone(),
                    frame_points,
                    frame,
                    has_host_surface_band: false,
                    rim_width,
                    corner_segments,
                    target_region: *region,
                    entry_loop: *entry_loop,
                    secondary_loop: *exit_loop,
                    outer_region: *outer_region,
                    rim_region: *rim_region,
                    wall_region: *wall_region,
                    edge_treatment: *edge_treatment,
                    entry_bevel: None,
                    secondary_bevel: None,
                })
            }
            ModelingOperationSpec::CircularThroughCut {
                operation,
                face,
                center,
                radius,
                radial_segments,
                rim_width,
                entry_loop,
                exit_loop,
                region,
                outer_region,
                rim_region,
                wall_region,
                edge_treatment,
                ..
            } => {
                let radius = finite_positive(*radius, "circular_through_cut.radius")?;
                let rim_width = finite_positive(*rim_width, "circular_through_cut.rim_width")?;
                let segments = (*radial_segments).max(6);
                let mut inner_points = Vec::with_capacity(segments as usize);
                for index in 0..segments {
                    let angle = 2.0 * PI * index as f32 / segments as f32;
                    let (sin, cos) = angle.sin_cos();
                    inner_points.push([center[0] + radius * cos, center[1] + radius * sin]);
                }
                let rim_radius = radius + rim_width;
                let mut rim_points = Vec::with_capacity(segments as usize);
                for index in 0..segments {
                    let angle = 2.0 * PI * index as f32 / segments as f32;
                    let (sin, cos) = angle.sin_cos();
                    rim_points.push([center[0] + rim_radius * cos, center[1] + rim_radius * sin]);
                }
                let frame = cut_frame_rect(*center, &rim_points, half_x, half_z, rim_width)?;
                let frame_points = rim_points
                    .iter()
                    .map(|point| ray_to_rect(*center, *point, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self {
                    kind: PlateCutKind::Through,
                    operation: *operation,
                    face: *face,
                    center: *center,
                    inner_points,
                    rim_points,
                    frame_points,
                    frame,
                    has_host_surface_band: true,
                    rim_width,
                    corner_segments: segments,
                    target_region: *region,
                    entry_loop: *entry_loop,
                    secondary_loop: *exit_loop,
                    outer_region: *outer_region,
                    rim_region: *rim_region,
                    wall_region: *wall_region,
                    edge_treatment: *edge_treatment,
                    entry_bevel: None,
                    secondary_bevel: None,
                })
            }
            _ => Err(ModelingError::InvalidInput(
                "build_cut_plate received a non-cut operation".to_owned(),
            )),
        }
    }
}

fn apply_boundary_loop_bevels(
    cuts: &mut [PlateCutPlan],
    bevels: &[BoundaryLoopBevelPlan],
    thickness: f32,
) -> Result<(), ModelingError> {
    for bevel in bevels {
        let mut matched = false;
        for cut in cuts.iter_mut() {
            if cut.entry_loop == bevel.target_loop {
                validate_plate_loop_bevel(cut, bevel, true, thickness)?;
                cut.entry_bevel = Some(bevel.clone());
                matched = true;
                break;
            }
            if cut.secondary_loop == bevel.target_loop {
                validate_plate_loop_bevel(cut, bevel, false, thickness)?;
                cut.secondary_bevel = Some(bevel.clone());
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(ModelingError::UnsupportedOperation {
                operation: bevel.operation,
                reason: format!(
                    "BevelBoundaryLoop target {} is not a supported Plate cut loop",
                    bevel.target_loop.0
                ),
            });
        }
    }
    Ok(())
}

fn validate_plate_loop_bevel(
    cut: &PlateCutPlan,
    bevel: &BoundaryLoopBevelPlan,
    entry_loop: bool,
    thickness: f32,
) -> Result<(), ModelingError> {
    if !matches!(cut.edge_treatment, CutEdgeTreatment::BevelEligible) {
        return Err(ModelingError::UnsupportedOperation {
            operation: bevel.operation,
            reason: "BevelBoundaryLoop target loop is authored as hard-only".to_owned(),
        });
    }
    if bevel.width >= cut.rim_width - EPSILON {
        return Err(ModelingError::UnsupportedOperation {
            operation: bevel.operation,
            reason: "boundary-loop bevel width must be smaller than the authored cut rim width"
                .to_owned(),
        });
    }
    let loop_radius = minimum_loop_radius(&cut.inner_points, cut.center);
    if bevel.width >= loop_radius * 0.5 {
        return Err(ModelingError::UnsupportedOperation {
            operation: bevel.operation,
            reason: "boundary-loop bevel width is too large for the target loop radius".to_owned(),
        });
    }
    match cut.kind {
        PlateCutKind::Recessed { depth, .. } => {
            if entry_loop {
                if bevel.width >= depth - EPSILON {
                    return Err(ModelingError::UnsupportedOperation {
                        operation: bevel.operation,
                        reason: "entry bevel width must leave vertical cut wall height".to_owned(),
                    });
                }
            } else if bevel.width >= depth - EPSILON {
                return Err(ModelingError::UnsupportedOperation {
                    operation: bevel.operation,
                    reason: "floor bevel width must be smaller than recess depth".to_owned(),
                });
            }
        }
        PlateCutKind::Through => {
            let entry_width = if entry_loop {
                bevel.width
            } else {
                cut.entry_bevel.as_ref().map_or(0.0, |entry| entry.width)
            };
            let exit_width = if entry_loop {
                cut.secondary_bevel.as_ref().map_or(0.0, |exit| exit.width)
            } else {
                bevel.width
            };
            if entry_width + exit_width >= thickness - EPSILON {
                return Err(ModelingError::UnsupportedOperation {
                    operation: bevel.operation,
                    reason: "opposing through-cut bevels must leave cut wall height".to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn minimum_loop_radius(points: &[[f32; 2]], center: [f32; 2]) -> f32 {
    points
        .iter()
        .map(|point| ((point[0] - center[0]).powi(2) + (point[1] - center[1]).powi(2)).sqrt())
        .fold(f32::INFINITY, f32::min)
}

fn validate_cut_frame_clearance(cuts: &[PlateCutPlan]) -> Result<(), ModelingError> {
    for (left_index, left) in cuts.iter().enumerate() {
        for right in cuts.iter().skip(left_index + 1) {
            if rects_touch_or_overlap(left.frame, right.frame) {
                return Err(ModelingError::UnsupportedOperation {
                    operation: right.operation,
                    reason: format!(
                        "cut frame for operation {:?} overlaps or touches operation {:?}; multi-cut composition requires separated cut footprints",
                        right.operation, left.operation
                    ),
                });
            }
            if frame_projection_splits(left.frame, right.frame)
                || frame_projection_splits(right.frame, left.frame)
            {
                return Err(ModelingError::UnsupportedOperation {
                    operation: right.operation,
                    reason: format!(
                        "cut frame for operation {:?} would split operation {:?}'s window boundary; align repeated cut columns/rows or separate their projections",
                        right.operation, left.operation
                    ),
                });
            }
        }
    }
    Ok(())
}

fn rects_touch_or_overlap(left: Rect2, right: Rect2) -> bool {
    left.min_x <= right.max_x + EPSILON
        && left.max_x + EPSILON >= right.min_x
        && left.min_z <= right.max_z + EPSILON
        && left.max_z + EPSILON >= right.min_z
}

fn frame_projection_splits(frame: Rect2, other: Rect2) -> bool {
    value_inside_open_interval(other.min_x, frame.min_x, frame.max_x)
        || value_inside_open_interval(other.max_x, frame.min_x, frame.max_x)
        || value_inside_open_interval(other.min_z, frame.min_z, frame.max_z)
        || value_inside_open_interval(other.max_z, frame.min_z, frame.max_z)
}

fn value_inside_open_interval(value: f32, min: f32, max: f32) -> bool {
    value > min + EPSILON && value < max - EPSILON
}

fn plate_cut_axis_samples(half_extent: f32, cuts: &[PlateCutPlan], axis: usize) -> Vec<f32> {
    let mut samples = vec![-half_extent, half_extent];
    for cut in cuts {
        match axis {
            0 => {
                samples.push(cut.frame.min_x);
                samples.push(cut.frame.max_x);
            }
            _ => {
                samples.push(cut.frame.min_z);
                samples.push(cut.frame.max_z);
            }
        }
    }
    dedup_sorted_f32(samples)
}

#[allow(clippy::too_many_arguments)]
fn add_plate_grid_face_with_holes(
    builder: &mut MeshBuilder,
    y: f32,
    xs: &[f32],
    zs: &[f32],
    holes: &[PlateCutPlan],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    region: RegionId,
) -> Result<(), ModelingError> {
    for x_index in 0..xs.len().saturating_sub(1) {
        for z_index in 0..zs.len().saturating_sub(1) {
            let cell = Rect2 {
                min_x: xs[x_index],
                max_x: xs[x_index + 1],
                min_z: zs[z_index],
                max_z: zs[z_index + 1],
            };
            let center = [
                (cell.min_x + cell.max_x) * 0.5,
                (cell.min_z + cell.max_z) * 0.5,
            ];
            if holes.iter().any(|cut| point_inside_rect(center, cut.frame)) {
                continue;
            }
            let points = rect_points(cell.min_x, cell.max_x, cell.min_z, cell.max_z);
            let vertices = builder.add_plate_ring(y, &points)?;
            add_oriented_face(
                builder,
                vertices,
                desired_normal,
                plate_metadata(context, region),
            );
        }
    }
    Ok(())
}

fn point_inside_rect(point: [f32; 2], rect: Rect2) -> bool {
    point[0] > rect.min_x + EPSILON
        && point[0] < rect.max_x - EPSILON
        && point[1] > rect.min_z + EPSILON
        && point[1] < rect.max_z - EPSILON
}

#[allow(clippy::too_many_arguments)]
fn add_cut_features_for_face(
    builder: &mut MeshBuilder,
    cut: &PlateCutPlan,
    y: f32,
    desired_normal: [f32; 3],
    xs: &[f32],
    zs: &[f32],
    host_region: RegionId,
    context: &GeneratorContext,
    bevel: Option<&BoundaryLoopBevelPlan>,
) -> Result<(Vec<u32>, Vec<u32>), ModelingError> {
    let window_points = frame_boundary_points(cut.frame, xs, zs);
    let window_ring = builder.add_plate_ring(y, &window_points)?;
    let surface_points = bevel
        .map(|bevel| offset_loop_points(&cut.inner_points, cut.center, bevel.width))
        .unwrap_or_else(|| cut.inner_points.clone());
    let surface_ring = builder.add_plate_ring(y, &surface_points)?;

    if !cut.has_host_surface_band {
        add_frame_boundary_to_ring_cap(
            builder,
            &window_ring,
            &window_points,
            &surface_ring,
            &surface_points,
            cut.frame,
            desired_normal,
            context,
            cut.operation,
            cut.rim_region,
            SurfaceRole::Rim,
        )?;
        return if let Some(bevel) = bevel {
            add_boundary_loop_bevel_band_from_outer(
                builder,
                surface_ring,
                y,
                &surface_points,
                y - desired_normal[1] * bevel.width,
                &cut.inner_points,
                desired_normal,
                context,
                bevel,
            )
        } else {
            Ok((surface_ring.clone(), surface_ring))
        };
    }

    let rim_ring = builder.add_plate_ring(y, &cut.rim_points)?;
    add_frame_boundary_to_ring_cap(
        builder,
        &window_ring,
        &window_points,
        &rim_ring,
        &cut.rim_points,
        cut.frame,
        desired_normal,
        context,
        cut.operation,
        host_region,
        SurfaceRole::PrimarySurface,
    )?;
    add_matched_ring_band(
        builder,
        &rim_ring,
        &surface_ring,
        &cut.rim_points,
        &surface_points,
        desired_normal,
        context,
        cut.operation,
        cut.rim_region,
        SurfaceRole::Rim,
    );
    if let Some(bevel) = bevel {
        add_boundary_loop_bevel_band_from_outer(
            builder,
            surface_ring,
            y,
            &surface_points,
            y - desired_normal[1] * bevel.width,
            &cut.inner_points,
            desired_normal,
            context,
            bevel,
        )
    } else {
        Ok((surface_ring.clone(), surface_ring))
    }
}

fn frame_boundary_points(frame: Rect2, xs: &[f32], zs: &[f32]) -> Vec<[f32; 2]> {
    let mut points = Vec::new();
    let x_values = values_in_range(xs, frame.min_x, frame.max_x);
    let z_values = values_in_range(zs, frame.min_z, frame.max_z);

    for x in x_values.iter().rev() {
        points.push([*x, frame.max_z]);
    }
    for z in z_values.iter().rev().skip(1) {
        points.push([frame.min_x, *z]);
    }
    for x in x_values.iter().skip(1) {
        points.push([*x, frame.min_z]);
    }
    if z_values.len() > 2 {
        for z in z_values.iter().skip(1).take(z_values.len() - 2) {
            points.push([frame.max_x, *z]);
        }
    }
    points
}

fn values_in_range(samples: &[f32], min: f32, max: f32) -> Vec<f32> {
    samples
        .iter()
        .copied()
        .filter(|value| *value >= min - EPSILON && *value <= max + EPSILON)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn add_frame_boundary_to_ring_cap(
    builder: &mut MeshBuilder,
    frame_vertices: &[u32],
    frame_points: &[[f32; 2]],
    ring_vertices: &[u32],
    ring_points: &[[f32; 2]],
    _frame: Rect2,
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
) -> Result<(), ModelingError> {
    add_triangulated_cap_with_vertices(
        builder,
        frame_vertices,
        frame_points,
        &[frame_points.len()],
        Some((ring_vertices, ring_points)),
        desired_normal,
        cut_metadata(context, region, role, operation, None),
    )
}

fn add_triangulated_cap_with_vertices(
    builder: &mut MeshBuilder,
    vertices: &[u32],
    points: &[[f32; 2]],
    hole_indices: &[usize],
    extra_ring: Option<(&[u32], &[[f32; 2]])>,
    desired_normal: [f32; 3],
    metadata: FaceMetadata,
) -> Result<(), ModelingError> {
    if points.len() < 3 || vertices.len() != points.len() {
        return Ok(());
    }
    if let Some((ring_vertices, ring_points)) = extra_ring
        && (ring_points.len() < 3 || ring_vertices.len() != ring_points.len())
    {
        return Ok(());
    }
    let total_points = points.len() + extra_ring.map(|(_, ring)| ring.len()).unwrap_or_default();
    let mut coords = Vec::with_capacity(total_points * 2);
    for point in points {
        coords.push(point[0] as f64);
        coords.push(point[1] as f64);
    }
    if let Some((_, ring_points)) = extra_ring {
        for point in ring_points {
            coords.push(point[0] as f64);
            coords.push(point[1] as f64);
        }
    }
    let indices = earcutr::earcut(&coords, hole_indices, 2).map_err(|error| {
        ModelingError::InvalidInput(format!("failed to triangulate cut window cap: {error:?}"))
    })?;
    for triangle in indices.chunks_exact(3) {
        let face = triangle
            .iter()
            .map(|index| {
                if *index < vertices.len() {
                    vertices[*index]
                } else if let Some((ring_vertices, _)) = extra_ring {
                    ring_vertices[*index - vertices.len()]
                } else {
                    vertices[*index]
                }
            })
            .collect::<Vec<_>>();
        add_oriented_face_if_non_degenerate(builder, face, desired_normal, metadata.clone());
    }
    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum CutPointCount {
    RoundedRect,
}

#[derive(Debug, Copy, Clone)]
struct Rect2 {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RectSide {
    Right,
    Top,
    Left,
    Bottom,
}

fn rounded_cut_points(
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    corner_segments: u32,
    _count: CutPointCount,
) -> Result<Vec<[f32; 2]>, ModelingError> {
    let width = finite_positive(size[0], "cut.size.x")?;
    let height = finite_positive(size[1], "cut.size.y")?;
    let radius = finite_non_negative(corner_radius, "cut.corner_radius")?;
    let max_radius = width.min(height) * 0.5;
    if radius > max_radius {
        return Err(ModelingError::InvalidInput(
            "cut corner radius must not exceed half the smaller cut dimension".to_owned(),
        ));
    }
    let segments = if radius > EPSILON {
        corner_segments.max(1)
    } else {
        1
    };
    let local = rounded_rect_points(width * 0.5, height * 0.5, radius.max(0.0), segments);
    Ok(local
        .into_iter()
        .map(|point| [point[0] + center[0], point[1] + center[1]])
        .collect())
}

fn rounded_frame_points(
    center: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    corner_segments: u32,
    frame: Rect2,
) -> Result<Vec<[f32; 2]>, ModelingError> {
    let frame_half_x = (frame.max_x - frame.min_x) * 0.5;
    let frame_half_z = (frame.max_z - frame.min_z) * 0.5;
    let rim_x = (frame_half_x - size[0] * 0.5).max(0.0);
    let rim_z = (frame_half_z - size[1] * 0.5).max(0.0);
    let rim = rim_x.min(rim_z);
    let radius = if corner_radius > EPSILON {
        finite_non_negative(corner_radius, "cut.corner_radius")? + rim
    } else {
        0.0
    }
    .min(frame_half_x.min(frame_half_z));
    let segments = if radius > EPSILON {
        corner_segments.max(1)
    } else {
        1
    };
    Ok(
        rounded_rect_points(frame_half_x, frame_half_z, radius, segments)
            .into_iter()
            .map(|point| [point[0] + center[0], point[1] + center[1]])
            .collect(),
    )
}

fn cut_frame_rect(
    center: [f32; 2],
    inner_points: &[[f32; 2]],
    half_x: f32,
    half_z: f32,
    rim_width: f32,
) -> Result<Rect2, ModelingError> {
    let inner_bounds = bounds_2d(inner_points)?;
    if inner_bounds.min_x <= -half_x + EPSILON
        || inner_bounds.max_x >= half_x - EPSILON
        || inner_bounds.min_z <= -half_z + EPSILON
        || inner_bounds.max_z >= half_z - EPSILON
    {
        return Err(ModelingError::InvalidInput(
            "cut boundary must stay inside the plate face".to_owned(),
        ));
    }
    let clearance = [
        inner_bounds.min_x - -half_x,
        half_x - inner_bounds.max_x,
        inner_bounds.min_z - -half_z,
        half_z - inner_bounds.max_z,
    ]
    .into_iter()
    .fold(f32::INFINITY, f32::min);
    if !clearance.is_finite() || clearance <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "cut has no safe margin to the host boundary".to_owned(),
        ));
    }
    let rim_width = finite_positive(rim_width, "cut.rim_width")?;
    if rim_width >= clearance - EPSILON {
        return Err(ModelingError::InvalidInput(
            "cut rim width exceeds the safe margin to the host boundary".to_owned(),
        ));
    }
    let frame = Rect2 {
        min_x: inner_bounds.min_x - rim_width,
        max_x: inner_bounds.max_x + rim_width,
        min_z: inner_bounds.min_z - rim_width,
        max_z: inner_bounds.max_z + rim_width,
    };
    if frame.min_x <= -half_x + EPSILON
        || frame.max_x >= half_x - EPSILON
        || frame.min_z <= -half_z + EPSILON
        || frame.max_z >= half_z - EPSILON
    {
        return Err(ModelingError::InvalidInput(
            "cut rim overlaps the host boundary".to_owned(),
        ));
    }
    if center[0] <= frame.min_x
        || center[0] >= frame.max_x
        || center[1] <= frame.min_z
        || center[1] >= frame.max_z
    {
        return Err(ModelingError::InvalidInput(
            "cut center must lie inside the generated frame".to_owned(),
        ));
    }
    Ok(frame)
}

fn bounds_2d(points: &[[f32; 2]]) -> Result<Rect2, ModelingError> {
    if points.len() < 3 {
        return Err(ModelingError::InvalidInput(
            "cut boundary requires at least three points".to_owned(),
        ));
    }
    let mut bounds = Rect2 {
        min_x: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        min_z: f32::INFINITY,
        max_z: f32::NEG_INFINITY,
    };
    for point in points {
        if !point.iter().copied().all(f32::is_finite) {
            return Err(ModelingError::InvalidInput(
                "cut boundary contains a non-finite point".to_owned(),
            ));
        }
        bounds.min_x = bounds.min_x.min(point[0]);
        bounds.max_x = bounds.max_x.max(point[0]);
        bounds.min_z = bounds.min_z.min(point[1]);
        bounds.max_z = bounds.max_z.max(point[1]);
    }
    Ok(bounds)
}

fn rect_points(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Vec<[f32; 2]> {
    vec![
        [max_x, max_z],
        [min_x, max_z],
        [min_x, min_z],
        [max_x, min_z],
    ]
}

fn ray_to_rect(center: [f32; 2], point: [f32; 2], rect: Rect2) -> Result<[f32; 2], ModelingError> {
    let delta = [point[0] - center[0], point[1] - center[1]];
    if dot2(delta, delta) <= EPSILON {
        return Err(ModelingError::InvalidInput(
            "cut boundary point cannot equal the cut center".to_owned(),
        ));
    }
    let mut t = f32::INFINITY;
    if delta[0] > EPSILON {
        t = t.min((rect.max_x - center[0]) / delta[0]);
    } else if delta[0] < -EPSILON {
        t = t.min((rect.min_x - center[0]) / delta[0]);
    }
    if delta[1] > EPSILON {
        t = t.min((rect.max_z - center[1]) / delta[1]);
    } else if delta[1] < -EPSILON {
        t = t.min((rect.min_z - center[1]) / delta[1]);
    }
    if !t.is_finite() || t <= 1.0 {
        return Err(ModelingError::InvalidInput(
            "cut rim must expand outward from the cut boundary".to_owned(),
        ));
    }
    Ok([center[0] + delta[0] * t, center[1] + delta[1] * t])
}

fn planar_plate_face_sign(
    face: PlanarCutFace,
    operation: OperationId,
) -> Result<f32, ModelingError> {
    match face {
        PlanarCutFace::PositiveY => Ok(1.0),
        PlanarCutFace::NegativeY => Ok(-1.0),
        _ => Err(ModelingError::UnsupportedOperation {
            operation,
            reason: "plate semantic cuts currently target only local +/-Y planar faces".to_owned(),
        }),
    }
}

fn plate_cut_face_regions(
    face: PlanarCutFace,
    operation: OperationId,
) -> Result<(RegionId, RegionId), ModelingError> {
    match face {
        PlanarCutFace::PositiveY => Ok((PLATE_FRONT_REGION, PLATE_BACK_REGION)),
        PlanarCutFace::NegativeY => Ok((PLATE_BACK_REGION, PLATE_FRONT_REGION)),
        _ => Err(ModelingError::UnsupportedOperation {
            operation,
            reason: "plate semantic cuts currently target only local +/-Y planar faces".to_owned(),
        }),
    }
}

fn add_plate_shell_sides(
    builder: &mut MeshBuilder,
    back_ring: &[u32],
    front_ring: &[u32],
    points: &[[f32; 2]],
    context: &GeneratorContext,
    operation: Option<OperationId>,
) {
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let midpoint = [
            (points[index][0] + points[next][0]) * 0.5,
            0.0,
            (points[index][1] + points[next][1]) * 0.5,
        ];
        let metadata = operation.map_or_else(
            || plate_metadata(context, PLATE_SIDE_REGION),
            |operation| {
                cut_metadata(
                    context,
                    PLATE_SIDE_REGION,
                    SurfaceRole::Side,
                    operation,
                    Some(2),
                )
            },
        );
        add_oriented_face(
            builder,
            vec![
                back_ring[index],
                back_ring[next],
                front_ring[next],
                front_ring[index],
            ],
            normalize_or(midpoint, [1.0, 0.0, 0.0]),
            metadata,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn add_host_to_ring_cap(
    builder: &mut MeshBuilder,
    host_vertices: &[u32],
    host_points: &[[f32; 2]],
    ring_vertices: &[u32],
    ring_points: &[[f32; 2]],
    frame: Rect2,
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
) {
    let mut by_side: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    for (index, point) in ring_points.iter().copied().enumerate() {
        for side in [
            RectSide::Right,
            RectSide::Top,
            RectSide::Left,
            RectSide::Bottom,
        ] {
            if point_on_frame_side(point, frame, side) {
                by_side[side_index(side)].push(index);
            }
        }
    }
    by_side[side_index(RectSide::Top)]
        .sort_by(|left, right| ring_points[*left][0].total_cmp(&ring_points[*right][0]));
    by_side[side_index(RectSide::Left)]
        .sort_by(|left, right| ring_points[*left][1].total_cmp(&ring_points[*right][1]));
    by_side[side_index(RectSide::Bottom)]
        .sort_by(|left, right| ring_points[*right][0].total_cmp(&ring_points[*left][0]));
    by_side[side_index(RectSide::Right)]
        .sort_by(|left, right| ring_points[*right][1].total_cmp(&ring_points[*left][1]));
    add_host_side_cap(
        builder,
        [host_vertices[0], host_vertices[1]],
        &by_side[side_index(RectSide::Top)],
        ring_vertices,
        desired_normal,
        context,
        operation,
        region,
        role.clone(),
    );
    add_host_side_cap(
        builder,
        [host_vertices[1], host_vertices[2]],
        &by_side[side_index(RectSide::Left)],
        ring_vertices,
        desired_normal,
        context,
        operation,
        region,
        role.clone(),
    );
    add_host_side_cap(
        builder,
        [host_vertices[2], host_vertices[3]],
        &by_side[side_index(RectSide::Bottom)],
        ring_vertices,
        desired_normal,
        context,
        operation,
        region,
        role.clone(),
    );
    add_host_side_cap(
        builder,
        [host_vertices[3], host_vertices[0]],
        &by_side[side_index(RectSide::Right)],
        ring_vertices,
        desired_normal,
        context,
        operation,
        region,
        role.clone(),
    );
    for index in 0..ring_points.len() {
        let next = (index + 1) % ring_points.len();
        if edge_lies_on_frame_side(ring_points[index], ring_points[next], frame) {
            continue;
        }
        let midpoint = [
            (ring_points[index][0] + ring_points[next][0]) * 0.5,
            (ring_points[index][1] + ring_points[next][1]) * 0.5,
        ];
        let corner = corner_for_frame_segment(midpoint, frame);
        let corner_index = nearest_host_corner(host_points, corner);
        add_oriented_face(
            builder,
            vec![
                host_vertices[corner_index],
                ring_vertices[index],
                ring_vertices[next],
            ],
            desired_normal,
            cut_metadata(context, region, role.clone(), operation, None),
        );
    }
}

fn corner_for_frame_segment(midpoint: [f32; 2], frame: Rect2) -> usize {
    let center = [
        (frame.min_x + frame.max_x) * 0.5,
        (frame.min_z + frame.max_z) * 0.5,
    ];
    match (midpoint[0] >= center[0], midpoint[1] >= center[1]) {
        (true, true) => 0,
        (false, true) => 1,
        (false, false) => 2,
        (true, false) => 3,
    }
}

fn point_on_frame_side(point: [f32; 2], frame: Rect2, side: RectSide) -> bool {
    let tolerance = EPSILON * 10.0;
    match side {
        RectSide::Right => (point[0] - frame.max_x).abs() <= tolerance,
        RectSide::Top => (point[1] - frame.max_z).abs() <= tolerance,
        RectSide::Left => (point[0] - frame.min_x).abs() <= tolerance,
        RectSide::Bottom => (point[1] - frame.min_z).abs() <= tolerance,
    }
}

fn edge_lies_on_frame_side(first: [f32; 2], second: [f32; 2], frame: Rect2) -> bool {
    [
        RectSide::Right,
        RectSide::Top,
        RectSide::Left,
        RectSide::Bottom,
    ]
    .into_iter()
    .any(|side| point_on_frame_side(first, frame, side) && point_on_frame_side(second, frame, side))
}

#[allow(clippy::too_many_arguments)]
fn add_host_side_cap(
    builder: &mut MeshBuilder,
    host_edge: [u32; 2],
    side_indices: &[usize],
    ring_vertices: &[u32],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
) {
    if side_indices.len() < 2 {
        return;
    }
    let mut vertices = vec![host_edge[0], host_edge[1]];
    for index in side_indices {
        vertices.push(ring_vertices[*index]);
    }
    add_oriented_face(
        builder,
        vertices,
        desired_normal,
        cut_metadata(context, region, role, operation, None),
    );
}

fn side_index(side: RectSide) -> usize {
    match side {
        RectSide::Right => 0,
        RectSide::Top => 1,
        RectSide::Left => 2,
        RectSide::Bottom => 3,
    }
}

fn nearest_host_corner(host_points: &[[f32; 2]], corner: usize) -> usize {
    let target = match corner {
        0 => [f32::INFINITY, f32::INFINITY],
        1 => [f32::NEG_INFINITY, f32::INFINITY],
        2 => [f32::NEG_INFINITY, f32::NEG_INFINITY],
        _ => [f32::INFINITY, f32::NEG_INFINITY],
    };
    host_points
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            let left_key = corner_sort_key(**left, target);
            let right_key = corner_sort_key(**right, target);
            left_key.total_cmp(&right_key)
        })
        .map(|(index, _)| index)
        .unwrap_or(corner.min(host_points.len().saturating_sub(1)))
}

fn corner_sort_key(point: [f32; 2], target: [f32; 2]) -> f32 {
    let x = if target[0].is_sign_positive() {
        -point[0]
    } else {
        point[0]
    };
    let z = if target[1].is_sign_positive() {
        -point[1]
    } else {
        point[1]
    };
    x + z
}

#[allow(clippy::too_many_arguments)]
fn add_matched_ring_band(
    builder: &mut MeshBuilder,
    outer_ring: &[u32],
    inner_ring: &[u32],
    outer_points: &[[f32; 2]],
    inner_points: &[[f32; 2]],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
) {
    for index in 0..outer_ring.len() {
        let next = (index + 1) % outer_ring.len();
        add_oriented_face(
            builder,
            vec![
                outer_ring[index],
                outer_ring[next],
                inner_ring[next],
                inner_ring[index],
            ],
            desired_normal,
            cut_metadata(context, region, role.clone(), operation, None),
        );
        let side_a = outer_points[index];
        let side_b = outer_points[next];
        if (side_a[0] - side_b[0]).abs() > EPSILON && (side_a[1] - side_b[1]).abs() > EPSILON {
            let midpoint = [
                (inner_points[index][0] + inner_points[next][0]) * 0.5,
                0.0,
                (inner_points[index][1] + inner_points[next][1]) * 0.5,
            ];
            let _ = midpoint;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_boundary_bevel_ring_band(
    builder: &mut MeshBuilder,
    outer_ring: &[u32],
    inner_ring: &[u32],
    outer_points: &[[f32; 2]],
    inner_points: &[[f32; 2]],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
) {
    let smoothing_group = Some(boundary_bevel_smoothing_group(operation));
    for index in 0..outer_ring.len() {
        let next = (index + 1) % outer_ring.len();
        add_oriented_face(
            builder,
            vec![
                outer_ring[index],
                outer_ring[next],
                inner_ring[next],
                inner_ring[index],
            ],
            desired_normal,
            cut_metadata(context, region, role.clone(), operation, smoothing_group),
        );
        let side_a = outer_points[index];
        let side_b = outer_points[next];
        if (side_a[0] - side_b[0]).abs() > EPSILON && (side_a[1] - side_b[1]).abs() > EPSILON {
            let midpoint = [
                (inner_points[index][0] + inner_points[next][0]) * 0.5,
                0.0,
                (inner_points[index][1] + inner_points[next][1]) * 0.5,
            ];
            let _ = midpoint;
        }
    }
}

fn boundary_bevel_smoothing_group(operation: OperationId) -> u32 {
    10_000 + (operation.0 % 1_000_000) as u32
}

#[allow(clippy::too_many_arguments)]
fn add_boundary_loop_bevel_band(
    builder: &mut MeshBuilder,
    outer_y: f32,
    outer_points: &[[f32; 2]],
    inner_y: f32,
    inner_points: &[[f32; 2]],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    bevel: &BoundaryLoopBevelPlan,
) -> Result<(Vec<u32>, Vec<u32>), ModelingError> {
    if outer_points.len() != inner_points.len() {
        return Err(ModelingError::InvalidInput(
            "boundary-loop bevel replacement loops must have matching topology".to_owned(),
        ));
    }
    let outer_ring = builder.add_plate_ring(outer_y, outer_points)?;
    add_boundary_loop_bevel_band_from_outer(
        builder,
        outer_ring,
        outer_y,
        outer_points,
        inner_y,
        inner_points,
        desired_normal,
        context,
        bevel,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_boundary_loop_bevel_band_from_outer(
    builder: &mut MeshBuilder,
    outer_ring: Vec<u32>,
    outer_y: f32,
    outer_points: &[[f32; 2]],
    inner_y: f32,
    inner_points: &[[f32; 2]],
    desired_normal: [f32; 3],
    context: &GeneratorContext,
    bevel: &BoundaryLoopBevelPlan,
) -> Result<(Vec<u32>, Vec<u32>), ModelingError> {
    if outer_points.len() != inner_points.len() {
        return Err(ModelingError::InvalidInput(
            "boundary-loop bevel replacement loops must have matching topology".to_owned(),
        ));
    }
    let mut previous_points = outer_points.to_vec();
    let mut previous_ring = outer_ring.clone();
    for step in 1..=bevel.segments {
        let radial_t = step as f32 / bevel.segments as f32;
        let depth_t = boundary_bevel_depth_t(radial_t, bevel.profile);
        let current_points = lerp_loop_points(outer_points, inner_points, radial_t);
        let current_y = lerp(outer_y, inner_y, depth_t);
        let current_ring = builder.add_plate_ring(current_y, &current_points)?;
        add_boundary_bevel_ring_band(
            builder,
            &previous_ring,
            &current_ring,
            &previous_points,
            &current_points,
            desired_normal,
            context,
            bevel.operation,
            bevel.bevel_region,
            SurfaceRole::BevelBand,
        );
        previous_points = current_points;
        previous_ring = current_ring;
    }
    Ok((outer_ring, previous_ring))
}

fn boundary_bevel_profile(profile: f32) -> Result<f32, ModelingError> {
    if !profile.is_finite()
        || !(BOUNDARY_BEVEL_PROFILE_MIN..=BOUNDARY_BEVEL_PROFILE_MAX).contains(&profile)
    {
        return Err(ModelingError::InvalidInput(format!(
            "bevel_boundary_loop.profile must be finite and between {BOUNDARY_BEVEL_PROFILE_MIN:.3} and {BOUNDARY_BEVEL_PROFILE_MAX:.3}"
        )));
    }
    Ok(profile)
}

fn boundary_bevel_depth_t(radial_t: f32, profile: f32) -> f32 {
    let t = radial_t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let forward = t.powf(profile);
    let reverse = (1.0 - t).powf(profile);
    let denominator = forward + reverse;
    if denominator <= EPSILON || !denominator.is_finite() {
        t
    } else {
        forward / denominator
    }
}

fn lerp_loop_points(from: &[[f32; 2]], to: &[[f32; 2]], t: f32) -> Vec<[f32; 2]> {
    from.iter()
        .zip(to)
        .map(|(from, to)| [lerp(from[0], to[0], t), lerp(from[1], to[1], t)])
        .collect()
}

fn offset_loop_points(points: &[[f32; 2]], center: [f32; 2], offset: f32) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|point| {
            let direction =
                normalize_or_2d([point[0] - center[0], point[1] - center[1]], [1.0, 0.0]);
            [
                point[0] + direction[0] * offset,
                point[1] + direction[1] * offset,
            ]
        })
        .collect()
}

fn normalize_or_2d(value: [f32; 2], fallback: [f32; 2]) -> [f32; 2] {
    let length = (value[0] * value[0] + value[1] * value[1]).sqrt();
    if length <= EPSILON {
        fallback
    } else {
        [value[0] / length, value[1] / length]
    }
}

#[allow(clippy::too_many_arguments)]
fn add_cut_wall_band(
    builder: &mut MeshBuilder,
    front_ring: &[u32],
    back_ring: &[u32],
    points: &[[f32; 2]],
    center: [f32; 2],
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
) {
    for index in 0..front_ring.len() {
        let next = (index + 1) % front_ring.len();
        let midpoint = [
            (points[index][0] + points[next][0]) * 0.5,
            0.0,
            (points[index][1] + points[next][1]) * 0.5,
        ];
        let desired = normalize_or(
            [center[0] - midpoint[0], 0.0, center[1] - midpoint[2]],
            [1.0, 0.0, 0.0],
        );
        add_oriented_face(
            builder,
            vec![
                front_ring[index],
                front_ring[next],
                back_ring[next],
                back_ring[index],
            ],
            desired,
            cut_metadata(context, region, SurfaceRole::CutWall, operation, None),
        );
    }
}

fn add_cap_oriented(
    builder: &mut MeshBuilder,
    vertices: Vec<u32>,
    desired_normal: [f32; 3],
    metadata: FaceMetadata,
) {
    add_oriented_face(builder, vertices, desired_normal, metadata);
}

fn add_oriented_face(
    builder: &mut MeshBuilder,
    mut vertices: Vec<u32>,
    desired_normal: [f32; 3],
    metadata: FaceMetadata,
) {
    if face_normal_dot(&builder.positions, &vertices, desired_normal) < 0.0 {
        vertices.reverse();
    }
    builder.add_face(vertices, metadata);
}

fn add_oriented_face_if_non_degenerate(
    builder: &mut MeshBuilder,
    vertices: Vec<u32>,
    desired_normal: [f32; 3],
    metadata: FaceMetadata,
) {
    if has_duplicate_indices(&vertices)
        || vertices.len() < 3
        || face_normal_dot(&builder.positions, &vertices, desired_normal).abs() <= EPSILON
    {
        return;
    }
    add_oriented_face(builder, vertices, desired_normal, metadata);
}

fn face_normal_dot(positions: &[[f32; 3]], vertices: &[u32], desired: [f32; 3]) -> f32 {
    let mut normal = [0.0; 3];
    for index in 0..vertices.len() {
        let current = positions[vertices[index] as usize];
        let next = positions[vertices[(index + 1) % vertices.len()] as usize];
        normal[0] += (current[1] - next[1]) * (current[2] + next[2]);
        normal[1] += (current[2] - next[2]) * (current[0] + next[0]);
        normal[2] += (current[0] - next[0]) * (current[1] + next[1]);
    }
    dot(normal, desired)
}

fn cut_metadata(
    context: &GeneratorContext,
    region: RegionId,
    surface_role: SurfaceRole,
    operation: OperationId,
    smoothing_group: Option<u32>,
) -> FaceMetadata {
    FaceMetadata {
        part_definition: Some(context.part_definition),
        part_instance: Some(context.part_instance),
        region: Some(region),
        operation: Some(operation),
        smoothing_group,
        surface_role: Some(surface_role),
    }
}

fn mark_boundary_loop(
    mesh: &mut PolygonMesh,
    ring: &[u32],
    operation: OperationId,
    boundary_loop: BoundaryLoopId,
    treatment: CutEdgeTreatment,
) {
    for index in 0..ring.len() {
        let next = (index + 1) % ring.len();
        let key = EdgeKey::new(ring[index], ring[next]);
        if let Some(metadata) = mesh.edge_metadata.get_mut(&key) {
            metadata.boundary_role = BoundaryRole::Feature;
            metadata.classification = EdgeClassification::Hard;
            metadata.seam_candidate = false;
            metadata.bevel_eligible = matches!(treatment, CutEdgeTreatment::BevelEligible);
            metadata.operation = Some(operation);
            metadata.boundary_loop = Some(boundary_loop);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn mark_cut_or_bevel_boundary_loop(
    mesh: &mut PolygonMesh,
    outer_ring: &[u32],
    inner_ring: &[u32],
    cut_operation: OperationId,
    source_loop: BoundaryLoopId,
    treatment: CutEdgeTreatment,
    bevel: Option<&BoundaryLoopBevelPlan>,
) {
    if let Some(bevel) = bevel {
        mark_boundary_loop(
            mesh,
            outer_ring,
            bevel.operation,
            bevel.outer_replacement_loop,
            CutEdgeTreatment::Hard,
        );
        mark_boundary_loop(
            mesh,
            inner_ring,
            bevel.operation,
            bevel.inner_replacement_loop,
            CutEdgeTreatment::Hard,
        );
    } else {
        mark_boundary_loop(mesh, outer_ring, cut_operation, source_loop, treatment);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_cut_or_bevel_boundary_marks(
    marks: &mut Vec<BoundaryLoopMark>,
    outer_ring: Vec<u32>,
    inner_ring: Vec<u32>,
    cut_operation: OperationId,
    source_loop: BoundaryLoopId,
    treatment: CutEdgeTreatment,
    bevel: Option<&BoundaryLoopBevelPlan>,
) {
    if let Some(bevel) = bevel {
        marks.push(BoundaryLoopMark {
            ring: outer_ring,
            operation: bevel.operation,
            boundary_loop: bevel.outer_replacement_loop,
            treatment: CutEdgeTreatment::Hard,
        });
        marks.push(BoundaryLoopMark {
            ring: inner_ring,
            operation: bevel.operation,
            boundary_loop: bevel.inner_replacement_loop,
            treatment: CutEdgeTreatment::Hard,
        });
    } else {
        marks.push(BoundaryLoopMark {
            ring: outer_ring,
            operation: cut_operation,
            boundary_loop: source_loop,
            treatment,
        });
    }
}

fn insert_cut_region(
    regions: &mut BTreeMap<RegionId, SurfaceRegionSpec>,
    id: RegionId,
    name: &'static str,
    role: SurfaceRole,
) {
    regions.entry(id).or_insert_with(|| {
        let mut tags = BTreeSet::new();
        tags.insert("cut".to_owned());
        tags.insert(name.replace('_', "-"));
        SurfaceRegionSpec {
            id,
            name: name.to_owned(),
            role,
            tags,
        }
    });
}

fn insert_boundary_bevel_region(
    regions: &mut BTreeMap<RegionId, SurfaceRegionSpec>,
    bevel: Option<&BoundaryLoopBevelPlan>,
) {
    if let Some(bevel) = bevel {
        insert_cut_region(
            regions,
            bevel.bevel_region,
            "boundary_loop_bevel",
            SurfaceRole::BevelBand,
        );
    }
}

fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    if length <= EPSILON {
        fallback
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}

fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

#[derive(Debug, Clone)]
struct Ring {
    y: f32,
    vertices: RingVertices,
    incoming_region: RegionId,
}

#[derive(Debug, Clone)]
enum RingVertices {
    Apex(u32),
    Circle(Vec<u32>),
}

#[derive(Debug, Copy, Clone)]
enum FaceSide {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl FaceSide {
    const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    #[must_use]
    fn fixed_axis(self) -> usize {
        match self {
            Self::PositiveX | Self::NegativeX => 0,
            Self::PositiveY | Self::NegativeY => 1,
            Self::PositiveZ | Self::NegativeZ => 2,
        }
    }

    #[must_use]
    fn sign(self) -> f32 {
        match self {
            Self::PositiveX | Self::PositiveY | Self::PositiveZ => 1.0,
            Self::NegativeX | Self::NegativeY | Self::NegativeZ => -1.0,
        }
    }

    #[must_use]
    fn tangent_axes(self) -> [usize; 2] {
        match self {
            Self::PositiveX => [1, 2],
            Self::NegativeX => [2, 1],
            Self::PositiveY => [2, 0],
            Self::NegativeY => [0, 2],
            Self::PositiveZ => [0, 1],
            Self::NegativeZ => [1, 0],
        }
    }
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    vertex_ids: Vec<ElementId>,
    vertex_lookup: BTreeMap<VertexKey, u32>,
    faces: Vec<PolygonFace>,
    face_metadata: Vec<FaceMetadata>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            vertex_ids: Vec::new(),
            vertex_lookup: BTreeMap::new(),
            faces: Vec::new(),
            face_metadata: Vec::new(),
        }
    }

    fn add_vertices(&mut self, positions: &[[f32; 3]]) -> Result<Vec<u32>, ModelingError> {
        positions
            .iter()
            .copied()
            .map(|position| self.add_vertex(position))
            .collect()
    }

    fn add_vertex(&mut self, position: [f32; 3]) -> Result<u32, ModelingError> {
        if !position.iter().copied().all(f32::is_finite) {
            return Err(ModelingError::InvalidInput(
                "generated non-finite vertex position".to_owned(),
            ));
        }
        let key = VertexKey::from_position(position);
        if let Some(index) = self.vertex_lookup.get(&key) {
            return Ok(*index);
        }
        let index = u32::try_from(self.positions.len()).map_err(|_| {
            ModelingError::InvalidInput("generated mesh exceeded u32 index range".to_owned())
        })?;
        self.positions.push(position);
        self.vertex_ids.push(ElementId(u64::from(index)));
        self.vertex_lookup.insert(key, index);
        Ok(index)
    }

    fn add_ring(
        &mut self,
        y: f32,
        radius: f32,
        radial_segments: u32,
        incoming_region: RegionId,
    ) -> Result<Ring, ModelingError> {
        if radius <= EPSILON {
            let vertex = self.add_vertex([0.0, y, 0.0])?;
            return Ok(Ring {
                y,
                vertices: RingVertices::Apex(vertex),
                incoming_region,
            });
        }
        let mut vertices = Vec::new();
        for index in 0..radial_segments {
            let angle = 2.0 * PI * index as f32 / radial_segments as f32;
            let (sin, cos) = angle.sin_cos();
            vertices.push(self.add_vertex([radius * cos, y, radius * sin])?);
        }
        Ok(Ring {
            y,
            vertices: RingVertices::Circle(vertices),
            incoming_region,
        })
    }

    fn add_plate_ring(&mut self, y: f32, points: &[[f32; 2]]) -> Result<Vec<u32>, ModelingError> {
        points
            .iter()
            .map(|point| self.add_vertex([point[0], y, point[1]]))
            .collect()
    }

    fn add_face(&mut self, vertices: Vec<u32>, metadata: FaceMetadata) {
        if has_duplicate_indices(&vertices) || vertices.len() < 3 {
            return;
        }
        let id = ElementId(self.faces.len() as u64);
        self.faces.push(PolygonFace { id, vertices });
        self.face_metadata.push(metadata);
    }

    fn finish(self) -> Result<PolygonMesh, ModelingError> {
        let bounds = bounds_from_positions(&self.positions)?;
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
        mesh.edge_metadata = build_edge_metadata(&mesh);
        Ok(mesh)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VertexKey(i64, i64, i64);

impl VertexKey {
    fn from_position(position: [f32; 3]) -> Self {
        Self(
            quantize(position[0]),
            quantize(position[1]),
            quantize(position[2]),
        )
    }
}

fn build_edge_metadata(mesh: &PolygonMesh) -> BTreeMap<EdgeKey, EdgeMetadata> {
    let mut edge_faces: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for index in 0..face.vertices.len() {
            let next = (index + 1) % face.vertices.len();
            edge_faces
                .entry(EdgeKey::new(face.vertices[index], face.vertices[next]))
                .or_default()
                .push(face_index);
        }
    }

    edge_faces
        .into_iter()
        .map(|(edge, faces)| {
            let metadata = if faces.len() == 1 {
                EdgeMetadata {
                    boundary_role: BoundaryRole::OpenBoundary,
                    classification: EdgeClassification::Hard,
                    seam_candidate: false,
                    bevel_eligible: false,
                    operation: None,
                    region_transition: None,
                    boundary_loop: None,
                }
            } else {
                let first = &mesh.face_metadata[faces[0]];
                let second = &mesh.face_metadata[faces[1]];
                let smooth = first.smoothing_group.is_some()
                    && first.smoothing_group == second.smoothing_group;
                EdgeMetadata {
                    boundary_role: if smooth {
                        BoundaryRole::Smooth
                    } else {
                        BoundaryRole::Hard
                    },
                    classification: if smooth {
                        EdgeClassification::Smooth
                    } else {
                        EdgeClassification::Hard
                    },
                    seam_candidate: false,
                    bevel_eligible: false,
                    operation: None,
                    region_transition: region_transition(first.region, second.region),
                    boundary_loop: None,
                }
            };
            (edge, metadata)
        })
        .collect()
}

fn rounded_box_position(
    side: FaceSide,
    u: f32,
    v: f32,
    half: [f32; 3],
    inner: [f32; 3],
    radius: f32,
) -> [f32; 3] {
    let mut base = [0.0; 3];
    base[side.fixed_axis()] = side.sign() * half[side.fixed_axis()];
    let [u_axis, v_axis] = side.tangent_axes();
    base[u_axis] = u;
    base[v_axis] = v;
    if radius <= EPSILON {
        return base;
    }
    let closest = [
        base[0].clamp(-inner[0], inner[0]),
        base[1].clamp(-inner[1], inner[1]),
        base[2].clamp(-inner[2], inner[2]),
    ];
    let delta = [
        base[0] - closest[0],
        base[1] - closest[1],
        base[2] - closest[2],
    ];
    let length = dot(delta, delta).sqrt();
    if length <= EPSILON {
        closest
    } else {
        [
            closest[0] + delta[0] * radius / length,
            closest[1] + delta[1] * radius / length,
            closest[2] + delta[2] * radius / length,
        ]
    }
}

fn rounded_box_region(
    axes: [usize; 2],
    center: [f32; 2],
    inner: [f32; 3],
    radius: f32,
) -> RegionId {
    if radius <= EPSILON {
        return ROUNDED_PRIMARY_REGION;
    }
    let outside = axes
        .into_iter()
        .zip(center)
        .filter(|(axis, value)| value.abs() > inner[*axis] + EPSILON)
        .count();
    match outside {
        0 => ROUNDED_PRIMARY_REGION,
        1 => ROUNDED_BEVEL_REGION,
        _ => ROUNDED_CORNER_REGION,
    }
}

fn rounded_box_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        ROUNDED_PRIMARY_REGION => (SurfaceRole::PrimarySurface, None),
        ROUNDED_BEVEL_REGION => (SurfaceRole::BevelBand, Some(1)),
        ROUNDED_CORNER_REGION => (SurfaceRole::Detail, Some(1)),
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn cylinder_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        CYLINDER_SIDE_REGION => (SurfaceRole::Side, Some(2)),
        CYLINDER_TOP_CAP_REGION | CYLINDER_BOTTOM_CAP_REGION => (SurfaceRole::Cap, None),
        CYLINDER_TOP_BEVEL_REGION | CYLINDER_BOTTOM_BEVEL_REGION => {
            (SurfaceRole::BevelBand, Some(1))
        }
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn plate_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        PLATE_FRONT_REGION | PLATE_BACK_REGION => (SurfaceRole::PrimarySurface, None),
        PLATE_SIDE_REGION => (SurfaceRole::Side, Some(2)),
        PLATE_BEVEL_REGION => (SurfaceRole::BevelBand, Some(1)),
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn metadata(
    context: &GeneratorContext,
    region: RegionId,
    surface_role: SurfaceRole,
    smoothing_group: Option<u32>,
) -> FaceMetadata {
    FaceMetadata {
        part_definition: Some(context.part_definition),
        part_instance: Some(context.part_instance),
        region: Some(region),
        operation: None,
        smoothing_group,
        surface_role: Some(surface_role),
    }
}

fn axis_samples(
    half: f32,
    inner: f32,
    radius: f32,
    bevel_segments: u32,
    face_subdivisions: u32,
) -> Vec<f32> {
    let mut samples = Vec::new();
    if radius <= EPSILON {
        for index in 0..=face_subdivisions {
            samples.push(lerp(-half, half, index as f32 / face_subdivisions as f32));
        }
    } else {
        for index in 0..=bevel_segments {
            samples.push(lerp(-half, -inner, index as f32 / bevel_segments as f32));
        }
        for index in 1..face_subdivisions {
            samples.push(lerp(-inner, inner, index as f32 / face_subdivisions as f32));
        }
        for index in 0..=bevel_segments {
            samples.push(lerp(inner, half, index as f32 / bevel_segments as f32));
        }
    }
    dedup_sorted_f32(samples)
}

fn rounded_rect_points(
    half_x: f32,
    half_z: f32,
    radius: f32,
    corner_segments: u32,
) -> Vec<[f32; 2]> {
    if radius <= EPSILON {
        return vec![
            [half_x, half_z],
            [-half_x, half_z],
            [-half_x, -half_z],
            [half_x, -half_z],
        ];
    }
    let centers = [
        [half_x - radius, half_z - radius],
        [-half_x + radius, half_z - radius],
        [-half_x + radius, -half_z + radius],
        [half_x - radius, -half_z + radius],
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

fn clamp_frustum_bevels(
    bottom: f32,
    top: f32,
    bottom_radius: f32,
    top_radius: f32,
    height: f32,
) -> (f32, f32) {
    let mut bottom = bottom.min(bottom_radius);
    let mut top = top.min(top_radius);
    let sum = bottom + top;
    if sum > height && sum > EPSILON {
        let scale = height / sum;
        bottom *= scale;
        top *= scale;
    }
    (bottom, top)
}

fn rounded_box_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (
            ROUNDED_PRIMARY_REGION,
            "primary_faces",
            SurfaceRole::PrimarySurface,
        ),
        (ROUNDED_BEVEL_REGION, "bevel_bands", SurfaceRole::BevelBand),
        (ROUNDED_CORNER_REGION, "corners", SurfaceRole::Detail),
    ])
}

fn cylinder_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (CYLINDER_SIDE_REGION, "side", SurfaceRole::Side),
        (CYLINDER_TOP_CAP_REGION, "top_cap", SurfaceRole::Cap),
        (CYLINDER_BOTTOM_CAP_REGION, "bottom_cap", SurfaceRole::Cap),
        (
            CYLINDER_TOP_BEVEL_REGION,
            "top_bevel",
            SurfaceRole::BevelBand,
        ),
        (
            CYLINDER_BOTTOM_BEVEL_REGION,
            "bottom_bevel",
            SurfaceRole::BevelBand,
        ),
    ])
}

fn plate_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (PLATE_FRONT_REGION, "front", SurfaceRole::PrimarySurface),
        (PLATE_BACK_REGION, "back", SurfaceRole::PrimarySurface),
        (PLATE_SIDE_REGION, "side", SurfaceRole::Side),
        (PLATE_BEVEL_REGION, "bevel", SurfaceRole::BevelBand),
    ])
}

fn regions<const N: usize>(
    specs: [(RegionId, &'static str, SurfaceRole); N],
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    specs
        .into_iter()
        .map(|(id, name, role)| {
            (
                id,
                SurfaceRegionSpec {
                    id,
                    name: name.to_owned(),
                    role,
                    tags: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn rounded_box_sockets(half: [f32; 3]) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SocketId(1),
            "positive_x",
            [half[0], 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ),
        (
            SocketId(2),
            "negative_x",
            [-half[0], 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
        ),
        (
            SocketId(3),
            "positive_y",
            [0.0, half[1], 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SocketId(4),
            "negative_y",
            [0.0, -half[1], 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
        (
            SocketId(5),
            "positive_z",
            [0.0, 0.0, half[2]],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ),
        (
            SocketId(6),
            "negative_z",
            [0.0, 0.0, -half[2]],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ),
    ])
}

fn cylinder_sockets(half_height: f32) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SOCKET_TOP,
            "top_center",
            [0.0, half_height, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SOCKET_BOTTOM,
            "bottom_center",
            [0.0, -half_height, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
        (
            SOCKET_AXIS,
            "axis_midpoint",
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
    ])
}

fn plate_sockets(half_thickness: f32) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SocketId(1),
            "front_center",
            [0.0, half_thickness, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SocketId(2),
            "back_center",
            [0.0, -half_thickness, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
    ])
}

fn sockets<const N: usize>(specs: [SocketTemplate; N]) -> BTreeMap<SocketId, SocketSpec> {
    specs
        .into_iter()
        .map(|(id, name, origin, x_axis, y_axis, z_axis)| {
            (
                id,
                SocketSpec {
                    id,
                    name: name.to_owned(),
                    local_frame: Frame3 {
                        origin,
                        x_axis,
                        y_axis,
                        z_axis,
                    },
                    role: "attachment".to_owned(),
                    tags: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn part(
    mesh: PolygonMesh,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    generator_signature: String,
) -> GeneratedPart {
    let local_bounds = mesh.bounds;
    GeneratedPart {
        mesh,
        sockets,
        regions,
        local_bounds,
        generator_signature,
    }
}

fn bevel_profile(
    definition: &PartDefinition,
    default_radius: f32,
    default_segments: u32,
) -> (f32, u32) {
    definition.geometry.operations.iter().fold(
        (default_radius, default_segments),
        |profile, operation| match operation {
            ModelingOperationSpec::SetBevelProfile {
                radius, segments, ..
            } => (*radius, *segments),
            _ => profile,
        },
    )
}

fn positive_triplet(values: [f32; 3], label: &'static str) -> Result<[f32; 3], ModelingError> {
    for value in values {
        finite_positive(value, label)?;
    }
    Ok(values)
}

fn finite_positive(value: f32, label: &'static str) -> Result<f32, ModelingError> {
    if value.is_finite() && value > EPSILON {
        Ok(value)
    } else {
        Err(ModelingError::InvalidInput(format!(
            "{label} must be finite and positive"
        )))
    }
}

fn finite_non_negative(value: f32, label: &'static str) -> Result<f32, ModelingError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(ModelingError::InvalidInput(format!(
            "{label} must be finite and non-negative"
        )))
    }
}

fn dedup_sorted_f32(mut values: Vec<f32>) -> Vec<f32> {
    values.sort_by(f32::total_cmp);
    values.dedup_by(|left, right| (*left - *right).abs() <= EPSILON);
    values
}

fn region_transition(
    first: Option<RegionId>,
    second: Option<RegionId>,
) -> Option<(RegionId, RegionId)> {
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(if first <= second {
            (first, second)
        } else {
            (second, first)
        }),
        _ => None,
    }
}

fn has_duplicate_indices(vertices: &[u32]) -> bool {
    let mut seen = BTreeSet::new();
    vertices.iter().any(|vertex| !seen.insert(*vertex))
}

fn quantize(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
