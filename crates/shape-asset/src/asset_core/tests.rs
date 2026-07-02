
#[cfg(test)]
mod tests {
    use super::*;

    fn test_recipe() -> AssetRecipe {
        let definition_id = PartDefinitionId(1);
        let instance_id = PartInstanceId(1);
        let parameter_id = ParameterId(1);
        let source = GeometrySource::RoundedBox {
            half_extents: [1.0, 0.5, 0.25],
            radius: 0.1,
        };
        let definition = PartDefinition {
            id: definition_id,
            name: "Body".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source,
                operations: vec![ModelingOperationSpec::LinearArray {
                    operation: OperationId(1),
                    count: 2,
                    offset: [1.0, 0.0, 0.0],
                }],
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        };
        let instance = PartInstance {
            id: instance_id,
            definition: definition_id,
            name: "Body".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let descriptor = ParameterDescriptor {
            id: parameter_id,
            path: definition_scalar_path(definition_id, "geometry.rounded_box.radius"),
            label: "Radius".to_owned(),
            group: "Form".to_owned(),
            minimum: 0.0,
            maximum: 1.0,
            step: 0.01,
            mutation_sigma: 0.05,
            topology_changing: false,
            beginner_description: "Corner radius".to_owned(),
        };
        let mut recipe = AssetRecipe::new(AssetId(1), "Contract");
        recipe.definitions.insert(definition_id, definition);
        recipe.instances.insert(instance_id, instance);
        recipe.root_instances.push(instance_id);
        recipe.parameters.insert(parameter_id, descriptor);
        recipe.next_ids.part_definition = 2;
        recipe.next_ids.part_instance = 2;
        recipe.next_ids.operation = 2;
        recipe.next_ids.parameter = 2;
        recipe
    }

    fn socket(id: SocketId, name: &str) -> SocketSpec {
        SocketSpec {
            id,
            name: name.to_owned(),
            local_frame: Frame3::default(),
            role: "mount".to_owned(),
            tags: BTreeSet::new(),
        }
    }

    fn multipart_recipe() -> AssetRecipe {
        let mut recipe = test_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("body definition should exist")
            .sockets
            .insert(SocketId(1), socket(SocketId(1), "body_mount"));

        let mut wheel_sockets = BTreeMap::new();
        wheel_sockets.insert(SocketId(2), socket(SocketId(2), "wheel_mount"));
        let wheel_definition = PartDefinition {
            id: PartDefinitionId(2),
            name: "Wheel".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source: GeometrySource::Cylinder {
                    radius: 0.25,
                    height: 0.2,
                    radial_segments: 16,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: wheel_sockets,
            local_pivot: Frame3::default(),
            variant_group: Some("wheel".to_owned()),
            production_hints: None,
        };
        let wheel_instance = PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(2),
            name: "Wheel L".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3::default(),
            attachment: Some(AttachmentSpec {
                parent_instance: PartInstanceId(1),
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                local_offset: Transform3::default(),
                mode: AttachmentMode::RigidSeparate,
            }),
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };

        recipe
            .definitions
            .insert(wheel_definition.id, wheel_definition);
        recipe.instances.insert(wheel_instance.id, wheel_instance);
        recipe.next_ids.part_definition = 3;
        recipe.next_ids.part_instance = 3;
        recipe.next_ids.socket = 3;
        recipe
            .variation
            .optional_instances
            .insert(PartInstanceId(2));
        recipe.variation.replacement_groups.insert(
            "wheel".to_owned(),
            ReplacementGroupHint {
                definitions: BTreeSet::from([PartDefinitionId(2)]),
            },
        );
        recipe.variation.count_ranges.insert(
            OperationId(1),
            CountRangeHint {
                minimum: 1,
                maximum: 6,
            },
        );
        recipe.variation.parameter_range_overrides.insert(
            ParameterId(1),
            ParameterRangeOverride {
                minimum: 0.0,
                maximum: 0.5,
                step: Some(0.01),
                mutation_sigma: None,
            },
        );
        recipe
    }

    fn issue_codes(report: &AssetValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    fn relationship_contract(
        id: RelationshipId,
        parent: PartInstanceId,
        child: PartInstanceId,
    ) -> RelationshipContract {
        RelationshipContract {
            id,
            relationship_type: RelationshipType::SurfaceMounted,
            parent: Some(parent),
            child: Some(child),
            parent_node_ref: None,
            child_node_ref: None,
            parent_anchor_id: None,
            child_anchor_id: None,
            label: "mounted child".to_owned(),
            export_profile: None,
            placement_policy: PlacementPolicy::default(),
            orientation_policy: OrientationPolicy::default(),
            scale_policy: ScalePolicy::default(),
            contact_policy: ContactPolicy::default(),
            edit_policy: RelationshipEditPolicy::default(),
            selection_policy: SelectionPolicy::default(),
            reset_policy: ResetPolicy::default(),
            export_realization: ExportRealizationPolicy::default(),
        }
    }

    fn linear_pattern_contract(id: PatternId, source_instance: PartInstanceId) -> PatternContract {
        PatternContract {
            id,
            pattern_type: PatternType::Linear,
            source_instance: Some(source_instance),
            count: Some(3),
            label: "repeat detail".to_owned(),
            count_policy: PatternCountPolicy::Exact(3),
            density_policy: None,
            export_instancing: PatternExportInstancingPolicy::default(),
            linear_axis: Some(PatternAxis::X),
            spacing: Some(0.25),
            generated_id_policy: GeneratedIdPolicy::PatternOccurrenceIndex,
        }
    }

    #[test]
    fn serde_json_round_trip_preserves_ordered_recipe() {
        let recipe = test_recipe();

        let json = serde_json::to_string(&recipe).expect("recipe should serialize");
        let round_tripped: AssetRecipe =
            serde_json::from_str(&json).expect("recipe should deserialize");

        assert_eq!(recipe, round_tripped);
    }

    #[test]
    fn asset_recipe_v8_empty_semantic_shells_validate() {
        let recipe = AssetRecipe::new(AssetId(9), "V8 semantic shells");

        assert_eq!(recipe.schema_version, 8);
        assert_eq!(recipe.semantic.review_state, ReviewState::default());
        assert_eq!(recipe.semantic.export_includes, ExportIncludes::default());
        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn schema_seven_recipe_migrates_to_v8_empty_semantic_shells() {
        let recipe = test_recipe();
        let mut value = serde_json::to_value(&recipe).expect("recipe should serialize");
        value["schema_version"] = serde_json::json!(7);
        value.as_object_mut().unwrap().remove("semantic");
        for key in [
            "relationship",
            "pattern",
            "surface_slot",
            "material_slot",
            "collision_body",
            "motion_channel",
            "terrain_patch",
            "export_profile",
            "authoring_op",
            "validation_report",
        ] {
            value["next_ids"].as_object_mut().unwrap().remove(key);
        }

        let migrated: AssetRecipe =
            serde_json::from_value(value).expect("schema 7 recipe should migrate");

        assert_eq!(migrated.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
        assert_eq!(migrated.semantic, AssetRecipeSemanticShells::default());
        assert!(validate_asset_recipe(&migrated).is_valid());
    }

    #[test]
    fn asset_recipe_v8_semantic_shells_round_trip_deterministically() {
        let mut recipe = multipart_recipe();
        recipe.semantic.relationships.insert(
            RelationshipId(1),
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2)),
        );
        recipe.semantic.patterns.insert(
            PatternId(1),
            linear_pattern_contract(PatternId(1), PartInstanceId(2)),
        );
        recipe.next_ids.relationship = 2;
        recipe.next_ids.pattern = 2;

        let first = serde_json::to_string(&recipe).expect("recipe serializes");
        let round_tripped: AssetRecipe = serde_json::from_str(&first).expect("recipe parses");
        let second = serde_json::to_string(&round_tripped).expect("recipe serializes");

        assert_eq!(first, second);
        assert_eq!(recipe, round_tripped);
        assert!(validate_asset_recipe(&round_tripped).is_valid());
    }

    #[test]
    fn semantic_shell_validation_rejects_unknown_references() {
        let mut recipe = test_recipe();
        recipe.semantic.relationships.insert(
            RelationshipId(1),
            RelationshipContract {
                parent: Some(PartInstanceId(404)),
                export_profile: Some(ExportProfileId(404)),
                ..relationship_contract(RelationshipId(1), PartInstanceId(404), PartInstanceId(1))
            },
        );
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                source_instance: Some(PartInstanceId(405)),
                count: Some(0),
                count_policy: PatternCountPolicy::Exact(0),
                ..linear_pattern_contract(PatternId(1), PartInstanceId(405))
            },
        );
        recipe.semantic.material_slots.insert(
            MaterialSlotId(1),
            MaterialSlotShell {
                id: MaterialSlotId(1),
                surface_slot: Some(SurfaceSlotId(404)),
                label: String::new(),
            },
        );
        recipe.semantic.authoring_ops.insert(
            AuthoringOpId(1),
            AuthoringOpShell {
                id: AuthoringOpId(1),
                target_parameter: Some(ParameterId(404)),
                target_instance: Some(PartInstanceId(406)),
                label: String::new(),
            },
        );
        recipe.next_ids.relationship = 2;
        recipe.next_ids.pattern = 2;
        recipe.next_ids.material_slot = 2;
        recipe.next_ids.authoring_op = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unknown_semantic_relationship_parent"));
        assert!(codes.contains("unknown_semantic_relationship_export_profile"));
        assert!(codes.contains("unknown_semantic_pattern_source"));
        assert!(codes.contains("invalid_semantic_pattern_count"));
        assert!(codes.contains("unknown_semantic_material_surface_slot"));
        assert!(codes.contains("unknown_semantic_authoring_parameter"));
        assert!(codes.contains("unknown_semantic_authoring_instance"));
    }

