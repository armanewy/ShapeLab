use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    BoundaryLoopId, CutEdgeTreatment, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, OperationId, PartDefinition, PartDefinitionId, PartInstanceId,
    PlanarCutFace, RegionId, SocketId, SurfaceRole,
};
use shape_modeling::generators::basic::{
    CapMode, CylinderParams, FaceMask, FrustumParams, PlateParams, RoundedBoxParams,
    build_cylinder, build_frustum, build_plate, build_rounded_box, generate_plate,
    generate_rounded_box,
};
use shape_modeling::{GeneratedPart, GeneratorContext, ModelingError};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, PolygonMesh, build_adjacency, compute_face_normals,
    compute_split_vertex_normals, triangulate_polygon_mesh, validate_polygon_mesh,
};

const EPSILON: f32 = 1.0e-5;

#[test]
fn rounded_box_closed_topology_is_stable_and_semantic() {
    let params = RoundedBoxParams {
        half_extents: [1.0, 0.75, 0.5],
        bevel_radius: 0.2,
        bevel_segments: 2,
        face_subdivisions: 2,
        face_mask: FaceMask::all(),
    };
    let part = build_rounded_box(&params, &context()).expect("rounded box should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(&part, &["primary_faces", "bevel_bands", "corners"]);
    assert_faces_use_regions(&part, &[RegionId(1), RegionId(2), RegionId(3)]);
    assert_eq!(part.sockets.len(), 6);
    assert_socket_origin(&part, SocketId(1), [1.0, 0.0, 0.0]);
    assert_socket_origin(&part, SocketId(3), [0.0, 0.75, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -0.75, -0.5], [1.0, 0.75, 0.5]);

    let repeated = build_rounded_box(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.bevel_radius = 0.15;
    let scalar_part =
        build_rounded_box(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.bevel_segments = 3;
    let topology_part =
        build_rounded_box(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );
}

#[test]
fn rounded_box_open_mask_reports_open_boundaries() {
    let params = RoundedBoxParams {
        half_extents: [1.0, 1.0, 1.0],
        bevel_radius: 0.15,
        bevel_segments: 2,
        face_subdivisions: 1,
        face_mask: FaceMask {
            positive_y: false,
            ..FaceMask::all()
        },
    };
    let part = build_rounded_box(&params, &context()).expect("open rounded box should generate");

    assert_valid_with_open_boundaries(&part.mesh, 20);
    assert_common_mesh_quality(&part.mesh);
}

#[test]
fn cylinder_closed_and_open_modes_are_indexed_and_semantic() {
    let params = CylinderParams {
        radius: 1.0,
        half_height: 1.25,
        radial_segments: 12,
        height_segments: 2,
        cap_mode: CapMode::Both,
        top_bevel_radius: 0.12,
        bottom_bevel_radius: 0.12,
        bevel_segments: 2,
    };
    let part = build_cylinder(&params, &context()).expect("cylinder should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(
        &part,
        &["side", "top_cap", "bottom_cap", "top_bevel", "bottom_bevel"],
    );
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(4),
            RegionId(5),
        ],
    );
    assert_socket_origin(&part, SocketId(1), [0.0, 1.25, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -1.25, 0.0]);
    assert_socket_origin(&part, SocketId(3), [0.0, 0.0, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -1.25, -1.0], [1.0, 1.25, 1.0]);

    let repeated = build_cylinder(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.radius = 1.1;
    let scalar_part =
        build_cylinder(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.radial_segments = 16;
    let topology_part =
        build_cylinder(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );

    let open = CylinderParams {
        cap_mode: CapMode::None,
        top_bevel_radius: 0.0,
        bottom_bevel_radius: 0.0,
        bevel_segments: 0,
        radial_segments: 8,
        height_segments: 1,
        ..scalar_change
    };
    let open_part = build_cylinder(&open, &context()).expect("open cylinder should generate");
    assert_valid_with_open_boundaries(&open_part.mesh, 16);
}

#[test]
fn frustum_closed_and_open_modes_preserve_regions() {
    let params = FrustumParams {
        bottom_radius: 1.0,
        top_radius: 0.45,
        half_height: 1.0,
        radial_segments: 12,
        height_segments: 3,
        cap_mode: CapMode::Both,
        top_bevel_radius: 0.08,
        bottom_bevel_radius: 0.1,
        bevel_segments: 2,
    };
    let part = build_frustum(&params, &context()).expect("frustum should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(4),
            RegionId(5),
        ],
    );
    assert_socket_origin(&part, SocketId(1), [0.0, 1.0, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -1.0, 0.0]);
    assert_bounds(&part.mesh, [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

    let repeated = build_frustum(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.top_radius = 0.6;
    let scalar_part =
        build_frustum(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.height_segments = 4;
    let topology_part =
        build_frustum(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );

    let open = FrustumParams {
        cap_mode: CapMode::Bottom,
        top_bevel_radius: 0.0,
        bottom_bevel_radius: 0.0,
        bevel_segments: 0,
        radial_segments: 12,
        height_segments: 1,
        ..scalar_change
    };
    let open_part = build_frustum(&open, &context()).expect("open frustum should generate");
    assert_valid_with_open_boundaries(&open_part.mesh, 12);
}

#[test]
fn plate_is_closed_rounded_and_semantic() {
    let params = PlateParams {
        width: 3.0,
        height: 2.0,
        thickness: 0.25,
        corner_radius: 0.25,
        corner_segments: 3,
        front_back_bevel: 0.05,
    };
    let part = build_plate(&params, &context()).expect("plate should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_names(&part, &["front", "back", "side", "bevel"]);
    assert_faces_use_regions(&part, &[RegionId(1), RegionId(2), RegionId(3), RegionId(4)]);
    assert_socket_origin(&part, SocketId(1), [0.0, 0.125, 0.0]);
    assert_socket_origin(&part, SocketId(2), [0.0, -0.125, 0.0]);
    assert_bounds(&part.mesh, [-1.5, -0.125, -1.0], [1.5, 0.125, 1.0]);

    let repeated = build_plate(&params, &context()).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);

    let mut scalar_change = params.clone();
    scalar_change.thickness = 0.35;
    let scalar_part =
        build_plate(&scalar_change, &context()).expect("scalar change should generate");
    assert_same_region_ids(&part, &scalar_part);
    assert_eq!(
        part.mesh.topology_signature,
        scalar_part.mesh.topology_signature
    );

    let mut topology_change = params;
    topology_change.corner_segments = 4;
    let topology_part =
        build_plate(&topology_change, &context()).expect("topology change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        topology_part.mesh.topology_signature
    );
}

#[test]
fn plate_recessed_panel_cut_is_closed_semantic_and_loop_tagged() {
    let operation = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(30),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [1.45, 0.72],
        depth: 0.08,
        corner_radius: 0.12,
        rim_width: 0.1152,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(7),
        floor_loop: BoundaryLoopId(8),
        outer_region: RegionId(1),
        rim_region: RegionId(20),
        wall_region: RegionId(21),
        floor_region: RegionId(22),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let part = generate_cut_plate(operation).expect("recessed panel cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(20),
            RegionId(21),
            RegionId(22),
        ],
    );
    assert_region_role(&part, RegionId(20), SurfaceRole::Rim);
    assert_region_role(&part, RegionId(21), SurfaceRole::CutWall);
    assert_region_role(&part, RegionId(22), SurfaceRole::Interior);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(7), OperationId(30), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(8), OperationId(30), true);
    assert_face_operation_present(&part.mesh, OperationId(30));
}

#[test]
fn plate_cut_rim_width_is_shape_only_and_corner_segments_change_topology() {
    let operation = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(36),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [1.45, 0.72],
        depth: 0.08,
        corner_radius: 0.12,
        rim_width: 0.10,
        corner_segments: 3,
        entry_loop: BoundaryLoopId(19),
        floor_loop: BoundaryLoopId(20),
        outer_region: RegionId(1),
        rim_region: RegionId(20),
        wall_region: RegionId(21),
        floor_region: RegionId(22),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let part = generate_cut_plate(operation.clone()).expect("cut should generate");

    let mut rim_change = operation.clone();
    if let ModelingOperationSpec::RecessedPanelCut { rim_width, .. } = &mut rim_change {
        *rim_width = 0.12;
    }
    let rim_part = generate_cut_plate(rim_change).expect("rim-width change should generate");
    assert_eq!(
        part.mesh.topology_signature,
        rim_part.mesh.topology_signature
    );
    assert_ne!(part.mesh.positions, rim_part.mesh.positions);

    let mut segment_change = operation;
    if let ModelingOperationSpec::RecessedPanelCut {
        corner_segments, ..
    } = &mut segment_change
    {
        *corner_segments = 5;
    }
    let segment_part =
        generate_cut_plate(segment_change).expect("corner segment change should generate");
    assert_ne!(
        part.mesh.topology_signature,
        segment_part.mesh.topology_signature
    );
}

#[test]
fn plate_rectangular_through_cut_is_closed_semantic_and_loop_tagged() {
    let operation = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::Hard,
    };
    let part = generate_cut_plate(operation).expect("rectangular through cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(23),
            RegionId(24),
        ],
    );
    assert_region_role(&part, RegionId(23), SurfaceRole::Rim);
    assert_region_role(&part, RegionId(24), SurfaceRole::CutWall);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(9), OperationId(31), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(10), OperationId(31), false);
    assert_face_operation_present(&part.mesh, OperationId(31));
}

#[test]
fn plate_boundary_loop_bevel_consumes_source_and_emits_replacements() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(40),
        target_loop: BoundaryLoopId(9),
        width: 0.035,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(27),
        outer_replacement_loop: BoundaryLoopId(30),
        inner_replacement_loop: BoundaryLoopId(31),
    };
    let part = generate_cut_plate_with_operations(vec![cut, bevel], [3.0, 2.0], 0.30)
        .expect("beveled cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_role(&part, RegionId(27), SurfaceRole::BevelBand);
    assert_no_boundary_loop(&part.mesh, BoundaryLoopId(9));
    assert_boundary_loop(&part.mesh, BoundaryLoopId(10), OperationId(31), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(30), OperationId(40), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(31), OperationId(40), false);
    assert_face_operation_present(&part.mesh, OperationId(31));
    assert_face_operation_present(&part.mesh, OperationId(40));
}

