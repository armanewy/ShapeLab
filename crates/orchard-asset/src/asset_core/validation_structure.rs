
fn validate_definitions(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    let mut seen_operation_ids = BTreeMap::new();
    let mut seen_boundary_loop_ids = BTreeMap::new();
    for (id, definition) in &recipe.definitions {
        if definition.id != *id {
            push_issue(
                report,
                Some(format!("definition.{}", id.0)),
                "definition_id_mismatch",
                "Part definition map key and payload ID differ.",
            );
        }
        if definition.name.trim().is_empty() {
            push_issue(
                report,
                Some(format!("definition.{}", id.0)),
                "empty_definition_name",
                "Part definition name cannot be empty.",
            );
        }
        validate_geometry_source(*id, &definition.geometry.source, report);
        validate_operations(definition, report);
        validate_semantic_cut_host_constraints(definition, report);
        let mut boundary_loop_state = BoundaryLoopValidationState::new(&mut seen_boundary_loop_ids);
        for operation in &definition.geometry.operations {
            let operation_id = operation.operation_id();
            if let Some(previous_definition) = seen_operation_ids.insert(operation_id, *id) {
                push_issue(
                    report,
                    Some(format!(
                        "definition.{}.operation.{}",
                        definition.id.0, operation_id.0
                    )),
                    "duplicate_operation_id",
                    format!(
                        "Operation ID is already used by definition {}.",
                        previous_definition.0
                    ),
                );
            }
            let mut local_declared_outputs = BTreeSet::new();
            for (output_index, boundary_loop) in operation
                .direct_boundary_loop_outputs()
                .into_iter()
                .enumerate()
            {
                if !local_declared_outputs.insert(boundary_loop) {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop.{}",
                            definition.id.0, operation_id.0, output_index
                        )),
                        "duplicate_direct_boundary_loop_output",
                        "Direct boundary loop outputs must be distinct.",
                    );
                }
            }
            for dependency in operation.boundary_loop_dependencies() {
                for (output_index, output) in dependency.outputs.iter().copied().enumerate() {
                    if !local_declared_outputs.insert(output) {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "ambiguous_boundary_loop_output_ownership",
                            "Boundary loop outputs must be owned by exactly one direct output or dependency output.",
                        );
                    }
                }
            }
            for boundary_loop in operation.all_declared_boundary_loop_outputs() {
                boundary_loop_state.validate_new(
                    report,
                    Some(format!(
                        "definition.{}.operation.{}.boundary_loop.{}",
                        definition.id.0, operation_id.0, boundary_loop.0
                    )),
                    boundary_loop,
                    *id,
                    operation_id,
                );
            }
            for dependency in operation.boundary_loop_dependencies() {
                if dependency.input == LEGACY_MISSING_BOUNDARY_LOOP {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input",
                            definition.id.0, operation_id.0
                        )),
                        "invalid_boundary_loop_id",
                        "Boundary loop dependency input must be non-zero.",
                    );
                } else if !boundary_loop_state
                    .definition_boundary_loop_ids
                    .contains_key(&dependency.input)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "unknown_boundary_loop_dependency",
                        "Boundary loop dependency input must be produced earlier in the same definition.",
                    );
                } else if !boundary_loop_state
                    .live_boundary_loop_ids
                    .contains_key(&dependency.input)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "consumed_boundary_loop_dependency",
                        "Boundary loop dependency input must still be live.",
                    );
                }
                if dependency.mode == BoundaryLoopDependencyMode::Consume
                    && let Some(previous_operation) = boundary_loop_state
                        .consumed_boundary_loop_ids
                        .insert(dependency.input, operation_id)
                {
                    push_issue(
                        report,
                        Some(format!(
                            "definition.{}.operation.{}.boundary_loop_dependency.input.{}",
                            definition.id.0, operation_id.0, dependency.input.0
                        )),
                        "duplicate_boundary_loop_consumption",
                        format!(
                            "Boundary loop is already consumed by operation {}.",
                            previous_operation.0
                        ),
                    );
                }
                if dependency.mode == BoundaryLoopDependencyMode::Consume {
                    boundary_loop_state
                        .live_boundary_loop_ids
                        .remove(&dependency.input);
                }
                let mut local_outputs = BTreeSet::new();
                for (output_index, output) in dependency.outputs.into_iter().enumerate() {
                    if output == dependency.input {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "boundary_loop_dependency_self_output",
                            "Replacement boundary loop output must differ from the dependency input.",
                        );
                    }
                    if !local_outputs.insert(output) {
                        push_issue(
                            report,
                            Some(format!(
                                "definition.{}.operation.{}.boundary_loop_dependency.output.{}",
                                definition.id.0, operation_id.0, output_index
                            )),
                            "duplicate_boundary_loop_dependency_output",
                            "Boundary loop dependency outputs must be distinct.",
                        );
                    }
                }
            }
        }
        for (region_id, region) in &definition.regions {
            if region.id != *region_id {
                push_issue(
                    report,
                    Some(format!("definition.{}.region.{}", id.0, region_id.0)),
                    "region_id_mismatch",
                    "Region map key and payload ID differ.",
                );
            }
        }
        for (socket_id, socket) in &definition.sockets {
            if socket.id != *socket_id {
                push_issue(
                    report,
                    Some(format!("definition.{}.socket.{}", id.0, socket_id.0)),
                    "socket_id_mismatch",
                    "Socket map key and payload ID differ.",
                );
            }
            validate_frame(
                report,
                Some(format!("definition.{}.socket.{}", id.0, socket_id.0)),
                &socket.local_frame,
            );
        }
        validate_frame(
            report,
            Some(format!("definition.{}.pivot", id.0)),
            &definition.local_pivot,
        );
    }
}

