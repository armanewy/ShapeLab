use std::collections::BTreeMap;

use shape_asset::AssetEditProgram;
use shape_family::{
    FamilyParameterKind, FamilyParameterSlot, LengthUnit, ParameterExecutionPolicy, ParameterRange,
};
use shape_family_compile::{
    FamilyValue,
    identity::{CatalogContentFingerprint, ContentFingerprint, GeometryInputFingerprint},
};
use shape_foundry::{
    CatalogContentRef, ClosedInterval, ControlBuildRequestKind, ControlDivergence,
    ControlEvaluationContext, ControlEvaluationError, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCommand, FoundryDocumentId, LocalRecipeOverride,
    LocalRecipeOverrideId, OverrideSurvivalPolicy, ProviderOption, ResponseCurve,
    TouchedSemanticTarget, WholeModelPreviewRef, canonicalize_control_value, control_divergence,
    default_control_state, default_control_value, effective_control_domain, evaluate_control,
    explain_control_delta, reset_control_state, validate_customizer_profile,
    validate_foundry_command, whole_model_exact_build_request, whole_model_preview_sample_requests,
};

#[test]
fn linked_edge_softness_axis_fans_out_and_samples_preview() {
    let control = edge_softness_control();
    let slots = heft_slots();
    let context = ControlEvaluationContext::new(&slots);

    let evaluated = evaluate_control(&control, context, ControlValue::Scalar(0.5)).unwrap();

    assert_family_scalar(evaluated.slot_values.get("box_width"), 1.55);
    assert_family_scalar(evaluated.slot_values.get("edge_radius"), 0.75);

    let previews = whole_model_preview_sample_requests(&control, context).unwrap();
    assert_eq!(previews.len(), 5);
    assert_eq!(
        previews
            .iter()
            .map(|request| request.build_kind)
            .collect::<Vec<_>>(),
        vec![
            ControlBuildRequestKind::PreviewSample,
            ControlBuildRequestKind::PreviewSample,
            ControlBuildRequestKind::PreviewSample,
            ControlBuildRequestKind::PreviewSample,
            ControlBuildRequestKind::PreviewSample,
        ]
    );
    assert_eq!(
        previews
            .iter()
            .map(|request| request.value.clone())
            .collect::<Vec<_>>(),
        vec![
            ControlValue::Scalar(-1.0),
            ControlValue::Scalar(-0.5),
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(0.5),
            ControlValue::Scalar(1.0),
        ]
    );

    let release =
        whole_model_exact_build_request(&control, context, ControlValue::Scalar(0.45)).unwrap();
    assert_eq!(release.build_kind, ControlBuildRequestKind::ExactOnRelease);
    assert_eq!(release.value, ControlValue::Scalar(0.45));
}

#[test]
fn piecewise_curve_evaluates_and_rejects_unsafe_authored_curves() {
    let mut control = edge_softness_control();
    control.bindings = vec![ControlSlotBinding {
        slot: "box_width".to_owned(),
        slot_policy: ParameterExecutionPolicy::RequiredBinding,
        response: ResponseCurve::Piecewise {
            points: vec![[-1.0, 0.0], [0.0, 0.5], [1.0, 2.0]],
            monotonic: true,
        },
    }];
    let slots = vec![length_slot("box_width", 0.0, 3.0, 0.1)];
    let context = ControlEvaluationContext::new(&slots);

    let evaluated = evaluate_control(&control, context, ControlValue::Scalar(0.5)).unwrap();
    assert_family_scalar(evaluated.slot_values.get("box_width"), 1.25);

    let mut non_monotonic = profile_with(control.clone());
    non_monotonic.controls[0].bindings[0].response = ResponseCurve::Piecewise {
        points: vec![[-1.0, 0.0], [0.0, 1.0], [1.0, 0.5]],
        monotonic: true,
    };
    let report = validate_customizer_profile(&non_monotonic);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "non_monotonic_response_curve")
    );

    let mut non_finite_output = profile_with(control);
    non_finite_output.controls[0].bindings[0].response = ResponseCurve::Piecewise {
        points: vec![[0.0, -f32::MAX], [1.0, f32::MAX]],
        monotonic: false,
    };
    let report = validate_customizer_profile(&non_finite_output);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "non_finite_response_curve_output")
    );
}

