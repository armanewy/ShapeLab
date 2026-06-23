use shape_asset::GeometrySource;
use shape_family::StyleKit;
use shape_family_compile::StyleImplementation;
use shape_foundry::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL,
    CandidateStrategy, CatalogContentRef, ClosedInterval, ControlKind, ControlTopologyBehavior,
    ControlValue, CustomizerControl, CustomizerProfile, DomainCertification, FeasibleControlDomain,
    FoundryCommand, FoundryLock, FoundryLockMode, FoundryLockTarget, ProviderOption,
    WholeModelPreviewRef, catalog_content_fingerprint_from_json,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, headless_fixture_catalogs, scifi_crate, stylized_lamp,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRejectionReason, FoundryCandidateRequest,
    generate_foundry_candidate_plans,
};
use std::collections::BTreeSet;

fn request(seed: u64, mode: FoundryCandidateMode) -> FoundryCandidateRequest {
    FoundryCandidateRequest {
        seed,
        proposal_count: 72,
        result_count: 6,
        mode,
        strategy_id: None,
    }
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
        let structural_controls = [
            "body_variant",
            "accent_style",
            "detail_density",
            "has_accessory",
        ]
        .into_iter()
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
        target: FoundryLockTarget::Control("body_proportions".to_owned()),
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
                assert_ne!(control_id, "body_proportions");
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
fn visually_duplicate_control_plans_collapse() {
    let mut fixture = scifi_crate::fixture_catalog();
    let mut profile = profile(&fixture);
    let control = profile
        .controls
        .iter_mut()
        .find(|control| control.id == "advisory_weathering")
        .expect("fixture should have advisory control");
    control.visible = true;
    control.primary = true;
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
