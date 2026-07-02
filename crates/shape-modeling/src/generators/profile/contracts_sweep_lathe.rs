
const EPSILON: f32 = 1.0e-5;
const PARALLEL_DOT_LIMIT: f32 = 0.999;
const FULL_REVOLUTION_DEGREES: f32 = 360.0;

/// Sweep side surface region.
pub const SWEEP_SIDE_REGION: RegionId = RegionId(1);
/// Sweep start cap surface region.
pub const SWEEP_START_CAP_REGION: RegionId = RegionId(2);
/// Sweep end cap surface region.
pub const SWEEP_END_CAP_REGION: RegionId = RegionId(3);
/// Sweep start attachment socket.
pub const SWEEP_START_SOCKET: SocketId = SocketId(1);
/// Sweep end attachment socket.
pub const SWEEP_END_SOCKET: SocketId = SocketId(2);

/// Lathe side surface region.
pub const LATHE_SIDE_REGION: RegionId = RegionId(1);
/// Lathe first profile end cap surface region.
pub const LATHE_START_CAP_REGION: RegionId = RegionId(2);
/// Lathe last profile end cap surface region.
pub const LATHE_END_CAP_REGION: RegionId = RegionId(3);
/// Partial-lathe start seam region.
pub const LATHE_START_SEAM_REGION: RegionId = RegionId(4);
/// Partial-lathe end seam region.
pub const LATHE_END_SEAM_REGION: RegionId = RegionId(5);
/// Lathe bottom attachment socket.
pub const LATHE_BOTTOM_SOCKET: SocketId = SocketId(1);
/// Lathe top attachment socket.
pub const LATHE_TOP_SOCKET: SocketId = SocketId(2);

/// Cap generation mode for profile generators.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CapMode {
    /// Leave profile ends open.
    None,
    /// Generate deterministic end caps where the topology supports them.
    Ends,
}

/// Sweep corner handling strategy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CornerStrategy {
    /// Preserve a path ring at every corner and tag adjacent faces as corners.
    PreserveRings,
}

/// Partial-lathe seam handling strategy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LatheSeamMode {
    /// Leave partial-revolution angular seams open, with seam edge metadata.
    Open,
    /// Reserve deterministic seam regions for capped seam faces.
    Capped,
}

/// Sweep generator input.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepSpec {
    /// Closed two-dimensional profile points. A duplicate trailing point is accepted.
    pub profile: Vec<[f32; 2]>,
    /// Ordered path samples used to build parallel-transport frames.
    pub path: Vec<[f32; 3]>,
    /// Initial up hint used to seed the transported frame.
    pub up_hint: [f32; 3],
    /// Optional per-path sample scale. Empty means scale 1.0 at every sample.
    pub scales: Vec<f32>,
    /// Optional per-path sample roll in degrees. Empty means zero roll.
    pub roll_degrees: Vec<f32>,
    /// End-cap behavior for open paths.
    pub cap_mode: CapMode,
    /// Corner behavior for bent paths.
    pub corner_strategy: CornerStrategy,
    /// Whether the last path sample connects back to the first.
    pub path_closed: bool,
}

impl SweepSpec {
    /// Create a capped open sweep with unit scale and zero roll.
    #[must_use]
    pub fn new(profile: Vec<[f32; 2]>, path: Vec<[f32; 3]>, up_hint: [f32; 3]) -> Self {
        Self {
            profile,
            path,
            up_hint,
            scales: Vec::new(),
            roll_degrees: Vec::new(),
            cap_mode: CapMode::Ends,
            corner_strategy: CornerStrategy::PreserveRings,
            path_closed: false,
        }
    }
}

/// Lathe generator input.
#[derive(Debug, Clone, PartialEq)]
pub struct LatheSpec {
    /// Ordered radius/height profile points.
    pub profile: Vec<[f32; 2]>,
    /// Axis frame. Local Y is the lathe axis; local X/Z span the radial plane.
    pub axis_frame: Frame3,
    /// Angular span in degrees. Values below 360 create partial revolutions.
    pub angular_span_degrees: f32,
    /// Number of angular strips.
    pub radial_segments: u32,
    /// End-cap behavior for the first and last profile points.
    pub cap_mode: CapMode,
    /// Seam behavior for partial revolutions.
    pub seam_mode: LatheSeamMode,
}

