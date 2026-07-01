//! Safe primitive composition contracts.
//!
//! Composition is modeled as primitive nodes connected through named anchors.
//! These contracts intentionally avoid raw transforms, mesh payloads, and
//! unrestricted scene editing.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    PrimitiveKind, PrimitivePropertySchema, PrimitivePropertyValidationReport,
    PrimitivePropertyValue, box_primitive_property_schema, flat_panel_primitive_property_schema,
    sphere_primitive_property_schema, validate_primitive_property_values,
};

/// Current schema version for primitive composition documents.
pub const PRIMITIVE_COMPOSITION_SCHEMA_VERSION: u32 = 1;

/// A validated primitive composition document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveCompositionDocument {
    /// Schema version.
    pub schema_version: u32,
    /// Stable document ID.
    pub document_id: String,
    /// Primitive nodes in the composition.
    pub nodes: Vec<PrimitiveNode>,
    /// Constrained attachments between nodes.
    pub attachments: Vec<PrimitiveAttachment>,
    /// Root node ID.
    pub root_node_id: String,
}

/// One primitive node in a composition document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveNode {
    /// Stable node ID.
    pub node_id: String,
    /// Primitive kind.
    pub primitive_kind: PrimitiveKind,
    /// Current property values keyed by primitive property ID.
    pub property_values: BTreeMap<String, PrimitivePropertyValue>,
    /// Product-safe local label.
    pub local_label: String,
    /// Node visibility.
    pub visibility: PrimitiveNodeVisibility,
}

/// Node visibility.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveNodeVisibility {
    /// Node is visible and exportable.
    Visible,
    /// Node is temporarily hidden from preview/export.
    Hidden,
}

/// A named primitive anchor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveAnchor {
    /// Stable anchor ID.
    pub anchor_id: String,
    /// Owning node ID.
    pub node_id: String,
    /// Product-facing anchor name.
    pub display_name: String,
    /// Anchor kind.
    pub anchor_kind: PrimitiveAnchorKind,
    /// Normalized location in primitive-local space.
    pub normalized_location: [f32; 3],
    /// Preferred outward direction.
    pub normal: [f32; 3],
    /// Preferred horizontal direction for bounded offsets.
    pub tangent: [f32; 3],
    /// Child primitive kinds allowed at this anchor.
    pub allowed_child_kinds: Vec<PrimitiveKind>,
    /// Product-safe description.
    pub product_safe_description: String,
}

/// Supported anchor kinds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveAnchorKind {
    /// Center of a broad primitive side.
    FaceCenter,
    /// Bounded band along an edge-like region.
    EdgeBand,
    /// Primitive corner.
    Corner,
    /// Named point on the surface.
    SurfacePoint,
    /// Primitive axis.
    Axis,
}

/// A constrained attachment between two primitive nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveAttachment {
    /// Stable attachment ID.
    pub attachment_id: String,
    /// Parent node ID.
    pub parent_node_id: String,
    /// Parent anchor ID.
    pub parent_anchor_id: String,
    /// Child node ID.
    pub child_node_id: String,
    /// Child anchor ID.
    pub child_anchor_id: String,
    /// Bounded offset policy.
    pub offset_policy: PrimitiveAttachmentOffsetPolicy,
    /// Derived orientation policy.
    pub orientation_policy: PrimitiveAttachmentOrientationPolicy,
    /// Scale policy.
    pub scale_policy: PrimitiveAttachmentScalePolicy,
}

/// Safe offset policy for an attachment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase", deny_unknown_fields)]
pub enum PrimitiveAttachmentOffsetPolicy {
    /// No user-adjustable offset.
    Fixed,
    /// Bounded normalized offset along the parent anchor tangent plane.
    BoundedNormalized {
        /// Current normalized X offset.
        x: f32,
        /// Current normalized Y offset.
        y: f32,
        /// Minimum X offset.
        minimum_x: f32,
        /// Maximum X offset.
        maximum_x: f32,
        /// Minimum Y offset.
        minimum_y: f32,
        /// Maximum Y offset.
        maximum_y: f32,
    },
}

/// Derived orientation policy for an attachment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveAttachmentOrientationPolicy {
    /// Child mount direction is aligned to the parent anchor direction.
    AlignChildToParentNormal,
    /// Child keeps its primitive-local forward orientation.
    PreserveChildForward,
}

