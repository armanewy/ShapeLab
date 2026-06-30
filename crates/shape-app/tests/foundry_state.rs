#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryAppStateError, FoundryJobEvent,
    FoundryJobRequest, FoundryPackView, MAKE_JOB_TRACE_DIR, MakeJobTraceEventKind, run_foundry_job,
};
use serde::Serialize;
use shape_asset::{
    ASSET_RECIPE_SCHEMA_VERSION, AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource,
    ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, Transform3, definition_scalar_path,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, BevelPolicy,
    ExaggerationPolicy, FamilyDefaultValue, FamilyParameterKind, FamilyStyleFacet,
    FamilyStylePolicyOverrides, LengthUnit, LengthValue, NormalizedBevelProfile,
    ParameterExecutionPolicy, ParameterRange, PartPrototype, PartRole, ProfileLanguage,
    RepetitionPolicy, RoleMultiplicity, RoleProvision, STYLE_KIT_SCHEMA_VERSION, StyleKit,
    SymmetryPolicy,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, ParameterBinding,
    RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports,
    STYLE_IMPLEMENTATION_SCHEMA_VERSION, ScalarTransform, StyleImplementation,
};
use shape_foundry::{
    CatalogContentRef, ClosedInterval, ControlDivergence, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCatalogError, FoundryCatalogLock, FoundryCatalogResolver,
    FoundryCommand, FoundryConformanceSummary, FoundryDocumentId, FoundryEdit, FoundryLock,
    FoundryLockMode, FoundryLockTarget, FoundryPackDocument, FoundryPackExportProfile,
    FoundryPreferenceEvent, FoundryProjectRevisionProgram, ProviderOverride, ResponseCurve,
    VariationChannel, VariationIntent, document_catalog_refs,
};
use shape_project::foundry::FoundryProjectFile;
use shape_render::foundry::FoundryPreviewCache;
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_draft_plans,
};

#[test]
fn request_build_schedules_compile_job_without_compiling_inline() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state.request_build().expect("build should schedule");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one compile job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompileCurrent { job_id: 1, .. }
    ));
    assert!(state.active_jobs.contains_key(&1));
    assert!(state.current_output.is_none());
}

#[test]
fn semantic_command_schedules_apply_edit_and_stales_superseded_jobs() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    state.request_build().expect("first build should schedule");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.7),
        }))
        .expect("edit should schedule");

    assert!(state.stale_jobs.contains(&1));
    assert!(!state.active_jobs.contains_key(&1));
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one apply-edit job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::ApplyEdit { job_id, edit, .. } => {
            assert_eq!(*job_id, 2);
            assert!(matches!(
                edit.commands.as_slice(),
                [FoundryCommand::SetControl { control_id, .. }] if control_id == "span"
            ));
        }
        other => panic!("expected apply edit job, got {other:?}"),
    }
}

#[test]
fn focus_part_group_applies_without_background_rebuild() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetFocusPartGroup {
            group_id: "body".to_owned(),
        }))
        .expect("focus scope should apply locally");

    assert!(effects.is_empty());
    assert!(state.active_jobs.is_empty());
    assert_eq!(
        state.document.as_ref().and_then(|document| document
            .variation_state
            .intent
            .scope
            .semantic_part_group_id()),
        Some("body")
    );

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::ClearFocusPartGroup))
        .expect("clearing focus should apply locally");

    assert!(effects.is_empty());
    assert!(state.active_jobs.is_empty());
    assert!(
        state
            .document
            .as_ref()
            .and_then(|document| document
                .variation_state
                .intent
                .scope
                .semantic_part_group_id())
            .is_none()
    );
}

#[test]
fn candidate_job_mode_comes_from_explicit_candidate_request() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let request = FoundryCandidateRequest {
        seed: 99,
        proposal_count: 24,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: Some("broad".to_owned()),
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };

    let effects = state
        .request_candidates(request)
        .expect("candidate generation should schedule");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one candidate job effect");
    };
    assert_eq!(job.candidate_mode(), Some(FoundryCandidateMode::Explore));
}

#[test]
fn starting_box_primitive_records_template_build_and_preview_trace() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = FoundryAppState::new(fixture.document.clone()).expect("fixture state");

    let build_request = start_job(state.request_build().expect("build should schedule"));
    let build_event = run_fixture_job(build_request, &fixture);
    assert!(state.handle_job_event(build_event));
    let preview_request = start_job(
        state
            .request_preview(512, 512)
            .expect("preview should schedule"),
    );

    assert!(matches!(
        preview_request,
        FoundryJobRequest::RenderPreview { .. }
    ));
    assert_trace_order(
        &state,
        &[
            MakeJobTraceEventKind::TemplateStarted,
            MakeJobTraceEventKind::BuildQueued,
            MakeJobTraceEventKind::PreviewQueued,
        ],
    );
}

#[test]
fn equivalent_preview_job_is_reused_without_queueing_second_job() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);

    let first = state
        .request_preview(512, 512)
        .expect("first preview should schedule");
    assert!(start_job_optional(first).is_some());
    let second = state
        .request_preview(512, 512)
        .expect("second equivalent preview should be reused");

    assert!(second.is_empty());
    let summary = state.make_job_trace.summary();
    assert_eq!(summary.reused_job_count, 1);
    assert_eq!(summary.coalesced_job_count, 1);
    assert_eq!(summary.duplicate_preview_jobs, 1);
}

#[test]
fn equivalent_candidate_job_is_reused_without_queueing_second_job() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    let request = FoundryCandidateRequest {
        seed: 77,
        proposal_count: 12,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };

    let first = state
        .request_candidates(request.clone())
        .expect("candidate generation should schedule");
    assert!(start_job_optional(first).is_some());
    let second = state
        .request_candidates(request)
        .expect("equivalent candidate generation should be reused");

    assert!(second.is_empty());
    let summary = state.make_job_trace.summary();
    assert_eq!(summary.reused_job_count, 1);
    assert_eq!(summary.coalesced_job_count, 1);
    assert_eq!(summary.duplicate_candidate_jobs, 1);
}

#[test]
fn stale_result_records_local_trace_event() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let accepted = state.handle_job_event(FoundryJobEvent::Failed {
        job_id: 99,
        message: "late job failed".to_owned(),
    });

    assert!(!accepted);
    assert!(state.make_job_trace.events.iter().any(|event| {
        event.event_kind == MakeJobTraceEventKind::JobIgnoredAsStale
            && event.job_id == Some(99)
            && event.ignored_as_stale
    }));
    assert_eq!(
        state.make_job_trace.summary().total_jobs_ignored_as_stale,
        1
    );
}