impl LatheSpec {
    /// Create a full-revolution lathe around local Y.
    #[must_use]
    pub fn new(profile: Vec<[f32; 2]>, radial_segments: u32) -> Self {
        Self {
            profile,
            axis_frame: Frame3::default(),
            angular_span_degrees: FULL_REVOLUTION_DEGREES,
            radial_segments,
            cap_mode: CapMode::Ends,
            seam_mode: LatheSeamMode::Open,
        }
    }
}

/// Generate a swept closed profile along path samples.
pub fn generate_sweep(
    spec: &SweepSpec,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let profile = normalize_closed_profile(&spec.profile)?;
    let scales = optional_values(&spec.scales, spec.path.len(), 1.0, "sweep scales")?;
    let rolls = optional_values(
        &spec.roll_degrees,
        spec.path.len(),
        0.0,
        "sweep roll values",
    )?;
    let frames = parallel_transport_frames(&spec.path, spec.up_hint, &rolls, spec.path_closed)?;
    for scale in &scales {
        if !scale.is_finite() || *scale <= EPSILON {
            return invalid_input("sweep scale values must be finite and positive");
        }
    }

    let profile_count = profile.len();
    let ring_count = frames.len();
    let mut positions = Vec::with_capacity(profile_count * ring_count);
    for (ring_index, frame) in frames.iter().enumerate() {
        let scale = scales[ring_index];
        for point in &profile {
            let position = frame
                .origin
                .add(frame.x.scale(point[0] * scale))
                .add(frame.z.scale(point[1] * scale));
            positions.push(position.to_array());
        }
    }

    let mut regions = BTreeMap::new();
    regions.insert(
        SWEEP_SIDE_REGION,
        region(
            SWEEP_SIDE_REGION,
            "Sweep Side",
            SurfaceRole::Side,
            &["sweep", "side"],
        ),
    );
    if spec.cap_mode == CapMode::Ends && !spec.path_closed {
        regions.insert(
            SWEEP_START_CAP_REGION,
            region(
                SWEEP_START_CAP_REGION,
                "Sweep Start Cap",
                SurfaceRole::Cap,
                &["sweep", "cap", "start"],
            ),
        );
        regions.insert(
            SWEEP_END_CAP_REGION,
            region(
                SWEEP_END_CAP_REGION,
                "Sweep End Cap",
                SurfaceRole::Cap,
                &["sweep", "cap", "end"],
            ),
        );
    }

    let corner_regions = sweep_corner_regions(&spec.path, spec.path_closed)?;
    for corner in &corner_regions {
        let id = sweep_corner_region_id(*corner);
        regions.insert(
            id,
            region(
                id,
                &format!("Sweep Corner {corner}"),
                SurfaceRole::Custom("corner".to_owned()),
                &["sweep", "corner"],
            ),
        );
    }

    let mut faces = Vec::new();
    let mut face_metadata = Vec::new();
    let segment_count = if spec.path_closed {
        ring_count
    } else {
        ring_count - 1
    };
    for segment in 0..segment_count {
        let next_ring = (segment + 1) % ring_count;
        let segment_region = sweep_segment_region(segment, ring_count, &corner_regions);
        let segment_role = regions
            .get(&segment_region)
            .map(|region| region.role.clone())
            .unwrap_or(SurfaceRole::Side);
        for profile_index in 0..profile_count {
            let next_profile = (profile_index + 1) % profile_count;
            faces.push(vec![
                sweep_vertex(segment, profile_index, profile_count)?,
                sweep_vertex(segment, next_profile, profile_count)?,
                sweep_vertex(next_ring, next_profile, profile_count)?,
                sweep_vertex(next_ring, profile_index, profile_count)?,
            ]);
            face_metadata.push(face_metadata_for(
                context.part_definition,
                context.part_instance,
                segment_region,
                segment_role.clone(),
                Some(1),
            ));
        }
    }

    if spec.cap_mode == CapMode::Ends && !spec.path_closed {
        let start_cap = (0..profile_count)
            .rev()
            .map(|profile_index| sweep_vertex(0, profile_index, profile_count))
            .collect::<Result<Vec<_>, _>>()?;
        faces.push(start_cap);
        face_metadata.push(face_metadata_for(
            context.part_definition,
            context.part_instance,
            SWEEP_START_CAP_REGION,
            SurfaceRole::Cap,
            None,
        ));

        let end_cap = (0..profile_count)
            .map(|profile_index| sweep_vertex(ring_count - 1, profile_index, profile_count))
            .collect::<Result<Vec<_>, _>>()?;
        faces.push(end_cap);
        face_metadata.push(face_metadata_for(
            context.part_definition,
            context.part_instance,
            SWEEP_END_CAP_REGION,
            SurfaceRole::Cap,
            None,
        ));
    }

    for face in &mut faces {
        face.reverse();
    }

    let mut mesh = polygon_mesh_from_faces(positions, faces, face_metadata)?;
    mesh.edge_metadata = sweep_edge_metadata(&mesh, profile_count, ring_count, spec.path_closed)?;

    let mut sockets = BTreeMap::new();
    sockets.insert(
        SWEEP_START_SOCKET,
        socket(
            SWEEP_START_SOCKET,
            "Sweep Start",
            frame_to_socket_frame(frames[0], false),
            "sweep-start",
            &["sweep", "start"],
        ),
    );
    sockets.insert(
        SWEEP_END_SOCKET,
        socket(
            SWEEP_END_SOCKET,
            "Sweep End",
            frame_to_socket_frame(frames[ring_count - 1], true),
            "sweep-end",
            &["sweep", "end"],
        ),
    );

    Ok(GeneratedPart {
        local_bounds: mesh.bounds,
        mesh,
        sockets,
        regions,
        generator_signature: format!(
            "sweep:v1:profile={profile_count}:rings={ring_count}:closed={}:cap={:?}:corner={:?}",
            spec.path_closed, spec.cap_mode, spec.corner_strategy
        ),
    })
}

