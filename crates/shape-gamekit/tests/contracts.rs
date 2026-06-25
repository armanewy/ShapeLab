use shape_asset::{AssetId, AssetRecipe, Frame3};
use shape_gamekit::export::{
    FrozenMeshArtifact, ManualReviewMarker, ManualReviewStatus, MaterialSlotAssignment,
    STATIC_PROP_GAME_READY_PACKAGE_SCHEMA_VERSION, StaticPropCollision, StaticPropFeatureStatus,
    StaticPropGameReadyPackage, StaticPropHandoff, StaticPropLodLevel, StaticPropLodPolicy,
    StaticPropReadinessStatus, StaticPropVisualEvidence, UvPolicy,
    validate_static_prop_game_ready_package, validate_static_prop_game_ready_package_with_root,
};
use shape_gamekit::gltf::{StaticPropGlbMetadata, encode_static_prop_glb};
use shape_gamekit::{
    CellBounds, CollisionProxy, ConstructionPhase, ConstructionProfile, ExportProfile,
    FixedCameraProfile, GAME_ASSET_PACK_SCHEMA_VERSION, GameAssetDefinition, GameAssetPack,
    GameplayTag, GridRotation, LayerBounds, LogicalFootprint, ModuleSemantics,
    MonotonicVisibilityPolicy, ReadabilityProfile, RotationSymmetry, SnapAnchor, SnapAnchorRole,
    SnapRelationship, SupportRole, SupportSurface, SurfaceShape, TraversalRole, TriangleBudget,
    WalkableSurface, validate_construction_profile, validate_game_asset_pack,
    validate_logical_footprint, validate_snap_anchors, validate_triangle_budget,
    validate_walkable_surfaces,
};
use shape_mesh::TriangleMesh;

fn valid_asset(runtime_key: &str) -> GameAssetDefinition {
    GameAssetDefinition {
        id: format!("asset:{runtime_key}"),
        display_name: runtime_key.replace('_', " "),
        family: "Contract Test".to_owned(),
        source_recipe: AssetRecipe::new(AssetId(1), "Contract Test"),
        module_semantics: valid_semantics(runtime_key),
        construction_profile: valid_construction_profile(),
        readability_profile: valid_readability_profile(),
        budgets: TriangleBudget {
            preview_maximum: 100,
            game_maximum: 200,
            repeated_instance_maximum: 100,
        },
        tags: vec!["test".to_owned()],
    }
}

fn valid_semantics(runtime_key: &str) -> ModuleSemantics {
    ModuleSemantics {
        runtime_key: runtime_key.to_owned(),
        logical_footprint: valid_footprint(),
        rotation_symmetry: RotationSymmetry::None,
        instanceable: true,
        snap_anchors: vec![SnapAnchor {
            id: "entry".to_owned(),
            role: SnapAnchorRole::Entry,
            local_frame: Frame3::default(),
            compatibility_tags: vec!["entry".to_owned()],
            relationship: SnapRelationship::Optional,
        }],
        support_surfaces: Vec::new(),
        walkable_surfaces: vec![WalkableSurface {
            id: "surface".to_owned(),
            polygon: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            elevation: 0.0,
            traversal_role: TraversalRole::Ground,
            entry_exit_anchors: vec!["entry".to_owned()],
        }],
        traversal_links: Vec::new(),
        collision_proxies: vec![CollisionProxy::Box {
            center: [0.5, 0.1, 0.5],
            half_extents: [0.5, 0.1, 0.5],
        }],
        gameplay_tags: vec![GameplayTag::Walkable],
    }
}

fn valid_footprint() -> LogicalFootprint {
    LogicalFootprint {
        cell_bounds: CellBounds {
            min: [0, 0],
            max: [0, 0],
        },
        vertical_layers: LayerBounds { min: 0, max: 0 },
        origin_cell: [0, 0],
        permitted_rotations: vec![GridRotation::R0],
    }
}