struct BoundaryLoopValidationState<'a> {
    seen_boundary_loop_ids: &'a mut BTreeMap<BoundaryLoopId, (PartDefinitionId, OperationId)>,
    definition_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
    live_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
    consumed_boundary_loop_ids: BTreeMap<BoundaryLoopId, OperationId>,
}

impl<'a> BoundaryLoopValidationState<'a> {
    fn new(
        seen_boundary_loop_ids: &'a mut BTreeMap<BoundaryLoopId, (PartDefinitionId, OperationId)>,
    ) -> Self {
        Self {
            seen_boundary_loop_ids,
            definition_boundary_loop_ids: BTreeMap::new(),
            live_boundary_loop_ids: BTreeMap::new(),
            consumed_boundary_loop_ids: BTreeMap::new(),
        }
    }

    fn validate_new(
        &mut self,
        report: &mut AssetValidationReport,
        subject: Option<String>,
        boundary_loop: BoundaryLoopId,
        definition: PartDefinitionId,
        operation: OperationId,
    ) {
        if boundary_loop == LEGACY_MISSING_BOUNDARY_LOOP {
            push_issue(
                report,
                subject,
                "invalid_boundary_loop_id",
                "Generated boundary loop IDs must be non-zero.",
            );
            return;
        }
        if let Some((previous_definition, previous_operation)) = self
            .seen_boundary_loop_ids
            .insert(boundary_loop, (definition, operation))
        {
            push_issue(
                report,
                subject,
                "duplicate_boundary_loop_id",
                format!(
                    "Boundary loop ID is already used by definition {} operation {}.",
                    previous_definition.0, previous_operation.0
                ),
            );
            return;
        }
        self.definition_boundary_loop_ids
            .insert(boundary_loop, operation);
        self.live_boundary_loop_ids.insert(boundary_loop, operation);
    }
}

fn validate_instances(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    let mut seen_roots = BTreeSet::new();
    let mut previous_root = None;
    for root in &recipe.root_instances {
        if !seen_roots.insert(*root) {
            push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "duplicate_root_instance",
                "Root instances must not contain duplicates.",
            );
        }
        if let Some(previous_root) = previous_root
            && previous_root > *root
        {
            push_issue(
                report,
                Some("root_instances".to_owned()),
                "unstable_root_order",
                "Root instances must be ordered by semantic instance ID.",
            );
        }
        previous_root = Some(*root);
        match recipe.instances.get(root) {
            Some(instance) if instance.parent.is_some() => push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "root_has_parent",
                "Root instances cannot also declare a parent.",
            ),
            Some(_) => {}
            None => push_issue(
                report,
                Some(format!("instance.{}", root.0)),
                "unknown_root_instance",
                "Root instance does not exist.",
            ),
        }
    }

    for (id, instance) in &recipe.instances {
        if instance.id != *id {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "instance_id_mismatch",
                "Instance map key and payload ID differ.",
            );
        }
        if !recipe.definitions.contains_key(&instance.definition) {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "unknown_instance_definition",
                "Instance references an unknown definition.",
            );
        }
        if instance.parent.is_none() && !seen_roots.contains(id) {
            push_issue(
                report,
                Some(format!("instance.{}", id.0)),
                "missing_root_instance",
                "Parentless instances must be listed as roots.",
            );
        }
        if let Some(parent) = instance.parent {
            if parent == *id {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "self_parent",
                    "Instance cannot parent itself.",
                );
            } else if !recipe.instances.contains_key(&parent) {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "unknown_parent_instance",
                    "Instance references an unknown parent.",
                );
            }
        }
        validate_transform(
            report,
            Some(format!("instance.{}.transform", id.0)),
            &instance.local_transform,
        );
        if let Some(attachment) = &instance.attachment {
            validate_attachment(recipe, *id, attachment, report);
        }
    }
    validate_parent_cycles(recipe, report);
}

