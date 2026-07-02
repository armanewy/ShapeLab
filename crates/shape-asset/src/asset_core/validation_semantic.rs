
fn validate_variation_metadata(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for instance in &recipe.variation.optional_instances {
        if !recipe.instances.contains_key(instance) {
            push_issue(
                report,
                Some(format!("variation.optional_instance.{}", instance.0)),
                "unknown_optional_instance",
                "Optional instance hint references an unknown instance.",
            );
        }
    }

    for (group, hint) in &recipe.variation.replacement_groups {
        if group.trim().is_empty() {
            push_issue(
                report,
                Some("variation.replacement_group".to_owned()),
                "empty_replacement_group",
                "Replacement group names cannot be empty.",
            );
        }
        if hint.definitions.is_empty() {
            push_issue(
                report,
                Some(format!("variation.replacement_group.{group}")),
                "empty_replacement_group_definitions",
                "Replacement groups must reference at least one definition.",
            );
        }
        for definition in &hint.definitions {
            if !recipe.definitions.contains_key(definition) {
                push_issue(
                    report,
                    Some(format!(
                        "variation.replacement_group.{group}.definition.{}",
                        definition.0
                    )),
                    "unknown_replacement_definition",
                    "Replacement group references an unknown definition.",
                );
            }
        }
    }

    for (operation, range) in &recipe.variation.count_ranges {
        if range.minimum > range.maximum {
            push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "invalid_count_range",
                "Count range minimum cannot exceed maximum.",
            );
        }
        if range.minimum == 0 {
            push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "count_range_too_small",
                "Array count ranges must start at one or greater.",
            );
        }
        match operation_by_id(recipe, *operation) {
            Some(operation_spec) if operation_is_array(operation_spec) => {}
            Some(_) => push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "invalid_count_range_operation",
                "Count range hint must target an array operation.",
            ),
            None => push_issue(
                report,
                Some(format!("variation.count_range.{}", operation.0)),
                "unknown_count_range_operation",
                "Count range hint references an unknown operation.",
            ),
        }
    }

    for (parameter, range) in &recipe.variation.parameter_range_overrides {
        if !recipe.parameters.contains_key(parameter) {
            push_issue(
                report,
                Some(format!("variation.parameter_range.{}", parameter.0)),
                "unknown_parameter_range_override",
                "Parameter range override references an unknown parameter.",
            );
        }
        validate_finite(
            report,
            Some(format!("variation.parameter_range.{}.minimum", parameter.0)),
            range.minimum,
        );
        validate_finite(
            report,
            Some(format!("variation.parameter_range.{}.maximum", parameter.0)),
            range.maximum,
        );
        if range.minimum > range.maximum {
            push_issue(
                report,
                Some(format!("variation.parameter_range.{}", parameter.0)),
                "invalid_parameter_range_override",
                "Parameter range override minimum cannot exceed maximum.",
            );
        }
        if let Some(step) = range.step {
            validate_positive(
                report,
                Some(format!("variation.parameter_range.{}.step", parameter.0)),
                step,
            );
        }
        if let Some(mutation_sigma) = range.mutation_sigma {
            validate_non_negative(
                report,
                Some(format!(
                    "variation.parameter_range.{}.mutation_sigma",
                    parameter.0
                )),
                mutation_sigma,
            );
        }
    }

    for (group, hint) in &recipe.variation.semantic_cut_groups {
        if group.trim().is_empty() {
            push_issue(
                report,
                Some("variation.semantic_cut_group".to_owned()),
                "empty_semantic_cut_group",
                "Semantic cut group IDs cannot be empty.",
            );
        }
        if hint.label.trim().is_empty() {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.label")),
                "empty_semantic_cut_group_label",
                "Semantic cut groups must have a non-empty label.",
            );
        }
        if hint.operations.is_empty() {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.operations")),
                "empty_semantic_cut_group_operations",
                "Semantic cut groups must reference at least one cut operation.",
            );
        }
        let Some(definition) = recipe.definitions.get(&hint.definition) else {
            push_issue(
                report,
                Some(format!("variation.semantic_cut_group.{group}.definition")),
                "unknown_semantic_cut_group_definition",
                "Semantic cut group references an unknown definition.",
            );
            continue;
        };
        let mut seen_operations = BTreeSet::new();
        for operation in &hint.operations {
            if !seen_operations.insert(*operation) {
                push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "duplicate_semantic_cut_group_operation",
                    "Semantic cut groups cannot list the same operation more than once.",
                );
            }
            match definition
                .geometry
                .operations
                .iter()
                .find(|candidate| candidate.operation_id() == *operation)
            {
                Some(operation_spec) if operation_is_cut(operation_spec) => {
                    if !cut_group_role_accepts_operation(&hint.role, operation_spec) {
                        push_issue(
                            report,
                            Some(format!(
                                "variation.semantic_cut_group.{group}.operation.{}",
                                operation.0
                            )),
                            "semantic_cut_group_role_mismatch",
                            "Semantic cut group role must match each member cut family.",
                        );
                    }
                }
                Some(_) => push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "invalid_semantic_cut_group_operation",
                    "Semantic cut groups must reference cut operations.",
                ),
                None => push_issue(
                    report,
                    Some(format!(
                        "variation.semantic_cut_group.{group}.operation.{}",
                        operation.0
                    )),
                    "unknown_semantic_cut_group_operation",
                    "Semantic cut group references an unknown operation on its definition.",
                ),
            }
        }
        if let Some(range) = hint.count_range {
            if range.minimum > range.maximum {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "invalid_semantic_cut_group_count_range",
                    "Semantic cut group count range minimum cannot exceed maximum.",
                );
            }
            if range.minimum == 0 {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "semantic_cut_group_count_range_too_small",
                    "Semantic cut group count ranges must start at one or greater.",
                );
            }
            let count = hint.operations.len() as u32;
            if count < range.minimum || count > range.maximum {
                push_issue(
                    report,
                    Some(format!("variation.semantic_cut_group.{group}.count_range")),
                    "semantic_cut_group_count_out_of_range",
                    "Semantic cut group member count must fit the authored count range.",
                );
            }
        }
    }
}

