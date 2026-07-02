
fn validate_operations(definition: &PartDefinition, report: &mut AssetValidationReport) {
    let mut seen = BTreeSet::new();
    let mut previous_phase: Option<(OperationId, OperationPhase)> = None;
    for operation in &definition.geometry.operations {
        let operation_id = operation.operation_id();
        let phase = operation.phase();
        if let Some((previous_operation, previous_phase)) = previous_phase
            && previous_phase > phase
        {
            push_issue(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                "invalid_operation_phase_order",
                format!(
                    "Operation phase {:?} cannot follow operation {} phase {:?}.",
                    phase, previous_operation.0, previous_phase
                ),
            );
        }
        previous_phase = Some((operation_id, phase));
        if !seen.insert(operation_id) {
            push_issue(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                "duplicate_operation_id",
                "Operation IDs must be unique within a definition.",
            );
        }
        match operation {
            ModelingOperationSpec::TransformGeometry { transform, .. } => validate_transform(
                report,
                Some(format!(
                    "definition.{}.operation.{}",
                    definition.id.0, operation_id.0
                )),
                transform,
            ),
            ModelingOperationSpec::SetBevelProfile {
                radius, segments, ..
            } => {
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "bevel.radius"),
                    *radius,
                );
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "bevel.segments"),
                    *segments,
                    1,
                );
            }
            ModelingOperationSpec::AddPanel {
                region,
                inset,
                depth,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "panel.inset"),
                    *inset,
                );
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "panel.depth"),
                    *depth,
                );
            }
            ModelingOperationSpec::AddTrim {
                region,
                width,
                height,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "trim.width"),
                    *width,
                );
                validate_non_negative(
                    report,
                    operation_subject(definition.id, operation_id, "trim.height"),
                    *height,
                );
            }
            ModelingOperationSpec::RecessedPanelCut {
                region,
                center,
                size,
                depth,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                floor_loop,
                outer_region,
                rim_region,
                wall_region,
                floor_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    *outer_region,
                    &[*rim_region, *wall_region, *floor_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    *entry_loop,
                    *floor_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    center,
                    report,
                );
                validate_cut_size(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    size,
                    report,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "recessed_panel_cut.depth"),
                    *depth,
                );
                validate_non_negative(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "recessed_panel_cut.corner_radius",
                    ),
                    *corner_radius,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "recessed_panel_cut.rim_width"),
                    *rim_width,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "recessed_panel_cut.corner_segments",
                    ),
                    *corner_segments,
                    1,
                );
                validate_rect_cut_corner_radius(
                    definition.id,
                    operation_id,
                    "recessed_panel_cut",
                    size,
                    *corner_radius,
                    report,
                );
            }
            ModelingOperationSpec::RectangularThroughCut {
                region,
                center,
                size,
                corner_radius,
                rim_width,
                corner_segments,
                entry_loop,
                exit_loop,
                outer_region,
                rim_region,
                wall_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    *outer_region,
                    &[*rim_region, *wall_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    *entry_loop,
                    *exit_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    center,
                    report,
                );
                validate_cut_size(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    size,
                    report,
                );
                validate_non_negative(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.corner_radius",
                    ),
                    *corner_radius,
                );
                validate_positive(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.rim_width",
                    ),
                    *rim_width,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "rectangular_through_cut.corner_segments",
                    ),
                    *corner_segments,
                    1,
                );
                validate_rect_cut_corner_radius(
                    definition.id,
                    operation_id,
                    "rectangular_through_cut",
                    size,
                    *corner_radius,
                    report,
                );
            }
            ModelingOperationSpec::CircularThroughCut {
                region,
                center,
                radius,
                radial_segments,
                rim_width,
                entry_loop,
                exit_loop,
                outer_region,
                rim_region,
                wall_region,
                ..
            } => {
                validate_region_reference(definition, *region, operation_id, report);
                validate_cut_generated_regions(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    *outer_region,
                    &[*rim_region, *wall_region],
                    report,
                );
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    *entry_loop,
                    *exit_loop,
                    report,
                );
                validate_cut_center(
                    definition.id,
                    operation_id,
                    "circular_through_cut",
                    center,
                    report,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "circular_through_cut.radius"),
                    *radius,
                );
                validate_count(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "circular_through_cut.radial_segments",
                    ),
                    *radial_segments,
                    6,
                );
                validate_positive(
                    report,
                    operation_subject(
                        definition.id,
                        operation_id,
                        "circular_through_cut.rim_width",
                    ),
                    *rim_width,
                );
            }
            ModelingOperationSpec::BevelBoundaryLoop {
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
                ..
            } => {
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.width"),
                    *width,
                );
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.segments"),
                    *segments,
                    1,
                );
                validate_positive(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.profile"),
                    *profile,
                );
                validate_range(
                    report,
                    operation_subject(definition.id, operation_id, "bevel_boundary_loop.profile"),
                    *profile,
                    BOUNDARY_BEVEL_PROFILE_MIN,
                    BOUNDARY_BEVEL_PROFILE_MAX,
                );
                if *bevel_region == RegionId(0) {
                    push_issue(
                        report,
                        operation_subject(
                            definition.id,
                            operation_id,
                            "bevel_boundary_loop.bevel_region",
                        ),
                        "invalid_region_id",
                        "Generated bevel region IDs must be non-zero.",
                    );
                }
                validate_cut_loop_pair(
                    definition.id,
                    operation_id,
                    "bevel_boundary_loop.replacement",
                    *outer_replacement_loop,
                    *inner_replacement_loop,
                    report,
                );
                if *target_loop == *outer_replacement_loop
                    || *target_loop == *inner_replacement_loop
                {
                    push_issue(
                        report,
                        operation_subject(
                            definition.id,
                            operation_id,
                            "bevel_boundary_loop.target_loop",
                        ),
                        "boundary_loop_dependency_self_output",
                        "Bevel replacement loops must differ from the consumed target loop.",
                    );
                }
            }
            ModelingOperationSpec::MirrorInstances {
                plane_normal,
                plane_offset,
                ..
            } => {
                if !array_is_finite(plane_normal) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "mirror.plane_normal"),
                        "non_finite",
                        "Mirror plane normal must be finite.",
                    );
                }
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "mirror.plane_offset"),
                    *plane_offset,
                );
            }
            ModelingOperationSpec::LinearArray { count, offset, .. } => {
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "linear_array.count"),
                    *count,
                    1,
                );
                if !array_is_finite(offset) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "linear_array.offset"),
                        "non_finite",
                        "Linear array offset must be finite.",
                    );
                }
            }
            ModelingOperationSpec::RadialArray {
                count,
                axis,
                angle_degrees,
                ..
            } => {
                validate_count(
                    report,
                    operation_subject(definition.id, operation_id, "radial_array.count"),
                    *count,
                    1,
                );
                if !array_is_finite(axis) {
                    push_issue(
                        report,
                        operation_subject(definition.id, operation_id, "radial_array.axis"),
                        "non_finite",
                        "Radial array axis must be finite.",
                    );
                }
                validate_finite(
                    report,
                    operation_subject(definition.id, operation_id, "radial_array.angle_degrees"),
                    *angle_degrees,
                );
            }
            ModelingOperationSpec::ReservedBoolean { .. }
            | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
        }
    }
}

