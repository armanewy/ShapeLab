#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::BTreeMap;

#[path = "../src/foundry/mod.rs"]
mod foundry;

use foundry::panels::directions::{
    DEFAULT_DIRECTION_PROPOSALS, DIRECTION_BOARD_MODES, DirectionBoardIntent,
    DirectionBoardRowKind, DirectionBoardState, DirectionCardKind, DirectionCardSlot,
    DirectionComparisonRole, DirectionImageScope, DirectionValidationState,
    VISIBLE_DIRECTION_CANDIDATE_CARDS, ab_flip_comparison, accept_candidate_command,
    candidate_request_for_mode, direction_board_view, direction_mode_actions,
    direction_variation_mode_actions, hover_candidate_intent, no_isolated_part_option_cards,
    reject_candidate_command, select_candidate_intent,
};
use foundry::{FoundryAppCommand, FoundryCandidateCard};
use shape_foundry::{
    CandidateLegibilityClass, FoundryCandidateId, FoundryCommand, FoundrySurfaceCapabilityView,
    SURFACE_PACKAGE_UNAVAILABLE_REASON, VariationChannel, VariationIntent,
};
use shape_render::OrbitCamera;
use shape_search::foundry::{
    FoundryCandidateChangeKind, FoundryCandidateControlChange, FoundryCandidateMode,
    FoundryCandidateRejectionReason,
};

#[test]
fn direction_board_builds_parent_row_and_six_whole_model_candidate_cards() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let candidates = six_candidates(&camera);
    let selected = candidates[2].id.clone();

    let board = direction_board_view(
        &parent,
        &candidates,
        DirectionBoardState {
            selected_candidate: Some(selected.clone()),
            active_mode: Some(FoundryCandidateMode::Explore),
            generation_seed: 91,
            ..DirectionBoardState::default()
        },
    );

    assert_eq!(board.rows.len(), 2);
    assert_eq!(board.rows[0].kind, DirectionBoardRowKind::Parent);
    assert_eq!(board.rows[1].kind, DirectionBoardRowKind::Candidates);
    assert_eq!(
        board.candidate_slots.len(),
        VISIBLE_DIRECTION_CANDIDATE_CARDS
    );
    assert_eq!(
        board.validation.filled_candidate_count,
        VISIBLE_DIRECTION_CANDIDATE_CARDS
    );
    assert!(board.validation.is_valid());
    assert!(no_isolated_part_option_cards(&board));

    assert_eq!(board.parent.kind, DirectionCardKind::UnchangedParent);
    assert_eq!(
        board.parent.validation_badge.state,
        DirectionValidationState::Unchanged
    );
    assert_eq!(board.parent.preview.scope, DirectionImageScope::WholeModel);
    assert_eq!(board.parent.preview.camera, Some(camera.clone()));

    for slot in &board.candidate_slots {
        let card = slot.as_card().expect("candidate slot is filled");
        assert_eq!(card.kind, DirectionCardKind::Candidate);
        assert_eq!(card.preview.scope, DirectionImageScope::WholeModel);
        assert_eq!(card.preview.camera, Some(camera.clone()));
        assert!(card.preview.has_image());
    }

    let comparison = board
        .comparison
        .expect("selected candidate has comparison data");
    assert_eq!(comparison.a.role, DirectionComparisonRole::Parent);
    assert_eq!(comparison.b.role, DirectionComparisonRole::Candidate);
    assert_eq!(comparison.b.card_id, selected);
    assert!(comparison.fixed_camera);
}

