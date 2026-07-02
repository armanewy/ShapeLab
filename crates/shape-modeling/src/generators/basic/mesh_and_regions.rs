
fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    if length <= EPSILON {
        fallback
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}

fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

#[derive(Debug, Clone)]
struct Ring {
    y: f32,
    vertices: RingVertices,
    incoming_region: RegionId,
}

#[derive(Debug, Clone)]
enum RingVertices {
    Apex(u32),
    Circle(Vec<u32>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FaceSide {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl FaceSide {
    const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    #[must_use]
    fn fixed_axis(self) -> usize {
        match self {
            Self::PositiveX | Self::NegativeX => 0,
            Self::PositiveY | Self::NegativeY => 1,
            Self::PositiveZ | Self::NegativeZ => 2,
        }
    }

    #[must_use]
    fn sign(self) -> f32 {
        match self {
            Self::PositiveX | Self::PositiveY | Self::PositiveZ => 1.0,
            Self::NegativeX | Self::NegativeY | Self::NegativeZ => -1.0,
        }
    }

    #[must_use]
    fn tangent_axes(self) -> [usize; 2] {
        match self {
            Self::PositiveX => [1, 2],
            Self::NegativeX => [2, 1],
            Self::PositiveY => [2, 0],
            Self::NegativeY => [0, 2],
            Self::PositiveZ => [0, 1],
            Self::NegativeZ => [1, 0],
        }
    }

    #[must_use]
    fn opposite(self) -> Self {
        match self {
            Self::PositiveX => Self::NegativeX,
            Self::NegativeX => Self::PositiveX,
            Self::PositiveY => Self::NegativeY,
            Self::NegativeY => Self::PositiveY,
            Self::PositiveZ => Self::NegativeZ,
            Self::NegativeZ => Self::PositiveZ,
        }
    }
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    vertex_ids: Vec<ElementId>,
    vertex_lookup: BTreeMap<VertexKey, u32>,
    faces: Vec<PolygonFace>,
    face_metadata: Vec<FaceMetadata>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            vertex_ids: Vec::new(),
            vertex_lookup: BTreeMap::new(),
            faces: Vec::new(),
            face_metadata: Vec::new(),
        }
    }

    fn add_vertices(&mut self, positions: &[[f32; 3]]) -> Result<Vec<u32>, ModelingError> {
        positions
            .iter()
            .copied()
            .map(|position| self.add_vertex(position))
            .collect()
    }

    fn add_vertex(&mut self, position: [f32; 3]) -> Result<u32, ModelingError> {
        if !position.iter().copied().all(f32::is_finite) {
            return Err(ModelingError::InvalidInput(
                "generated non-finite vertex position".to_owned(),
            ));
        }
        let key = VertexKey::from_position(position);
        if let Some(index) = self.vertex_lookup.get(&key) {
            return Ok(*index);
        }
        let index = u32::try_from(self.positions.len()).map_err(|_| {
            ModelingError::InvalidInput("generated mesh exceeded u32 index range".to_owned())
        })?;
        self.positions.push(position);
        self.vertex_ids.push(ElementId(u64::from(index)));
        self.vertex_lookup.insert(key, index);
        Ok(index)
    }

    fn add_ring(
        &mut self,
        y: f32,
        radius: f32,
        radial_segments: u32,
        incoming_region: RegionId,
    ) -> Result<Ring, ModelingError> {
        if radius <= EPSILON {
            let vertex = self.add_vertex([0.0, y, 0.0])?;
            return Ok(Ring {
                y,
                vertices: RingVertices::Apex(vertex),
                incoming_region,
            });
        }
        let mut vertices = Vec::new();
        for index in 0..radial_segments {
            let angle = 2.0 * PI * index as f32 / radial_segments as f32;
            let (sin, cos) = angle.sin_cos();
            vertices.push(self.add_vertex([radius * cos, y, radius * sin])?);
        }
        Ok(Ring {
            y,
            vertices: RingVertices::Circle(vertices),
            incoming_region,
        })
    }

    fn add_plate_ring(&mut self, y: f32, points: &[[f32; 2]]) -> Result<Vec<u32>, ModelingError> {
        points
            .iter()
            .map(|point| self.add_vertex([point[0], y, point[1]]))
            .collect()
    }

    fn add_host_ring(
        &mut self,
        host: &PlanarHostPatch,
        depth: f32,
        points: &[[f32; 2]],
    ) -> Result<Vec<u32>, ModelingError> {
        points
            .iter()
            .map(|point| self.add_vertex(host.position(depth, *point)))
            .collect()
    }

    fn add_face(&mut self, vertices: Vec<u32>, metadata: FaceMetadata) {
        if has_duplicate_indices(&vertices) || vertices.len() < 3 {
            return;
        }
        let id = ElementId(self.faces.len() as u64);
        self.faces.push(PolygonFace { id, vertices });
        self.face_metadata.push(metadata);
    }

    fn finish(self) -> Result<PolygonMesh, ModelingError> {
        let bounds = bounds_from_positions(&self.positions)?;
        let mut mesh = PolygonMesh {
            positions: self.positions,
            vertex_ids: self.vertex_ids,
            faces: self.faces,
            face_metadata: self.face_metadata,
            edge_metadata: BTreeMap::new(),
            topology_signature: 0,
            bounds,
        };
        mesh.topology_signature = compute_topology_signature(&mesh.positions, &mesh.faces);
        mesh.edge_metadata = build_edge_metadata(&mesh);
        Ok(mesh)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VertexKey(i64, i64, i64);

impl VertexKey {
    fn from_position(position: [f32; 3]) -> Self {
        Self(
            quantize(position[0]),
            quantize(position[1]),
            quantize(position[2]),
        )
    }
}

fn build_edge_metadata(mesh: &PolygonMesh) -> BTreeMap<EdgeKey, EdgeMetadata> {
    let mut edge_faces: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for index in 0..face.vertices.len() {
            let next = (index + 1) % face.vertices.len();
            edge_faces
                .entry(EdgeKey::new(face.vertices[index], face.vertices[next]))
                .or_default()
                .push(face_index);
        }
    }

    edge_faces
        .into_iter()
        .map(|(edge, faces)| {
            let metadata = if faces.len() == 1 {
                EdgeMetadata {
                    boundary_role: BoundaryRole::OpenBoundary,
                    classification: EdgeClassification::Hard,
                    seam_candidate: false,
                    bevel_eligible: false,
                    operation: None,
                    region_transition: None,
                    boundary_loop: None,
                }
            } else {
                let first = &mesh.face_metadata[faces[0]];
                let second = &mesh.face_metadata[faces[1]];
                let smooth = first.smoothing_group.is_some()
                    && first.smoothing_group == second.smoothing_group;
                EdgeMetadata {
                    boundary_role: if smooth {
                        BoundaryRole::Smooth
                    } else {
                        BoundaryRole::Hard
                    },
                    classification: if smooth {
                        EdgeClassification::Smooth
                    } else {
                        EdgeClassification::Hard
                    },
                    seam_candidate: false,
                    bevel_eligible: false,
                    operation: None,
                    region_transition: region_transition(first.region, second.region),
                    boundary_loop: None,
                }
            };
            (edge, metadata)
        })
        .collect()
}

fn rounded_box_position(
    side: FaceSide,
    u: f32,
    v: f32,
    half: [f32; 3],
    inner: [f32; 3],
    radius: f32,
) -> [f32; 3] {
    let mut base = [0.0; 3];
    base[side.fixed_axis()] = side.sign() * half[side.fixed_axis()];
    let [u_axis, v_axis] = side.tangent_axes();
    base[u_axis] = u;
    base[v_axis] = v;
    if radius <= EPSILON {
        return base;
    }
    let closest = [
        base[0].clamp(-inner[0], inner[0]),
        base[1].clamp(-inner[1], inner[1]),
        base[2].clamp(-inner[2], inner[2]),
    ];
    let delta = [
        base[0] - closest[0],
        base[1] - closest[1],
        base[2] - closest[2],
    ];
    let length = dot(delta, delta).sqrt();
    if length <= EPSILON {
        closest
    } else {
        [
            closest[0] + delta[0] * radius / length,
            closest[1] + delta[1] * radius / length,
            closest[2] + delta[2] * radius / length,
        ]
    }
}

fn rounded_box_region(
    axes: [usize; 2],
    center: [f32; 2],
    inner: [f32; 3],
    radius: f32,
) -> RegionId {
    if radius <= EPSILON {
        return ROUNDED_PRIMARY_REGION;
    }
    let outside = axes
        .into_iter()
        .zip(center)
        .filter(|(axis, value)| value.abs() > inner[*axis] + EPSILON)
        .count();
    match outside {
        0 => ROUNDED_PRIMARY_REGION,
        1 => ROUNDED_BEVEL_REGION,
        _ => ROUNDED_CORNER_REGION,
    }
}

fn rounded_box_region_for_primary(region: RegionId, primary_region: RegionId) -> RegionId {
    if region == ROUNDED_PRIMARY_REGION {
        primary_region
    } else {
        region
    }
}

fn rounded_box_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        ROUNDED_PRIMARY_REGION => (SurfaceRole::PrimarySurface, None),
        ROUNDED_BEVEL_REGION => (SurfaceRole::BevelBand, Some(1)),
        ROUNDED_CORNER_REGION => (SurfaceRole::Detail, Some(1)),
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn rounded_box_metadata_for_primary(
    context: &GeneratorContext,
    region: RegionId,
    primary_region: RegionId,
) -> FaceMetadata {
    if region == primary_region {
        metadata(context, region, SurfaceRole::PrimarySurface, None)
    } else {
        rounded_box_metadata(context, region)
    }
}

fn cylinder_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        CYLINDER_SIDE_REGION => (SurfaceRole::Side, Some(2)),
        CYLINDER_TOP_CAP_REGION | CYLINDER_BOTTOM_CAP_REGION => (SurfaceRole::Cap, None),
        CYLINDER_TOP_BEVEL_REGION | CYLINDER_BOTTOM_BEVEL_REGION => {
            (SurfaceRole::BevelBand, Some(1))
        }
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn plate_metadata(context: &GeneratorContext, region: RegionId) -> FaceMetadata {
    let (surface_role, smoothing_group) = match region {
        PLATE_FRONT_REGION | PLATE_BACK_REGION => (SurfaceRole::PrimarySurface, None),
        PLATE_SIDE_REGION => (SurfaceRole::Side, Some(2)),
        PLATE_BEVEL_REGION => (SurfaceRole::BevelBand, Some(1)),
        _ => (SurfaceRole::Detail, None),
    };
    metadata(context, region, surface_role, smoothing_group)
}

fn metadata(
    context: &GeneratorContext,
    region: RegionId,
    surface_role: SurfaceRole,
    smoothing_group: Option<u32>,
) -> FaceMetadata {
    FaceMetadata {
        part_definition: Some(context.part_definition),
        part_instance: Some(context.part_instance),
        region: Some(region),
        operation: None,
        smoothing_group,
        surface_role: Some(surface_role),
    }
}

fn axis_samples(
    half: f32,
    inner: f32,
    radius: f32,
    bevel_segments: u32,
    face_subdivisions: u32,
) -> Vec<f32> {
    let mut samples = Vec::new();
    if radius <= EPSILON {
        for index in 0..=face_subdivisions {
            samples.push(lerp(-half, half, index as f32 / face_subdivisions as f32));
        }
    } else {
        for index in 0..=bevel_segments {
            samples.push(lerp(-half, -inner, index as f32 / bevel_segments as f32));
        }
        for index in 1..face_subdivisions {
            samples.push(lerp(-inner, inner, index as f32 / face_subdivisions as f32));
        }
        for index in 0..=bevel_segments {
            samples.push(lerp(inner, half, index as f32 / bevel_segments as f32));
        }
    }
    dedup_sorted_f32(samples)
}

fn rounded_rect_points(
    half_x: f32,
    half_z: f32,
    radius: f32,
    corner_segments: u32,
) -> Vec<[f32; 2]> {
    if radius <= EPSILON {
        return vec![
            [half_x, half_z],
            [-half_x, half_z],
            [-half_x, -half_z],
            [half_x, -half_z],
        ];
    }
    let centers = [
        [half_x - radius, half_z - radius],
        [-half_x + radius, half_z - radius],
        [-half_x + radius, -half_z + radius],
        [half_x - radius, -half_z + radius],
    ];
    let starts = [0.0, FRAC_PI_2, PI, PI + FRAC_PI_2];
    let mut points = Vec::new();
    for (center, start) in centers.into_iter().zip(starts) {
        for index in 0..=corner_segments {
            let t = index as f32 / corner_segments as f32;
            let angle = start + t * FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            points.push([center[0] + radius * cos, center[1] + radius * sin]);
        }
    }
    points
}

fn circle_points(radius: f32, segments: u32) -> Vec<[f32; 2]> {
    (0..segments)
        .map(|index| {
            let angle = 2.0 * PI * index as f32 / segments as f32;
            let (sin, cos) = angle.sin_cos();
            [radius * cos, radius * sin]
        })
        .collect()
}

fn clamp_frustum_bevels(
    bottom: f32,
    top: f32,
    bottom_radius: f32,
    top_radius: f32,
    height: f32,
) -> (f32, f32) {
    let mut bottom = bottom.min(bottom_radius);
    let mut top = top.min(top_radius);
    let sum = bottom + top;
    if sum > height && sum > EPSILON {
        let scale = height / sum;
        bottom *= scale;
        top *= scale;
    }
    (bottom, top)
}

fn rounded_box_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (
            ROUNDED_PRIMARY_REGION,
            "primary_faces",
            SurfaceRole::PrimarySurface,
        ),
        (ROUNDED_BEVEL_REGION, "bevel_bands", SurfaceRole::BevelBand),
        (ROUNDED_CORNER_REGION, "corners", SurfaceRole::Detail),
    ])
}