fn validate_region_reference(
    definition: &PartDefinition,
    region: RegionId,
    operation: OperationId,
    report: &mut AssetValidationReport,
) {
    if !definition.regions.contains_key(&region) {
        push_issue(
            report,
            operation_subject(definition.id, operation, "region"),
            "unknown_operation_region",
            "Operation references an unknown region.",
        );
    }
}

fn validate_cut_center(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    center: &[f32; 2],
    report: &mut AssetValidationReport,
) {
    for (component, value) in ["x", "y"].into_iter().zip(center) {
        validate_finite(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.center.{component}"),
            ),
            *value,
        );
    }
}

fn validate_cut_size(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    size: &[f32; 2],
    report: &mut AssetValidationReport,
) {
    for (component, value) in ["x", "y"].into_iter().zip(size) {
        validate_positive(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.size.{component}"),
            ),
            *value,
        );
    }
}

fn validate_cut_generated_regions(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    outer_region: RegionId,
    generated_regions: &[RegionId],
    report: &mut AssetValidationReport,
) {
    let mut seen = BTreeSet::new();
    for region in generated_regions {
        if *region == outer_region {
            push_issue(
                report,
                operation_subject(
                    definition,
                    operation,
                    format!("{operation_kind}.generated_region"),
                ),
                "cut_region_collision",
                "Generated cut detail regions must not reuse the surviving outer host region.",
            );
        }
        if !seen.insert(*region) {
            push_issue(
                report,
                operation_subject(
                    definition,
                    operation,
                    format!("{operation_kind}.generated_region"),
                ),
                "duplicate_cut_generated_region",
                "Generated cut detail regions must be distinct.",
            );
        }
    }
}