/// Scale policy for a child attachment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveAttachmentScalePolicy {
    /// Child primitive keeps its own schema-controlled size.
    KeepChildScale,
}

/// One primitive composition validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveCompositionValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Primitive composition validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveCompositionValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PrimitiveCompositionValidationIssue>,
}

impl PrimitiveCompositionValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(PrimitiveCompositionValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return the built-in anchors for a primitive node.
#[must_use]
pub fn primitive_anchors_for_node(node: &PrimitiveNode) -> Vec<PrimitiveAnchor> {
    primitive_anchor_definitions(node.primitive_kind, &node.node_id)
}

/// Return the built-in anchors for a primitive kind and node ID.
#[must_use]
pub fn primitive_anchor_definitions(
    primitive_kind: PrimitiveKind,
    node_id: &str,
) -> Vec<PrimitiveAnchor> {
    match primitive_kind {
        PrimitiveKind::FlatPanelPrimitive => flat_panel_anchors(node_id),
        PrimitiveKind::SpherePrimitive => sphere_anchors(node_id),
        PrimitiveKind::BoxPrimitive => box_anchors(node_id),
        PrimitiveKind::CylinderPrimitive => Vec::new(),
    }
}

/// Validate one anchor definition.
#[must_use]
pub fn validate_primitive_anchor(anchor: &PrimitiveAnchor) -> PrimitiveCompositionValidationReport {
    let mut report = PrimitiveCompositionValidationReport::default();
    validate_identifier(
        &mut report,
        "anchor.anchor_id",
        &anchor.anchor_id,
        "invalid_anchor_id",
    );
    validate_identifier(
        &mut report,
        "anchor.node_id",
        &anchor.node_id,
        "invalid_anchor_node_id",
    );
    validate_product_label(
        &mut report,
        "anchor.display_name",
        &anchor.display_name,
        "invalid_anchor_label",
    );
    validate_product_label(
        &mut report,
        "anchor.product_safe_description",
        &anchor.product_safe_description,
        "invalid_anchor_description",
    );
    validate_normalized_vector(
        &mut report,
        "anchor.normalized_location",
        anchor.normalized_location,
        true,
    );
    validate_direction_vector(&mut report, "anchor.normal", anchor.normal);
    validate_direction_vector(&mut report, "anchor.tangent", anchor.tangent);
    report
}

/// Validate a primitive composition document before it can become state.
#[must_use]
pub fn validate_primitive_composition_document(
    document: &PrimitiveCompositionDocument,
) -> PrimitiveCompositionValidationReport {
    let mut report = PrimitiveCompositionValidationReport::default();

    if document.schema_version != PRIMITIVE_COMPOSITION_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_primitive_composition_schema",
            "Primitive composition schema version is not supported.",
        );
    }
    validate_identifier(
        &mut report,
        "document_id",
        &document.document_id,
        "invalid_composition_document_id",
    );
    validate_identifier(
        &mut report,
        "root_node_id",
        &document.root_node_id,
        "invalid_root_node_id",
    );
    if document.nodes.is_empty() {
        report.push(
            "nodes",
            "missing_composition_nodes",
            "Primitive composition documents must contain at least one node.",
        );
    }

    let mut node_ids = BTreeSet::new();
    let mut nodes = BTreeMap::new();
    for (index, node) in document.nodes.iter().enumerate() {
        validate_node(&mut report, index, node);
        if !node_ids.insert(node.node_id.as_str()) {
            report.push(
                format!("nodes.{index}.node_id"),
                "duplicate_composition_node",
                "Primitive composition node IDs must be unique.",
            );
        }
        nodes.insert(node.node_id.as_str(), node);
    }
    if !nodes.contains_key(document.root_node_id.as_str()) {
        report.push(
            "root_node_id",
            "unknown_root_node",
            "Primitive composition root node must reference an existing node.",
        );
    }

    let mut attachment_ids = BTreeSet::new();
    for (index, attachment) in document.attachments.iter().enumerate() {
        validate_attachment(&mut report, index, attachment, &nodes, &mut attachment_ids);
    }

    report
}