#[test]
fn candidate_job_records_started_and_finished_events() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    let request = FoundryCandidateRequest {
        seed: 77,
        proposal_count: 12,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };

    let candidate_request = start_job(
        state
            .request_candidates(request)
            .expect("candidate generation should schedule"),
    );
    let candidate_event = run_fixture_job(candidate_request, &fixture);
    let preview_request = match &candidate_event {
        FoundryJobEvent::CandidatesGenerated {
            request, output, ..
        } => {
            assert!(state.handle_job_event(candidate_event.clone()));
            start_job(
                state
                    .request_candidate_previews(request.clone(), output.as_ref().clone())
                    .expect("candidate preview should schedule"),
            )
        }
        other => panic!("expected candidate generation, got {other:?}"),
    };
    let preview_event = run_fixture_job(preview_request, &fixture);
    assert!(state.handle_job_event(preview_event));

    assert!(
        state
            .make_job_trace
            .events
            .iter()
            .any(|event| { event.event_kind == MakeJobTraceEventKind::CandidateStarted })
    );
    assert!(
        state
            .make_job_trace
            .events
            .iter()
            .any(|event| { event.event_kind == MakeJobTraceEventKind::CandidateFinished })
    );
}

#[test]
fn latency_summary_serializes_deterministically() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let state = compiled_fixture_state(&fixture);
    let summary = state.make_job_trace.summary();

    let first = serde_json::to_string_pretty(&summary).expect("summary should serialize");
    let second = serde_json::to_string_pretty(&summary).expect("summary should serialize again");

    assert_eq!(first, second);
    assert!(first.contains("time_to_first_build_started_ms"));
    assert!(first.contains("time_to_first_visible_model_ms"));
    assert!(first.contains("time_to_first_skeleton_idea_tray_ms"));
    assert!(first.contains("time_to_first_candidate_shell_ms"));
    assert!(first.contains("time_to_first_candidate_preview_ms"));
    assert!(first.contains("reused_job_count"));
    assert!(first.contains("coalesced_job_count"));
}

#[test]
fn trace_contains_no_absolute_paths_or_mesh_payloads() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let state = compiled_fixture_state(&fixture);
    let trace_json =
        serde_json::to_string_pretty(&state.make_job_trace.events).expect("trace should serialize");
    let cwd = std::env::current_dir().expect("cwd");
    let cwd = cwd.to_string_lossy();

    assert!(!trace_json.contains(cwd.as_ref()));
    assert!(!trace_json.contains("/Users/"));
    assert!(!trace_json.contains("vertices"));
    assert!(!trace_json.contains("triangles"));
    assert!(!trace_json.contains("rgba8"));
}

#[test]
fn make_job_trace_dogfood_hook_writes_trace_files() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);

    state.set_make_trace_elapsed_ms(120);
    let preview_request = start_job(
        state
            .request_preview(512, 512)
            .expect("preview should schedule"),
    );
    let preview_event = run_fixture_job(preview_request, &fixture);
    state.set_make_trace_elapsed_ms(220);
    assert!(state.handle_job_event(preview_event));

    run_candidate_cycle(
        &mut state,
        &fixture,
        FoundryCandidateRequest {
            seed: 77,
            proposal_count: 12,
            result_count: 4,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::default(),
        },
        CandidateCycleTimes {
            queued_ms: 300,
            compiled_ms: 620,
            render_queued_ms: 640,
            rendered_ms: 940,
        },
    );

    assert!(
        state
            .handle_command(FoundryAppCommand::run(FoundryCommand::SetFocusPartGroup {
                group_id: "body".to_owned(),
            }))
            .expect("focus should apply")
            .is_empty()
    );
    run_candidate_cycle(
        &mut state,
        &fixture,
        FoundryCandidateRequest {
            seed: 78,
            proposal_count: 12,
            result_count: 4,
            mode: FoundryCandidateMode::Refine,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::focus_part_shape("body", "Body"),
        },
        CandidateCycleTimes {
            queued_ms: 1_100,
            compiled_ms: 1_430,
            render_queued_ms: 1_450,
            rendered_ms: 1_780,
        },
    );

    state.set_make_trace_elapsed_ms(1_900);
    let pack_effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
            pack_id: "dogfood-pack".to_owned(),
            member_id: "box-primitive-baseline".to_owned(),
        }))
        .expect("add to pack should schedule");
    let pack_event = run_fixture_job(start_job(pack_effects), &fixture);
    state.set_make_trace_elapsed_ms(2_050);
    assert!(state.handle_job_event(pack_event));

    let summary = state.make_job_trace.summary();
    assert_eq!(summary.time_to_first_visible_model_ms, Some(0));
    assert!(summary.time_to_first_preview_ready_ms.is_some());
    assert_eq!(summary.time_to_first_skeleton_idea_tray_ms, Some(300));
    assert_eq!(summary.time_to_first_candidate_shell_ms, Some(620));
    assert_eq!(summary.time_to_first_candidate_preview_ms, Some(940));
    assert!(summary.time_to_first_selectable_candidate_ms.is_some());
    assert_eq!(summary.total_jobs_ignored_as_stale, 0);
    assert!(!state.make_job_trace.events.iter().any(|event| {
        event.event_kind == MakeJobTraceEventKind::JobIgnoredAsStale
            || event.message.contains("Ignored a background result")
    }));

    let out_dir = workspace_root().join(MAKE_JOB_TRACE_DIR);
    state
        .write_make_job_trace_outputs(&out_dir)
        .expect("trace files should write");

    assert!(out_dir.join("make-job-trace.json").is_file());
    assert!(out_dir.join("make-latency-summary.json").is_file());

    let out_dir = workspace_root().join("target/make-latency-followup-v4");
    state
        .write_make_job_trace_outputs(&out_dir)
        .expect("v4 follow-up trace files should write");

    assert!(out_dir.join("make-job-trace.json").is_file());
    assert!(out_dir.join("make-latency-summary.json").is_file());
}

fn run_candidate_cycle(
    state: &mut FoundryAppState,
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    request: FoundryCandidateRequest,
    times: CandidateCycleTimes,
) {
    state.set_make_trace_elapsed_ms(times.queued_ms);
    let candidate_request = start_job(
        state
            .request_candidates(request)
            .expect("candidate generation should schedule"),
    );
    let candidate_event = run_fixture_job(candidate_request, fixture);
    let preview_request = match &candidate_event {
        FoundryJobEvent::CandidatesGenerated {
            request, output, ..
        } => {
            state.set_make_trace_elapsed_ms(times.compiled_ms);
            assert!(state.handle_job_event(candidate_event.clone()));
            state.set_make_trace_elapsed_ms(times.render_queued_ms);
            start_job(
                state
                    .request_candidate_previews(request.clone(), output.as_ref().clone())
                    .expect("candidate preview should schedule"),
            )
        }
        other => panic!("expected candidate generation, got {other:?}"),
    };
    let preview_event = run_fixture_job(preview_request, fixture);
    state.set_make_trace_elapsed_ms(times.rendered_ms);
    assert!(state.handle_job_event(preview_event));
}

#[derive(Debug, Copy, Clone)]
struct CandidateCycleTimes {
    queued_ms: u64,
    compiled_ms: u64,
    render_queued_ms: u64,
    rendered_ms: u64,
}

