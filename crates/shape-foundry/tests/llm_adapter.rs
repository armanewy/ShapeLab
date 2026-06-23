use std::collections::BTreeMap;

use shape_family::ParameterExecutionPolicy;
use shape_family_compile::identity::{CatalogContentFingerprint, ContentFingerprint};
use shape_foundry::{
    CatalogContentRef, ChoiceOption, ClosedInterval, ControlDivergence, ControlKind,
    ControlSlotBinding, ControlTopologyBehavior, ControlValue, CustomizerControl,
    CustomizerProfile, DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
    FeasibleControlDomain, FoundryAssetDocument, FoundryCandidateId, FoundryCandidateStatus,
    FoundryCandidateSummary, FoundryCommand, FoundryDocumentId, FoundryLlmAdapterContext,
    FoundryLlmAdapterError, FoundryLlmAdapterResponse, FoundryLlmControlKind, FoundryLlmIntent,
    FoundryLock, FoundryLockMode, FoundryLockTarget, GenerateCandidatesRequest, ProviderOption,
    ProviderOverride, ResponseCurve, WholeModelPreviewRef, foundry_llm_visible_controls,
    plan_foundry_llm_intent,
};

#[test]
fn list_controls_exposes_visible_control_surface_without_hidden_paths() {
    let document = document_fixture();
    let profile = profile_fixture();

    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::ListControls,
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap();

    let FoundryLlmAdapterResponse::ControlList { controls } = plan.response else {
        panic!("expected control list");
    };
    assert_eq!(controls.len(), 4);
    assert!(
        controls
            .iter()
            .all(|control| control.id != "internal_scalar")
    );
    assert!(controls.iter().all(|control| control.primary));
    assert!(controls.iter().any(|control| control.id == "span_length"
        && control.kind == FoundryLlmControlKind::Scalar
        && control.continuous_intervals == vec![interval(-1.0, 1.0)]));
    assert!(controls.iter().any(|control| {
        control.id == "support_provider"
            && control.kind == FoundryLlmControlKind::Provider
            && control
                .options
                .iter()
                .any(|option| option.value == "timber_support" && option.available)
    }));
    assert!(!plan.safety.direct_recipe_mutation_allowed);
    assert!(!plan.safety.hidden_scalar_paths_exposed);
    assert!(!plan.safety.preview_required_before_commit);
}

#[test]
fn set_control_translates_to_validated_previewable_command() {
    let document = document_fixture();
    let profile = profile_fixture();

    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::SetControl {
            control_id: "span_length".to_owned(),
            value: ControlValue::Scalar(0.4),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap();

    assert_eq!(
        plan.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::SetControl {
                control_id: "span_length".to_owned(),
                value: ControlValue::Scalar(0.4),
            }
        }
    );
    assert!(plan.safety.command_validated);
    assert!(plan.safety.preview_required_before_commit);
    assert!(plan.safety.undo_checkpoint_required);
    assert!(!plan.safety.host_confirmation_required);
}

#[test]
fn invalid_or_hidden_control_intents_are_rejected_before_command_output() {
    let document = document_fixture();
    let profile = profile_fixture();
    let context = FoundryLlmAdapterContext::new(&document, &profile);

    let hidden = plan_foundry_llm_intent(
        FoundryLlmIntent::SetControl {
            control_id: "internal_scalar".to_owned(),
            value: ControlValue::Scalar(0.0),
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        hidden,
        FoundryLlmAdapterError::HiddenControl {
            control_id: "internal_scalar".to_owned(),
        }
    );

    let invalid = plan_foundry_llm_intent(
        FoundryLlmIntent::SetControl {
            control_id: "span_length".to_owned(),
            value: ControlValue::Scalar(8.0),
        },
        context,
    )
    .unwrap_err();
    let FoundryLlmAdapterError::InvalidCommand { report } = invalid else {
        panic!("expected invalid command report");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "control_value_outside_domain")
    );
}