fn validate_semantic_shells(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for (id, relationship) in &recipe.semantic.relationships {
        validate_shell_id(
            report,
            format!("semantic.relationships.{}", id.0),
            id.0,
            relationship.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.relationships.{}.parent", id.0),
            relationship.parent,
            "unknown_semantic_relationship_parent",
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.relationships.{}.child", id.0),
            relationship.child,
            "unknown_semantic_relationship_child",
        );
        validate_optional_export_profile(
            recipe,
            report,
            format!("semantic.relationships.{}.export_profile", id.0),
            relationship.export_profile,
            "unknown_semantic_relationship_export_profile",
        );
        validate_relationship_contract_policy(report, *id, relationship);
    }
    validate_semantic_relationship_cycles(recipe, report);

    for (id, pattern) in &recipe.semantic.patterns {
        validate_shell_id(
            report,
            format!("semantic.patterns.{}", id.0),
            id.0,
            pattern.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.patterns.{}.source_instance", id.0),
            pattern.source_instance,
            "unknown_semantic_pattern_source",
        );
        if let Some(count) = pattern.count
            && !(1..=10_000).contains(&count)
        {
            push_issue(
                report,
                Some(format!("semantic.patterns.{}.count", id.0)),
                "invalid_semantic_pattern_count",
                "Pattern count must be between 1 and 10000.",
            );
        }
        validate_pattern_contract_policy(report, *id, pattern);
    }

    for (id, slot) in &recipe.semantic.surface_slots {
        validate_shell_id(
            report,
            format!("semantic.surface_slots.{}", id.0),
            id.0,
            slot.id.0,
        );
        validate_optional_definition(
            recipe,
            report,
            format!("semantic.surface_slots.{}.owner_definition", id.0),
            slot.owner_definition,
            "unknown_semantic_surface_owner",
        );
    }

    for (id, slot) in &recipe.semantic.material_slots {
        validate_shell_id(
            report,
            format!("semantic.material_slots.{}", id.0),
            id.0,
            slot.id.0,
        );
        if let Some(surface_slot) = slot.surface_slot
            && !recipe.semantic.surface_slots.contains_key(&surface_slot)
        {
            push_issue(
                report,
                Some(format!("semantic.material_slots.{}.surface_slot", id.0)),
                "unknown_semantic_material_surface_slot",
                "Material slot references an unknown surface slot.",
            );
        }
    }

    for (id, body) in &recipe.semantic.collision_bodies {
        validate_shell_id(
            report,
            format!("semantic.collision_bodies.{}", id.0),
            id.0,
            body.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.collision_bodies.{}.target_instance", id.0),
            body.target_instance,
            "unknown_semantic_collision_target",
        );
    }

    for (id, channel) in &recipe.semantic.motion_channels {
        validate_shell_id(
            report,
            format!("semantic.motion_channels.{}", id.0),
            id.0,
            channel.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.motion_channels.{}.target_instance", id.0),
            channel.target_instance,
            "unknown_semantic_motion_target",
        );
    }

    for (id, patch) in &recipe.semantic.terrain_patches {
        validate_shell_id(
            report,
            format!("semantic.terrain_patches.{}", id.0),
            id.0,
            patch.id.0,
        );
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.terrain_patches.{}.root_instance", id.0),
            patch.root_instance,
            "unknown_semantic_terrain_root",
        );
    }

    for (id, profile) in &recipe.semantic.export_profiles {
        validate_shell_id(
            report,
            format!("semantic.export_profiles.{}", id.0),
            id.0,
            profile.id.0,
        );
        validate_export_includes(
            report,
            Some(format!("semantic.export_profiles.{}.includes", id.0)),
            &profile.includes,
        );
    }

    for (id, op) in &recipe.semantic.authoring_ops {
        validate_shell_id(
            report,
            format!("semantic.authoring_ops.{}", id.0),
            id.0,
            op.id.0,
        );
        if let Some(parameter) = op.target_parameter
            && !recipe.parameters.contains_key(&parameter)
        {
            push_issue(
                report,
                Some(format!("semantic.authoring_ops.{}.target_parameter", id.0)),
                "unknown_semantic_authoring_parameter",
                "Authoring op references an unknown parameter.",
            );
        }
        validate_optional_instance(
            recipe,
            report,
            format!("semantic.authoring_ops.{}.target_instance", id.0),
            op.target_instance,
            "unknown_semantic_authoring_instance",
        );
    }

    for (id, validation) in &recipe.semantic.validation_reports {
        validate_shell_id(
            report,
            format!("semantic.validation_reports.{}", id.0),
            id.0,
            validation.id.0,
        );
        validate_optional_export_profile(
            recipe,
            report,
            format!("semantic.validation_reports.{}.export_profile", id.0),
            validation.export_profile,
            "unknown_semantic_validation_export_profile",
        );
    }

    validate_review_state(report, &recipe.semantic.review_state);
    validate_export_includes(
        report,
        Some("semantic.export_includes".to_owned()),
        &recipe.semantic.export_includes,
    );
}

