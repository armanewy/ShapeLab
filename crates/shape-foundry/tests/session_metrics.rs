use shape_foundry::{
    FoundryDocumentId, FoundrySession, FoundrySessionId, FoundryUsabilityEvent,
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
    };

    let json = serde_json::to_string(&session).expect("session serializes");

    assert!(!json.contains("local_usability"));

    let decoded: FoundrySession =
        serde_json::from_str(r#"{"id":"session-1","document_id":"doc-1","candidates":[]}"#)
            .expect("session without metrics remains compatible");

    assert_eq!(decoded.local_usability, None);
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