#[test]
fn direction_modes_cover_refine_explore_silhouette_structure_and_detail() {
    let actions = direction_mode_actions(
        Some(FoundryCandidateMode::Structure),
        17,
        Some("box_primitive".to_owned()),
    );

    assert_eq!(DIRECTION_BOARD_MODES.len(), 5);
    assert_eq!(
        actions
            .iter()
            .map(|action| action.label)
            .collect::<Vec<_>>(),
        vec!["Refine", "Explore", "Silhouette", "Structure", "Detail"]
    );
    assert_eq!(
        actions.iter().map(|action| action.mode).collect::<Vec<_>>(),
        DIRECTION_BOARD_MODES
    );
    assert!(
        actions
            .iter()
            .find(|action| action.mode == FoundryCandidateMode::Structure)
            .expect("structure mode exists")
            .selected
    );
    assert!(actions.iter().all(|action| {
        action.request.proposal_count == DEFAULT_DIRECTION_PROPOSALS
            && action.request.result_count == VISIBLE_DIRECTION_CANDIDATE_CARDS
            && action.request.strategy_id.as_deref() == Some("box_primitive")
    }));

    let request = candidate_request_for_mode(FoundryCandidateMode::Detail, 23, None);
    assert_eq!(request.mode, FoundryCandidateMode::Detail);
    assert_eq!(request.seed, 23);
    assert_eq!(request.result_count, VISIBLE_DIRECTION_CANDIDATE_CARDS);
    assert_eq!(
        request.variation_intent.channels,
        vec![VariationChannel::Detail]
    );

    let explore_action = actions
        .iter()
        .find(|action| action.mode == FoundryCandidateMode::Explore)
        .expect("explore mode exists");
    assert!(matches!(
        explore_action.app_command(),
        FoundryAppCommand::RequestCandidates(request)
            if request.mode == FoundryCandidateMode::Explore
    ));
}

#[test]
fn foundry_variation_modes_expose_product_scope_and_channel_actions() {
    let actions = direction_variation_mode_actions(
        &VariationIntent::default(),
        29,
        Some("box_primitive".to_owned()),
        None,
        &[],
    );

    assert_eq!(
        actions
            .iter()
            .map(|action| action.label)
            .collect::<Vec<_>>(),
        vec!["Looks", "Shape", "Surface", "Focused part"]
    );
    assert!(actions[0].selected);
    assert!(actions[0].enabled);
    assert_eq!(
        actions[0]
            .request
            .as_ref()
            .unwrap()
            .variation_intent
            .channels,
        vec![VariationChannel::CompleteLook]
    );
    assert!(actions[1].enabled);
    assert_eq!(
        actions[1]
            .request
            .as_ref()
            .unwrap()
            .variation_intent
            .channels,
        vec![VariationChannel::Shape]
    );
    assert!(!actions[2].enabled);
    assert_eq!(
        actions[2].unavailable_reason,
        Some(SURFACE_PACKAGE_UNAVAILABLE_REASON)
    );
    assert!(!actions[3].enabled);
    assert_eq!(
        actions[3].unavailable_reason,
        Some("Focused part ideas are available when this asset exposes editable part groups.")
    );
}

#[test]
fn unavailable_surface_capability_does_not_enable_visual_surface_mode() {
    let capability = FoundrySurfaceCapabilityView::unavailable("box-primitive", "Surface package");
    let actions = direction_variation_mode_actions(
        &VariationIntent::default(),
        31,
        Some("box_primitive".to_owned()),
        Some(&capability),
        &[],
    );
    let surface = actions
        .iter()
        .find(|action| action.label == "Surface")
        .expect("Surface action exists");

    assert!(!surface.enabled);
    assert_eq!(
        surface.unavailable_reason,
        Some(SURFACE_PACKAGE_UNAVAILABLE_REASON)
    );
    assert!(surface.request.is_none());
}

#[test]
fn box_primitive_exposes_no_focus_part_groups() {
    let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
    let groups = foundry::panels::directions::direction_part_groups_for_document(&fixture.document);
    assert!(
        groups.is_empty(),
        "Box Primitive should not expose part chips"
    );
}

#[test]
fn lidded_box_exposes_no_focus_part_groups() {
    let fixture = shape_foundry_catalog::box_primitive::lidded_box_fixture_catalog();
    let groups = foundry::panels::directions::direction_part_groups_for_document(&fixture.document);
    assert!(groups.is_empty(), "Lidded Box should not expose part chips");
}