#[test]
fn plate_boundary_loop_bevel_profile_curves_geometry_and_smooths_band() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let linear = generate_cut_plate_with_operations(
        vec![
            cut.clone(),
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(40),
                target_loop: BoundaryLoopId(9),
                width: 0.035,
                segments: 3,
                profile: 1.0,
                bevel_region: RegionId(27),
                outer_replacement_loop: BoundaryLoopId(30),
                inner_replacement_loop: BoundaryLoopId(31),
            },
        ],
        [3.0, 2.0],
        0.30,
    )
    .expect("linear bevel should generate");
    let curved = generate_cut_plate_with_operations(
        vec![
            cut,
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(40),
                target_loop: BoundaryLoopId(9),
                width: 0.035,
                segments: 3,
                profile: 2.0,
                bevel_region: RegionId(27),
                outer_replacement_loop: BoundaryLoopId(30),
                inner_replacement_loop: BoundaryLoopId(31),
            },
        ],
        [3.0, 2.0],
        0.30,
    )
    .expect("curved bevel should generate");

    assert_ne!(
        operation_y_samples(&linear.mesh, OperationId(40)),
        operation_y_samples(&curved.mesh, OperationId(40)),
        "profile should change bevel-band depth samples, not only vertex spacing"
    );
    let smoothing_groups = operation_smoothing_groups(&curved.mesh, OperationId(40));
    assert_eq!(
        smoothing_groups.len(),
        1,
        "bevel band should use one smoothing group"
    );
    assert!(
        smoothing_groups.iter().all(Option::is_some),
        "bevel band faces should carry smoothing metadata"
    );
    assert!(
        bevel_internal_smooth_edge_count(&curved.mesh, OperationId(40), RegionId(27)) > 0,
        "multi-segment bevel should smooth internal band edges"
    );
    assert_boundary_loop(&curved.mesh, BoundaryLoopId(30), OperationId(40), false);
    assert_boundary_loop(&curved.mesh, BoundaryLoopId(31), OperationId(40), false);
}

#[test]
fn plate_boundary_loop_bevel_profile_affects_two_segment_midpoint() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let shallow = generate_cut_plate_with_operations(
        vec![
            cut.clone(),
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(40),
                target_loop: BoundaryLoopId(9),
                width: 0.035,
                segments: 2,
                profile: 0.5,
                bevel_region: RegionId(27),
                outer_replacement_loop: BoundaryLoopId(30),
                inner_replacement_loop: BoundaryLoopId(31),
            },
        ],
        [3.0, 2.0],
        0.30,
    )
    .expect("shallow two-segment bevel should generate");
    let rounded = generate_cut_plate_with_operations(
        vec![
            cut,
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(40),
                target_loop: BoundaryLoopId(9),
                width: 0.035,
                segments: 2,
                profile: 2.0,
                bevel_region: RegionId(27),
                outer_replacement_loop: BoundaryLoopId(30),
                inner_replacement_loop: BoundaryLoopId(31),
            },
        ],
        [3.0, 2.0],
        0.30,
    )
    .expect("rounded two-segment bevel should generate");

    assert_ne!(
        operation_y_samples(&shallow.mesh, OperationId(40)),
        operation_y_samples(&rounded.mesh, OperationId(40)),
        "profile should move the intermediate depth sample even with two bevel segments"
    );
}