fn validate_attachment(
    recipe: &AssetRecipe,
    child: PartInstanceId,
    attachment: &AttachmentSpec,
    report: &mut AssetValidationReport,
) {
    let Some(parent) = recipe.instances.get(&attachment.parent_instance) else {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_attachment_parent",
            "Attachment parent instance does not exist.",
        );
        return;
    };
    let Some(child_instance) = recipe.instances.get(&child) else {
        return;
    };
    if attachment.parent_instance == child {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "self_attachment",
            "Instance cannot attach to itself.",
        );
    }
    if child_instance.parent != Some(attachment.parent_instance) {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "attachment_parent_mismatch",
            "Attachment parent must match the instance parent.",
        );
    }
    let Some(parent_definition) = recipe.definitions.get(&parent.definition) else {
        return;
    };
    let Some(child_definition) = recipe.definitions.get(&child_instance.definition) else {
        return;
    };
    if !parent_definition
        .sockets
        .contains_key(&attachment.parent_socket)
    {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_parent_socket",
            "Attachment parent socket does not exist on the parent definition.",
        );
    }
    if !child_definition
        .sockets
        .contains_key(&attachment.child_socket)
    {
        push_issue(
            report,
            Some(format!("instance.{}", child.0)),
            "unknown_child_socket",
            "Attachment child socket does not exist on the child definition.",
        );
    }
    validate_transform(
        report,
        Some(format!("instance.{}.attachment_offset", child.0)),
        &attachment.local_offset,
    );
}

fn validate_parameters(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (id, parameter) in &recipe.parameters {
        if parameter.id != *id {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "parameter_id_mismatch",
                "Parameter map key and payload ID differ.",
            );
        }
        validate_finite(
            report,
            Some(format!("parameter.{}.minimum", id.0)),
            parameter.minimum,
        );
        validate_finite(
            report,
            Some(format!("parameter.{}.maximum", id.0)),
            parameter.maximum,
        );
        validate_positive(
            report,
            Some(format!("parameter.{}.step", id.0)),
            parameter.step,
        );
        validate_non_negative(
            report,
            Some(format!("parameter.{}.mutation_sigma", id.0)),
            parameter.mutation_sigma,
        );
        if parameter.minimum > parameter.maximum {
            push_issue(
                report,
                Some(format!("parameter.{}", id.0)),
                "invalid_parameter_range",
                "Parameter minimum cannot exceed maximum.",
            );
        }
        match get_scalar(recipe, &parameter.path) {
            Ok(value) => {
                if !value.is_finite() {
                    push_issue(
                        report,
                        Some(format!("parameter.{}", id.0)),
                        "non_finite_parameter_value",
                        "Parameter path resolves to a non-finite scalar.",
                    );
                } else if parameter_range_is_valid(parameter)
                    && (value < parameter.minimum || value > parameter.maximum)
                {
                    push_issue(
                        report,
                        Some(format!("parameter.{}", id.0)),
                        "parameter_value_out_of_range",
                        "Parameter value is outside its descriptor range.",
                    );
                }
            }
            Err(_) => {
                push_issue(
                    report,
                    Some(format!("parameter.{}", id.0)),
                    "unknown_parameter_path",
                    "Parameter path does not resolve to a scalar.",
                );
            }
        }
    }
    for lock in &recipe.locks {
        if !recipe.parameters.contains_key(lock) {
            push_issue(
                report,
                Some(format!("parameter.{}", lock.0)),
                "unknown_locked_parameter",
                "Locked parameter does not exist.",
            );
        }
    }
}

fn parameter_is_reflectable(
    recipe: &AssetRecipe,
    id: ParameterId,
    parameter: &ParameterDescriptor,
) -> bool {
    if parameter.id != id || !parameter_range_is_valid(parameter) {
        return false;
    }
    if !parameters::is_beginner_safe_parameter_path(&parameter.path) {
        return false;
    }
    let Ok(value) = get_scalar(recipe, &parameter.path) else {
        return false;
    };
    value.is_finite() && value >= parameter.minimum && value <= parameter.maximum
}

