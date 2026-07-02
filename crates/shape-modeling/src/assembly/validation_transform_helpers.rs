
fn validate_attachments(recipe: &AssetRecipe) -> Result<(), AssemblyError> {
    for instance in recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
    {
        if let Some(attachment) = &instance.attachment {
            if attachment.mode == AttachmentMode::WeldBoundaryReserved {
                return Err(AssemblyError::Unsupported {
                    feature: "WeldBoundaryReserved".to_owned(),
                });
            }
            let parent = recipe
                .instances
                .get(&attachment.parent_instance)
                .ok_or(AssemblyError::UnknownInstance(attachment.parent_instance))?;
            if !parent.enabled {
                return Err(AssemblyError::InvalidInput(format!(
                    "attachment parent {} is disabled",
                    parent.id.0
                )));
            }
        }
    }
    Ok(())
}

fn detect_attachment_cycles(
    recipe: &AssetRecipe,
    enabled_instances: &BTreeSet<PartInstanceId>,
) -> Result<(), AssemblyError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for instance_id in enabled_instances {
        detect_cycle_from(
            recipe,
            *instance_id,
            enabled_instances,
            &mut visiting,
            &mut visited,
        )?;
    }
    Ok(())
}

fn detect_cycle_from(
    recipe: &AssetRecipe,
    instance_id: PartInstanceId,
    enabled_instances: &BTreeSet<PartInstanceId>,
    visiting: &mut BTreeSet<PartInstanceId>,
    visited: &mut BTreeSet<PartInstanceId>,
) -> Result<(), AssemblyError> {
    if visited.contains(&instance_id) {
        return Ok(());
    }
    if !visiting.insert(instance_id) {
        return Err(AssemblyError::AttachmentCycle(instance_id));
    }
    let instance = recipe
        .instances
        .get(&instance_id)
        .ok_or(AssemblyError::UnknownInstance(instance_id))?;
    if let Some(parent) = parent_relation(instance) {
        if enabled_instances.contains(&parent) {
            detect_cycle_from(recipe, parent, enabled_instances, visiting, visited)?;
        } else {
            return Err(AssemblyError::UnknownInstance(parent));
        }
    }
    visiting.remove(&instance_id);
    visited.insert(instance_id);
    Ok(())
}

fn ordered_enabled_instances(
    recipe: &AssetRecipe,
    enabled_instances: &BTreeSet<PartInstanceId>,
) -> Result<Vec<PartInstanceId>, AssemblyError> {
    let mut children = BTreeMap::<PartInstanceId, Vec<PartInstanceId>>::new();
    for instance_id in enabled_instances {
        let instance = recipe
            .instances
            .get(instance_id)
            .ok_or(AssemblyError::UnknownInstance(*instance_id))?;
        if let Some(parent) = parent_relation(instance) {
            children.entry(parent).or_default().push(*instance_id);
        }
    }
    for child_ids in children.values_mut() {
        child_ids.sort();
    }

    let mut order = Vec::new();
    let mut visited = BTreeSet::new();
    for root in &recipe.root_instances {
        if enabled_instances.contains(root) {
            visit_instance_order(*root, &children, &mut visited, &mut order);
        }
    }
    for instance_id in enabled_instances {
        visit_instance_order(*instance_id, &children, &mut visited, &mut order);
    }
    Ok(order)
}

fn visit_instance_order(
    instance_id: PartInstanceId,
    children: &BTreeMap<PartInstanceId, Vec<PartInstanceId>>,
    visited: &mut BTreeSet<PartInstanceId>,
    order: &mut Vec<PartInstanceId>,
) {
    if !visited.insert(instance_id) {
        return;
    }
    order.push(instance_id);
    if let Some(child_ids) = children.get(&instance_id) {
        for child in child_ids {
            visit_instance_order(*child, children, visited, order);
        }
    }
}

fn parent_relation(instance: &PartInstance) -> Option<PartInstanceId> {
    instance
        .attachment
        .as_ref()
        .map(|attachment| attachment.parent_instance)
        .or(instance.parent)
}

fn transform_mesh_for_instance(
    mesh: &PolygonMesh,
    definition_id: PartDefinitionId,
    instance_id: PartInstanceId,
    generated_by: Option<OperationId>,
    transform: &AffineTransform3,
) -> Result<PolygonMesh, AssemblyError> {
    let mut transformed = mesh.clone();
    transformed.positions = mesh
        .positions
        .iter()
        .map(|position| transform.transform_point(*position))
        .collect();
    if transform.determinant() < 0.0 {
        for face in &mut transformed.faces {
            face.vertices.reverse();
        }
    }
    for metadata in &mut transformed.face_metadata {
        fill_metadata(metadata, definition_id, instance_id, generated_by);
    }
    transformed.bounds = bounds_from_positions(&transformed.positions)?;
    transformed.topology_signature =
        compute_topology_signature(&transformed.positions, &transformed.faces);
    Ok(transformed)
}

