#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
    FoundryPackView, run_foundry_job,
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
    FoundryCommand, FoundryConformanceSummary, FoundryDocumentId, FoundryEdit, FoundryPackDocument,
    FoundryPackExportProfile, FoundryProjectRevisionProgram, ProviderOverride, ResponseCurve,
    document_catalog_refs,
};
use shape_project::foundry::FoundryProjectFile;
use shape_render::foundry::FoundryPreviewCache;
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
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
fn candidate_job_mode_comes_from_explicit_candidate_request() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let request = FoundryCandidateRequest {
        seed: 99,
        proposal_count: 24,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: Some("broad".to_owned()),
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
fn stale_job_event_is_rejected_without_touching_current_job() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    state.request_build().expect("first build should schedule");
    state.request_build().expect("second build should schedule");

    let accepted = state.handle_job_event(FoundryJobEvent::Failed {
        job_id: 1,
        message: "old compile failed".to_owned(),
    });

    assert!(!accepted);
    assert!(state.stale_jobs.contains(&1));
    assert!(state.active_jobs.contains_key(&2));
    assert_eq!(
        state.status.as_deref(),
        Some("Ignored a background result because newer work is active.")
    );
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
fn edit_completion_stales_jobs_scheduled_from_previous_document() {
    let fixture = RuntimeFixture::new();
    let mut state = FoundryAppState::new(fixture.document.clone()).expect("valid state");
    let edit_effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "radius".to_owned(),
            value: ControlValue::Scalar(0.2),
        }))
        .expect("edit should schedule");
    let edit_request = start_job(edit_effects);

    let stale_build_effects = state
        .request_build()
        .expect("old-source build should still be schedulable while edit runs");
    let stale_build_request = start_job(stale_build_effects);
    assert_eq!(stale_build_request.job_id(), 2);
    assert!(state.active_jobs.contains_key(&2));

    let edit_event = run_foundry_job(
        edit_request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    assert!(state.handle_job_event(edit_event));

    assert!(state.stale_jobs.contains(&2));
    assert!(!state.active_jobs.contains_key(&2));
    assert_eq!(
        state
            .document
            .as_ref()
            .and_then(|document| document.control_state.get("radius")),
        Some(&ControlValue::Scalar(0.2))
    );

    let stale_event = run_foundry_job(
        stale_build_request,
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    );
    assert!(!state.handle_job_event(stale_event));
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
            label: "Modern style".to_owned(),
            commands: vec![FoundryCommand::SetStyle {
                style_content_ref: fixture.modern_style_ref.clone(),
                style_implementation_ref: fixture.modern_style_impl_ref.clone(),
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

    assert_eq!(output.document.style_content_ref, fixture.modern_style_ref);
    assert_eq!(
        output.document.style_implementation_ref,
        fixture.modern_style_impl_ref
    );
    assert!(output.document.provider_overrides.is_empty());
}

#[test]
fn candidate_generation_job_renders_preview_images_for_cards() {
    let fixture = RuntimeFixture::new();
    let request = FoundryJobRequest::GenerateCandidates {
        job_id: 1,
        document: Box::new(fixture.document.clone()),
        request: FoundryCandidateRequest {
            seed: 33,
            proposal_count: 24,
            result_count: 4,
            mode: FoundryCandidateMode::Refine,
            strategy_id: None,
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
        card.width > 0
            && card.height > 0
            && card.rgba8.len() == (card.width * card.height * 4) as usize
            && card.camera.is_some()
    }));
}

#[test]
fn candidate_preview_failures_are_isolated_to_their_cards() {
    let fixture = RuntimeFixture::new();
    let request = FoundryCandidateRequest {
        seed: 51,
        proposal_count: 24,
        result_count: 4,
        mode: FoundryCandidateMode::Refine,
        strategy_id: None,
    };
    let mut output =
        generate_foundry_candidate_plans(&fixture.document, &fixture.catalog, &request)
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
        &fixture.catalog,
        &mut FoundryPreviewCache::default(),
    )
    .expect("preview failures should not fail candidate card generation");

    assert_eq!(cards.len(), output.candidates.len());
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
    let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
    let state = compiled_fixture_state(&fixture);
    let support = state
        .controls
        .iter()
        .find(|control| control.id == "support_rhythm")
        .expect("support rhythm control");

    assert!(support.options.len() >= 3);
    assert!(support.options.iter().all(|option| {
        option.preview_id.is_some()
            && option.width == 64
            && option.height == 64
            && option.rgba8.len() == (option.width * option.height * 4) as usize
            && option.camera.is_some()
    }));
    assert!(
        support
            .options
            .windows(2)
            .any(|pair| pair[0].rgba8 != pair[1].rgba8),
        "whole-model option thumbnails should reflect option-applied geometry"
    );
}