#[test]
fn count_stepper_snaps_to_available_integer_domain() {
    let control = count_stepper_control();
    let slots = vec![count_slot("box_count", 2.0, 8.0, 2.0)];
    let context = ControlEvaluationContext::new(&slots);

    let default_state = default_control_state(&profile_with(control.clone()), context).unwrap();
    assert_eq!(
        default_state.get("box_count"),
        Some(&ControlValue::Integer(4))
    );
    assert_eq!(
        canonicalize_control_value(&control, context, ControlValue::Integer(5)).unwrap(),
        ControlValue::Integer(4)
    );

    let evaluated = evaluate_control(&control, context, ControlValue::Integer(5)).unwrap();
    assert_eq!(
        evaluated.slot_values.get("box_count"),
        Some(&FamilyValue::Integer(4))
    );
}

#[test]
fn provider_gallery_uses_available_options_and_rejects_unavailable_selection() {
    let control = provider_control();
    let context = ControlEvaluationContext::new(&[]);

    assert_eq!(
        default_control_value(&control, context).unwrap(),
        ControlValue::Provider("compact_body".to_owned())
    );
    assert_eq!(
        canonicalize_control_value(
            &control,
            context,
            ControlValue::Provider("wide_body".to_owned())
        )
        .unwrap_err(),
        ControlEvaluationError::UnavailableOption {
            control_id: "body_provider".to_owned(),
            option: "wide_body".to_owned(),
            reason: "option is unavailable for Box Primitive".to_owned(),
        }
    );

    let previews = whole_model_preview_sample_requests(&control, context).unwrap();
    assert_eq!(previews.len(), 1);
    assert_eq!(
        previews[0].value,
        ControlValue::Provider("compact_body".to_owned())
    );

    let command = FoundryCommand::SetControl {
        control_id: "body_provider".to_owned(),
        value: ControlValue::Provider("wide_body".to_owned()),
    };
    let report = validate_foundry_command(&command, None, Some(&profile_with(control)));
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unavailable_control_option")
    );
}

#[test]
fn constraint_provider_narrows_with_full_feasible_domain() {
    let control = edge_softness_control();
    let slots = vec![
        length_slot("box_width", 1.0, 1.6, 0.1),
        length_slot("edge_radius", 0.25, 1.0, 0.05),
    ];
    let conformance_domains = BTreeMap::from([(
        "edge_softness".to_owned(),
        FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: -0.5,
                maximum: 0.25,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
    )]);
    let context =
        ControlEvaluationContext::with_constraint_range_provider(&slots, &conformance_domains);

    let domain = effective_control_domain(&control, context).unwrap();
    assert_eq!(
        domain.continuous_intervals,
        vec![ClosedInterval {
            minimum: -0.5,
            maximum: 0.25,
        }]
    );
    let evaluated = evaluate_control(&control, context, ControlValue::Scalar(0.0)).unwrap();
    assert_eq!(evaluated.divergence, ControlDivergence::ConstraintLimited);

    let previews = whole_model_preview_sample_requests(&control, context).unwrap();
    assert_eq!(previews.len(), 5);
    assert_eq!(previews[0].value, ControlValue::Scalar(-0.5));
    assert_eq!(previews[4].value, ControlValue::Scalar(0.25));
}

#[test]
fn uncertified_continuous_domains_use_discrete_preview_samples() {
    let control = edge_softness_control();
    let slots = heft_slots();
    let conformance_domains = BTreeMap::from([(
        "edge_softness".to_owned(),
        FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: -0.5,
                maximum: 0.5,
            }],
            discrete_values: vec![ControlValue::Scalar(-0.5), ControlValue::Scalar(0.5)],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::Uncertified {
                reason: "nonconvex visual survival".to_owned(),
            },
        },
    )]);
    let context =
        ControlEvaluationContext::with_constraint_range_provider(&slots, &conformance_domains);

    let previews = whole_model_preview_sample_requests(&control, context).unwrap();

    assert_eq!(
        previews
            .iter()
            .map(|request| request.value.clone())
            .collect::<Vec<_>>(),
        vec![ControlValue::Scalar(-0.5), ControlValue::Scalar(0.5)]
    );
}

