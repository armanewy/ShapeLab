
fn build_cut_rounded_box(
    params: &RoundedBoxParams,
    operations: &[&ModelingOperationSpec],
    bevels: &[BoundaryLoopBevelPlan],
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

    let first_operation = operations.first().ok_or_else(|| {
        ModelingError::InvalidInput(
            "rounded-box cut generation requires at least one cut".to_owned(),
        )
    })?;
    let face = operation_face(first_operation)?;
    let host = PlanarHostPatch::rounded_box(face, half, radius)?;
    let cuts = operations
        .iter()
        .map(|operation| {
            if operation_face(operation)? != face {
                return Err(ModelingError::UnsupportedOperation {
                    operation: operation.operation_id(),
                    reason: "rounded-box cut composition currently supports one selected face per definition"
                        .to_owned(),
                });
            }
            let cut = PlateCutPlan::from_operation(operation, host.half_u, host.half_v, host.thickness)?;
            Ok(cut)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let primary_region = cuts
        .first()
        .expect("at least one rounded-box cut should exist")
        .target_region;
    for cut in &cuts {
        if cut.target_region != cut.outer_region || cut.target_region != primary_region {
            return Err(ModelingError::InvalidInput(
                "rounded-box cuts must target one shared primary flat face region".to_owned(),
            ));
        }
    }
    let mut cuts = cuts;
    apply_boundary_loop_bevels(&mut cuts, bevels, host.thickness)?;
    validate_cut_frame_clearance(&cuts)?;

    let mut axis_samples = [
        axis_samples(half[0], inner[0], radius, bevel_segments, face_subdivisions),
        axis_samples(half[1], inner[1], radius, bevel_segments, face_subdivisions),
        axis_samples(half[2], inner[2], radius, bevel_segments, face_subdivisions),
    ];
    for cut in &cuts {
        axis_samples[host.u_axis_index].push(cut.frame.min_x);
        axis_samples[host.u_axis_index].push(cut.frame.max_x);
        axis_samples[host.v_axis_index].push(cut.frame.min_z);
        axis_samples[host.v_axis_index].push(cut.frame.max_z);
    }
    for samples in &mut axis_samples {
        *samples = dedup_sorted_f32(std::mem::take(samples));
    }

    let entry_side = face_side_for_cut(face);
    let opposite_side = entry_side.opposite();
    let through_cuts = cuts
        .iter()
        .filter(|cut| matches!(cut.kind, PlateCutKind::Through))
        .cloned()
        .collect::<Vec<_>>();
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
                let local_region = rounded_box_region(
                    [u_axis, v_axis],
                    [
                        (u_samples[u] + u_samples[u + 1]) * 0.5,
                        (v_samples[v] + v_samples[v + 1]) * 0.5,
                    ],
                    inner,
                    radius,
                );
                if local_region == ROUNDED_PRIMARY_REGION
                    && (side == entry_side || (side == opposite_side && !through_cuts.is_empty()))
                {
                    continue;
                }
                let region = rounded_box_region_for_primary(local_region, primary_region);
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
                builder.add_face(
                    vertices,
                    rounded_box_metadata_for_primary(context, region, primary_region),
                );
            }
        }
    }

    add_cut_patch_for_host_face(
        &mut builder,
        &host,
        &cuts,
        &cuts,
        HostPatchSurface {
            depth: 0.0,
            desired_normal: host.outward_normal,
            region: primary_region,
        },
        context,
    )?;
    if !through_cuts.is_empty() {
        add_cut_patch_for_host_face(
            &mut builder,
            &host,
            &through_cuts,
            &cuts,
            HostPatchSurface {
                depth: host.thickness,
                desired_normal: negate(host.outward_normal),
                region: primary_region,
            },
            context,
        )?;
    }

    let mut boundary_marks = Vec::new();
    let mut entry_wall_rings: BTreeMap<OperationId, Vec<u32>> = BTreeMap::new();
    let mut secondary_wall_rings: BTreeMap<OperationId, Vec<u32>> = BTreeMap::new();
    let xs = plate_cut_axis_samples(host.half_u, &cuts, 0);
    let zs = plate_cut_axis_samples(host.half_v, &cuts, 1);
    for cut in &cuts {
        let (entry_surface_ring, entry_wall_ring) = add_cut_features_for_host_face(
            &mut builder,
            cut,
            &host,
            0.0,
            host.outward_normal,
            &xs,
            &zs,
            primary_region,
            context,
            cut.entry_bevel.as_ref(),
        )?;
        push_cut_or_bevel_boundary_marks(
            &mut boundary_marks,
            entry_surface_ring,
            entry_wall_ring.clone(),
            cut.operation,
            cut.entry_loop,
            cut.edge_treatment,
            cut.entry_bevel.as_ref(),
        );
        entry_wall_rings.insert(cut.operation, entry_wall_ring);
        if matches!(cut.kind, PlateCutKind::Through) {
            let (secondary_surface_ring, secondary_wall_ring) = add_cut_features_for_host_face(
                &mut builder,
                cut,
                &host,
                host.thickness,
                negate(host.outward_normal),
                &xs,
                &zs,
                primary_region,
                context,
                cut.secondary_bevel.as_ref(),
            )?;
            push_cut_or_bevel_boundary_marks(
                &mut boundary_marks,
                secondary_surface_ring,
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
            let outside_inner = entry_wall_rings
                .get(&cut.operation)
                .cloned()
                .expect("entry wall ring should be generated before recessed wall");
            let floor_surface_points = cut
                .secondary_bevel
                .as_ref()
                .map(|bevel| cut.offset_inner_points(-bevel.width))
                .transpose()?
                .unwrap_or_else(|| cut.inner_points.clone());
            let (wall_bottom_ring, floor_inner) = if let Some(bevel) = &cut.secondary_bevel {
                add_boundary_loop_bevel_band_on_host(
                    &mut builder,
                    &host,
                    depth - bevel.width,
                    &cut.inner_points,
                    depth,
                    &floor_surface_points,
                    host.outward_normal,
                    context,
                    bevel,
                )?
            } else {
                let ring = builder.add_host_ring(&host, depth, &cut.inner_points)?;
                (ring.clone(), ring)
            };
            add_cut_wall_band_on_host(
                &mut builder,
                &outside_inner,
                &wall_bottom_ring,
                &cut.inner_points,
                cut.center,
                &host,
                context,
                cut.operation,
                cut.wall_region,
            );
            add_cap_oriented(
                &mut builder,
                floor_inner.clone(),
                host.outward_normal,
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
        }
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
        add_cut_wall_band_on_host(
            &mut builder,
            outside_inner,
            opposite_inner,
            &cut.inner_points,
            cut.center,
            &host,
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
    let mut regions = rounded_box_regions_for_primary(primary_region);
    for cut in &cuts {
        insert_cut_region(&mut regions, cut.rim_region, "cut_rim", SurfaceRole::Rim);
        insert_cut_region(
            &mut regions,
            cut.wall_region,
            "cut_wall",
            SurfaceRole::CutWall,
        );
        if let PlateCutKind::Recessed { floor_region, .. } = cut.kind {
            insert_cut_region(
                &mut regions,
                floor_region,
                "recess_floor",
                SurfaceRole::Interior,
            );
        }
        insert_boundary_bevel_region(&mut regions, cut.entry_bevel.as_ref());
        insert_boundary_bevel_region(&mut regions, cut.secondary_bevel.as_ref());
    }

    Ok(part(
        mesh,
        regions,
        rounded_box_sockets(half),
        format!(
            "rounded_box_cut:h={:.6},{:.6},{:.6}:r={:.6}:bs={}:face={:?}:cuts={}",
            half[0],
            half[1],
            half[2],
            radius,
            bevel_segments,
            face,
            cuts.iter()
                .map(|cut| cut.operation.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
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
