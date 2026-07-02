
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
                    "BevelBoundaryLoop target {} is not a supported cut loop",
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
            if !entry_loop
                && let PlateCutLoopShape::RoundedRect { corner_radius, .. } = cut.loop_shape
                && corner_radius > EPSILON
                && bevel.width >= corner_radius - EPSILON
            {
                return Err(ModelingError::UnsupportedOperation {
                    operation: bevel.operation,
                    reason: "floor boundary-loop bevel width must be smaller than the target rounded corner radius".to_owned(),
                });
            }
            let entry_width = if entry_loop {
                bevel.width
            } else {
                cut.entry_bevel.as_ref().map_or(0.0, |entry| entry.width)
            };
            let floor_width = if entry_loop {
                cut.secondary_bevel
                    .as_ref()
                    .map_or(0.0, |floor| floor.width)
            } else {
                bevel.width
            };
            if entry_width + floor_width >= depth - EPSILON {
                return Err(ModelingError::UnsupportedOperation {
                    operation: bevel.operation,
                    reason: "opposing recessed bevels must leave vertical cut wall height"
                        .to_owned(),
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
    (ranges_overlap_open(frame.min_z, frame.max_z, other.min_z, other.max_z)
        && (value_inside_open_interval(other.min_x, frame.min_x, frame.max_x)
            || value_inside_open_interval(other.max_x, frame.min_x, frame.max_x)))
        || (ranges_overlap_open(frame.min_x, frame.max_x, other.min_x, other.max_x)
            && (value_inside_open_interval(other.min_z, frame.min_z, frame.max_z)
                || value_inside_open_interval(other.max_z, frame.min_z, frame.max_z)))
}

fn value_inside_open_interval(value: f32, min: f32, max: f32) -> bool {
    value > min + EPSILON && value < max - EPSILON
}

fn ranges_overlap_open(first_min: f32, first_max: f32, second_min: f32, second_max: f32) -> bool {
    first_min < second_max - EPSILON && second_min < first_max - EPSILON
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
        .map(|bevel| cut.offset_inner_points(bevel.width))
        .transpose()?
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

#[derive(Debug, Copy, Clone)]
struct HostPatchSurface {
    depth: f32,
    desired_normal: [f32; 3],
    region: RegionId,
}

fn add_cut_patch_for_host_face(
    builder: &mut MeshBuilder,
    host: &PlanarHostPatch,
    holes: &[PlateCutPlan],
    sample_cuts: &[PlateCutPlan],
    surface: HostPatchSurface,
    context: &GeneratorContext,
) -> Result<(), ModelingError> {
    let xs = plate_cut_axis_samples(host.half_u, sample_cuts, 0);
    let zs = plate_cut_axis_samples(host.half_v, sample_cuts, 1);
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
            let vertices = builder.add_host_ring(host, surface.depth, &points)?;
            add_oriented_face(
                builder,
                vertices,
                surface.desired_normal,
                rounded_box_metadata(context, surface.region),
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_cut_features_for_host_face(
    builder: &mut MeshBuilder,
    cut: &PlateCutPlan,
    host: &PlanarHostPatch,
    depth: f32,
    desired_normal: [f32; 3],
    xs: &[f32],
    zs: &[f32],
    host_region: RegionId,
    context: &GeneratorContext,
    bevel: Option<&BoundaryLoopBevelPlan>,
) -> Result<(Vec<u32>, Vec<u32>), ModelingError> {
    let window_points = frame_boundary_points(cut.frame, xs, zs);
    let window_ring = builder.add_host_ring(host, depth, &window_points)?;
    let surface_points = bevel
        .map(|bevel| cut.offset_inner_points(bevel.width))
        .transpose()?
        .unwrap_or_else(|| cut.inner_points.clone());
    let surface_ring = builder.add_host_ring(host, depth, &surface_points)?;

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
            add_boundary_loop_bevel_band_from_outer_on_host(
                builder,
                host,
                surface_ring,
                depth,
                &surface_points,
                depth + dot(desired_normal, host.outward_normal) * bevel.width,
                &cut.inner_points,
                desired_normal,
                context,
                bevel,
            )
        } else {
            Ok((surface_ring.clone(), surface_ring))
        };
    }

    let rim_ring = builder.add_host_ring(host, depth, &cut.rim_points)?;
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
        add_boundary_loop_bevel_band_from_outer_on_host(
            builder,
            host,
            surface_ring,
            depth,
            &surface_points,
            depth + dot(desired_normal, host.outward_normal) * bevel.width,
            &cut.inner_points,
            desired_normal,
            context,
            bevel,
        )
    } else {
        Ok((surface_ring.clone(), surface_ring))
    }
}

#[allow(clippy::too_many_arguments)]
fn add_boundary_loop_bevel_band_on_host(
    builder: &mut MeshBuilder,
    host: &PlanarHostPatch,
    outer_depth: f32,
    outer_points: &[[f32; 2]],
    inner_depth: f32,
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
    let outer_ring = builder.add_host_ring(host, outer_depth, outer_points)?;
    add_boundary_loop_bevel_band_from_outer_on_host(
        builder,
        host,
        outer_ring,
        outer_depth,
        outer_points,
        inner_depth,
        inner_points,
        desired_normal,
        context,
        bevel,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_boundary_loop_bevel_band_from_outer_on_host(
    builder: &mut MeshBuilder,
    host: &PlanarHostPatch,
    outer_ring: Vec<u32>,
    outer_depth: f32,
    outer_points: &[[f32; 2]],
    inner_depth: f32,
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
        let t = step as f32 / bevel.segments as f32;
        let (radial_t, depth_t) = boundary_bevel_curve_t(t, bevel.profile);
        let current_points = lerp_loop_points(outer_points, inner_points, radial_t);
        let current_depth = lerp(outer_depth, inner_depth, depth_t);
        let current_ring = builder.add_host_ring(host, current_depth, &current_points)?;
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

#[allow(clippy::too_many_arguments)]
fn add_cut_wall_band_on_host(
    builder: &mut MeshBuilder,
    front_ring: &[u32],
    back_ring: &[u32],
    points: &[[f32; 2]],
    center: [f32; 2],
    host: &PlanarHostPatch,
    context: &GeneratorContext,
    operation: OperationId,
    region: RegionId,
) {
    for index in 0..front_ring.len() {
        let next = (index + 1) % front_ring.len();
        let midpoint = [
            (points[index][0] + points[next][0]) * 0.5,
            (points[index][1] + points[next][1]) * 0.5,
        ];
        let desired = normalize_or(
            [
                host.u_axis[0] * (center[0] - midpoint[0])
                    + host.v_axis[0] * (center[1] - midpoint[1]),
                host.u_axis[1] * (center[0] - midpoint[0])
                    + host.v_axis[1] * (center[1] - midpoint[1]),
                host.u_axis[2] * (center[0] - midpoint[0])
                    + host.v_axis[2] * (center[1] - midpoint[1]),
            ],
            host.u_axis,
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