#[test]
fn provider_selection_uses_visible_provider_gallery_not_catalog_refs() {
    let document = document_fixture();
    let profile = profile_fixture();

    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::SelectProvider {
            control_id: "support_provider".to_owned(),
            provider_id: "stone_support".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap();

    assert_eq!(
        plan.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::SetControl {
                control_id: "support_provider".to_owned(),
                value: ControlValue::Provider("stone_support".to_owned()),
            }
        }
    );

    let wrong_kind = plan_foundry_llm_intent(
        FoundryLlmIntent::SelectProvider {
            control_id: "span_length".to_owned(),
            provider_id: "stone_support".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        wrong_kind,
        FoundryLlmAdapterError::WrongControlKind {
            control_id: "span_length".to_owned(),
            expected: "provider".to_owned(),
        }
    );
}

#[test]
fn provider_overrides_are_reported_and_conflicts_are_rejected() {
    let mut document = document_fixture();
    document.provider_overrides.insert(
        "support".to_owned(),
        ProviderOverride {
            role: "support".to_owned(),
            provider_ref: content_ref("stone_support", 6),
        },
    );
    let profile = profile_fixture();

    let controls = foundry_llm_visible_controls(&document, &profile);
    let provider = controls
        .iter()
        .find(|control| control.id == "support_provider")
        .unwrap();
    assert_eq!(
        provider.current_value,
        ControlValue::Provider("stone_support".to_owned())
    );

    let conflict = plan_foundry_llm_intent(
        FoundryLlmIntent::SelectProvider {
            control_id: "support_provider".to_owned(),
            provider_id: "timber_support".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        conflict,
        FoundryLlmAdapterError::ExistingProviderOverrideConflict {
            control_id: "support_provider".to_owned(),
            role: "support".to_owned(),
            existing_provider_id: "stone_support".to_owned(),
            requested_provider_id: "timber_support".to_owned(),
        }
    );

    let set_control_bypass = plan_foundry_llm_intent(
        FoundryLlmIntent::SetControl {
            control_id: "support_provider".to_owned(),
            value: ControlValue::Provider("timber_support".to_owned()),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(set_control_bypass, conflict);

    let same_provider = plan_foundry_llm_intent(
        FoundryLlmIntent::SelectProvider {
            control_id: "support_provider".to_owned(),
            provider_id: "stone_support".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap();
    assert_eq!(
        same_provider.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::SetControl {
                control_id: "support_provider".to_owned(),
                value: ControlValue::Provider("stone_support".to_owned()),
            }
        }
    );
}

#[test]
fn invisible_provider_overrides_do_not_leak_catalog_ids() {
    let mut document = document_fixture();
    document.provider_overrides.insert(
        "support".to_owned(),
        ProviderOverride {
            role: "support".to_owned(),
            provider_ref: content_ref("lower_level_support", 7),
        },
    );
    let profile = profile_fixture();

    let controls = foundry_llm_visible_controls(&document, &profile);
    let provider = controls
        .iter()
        .find(|control| control.id == "support_provider")
        .unwrap();
    assert_ne!(
        provider.current_value,
        ControlValue::Provider("lower_level_support".to_owned())
    );

    let conflict = plan_foundry_llm_intent(
        FoundryLlmIntent::SelectProvider {
            control_id: "support_provider".to_owned(),
            provider_id: "timber_support".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        conflict,
        FoundryLlmAdapterError::ExistingProviderOverrideOutsideVisibleOptions {
            control_id: "support_provider".to_owned(),
            role: "support".to_owned(),
        }
    );
}

#[test]
fn locked_controls_and_large_candidate_requests_are_rejected() {
    let mut document = document_fixture();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("span_length".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("User locked Span Length".to_owned()),
    });
    let profile = profile_fixture();

    let locked = plan_foundry_llm_intent(
        FoundryLlmIntent::SetControl {
            control_id: "span_length".to_owned(),
            value: ControlValue::Scalar(0.1),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        locked,
        FoundryLlmAdapterError::LockedControl {
            control_id: "span_length".to_owned(),
            reason: Some("User locked Span Length".to_owned()),
        }
    );

    let too_large = plan_foundry_llm_intent(
        FoundryLlmIntent::GenerateCandidates {
            strategy_id: None,
            count: 12,
            seed: None,
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        too_large,
        FoundryLlmAdapterError::CandidateRequestTooLarge {
            requested: 12,
            maximum: 6,
        }
    );

    let missing_strategy = plan_foundry_llm_intent(
        FoundryLlmIntent::GenerateCandidates {
            strategy_id: None,
            count: 3,
            seed: None,
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        missing_strategy,
        FoundryLlmAdapterError::CandidateStrategyRequired
    );
}

#[test]
fn candidate_acceptance_requires_current_proposed_candidate() {
    let document = document_fixture();
    let profile = profile_fixture();
    let proposed = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-a".to_owned()),
        label: "Candidate A".to_owned(),
        status: FoundryCandidateStatus::Proposed,
        changed_controls: vec!["span_length".to_owned()],
        preview_id: Some("preview-a".to_owned()),
    };
    let accepted = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-b".to_owned()),
        label: "Candidate B".to_owned(),
        status: FoundryCandidateStatus::Accepted,
        changed_controls: vec!["span_length".to_owned()],
        preview_id: Some("preview-b".to_owned()),
    };
    let candidates = vec![proposed, accepted];
    let context = FoundryLlmAdapterContext::new(&document, &profile).with_candidates(&candidates);

    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::AcceptCandidate {
            candidate_id: FoundryCandidateId("candidate-a".to_owned()),
        },
        context,
    )
    .unwrap();
    assert_eq!(
        plan.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::AcceptCandidate {
                candidate_id: FoundryCandidateId("candidate-a".to_owned()),
            }
        }
    );

    let stale = plan_foundry_llm_intent(
        FoundryLlmIntent::AcceptCandidate {
            candidate_id: FoundryCandidateId("candidate-b".to_owned()),
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        stale,
        FoundryLlmAdapterError::CandidateNotProposed {
            candidate_id: FoundryCandidateId("candidate-b".to_owned()),
            status: FoundryCandidateStatus::Accepted,
        }
    );
}

#[test]
fn candidate_generation_and_acceptance_stay_inside_visible_unlocked_controls() {
    let mut document = document_fixture();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("span_length".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("Protected from candidate search".to_owned()),
    });
    let mut profile = profile_fixture();
    profile
        .candidate_strategies
        .push(shape_foundry::CandidateStrategy {
            id: "hidden_refine".to_owned(),
            label: "Hidden Refine".to_owned(),
            control_ids: vec!["internal_scalar".to_owned()],
        });
    let locked_candidate = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-locked".to_owned()),
        label: "Locked Candidate".to_owned(),
        status: FoundryCandidateStatus::Proposed,
        changed_controls: vec!["span_length".to_owned()],
        preview_id: Some("preview-locked".to_owned()),
    };
    let hidden_candidate = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-hidden".to_owned()),
        label: "Hidden Candidate".to_owned(),
        status: FoundryCandidateStatus::Proposed,
        changed_controls: vec!["internal_scalar".to_owned()],
        preview_id: Some("preview-hidden".to_owned()),
    };
    let candidates = vec![locked_candidate, hidden_candidate];
    let context = FoundryLlmAdapterContext::new(&document, &profile).with_candidates(&candidates);

    let locked_strategy = plan_foundry_llm_intent(
        FoundryLlmIntent::GenerateCandidates {
            strategy_id: Some("refine".to_owned()),
            count: 3,
            seed: None,
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        locked_strategy,
        FoundryLlmAdapterError::CandidateStrategyTouchesLockedControl {
            strategy_id: "refine".to_owned(),
            control_id: "span_length".to_owned(),
            reason: Some("Protected from candidate search".to_owned()),
        }
    );

    let hidden_strategy = plan_foundry_llm_intent(
        FoundryLlmIntent::GenerateCandidates {
            strategy_id: Some("hidden_refine".to_owned()),
            count: 3,
            seed: None,
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        hidden_strategy,
        FoundryLlmAdapterError::CandidateStrategyTouchesHiddenControl {
            strategy_id: "hidden_refine".to_owned(),
            control_id: "internal_scalar".to_owned(),
        }
    );

    let locked_accept = plan_foundry_llm_intent(
        FoundryLlmIntent::AcceptCandidate {
            candidate_id: FoundryCandidateId("candidate-locked".to_owned()),
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        locked_accept,
        FoundryLlmAdapterError::CandidateTouchesLockedControl {
            candidate_id: FoundryCandidateId("candidate-locked".to_owned()),
            control_id: "span_length".to_owned(),
            reason: Some("Protected from candidate search".to_owned()),
        }
    );

    let hidden_accept = plan_foundry_llm_intent(
        FoundryLlmIntent::AcceptCandidate {
            candidate_id: FoundryCandidateId("candidate-hidden".to_owned()),
        },
        context,
    )
    .unwrap_err();
    assert_eq!(
        hidden_accept,
        FoundryLlmAdapterError::CandidateTouchesHiddenControl {
            candidate_id: FoundryCandidateId("candidate-hidden".to_owned()),
            control_id: "internal_scalar".to_owned(),
        }
    );
}

#[test]
fn export_requires_host_allow_list_and_confirmation_gate() {
    let document = document_fixture();
    let profile = profile_fixture();

    let missing_allow_list = plan_foundry_llm_intent(
        FoundryLlmIntent::Export {
            profile: "model-package".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap_err();
    assert_eq!(
        missing_allow_list,
        FoundryLlmAdapterError::ExportProfilesRequired
    );

    let export_profiles = vec!["model-package".to_owned()];
    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::Export {
            profile: "model-package".to_owned(),
        },
        FoundryLlmAdapterContext::new(&document, &profile).with_export_profiles(&export_profiles),
    )
    .unwrap();
    assert_eq!(
        plan.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::Export {
                profile: "model-package".to_owned(),
                out_dir: None,
            }
        }
    );
    assert!(plan.safety.command_validated);
    assert!(plan.safety.preview_required_before_commit);
    assert!(plan.safety.host_confirmation_required);
    assert!(!plan.safety.undo_checkpoint_required);
}

#[test]
fn describe_state_and_generate_candidates_stay_on_typed_surfaces() {
    let document = document_fixture();
    let profile = profile_fixture();
    let export_profiles = vec!["model-package".to_owned()];
    let hidden_candidate = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-hidden".to_owned()),
        label: "Hidden Candidate".to_owned(),
        status: FoundryCandidateStatus::Proposed,
        changed_controls: vec!["internal_scalar".to_owned()],
        preview_id: Some("preview-hidden".to_owned()),
    };
    let visible_candidate = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-visible".to_owned()),
        label: "Visible Candidate".to_owned(),
        status: FoundryCandidateStatus::Proposed,
        changed_controls: vec!["span_length".to_owned()],
        preview_id: Some("preview-visible".to_owned()),
    };
    let accepted_visible_candidate = FoundryCandidateSummary {
        id: FoundryCandidateId("candidate-accepted".to_owned()),
        label: "Accepted Candidate".to_owned(),
        status: FoundryCandidateStatus::Accepted,
        changed_controls: vec!["span_length".to_owned()],
        preview_id: Some("preview-accepted".to_owned()),
    };
    let candidates = vec![
        hidden_candidate,
        visible_candidate,
        accepted_visible_candidate,
    ];
    let context = FoundryLlmAdapterContext::new(&document, &profile)
        .with_candidates(&candidates)
        .with_export_profiles(&export_profiles);

    let describe = plan_foundry_llm_intent(FoundryLlmIntent::DescribeState, context).unwrap();
    let FoundryLlmAdapterResponse::StateSummary { state } = describe.response else {
        panic!("expected LLM-safe state summary");
    };
    assert_eq!(state.controls.len(), 4);
    assert_eq!(state.candidates.len(), 1);
    assert_eq!(
        state.candidates[0].id,
        FoundryCandidateId("candidate-visible".to_owned())
    );
    assert_eq!(state.export_profiles, export_profiles);
    assert!(!describe.safety.command_validated);

    let generate = plan_foundry_llm_intent(
        FoundryLlmIntent::GenerateCandidates {
            strategy_id: Some("refine".to_owned()),
            count: 3,
            seed: None,
        },
        context,
    )
    .unwrap();
    assert_eq!(
        generate.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::GenerateCandidates(GenerateCandidatesRequest {
                strategy_id: Some("refine".to_owned()),
                count: 3,
                seed: 11,
            })
        }
    );
}

#[test]
fn visible_control_helper_marks_locked_provider_roles() {
    let mut document = document_fixture();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Provider("support".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("Provider locked".to_owned()),
    });
    let profile = profile_fixture();

    let controls = foundry_llm_visible_controls(&document, &profile);
    let provider = controls
        .iter()
        .find(|control| control.id == "support_provider")
        .unwrap();
    assert!(provider.locked);
    assert_eq!(provider.locked_reason.as_deref(), Some("Provider locked"));
}

#[test]
fn unlock_provider_control_clears_provider_role_lock_target() {
    let mut document = document_fixture();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Provider("support".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("Provider locked".to_owned()),
    });
    let profile = profile_fixture();

    let plan = plan_foundry_llm_intent(
        FoundryLlmIntent::LockControl {
            control_id: "support_provider".to_owned(),
            locked: false,
        },
        FoundryLlmAdapterContext::new(&document, &profile),
    )
    .unwrap();

    assert_eq!(
        plan.response,
        FoundryLlmAdapterResponse::Command {
            command: FoundryCommand::ClearLock {
                target: FoundryLockTarget::Provider("support".to_owned()),
            }
        }
    );
}