#[test]
fn local_override_marks_bound_control_diverged_and_reset_is_explained() {
    let control = edge_softness_control();
    let profile = profile_with(control.clone());
    let slots = heft_slots();
    let context = ControlEvaluationContext::new(&slots);
    let mut document = document_fixture();
    document.local_recipe_overrides.push(LocalRecipeOverride {
        id: LocalRecipeOverrideId("override-1".to_owned()),
        base_geometry_fingerprint: GeometryInputFingerprint(ContentFingerprint([9; 32])),
        edit_program: AssetEditProgram {
            label: "Manual edge tweak".to_owned(),
            seed: 0,
            operations: Vec::new(),
        },
        touched_targets: vec![TouchedSemanticTarget::FamilySlot("box_width".to_owned())],
        survival_policy: OverrideSurvivalPolicy::Revalidate,
    });

    assert_eq!(
        control_divergence(&control, &document),
        ControlDivergence::DivergedByOverride
    );

    let mut state = BTreeMap::from([("edge_softness".to_owned(), ControlValue::Scalar(0.75))]);
    let delta = reset_control_state(&profile, context, &mut state, "edge_softness").unwrap();
    assert_eq!(state.get("edge_softness"), Some(&ControlValue::Scalar(0.0)));
    assert_eq!(delta.current, ControlValue::Scalar(0.0));
    assert_eq!(
        delta
            .explanations
            .iter()
            .map(|explanation| (explanation.subject.as_str(), explanation.code.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("controls.edge_softness", "control_reset_to_default"),
            (
                "controls.edge_softness.bindings.box_width",
                "slot_value_changed"
            ),
            (
                "controls.edge_softness.bindings.edge_radius",
                "slot_value_changed"
            ),
        ]
    );

    let repeat = explain_control_delta(
        &profile,
        context,
        "edge_softness",
        Some(ControlValue::Scalar(0.75)),
        ControlValue::Scalar(0.0),
    )
    .unwrap();
    assert_eq!(delta.explanations, repeat.explanations);
}

#[test]
fn reset_control_state_is_atomic_when_previous_value_is_invalid() {
    let control = edge_softness_control();
    let profile = profile_with(control);
    let slots = heft_slots();
    let context = ControlEvaluationContext::new(&slots);
    let mut state = BTreeMap::from([(
        "edge_softness".to_owned(),
        ControlValue::Provider("wrong-kind".to_owned()),
    )]);

    let error = reset_control_state(&profile, context, &mut state, "edge_softness")
        .expect_err("invalid previous value should fail explanation");

    assert_eq!(
        error,
        ControlEvaluationError::WrongValueKind {
            control_id: "edge_softness".to_owned()
        }
    );
    assert_eq!(
        state.get("edge_softness"),
        Some(&ControlValue::Provider("wrong-kind".to_owned()))
    );
}

#[test]
fn validation_rejects_provider_role_conflicts_and_unknown_reset_controls() {
    let mut first = provider_control();
    first.id = "body_provider_a".to_owned();
    let mut second = provider_control();
    second.id = "body_provider_b".to_owned();
    let mut profile = CustomizerProfile::empty("box", Some("test-style".to_owned()));
    profile.controls = vec![first, second];

    let report = validate_customizer_profile(&profile);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "conflicting_provider_role_owner")
    );

    let command = FoundryCommand::ResetControl {
        control_id: "missing-control".to_owned(),
    };
    let report = validate_foundry_command(&command, None, Some(&profile));
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unknown_command_control")
    );
}

fn edge_softness_control() -> CustomizerControl {
    CustomizerControl {
        id: "edge_softness".to_owned(),
        label: "Edge Softness".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default: 0.0 },
        bindings: vec![
            ControlSlotBinding {
                slot: "box_width".to_owned(),
                slot_policy: ParameterExecutionPolicy::RequiredBinding,
                response: ResponseCurve::Piecewise {
                    points: vec![[-1.0, 0.8], [1.0, 1.8]],
                    monotonic: true,
                },
            },
            ControlSlotBinding {
                slot: "edge_radius".to_owned(),
                slot_policy: ParameterExecutionPolicy::RequiredBinding,
                response: ResponseCurve::Piecewise {
                    points: vec![[-1.0, 0.3], [1.0, 0.9]],
                    monotonic: true,
                },
            },
        ],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: -1.0,
                maximum: 1.0,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    }
}

