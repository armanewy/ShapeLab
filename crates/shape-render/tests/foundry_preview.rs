use std::collections::BTreeSet;

use glam::{Vec3, Vec4};
use shape_core::Aabb;
use shape_foundry::{
    CandidateLegibilityClass, FoundryClayQualityGateRecord, FoundryPreviewDisplayMode,
    SemanticClayRoleAssignment, VariationChannel, VariationScope,
};
use shape_mesh::TriangleMesh;
use shape_render::foundry::{
    FoundryChangedRoleOverlay, FoundryPreviewBatchRequest, FoundryPreviewCache,
    FoundryPreviewCacheStatus, FoundryPreviewControlValue, FoundryPreviewError, FoundryPreviewKind,
    FoundryPreviewRequest, FoundryPreviewResolution, FoundryPreviewVariationMetadata,
    classify_foundry_rendered_perceptual_report, compare_foundry_rendered_visible_delta,
    render_foundry_previews,
};
use shape_render::{OrbitCamera, RenderError, RenderSettings, RenderedImage};

#[test]
fn cache_hit_reuses_whole_model_preview() {
    let mut cache = FoundryPreviewCache::new(4);
    let request = candidate_request("candidate-a", "geom-a", 1.0);

    let first = render_foundry_previews(
        &mut cache,
        batch(
            "cache-hit",
            vec![request.clone()],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("first render should succeed");
    let second = render_foundry_previews(
        &mut cache,
        batch("cache-hit", vec![request], FoundryPreviewResolution::Px64),
    )
    .expect("second render should succeed");

    assert_eq!(
        first.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );
    assert_eq!(
        second.previews[0].cache_status,
        FoundryPreviewCacheStatus::Hit
    );
    assert_eq!(first.previews[0].image, second.previews[0].image);
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn foundry_pure_clay_preview_produces_one_display_class() {
    let mut cache = FoundryPreviewCache::new(4);
    let output = render_foundry_previews(
        &mut cache,
        batch(
            "pure-clay",
            vec![candidate_request("pure", "geom-pure", 1.0)],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("pure clay render should succeed");

    let preview = &output.previews[0];
    assert_eq!(
        preview.key.display_mode,
        FoundryPreviewDisplayMode::PureClay
    );
    assert!(preview.key.semantic_clay_assignments.is_empty());
    assert_eq!(
        unique_foreground_rgbs(&preview.image, output.render_settings.background).len(),
        1
    );
}

#[test]
fn foundry_semantic_clay_preview_maps_neutral_gray_assignments() {
    let mut request = candidate_request("semantic", "geom-semantic", 1.0);
    request.semantic_clay_assignments = semantic_assignments();
    request.use_novice_default_display_mode();
    let mut cache = FoundryPreviewCache::new(4);

    let output = render_foundry_previews(
        &mut cache,
        batch(
            "semantic-clay",
            vec![request],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("semantic clay render should succeed");

    let preview = &output.previews[0];
    assert_eq!(
        preview.key.display_mode,
        FoundryPreviewDisplayMode::SemanticClay
    );
    assert!(preview.key.semantic_clay_assignments.len() >= 2);
    let colors = unique_foreground_rgbs(&preview.image, output.render_settings.background);
    assert!(
        colors.len() >= 2,
        "semantic clay should show multiple grays"
    );
    assert!(
        colors
            .iter()
            .all(|[red, green, blue]| red == green && green == blue),
        "semantic clay must use neutral gray values only: {colors:?}"
    );
}

#[test]
fn foundry_semantic_clay_cache_key_is_separate_from_pure_clay() {
    let mut pure = candidate_request("preview", "geom-shared", 1.0);
    let mut semantic = pure.clone();
    semantic.semantic_clay_assignments = semantic_assignments();
    semantic.use_novice_default_display_mode();
    pure.display_mode = FoundryPreviewDisplayMode::PureClay;
    let mut cache = FoundryPreviewCache::new(4);

    let output = render_foundry_previews(
        &mut cache,
        batch(
            "display-key",
            vec![pure, semantic],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("display key render should succeed");

    assert_ne!(output.previews[0].key, output.previews[1].key);
    assert_eq!(cache.stats().len, 2);
}

#[test]
fn foundry_diagnostic_part_color_is_not_a_novice_default() {
    assert_eq!(
        FoundryPreviewDisplayMode::novice_default(&[]),
        FoundryPreviewDisplayMode::PureClay
    );
    assert_eq!(
        FoundryPreviewDisplayMode::novice_default(&semantic_assignments()),
        FoundryPreviewDisplayMode::SemanticClay
    );
    assert!(!FoundryPreviewDisplayMode::DiagnosticPartColor.default_novice_safe());
}

#[test]
fn foundry_candidate_comparison_uses_same_display_mode_for_all_items() {
    let assignments = semantic_assignments();
    let items = ["a", "b"]
        .into_iter()
        .map(|id| {
            let mut item = candidate_request(id, &format!("geom-{id}"), 1.0);
            item.semantic_clay_assignments = assignments.clone();
            item.use_novice_default_display_mode();
            item
        })
        .collect::<Vec<_>>();
    let mut cache = FoundryPreviewCache::new(4);

    let output = render_foundry_previews(
        &mut cache,
        batch("same-display", items, FoundryPreviewResolution::Px64),
    )
    .expect("semantic comparison should render");

    assert!(output.previews.iter().all(|preview| {
        preview.key.display_mode == FoundryPreviewDisplayMode::SemanticClay
            && preview.key.semantic_clay_assignments.len() == assignments.len()
    }));
}

#[test]
fn foundry_semantic_clay_output_is_deterministic() {
    let mut item = candidate_request("semantic", "geom-semantic", 1.0);
    item.semantic_clay_assignments = semantic_assignments();
    item.use_novice_default_display_mode();
    let mut first_cache = FoundryPreviewCache::new(4);
    let mut second_cache = FoundryPreviewCache::new(4);

    let first = render_foundry_previews(
        &mut first_cache,
        batch(
            "semantic-deterministic",
            vec![item.clone()],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("first semantic render");
    let second = render_foundry_previews(
        &mut second_cache,
        batch(
            "semantic-deterministic",
            vec![item],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("second semantic render");

    assert_eq!(first.previews[0].image, second.previews[0].image);
}

#[test]
fn foundry_pure_clay_and_semantic_clay_quality_gates_are_separate() {
    let record = FoundryClayQualityGateRecord {
        pure_clay_pass: false,
        semantic_clay_readability_pass: true,
        display_mode_used: FoundryPreviewDisplayMode::SemanticClay,
    };

    assert!(!record.both_pass());
    assert!(!record.pure_clay_pass);
    assert!(record.semantic_clay_readability_pass);
}

#[test]
fn duplicate_batch_misses_render_once_and_preserve_output_slots() {
    let mut cache = FoundryPreviewCache::new(4);
    let first = candidate_request("candidate-a", "geom-shared", 1.0);
    let second = candidate_request("candidate-b", "geom-shared", 1.0);

    let output = render_foundry_previews(
        &mut cache,
        batch(
            "coalesced-miss",
            vec![first, second],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("duplicate batch misses should render");

    assert_eq!(
        preview_ids(&output.previews),
        vec!["candidate-a", "candidate-b"]
    );
    assert_eq!(output.previews[0].key, output.previews[1].key);
    assert_eq!(output.previews[0].image, output.previews[1].image);
    assert!(
        output
            .previews
            .iter()
            .all(|preview| preview.cache_status == FoundryPreviewCacheStatus::Miss)
    );
    let stats = cache.stats();
    assert_eq!(stats.len, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.coalesced_misses, 1);
    assert_eq!(stats.hits, 0);
}

#[test]
fn eviction_uses_lru_order() {
    let mut cache = FoundryPreviewCache::new(2);
    let first = candidate_request("candidate-a", "geom-a", 1.0);
    let second = candidate_request("candidate-b", "geom-b", 1.2);
    let third = candidate_request("candidate-c", "geom-c", 1.4);

    let initial = render_foundry_previews(
        &mut cache,
        batch(
            "eviction",
            vec![first.clone(), second.clone()],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("initial render should succeed");
    let camera = initial.camera.clone();
    let first_key = initial.previews[0].key.clone();
    let second_key = initial.previews[1].key.clone();

    let mut touch_batch = batch("eviction", vec![first], FoundryPreviewResolution::Px64);
    touch_batch.camera = Some(camera.clone());
    render_foundry_previews(&mut cache, touch_batch)
        .expect("touching first preview should succeed");
    let mut insert_batch = batch("eviction", vec![third], FoundryPreviewResolution::Px64);
    insert_batch.camera = Some(camera);
    let inserted =
        render_foundry_previews(&mut cache, insert_batch).expect("third render should succeed");
    let third_key = inserted.previews[0].key.clone();

    assert!(cache.contains_key(&first_key));
    assert!(!cache.contains_key(&second_key));
    assert!(cache.contains_key(&third_key));
    assert_eq!(cache.stats().len, 2);
    assert_eq!(cache.stats().evictions, 1);
}

#[test]
fn deterministic_order_survives_parallel_rendering() {
    let items = vec![
        candidate_request("candidate-z", "geom-z", 1.5),
        candidate_request("candidate-a", "geom-a", 0.8),
        candidate_request("candidate-m", "geom-m", 1.1),
    ];
    let mut first_cache = FoundryPreviewCache::new(8);
    let mut second_cache = FoundryPreviewCache::new(8);
    let mut first_batch = batch(
        "deterministic",
        items.clone(),
        FoundryPreviewResolution::Px96,
    );
    first_batch.max_parallel_jobs = 2;
    let mut second_batch = batch("deterministic", items, FoundryPreviewResolution::Px96);
    second_batch.max_parallel_jobs = 2;

    let first = render_foundry_previews(&mut first_cache, first_batch)
        .expect("first parallel render should succeed");
    let second = render_foundry_previews(&mut second_cache, second_batch)
        .expect("second parallel render should succeed");

    assert_eq!(
        preview_ids(&first.previews),
        vec!["candidate-z", "candidate-a", "candidate-m"]
    );
    assert_eq!(preview_ids(&first.previews), preview_ids(&second.previews));
    assert_eq!(
        first
            .previews
            .iter()
            .map(|preview| &preview.image)
            .collect::<Vec<_>>(),
        second
            .previews
            .iter()
            .map(|preview| &preview.image)
            .collect::<Vec<_>>()
    );
}

#[test]
fn filmstrip_samples_keep_input_order() {
    let mut items = Vec::new();
    for (index, value) in [0.0_f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
        let mut item = FoundryPreviewRequest::new(
            format!("width-preview-{index}"),
            FoundryPreviewKind::SliderFilmstrip {
                control_id: "width".to_owned(),
                sample_index: index as u32,
            },
            format!("geom-width-{index}"),
            panel_mesh(1.0 + value, 1.0, Vec3::ZERO),
        );
        item.sampled_control_state.insert(
            "width".to_owned(),
            FoundryPreviewControlValue::Scalar(value),
        );
        items.push(item);
    }
    let mut cache = FoundryPreviewCache::new(8);

    let output = render_foundry_previews(
        &mut cache,
        batch("filmstrip", items, FoundryPreviewResolution::Px64),
    )
    .expect("filmstrip render should succeed");

    let sample_indices = output
        .previews
        .iter()
        .map(|preview| match preview.kind {
            FoundryPreviewKind::SliderFilmstrip { sample_index, .. } => sample_index,
            _ => panic!("unexpected preview kind"),
        })
        .collect::<Vec<_>>();
    assert_eq!(sample_indices, vec![0, 1, 2, 3]);
}

#[test]
fn provider_gallery_keeps_option_order() {
    let providers = ["body-light", "body-heavy", "body-armored"];
    let items = providers
        .iter()
        .enumerate()
        .map(|(index, provider_id)| {
            let mut item = FoundryPreviewRequest::new(
                format!("provider-{index}"),
                FoundryPreviewKind::ProviderGallery {
                    role: "body".to_owned(),
                    provider_id: (*provider_id).to_owned(),
                    option_index: index as u32,
                },
                format!("geom-provider-{index}"),
                panel_mesh(1.0 + index as f32 * 0.2, 1.0, Vec3::ZERO),
            );
            item.provider_choices
                .insert("body".to_owned(), (*provider_id).to_owned());
            item
        })
        .collect::<Vec<_>>();
    let mut cache = FoundryPreviewCache::new(8);

    let output = render_foundry_previews(
        &mut cache,
        batch("gallery", items, FoundryPreviewResolution::Px96),
    )
    .expect("gallery render should succeed");

    let rendered_providers = output
        .previews
        .iter()
        .map(|preview| match &preview.kind {
            FoundryPreviewKind::ProviderGallery {
                provider_id,
                option_index,
                ..
            } => (*option_index, provider_id.as_str()),
            _ => panic!("unexpected preview kind"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered_providers,
        vec![(0, "body-light"), (1, "body-heavy"), (2, "body-armored")]
    );
}

#[test]
fn comparison_camera_fits_whole_model_bounds() {
    let items = vec![
        candidate_request("compact", "geom-compact", 0.7),
        candidate_request("wide", "geom-wide", 2.4),
    ];
    let mut cache = FoundryPreviewCache::new(4);

    let output = render_foundry_previews(
        &mut cache,
        batch("bounds", items, FoundryPreviewResolution::Px128),
    )
    .expect("bounds render should succeed");

    for preview in &output.previews {
        assert!(
            output
                .comparison_bounds
                .contains_aabb(&preview.whole_model_bounds)
        );
        assert_eq!(preview.camera, output.camera);
        assert_eq!(preview.image.width, 128);
        assert_eq!(preview.image.height, 128);
        assert_has_foreground(&preview.image, preview.render_settings.background);
    }
    assert_camera_contains_bounds(&output.camera, output.comparison_bounds);
}

#[test]
fn explicit_camera_is_refit_to_comparison_bounds() {
    let items = vec![
        candidate_request("left", "geom-left", 1.0),
        FoundryPreviewRequest::new(
            "far-right",
            FoundryPreviewKind::CandidateCard {
                candidate_id: "far-right".to_owned(),
            },
            "geom-far-right",
            panel_mesh(0.8, 1.0, Vec3::new(6.0, 0.0, 0.0)),
        ),
    ];
    let mut request = batch("explicit-camera", items, FoundryPreviewResolution::Px128);
    request.camera = Some(OrbitCamera {
        target: Vec3::new(1_000.0, 1_000.0, 1_000.0),
        yaw_degrees: 25.0,
        pitch_degrees: 15.0,
        distance: 0.05,
        vertical_fov_degrees: 5.0,
    });
    let mut cache = FoundryPreviewCache::new(4);

    let output = render_foundry_previews(&mut cache, request)
        .expect("explicit camera should be adjusted to contain the model");

    assert_camera_contains_bounds(&output.camera, output.comparison_bounds);
    assert!((output.camera.target - output.comparison_bounds.center()).length() < 1.0e-4);
    assert!(output.camera.distance > 0.05);
    for preview in &output.previews {
        assert_has_foreground(&preview.image, preview.render_settings.background);
    }
}

#[test]
fn invalid_render_settings_do_not_return_cache_hits() {
    let mut cache = FoundryPreviewCache::new(4);
    let request = batch(
        "invalid-settings",
        vec![candidate_request("candidate-a", "geom-a", 1.0)],
        FoundryPreviewResolution::Px64,
    );
    render_foundry_previews(&mut cache, request.clone())
        .expect("initial valid preview should populate cache");

    let mut invalid = request.clone();
    invalid.render_settings.ambient = f32::NAN;
    let error =
        render_foundry_previews(&mut cache, invalid).expect_err("invalid settings must fail");

    assert!(matches!(
        error,
        FoundryPreviewError::Render {
            source: RenderError::InvalidSettings("ambient must be finite"),
            ..
        }
    ));
    assert_eq!(cache.stats().hits, 0);

    let mut valid_light = request.clone();
    valid_light.render_settings.light_direction = Vec3::X;
    render_foundry_previews(&mut cache, valid_light)
        .expect("valid light-direction preview should populate cache");

    let mut invalid_light = request;
    invalid_light.render_settings.light_direction = Vec3::new(1.0e-8, 0.0, 0.0);
    let error = render_foundry_previews(&mut cache, invalid_light)
        .expect_err("near-zero light direction must fail before cache lookup");

    assert!(matches!(
        error,
        FoundryPreviewError::Render {
            source: RenderError::InvalidSettings("light direction must be finite and non-zero"),
            ..
        }
    ));
}

#[test]
fn invalid_normals_use_renderer_fallback_instead_of_preflight_rejection() {
    let mut item = candidate_request("invalid-normal", "geom-invalid-normal", 1.0);
    item.mesh.normals = vec![[f32::NAN, 0.0, 0.0]; item.mesh.positions.len()];
    let mut cache = FoundryPreviewCache::new(2);

    let output = render_foundry_previews(
        &mut cache,
        batch("invalid-normal", vec![item], FoundryPreviewResolution::Px64),
    )
    .expect("renderer should fall back to face normals");

    assert_has_foreground(
        &output.previews[0].image,
        output.previews[0].render_settings.background,
    );
}

#[test]
fn cache_key_covers_all_foundry_preview_inputs() {
    let base_item = candidate_request("candidate-a", "geom-a", 1.0);
    let mut cache = FoundryPreviewCache::new(16);
    let base = render_foundry_previews(
        &mut cache,
        batch(
            "key-coverage",
            vec![base_item.clone()],
            FoundryPreviewResolution::Px64,
        ),
    )
    .expect("base render should succeed");
    let base_key = base.previews[0].key.clone();

    let mut changed_state_item = base_item.clone();
    changed_state_item.sampled_control_state.insert(
        "height".to_owned(),
        FoundryPreviewControlValue::Scalar(1.25),
    );
    let changed_state = render_single_variant(&mut cache, changed_state_item, None, None);
    assert_ne!(base_key, changed_state.previews[0].key);
    assert_eq!(
        changed_state.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let changed_fingerprint = render_single_variant(
        &mut cache,
        candidate_request("candidate-a", "geom-b", 1.0),
        Some(base.camera.clone()),
        None,
    );
    assert_ne!(base_key, changed_fingerprint.previews[0].key);
    assert_eq!(
        changed_fingerprint.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_provider_item = base_item.clone();
    changed_provider_item
        .provider_choices
        .insert("body".to_owned(), "armored".to_owned());
    let changed_provider = render_single_variant(
        &mut cache,
        changed_provider_item,
        Some(base.camera.clone()),
        None,
    );
    assert_ne!(base_key, changed_provider.previews[0].key);
    assert_eq!(
        changed_provider.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_camera = base.camera.clone();
    changed_camera.yaw_degrees += 30.0;
    let changed_camera =
        render_single_variant(&mut cache, base_item.clone(), Some(changed_camera), None);
    assert_ne!(base_key, changed_camera.previews[0].key);
    assert_eq!(
        changed_camera.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_settings = render_settings();
    changed_settings.ambient = 0.25;
    let changed_settings = render_single_variant(
        &mut cache,
        base_item.clone(),
        Some(base.camera.clone()),
        Some(changed_settings),
    );
    assert_ne!(base_key, changed_settings.previews[0].key);
    assert_eq!(
        changed_settings.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_light = render_settings();
    changed_light.light_direction = Vec3::new(1.0, -1.0, -1.0).normalize();
    let changed_light = render_single_variant(
        &mut cache,
        base_item.clone(),
        Some(base.camera.clone()),
        Some(changed_light),
    );
    assert_ne!(base_key, changed_light.previews[0].key);
    assert_eq!(
        changed_light.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_background = render_settings();
    changed_background.background = [3, 5, 7, 255];
    let changed_background = render_single_variant(
        &mut cache,
        base_item.clone(),
        Some(base.camera.clone()),
        Some(changed_background),
    );
    assert_ne!(base_key, changed_background.previews[0].key);
    assert_eq!(
        changed_background.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_wireframe = render_settings();
    changed_wireframe.wireframe = true;
    let changed_wireframe = render_single_variant(
        &mut cache,
        base_item.clone(),
        Some(base.camera.clone()),
        Some(changed_wireframe),
    );
    assert_ne!(base_key, changed_wireframe.previews[0].key);
    assert_eq!(
        changed_wireframe.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );

    let mut changed_resolution = batch(
        "key-coverage",
        vec![base_item],
        FoundryPreviewResolution::Px96,
    );
    changed_resolution.camera = Some(base.camera);
    let changed_resolution = render_foundry_previews(&mut cache, changed_resolution)
        .expect("resolution variant should render");
    assert_ne!(base_key, changed_resolution.previews[0].key);
    assert_eq!(
        changed_resolution.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );
}

#[test]
fn changed_role_metadata_is_not_cached_with_image() {
    let mut cache = FoundryPreviewCache::new(2);
    let mut first = candidate_request("role-overlay", "geom-role", 1.0);
    first.kind = FoundryPreviewKind::ChangedRoleOverlay {
        role: "turret".to_owned(),
    };
    first.changed_role_overlays = vec![FoundryChangedRoleOverlay {
        role: "turret".to_owned(),
        previous_provider: Some("turret-light".to_owned()),
        current_provider: Some("turret-heavy".to_owned()),
        changed_controls: vec!["turret-provider".to_owned()],
    }];

    let mut second = first.clone();
    second.changed_role_overlays = vec![FoundryChangedRoleOverlay {
        role: "turret".to_owned(),
        previous_provider: Some("turret-heavy".to_owned()),
        current_provider: Some("turret-armored".to_owned()),
        changed_controls: vec!["turret-provider".to_owned(), "armor-toggle".to_owned()],
    }];

    let miss = render_foundry_previews(
        &mut cache,
        batch("overlay", vec![first], FoundryPreviewResolution::Px64),
    )
    .expect("overlay miss render should succeed");
    let hit = render_foundry_previews(
        &mut cache,
        batch("overlay", vec![second], FoundryPreviewResolution::Px64),
    )
    .expect("overlay hit render should succeed");

    assert_eq!(
        miss.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );
    assert_eq!(hit.previews[0].cache_status, FoundryPreviewCacheStatus::Hit);
    assert_eq!(
        hit.previews[0].changed_role_overlays[0]
            .current_provider
            .as_deref(),
        Some("turret-armored")
    );
    assert_eq!(
        hit.previews[0].changed_role_overlays[0].changed_controls,
        vec!["turret-provider".to_owned(), "armor-toggle".to_owned()]
    );
    assert_ne!(miss.previews[0].image.rgba8, hit.previews[0].image.rgba8);
}

#[test]
fn foundry_variation_metadata_is_preserved_without_changing_cache_key() {
    let mut cache = FoundryPreviewCache::new(2);
    let mut first = candidate_request("variation", "geom-variation", 1.0);
    first.variation_metadata = FoundryPreviewVariationMetadata {
        scope: VariationScope::WholeAsset,
        channels: vec![VariationChannel::CompleteLook],
        selected_part_group: None,
        material_slot_id: None,
        legibility_class: Some(CandidateLegibilityClass::Clear),
    };

    let mut second = first.clone();
    second.variation_metadata = FoundryPreviewVariationMetadata {
        scope: VariationScope::SemanticPartGroup {
            group_id: "body".to_owned(),
            display_name: "Body".to_owned(),
        },
        channels: vec![VariationChannel::Shape],
        selected_part_group: Some("body".to_owned()),
        material_slot_id: None,
        legibility_class: Some(CandidateLegibilityClass::Strong),
    };

    let miss = render_foundry_previews(
        &mut cache,
        batch("variation", vec![first], FoundryPreviewResolution::Px64),
    )
    .expect("variation metadata miss render should succeed");
    let hit = render_foundry_previews(
        &mut cache,
        batch("variation", vec![second], FoundryPreviewResolution::Px64),
    )
    .expect("variation metadata hit render should succeed");

    assert_eq!(
        miss.previews[0].cache_status,
        FoundryPreviewCacheStatus::Miss
    );
    assert_eq!(hit.previews[0].cache_status, FoundryPreviewCacheStatus::Hit);
    assert_eq!(miss.previews[0].key, hit.previews[0].key);
    assert_eq!(
        hit.previews[0]
            .variation_metadata
            .selected_part_group
            .as_deref(),
        Some("body")
    );
    assert_eq!(
        hit.previews[0].variation_metadata.legibility_class,
        Some(CandidateLegibilityClass::Strong)
    );
}

#[test]
fn foundry_rendered_visible_delta_ignores_shared_background_and_clamps_scores() {
    let background = [7, 9, 11, 255];
    let parent = RenderedImage {
        width: 2,
        height: 1,
        rgba8: vec![
            7, 9, 11, 255, // shared background
            200, 200, 200, 255,
        ],
    };
    let candidate = RenderedImage {
        width: 2,
        height: 1,
        rgba8: vec![
            7, 9, 11, 255, // ignored
            20, 20, 20, 255,
        ],
    };

    let delta = compare_foundry_rendered_visible_delta(&parent, &candidate, background);

    assert!(delta.available());
    assert!(delta.mean_pixel_delta > 0.70);
    assert_eq!(delta.changed_pixel_ratio, 1.0);
    assert_eq!(delta.silhouette_delta, 0.0);
    assert!((0.0..=1.0).contains(&delta.score));
}

#[test]
fn foundry_rendered_visible_delta_reports_unavailable_for_hidden_or_mismatched_previews() {
    let background = [7, 9, 11, 255];
    let hidden = RenderedImage {
        width: 1,
        height: 1,
        rgba8: vec![7, 9, 11, 255],
    };
    let mismatch = RenderedImage {
        width: 2,
        height: 1,
        rgba8: vec![7, 9, 11, 255, 7, 9, 11, 255],
    };

    let hidden_delta = compare_foundry_rendered_visible_delta(&hidden, &hidden, background);
    let mismatch_delta = compare_foundry_rendered_visible_delta(&hidden, &mismatch, background);

    assert!(!hidden_delta.available());
    assert_eq!(hidden_delta.score, 0.0);
    assert!(!mismatch_delta.available());
    assert_eq!(mismatch_delta.score, 0.0);
}

#[test]
fn foundry_rendered_perceptual_report_requires_multiple_views() {
    let background = [7, 9, 11, 255];
    let parent = RenderedImage {
        width: 1,
        height: 2,
        rgba8: vec![7, 9, 11, 255, 200, 200, 200, 255],
    };
    let candidate = RenderedImage {
        width: 1,
        height: 2,
        rgba8: vec![200, 200, 200, 255, 7, 9, 11, 255],
    };

    let single_view = classify_foundry_rendered_perceptual_report(
        "single-view",
        &[(&parent, &candidate)],
        background,
    );
    let multi_view = classify_foundry_rendered_perceptual_report(
        "multi-view",
        &[(&parent, &candidate), (&parent, &candidate)],
        background,
    );

    assert_eq!(
        single_view.legibility_class,
        CandidateLegibilityClass::Unsupported
    );
    assert!(matches!(
        multi_view.legibility_class,
        CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
    ));
}

fn batch(
    comparison_set_id: &str,
    items: Vec<FoundryPreviewRequest>,
    resolution: FoundryPreviewResolution,
) -> FoundryPreviewBatchRequest {
    let mut request = FoundryPreviewBatchRequest::new(comparison_set_id, items, resolution);
    request.render_settings = render_settings();
    request.max_parallel_jobs = 3;
    request
}

fn candidate_request(preview_id: &str, fingerprint: &str, width: f32) -> FoundryPreviewRequest {
    let mut item = FoundryPreviewRequest::new(
        preview_id,
        FoundryPreviewKind::CandidateCard {
            candidate_id: preview_id.to_owned(),
        },
        fingerprint,
        panel_mesh(width, 1.0, Vec3::ZERO),
    );
    item.sampled_control_state.insert(
        "width".to_owned(),
        FoundryPreviewControlValue::Scalar(width),
    );
    item
}

fn panel_mesh(width: f32, height: f32, center: Vec3) -> TriangleMesh {
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let positions = vec![
        [center.x - half_width, center.y - half_height, center.z],
        [center.x + half_width, center.y - half_height, center.z],
        [center.x + half_width, center.y + half_height, center.z],
        [center.x - half_width, center.y + half_height, center.z],
    ];
    TriangleMesh {
        positions,
        normals: vec![[0.0, 0.0, 1.0]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds: Aabb {
            min: Vec3::new(center.x - half_width, center.y - half_height, center.z),
            max: Vec3::new(center.x + half_width, center.y + half_height, center.z),
        },
    }
}

fn render_settings() -> RenderSettings {
    RenderSettings {
        width: 512,
        height: 512,
        background: [7, 9, 11, 255],
        ambient: 0.65,
        light_direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
        wireframe: false,
    }
}

fn preview_ids(previews: &[shape_render::foundry::FoundryRenderedPreview]) -> Vec<&str> {
    previews
        .iter()
        .map(|preview| preview.preview_id.as_str())
        .collect()
}

fn semantic_assignments() -> Vec<SemanticClayRoleAssignment> {
    vec![
        SemanticClayRoleAssignment::new("body", "Primary Mass", 0.72, 10, true),
        SemanticClayRoleAssignment::new("panels", "Secondary Panels", 0.58, 20, true),
        SemanticClayRoleAssignment::new("vents", "Recesses / Vents", 0.34, 30, true),
    ]
}

fn unique_foreground_rgbs(
    image: &shape_render::RenderedImage,
    background: [u8; 4],
) -> BTreeSet<[u8; 3]> {
    image
        .rgba8
        .chunks_exact(4)
        .filter(|pixel| *pixel != background)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect()
}

fn render_single_variant(
    cache: &mut FoundryPreviewCache,
    item: FoundryPreviewRequest,
    camera: Option<OrbitCamera>,
    settings: Option<RenderSettings>,
) -> shape_render::foundry::FoundryPreviewBatchOutput {
    let mut request = batch("key-coverage", vec![item], FoundryPreviewResolution::Px64);
    request.camera = camera;
    if let Some(settings) = settings {
        request.render_settings = settings;
    }
    render_foundry_previews(cache, request).expect("variant should render")
}

fn assert_has_foreground(image: &shape_render::RenderedImage, background: [u8; 4]) {
    assert!(
        image.rgba8.chunks_exact(4).any(|pixel| pixel != background),
        "preview should contain foreground pixels"
    );
}

fn assert_camera_contains_bounds(camera: &shape_render::OrbitCamera, bounds: Aabb) {
    let view_projection = camera.view_projection_matrix(1.0);
    for corner in bounds_corners(bounds) {
        let clip = view_projection * Vec4::new(corner.x, corner.y, corner.z, 1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() <= 1.0, "x outside view: {ndc:?}");
        assert!(ndc.y.abs() <= 1.0, "y outside view: {ndc:?}");
        assert!(ndc.z >= -1.0 && ndc.z <= 1.0, "z outside view: {ndc:?}");
    }
}

fn bounds_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}