#[test]
fn focused_candidate_command_sets_part_intent() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let effects = state
        .handle_command(FoundryAppCommand::run(
            FoundryCommand::GenerateFocusedPartCandidates {
                group_id: "body".to_owned(),
                channels: vec![VariationChannel::Shape],
                mode: "refine".to_owned(),
            },
        ))
        .expect("focused command should schedule");
    let job = start_job(effects);
    let FoundryJobRequest::GenerateCandidates { request, .. } = job else {
        panic!("expected candidate job");
    };
    assert_eq!(
        request.variation_intent.scope.semantic_part_group_id(),
        Some("body")
    );
    assert_eq!(
        request.variation_intent.channels,
        vec![VariationChannel::Shape]
    );
}

#[test]
fn make_job_trace_dogfood_hook_candidate_compile_only_path_remains_supported() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    let request = FoundryCandidateRequest {
        seed: 77,
        proposal_count: 12,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };
    let candidate_request = start_job(
        state
            .request_candidates(request)
            .expect("candidate generation should schedule"),
    );
    let candidate_event = run_fixture_job(candidate_request, &fixture);
    assert!(state.handle_job_event(candidate_event));

    assert!(
        state
            .make_job_trace
            .events
            .iter()
            .any(|event| event.event_kind == MakeJobTraceEventKind::CandidateCompiled)
    );
}

#[test]
fn direction_mode_command_preserves_requested_search_mode() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let effects = state
        .handle_command(FoundryAppCommand::RequestCandidates(
            FoundryCandidateRequest {
                seed: 12,
                proposal_count: 24,
                result_count: 6,
                mode: FoundryCandidateMode::Structure,
                strategy_id: Some("macro".to_owned()),
                preference_profile: None,
                variation_intent: VariationIntent::default(),
            },
        ))
        .expect("mode request should schedule");

    let job = start_job(effects);
    assert_eq!(job.candidate_mode(), Some(FoundryCandidateMode::Structure));
}

#[test]
fn transient_control_preview_does_not_schedule_persistent_edit() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let effects = state
        .handle_command(FoundryAppCommand::PreviewControlValue {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.25),
        })
        .expect("preview should schedule");

    let job = start_job(effects);
    match job {
        FoundryJobRequest::PreviewControlValue {
            job_id,
            control_id,
            value,
            ..
        } => {
            assert_eq!(job_id, 1);
            assert_eq!(control_id, "span");
            assert_eq!(value, ControlValue::Scalar(0.25));
        }
        other => panic!("expected transient preview job, got {other:?}"),
    }
    assert!(
        state
            .document
            .as_ref()
            .is_some_and(|document| !document.control_state.contains_key("span"))
    );
    assert!(
        state
            .project_file
            .as_ref()
            .is_some_and(|project| { project.project.revisions.len() == 1 })
    );
}

#[test]
fn accepting_candidate_schedules_its_replayable_foundry_edit() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let candidate_id = shape_foundry::FoundryCandidateId("candidate-a".to_owned());
    let edit = FoundryEdit {
        label: "Candidate A".to_owned(),
        commands: vec![FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.25),
        }],
    };
    state
        .candidate_edits
        .insert(candidate_id.clone(), edit.clone());

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AcceptCandidate {
            candidate_id,
        }))
        .expect("candidate should schedule edit application");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one apply-edit job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::ApplyEdit {
            edit: scheduled, ..
        } => {
            assert_eq!(scheduled.as_ref(), &edit);
        }
        other => panic!("expected apply edit job, got {other:?}"),
    }
}

#[test]
fn local_preferences_record_visible_lock_and_reset_actions() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "edge_softness".to_owned(),
            value: ControlValue::Scalar(0.4),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetLock {
            lock: FoundryLock {
                target: FoundryLockTarget::Control("proportions".to_owned()),
                mode: FoundryLockMode::SearchProtected,
                reason: Some("test".to_owned()),
            },
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::ResetControl {
            control_id: "edge_softness".to_owned(),
        },
    );

    assert!(state.local_preferences.events.iter().any(|event| {
        matches!(
            event,
            FoundryPreferenceEvent::ControlLocked { control_id, .. }
                if control_id == "proportions"
        )
    }));
    assert!(state.local_preferences.events.iter().any(|event| {
        matches!(
            event,
            FoundryPreferenceEvent::ControlReset { control_id, .. }
                if control_id == "edge_softness"
        )
    }));
}

#[test]
fn equivalent_build_job_is_reused_without_queueing_second_job() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let first = state.request_build().expect("first build should schedule");
    assert!(start_job_optional(first).is_some());

    let second = state
        .request_build()
        .expect("second equivalent build should be reused");

    assert!(second.is_empty());
    assert!(state.active_jobs.contains_key(&1));
    assert!(!state.active_jobs.contains_key(&2));
    assert!(state.make_job_trace.events.iter().any(|event| {
        event.event_kind == MakeJobTraceEventKind::JobReused
            && event.job_slot.as_deref() == Some("CompileCurrent")
    }));
    let summary = state.make_job_trace.summary();
    assert_eq!(summary.reused_job_count, 1);
    assert_eq!(summary.coalesced_job_count, 1);
    assert_eq!(summary.duplicate_build_jobs, 1);
}

#[test]
fn replacing_project_preserves_monotonic_job_ids_and_rejects_old_events() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    state
        .request_build()
        .expect("old project build should schedule");
    assert!(state.active_jobs.contains_key(&1));

    let mut loaded_document = minimal_foundry_document();
    loaded_document.document_id = FoundryDocumentId("loaded-doc".to_owned());
    let loaded_project = FoundryProjectFile::new(
        "Loaded",
        loaded_document.clone(),
        FoundryCatalogLock::from_document_refs(&loaded_document),
        None,
        None,
        FoundryConformanceSummary::default(),
    )
    .expect("loaded project should be valid");

    let effects = state
        .replace_loaded_project(loaded_project)
        .expect("project replacement should schedule a build");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one compile job effect for loaded project");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompileCurrent { job_id: 2, document }
            if document.document_id.0 == "loaded-doc"
    ));
    assert!(state.stale_jobs.contains(&1));
    assert!(!state.active_jobs.contains_key(&1));
    assert!(state.active_jobs.contains_key(&2));

    let accepted = state.handle_job_event(FoundryJobEvent::Failed {
        job_id: 1,
        message: "old project job failed late".to_owned(),
    });

    assert!(!accepted);
    assert_eq!(
        state.status.as_deref(),
        Some("Ignored a background result because newer work is active.")
    );
    assert!(state.active_jobs.contains_key(&2));
    assert_eq!(
        state
            .document
            .as_ref()
            .map(|document| document.document_id.0.as_str()),
        Some("loaded-doc")
    );
}

#[test]
fn build_request_while_edit_runs_is_blocked_and_traced() {
    let fixture = RuntimeFixture::new();
    let mut state = FoundryAppState::new(fixture.document.clone()).expect("valid state");
    let edit_effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "radius".to_owned(),
            value: ControlValue::Scalar(0.2),
        }))
        .expect("edit should schedule");
    let edit_request = start_job(edit_effects);

    let blocked_build_effects = state
        .request_build()
        .expect("old-source build should not queue while edit runs");
    assert!(blocked_build_effects.is_empty());
    assert!(state.active_jobs.contains_key(&1));
    assert!(state.make_job_trace.events.iter().any(|event| {
        event.event_kind == MakeJobTraceEventKind::UserActionBlocked
            && event.message.contains("Build request blocked")
    }));

    let edit_event = run_foundry_job(
        edit_request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    assert!(state.handle_job_event(edit_event));

    assert_eq!(
        state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("radius")),
        Some(&ControlValue::Scalar(0.2))
    );
}