fn rounded_box_regions_for_primary(
    primary_region: RegionId,
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    let mut regions = rounded_box_regions();
    if primary_region != ROUNDED_PRIMARY_REGION {
        let mut region = regions
            .remove(&ROUNDED_PRIMARY_REGION)
            .expect("rounded box primary region exists");
        region.id = primary_region;
        regions.insert(primary_region, region);
    }
    regions
}

fn cylinder_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (CYLINDER_SIDE_REGION, "side", SurfaceRole::Side),
        (CYLINDER_TOP_CAP_REGION, "top_cap", SurfaceRole::Cap),
        (CYLINDER_BOTTOM_CAP_REGION, "bottom_cap", SurfaceRole::Cap),
        (
            CYLINDER_TOP_BEVEL_REGION,
            "top_bevel",
            SurfaceRole::BevelBand,
        ),
        (
            CYLINDER_BOTTOM_BEVEL_REGION,
            "bottom_bevel",
            SurfaceRole::BevelBand,
        ),
    ])
}

fn plate_regions() -> BTreeMap<RegionId, SurfaceRegionSpec> {
    regions([
        (PLATE_FRONT_REGION, "front", SurfaceRole::PrimarySurface),
        (PLATE_BACK_REGION, "back", SurfaceRole::PrimarySurface),
        (PLATE_SIDE_REGION, "side", SurfaceRole::Side),
        (PLATE_BEVEL_REGION, "bevel", SurfaceRole::BevelBand),
    ])
}

