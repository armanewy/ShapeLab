use shape_asset::{
    ContactPolicy, ExportRealizationPolicy, OrientationPolicy, PlacementPolicy, PositionRule,
    RelationshipContract, RelationshipEditPolicy, RelationshipId, RelationshipType, ResetPolicy,
    ScalePolicy, SelectionPolicy,
};
use shape_compile::export::{
    RelationshipChildOutput, relationship_realization_summaries_for_geometry_export,
};

fn surface_mount_relationship() -> RelationshipContract {
    RelationshipContract {
        id: RelationshipId(1),
        relationship_type: RelationshipType::SurfaceMounted,
        parent: None,
        child: None,
        parent_node_ref: Some("panel".to_owned()),
        child_node_ref: Some("knob".to_owned()),
        parent_anchor_id: Some("front_handle_zone".to_owned()),
        child_anchor_id: Some("back_mount_point".to_owned()),
        label: "Panel with Knob surface mount".to_owned(),
        export_profile: None,
        placement_policy: PlacementPolicy {
            position_rule: PositionRule::ProportionalUv { u: 0.5, v: 0.5 },
        },
        orientation_policy: OrientationPolicy::AlignToSurfaceNormal {
            max_angle_degrees: 0.0,
        },
        scale_policy: ScalePolicy::PreserveChild,
        contact_policy: ContactPolicy::SurfaceContact { clearance: 0.0 },
        edit_policy: RelationshipEditPolicy::Editable,
        selection_policy: SelectionPolicy::Independent,
        reset_policy: ResetPolicy::AuthoredPlacement,
        export_realization: ExportRealizationPolicy::PreserveSemanticSidecar,
    }
}

#[test]
fn export_realization_empty_relationships_report_empty_summaries() {
    let summaries = relationship_realization_summaries_for_geometry_export(&[], "asset.glb#mesh0");

    assert!(summaries.is_empty());
}

#[test]
fn export_realization_reports_combined_mesh_without_bake() {
    let relationship = surface_mount_relationship();

    let summaries =
        relationship_realization_summaries_for_geometry_export(&[relationship], "asset.glb#mesh0");

    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.relationship_id, RelationshipId(1));
    assert_eq!(summary.relationship_type, RelationshipType::SurfaceMounted);
    assert_eq!(
        summary.realization_policy,
        ExportRealizationPolicy::PreserveSemanticSidecar
    );
    assert_eq!(summary.output_node, None);
    assert_eq!(summary.output_mesh.as_deref(), Some("asset.glb#mesh0"));
    assert_eq!(summary.child_output, RelationshipChildOutput::CombinedMesh);
    assert!(!summary.baked);
    assert!(summary.semantics_preserved_in_sidecar);
}