/// Generate a lathe mesh from a radius/height profile.
pub fn generate_lathe(
    spec: &LatheSpec,
    context: &GeneratorContext,
) -> Result<GeneratedPart, ModelingError> {
    let profile = normalize_lathe_profile(&spec.profile)?;
    if spec.radial_segments < 3 {
        return invalid_input("lathe requires at least three radial segments");
    }
    if !spec.angular_span_degrees.is_finite()
        || spec.angular_span_degrees <= EPSILON
        || spec.angular_span_degrees - FULL_REVOLUTION_DEGREES > EPSILON
    {
        return invalid_input("lathe angular span must be finite and in the range (0, 360]");
    }
    let axis = basis_from_frame(&spec.axis_frame)?;
    let full_revolution = (FULL_REVOLUTION_DEGREES - spec.angular_span_degrees).abs() <= EPSILON;
    let columns = if full_revolution {
        spec.radial_segments as usize
    } else {
        spec.radial_segments as usize + 1
    };

    let mut positions = Vec::new();
    let mut vertex_map = Vec::new();
    let mut profile_vertices: Vec<Vec<u32>> = Vec::with_capacity(profile.len());
    for point in &profile {
        if point.radius <= EPSILON {
            let position = axis.origin.add(axis.y.scale(point.height));
            let vertex = push_lathe_vertex(
                &mut positions,
                &mut vertex_map,
                position,
                LatheVertexKind::Profile { column: None },
            )?;
            profile_vertices.push(vec![vertex; columns]);
        } else {
            let mut ring = Vec::with_capacity(columns);
            for column in 0..columns {
                let angle = lathe_angle(spec.angular_span_degrees, spec.radial_segments, column);
                let radial = axis
                    .x
                    .scale(angle.cos())
                    .add(axis.z.scale(angle.sin()))
                    .scale(point.radius);
                let position = axis.origin.add(axis.y.scale(point.height)).add(radial);
                ring.push(push_lathe_vertex(
                    &mut positions,
                    &mut vertex_map,
                    position,
                    LatheVertexKind::Profile {
                        column: Some(column),
                    },
                )?);
            }
            profile_vertices.push(ring);
        }
    }

    let mut regions = BTreeMap::new();
    regions.insert(
        LATHE_SIDE_REGION,
        region(
            LATHE_SIDE_REGION,
            "Lathe Side",
            SurfaceRole::Side,
            &["lathe", "side"],
        ),
    );
    if spec.cap_mode == CapMode::Ends {
        regions.insert(
            LATHE_START_CAP_REGION,
            region(
                LATHE_START_CAP_REGION,
                "Lathe Start Cap",
                SurfaceRole::Cap,
                &["lathe", "cap", "start"],
            ),
        );
        regions.insert(
            LATHE_END_CAP_REGION,
            region(
                LATHE_END_CAP_REGION,
                "Lathe End Cap",
                SurfaceRole::Cap,
                &["lathe", "cap", "end"],
            ),
        );
    }
    if !full_revolution {
        regions.insert(
            LATHE_START_SEAM_REGION,
            region(
                LATHE_START_SEAM_REGION,
                "Lathe Start Seam",
                SurfaceRole::Custom("seam".to_owned()),
                &["lathe", "seam", "start"],
            ),
        );
        regions.insert(
            LATHE_END_SEAM_REGION,
            region(
                LATHE_END_SEAM_REGION,
                "Lathe End Seam",
                SurfaceRole::Custom("seam".to_owned()),
                &["lathe", "seam", "end"],
            ),
        );
    }

    let mut faces = Vec::new();
    let mut face_metadata = Vec::new();
    let angular_segments = spec.radial_segments as usize;
    for profile_index in 0..profile.len() - 1 {
        if profile[profile_index].radius <= EPSILON && profile[profile_index + 1].radius <= EPSILON
        {
            return invalid_input("lathe profile cannot contain adjacent axis points");
        }
        for column in 0..angular_segments {
            let next_column = if full_revolution {
                (column + 1) % columns
            } else {
                column + 1
            };
            let a = profile_vertices[profile_index][column];
            let b = profile_vertices[profile_index + 1][column];
            let c = profile_vertices[profile_index + 1][next_column];
            let d = profile_vertices[profile_index][next_column];
            if a == d {
                faces.push(vec![a, b, c]);
            } else if b == c {
                faces.push(vec![a, b, d]);
            } else {
                faces.push(vec![a, b, c, d]);
            }
            face_metadata.push(face_metadata_for(
                context.part_definition,
                context.part_instance,
                LATHE_SIDE_REGION,
                SurfaceRole::Side,
                Some(1),
            ));
        }
    }

    if spec.cap_mode == CapMode::Ends {
        let cap_context = LatheCapContext {
            profile: &profile,
            profile_vertices: &profile_vertices,
            full_revolution,
            axis,
            generator_context: context,
        };
        let mut cap_buffers = LatheMeshBuffers {
            positions: &mut positions,
            vertex_map: &mut vertex_map,
            faces: &mut faces,
            face_metadata: &mut face_metadata,
        };
        add_lathe_profile_cap(&cap_context, &mut cap_buffers, 0, 1, LATHE_START_CAP_REGION)?;
        add_lathe_profile_cap(
            &cap_context,
            &mut cap_buffers,
            cap_context.profile.len() - 1,
            cap_context.profile.len() - 2,
            LATHE_END_CAP_REGION,
        )?;
    }

    let mut mesh = polygon_mesh_from_faces(positions, faces, face_metadata)?;
    mesh.edge_metadata = lathe_edge_metadata(&mesh, &vertex_map, full_revolution, columns)?;

    let mut sockets = BTreeMap::new();
    let min_height = profile
        .iter()
        .map(|point| point.height)
        .fold(f32::INFINITY, f32::min);
    let max_height = profile
        .iter()
        .map(|point| point.height)
        .fold(f32::NEG_INFINITY, f32::max);
    sockets.insert(
        LATHE_BOTTOM_SOCKET,
        socket(
            LATHE_BOTTOM_SOCKET,
            "Lathe Bottom",
            FrameBasis {
                origin: axis.origin.add(axis.y.scale(min_height)),
                x: axis.x,
                y: axis.y.scale(-1.0),
                z: axis.z,
            }
            .to_frame3(),
            "lathe-bottom",
            &["lathe", "bottom"],
        ),
    );
    sockets.insert(
        LATHE_TOP_SOCKET,
        socket(
            LATHE_TOP_SOCKET,
            "Lathe Top",
            FrameBasis {
                origin: axis.origin.add(axis.y.scale(max_height)),
                x: axis.x,
                y: axis.y,
                z: axis.z,
            }
            .to_frame3(),
            "lathe-top",
            &["lathe", "top"],
        ),
    );

    Ok(GeneratedPart {
        local_bounds: mesh.bounds,
        mesh,
        sockets,
        regions,
        generator_signature: format!(
            "lathe:v1:profile={}:segments={}:span={:.6}:cap={:?}:seam={:?}",
            profile.len(),
            spec.radial_segments,
            spec.angular_span_degrees,
            spec.cap_mode,
            spec.seam_mode
        ),
    })
}
