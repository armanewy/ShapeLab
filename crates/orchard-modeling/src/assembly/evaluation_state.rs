
/// Generator adapter that delegates to [`generate_geometry`].
#[derive(Debug, Copy, Clone, Default)]
pub struct DispatchGeometryGenerator;

impl GeometryGenerator for DispatchGeometryGenerator {
    fn generate(
        &self,
        definition: &PartDefinition,
        context: &mut GeneratorContext,
    ) -> Result<GeneratedPart, ModelingError> {
        generate_geometry(definition, context)
    }
}

/// Evaluate an asset recipe using the default generator dispatch and recipe-level assembly ops.
pub fn evaluate_assembly(recipe: &AssetRecipe) -> Result<AssemblyEvaluation, AssemblyError> {
    let generator = DispatchGeometryGenerator;
    evaluate_assembly_with_generator(recipe, &generator)
}

/// Evaluate an asset recipe using an injected generator and recipe-level assembly ops.
pub fn evaluate_assembly_with_generator(
    recipe: &AssetRecipe,
    generator: &impl GeometryGenerator,
) -> Result<AssemblyEvaluation, AssemblyError> {
    let plan = AssemblyPlan::from_recipe_operations(recipe);
    evaluate_assembly_plan_with_generator(recipe, &plan, generator)
}

/// Evaluate an asset recipe using an explicit assembly plan and injected generator.
pub fn evaluate_assembly_plan_with_generator(
    recipe: &AssetRecipe,
    plan: &AssemblyPlan,
    generator: &impl GeometryGenerator,
) -> Result<AssemblyEvaluation, AssemblyError> {
    validate_attachments(recipe)?;
    let enabled_instances = recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
        .map(|instance| instance.id)
        .collect::<BTreeSet<_>>();
    detect_attachment_cycles(recipe, &enabled_instances)?;
    let base_order = ordered_enabled_instances(recipe, &enabled_instances)?;
    let mut state = AssemblyState::new(recipe, generator, &enabled_instances)?;

    for instance_id in base_order {
        let transform = state.resolve_base_transform(instance_id)?;
        let instance = recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        state.push_occurrence(OccurrenceInput {
            instance_id,
            definition_id: instance.definition,
            transform,
            prototype_instance_id: None,
            generated_by: instance.generated_by,
            source_recipe_instance: true,
        })?;
    }

    for operation in &plan.operations {
        state.apply_operation(operation)?;
    }

    state.finish()
}

struct OccurrenceInput {
    instance_id: PartInstanceId,
    definition_id: PartDefinitionId,
    transform: AffineTransform3,
    prototype_instance_id: Option<PartInstanceId>,
    generated_by: Option<OperationId>,
    source_recipe_instance: bool,
}

struct AssemblyState<'a, G> {
    recipe: &'a AssetRecipe,
    local_parts: BTreeMap<PartDefinitionId, AssemblyCompiledPart>,
    instances: Vec<AssemblyInstance>,
    world_transforms: BTreeMap<PartInstanceId, AffineTransform3>,
    world_sockets: BTreeMap<PartInstanceId, BTreeMap<SocketId, SocketSpec>>,
    world_meshes: BTreeMap<PartInstanceId, PolygonMesh>,
    instance_bounds: BTreeMap<PartInstanceId, MeshBounds>,
    base_transforms: BTreeMap<PartInstanceId, AffineTransform3>,
    resolving: BTreeSet<PartInstanceId>,
    next_generated_instance_id: u64,
    definition_generation_order: Vec<PartDefinitionId>,
    _generator: &'a G,
}

impl<'a, G: GeometryGenerator> AssemblyState<'a, G> {
    fn new(
        recipe: &'a AssetRecipe,
        generator: &'a G,
        enabled_instances: &BTreeSet<PartInstanceId>,
    ) -> Result<Self, AssemblyError> {
        let mut definitions = enabled_instances
            .iter()
            .filter_map(|instance_id| recipe.instances.get(instance_id))
            .map(|instance| instance.definition)
            .collect::<BTreeSet<_>>();
        for definition in &definitions {
            if !recipe.definitions.contains_key(definition) {
                return Err(AssemblyError::UnknownDefinition(*definition));
            }
        }

        let mut local_parts = BTreeMap::new();
        let mut definition_generation_order = Vec::new();
        for definition_id in std::mem::take(&mut definitions) {
            let definition = recipe
                .definitions
                .get(&definition_id)
                .ok_or(AssemblyError::UnknownDefinition(definition_id))?;
            let context_instance = enabled_instances
                .iter()
                .filter_map(|instance_id| recipe.instances.get(instance_id))
                .find(|instance| instance.definition == definition_id)
                .map(|instance| instance.id)
                .unwrap_or_default();
            let mut context = GeneratorContext::new(
                definition_id,
                context_instance,
                recipe.next_ids.operation,
                recipe.next_ids.revision,
            );
            let generated = generator.generate(definition, &mut context)?;
            let mut sockets = definition.sockets.clone();
            sockets.extend(generated.sockets);
            let mut regions = definition.regions.clone();
            regions.extend(generated.regions);
            let local_bounds = if generated.local_bounds.is_empty() {
                generated.mesh.bounds
            } else {
                generated.local_bounds
            };
            local_parts.insert(
                definition_id,
                AssemblyCompiledPart {
                    definition_id,
                    local_mesh: generated.mesh,
                    sockets,
                    regions,
                    local_bounds,
                    generator_signature: generated.generator_signature,
                },
            );
            definition_generation_order.push(definition_id);
        }

        let max_instance_id = recipe
            .instances
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or_default();
        let next_generated_instance_id = recipe
            .next_ids
            .part_instance
            .max(max_instance_id.saturating_add(1));

        Ok(Self {
            recipe,
            local_parts,
            instances: Vec::new(),
            world_transforms: BTreeMap::new(),
            world_sockets: BTreeMap::new(),
            world_meshes: BTreeMap::new(),
            instance_bounds: BTreeMap::new(),
            base_transforms: BTreeMap::new(),
            resolving: BTreeSet::new(),
            next_generated_instance_id,
            definition_generation_order,
            _generator: generator,
        })
    }

