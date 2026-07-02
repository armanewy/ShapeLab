use super::*;
use orchard_foundry::compile_foundry_document;

#[test]
fn desktop_foundry_effects_execute_background_jobs() {
    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();
    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if app.state.current_output.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(app.state.current_output.is_some());
    assert!(app.state.current_preview.is_some() || !app.state.active_jobs.is_empty());
}

#[test]
fn save_as_paths_use_loadable_foundry_suffix() {
    let normalized = normalize_foundry_project_path(PathBuf::from("box-primitive.json"));
    assert_eq!(
        normalized,
        PathBuf::from("box-primitive.shapelab-foundry.json")
    );
    ensure_foundry_project_path(&normalized).expect("normalized path is loadable");
}

#[test]
fn desktop_foundry_pack_action_dispatches_through_reducer() {
    let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let app = FoundryDesktopApp {
        state: FoundryAppState::new(fixture.document).expect("fixture state"),
        ..FoundryDesktopApp::default()
    };

    assert!(matches!(
        app.add_current_to_pack_command()
            .and_then(|command| command.single_foundry_command().cloned()),
        Some(orchard_foundry::FoundryCommand::AddCurrentToPack { .. })
    ));
}

#[test]
fn pack_member_ids_increment_for_repeated_ui_adds() {
    let mut pack = crate::foundry::view_model::FoundryPackView::default();
    assert_eq!(
        unique_pack_member_id(&pack, "box-primitive-doc"),
        "box-primitive-doc"
    );

    pack.members.insert(
        "box-primitive-doc".to_owned(),
        orchard_foundry::FoundryDocumentId("box-primitive-doc".to_owned()),
    );
    assert_eq!(
        unique_pack_member_id(&pack, "box-primitive-doc"),
        "box-primitive-doc-2"
    );

    pack.members.insert(
        "box-primitive-doc-2".to_owned(),
        orchard_foundry::FoundryDocumentId("box-primitive-doc-2".to_owned()),
    );
    assert_eq!(
        unique_pack_member_id(&pack, "box-primitive-doc"),
        "box-primitive-doc-3"
    );
}

#[test]
fn product_app_launches_on_choose_home() {
    let app = FoundryDesktopApp::default();

    assert_eq!(app.tab, FoundryTab::Home);
    assert!(app.state.document.is_none());
}

#[test]
fn choose_header_uses_selected_starting_point_title() {
    let flat_panel_app = FoundryDesktopApp {
        selected_home_profile_slug: Some(FLAT_PANEL_PRIMITIVE_PROFILE_ID.to_owned()),
        ..FoundryDesktopApp::default()
    };
    assert_eq!(
        flat_panel_app.current_project_title(),
        "Start with Flat Panel Primitive"
    );

    let panel_knob_app = FoundryDesktopApp {
        selected_home_profile_slug: Some(PANEL_KNOB_PROFILE_ID.to_owned()),
        ..FoundryDesktopApp::default()
    };
    assert_eq!(
        panel_knob_app.current_project_title(),
        "Start with Panel with Knob"
    );
}

#[test]
fn product_home_shows_curated_usable_kits_by_default_and_preview_mode_hides_drafts() {
    assert_eq!(installed_product_kit_count(), 7);
    assert_eq!(default_product_home_profile_count(), 6);

    let default_profiles = product_home_profiles(false);
    let default_labels = default_profiles
        .iter()
        .map(|profile| profile.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(default_profiles[0].fixture.slug, "box-primitive");
    assert_eq!(default_profiles[1].fixture.slug, "lidded-box");
    assert_eq!(default_profiles[2].fixture.slug, "flat-panel-primitive");
    assert_eq!(default_profiles[3].fixture.slug, "sphere-primitive");
    assert_eq!(default_profiles[4].fixture.slug, "hinged-panel");
    assert_eq!(default_profiles[5].fixture.slug, "panel-with-knob");
    assert_eq!(
        default_labels,
        vec![
            "Box Primitive",
            "Lidded Box",
            "Flat Panel Primitive",
            "Sphere Primitive",
            "Hinged Panel",
            "Panel with Knob"
        ]
    );

    let profiles = product_home_profiles(true);
    let labels = profiles
        .iter()
        .map(|profile| profile.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(profiles.len(), 7);
    assert_eq!(
        labels,
        vec![
            "Box Primitive",
            "Lidded Box",
            "Flat Panel Primitive",
            "Sphere Primitive",
            "Hinged Panel",
            "Handled Panel",
            "Panel with Knob"
        ]
    );
    assert!(!labels.iter().any(|label| label.contains("MVP")));
}

#[test]
fn choose_screen_starter_profile_mode_has_no_category_filters_or_catalog_count() {
    let profiles = product_home_profiles(false);
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n");

    assert_eq!(profiles.len(), 6);
    assert_eq!(profiles[0].fixture.slug, BOX_PRIMITIVE_PROFILE_ID);
    assert_eq!(profiles[1].fixture.slug, LIDDED_BOX_PROFILE_ID);
    assert_eq!(profiles[2].fixture.slug, FLAT_PANEL_PRIMITIVE_PROFILE_ID);
    assert_eq!(profiles[3].fixture.slug, SPHERE_PRIMITIVE_PROFILE_ID);
    assert_eq!(profiles[4].fixture.slug, HINGED_PANEL_PROFILE_ID);
    assert_eq!(profiles[5].fixture.slug, PANEL_KNOB_PROFILE_ID);
    assert!(HOME_TEMPLATE_FILTERS.is_empty());
    assert!(strings.contains(&"Choose a starting point"));
    assert!(strings.contains(&"Primitives"));
    assert!(strings.contains(&"Derived from Box Primitive"));
    assert!(strings.contains(&"Derived from Flat Panel Primitive"));
    assert!(strings.contains(&"Preset"));
    assert!(strings.contains(&"A simple box with a visible lid seam."));
    assert!(strings.contains(&"You can vary proportions, edge softness, and lid seam."));
    assert!(
        strings.contains(&"One upright clay panel with readable width, height, and thickness.")
    );
    assert!(
        strings.contains(&"One closed round clay volume with readable dimensions and flattening.")
    );
    assert!(strings.contains(&"One upright clay panel with a visible hinge edge."));
    assert!(strings.contains(&"You can vary proportions, edge softness, and hinge edge."));
    assert!(strings.contains(
        &"One upright clay panel with a bounded knob-like sphere form attached through a safe anchor."
    ));
    assert!(strings.contains(&"You can adjust panel size, knob form, and bounded knob position."));

    for hidden in [
        "Props",
        "Architecture",
        "Gear",
        "Furniture",
        "Environment",
        "18 templates",
        "1 starting point",
        "2 starting points",
        "3 starting points",
        "4 starting points",
        "5 starting points",
        "6 starting points",
        "7 starting points",
        "Search starting point...",
        "Choose what to make",
        "Handled Panel",
        "Historical proof",
    ] {
        assert!(
            !joined.contains(hidden),
            "starter-profile Choose copy should not expose {hidden}: {joined}"
        );
    }
}

#[test]
fn choose_page_provenance_groups_primitives_before_derived_items() {
    let profiles = product_home_profiles(false);
    let groups = product_home_starting_point_groups(&profiles, false);
    let group_names = groups
        .iter()
        .map(|group| group.display_name)
        .collect::<Vec<_>>();

    assert_eq!(
        group_names,
        vec!["Box Primitive", "Flat Panel Primitive", "Sphere Primitive"]
    );
    assert_eq!(groups[0].source_primitive_slug, BOX_PRIMITIVE_PROFILE_ID);
    assert_eq!(groups[0].derived_items[0].display_name, "Lidded Box");
    assert_eq!(
        groups[0].derived_items[0].derived_from_label,
        "Box Primitive"
    );
    assert_eq!(
        groups[0].derived_items[0].derivation_summary,
        "Derived from Box Primitive + Lid Seam."
    );
    assert_eq!(
        groups[1]
            .derived_items
            .iter()
            .map(|item| item.display_name)
            .collect::<Vec<_>>(),
        vec!["Hinged Panel", "Panel with Knob"]
    );
    assert_eq!(
        groups[1].derived_items[0].derived_from_label,
        "Flat Panel Primitive"
    );
    assert_eq!(
        groups[1].derived_items[1].derivation_summary,
        "Derived from Flat Panel Primitive + Sphere attachment."
    );
    assert_eq!(groups[2].derived_items[0].display_name, "Knob-like Form");
    assert!(groups[2].derived_items[0].preset);
    assert!(groups[2].derived_items[0].profile.is_none());
    assert!(
        !groups
            .iter()
            .flat_map(|group| group.derived_items.iter())
            .any(|item| item.display_name == "Handled Panel")
    );
}

#[test]
fn choose_page_preview_mode_contains_historical_proofs_only_internally() {
    let profiles = product_home_profiles(true);
    let groups = product_home_starting_point_groups(&profiles, true);
    let flat_panel_group = groups
        .iter()
        .find(|group| group.display_name == "Flat Panel Primitive")
        .expect("flat panel group");
    let handled = flat_panel_group
        .derived_items
        .iter()
        .find(|item| item.display_name == "Handled Panel")
        .expect("handled panel historical proof");

    assert_eq!(handled.status, StartingPointStatus::HistoricalProof);
    assert_eq!(handled.derived_from_label, "Flat Panel Primitive");
    assert_eq!(
        handled.derivation_summary,
        "Historical proof from Flat Panel + Hinge Edge + Handle."
    );
}

#[test]
fn choose_page_provenance_groups_cover_startable_profiles() {
    let profiles = product_home_profiles(false);
    let groups = product_home_starting_point_groups(&profiles, false);
    let startable_slugs = groups
        .iter()
        .flat_map(|group| {
            group.primitive_profile.iter().chain(
                group
                    .derived_items
                    .iter()
                    .filter_map(|item| item.profile.as_ref()),
            )
        })
        .map(|profile| profile.fixture.slug.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let profile_slugs = profiles
        .iter()
        .map(|profile| profile.fixture.slug.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(startable_slugs, profile_slugs);
    assert_eq!(
        groups
            .iter()
            .map(|group| {
                group.primitive_profile.iter().count()
                    + group
                        .derived_items
                        .iter()
                        .filter(|item| item.profile.is_some())
                        .count()
            })
            .sum::<usize>(),
        profiles.len()
    );
}

#[test]
fn home_template_search_defaults_to_first_matching_profile() {
    let profiles = product_home_profiles(false);
    let selected_slug =
        default_filtered_home_profile_slug(&profiles, "box", HomeTemplateFilter::All);

    assert_eq!(selected_slug.as_deref(), Some("box-primitive"));
}

#[test]
fn home_template_selection_tracks_filter_visibility() {
    let profiles = product_home_profiles(false);
    let mut selected_slug = Some("box-primitive".to_owned());

    normalize_home_selection(&profiles, "", HomeTemplateFilter::All, &mut selected_slug);

    assert_eq!(selected_slug.as_deref(), Some("box-primitive"));
    assert_eq!(
        filtered_home_profile_indices(&profiles, "", HomeTemplateFilter::All)
            .iter()
            .map(|index| profiles[*index].fixture.slug.as_str())
            .collect::<Vec<_>>(),
        vec![
            "box-primitive",
            "lidded-box",
            "flat-panel-primitive",
            "sphere-primitive",
            "hinged-panel",
            "panel-with-knob"
        ]
    );
}

#[test]
fn product_home_grouping_uses_stable_family_ids() {
    let profiles = product_home_profiles(false);
    let groups = product_home_starting_point_groups(&profiles, false);
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].source_primitive_slug, "box-primitive");
    assert_eq!(groups[1].source_primitive_slug, "flat-panel-primitive");
    assert_eq!(groups[2].source_primitive_slug, "sphere-primitive");
    assert_eq!(groups[0].derived_items.len(), 1);
    assert_eq!(groups[1].derived_items.len(), 2);
    assert_eq!(groups[2].derived_items.len(), 1);
}

#[test]
fn preview_draw_size_scales_to_model_centric_stage() {
    assert_eq!(
        scaled_preview_size(128, 128, 420.0).expect("valid preview"),
        egui::vec2(420.0, 420.0)
    );
    assert_eq!(
        scaled_preview_size(512, 256, 256.0).expect("valid preview"),
        egui::vec2(256.0, 128.0)
    );
    assert_eq!(
        scaled_preview_size(128, 64, 320.0).expect("valid preview"),
        egui::vec2(320.0, 160.0)
    );
    assert!(scaled_preview_size(0, 128, 256.0).is_none());
}

#[test]
fn current_preview_viewport_keeps_model_smaller_than_studio_stage() {
    assert_eq!(
        current_preview_viewport_size(960.0, 520.0),
        egui::vec2(960.0, 520.0)
    );
    assert_eq!(
        current_preview_model_image_size(egui::vec2(520.0, 520.0)),
        egui::vec2(468.0, 468.0)
    );
}

#[test]
fn current_preview_defaults_to_studio_stage_without_axis_view() {
    assert_eq!(
        current_preview_default_stage_style(),
        CurrentPreviewStageStyle::Studio
    );
    assert_eq!(
        current_preview_stage_style_for_axis_view(false),
        CurrentPreviewStageStyle::Studio
    );
    assert_eq!(
        current_preview_stage_style_for_axis_view(true),
        CurrentPreviewStageStyle::CoordinateReference
    );
    assert_eq!(make_view_axis_default_tone(), StatusTone::Neutral);
}

#[test]
fn current_preview_stage_hides_refresh_status_when_image_is_available() {
    assert_eq!(
        current_preview_stage_status_message(true, true, false, true),
        None
    );
    assert_eq!(
        current_preview_stage_status_message(true, true, true, true),
        Some(PREVIEW_UPDATING_REASON)
    );
    assert_eq!(
        current_preview_stage_status_message(false, true, false, true),
        Some(PREVIEW_UPDATING_REASON)
    );
    assert_eq!(
        current_preview_stage_status_message(false, true, false, false),
        Some(PREVIEW_PREPARING_REASON)
    );
}

#[test]
fn home_thumbnail_drag_delta_orbits_camera() {
    let camera = OrbitCamera::default();
    let rotated = orbit_home_thumbnail_camera(&camera, egui::vec2(20.0, -10.0));

    assert!((rotated.yaw_degrees - 44.0).abs() < f32::EPSILON);
    assert_eq!(rotated.pitch_degrees, camera.pitch_degrees);
    assert_eq!(rotated.target, camera.target);
    assert_eq!(rotated.distance, camera.distance);
}

#[test]
fn current_preview_drag_orbits_yaw_and_pitch_around_origin() {
    let mut preview = test_preview_image("current");
    preview.camera.pan(1.0, 2.0);
    let rotated = current_preview_orbit_camera(&preview, egui::vec2(10.0, -12.0))
        .expect("nonzero drag should orbit the preview camera");

    assert!((rotated.yaw_degrees - 38.5).abs() < f32::EPSILON);
    assert!((rotated.pitch_degrees - 29.2).abs() < f32::EPSILON);
    assert_ne!(preview.camera.target, OrbitCamera::default().target);
    assert_eq!(rotated.target, OrbitCamera::default().target);
    assert_eq!(rotated.distance, preview.camera.distance);
    assert!(current_preview_orbit_camera(&preview, egui::Vec2::ZERO).is_none());
}

#[test]
fn reset_view_command_restores_authored_preview_camera() {
    let mut preview = test_preview_image("current");
    preview.camera.orbit(18.0, -12.0);

    let command =
        reset_current_preview_view_command(Some(&preview)).expect("reset command available");
    let FoundryAppCommand::RequestPreview {
        width,
        height,
        camera: Some(camera),
    } = command
    else {
        panic!("reset view should request a preview with the default camera");
    };

    assert_eq!(width, preview.width);
    assert_eq!(height, preview.height);
    assert_eq!(camera.target, OrbitCamera::default().target);
    assert_eq!(camera.distance, OrbitCamera::default().distance);
    assert_eq!(camera.yaw_degrees, OrbitCamera::default().yaw_degrees);
    assert_eq!(camera.pitch_degrees, OrbitCamera::default().pitch_degrees);
    assert!(reset_current_preview_view_command(None).is_none());
}

#[test]
fn current_preview_orbit_drag_accumulates_vertical_motion_from_drag_start() {
    let mut preview = test_preview_image("current");
    preview.camera.distance = 7.5;
    preview.camera.pitch_degrees = 20.0;
    let mut orbit = CurrentPreviewOrbitState::default();

    let first = orbit
        .camera_for_drag_delta(&preview, true, egui::vec2(0.0, 6.0))
        .expect("vertical drag should orbit the preview camera");
    let mut updated_preview = preview.clone();
    updated_preview.camera = first;
    let second = orbit
        .camera_for_drag_delta(&updated_preview, true, egui::vec2(0.0, 18.0))
        .expect("continued vertical drag should keep orbiting from drag start");

    let expected_pitch =
        preview.camera.pitch_degrees - 18.0 * CURRENT_PREVIEW_ORBIT_DEGREES_PER_POINT;
    assert!((second.pitch_degrees - expected_pitch).abs() < 1.0e-5);
    assert_eq!(second.yaw_degrees, preview.camera.yaw_degrees);
    assert_eq!(second.target, OrbitCamera::default().target);
    assert_eq!(second.distance, preview.camera.distance);
    assert!(
        orbit
            .camera_for_drag_delta(&updated_preview, false, egui::Vec2::ZERO)
            .is_none()
    );
}

#[test]
fn current_preview_orbit_drag_uses_secondary_button() {
    assert_eq!(
        current_preview_orbit_button(),
        egui::PointerButton::Secondary
    );
}

#[test]
fn home_turntable_frame_index_wraps_yaw() {
    assert_eq!(home_turntable_frame_index(0.0), 0);
    assert_eq!(home_turntable_frame_index(359.0), 0);
    assert_eq!(home_turntable_frame_index(15.0), 1);
    assert_eq!(home_turntable_frame_index(-15.0), 23);
    assert_eq!(home_turntable_frame_distance(0, 23), 1);
}

#[test]
fn current_preview_pixels_are_dpi_aware_and_capped() {
    assert_eq!(
        current_preview_pixels_for_scale(1.0),
        DEFAULT_PREVIEW_PIXELS
    );
    assert_eq!(
        current_preview_pixels_for_scale(1.5),
        DEFAULT_PREVIEW_PIXELS
    );
    assert_eq!(
        current_preview_pixels_for_scale(0.5),
        DEFAULT_PREVIEW_PIXELS
    );
    assert_eq!(
        current_preview_pixels_for_scale(4.0),
        MAX_CURRENT_PREVIEW_PIXELS
    );
}

#[test]
fn default_product_strings_hide_legacy_and_technical_surfaces() {
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n");
    let joined_lower = joined.to_ascii_lowercase();

    for forbidden in [
        "Legacy",
        "Implicit",
        "Asset Modeling Lab",
        "Modeling Workspace",
        "Advanced Recipe",
        "From Existing Recipe",
        "scalar",
        "provider",
        "semantic",
        "operation",
        "compiler",
        "decompiler",
        "Build model",
        "Preview model",
        "toolbar",
    ] {
        assert!(
            !joined_lower.contains(&forbidden.to_ascii_lowercase()),
            "default product strings unexpectedly contain {forbidden}: {joined}"
        );
    }
}

#[test]
fn semantic_clay_docs_keep_preview_display_separate_from_material_support() {
    let docs = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/SEMANTIC_CLAY_PREVIEW_MODE.md"
    ));
    let lower = docs.to_ascii_lowercase();

    assert!(docs.contains("untextured display shading only"));
    assert!(docs.contains("Pure Clay remains the strict mesh gate"));
    assert!(docs.contains("Quality reports must record Pure Clay pass/fail separately"));
    assert!(docs.contains("DiagnosticPartColor: developer/author diagnostic mode only"));
    assert!(lower.contains("does not imply uv/texturing support"));
    assert!(lower.contains("affect export payloads"));

    for forbidden in [
        "uv/texturing support is approved",
        "texture files are supported",
        "material editor is supported",
        "broad surface mode is approved",
    ] {
        assert!(
            !lower.contains(forbidden),
            "Semantic Clay docs must not overclaim material or texturing support: {forbidden}"
        );
    }

    assert_eq!(
        orchard_foundry::FoundryPreviewDisplayMode::novice_default(&[]),
        orchard_foundry::FoundryPreviewDisplayMode::PureClay
    );
    let assignments = vec![orchard_foundry::SemanticClayRoleAssignment::new(
        "body",
        "Primary Mass",
        0.72,
        10,
        true,
    )];
    assert_eq!(
        orchard_foundry::FoundryPreviewDisplayMode::novice_default(&assignments),
        orchard_foundry::FoundryPreviewDisplayMode::SemanticClay
    );
    assert!(!orchard_foundry::FoundryPreviewDisplayMode::DiagnosticPartColor.default_novice_safe());
    let strings = product_visible_strings_for_default_shell().join("\n");
    assert!(!strings.contains("DiagnosticPartColor"));
    assert!(!strings.contains("Surface mode"));
    assert!(!strings.contains("Texturing"));
}