#[test]
fn set_style_apply_edit_prunes_incompatible_provider_override() {
    let mut fixture = RuntimeFixture::new();
    fixture.document.provider_overrides.insert(
        "body".to_owned(),
        ProviderOverride {
            role: "body".to_owned(),
            provider_ref: provider_ref("heavy_body"),
        },
    );
    fixture.document.catalog_lock = Some(FoundryCatalogLock::from_document_refs(&fixture.document));

    let request = FoundryJobRequest::ApplyEdit {
        job_id: 1,
        document: Box::new(fixture.document.clone()),
        edit: Box::new(FoundryEdit {
            label: "Soft style".to_owned(),
            commands: vec![FoundryCommand::SetStyle {
                style_content_ref: fixture.soft_style_ref.clone(),
                style_implementation_ref: fixture.soft_style_impl_ref.clone(),
            }],
        }),
    };

    let event = run_foundry_job(
        request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    let FoundryJobEvent::EditApplied { output, .. } = event else {
        panic!("style edit should compile after pruning, got {event:?}");
    };

    assert_eq!(output.document.style_content_ref, fixture.soft_style_ref);
    assert_eq!(
        output.document.style_implementation_ref,
        fixture.soft_style_impl_ref
    );
    assert!(output.document.provider_overrides.is_empty());
}

#[test]
fn candidate_generation_job_returns_pending_cards_before_preview_rendering() {
    let fixture = RuntimeFixture::new();
    let request = FoundryJobRequest::GenerateCandidates {
        job_id: 1,
        document: Box::new(fixture.document.clone()),
        request: FoundryCandidateRequest {
            seed: 33,
            proposal_count: 12,
            result_count: 4,
            mode: FoundryCandidateMode::Refine,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::default(),
        },
    };

    let event = run_foundry_job(
        request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    let FoundryJobEvent::CandidatesGenerated { cards, .. } = event else {
        panic!("candidate generation should complete, got {event:?}");
    };

    assert!(!cards.is_empty());
    assert!(cards.iter().all(|card| {
        card.width == 0
            && card.height == 0
            && card.rgba8.is_empty()
            && card.camera.is_none()
            && !card.selectable
            && card
                .preview_failure
                .as_deref()
                .is_some_and(|reason| reason.contains("Preview rendering"))
    }));
}

#[test]
fn candidate_preview_job_renders_preview_images_for_cards() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let request = FoundryCandidateRequest {
        seed: 33,
        proposal_count: 12,
        result_count: 4,
        mode: FoundryCandidateMode::Refine,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };
    let output = generate_foundry_candidate_draft_plans(&fixture.document, &fixture, &request)
        .expect("candidate generation should succeed");
    let output_count = output.candidates.len();
    let preview_request = FoundryJobRequest::RenderCandidatePreviews {
        job_id: 2,
        document: Box::new(fixture.document.clone()),
        request: request.clone(),
        output: Box::new(output),
    };

    let event = run_foundry_job(
        preview_request,
        &fixture,
        &mut FoundryPreviewCache::default(),
    );
    let FoundryJobEvent::CandidatePreviewsRendered {
        cards,
        rejected_count,
        ..
    } = event
    else {
        panic!("candidate preview rendering should complete, got {event:?}");
    };

    assert!(!cards.is_empty());
    assert!(cards.len() <= output_count);
    assert_eq!(cards.len() + rejected_count, output_count);
    assert!(cards.iter().all(|card| {
        card.width > 0
            && card.height > 0
            && card.rgba8.len() == (card.width * card.height * 4) as usize
            && card.camera.is_some()
            && card.preview_failure.is_none()
            && card.selectable
    }));
}

#[test]
fn candidate_preview_failures_are_isolated_to_their_cards() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let request = FoundryCandidateRequest {
        seed: 51,
        proposal_count: 12,
        result_count: 4,
        mode: FoundryCandidateMode::Refine,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };
    let mut output = generate_foundry_candidate_draft_plans(&fixture.document, &fixture, &request)
        .expect("candidate generation should succeed");
    assert!(
        output.candidates.len() > 1,
        "fixture should return multiple candidates for isolation coverage"
    );
    output.candidates[0].document.family_content_ref = content_ref("missing-family", 91);

    let cards = foundry::jobs::candidate_cards_from_output_with_previews(
        &fixture.document,
        &output,
        Some(request.mode),
        None,
        &fixture,
        &mut FoundryPreviewCache::default(),
    )
    .expect("preview failures should not fail candidate card generation");

    assert!(cards.len() <= output.candidates.len());
    assert!(cards.iter().any(|card| {
        card.preview_failure
            .as_deref()
            .is_some_and(|reason| reason.contains("Preview unavailable"))
            && card.width == 0
            && card.rgba8.is_empty()
            && !card.selectable
    }));
    assert!(cards.iter().any(|card| {
        card.preview_failure.is_none()
            && card.width > 0
            && card.height > 0
            && card.rgba8.len() == (card.width * card.height * 4) as usize
    }));
}

#[test]
fn option_cards_render_distinct_whole_model_thumbnails() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let state = compiled_fixture_state(&fixture);
    let proportions = state
        .controls
        .iter()
        .find(|control| control.id == "proportions")
        .expect("proportions control");

    assert!(proportions.options.len() >= 3);
    assert!(proportions.options.iter().all(|option| {
        option.preview_id.is_some()
            && option.width == 64
            && option.height == 64
            && option.rgba8.len() == (option.width * option.height * 4) as usize
            && option.camera.is_some()
    }));
    assert!(
        proportions
            .options
            .windows(2)
            .any(|pair| pair[0].rgba8 != pair[1].rgba8),
        "whole-model option thumbnails should reflect option-applied geometry"
    );
}

