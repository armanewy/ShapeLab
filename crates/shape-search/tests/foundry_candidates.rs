use shape_asset::GeometrySource;
use shape_family::StyleKit;
use shape_family_compile::StyleImplementation;
use shape_foundry::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL,
    CandidateLegibilityClass, CandidateStrategy, CatalogContentRef, ClosedInterval, ControlKind,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FeasibleControlDomain, FoundryCandidateId, FoundryCommand, FoundryLock,
    FoundryLockMode, FoundryLockTarget, FoundryPreferenceEvent, FoundryPreferenceLog,
    FoundryPreferenceScope, ProviderOption, SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON,
    VariationChannel, VariationIntent, WholeModelPreviewRef, catalog_content_fingerprint_from_json,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, headless_fixture_catalogs, roman_bridge, scifi_crate, showcase_gear,
    stylized_lamp,
};
use shape_search::foundry::{
    FOUNDRY_MAX_PROPOSAL_COUNT, FOUNDRY_MAX_RESULT_COUNT, FOUNDRY_MIN_PROPOSAL_COUNT,
    FoundryCandidateFailureReason, FoundryCandidateFallbackAction, FoundryCandidateMinimumResult,
    FoundryCandidateMode, FoundryCandidateRejectionReason, FoundryCandidateRequest,
    generate_foundry_candidate_plans, generate_foundry_control_endpoint_visibility_report,
};
use std::collections::BTreeSet;

fn request(seed: u64, mode: FoundryCandidateMode) -> FoundryCandidateRequest {
    let variation_intent = if mode == FoundryCandidateMode::Detail {
        VariationIntent::whole_asset_detail()
    } else {
        VariationIntent::default()
    };
    FoundryCandidateRequest {
        seed,
        proposal_count: 72,
        result_count: 6,
        mode,
        strategy_id: None,
        preference_profile: None,
        variation_intent,
    }
}

fn candidate_label_is_product_safe(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    let forbidden_phrases = [
        "provider",
        "provider id",
        "scalar path",
        "semantic id",
        "operation id",
        "compiler",
        "decompiler",
        "fragment",
        "remap",
        "conformance",
        "socket",
        "port",
        "raw recipe",
    ]
    .iter()
    .any(|term| contains_forbidden_label_term(&lower, term));
    !forbidden_phrases && !label.contains('#')
}

fn contains_forbidden_label_term(label: &str, term: &str) -> bool {
    label
        .split(|character: char| !character.is_ascii_alphanumeric())
        .collect::<Vec<_>>()
        .windows(term.split_whitespace().count().max(1))
        .any(|window| window.join(" ") == term)
}

fn assert_endpoint_controls_clear(
    report: &shape_search::foundry::FoundryControlEndpointVisibilityReport,
    major_controls: &[&str],
    subtle_allowed_controls: &[&str],
) {
    let rows = report
        .controls
        .iter()
        .map(|row| (row.control_id.as_str(), row.legibility_class))
        .collect::<std::collections::BTreeMap<_, _>>();
    for control_id in major_controls {
        assert!(
            matches!(
                rows.get(control_id),
                Some(CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong)
            ),
            "{} {control_id} should be at least Clear: {:?}",
            report.profile_id,
            report
                .controls
                .iter()
                .find(|row| row.control_id == *control_id)
        );
    }
    for control_id in subtle_allowed_controls {
        assert!(
            matches!(
                rows.get(control_id),
                Some(
                    CandidateLegibilityClass::SubtleButExplainable
                        | CandidateLegibilityClass::Clear
                        | CandidateLegibilityClass::Strong
                )
            ),
            "{} {control_id} should be visible or explicitly subtle: {:?}",
            report.profile_id,
            report
                .controls
                .iter()
                .find(|row| row.control_id == *control_id)
        );
    }
}

fn top_failure_reason(
    output: &shape_search::foundry::FoundryCandidateOutput,
) -> Option<FoundryCandidateFailureReason> {
    output
        .reliability_report
        .top_reasons
        .first()
        .map(|row| row.reason)
}

#[test]
fn same_seed_is_deterministic() {
    let fixture = scifi_crate::fixture_catalog();
    let first = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(7, FoundryCandidateMode::Explore),
    )
    .expect("candidates should generate");
    let second = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(7, FoundryCandidateMode::Explore),
    )
    .expect("candidates should generate");

    assert_eq!(first, second);
    assert!(!first.candidates.is_empty());
}