#[test]
fn source_and_markdown_hygiene_targets_are_audit_friendly() {
    let targets = [
        (
            "README.md",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")),
        ),
        (
            "docs/CURRENT_PRODUCT_STATUS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CURRENT_PRODUCT_STATUS.md"
            )),
        ),
        (
            "docs/README.md",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/README.md")),
        ),
        (
            "docs/KNOWN_LIMITATIONS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/KNOWN_LIMITATIONS.md"
            )),
        ),
        (
            "docs/ARCHITECTURE_STATUS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/ARCHITECTURE_STATUS.md"
            )),
        ),
        (
            "docs/CONTRACT_BOUNDARIES.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CONTRACT_BOUNDARIES.md"
            )),
        ),
        (
            "docs/CLEANUP_PLAN.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CLEANUP_PLAN.md"
            )),
        ),
        (
            "docs/PRIMITIVE_DIRECT_MAKE_VISION.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/PRIMITIVE_DIRECT_MAKE_VISION.md"
            )),
        ),
        (
            "docs/OBJECT_PLAN_MATERIALIZATION_V1_INTEGRATION_REPORT.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/OBJECT_PLAN_MATERIALIZATION_V1_INTEGRATION_REPORT.md"
            )),
        ),
        (
            "docs/GEOMETRY_EXPORT_V0_INTEGRATION_REPORT.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/GEOMETRY_EXPORT_V0_INTEGRATION_REPORT.md"
            )),
        ),
    ];

    for (path, contents) in targets {
        let line_count = contents.lines().count();
        let max_line_len = contents.lines().map(str::len).max().unwrap_or_default();
        assert!(line_count >= 20, "{path} is too short to be audit-friendly");
        assert!(
            max_line_len <= 180,
            "{path} has a line longer than 180 characters: {max_line_len}"
        );
    }

    let app_source = include_str!("../../app.rs");
    let app_line_count = app_source.lines().count();
    let app_max_line_len = app_source.lines().map(str::len).max().unwrap_or_default();
    assert!(
        (100..=1_000).contains(&app_line_count),
        "foundry app.rs should stay split and audit-friendly: {app_line_count} lines"
    );
    assert!(
        app_max_line_len <= 180,
        "foundry app.rs has a line longer than 180 characters: {app_max_line_len}"
    );
}

#[test]
fn product_docs_keep_surface_rig_motion_and_game_ready_claims_caveated() {
    let docs = [
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/CURRENT_PRODUCT_STATUS.md"
        )),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/README.md")),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/KNOWN_LIMITATIONS.md"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/PRODUCT_CLAIM_GATE.md"
        )),
    ];
    let joined = docs.join("\n").to_ascii_lowercase();

    for forbidden in [
        "broad uv/texturing support is ready",
        "broad texturing support is ready",
        "rigging integration is ready",
        "animation integration is ready",
        "full game-ready support",
        "showcase claim approved",
    ] {
        assert!(
            !joined.contains(forbidden),
            "product docs must not overclaim unsupported work: {forbidden}"
        );
    }

    for claim in ["game-ready", "rigging", "animation", "texturing"] {
        if joined.contains(claim) {
            assert!(
                joined.contains("do not claim")
                    || joined.contains("outside")
                    || joined.contains("not product-supported")
                    || joined.contains("blocked")
                    || joined.contains("manual"),
                "mentions of {claim} must remain caveated"
            );
        }
    }
}

#[test]
fn product_docs_use_object_orchard_brand_with_one_migration_note() {
    let product_docs = [
        (
            "README.md",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")),
        ),
        (
            "docs/CURRENT_PRODUCT_STATUS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CURRENT_PRODUCT_STATUS.md"
            )),
        ),
        (
            "docs/KNOWN_LIMITATIONS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/KNOWN_LIMITATIONS.md"
            )),
        ),
        (
            "docs/ARCHITECTURE_STATUS.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/ARCHITECTURE_STATUS.md"
            )),
        ),
        (
            "docs/CONTRACT_BOUNDARIES.md",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CONTRACT_BOUNDARIES.md"
            )),
        ),
    ];

    for (path, contents) in product_docs {
        assert!(
            contents.contains("Object Orchard"),
            "{path} should use the Object Orchard product name"
        );
        for old_name in ["Shape Lab", "ShapeLab"] {
            assert!(
                !contents.contains(old_name),
                "{path} still contains old product name {old_name}"
            );
        }
    }

    let migration_note = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/OBJECT_ORCHARD_NAMING_TRANSITION.md"
    ));
    assert!(migration_note.contains("Object Orchard"));
    assert_eq!(
        migration_note.matches("ShapeLab").count(),
        1,
        "only the migration note should mention the old repository name"
    );
    assert!(!migration_note.contains("Shape Lab"));
}

#[test]
fn direct_make_docs_cover_current_recovery_contract() {
    let docs = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/PRIMITIVE_DIRECT_MAKE_VISION.md"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/ACTIVE_VARIATION_UI_RETIREMENT.md"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/CURRENT_PRODUCT_STATUS.md"
        )),
    ];
    let joined = docs.join("\n");

    for required in [
        "Choose Primitive",
        "edit bounded primitive properties",
        "Add to Pack",
        "Export",
        "Candidate generation is inactive in the current primitive product flow",
        "previous valid preview remains visible",
        "Invalid values cannot become current primitive state",
        "Future suggestions may return only as deterministic property presets",
        "generated candidate trays",
    ] {
        assert!(
            joined.contains(required),
            "Direct Make docs missing {required}"
        );
    }
}

#[test]
fn box_primitive_default_shell_hides_surface_export_copy() {
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n").to_ascii_lowercase();
    for hidden in [
        orchard_foundry::STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL,
        orchard_foundry::STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION,
        orchard_foundry::STATIC_PROP_FULL_READY_BLOCKED_NOTE,
        ACTION_TRY_MATERIAL_LOOKS,
        MATERIAL_LOOK_SECTION_TITLE,
        MATERIAL_LOOK_SURFACE_ONLY_COPY,
        MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        MATERIAL_LOOK_PREVIEW_ONLY_COPY,
        MATERIAL_LOOK_EXPORT_INCLUDED_COPY,
        MATERIAL_LOOK_FULL_READY_BLOCKED_COPY,
        SURFACE_PACKAGE_COMMAND_COPY,
        SURFACE_PACKAGE_COMMAND,
        "static surface package",
    ] {
        assert!(
            !strings.contains(&hidden),
            "Box Primitive default shell should not expose {hidden}"
        );
    }
    for overclaim in [
        "game-ready textured asset",
        "visual foundry previews are textured",
        "unity package",
        "unreal package",
        "godot package",
        "surface mode ready",
        "material editor",
        "rigartifact",
        "motionartifact",
        "skeleton template",
        "joint id",
        "skinning",
        "retarget",
    ] {
        assert!(
            !joined.contains(overclaim),
            "export copy should not overclaim {overclaim}: {joined}"
        );
    }
}

