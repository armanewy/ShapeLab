//! Profile-driven sweep and lathe generators.
//!
//! The public specs in this module model the richer profile-generator controls
//! before those controls are represented directly in `shape-asset`.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    Frame3, PartDefinitionId, PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec,
    SurfaceRole,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, FaceMetadata, PolygonMesh,
    build_adjacency, polygon_mesh_from_faces,
};

use crate::{GeneratedPart, GeneratorContext, ModelingError};

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
        .ok_or(shape_poly::PolyError::IndexOverflow)
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

fn add_lathe_profile_cap(
    cap_context: &LatheCapContext<'_>,
    buffers: &mut LatheMeshBuffers<'_>,
    endpoint: usize,
    neighbor: usize,
    region_id: RegionId,
) -> Result<(), ModelingError> {
    if cap_context.profile[endpoint].radius <= EPSILON {
        return Ok(());
    }
    let mut face = cap_context.profile_vertices[endpoint].clone();
    let wants_positive_y =
        cap_context.profile[endpoint].height > cap_context.profile[neighbor].height;
    if wants_positive_y {
        face.reverse();
    }
    if !cap_context.full_revolution {
        let center = cap_context.axis.origin.add(
            cap_context
                .axis
                .y
                .scale(cap_context.profile[endpoint].height),
        );
        let center_vertex = push_lathe_vertex(
            buffers.positions,
            buffers.vertex_map,
            center,
            LatheVertexKind::CapCenter,
        )?;
        if wants_positive_y {
            face.push(center_vertex);
        } else {
            face.insert(0, center_vertex);
        }
    }
    buffers.faces.push(face);
    buffers.face_metadata.push(face_metadata_for(
        cap_context.generator_context.part_definition,
        cap_context.generator_context.part_instance,
        region_id,
        SurfaceRole::Cap,
        None,
    ));
    Ok(())
}

fn lathe_angle(span_degrees: f32, radial_segments: u32, column: usize) -> f32 {
    let fraction = column as f32 / radial_segments as f32;
    (span_degrees * fraction).to_radians()
}

fn lathe_edge_metadata(
    mesh: &PolygonMesh,
    vertex_map: &[LatheVertexKind],
    full_revolution: bool,
    columns: usize,
) -> Result<BTreeMap<EdgeKey, EdgeMetadata>, ModelingError> {
    let adjacency = build_adjacency(mesh)?;
    let mut metadata = BTreeMap::new();
    for (edge, faces) in adjacency.edge_faces {
        let transition = region_transition(mesh, &faces);
        let seam_candidate = lathe_edge_is_seam(edge, vertex_map, full_revolution, columns);
        let open = faces.len() == 1;
        let region_changes = transition.is_some();
        let classification = if open || region_changes {
            EdgeClassification::Hard
        } else {
            EdgeClassification::Smooth
        };
        let boundary_role = if seam_candidate {
            BoundaryRole::SeamCandidate
        } else if open {
            BoundaryRole::OpenBoundary
        } else if region_changes {
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
                operation: None,
                region_transition: transition,
                boundary_loop: None,
            },
        );
    }
    Ok(metadata)
}

fn lathe_edge_is_seam(
    edge: EdgeKey,
    vertex_map: &[LatheVertexKind],
    full_revolution: bool,
    columns: usize,
) -> bool {
    let Some(a) = vertex_map.get(edge.a as usize) else {
        return false;
    };
    let Some(b) = vertex_map.get(edge.b as usize) else {
        return false;
    };
    match (a, b) {
        (
            LatheVertexKind::Profile {
                column: Some(a_column),
                ..
            },
            LatheVertexKind::Profile {
                column: Some(b_column),
                ..
            },
        ) => {
            if full_revolution {
                (*a_column).min(*b_column) == 0 && (*a_column).max(*b_column) + 1 == columns
            } else {
                *a_column == 0
                    || *b_column == 0
                    || *a_column + 1 == columns
                    || *b_column + 1 == columns
            }
        }
        _ => false,
    }
}

