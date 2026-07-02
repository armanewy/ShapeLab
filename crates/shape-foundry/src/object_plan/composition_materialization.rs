
fn validate_policy(report: &mut ObjectPlanValidationReport, policy: &ObjectPlanValidationPolicy) {
    if !policy.require_primitive_schema_validation {
        report.push(
            "validation_policy.require_primitive_schema_validation",
            "primitive_schema_validation_required",
            "ObjectPlan intake cannot bypass primitive schema validation.",
        );
    }
    if !policy.require_anchor_validation {
        report.push(
            "validation_policy.require_anchor_validation",
            "anchor_validation_required",
            "ObjectPlan intake cannot bypass safe-anchor validation.",
        );
    }
    if policy.allow_public_catalog_publish {
        report.push(
            "validation_policy.allow_public_catalog_publish",
            "public_catalog_publish_rejected",
            "ObjectPlan intake cannot publish directly to the public catalog.",
        );
    }
}

fn validate_provenance(
    report: &mut ObjectPlanValidationReport,
    provenance: &ObjectPlanProvenance,
    review_tier: ObjectPlanReviewTier,
) {
    if let Some(hash) = &provenance.source_prompt_hash {
        validate_reference_text(
            report,
            "provenance.source_prompt_hash",
            hash,
            "invalid_source_prompt_hash",
        );
    }
    for (index, seed_ref) in provenance.source_seed_refs.iter().enumerate() {
        validate_reference_text(
            report,
            format!("provenance.source_seed_refs.{index}"),
            seed_ref,
            "invalid_source_seed_ref",
        );
    }
    validate_reference_text(
        report,
        "provenance.created_at",
        &provenance.created_at,
        "invalid_object_plan_created_at",
    );
    if provenance.created_by == ObjectPlanCreatedBy::LlmDraft
        && review_tier != ObjectPlanReviewTier::Draft
    {
        report.push(
            "review_tier",
            "llm_draft_must_remain_draft",
            "Offline LLM ObjectPlans remain Draft until reviewed.",
        );
    }
}

fn validate_node(report: &mut ObjectPlanValidationReport, index: usize, node: &ObjectPlanNode) {
    let subject = format!("nodes.{index}");
    validate_identifier(
        report,
        format!("{subject}.node_id"),
        &node.node_id,
        "invalid_object_plan_node_id",
    );
    validate_product_text(
        report,
        format!("{subject}.display_name"),
        &node.display_name,
        "invalid_object_plan_node_display_name",
    );
    validate_product_text(
        report,
        format!("{subject}.role_hint"),
        &node.role_hint,
        "invalid_object_plan_node_role_hint",
    );

    let Some(schema) = primitive_property_schema_for_kind(node.primitive_kind) else {
        report.push(
            format!("{subject}.primitive_kind"),
            "unsupported_object_plan_primitive_kind",
            "ObjectPlan node primitive kind is not supported.",
        );
        return;
    };
    extend_property_report(
        report,
        &subject,
        validate_primitive_property_values(&schema, &node.property_values),
    );
}

fn object_plan_composition_document(plan: &ObjectPlan) -> PrimitiveCompositionDocument {
    PrimitiveCompositionDocument {
        schema_version: crate::PRIMITIVE_COMPOSITION_SCHEMA_VERSION,
        document_id: plan.plan_id.clone(),
        nodes: plan
            .nodes
            .iter()
            .map(|node| PrimitiveNode {
                node_id: node.node_id.clone(),
                primitive_kind: node.primitive_kind,
                property_values: node.property_values.clone(),
                local_label: node.display_name.clone(),
                visibility: PrimitiveNodeVisibility::Visible,
            })
            .collect(),
        attachments: plan
            .attachments
            .iter()
            .map(|attachment| PrimitiveAttachment {
                attachment_id: attachment.attachment_id.clone(),
                parent_node_id: attachment.parent_node_id.clone(),
                parent_anchor_id: attachment.parent_anchor_id.clone(),
                child_node_id: attachment.child_node_id.clone(),
                child_anchor_id: attachment.child_anchor_id.clone(),
                offset_policy: attachment.offset.clone(),
                orientation_policy: attachment.orientation_policy,
                scale_policy: attachment.scale_policy,
            })
            .collect(),
        root_node_id: plan
            .nodes
            .first()
            .map(|node| node.node_id.clone())
            .unwrap_or_default(),
    }
}