#[test]
fn material_look_evidence_loader_requires_valid_box_package() {
    let root = temp_material_look_package_root("valid-material-looks");
    let report = write_test_material_look_evidence(
        &root,
        TestMaterialLookEvidenceOptions {
            frozen_mesh_fingerprint: "box-fingerprint",
            ..TestMaterialLookEvidenceOptions::default()
        },
    );

    let evidence = load_material_look_evidence(&report, Some("box-fingerprint"))
        .expect("valid evidence loads");

    assert_eq!(evidence.candidates.len(), MATERIAL_LOOK_TITLES.len());
    assert!(full_ready_blockers_are_honest(
        &evidence.full_ready_blocker_codes
    ));
    for (candidate, title) in evidence.candidates.iter().zip(MATERIAL_LOOK_TITLES) {
        assert_eq!(candidate.display_name, title);
        assert!(
            candidate
                .textured_preview_ref
                .ends_with("textured-preview.png")
        );
        assert!(
            candidate
                .material_override_ref
                .ends_with("material-override.json")
        );
        assert!(candidate.surface_delta_ref.ends_with("surface-delta.json"));
        assert!(candidate.validation_ref.ends_with("validation.json"));
        assert_eq!(candidate.width, 2);
        assert_eq!(candidate.height, 2);
        assert_eq!(candidate.rgba8.len(), 16);
    }

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn material_look_evidence_loader_rejects_missing_mismatch_and_shape_delta() {
    let missing_root = temp_material_look_package_root("missing-material-looks");
    let missing_report = missing_root.join("surface/variants/surface-candidate-report.json");
    assert_eq!(
        load_material_look_evidence(&missing_report, None).expect_err("missing rejects"),
        MATERIAL_LOOK_MISSING_MESSAGE
    );

    let mismatch_root = temp_material_look_package_root("mismatch-material-looks");
    let mismatch_report = write_test_material_look_evidence(
        &mismatch_root,
        TestMaterialLookEvidenceOptions {
            frozen_mesh_fingerprint: "expected-fingerprint",
            ..TestMaterialLookEvidenceOptions::default()
        },
    );
    assert!(
        load_material_look_evidence(&mismatch_report, Some("different-fingerprint"))
            .expect_err("mismatch rejects")
            .contains("do not match")
    );

    let leak_root = temp_material_look_package_root("shape-delta-material-looks");
    let leak_report = write_test_material_look_evidence(
        &leak_root,
        TestMaterialLookEvidenceOptions {
            frozen_mesh_fingerprint: "box-fingerprint",
            shape_delta_leak: true,
            ..TestMaterialLookEvidenceOptions::default()
        },
    );
    assert!(
        load_material_look_evidence(&leak_report, Some("box-fingerprint"))
            .expect_err("shape delta rejects")
            .contains("shape change")
    );

    let _ = std::fs::remove_dir_all(missing_root);
    let _ = std::fs::remove_dir_all(mismatch_root);
    let _ = std::fs::remove_dir_all(leak_root);
}

#[test]
fn material_look_action_disabled_for_box_and_open_uses_recovery_copy() {
    let mut box_app = ready_visible_state_test_app();
    let visible = box_app.make_canvas_view_state();
    assert!(!box_app.material_look_action_visible(&visible));

    let missing_root = temp_material_look_package_root("missing-action-material-looks");
    box_app.material_looks.evidence_report_path =
        Some(missing_root.join("surface/variants/surface-candidate-report.json"));
    box_app.open_material_looks_panel();
    assert!(box_app.material_looks.tray_open);
    assert!(!box_app.make_canvas_view_state().material_look_tray_visible);
    assert_eq!(
        box_app.material_looks.load_error.as_deref(),
        Some(MATERIAL_LOOK_MISSING_MESSAGE)
    );

    let _ = std::fs::remove_dir_all(missing_root);
}

#[test]
fn selecting_material_look_is_preview_only_and_preserves_geometry_state() {
    let mut app = ready_visible_state_test_app();
    let fingerprint = app
        .current_artifact_fingerprint_hex()
        .expect("build fingerprint");
    let root = temp_material_look_package_root("select-material-looks");
    let report = write_test_material_look_evidence(
        &root,
        TestMaterialLookEvidenceOptions {
            frozen_mesh_fingerprint: fingerprint.as_str(),
            ..TestMaterialLookEvidenceOptions::default()
        },
    );
    let before_build = app.state.current_build.clone();
    let before_controls = app.state.controls.clone();

    app.material_looks.evidence_report_path = Some(report);
    app.open_material_looks_panel();
    let second_id = app
        .material_looks
        .evidence
        .as_ref()
        .expect("evidence")
        .candidates[1]
        .candidate_id
        .clone();
    app.material_looks.selected_candidate_id = Some(second_id);

    assert_eq!(app.state.current_build, before_build);
    assert_eq!(app.state.controls, before_controls);
    assert_eq!(app.material_look_export_copy(), None);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn box_primitive_default_copy_hides_material_look_terms() {
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n").to_ascii_lowercase();

    for hidden in [
        ACTION_TRY_MATERIAL_LOOKS,
        MATERIAL_LOOK_SECTION_TITLE,
        MATERIAL_LOOK_SURFACE_ONLY_COPY,
        MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        "Current Material",
        "Candidate Material",
        MATERIAL_LOOK_MISSING_MESSAGE,
    ] {
        assert!(
            !strings.contains(&hidden),
            "Box Primitive default copy should not expose {hidden}"
        );
    }

    for forbidden in [
        "surface",
        "surfaceartifact",
        "material looks",
        "uv set",
        "material slot id",
        "texture file path",
        "gltf primitive",
        "rigging",
        "game-ready surface",
    ] {
        assert!(
            !joined.contains(forbidden),
            "material look copy leaked {forbidden}: {joined}"
        );
    }
    assert!(strings.contains(&BOX_PRIMITIVE_EXPORT_LIMITATION));
}

#[test]
fn box_primitive_default_copy_has_no_crate_case_or_part_focus_language() {
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n").to_ascii_lowercase();

    for forbidden in [
        "crate",
        "case",
        "cargo",
        "sci-fi",
        "body",
        "body chip",
        "parts",
        "focus",
        "focused part",
        "focus part",
        "part chip",
        "family studio",
    ] {
        assert!(
            !joined.contains(forbidden),
            "Box Primitive default UI copy should not expose {forbidden}: {joined}"
        );
    }
}

#[test]
fn box_primitive_default_copy_has_no_unsupported_pipeline_overclaim() {
    let strings = product_visible_strings_for_default_shell();
    let joined = strings.join("\n").to_ascii_lowercase();

    for forbidden in ["uv", "texturing", "rigging", "animation"] {
        assert!(
            !joined.contains(forbidden),
            "Box Primitive UI copy should not expose unsupported pipeline term {forbidden}: {joined}"
        );
    }
    assert!(strings.contains(&BOX_PRIMITIVE_EXPORT_LIMITATION));
    assert!(
        joined.contains("not a textured, rigged, animated, or game-ready package"),
        "export copy must explicitly block game-ready overclaim: {joined}"
    );
}

#[test]
fn product_shell_steps_are_novice_facing() {
    let strings = product_visible_strings_for_default_shell();

    for required in [
        "Start with Box Primitive",
        "Choose",
        "Make",
        "Export",
        "Visual Foundry",
        "Project",
        "Open Project",
        "Save Project",
        "Save Project As",
        "Start Another Asset",
        "History",
        ACTION_CHOOSE_TEMPLATE,
        "Start",
    ] {
        assert!(
            strings.contains(&required),
            "missing product string {required}"
        );
    }
}

#[test]
fn make_canvas_product_copy_replaces_old_primary_modes() {
    let strings = product_visible_strings_for_default_shell();
    let steps = WORKFLOW_STEPS
        .iter()
        .map(|step| step.label)
        .collect::<Vec<_>>();
    assert_eq!(steps, vec!["Choose", "Make"]);
    assert!(!steps.contains(&"Directions"));
    assert!(!steps.contains(&"Customize"));
    assert!(!steps.contains(&"Pack"));
    assert!(!steps.contains(&"Export"));

    for forbidden in [
        "Variation Mode",
        "Variation mode",
        "Complete Looks",
        "Candidate tray",
        "candidate",
        "generated ideas",
        "survivor",
        "Model workspace",
        "Focus Part",
        ACTION_FOCUS,
        "Generate 6 Directions",
        "Use this idea",
        "Try body ideas",
        ACTION_TRY_BOX_IDEAS,
        ACTION_CHOOSE_DIRECTION,
        "Material looks are not previewable yet.",
    ] {
        assert!(
            !strings.iter().any(|string| string
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase())),
            "Make canvas product copy should not expose {forbidden}"
        );
    }

    for required in [
        ACTION_ADJUST_DIMENSIONS,
        ACTION_EDIT_BOX_PRIMITIVE,
        ACTION_EDIT_FLAT_PANEL,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
        "Width",
        "Depth",
        "Height",
        "Thickness",
        "Edge Softness",
    ] {
        assert!(
            strings.contains(&required),
            "missing Make canvas product copy {required}"
        );
    }
}

#[test]
fn direct_make_ignores_generated_candidates_and_comparison() {
    let mut app = visible_state_test_app();
    app.state.current_preview = Some(test_preview_image("current"));
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    let selected = orchard_foundry::FoundryCandidateId("candidate-a".to_owned());
    app.state.selected_candidate = Some(selected.clone());
    app.state.candidates = vec![test_candidate_card(&selected.0, true, None)];

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.candidate_count, 0);
    assert!(!visible.candidate_tray_visible);
    assert!(!visible.selected_comparison_visible);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(
        visible.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
}

#[test]
fn direct_make_has_no_selected_candidate_comparison() {
    let mut app = visible_state_test_app();
    app.state.current_preview = Some(test_preview_image("current"));
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    app.state.candidates = vec![test_candidate_card("candidate-a", false, None)];

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.candidate_count, 0);
    assert!(!visible.candidate_tray_visible);
    assert!(!visible.selected_comparison_visible);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(
        visible.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
}

#[test]
fn box_primitive_ignores_stale_focus_part_scope() {
    let mut app = visible_state_test_app();
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    app.state.current_preview = Some(test_preview_image("current"));
    set_test_focus_scope(&mut app, "body", "Body");

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.primary_title, "Box Primitive");
    assert!(visible.focused_part_label.is_none());
    assert!(!visible.focused_part_visible);
    assert!(!visible.focused_part_actions_visible);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(visible.property_panel_title, ACTION_EDIT_BOX_PRIMITIVE);
    assert_eq!(
        visible.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
}

#[test]
fn box_primitive_focus_body_does_not_change_visible_scope_or_action() {
    let mut app = visible_state_test_app();
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    app.state.current_preview = Some(test_preview_image("current"));
    set_test_focus_scope(&mut app, "body", "Body");

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.primary_title, "Box Primitive");
    assert!(visible.focused_part_label.is_none());
    assert!(!visible.focused_part_visible);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(visible.property_panel_title, ACTION_EDIT_BOX_PRIMITIVE);
}

#[test]
fn make_canvas_primary_action_changes_by_state() {
    let ready = ready_visible_state_test_app().make_canvas_view_state();
    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert!(ready.primary_action_enabled);

    let mut focused = ready_visible_state_test_app();
    set_test_focus_scope(&mut focused, "body", "Body");
    let focused = focused.make_canvas_view_state();
    assert_eq!(focused.mode, MakeCanvasMode::Ready);
    assert_eq!(focused.primary_action_label, ACTION_ADJUST_DIMENSIONS);

    let mut generating = ready_visible_state_test_app();
    generating
        .state
        .request_candidates(FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        })
        .expect("candidate job schedules");
    let generating = generating.make_canvas_view_state();
    assert_eq!(generating.mode, MakeCanvasMode::Ready);
    assert_eq!(generating.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert!(generating.primary_action_enabled);
    assert!(!generating.candidate_tray_visible);

    let mut reviewing = ready_visible_state_test_app();
    reviewing.state.candidates = vec![test_candidate_card("candidate-a", false, None)];
    let reviewing = reviewing.make_canvas_view_state();
    assert_eq!(reviewing.mode, MakeCanvasMode::Ready);
    assert_eq!(reviewing.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert!(reviewing.primary_action_enabled);
    assert!(!reviewing.selected_comparison_visible);
}

#[test]
fn focused_inspector_filters_controls_and_keeps_show_all_available() {
    let controls = vec![
        test_control_view("proportions", "Proportions"),
        test_control_view("edge_softness", "Edge Softness"),
        test_control_view("box_profile", "Box Profile"),
        test_control_view("draft_note", "Draft Note"),
    ];
    let body = directions::DirectionPartGroup {
        group_id: "body".to_owned(),
        label: "Body".to_owned(),
        focusable: true,
        unavailable_reason: None,
    };

    let sections = make_context_inspector_controls(&controls, Some(&body));
    let visible_ids = sections
        .visible
        .iter()
        .map(|control| control.id.as_str())
        .collect::<Vec<_>>();
    let overflow_ids = sections
        .overflow
        .iter()
        .map(|control| control.id.as_str())
        .collect::<Vec<_>>();

    assert!(visible_ids.contains(&"proportions"));
    assert!(visible_ids.contains(&"edge_softness"));
    assert!(visible_ids.contains(&"box_profile"));
    assert!(!visible_ids.contains(&"draft_note"));
    assert!(overflow_ids.contains(&"draft_note"));
    assert_eq!(sections.disclosure_label, "Show all controls");
}

#[test]
fn whole_asset_inspector_starts_with_short_control_list() {
    let controls = vec![
        test_control_view("proportions", "Proportions"),
        test_control_view("edge_softness", "Edge Softness"),
        test_control_view("box_profile", "Box Profile"),
        test_control_view("draft_note", "Draft Note"),
    ];

    let sections = make_context_inspector_controls(&controls, None);

    assert_eq!(sections.visible.len(), MAKE_CONTEXT_INITIAL_CONTROL_LIMIT);
    assert_eq!(
        sections.overflow.len(),
        controls.len() - MAKE_CONTEXT_INITIAL_CONTROL_LIMIT
    );
    assert_eq!(sections.disclosure_label, "More controls");
}

#[test]
fn focused_generation_state_does_not_render_whole_asset_heading() {
    let mut app = visible_state_test_app();
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    set_test_focus_scope(&mut app, "body", "Body");
    let selected = orchard_foundry::FoundryCandidateId("body-candidate".to_owned());
    app.state.current_preview = Some(test_preview_image("current"));
    app.state.selected_candidate = Some(selected.clone());
    app.state.candidates = vec![test_candidate_card(
        &selected.0,
        true,
        Some("Body".to_owned()),
    )];

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.primary_title, "Box Primitive");
    assert_eq!(visible.candidate_count, 0);
    assert!(!visible.candidate_tray_visible);
    assert!(!visible.selected_comparison_visible);
    assert_ne!(visible.primary_title, "Whole asset");
}

#[test]
fn pack_and_export_actions_open_visible_drawer_state() {
    let mut app = visible_state_test_app();
    app.drawer = Some(FoundryDrawer::Pack);
    let pack_visible = app.make_canvas_view_state();
    assert_eq!(pack_visible.mode, MakeCanvasMode::PackDrawerOpen);
    assert!(pack_visible.pack_drawer_visible);
    assert!(!pack_visible.export_drawer_visible);

    app.drawer = Some(FoundryDrawer::Export);
    let export_visible = app.make_canvas_view_state();
    assert_eq!(export_visible.mode, MakeCanvasMode::ExportDrawerOpen);
    assert!(export_visible.export_drawer_visible);
    assert!(!export_visible.pack_drawer_visible);
}