#[test]
fn plate_boundary_loop_bevel_profile_one_remains_linear() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let part = generate_cut_plate_with_operations(
        vec![
            cut,
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(40),
                target_loop: BoundaryLoopId(9),
                width: 0.035,
                segments: 2,
                profile: 1.0,
                bevel_region: RegionId(27),
                outer_replacement_loop: BoundaryLoopId(30),
                inner_replacement_loop: BoundaryLoopId(31),
            },
        ],
        [3.0, 2.0],
        0.30,
    )
    .expect("linear two-segment bevel should generate");

    let samples = operation_y_samples(&part.mesh, OperationId(40));
    assert!(
        samples.contains(&quantize(0.1325)),
        "profile 1.0 should keep the midpoint depth linear; samples were {samples:?}"
    );
}

#[test]
fn rounded_rect_boundary_loop_bevel_uses_uniform_offset_extents() {
    let center = [0.08, -0.05];
    let size = [1.60, 0.40];
    let bevel_width = 0.07;
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center,
        size,
        corner_radius: 0.08,
        rim_width: 0.10,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(40),
        target_loop: BoundaryLoopId(9),
        width: bevel_width,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(27),
        outer_replacement_loop: BoundaryLoopId(30),
        inner_replacement_loop: BoundaryLoopId(31),
    };
    let part = generate_cut_plate_with_operations(vec![cut, bevel], [3.0, 2.0], 0.30)
        .expect("beveled rounded-rect cut should generate");

    let (min_x, max_x, min_z, max_z) = boundary_loop_xz_bounds(&part.mesh, BoundaryLoopId(30));
    assert_close(min_x, center[0] - size[0] * 0.5 - bevel_width);
    assert_close(max_x, center[0] + size[0] * 0.5 + bevel_width);
    assert_close(min_z, center[1] - size[1] * 0.5 - bevel_width);
    assert_close(max_z, center[1] + size[1] * 0.5 + bevel_width);
}

#[test]
fn plate_boundary_loop_bevel_rejects_hard_only_loop() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::Hard,
    };
    let bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(40),
        target_loop: BoundaryLoopId(9),
        width: 0.035,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(27),
        outer_replacement_loop: BoundaryLoopId(30),
        inner_replacement_loop: BoundaryLoopId(31),
    };

    assert!(generate_cut_plate_with_operations(vec![cut, bevel], [3.0, 2.0], 0.30).is_err());
}

#[test]
fn recessed_boundary_loop_bevels_share_depth_budget() {
    let cut = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        depth: 0.08,
        corner_radius: 0.08,
        rim_width: 0.0928,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        floor_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        floor_region: RegionId(25),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let entry_bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(40),
        target_loop: BoundaryLoopId(9),
        width: 0.06,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(27),
        outer_replacement_loop: BoundaryLoopId(30),
        inner_replacement_loop: BoundaryLoopId(31),
    };
    let floor_bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(41),
        target_loop: BoundaryLoopId(10),
        width: 0.06,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(28),
        outer_replacement_loop: BoundaryLoopId(32),
        inner_replacement_loop: BoundaryLoopId(33),
    };

    for (operations, rejected_operation) in [
        (
            vec![cut.clone(), entry_bevel.clone(), floor_bevel.clone()],
            OperationId(41),
        ),
        (vec![cut.clone(), floor_bevel, entry_bevel], OperationId(40)),
    ] {
        let error = generate_cut_plate_with_operations(operations, [3.0, 2.0], 0.30)
            .expect_err("opposing recessed bevels should reject when they consume full depth");
        assert_unsupported_operation(&error, rejected_operation, "opposing recessed bevels");
    }
}

#[test]
fn through_cut_boundary_loop_bevels_share_depth_budget() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.4, 1.0],
        corner_radius: 0.08,
        rim_width: 0.22,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        exit_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let entry_bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(40),
        target_loop: BoundaryLoopId(9),
        width: 0.18,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(27),
        outer_replacement_loop: BoundaryLoopId(30),
        inner_replacement_loop: BoundaryLoopId(31),
    };
    let exit_bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(41),
        target_loop: BoundaryLoopId(10),
        width: 0.18,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(28),
        outer_replacement_loop: BoundaryLoopId(32),
        inner_replacement_loop: BoundaryLoopId(33),
    };

    for (operations, rejected_operation) in [
        (
            vec![cut.clone(), entry_bevel.clone(), exit_bevel.clone()],
            OperationId(41),
        ),
        (vec![cut.clone(), exit_bevel, entry_bevel], OperationId(40)),
    ] {
        let error = generate_cut_plate_with_operations(operations, [3.0, 2.0], 0.30)
            .expect_err("opposing through-cut bevels should reject when they consume full depth");
        assert_unsupported_operation(&error, rejected_operation, "opposing through-cut bevels");
    }
}

#[test]
fn recessed_floor_bevel_rejects_collapsed_rounded_corner_radius() {
    let cut = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(31),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.08, -0.05],
        size: [1.18, 0.58],
        depth: 0.20,
        corner_radius: 0.08,
        rim_width: 0.10,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(9),
        floor_loop: BoundaryLoopId(10),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        floor_region: RegionId(25),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let floor_bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(41),
        target_loop: BoundaryLoopId(10),
        width: 0.085,
        segments: 2,
        profile: 1.0,
        bevel_region: RegionId(28),
        outer_replacement_loop: BoundaryLoopId(32),
        inner_replacement_loop: BoundaryLoopId(33),
    };

    let error = generate_cut_plate_with_operations(vec![cut, floor_bevel], [3.0, 2.0], 0.30)
        .expect_err("floor bevel should reject before collapsing rounded corner radius");
    assert_unsupported_operation(&error, OperationId(41), "target rounded corner radius");
}