fn valid_construction_profile() -> ConstructionProfile {
    ConstructionProfile {
        phases: vec![
            ConstructionPhase {
                id: "placed".to_owned(),
                label: "Placed".to_owned(),
                progress_threshold: 0.0,
                visible_part_tags: vec!["base".to_owned()],
                required_predecessor: None,
            },
            ConstructionPhase {
                id: "complete".to_owned(),
                label: "Complete".to_owned(),
                progress_threshold: 1.0,
                visible_part_tags: vec!["base".to_owned(), "detail".to_owned()],
                required_predecessor: Some("placed".to_owned()),
            },
        ],
        optional_damaged_state: None,
        final_phase: "complete".to_owned(),
        monotonic_visibility_policy: MonotonicVisibilityPolicy::Strict,
    }
}

fn valid_readability_profile() -> ReadabilityProfile {
    ReadabilityProfile {
        fixed_camera_profiles: vec![FixedCameraProfile::Oblique],
        minimum_recognizable_pixel_size: 32,
        silhouette_importance: 0.5,
        maximum_hidden_area_fraction: 0.3,
        orientation_coverage: vec![GridRotation::R0],
    }
}

fn pack_with(assets: Vec<GameAssetDefinition>) -> GameAssetPack {
    GameAssetPack {
        schema_version: GAME_ASSET_PACK_SCHEMA_VERSION,
        id: "contract-pack".to_owned(),
        title: "Contract Pack".to_owned(),
        assets,
        export_profile: ExportProfile::internal_dogfood(),
        source_revision: "test".to_owned(),
    }
}

