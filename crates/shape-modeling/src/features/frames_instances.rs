
fn fastener_frames(placement: &FastenerPlacement) -> FeatureResult<Vec<Frame3>> {
    match placement {
        FastenerPlacement::Linear {
            start,
            end,
            count,
            spacing,
            up_hint,
        } => span_frames(*start, *end, *count, *spacing, *up_hint, "fastener"),
        FastenerPlacement::Radial {
            center,
            radius,
            axis,
            count,
            start_angle_degrees,
            angular_spacing_degrees,
        } => radial_frames(
            *center,
            *radius,
            *axis,
            *count,
            *start_angle_degrees,
            *angular_spacing_degrees,
        ),
        FastenerPlacement::Perimeter {
            points,
            closed,
            count,
            spacing,
            up_hint,
        } => perimeter_frames(points, *closed, *count, *spacing, *up_hint),
    }
}

fn span_frames(
    start: [f32; 3],
    end: [f32; 3],
    count: u32,
    spacing: PatternSpacing,
    up_hint: [f32; 3],
    feature: &'static str,
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation(feature, "count must be at least one");
    }
    let start = Vec3::from_array(start, feature)?;
    let end = Vec3::from_array(end, feature)?;
    let span = end.sub(start);
    let length = span.length();
    if length <= EPSILON {
        return validation(feature, "span endpoints must not collapse");
    }
    let y = span.scale(1.0 / length);
    let up = Vec3::from_array(up_hint, feature)?.normalized(feature, "up hint must be non-zero")?;
    if y.dot(up).abs() > 0.999 {
        return validation(feature, "up hint must not be parallel to the span");
    }
    let z = up
        .sub(y.scale(up.dot(y)))
        .normalized(feature, "frame is degenerate")?;
    let x = y.cross(z).normalized(feature, "frame is degenerate")?;
    let spacing_mode = spacing;
    let spacing = match spacing_mode {
        PatternSpacing::Fit => {
            if count <= 1 {
                0.0
            } else {
                length / (count - 1) as f32
            }
        }
        PatternSpacing::Fixed(value) => {
            let value = positive(value, feature, "spacing")?;
            let occupied = value * count.saturating_sub(1) as f32;
            if occupied > length + EPSILON {
                return validation(feature, "fixed spacing does not fit between endpoints");
            }
            value
        }
    };
    let start_offset = if count <= 1 {
        length * 0.5
    } else {
        match spacing_mode {
            PatternSpacing::Fit => 0.0,
            PatternSpacing::Fixed(_) => (length - spacing * count.saturating_sub(1) as f32) * 0.5,
        }
    };
    Ok((0..count)
        .map(|index| {
            let origin = start.add(y.scale(start_offset + spacing * index as f32));
            Frame3 {
                origin: origin.to_array(),
                x_axis: x.to_array(),
                y_axis: y.to_array(),
                z_axis: z.to_array(),
            }
        })
        .collect())
}

fn radial_frames(
    center: [f32; 3],
    radius: f32,
    axis: [f32; 3],
    count: u32,
    start_angle_degrees: f32,
    angular_spacing_degrees: f32,
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation("fastener", "count must be at least one");
    }
    let radius = positive(radius, "fastener", "radius")?;
    let center = Vec3::from_array(center, "fastener")?;
    let y = Vec3::from_array(axis, "fastener")?.normalized("fastener", "axis must be non-zero")?;
    let reference = perpendicular(y);
    let z0 = y
        .cross(reference)
        .normalized("fastener", "radial frame is degenerate")?;
    let x0 = y
        .cross(z0)
        .normalized("fastener", "radial frame is degenerate")?;
    let mut frames = Vec::with_capacity(count as usize);
    for index in 0..count {
        let angle = (start_angle_degrees + angular_spacing_degrees * index as f32).to_radians();
        if !angle.is_finite() {
            return validation("fastener", "radial angle must be finite");
        }
        let radial = x0.scale(angle.cos()).add(z0.scale(angle.sin()));
        let tangent = y
            .cross(radial)
            .normalized("fastener", "radial tangent is degenerate")?;
        frames.push(Frame3 {
            origin: center.add(radial.scale(radius)).to_array(),
            x_axis: tangent.to_array(),
            y_axis: y.to_array(),
            z_axis: radial.to_array(),
        });
    }
    Ok(frames)
}