#[test]
#[ignore = "explicit Wave 30 release gate; renders option thumbnails for all built-in profiles"]
fn release_gate_all_builtin_profiles_render_real_option_thumbnails() {
    for fixture in shape_foundry_catalog::headless_fixture_catalogs() {
        let state = compiled_fixture_state(&fixture);
        assert_default_foundry_surface(&state);

        let primary_controls = state
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .collect::<Vec<_>>();
        assert_eq!(
            primary_controls.len(),
            7,
            "{} should expose seven novice-facing controls",
            fixture.slug
        );

        let mut previewed_options = 0;
        let mut controls_with_options = 0;
        for control in primary_controls {
            if control.options.is_empty() {
                continue;
            }
            controls_with_options += 1;

            for option in &control.options {
                assert!(
                    option.preview_id.is_some(),
                    "{} {} option {} should carry a stable preview id",
                    fixture.slug,
                    control.id,
                    option.label
                );
                assert_eq!(
                    (option.width, option.height),
                    (64, 64),
                    "{} {} option {} should be a rendered whole-model thumbnail, not a placeholder",
                    fixture.slug,
                    control.id,
                    option.label
                );
                assert_eq!(
                    option.rgba8.len(),
                    (option.width * option.height * 4) as usize,
                    "{} {} option {} should carry complete RGBA bytes",
                    fixture.slug,
                    control.id,
                    option.label
                );
                assert!(
                    option.camera.is_some(),
                    "{} {} option {} should record the preview camera",
                    fixture.slug,
                    control.id,
                    option.label
                );
                previewed_options += 1;
            }

            assert!(
                control
                    .options
                    .windows(2)
                    .any(|pair| pair[0].rgba8 != pair[1].rgba8),
                "{} {} should render visible whole-model differences across its option thumbnails",
                fixture.slug,
                control.id
            );
        }

        assert!(
            controls_with_options > 0,
            "{} should expose at least one option-bearing default control",
            fixture.slug
        );
        assert!(
            previewed_options > 0,
            "{} should expose at least one option thumbnail in the default product path",
            fixture.slug
        );
    }
}

#[test]
fn novice_can_create_reinforced_box_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "proportions".to_owned(),
            value: ControlValue::Choice("wide_box".to_owned()),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "edge_softness".to_owned(),
            value: ControlValue::Scalar(0.92),
        },
    );

    let document = state.document.as_ref().expect("current box document");
    assert_eq!(
        document.control_state.get("edge_softness"),
        Some(&ControlValue::Scalar(0.92))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_create_compact_box_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "proportions".to_owned(),
            value: ControlValue::Choice("compact_box".to_owned()),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "edge_softness".to_owned(),
            value: ControlValue::Scalar(0.18),
        },
    );

    let document = state.document.as_ref().expect("current box document");
    assert_eq!(
        document.control_state.get("proportions"),
        Some(&ControlValue::Choice("compact_box".to_owned()))
    );
    assert_eq!(
        document.control_state.get("edge_softness"),
        Some(&ControlValue::Scalar(0.18))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_create_tall_box_with_soft_edges_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "proportions".to_owned(),
            value: ControlValue::Choice("tall_box".to_owned()),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "edge_softness".to_owned(),
            value: ControlValue::Scalar(0.72),
        },
    );

    let document = state.document.as_ref().expect("current box document");
    assert_eq!(
        document.control_state.get("proportions"),
        Some(&ControlValue::Choice("tall_box".to_owned()))
    );
    assert_eq!(
        document.control_state.get("edge_softness"),
        Some(&ControlValue::Scalar(0.72))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_export_three_member_pack_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    add_current_to_fixture_pack(&mut state, &fixture, "balanced");

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "proportions".to_owned(),
            value: ControlValue::Choice("flat_box".to_owned()),
        },
    );
    add_current_to_fixture_pack(&mut state, &fixture, "light");

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "proportions".to_owned(),
            value: ControlValue::Choice("tall_box".to_owned()),
        },
    );
    add_current_to_fixture_pack(&mut state, &fixture, "reinforced");

    assert_eq!(state.pack.members.len(), 3);
    assert!(state.pack.can_export);
    assert_eq!(
        state
            .local_preferences
            .events
            .iter()
            .filter(|event| matches!(event, FoundryPreferenceEvent::PackMemberAdded { .. }))
            .count(),
        3
    );

    let out_dir = temp_test_dir("foundry-wave22-pack");
    let _ = fs::remove_dir_all(&out_dir);
    let effects = state
        .handle_command(FoundryAppCommand::RequestPackBatchExport {
            out_dir: out_dir.clone(),
        })
        .expect("pack export should schedule");
    let event = run_fixture_job(start_job(effects), &fixture);
    let FoundryJobEvent::PackExportFinished { member_count, .. } = event.clone() else {
        panic!("expected pack export, got {event:?}");
    };
    assert_eq!(member_count, 3);
    assert!(state.handle_job_event(event));
    for member in ["balanced", "light", "reinforced"] {
        assert!(
            out_dir.join(member).join("asset-manifest.json").is_file(),
            "missing exported pack member {member}"
        );
    }

    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn novice_can_reject_bad_candidate_and_branch_to_another_direction() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    let effects = state
        .request_candidates(FoundryCandidateRequest {
            seed: 77,
            proposal_count: 12,
            result_count: 4,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::default(),
        })
        .expect("candidate generation should schedule");
    let event = run_fixture_job(start_job(effects), &fixture);
    assert!(state.handle_job_event(event));
    assert!(
        state.candidates.len() > 1,
        "fixture should produce alternatives for reject/branch flow"
    );

    let rejected = state.candidates[0].id.clone();
    let accepted = state.candidates[1].id.clone();
    state.read_only = true;
    let blocked = state.handle_command(FoundryAppCommand::run(FoundryCommand::AcceptCandidate {
        candidate_id: accepted.clone(),
    }));
    state.read_only = false;
    assert!(matches!(blocked, Err(FoundryAppStateError::ReadOnly)));
    assert!(state.local_preferences.events.is_empty());

    assert!(
        state
            .handle_command(FoundryAppCommand::run(FoundryCommand::RejectCandidate {
                candidate_id: rejected.clone()
            }))
            .expect("reject should not schedule a job")
            .is_empty()
    );
    assert!(!state.candidate_edits.contains_key(&rejected));
    assert_eq!(state.local_preferences.events.len(), 1);

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AcceptCandidate {
            candidate_id: accepted.clone(),
        }))
        .expect("accept should schedule candidate edit");
    assert_eq!(state.local_preferences.events.len(), 2);
    let event = run_fixture_job(start_job(effects), &fixture);
    assert!(matches!(event, FoundryJobEvent::EditApplied { .. }));
    assert!(state.handle_job_event(event));

    assert_current_mesh_valid(&state);
    assert!(state.selected_candidate.is_none());
    assert!(
        state
            .project_file
            .as_ref()
            .is_some_and(|project| project.project.revisions.len() > 1)
    );

    let effects = state
        .request_candidates(FoundryCandidateRequest {
            seed: 78,
            proposal_count: 12,
            result_count: 4,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::default(),
        })
        .expect("preference-biased generation should schedule");
    let job = start_job(effects);
    let FoundryJobRequest::GenerateCandidates { request, .. } = &job else {
        panic!("expected candidate generation request");
    };
    let profile = request
        .preference_profile
        .as_ref()
        .expect("local preferences should attach a profile");
    assert_eq!(profile.source_event_count, 2);
    assert!(profile.local_only);
}

