
fn object_plan_materialization_supports_primitive(primitive_kind: PrimitiveKind) -> bool {
    matches!(
        primitive_kind,
        PrimitiveKind::BoxPrimitive
            | PrimitiveKind::FlatPanelPrimitive
            | PrimitiveKind::SpherePrimitive
    )
}

fn object_plan_materialization_supports_attachment(
    attachment: &ObjectPlanAttachment,
    plan: &ObjectPlan,
) -> bool {
    let nodes = plan
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.primitive_kind))
        .collect::<BTreeMap<_, _>>();
    matches!(
        (
            nodes.get(attachment.parent_node_id.as_str()).copied(),
            nodes.get(attachment.child_node_id.as_str()).copied(),
            attachment.parent_anchor_id.as_str(),
            attachment.child_anchor_id.as_str(),
            attachment.orientation_policy,
            attachment.scale_policy,
        ),
        (
            Some(PrimitiveKind::FlatPanelPrimitive),
            Some(PrimitiveKind::SpherePrimitive),
            "front_handle_zone" | "right_side_handle_zone",
            "back_mount_point",
            PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal,
            PrimitiveAttachmentScalePolicy::KeepChildScale,
        )
    )
}

fn extend_property_report(
    report: &mut ObjectPlanValidationReport,
    subject: &str,
    nested: PrimitivePropertyValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("{subject}.property_values.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn extend_composition_report(
    report: &mut ObjectPlanValidationReport,
    nested: PrimitiveCompositionValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("composition.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
}

fn attachment_summary(
    attachment: &ObjectPlanAttachment,
    nodes: &BTreeMap<&str, &ObjectPlanNode>,
) -> String {
    let parent = nodes
        .get(attachment.parent_node_id.as_str())
        .map(|node| node.display_name.as_str())
        .unwrap_or("Parent primitive");
    let child = nodes
        .get(attachment.child_node_id.as_str())
        .map(|node| node.display_name.as_str())
        .unwrap_or("Child primitive");
    let anchor = nodes
        .get(attachment.parent_node_id.as_str())
        .and_then(|node| {
            parent_anchor_display_name(node.primitive_kind, &attachment.parent_anchor_id)
        })
        .unwrap_or_else(|| "approved anchor".to_owned());
    format!("{child} attaches to {parent} at {anchor}.")
}

fn parent_anchor_display_name(primitive_kind: PrimitiveKind, anchor_id: &str) -> Option<String> {
    primitive_anchor_definitions(primitive_kind, "summary")
        .into_iter()
        .find(|anchor| anchor.anchor_id == anchor_id)
        .map(|anchor| anchor.display_name)
}

fn primitive_property_schema_for_kind(
    primitive_kind: PrimitiveKind,
) -> Option<PrimitivePropertySchema> {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => Some(box_primitive_property_schema()),
        PrimitiveKind::FlatPanelPrimitive => Some(flat_panel_primitive_property_schema()),
        PrimitiveKind::SpherePrimitive => Some(sphere_primitive_property_schema()),
        PrimitiveKind::CylinderPrimitive => None,
    }
}

fn primitive_display_name(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "Box Primitive",
        PrimitiveKind::FlatPanelPrimitive => "Flat Panel Primitive",
        PrimitiveKind::SpherePrimitive => "Sphere Primitive",
        PrimitiveKind::CylinderPrimitive => "Unsupported primitive",
    }
}

fn validate_identifier(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
        || contains_internal_term(value)
        || looks_like_path(value)
    {
        report.push(
            subject,
            code,
            "ObjectPlan IDs must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_text(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || looks_like_path(trimmed)
        || trimmed.contains("::")
    {
        report.push(
            subject,
            code,
            "ObjectPlan product-facing text must be product-safe.",
        );
    }
}

fn validate_reference_text(
    report: &mut ObjectPlanValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || looks_like_path(trimmed)
        || trimmed.contains("::")
    {
        report.push(
            subject,
            code,
            "ObjectPlan source references must be product-safe and local-path free.",
        );
    }
}

fn contains_internal_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "kernel",
        "module",
        "provider",
        "slot",
        "fingerprint",
        "operation id",
        "scalar path",
        "raw transform",
        "matrix",
        "mesh payload",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn contains_blender_like_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "vertex", "vertices", "face", "faces", "loop", "loops", "cage", "boolean", "sculpt",
        "topology", "mesh", "gizmo", "blender",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.contains("~/")
}