#[test]
fn plate_circular_through_cut_is_deterministic_and_loop_tagged() {
    let operation = ModelingOperationSpec::CircularThroughCut {
        operation: OperationId(32),
        region: RegionId(2),
        face: PlanarCutFace::NegativeY,
        center: [-0.12, 0.06],
        radius: 0.36,
        radial_segments: 12,
        rim_width: 0.1152,
        entry_loop: BoundaryLoopId(11),
        exit_loop: BoundaryLoopId(12),
        outer_region: RegionId(2),
        rim_region: RegionId(25),
        wall_region: RegionId(26),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let part = generate_cut_plate(operation.clone()).expect("circular through cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_role(&part, RegionId(25), SurfaceRole::Rim);
    assert_region_role(&part, RegionId(26), SurfaceRole::CutWall);
    assert_circular_rim_region_is_radial_band(&part, RegionId(25), [-0.12, 0.06], 0.36, 0.1152);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(11), OperationId(32), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(12), OperationId(32), true);

    let repeated = generate_cut_plate(operation).expect("repeat should generate");
    assert_deterministic_ids(&part.mesh, &repeated.mesh);
}

#[test]
fn plate_cut_rejects_host_boundary_overlap() {
    let operation = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(33),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [2.95, 1.85],
        corner_radius: 0.0,
        rim_width: 0.296,
        corner_segments: 1,
        entry_loop: BoundaryLoopId(13),
        exit_loop: BoundaryLoopId(14),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::Hard,
    };

    assert!(generate_cut_plate(operation).is_err());
}

#[test]
fn crate_recessed_panel_proportions_are_directed_closed() {
    let operation = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(34),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [2.38, 0.48],
        depth: 0.045,
        corner_radius: 0.075,
        rim_width: 0.0768,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(15),
        floor_loop: BoundaryLoopId(16),
        outer_region: RegionId(1),
        rim_region: RegionId(20),
        wall_region: RegionId(21),
        floor_region: RegionId(22),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let part = generate_cut_plate_with_source(operation, [3.25, 0.82], 0.10)
        .expect("crate panel cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
}

#[test]
fn crate_ventilation_slat_cut_proportions_are_directed_closed() {
    let operation = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(35),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [0.42, 0.032],
        corner_radius: 0.006,
        rim_width: 0.00512,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(17),
        exit_loop: BoundaryLoopId(18),
        outer_region: RegionId(1),
        rim_region: RegionId(23),
        wall_region: RegionId(24),
        edge_treatment: CutEdgeTreatment::Hard,
    };
    let part = generate_cut_plate_with_source(operation, [0.84, 0.08], 0.045)
        .expect("crate ventilation slat cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
}

#[test]
fn plate_multiple_same_face_cuts_compose_closed_semantic_geometry() {
    let mut operations = Vec::new();
    operations.push(ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(40),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [-1.40, 0.0],
        size: [0.55, 0.44],
        depth: 0.08,
        corner_radius: 0.08,
        rim_width: 0.07,
        corner_segments: 4,
        entry_loop: BoundaryLoopId(40),
        floor_loop: BoundaryLoopId(41),
        outer_region: RegionId(1),
        rim_region: RegionId(140),
        wall_region: RegionId(141),
        floor_region: RegionId(142),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    });
    for (index, x) in [-0.45, 0.15, 0.75, 1.35].into_iter().enumerate() {
        let id = 50 + index as u64;
        operations.push(ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(id),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [x, -0.72],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.04,
            entry_loop: BoundaryLoopId(50 + index as u64 * 2),
            exit_loop: BoundaryLoopId(51 + index as u64 * 2),
            outer_region: RegionId(1),
            rim_region: RegionId(150 + index as u64 * 2),
            wall_region: RegionId(151 + index as u64 * 2),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        });
    }
    for (index, x) in [-0.45, 0.15, 0.75].into_iter().enumerate() {
        let id = 60 + index as u64;
        operations.push(ModelingOperationSpec::RectangularThroughCut {
            operation: OperationId(id),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [x, 0.70],
            size: [0.24, 0.05],
            corner_radius: 0.01,
            rim_width: 0.04,
            corner_segments: 3,
            entry_loop: BoundaryLoopId(70 + index as u64 * 2),
            exit_loop: BoundaryLoopId(71 + index as u64 * 2),
            outer_region: RegionId(1),
            rim_region: RegionId(170 + index as u64 * 2),
            wall_region: RegionId(171 + index as u64 * 2),
            edge_treatment: CutEdgeTreatment::Hard,
        });
    }

    let part = generate_cut_plate_with_operations(operations.clone(), [4.0, 2.0], 0.30)
        .expect("multi-cut plate should generate");
    let reversed_part = generate_cut_plate_with_operations(
        operations.into_iter().rev().collect(),
        [4.0, 2.0],
        0.30,
    )
    .expect("reordered independent cuts should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_faces_have_no_operation(&part, RegionId(3));
    assert_region_faces_have_no_operation(&reversed_part, RegionId(3));
    assert_operation_faces_on_plane_use_region(
        &part,
        OperationId(50),
        SurfaceRole::PrimarySurface,
        -0.15,
        RegionId(2),
    );
    for operation in [
        OperationId(40),
        OperationId(50),
        OperationId(51),
        OperationId(52),
        OperationId(53),
        OperationId(60),
        OperationId(61),
        OperationId(62),
    ] {
        assert_face_operation_present(&part.mesh, operation);
    }
    for boundary_loop in [
        BoundaryLoopId(40),
        BoundaryLoopId(41),
        BoundaryLoopId(50),
        BoundaryLoopId(51),
        BoundaryLoopId(52),
        BoundaryLoopId(53),
        BoundaryLoopId(54),
        BoundaryLoopId(55),
        BoundaryLoopId(56),
        BoundaryLoopId(57),
        BoundaryLoopId(70),
        BoundaryLoopId(71),
        BoundaryLoopId(72),
        BoundaryLoopId(73),
        BoundaryLoopId(74),
        BoundaryLoopId(75),
    ] {
        assert!(
            part.mesh
                .edge_metadata
                .values()
                .any(|metadata| metadata.boundary_loop == Some(boundary_loop)),
            "missing boundary loop {boundary_loop:?}"
        );
    }
    assert!(
        part.generator_signature
            .contains("plate_multi_cut:w=4.000000:h=2.000000:t=0.300000")
    );
}