fn validate_relationship_contract_policy(
    report: &mut AssetValidationReport,
    id: RelationshipId,
    relationship: &RelationshipContract,
) {
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.parent_node_ref", id.0)),
        relationship.parent_node_ref.as_deref(),
        "invalid_semantic_relationship_parent_node_ref",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.child_node_ref", id.0)),
        relationship.child_node_ref.as_deref(),
        "invalid_semantic_relationship_child_node_ref",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.parent_anchor_id", id.0)),
        relationship.parent_anchor_id.as_deref(),
        "invalid_semantic_relationship_parent_anchor",
    );
    validate_optional_semantic_identifier(
        report,
        Some(format!("semantic.relationships.{}.child_anchor_id", id.0)),
        relationship.child_anchor_id.as_deref(),
        "invalid_semantic_relationship_child_anchor",
    );

    match &relationship.placement_policy.position_rule {
        PositionRule::FixedOffsetFromEdge { edge, offset } => {
            if edge.trim().is_empty() {
                push_issue(
                    report,
                    Some(format!("semantic.relationships.{}.placement.edge", id.0)),
                    "empty_relationship_edge",
                    "Fixed edge placement must name an edge.",
                );
            }
            validate_finite_array(
                report,
                Some(format!("semantic.relationships.{}.placement.offset", id.0)),
                offset,
            );
        }
        PositionRule::ProportionalUv { u, v } => {
            validate_range(
                report,
                Some(format!("semantic.relationships.{}.placement.u", id.0)),
                *u,
                0.0,
                1.0,
            );
            validate_range(
                report,
                Some(format!("semantic.relationships.{}.placement.v", id.0)),
                *v,
                0.0,
                1.0,
            );
        }
        PositionRule::CenteredInZone { zone } => {
            if zone.trim().is_empty() {
                push_issue(
                    report,
                    Some(format!("semantic.relationships.{}.placement.zone", id.0)),
                    "empty_relationship_zone",
                    "Centered placement must name a zone.",
                );
            }
        }
        PositionRule::PreserveCurrentOnDetach => {}
    }

    if let OrientationPolicy::AlignToSurfaceNormal { max_angle_degrees } =
        relationship.orientation_policy
    {
        validate_range(
            report,
            Some(format!(
                "semantic.relationships.{}.orientation.max_angle_degrees",
                id.0
            )),
            max_angle_degrees,
            0.0,
            180.0,
        );
    }

    if let ScalePolicy::ClampToRange { minimum, maximum } = relationship.scale_policy {
        validate_positive(
            report,
            Some(format!("semantic.relationships.{}.scale.minimum", id.0)),
            minimum,
        );
        validate_positive(
            report,
            Some(format!("semantic.relationships.{}.scale.maximum", id.0)),
            maximum,
        );
        if minimum > maximum {
            push_issue(
                report,
                Some(format!("semantic.relationships.{}.scale", id.0)),
                "invalid_relationship_scale_range",
                "Scale policy minimum must be less than or equal to maximum.",
            );
        }
    }

    match relationship.contact_policy {
        ContactPolicy::SurfaceContact { clearance }
        | ContactPolicy::IntentionalGap { clearance } => validate_non_negative(
            report,
            Some(format!("semantic.relationships.{}.contact.clearance", id.0)),
            clearance,
        ),
        ContactPolicy::NotChecked | ContactPolicy::IntentionalOverlap => {}
    }
}

