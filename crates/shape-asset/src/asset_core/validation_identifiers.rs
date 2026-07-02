
fn validate_optional_instance(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    instance: Option<PartInstanceId>,
    code: &'static str,
) {
    if let Some(instance) = instance
        && !recipe.instances.contains_key(&instance)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown instance.",
        );
    }
}

fn validate_optional_definition(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    definition: Option<PartDefinitionId>,
    code: &'static str,
) {
    if let Some(definition) = definition
        && !recipe.definitions.contains_key(&definition)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown definition.",
        );
    }
}

fn validate_optional_export_profile(
    recipe: &AssetRecipe,
    report: &mut AssetValidationReport,
    subject: String,
    export_profile: Option<ExportProfileId>,
    code: &'static str,
) {
    if let Some(export_profile) = export_profile
        && !recipe
            .semantic
            .export_profiles
            .contains_key(&export_profile)
    {
        push_issue(
            report,
            Some(subject),
            code,
            "Semantic shell references an unknown export profile.",
        );
    }
}

fn validate_review_state(report: &mut AssetValidationReport, review: &ReviewState) {
    if matches!(review.tier, ReviewTier::Reviewed | ReviewTier::Published) {
        push_issue(
            report,
            Some("semantic.review_state.tier".to_owned()),
            "unsupported_semantic_review_tier",
            "Phase A semantic shells cannot mark assets reviewed or published.",
        );
    }
    if !review.human_review_required {
        push_issue(
            report,
            Some("semantic.review_state.human_review_required".to_owned()),
            "semantic_human_review_required_false",
            "Phase A semantic shells must keep human review required.",
        );
    }
    if review.publish_allowed {
        push_issue(
            report,
            Some("semantic.review_state.publish_allowed".to_owned()),
            "semantic_publish_allowed",
            "Phase A semantic shells must not allow publishing.",
        );
    }
    if review.public_catalog_visible {
        push_issue(
            report,
            Some("semantic.review_state.public_catalog_visible".to_owned()),
            "semantic_public_catalog_visible",
            "Phase A semantic shells must not make assets public catalog visible.",
        );
    }
}

fn validate_export_includes(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    includes: &ExportIncludes,
) {
    for (enabled, suffix) in [
        (includes.includes_uvs, "includes_uvs"),
        (includes.includes_textures, "includes_textures"),
        (includes.includes_material_looks, "includes_material_looks"),
        (includes.includes_collision, "includes_collision"),
        (
            includes.includes_gameplay_metadata,
            "includes_gameplay_metadata",
        ),
        (includes.includes_rig, "includes_rig"),
        (includes.includes_skinning, "includes_skinning"),
        (includes.includes_animation, "includes_animation"),
        (
            includes.includes_terrain_collision,
            "includes_terrain_collision",
        ),
        (includes.includes_godot_scene, "includes_godot_scene"),
    ] {
        if enabled {
            push_issue(
                report,
                append_subject(subject.clone(), suffix),
                "unsupported_semantic_export_include",
                "Phase A semantic shells cannot claim unsupported export includes.",
            );
        }
    }
    if includes.game_ready {
        push_issue(
            report,
            append_subject(subject.clone(), "game_ready"),
            "semantic_game_ready_claim",
            "Phase A semantic shells must keep game_ready false.",
        );
    }
    if !includes.human_review_required {
        push_issue(
            report,
            append_subject(subject, "human_review_required"),
            "semantic_export_review_required_false",
            "Phase A semantic shells must keep export review required.",
        );
    }
}