fn parameter_range_is_valid(parameter: &ParameterDescriptor) -> bool {
    parameter.minimum.is_finite()
        && parameter.maximum.is_finite()
        && parameter.step.is_finite()
        && parameter.step > 0.0
        && parameter.mutation_sigma.is_finite()
        && parameter.mutation_sigma >= 0.0
        && parameter.minimum <= parameter.maximum
}

fn validate_locks(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for instance in &recipe.instance_locks {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("lock.instance.{}", instance.0)),
                "unknown_locked_instance",
                "Locked instance does not exist.",
            );
        }
    }
    for instance in &recipe.subtree_locks {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("lock.subtree.{}", instance.0)),
                "unknown_locked_subtree",
                "Locked subtree root does not exist.",
            );
        }
    }
    for definition in &recipe.topology_locks {
        if !recipe.definitions.contains_key(definition) {
            push_issue(
                report,
                Some(format!("lock.topology.{}", definition.0)),
                "unknown_locked_topology",
                "Locked topology definition does not exist.",
            );
        }
    }
}

fn validate_constraints(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for constraint in &recipe.constraints {
        if let AssetConstraint::RequireInstance { instance } = constraint
            && !recipe.instances.contains_key(instance)
        {
            push_issue(
                report,
                Some(format!("constraint.instance.{}", instance.0)),
                "unknown_required_instance",
                "Constraint references an unknown instance.",
            );
        }
    }
}

fn validate_relationships(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (index, relationship) in recipe.relationships.iter().enumerate() {
        match relationship {
            AssetRelationshipPolicy::MayOverlap {
                first,
                second,
                pairing,
                reason,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                if reason.trim().is_empty() {
                    push_issue(
                        report,
                        Some(format!("relationship.{index}.reason")),
                        "empty_relationship_reason",
                        "MayOverlap relationships must explain why overlap is intentional.",
                    );
                }
            }
            AssetRelationshipPolicy::MustNotIntersect {
                first,
                second,
                pairing,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
            }
            AssetRelationshipPolicy::MustTouch {
                first,
                second,
                pairing,
                max_clearance,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_clearance")),
                    *max_clearance,
                );
            }
            AssetRelationshipPolicy::MustContain {
                container,
                contained,
                pairing,
            } => {
                validate_relationship_pair(recipe, report, index, container, contained);
                validate_relationship_pairing(recipe, report, index, pairing);
            }
            AssetRelationshipPolicy::MinimumClearance {
                first,
                second,
                pairing,
                clearance,
            } => {
                validate_relationship_pair(recipe, report, index, first, second);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.clearance")),
                    *clearance,
                );
            }
            AssetRelationshipPolicy::SocketAttached {
                parent,
                child,
                pairing,
                parent_socket,
                child_socket,
                max_origin_distance,
                max_axis_angle_degrees,
                max_clearance,
            } => {
                validate_relationship_pair(recipe, report, index, parent, child);
                validate_relationship_pairing(recipe, report, index, pairing);
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_origin_distance")),
                    *max_origin_distance,
                );
                validate_non_negative(
                    report,
                    Some(format!("relationship.{index}.max_axis_angle_degrees")),
                    *max_axis_angle_degrees,
                );
                if let Some(clearance) = max_clearance {
                    validate_non_negative(
                        report,
                        Some(format!("relationship.{index}.max_clearance")),
                        *clearance,
                    );
                }
                validate_relationship_socket(
                    recipe,
                    report,
                    index,
                    parent,
                    child,
                    *parent_socket,
                    *child_socket,
                );
            }
        }
    }
}

fn validate_relationship_pair(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    first: &AssetPartSelector,
    second: &AssetPartSelector,
) {
    if first == second {
        push_issue(
            report,
            Some(format!("relationship.{index}")),
            "self_relationship",
            "Relationship endpoints must be different instances.",
        );
    }
    validate_part_selector(recipe, report, format!("relationship.{index}.first"), first);
    validate_part_selector(
        recipe,
        report,
        format!("relationship.{index}.second"),
        second,
    );
}