fn validate_semantic_relationship_cycles(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    let mut graph: BTreeMap<PartInstanceId, Vec<PartInstanceId>> = BTreeMap::new();
    for relationship in recipe.semantic.relationships.values() {
        if let (Some(parent), Some(child)) = (relationship.parent, relationship.child) {
            graph.entry(parent).or_default().push(child);
        }
    }

    for (parent, children) in &graph {
        for child in children {
            let mut visited = BTreeSet::new();
            if *parent == *child || relationship_path_exists(&graph, *child, *parent, &mut visited)
            {
                push_issue(
                    report,
                    Some("semantic.relationships".to_owned()),
                    "semantic_relationship_cycle",
                    "Relationship contracts must not form cycles.",
                );
                return;
            }
        }
    }
}

fn relationship_path_exists(
    graph: &BTreeMap<PartInstanceId, Vec<PartInstanceId>>,
    current: PartInstanceId,
    target: PartInstanceId,
    visited: &mut BTreeSet<PartInstanceId>,
) -> bool {
    if !visited.insert(current) {
        return false;
    }
    graph.get(&current).is_some_and(|children| {
        children.iter().any(|child| {
            *child == target || relationship_path_exists(graph, *child, target, visited)
        })
    })
}

fn validate_pattern_contract_policy(
    report: &mut AssetValidationReport,
    id: PatternId,
    pattern: &PatternContract,
) {
    match pattern.count_policy {
        PatternCountPolicy::Unspecified => {}
        PatternCountPolicy::Exact(count) => validate_pattern_count(report, id, count),
        PatternCountPolicy::Range { minimum, maximum } => {
            validate_pattern_count(report, id, minimum);
            validate_pattern_count(report, id, maximum);
            if minimum > maximum {
                push_issue(
                    report,
                    Some(format!("semantic.patterns.{}.count_policy", id.0)),
                    "invalid_semantic_pattern_count_range",
                    "Pattern count range minimum must be less than or equal to maximum.",
                );
            }
        }
    }

    if let Some(density) = pattern.density_policy {
        match density {
            PatternDensityPolicy::Exact(value) => validate_non_negative(
                report,
                Some(format!("semantic.patterns.{}.density", id.0)),
                value,
            ),
            PatternDensityPolicy::Range { minimum, maximum } => {
                validate_non_negative(
                    report,
                    Some(format!("semantic.patterns.{}.density.minimum", id.0)),
                    minimum,
                );
                validate_non_negative(
                    report,
                    Some(format!("semantic.patterns.{}.density.maximum", id.0)),
                    maximum,
                );
                if minimum > maximum {
                    push_issue(
                        report,
                        Some(format!("semantic.patterns.{}.density", id.0)),
                        "invalid_semantic_pattern_density_range",
                        "Pattern density range minimum must be less than or equal to maximum.",
                    );
                }
            }
        }
    }

    if let Some(spacing) = pattern.spacing {
        validate_non_negative(
            report,
            Some(format!("semantic.patterns.{}.spacing", id.0)),
            spacing,
        );
    }
}

fn validate_pattern_count(report: &mut AssetValidationReport, id: PatternId, count: u32) {
    if !(1..=10_000).contains(&count) {
        push_issue(
            report,
            Some(format!("semantic.patterns.{}.count_policy", id.0)),
            "invalid_semantic_pattern_count",
            "Pattern count must be between 1 and 10000.",
        );
    }
}

fn validate_optional_semantic_identifier(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: Option<&str>,
    code: &'static str,
) {
    let Some(value) = value else {
        return;
    };
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        push_issue(
            report,
            subject,
            code,
            "Semantic relationship references must use stable lowercase identifiers.",
        );
    }
}

fn validate_shell_id(
    report: &mut AssetValidationReport,
    subject: String,
    map_id: u64,
    payload_id: u64,
) {
    if payload_id == 0 {
        push_issue(
            report,
            Some(subject.clone()),
            "zero_semantic_shell_id",
            "Semantic shell IDs must be non-zero.",
        );
    }
    if map_id != payload_id {
        push_issue(
            report,
            Some(subject),
            "semantic_shell_id_mismatch",
            "Semantic shell map key and payload ID differ.",
        );
    }
}