#[test]
fn pack_compile_preserves_selected_member_when_it_still_exists() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let mut pack = FoundryPackDocument::new(
        "props".to_owned(),
        content_ref("family", 1),
        content_ref("style", 2),
        FoundryPackExportProfile {
            profile: "default".to_owned(),
            require_all_members: true,
        },
    );
    let mut box_document = minimal_foundry_document();
    box_document.document_id = FoundryDocumentId("box".to_owned());
    let mut barrel_document = minimal_foundry_document();
    barrel_document.document_id = FoundryDocumentId("barrel".to_owned());
    pack.members.insert("barrel".to_owned(), barrel_document);
    pack.members.insert("box".to_owned(), box_document);
    state.pack = FoundryPackView {
        pack_id: Some("props".to_owned()),
        members: pack
            .members
            .iter()
            .map(|(member_id, document)| (member_id.clone(), document.document_id.clone()))
            .collect(),
        selected_member: Some("box".to_owned()),
        pack: Some(pack.clone()),
        ..FoundryPackView::default()
    };
    state.selected_pack_member = Some("box".to_owned());
    state.active_jobs.insert(
        1,
        FoundryJobRequest::CompilePack {
            job_id: 1,
            pack: Box::new(pack.clone()),
        },
    );

    let accepted = state.handle_job_event(FoundryJobEvent::PackCompiled {
        job_id: 1,
        pack: Box::new(FoundryPackView {
            pack_id: Some("props".to_owned()),
            members: state.pack.members.clone(),
            selected_member: Some("barrel".to_owned()),
            pack: Some(pack),
            ..FoundryPackView::default()
        }),
    });

    assert!(accepted);
    assert_eq!(state.selected_pack_member.as_deref(), Some("box"));
    assert_eq!(state.pack.selected_member.as_deref(), Some("box"));
}

#[test]
fn add_current_to_pack_tracks_membership_and_schedules_pack_compile() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
            pack_id: "props".to_owned(),
            member_id: "box".to_owned(),
        }))
        .expect("pack membership should schedule compile");

    assert_eq!(state.pack.pack_id.as_deref(), Some("props"));
    assert!(state.pack.members.contains_key("box"));
    assert_eq!(
        state
            .pack
            .members
            .get("box")
            .map(|document_id| document_id.0.as_str()),
        Some("box")
    );
    assert_eq!(state.selected_pack_member.as_deref(), Some("box"));
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one pack compile job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::CompilePack { job_id, pack } => {
            assert_eq!(*job_id, 1);
            assert_eq!(
                pack.members
                    .get("box")
                    .map(|document| document.document_id.0.as_str()),
                Some("box")
            );
        }
        other => panic!("expected pack compile job effect, got {other:?}"),
    }
}

#[test]
fn batch_export_command_registers_pack_export_job_in_reducer() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let out_dir = PathBuf::from("pack-export");
    let pack = FoundryPackDocument::new(
        "props".to_owned(),
        content_ref("family", 1),
        content_ref("style", 2),
        FoundryPackExportProfile {
            profile: "default".to_owned(),
            require_all_members: true,
        },
    );
    state.pack = FoundryPackView {
        pack_id: Some("props".to_owned()),
        pack: Some(pack),
        can_export: true,
        coherent: true,
        ..FoundryPackView::default()
    };

    let effects = state
        .handle_command(FoundryAppCommand::RequestPackBatchExport {
            out_dir: out_dir.clone(),
        })
        .expect("batch export should schedule pack export");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one pack export job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::ExportPack {
            job_id: 1,
            out_dir: actual,
            ..
        } if actual == &out_dir
    ));
    assert!(state.active_jobs.contains_key(&1));
}

#[test]
fn pack_export_job_writes_member_packages() {
    let fixture = RuntimeFixture::new();
    let mut pack = FoundryPackDocument::new(
        "props".to_owned(),
        fixture.document.family_content_ref.clone(),
        fixture.document.style_content_ref.clone(),
        FoundryPackExportProfile {
            profile: "default".to_owned(),
            require_all_members: true,
        },
    );
    pack.members
        .insert("box_a".to_owned(), fixture.document.clone());
    let mut second = fixture.document.clone();
    second.document_id = FoundryDocumentId("box_b".to_owned());
    second
        .control_state
        .insert("radius".to_owned(), ControlValue::Scalar(0.2));
    pack.members.insert("box_b".to_owned(), second);
    let out_dir = temp_test_dir("foundry-pack-export");
    let _ = fs::remove_dir_all(&out_dir);

    let event = run_foundry_job(
        FoundryJobRequest::ExportPack {
            job_id: 1,
            pack: Box::new(pack),
            out_dir: out_dir.clone(),
        },
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );

    let FoundryJobEvent::PackExportFinished {
        out_dir: actual_dir,
        member_count,
        ..
    } = event
    else {
        panic!("expected pack export success, got {event:?}");
    };
    assert_eq!(actual_dir, out_dir);
    assert_eq!(member_count, 2);
    assert!(out_dir.join("box_a").join("asset-manifest.json").is_file());
    assert!(out_dir.join("box_b").join("asset-manifest.json").is_file());

    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn current_asset_export_command_writes_model_package() {
    let fixture = RuntimeFixture::new();
    let mut state = FoundryAppState::new(fixture.document.clone()).expect("valid state");
    let compile_request = start_job(state.request_build().expect("build should schedule"));
    let compile_event = run_foundry_job(
        compile_request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    assert!(matches!(
        compile_event,
        FoundryJobEvent::CompileFinished { .. }
    ));
    assert!(state.handle_job_event(compile_event));

    let out_dir = temp_test_dir("foundry-current-export");
    let _ = fs::remove_dir_all(&out_dir);
    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::Export {
            profile: "default".to_owned(),
            out_dir: Some(out_dir.to_string_lossy().to_string()),
        }))
        .expect("export should schedule");
    let export_request = start_job(effects);
    assert!(matches!(
        export_request,
        FoundryJobRequest::Export {
            job_id: 2,
            out_dir: ref actual_dir,
            ..
        } if actual_dir == &out_dir
    ));

    let event = run_foundry_job(
        export_request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    let FoundryJobEvent::ExportFinished {
        out_dir: actual_dir,
        ..
    } = event.clone()
    else {
        panic!("expected export success, got {event:?}");
    };
    assert_eq!(actual_dir, out_dir);
    assert!(state.handle_job_event(event));
    assert!(
        state
            .local_preferences
            .events
            .iter()
            .any(|event| { matches!(event, FoundryPreferenceEvent::VariantExported { .. }) })
    );
    assert!(out_dir.join("asset-manifest.json").is_file());

    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn undo_switches_to_parent_revision_and_rebuilds_off_thread() {
    let root = minimal_foundry_document();
    let edit = FoundryEdit {
        label: "Set span".to_owned(),
        commands: vec![FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.5),
        }],
    };
    let mut child = root.clone();
    child
        .control_state
        .insert("span".to_owned(), ControlValue::Scalar(0.5));

    let mut project_file = FoundryProjectFile::new(
        "History",
        root.clone(),
        FoundryCatalogLock::from_document_refs(&root),
        None,
        None,
        FoundryConformanceSummary::default(),
    )
    .expect("project should be valid");
    project_file
        .accept_program(
            FoundryProjectRevisionProgram::from_edit(edit),
            child,
            FoundryCatalogLock::from_document_refs(&root),
            None,
            None,
            FoundryConformanceSummary::default(),
        )
        .expect("child revision should be accepted");
    let mut state = FoundryAppState::from_project_file(project_file).expect("state should load");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::Undo))
        .expect("undo should schedule rebuild");

    assert_eq!(state.current_revision, Some(shape_asset::RevisionId(0)));
    assert!(
        state
            .document
            .as_ref()
            .is_some_and(|document| !document.control_state.contains_key("span"))
    );
    assert!(state.current_output.is_none());
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one compile job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompileCurrent { job_id: 1, .. }
    ));
}

