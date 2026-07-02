
fn operation_face(operation: &ModelingOperationSpec) -> Result<PlanarCutFace, ModelingError> {
    match operation {
        ModelingOperationSpec::RecessedPanelCut { face, .. }
        | ModelingOperationSpec::RectangularThroughCut { face, .. }
        | ModelingOperationSpec::CircularThroughCut { face, .. } => Ok(*face),
        _ => Err(ModelingError::InvalidInput(
            "expected semantic cut operation".to_owned(),
        )),
    }
}

fn face_side_for_cut(face: PlanarCutFace) -> FaceSide {
    match face {
        PlanarCutFace::PositiveX => FaceSide::PositiveX,
        PlanarCutFace::NegativeX => FaceSide::NegativeX,
        PlanarCutFace::PositiveY => FaceSide::PositiveY,
        PlanarCutFace::NegativeY => FaceSide::NegativeY,
        PlanarCutFace::PositiveZ => FaceSide::PositiveZ,
        PlanarCutFace::NegativeZ => FaceSide::NegativeZ,
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
        let t = step as f32 / bevel.segments as f32;
        let (radial_t, depth_t) = boundary_bevel_curve_t(t, bevel.profile);
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

fn boundary_bevel_curve_t(linear_t: f32, profile: f32) -> (f32, f32) {
    let t = linear_t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return (t, t);
    }
    let profile = profile.clamp(BOUNDARY_BEVEL_PROFILE_MIN, BOUNDARY_BEVEL_PROFILE_MAX);
    if (profile - 1.0).abs() <= EPSILON {
        return (t, t);
    }
    let radial_t = t.powf(1.0 / profile);
    let depth_t = t.powf(profile);
    (radial_t.clamp(0.0, 1.0), depth_t.clamp(0.0, 1.0))
}

fn lerp_loop_points(from: &[[f32; 2]], to: &[[f32; 2]], t: f32) -> Vec<[f32; 2]> {
    from.iter()
        .zip(to)
        .map(|(from, to)| [lerp(from[0], to[0], t), lerp(from[1], to[1], t)])
        .collect()
}

fn offset_loop_points(
    shape: PlateCutLoopShape,
    center: [f32; 2],
    segments: u32,
    offset: f32,
) -> Result<Vec<[f32; 2]>, ModelingError> {
    let local = match shape {
        PlateCutLoopShape::Circle { radius } => {
            let radius = radius + offset;
            if radius <= EPSILON {
                return Err(ModelingError::InvalidInput(
                    "boundary-loop bevel offset collapses circular cut radius".to_owned(),
                ));
            }
            circle_points(radius, segments.max(6))
        }
        PlateCutLoopShape::RoundedRect {
            half_extents,
            corner_radius,
        } => {
            let half_x = half_extents[0] + offset;
            let half_z = half_extents[1] + offset;
            if half_x <= EPSILON || half_z <= EPSILON {
                return Err(ModelingError::InvalidInput(
                    "boundary-loop bevel offset collapses rectangular cut extents".to_owned(),
                ));
            }
            let radius = if corner_radius <= EPSILON {
                0.0
            } else {
                let radius = corner_radius + offset;
                if radius <= EPSILON {
                    return Err(ModelingError::InvalidInput(
                        "boundary-loop bevel offset collapses rounded cut corner radius".to_owned(),
                    ));
                }
                radius
            };
            if radius > half_x.min(half_z) + EPSILON {
                return Err(ModelingError::InvalidInput(
                    "boundary-loop bevel offset exceeds rounded cut half extents".to_owned(),
                ));
            }
            rounded_rect_points(half_x, half_z, radius, segments.max(1))
        }
    };
    Ok(local
        .into_iter()
        .map(|point| [point[0] + center[0], point[1] + center[1]])
        .collect())
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