fn validate_node(
    report: &mut PrimitiveCompositionValidationReport,
    index: usize,
    node: &PrimitiveNode,
) {
    let subject = format!("nodes.{index}");
    validate_identifier(
        report,
        format!("{subject}.node_id"),
        &node.node_id,
        "invalid_composition_node_id",
    );
    validate_product_label(
        report,
        format!("{subject}.local_label"),
        &node.local_label,
        "invalid_composition_node_label",
    );

    let Some(schema) = primitive_property_schema_for_kind(node.primitive_kind) else {
        report.push(
            format!("{subject}.primitive_kind"),
            "unsupported_composition_primitive_kind",
            "Primitive kind is not supported by composition contracts.",
        );
        return;
    };
    extend_property_report(
        report,
        &subject,
        validate_primitive_property_values(&schema, &node.property_values),
    );

    for anchor in primitive_anchors_for_node(node) {
        extend_anchor_report(report, &subject, validate_primitive_anchor(&anchor));
    }
}

fn validate_attachment(
    report: &mut PrimitiveCompositionValidationReport,
    index: usize,
    attachment: &PrimitiveAttachment,
    nodes: &BTreeMap<&str, &PrimitiveNode>,
    attachment_ids: &mut BTreeSet<String>,
) {
    let subject = format!("attachments.{index}");
    validate_identifier(
        report,
        format!("{subject}.attachment_id"),
        &attachment.attachment_id,
        "invalid_attachment_id",
    );
    if !attachment_ids.insert(attachment.attachment_id.clone()) {
        report.push(
            format!("{subject}.attachment_id"),
            "duplicate_attachment",
            "Attachment IDs must be unique.",
        );
    }
    validate_identifier(
        report,
        format!("{subject}.parent_node_id"),
        &attachment.parent_node_id,
        "invalid_attachment_parent_node_id",
    );
    validate_identifier(
        report,
        format!("{subject}.parent_anchor_id"),
        &attachment.parent_anchor_id,
        "invalid_attachment_parent_anchor_id",
    );
    validate_identifier(
        report,
        format!("{subject}.child_node_id"),
        &attachment.child_node_id,
        "invalid_attachment_child_node_id",
    );
    validate_identifier(
        report,
        format!("{subject}.child_anchor_id"),
        &attachment.child_anchor_id,
        "invalid_attachment_child_anchor_id",
    );
    if attachment.parent_node_id == attachment.child_node_id {
        report.push(
            format!("{subject}.child_node_id"),
            "self_attachment_rejected",
            "Primitive nodes cannot attach to themselves.",
        );
    }

    let parent = nodes.get(attachment.parent_node_id.as_str()).copied();
    let child = nodes.get(attachment.child_node_id.as_str()).copied();
    let Some(parent) = parent else {
        report.push(
            format!("{subject}.parent_node_id"),
            "unknown_attachment_parent_node",
            "Attachment parent must reference an existing node.",
        );
        validate_offset_policy(report, &subject, &attachment.offset_policy);
        return;
    };
    let Some(child) = child else {
        report.push(
            format!("{subject}.child_node_id"),
            "unknown_attachment_child_node",
            "Attachment child must reference an existing node.",
        );
        validate_offset_policy(report, &subject, &attachment.offset_policy);
        return;
    };

    let parent_anchors = primitive_anchors_for_node(parent);
    let child_anchors = primitive_anchors_for_node(child);
    let parent_anchor = parent_anchors
        .iter()
        .find(|anchor| anchor.anchor_id == attachment.parent_anchor_id);
    let child_anchor = child_anchors
        .iter()
        .find(|anchor| anchor.anchor_id == attachment.child_anchor_id);

    match parent_anchor {
        Some(anchor) if anchor.allowed_child_kinds.contains(&child.primitive_kind) => {}
        Some(_) => report.push(
            format!("{subject}.parent_anchor_id"),
            "incompatible_attachment_anchor",
            "Parent anchor does not allow this child primitive kind.",
        ),
        None => report.push(
            format!("{subject}.parent_anchor_id"),
            "unknown_parent_anchor",
            "Attachment parent anchor must exist on the parent primitive.",
        ),
    }
    match child_anchor {
        Some(anchor) if child_anchor_can_mount(anchor) => {}
        Some(_) => report.push(
            format!("{subject}.child_anchor_id"),
            "incompatible_child_anchor",
            "Child anchor is not a supported mount point.",
        ),
        None => report.push(
            format!("{subject}.child_anchor_id"),
            "unknown_child_anchor",
            "Attachment child anchor must exist on the child primitive.",
        ),
    }

    validate_offset_policy(report, &subject, &attachment.offset_policy);
}