fn object_plan_materialized_composition_document(
    plan: &ObjectPlan,
    primitive_instances: &[MaterializedPrimitiveInstance],
    status: MaterializationStatus,
) -> PrimitiveCompositionDocument {
    let supported_node_ids = primitive_instances
        .iter()
        .map(|instance| instance.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let attachments =
        if status == MaterializationStatus::Passed || status == MaterializationStatus::Partial {
            plan.attachments
                .iter()
                .filter(|attachment| {
                    supported_node_ids.contains(attachment.parent_node_id.as_str())
                        && supported_node_ids.contains(attachment.child_node_id.as_str())
                        && object_plan_materialization_supports_attachment(attachment, plan)
                })
                .map(|attachment| PrimitiveAttachment {
                    attachment_id: attachment.attachment_id.clone(),
                    parent_node_id: attachment.parent_node_id.clone(),
                    parent_anchor_id: attachment.parent_anchor_id.clone(),
                    child_node_id: attachment.child_node_id.clone(),
                    child_anchor_id: attachment.child_anchor_id.clone(),
                    offset_policy: attachment.offset.clone(),
                    orientation_policy: attachment.orientation_policy,
                    scale_policy: attachment.scale_policy,
                })
                .collect()
        } else {
            Vec::new()
        };

    PrimitiveCompositionDocument {
        schema_version: crate::PRIMITIVE_COMPOSITION_SCHEMA_VERSION,
        document_id: format!("{}_materialized", plan.plan_id),
        nodes: primitive_instances
            .iter()
            .map(|instance| PrimitiveNode {
                node_id: instance.node_id.clone(),
                primitive_kind: instance.primitive_kind,
                property_values: instance.property_values.clone(),
                local_label: instance.display_name.clone(),
                visibility: PrimitiveNodeVisibility::Visible,
            })
            .collect(),
        attachments,
        root_node_id: primitive_instances
            .first()
            .map(|instance| instance.node_id.clone())
            .unwrap_or_default(),
    }
}

fn object_plan_materialized_relationship_contracts(
    plan: &ObjectPlan,
    status: MaterializationStatus,
) -> Vec<RelationshipContract> {
    if !matches!(
        status,
        MaterializationStatus::Passed | MaterializationStatus::Partial
    ) {
        return Vec::new();
    }

    plan.attachments
        .iter()
        .filter(|attachment| object_plan_materialization_supports_attachment(attachment, plan))
        .enumerate()
        .map(|(index, attachment)| object_plan_attachment_relationship(index, attachment))
        .collect()
}

fn object_plan_attachment_relationship(
    index: usize,
    attachment: &ObjectPlanAttachment,
) -> RelationshipContract {
    let id = RelationshipId(index as u64 + 1);
    RelationshipContract {
        id,
        relationship_type: RelationshipType::SurfaceMounted,
        parent: None,
        child: None,
        parent_node_ref: Some(attachment.parent_node_id.clone()),
        child_node_ref: Some(attachment.child_node_id.clone()),
        parent_anchor_id: Some(attachment.parent_anchor_id.clone()),
        child_anchor_id: Some(attachment.child_anchor_id.clone()),
        label: "Panel with Knob surface mount".to_owned(),
        export_profile: None,
        placement_policy: PlacementPolicy {
            position_rule: attachment_position_rule(attachment),
        },
        orientation_policy: match attachment.orientation_policy {
            PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal => {
                OrientationPolicy::AlignToSurfaceNormal {
                    max_angle_degrees: 0.0,
                }
            }
            PrimitiveAttachmentOrientationPolicy::PreserveChildForward => {
                OrientationPolicy::PreserveChild
            }
        },
        scale_policy: match attachment.scale_policy {
            PrimitiveAttachmentScalePolicy::KeepChildScale => ScalePolicy::PreserveChild,
        },
        contact_policy: ContactPolicy::SurfaceContact { clearance: 0.0 },
        edit_policy: Default::default(),
        selection_policy: Default::default(),
        reset_policy: Default::default(),
        export_realization: ExportRealizationPolicy::PreserveSemanticSidecar,
    }
}

fn attachment_position_rule(attachment: &ObjectPlanAttachment) -> PositionRule {
    match &attachment.offset {
        PrimitiveAttachmentOffsetPolicy::Fixed => PositionRule::CenteredInZone {
            zone: attachment.parent_anchor_id.clone(),
        },
        PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
            x,
            y,
            minimum_x,
            maximum_x,
            minimum_y,
            maximum_y,
        } => PositionRule::ProportionalUv {
            u: normalize_bounded_offset(*x, *minimum_x, *maximum_x),
            v: normalize_bounded_offset(*y, *minimum_y, *maximum_y),
        },
    }
}

fn normalize_bounded_offset(value: f32, minimum: f32, maximum: f32) -> f32 {
    if (maximum - minimum).abs() <= f32::EPSILON {
        0.5
    } else {
        ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
    }
}