#[test]
fn object_plan_review_ui_hidden_by_default() {
    let strings = product_visible_strings_for_default_shell();
    assert!(!strings.contains(&ACTION_REVIEW_OBJECT_PLANS));
    assert!(!strings.contains(&"ObjectPlan Review"));
    assert!(!strings.contains(&"Draft only"));
    assert!(!strings.contains(&"Human review required"));

    let mut app = FoundryDesktopApp::default();
    let hidden = app.object_plan_review_ui_state();
    assert!(!hidden.entry_visible);
    assert!(!hidden.drawer_visible);

    app.drawer = Some(FoundryDrawer::ObjectPlanReview);
    let forced = app.object_plan_review_ui_state();
    assert!(!forced.drawer_visible);
}

#[test]
fn object_plan_review_ui_visible_only_under_dev_flag() {
    let mut app = FoundryDesktopApp {
        object_plan_review_enabled: true,
        ..FoundryDesktopApp::default()
    };

    let entry = app.object_plan_review_ui_state();
    assert!(entry.entry_visible);
    assert!(!entry.drawer_visible);

    app.drawer = Some(FoundryDrawer::ObjectPlanReview);
    let drawer = app.object_plan_review_ui_state();
    assert!(drawer.drawer_visible);
    assert!(drawer.batch_report_visible);
    assert!(drawer.contact_sheet_visible);
    assert_eq!(
        drawer.review_labels,
        vec!["Keep", "Regenerate", "Simplify", "Blocked"]
    );
    assert_eq!(
        drawer.safety_labels,
        vec![
            "Draft only",
            "Not catalog published",
            "Human review required"
        ]
    );
    assert!(!drawer.publish_action_visible);
    assert!(!drawer.catalog_mutation_allowed);
    assert!(!drawer.runtime_llm_action_visible);
}

#[test]
fn object_plan_review_ui_has_no_noob_facing_runtime_or_publish_copy() {
    let strings = product_visible_strings_for_default_shell()
        .join("\n")
        .to_ascii_lowercase();

    for hidden in [
        "review objectplans",
        "objectplan review",
        "runtime llm",
        "publish plan",
        "publish kit",
    ] {
        assert!(
            !strings.contains(hidden),
            "default UI must not expose internal ObjectPlan review copy: {hidden}"
        );
    }
}

#[test]
fn family_studio_lite_entry_hidden_by_default() {
    let strings = product_visible_strings_for_default_shell();
    assert!(!strings.contains(&"Reusable Kit"));

    let mut app = ready_visible_state_test_app();
    let hidden = app.family_studio_lite_ui_state();
    assert!(!hidden.entry_visible);
    assert!(!hidden.drawer_visible);

    app.drawer = Some(FoundryDrawer::FamilyStudioLite);
    let forced = app.family_studio_lite_ui_state();
    assert!(!forced.drawer_visible);
}

#[test]
fn family_studio_lite_visible_under_preview_flag() {
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;

    let entry = app.family_studio_lite_ui_state();
    assert!(entry.entry_visible);
    assert!(!entry.drawer_visible);

    app.drawer = Some(FoundryDrawer::FamilyStudioLite);
    let drawer = app.family_studio_lite_ui_state();
    assert!(drawer.drawer_visible);
    assert_eq!(drawer.starting_point_title, "Box Primitive");
    assert_eq!(drawer.source_label, "Current shape");
    assert!(drawer.supported);
    assert!(!drawer.approved);
    assert!(!drawer.publish_allowed);
    assert!(!drawer.runtime_llm_action_visible);
    assert!(!drawer.generated_variation_copy_visible);
}

#[test]
fn family_studio_lite_capability_cards_come_from_adapter() {
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;
    app.drawer = Some(FoundryDrawer::FamilyStudioLite);

    let drawer = app.family_studio_lite_ui_state();
    let expected = kit_capability_cards_for_primitive(PrimitiveKind::BoxPrimitive, false)
        .into_iter()
        .filter(|card| card.source_kind != KitCapabilitySourceKind::SurfaceLook)
        .map(|card| card.capability_id)
        .collect::<Vec<_>>();
    let actual = drawer
        .capability_cards
        .iter()
        .map(|card| card.capability_id.clone())
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
    assert!(
        drawer
            .capability_cards
            .iter()
            .any(|card| card.display_name == "Width")
    );
    assert!(
        drawer
            .capability_cards
            .iter()
            .any(|card| card.display_name == "Saved Shapes")
    );
    assert!(
        orchard_foundry::validate_kit_capability_cards(&kit_capability_cards_for_primitive(
            PrimitiveKind::BoxPrimitive,
            false
        ))
        .is_valid()
    );
}

#[test]
fn family_studio_lite_user_copy_hides_technical_terms() {
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;
    app.drawer = Some(FoundryDrawer::FamilyStudioLite);
    app.run_family_studio_lite_test();
    app.family_studio_lite.saved_visibility = Some(DirectKitVisibility::PersonalOnly);

    let strings = family_studio_lite_strings(&app.family_studio_lite_ui_state())
        .join("\n")
        .to_ascii_lowercase();

    for forbidden in [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "conformance",
        "artifact",
        "raw transform",
        "mesh payload",
        "generated variation",
        "candidate",
        "runtime llm",
        "public catalog",
        "publish",
        "uv",
        "texturing",
        "rigging",
        "animation",
        "game-ready",
    ] {
        assert!(
            !strings.contains(forbidden),
            "Family Studio Lite copy leaked {forbidden}: {strings}"
        );
    }
}

#[test]
fn family_studio_lite_test_result_appears_without_approval() {
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;
    app.drawer = Some(FoundryDrawer::FamilyStudioLite);

    app.run_family_studio_lite_test();
    let drawer = app.family_studio_lite_ui_state();
    let result = drawer.test_result.expect("test result");
    assert_eq!(result.status, FamilyStudioLiteTestStatus::Warnings);
    assert_eq!(result.tested_capabilities, 1);
    assert!(result.human_review_required);
    assert!(!result.approved);
    assert!(!result.publish_allowed);
}

#[test]
fn family_studio_lite_hides_irrelevant_stale_background_warning() {
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;
    app.drawer = Some(FoundryDrawer::FamilyStudioLite);
    app.state.status = Some("Ignored a background result because newer work is active.".to_owned());

    let drawer = app.family_studio_lite_ui_state();
    let visible = app.make_canvas_view_state();

    assert!(drawer.drawer_visible);
    assert!(app.suppresses_background_result_status(app.state.status.as_deref().expect("status")));
    assert!(visible.local_warning_message.is_none());
    assert_eq!(visible.local_banner_title, "Ready");
    assert_eq!(app.status_summary(), "Ready");
    assert!(
        !visible
            .local_banner_message
            .contains("An older result was ignored")
    );
}

#[test]
fn family_studio_lite_save_draft_creates_draft_kit() {
    let store = isolated_family_studio_lite_store("draft");
    let _ = std::fs::remove_dir_all(&store);
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;

    app.save_family_studio_lite_kit_to(DirectKitVisibility::Draft, &store);
    let drawer = app.family_studio_lite_ui_state();
    assert_eq!(drawer.saved_visibility, Some(DirectKitVisibility::Draft));

    let stored = orchard_foundry::load_direct_kit(&store, "box_primitive_reusable_kit")
        .expect("load saved draft");
    assert_eq!(stored.visibility, DirectKitVisibility::Draft);
    assert!(!stored.public_catalog_visible);
    assert!(!stored.novice_visible);
    let _ = std::fs::remove_dir_all(&store);
}

#[test]
fn family_studio_lite_use_personally_creates_personal_only_kit() {
    let store = isolated_family_studio_lite_store("personal");
    let _ = std::fs::remove_dir_all(&store);
    let mut app = ready_visible_state_test_app();
    app.family_studio_lite_enabled = true;

    app.save_family_studio_lite_kit_to(DirectKitVisibility::PersonalOnly, &store);
    let drawer = app.family_studio_lite_ui_state();
    assert_eq!(
        drawer.saved_visibility,
        Some(DirectKitVisibility::PersonalOnly)
    );

    let stored = orchard_foundry::load_direct_kit(&store, "box_primitive_reusable_kit")
        .expect("load saved personal kit");
    assert_eq!(stored.visibility, DirectKitVisibility::PersonalOnly);
    assert_eq!(
        stored.direct_kit.review_tier,
        ObjectPlanReviewTier::Personal
    );
    assert!(!stored.public_catalog_visible);
    assert!(!stored.novice_visible);
    let _ = std::fs::remove_dir_all(&store);
}

#[test]
fn family_studio_lite_screenshot_state_assertions_cover_required_states() {
    let mut hidden = ready_visible_state_test_app();
    hidden.family_studio_lite_enabled = false;
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::FamilyStudioLiteHiddenDefault,
            &hidden.make_canvas_view_state(),
        )
        .is_ok()
    );

    let mut drawer = ready_visible_state_test_app();
    drawer.family_studio_lite_enabled = true;
    drawer.drawer = Some(FoundryDrawer::FamilyStudioLite);
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::FamilyStudioLiteDrawer,
            &drawer.make_canvas_view_state(),
        )
        .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::FamilyStudioLiteTestResult,
            &drawer.make_canvas_view_state(),
        )
        .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::FamilyStudioLiteSaveDraft,
            &drawer.make_canvas_view_state(),
        )
        .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::FamilyStudioLitePersonalSaved,
            &drawer.make_canvas_view_state(),
        )
        .is_ok()
    );
}

#[test]
fn screenshot_focus_scenario_helper_does_not_focus_box_body() {
    let mut app = visible_state_test_app();

    let commands = app.ensure_screenshot_focus("body");
    assert_eq!(app.screenshot_scenario_step, 0);
    assert!(commands.is_empty());
    assert_eq!(
        app.make_canvas_view_state().focused_part_label.as_deref(),
        None
    );

    set_test_focus_scope(&mut app, "body", "Body");
    let commands = app.ensure_screenshot_focus("body");
    assert!(commands.is_empty());
    assert_eq!(app.screenshot_scenario_step, 0);
    assert_eq!(
        app.make_canvas_view_state().focused_part_label.as_deref(),
        None
    );
}

#[test]
fn screenshot_state_assertions_cover_required_make_scenarios() {
    let mut app = ready_visible_state_test_app();
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::BoxDirectMakeReady,
            &app.make_canvas_view_state(),
        )
        .is_ok()
    );

    let box_edited = ready_visible_state_test_app().make_canvas_view_state();
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::BoxPropertyEdit, &box_edited,).is_ok()
    );

    let flat_panel =
        ready_fixture_state_test_app(orchard_foundry_catalog::flat_panel::fixture_catalog())
            .make_canvas_view_state();
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::FlatPanelDirectMakeReady, &flat_panel,)
            .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::FlatPanelPropertyEdit, &flat_panel)
            .is_ok()
    );

    let sphere =
        ready_fixture_state_test_app(orchard_foundry_catalog::sphere_primitive::fixture_catalog())
            .make_canvas_view_state();
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::SphereDirectMakeReady, &sphere).is_ok()
    );
    assert!(screenshot_scenario_assertion(ScreenshotScenario::SpherePropertyEdit, &sphere).is_ok());
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::SphereKnobLikePreset, &sphere).is_ok()
    );

    let panel_knob =
        ready_fixture_state_test_app(orchard_foundry_catalog::panel_knob::fixture_catalog())
            .make_canvas_view_state();
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::PanelKnobDirectMakeReady, &panel_knob,)
            .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::OrbitAfterDragOrTool,
            &app.make_canvas_view_state(),
        )
        .is_ok()
    );
    assert!(
        screenshot_scenario_assertion(ScreenshotScenario::ResetView, &app.make_canvas_view_state())
            .is_ok()
    );

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::PackDrawer,
            &app.make_canvas_view_state()
        )
        .is_ok()
    );
    app.drawer = Some(FoundryDrawer::Export);
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::ExportDrawer,
            &app.make_canvas_view_state(),
        )
        .is_ok()
    );
    let mut sphere_export_app =
        ready_fixture_state_test_app(orchard_foundry_catalog::sphere_primitive::fixture_catalog());
    sphere_export_app.drawer = Some(FoundryDrawer::Export);
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::SphereExportDrawer,
            &sphere_export_app.make_canvas_view_state(),
        )
        .is_ok()
    );
    let mut object_plan_app = ready_visible_state_test_app();
    object_plan_app.object_plan_review_enabled = true;
    object_plan_app.drawer = Some(FoundryDrawer::ObjectPlanReview);
    assert!(
        screenshot_scenario_assertion(
            ScreenshotScenario::ObjectPlanReviewDrawer,
            &object_plan_app.make_canvas_view_state(),
        )
        .is_ok()
    );
}

#[test]
fn preparing_asset_disables_idea_generation_with_local_reason() {
    let app = visible_state_test_app();

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
    assert!(!visible.primary_action_enabled);
    assert!(visible.quick_template_preview_visible);
    assert_eq!(
        visible.primary_action_disabled_reason.as_deref(),
        Some(ASSET_PREPARING_REASON)
    );
    assert_eq!(
        visible.local_busy_label.as_deref(),
        Some("Preparing Box Primitive...")
    );
    assert!(visible.local_busy_visible);
}

#[test]
fn preparation_phases_timeout_and_recovery_actions_are_visible() {
    let mut app = visible_state_test_app();

    let visible = app.make_canvas_view_state();
    assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
    assert_eq!(
        visible.preparation_phase,
        MakePreparationPhase::PreparingModel
    );
    assert_eq!(visible.local_banner_title, "Preparing Box Primitive");
    assert_eq!(visible.local_banner_message, "Preparing model");
    assert!(!visible.preparation_fallback_visible);

    let output = compile_foundry_document(
        app.state.document.as_ref().expect("document"),
        &orchard_foundry_catalog::box_primitive::fixture_catalog(),
    )
    .expect("fixture compiles");
    app.state.current_build = Some(output.build_stamp.clone());
    app.state.current_output = Some(Box::new(output));
    let visible = app.make_canvas_view_state();
    assert_eq!(
        visible.preparation_phase,
        MakePreparationPhase::RenderingPreview
    );
    assert!(!visible.quick_template_preview_visible);
    assert!(visible.preview_update_required);
    assert_eq!(visible.local_banner_message, "Rendering preview");

    app.state.current_preview = Some(test_preview_image_for_build(
        "current",
        app.state.current_build.clone(),
    ));
    let visible = app.make_canvas_view_state();
    assert_eq!(visible.preparation_phase, MakePreparationPhase::Ready);
    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.local_banner_title, "Ready");

    let mut timed_out = visible_state_test_app();
    timed_out.make_preparation_started_at =
        Some(Instant::now() - PREPARATION_TIMEOUT - Duration::from_secs(1));
    let visible = timed_out.make_canvas_view_state();
    assert!(visible.preparation_timed_out);
    assert!(visible.preparation_fallback_visible);
    assert_eq!(visible.local_banner_message, PREPARATION_TIMEOUT_MESSAGE);

    for label in [
        ACTION_RETRY_PREPARATION,
        ACTION_CHOOSE_ANOTHER_TEMPLATE,
        ACTION_OPEN_PROJECT,
    ] {
        assert!(rendered_action_labels_for_default_shell().contains(&label));
    }
}