#[test]
fn focus_part_mode_targets_active_group_when_selected() {
    let intent = VariationIntent::focus_part_shape("body", "Body");
    let groups = vec![
        foundry::panels::directions::DirectionPartGroup {
            group_id: "body".to_owned(),
            label: "Body".to_owned(),
            focusable: true,
            unavailable_reason: None,
        },
        foundry::panels::directions::DirectionPartGroup {
            group_id: "edge_softness".to_owned(),
            label: "Edge Softness".to_owned(),
            focusable: true,
            unavailable_reason: None,
        },
    ];

    let actions = direction_variation_mode_actions(&intent, 73, None, None, &groups);
    let focus = actions
        .iter()
        .find(|action| action.label == "Focused part")
        .expect("focus action exists");

    assert!(focus.selected);
    assert!(focus.enabled);
    assert_eq!(
        focus
            .request
            .as_ref()
            .and_then(|request| request.variation_intent.scope.semantic_part_group_id()),
        Some("body")
    );
}

#[test]
fn candidate_cards_expose_explanations_role_badges_and_hover_highlight() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let mut candidate = candidate_card(
        "candidate-role",
        0,
        FoundryCandidateMode::Structure,
        &camera,
    );
    candidate.changed_roles = vec![
        "body".to_owned(),
        "edge_softness".to_owned(),
        "body".to_owned(),
        " ".to_owned(),
    ];
    candidate.explanations = vec![control_change(
        FoundryCandidateChangeKind::Provider,
        "body_provider",
        "Body provider",
        "Compact",
        "Tall",
        "Body provider changed from `Compact` to `Tall`.",
    )];

    let board = direction_board_view(
        &parent,
        &[candidate.clone()],
        DirectionBoardState {
            hovered_candidate: Some(candidate.id.clone()),
            ..DirectionBoardState::default()
        },
    );

    let card = board.candidate_slots[0]
        .as_card()
        .expect("first candidate is present");
    assert!(card.hovered);
    assert!(card.highlighted);
    assert!(!card.selected);
    assert_eq!(card.mode_label, "Structure");
    assert_eq!(
        card.explanations,
        vec!["Body provider changed from `Compact` to `Tall`."]
    );
    assert_eq!(
        card.changed_role_badges
            .iter()
            .map(|badge| badge.label.as_str())
            .collect::<Vec<_>>(),
        vec!["body", "edge_softness"]
    );
    assert_eq!(card.validation_badge.state, DirectionValidationState::Valid);
}

#[test]
fn card_intents_map_to_selection_accept_and_reject_app_commands() {
    let id = FoundryCandidateId("candidate-42".to_owned());

    assert_eq!(
        select_candidate_intent(Some(id.clone())).app_command(),
        Some(FoundryAppCommand::SelectCandidate(Some(id.clone())))
    );
    assert_eq!(hover_candidate_intent(Some(id.clone())).app_command(), None);

    let accept = accept_candidate_command(id.clone());
    assert!(matches!(
        accept.single_foundry_command(),
        Some(FoundryCommand::AcceptCandidate { candidate_id }) if candidate_id == &id
    ));

    let reject = reject_candidate_command(id.clone());
    assert!(matches!(
        reject.single_foundry_command(),
        Some(FoundryCommand::RejectCandidate { candidate_id }) if candidate_id == &id
    ));

    assert_eq!(
        DirectionBoardIntent::Accept(id.clone()).app_command(),
        Some(accept)
    );
    assert_eq!(
        DirectionBoardIntent::Reject(id.clone()).app_command(),
        Some(reject)
    );
}

#[test]
fn invalid_candidate_validation_blocks_choose_but_keeps_reject_available() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let mut candidate = candidate_card(
        "blocked-candidate",
        0,
        FoundryCandidateMode::Explore,
        &camera,
    );
    candidate.selectable = false;
    candidate.validation_label = "Blocked".to_owned();
    candidate
        .rejections
        .insert(FoundryCandidateRejectionReason::CompileRejected, 2);

    let board = direction_board_view(&parent, &[candidate], DirectionBoardState::default());
    let card = board.candidate_slots[0]
        .as_card()
        .expect("blocked candidate is present");

    assert_eq!(
        card.validation_badge.state,
        DirectionValidationState::Invalid
    );
    assert_eq!(card.validation_badge.rejection_count, 2);
    assert_eq!(
        card.validation_badge.detail.as_deref(),
        Some("2 build unavailable")
    );
    assert!(card.actions.choose.is_none());
    assert!(matches!(
        card.actions.reject,
        Some(DirectionBoardIntent::Reject(_))
    ));
}