fn count_stepper_control() -> CustomizerControl {
    CustomizerControl {
        id: "box_count".to_owned(),
        label: "Box Count".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::IntegerStepper { default: 5 },
        bindings: vec![ControlSlotBinding {
            slot: "box_count".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Integer(2),
                ControlValue::Integer(4),
                ControlValue::Integer(8),
            ],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn provider_control() -> CustomizerControl {
    CustomizerControl {
        id: "body_provider".to_owned(),
        label: "Body Provider".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ProviderGallery {
            role: "body".to_owned(),
            options: vec![
                ProviderOption {
                    provider_id: "compact_body".to_owned(),
                    label: "Compact Body".to_owned(),
                    preview: preview("compact-body"),
                },
                ProviderOption {
                    provider_id: "wide_body".to_owned(),
                    label: "Wide Body".to_owned(),
                    preview: preview("wide-body"),
                },
            ],
        },
        bindings: vec![ControlSlotBinding {
            slot: "body_provider".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Provider("compact_body".to_owned()),
                ControlValue::Provider("wide_body".to_owned()),
            ],
            unavailable_options: BTreeMap::from([(
                "wide_body".to_owned(),
                "option is unavailable for Box Primitive".to_owned(),
            )]),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn profile_with(control: CustomizerControl) -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty("box", Some("test-style".to_owned()));
    profile.controls.push(control);
    profile
}

fn heft_slots() -> Vec<FamilyParameterSlot> {
    vec![
        length_slot("box_width", 0.5, 2.0, 0.1),
        length_slot("edge_radius", 0.25, 1.0, 0.05),
    ]
}

fn length_slot(id: &str, minimum: f32, maximum: f32, step: f32) -> FamilyParameterSlot {
    FamilyParameterSlot {
        id: id.to_owned(),
        label: id.replace('_', " "),
        target_role: None,
        kind: FamilyParameterKind::Length {
            unit: LengthUnit::FamilyUnits,
        },
        range: Some(ParameterRange {
            minimum,
            maximum,
            step,
        }),
        default_value: None,
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: false,
    }
}

fn count_slot(id: &str, minimum: f32, maximum: f32, step: f32) -> FamilyParameterSlot {
    FamilyParameterSlot {
        id: id.to_owned(),
        label: id.replace('_', " "),
        target_role: None,
        kind: FamilyParameterKind::Count,
        range: Some(ParameterRange {
            minimum,
            maximum,
            step,
        }),
        default_value: None,
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: true,
    }
}

fn preview(id: &str) -> WholeModelPreviewRef {
    WholeModelPreviewRef {
        preview_id: format!("preview-{id}"),
        artifact_fingerprint: None,
    }
}

fn document_fixture() -> FoundryAssetDocument {
    FoundryAssetDocument {
        schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
        document_id: FoundryDocumentId("doc-1".to_owned()),
        family_content_ref: content_ref("box-family", 1),
        style_content_ref: content_ref("test-style", 2),
        family_implementation_ref: content_ref("box-family-impl", 3),
        style_implementation_ref: content_ref("test-style-impl", 4),
        customizer_profile_ref: content_ref("box-profile", 5),
        control_state: BTreeMap::new(),
        provider_overrides: BTreeMap::new(),
        foundry_locks: Vec::new(),
        variation_state: shape_foundry::FoundryVariationState::default(),
        local_recipe_overrides: Vec::new(),
        seed: 0,
        catalog_lock: None,
        build_stamp: None,
    }
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: CatalogContentFingerprint(ContentFingerprint([byte; 32])),
    }
}

fn assert_family_scalar(value: Option<&FamilyValue>, expected: f32) {
    let Some(FamilyValue::Scalar(actual)) = value else {
        panic!("expected scalar family value, got {value:?}");
    };
    assert!(
        (*actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}