#[test]
fn plate_multi_cut_boundary_bevel_replaces_target_loop() {
    let operations = vec![
        ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(50),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [-0.55, -0.20],
            radius: 0.10,
            radial_segments: 12,
            rim_width: 0.05,
            entry_loop: BoundaryLoopId(50),
            exit_loop: BoundaryLoopId(51),
            outer_region: RegionId(1),
            rim_region: RegionId(150),
            wall_region: RegionId(151),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        },
        ModelingOperationSpec::RectangularThroughCut {
            operation: OperationId(60),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.60, 0.35],
            size: [0.28, 0.08],
            corner_radius: 0.01,
            rim_width: 0.04,
            corner_segments: 3,
            entry_loop: BoundaryLoopId(70),
            exit_loop: BoundaryLoopId(71),
            outer_region: RegionId(1),
            rim_region: RegionId(170),
            wall_region: RegionId(171),
            edge_treatment: CutEdgeTreatment::Hard,
        },
        ModelingOperationSpec::BevelBoundaryLoop {
            operation: OperationId(90),
            target_loop: BoundaryLoopId(50),
            width: 0.02,
            segments: 2,
            profile: 1.0,
            bevel_region: RegionId(190),
            outer_replacement_loop: BoundaryLoopId(92),
            inner_replacement_loop: BoundaryLoopId(93),
        },
    ];

    let part = generate_cut_plate_with_operations(operations, [3.0, 2.0], 0.30)
        .expect("multi-cut bevel should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_role(&part, RegionId(190), SurfaceRole::BevelBand);
    assert_no_boundary_loop(&part.mesh, BoundaryLoopId(50));
    assert_boundary_loop(&part.mesh, BoundaryLoopId(51), OperationId(50), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(92), OperationId(90), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(93), OperationId(90), false);
    assert_face_operation_present(&part.mesh, OperationId(90));
}

#[test]
fn plate_multi_cut_rejects_overlapping_footprints() {
    let operations = vec![
        ModelingOperationSpec::RectangularThroughCut {
            operation: OperationId(80),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            size: [0.70, 0.30],
            corner_radius: 0.0,
            rim_width: 0.05,
            corner_segments: 1,
            entry_loop: BoundaryLoopId(80),
            exit_loop: BoundaryLoopId(81),
            outer_region: RegionId(1),
            rim_region: RegionId(180),
            wall_region: RegionId(181),
            edge_treatment: CutEdgeTreatment::Hard,
        },
        ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(81),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.20, 0.0],
            radius: 0.16,
            radial_segments: 10,
            rim_width: 0.04,
            entry_loop: BoundaryLoopId(82),
            exit_loop: BoundaryLoopId(83),
            outer_region: RegionId(1),
            rim_region: RegionId(182),
            wall_region: RegionId(183),
            edge_treatment: CutEdgeTreatment::Hard,
        },
    ];

    assert!(generate_cut_plate_with_operations(operations, [3.0, 2.0], 0.30).is_err());
}

#[test]
fn negative_face_multi_cut_uses_front_region_for_opposite_support() {
    let operations = vec![
        ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(90),
            region: RegionId(2),
            face: PlanarCutFace::NegativeY,
            center: [-0.40, 0.0],
            radius: 0.12,
            radial_segments: 12,
            rim_width: 0.04,
            entry_loop: BoundaryLoopId(90),
            exit_loop: BoundaryLoopId(91),
            outer_region: RegionId(2),
            rim_region: RegionId(190),
            wall_region: RegionId(191),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        },
        ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(91),
            region: RegionId(2),
            face: PlanarCutFace::NegativeY,
            center: [0.40, 0.0],
            radius: 0.12,
            radial_segments: 12,
            rim_width: 0.04,
            entry_loop: BoundaryLoopId(92),
            exit_loop: BoundaryLoopId(93),
            outer_region: RegionId(2),
            rim_region: RegionId(192),
            wall_region: RegionId(193),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        },
    ];

    let part = generate_cut_plate_with_operations(operations, [2.0, 1.2], 0.30)
        .expect("negative-face multi-cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_faces_have_no_operation(&part, RegionId(3));
    assert_operation_faces_on_plane_use_region(
        &part,
        OperationId(90),
        SurfaceRole::PrimarySurface,
        0.15,
        RegionId(1),
    );
    assert_operation_faces_on_plane_use_region(
        &part,
        OperationId(90),
        SurfaceRole::PrimarySurface,
        -0.15,
        RegionId(2),
    );
}

#[test]
fn rounded_box_recessed_panel_cut_is_closed_and_preserves_unaffected_rounding() {
    let operation = ModelingOperationSpec::RecessedPanelCut {
        operation: OperationId(120),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [0.86, 0.28],
        depth: 0.08,
        corner_radius: 0.05,
        rim_width: 0.06,
        corner_segments: 3,
        entry_loop: BoundaryLoopId(120),
        floor_loop: BoundaryLoopId(121),
        outer_region: RegionId(1),
        rim_region: RegionId(220),
        wall_region: RegionId(221),
        floor_region: RegionId(222),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };

    let part = generate_cut_rounded_box_with_operations(vec![operation], [1.2, 0.7, 0.55], 0.16)
        .expect("rounded-box recessed cut should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_faces_use_regions(
        &part,
        &[
            RegionId(1),
            RegionId(2),
            RegionId(3),
            RegionId(220),
            RegionId(221),
            RegionId(222),
        ],
    );
    assert_region_faces_have_no_operation(&part, RegionId(2));
    assert_region_faces_have_no_operation(&part, RegionId(3));
    assert_boundary_loop(&part.mesh, BoundaryLoopId(120), OperationId(120), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(121), OperationId(120), true);
    assert_face_operation_present(&part.mesh, OperationId(120));
}

#[test]
fn rounded_box_through_cuts_work_on_all_primary_faces() {
    for (index, face) in [
        PlanarCutFace::PositiveX,
        PlanarCutFace::NegativeX,
        PlanarCutFace::PositiveY,
        PlanarCutFace::NegativeY,
        PlanarCutFace::PositiveZ,
        PlanarCutFace::NegativeZ,
    ]
    .into_iter()
    .enumerate()
    {
        let operation = OperationId(130 + index as u64);
        let entry_loop = BoundaryLoopId(130 + index as u64 * 2);
        let exit_loop = BoundaryLoopId(131 + index as u64 * 2);
        let cut = ModelingOperationSpec::CircularThroughCut {
            operation,
            region: RegionId(1),
            face,
            center: [0.0, 0.0],
            radius: 0.12,
            radial_segments: 12,
            rim_width: 0.045,
            entry_loop,
            exit_loop,
            outer_region: RegionId(1),
            rim_region: RegionId(230 + index as u64 * 2),
            wall_region: RegionId(231 + index as u64 * 2),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        };

        let part = generate_cut_rounded_box_with_operations(vec![cut], [1.2, 0.8, 0.65], 0.14)
            .unwrap_or_else(|error| panic!("{face:?} should generate: {error:?}"));

        assert_closed_mesh(&part.mesh);
        assert_common_mesh_quality(&part.mesh);
        assert_boundary_loop(&part.mesh, entry_loop, operation, true);
        assert_boundary_loop(&part.mesh, exit_loop, operation, true);
        assert_face_operation_present(&part.mesh, operation);
        assert_region_faces_have_no_operation(&part, RegionId(2));
        assert_region_faces_have_no_operation(&part, RegionId(3));
    }
}

