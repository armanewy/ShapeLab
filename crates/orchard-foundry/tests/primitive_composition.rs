use orchard_foundry::{
    PrimitiveAnchor, PrimitiveAttachment, PrimitiveAttachmentOffsetPolicy,
    PrimitiveAttachmentOrientationPolicy, PrimitiveAttachmentScalePolicy,
    PrimitiveCompositionDocument, PrimitiveCompositionValidationReport, PrimitiveKind,
    PrimitiveNode, PrimitiveNodeVisibility, flat_panel_primitive_property_schema,
    primitive_anchor_definitions, primitive_default_property_values,
    sphere_primitive_property_schema, validate_primitive_anchor,
    validate_primitive_composition_document,
};

#[test]
fn primitive_composition_document_validates() {
    let document = panel_with_sphere_document();

    let report = validate_primitive_composition_document(&document);

    assert_valid(&report);
}

#[test]
fn primitive_composition_sphere_can_attach_to_flat_panel_handle_zone() {
    let document = panel_with_sphere_document();

    let report = validate_primitive_composition_document(&document);

    assert_valid(&report);
    assert_eq!(
        document.attachments[0].parent_anchor_id,
        "front_handle_zone"
    );
    assert_eq!(document.attachments[0].child_anchor_id, "back_mount_point");
}

#[test]
fn primitive_composition_sphere_cannot_attach_to_invalid_anchor() {
    let mut document = panel_with_sphere_document();
    document.attachments[0].parent_anchor_id = "hinge_edge_zone".to_owned();

    let report = validate_primitive_composition_document(&document);

    assert_issue(&report, "incompatible_attachment_anchor");
}

#[test]
fn primitive_composition_raw_free_transform_rejected() {
    let document = panel_with_sphere_document();
    let mut value = serde_json::to_value(&document).expect("document serializes");
    value["attachments"][0]
        .as_object_mut()
        .expect("attachment is an object")
        .insert(
            "raw_matrix".to_owned(),
            serde_json::json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]),
        );

    let decoded = serde_json::from_value::<PrimitiveCompositionDocument>(value);

    assert!(
        decoded.is_err(),
        "raw transform payload should be rejected by the closed document schema"
    );
}

#[test]
fn primitive_composition_invalid_normalized_location_rejected() {
    let mut anchor = flat_panel_anchor("front_handle_zone");
    anchor.normalized_location = [1.25, 0.0, -1.0];

    let report = validate_primitive_anchor(&anchor);

    assert_issue(&report, "invalid_normalized_location");
}

#[test]
fn primitive_composition_absolute_paths_are_not_product_safe() {
    let mut document = panel_with_sphere_document();
    document.nodes[0].local_label = "/Users/arman/panel".to_owned();

    let report = validate_primitive_composition_document(&document);

    assert_issue(&report, "invalid_composition_node_label");
}

#[test]
fn primitive_composition_blender_like_user_copy_rejected() {
    let mut document = panel_with_sphere_document();
    document.nodes[0].local_label = "Vertex mesh controls".to_owned();

    let report = validate_primitive_composition_document(&document);

    assert_issue(&report, "invalid_composition_node_label");
}

#[test]
fn primitive_composition_serde_roundtrip_is_deterministic() {
    let document = panel_with_sphere_document();

    let first = serde_json::to_string(&document).expect("document serializes");
    let decoded =
        serde_json::from_str::<PrimitiveCompositionDocument>(&first).expect("document decodes");
    let second = serde_json::to_string(&decoded).expect("document serializes again");

    assert_eq!(first, second);
    assert_eq!(document, decoded);
}

fn panel_with_sphere_document() -> PrimitiveCompositionDocument {
    PrimitiveCompositionDocument {
        schema_version: orchard_foundry::PRIMITIVE_COMPOSITION_SCHEMA_VERSION,
        document_id: "panel_with_sphere".to_owned(),
        nodes: vec![flat_panel_node(), sphere_node()],
        attachments: vec![PrimitiveAttachment {
            attachment_id: "panel_knob_mount".to_owned(),
            parent_node_id: "panel".to_owned(),
            parent_anchor_id: "front_handle_zone".to_owned(),
            child_node_id: "knob".to_owned(),
            child_anchor_id: "back_mount_point".to_owned(),
            offset_policy: PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
                x: 0.25,
                y: 0.0,
                minimum_x: -0.6,
                maximum_x: 0.6,
                minimum_y: -0.5,
                maximum_y: 0.5,
            },
            orientation_policy: PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal,
            scale_policy: PrimitiveAttachmentScalePolicy::KeepChildScale,
        }],
        root_node_id: "panel".to_owned(),
    }
}

fn flat_panel_node() -> PrimitiveNode {
    let schema = flat_panel_primitive_property_schema();
    PrimitiveNode {
        node_id: "panel".to_owned(),
        primitive_kind: PrimitiveKind::FlatPanelPrimitive,
        property_values: primitive_default_property_values(&schema),
        local_label: "Panel".to_owned(),
        visibility: PrimitiveNodeVisibility::Visible,
    }
}

fn sphere_node() -> PrimitiveNode {
    let schema = sphere_primitive_property_schema();
    PrimitiveNode {
        node_id: "knob".to_owned(),
        primitive_kind: PrimitiveKind::SpherePrimitive,
        property_values: primitive_default_property_values(&schema),
        local_label: "Knob-like form".to_owned(),
        visibility: PrimitiveNodeVisibility::Visible,
    }
}

fn flat_panel_anchor(anchor_id: &str) -> PrimitiveAnchor {
    primitive_anchor_definitions(PrimitiveKind::FlatPanelPrimitive, "panel")
        .into_iter()
        .find(|anchor| anchor.anchor_id == anchor_id)
        .expect("flat panel anchor exists")
}

fn assert_valid(report: &PrimitiveCompositionValidationReport) {
    assert!(
        report.is_valid(),
        "expected valid composition, got {:?}",
        report.issues
    );
}

fn assert_issue(report: &PrimitiveCompositionValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "expected issue {expected_code}, got {:?}",
        report.issues
    );
}