#[test]
fn stale_preview_uses_update_preview_copy_and_no_legacy_make_actions() {
    let mut app = visible_state_test_app();
    let output = compile_foundry_document(
        app.state.document.as_ref().expect("document"),
        &orchard_foundry_catalog::box_primitive::fixture_catalog(),
    )
    .expect("fixture compiles");
    app.state.current_build = Some(output.build_stamp.clone());
    app.state.current_output = Some(Box::new(output));
    app.state.current_preview = Some(test_preview_image("old-current"));

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert!(visible.preview_updating);
    assert!(visible.preview_update_required);
    assert!(visible.primary_action_enabled);
    assert_eq!(visible.local_banner_title, "Ready");
    assert!(visible.local_busy_label.is_none());
    assert!(!visible.local_busy_visible);
    assert_eq!(
        visible.next_action_hint,
        "Update preview to keep making changes."
    );

    let strings = product_visible_strings_for_default_shell();
    assert!(strings.contains(&ACTION_UPDATE_PREVIEW));
    assert!(!strings.contains(&"Build Asset"));
    assert!(!strings.contains(&"Refresh Preview"));
}

#[test]
fn camera_preview_refresh_stays_ready_without_busy_overlay() {
    let mut app = ready_visible_state_test_app();
    let preview = app
        .state
        .current_preview
        .as_ref()
        .expect("ready fixture has preview")
        .clone();
    let camera = current_preview_orbit_camera(&preview, egui::vec2(16.0, 8.0))
        .expect("drag should request a different camera");
    let effects = app
        .state
        .request_preview(preview.width, preview.height, Some(camera))
        .expect("preview request schedules");

    assert_eq!(effects.len(), 1);
    assert!(
        app.state
            .active_jobs
            .values()
            .any(|request| request.slot() == FoundryJobSlot::RenderPreview)
    );
    assert!(!app.make_is_preparing_now());
    assert_eq!(app.preview_status(), "Ready");

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.preparation_phase, MakePreparationPhase::Ready);
    assert!(visible.preview_ready);
    assert!(!visible.preview_updating);
    assert_eq!(visible.local_banner_title, "Ready");
    assert_eq!(visible.local_banner_tone, BannerTone::Success);
    assert!(visible.primary_action_enabled);
    assert!(visible.local_busy_label.is_none());
    assert!(!visible.local_busy_visible);
}

#[test]
fn make_canvas_responsive_layout_keeps_adjust_visible_on_wide_short_viewports() {
    let app = visible_state_test_app();
    let visible = app.make_canvas_view_state();

    let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

    assert!(layout.compact_ideas);
    assert!(!layout.stacked_columns);
    assert!(layout.inspector_width >= 500.0);
    assert!(layout.tray_height <= 128.0);
    assert!(layout.top_height >= 700.0);
    assert!(layout.stage_width > layout.inspector_width * 2.0);
    assert!(!make_canvas_inspector_build_actions_visible(&visible));
}

#[test]
fn make_canvas_responsive_layout_expands_ideas_when_candidates_exist() {
    let mut app = ready_visible_state_test_app();
    app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
    let visible = app.make_canvas_view_state();

    let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

    assert!(layout.compact_ideas);
    assert!(!layout.inline_ideas);
    assert_eq!(layout.tray_height, 0.0);
    assert_eq!(layout.top_height, 860.0);
    assert_eq!(layout.ideas_width, 0.0);
    assert!(make_canvas_inspector_build_actions_visible(&visible));
}

#[test]
fn make_canvas_responsive_layout_hides_material_looks_tray_for_box() {
    let mut app = ready_visible_state_test_app();
    app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
    app.material_looks.tray_open = true;
    let visible = app.make_canvas_view_state();

    let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

    assert!(!visible.material_look_tray_visible);
    assert!(!visible.candidate_tray_visible);
    assert!(!layout.inline_ideas);
    assert_eq!(layout.tray_height, 0.0);
    assert_eq!(layout.top_height, 860.0);
}

#[test]
fn make_canvas_responsive_layout_preview_edge_uses_available_viewport_without_overgrowing() {
    assert_eq!(make_stage_preview_edge(1400.0, 900.0), 520.0);
    assert_eq!(make_stage_preview_edge(260.0, 220.0), 180.0);
    assert!((make_canvas_stacked_stage_height(360.0) - 201.6).abs() < 0.01);
    assert_eq!(compact_direction_grid_columns(734.0), 3);
    assert_eq!(compact_direction_card_preview_edge(330.0), 64.0);
}

#[test]
fn candidate_tray_state_enum_covers_every_rendering_state() {
    let mut app = ready_visible_state_test_app();
    assert_eq!(
        app.make_canvas_view_state().candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );

    app.state
        .request_candidates(FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        })
        .expect("candidate job schedules");
    assert_eq!(
        app.make_canvas_view_state().candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );
    assert!(!app.make_canvas_view_state().candidate_tray_visible);

    app.state.active_jobs.clear();
    let mut pending_card = test_candidate_card("candidate-pending", false, None);
    pending_card.validation_label = "Preview pending".to_owned();
    pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
    pending_card.rgba8.clear();
    pending_card.width = 0;
    pending_card.height = 0;
    pending_card.camera = None;
    pending_card.selectable = false;
    app.state.candidates = vec![pending_card];
    let visible = app.make_canvas_view_state();
    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(
        visible.candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );
    assert!(!visible.candidate_tray_visible);

    app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
    assert_eq!(
        app.make_canvas_view_state().candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );

    app.state.candidates.clear();
    app.state.candidate_output = Some(Box::new(empty_test_candidate_output(3, 1, 0)));
    assert_eq!(
        app.make_canvas_view_state().candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );

    app.state.status = Some("Candidate search failed locally.".to_owned());
    assert_eq!(
        app.make_canvas_view_state().candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );
}

#[test]
fn box_primitive_zero_candidates_show_whole_box_recovery_copy() {
    let mut app = ready_visible_state_test_app();
    set_test_focus_scope(&mut app, "body", "Body");
    app.state.candidates.clear();
    app.state.candidate_output = Some(Box::new(empty_test_candidate_output(4, 0, 0)));

    let visible = app.make_canvas_view_state();

    assert_eq!(
        visible.candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );
    assert_eq!(visible.local_banner_title, "Ready");
    assert_eq!(
        visible.local_banner_message,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!visible.focused_no_candidates_recovery_visible);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
}

#[test]
fn compiled_candidate_shells_show_before_all_previews_render() {
    let mut app = ready_visible_state_test_app();
    let mut pending_card = test_candidate_card("candidate-pending", true, None);
    pending_card.validation_label = "Preview pending".to_owned();
    pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
    pending_card.rgba8.clear();
    pending_card.width = 0;
    pending_card.height = 0;
    pending_card.camera = None;
    pending_card.selectable = false;
    app.state.selected_candidate = Some(pending_card.id.clone());
    app.state.candidates = vec![pending_card];

    let pending_visible = app.make_canvas_view_state();

    assert_eq!(pending_visible.mode, MakeCanvasMode::Ready);
    assert_eq!(
        pending_visible.candidate_tray_state,
        MakeCandidateTrayState::EmptyReady
    );
    assert_eq!(pending_visible.local_banner_title, "Ready");
    assert!(!pending_visible.candidate_tray_visible);
    assert!(!pending_visible.selected_comparison_visible);
    assert!(pending_visible.primary_action_enabled);
    assert!(app.accept_visible_candidate_command().is_none());

    let mut pending_card = test_candidate_card("candidate-pending", false, None);
    pending_card.validation_label = "Preview pending".to_owned();
    pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
    pending_card.rgba8.clear();
    pending_card.width = 0;
    pending_card.height = 0;
    pending_card.camera = None;
    pending_card.selectable = false;
    let ready_card = test_candidate_card("candidate-ready", true, None);
    app.state.selected_candidate = Some(ready_card.id.clone());
    app.state.candidates = vec![pending_card, ready_card];

    let mixed_visible = app.make_canvas_view_state();

    assert_eq!(mixed_visible.mode, MakeCanvasMode::Ready);
    assert_eq!(mixed_visible.local_banner_title, "Ready");
    assert!(!mixed_visible.selected_comparison_visible);
    assert!(mixed_visible.primary_action_enabled);
    assert!(!make_canvas_candidate_actions_enabled(&mixed_visible));
    assert!(app.accept_visible_candidate_command().is_none());
}

#[test]
fn stale_result_warning_is_local_and_recoverable_with_try_again() {
    let mut app = ready_visible_state_test_app();
    app.legacy_candidate_ui_enabled = true;
    force_other_profile(&mut app);
    app.state.status = Some("Ignored a background result because newer work is active.".to_owned());

    let visible = app.make_canvas_view_state();

    assert_eq!(
        visible.local_warning_message.as_deref(),
        Some(STALE_RESULT_WARNING)
    );
    assert_eq!(visible.local_banner_title, "Older result ignored");
    assert!(visible.local_banner_message.contains(ACTION_TRY_AGAIN));
    assert!(rendered_action_labels_for_default_shell().contains(&ACTION_TRY_AGAIN));
}

#[test]
fn direct_primitive_make_suppresses_stale_background_warning() {
    let mut app = ready_visible_state_test_app();
    app.state.status = Some("Ignored a background result because newer work is active.".to_owned());

    let visible = app.make_canvas_view_state();

    assert!(app.active_make_profile_kind().direct_primitive_workflow());
    assert!(app.suppresses_background_result_status(app.state.status.as_deref().expect("status")));
    assert!(visible.local_warning_message.is_none());
    assert_eq!(visible.local_banner_title, "Ready");
    assert!(!app.status_summary().contains("Ignored a background result"));
    assert!(
        !visible
            .local_banner_message
            .contains("An older result was ignored")
    );
}

#[test]
fn busy_candidate_request_does_not_accept_duplicate_clicks() {
    let mut app = ready_visible_state_test_app();
    let request = FoundryCandidateRequest {
        seed: 1,
        proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
        result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::complete_look(),
    };

    let first = app
        .state
        .request_candidates(request.clone())
        .expect("first request schedules");
    let second = app
        .state
        .request_candidates(request)
        .expect("duplicate request is ignored");

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert_eq!(
        app.state
            .active_jobs
            .values()
            .filter(|request| request.slot() == FoundryJobSlot::GenerateCandidates)
            .count(),
        1
    );
    let visible = app.make_canvas_view_state();
    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert!(visible.primary_action_enabled);
    assert!(!visible.candidate_tray_visible);
}

#[test]
fn idea_generation_timeout_recovery_actions_are_visible() {
    let mut app = ready_visible_state_test_app();
    app.state
        .request_candidates(FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        })
        .expect("candidate job schedules");
    app.make_generation_started_at =
        Some(Instant::now() - IDEA_GENERATION_TIMEOUT - Duration::from_secs(1));

    let visible = app.make_canvas_view_state();

    assert!(!visible.idea_generation_timed_out);
    assert!(!visible.idea_generation_fallback_visible);
    assert_eq!(
        visible.local_banner_message,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert_eq!(
        visible.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    let labels = rendered_action_labels_for_default_shell();
    assert!(labels.contains(&ACTION_CANCEL));
    assert!(labels.contains(&ACTION_KEEP_WAITING));
}

#[test]
fn cancel_idea_generation_cancels_active_job_with_local_warning() {
    let mut app = ready_visible_state_test_app();
    let request = FoundryCandidateRequest {
        seed: 1,
        proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
        result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::complete_look(),
    };
    let effects = app
        .state
        .request_candidates(request)
        .expect("candidate job schedules");
    let job_id = effects
        .iter()
        .find_map(|effect| match effect {
            FoundryAppEffect::StartJob(job) => Some(job.job_id()),
            _ => None,
        })
        .expect("candidate job id");

    let cancel_effects = app
        .state
        .handle_command(FoundryAppCommand::CancelIdeaGeneration)
        .expect("cancel succeeds");
    let visible = app.make_canvas_view_state();

    assert!(cancel_effects.is_empty());
    assert!(!app.state.active_jobs.contains_key(&job_id));
    assert!(app.state.stale_jobs.contains(&job_id));
    assert_eq!(app.state.make_job_trace.summary().total_jobs_canceled, 1);
    assert_eq!(visible.local_warning_message.as_deref(), None);
    assert_eq!(visible.local_banner_title, "Ready");
    assert_eq!(
        visible.local_banner_message,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
}

#[test]
fn starting_template_queues_model_and_preview_automatically() {
    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();

    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    assert_eq!(app.tab, FoundryTab::Make);
    assert!(
        app.state
            .active_jobs
            .values()
            .any(|request| matches!(request, FoundryJobRequest::CompileCurrent { .. }))
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if app.state.current_output.is_some() && app.state.current_preview.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(app.state.current_output.is_some());
    assert!(app.state.current_preview.is_some());
}

#[test]
fn box_primitive_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let mut app = box_primitive_ready_state_test_app();
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_BOX_PRIMITIVE);
    assert_eq!(
        ready.property_labels,
        vec!["Width", "Depth", "Height", "Edge Softness"]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    let groups = app
        .state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)
        .expect("box document has direction groups");
    assert!(
        groups.is_empty(),
        "Box Primitive should not expose part focus chips"
    );
    assert!(app.make_primary_candidate_command().is_none());
    let controls = app
        .state
        .current_output
        .as_ref()
        .expect("compiled box output")
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(controls, vec!["Width", "Depth", "Height", "Edge Softness"]);

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(2.2),
        }))
        .expect("adjust box schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("width")),
        Some(&orchard_foundry::ControlValue::Scalar(2.2))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "surface",
        "material",
        "rig",
        "motion",
        "focus part",
        "try ideas",
        "candidate",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Box Primitive baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
}

