
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
    let cut_operations = cut_operations(definition);
    let boundary_loop_bevels = boundary_loop_bevel_operations(definition)?;
    if !cut_operations.is_empty() {
        return build_cut_rounded_box(&params, &cut_operations, &boundary_loop_bevels, context);
    }
    if let Some(bevel) = boundary_loop_bevels.first() {
        return Err(ModelingError::UnsupportedOperation {
            operation: bevel.operation,
            reason:
                "BevelBoundaryLoop requires a supported cut operation in the same RoundedBox definition"
                    .to_owned(),
        });
    }
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