#[test]
fn rounded_box_multi_cut_face_supports_beveled_holes_and_hard_vents() {
    let hole = ModelingOperationSpec::CircularThroughCut {
        operation: OperationId(150),
        region: RegionId(1),
        face: PlanarCutFace::PositiveZ,
        center: [-0.30, 0.0],
        radius: 0.09,
        radial_segments: 12,
        rim_width: 0.055,
        entry_loop: BoundaryLoopId(150),
        exit_loop: BoundaryLoopId(151),
        outer_region: RegionId(1),
        rim_region: RegionId(250),
        wall_region: RegionId(251),
        edge_treatment: CutEdgeTreatment::BevelEligible,
    };
    let vent = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(151),
        region: RegionId(1),
        face: PlanarCutFace::PositiveZ,
        center: [0.32, 0.32],
        size: [0.28, 0.06],
        corner_radius: 0.0,
        rim_width: 0.04,
        corner_segments: 1,
        entry_loop: BoundaryLoopId(152),
        exit_loop: BoundaryLoopId(153),
        outer_region: RegionId(1),
        rim_region: RegionId(252),
        wall_region: RegionId(253),
        edge_treatment: CutEdgeTreatment::Hard,
    };
    let bevel = ModelingOperationSpec::BevelBoundaryLoop {
        operation: OperationId(160),
        target_loop: BoundaryLoopId(150),
        width: 0.02,
        segments: 2,
        profile: 1.4,
        bevel_region: RegionId(260),
        outer_replacement_loop: BoundaryLoopId(260),
        inner_replacement_loop: BoundaryLoopId(261),
    };

    let part =
        generate_cut_rounded_box_with_operations(vec![hole, vent, bevel], [1.2, 0.85, 0.70], 0.14)
            .expect("rounded-box multi-cut face should generate");

    assert_closed_mesh(&part.mesh);
    assert_common_mesh_quality(&part.mesh);
    assert_region_role(&part, RegionId(260), SurfaceRole::BevelBand);
    assert_no_boundary_loop(&part.mesh, BoundaryLoopId(150));
    assert_boundary_loop(&part.mesh, BoundaryLoopId(151), OperationId(150), true);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(152), OperationId(151), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(153), OperationId(151), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(260), OperationId(160), false);
    assert_boundary_loop(&part.mesh, BoundaryLoopId(261), OperationId(160), false);
    assert_face_operation_present(&part.mesh, OperationId(150));
    assert_face_operation_present(&part.mesh, OperationId(151));
    assert_face_operation_present(&part.mesh, OperationId(160));
}

#[test]
fn rounded_box_cut_rejects_host_bevel_overlap() {
    let cut = ModelingOperationSpec::RectangularThroughCut {
        operation: OperationId(170),
        region: RegionId(1),
        face: PlanarCutFace::PositiveY,
        center: [0.0, 0.0],
        size: [1.55, 0.62],
        corner_radius: 0.0,
        rim_width: 0.05,
        corner_segments: 1,
        entry_loop: BoundaryLoopId(170),
        exit_loop: BoundaryLoopId(171),
        outer_region: RegionId(1),
        rim_region: RegionId(270),
        wall_region: RegionId(271),
        edge_treatment: CutEdgeTreatment::Hard,
    };

    let error = generate_cut_rounded_box_with_operations(vec![cut], [1.0, 0.75, 0.50], 0.20)
        .expect_err("cut should reject when the rim enters rounded host bevel bands");
    assert!(
        format!("{error:?}").contains("host boundary")
            || format!("{error:?}").contains("plate face")
            || format!("{error:?}").contains("host bevel"),
        "unexpected error: {error:?}"
    );
}

fn context() -> GeneratorContext {
    GeneratorContext::new(PartDefinitionId(7), PartInstanceId(11), 100, 0)
}

fn generate_cut_plate(
    operation: ModelingOperationSpec,
) -> Result<GeneratedPart, shape_modeling::ModelingError> {
    generate_cut_plate_with_source(operation, [3.0, 2.0], 0.30)
}

fn generate_cut_plate_with_source(
    operation: ModelingOperationSpec,
    size: [f32; 2],
    thickness: f32,
) -> Result<GeneratedPart, shape_modeling::ModelingError> {
    generate_cut_plate_with_operations(vec![operation], size, thickness)
}

fn generate_cut_plate_with_operations(
    operations: Vec<ModelingOperationSpec>,
    size: [f32; 2],
    thickness: f32,
) -> Result<GeneratedPart, shape_modeling::ModelingError> {
    let definition = PartDefinition {
        id: PartDefinitionId(7),
        name: "cut plate".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::Plate { size, thickness },
            operations,
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let mut context = context();
    generate_plate(&definition, &mut context)
}

fn generate_cut_rounded_box_with_operations(
    operations: Vec<ModelingOperationSpec>,
    half_extents: [f32; 3],
    radius: f32,
) -> Result<GeneratedPart, shape_modeling::ModelingError> {
    let definition = PartDefinition {
        id: PartDefinitionId(7),
        name: "cut rounded box".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents,
                radius,
            },
            operations,
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let mut context = context();
    generate_rounded_box(&definition, &mut context)
}

fn assert_closed_mesh(mesh: &PolygonMesh) {
    let adjacency = build_adjacency(mesh).expect("adjacency should build");
    let bad_edges = adjacency
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() != 2)
        .take(8)
        .map(|(edge, faces)| {
            format!(
                "{}.{}, {:?}->{:?} -> {}",
                edge.a,
                edge.b,
                mesh.positions[edge.a as usize],
                mesh.positions[edge.b as usize],
                faces.len()
            )
        })
        .collect::<Vec<_>>();
    assert!(
        adjacency.edge_faces.values().all(|faces| faces.len() == 2),
        "closed mesh should have exactly two incident faces per edge; bad edges: {bad_edges:?}"
    );
    assert_eq!(open_boundary_count(mesh), 0);
    assert_directed_edges_are_paired(mesh);
    assert!(
        signed_volume(mesh) > EPSILON,
        "closed mesh should have consistent outward winding"
    );
}