fn issue_codes(report: &shape_gamekit::GameAssetValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

#[test]
fn serde_round_trip_preserves_pack() {
    let pack = pack_with(vec![valid_asset("deck")]);

    let json = serde_json::to_string_pretty(&pack).expect("pack should serialize");
    let round_tripped: GameAssetPack =
        serde_json::from_str(&json).expect("pack should deserialize");

    assert_eq!(pack, round_tripped);
}

#[test]
fn invalid_footprint_is_reported() {
    let mut footprint = valid_footprint();
    footprint.cell_bounds.min = [2, 0];
    footprint.cell_bounds.max = [0, 0];

    let report = validate_logical_footprint(&footprint);

    assert!(issue_codes(&report).contains(&"invalid_footprint_bounds"));
}

#[test]
fn duplicate_runtime_key_is_reported() {
    let pack = pack_with(vec![valid_asset("deck"), valid_asset("deck")]);

    let report = validate_game_asset_pack(&pack);

    assert!(issue_codes(&report).contains(&"duplicate_runtime_key"));
}

#[test]
fn invalid_snap_relationship_is_reported() {
    let anchors = vec![SnapAnchor {
        id: "support".to_owned(),
        role: SnapAnchorRole::Support,
        local_frame: Frame3::default(),
        compatibility_tags: Vec::new(),
        relationship: SnapRelationship::Required,
    }];

    let report = validate_snap_anchors(&anchors);

    assert!(issue_codes(&report).contains(&"invalid_snap_relationship"));
}

#[test]
fn walkable_surface_outside_bounds_is_reported() {
    let mut semantics = valid_semantics("deck");
    semantics.walkable_surfaces[0].polygon[0] = [4.0, 0.0];

    let report = validate_walkable_surfaces(&semantics);

    assert!(issue_codes(&report).contains(&"walkable_surface_outside_bounds"));
}

#[test]
fn construction_phase_cycle_is_reported() {
    let mut profile = valid_construction_profile();
    profile.phases[0].required_predecessor = Some("complete".to_owned());

    let report = validate_construction_profile(&profile);

    assert!(issue_codes(&report).contains(&"construction_phase_cycle"));
}

#[test]
fn non_monotonic_phase_visibility_is_reported() {
    let mut profile = valid_construction_profile();
    profile.phases[1].visible_part_tags = vec!["detail".to_owned()];

    let report = validate_construction_profile(&profile);

    assert!(issue_codes(&report).contains(&"non_monotonic_phase_visibility"));
}

#[test]
fn unknown_damaged_construction_state_is_reported() {
    let mut profile = valid_construction_profile();
    profile.optional_damaged_state = Some("missing".to_owned());

    let report = validate_construction_profile(&profile);

    assert!(issue_codes(&report).contains(&"unknown_damaged_construction_phase"));
}

#[test]
fn duplicate_and_degenerate_support_surfaces_are_reported() {
    let mut asset = valid_asset("deck");
    asset.module_semantics.support_surfaces = vec![
        SupportSurface {
            id: "support".to_owned(),
            shape: SurfaceShape::Rectangle {
                center: [0.5, 0.5],
                size: [0.0, 1.0],
            },
            support_role: SupportRole::DeckSupport,
            maximum_supported_layer_hint: Some(1),
        },
        SupportSurface {
            id: "support".to_owned(),
            shape: SurfaceShape::Polygon {
                points: vec![[0.0, 0.0], [1.0, 0.0]],
            },
            support_role: SupportRole::DeckSupport,
            maximum_supported_layer_hint: Some(1),
        },
    ];

    let report = validate_game_asset_pack(&pack_with(vec![asset]));
    let codes = issue_codes(&report);

    assert!(codes.contains(&"duplicate_support_surface_id"));
    assert!(codes.contains(&"invalid_support_surface_shape"));
}

#[test]
fn invalid_triangle_budget_is_reported() {
    let report = validate_triangle_budget(&TriangleBudget {
        preview_maximum: 0,
        game_maximum: 100,
        repeated_instance_maximum: 200,
    });
    let codes = issue_codes(&report);

    assert!(codes.contains(&"invalid_triangle_budget"));
    assert!(codes.contains(&"repeated_budget_exceeds_game_budget"));
}

#[test]
fn deterministic_ordering_is_required_and_serialized_stably() {
    let sorted = pack_with(vec![valid_asset("deck"), valid_asset("pile")]);
    let unsorted = pack_with(vec![valid_asset("pile"), valid_asset("deck")]);

    assert!(validate_game_asset_pack(&sorted).is_valid());
    assert!(
        issue_codes(&validate_game_asset_pack(&unsorted))
            .contains(&"asset_order_not_deterministic")
    );
    let first = serde_json::to_string(&sorted).expect("pack should serialize");
    let second = serde_json::to_string(&sorted).expect("pack should serialize");
    assert_eq!(first, second);
}

fn valid_static_prop_package() -> StaticPropGameReadyPackage {
    StaticPropGameReadyPackage {
        schema_version: STATIC_PROP_GAME_READY_PACKAGE_SCHEMA_VERSION,
        profile_id: "sci-fi-crate".to_owned(),
        display_name: "Sci-Fi Crate".to_owned(),
        asset_family: "Static Prop".to_owned(),
        source_recipe_hash: 42,
        artifact_fingerprint: "artifact:abc".to_owned(),
        frozen_mesh: FrozenMeshArtifact {
            canonical_model_package: "model-package".to_owned(),
            asset_manifest: "model-package/asset-manifest.json".to_owned(),
            package_verification: "package-verification.json".to_owned(),
            grouped_obj: "frozen.obj".to_owned(),
            blender_reconstruct_script: "model-package/blender_reconstruct.py".to_owned(),
            compile_validation_passed: true,
            model_validation_passed: true,
        },
        lod_policy: StaticPropLodPolicy {
            policy: "LOD0 exact plus deterministic proxy fallbacks".to_owned(),
            levels: vec![
                StaticPropLodLevel {
                    index: 0,
                    id: "lod0_exact".to_owned(),
                    source: "canonical_model_package".to_owned(),
                    artifact: "model-package".to_owned(),
                    target_triangle_count: 256,
                    exact_source_geometry: true,
                },
                StaticPropLodLevel {
                    index: 1,
                    id: "lod1_box_proxy".to_owned(),
                    source: "compiled_bounds_proxy".to_owned(),
                    artifact: "lods/lod1-proxy.obj".to_owned(),
                    target_triangle_count: 24,
                    exact_source_geometry: false,
                },
                StaticPropLodLevel {
                    index: 2,
                    id: "lod2_collision_proxy".to_owned(),
                    source: "collision_proxy".to_owned(),
                    artifact: "lods/lod2-collision.obj".to_owned(),
                    target_triangle_count: 12,
                    exact_source_geometry: false,
                },
            ],
        },
        material_slots: vec![MaterialSlotAssignment {
            slot_id: "hard_surface_shell".to_owned(),
            display_name: "Hard Surface Shell".to_owned(),
            semantic_roles: vec!["body".to_owned(), "panel".to_owned()],
            policy: "single hard-surface slot assignment".to_owned(),
            material_payload_ready: false,
        }],
        uv_policy: UvPolicy {
            status: StaticPropFeatureStatus::Ready,
            required_for_game_ready: true,
            blocker_code: None,
            explanation: "UV layout is present.".to_owned(),
        },
        collision: StaticPropCollision {
            source: "compiled_bounds".to_owned(),
            proxies: vec![CollisionProxy::Box {
                center: [0.0, 0.0, 0.0],
                half_extents: [1.0, 0.5, 0.75],
            }],
        },
        handoff: StaticPropHandoff {
            primary_package_format: "shape-lab-model-package".to_owned(),
            blender_handoff_script: "model-package/blender_reconstruct.py".to_owned(),
            blender_status: StaticPropFeatureStatus::Ready,
            glb_artifact: Some("asset.glb".to_owned()),
            glb_status: StaticPropFeatureStatus::Ready,
            glb_blocker_code: None,
        },
        visual_evidence: StaticPropVisualEvidence {
            front: "visual-evidence/front.png".to_owned(),
            three_quarter: "visual-evidence/three-quarter.png".to_owned(),
            side: "visual-evidence/side.png".to_owned(),
            wireframe: "visual-evidence/wireframe.png".to_owned(),
            contact_sheet: "visual-evidence/contact-sheet.png".to_owned(),
        },
        manual_review: ManualReviewMarker {
            status: ManualReviewStatus::Approved,
            reviewer: Some("qa".to_owned()),
            notes: "Approved for contract test.".to_owned(),
        },
    }
}

fn create_static_prop_artifact_files(root: &std::path::Path, package: &StaticPropGameReadyPackage) {
    std::fs::create_dir_all(root.join("model-package")).expect("model package dir");
    std::fs::create_dir_all(root.join("lods")).expect("lod dir");
    std::fs::create_dir_all(root.join("visual-evidence")).expect("visual dir");
    for path in [
        package.frozen_mesh.asset_manifest.as_str(),
        package.frozen_mesh.package_verification.as_str(),
        package.frozen_mesh.grouped_obj.as_str(),
        package.frozen_mesh.blender_reconstruct_script.as_str(),
        package.lod_policy.levels[1].artifact.as_str(),
        package.lod_policy.levels[2].artifact.as_str(),
        package
            .handoff
            .glb_artifact
            .as_deref()
            .unwrap_or("asset.glb"),
        package.visual_evidence.front.as_str(),
        package.visual_evidence.three_quarter.as_str(),
        package.visual_evidence.side.as_str(),
        package.visual_evidence.wireframe.as_str(),
        package.visual_evidence.contact_sheet.as_str(),
    ] {
        let bytes = if path.ends_with(".glb") {
            static_prop_glb_fixture_bytes()
        } else {
            static_prop_artifact_fixture_bytes(path).to_vec()
        };
        std::fs::write(root.join(path), bytes).unwrap_or_else(|error| {
            panic!("write static prop artifact fixture {path}: {error}");
        });
    }
}

fn static_prop_glb_fixture_bytes() -> Vec<u8> {
    encode_static_prop_glb(
        &TriangleMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: shape_core::Aabb {
                min: glam::Vec3::ZERO,
                max: glam::Vec3::new(1.0, 1.0, 0.0),
            },
        },
        &StaticPropGlbMetadata {
            profile_id: "sci-fi-crate".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            material_slots: vec!["painted_metal_body".to_owned()],
        },
    )
    .expect("valid static prop GLB fixture")
}

