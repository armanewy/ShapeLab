
fn normalize_closed_profile(profile: &[[f32; 2]]) -> Result<Vec<[f32; 2]>, ModelingError> {
    if profile.len() < 3 {
        return invalid_input("sweep profile requires at least three points");
    }
    let mut normalized = profile.to_vec();
    if points2_close(
        normalized[0],
        *normalized.last().expect("profile is not empty"),
    ) {
        normalized.pop();
    }
    if normalized.len() < 3 {
        return invalid_input("sweep profile requires at least three unique points");
    }
    for point in &normalized {
        if !point[0].is_finite() || !point[1].is_finite() {
            return invalid_input("sweep profile points must be finite");
        }
    }
    for index in 0..normalized.len() {
        if points2_close(
            normalized[index],
            normalized[(index + 1) % normalized.len()],
        ) {
            return invalid_input("sweep profile cannot contain collapsed edges");
        }
    }
    let area = signed_area(&normalized);
    if area.abs() <= EPSILON {
        return invalid_input("sweep profile area must be non-zero");
    }
    if area < 0.0 {
        let first = normalized[0];
        let mut rewound = Vec::with_capacity(normalized.len());
        rewound.push(first);
        rewound.extend(normalized[1..].iter().rev().copied());
        normalized = rewound;
    }
    Ok(normalized)
}

fn normalize_lathe_profile(profile: &[[f32; 2]]) -> Result<Vec<LathePoint>, ModelingError> {
    if profile.len() < 2 {
        return invalid_input("lathe profile requires at least two points");
    }
    let mut normalized = Vec::with_capacity(profile.len());
    for point in profile {
        if !point[0].is_finite() || !point[1].is_finite() {
            return invalid_input("lathe profile points must be finite");
        }
        if point[0] < -EPSILON {
            return invalid_input("lathe radii must be non-negative");
        }
        normalized.push(LathePoint {
            radius: point[0].max(0.0),
            height: point[1],
        });
    }
    for pair in normalized.windows(2) {
        if (pair[0].radius - pair[1].radius).abs() <= EPSILON
            && (pair[0].height - pair[1].height).abs() <= EPSILON
        {
            return invalid_input("lathe profile cannot contain collapsed segments");
        }
    }
    Ok(normalized)
}

fn optional_values(
    values: &[f32],
    count: usize,
    default: f32,
    label: &str,
) -> Result<Vec<f32>, ModelingError> {
    if values.is_empty() {
        return Ok(vec![default; count]);
    }
    if values.len() != count {
        return invalid_input(&format!(
            "{label} must be empty or match the path sample count"
        ));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return invalid_input(&format!("{label} must be finite"));
    }
    Ok(values.to_vec())
}

fn parallel_transport_frames(
    path: &[[f32; 3]],
    up_hint: [f32; 3],
    roll_degrees: &[f32],
    closed: bool,
) -> Result<Vec<FrameBasis>, ModelingError> {
    let min_samples = if closed { 3 } else { 2 };
    if path.len() < min_samples {
        return invalid_input("sweep path has too few samples");
    }
    let points = path
        .iter()
        .map(|point| Vec3::from_array(*point))
        .collect::<Result<Vec<_>, _>>()?;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        if (!closed && index + 1 == points.len()) || !points[index].close_to(points[next]) {
            continue;
        }
        return invalid_input("sweep path cannot contain collapsed rings");
    }

    let tangents = path_tangents(&points, closed)?;
    let up = Vec3::from_array(up_hint)?.normalized("sweep up hint must be non-zero")?;
    if tangents[0].dot(up).abs() > PARALLEL_DOT_LIMIT {
        return invalid_input("sweep up hint cannot be parallel to the initial path tangent");
    }

    let mut frames = Vec::with_capacity(points.len());
    let first_y = tangents[0];
    let first_z = up
        .sub(first_y.scale(up.dot(first_y)))
        .normalized("sweep up hint cannot be parallel to the initial path tangent")?;
    let first_x = first_y
        .cross(first_z)
        .normalized("sweep frame is degenerate")?;
    frames.push(
        FrameBasis {
            origin: points[0],
            x: first_x,
            y: first_y,
            z: first_z,
        }
        .rolled(roll_degrees[0]),
    );

    for index in 1..points.len() {
        let previous = frames[index - 1];
        let current_y = tangents[index];
        let rotation_axis = previous.y.cross(current_y);
        let axis_length = rotation_axis.length();
        if previous.y.dot(current_y) < -PARALLEL_DOT_LIMIT {
            return invalid_input("sweep path reverses direction too abruptly for stable frames");
        }
        let transported = if axis_length <= EPSILON {
            FrameBasis {
                origin: points[index],
                x: previous.x,
                y: current_y,
                z: previous.z,
            }
        } else {
            let axis = rotation_axis.scale(1.0 / axis_length);
            let angle = axis_length.atan2(previous.y.dot(current_y));
            FrameBasis {
                origin: points[index],
                x: previous.x.rotate_about(axis, angle),
                y: current_y,
                z: previous.z.rotate_about(axis, angle),
            }
        };
        frames.push(transported.orthonormalized()?.rolled(roll_degrees[index]));
    }

    Ok(frames)
}