fn regions<const N: usize>(
    specs: [(RegionId, &'static str, SurfaceRole); N],
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    specs
        .into_iter()
        .map(|(id, name, role)| {
            (
                id,
                SurfaceRegionSpec {
                    id,
                    name: name.to_owned(),
                    role,
                    tags: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn rounded_box_sockets(half: [f32; 3]) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SocketId(1),
            "positive_x",
            [half[0], 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ),
        (
            SocketId(2),
            "negative_x",
            [-half[0], 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
        ),
        (
            SocketId(3),
            "positive_y",
            [0.0, half[1], 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SocketId(4),
            "negative_y",
            [0.0, -half[1], 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
        (
            SocketId(5),
            "positive_z",
            [0.0, 0.0, half[2]],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ),
        (
            SocketId(6),
            "negative_z",
            [0.0, 0.0, -half[2]],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ),
    ])
}

fn cylinder_sockets(half_height: f32) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SOCKET_TOP,
            "top_center",
            [0.0, half_height, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SOCKET_BOTTOM,
            "bottom_center",
            [0.0, -half_height, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
        (
            SOCKET_AXIS,
            "axis_midpoint",
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
    ])
}

fn plate_sockets(half_thickness: f32) -> BTreeMap<SocketId, SocketSpec> {
    sockets([
        (
            SocketId(1),
            "front_center",
            [0.0, half_thickness, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ),
        (
            SocketId(2),
            "back_center",
            [0.0, -half_thickness, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
        ),
    ])
}

fn sockets<const N: usize>(specs: [SocketTemplate; N]) -> BTreeMap<SocketId, SocketSpec> {
    specs
        .into_iter()
        .map(|(id, name, origin, x_axis, y_axis, z_axis)| {
            (
                id,
                SocketSpec {
                    id,
                    name: name.to_owned(),
                    local_frame: Frame3 {
                        origin,
                        x_axis,
                        y_axis,
                        z_axis,
                    },
                    role: "attachment".to_owned(),
                    tags: BTreeSet::new(),
                },
            )
        })
        .collect()
}

fn part(
    mesh: PolygonMesh,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    generator_signature: String,
) -> GeneratedPart {
    let local_bounds = mesh.bounds;
    GeneratedPart {
        mesh,
        sockets,
        regions,
        local_bounds,
        generator_signature,
    }
}