fn static_prop_artifact_fixture_bytes(path: &str) -> &'static [u8] {
    if path.ends_with(".png") {
        b"\x89PNG\r\n\x1a\nfixture"
    } else if path.ends_with(".obj") {
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"
    } else if path.ends_with(".py") {
        b"# Shape Lab test script\nimport bpy\n"
    } else if path.ends_with("package-verification.json") {
        br#"{"checksums_match":true,"topology_matches_manifest":true,"finite_numeric_payloads":true}"#
    } else if path.ends_with("asset-manifest.json") {
        br#"{"schema_version":1,"files":{},"parts":[]}"#
    } else {
        b"fixture"
    }
}

#[test]
fn static_prop_package_ready_when_required_evidence_and_files_are_present() {
    let package = valid_static_prop_package();
    let temp = tempfile::tempdir().expect("tempdir");
    create_static_prop_artifact_files(temp.path(), &package);

    let report = validate_static_prop_game_ready_package_with_root(&package, temp.path());

    assert!(report.is_ready(), "{report:#?}");
    assert_eq!(report.status, StaticPropReadinessStatus::Ready);
    assert!(report.warnings.iter().any(|warning| {
        warning.code == "material_payload_policy_only" && warning.message.contains("policy only")
    }));
}

#[test]
fn static_prop_package_blocks_manifest_only_readiness() {
    let package = valid_static_prop_package();

    let report = validate_static_prop_game_ready_package(&package);
    let blocker_codes = report
        .blockers
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    assert_eq!(report.status, StaticPropReadinessStatus::Blocked);
    assert!(blocker_codes.contains(&"artifact_files_not_verified"));
}