fn path_tangents(points: &[Vec3], closed: bool) -> Result<Vec<Vec3>, ModelingError> {
    let mut tangents = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let raw = if closed {
            let prev = points[(index + points.len() - 1) % points.len()];
            let next = points[(index + 1) % points.len()];
            next.sub(prev)
        } else if index == 0 {
            points[1].sub(points[0])
        } else if index + 1 == points.len() {
            points[index].sub(points[index - 1])
        } else {
            points[index + 1].sub(points[index - 1])
        };
        tangents.push(raw.normalized("sweep path contains a zero-length tangent")?);
    }
    Ok(tangents)
}

fn sweep_corner_regions(path: &[[f32; 3]], closed: bool) -> Result<Vec<usize>, ModelingError> {
    let points = path
        .iter()
        .map(|point| Vec3::from_array(*point))
        .collect::<Result<Vec<_>, _>>()?;
    let mut corners = Vec::new();
    let start = if closed { 0 } else { 1 };
    let end = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    for index in start..end {
        let previous = if index == 0 {
            points[points.len() - 1]
        } else {
            points[index - 1]
        };
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let incoming = current
            .sub(previous)
            .normalized("sweep corner has a collapsed incoming path segment")?;
        let outgoing = next
            .sub(current)
            .normalized("sweep corner has a collapsed outgoing path segment")?;
        if incoming.dot(outgoing) < 0.999 {
            corners.push(index);
        }
    }
    Ok(corners)
}

fn sweep_segment_region(segment: usize, ring_count: usize, corner_regions: &[usize]) -> RegionId {
    for corner in corner_regions {
        if *corner == segment || (*corner > 0 && *corner - 1 == segment) {
            return sweep_corner_region_id(*corner);
        }
        if *corner == 0 && segment + 1 == ring_count {
            return sweep_corner_region_id(*corner);
        }
    }
    SWEEP_SIDE_REGION
}

fn sweep_corner_region_id(corner_index: usize) -> RegionId {
    RegionId(100 + corner_index as u64)
}

fn sweep_vertex(
    ring_index: usize,
    profile_index: usize,
    profile_count: usize,
) -> Result<u32, ModelingError> {
    ring_index
        .checked_mul(profile_count)
        .and_then(|base| base.checked_add(profile_index))
        .and_then(|index| u32::try_from(index).ok())
        .ok_or(orchard_poly::PolyError::IndexOverflow)
        .map_err(ModelingError::from)
}

fn sweep_edge_metadata(
    mesh: &PolygonMesh,
    profile_count: usize,
    ring_count: usize,
    path_closed: bool,
) -> Result<BTreeMap<EdgeKey, EdgeMetadata>, ModelingError> {
    let adjacency = build_adjacency(mesh)?;
    let mut metadata = BTreeMap::new();
    for (edge, faces) in adjacency.edge_faces {
        let transition = region_transition(mesh, &faces);
        let sweep_kind = classify_sweep_edge(edge, profile_count, ring_count, path_closed);
        let seam_candidate = sweep_kind.seam_candidate;
        let open = faces.len() == 1;
        let region_changes = transition.is_some();
        let profile_feature = sweep_kind.longitudinal_profile_corner;
        let classification = if open || region_changes || profile_feature {
            EdgeClassification::Hard
        } else {
            EdgeClassification::Smooth
        };
        let boundary_role = if seam_candidate {
            BoundaryRole::SeamCandidate
        } else if open {
            BoundaryRole::OpenBoundary
        } else if profile_feature || region_changes {
            BoundaryRole::Feature
        } else {
            BoundaryRole::Smooth
        };
        metadata.insert(
            edge,
            EdgeMetadata {
                boundary_role,
                classification,
                seam_candidate,
                bevel_eligible: false,
                operation: None,
                region_transition: transition,
                boundary_loop: None,
            },
        );
    }
    Ok(metadata)
}

fn classify_sweep_edge(
    edge: EdgeKey,
    profile_count: usize,
    ring_count: usize,
    path_closed: bool,
) -> SweepEdgeKind {
    let a = edge.a as usize;
    let b = edge.b as usize;
    let a_ring = a / profile_count;
    let b_ring = b / profile_count;
    let a_profile = a % profile_count;
    let b_profile = b % profile_count;
    let same_profile = a_profile == b_profile;
    let adjacent_rings = a_ring.abs_diff(b_ring) == 1
        || (path_closed && a_ring.min(b_ring) == 0 && a_ring.max(b_ring) + 1 == ring_count);
    let same_ring = a_ring == b_ring;
    let profile_wrap =
        same_ring && a_profile.min(b_profile) == 0 && a_profile.max(b_profile) + 1 == profile_count;
    SweepEdgeKind {
        longitudinal_profile_corner: same_profile && adjacent_rings,
        seam_candidate: profile_wrap || (same_profile && a_profile == 0 && adjacent_rings),
    }
}