#[test]
fn novice_can_create_reinforced_bridge_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "structural_heft".to_owned(),
            value: ControlValue::Scalar(0.92),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "support_rhythm".to_owned(),
            value: ControlValue::Provider("marching_pile_bents".to_owned()),
        },
    );

    let document = state.document.as_ref().expect("current bridge document");
    assert_eq!(
        document.control_state.get("structural_heft"),
        Some(&ControlValue::Scalar(0.92))
    );
    assert_eq!(
        document.control_state.get("support_rhythm"),
        Some(&ControlValue::Provider("marching_pile_bents".to_owned()))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_create_compact_vented_crate_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "body_proportions".to_owned(),
            value: ControlValue::Scalar(0.12),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "vent_density".to_owned(),
            value: ControlValue::Choice("dense".to_owned()),
        },
    );

    let document = state.document.as_ref().expect("current crate document");
    assert_eq!(
        document.control_state.get("body_proportions"),
        Some(&ControlValue::Scalar(0.12))
    );
    assert_eq!(
        document.control_state.get("vent_density"),
        Some(&ControlValue::Choice("dense".to_owned()))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_create_tall_lamp_with_new_shade_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::stylized_lamp::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    assert_default_foundry_surface(&state);

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "overall_height".to_owned(),
            value: ControlValue::Scalar(2.05),
        },
    );
    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "shade_style".to_owned(),
            value: ControlValue::Choice("drum".to_owned()),
        },
    );

    let document = state.document.as_ref().expect("current lamp document");
    assert_eq!(
        document.control_state.get("overall_height"),
        Some(&ControlValue::Scalar(2.05))
    );
    assert_eq!(
        document.control_state.get("shade_style"),
        Some(&ControlValue::Choice("drum".to_owned()))
    );
    assert_current_mesh_valid(&state);
}

#[test]
fn novice_can_export_three_member_pack_without_advanced_recipe() {
    let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    add_current_to_fixture_pack(&mut state, &fixture, "balanced");

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "structural_heft".to_owned(),
            value: ControlValue::Scalar(0.25),
        },
    );
    add_current_to_fixture_pack(&mut state, &fixture, "light");

    apply_fixture_command(
        &mut state,
        &fixture,
        FoundryCommand::SetControl {
            control_id: "support_rhythm".to_owned(),
            value: ControlValue::Provider("marching_pile_bents".to_owned()),
        },
    );
    add_current_to_fixture_pack(&mut state, &fixture, "reinforced");

    assert_eq!(state.pack.members.len(), 3);
    assert!(state.pack.can_export);

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
    let fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
    let mut state = compiled_fixture_state(&fixture);
    let effects = state
        .request_candidates(FoundryCandidateRequest {
            seed: 77,
            proposal_count: 24,
            result_count: 4,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
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
    assert!(
        state
            .handle_command(FoundryAppCommand::run(FoundryCommand::RejectCandidate {
                candidate_id: rejected.clone()
            }))
            .expect("reject should not schedule a job")
            .is_empty()
    );
    assert!(!state.candidate_edits.contains_key(&rejected));

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AcceptCandidate {
            candidate_id: accepted.clone(),
        }))
        .expect("accept should schedule candidate edit");
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
    let mut crate_document = minimal_foundry_document();
    crate_document.document_id = FoundryDocumentId("crate".to_owned());
    let mut barrel_document = minimal_foundry_document();
    barrel_document.document_id = FoundryDocumentId("barrel".to_owned());
    pack.members.insert("barrel".to_owned(), barrel_document);
    pack.members.insert("crate".to_owned(), crate_document);
    state.pack = FoundryPackView {
        pack_id: Some("props".to_owned()),
        members: pack
            .members
            .iter()
            .map(|(member_id, document)| (member_id.clone(), document.document_id.clone()))
            .collect(),
        selected_member: Some("crate".to_owned()),
        pack: Some(pack.clone()),
        ..FoundryPackView::default()
    };
    state.selected_pack_member = Some("crate".to_owned());
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
    assert_eq!(state.selected_pack_member.as_deref(), Some("crate"));
    assert_eq!(state.pack.selected_member.as_deref(), Some("crate"));
}

