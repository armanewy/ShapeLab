use shape_asset::{AssetId, AssetRecipe, Frame3};
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