    fn resolve_base_transform(
        &mut self,
        instance_id: PartInstanceId,
    ) -> Result<AffineTransform3, AssemblyError> {
        if let Some(transform) = self.base_transforms.get(&instance_id) {
            return Ok(*transform);
        }
        if !self.resolving.insert(instance_id) {
            return Err(AssemblyError::AttachmentCycle(instance_id));
        }

        let instance = self
            .recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        let transform = if let Some(attachment) = &instance.attachment {
            if attachment.mode != AttachmentMode::RigidSeparate {
                return Err(AssemblyError::Unsupported {
                    feature: "WeldBoundaryReserved".to_owned(),
                });
            }
            let parent_transform = self.resolve_base_transform(attachment.parent_instance)?;
            let parent = self
                .recipe
                .instances
                .get(&attachment.parent_instance)
                .ok_or(AssemblyError::UnknownInstance(attachment.parent_instance))?;
            let parent_socket =
                self.socket(parent.id, parent.definition, attachment.parent_socket)?;
            let child_socket =
                self.socket(instance.id, instance.definition, attachment.child_socket)?;
            parent_transform
                .compose(&AffineTransform3::from_frame(&parent_socket.local_frame))
                .compose(&AffineTransform3::from_transform(&attachment.local_offset))
                .compose(&AffineTransform3::from_frame(&child_socket.local_frame).inverse()?)
        } else if let Some(parent_id) = instance.parent {
            let parent_transform = self.resolve_base_transform(parent_id)?;
            parent_transform.compose(&AffineTransform3::from_transform(&instance.local_transform))
        } else {
            AffineTransform3::from_transform(&instance.local_transform)
        };

        self.resolving.remove(&instance_id);
        self.base_transforms.insert(instance_id, transform);
        Ok(transform)
    }

    fn socket(
        &self,
        instance: PartInstanceId,
        definition: PartDefinitionId,
        socket: SocketId,
    ) -> Result<&SocketSpec, AssemblyError> {
        self.local_parts
            .get(&definition)
            .ok_or(AssemblyError::UnknownDefinition(definition))?
            .sockets
            .get(&socket)
            .ok_or(AssemblyError::MissingSocket {
                instance,
                definition,
                socket,
            })
    }

    fn push_occurrence(&mut self, input: OccurrenceInput) -> Result<(), AssemblyError> {
        let local_part = self
            .local_parts
            .get(&input.definition_id)
            .ok_or(AssemblyError::UnknownDefinition(input.definition_id))?;
        let world_mesh = transform_mesh_for_instance(
            &local_part.local_mesh,
            input.definition_id,
            input.instance_id,
            input.generated_by,
            &input.transform,
        )?;
        let sockets = transform_sockets(&local_part.sockets, &input.transform);
        self.instance_bounds
            .insert(input.instance_id, world_mesh.bounds);
        self.world_transforms
            .insert(input.instance_id, input.transform);
        self.world_sockets.insert(input.instance_id, sockets);
        self.world_meshes.insert(input.instance_id, world_mesh);
        self.instances.push(AssemblyInstance {
            instance_id: input.instance_id,
            definition_id: input.definition_id,
            prototype_instance_id: input.prototype_instance_id,
            generated_by: input.generated_by,
            source_recipe_instance: input.source_recipe_instance,
        });
        Ok(())
    }

    fn apply_operation(&mut self, operation: &AssemblyOperation) -> Result<(), AssemblyError> {
        match operation {
            AssemblyOperation::Mirror(operation) => self.apply_mirror(operation),
            AssemblyOperation::LinearArray(operation) => self.apply_linear_array(operation),
            AssemblyOperation::RadialArray(operation) => self.apply_radial_array(operation),
        }
    }

