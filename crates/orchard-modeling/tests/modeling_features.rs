use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    Frame3, OperationId, PartDefinitionId, PartInstanceId, RegionId, SocketId, SocketSpec,
    SurfaceRegionSpec, SurfaceRole,
};
use orchard_modeling::GeneratorContext;
use orchard_modeling::features::{
    FastenerPattern, FastenerPlacement, FastenerPrototype, FeatureError, FeatureHost,
    PANEL_BACK_REGION, PANEL_BORDER_REGION, PANEL_FRONT_REGION, PANEL_SIDE_REGION, PanelFeature,
    PanelVisualMode, PatternSpacing, PlanarHost, RIB_BACK_REGION, RIB_EDGE_REGION,
    RIB_FRONT_REGION, RibBuildMode, RibFeature, RibProfile, TRIM_BODY_REGION, TRIM_END_CAP_REGION,
    TRIM_START_CAP_REGION, TrimFeature, TrimPath, build_fastener_pattern, build_panel_feature,
    build_rib_feature, build_trim_feature,
};
use orchard_poly::{PolygonMesh, triangulate_polygon_mesh, validate_polygon_mesh};

const PANEL_OPERATION: OperationId = OperationId(41);
const TRIM_OPERATION: OperationId = OperationId(42);
const RIB_OPERATION: OperationId = OperationId(43);
const FASTENER_OPERATION: OperationId = OperationId(44);

#[test]
fn raised_panel_generates_separate_semantic_regions() {
    let feature = panel(PanelVisualMode::Raised, 0.1, 0.12);
    let part = build_panel_feature(&feature, &host_with_socket(SocketId(7)), &context())
        .expect("raised panel should generate");

    assert_valid_feature_mesh(&part.mesh, PANEL_OPERATION);
    assert_region_faces(
        &part.mesh,
        &[
            PANEL_FRONT_REGION,
            PANEL_BORDER_REGION,
            PANEL_SIDE_REGION,
            PANEL_BACK_REGION,
        ],
    );
    assert_eq!(part.regions[&PANEL_FRONT_REGION].name, "panel_front");
    assert_close(part.mesh.bounds.min[2], 0.0);
    assert_close(part.mesh.bounds.max[2], 0.12);
}

#[test]
fn recessed_visual_panel_extends_inward_without_fusing_host() {
    let feature = panel(PanelVisualMode::Recessed, 0.1, 0.2);
    let part = build_panel_feature(&feature, &host_with_socket(SocketId(7)), &context())
        .expect("recessed panel should generate");

    assert_valid_feature_mesh(&part.mesh, PANEL_OPERATION);
    assert_region_faces(&part.mesh, &[PANEL_FRONT_REGION, PANEL_BACK_REGION]);
    assert_close(part.mesh.bounds.min[2], -0.2);
    assert_close(part.mesh.bounds.max[2], 0.0);
}

#[test]
fn rounded_panel_has_stable_semantic_regions() {
    let rounded = panel(PanelVisualMode::Raised, 0.25, 0.08);
    let square = panel(PanelVisualMode::Raised, 0.0, 0.08);

    let rounded_part = build_panel_feature(&rounded, &host_with_socket(SocketId(7)), &context())
        .expect("rounded panel should generate");
    let square_part = build_panel_feature(&square, &host_with_socket(SocketId(7)), &context())
        .expect("square panel should generate");

    assert!(rounded_part.mesh.positions.len() > square_part.mesh.positions.len());
    assert_eq!(
        rounded_part.regions.keys().copied().collect::<Vec<_>>(),
        square_part.regions.keys().copied().collect::<Vec<_>>()
    );
}