fn assert_directed_edges_are_paired(mesh: &PolygonMesh) {
    let mut edge_uses = BTreeMap::<shape_poly::EdgeKey, Vec<(u32, u32)>>::new();
    for face in &mesh.faces {
        for index in 0..face.vertices.len() {
            let from = face.vertices[index];
            let to = face.vertices[(index + 1) % face.vertices.len()];
            edge_uses
                .entry(shape_poly::EdgeKey::new(from, to))
                .or_default()
                .push((from, to));
        }
    }
    let bad_edges = edge_uses
        .iter()
        .filter(|(_, uses)| uses.len() != 2 || uses[0] == uses[1])
        .take(8)
        .map(|(edge, uses)| format!("{}.{} -> {uses:?}", edge.a, edge.b))
        .collect::<Vec<_>>();
    assert!(
        bad_edges.is_empty(),
        "closed mesh should use every edge in opposite directions; bad edges: {bad_edges:?}"
    );
}

fn assert_valid_with_open_boundaries(mesh: &PolygonMesh, expected_open_edges: usize) {
    let adjacency = build_adjacency(mesh).expect("adjacency should build");
    assert!(
        adjacency
            .edge_faces
            .values()
            .all(|faces| (1..=2).contains(&faces.len())),
        "open mesh should remain manifold"
    );
    assert_eq!(open_boundary_count(mesh), expected_open_edges);
}

fn assert_common_mesh_quality(mesh: &PolygonMesh) {
    assert!(
        validate_polygon_mesh(mesh).is_valid(),
        "mesh contract validation should pass"
    );
    assert_no_duplicate_positions(mesh);
    assert_no_degenerate_faces(mesh);
    let face_normals = compute_face_normals(mesh).expect("face normals should compute");
    assert!(
        face_normals.iter().copied().all(finite_vector),
        "face normals should be finite"
    );
    let split_normals = compute_split_vertex_normals(mesh).expect("split normals should compute");
    assert!(
        split_normals.iter().copied().all(finite_vector),
        "split vertex normals should be finite"
    );
    let triangulated = triangulate_polygon_mesh(mesh).expect("triangulation should succeed");
    assert_eq!(
        triangulated.mesh.indices.len() % 3,
        0,
        "triangulation should produce whole triangles"
    );
}

fn assert_faces_use_regions(part: &GeneratedPart, expected_regions: &[RegionId]) {
    let counts = region_face_counts(&part.mesh);
    for region in expected_regions {
        assert!(
            counts.get(region).copied().unwrap_or_default() > 0,
            "expected region {region:?} to have faces"
        );
    }
}

fn assert_region_names(part: &GeneratedPart, expected_names: &[&str]) {
    let names = part
        .regions
        .values()
        .map(|region| region.name.as_str())
        .collect::<BTreeSet<_>>();
    for expected in expected_names {
        assert!(names.contains(expected), "missing region name {expected}");
    }
}

fn assert_region_role(part: &GeneratedPart, region: RegionId, role: SurfaceRole) {
    let actual = part
        .regions
        .get(&region)
        .unwrap_or_else(|| panic!("missing region {region:?}"))
        .role
        .clone();
    assert_eq!(actual, role);
}

fn assert_region_faces_have_no_operation(part: &GeneratedPart, region: RegionId) {
    let mut checked = 0;
    for metadata in &part.mesh.face_metadata {
        if metadata.region != Some(region) {
            continue;
        }
        checked += 1;
        assert_eq!(
            metadata.operation, None,
            "base region {region:?} should not carry cut operation provenance"
        );
    }
    assert!(checked > 0, "expected faces for region {region:?}");
}

fn assert_operation_faces_on_plane_use_region(
    part: &GeneratedPart,
    operation: OperationId,
    role: SurfaceRole,
    y: f32,
    region: RegionId,
) {
    let mut checked = 0;
    for (face, metadata) in part.mesh.faces.iter().zip(&part.mesh.face_metadata) {
        if metadata.operation != Some(operation) || metadata.surface_role != Some(role.clone()) {
            continue;
        }
        let on_plane = face
            .vertices
            .iter()
            .all(|vertex| (part.mesh.positions[*vertex as usize][1] - y).abs() <= 0.0001);
        if !on_plane {
            continue;
        }
        checked += 1;
        assert_eq!(
            metadata.region,
            Some(region),
            "operation {operation:?} faces on y={y} should use host region {region:?}"
        );
    }
    assert!(
        checked > 0,
        "expected operation {operation:?} {role:?} faces on y={y}"
    );
}

fn assert_circular_rim_region_is_radial_band(
    part: &GeneratedPart,
    region: RegionId,
    center: [f32; 2],
    radius: f32,
    rim_width: f32,
) {
    let outer = radius + rim_width;
    let mut checked = 0;
    for (face, metadata) in part.mesh.faces.iter().zip(&part.mesh.face_metadata) {
        if metadata.region != Some(region) {
            continue;
        }
        checked += 1;
        for vertex in &face.vertices {
            let position = part.mesh.positions[*vertex as usize];
            let distance =
                ((position[0] - center[0]).powi(2) + (position[2] - center[1]).powi(2)).sqrt();
            assert!(
                distance >= radius - 0.0001 && distance <= outer + 0.0001,
                "rim vertex radius {distance} fell outside [{radius}, {outer}]"
            );
        }
    }
    assert!(checked > 0, "expected rim-region faces");
}

fn assert_boundary_loop(
    mesh: &PolygonMesh,
    boundary_loop: BoundaryLoopId,
    operation: OperationId,
    bevel_eligible: bool,
) {
    let edges = mesh
        .edge_metadata
        .values()
        .filter(|metadata| metadata.boundary_loop == Some(boundary_loop))
        .collect::<Vec<_>>();
    assert!(!edges.is_empty(), "missing boundary loop {boundary_loop:?}");
    assert!(edges.iter().all(|metadata| {
        metadata.boundary_role == BoundaryRole::Feature
            && metadata.classification == EdgeClassification::Hard
            && metadata.operation == Some(operation)
            && !metadata.seam_candidate
            && metadata.bevel_eligible == bevel_eligible
    }));
}

