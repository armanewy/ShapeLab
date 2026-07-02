
impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            vertex_ids: Vec::new(),
            faces: Vec::new(),
            face_metadata: Vec::new(),
        }
    }

    fn add_ring(
        &mut self,
        points: &[[f32; 2]],
        z: f32,
        basis: &FrameBasis,
    ) -> FeatureResult<Vec<u32>> {
        let positions = points
            .iter()
            .map(|point| basis.transform_local([point[0], point[1], z]))
            .collect::<Vec<_>>();
        self.add_positions(&positions)
    }

    fn add_positions(&mut self, positions: &[[f32; 3]]) -> FeatureResult<Vec<u32>> {
        positions
            .iter()
            .copied()
            .map(|position| self.add_position(position))
            .collect()
    }

    fn add_position(&mut self, position: [f32; 3]) -> FeatureResult<u32> {
        if position.iter().any(|component| !component.is_finite()) {
            return validation("feature", "generated non-finite vertex position");
        }
        let index = u32::try_from(self.positions.len()).map_err(|_| {
            FeatureError::Modeling(ModelingError::InvalidInput(
                "generated mesh exceeded u32 index range".to_owned(),
            ))
        })?;
        self.positions.push(position);
        self.vertex_ids.push(ElementId(u64::from(index)));
        Ok(index)
    }

    fn add_face(
        &mut self,
        vertices: Vec<u32>,
        operation: OperationId,
        region: RegionId,
        role: SurfaceRole,
        smoothing_group: Option<u32>,
    ) {
        if vertices.len() < 3 || has_duplicate_indices(&vertices) {
            return;
        }
        let id = ElementId(self.faces.len() as u64);
        self.faces.push(PolygonFace { id, vertices });
        self.face_metadata.push(FaceMetadata {
            part_definition: None,
            part_instance: None,
            region: Some(region),
            operation: Some(operation),
            smoothing_group,
            surface_role: Some(role),
        });
    }

    fn finish(self, operation: OperationId) -> FeatureResult<PolygonMesh> {
        let bounds = bounds_from_positions(&self.positions).map_err(ModelingError::from)?;
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
        mesh.edge_metadata = semantic_edge_metadata(&mesh, operation)?;
        Ok(mesh)
    }
}

fn add_loop_band(
    builder: &mut MeshBuilder,
    first: &[u32],
    second: &[u32],
    style: FaceStyle,
    forward: bool,
) {
    for index in 0..first.len() {
        let next = (index + 1) % first.len();
        let mut face = vec![first[index], first[next], second[next], second[index]];
        if !forward {
            face.reverse();
        }
        builder.add_face(
            face,
            style.operation,
            style.region,
            style.role.clone(),
            style.smoothing_group,
        );
    }
}

#[derive(Clone)]
struct FaceStyle {
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
    smoothing_group: Option<u32>,
}

fn add_cap_face(
    builder: &mut MeshBuilder,
    ring: &[u32],
    operation: OperationId,
    region: RegionId,
    role: SurfaceRole,
    forward: bool,
) {
    let mut face = ring.to_vec();
    if !forward {
        face.reverse();
    }
    builder.add_face(face, operation, region, role, None);
}

fn semantic_edge_metadata(
    mesh: &PolygonMesh,
    operation: OperationId,
) -> Result<BTreeMap<EdgeKey, EdgeMetadata>, ModelingError> {
    let adjacency = build_adjacency(mesh)?;
    let mut metadata = BTreeMap::new();
    for (edge, faces) in adjacency.edge_faces {
        let transition = region_transition(mesh, &faces);
        let open = faces.len() == 1;
        let region_changes = transition.is_some();
        let boundary_role = if open {
            BoundaryRole::OpenBoundary
        } else if region_changes {
            BoundaryRole::Feature
        } else {
            BoundaryRole::Smooth
        };
        let classification = if open || region_changes {
            EdgeClassification::Hard
        } else {
            EdgeClassification::Smooth
        };
        metadata.insert(
            edge,
            EdgeMetadata {
                boundary_role,
                classification,
                seam_candidate: open,
                bevel_eligible: false,
                operation: Some(operation),
                region_transition: transition,
                boundary_loop: None,
            },
        );
    }
    Ok(metadata)
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

fn remap_mesh_ids(mesh: &mut PolygonMesh) {
    for (index, vertex_id) in mesh.vertex_ids.iter_mut().enumerate() {
        *vertex_id = ElementId(index as u64);
    }
    for (index, face) in mesh.faces.iter_mut().enumerate() {
        face.id = ElementId(index as u64);
    }
}