fn start_job(effects: Vec<FoundryAppEffect>) -> FoundryJobRequest {
    effects
        .into_iter()
        .find_map(|effect| match effect {
            FoundryAppEffect::StartJob(job) => Some(*job),
            _ => None,
        })
        .expect("start job effect")
}

fn start_job_optional(effects: Vec<FoundryAppEffect>) -> Option<FoundryJobRequest> {
    effects.into_iter().find_map(|effect| match effect {
        FoundryAppEffect::StartJob(job) => Some(*job),
        _ => None,
    })
}

fn assert_trace_order(state: &FoundryAppState, expected: &[MakeJobTraceEventKind]) {
    let mut cursor = 0;
    for event in &state.make_job_trace.events {
        if event.event_kind == expected[cursor] {
            cursor += 1;
            if cursor == expected.len() {
                return;
            }
        }
    }
    panic!(
        "trace did not contain expected ordered events {:?}; trace was {:?}",
        expected,
        state
            .make_job_trace
            .events
            .iter()
            .map(|event| event.event_kind)
            .collect::<Vec<_>>()
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("shape-app lives two levels below the workspace")
        .to_path_buf()
}

fn compiled_fixture_state(
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
) -> FoundryAppState {
    let mut state = FoundryAppState::new(fixture.document.clone()).expect("fixture state");
    let request = start_job(state.request_build().expect("build should schedule"));
    let event = run_fixture_job(request, fixture);
    assert!(matches!(event, FoundryJobEvent::CompileFinished { .. }));
    assert!(state.handle_job_event(event));
    assert!(state.current_output.is_some());
    assert!(!state.controls.is_empty());
    state
}

fn apply_fixture_command(
    state: &mut FoundryAppState,
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    command: FoundryCommand,
) {
    let effects = state
        .handle_command(FoundryAppCommand::run(command))
        .expect("command should schedule an edit");
    let event = run_fixture_job(start_job(effects), fixture);
    assert!(matches!(event, FoundryJobEvent::EditApplied { .. }));
    assert!(state.handle_job_event(event));
}

fn add_current_to_fixture_pack(
    state: &mut FoundryAppState,
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
    member_id: &str,
) {
    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
            pack_id: "novice_pack".to_owned(),
            member_id: member_id.to_owned(),
        }))
        .expect("add current to pack should schedule compilation");
    let event = run_fixture_job(start_job(effects), fixture);
    assert!(
        matches!(event, FoundryJobEvent::PackCompiled { .. }),
        "expected pack compilation, got {event:?}"
    );
    assert!(state.handle_job_event(event));
}

fn run_fixture_job(
    request: FoundryJobRequest,
    fixture: &shape_foundry_catalog::FoundryFixtureCatalog,
) -> FoundryJobEvent {
    run_foundry_job(request, fixture, &mut FoundryPreviewCache::default())
}

fn assert_default_foundry_surface(state: &FoundryAppState) {
    assert!(!state.advanced_recipe_open);
    assert!(
        state
            .controls
            .iter()
            .any(|control| control.primary && control.visible)
    );
    assert!(
        state
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .all(|control| {
                !control.label.contains("controls.")
                    && !control.kind.contains("controls.")
                    && !control.id.contains("recipe")
            })
    );
}

fn assert_current_mesh_valid(state: &FoundryAppState) {
    let output = state.current_output.as_ref().expect("current output");
    let mesh = &output.artifact.combined_preview.mesh;
    assert!(!mesh.positions.is_empty());
    assert!(!mesh.indices.is_empty());
    assert_eq!(mesh.indices.len() % 3, 0);
    assert!(mesh.bounds.min.iter().all(|value| value.is_finite()));
    assert!(mesh.bounds.max.iter().all(|value| value.is_finite()));
}

fn temp_test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}-{}", std::process::id()))
}

#[derive(Clone, Default)]
struct TestCatalog {
    entries: BTreeMap<String, String>,
}

impl FoundryCatalogResolver for TestCatalog {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.entries
            .get(&content_ref.stable_id)
            .cloned()
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

struct RuntimeFixture {
    document: FoundryAssetDocument,
    catalog: TestCatalog,
    soft_style_ref: CatalogContentRef,
    soft_style_impl_ref: CatalogContentRef,
}

impl RuntimeFixture {
    fn new() -> Self {
        let family = family_schema();
        let plain_style = style_kit("plain", "Plain", "box", &["plain_body", "heavy_body"]);
        let soft_style = style_kit("soft", "Soft", "box", &["soft_body"]);
        let family_impl = family_implementation();
        let plain_style_impl = style_implementation(
            "plain",
            "box",
            "plain_body",
            vec![
                provider_fragment("plain_body", 0.1),
                provider_fragment("heavy_body", 0.2),
            ],
        );
        let soft_style_impl = style_implementation(
            "soft",
            "box",
            "soft_body",
            vec![provider_fragment("soft_body", 0.12)],
        );
        let profile = customizer_profile();

        let (family_ref, family_json) =
            catalog_entry("box-family", ASSET_FAMILY_SCHEMA_VERSION, &family);
        let (plain_style_ref, plain_style_json) =
            catalog_entry("plain-style", STYLE_KIT_SCHEMA_VERSION, &plain_style);
        let (soft_style_ref, soft_style_json) =
            catalog_entry("soft-style", STYLE_KIT_SCHEMA_VERSION, &soft_style);
        let (family_impl_ref, family_impl_json) = catalog_entry(
            "box-family-impl",
            FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
            &family_impl,
        );
        let (plain_style_impl_ref, plain_style_impl_json) = catalog_entry(
            "plain-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &plain_style_impl,
        );
        let (soft_style_impl_ref, soft_style_impl_json) = catalog_entry(
            "soft-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &soft_style_impl,
        );
        let (profile_ref, profile_json) = catalog_entry(
            "box-profile",
            shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            &profile,
        );

        let mut document = FoundryAssetDocument {
            schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
            document_id: FoundryDocumentId("doc-runtime".to_owned()),
            family_content_ref: family_ref,
            style_content_ref: plain_style_ref,
            family_implementation_ref: family_impl_ref,
            style_implementation_ref: plain_style_impl_ref,
            customizer_profile_ref: profile_ref,
            control_state: BTreeMap::from([("radius".to_owned(), ControlValue::Scalar(0.15))]),
            provider_overrides: BTreeMap::new(),
            foundry_locks: Vec::new(),
            variation_state: shape_foundry::FoundryVariationState::default(),
            local_recipe_overrides: Vec::new(),
            seed: 42,
            catalog_lock: None,
            build_stamp: None,
        };
        document.catalog_lock = Some(FoundryCatalogLock {
            exact_refs: document_catalog_refs(&document),
            embedded_snapshots: Vec::new(),
            compiler_version: "0.1.0".to_owned(),
            catalog_version: 1,
        });

        let catalog = TestCatalog {
            entries: BTreeMap::from([
                ("box-family".to_owned(), family_json),
                ("plain-style".to_owned(), plain_style_json),
                ("soft-style".to_owned(), soft_style_json),
                ("box-family-impl".to_owned(), family_impl_json),
                ("plain-style-impl".to_owned(), plain_style_impl_json),
                ("soft-style-impl".to_owned(), soft_style_impl_json),
                ("box-profile".to_owned(), profile_json),
            ]),
        };

        Self {
            document,
            catalog,
            soft_style_ref,
            soft_style_impl_ref,
        }
    }
}

fn family_schema() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "box".to_owned(),
        display_name: "Box".to_owned(),
        summary: "Runtime test box family".to_owned(),
        part_roles: vec![PartRole {
            id: "body".to_owned(),
            display_name: "Body".to_owned(),
            required: true,
            multiplicity: RoleMultiplicity::Single,
            provision: RoleProvision::StyleRequired,
            semantic_tags: vec!["body".to_owned()],
        }],
        attachment_rules: Vec::new(),
        allowed_operations: vec![AllowedOperationKind::Primitive],
        parameter_slots: vec![shape_family::FamilyParameterSlot {
            id: "radius".to_owned(),
            label: "Radius".to_owned(),
            target_role: Some("body".to_owned()),
            kind: FamilyParameterKind::Length {
                unit: LengthUnit::FamilyUnits,
            },
            range: Some(ParameterRange {
                minimum: 0.01,
                maximum: 0.5,
                step: 0.01,
            }),
            default_value: Some(FamilyDefaultValue::Scalar(0.1)),
            execution_policy: ParameterExecutionPolicy::RequiredBinding,
            topology_changing: false,
        }],
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: vec!["plain".to_owned(), "soft".to_owned()],
        tags: Vec::new(),
    }
}

