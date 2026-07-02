
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
    loop_shape: PlateCutLoopShape,
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

#[derive(Debug, Copy, Clone)]
enum PlateCutLoopShape {
    RoundedRect {
        half_extents: [f32; 2],
        corner_radius: f32,
    },
    Circle {
        radius: f32,
    },
}

struct BoundaryLoopMark {
    ring: Vec<u32>,
    operation: OperationId,
    boundary_loop: BoundaryLoopId,
    treatment: CutEdgeTreatment,
}

#[derive(Debug, Copy, Clone)]
struct PlanarHostPatch {
    origin: [f32; 3],
    u_axis: [f32; 3],
    v_axis: [f32; 3],
    outward_normal: [f32; 3],
    u_axis_index: usize,
    v_axis_index: usize,
    half_u: f32,
    half_v: f32,
    thickness: f32,
}

impl PlanarHostPatch {
    fn rounded_box(
        face: PlanarCutFace,
        half: [f32; 3],
        radius: f32,
    ) -> Result<Self, ModelingError> {
        let (fixed_axis, sign, u_axis_index, v_axis_index) = match face {
            PlanarCutFace::PositiveX => (0, 1.0, 2, 1),
            PlanarCutFace::NegativeX => (0, -1.0, 2, 1),
            PlanarCutFace::PositiveY => (1, 1.0, 0, 2),
            PlanarCutFace::NegativeY => (1, -1.0, 0, 2),
            PlanarCutFace::PositiveZ => (2, 1.0, 0, 1),
            PlanarCutFace::NegativeZ => (2, -1.0, 0, 1),
        };
        let half_u = (half[u_axis_index] - radius).max(0.0);
        let half_v = (half[v_axis_index] - radius).max(0.0);
        if half_u <= EPSILON || half_v <= EPSILON {
            return Err(ModelingError::UnsupportedOperation {
                operation: OperationId(0),
                reason: "rounded-box cuts require a non-zero flat primary face patch".to_owned(),
            });
        }
        let mut origin = [0.0; 3];
        origin[fixed_axis] = sign * half[fixed_axis];
        let mut u_axis = [0.0; 3];
        u_axis[u_axis_index] = 1.0;
        let mut v_axis = [0.0; 3];
        v_axis[v_axis_index] = 1.0;
        let mut outward_normal = [0.0; 3];
        outward_normal[fixed_axis] = sign;
        Ok(Self {
            origin,
            u_axis,
            v_axis,
            outward_normal,
            u_axis_index,
            v_axis_index,
            half_u,
            half_v,
            thickness: half[fixed_axis] * 2.0,
        })
    }

    fn position(self, depth: f32, point: [f32; 2]) -> [f32; 3] {
        [
            self.origin[0] + self.u_axis[0] * point[0] + self.v_axis[0] * point[1]
                - self.outward_normal[0] * depth,
            self.origin[1] + self.u_axis[1] * point[0] + self.v_axis[1] * point[1]
                - self.outward_normal[1] * depth,
            self.origin[2] + self.u_axis[2] * point[0] + self.v_axis[2] * point[1]
                - self.outward_normal[2] * depth,
        ]
    }
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
                let width = finite_positive(size[0], "cut.size.x")?;
                let height = finite_positive(size[1], "cut.size.y")?;
                let corner_radius = finite_non_negative(*corner_radius, "cut.corner_radius")?;
                let corner_segments = (*corner_segments).max(1);
                let inner_points = rounded_cut_points(
                    *center,
                    *size,
                    corner_radius,
                    corner_segments,
                    CutPointCount::RoundedRect,
                )?;
                let frame = cut_frame_rect(*center, &inner_points, half_x, half_z, rim_width)?;
                let frame_points =
                    rounded_frame_points(*center, *size, corner_radius, corner_segments, frame)?;
                Ok(Self {
                    kind: PlateCutKind::Recessed {
                        depth,
                        floor_region: *floor_region,
                    },
                    operation: *operation,
                    face: *face,
                    center: *center,
                    loop_shape: PlateCutLoopShape::RoundedRect {
                        half_extents: [width * 0.5, height * 0.5],
                        corner_radius,
                    },
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
                let width = finite_positive(size[0], "cut.size.x")?;
                let height = finite_positive(size[1], "cut.size.y")?;
                let corner_radius = finite_non_negative(*corner_radius, "cut.corner_radius")?;
                let corner_segments = (*corner_segments).max(1);
                let inner_points = rounded_cut_points(
                    *center,
                    *size,
                    corner_radius,
                    corner_segments,
                    CutPointCount::RoundedRect,
                )?;
                let frame = cut_frame_rect(*center, &inner_points, half_x, half_z, rim_width)?;
                let frame_points =
                    rounded_frame_points(*center, *size, corner_radius, corner_segments, frame)?;
                Ok(Self {
                    kind: PlateCutKind::Through,
                    operation: *operation,
                    face: *face,
                    center: *center,
                    loop_shape: PlateCutLoopShape::RoundedRect {
                        half_extents: [width * 0.5, height * 0.5],
                        corner_radius,
                    },
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
                    loop_shape: PlateCutLoopShape::Circle { radius },
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

    fn offset_inner_points(&self, offset: f32) -> Result<Vec<[f32; 2]>, ModelingError> {
        offset_loop_points(self.loop_shape, self.center, self.corner_segments, offset)
    }
}