fn push_lathe_vertex(
    positions: &mut Vec<[f32; 3]>,
    vertex_map: &mut Vec<LatheVertexKind>,
    position: Vec3,
    kind: LatheVertexKind,
) -> Result<u32, ModelingError> {
    let index = u32::try_from(positions.len())
        .map_err(|_| ModelingError::from(shape_poly::PolyError::IndexOverflow))?;
    positions.push(position.to_array());
    vertex_map.push(kind);
    Ok(index)
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

fn basis_from_frame(frame: &Frame3) -> Result<FrameBasis, ModelingError> {
    let origin = Vec3::from_array(frame.origin)?;
    let x = Vec3::from_array(frame.x_axis)?.normalized("frame x axis must be non-zero")?;
    let y = Vec3::from_array(frame.y_axis)?.normalized("frame y axis must be non-zero")?;
    let z = Vec3::from_array(frame.z_axis)?.normalized("frame z axis must be non-zero")?;
    if x.dot(y).abs() > 1.0e-3 || y.dot(z).abs() > 1.0e-3 || z.dot(x).abs() > 1.0e-3 {
        return invalid_input("frame axes must be orthogonal");
    }
    let right_handed_z = x.cross(y).normalized("frame axes are degenerate")?;
    if right_handed_z.dot(z) < 0.99 {
        return invalid_input("frame axes must be right-handed");
    }
    Ok(FrameBasis { origin, x, y, z })
}

fn frame_to_socket_frame(frame: FrameBasis, reverse_y: bool) -> Frame3 {
    let y = if reverse_y {
        frame.y.scale(-1.0)
    } else {
        frame.y
    };
    Frame3 {
        origin: frame.origin.to_array(),
        x_axis: frame.x.to_array(),
        y_axis: y.to_array(),
        z_axis: frame.z.to_array(),
    }
}

fn face_metadata_for(
    part_definition: PartDefinitionId,
    part_instance: PartInstanceId,
    region_id: RegionId,
    role: SurfaceRole,
    smoothing_group: Option<u32>,
) -> FaceMetadata {
    FaceMetadata {
        part_definition: Some(part_definition),
        part_instance: Some(part_instance),
        region: Some(region_id),
        operation: None,
        smoothing_group,
        surface_role: Some(role),
    }
}

fn region(id: RegionId, name: &str, role: SurfaceRole, tags: &[&str]) -> SurfaceRegionSpec {
    SurfaceRegionSpec {
        id,
        name: name.to_owned(),
        role,
        tags: string_set(tags),
    }
}

fn socket(id: SocketId, name: &str, local_frame: Frame3, role: &str, tags: &[&str]) -> SocketSpec {
    SocketSpec {
        id,
        name: name.to_owned(),
        local_frame,
        role: role.to_owned(),
        tags: string_set(tags),
    }
}

fn string_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
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

fn invalid_input<T>(message: &str) -> Result<T, ModelingError> {
    Err(ModelingError::InvalidInput(message.to_owned()))
}

#[derive(Debug, Copy, Clone)]
struct SweepEdgeKind {
    longitudinal_profile_corner: bool,
    seam_candidate: bool,
}

#[derive(Debug, Copy, Clone)]
struct LathePoint {
    radius: f32,
    height: f32,
}

struct LatheCapContext<'a> {
    profile: &'a [LathePoint],
    profile_vertices: &'a [Vec<u32>],
    full_revolution: bool,
    axis: FrameBasis,
    generator_context: &'a GeneratorContext,
}

struct LatheMeshBuffers<'a> {
    positions: &'a mut Vec<[f32; 3]>,
    vertex_map: &'a mut Vec<LatheVertexKind>,
    faces: &'a mut Vec<Vec<u32>>,
    face_metadata: &'a mut Vec<FaceMetadata>,
}

#[derive(Debug, Copy, Clone)]
enum LatheVertexKind {
    Profile { column: Option<usize> },
    CapCenter,
}

#[derive(Debug, Copy, Clone)]
struct FrameBasis {
    origin: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl FrameBasis {
    fn orthonormalized(self) -> Result<Self, ModelingError> {
        let y = self.y.normalized("frame y axis must be non-zero")?;
        let x = self
            .x
            .sub(y.scale(self.x.dot(y)))
            .normalized("frame x axis must be non-zero")?;
        let z = x.cross(y).normalized("frame z axis must be non-zero")?;
        Ok(Self {
            origin: self.origin,
            x,
            y,
            z,
        })
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

    fn to_frame3(self) -> Frame3 {
        Frame3 {
            origin: self.origin.to_array(),
            x_axis: self.x.to_array(),
            y_axis: self.y.to_array(),
            z_axis: self.z.to_array(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn from_array(value: [f32; 3]) -> Result<Self, ModelingError> {
        if value.iter().any(|component| !component.is_finite()) {
            return invalid_input("3D vectors must be finite");
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

    fn normalized(self, message: &str) -> Result<Self, ModelingError> {
        let length = self.length();
        if !length.is_finite() || length <= EPSILON {
            return invalid_input(message);
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

    fn close_to(self, other: Self) -> bool {
        self.sub(other).length() <= EPSILON
    }
}