#[test]
fn panel_can_attach_to_named_planar_surface_region() {
    let mut regions = BTreeMap::new();
    regions.insert(
        RegionId(30),
        SurfaceRegionSpec {
            id: RegionId(30),
            name: "door_face".to_owned(),
            role: SurfaceRole::PrimarySurface,
            tags: BTreeSet::new(),
        },
    );
    let host = FeatureHost {
        sockets: BTreeMap::new(),
        regions,
        region_frames: BTreeMap::from([(
            RegionId(30),
            Frame3 {
                origin: [0.0, 0.0, 1.0],
                ..Frame3::default()
            },
        )]),
    };
    let mut feature = panel(PanelVisualMode::Raised, 0.1, 0.12);
    feature.host = PlanarHost::SurfaceRegionName("door_face".to_owned());

    let part = build_panel_feature(&feature, &host, &context())
        .expect("panel should attach to planar region frame");

    assert_valid_feature_mesh(&part.mesh, PANEL_OPERATION);
    assert_close(part.mesh.bounds.min[2], 1.0);
    assert_close(part.mesh.bounds.max[2], 1.12);
}

#[test]
fn trim_around_rectangular_edge_loop_preserves_trim_provenance() {
    let feature = TrimFeature {
        operation: TRIM_OPERATION,
        path: TrimPath::EdgeLoop {
            points: vec![
                [-1.0, 0.0, -1.0],
                [1.0, 0.0, -1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
        },
        profile: square_profile(0.08),
        offset: [0.0, 0.05, 0.0],
        profile_offset: [0.0, 0.0],
        up_hint: [0.0, 1.0, 0.0],
        roll_degrees: 0.0,
        start_cap: false,
        end_cap: false,
    };

    let part = build_trim_feature(&feature, &context()).expect("rectangular trim should generate");

    assert_valid_feature_mesh(&part.mesh, TRIM_OPERATION);
    assert_region_faces(&part.mesh, &[TRIM_BODY_REGION]);
    assert_eq!(part.regions.len(), 1);
    assert_eq!(part.mesh.faces.len(), 16);
}

#[test]
fn curved_trim_path_uses_start_and_end_cap_controls() {
    let feature = TrimFeature {
        operation: TRIM_OPERATION,
        path: TrimPath::AuthoredPath {
            points: vec![
                [-1.0, 0.0, 0.0],
                [-0.25, 0.0, 0.5],
                [0.5, 0.0, 0.45],
                [1.0, 0.0, 0.0],
            ],
            closed: false,
        },
        profile: square_profile(0.06),
        offset: [0.0, 0.0, 0.0],
        profile_offset: [0.02, 0.0],
        up_hint: [0.0, 1.0, 0.0],
        roll_degrees: 15.0,
        start_cap: true,
        end_cap: true,
    };

    let part = build_trim_feature(&feature, &context()).expect("curved trim should generate");

    assert_valid_feature_mesh(&part.mesh, TRIM_OPERATION);
    assert_region_faces(
        &part.mesh,
        &[TRIM_BODY_REGION, TRIM_START_CAP_REGION, TRIM_END_CAP_REGION],
    );
    assert!(part.mesh.bounds.max[0] > 0.9);
}

#[test]
fn ribs_can_generate_instances_or_combined_part() {
    let feature = RibFeature {
        operation: RIB_OPERATION,
        start: [-1.5, 0.0, 0.0],
        end: [1.5, 0.0, 0.0],
        count: 4,
        spacing: PatternSpacing::Fit,
        up_hint: [0.0, 0.0, 1.0],
        profile: RibProfile::Plate {
            width: 0.4,
            height: 0.8,
            thickness: 0.08,
        },
        mode: RibBuildMode::SeparateInstances,
    };

    let build = build_rib_feature(&feature, &context()).expect("ribs should generate");

    assert_eq!(build.definitions.len(), 1);
    assert_eq!(build.instances.len(), 4);
    assert!(build.combined_part.is_none());
    assert_region_faces(
        &build.generated_parts[&PartDefinitionId(7)].mesh,
        &[RIB_FRONT_REGION, RIB_BACK_REGION, RIB_EDGE_REGION],
    );
    assert!(
        build
            .instances
            .values()
            .all(|instance| instance.part_instance.definition == PartDefinitionId(7))
    );

    let mut combined_feature = feature;
    combined_feature.mode = RibBuildMode::CombinedPart;
    let combined = build_rib_feature(&combined_feature, &context()).expect("combined ribs");
    let combined_part = combined
        .combined_part
        .as_ref()
        .expect("combined mode should produce one part");
    assert_valid_feature_mesh(&combined_part.mesh, RIB_OPERATION);
    assert_eq!(
        combined_part.mesh.faces.len(),
        combined.generated_parts[&PartDefinitionId(7)]
            .mesh
            .faces
            .len()
            * 4
    );
}

#[test]
fn repeated_fasteners_share_one_definition_with_many_instances() {
    let feature = FastenerPattern {
        operation: FASTENER_OPERATION,
        prototype: FastenerPrototype::Cylinder {
            radius: 0.08,
            height: 0.18,
            radial_segments: 12,
        },
        placement: FastenerPlacement::Linear {
            start: [-1.0, 0.0, 0.0],
            end: [1.0, 0.0, 0.0],
            count: 5,
            spacing: PatternSpacing::Fit,
            up_hint: [0.0, 1.0, 0.0],
        },
    };

    let build = build_fastener_pattern(&feature, &context()).expect("fasteners should generate");

    assert_eq!(build.definitions.len(), 1);
    assert_eq!(build.generated_parts.len(), 1);
    assert_eq!(build.instances.len(), 5);
    assert!(
        build
            .instances
            .values()
            .all(|instance| instance.part_instance.definition == PartDefinitionId(7))
    );
    assert_valid_feature_mesh(
        &build.generated_parts[&PartDefinitionId(7)].mesh,
        FASTENER_OPERATION,
    );
    assert_eq!(
        build.provenance.instance_ids,
        vec![
            PartInstanceId(11),
            PartInstanceId(12),
            PartInstanceId(13),
            PartInstanceId(14),
            PartInstanceId(15)
        ]
    );
}

#[test]
fn invalid_host_socket_reports_validation_error() {
    let feature = panel(PanelVisualMode::Raised, 0.1, 0.12);
    let error = build_panel_feature(&feature, &FeatureHost::default(), &context())
        .expect_err("missing socket should fail");

    assert!(matches!(
        error,
        FeatureError::Validation {
            feature: "panel",
            ..
        }
    ));
}

#[test]
fn feature_generation_is_deterministic_for_same_parameters() {
    let feature = panel(PanelVisualMode::Raised, 0.2, 0.1);

    let first = build_panel_feature(&feature, &host_with_socket(SocketId(7)), &context())
        .expect("first panel");
    let second = build_panel_feature(&feature, &host_with_socket(SocketId(7)), &context())
        .expect("second panel");

    assert_eq!(
        first.mesh.topology_signature,
        second.mesh.topology_signature
    );
    assert_eq!(first.mesh.vertex_ids, second.mesh.vertex_ids);
    assert_eq!(
        first
            .mesh
            .faces
            .iter()
            .map(|face| face.id)
            .collect::<Vec<_>>(),
        second
            .mesh
            .faces
            .iter()
            .map(|face| face.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn parameter_changes_preserve_semantic_feature_ids() {
    let first = panel(PanelVisualMode::Raised, 0.2, 0.1);
    let mut changed = first.clone();
    changed.width = 2.2;
    changed.depth = 0.18;

    let first_part = build_panel_feature(&first, &host_with_socket(SocketId(7)), &context())
        .expect("first panel");
    let changed_part = build_panel_feature(&changed, &host_with_socket(SocketId(7)), &context())
        .expect("changed panel");

    assert_eq!(
        first_part.regions.keys().copied().collect::<Vec<_>>(),
        changed_part.regions.keys().copied().collect::<Vec<_>>()
    );
    assert!(
        changed_part
            .mesh
            .face_metadata
            .iter()
            .all(|metadata| metadata.operation == Some(PANEL_OPERATION))
    );
}

#[test]
fn triangulation_and_validation_succeed_for_generated_features() {
    let panel = build_panel_feature(
        &panel(PanelVisualMode::Raised, 0.2, 0.1),
        &host_with_socket(SocketId(7)),
        &context(),
    )
    .expect("panel");
    let trim = build_trim_feature(
        &TrimFeature {
            operation: TRIM_OPERATION,
            path: TrimPath::AuthoredPath {
                points: vec![[0.0, 0.0, 0.0], [0.8, 0.0, 0.2], [1.4, 0.0, 0.0]],
                closed: false,
            },
            profile: square_profile(0.05),
            offset: [0.0, 0.0, 0.0],
            profile_offset: [0.0, 0.0],
            up_hint: [0.0, 1.0, 0.0],
            roll_degrees: 0.0,
            start_cap: true,
            end_cap: true,
        },
        &context(),
    )
    .expect("trim");

    for mesh in [&panel.mesh, &trim.mesh] {
        assert!(validate_polygon_mesh(mesh).is_valid());
        let triangulated = triangulate_polygon_mesh(mesh).expect("triangulate feature mesh");
        assert_eq!(triangulated.mesh.indices.len() % 3, 0);
        assert!(
            triangulated
                .triangle_to_operation
                .iter()
                .all(Option::is_some)
        );
    }
}

fn context() -> GeneratorContext {
    GeneratorContext::new(PartDefinitionId(7), PartInstanceId(11), 100, 0)
}

fn panel(mode: PanelVisualMode, corner_radius: f32, depth: f32) -> PanelFeature {
    PanelFeature {
        operation: PANEL_OPERATION,
        host: PlanarHost::Socket(SocketId(7)),
        width: 2.0,
        height: 1.2,
        depth,
        corner_radius,
        border_width: 0.18,
        mode,
    }
}

fn host_with_socket(socket: SocketId) -> FeatureHost {
    let mut sockets = BTreeMap::new();
    sockets.insert(
        socket,
        SocketSpec {
            id: socket,
            name: "front_panel_socket".to_owned(),
            local_frame: Frame3::default(),
            role: "attachment".to_owned(),
            tags: BTreeSet::new(),
        },
    );
    FeatureHost {
        sockets,
        regions: BTreeMap::new(),
        region_frames: BTreeMap::new(),
    }
}

fn square_profile(half: f32) -> Vec<[f32; 2]> {
    vec![[half, half], [-half, half], [-half, -half], [half, -half]]
}

fn assert_valid_feature_mesh(mesh: &PolygonMesh, operation: OperationId) {
    assert!(
        validate_polygon_mesh(mesh).is_valid(),
        "feature mesh should pass polygon validation"
    );
    assert!(
        mesh.face_metadata.iter().all(|metadata| {
            metadata.region.is_some()
                && metadata.operation == Some(operation)
                && metadata.surface_role.is_some()
        }),
        "every face should carry region, operation, and role provenance"
    );
    let triangulated = triangulate_polygon_mesh(mesh).expect("feature mesh should triangulate");
    assert_eq!(triangulated.mesh.indices.len() % 3, 0);
}

fn assert_region_faces(mesh: &PolygonMesh, expected_regions: &[RegionId]) {
    let mut counts = BTreeMap::<RegionId, usize>::new();
    for metadata in &mesh.face_metadata {
        if let Some(region) = metadata.region {
            *counts.entry(region).or_default() += 1;
        }
    }
    for region in expected_regions {
        assert!(
            counts.get(region).copied().unwrap_or_default() > 0,
            "expected region {region:?} to own generated faces"
        );
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-4,
        "expected {expected}, got {actual}"
    );
}