fn boundary_loop_xz_bounds(
    mesh: &PolygonMesh,
    boundary_loop: BoundaryLoopId,
) -> (f32, f32, f32, f32) {
    let mut vertices = BTreeSet::new();
    for (edge, metadata) in &mesh.edge_metadata {
        if metadata.boundary_loop == Some(boundary_loop) {
            vertices.insert(edge.a);
            vertices.insert(edge.b);
        }
    }
    assert!(
        !vertices.is_empty(),
        "boundary loop {boundary_loop:?} should have vertices"
    );
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for vertex in vertices {
        let position = mesh.positions[vertex as usize];
        min_x = min_x.min(position[0]);
        max_x = max_x.max(position[0]);
        min_z = min_z.min(position[2]);
        max_z = max_z.max(position[2]);
    }
    (min_x, max_x, min_z, max_z)
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn assert_no_boundary_loop(mesh: &PolygonMesh, boundary_loop: BoundaryLoopId) {
    assert!(
        mesh.edge_metadata
            .values()
            .all(|metadata| metadata.boundary_loop != Some(boundary_loop)),
        "boundary loop {boundary_loop:?} should have been consumed"
    );
}

fn assert_face_operation_present(mesh: &PolygonMesh, operation: OperationId) {
    assert!(
        mesh.face_metadata
            .iter()
            .any(|metadata| metadata.operation == Some(operation)),
        "expected at least one face sourced by {operation:?}"
    );
}

fn assert_unsupported_operation(error: &ModelingError, operation: OperationId, reason: &str) {
    match error {
        ModelingError::UnsupportedOperation {
            operation: actual,
            reason: actual_reason,
        } => {
            assert_eq!(*actual, operation);
            assert!(
                actual_reason.contains(reason),
                "expected reason containing {reason:?}, got {actual_reason:?}"
            );
        }
        other => panic!("expected unsupported operation error, got {other:?}"),
    }
}

fn operation_y_samples(mesh: &PolygonMesh, operation: OperationId) -> Vec<i64> {
    let mut samples = BTreeSet::new();
    for (face, metadata) in mesh.faces.iter().zip(&mesh.face_metadata) {
        if metadata.operation == Some(operation) {
            for vertex in &face.vertices {
                samples.insert(quantize(mesh.positions[*vertex as usize][1]));
            }
        }
    }
    samples.into_iter().collect()
}

fn operation_smoothing_groups(mesh: &PolygonMesh, operation: OperationId) -> BTreeSet<Option<u32>> {
    mesh.face_metadata
        .iter()
        .filter(|metadata| metadata.operation == Some(operation))
        .map(|metadata| metadata.smoothing_group)
        .collect()
}

fn bevel_internal_smooth_edge_count(
    mesh: &PolygonMesh,
    operation: OperationId,
    region: RegionId,
) -> usize {
    let mut edge_faces: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for index in 0..face.vertices.len() {
            let next = (index + 1) % face.vertices.len();
            edge_faces
                .entry(EdgeKey::new(face.vertices[index], face.vertices[next]))
                .or_default()
                .push(face_index);
        }
    }
    edge_faces
        .into_iter()
        .filter(|(edge, faces)| {
            faces.len() == 2
                && faces.iter().all(|face| {
                    let metadata = &mesh.face_metadata[*face];
                    metadata.operation == Some(operation) && metadata.region == Some(region)
                })
                && mesh.edge_metadata.get(edge).is_some_and(|metadata| {
                    metadata.boundary_role == BoundaryRole::Smooth
                        && metadata.classification == EdgeClassification::Smooth
                })
        })
        .count()
}

fn assert_socket_origin(part: &GeneratedPart, socket: SocketId, expected: [f32; 3]) {
    let actual = part
        .sockets
        .get(&socket)
        .unwrap_or_else(|| panic!("missing socket {socket:?}"))
        .local_frame
        .origin;
    assert_vec3_close(actual, expected);
}

fn assert_bounds(mesh: &PolygonMesh, expected_min: [f32; 3], expected_max: [f32; 3]) {
    assert_vec3_close(mesh.bounds.min, expected_min);
    assert_vec3_close(mesh.bounds.max, expected_max);
}

fn assert_deterministic_ids(first: &PolygonMesh, second: &PolygonMesh) {
    assert_eq!(first.topology_signature, second.topology_signature);
    assert_eq!(first.vertex_ids, second.vertex_ids);
    let first_face_ids = first.faces.iter().map(|face| face.id).collect::<Vec<_>>();
    let second_face_ids = second.faces.iter().map(|face| face.id).collect::<Vec<_>>();
    assert_eq!(first_face_ids, second_face_ids);
}

fn assert_same_region_ids(first: &GeneratedPart, second: &GeneratedPart) {
    assert_eq!(
        first.regions.keys().copied().collect::<Vec<_>>(),
        second.regions.keys().copied().collect::<Vec<_>>()
    );
}

fn assert_no_duplicate_positions(mesh: &PolygonMesh) {
    let mut seen = BTreeSet::new();
    for position in &mesh.positions {
        assert!(
            seen.insert(VertexKey::from_position(*position)),
            "duplicate vertex position {position:?}"
        );
    }
}

fn assert_no_degenerate_faces(mesh: &PolygonMesh) {
    for face in &mesh.faces {
        let area = polygon_area(mesh, &face.vertices);
        assert!(
            area > EPSILON,
            "degenerate face {:?}: {:?}",
            face.id,
            face.vertices
                .iter()
                .map(|vertex| mesh.positions[*vertex as usize])
                .collect::<Vec<_>>()
        );
    }
}

fn open_boundary_count(mesh: &PolygonMesh) -> usize {
    mesh.edge_metadata
        .values()
        .filter(|metadata| metadata.boundary_role == BoundaryRole::OpenBoundary)
        .count()
}

fn region_face_counts(mesh: &PolygonMesh) -> BTreeMap<RegionId, usize> {
    let mut counts = BTreeMap::new();
    for metadata in &mesh.face_metadata {
        if let Some(region) = metadata.region {
            *counts.entry(region).or_insert(0) += 1;
        }
    }
    counts
}

fn signed_volume(mesh: &PolygonMesh) -> f32 {
    let triangles = triangulate_polygon_mesh(mesh).expect("closed mesh should triangulate");
    let mut volume = 0.0;
    for triangle in triangles.mesh.indices.chunks_exact(3) {
        let a = triangles.mesh.positions[triangle[0] as usize];
        let b = triangles.mesh.positions[triangle[1] as usize];
        let c = triangles.mesh.positions[triangle[2] as usize];
        volume += dot(a, cross(b, c)) / 6.0;
    }
    volume
}

fn polygon_area(mesh: &PolygonMesh, vertices: &[u32]) -> f32 {
    let origin = mesh.positions[vertices[0] as usize];
    let mut area = 0.0;
    for index in 1..vertices.len() - 1 {
        let a = mesh.positions[vertices[index] as usize];
        let b = mesh.positions[vertices[index + 1] as usize];
        area += length(cross(sub(a, origin), sub(b, origin))) * 0.5;
    }
    area
}

fn finite_vector(vector: [f32; 3]) -> bool {
    vector.iter().copied().all(f32::is_finite)
}

fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(vector: [f32; 3]) -> f32 {
    dot(vector, vector).sqrt()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VertexKey(i64, i64, i64);

impl VertexKey {
    fn from_position(position: [f32; 3]) -> Self {
        Self(
            quantize(position[0]),
            quantize(position[1]),
            quantize(position[2]),
        )
    }
}

fn quantize(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}