fn remap_preview_element_ids(
    mesh: &mut PolygonMesh,
    next_vertex_id: &mut u64,
    next_face_id: &mut u64,
) -> Result<(), AssemblyError> {
    for vertex_id in &mut mesh.vertex_ids {
        *vertex_id = ElementId(*next_vertex_id);
        *next_vertex_id = next_vertex_id.checked_add(1).ok_or_else(|| {
            AssemblyError::InvalidInput("combined preview vertex ElementId overflow".to_owned())
        })?;
    }
    for face in &mut mesh.faces {
        face.id = ElementId(*next_face_id);
        *next_face_id = next_face_id.checked_add(1).ok_or_else(|| {
            AssemblyError::InvalidInput("combined preview face ElementId overflow".to_owned())
        })?;
    }
    Ok(())
}

fn fill_metadata(
    metadata: &mut FaceMetadata,
    definition_id: PartDefinitionId,
    instance_id: PartInstanceId,
    generated_by: Option<OperationId>,
) {
    metadata.part_definition = Some(definition_id);
    metadata.part_instance = Some(instance_id);
    metadata.operation = metadata.operation.or(generated_by);
}

fn transform_sockets(
    sockets: &BTreeMap<SocketId, SocketSpec>,
    transform: &AffineTransform3,
) -> BTreeMap<SocketId, SocketSpec> {
    sockets
        .iter()
        .map(|(socket_id, socket)| {
            let mut socket = socket.clone();
            socket.local_frame = transform.transform_frame(&socket.local_frame);
            (*socket_id, socket)
        })
        .collect()
}

fn build_provenance<G>(state: &AssemblyState<'_, G>) -> AssemblyProvenance {
    let instances = state
        .instances
        .iter()
        .filter_map(|instance| {
            state
                .world_meshes
                .get(&instance.instance_id)
                .map(|mesh| AssemblyInstanceProvenance {
                    instance_id: instance.instance_id,
                    definition_id: instance.definition_id,
                    prototype_instance_id: instance.prototype_instance_id,
                    generated_by: instance.generated_by,
                    polygon_vertex_count: mesh.positions.len() as u64,
                    polygon_face_count: mesh.faces.len() as u64,
                })
        })
        .collect::<Vec<_>>();
    AssemblyProvenance {
        definition_generation_order: state.definition_generation_order.clone(),
        instance_order: state
            .instances
            .iter()
            .map(|instance| instance.instance_id)
            .collect(),
        instances,
    }
}

fn linear_generated_indices(count: u32, centered: bool) -> Vec<i32> {
    if count <= 1 {
        return Vec::new();
    }
    if centered {
        let center = (count / 2) as i32;
        (0..count)
            .map(|index| index as i32 - center)
            .filter(|index| *index != 0)
            .collect()
    } else {
        (1..count).map(|index| index as i32).collect()
    }
}

fn transform_power(
    transform: &AffineTransform3,
    exponent: i32,
) -> Result<AffineTransform3, AssemblyError> {
    if exponent == 0 {
        return Ok(AffineTransform3::identity());
    }
    if exponent < 0 {
        return transform_power(&transform.inverse()?, exponent.saturating_abs());
    }
    let mut result = AffineTransform3::identity();
    for _ in 0..exponent {
        result = result.compose(transform);
    }
    Ok(result)
}

fn determinant_3x3(matrix: [[f32; 3]; 3]) -> f32 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn inverse_3x3(matrix: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let determinant = determinant_3x3(matrix);
    if !determinant.is_finite() || determinant.abs() <= EPSILON {
        return None;
    }
    let inv_det = 1.0 / determinant;
    Some([
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) * inv_det,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) * inv_det,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) * inv_det,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) * inv_det,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) * inv_det,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) * inv_det,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) * inv_det,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) * inv_det,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) * inv_det,
        ],
    ])
}

fn mul_mat3_vec(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    if !array_is_finite(vector) {
        return None;
    }
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= EPSILON {
        None
    } else {
        Some([vector[0] / length, vector[1] / length, vector[2] / length])
    }
}

fn array_is_finite(values: [f32; 3]) -> bool {
    values.iter().copied().all(f32::is_finite)
}

fn scale(vector: [f32; 3], scale: f32) -> [f32; 3] {
    [vector[0] * scale, vector[1] * scale, vector[2] * scale]
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}