#[test]
fn add_current_to_pack_tracks_membership_and_schedules_pack_compile() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
            pack_id: "props".to_owned(),
            member_id: "crate".to_owned(),
        }))
        .expect("pack membership should schedule compile");

    assert_eq!(state.pack.pack_id.as_deref(), Some("props"));
    assert!(state.pack.members.contains_key("crate"));
    assert_eq!(
        state
            .pack
            .members
            .get("crate")
            .map(|document_id| document_id.0.as_str()),
        Some("crate")
    );
    assert_eq!(state.selected_pack_member.as_deref(), Some("crate"));
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one pack compile job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::CompilePack { job_id, pack } => {
            assert_eq!(*job_id, 1);
            assert_eq!(
                pack.members
                    .get("crate")
                    .map(|document| document.document_id.0.as_str()),
                Some("crate")
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
        .insert("bridge_a".to_owned(), fixture.document.clone());
    let mut second = fixture.document.clone();
    second.document_id = FoundryDocumentId("bridge_b".to_owned());
    second
        .control_state
        .insert("radius".to_owned(), ControlValue::Scalar(0.2));
    pack.members.insert("bridge_b".to_owned(), second);
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
    assert!(
        out_dir
            .join("bridge_a")
            .join("asset-manifest.json")
            .is_file()
    );
    assert!(
        out_dir
            .join("bridge_b")
            .join("asset-manifest.json")
            .is_file()
    );

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
    } = event
    else {
        panic!("expected export success, got {event:?}");
    };
    assert_eq!(actual_dir, out_dir);
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
    modern_style_ref: CatalogContentRef,
    modern_style_impl_ref: CatalogContentRef,
}

impl RuntimeFixture {
    fn new() -> Self {
        let family = family_schema();
        let roman_style = style_kit("roman", "Roman", "bridge", &["roman_body", "heavy_body"]);
        let modern_style = style_kit("modern", "Modern", "bridge", &["modern_body"]);
        let family_impl = family_implementation();
        let roman_style_impl = style_implementation(
            "roman",
            "bridge",
            "roman_body",
            vec![
                provider_fragment("roman_body", 0.1),
                provider_fragment("heavy_body", 0.2),
            ],
        );
        let modern_style_impl = style_implementation(
            "modern",
            "bridge",
            "modern_body",
            vec![provider_fragment("modern_body", 0.12)],
        );
        let profile = customizer_profile();

        let (family_ref, family_json) =
            catalog_entry("bridge-family", ASSET_FAMILY_SCHEMA_VERSION, &family);
        let (roman_style_ref, roman_style_json) =
            catalog_entry("roman-style", STYLE_KIT_SCHEMA_VERSION, &roman_style);
        let (modern_style_ref, modern_style_json) =
            catalog_entry("modern-style", STYLE_KIT_SCHEMA_VERSION, &modern_style);
        let (family_impl_ref, family_impl_json) = catalog_entry(
            "bridge-family-impl",
            FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
            &family_impl,
        );
        let (roman_style_impl_ref, roman_style_impl_json) = catalog_entry(
            "roman-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &roman_style_impl,
        );
        let (modern_style_impl_ref, modern_style_impl_json) = catalog_entry(
            "modern-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &modern_style_impl,
        );
        let (profile_ref, profile_json) = catalog_entry(
            "bridge-profile",
            shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            &profile,
        );

        let mut document = FoundryAssetDocument {
            schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
            document_id: FoundryDocumentId("doc-runtime".to_owned()),
            family_content_ref: family_ref,
            style_content_ref: roman_style_ref,
            family_implementation_ref: family_impl_ref,
            style_implementation_ref: roman_style_impl_ref,
            customizer_profile_ref: profile_ref,
            control_state: BTreeMap::from([("radius".to_owned(), ControlValue::Scalar(0.15))]),
            provider_overrides: BTreeMap::new(),
            foundry_locks: Vec::new(),
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
                ("bridge-family".to_owned(), family_json),
                ("roman-style".to_owned(), roman_style_json),
                ("modern-style".to_owned(), modern_style_json),
                ("bridge-family-impl".to_owned(), family_impl_json),
                ("roman-style-impl".to_owned(), roman_style_impl_json),
                ("modern-style-impl".to_owned(), modern_style_impl_json),
                ("bridge-profile".to_owned(), profile_json),
            ]),
        };

        Self {
            document,
            catalog,
            modern_style_ref,
            modern_style_impl_ref,
        }
    }
}

fn family_schema() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "bridge".to_owned(),
        display_name: "Bridge".to_owned(),
        summary: "Runtime test bridge family".to_owned(),
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
        compatible_style_kits: vec!["roman".to_owned(), "modern".to_owned()],
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
        family_id: "bridge".to_owned(),
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
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
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