fn validate_next_ids(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    validate_counter(
        report,
        "part_definition",
        recipe.next_ids.part_definition,
        recipe.definitions.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "part_instance",
        recipe.next_ids.part_instance,
        recipe.instances.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "parameter",
        recipe.next_ids.parameter,
        recipe.parameters.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "operation",
        recipe.next_ids.operation,
        max_operation_id(recipe),
    );
    validate_counter(
        report,
        "region",
        recipe.next_ids.region,
        max_region_id(recipe),
    );
    validate_counter(
        report,
        "boundary_loop",
        recipe.next_ids.boundary_loop,
        max_boundary_loop_id(recipe),
    );
    validate_counter(
        report,
        "socket",
        recipe.next_ids.socket,
        max_socket_id(recipe),
    );
    validate_counter(
        report,
        "relationship",
        recipe.next_ids.relationship,
        recipe.semantic.relationships.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "pattern",
        recipe.next_ids.pattern,
        recipe.semantic.patterns.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "surface_slot",
        recipe.next_ids.surface_slot,
        recipe.semantic.surface_slots.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "material_slot",
        recipe.next_ids.material_slot,
        recipe.semantic.material_slots.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "collision_body",
        recipe.next_ids.collision_body,
        recipe.semantic.collision_bodies.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "motion_channel",
        recipe.next_ids.motion_channel,
        recipe.semantic.motion_channels.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "terrain_patch",
        recipe.next_ids.terrain_patch,
        recipe.semantic.terrain_patches.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "export_profile",
        recipe.next_ids.export_profile,
        recipe.semantic.export_profiles.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "authoring_op",
        recipe.next_ids.authoring_op,
        recipe.semantic.authoring_ops.keys().map(|id| id.0).max(),
    );
    validate_counter(
        report,
        "validation_report",
        recipe.next_ids.validation_report,
        recipe
            .semantic
            .validation_reports
            .keys()
            .map(|id| id.0)
            .max(),
    );
}

fn validate_counter(
    report: &mut AssetValidationReport,
    name: &'static str,
    next: u64,
    max_existing: Option<u64>,
) {
    if let Some(max_existing) = max_existing
        && next <= max_existing
    {
        push_issue(
            report,
            Some(format!("next_ids.{name}")),
            "next_id_not_fresh",
            "Next ID counter would reallocate an existing semantic ID.",
        );
    }
}

fn max_operation_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .map(|operation| operation.operation_id().0)
        .max()
}

fn max_region_id(recipe: &AssetRecipe) -> Option<u64> {
    let declared = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.regions.keys().map(|id| id.0));
    let generated = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::generated_region_ids)
        .map(|id| id.0);
    declared.chain(generated).max()
}

fn max_boundary_loop_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .map(|id| id.0)
        .max()
}

fn max_socket_id(recipe: &AssetRecipe) -> Option<u64> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.sockets.keys().map(|id| id.0))
        .max()
}

fn operation_by_id(recipe: &AssetRecipe, operation: OperationId) -> Option<&ModelingOperationSpec> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .find(|candidate| candidate.operation_id() == operation)
}

fn operation_is_array(operation: &ModelingOperationSpec) -> bool {
    matches!(
        operation,
        ModelingOperationSpec::LinearArray { .. } | ModelingOperationSpec::RadialArray { .. }
    )
}

fn operation_is_cut(operation: &ModelingOperationSpec) -> bool {
    matches!(
        operation,
        ModelingOperationSpec::RecessedPanelCut { .. }
            | ModelingOperationSpec::RectangularThroughCut { .. }
            | ModelingOperationSpec::CircularThroughCut { .. }
    )
}

fn cut_group_role_accepts_operation(
    role: &CutGroupRole,
    operation: &ModelingOperationSpec,
) -> bool {
    match role {
        CutGroupRole::MountHoles => {
            matches!(operation, ModelingOperationSpec::CircularThroughCut { .. })
        }
        CutGroupRole::Vents => {
            matches!(
                operation,
                ModelingOperationSpec::RectangularThroughCut { .. }
            )
        }
        CutGroupRole::Recesses => {
            matches!(operation, ModelingOperationSpec::RecessedPanelCut { .. })
        }
        CutGroupRole::Custom(_) => true,
    }
}

fn validate_semantic_cut_host_constraints(
    definition: &PartDefinition,
    report: &mut AssetValidationReport,
) {
    let mut rounded_box_cut_faces = BTreeSet::new();
    for operation in &definition.geometry.operations {
        let Some(face) = operation_cut_face(operation) else {
            continue;
        };
        let operation_id = operation.operation_id();
        match &definition.geometry.source {
            GeometrySource::Plate { .. } => {
                if !matches!(face, PlanarCutFace::PositiveY | PlanarCutFace::NegativeY) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "cut.face"),
                        "unsupported_plate_cut_face",
                        "Plate semantic cuts currently target only local +/-Y planar faces.",
                    );
                }
            }
            GeometrySource::RoundedBox { .. } => {
                rounded_box_cut_faces.insert(face);
            }
            _ => {
                push_issue(
                    report,
                    operation_subject(definition.id, operation_id, "cut.host"),
                    "unsupported_semantic_cut_host",
                    "Semantic cuts currently target only Plate or RoundedBox geometry sources.",
                );
            }
        }
    }

    if rounded_box_cut_faces.len() > 1 {
        push_issue(
            report,
            definition_subject(definition.id, "rounded_box.semantic_cuts"),
            "unsupported_rounded_box_cut_face_set",
            "RoundedBox semantic cuts currently support one selected primary face per definition.",
        );
    }
}