fn validate_offset_policy(
    report: &mut PrimitiveCompositionValidationReport,
    subject: &str,
    offset_policy: &PrimitiveAttachmentOffsetPolicy,
) {
    match offset_policy {
        PrimitiveAttachmentOffsetPolicy::Fixed => {}
        PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
            x,
            y,
            minimum_x,
            maximum_x,
            minimum_y,
            maximum_y,
        } => {
            validate_normalized_scalar(report, format!("{subject}.offset_policy.x"), *x);
            validate_normalized_scalar(report, format!("{subject}.offset_policy.y"), *y);
            validate_normalized_scalar(
                report,
                format!("{subject}.offset_policy.minimum_x"),
                *minimum_x,
            );
            validate_normalized_scalar(
                report,
                format!("{subject}.offset_policy.maximum_x"),
                *maximum_x,
            );
            validate_normalized_scalar(
                report,
                format!("{subject}.offset_policy.minimum_y"),
                *minimum_y,
            );
            validate_normalized_scalar(
                report,
                format!("{subject}.offset_policy.maximum_y"),
                *maximum_y,
            );
            if minimum_x > maximum_x || minimum_y > maximum_y {
                report.push(
                    format!("{subject}.offset_policy"),
                    "invalid_offset_bounds",
                    "Attachment offset bounds must be ordered.",
                );
            }
            if x < minimum_x || x > maximum_x || y < minimum_y || y > maximum_y {
                report.push(
                    format!("{subject}.offset_policy"),
                    "attachment_offset_out_of_bounds",
                    "Attachment offset must stay inside its bounded range.",
                );
            }
        }
    }
}