#[test]
fn lidded_box_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::box_primitive::lidded_box_fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert!(ready.lidded_box_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_LIDDED_BOX);
    assert_eq!(
        ready.property_labels,
        vec!["Width", "Depth", "Height", "Edge Softness", "Lid Seam"]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    assert_eq!(app.material_look_export_copy(), None);
    assert_eq!(
        app.current_export_copy(),
        (LIDDED_BOX_EXPORT_TITLE, LIDDED_BOX_EXPORT_DETAIL)
    );
    assert!(
        app.state
            .current_output
            .as_ref()
            .expect("compiled lidded output")
            .catalog
            .customizer_profile
            .controls
            .iter()
            .any(|control| control.id == "lid_height" && control.label == "Lid Seam"),
        "Lidded Box Make should expose the Lid Seam control"
    );
    let groups = app
        .state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)
        .expect("lidded document has direction groups");
    assert!(
        groups.is_empty(),
        "Lidded Box should not expose part focus chips"
    );
    assert!(app.make_primary_candidate_command().is_none());

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "lid_height".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(0.8),
        }))
        .expect("adjust lid seam schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("lid_height")),
        Some(&orchard_foundry::ControlValue::Scalar(0.8))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        LIDDED_BOX_EXPORT_TITLE,
        LIDDED_BOX_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "crate",
        "case",
        "surface",
        "material looks",
        "focus part",
        "try ideas",
        "candidate",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Lidded Box baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn flat_panel_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::flat_panel::fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert!(ready.flat_panel_baseline);
    assert!(!ready.lidded_box_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_FLAT_PANEL);
    assert_eq!(
        ready.property_labels,
        vec!["Width", "Height", "Thickness", "Edge Softness"]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    assert_eq!(app.material_look_export_copy(), None);
    assert_eq!(
        app.current_export_copy(),
        (FLAT_PANEL_EXPORT_TITLE, FLAT_PANEL_EXPORT_DETAIL)
    );
    let controls = app
        .state
        .current_output
        .as_ref()
        .expect("compiled flat panel output")
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        vec!["Width", "Height", "Thickness", "Edge Softness"]
    );
    let groups = app
        .state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)
        .expect("flat panel document has direction groups");
    assert!(
        groups.is_empty(),
        "Flat Panel should not expose part focus chips"
    );
    assert!(app.make_primary_candidate_command().is_none());

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "edge_softness".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(0.08),
        }))
        .expect("adjust panel schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("edge_softness")),
        Some(&orchard_foundry::ControlValue::Scalar(0.08))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        FLAT_PANEL_EXPORT_TITLE,
        FLAT_PANEL_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "hinge",
        "handle",
        "knob",
        "material looks",
        "focus part",
        "open",
        "close",
        "try ideas",
        "candidate",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Flat Panel baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn sphere_primitive_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::sphere_primitive::fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_SPHERE_PRIMITIVE);
    assert_eq!(
        ready.property_labels,
        vec!["Width", "Height", "Depth", "Front Flatten", "Back Flatten"]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    assert_eq!(app.material_look_export_copy(), None);
    assert_eq!(
        app.current_export_copy(),
        (
            SPHERE_PRIMITIVE_EXPORT_TITLE,
            SPHERE_PRIMITIVE_EXPORT_DETAIL
        )
    );
    let controls = app
        .state
        .current_output
        .as_ref()
        .expect("compiled sphere output")
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        vec!["Width", "Height", "Depth", "Front Flatten", "Back Flatten"]
    );
    assert!(app.make_primary_candidate_command().is_none());

    let preset_commands = orchard_foundry_catalog::sphere_primitive::knob_like_form_preset_values()
        .into_iter()
        .map(|(control_id, value)| FoundryCommand::SetControl { control_id, value })
        .collect();
    let preset_effects = app
        .state
        .handle_command(FoundryAppCommand::RunFoundryCommandProgram {
            label: "Knob-like form".to_owned(),
            commands: preset_commands,
        })
        .expect("preset schedules");
    let preset_event = run_fixture_effect(preset_effects, &fixture);
    assert!(app.state.handle_job_event(preset_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("back_flatten")),
        Some(&orchard_foundry::ControlValue::Scalar(0.42))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        SPHERE_PRIMITIVE_EXPORT_TITLE,
        SPHERE_PRIMITIVE_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "material looks",
        "focus part",
        "open",
        "close",
        "try ideas",
        "candidate",
        "sculpt",
        "vertex",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Sphere Primitive baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn direct_property_controls_validate_reset_and_keep_view_copy_safe() {
    let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let profile = &app
        .state
        .current_output
        .as_ref()
        .expect("compiled box output")
        .catalog
        .customizer_profile;
    let width = profile
        .controls
        .iter()
        .find(|control| control.id == "width")
        .expect("width control");
    assert!(
        width
            .domain
            .contains_available_value(&orchard_foundry::ControlValue::Scalar(2.0))
    );
    assert!(
        !width
            .domain
            .contains_available_value(&orchard_foundry::ControlValue::Scalar(9.0))
    );

    let invalid_report = orchard_foundry::validate_foundry_command(
        &FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(9.0),
        },
        app.state.document.as_ref(),
        Some(profile),
    );
    assert!(!invalid_report.is_valid());
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("width")),
        Some(&orchard_foundry::ControlValue::Scalar(2.0))
    );

    let set_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(2.2),
        }))
        .expect("valid width edit schedules");
    assert_eq!(app.state.authoring_breadcrumbs.len(), 1);
    let breadcrumb = &app.state.authoring_breadcrumbs[0];
    assert_eq!(breadcrumb.control_id, "width");
    assert_eq!(breadcrumb.property_id, "box.width");
    assert_eq!(
        breadcrumb.entry.effect,
        orchard_authoring::AuthoringEffect::SetProperty
    );
    assert_eq!(breadcrumb.requested_control_value, 2.2);
    assert_eq!(breadcrumb.authored_recipe_value, 1.1);
    assert!(breadcrumb.validation_report.accepted);
    let set_event = run_fixture_effect(set_effects, &fixture);
    assert!(app.state.handle_job_event(set_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("width")),
        Some(&orchard_foundry::ControlValue::Scalar(2.2))
    );

    let reset_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::ResetControl {
            control_id: "width".to_owned(),
        }))
        .expect("reset schedules");
    let reset_event = run_fixture_effect(reset_effects, &fixture);
    assert!(app.state.handle_job_event(reset_event));
    assert!(
        !app.state
            .document
            .as_ref()
            .expect("document")
            .control_state
            .contains_key("width")
    );

    let strings = product_visible_strings_for_default_shell()
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("\n");
    for required in [VIEW_ORBIT_LABEL, VIEW_RESET_LABEL, VIEW_AXIS_LABEL] {
        assert!(strings.contains(&required.to_ascii_lowercase()));
    }
    for forbidden in [
        "mesh edit",
        "gizmo",
        "vertex",
        "face selection",
        "edit face",
    ] {
        assert!(
            !strings.contains(forbidden),
            "direct property UI must not expose {forbidden}: {strings}"
        );
    }
}

#[test]
fn hinged_panel_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::flat_panel::hinged_panel_fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert!(ready.hinged_panel_baseline);
    assert!(!ready.flat_panel_baseline);
    assert!(!ready.lidded_box_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_HINGED_PANEL);
    assert_eq!(
        ready.property_labels,
        vec![
            "Width",
            "Height",
            "Thickness",
            "Edge Softness",
            "Hinge Edge"
        ]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    assert_eq!(app.material_look_export_copy(), None);
    assert_eq!(
        app.current_export_copy(),
        (HINGED_PANEL_EXPORT_TITLE, HINGED_PANEL_EXPORT_DETAIL)
    );
    let controls = app
        .state
        .current_output
        .as_ref()
        .expect("compiled hinged panel output")
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(controls, vec!["Proportions", "Edge Softness", "Hinge Edge"]);
    let groups = app
        .state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)
        .expect("hinged panel document has direction groups");
    assert!(
        groups.is_empty(),
        "Hinged Panel should not expose part focus chips"
    );
    assert!(app.make_primary_candidate_command().is_none());

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "hinge_edge_style".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(0.75),
        }))
        .expect("adjust hinge edge schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("hinge_edge_style")),
        Some(&orchard_foundry::ControlValue::Scalar(0.75))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        HINGED_PANEL_EXPORT_TITLE,
        HINGED_PANEL_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "handle",
        "knob",
        "material looks",
        "focus part",
        "open",
        "close",
        "try ideas",
        "candidate",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Hinged Panel baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn handled_panel_make_baseline_flow_is_plain_and_complete() {
    let fixture = orchard_foundry_catalog::flat_panel::handled_panel_fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert!(ready.handled_panel_baseline);
    assert!(!ready.hinged_panel_baseline);
    assert!(!ready.flat_panel_baseline);
    assert!(!ready.lidded_box_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_HANDLED_PANEL);
    assert_eq!(
        ready.property_labels,
        vec![
            "Width",
            "Height",
            "Thickness",
            "Edge Softness",
            "Hinge Edge",
            "Handle"
        ]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    let (empty_tray_title, empty_tray_message) = empty_candidate_tray_copy(&ready);
    assert_eq!(empty_tray_title, "Direct properties ready");
    assert_eq!(
        empty_tray_message,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
    assert!(
        !empty_tray_message.contains("box"),
        "Handled Panel empty tray copy must stay panel-specific: {empty_tray_message}"
    );
    assert!(!app.material_look_action_visible(&ready));
    assert!(!ready.material_look_tray_visible);
    assert_eq!(app.material_look_export_copy(), None);
    assert_eq!(
        app.current_export_copy(),
        (HANDLED_PANEL_EXPORT_TITLE, HANDLED_PANEL_EXPORT_DETAIL)
    );
    let controls = app
        .state
        .current_output
        .as_ref()
        .expect("compiled handled panel output")
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        vec![
            "Proportions",
            "Edge Softness",
            "Hinge Edge",
            "Handle / Knob Style"
        ]
    );
    let groups = app
        .state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)
        .expect("handled panel document has direction groups");
    assert!(
        groups.is_empty(),
        "Handled Panel should not expose part focus chips"
    );
    assert!(app.make_primary_candidate_command().is_none());

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "handle_knob_style".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(0.85),
        }))
        .expect("adjust handle schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("handle_knob_style")),
        Some(&orchard_foundry::ControlValue::Scalar(0.85))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        HANDLED_PANEL_EXPORT_TITLE,
        HANDLED_PANEL_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "material looks",
        "focus part",
        "open",
        "close",
        "try ideas",
        "candidate",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Handled Panel baseline copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn panel_knob_make_flow_is_direct_composition_without_generated_ideas() {
    let fixture = orchard_foundry_catalog::panel_knob::fixture_catalog();
    let mut app = ready_fixture_state_test_app(fixture.clone());
    let ready = app.make_canvas_view_state();

    assert_eq!(ready.mode, MakeCanvasMode::Ready);
    assert!(ready.direct_primitive_workflow);
    assert!(ready.simple_box_make_baseline);
    assert!(ready.panel_knob_baseline);
    assert!(!ready.handled_panel_baseline);
    assert_eq!(ready.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert_eq!(ready.property_panel_title, ACTION_EDIT_PANEL_KNOB);
    assert_eq!(
        ready.property_labels,
        vec![
            "Panel Width",
            "Panel Height",
            "Panel Thickness",
            "Panel Edge Softness",
            "Knob Width",
            "Knob Height",
            "Knob Depth",
            "Knob Front Flatten",
            "Knob Back Flatten",
            "Knob Horizontal Position",
            "Knob Vertical Position",
        ]
    );
    assert_eq!(
        ready.next_action_hint,
        "Adjust properties, Add to Pack, or Export current asset."
    );
    assert!(!ready.candidate_tray_visible);
    assert!(!ready.selected_comparison_visible);
    let (empty_tray_title, empty_tray_message) = empty_candidate_tray_copy(&ready);
    assert_eq!(empty_tray_title, "Direct properties ready");
    assert_eq!(
        empty_tray_message,
        "Adjust properties, Add to Pack, or Export current asset."
    );
    assert_eq!(
        app.current_export_copy(),
        (PANEL_KNOB_EXPORT_TITLE, PANEL_KNOB_EXPORT_DETAIL)
    );

    let visible_controls = app
        .state
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        visible_controls,
        vec![
            "Panel Width",
            "Panel Height",
            "Panel Thickness",
            "Panel Edge Softness",
            "Knob Width",
            "Knob Height",
            "Knob Depth",
            "Knob Front Flatten",
            "Knob Back Flatten",
            "Knob Horizontal Position",
            "Knob Vertical Position",
        ]
    );
    assert_eq!(
        app.state
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count(),
        7
    );
    let sections = make_context_inspector_controls(&app.state.controls, None);
    assert_eq!(sections.visible.len(), MAKE_CONTEXT_INITIAL_CONTROL_LIMIT);
    assert!(
        sections
            .overflow
            .iter()
            .any(|control| control.id == "knob_x_offset")
    );
    assert!(
        sections
            .overflow
            .iter()
            .any(|control| control.id == "knob_y_offset")
    );

    let adjust_effects = app
        .state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "knob_x_offset".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(0.96),
        }))
        .expect("adjust knob offset schedules");
    let adjust_event = run_fixture_effect(adjust_effects, &fixture);
    assert!(app.state.handle_job_event(adjust_event));
    assert_eq!(
        app.state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("knob_x_offset")),
        Some(&orchard_foundry::ControlValue::Scalar(0.96))
    );

    let pack_effects = app
        .state
        .handle_command(app.add_current_to_pack_command().expect("pack command"))
        .expect("Add to Pack schedules");
    let pack_event = run_fixture_effect(pack_effects, &fixture);
    assert!(app.state.handle_job_event(pack_event));
    assert_eq!(app.state.pack.members.len(), 1);

    app.drawer = Some(FoundryDrawer::Pack);
    assert!(app.make_canvas_view_state().pack_drawer_visible);
    app.drawer = Some(FoundryDrawer::Export);
    assert!(app.make_canvas_view_state().export_drawer_visible);
    assert!(app.make_primary_candidate_command().is_none());

    let simple_make_copy = [
        ready.primary_action_label.as_str(),
        ready.property_panel_title,
        PANEL_KNOB_EXPORT_TITLE,
        PANEL_KNOB_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        ACTION_ADD_TO_PACK,
        ACTION_EXPORT_CURRENT_ASSET,
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "material looks",
        "focus part",
        "open",
        "close",
        "try ideas",
        "candidate",
        "free transform",
        "vertex",
        "face edit",
    ] {
        assert!(
            !simple_make_copy.contains(forbidden),
            "Panel with Knob copy must not expose {forbidden}: {simple_make_copy}"
        );
    }
    assert!(simple_make_copy.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn active_candidate_job_disables_conflicting_actions() {
    let mut app = visible_state_test_app();
    app.state.current_output = Some(Box::new(
        compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &orchard_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles"),
    ));
    app.state.current_preview = Some(test_preview_image("current"));
    app.state
        .request_candidates(FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        })
        .expect("candidate job schedules");

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert_eq!(visible.primary_action_label, ACTION_ADJUST_DIMENSIONS);
    assert!(visible.primary_action_enabled);
    assert!(!make_canvas_candidate_actions_enabled(&visible));
    assert!(visible.local_busy_label.is_none());
    assert!(!visible.local_busy_visible);
    assert!(!visible.candidate_tray_visible);
    assert_eq!(
        visible.next_action_hint,
        "Adjust dimensions, Add to Pack, or Export current primitive."
    );
}