fn perimeter_frames(
    points: &[[f32; 3]],
    closed: bool,
    count: u32,
    spacing: PatternSpacing,
    up_hint: [f32; 3],
) -> FeatureResult<Vec<Frame3>> {
    if count == 0 {
        return validation("fastener", "count must be at least one");
    }
    let samples = sample_polyline(points, closed, count, spacing, "fastener")?;
    let up = Vec3::from_array(up_hint, "fastener")?
        .normalized("fastener", "up hint must be non-zero")?;
    samples
        .into_iter()
        .map(|sample| {
            let y = sample
                .tangent
                .normalized("fastener", "perimeter tangent must be non-zero")?;
            if y.dot(up).abs() > 0.999 {
                return validation(
                    "fastener",
                    "up hint must not be parallel to perimeter tangent",
                );
            }
            let z = up
                .sub(y.scale(up.dot(y)))
                .normalized("fastener", "perimeter frame is degenerate")?;
            let x = y
                .cross(z)
                .normalized("fastener", "perimeter frame is degenerate")?;
            Ok(Frame3 {
                origin: sample.position.to_array(),
                x_axis: x.to_array(),
                y_axis: y.to_array(),
                z_axis: z.to_array(),
            })
        })
        .collect()
}

fn sample_polyline(
    points: &[[f32; 3]],
    closed: bool,
    count: u32,
    spacing: PatternSpacing,
    feature: &'static str,
) -> FeatureResult<Vec<PathSample>> {
    let min_points = if closed { 3 } else { 2 };
    if points.len() < min_points {
        return validation(feature, "path has too few points");
    }
    let points = points
        .iter()
        .map(|point| Vec3::from_array(*point, feature))
        .collect::<FeatureResult<Vec<_>>>()?;
    let segment_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut segments = Vec::with_capacity(segment_count);
    let mut total_length = 0.0;
    for index in 0..segment_count {
        let next = (index + 1) % points.len();
        let delta = points[next].sub(points[index]);
        let length = delta.length();
        if length <= EPSILON {
            return validation(feature, "path contains a collapsed segment");
        }
        segments.push((index, next, length));
        total_length += length;
    }
    let step = match spacing {
        PatternSpacing::Fit => {
            if count <= 1 {
                0.0
            } else if closed {
                total_length / count as f32
            } else {
                total_length / (count - 1) as f32
            }
        }
        PatternSpacing::Fixed(value) => {
            let value = positive(value, feature, "spacing")?;
            let occupied = value * count.saturating_sub(1) as f32;
            if !closed && occupied > total_length + EPSILON {
                return validation(feature, "fixed spacing does not fit along path");
            }
            value
        }
    };
    let start_offset = if count <= 1 {
        total_length * 0.5
    } else if closed || matches!(spacing, PatternSpacing::Fit) {
        0.0
    } else {
        (total_length - step * count.saturating_sub(1) as f32) * 0.5
    };
    let mut samples = Vec::with_capacity(count as usize);
    for index in 0..count {
        let mut distance = start_offset + step * index as f32;
        if closed {
            distance %= total_length;
        }
        let mut cursor = 0.0;
        for (from, to, length) in &segments {
            if distance <= cursor + *length + EPSILON {
                let t = ((distance - cursor) / *length).clamp(0.0, 1.0);
                let tangent = points[*to].sub(points[*from]);
                samples.push(PathSample {
                    position: points[*from].add(tangent.scale(t)),
                    tangent,
                });
                break;
            }
            cursor += *length;
        }
    }
    Ok(samples)
}

fn feature_instances(
    name_prefix: &str,
    definition: PartDefinitionId,
    first_instance: PartInstanceId,
    operation: OperationId,
    frames: &[Frame3],
    tags: &[&str],
) -> BTreeMap<PartInstanceId, SemanticFeatureInstance> {
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let id = PartInstanceId(first_instance.0 + index as u64);
            let part_instance = PartInstance {
                id,
                definition,
                name: format!("{name_prefix} {}", index + 1),
                parent: None,
                local_transform: Transform3 {
                    translation: frame.origin,
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: string_set(tags),
                generated_by: Some(operation),
            };
            (
                id,
                SemanticFeatureInstance {
                    part_instance,
                    frame: frame.clone(),
                },
            )
        })
        .collect()
}

fn combine_feature_instances(
    prototype: &GeneratedPart,
    frames: &[Frame3],
    operation: OperationId,
    definition: PartDefinitionId,
    instance: PartInstanceId,
    label: &'static str,
) -> FeatureResult<GeneratedPart> {
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    let mut metadata = Vec::new();
    for frame in frames {
        let basis = basis_from_frame(frame, label)?;
        let vertex_offset = u32::try_from(positions.len()).map_err(|_| {
            FeatureError::Modeling(ModelingError::InvalidInput(
                "combined feature exceeded u32 index range".to_owned(),
            ))
        })?;
        positions.extend(
            prototype
                .mesh
                .positions
                .iter()
                .map(|position| basis.transform_local(*position)),
        );
        for face in &prototype.mesh.faces {
            faces.push(
                face.vertices
                    .iter()
                    .map(|vertex| vertex + vertex_offset)
                    .collect::<Vec<_>>(),
            );
        }
        metadata.extend(
            prototype
                .mesh
                .face_metadata
                .iter()
                .cloned()
                .map(|mut item| {
                    item.part_definition = Some(definition);
                    item.part_instance = Some(instance);
                    item.operation = Some(operation);
                    item
                }),
        );
    }
    let mut mesh =
        polygon_mesh_from_faces(positions, faces, metadata).map_err(ModelingError::from)?;
    remap_mesh_ids(&mut mesh);
    mesh.edge_metadata = semantic_edge_metadata(&mesh, operation)?;
    Ok(generated_part(
        mesh,
        prototype.regions.clone(),
        BTreeMap::new(),
        format!("{label}:v1:instances={}", frames.len()),
    ))
}

