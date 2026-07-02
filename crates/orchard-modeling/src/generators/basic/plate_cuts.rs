
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
                .map(|bevel| cut.offset_inner_points(bevel.width))
                .transpose()?
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
                .map(|bevel| cut.offset_inner_points(-bevel.width))
                .transpose()?
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
                .map(|bevel| cut.offset_inner_points(bevel.width))
                .transpose()?
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
                .map(|bevel| cut.offset_inner_points(bevel.width))
                .transpose()?
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
                .map(|bevel| cut.offset_inner_points(-bevel.width))
                .transpose()?
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