fn validate_relationship_pairing(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    pairing: &RelationshipPairing,
) {
    let RelationshipPairing::Explicit(pairs) = pairing else {
        return;
    };
    if pairs.is_empty() {
        push_issue(
            report,
            Some(format!("relationship.{index}.pairing")),
            "empty_relationship_pairing",
            "Explicit relationship pairing must include at least one pair.",
        );
    }
    for (pair_index, (first, second)) in pairs.iter().enumerate() {
        if first == second {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}")),
                "self_relationship_pairing",
                "Explicit relationship pairing endpoints must be different instances.",
            );
        }
        if !recipe.instances.contains_key(first) {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}.first")),
                "unknown_relationship_pairing_instance",
                "Explicit relationship pairing references an unknown first instance.",
            );
        }
        if !recipe.instances.contains_key(second) {
            push_issue(
                report,
                Some(format!("relationship.{index}.pairing.{pair_index}.second")),
                "unknown_relationship_pairing_instance",
                "Explicit relationship pairing references an unknown second instance.",
            );
        }
    }
}

fn validate_relationship_socket(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    index: usize,
    parent: &AssetPartSelector,
    child: &AssetPartSelector,
    parent_socket: SocketId,
    child_socket: SocketId,
) {
    let parent_definitions = selector_definitions(recipe, parent);
    let child_definitions = selector_definitions(recipe, child);
    if parent_definitions
        .iter()
        .any(|definition| !definition.sockets.contains_key(&parent_socket))
    {
        push_issue(
            report,
            Some(format!("relationship.{index}.parent_socket")),
            "unknown_relationship_parent_socket",
            "SocketAttached relationship references a missing parent socket.",
        );
    }
    if child_definitions
        .iter()
        .any(|definition| !definition.sockets.contains_key(&child_socket))
    {
        push_issue(
            report,
            Some(format!("relationship.{index}.child_socket")),
            "unknown_relationship_child_socket",
            "SocketAttached relationship references a missing child socket.",
        );
    }
}

fn validate_part_selector(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    selector: &AssetPartSelector,
) {
    match selector {
        AssetPartSelector::SpecificInstance { instance } => {
            if !recipe.instances.contains_key(instance) {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_instance",
                    "Relationship selector references an unknown instance.",
                );
            }
        }
        AssetPartSelector::GeneratedByOperation { operation } => {
            let exists = recipe
                .definitions
                .values()
                .flat_map(|definition| &definition.geometry.operations)
                .any(|candidate| candidate.operation_id() == *operation);
            if !exists {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_operation",
                    "Relationship selector references an unknown generator operation.",
                );
            }
        }
        AssetPartSelector::PrototypeAndGeneratedOccurrences { prototype } => {
            if !recipe.instances.contains_key(prototype) {
                push_issue(
                    report,
                    Some(subject),
                    "unknown_relationship_instance",
                    "Relationship selector references an unknown prototype instance.",
                );
            }
        }
        AssetPartSelector::PartTag { tag } => {
            validate_selector_tag(recipe, report, subject, tag, "part tag");
        }
        AssetPartSelector::DefinitionRole { role } => {
            validate_selector_tag(recipe, report, subject, role, "definition role");
        }
    }
}

fn validate_selector_tag(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    tag: &str,
    label: &'static str,
) {
    if tag.trim().is_empty() {
        push_issue(
            report,
            Some(subject),
            "empty_relationship_selector",
            format!("Relationship selector {label} cannot be empty."),
        );
        return;
    }
    if !recipe
        .definitions
        .values()
        .any(|definition| definition.tags.contains(tag))
    {
        push_issue(
            report,
            Some(subject),
            "unknown_relationship_selector",
            format!("Relationship selector references an unknown {label}."),
        );
    }
}

fn selector_definitions<'a>(
    recipe: &'a AssetRecipe,
    selector: &AssetPartSelector,
) -> Vec<&'a PartDefinition> {
    match selector {
        AssetPartSelector::SpecificInstance { instance }
        | AssetPartSelector::PrototypeAndGeneratedOccurrences {
            prototype: instance,
        } => recipe
            .instances
            .get(instance)
            .and_then(|instance| recipe.definitions.get(&instance.definition))
            .into_iter()
            .collect(),
        AssetPartSelector::GeneratedByOperation { operation } => recipe
            .definitions
            .values()
            .filter(|definition| {
                definition
                    .geometry
                    .operations
                    .iter()
                    .any(|candidate| candidate.operation_id() == *operation)
            })
            .collect(),
        AssetPartSelector::PartTag { tag } | AssetPartSelector::DefinitionRole { role: tag } => {
            recipe
                .definitions
                .values()
                .filter(|definition| definition.tags.contains(tag))
                .collect()
        }
    }
}
