
/// Validate an ObjectPlan before an offline tool can render or save it.
#[must_use]
pub fn validate_object_plan(plan: &ObjectPlan) -> ObjectPlanValidationReport {
    let mut report = ObjectPlanValidationReport::default();

    if plan.schema_version != OBJECT_PLAN_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_object_plan_schema",
            "ObjectPlan schema version is not supported.",
        );
    }
    validate_identifier(
        &mut report,
        "plan_id",
        &plan.plan_id,
        "invalid_object_plan_id",
    );
    validate_product_text(
        &mut report,
        "display_name",
        &plan.display_name,
        "invalid_object_plan_display_name",
    );
    validate_product_text(
        &mut report,
        "intent_summary",
        &plan.intent_summary,
        "invalid_object_plan_intent_summary",
    );
    validate_policy(&mut report, &plan.validation_policy);
    validate_provenance(&mut report, &plan.provenance, plan.review_tier);

    if plan.nodes.is_empty() {
        report.push(
            "nodes",
            "missing_object_plan_nodes",
            "ObjectPlans must contain at least one primitive node.",
        );
    }

    let mut node_ids = BTreeSet::new();
    for (index, node) in plan.nodes.iter().enumerate() {
        validate_node(&mut report, index, node);
        if !node_ids.insert(node.node_id.as_str()) {
            report.push(
                format!("nodes.{index}.node_id"),
                "duplicate_object_plan_node",
                "ObjectPlan node IDs must be unique.",
            );
        }
    }

    extend_composition_report(
        &mut report,
        validate_primitive_composition_document(&object_plan_composition_document(plan)),
    );

    report
}

/// Materialize an ObjectPlan into an internal draft graph.
#[must_use]
pub fn materialize_object_plan(
    request: ObjectPlanMaterializationRequest,
) -> MaterializedObjectDraft {
    let mut validation_report = validate_object_plan(&request.plan);
    if !request.materialization_policy.forbid_catalog_publish {
        validation_report.push(
            "materialization_policy.forbid_catalog_publish",
            "materialization_catalog_publish_forbidden",
            "ObjectPlan materialization cannot enable catalog publishing.",
        );
    }

    let validation_failed = !validation_report.is_valid();
    let fail_on_invalid = request.materialization_policy.require_valid_plan && validation_failed;
    let mut unresolved_nodes = Vec::new();
    let mut unresolved_attachments = Vec::new();
    let mut primitive_instances = Vec::new();

    if fail_on_invalid {
        unresolved_nodes.extend(
            request
                .plan
                .nodes
                .iter()
                .map(|node| UnresolvedObjectPlanNode {
                    node_id: node.node_id.clone(),
                    display_name: node.display_name.clone(),
                    reason: "Plan validation failed before materialization.".to_owned(),
                }),
        );
        unresolved_attachments.extend(request.plan.attachments.iter().map(|attachment| {
            UnresolvedObjectPlanAttachment {
                attachment_id: attachment.attachment_id.clone(),
                parent_node_id: attachment.parent_node_id.clone(),
                child_node_id: attachment.child_node_id.clone(),
                reason: "Plan validation failed before materialization.".to_owned(),
            }
        }));
    } else {
        for node in &request.plan.nodes {
            if object_plan_materialization_supports_primitive(node.primitive_kind) {
                primitive_instances.push(MaterializedPrimitiveInstance {
                    node_id: node.node_id.clone(),
                    primitive_kind: node.primitive_kind,
                    display_name: if request.materialization_policy.preserve_node_labels {
                        node.display_name.clone()
                    } else {
                        primitive_display_name(node.primitive_kind).to_owned()
                    },
                    property_values: node.property_values.clone(),
                    locked: node.locked,
                });
            } else {
                unresolved_nodes.push(UnresolvedObjectPlanNode {
                    node_id: node.node_id.clone(),
                    display_name: node.display_name.clone(),
                    reason: "Primitive is not supported by ObjectPlan materialization v1."
                        .to_owned(),
                });
            }
        }

        let supported_node_ids = primitive_instances
            .iter()
            .map(|instance| instance.node_id.as_str())
            .collect::<BTreeSet<_>>();
        for attachment in &request.plan.attachments {
            if supported_node_ids.contains(attachment.parent_node_id.as_str())
                && supported_node_ids.contains(attachment.child_node_id.as_str())
                && object_plan_materialization_supports_attachment(attachment, &request.plan)
            {
                continue;
            }
            unresolved_attachments.push(UnresolvedObjectPlanAttachment {
                attachment_id: attachment.attachment_id.clone(),
                parent_node_id: attachment.parent_node_id.clone(),
                child_node_id: attachment.child_node_id.clone(),
                reason: "Attachment is not supported by ObjectPlan materialization v1.".to_owned(),
            });
        }
    }

    let status = if validation_failed
        || (!unresolved_nodes.is_empty()
            && request.materialization_policy.require_supported_primitives)
        || (!unresolved_attachments.is_empty()
            && request.materialization_policy.require_supported_attachments)
    {
        MaterializationStatus::Failed
    } else if unresolved_nodes.is_empty() && unresolved_attachments.is_empty() {
        MaterializationStatus::Passed
    } else {
        MaterializationStatus::Partial
    };

    let composition_document =
        object_plan_materialized_composition_document(&request.plan, &primitive_instances, status);
    let relationship_contracts =
        object_plan_materialized_relationship_contracts(&request.plan, status);
    MaterializedObjectDraft {
        draft_id: format!("{}_draft", request.plan.plan_id),
        source_plan_id: request.plan.plan_id,
        status,
        primitive_instances,
        composition_document,
        relationship_contracts,
        unresolved_nodes,
        unresolved_attachments,
        validation_report,
        review_tier: ObjectPlanReviewTier::Draft,
        user_review_required: true,
        publish_allowed: false,
    }
}