#[test]
fn rejected_candidate_cannot_be_chosen_even_when_marked_selectable() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let mut candidate = candidate_card(
        "rejected-selectable",
        0,
        FoundryCandidateMode::Explore,
        &camera,
    );
    candidate.selectable = true;
    candidate
        .rejections
        .insert(FoundryCandidateRejectionReason::DescriptorRejected, 1);

    let board = direction_board_view(&parent, &[candidate], DirectionBoardState::default());
    let card = board.candidate_slots[0]
        .as_card()
        .expect("rejected candidate is present");

    assert_eq!(
        card.validation_badge.state,
        DirectionValidationState::Invalid
    );
    assert!(card.actions.choose.is_none());
}

#[test]
fn ab_flip_comparison_swaps_sides_and_keeps_fixed_camera_whole_model_images() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let candidate = candidate_card(
        "candidate-flip",
        0,
        FoundryCandidateMode::Silhouette,
        &camera,
    );
    let board = direction_board_view(
        &parent,
        std::slice::from_ref(&candidate),
        DirectionBoardState {
            selected_candidate: Some(candidate.id.clone()),
            comparison_flipped: true,
            ..DirectionBoardState::default()
        },
    );

    let comparison = board.comparison.expect("flipped comparison exists");
    assert!(comparison.flipped);
    assert_eq!(comparison.a.label, "A");
    assert_eq!(comparison.a.role, DirectionComparisonRole::Candidate);
    assert_eq!(comparison.a.card_id, candidate.id);
    assert_eq!(comparison.b.label, "B");
    assert_eq!(comparison.b.role, DirectionComparisonRole::Parent);
    assert!(comparison.fixed_camera);
    assert_eq!(comparison.scope, DirectionImageScope::WholeModel);
    assert!(comparison.a.preview.has_image());
    assert!(comparison.b.preview.has_image());

    let direct = ab_flip_comparison(
        &board.parent,
        board.candidate_slots[0].as_card().unwrap(),
        false,
    );
    assert_eq!(direct.a.role, DirectionComparisonRole::Parent);
    assert_eq!(direct.b.role, DirectionComparisonRole::Candidate);
}

#[test]
fn card_selected_fallback_drives_highlight_and_comparison() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let mut candidate = candidate_card(
        "selected-fallback",
        0,
        FoundryCandidateMode::Structure,
        &camera,
    );
    candidate.selected = true;

    let board = direction_board_view(
        &parent,
        std::slice::from_ref(&candidate),
        DirectionBoardState::default(),
    );
    let card = board.candidate_slots[0]
        .as_card()
        .expect("selected candidate is present");

    assert!(card.selected);
    assert!(card.highlighted);
    assert_eq!(
        board
            .comparison
            .as_ref()
            .expect("selected fallback creates comparison")
            .b
            .card_id,
        candidate.id
    );
}

#[test]
fn board_guard_truncates_to_six_candidates_and_emits_no_isolated_part_options() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());
    let mut candidates = six_candidates(&camera);
    candidates.push(candidate_card(
        "candidate-overflow",
        6,
        FoundryCandidateMode::Detail,
        &camera,
    ));

    let board = direction_board_view(&parent, &candidates, DirectionBoardState::default());

    assert_eq!(
        board.candidate_slots.len(),
        VISIBLE_DIRECTION_CANDIDATE_CARDS
    );
    assert_eq!(board.validation.overflow_candidate_count, 1);
    assert_eq!(board.isolated_part_option_card_count, 0);
    assert!(no_isolated_part_option_cards(&board));
    assert!(
        board
            .candidate_slots
            .iter()
            .all(|slot| matches!(slot, DirectionCardSlot::Filled(_)))
    );
    assert!(board.rows[1].cards.iter().all(|slot| !slot.is_empty()));
}