#[test]
fn release_candidate_budget_rejects_unbounded_proposals_and_caps_results() {
    let fixture = scifi_crate::fixture_catalog();
    let mut too_few = request(7, FoundryCandidateMode::Explore);
    too_few.proposal_count = FOUNDRY_MIN_PROPOSAL_COUNT - 1;
    let error = generate_foundry_candidate_plans(&fixture.document, &fixture, &too_few)
        .expect_err("proposal budget below release minimum should be rejected");
    assert!(error.to_string().contains("between 8 and 72"));

    let mut too_many = request(7, FoundryCandidateMode::Explore);
    too_many.proposal_count = FOUNDRY_MAX_PROPOSAL_COUNT + 1;
    let error = generate_foundry_candidate_plans(&fixture.document, &fixture, &too_many)
        .expect_err("proposal budget above release maximum should be rejected");
    assert!(error.to_string().contains("between 8 and 72"));

    let mut oversized_results = request(7, FoundryCandidateMode::Explore);
    oversized_results.proposal_count = FOUNDRY_MIN_PROPOSAL_COUNT;
    oversized_results.result_count = FOUNDRY_MAX_RESULT_COUNT * 10;
    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &oversized_results)
        .expect("oversized result requests should be capped after validation");

    assert_eq!(
        output.diagnostics.requested_candidates,
        FOUNDRY_MAX_RESULT_COUNT * 10
    );
    assert!(output.candidates.len() <= FOUNDRY_MAX_RESULT_COUNT);
    assert!(output.scoring_report.representatives.len() <= FOUNDRY_MAX_RESULT_COUNT);
}

#[test]
fn foundry_surface_variation_is_unavailable_without_surface_payloads() {
    let fixture = scifi_crate::fixture_catalog();
    let mut search_request = request(21, FoundryCandidateMode::Explore);
    search_request.variation_intent = VariationIntent::whole_asset_surface();

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("unsupported variation mode should return diagnostics");

    assert!(output.candidates.is_empty());
    assert_eq!(output.diagnostics.returned_candidates, 0);
    assert!(
        output
            .diagnostics
            .rejections
            .get(&FoundryCandidateRejectionReason::UnsupportedChannel)
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        output
            .preference_report
            .ignored_reason
            .as_deref()
            .is_some_and(|reason| reason == SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON)
    );
}

#[test]
fn foundry_shape_variation_metadata_excludes_surface_claims() {
    let fixture = scifi_crate::fixture_catalog();
    let mut search_request = request(22, FoundryCandidateMode::Explore);
    search_request.variation_intent = VariationIntent::whole_asset_shape();

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("shape candidates should generate");

    assert!(!output.candidates.is_empty());
    for candidate in &output.candidates {
        let metadata = &candidate.variation_metadata;
        assert_eq!(metadata.intent.channels, vec![VariationChannel::Shape]);
        assert!(metadata.changed_material_slots.is_empty());
        assert_eq!(metadata.visible_delta.surface_delta_score, 0.0);
        assert!(metadata.visible_delta.shape_delta_score > 0.0);
        assert!(candidate_label_is_product_safe(&candidate.label));
    }
}