fn validate_geometry_source(
    definition: PartDefinitionId,
    source: &GeometrySource,
    report: &mut AssetValidationReport,
) {
    match source {
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        } => {
            validate_positive_array(
                report,
                Some(format!(
                    "definition.{}.rounded_box.half_extents",
                    definition.0
                )),
                half_extents,
            );
            validate_non_negative(
                report,
                Some(format!("definition.{}.rounded_box.radius", definition.0)),
                *radius,
            );
        }
        GeometrySource::Cylinder {
            radius,
            height,
            radial_segments,
        } => {
            validate_positive(
                report,
                definition_subject(definition, "cylinder.radius"),
                *radius,
            );
            validate_positive(
                report,
                definition_subject(definition, "cylinder.height"),
                *height,
            );
            validate_count(
                report,
                definition_subject(definition, "cylinder.radial_segments"),
                *radial_segments,
                3,
            );
        }
        GeometrySource::Frustum {
            bottom_radius,
            top_radius,
            height,
            radial_segments,
        } => {
            validate_non_negative(
                report,
                definition_subject(definition, "frustum.bottom_radius"),
                *bottom_radius,
            );
            validate_non_negative(
                report,
                definition_subject(definition, "frustum.top_radius"),
                *top_radius,
            );
            validate_positive(
                report,
                definition_subject(definition, "frustum.height"),
                *height,
            );
            validate_count(
                report,
                definition_subject(definition, "frustum.radial_segments"),
                *radial_segments,
                3,
            );
        }
        GeometrySource::Plate { size, thickness } => {
            validate_positive_array(report, definition_subject(definition, "plate.size"), size);
            validate_positive(
                report,
                definition_subject(definition, "plate.thickness"),
                *thickness,
            );
        }
        GeometrySource::Sweep { profile, path } => {
            if profile.len() < 2 || path.len() < 2 {
                push_issue(
                    report,
                    definition_subject(definition, "sweep"),
                    "insufficient_sweep_data",
                    "Sweep requires at least two profile points and two path frames.",
                );
            }
            for point in profile {
                if !array_is_finite(point) {
                    push_issue(
                        report,
                        definition_subject(definition, "sweep.profile"),
                        "non_finite",
                        "Sweep profile points must be finite.",
                    );
                }
            }
            for frame in path {
                validate_frame(report, definition_subject(definition, "sweep.path"), frame);
            }
        }
        GeometrySource::Lathe { profile, segments } => {
            if profile.len() < 2 {
                push_issue(
                    report,
                    definition_subject(definition, "lathe.profile"),
                    "insufficient_lathe_profile",
                    "Lathe requires at least two profile points.",
                );
            }
            for point in profile {
                if !array_is_finite(point) {
                    push_issue(
                        report,
                        definition_subject(definition, "lathe.profile"),
                        "non_finite",
                        "Lathe profile points must be finite.",
                    );
                }
            }
            validate_count(
                report,
                definition_subject(definition, "lathe.segments"),
                *segments,
                3,
            );
        }
        GeometrySource::LiteralMesh { positions, faces } => {
            if positions.is_empty() || faces.is_empty() {
                push_issue(
                    report,
                    definition_subject(definition, "literal_mesh"),
                    "empty_literal_mesh",
                    "Literal mesh must contain positions and faces.",
                );
            }
            for position in positions {
                if !array_is_finite(position) {
                    push_issue(
                        report,
                        definition_subject(definition, "literal_mesh.positions"),
                        "non_finite_literal_position",
                        "Literal mesh positions must be finite.",
                    );
                }
            }
            for face in faces {
                if face.len() < 3 {
                    push_issue(
                        report,
                        definition_subject(definition, "literal_mesh.faces"),
                        "invalid_literal_face",
                        "Literal mesh faces must contain at least three vertices.",
                    );
                }
                for index in face {
                    if (*index as usize) >= positions.len() {
                        push_issue(
                            report,
                            definition_subject(definition, "literal_mesh.faces"),
                            "literal_face_index_out_of_bounds",
                            "Literal mesh face indices must reference positions.",
                        );
                    }
                }
            }
        }
        GeometrySource::ReservedBooleanResult { .. } => {}
    }
}