/// Build a product-safe summary for materialized ObjectPlan review.
#[must_use]
pub fn materialized_object_summary(
    plan: &ObjectPlan,
    draft: &MaterializedObjectDraft,
) -> MaterializedObjectSummary {
    MaterializedObjectSummary {
        source_plan_label: plan.display_name.clone(),
        supported_primitive_count: draft.primitive_instances.len(),
        unresolved_primitive_count: draft.unresolved_nodes.len(),
        supported_attachment_count: draft.composition_document.attachments.len(),
        unresolved_attachment_count: draft.unresolved_attachments.len(),
        user_review_required: draft.user_review_required,
        next_action: match draft.status {
            MaterializationStatus::Passed => MaterializedObjectNextAction::Review,
            MaterializationStatus::Partial if !draft.primitive_instances.is_empty() => {
                MaterializedObjectNextAction::Simplify
            }
            MaterializationStatus::Partial => MaterializedObjectNextAction::Regenerate,
            MaterializationStatus::Failed => MaterializedObjectNextAction::Blocked,
        },
    }
}

/// Build a product-safe summary for review UI or offline reports.
#[must_use]
pub fn object_plan_user_summary(plan: &ObjectPlan) -> ObjectPlanUserSummary {
    let nodes = plan
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let primitives_used = plan
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{} as {}",
                primitive_display_name(node.primitive_kind),
                node.display_name
            )
        })
        .collect::<Vec<_>>();
    let adjustable_properties = plan
        .nodes
        .iter()
        .map(|node| {
            let properties = primitive_property_schema_for_kind(node.primitive_kind)
                .map(|schema| {
                    schema
                        .properties
                        .iter()
                        .map(|property| property.display_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "No approved properties".to_owned());
            format!("{}: {}", node.display_name, properties)
        })
        .collect::<Vec<_>>();
    let attachments = plan
        .attachments
        .iter()
        .map(|attachment| attachment_summary(attachment, &nodes))
        .collect::<Vec<_>>();

    ObjectPlanUserSummary {
        display_name: plan.display_name.clone(),
        intent_summary: plan.intent_summary.clone(),
        primitives_used,
        adjustable_properties,
        attachments,
        review_summary: match plan.review_tier {
            ObjectPlanReviewTier::Draft => "Draft plan. Review before keeping.".to_owned(),
            ObjectPlanReviewTier::Personal => "Personal plan kept for local use.".to_owned(),
            ObjectPlanReviewTier::Reviewed => "Reviewed plan.".to_owned(),
        },
    }
}

/// Validate an offline ObjectPlan repair suggestion.
#[must_use]
pub fn validate_object_plan_repair_suggestion(
    suggestion: &ObjectPlanRepairSuggestion,
) -> ObjectPlanValidationReport {
    let mut report = ObjectPlanValidationReport::default();
    validate_identifier(
        &mut report,
        "finding_id",
        &suggestion.finding_id,
        "invalid_repair_finding_id",
    );
    validate_product_text(
        &mut report,
        "summary",
        &suggestion.summary,
        "invalid_repair_summary",
    );
    validate_product_text(
        &mut report,
        "suggested_change",
        &suggestion.suggested_change,
        "invalid_repair_suggested_change",
    );
    if let Some(node_id) = &suggestion.target_node_id {
        validate_identifier(
            &mut report,
            "target_node_id",
            node_id,
            "invalid_repair_target_node_id",
        );
    }
    if let Some(property_id) = &suggestion.target_property_id {
        validate_identifier(
            &mut report,
            "target_property_id",
            property_id,
            "invalid_repair_target_property_id",
        );
    }
    if let Some(attachment_id) = &suggestion.target_attachment_id {
        validate_identifier(
            &mut report,
            "target_attachment_id",
            attachment_id,
            "invalid_repair_target_attachment_id",
        );
    }
    if !suggestion.requires_human_review {
        report.push(
            "requires_human_review",
            "repair_requires_human_review",
            "ObjectPlan repair suggestions require human review.",
        );
    }
    report
}
