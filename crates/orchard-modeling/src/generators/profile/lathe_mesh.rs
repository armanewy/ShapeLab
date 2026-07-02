
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
                bevel_eligible: false,
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
        .map_err(|_| ModelingError::from(orchard_poly::PolyError::IndexOverflow))?;
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