#[test]
fn foundry_focus_part_variation_changes_only_selected_part_group_controls() {
    let fixture = scifi_crate::fixture_catalog();
    let mut search_request = request(23, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("body", "Body");

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("focus part candidates should generate");

    assert!(!output.candidates.is_empty());
    assert!(output.candidates.len() <= 6);
    for candidate in &output.candidates {
        let metadata = &candidate.variation_metadata;
        assert_eq!(metadata.intent.scope.semantic_part_group_id(), Some("body"));
        assert!(metadata.visible_delta.selected_part_delta_score >= 0.065);
        assert!(
            metadata
                .changed_part_groups
                .iter()
                .all(|group| group.group_id == "body")
        );
        assert!(candidate_label_is_product_safe(&candidate.label));
    }
}

#[test]
fn foundry_focused_vents_zero_candidates_suggest_detail_mode() {
    let fixture = scifi_crate::fixture_catalog();
    let mut search_request = request(24, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("vents", "Vents");

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("focused vents request should return reliability diagnostics");

    assert!(output.candidates.is_empty());
    assert_eq!(
        output.reliability_report.minimum_result,
        FoundryCandidateMinimumResult::NoFocusedCandidates
    );
    assert_eq!(
        top_failure_reason(&output),
        Some(FoundryCandidateFailureReason::ControlTooSubtle)
    );
    assert_eq!(
        output.reliability_report.suggested_action,
        Some(FoundryCandidateFallbackAction::UseDetailMode)
    );
    let vents = output
        .reliability_report
        .focused_part_capabilities
        .iter()
        .find(|row| row.group_id == "vents")
        .expect("vents capability row should be reported");
    assert!(!vents.can_generate_shape_ideas);
    assert_eq!(vents.likely_candidate_count, 0);
    assert!(
        vents
            .blocked_reasons
            .contains(&FoundryCandidateFailureReason::ControlTooSubtle)
    );
}

#[test]
fn foundry_focused_part_locked_out_suggests_unlock_controls() {
    let mut fixture = scifi_crate::fixture_catalog();
    for control_id in ["overall_proportions", "structural_heft"] {
        fixture.document.foundry_locks.push(FoundryLock {
            target: FoundryLockTarget::Control(control_id.to_owned()),
            mode: FoundryLockMode::SearchProtected,
            reason: Some("test lock".to_owned()),
        });
    }
    let mut search_request = request(25, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("body", "Body");

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("locked focused request should return reliability diagnostics");

    assert!(output.candidates.is_empty());
    assert!(output.diagnostics.locked_targets_skipped >= 2);
    assert_eq!(
        top_failure_reason(&output),
        Some(FoundryCandidateFailureReason::LockedOut)
    );
    assert_eq!(
        output.reliability_report.suggested_action,
        Some(FoundryCandidateFallbackAction::UnlockControls)
    );
}

#[test]
fn foundry_focused_part_without_bound_controls_reports_no_focused_variants() {
    let fixture = roman_bridge::fixture_catalog();
    let mut search_request = request(26, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("ramps", "Ramps");

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("unbound focused part should return reliability diagnostics");

    assert!(output.candidates.is_empty());
    assert_eq!(
        top_failure_reason(&output),
        Some(FoundryCandidateFailureReason::NoBoundControls)
    );
    assert_eq!(
        output.reliability_report.suggested_action,
        Some(FoundryCandidateFallbackAction::NoFocusedVariants)
    );
    let ramps = output
        .reliability_report
        .focused_part_capabilities
        .iter()
        .find(|row| row.group_id == "ramps")
        .expect("ramps capability row should be reported");
    assert!(!ramps.can_generate_shape_ideas);
    assert_eq!(ramps.likely_candidate_count, 0);
    assert_eq!(
        ramps.suggested_action,
        Some(FoundryCandidateFallbackAction::NoFocusedVariants)
    );
}

#[test]
fn foundry_focused_detail_control_reports_control_too_subtle_for_shape() {
    let fixture = scifi_crate::fixture_catalog();
    let mut search_request = request(27, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("fasteners", "Fasteners");

    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("detail-only focused part should return reliability diagnostics");

    assert!(output.candidates.is_empty());
    assert_eq!(
        top_failure_reason(&output),
        Some(FoundryCandidateFailureReason::ControlTooSubtle)
    );
    assert_eq!(
        output.reliability_report.suggested_action,
        Some(FoundryCandidateFallbackAction::UseDetailMode)
    );
}

#[test]
fn endpoint_visibility_reports_cover_starter_templates() {
    let crate_fixture = scifi_crate::fixture_catalog();
    let crate_report = generate_foundry_control_endpoint_visibility_report(
        &crate_fixture.document,
        &crate_fixture,
    )
    .expect("crate endpoint report should generate");
    assert_endpoint_controls_clear(
        &crate_report,
        &[
            "overall_proportions",
            "structural_heft",
            "panel_complexity",
            "vent_density",
            "handle_style",
            "trim_style",
            "detail_density",
        ],
        &[],
    );

    let bridge = roman_bridge::fixture_catalog();
    let bridge_report =
        generate_foundry_control_endpoint_visibility_report(&bridge.document, &bridge)
            .expect("bridge endpoint report should generate");
    assert_endpoint_controls_clear(
        &bridge_report,
        &[
            "span_length",
            "deck_width",
            "structural_heft",
            "support_rhythm",
            "bracing_style",
            "railing",
            "edge_finish",
        ],
        &[],
    );

    let lamp = stylized_lamp::fixture_catalog();
    let lamp_report = generate_foundry_control_endpoint_visibility_report(&lamp.document, &lamp)
        .expect("lamp endpoint report should generate");
    assert_endpoint_controls_clear(
        &lamp_report,
        &[
            "overall_height",
            "base_weight",
            "stem_curvature",
            "joint_size",
            "shade_style",
            "shade_scale",
        ],
        &["edge_softness"],
    );
}

#[test]
fn expanded_builtin_profiles_generate_six_explore_whole_model_directions() {
    for fixture in headless_fixture_catalogs().into_iter().filter(|fixture| {
        !matches!(
            fixture.slug.as_str(),
            "roman-bridge" | "sci-fi-crate" | "stylized-lamp"
        )
    }) {
        let output = generate_foundry_candidate_plans(
            &fixture.document,
            &fixture,
            &FoundryCandidateRequest {
                seed: 101,
                proposal_count: 24,
                result_count: 6,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::default(),
            },
        )
        .unwrap_or_else(|error| panic!("{} candidates should generate: {error:#?}", fixture.slug));

        assert_eq!(
            output.candidates.len(),
            6,
            "{} should produce six direction candidates",
            fixture.slug
        );
        assert!(
            output
                .candidates
                .iter()
                .all(|candidate| candidate.changed_controls.len() >= 2),
            "{} candidates should combine multiple whole-model controls",
            fixture.slug
        );
        let changed_controls = output
            .candidates
            .iter()
            .flat_map(|candidate| candidate.changed_controls.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        let structural_control_ids: &[&str] = match fixture.slug.as_str() {
            "roman-bridge-hq" => &[
                "support_style",
                "bracing_style",
                "railing_style",
                "detail_density",
                "structural_heft",
            ],
            slug if showcase_gear::is_showcase_gear_slug(slug) => {
                &["silhouette", "ornament", "detail_density", "has_accessory"]
            }
            "moba-hero-clay" => &[
                "armor_mass",
                "head_face",
                "hair_headgear",
                "weapon_accessory",
                "silhouette",
            ],
            _ => &[
                "body_variant",
                "accent_style",
                "detail_density",
                "has_accessory",
            ],
        };
        let structural_controls = structural_control_ids
            .iter()
            .copied()
            .filter(|control| changed_controls.contains(control))
            .count();
        assert!(
            structural_controls >= 3,
            "{} candidates should cover at least three structural/detail/accessory controls: {changed_controls:?}",
            fixture.slug
        );
    }
}

#[test]
fn locks_and_search_protection_are_honored() {
    let mut fixture = scifi_crate::fixture_catalog();
    fixture.document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("overall_proportions".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("test lock".to_owned()),
    });
    fixture.document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("handle_style".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("test lock".to_owned()),
    });

    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(11, FoundryCandidateMode::Explore),
    )
    .expect("candidates should generate");

    for candidate in &output.candidates {
        for command in &candidate.edit.commands {
            if let FoundryCommand::SetControl { control_id, .. } = command {
                assert_ne!(control_id, "overall_proportions");
                assert_ne!(control_id, "handle_style");
            }
        }
    }
    assert!(output.diagnostics.locked_targets_skipped >= 2);
}

#[test]
fn linked_macro_axis_remains_one_control_edit() {
    let fixture = macro_axis_fixture();

    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(13, FoundryCandidateMode::Refine),
    )
    .expect("candidates should generate");

    let candidate = output
        .candidates
        .iter()
        .find(|candidate| candidate.changed_controls == vec!["macro_axis"])
        .expect("macro axis candidate should survive");
    assert_eq!(candidate.edit.commands.len(), 1);
    let change = &candidate.diagnostics.changes[0];
    assert_eq!(change.control_label, "Macro Axis");
    assert!(
        change
            .details
            .iter()
            .any(|detail| detail.subject.ends_with(".shade_scale"))
    );
    assert!(
        change
            .details
            .iter()
            .any(|detail| detail.subject.ends_with(".stem_curvature"))
    );
}

#[test]
fn provider_choice_is_generated_as_control_space_edit() {
    let fixture = provider_fixture();

    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(17, FoundryCandidateMode::Structure),
    )
    .expect("candidates should generate");

    assert!(output.candidates.iter().any(|candidate| {
        candidate.edit.commands.iter().any(|command| {
            matches!(
                command,
                FoundryCommand::SetControl {
                    control_id,
                    value: ControlValue::Provider(provider),
                } if control_id == "handle_provider" && provider == "wide_side_rail_handle"
            )
        })
    }));
}

#[test]
fn explore_does_not_return_one_control_provider_role_fallback() {
    let mut fixture = provider_fixture();
    let mut profile = profile(&fixture);
    profile.candidate_strategies.push(CandidateStrategy {
        id: "provider-role-only".to_owned(),
        label: "Provider and role only".to_owned(),
        control_ids: vec!["handle_provider".to_owned(), "has_trim".to_owned()],
    });
    replace_profile(&mut fixture, &profile);

    let mut search_request = request(18, FoundryCandidateMode::Explore);
    search_request.strategy_id = Some("provider-role-only".to_owned());
    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("search should complete");

    assert!(output.candidates.is_empty());
    assert!(
        output
            .diagnostics
            .rejections
            .get(&FoundryCandidateRejectionReason::EmptyProgram)
            .copied()
            .unwrap_or(0)
            > 0
    );
}

#[test]
fn foundry_visually_duplicate_control_plans_collapse_with_whole_asset_fallback() {
    let mut fixture = scifi_crate::fixture_catalog();
    let mut profile = profile(&fixture);
    let mut control = profile
        .controls
        .iter()
        .find(|control| control.id == "overall_proportions")
        .expect("fixture should have overall proportions control")
        .clone();
    control.id = "advisory_weathering".to_owned();
    control.label = "Advisory Weathering".to_owned();
    control.bindings.clear();
    control.visible = true;
    control.primary = true;
    profile.controls.push(control);
    profile.candidate_strategies.push(CandidateStrategy {
        id: "advisory-only".to_owned(),
        label: "Advisory only".to_owned(),
        control_ids: vec!["advisory_weathering".to_owned()],
    });
    replace_profile(&mut fixture, &profile);

    let mut search_request = request(19, FoundryCandidateMode::Detail);
    search_request.strategy_id = Some("advisory-only".to_owned());
    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("candidates should generate");

    assert!(!output.scoring_report.duplicate_groups.is_empty());
    assert_eq!(output.candidates.len(), 1);
    assert_eq!(
        output.reliability_report.minimum_result,
        FoundryCandidateMinimumResult::NoUsefulCandidates
    );
    assert!(!output.reliability_report.top_reasons.is_empty());
    assert_eq!(
        output.reliability_report.suggested_action,
        Some(FoundryCandidateFallbackAction::TryWholeAssetIdeas)
    );
}

#[test]
fn foundry_candidate_failure_report_is_deterministic() {
    let fixture = roman_bridge::fixture_catalog();
    let mut search_request = request(20, FoundryCandidateMode::Refine);
    search_request.variation_intent = VariationIntent::focus_part_shape("ramps", "Ramps");

    let first = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("first reliability report should generate");
    let second = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("second reliability report should generate");

    assert_eq!(first.reliability_report, second.reliability_report);
    assert_eq!(
        serde_json::to_string(&first.reliability_report).expect("serialize first report"),
        serde_json::to_string(&second.reliability_report).expect("serialize second report")
    );
}

#[test]
fn invalid_candidate_isolation_keeps_valid_survivors() {
    let mut fixture = scifi_crate::fixture_catalog();
    fixture
        .document
        .control_state
        .insert("has_trim".to_owned(), ControlValue::Toggle(false));
    let mut style_impl = style_impl(&fixture);
    style_impl.default_role_providers.remove("trim");
    replace_style_impl(&mut fixture, &style_impl);
    let mut profile = profile(&fixture);
    let trim_control = profile
        .controls
        .iter_mut()
        .find(|control| control.id == "has_trim")
        .expect("trim control exists");
    trim_control.primary = true;
    trim_control.visible = true;
    profile.candidate_strategies.push(CandidateStrategy {
        id: "invalid-trim-toggle".to_owned(),
        label: "Invalid trim toggle".to_owned(),
        control_ids: vec![
            "has_trim".to_owned(),
            "body_proportions".to_owned(),
            "edge_softness".to_owned(),
        ],
    });
    replace_profile(&mut fixture, &profile);

    let mut search_request = request(23, FoundryCandidateMode::Explore);
    search_request.strategy_id = Some("invalid-trim-toggle".to_owned());
    let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &search_request)
        .expect("candidates should generate");

    assert!(!output.candidates.is_empty());
    assert!(
        output
            .diagnostics
            .rejections
            .get(&FoundryCandidateRejectionReason::CompileRejected)
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(!output.scoring_report.rejected_candidates.is_empty());
}

#[test]
fn explanations_use_control_labels() {
    let fixture = scifi_crate::fixture_catalog();
    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(29, FoundryCandidateMode::Explore),
    )
    .expect("candidates should generate");

    for change in output
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.diagnostics.changes)
    {
        assert!(!change.control_label.is_empty());
        assert!(change.message.contains(&change.control_label));
        assert!(!change.details.is_empty());
    }
}