    #[test]
    fn semantic_shell_validation_rejects_product_claims() {
        let mut recipe = test_recipe();
        recipe.semantic.review_state = ReviewState {
            tier: ReviewTier::Published,
            human_review_required: false,
            publish_allowed: true,
            public_catalog_visible: true,
        };
        recipe.semantic.export_profiles.insert(
            ExportProfileId(1),
            ExportProfileShell {
                id: ExportProfileId(1),
                label: "invalid export".to_owned(),
                includes: ExportIncludes {
                    includes_geometry: true,
                    includes_textures: true,
                    includes_collision: true,
                    game_ready: true,
                    ..ExportIncludes::default()
                },
            },
        );
        recipe.next_ids.export_profile = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unsupported_semantic_review_tier"));
        assert!(codes.contains("semantic_human_review_required_false"));
        assert!(codes.contains("semantic_publish_allowed"));
        assert!(codes.contains("semantic_public_catalog_visible"));
        assert!(codes.contains("unsupported_semantic_export_include"));
        assert!(codes.contains("semantic_game_ready_claim"));
    }

    #[test]
    fn relationship_contract_accepts_fixed_and_proportional_placement() {
        let mut recipe = multipart_recipe();
        let mut fixed =
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2));
        fixed.parent_node_ref = Some("panel".to_owned());
        fixed.child_node_ref = Some("knob".to_owned());
        fixed.parent_anchor_id = Some("front_handle_zone".to_owned());
        fixed.child_anchor_id = Some("back_mount_point".to_owned());
        fixed.placement_policy = PlacementPolicy {
            position_rule: PositionRule::FixedOffsetFromEdge {
                edge: "right".to_owned(),
                offset: [0.1, 0.0, 0.0],
            },
        };
        fixed.contact_policy = ContactPolicy::SurfaceContact { clearance: 0.0 };
        let mut proportional =
            relationship_contract(RelationshipId(2), PartInstanceId(1), PartInstanceId(2));
        proportional.placement_policy = PlacementPolicy {
            position_rule: PositionRule::ProportionalUv { u: 0.5, v: 0.25 },
        };
        proportional.scale_policy = ScalePolicy::ClampToRange {
            minimum: 0.5,
            maximum: 2.0,
        };
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(1), fixed);
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(2), proportional);
        recipe.next_ids.relationship = 3;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn relationship_contract_rejects_cycles_and_invalid_domains() {
        let mut recipe = multipart_recipe();
        let mut first =
            relationship_contract(RelationshipId(1), PartInstanceId(1), PartInstanceId(2));
        first.parent_anchor_id = Some("Front Handle Zone".to_owned());
        first.placement_policy = PlacementPolicy {
            position_rule: PositionRule::ProportionalUv { u: 2.0, v: 0.5 },
        };
        first.scale_policy = ScalePolicy::ClampToRange {
            minimum: 2.0,
            maximum: 1.0,
        };
        first.contact_policy = ContactPolicy::IntentionalGap { clearance: -0.1 };
        recipe
            .semantic
            .relationships
            .insert(RelationshipId(1), first);
        recipe.semantic.relationships.insert(
            RelationshipId(2),
            relationship_contract(RelationshipId(2), PartInstanceId(2), PartInstanceId(1)),
        );
        recipe.next_ids.relationship = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("value_out_of_range"));
        assert!(codes.contains("invalid_relationship_scale_range"));
        assert!(codes.contains("negative_value"));
        assert!(codes.contains("semantic_relationship_cycle"));
        assert!(codes.contains("invalid_semantic_relationship_parent_anchor"));
    }

    #[test]
    fn pattern_contract_accepts_valid_linear_pattern() {
        let mut recipe = multipart_recipe();
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                count_policy: PatternCountPolicy::Range {
                    minimum: 2,
                    maximum: 6,
                },
                density_policy: Some(PatternDensityPolicy::Range {
                    minimum: 0.0,
                    maximum: 1.0,
                }),
                export_instancing: PatternExportInstancingPolicy::Disabled,
                ..linear_pattern_contract(PatternId(1), PartInstanceId(2))
            },
        );
        recipe.next_ids.pattern = 2;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn pattern_contract_rejects_invalid_count_and_density() {
        let mut recipe = multipart_recipe();
        recipe.semantic.patterns.insert(
            PatternId(1),
            PatternContract {
                count_policy: PatternCountPolicy::Range {
                    minimum: 0,
                    maximum: 20_000,
                },
                density_policy: Some(PatternDensityPolicy::Range {
                    minimum: 3.0,
                    maximum: 1.0,
                }),
                ..linear_pattern_contract(PatternId(1), PartInstanceId(2))
            },
        );
        recipe.next_ids.pattern = 2;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("invalid_semantic_pattern_count"));
        assert!(codes.contains("invalid_semantic_pattern_density_range"));
    }

    #[test]
    fn asset_recipe_v8_fixtures_parse_migrate_and_validate() {
        for (name, raw) in [
            (
                "old_minimal_asset_recipe_v7.json",
                include_str!("../../../../fixtures/shape-asset/old_minimal_asset_recipe_v7.json"),
            ),
            (
                "new_minimal_asset_recipe_v8_shell.json",
                include_str!(
                    "../../../../fixtures/shape-asset/new_minimal_asset_recipe_v8_shell.json"
                ),
            ),
            (
                "box_like_asset_recipe_v8.json",
                include_str!("../../../../fixtures/shape-asset/box_like_asset_recipe_v8.json"),
            ),
            (
                "panel_knob_like_asset_recipe_v8.json",
                include_str!("../../../../fixtures/shape-asset/panel_knob_like_asset_recipe_v8.json"),
            ),
        ] {
            let recipe: AssetRecipe =
                serde_json::from_str(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(
                recipe.schema_version, ASSET_RECIPE_SCHEMA_VERSION,
                "{name} should parse or migrate to current schema"
            );
            assert!(
                validate_asset_recipe(&recipe).is_valid(),
                "{name} should validate"
            );
            assert!(!recipe.semantic.review_state.publish_allowed);
            assert!(!recipe.semantic.review_state.public_catalog_visible);
            assert!(!recipe.semantic.export_includes.game_ready);
        }
    }

    #[test]
    fn schema_one_relationships_migrate_to_schema_two() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: Some(0.001),
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::specific(PartInstanceId(1)),
                second: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                reason: "legacy authored contact".to_owned(),
            });
        let mut value = serde_json::to_value(&recipe).expect("recipe should serialize");
        value["schema_version"] = serde_json::json!(1);
        value["relationships"] = serde_json::json!([
            {
                "SocketAttached": {
                    "parent": 1,
                    "child": 2,
                    "socket": 7,
                    "max_origin_distance": 0.001,
                    "max_axis_angle_degrees": 1.0,
                    "max_clearance": 0.001
                }
            },
            {
                "MayOverlap": {
                    "first": 1,
                    "second": 2,
                    "reason": "legacy authored contact"
                }
            }
        ]);

        let migrated: AssetRecipe =
            serde_json::from_value(value).expect("schema one recipe should migrate");

        assert_eq!(migrated.schema_version, ASSET_RECIPE_SCHEMA_VERSION);
        assert!(matches!(
            &migrated.relationships[0],
            AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(1)
                },
                child: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(2)
                },
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(7),
                child_socket: SocketId(7),
                ..
            }
        ));
        assert!(matches!(
            &migrated.relationships[1],
            AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(1)
                },
                second: AssetPartSelector::SpecificInstance {
                    instance: PartInstanceId(2)
                },
                pairing: RelationshipPairing::AllPairs,
                ..
            }
        ));
        let saved = serde_json::to_value(&migrated).expect("migrated recipe should serialize");
        assert_eq!(
            saved["schema_version"],
            serde_json::json!(ASSET_RECIPE_SCHEMA_VERSION)
        );
        assert!(
            saved["relationships"][0]["SocketAttached"]
                .get("socket")
                .is_none()
        );
    }

    #[test]
    fn validation_accepts_minimal_valid_recipe() {
        let recipe = test_recipe();

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_rejects_invalid_semantic_cut_groups() {
        let mut recipe = test_recipe();
        recipe.variation.semantic_cut_groups.insert(
            "body_rows".to_owned(),
            SemanticCutGroupHint {
                label: String::new(),
                definition: PartDefinitionId(1),
                operations: vec![OperationId(1), OperationId(1), OperationId(99)],
                role: CutGroupRole::Vents,
                count_range: Some(CountRangeHint {
                    minimum: 0,
                    maximum: 1,
                }),
            },
        );

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("empty_semantic_cut_group_label"));
        assert!(codes.contains("duplicate_semantic_cut_group_operation"));
        assert!(codes.contains("invalid_semantic_cut_group_operation"));
        assert!(codes.contains("unknown_semantic_cut_group_operation"));
        assert!(codes.contains("semantic_cut_group_count_range_too_small"));
    }

    #[test]
    fn validation_rejects_phase_inverted_loaded_operations() {
        let mut recipe = test_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist")
            .geometry
            .operations
            .push(ModelingOperationSpec::SetBevelProfile {
                operation: OperationId(2),
                radius: 0.02,
                segments: 1,
            });
        recipe.next_ids.operation = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("invalid_operation_phase_order"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_unsupported_semantic_cut_hosts() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Cylinder {
            radius: 0.5,
            height: 1.0,
            radial_segments: 16,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(2),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::Hard,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("unsupported_semantic_cut_host"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_multi_face_rounded_box_cut_sets() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::Hard,
            },
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(3),
                region: RegionId(1),
                face: PlanarCutFace::PositiveZ,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(3),
                exit_loop: BoundaryLoopId(4),
                outer_region: RegionId(1),
                rim_region: RegionId(4),
                wall_region: RegionId(5),
                edge_treatment: CutEdgeTreatment::Hard,
            },
        ];
        recipe.next_ids.operation = 4;
        recipe.next_ids.region = 6;
        recipe.next_ids.boundary_loop = 5;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("unsupported_rounded_box_cut_face_set"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_duplicate_direct_boundary_loop_outputs() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(1),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::Hard,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 2;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("duplicate_direct_boundary_loop_output"),
            "{report:?}"
        );
    }

    #[test]
    fn validation_rejects_out_of_range_boundary_bevel_profile() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![
            ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::BevelEligible,
            },
            ModelingOperationSpec::BevelBoundaryLoop {
                operation: OperationId(3),
                target_loop: BoundaryLoopId(1),
                width: 0.01,
                segments: 2,
                profile: 100.0,
                bevel_region: RegionId(4),
                outer_replacement_loop: BoundaryLoopId(3),
                inner_replacement_loop: BoundaryLoopId(4),
            },
        ];
        recipe.next_ids.operation = 4;
        recipe.next_ids.region = 5;
        recipe.next_ids.boundary_loop = 5;

        let report = validate_asset_recipe(&recipe);

        assert!(
            issue_codes(&report).contains("value_out_of_range"),
            "{report:?}"
        );
    }

    #[test]
    fn inserting_boundary_bevel_rejects_reused_generated_region() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.geometry.source = GeometrySource::Plate {
            size: [1.0, 1.0],
            thickness: 0.1,
        };
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition.geometry.operations = vec![ModelingOperationSpec::CircularThroughCut {
            operation: OperationId(2),
            region: RegionId(1),
            face: PlanarCutFace::PositiveY,
            center: [0.0, 0.0],
            radius: 0.08,
            radial_segments: 12,
            rim_width: 0.03,
            entry_loop: BoundaryLoopId(1),
            exit_loop: BoundaryLoopId(2),
            outer_region: RegionId(1),
            rim_region: RegionId(2),
            wall_region: RegionId(3),
            edge_treatment: CutEdgeTreatment::BevelEligible,
        }];
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let result = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "reuse bevel region".to_owned(),
                seed: 9,
                operations: vec![AssetEdit::InsertModelingOperation {
                    definition: PartDefinitionId(1),
                    index: 1,
                    operation: ModelingOperationSpec::BevelBoundaryLoop {
                        operation: OperationId(3),
                        target_loop: BoundaryLoopId(1),
                        width: 0.01,
                        segments: 2,
                        profile: 1.0,
                        bevel_region: RegionId(3),
                        outer_replacement_loop: BoundaryLoopId(3),
                        inner_replacement_loop: BoundaryLoopId(4),
                    },
                }],
            },
        );

        assert!(
            matches!(result, Err(AssetError::UnsupportedEdit(ref message)) if message.contains("duplicate generated region")),
            "{result:?}"
        );
    }

    #[test]
    fn validation_rejects_cut_group_role_and_count_mismatch() {
        let mut recipe = test_recipe();
        let definition = recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("definition should exist");
        definition.regions.insert(
            RegionId(1),
            SurfaceRegionSpec {
                id: RegionId(1),
                name: "front".to_owned(),
                role: SurfaceRole::PrimarySurface,
                tags: BTreeSet::new(),
            },
        );
        definition
            .geometry
            .operations
            .push(ModelingOperationSpec::CircularThroughCut {
                operation: OperationId(2),
                region: RegionId(1),
                face: PlanarCutFace::PositiveY,
                center: [0.0, 0.0],
                radius: 0.08,
                radial_segments: 12,
                rim_width: 0.03,
                entry_loop: BoundaryLoopId(1),
                exit_loop: BoundaryLoopId(2),
                outer_region: RegionId(1),
                rim_region: RegionId(2),
                wall_region: RegionId(3),
                edge_treatment: CutEdgeTreatment::Hard,
            });
        recipe.variation.semantic_cut_groups.insert(
            "vents".to_owned(),
            SemanticCutGroupHint {
                label: "Vents".to_owned(),
                definition: PartDefinitionId(1),
                operations: vec![OperationId(2)],
                role: CutGroupRole::Vents,
                count_range: Some(CountRangeHint {
                    minimum: 2,
                    maximum: 4,
                }),
            },
        );
        recipe.next_ids.operation = 3;
        recipe.next_ids.region = 4;
        recipe.next_ids.boundary_loop = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("semantic_cut_group_role_mismatch"));
        assert!(codes.contains("semantic_cut_group_count_out_of_range"));
    }

    #[test]
    fn set_scalar_uses_descriptor_path() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "resize".to_owned(),
            seed: 7,
            operations: vec![AssetEdit::SetScalar {
                parameter: ParameterId(1),
                value: 0.2,
            }],
        };

        let edited = apply_edit_program(&recipe, &program).expect("edit should apply");

        assert_eq!(
            get_scalar(
                &edited,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.2
        );
    }

    #[test]
    fn locked_parameter_rejects_edit_atomically() {
        let mut recipe = test_recipe();
        recipe.locks.insert(ParameterId(1));
        let program = AssetEditProgram {
            label: "locked".to_owned(),
            seed: 1,
            operations: vec![AssetEdit::SetScalar {
                parameter: ParameterId(1),
                value: 0.3,
            }],
        };

        assert!(matches!(
            apply_edit_program(&recipe, &program),
            Err(AssetError::LockedParameter(ParameterId(1)))
        ));
        assert_eq!(
            get_scalar(
                &recipe,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.1
        );
    }

    #[test]
    fn descendants_and_definition_instances_are_stable() {
        let mut recipe = test_recipe();
        let child = PartInstance {
            id: PartInstanceId(2),
            definition: PartDefinitionId(1),
            name: "Child".to_owned(),
            parent: Some(PartInstanceId(1)),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        recipe.instances.insert(child.id, child);
        recipe.next_ids.part_instance = 3;

        assert_eq!(
            descendants_of(&recipe, PartInstanceId(1)).expect("root should exist"),
            vec![PartInstanceId(2)]
        );
        assert_eq!(
            instances_of_definition(&recipe, PartDefinitionId(1)),
            vec![PartInstanceId(1), PartInstanceId(2)]
        );
    }

    #[test]
    fn set_array_count_targets_array_operations() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "array count".to_owned(),
            seed: 2,
            operations: vec![AssetEdit::SetArrayCount {
                definition: PartDefinitionId(1),
                operation: OperationId(1),
                count: 4,
            }],
        };

        let edited = apply_edit_program(&recipe, &program).expect("array edit should apply");

        let definition = edited
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist");
        assert!(matches!(
            definition.geometry.operations.as_slice(),
            [ModelingOperationSpec::LinearArray { count: 4, .. }]
        ));
    }

    #[test]
    fn validation_accepts_valid_multipart_assembly() {
        let recipe = multipart_recipe();

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_accepts_relationship_selectors_and_separate_sockets() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(1),
                child_socket: SocketId(2),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: Some(0.001),
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MayOverlap {
                first: AssetPartSelector::PrototypeAndGeneratedOccurrences {
                    prototype: PartInstanceId(1),
                },
                second: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(1),
                },
                pairing: RelationshipPairing::AllPairs,
                reason: "arrayed prototype contacts are authored".to_owned(),
            });
        recipe
            .definitions
            .get_mut(&PartDefinitionId(2))
            .expect("wheel definition should exist")
            .tags
            .insert("support".to_owned());
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MustTouch {
                first: AssetPartSelector::DefinitionRole {
                    role: "support".to_owned(),
                },
                second: AssetPartSelector::specific(PartInstanceId(1)),
                pairing: RelationshipPairing::AllPairs,
                max_clearance: 0.02,
            });

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_reports_unknown_relationship_selectors_and_sockets() {
        let mut recipe = multipart_recipe();
        recipe
            .relationships
            .push(AssetRelationshipPolicy::SocketAttached {
                parent: AssetPartSelector::specific(PartInstanceId(1)),
                child: AssetPartSelector::specific(PartInstanceId(2)),
                pairing: RelationshipPairing::AllPairs,
                parent_socket: SocketId(99),
                child_socket: SocketId(98),
                max_origin_distance: 0.001,
                max_axis_angle_degrees: 1.0,
                max_clearance: None,
            });
        recipe
            .relationships
            .push(AssetRelationshipPolicy::MinimumClearance {
                first: AssetPartSelector::GeneratedByOperation {
                    operation: OperationId(99),
                },
                second: AssetPartSelector::PartTag {
                    tag: "missing".to_owned(),
                },
                pairing: RelationshipPairing::AllPairs,
                clearance: 0.01,
            });

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("unknown_relationship_parent_socket"));
        assert!(codes.contains("unknown_relationship_child_socket"));
        assert!(codes.contains("unknown_relationship_operation"));
        assert!(codes.contains("unknown_relationship_selector"));
    }

    #[test]
    fn validation_accepts_reused_part_definition() {
        let mut recipe = test_recipe();
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(1),
                name: "Body copy".to_owned(),
                parent: Some(PartInstanceId(1)),
                local_transform: Transform3::default(),
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.next_ids.part_instance = 3;

        assert!(validate_asset_recipe(&recipe).is_valid());
    }

    #[test]
    fn validation_reports_hierarchy_cycle() {
        let mut recipe = test_recipe();
        recipe.root_instances.clear();
        recipe
            .instances
            .get_mut(&PartInstanceId(1))
            .expect("root should exist")
            .parent = Some(PartInstanceId(2));
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(1),
                name: "Cycle".to_owned(),
                parent: Some(PartInstanceId(1)),
                local_transform: Transform3::default(),
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.next_ids.part_instance = 3;

        let report = validate_asset_recipe(&recipe);

        assert!(issue_codes(&report).contains("parent_cycle"));
    }

    #[test]
    fn validation_reports_dangling_socket() {
        let mut recipe = multipart_recipe();
        recipe
            .definitions
            .get_mut(&PartDefinitionId(1))
            .expect("body definition should exist")
            .sockets
            .remove(&SocketId(1));

        let report = validate_asset_recipe(&recipe);

        assert!(issue_codes(&report).contains("unknown_parent_socket"));
    }

    #[test]
    fn validation_reports_invalid_attachment() {
        let mut recipe = multipart_recipe();
        let wheel = recipe
            .instances
            .get_mut(&PartInstanceId(2))
            .expect("wheel instance should exist");
        wheel.parent = None;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(codes.contains("attachment_parent_mismatch"));
        assert!(codes.contains("missing_root_instance"));
    }

    #[test]
    fn scalar_get_set_round_trip() {
        let mut recipe = test_recipe();
        let path = definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius");

        set_scalar(&mut recipe, &path, 0.4).expect("scalar edit should apply");

        assert_eq!(
            get_scalar(&recipe, &path).expect("scalar should exist"),
            0.4
        );
    }

    #[test]
    fn failed_validation_keeps_edit_program_atomic() {
        let recipe = test_recipe();
        let program = AssetEditProgram {
            label: "bad transform".to_owned(),
            seed: 3,
            operations: vec![
                AssetEdit::SetScalar {
                    parameter: ParameterId(1),
                    value: 0.2,
                },
                AssetEdit::SetTransform {
                    instance: PartInstanceId(1),
                    transform: Transform3 {
                        scale: [0.0, 1.0, 1.0],
                        ..Transform3::default()
                    },
                },
            ],
        };

        assert!(matches!(
            apply_edit_program(&recipe, &program),
            Err(AssetError::ValidationFailed(report))
                if issue_codes(&report).contains("zero_scale")
        ));
        assert_eq!(
            get_scalar(
                &recipe,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.1
        );
    }

    #[test]
    fn add_and_remove_instance_updates_roots_deterministically() {
        let recipe = test_recipe();
        let added = PartInstance {
            id: PartInstanceId(3),
            definition: PartDefinitionId(1),
            name: "Accessory".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "add".to_owned(),
                seed: 4,
                operations: vec![AssetEdit::AddInstance { instance: added }],
            },
        )
        .expect("add should apply");

        assert_eq!(
            edited.root_instances,
            vec![PartInstanceId(1), PartInstanceId(3)]
        );
        assert_eq!(edited.next_ids.part_instance, 4);

        let removed = apply_edit_program(
            &edited,
            &AssetEditProgram {
                label: "remove".to_owned(),
                seed: 5,
                operations: vec![AssetEdit::RemoveInstance {
                    instance: PartInstanceId(3),
                }],
            },
        )
        .expect("remove should apply");

        assert_eq!(removed.root_instances, vec![PartInstanceId(1)]);
        assert!(!removed.instances.contains_key(&PartInstanceId(3)));
    }

    #[test]
    fn removing_instance_with_descendants_is_rejected() {
        let recipe = multipart_recipe();

        assert!(matches!(
            apply_edit_program(
                &recipe,
                &AssetEditProgram {
                    label: "remove parent".to_owned(),
                    seed: 6,
                    operations: vec![AssetEdit::RemoveInstance {
                        instance: PartInstanceId(1),
                    }],
                },
            ),
            Err(AssetError::UnsupportedEdit(message))
                if message.contains("descendants")
        ));
    }

    #[test]
    fn replace_definition_preserves_instances() {
        let recipe = test_recipe();
        let mut replacement = recipe
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist")
            .clone();
        replacement.name = "Replacement Body".to_owned();
        if let GeometrySource::RoundedBox { radius, .. } = &mut replacement.geometry.source {
            *radius = 0.2;
        }

        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "replace".to_owned(),
                seed: 8,
                operations: vec![AssetEdit::ReplaceDefinition {
                    definition: replacement,
                }],
            },
        )
        .expect("replace should apply");

        assert_eq!(
            edited
                .instances
                .get(&PartInstanceId(1))
                .expect("instance should exist")
                .definition,
            PartDefinitionId(1)
        );
        assert_eq!(
            get_scalar(
                &edited,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("radius should exist"),
            0.2
        );
    }

    #[test]
    fn locked_parameters_remain_inspectable() {
        let mut recipe = test_recipe();
        recipe.locks.insert(ParameterId(1));

        assert_eq!(enumerate_parameters(&recipe).len(), 1);
        assert_eq!(
            get_scalar(
                &recipe,
                definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius")
            )
            .expect("locked parameter should remain readable"),
            0.1
        );
    }

    #[test]
    fn deterministic_serialization_orders_semantic_ids() {
        let mut recipe = test_recipe();
        let mut second = recipe
            .definitions
            .get(&PartDefinitionId(1))
            .expect("definition should exist")
            .clone();
        second.id = PartDefinitionId(2);
        second.name = "Second".to_owned();
        recipe.definitions.insert(second.id, second);
        recipe.next_ids.part_definition = 3;

        let json = serde_json::to_string(&recipe).expect("recipe should serialize");
        let first_position = json.find("\"1\"").expect("id 1 key should serialize");
        let second_position = json.find("\"2\"").expect("id 2 key should serialize");

        assert!(first_position < second_position);
        assert_eq!(
            json,
            serde_json::to_string(&recipe).expect("recipe should serialize deterministically")
        );
    }

    #[test]
    fn unrelated_parameter_edit_preserves_semantic_ids() {
        let recipe = multipart_recipe();
        let definition_ids = recipe.definitions.keys().copied().collect::<Vec<_>>();
        let instance_ids = recipe.instances.keys().copied().collect::<Vec<_>>();
        let operation_ids = recipe.definitions[&PartDefinitionId(1)]
            .geometry
            .operations
            .iter()
            .map(ModelingOperationSpec::operation_id)
            .collect::<Vec<_>>();
        let next_ids = recipe.next_ids.clone();

        let edited = apply_edit_program(
            &recipe,
            &AssetEditProgram {
                label: "radius".to_owned(),
                seed: 9,
                operations: vec![AssetEdit::SetScalar {
                    parameter: ParameterId(1),
                    value: 0.2,
                }],
            },
        )
        .expect("edit should apply");

        assert_eq!(
            edited.definitions.keys().copied().collect::<Vec<_>>(),
            definition_ids
        );
        assert_eq!(
            edited.instances.keys().copied().collect::<Vec<_>>(),
            instance_ids
        );
        assert_eq!(
            edited.definitions[&PartDefinitionId(1)]
                .geometry
                .operations
                .iter()
                .map(ModelingOperationSpec::operation_id)
                .collect::<Vec<_>>(),
            operation_ids
        );
        assert_eq!(edited.next_ids, next_ids);
    }

    #[test]
    fn validation_reports_multiple_issues() {
        let mut recipe = test_recipe();
        recipe.root_instances.push(PartInstanceId(1));
        recipe.instances.insert(
            PartInstanceId(2),
            PartInstance {
                id: PartInstanceId(2),
                definition: PartDefinitionId(99),
                name: "Invalid".to_owned(),
                parent: Some(PartInstanceId(42)),
                local_transform: Transform3 {
                    translation: [f32::NAN, 0.0, 0.0],
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: BTreeSet::new(),
                generated_by: None,
            },
        );
        recipe.parameters.insert(
            ParameterId(2),
            ParameterDescriptor {
                id: ParameterId(2),
                path: "definition.1.geometry.rounded_box.nope".to_owned(),
                label: "Bad".to_owned(),
                group: "Bad".to_owned(),
                minimum: 1.0,
                maximum: 0.0,
                step: 0.0,
                mutation_sigma: -1.0,
                topology_changing: false,
                beginner_description: "Bad".to_owned(),
            },
        );
        recipe
            .variation
            .optional_instances
            .insert(PartInstanceId(404));
        recipe.next_ids.part_instance = 3;
        recipe.next_ids.parameter = 3;

        let report = validate_asset_recipe(&recipe);
        let codes = issue_codes(&report);

        assert!(report.issues.len() >= 7);
        assert!(codes.contains("duplicate_root_instance"));
        assert!(codes.contains("unknown_instance_definition"));
        assert!(codes.contains("unknown_parent_instance"));
        assert!(codes.contains("non_finite"));
        assert!(codes.contains("invalid_parameter_range"));
        assert!(codes.contains("unknown_parameter_path"));
        assert!(codes.contains("unknown_optional_instance"));
    }

    #[test]
    fn parameter_reflection_filters_invalid_descriptors_but_not_locks() {
        let mut recipe = test_recipe();
        recipe.parameters.insert(
            ParameterId(2),
            ParameterDescriptor {
                id: ParameterId(2),
                path: "definition.1.geometry.rounded_box.nope".to_owned(),
                label: "Invalid".to_owned(),
                group: "Form".to_owned(),
                minimum: 0.0,
                maximum: 1.0,
                step: 0.01,
                mutation_sigma: 0.05,
                topology_changing: false,
                beginner_description: "Invalid".to_owned(),
            },
        );
        recipe.locks.insert(ParameterId(1));
        recipe.next_ids.parameter = 3;

        let reflected = enumerate_parameters(&recipe);

        assert_eq!(
            reflected
                .iter()
                .map(|parameter| parameter.id)
                .collect::<Vec<_>>(),
            vec![ParameterId(1)]
        );
    }
}