fn validate_cut_loop_pair(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    first: BoundaryLoopId,
    second: BoundaryLoopId,
    report: &mut AssetValidationReport,
) {
    if first == second {
        push_issue(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.boundary_loop"),
            ),
            "duplicate_cut_boundary_loop",
            "Each physical cut boundary loop must have a distinct semantic ID.",
        );
    }
}

fn validate_rect_cut_corner_radius(
    definition: PartDefinitionId,
    operation: OperationId,
    operation_kind: &'static str,
    size: &[f32; 2],
    corner_radius: f32,
    report: &mut AssetValidationReport,
) {
    if size
        .iter()
        .copied()
        .all(|value| value.is_finite() && value > 0.0)
        && corner_radius.is_finite()
        && corner_radius > size[0].min(size[1]) * 0.5
    {
        push_issue(
            report,
            operation_subject(
                definition,
                operation,
                format!("{operation_kind}.corner_radius"),
            ),
            "cut_corner_radius_too_large",
            "Cut corner radius must not exceed half the smaller cut dimension.",
        );
    }
}

fn validate_parent_cycles(recipe: &AssetRecipe, report: &mut AssetValidationReport) {
    for id in recipe.instances.keys() {
        let mut seen = BTreeSet::new();
        let mut cursor = Some(*id);
        while let Some(current) = cursor {
            if !seen.insert(current) {
                push_issue(
                    report,
                    Some(format!("instance.{}", id.0)),
                    "parent_cycle",
                    "Instance parent chain contains a cycle.",
                );
                break;
            }
            cursor = recipe
                .instances
                .get(&current)
                .and_then(|instance| instance.parent);
        }
    }
}

fn validate_transform(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    transform: &Transform3,
) {
    validate_finite_array(
        report,
        append_subject(subject.clone(), "translation"),
        &transform.translation,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "rotation_degrees"),
        &transform.rotation_degrees,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "scale"),
        &transform.scale,
    );
    if transform
        .scale
        .iter()
        .copied()
        .any(|value| value.is_finite() && value == 0.0)
    {
        push_issue(
            report,
            append_subject(subject, "scale"),
            "zero_scale",
            "Transform scale axes must be non-zero.",
        );
    }
}

fn validate_frame(report: &mut AssetValidationReport, subject: Option<String>, frame: &Frame3) {
    validate_finite_array(
        report,
        append_subject(subject.clone(), "origin"),
        &frame.origin,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "x_axis"),
        &frame.x_axis,
    );
    validate_finite_array(
        report,
        append_subject(subject.clone(), "y_axis"),
        &frame.y_axis,
    );
    validate_finite_array(report, append_subject(subject, "z_axis"), &frame.z_axis);
}

fn validate_finite_array(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    values: &[f32],
) {
    if !values.iter().copied().all(f32::is_finite) {
        push_issue(
            report,
            subject,
            "non_finite",
            "All numeric components must be finite.",
        );
    }
}

fn validate_positive_array(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    values: &[f32],
) {
    for value in values {
        validate_positive(report, subject.clone(), *value);
    }
}

fn validate_positive(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && value <= 0.0 {
        push_issue(
            report,
            subject,
            "not_positive",
            "Value must be greater than zero.",
        );
    }
}

fn validate_non_negative(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && value < 0.0 {
        push_issue(
            report,
            subject,
            "negative_value",
            "Value must not be negative.",
        );
    }
}

fn validate_finite(report: &mut AssetValidationReport, subject: Option<String>, value: f32) {
    if !value.is_finite() {
        push_issue(report, subject, "non_finite", "Value must be finite.");
    }
}

fn validate_range(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: f32,
    minimum: f32,
    maximum: f32,
) {
    validate_finite(report, subject.clone(), value);
    if value.is_finite() && (value < minimum || value > maximum) {
        push_issue(
            report,
            subject,
            "value_out_of_range",
            format!("Value must be between {minimum:.3} and {maximum:.3}."),
        );
    }
}

fn validate_count(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    value: u32,
    minimum: u32,
) {
    if value < minimum {
        push_issue(
            report,
            subject,
            "count_too_small",
            format!("Count must be at least {minimum}."),
        );
    }
}