#[test]
fn six_diverse_survivors_are_returned() {
    let fixture = scifi_crate::fixture_catalog();
    let output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &request(31, FoundryCandidateMode::Explore),
    )
    .expect("candidates should generate");

    assert_eq!(output.candidates.len(), 6);
    assert_eq!(output.scoring_report.representatives.len(), 6);
    assert!(
        output
            .candidates
            .windows(2)
            .all(|pair| pair[0].id != pair[1].id)
    );
}

#[test]
fn local_preference_profile_biases_selection_without_replacing_validity_gates() {
    let fixture = scifi_crate::fixture_catalog();
    let base_request = request(37, FoundryCandidateMode::Explore);
    let base = generate_foundry_candidate_plans(&fixture.document, &fixture, &base_request)
        .expect("base candidates should generate");
    let preferred = base
        .candidates
        .iter()
        .rev()
        .find(|candidate| !candidate.changed_controls.is_empty())
        .expect("fixture should produce changed controls");
    let preferred_controls = preferred.changed_controls.clone();
    let mut profile = preference_profile_for_fixture(&fixture, &preferred_controls);
    profile.selection_strength = 0.35;
    profile.novelty_floor = 0.0;
    let mut biased_request = base_request;
    biased_request.preference_profile = Some(profile);

    let biased = generate_foundry_candidate_plans(&fixture.document, &fixture, &biased_request)
        .expect("biased candidates should generate");

    assert!(biased.preference_report.requested);
    assert!(biased.preference_report.applied);
    assert!(biased.preference_report.scope_matched);
    assert_eq!(biased.candidates.len(), base.candidates.len());
    let mut biased_ids = biased
        .candidates
        .iter()
        .map(|candidate| candidate.id.0.as_str())
        .collect::<Vec<_>>();
    biased_ids.sort();
    let mut representative_ids = biased
        .scoring_report
        .representatives
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect::<Vec<_>>();
    representative_ids.sort();
    assert_eq!(biased_ids, representative_ids);
    assert_eq!(
        biased.diagnostics.returned_candidates,
        base.candidates.len()
    );
    assert!(
        biased.candidates[0]
            .changed_controls
            .iter()
            .any(|control| preferred_controls.contains(control)),
        "first biased candidate should include one preferred control; preferred={preferred_controls:?} first={:?}",
        biased.candidates[0].changed_controls
    );
    assert!(
        biased.preference_report.selected_scores[0].score > 0.0,
        "first selected candidate should carry positive preference score"
    );
}