fn profile_fixture() -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile.controls = vec![
        slider_control("span_length", "Span Length", true),
        integer_control(),
        choice_control(),
        provider_control(),
        slider_control("internal_scalar", "Internal Scalar", false),
    ];
    profile
        .candidate_strategies
        .push(shape_foundry::CandidateStrategy {
            id: "refine".to_owned(),
            label: "Refine".to_owned(),
            control_ids: vec!["span_length".to_owned()],
        });
    profile
}

fn slider_control(id: &str, label: &str, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: visible,
        visible,
        kind: ControlKind::ContinuousAxis { default: 0.0 },
        bindings: vec![ControlSlotBinding {
            slot: id.to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![interval(-1.0, 1.0)],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    }
}

fn integer_control() -> CustomizerControl {
    CustomizerControl {
        id: "support_count".to_owned(),
        label: "Support Count".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::IntegerStepper { default: 2 },
        bindings: vec![ControlSlotBinding {
            slot: "support_count".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![ControlValue::Integer(2), ControlValue::Integer(4)],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn choice_control() -> CustomizerControl {
    CustomizerControl {
        id: "deck_style".to_owned(),
        label: "Deck Style".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ChoiceGallery {
            options: vec![option("plain", "Plain"), option("braced", "Braced")],
        },
        bindings: vec![ControlSlotBinding {
            slot: "deck_style".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Choice("plain".to_owned()),
                ControlValue::Choice("braced".to_owned()),
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
        id: "support_provider".to_owned(),
        label: "Support Provider".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ProviderGallery {
            role: "support".to_owned(),
            options: vec![
                provider_option("timber_support", "Timber Support"),
                provider_option("stone_support", "Stone Support"),
            ],
        },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Provider("timber_support".to_owned()),
                ControlValue::Provider("stone_support".to_owned()),
            ],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn option(value: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        value: value.to_owned(),
        label: label.to_owned(),
        preview: WholeModelPreviewRef {
            preview_id: format!("preview-{value}"),
            artifact_fingerprint: None,
        },
    }
}

fn provider_option(provider_id: &str, label: &str) -> ProviderOption {
    ProviderOption {
        provider_id: provider_id.to_owned(),
        label: label.to_owned(),
        preview: WholeModelPreviewRef {
            preview_id: format!("preview-{provider_id}"),
            artifact_fingerprint: None,
        },
    }
}

fn document_fixture() -> FoundryAssetDocument {
    FoundryAssetDocument {
        schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
        document_id: FoundryDocumentId("doc-1".to_owned()),
        family_content_ref: content_ref("bridge-family", 1),
        style_content_ref: content_ref("roman-style", 2),
        family_implementation_ref: content_ref("bridge-family-impl", 3),
        style_implementation_ref: content_ref("roman-style-impl", 4),
        customizer_profile_ref: content_ref("bridge-profile", 5),
        control_state: BTreeMap::from([(
            "span_length".to_owned(),
            shape_foundry::ControlValue::Scalar(0.0),
        )]),
        provider_overrides: BTreeMap::new(),
        foundry_locks: Vec::new(),
        local_recipe_overrides: Vec::new(),
        seed: 11,
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

fn interval(minimum: f32, maximum: f32) -> ClosedInterval {
    ClosedInterval { minimum, maximum }
}