#[test]
fn board_validation_accepts_fewer_clear_candidate_images() {
    let camera = OrbitCamera::default();
    let parent = parent_card(camera.clone());

    let empty_board = direction_board_view(&parent, &[], DirectionBoardState::default());
    assert_eq!(empty_board.validation.filled_candidate_count, 0);
    assert!(empty_board.validation.preview_images_present);
    assert!(!empty_board.validation.is_valid());

    let two_candidates = six_candidates(&camera)
        .into_iter()
        .take(2)
        .collect::<Vec<_>>();
    let partial_board =
        direction_board_view(&parent, &two_candidates, DirectionBoardState::default());
    assert_eq!(partial_board.validation.filled_candidate_count, 2);
    assert!(partial_board.validation.preview_images_present);
    assert!(partial_board.validation.is_valid());

    let mut candidates = six_candidates(&camera);
    candidates[3].rgba8 = vec![1, 2, 3];
    let missing_image_board =
        direction_board_view(&parent, &candidates, DirectionBoardState::default());

    assert_eq!(
        missing_image_board.validation.filled_candidate_count,
        VISIBLE_DIRECTION_CANDIDATE_CARDS
    );
    assert!(!missing_image_board.validation.preview_images_present);
    assert!(!missing_image_board.validation.is_valid());

    candidates[3].rgba8.clear();
    let empty_image_board =
        direction_board_view(&parent, &candidates, DirectionBoardState::default());
    assert!(!empty_image_board.validation.preview_images_present);
    assert!(!empty_image_board.validation.is_valid());
}

fn parent_card(camera: OrbitCamera) -> FoundryCandidateCard {
    let mut card = candidate_card("parent", 0, FoundryCandidateMode::Refine, &camera);
    card.parent = true;
    card.mode = None;
    card.title = "Current model".to_owned();
    card.subtitle = "Unchanged parent".to_owned();
    card.selectable = false;
    card.validation_label = "Unchanged".to_owned();
    card.changed_controls.clear();
    card.changed_roles.clear();
    card.explanations.clear();
    card
}

fn six_candidates(camera: &OrbitCamera) -> Vec<FoundryCandidateCard> {
    [
        FoundryCandidateMode::Refine,
        FoundryCandidateMode::Explore,
        FoundryCandidateMode::Silhouette,
        FoundryCandidateMode::Structure,
        FoundryCandidateMode::Detail,
        FoundryCandidateMode::Explore,
    ]
    .into_iter()
    .enumerate()
    .map(|(slot, mode)| candidate_card(&format!("candidate-{slot}"), slot, mode, camera))
    .collect()
}

fn candidate_card(
    id: &str,
    slot: usize,
    mode: FoundryCandidateMode,
    camera: &OrbitCamera,
) -> FoundryCandidateCard {
    FoundryCandidateCard {
        id: FoundryCandidateId(id.to_owned()),
        slot,
        mode: Some(mode),
        parent: false,
        title: format!("Direction {}", slot + 1),
        subtitle: "Whole-model candidate".to_owned(),
        preview_id: Some(format!("preview-{id}")),
        rgba8: vec![slot as u8, 16, 32, 255],
        width: 1,
        height: 1,
        camera: Some(camera.clone()),
        preview_failure: None,
        changed_controls: vec![format!("control-{slot}")],
        changed_roles: Vec::new(),
        explanations: vec![control_change(
            FoundryCandidateChangeKind::Numeric,
            &format!("control-{slot}"),
            "Height",
            "1.000",
            "1.250",
            "Height changed from `1.000` to `1.250`.",
        )],
        rejections: BTreeMap::new(),
        validation_label: "Valid".to_owned(),
        validation_detail: None,
        selectable: true,
        selected: false,
        variation_intent_label: "Complete Looks".to_owned(),
        variation_scope_label: "Whole Asset".to_owned(),
        variation_channel_labels: vec!["Complete Look".to_owned()],
        visible_delta_label: "Clear change".to_owned(),
        what_changed_summary: "Whole Asset: Clear change".to_owned(),
        legibility_class: CandidateLegibilityClass::Clear,
        focus_part_label: None,
        surface_unavailable_reason: None,
    }
}

fn control_change(
    kind: FoundryCandidateChangeKind,
    control_id: &str,
    control_label: &str,
    before: &str,
    after: &str,
    message: &str,
) -> FoundryCandidateControlChange {
    FoundryCandidateControlChange {
        kind,
        control_id: control_id.to_owned(),
        control_label: control_label.to_owned(),
        before: before.to_owned(),
        after: after.to_owned(),
        message: message.to_owned(),
        details: Vec::new(),
        topology_changing: false,
    }
}