#[test]
fn wrong_scope_preference_profile_is_ignored() {
    let fixture = scifi_crate::fixture_catalog();
    let base_request = request(41, FoundryCandidateMode::Explore);
    let base = generate_foundry_candidate_plans(&fixture.document, &fixture, &base_request)
        .expect("base candidates should generate");
    let mut profile = preference_profile_for_fixture(&fixture, &["overall_proportions".to_owned()]);
    profile.scope = FoundryPreferenceScope::new("lamp", "lamp-profile");
    let mut scoped_request = base_request;
    scoped_request.preference_profile = Some(profile);

    let scoped = generate_foundry_candidate_plans(&fixture.document, &fixture, &scoped_request)
        .expect("scoped candidates should generate");

    assert!(scoped.preference_report.requested);
    assert!(!scoped.preference_report.applied);
    assert!(!scoped.preference_report.scope_matched);
    assert_eq!(
        scoped.preference_report.ignored_reason.as_deref(),
        Some("preference_scope_mismatch")
    );
    assert_eq!(scoped.candidates, base.candidates);
}

fn macro_axis_fixture() -> FoundryFixtureCatalog {
    let mut fixture = stylized_lamp::fixture_catalog();
    let mut profile = profile(&fixture);
    let shade = profile
        .controls
        .iter()
        .find(|control| control.id == "shade_scale")
        .expect("shade control exists")
        .clone();
    let stem = profile
        .controls
        .iter()
        .find(|control| control.id == "stem_curvature")
        .expect("stem curvature control exists")
        .clone();
    let mut macro_axis = shade.clone();
    macro_axis.id = "macro_axis".to_owned();
    macro_axis.label = "Macro Axis".to_owned();
    macro_axis.kind = ControlKind::ContinuousAxis { default: 0.75 };
    macro_axis.bindings = vec![shade.bindings[0].clone(), stem.bindings[0].clone()];
    macro_axis.domain = FeasibleControlDomain {
        continuous_intervals: vec![ClosedInterval {
            minimum: 0.4,
            maximum: 1.0,
        }],
        discrete_values: Vec::new(),
        unavailable_options: Default::default(),
        certification: DomainCertification::CertifiedContinuous,
    };
    macro_axis.topology_behavior = ControlTopologyBehavior::TopologyPreserving;
    profile.controls = vec![macro_axis];
    profile.candidate_strategies = vec![CandidateStrategy {
        id: "macro-only".to_owned(),
        label: "Macro only".to_owned(),
        control_ids: vec!["macro_axis".to_owned()],
    }];
    fixture.document.control_state.clear();
    fixture
        .document
        .control_state
        .insert("macro_axis".to_owned(), ControlValue::Scalar(0.75));
    replace_profile(&mut fixture, &profile);
    fixture
}