fn literal_definition(
    id: PartDefinitionId,
    name: &str,
    generated: &GeneratedPart,
    tags: &[&str],
) -> PartDefinition {
    PartDefinition {
        id,
        name: name.to_owned(),
        tags: string_set(tags),
        geometry: GeometryRecipe {
            source: GeometrySource::LiteralMesh {
                positions: generated.mesh.positions.clone(),
                faces: generated
                    .mesh
                    .faces
                    .iter()
                    .map(|face| face.vertices.clone())
                    .collect(),
            },
            operations: Vec::new(),
        },
        regions: generated.regions.clone(),
        sockets: generated.sockets.clone(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn assign_operation(mesh: &mut PolygonMesh, operation: OperationId) -> FeatureResult<()> {
    for metadata in &mut mesh.face_metadata {
        metadata.operation = Some(operation);
    }
    mesh.edge_metadata = semantic_edge_metadata(mesh, operation)?;
    Ok(())
}

fn assign_part_context(mesh: &mut PolygonMesh, context: &GeneratorContext) {
    for metadata in &mut mesh.face_metadata {
        metadata.part_definition = Some(context.part_definition);
        metadata.part_instance = Some(context.part_instance);
    }
}

fn resolve_planar_frame(
    host: &FeatureHost,
    target: &PlanarHost,
    feature: &'static str,
) -> FeatureResult<Frame3> {
    match target {
        PlanarHost::Socket(socket) => host
            .sockets
            .get(socket)
            .map(|socket| socket.local_frame.clone())
            .ok_or_else(|| FeatureError::Validation {
                feature,
                message: format!("host socket {} does not exist", socket.0),
            }),
        PlanarHost::SocketName(name) => host
            .sockets
            .values()
            .find(|socket| socket.name == *name)
            .map(|socket| socket.local_frame.clone())
            .ok_or_else(|| FeatureError::Validation {
                feature,
                message: format!("host socket {name} does not exist"),
            }),
        PlanarHost::SurfaceRegion(region) => resolve_region_frame(host, *region, feature),
        PlanarHost::SurfaceRegionName(name) => {
            let region = host
                .regions
                .values()
                .find(|region| region.name == *name)
                .map(|region| region.id)
                .ok_or_else(|| FeatureError::Validation {
                    feature,
                    message: format!("host region {name} does not exist"),
                })?;
            resolve_region_frame(host, region, feature)
        }
    }
}

fn resolve_region_frame(
    host: &FeatureHost,
    region: RegionId,
    feature: &'static str,
) -> FeatureResult<Frame3> {
    if !host.regions.contains_key(&region) {
        return validation(feature, &format!("host region {} does not exist", region.0));
    }
    host.region_frames
        .get(&region)
        .cloned()
        .ok_or_else(|| FeatureError::Validation {
            feature,
            message: format!("host region {} has no planar frame", region.0),
        })
}

fn generated_part(
    mesh: PolygonMesh,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    generator_signature: String,
) -> GeneratedPart {
    GeneratedPart {
        local_bounds: mesh.bounds,
        mesh,
        sockets,
        regions,
        generator_signature,
    }
}

fn feature_regions<const N: usize>(
    specs: [(RegionId, &str, SurfaceRole, &[&str]); N],
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    feature_regions_from_vec(specs.into_iter().collect())
}

fn feature_regions_from_vec(
    specs: Vec<(RegionId, &str, SurfaceRole, &[&str])>,
) -> BTreeMap<RegionId, SurfaceRegionSpec> {
    specs
        .into_iter()
        .map(|(id, name, role, tags)| {
            (
                id,
                SurfaceRegionSpec {
                    id,
                    name: name.to_owned(),
                    role,
                    tags: string_set(tags),
                },
            )
        })
        .collect()
}

fn string_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    vertex_ids: Vec<ElementId>,
    faces: Vec<PolygonFace>,
    face_metadata: Vec<FaceMetadata>,
}