#[test]
fn rejected_candidate_summary_is_local_make_state() {
    let mut app = ready_visible_state_test_app();
    app.state.status = Some("Found 4 clear ideas. Rejected 2 that looked too similar.".to_owned());

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.rejected_candidate_summary.as_deref(), None);
    assert!(!visible.candidate_tray_visible);
}

#[test]
fn active_edit_job_marks_current_build_stale_for_pack_and_export() {
    let mut app = ready_visible_state_test_app();

    app.state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: orchard_foundry::ControlValue::Scalar(2.2),
        }))
        .expect("edit schedules rebuild");

    let visible = app.make_canvas_view_state();

    assert_eq!(visible.mode, MakeCanvasMode::Ready);
    assert!(visible.model_ready);
    assert!(visible.preview_ready);
    assert!(visible.preview_updating);
    assert!(visible.local_busy_label.is_none());
    assert!(!visible.local_busy_visible);
    assert!(!make_canvas_build_dependent_actions_enabled(&visible));
    assert_eq!(
        make_canvas_build_dependent_disabled_reason(&visible),
        PREVIEW_UPDATING_REASON
    );
}

#[test]
fn stale_result_status_becomes_local_make_warning() {
    let mut app = ready_visible_state_test_app();
    app.legacy_candidate_ui_enabled = true;
    force_other_profile(&mut app);
    app.state.status = Some("Ignored a background result because newer work is active.".to_owned());

    let visible = app.make_canvas_view_state();

    assert_eq!(
        visible.local_warning_message.as_deref(),
        Some(STALE_RESULT_WARNING)
    );
    assert!(visible.local_error_message.is_none());
}

#[test]
fn make_canvas_forbidden_product_terms_are_absent_from_default_strings() {
    let strings = product_visible_strings_for_default_shell();

    for label in &strings {
        assert!(
            crate::foundry::ui::copy::first_forbidden_product_term(label).is_none(),
            "default product string contains forbidden implementation copy: {label}"
        );
    }

    let joined = strings.join("\n").to_ascii_lowercase();
    for forbidden_phrase in ["fingerprint", "gltf primitive"] {
        assert!(
            !joined.contains(forbidden_phrase),
            "default product strings contain forbidden phrase {forbidden_phrase}: {joined}"
        );
    }
    assert!(joined.contains("not a textured, rigged, animated, or game-ready package"));
}

#[test]
fn rendered_action_labels_are_in_product_visible_inventory() {
    let strings = product_visible_strings_for_default_shell();
    for label in rendered_action_labels_for_default_shell() {
        assert!(
            strings.contains(label),
            "missing rendered action label {label}"
        );
        assert!(
            crate::foundry::ui::copy::first_forbidden_product_term(label).is_none(),
            "rendered action label contains forbidden product copy: {label}"
        );
    }
}

#[test]
fn core_make_actions_are_not_quiet_text_buttons() {
    for spec in core_make_action_specs_for_default_shell() {
        assert_ne!(
            spec.tone,
            ButtonTone::Quiet,
            "core Make action should render as a visible button: {}",
            spec.label
        );
        assert!(spec.validate().is_ok());
    }
}

#[test]
fn directions_panel_exposes_all_generation_modes() {
    let labels = direction_mode_actions_for_panel()
        .into_iter()
        .map(|action| action.label)
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec!["Refine", "Explore", "Silhouette", "Structure", "Detail"]
    );
    for label in labels {
        assert!(crate::foundry::ui::copy::first_forbidden_product_term(label).is_none());
    }
}

#[test]
fn launch_save_state_does_not_claim_project_is_saved() {
    let mut app = FoundryDesktopApp::default();
    assert_eq!(
        app.save_state_pill(),
        ("Choose starting point", StatusTone::Neutral)
    );

    app.state =
        FoundryAppState::new(orchard_foundry_catalog::box_primitive::fixture_catalog().document)
            .expect("fixture state");
    app.state.project_path = None;
    app.state.dirty = false;
    assert_eq!(app.save_state_pill(), ("Not saved", StatusTone::Warning));

    app.state.project_path = Some(PathBuf::from("box_primitive.shapelab-foundry.json"));
    app.state.dirty = true;
    assert_eq!(app.save_state_pill(), ("Unsaved", StatusTone::Warning));

    app.state.dirty = false;
    assert_eq!(app.save_state_pill(), ("Saved", StatusTone::Ready));
}

#[test]
fn recent_projects_are_real_load_targets_and_keep_newest_first() {
    let mut app = FoundryDesktopApp::default();
    let first = PathBuf::from("first.shapelab-foundry.json");
    let second = PathBuf::from("second.shapelab-foundry.json");
    let third = PathBuf::from("third.shapelab-foundry.json");

    app.remember_recent_project(first.clone());
    app.remember_recent_project(second.clone());
    app.remember_recent_project(third.clone());
    app.remember_recent_project(first.clone());

    assert_eq!(app.recent_projects, vec![first, third, second]);
}

#[test]
fn default_customize_summaries_hide_internal_control_kinds() {
    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();
    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if !app.state.controls.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let controls = display_customize_controls(&app.state.controls);
    assert!(controls.len() <= CUSTOMIZE_PRIMARY_CONTROL_LIMIT);
    assert!(!controls.is_empty());
    assert!(controls.iter().any(|control| control.id == "width"));
    assert!(controls.iter().any(|control| control.id == "depth"));
    assert!(controls.iter().any(|control| control.id == "height"));
    assert!(controls.iter().any(|control| control.id == "edge_softness"));
    assert!(
        controls
            .iter()
            .filter(|control| control.numeric_range.is_some())
            .count()
            >= 4
    );
    let visible = controls
        .iter()
        .map(|control| product_control_summary(control))
        .collect::<Vec<_>>();
    assert!(!visible.contains(&"Bounded property"));
    assert!(visible.contains(&"Controls width."));
    assert!(visible.contains(&"Controls depth."));
    assert!(visible.contains(&"Controls height."));
    assert!(visible.contains(&"Controls corner softness."));
    assert!(
        crate::foundry::ui::copy::labels_are_product_safe(&visible),
        "visible customize summaries contain implementation copy: {visible:?}"
    );
}

#[test]
fn direct_exact_value_fallback_lists_primary_make_controls() {
    let cases = [
        (
            orchard_foundry_catalog::box_primitive::fixture_catalog(),
            vec!["Width", "Depth", "Height", "Edge Softness"],
        ),
        (
            orchard_foundry_catalog::flat_panel::fixture_catalog(),
            vec!["Width", "Height", "Thickness", "Edge Softness"],
        ),
        (
            orchard_foundry_catalog::sphere_primitive::fixture_catalog(),
            vec!["Width", "Height", "Depth", "Front Flatten", "Back Flatten"],
        ),
        (
            orchard_foundry_catalog::panel_knob::fixture_catalog(),
            vec![
                "Panel Width",
                "Panel Height",
                "Panel Thickness",
                "Panel Edge Softness",
                "Knob Width",
                "Knob Height",
                "Knob Depth",
                "Knob Front Flatten",
                "Knob Back Flatten",
                "Knob Horizontal Position",
                "Knob Vertical Position",
            ],
        ),
    ];

    for (fixture, expected_labels) in cases {
        let app = ready_fixture_state_test_app(fixture);
        let visible_labels = direct_exact_value_controls(&app.state.controls)
            .iter()
            .map(|control| control.label.clone())
            .collect::<Vec<_>>();
        for expected_label in expected_labels {
            assert!(
                visible_labels.iter().any(|label| label == expected_label),
                "direct exact-value fallback should include {expected_label}: {visible_labels:?}"
            );
        }
    }
}

#[test]
fn direct_make_ready_actions_are_available_from_exact_value_panel() {
    let ready = ready_visible_state_test_app().make_canvas_view_state();

    assert!(ready.direct_primitive_workflow);
    assert!(make_canvas_inspector_build_actions_visible(&ready));
    assert!(make_canvas_build_dependent_actions_enabled(&ready));
    assert!(rendered_action_labels_for_default_shell().contains(&ACTION_ADD_TO_PACK));
    assert!(rendered_action_labels_for_default_shell().contains(&ACTION_EXPORT_CURRENT_PRIMITIVE));
}

#[test]
fn box_primitive_filmstrip_shows_all_options_without_overflow() {
    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();
    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if !app.state.controls.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let controls = display_customize_controls(&app.state.controls);
    assert!(!controls.is_empty());
    assert!(
        controls
            .iter()
            .all(|control| control.options.len() <= CONTROL_FILMSTRIP_LIMIT)
    );
}

#[test]
fn pack_member_labels_hide_source_document_ids() {
    let member = pack::PackMemberRow {
        member_id: "box-primitive-doc".to_owned(),
        name: "box-primitive-doc".to_owned(),
        document_id: "box-primitive-doc".to_owned(),
        selected: false,
        override_count: 0,
    };

    let label = pack_member_display_name(&member);
    assert_eq!(label, "Box Primitive");
    assert!(!label.contains("-doc"));

    let cell = pack::PackContactSheetCell {
        row: 0,
        column: 0,
        member_id: "box-primitive-doc".to_owned(),
        name: "box-primitive-doc".to_owned(),
        document_id: "box-primitive-doc".to_owned(),
        status: pack::PackMemberStatus::Ready,
        override_count: 0,
        selected: false,
    };
    let cell_label = pack_cell_display_name(&cell);
    assert_eq!(cell_label, "Box Primitive");
    assert!(!cell_label.contains("-doc"));
}

#[test]
fn pack_contact_sheet_uses_product_safe_thumbnail_markers() {
    let current_cell = pack::PackContactSheetCell {
        row: 0,
        column: 0,
        member_id: "box-primitive-doc".to_owned(),
        name: "box-primitive-doc".to_owned(),
        document_id: "box-primitive-doc".to_owned(),
        status: pack::PackMemberStatus::Ready,
        override_count: 0,
        selected: true,
    };
    let other_cell = pack::PackContactSheetCell {
        selected: false,
        ..current_cell.clone()
    };

    assert_eq!(pack_thumbnail_marker(&current_cell), "Current");
    assert_eq!(pack_thumbnail_marker(&other_cell), "Preview");
    assert!(crate::foundry::ui::copy::labels_are_product_safe(&[
        pack_thumbnail_marker(&current_cell),
        pack_thumbnail_marker(&other_cell)
    ]));
}

#[test]
fn product_panel_messages_replace_raw_backend_details() {
    assert_eq!(
        product_panel_message(
            "members.box-primitive-doc.document_id failed validation",
            "Pack needs attention before export."
        ),
        "Pack needs attention before export."
    );
    assert_eq!(
        product_panel_message(
            "Could not render C:\\tmp\\preview.png",
            "Preview could not be rendered for this direction."
        ),
        "Preview could not be rendered for this direction."
    );
    assert_eq!(
        product_panel_message("This option is locked.", "Fallback"),
        "This option is locked."
    );
}

#[test]
fn workflow_copy_maps_to_foundry_tabs_without_history_as_primary_step() {
    let tabs = WORKFLOW_STEPS
        .iter()
        .map(|step| tab_for_workflow_step(step.index))
        .collect::<Vec<_>>();

    assert_eq!(tabs, vec![FoundryTab::Home, FoundryTab::Make]);
    assert!(!tabs.contains(&FoundryTab::History));
}

#[test]
fn product_status_hides_raw_paths_from_status_strip() {
    assert_eq!(
        product_safe_status("Saved C:\\work\\box.shapelab-foundry.json"),
        "Project saved"
    );
    assert_eq!(
        product_safe_status("Loaded C:\\work\\box.shapelab-foundry.json"),
        "Project loaded"
    );
    assert_eq!(
        product_safe_status("Could not use C:\\work\\broken.json"),
        "Project path needs attention"
    );
    assert_eq!(
        product_safe_status("Exported default to C:\\exports\\box"),
        "Export complete"
    );
    assert_eq!(
        product_safe_status("Exported 3 pack member(s) with default to C:\\exports\\pack"),
        "Pack export complete"
    );
    assert_eq!(
        product_safe_status("provider socket failed conformance check"),
        "Project needs attention"
    );
    assert_eq!(
        product_safe_status("recipe fragment remap failed"),
        "Project needs attention"
    );
}

#[test]
fn preview_texture_identity_tracks_render_metadata_without_pixel_scan() {
    let box_a = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let build_a = compile_foundry_document(&box_a.document, &box_a)
        .expect("box fixture compiles")
        .build_stamp;

    let identity = FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1);

    assert_eq!(
        identity,
        FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1)
    );
    assert_ne!(
        identity,
        FoundryTextureIdentity::new("option-b", Some(&build_a), 2, 1)
    );
    assert_eq!(
        identity,
        FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1)
    );
    assert_ne!(
        identity,
        FoundryTextureIdentity::new("option-a", Some(&build_a), 1, 2)
    );
}

#[test]
fn desktop_foundry_exposes_product_steps_and_box_profile() {
    let tabs = [FoundryTab::Home, FoundryTab::Make, FoundryTab::History];
    assert_eq!(tabs.len(), 3);

    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();
    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if app.state.current_output.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(app.tab, FoundryTab::Make);
    assert_eq!(
        app.state
            .document
            .as_ref()
            .map(|document| document.family_content_ref.stable_id.as_str()),
        Some("box-primitive-family")
    );
    assert!(app.state.current_output.is_some());
}

#[test]
fn loading_project_enters_workflow_step() {
    let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let state = FoundryAppState::new(fixture.document).expect("fixture state");
    let mut project_file = state.project_file.expect("project file");
    let path = temp_foundry_project_path("load-enters-workflow");
    project_file.save_as(&path).expect("project saves");
    let mut app = FoundryDesktopApp::default();

    app.load_project(path.clone(), &egui::Context::default());

    assert_eq!(app.tab, FoundryTab::Make);
    assert!(app.state.document.is_some());
    let _ = std::fs::remove_file(path);
}