fn extend_property_report(
    report: &mut PrimitiveCompositionValidationReport,
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

fn extend_anchor_report(
    report: &mut PrimitiveCompositionValidationReport,
    subject: &str,
    nested: PrimitiveCompositionValidationReport,
) {
    for issue in nested.issues {
        report.push(
            format!("{subject}.anchors.{}", issue.subject),
            issue.code,
            issue.message,
        );
    }
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

fn flat_panel_anchors(node_id: &str) -> Vec<PrimitiveAnchor> {
    vec![
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "front_center",
                display_name: "Front center",
                anchor_kind: PrimitiveAnchorKind::FaceCenter,
                normalized_location: [0.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: vec![
                    PrimitiveKind::SpherePrimitive,
                    PrimitiveKind::BoxPrimitive,
                ],
                product_safe_description: "Attach another primitive to the panel front.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "right_side_handle_zone",
                display_name: "Right handle zone",
                anchor_kind: PrimitiveAnchorKind::SurfacePoint,
                normalized_location: [0.45, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: vec![PrimitiveKind::SpherePrimitive],
                product_safe_description: "Attach a small rounded primitive on the right side of the panel.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "left_side_handle_zone",
                display_name: "Left handle zone",
                anchor_kind: PrimitiveAnchorKind::SurfacePoint,
                normalized_location: [-0.45, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: vec![PrimitiveKind::SpherePrimitive],
                product_safe_description: "Attach a small rounded primitive on the left side of the panel.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "hinge_edge_zone",
                display_name: "Hinge edge zone",
                anchor_kind: PrimitiveAnchorKind::EdgeBand,
                normalized_location: [-1.0, 0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 1.0, 0.0],
                allowed_child_kinds: Vec::new(),
                product_safe_description: "Reserved panel edge zone for later constrained features.",
            },
        ),
    ]
}

fn sphere_anchors(node_id: &str) -> Vec<PrimitiveAnchor> {
    vec![
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "back_mount_point",
                display_name: "Back mount point",
                anchor_kind: PrimitiveAnchorKind::SurfacePoint,
                normalized_location: [0.0, 0.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: Vec::new(),
                product_safe_description: "Use the back of the rounded primitive as a mount point.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "front_center",
                display_name: "Front center",
                anchor_kind: PrimitiveAnchorKind::SurfacePoint,
                normalized_location: [0.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: Vec::new(),
                product_safe_description: "Use the front of the rounded primitive as an inspection point.",
            },
        ),
    ]
}

fn box_anchors(node_id: &str) -> Vec<PrimitiveAnchor> {
    vec![
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "top_center",
                display_name: "Top center",
                anchor_kind: PrimitiveAnchorKind::FaceCenter,
                normalized_location: [0.0, 1.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: vec![
                    PrimitiveKind::BoxPrimitive,
                    PrimitiveKind::FlatPanelPrimitive,
                    PrimitiveKind::SpherePrimitive,
                ],
                product_safe_description: "Attach a primitive to the top of the box.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "front_center",
                display_name: "Front center",
                anchor_kind: PrimitiveAnchorKind::FaceCenter,
                normalized_location: [0.0, 0.0, -1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [1.0, 0.0, 0.0],
                allowed_child_kinds: vec![
                    PrimitiveKind::FlatPanelPrimitive,
                    PrimitiveKind::SpherePrimitive,
                ],
                product_safe_description: "Attach a primitive to the front of the box.",
            },
        ),
        anchor(
            node_id,
            AnchorSpec {
                anchor_id: "side_centers",
                display_name: "Side centers",
                anchor_kind: PrimitiveAnchorKind::Axis,
                normalized_location: [1.0, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 1.0, 0.0],
                allowed_child_kinds: vec![
                    PrimitiveKind::FlatPanelPrimitive,
                    PrimitiveKind::SpherePrimitive,
                ],
                product_safe_description: "Attach a primitive to a side of the box.",
            },
        ),
    ]
}

struct AnchorSpec<'a> {
    anchor_id: &'a str,
    display_name: &'a str,
    anchor_kind: PrimitiveAnchorKind,
    normalized_location: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 3],
    allowed_child_kinds: Vec<PrimitiveKind>,
    product_safe_description: &'a str,
}

fn anchor(node_id: &str, spec: AnchorSpec<'_>) -> PrimitiveAnchor {
    PrimitiveAnchor {
        anchor_id: spec.anchor_id.to_owned(),
        node_id: node_id.to_owned(),
        display_name: spec.display_name.to_owned(),
        anchor_kind: spec.anchor_kind,
        normalized_location: spec.normalized_location,
        normal: spec.normal,
        tangent: spec.tangent,
        allowed_child_kinds: spec.allowed_child_kinds,
        product_safe_description: spec.product_safe_description.to_owned(),
    }
}

fn child_anchor_can_mount(anchor: &PrimitiveAnchor) -> bool {
    matches!(
        anchor.anchor_id.as_str(),
        "back_mount_point" | "front_center"
    )
}

fn validate_identifier(
    report: &mut PrimitiveCompositionValidationReport,
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
            "Stable composition IDs must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_label(
    report: &mut PrimitiveCompositionValidationReport,
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
            "Composition labels and descriptions must be product-safe.",
        );
    }
}

fn validate_normalized_vector(
    report: &mut PrimitiveCompositionValidationReport,
    subject: impl Into<String>,
    vector: [f32; 3],
    bounded: bool,
) {
    let subject = subject.into();
    if vector
        .iter()
        .any(|component| !component.is_finite() || (bounded && !(-1.0..=1.0).contains(component)))
    {
        report.push(
            subject,
            "invalid_normalized_location",
            "Normalized anchor coordinates must be finite values within the approved range.",
        );
    }
}

fn validate_direction_vector(
    report: &mut PrimitiveCompositionValidationReport,
    subject: impl Into<String>,
    vector: [f32; 3],
) {
    let subject = subject.into();
    if vector.iter().any(|component| !component.is_finite()) || vector == [0.0, 0.0, 0.0] {
        report.push(
            subject,
            "invalid_anchor_direction",
            "Anchor direction vectors must be finite and non-zero.",
        );
    }
}

fn validate_normalized_scalar(
    report: &mut PrimitiveCompositionValidationReport,
    subject: impl Into<String>,
    value: f32,
) {
    if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
        report.push(
            subject,
            "invalid_normalized_offset",
            "Attachment offsets must be finite normalized values.",
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
