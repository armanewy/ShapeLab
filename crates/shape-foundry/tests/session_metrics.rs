use shape_foundry::{
    FoundryCandidateId, FoundryDocumentId, FoundryPreferenceEvent, FoundryPreferenceLog,
    FoundryPreferenceScope, FoundrySession, FoundrySessionId, FoundryUsabilityEvent,
    FoundryUsabilityLog, FoundryUsabilityRecord,
};

#[test]
fn usability_metrics_calculate_rates_counts_and_first_times() {
    let mut log = FoundryUsabilityLog::new();
    log.record(FoundryUsabilityRecord::new(
        0,
        FoundryUsabilityEvent::ProfileOpened,
    ));
    log.record(FoundryUsabilityRecord::new(
        250,
        FoundryUsabilityEvent::BuildCompleted,
    ));
    log.record(FoundryUsabilityRecord::new(
        300,
        FoundryUsabilityEvent::BuildCompleted,
    ));
    log.record(FoundryUsabilityRecord::new(
        400,
        FoundryUsabilityEvent::CandidateRequest { requested_count: 4 },
    ));
    log.record(FoundryUsabilityRecord::new(
        450,
        FoundryUsabilityEvent::CandidateSurvival { survived_count: 3 },
    ));
    log.record(FoundryUsabilityRecord::new(
        500,
        FoundryUsabilityEvent::CandidateAccepted { accepted_count: 1 },
    ));
    log.record(FoundryUsabilityRecord::new(
        600,
        FoundryUsabilityEvent::ControlChange { accepted: true },
    ));
    log.record(FoundryUsabilityRecord::new(
        700,
        FoundryUsabilityEvent::ControlChange { accepted: false },
    ));
    log.record(FoundryUsabilityRecord::new(
        800,
        FoundryUsabilityEvent::InvalidAttempt,
    ));
    log.record(FoundryUsabilityRecord::new(
        900,
        FoundryUsabilityEvent::AdvancedRecipeViewOpened,
    ));
    log.record(FoundryUsabilityRecord::new(
        950,
        FoundryUsabilityEvent::AdvancedRecipeViewOpened,
    ));
    log.record(FoundryUsabilityRecord::new(
        1_300,
        FoundryUsabilityEvent::Export,
    ));
    log.record(FoundryUsabilityRecord::new(
        1_500,
        FoundryUsabilityEvent::Export,
    ));

    let metrics = log.metrics();

    assert_eq!(metrics.control_success_rate, Some(0.5));
    assert_eq!(metrics.candidate_survival_rate, Some(0.75));
    assert_eq!(metrics.accepted_change_count, 2);
    assert_eq!(metrics.invalid_state_attempts, 1);
    assert_eq!(metrics.advanced_view_visits, 2);
    assert_eq!(metrics.total_session_time_ms, 1_500);
    assert_eq!(metrics.time_to_first_build_ms, Some(250));
    assert_eq!(metrics.time_to_first_export_ms, Some(1_300));
}

#[test]
fn usability_metrics_leave_rates_empty_when_no_attempts_exist() {
    let mut log = FoundryUsabilityLog::new();
    log.record(FoundryUsabilityRecord::new(
        15,
        FoundryUsabilityEvent::Reset,
    ));
    log.record(FoundryUsabilityRecord::new(20, FoundryUsabilityEvent::Lock));
    log.record(FoundryUsabilityRecord::new(25, FoundryUsabilityEvent::Undo));

    let metrics = log.metrics();

    assert_eq!(metrics.control_success_rate, None);
    assert_eq!(metrics.candidate_survival_rate, None);
    assert_eq!(metrics.accepted_change_count, 0);
    assert_eq!(metrics.invalid_state_attempts, 0);
    assert_eq!(metrics.advanced_view_visits, 0);
    assert_eq!(metrics.total_session_time_ms, 25);
    assert_eq!(metrics.time_to_first_build_ms, None);
    assert_eq!(metrics.time_to_first_export_ms, None);
}