    fn apply_mirror(&mut self, operation: &MirrorOperation) -> Result<(), AssemblyError> {
        let reflection = AffineTransform3::reflection(operation.plane)?;
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            let instance_id = self.allocate_generated_instance_id();
            let transform = reflection.compose(&prototype_transform);
            self.push_occurrence(OccurrenceInput {
                instance_id,
                definition_id,
                transform,
                prototype_instance_id: Some(*prototype),
                generated_by: Some(operation.operation),
                source_recipe_instance: false,
            })?;
        }
        Ok(())
    }

    fn apply_linear_array(
        &mut self,
        operation: &LinearArrayOperation,
    ) -> Result<(), AssemblyError> {
        if operation.count == 0 {
            return Err(AssemblyError::InvalidInput(
                "linear array count must be at least one".to_owned(),
            ));
        }
        let step = AffineTransform3::from_transform(&operation.step);
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            for index in linear_generated_indices(operation.count, operation.centered) {
                let instance_id = self.allocate_generated_instance_id();
                let step_transform = transform_power(&step, index)?;
                let transform = prototype_transform.compose(&step_transform);
                self.push_occurrence(OccurrenceInput {
                    instance_id,
                    definition_id,
                    transform,
                    prototype_instance_id: Some(*prototype),
                    generated_by: Some(operation.operation),
                    source_recipe_instance: false,
                })?;
            }
        }
        Ok(())
    }

    fn apply_radial_array(
        &mut self,
        operation: &RadialArrayOperation,
    ) -> Result<(), AssemblyError> {
        if operation.count == 0 {
            return Err(AssemblyError::InvalidInput(
                "radial array count must be at least one".to_owned(),
            ));
        }
        let denominator = operation.count.saturating_sub(1).max(1) as f32;
        for prototype in &operation.prototypes {
            let (definition_id, prototype_transform) = self.prototype(*prototype)?;
            for index in 1..operation.count {
                let angle = operation.angular_span_degrees * index as f32 / denominator;
                let radial =
                    AffineTransform3::rotation_about_axis(operation.center, operation.axis, angle)?;
                let transform = if operation.rotate_instances {
                    radial.compose(&prototype_transform)
                } else {
                    let origin = prototype_transform.transform_point([0.0, 0.0, 0.0]);
                    let rotated_origin = radial.transform_point(origin);
                    AffineTransform3::translation(sub(rotated_origin, origin))
                        .compose(&prototype_transform)
                };
                let instance_id = self.allocate_generated_instance_id();
                self.push_occurrence(OccurrenceInput {
                    instance_id,
                    definition_id,
                    transform,
                    prototype_instance_id: Some(*prototype),
                    generated_by: Some(operation.operation),
                    source_recipe_instance: false,
                })?;
            }
        }
        Ok(())
    }

    fn prototype(
        &self,
        instance_id: PartInstanceId,
    ) -> Result<(PartDefinitionId, AffineTransform3), AssemblyError> {
        let instance = self
            .recipe
            .instances
            .get(&instance_id)
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        if !instance.enabled {
            return Err(AssemblyError::InvalidInput(format!(
                "prototype instance {} is disabled",
                instance_id.0
            )));
        }
        let transform = self
            .world_transforms
            .get(&instance_id)
            .copied()
            .ok_or(AssemblyError::UnknownInstance(instance_id))?;
        Ok((instance.definition, transform))
    }

    fn allocate_generated_instance_id(&mut self) -> PartInstanceId {
        let id = PartInstanceId(self.next_generated_instance_id);
        self.next_generated_instance_id = self.next_generated_instance_id.saturating_add(1);
        id
    }

    fn finish(self) -> Result<AssemblyEvaluation, AssemblyError> {
        let mut ordered_meshes = Vec::new();
        let mut next_preview_vertex_id = 0;
        let mut next_preview_face_id = 0;
        for instance in &self.instances {
            if let Some(mesh) = self.world_meshes.get(&instance.instance_id) {
                let mut mesh = mesh.clone();
                remap_preview_element_ids(
                    &mut mesh,
                    &mut next_preview_vertex_id,
                    &mut next_preview_face_id,
                )?;
                ordered_meshes.push(mesh);
            }
        }
        let combined_preview_mesh = combine_polygon_meshes(&ordered_meshes)?;
        let combined_preview = triangulate_polygon_mesh(&combined_preview_mesh)?;
        let provenance = build_provenance(&self);
        Ok(AssemblyEvaluation {
            local_parts: self.local_parts,
            instances: self.instances,
            world_transforms: self.world_transforms,
            world_sockets: self.world_sockets,
            world_meshes: self.world_meshes,
            combined_preview_mesh,
            combined_preview,
            instance_bounds: self.instance_bounds,
            provenance,
        })
    }
}