fn preference_profile_for_fixture(
    fixture: &FoundryFixtureCatalog,
    accepted_controls: &[String],
) -> shape_foundry::FoundryPreferenceProfile {
    let scope = FoundryPreferenceScope::new(
        "crate",
        fixture.document.customizer_profile_ref.stable_id.clone(),
    );
    let mut log = FoundryPreferenceLog::new();
    log.record(FoundryPreferenceEvent::CandidateComparison {
        scope: scope.clone(),
        mode: Some("explore".to_owned()),
        accepted_candidate_id: FoundryCandidateId("preferred".to_owned()),
        accepted_controls: accepted_controls.to_vec(),
        rejected_candidate_ids: Vec::new(),
        rejected_controls: Vec::new(),
        weight: 1.0,
    });
    log.profile_for_scope(scope)
}

fn provider_fixture() -> FoundryFixtureCatalog {
    let mut fixture = scifi_crate::fixture_catalog();
    let mut style = style(&fixture);
    let facet = style
        .family_facets
        .get_mut("crate")
        .expect("crate style facet exists");
    let mut declared = facet
        .part_prototypes
        .iter()
        .find(|prototype| prototype.id == "side_rail_handle")
        .expect("side rail handle declaration exists")
        .clone();
    declared.id = "wide_side_rail_handle".to_owned();
    declared.display_name = "Wide side rail handle".to_owned();
    facet.part_prototypes.push(declared);
    replace_style(&mut fixture, &style);

    let mut style_impl = style_impl(&fixture);
    let mut wide = style_impl
        .prototypes
        .get("side_rail_handle")
        .expect("side rail handle prototype exists")
        .clone();
    wide.id = "wide_side_rail_handle".to_owned();
    for definition in wide.recipe.definitions.values_mut() {
        if let GeometrySource::RoundedBox { half_extents, .. } = &mut definition.geometry.source {
            half_extents[0] *= 1.8;
            half_extents[2] *= 1.2;
        }
    }
    style_impl.prototypes.insert(wide.id.clone(), wide);
    replace_style_impl(&mut fixture, &style_impl);

    let mut profile = profile(&fixture);
    profile.controls.push(provider_control());
    profile.candidate_strategies.push(CandidateStrategy {
        id: "provider-only".to_owned(),
        label: "Provider only".to_owned(),
        control_ids: vec!["handle_provider".to_owned()],
    });
    fixture.document.control_state.insert(
        "handle_provider".to_owned(),
        ControlValue::Provider("side_rail_handle".to_owned()),
    );
    replace_profile(&mut fixture, &profile);
    fixture
}