#[test]
fn local_usability_records_are_optional_and_private_by_default() {
    let session = FoundrySession {
        id: FoundrySessionId("session-1".to_owned()),
        document_id: FoundryDocumentId("doc-1".to_owned()),
        candidates: Vec::new(),
        local_usability: None,
        local_preferences: None,
    };

    let json = serde_json::to_string(&session).expect("session serializes");

    assert!(!json.contains("local_usability"));
    assert!(!json.contains("local_preferences"));

    let decoded: FoundrySession =
        serde_json::from_str(r#"{"id":"session-1","document_id":"doc-1","candidates":[]}"#)
            .expect("session without metrics remains compatible");

    assert_eq!(decoded.local_usability, None);
    assert_eq!(decoded.local_preferences, None);
}

#[test]
fn default_usability_payload_omits_paths_and_geometry() {
    let mut log = FoundryUsabilityLog::new();
    log.record(FoundryUsabilityRecord::new(
        100,
        FoundryUsabilityEvent::ProfileOpened,
    ));
    log.record(FoundryUsabilityRecord::new(
        200,
        FoundryUsabilityEvent::ControlChange { accepted: true },
    ));
    log.record(FoundryUsabilityRecord::new(
        300,
        FoundryUsabilityEvent::CandidateRequest { requested_count: 2 },
    ));
    log.record(FoundryUsabilityRecord::new(
        350,
        FoundryUsabilityEvent::CandidateSurvival { survived_count: 1 },
    ));
    log.record(FoundryUsabilityRecord::new(
        400,
        FoundryUsabilityEvent::Export,
    ));

    let json = serde_json::to_string(&log).expect("log serializes");

    assert!(!json.contains("out_dir"));
    assert!(!json.contains("path"));
    assert!(!json.contains("geometry"));
    assert!(!json.contains("mesh"));
    assert!(!json.contains("vertices"));
    assert!(!json.contains("C:\\"));
    assert!(!json.contains("/tmp/"));
}

#[test]
fn preference_profile_derives_bounded_control_scores_from_explicit_local_signals() {
    let scope = FoundryPreferenceScope::new("crate", "crate-profile");
    let mut log = FoundryPreferenceLog::new();
    log.record(FoundryPreferenceEvent::CandidateComparison {
        scope: scope.clone(),
        mode: Some("explore".to_owned()),
        accepted_candidate_id: FoundryCandidateId("candidate-a".to_owned()),
        accepted_controls: vec!["body_proportions".to_owned(), "edge_softness".to_owned()],
        rejected_candidate_ids: vec![FoundryCandidateId("candidate-b".to_owned())],
        rejected_controls: vec!["handle_style".to_owned()],
        weight: 1.0,
    });
    log.record(FoundryPreferenceEvent::ControlLocked {
        scope: scope.clone(),
        control_id: "handle_style".to_owned(),
        weight: 1.0,
    });
    log.record(FoundryPreferenceEvent::VariantExported {
        scope: scope.clone(),
        changed_controls: vec!["body_proportions".to_owned()],
        weight: 1.0,
    });
    log.record(FoundryPreferenceEvent::PackMemberAdded {
        scope: scope.clone(),
        changed_controls: vec!["edge_softness".to_owned()],
        weight: 1.0,
    });

    let profile = log.profile_for_scope(scope);

    assert_eq!(profile.source_event_count, 4);
    assert!(profile.local_only);
    assert!(
        profile
            .control_preferences
            .get("body_proportions")
            .expect("body preference")
            .score
            > 0.0
    );
    assert!(
        profile
            .control_preferences
            .get("edge_softness")
            .expect("edge preference")
            .score
            > 0.0
    );
    assert!(
        profile
            .control_preferences
            .get("handle_style")
            .expect("handle preference")
            .score
            < 0.0
    );
    assert!(profile.score_changed_controls(&["body_proportions".to_owned()]) > 0.0);
    assert!(profile.score_changed_controls(&["handle_style".to_owned()]) < 0.0);
    assert!(profile.score_changed_controls(&["unknown".to_owned()]) == 0.0);
}

#[test]
fn preference_profile_ignores_other_catalog_scopes() {
    let scope = FoundryPreferenceScope::new("crate", "crate-profile");
    let other_scope = FoundryPreferenceScope::new("lamp", "lamp-profile");
    let mut log = FoundryPreferenceLog::new();
    log.record(FoundryPreferenceEvent::CandidateRejected {
        scope: other_scope,
        candidate_id: FoundryCandidateId("candidate-b".to_owned()),
        changed_controls: vec!["body_proportions".to_owned()],
        weight: 1.0,
    });

    let profile = log.profile_for_scope(scope);

    assert!(profile.is_empty());
    assert_eq!(profile.source_event_count, 0);
}

#[test]
fn non_local_or_unsupported_preference_logs_do_not_derive_usable_profiles() {
    let scope = FoundryPreferenceScope::new("crate", "crate-profile");
    let mut non_local = preference_log_with_positive_signal(scope.clone());
    non_local.local_only = false;

    let profile = non_local.profile_for_scope(scope.clone());

    assert!(!profile.local_only);
    assert!(profile.is_empty());
    assert_eq!(profile.source_event_count, 0);

    let mut unsupported = preference_log_with_positive_signal(scope.clone());
    unsupported.schema_version = 99;
    let profile = unsupported.profile_for_scope(scope);

    assert!(!profile.local_only);
    assert_eq!(profile.schema_version, 99);
    assert!(profile.is_empty());
    assert_eq!(profile.source_event_count, 0);
}

#[test]
fn preference_payload_omits_paths_geometry_and_recipes() {
    let log =
        preference_log_with_positive_signal(FoundryPreferenceScope::new("crate", "crate-profile"));

    let json = serde_json::to_string(&log).expect("preference log serializes");

    assert!(!json.contains("out_dir"));
    assert!(!json.contains("path"));
    assert!(!json.contains("geometry"));
    assert!(!json.contains("mesh"));
    assert!(!json.contains("vertices"));
    assert!(!json.contains("recipe"));
    assert!(!json.contains("C:\\"));
    assert!(!json.contains("/tmp/"));
}

fn preference_log_with_positive_signal(scope: FoundryPreferenceScope) -> FoundryPreferenceLog {
    let mut log = FoundryPreferenceLog::new();
    log.record(FoundryPreferenceEvent::CandidateComparison {
        scope,
        mode: Some("explore".to_owned()),
        accepted_candidate_id: FoundryCandidateId("candidate-a".to_owned()),
        accepted_controls: vec!["body_proportions".to_owned()],
        rejected_candidate_ids: Vec::new(),
        rejected_controls: Vec::new(),
        weight: 1.0,
    });
    log
}