#[test]
fn default_customize_surface_hides_non_primary_and_hidden_controls() {
    let ctx = egui::Context::default();
    let mut app = FoundryDesktopApp::default();
    app.load_fixture(
        orchard_foundry_catalog::box_primitive::fixture_catalog(),
        &ctx,
    );

    for _ in 0..3000 {
        app.poll_jobs(&ctx);
        if default_customize_controls(&app.state.controls).any(|control| control.id == "width") {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let default_ids = default_customize_controls(&app.state.controls)
        .map(|control| control.id.as_str())
        .collect::<Vec<_>>();
    assert!(default_ids.contains(&"width"));
    assert!(default_ids.contains(&"depth"));
    assert!(default_ids.contains(&"height"));
    assert!(default_ids.contains(&"edge_softness"));
    assert!(
        default_customize_controls(&app.state.controls)
            .all(|control| control.primary && control.visible)
    );
}

fn visible_state_test_app() -> FoundryDesktopApp {
    fixture_state_test_app(orchard_foundry_catalog::box_primitive::fixture_catalog())
}

fn family_studio_lite_strings(state: &FamilyStudioLiteUiState) -> Vec<String> {
    let mut strings = vec![
        state.starting_point_title.clone(),
        state.starting_point_summary.clone(),
        state.source_label.to_owned(),
    ];
    strings.extend(state.disabled_reason.clone());
    strings.extend(state.stays_same.clone());
    for card in &state.capability_cards {
        strings.push(card.display_name.clone());
        strings.push(card.description.clone());
        strings.push(card.status_label.to_owned());
    }
    if let Some(result) = &state.test_result {
        strings.push(result.message.clone());
        strings.push(format!("{:?}", result.status));
    }
    if let Some(error) = &state.save_error {
        strings.push(error.clone());
    }
    if let Some(visibility) = state.saved_visibility {
        strings.push(format!("{visibility:?}"));
    }
    strings
}

fn isolated_family_studio_lite_store(name: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "object-orchard-family-studio-lite-test-{name}-{}",
        std::process::id()
    ))
}

fn fixture_state_test_app(fixture: FoundryFixtureCatalog) -> FoundryDesktopApp {
    FoundryDesktopApp {
        tab: FoundryTab::Make,
        state: FoundryAppState::new(fixture.document).expect("fixture state"),
        ..FoundryDesktopApp::default()
    }
}

fn ready_visible_state_test_app() -> FoundryDesktopApp {
    ready_fixture_state_test_app(orchard_foundry_catalog::box_primitive::fixture_catalog())
}

fn force_other_profile(app: &mut FoundryDesktopApp) {
    if let Some(document) = app.state.document.as_mut() {
        document.document_id.0 = "legacy-candidate-flow-doc".to_owned();
        document.family_content_ref.stable_id = "legacy-candidate-flow-family".to_owned();
        document.customizer_profile_ref.stable_id = "legacy-candidate-flow-profile".to_owned();
    }
}

fn ready_fixture_state_test_app(fixture: FoundryFixtureCatalog) -> FoundryDesktopApp {
    let mut app = fixture_state_test_app(fixture.clone());
    let effects = app.state.request_build().expect("fixture schedules build");
    let event = run_fixture_effect(effects, &fixture);
    assert!(app.state.handle_job_event(event));
    let build = app.state.current_build.clone();
    app.state.current_preview = Some(test_preview_image_for_build("current", build));
    app
}

fn box_primitive_ready_state_test_app() -> FoundryDesktopApp {
    let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
    let mut app = FoundryDesktopApp {
        tab: FoundryTab::Make,
        state: FoundryAppState::new(fixture.document.clone()).expect("fixture state"),
        ..FoundryDesktopApp::default()
    };
    let output = compile_foundry_document(app.state.document.as_ref().expect("document"), &fixture)
        .expect("fixture compiles");
    let build = output.build_stamp.clone();
    app.state.current_build = Some(build.clone());
    app.state.current_output = Some(Box::new(output));
    app.state.current_preview = Some(test_preview_image_for_build("current", Some(build)));
    app
}

fn run_fixture_effect(
    effects: Vec<FoundryAppEffect>,
    fixture: &FoundryFixtureCatalog,
) -> FoundryJobEvent {
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected exactly one start job effect, got {effects:?}");
    };
    run_foundry_job(
        job.as_ref().clone(),
        fixture,
        &mut FoundryPreviewCache::default(),
    )
}

fn test_preview_image(preview_id: &str) -> FoundryPreviewImage {
    test_preview_image_for_build(preview_id, None)
}

fn test_preview_image_for_build(
    preview_id: &str,
    build: Option<FoundryBuildStamp>,
) -> FoundryPreviewImage {
    FoundryPreviewImage {
        preview_id: preview_id.to_owned(),
        rgba8: vec![24, 32, 40, 255],
        width: 1,
        height: 1,
        camera: OrbitCamera::default(),
        build,
    }
}

fn empty_test_candidate_output(
    hidden_internal_rejections: usize,
    duplicate_looking_rejections: usize,
    wrong_scope_rejections: usize,
) -> FoundryCandidateOutput {
    FoundryCandidateOutput {
        candidates: Vec::new(),
        diagnostics: orchard_search_internal::foundry::FoundryCandidateGenerationDiagnostics {
            requested_proposals: directions::DEFAULT_DIRECTION_PROPOSALS,
            requested_candidates: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            attempted_proposals: directions::DEFAULT_DIRECTION_PROPOSALS,
            scored_candidates: 0,
            accepted_candidates: 0,
            returned_candidates: 0,
            available_control_count: 0,
            locked_targets_skipped: 0,
            rejections: std::collections::BTreeMap::new(),
            duplicate_looking_rejections,
            hidden_internal_rejections,
            wrong_scope_rejections,
            human_summary: "Generated 0 clear ideas.".to_owned(),
        },
        reliability_report:
            orchard_search_internal::foundry::FoundryCandidateReliabilityReport::default(),
        scoring_report: orchard_search_internal::asset::scoring::AssetScoringReport {
            rejected_candidates: Vec::new(),
            scored_candidates: Vec::new(),
            unique_candidates: Vec::new(),
            duplicate_groups: Vec::new(),
            representatives: Vec::new(),
        },
        preference_report: orchard_search_internal::foundry::FoundryCandidatePreferenceReport {
            requested: false,
            applied: false,
            scope_matched: false,
            scope: orchard_foundry::FoundryPreferenceScope::new("test-family", "test-profile"),
            ignored_reason: None,
            selected_scores: Vec::new(),
        },
    }
}

fn test_candidate_card(
    candidate_id: &str,
    selected: bool,
    focus_part_label: Option<String>,
) -> crate::foundry::view_model::FoundryCandidateCard {
    crate::foundry::view_model::FoundryCandidateCard {
        id: orchard_foundry::FoundryCandidateId(candidate_id.to_owned()),
        slot: 0,
        mode: Some(FoundryCandidateMode::Explore),
        parent: false,
        title: "Readable box idea".to_owned(),
        subtitle: "Clear model change".to_owned(),
        preview_id: Some(format!("{candidate_id}-preview")),
        rgba8: vec![220, 225, 214, 255],
        width: 1,
        height: 1,
        camera: Some(OrbitCamera::default()),
        preview_failure: None,
        changed_controls: vec!["Proportions".to_owned()],
        changed_roles: vec!["Body".to_owned()],
        explanations: Vec::new(),
        rejections: std::collections::BTreeMap::new(),
        validation_label: "Ready".to_owned(),
        validation_detail: None,
        selectable: true,
        selected,
        variation_intent_label: "Body idea".to_owned(),
        variation_scope_label: "Focused: Body".to_owned(),
        variation_channel_labels: vec!["Shape".to_owned()],
        visible_delta_label: "Clear change".to_owned(),
        what_changed_summary: "Body proportions change visibly.".to_owned(),
        legibility_class: orchard_foundry::CandidateLegibilityClass::Clear,
        focus_part_label,
        surface_unavailable_reason: None,
    }
}

fn set_test_focus_scope(app: &mut FoundryDesktopApp, group_id: &str, display_name: &str) {
    let document = app.state.document.as_mut().expect("fixture document");
    document.variation_state.intent = VariationIntent {
        scope: orchard_foundry::VariationScope::SemanticPartGroup {
            group_id: group_id.to_owned(),
            display_name: display_name.to_owned(),
        },
        channels: vec![orchard_foundry::VariationChannel::Shape],
        human_label: format!("Focused: {display_name}"),
        human_summary: format!("Try {display_name} ideas."),
    };
}

fn test_control_view(id: &str, label: &str) -> crate::foundry::view_model::FoundryControlView {
    crate::foundry::view_model::FoundryControlView {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        kind: "Control".to_owned(),
        presentation: crate::foundry::view_model::FoundryControlPresentation::ContinuousMacroAxis,
        value: None,
        default_value: None,
        primary: true,
        visible: true,
        locked: false,
        locked_reason: None,
        topology_behavior: orchard_foundry::ControlTopologyBehavior::TopologyPreserving,
        divergence: orchard_foundry::ControlDivergence::Synced,
        options: Vec::new(),
        numeric_range: None,
        advanced_path: None,
        help: None,
    }
}

#[derive(Clone, Copy)]
struct TestMaterialLookEvidenceOptions<'a> {
    frozen_mesh_fingerprint: &'a str,
    profile_id: &'a str,
    shape_delta_leak: bool,
    missing_preview: bool,
    full_ready_status: &'a str,
}

impl Default for TestMaterialLookEvidenceOptions<'_> {
    fn default() -> Self {
        Self {
            frozen_mesh_fingerprint: "box-fingerprint",
            profile_id: BOX_PRIMITIVE_PROFILE_ID,
            shape_delta_leak: false,
            missing_preview: false,
            full_ready_status: "blocked",
        }
    }
}

fn write_test_material_look_evidence(
    root: &Path,
    options: TestMaterialLookEvidenceOptions<'_>,
) -> PathBuf {
    let variants_dir = root.join("surface/variants");
    std::fs::create_dir_all(&variants_dir).expect("variants dir");
    let candidate_ids = [
        "clean-lab-white",
        "worn-hazard-yellow",
        "dark-industrial-metal",
        "field-blue-utility",
        "graphite-box",
        "orange-warning-edge-detail",
    ];
    let blockers = [
        "manual_review_pending",
        "engine_import_proof_missing",
        "engine_native_package_not_implemented",
        "surface_manual_review_required",
    ];
    let mut report_rows = Vec::new();
    let mut candidate_rows = Vec::new();
    for (index, (candidate_id, display_name)) in
        candidate_ids.iter().zip(MATERIAL_LOOK_TITLES).enumerate()
    {
        let variant_dir = variants_dir.join(candidate_id);
        std::fs::create_dir_all(&variant_dir).expect("variant dir");
        let rel_dir = format!("surface/variants/{candidate_id}");
        let material_override_ref = format!("{rel_dir}/material-override.json");
        let textured_preview_ref = format!("{rel_dir}/textured-preview.png");
        let surface_delta_ref = format!("{rel_dir}/surface-delta.json");
        let validation_ref = format!("{rel_dir}/validation.json");
        let shape_delta = options.shape_delta_leak && index == 0;

        std::fs::write(
            root.join(&material_override_ref),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "profile_id": options.profile_id,
                "candidate_id": candidate_id,
                "display_name": display_name
            }))
            .expect("material override json"),
        )
        .expect("material override writes");
        if !(options.missing_preview && index == 0) {
            let pixel = image::Rgba([
                32_u8.saturating_add((index as u8).saturating_mul(24)),
                96,
                160,
                255,
            ]);
            let preview = image::RgbaImage::from_pixel(2, 2, pixel);
            preview
                .save(root.join(&textured_preview_ref))
                .expect("preview writes");
        }
        std::fs::write(
            root.join(&surface_delta_ref),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "profile_id": options.profile_id,
                "candidate_id": candidate_id,
                "shape_delta_leak_detected": shape_delta,
                "result_class": "clear"
            }))
            .expect("delta json"),
        )
        .expect("delta writes");
        std::fs::write(
            root.join(&validation_ref),
            serde_json::to_vec_pretty(&serde_json::json!({
                "valid": !(shape_delta || options.missing_preview && index == 0),
                "blocker_codes": []
            }))
            .expect("validation json"),
        )
        .expect("validation writes");

        report_rows.push(serde_json::json!({
            "candidate_id": candidate_id,
            "display_name": display_name,
            "material_override_ref": material_override_ref,
            "textured_preview_ref": textured_preview_ref,
            "surface_delta_ref": surface_delta_ref,
            "validation_ref": validation_ref,
            "result_class": "clear",
            "shape_delta_leak_detected": shape_delta,
            "visible_surface_pixel_delta": 0.05
        }));
        candidate_rows.push(serde_json::json!({
            "candidate_id": candidate_id,
            "display_name": display_name,
            "changed_material_slots": [
                "painted_metal_body",
                "shadowed_body_edges",
                "exposed_edge_detail",
                "soft_edge_highlights",
                "fallback_hard_surface"
            ],
            "material_override_ref": material_override_ref,
            "textured_preview_ref": textured_preview_ref,
            "surface_delta_ref": surface_delta_ref,
            "validation_ref": validation_ref,
            "frozen_mesh_fingerprint": options.frozen_mesh_fingerprint,
            "preserves_frozen_geometry": !shape_delta,
            "full_ready_status": options.full_ready_status,
            "blocked_full_ready": options.full_ready_status == "blocked"
        }));
    }

    std::fs::write(
        variants_dir.join(SURFACE_CANDIDATE_SET_FILE),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "profile_id": options.profile_id,
            "candidates": candidate_rows
        }))
        .expect("candidate set json"),
    )
    .expect("candidate set writes");
    let report_path = variants_dir.join("surface-candidate-report.json");
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "profile_id": options.profile_id,
            "visual_foundry_surface_mode_enabled": false,
            "candidate_count": MATERIAL_LOOK_TITLES.len(),
            "all_candidates_valid": !options.missing_preview,
            "full_ready_status": options.full_ready_status,
            "full_ready_blocker_codes": blockers,
            "candidates": report_rows
        }))
        .expect("report json"),
    )
    .expect("report writes");
    report_path
}

fn temp_material_look_package_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "object-orchard-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn temp_foundry_project_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "object-orchard-{name}-{}-{nanos}{FOUNDRY_PROJECT_FILE_SUFFIX}",
        std::process::id()
    ))
}