fn provider_control() -> CustomizerControl {
    CustomizerControl {
        id: "handle_provider".to_owned(),
        label: "Handle Provider".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ProviderGallery {
            role: "handle".to_owned(),
            options: vec![
                ProviderOption {
                    provider_id: "side_rail_handle".to_owned(),
                    label: "Side Rail Handle".to_owned(),
                    preview: preview("side-rail-handle"),
                },
                ProviderOption {
                    provider_id: "wide_side_rail_handle".to_owned(),
                    label: "Wide Side Rail Handle".to_owned(),
                    preview: preview("wide-side-rail-handle"),
                },
            ],
        },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Provider("side_rail_handle".to_owned()),
                ControlValue::Provider("wide_side_rail_handle".to_owned()),
            ],
            unavailable_options: Default::default(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn preview(id: &str) -> WholeModelPreviewRef {
    WholeModelPreviewRef {
        preview_id: format!("preview-{id}"),
        artifact_fingerprint: None,
    }
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    let profile_id = &fixture.document.customizer_profile_ref.stable_id;
    serde_json::from_str(&fixture.entries[profile_id].canonical_json)
        .expect("profile JSON should decode")
}

fn style_impl(fixture: &FoundryFixtureCatalog) -> StyleImplementation {
    let style_impl_id = &fixture.document.style_implementation_ref.stable_id;
    serde_json::from_str(&fixture.entries[style_impl_id].canonical_json)
        .expect("style implementation JSON should decode")
}

fn style(fixture: &FoundryFixtureCatalog) -> StyleKit {
    let style_id = &fixture.document.style_content_ref.stable_id;
    serde_json::from_str(&fixture.entries[style_id].canonical_json)
        .expect("style kit JSON should decode")
}

fn replace_profile(fixture: &mut FoundryFixtureCatalog, profile: &CustomizerProfile) {
    let profile_id = fixture.document.customizer_profile_ref.stable_id.clone();
    let content_ref = replace_catalog_payload(fixture, &profile_id, profile);
    fixture.document.customizer_profile_ref = content_ref.clone();
    fixture
        .document
        .catalog_lock
        .as_mut()
        .expect("fixture has lock")
        .exact_refs
        .insert(CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE.to_owned(), content_ref);
}

fn replace_style(fixture: &mut FoundryFixtureCatalog, style: &StyleKit) {
    let style_id = fixture.document.style_content_ref.stable_id.clone();
    let content_ref = replace_catalog_payload(fixture, &style_id, style);
    fixture.document.style_content_ref = content_ref.clone();
    fixture
        .document
        .catalog_lock
        .as_mut()
        .expect("fixture has lock")
        .exact_refs
        .insert(CATALOG_LOCK_KEY_STYLE.to_owned(), content_ref);
}

fn replace_style_impl(fixture: &mut FoundryFixtureCatalog, style_impl: &StyleImplementation) {
    let style_impl_id = fixture.document.style_implementation_ref.stable_id.clone();
    let content_ref = replace_catalog_payload(fixture, &style_impl_id, style_impl);
    fixture.document.style_implementation_ref = content_ref.clone();
    fixture
        .document
        .catalog_lock
        .as_mut()
        .expect("fixture has lock")
        .exact_refs
        .insert(CATALOG_LOCK_KEY_STYLE_IMPL.to_owned(), content_ref);
}

fn replace_catalog_payload<T: serde::Serialize>(
    fixture: &mut FoundryFixtureCatalog,
    stable_id: &str,
    value: &T,
) -> CatalogContentRef {
    let canonical_json = serde_json::to_string(value).expect("catalog payload serializes");
    let entry = fixture
        .entries
        .get_mut(stable_id)
        .expect("catalog entry exists");
    entry.canonical_json = canonical_json.clone();
    entry.content_ref.fingerprint =
        catalog_content_fingerprint_from_json(stable_id, &canonical_json)
            .expect("catalog payload fingerprints");
    entry.content_ref.clone()
}