fn style_kit(id: &str, label: &str, family_id: &str, prototypes: &[&str]) -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: label.to_owned(),
        compatible_families: vec![family_id.to_owned()],
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(0.05),
            segments: 2,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "rounded".to_owned(),
            allowed_profiles: vec!["soft".to_owned()],
            allow_asymmetry: false,
        },
        repetition: RepetitionPolicy {
            density: 0.5,
            preferred_spacing: LengthValue::FamilyUnits(1.0),
            maximum_default_count: 4,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: false,
            allowed_axes: Vec::new(),
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.0,
            detail: 0.0,
        },
        family_facets: BTreeMap::from([(
            family_id.to_owned(),
            FamilyStyleFacet {
                family_id: family_id.to_owned(),
                proportions: Vec::new(),
                part_prototypes: prototypes
                    .iter()
                    .map(|prototype| PartPrototype {
                        id: (*prototype).to_owned(),
                        display_name: (*prototype).to_owned(),
                        role: "body".to_owned(),
                        operation_tags: vec![AllowedOperationKind::Primitive],
                        style_tags: Vec::new(),
                    })
                    .collect(),
                detail_modules: Vec::new(),
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags: Vec::new(),
    }
}

fn family_implementation() -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "box".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Base"),
        parameter_bindings: vec![ParameterBinding::Scalar {
            slot: "radius".to_owned(),
            role: "body".to_owned(),
            local_path: definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius"),
            transform: ScalarTransform::Direct,
        }],
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn style_implementation(
    style_id: &str,
    family_id: &str,
    default_provider: &str,
    fragments: Vec<RecipeFragment>,
) -> StyleImplementation {
    StyleImplementation {
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: style_id.to_owned(),
        family_id: family_id.to_owned(),
        default_role_providers: BTreeMap::from([("body".to_owned(), default_provider.to_owned())]),
        prototypes: fragments
            .into_iter()
            .map(|fragment| (fragment.id.clone(), fragment))
            .collect(),
        detail_modules: BTreeMap::new(),
    }
}

fn provider_fragment(id: &str, radius: f32) -> RecipeFragment {
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: "body".to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(1)],
            internal_roots: Vec::new(),
            socket_ports: Vec::new(),
            surface_ports: Vec::new(),
        },
        recipe: body_recipe(id, radius),
    }
}

fn body_recipe(title: &str, radius: f32) -> AssetRecipe {
    let definition_id = PartDefinitionId(1);
    let instance_id = PartInstanceId(1);
    let parameter_id = ParameterId(1);
    let definition = PartDefinition {
        id: definition_id,
        name: "Body".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [1.0, 0.5, 0.25],
                radius,
            },
            operations: Vec::new(),
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
        maximum: 0.5,
        step: 0.01,
        mutation_sigma: 0.05,
        topology_changing: false,
        beginner_description: "Corner radius".to_owned(),
    };
    let mut recipe = AssetRecipe::new(AssetId(7), title);
    recipe.schema_version = ASSET_RECIPE_SCHEMA_VERSION;
    recipe.definitions.insert(definition_id, definition);
    recipe.instances.insert(instance_id, instance);
    recipe.root_instances.push(instance_id);
    recipe.parameters.insert(parameter_id, descriptor);
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.parameter = 2;
    recipe
}

fn customizer_profile() -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty("box", Some("plain".to_owned()));
    profile.controls.push(CustomizerControl {
        id: "radius".to_owned(),
        label: "Radius".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default: 0.1 },
        bindings: vec![ControlSlotBinding {
            slot: "radius".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: 0.01,
                maximum: 0.5,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    });
    profile
}

fn provider_ref(stable_id: &str) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: shape_family_compile::identity::CatalogContentFingerprint(
            shape_family_compile::identity::ContentFingerprint([9; 32]),
        ),
    }
}

fn catalog_entry<T: Serialize>(
    stable_id: &str,
    schema_version: u32,
    value: &T,
) -> (CatalogContentRef, String) {
    let canonical_json = json(value);
    let fingerprint =
        shape_foundry::catalog_content_fingerprint_from_json(stable_id, &canonical_json)
            .expect("catalog content should fingerprint");
    (
        CatalogContentRef {
            stable_id: stable_id.to_owned(),
            schema_version,
            fingerprint,
        },
        canonical_json,
    )
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("fixture should serialize")
}

fn minimal_foundry_document() -> FoundryAssetDocument {
    FoundryAssetDocument::new(
        FoundryDocumentId("doc".to_string()),
        content_ref("family", 1),
        content_ref("style", 2),
        content_ref("family_impl", 3),
        content_ref("style_impl", 4),
        content_ref("profile", 5),
    )
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    let fingerprint = format!("{byte:02x}").repeat(32);
    serde_json::from_value(serde_json::json!({
        "stable_id": stable_id,
        "schema_version": 1,
        "fingerprint": fingerprint,
    }))
    .expect("test catalog reference is valid")
}