#[test]
fn static_prop_package_blocks_false_game_ready_claims() {
    let mut package = valid_static_prop_package();
    package.uv_policy = UvPolicy {
        status: StaticPropFeatureStatus::NotImplemented,
        required_for_game_ready: true,
        blocker_code: Some("uv_layout_not_implemented".to_owned()),
        explanation: "UV layout is not implemented.".to_owned(),
    };
    package.handoff.glb_artifact = None;
    package.handoff.glb_status = StaticPropFeatureStatus::NotImplemented;
    package.handoff.glb_blocker_code = Some("glb_export_not_implemented".to_owned());
    package.manual_review.status = ManualReviewStatus::Pending;
    package.manual_review.reviewer = None;

    let report = validate_static_prop_game_ready_package(&package);
    let blocker_codes = report
        .blockers
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    assert_eq!(report.status, StaticPropReadinessStatus::Blocked);
    assert!(!report.is_ready());
    assert!(blocker_codes.contains(&"uv_layout_not_implemented"));
    assert!(blocker_codes.contains(&"glb_export_not_implemented"));
    assert!(blocker_codes.contains(&"manual_review_pending"));
}

#[test]
fn static_prop_package_blocks_uv_not_required_for_game_ready() {
    let mut package = valid_static_prop_package();
    package.uv_policy.required_for_game_ready = false;
    let temp = tempfile::tempdir().expect("tempdir");
    create_static_prop_artifact_files(temp.path(), &package);

    let report = validate_static_prop_game_ready_package_with_root(&package, temp.path());
    let blocker_codes = report
        .blockers
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    assert!(blocker_codes.contains(&"uv_not_required_for_game_ready"));
}

#[test]
fn static_prop_lod0_must_be_exact_source_geometry() {
    let mut package = valid_static_prop_package();
    package.lod_policy.levels[0].exact_source_geometry = false;
    let temp = tempfile::tempdir().expect("tempdir");
    create_static_prop_artifact_files(temp.path(), &package);

    let report = validate_static_prop_game_ready_package_with_root(&package, temp.path());
    let blocker_codes = report
        .blockers
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    assert!(blocker_codes.contains(&"lod0_not_exact_source"));
}

#[test]
fn static_prop_lower_lods_must_be_distinct_decreasing_and_non_exact() {
    let mut package = valid_static_prop_package();
    package.lod_policy.levels[2].artifact = package.lod_policy.levels[1].artifact.clone();
    package.lod_policy.levels[2].target_triangle_count =
        package.lod_policy.levels[1].target_triangle_count;
    package.lod_policy.levels[2].exact_source_geometry = true;
    let temp = tempfile::tempdir().expect("tempdir");
    create_static_prop_artifact_files(temp.path(), &package);

    let report = validate_static_prop_game_ready_package_with_root(&package, temp.path());
    let blocker_codes = report
        .blockers
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    assert!(blocker_codes.contains(&"duplicate_lod_artifact"));
    assert!(blocker_codes.contains(&"lod_triangle_targets_not_decreasing"));
    assert!(blocker_codes.contains(&"lower_lod_marked_exact_source"));
}
