#![allow(dead_code)]

#[path = "../src/asset/mod.rs"]
mod asset;
#[path = "../src/viewport.rs"]
mod viewport;

use std::collections::BTreeSet;

use asset::panels::{candidate_gallery, history, inspector, part_tree};
use asset::viewport as asset_viewport;
use asset::{
    AssetAppCommand, AssetCandidate, AssetCandidateEdit, AssetCandidateId, AssetHistoryRevision,
    AssetJobId, AssetJobKind, AssetJobProgress, AssetParameter, AssetParameterGroup, AssetPart,
    AssetRevisionId, AssetUiState, AssetValidationState, GeneratedPartKind, ParameterId,
    PartDefinitionId, PartInstanceId,
};

#[test]
fn asset_panel_helpers_emit_commands() {
    assert_eq!(
        part_tree::select_part_command(Some(PartInstanceId(1)), PartInstanceId(2)),
        Some(AssetAppCommand::SelectPart(PartInstanceId(2)))
    );
    assert_eq!(
        part_tree::select_part_command(Some(PartInstanceId(2)), PartInstanceId(2)),
        None
    );
    assert_eq!(
        candidate_gallery::accept_candidate_command(AssetCandidateId(5)),
        AssetAppCommand::AcceptCandidate(AssetCandidateId(5))
    );
    assert_eq!(
        candidate_gallery::reject_candidate_command(AssetCandidateId(5)),
        AssetAppCommand::RejectCandidate(AssetCandidateId(5))
    );
    assert_eq!(
        history::switch_revision_command(AssetRevisionId(2), false),
        Some(AssetAppCommand::SwitchRevision(AssetRevisionId(2)))
    );
    assert_eq!(
        history::switch_revision_command(AssetRevisionId(2), true),
        None
    );
    assert_eq!(
        asset_viewport::wireframe_command(false, true),
        Some(AssetAppCommand::SetWireframe(true))
    );
}

#[test]
fn asset_inspector_uses_beginner_parameter_groups() {
    let parameters = AssetParameterGroup::all()
        .into_iter()
        .enumerate()
        .map(|(index, group)| parameter(ParameterId(index as u64 + 1), group, false))
        .collect::<Vec<_>>();

    let sections = inspector::grouped_parameter_sections(&parameters);
    let labels = sections
        .iter()
        .map(|section| section.group.label())
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "Size",
            "Proportions",
            "Placement",
            "Curvature",
            "Edge Softness",
            "Repetition",
            "Part Presence",
            "Detail Density"
        ]
    );
    assert_eq!(
        inspector::parameter_ids(&sections[0].parameters),
        vec![ParameterId(1)]
    );
}

#[test]
fn asset_panels_have_empty_states() {
    let state = AssetUiState::empty("Empty asset");

    assert!(part_tree::build_part_tree_rows(&state).is_empty());
    assert_eq!(
        part_tree::empty_part_tree_message(),
        "No asset parts are available yet."
    );
    assert_eq!(
        inspector::empty_inspector_message(),
        "Select a part to edit its beginner controls."
    );
    assert_eq!(history::empty_history_message(), "No asset revisions yet.");
}

#[test]
fn part_tree_rows_show_shared_optional_generated_and_warning_state() {
    let state = sample_state();
    let rows = part_tree::build_part_tree_rows(&state);

    assert_eq!(rows.len(), 3);
    assert!(rows[1].shared_definition);
    assert!(rows[2].shared_definition);
    assert!(rows[2].optional);
    assert_eq!(rows[2].generated_label.as_deref(), Some("Array 2/4"));
    assert_eq!(rows[2].validation_badge.as_deref(), Some("1!"));
    assert!(rows[1].selected);
}

#[test]
fn candidate_summary_and_progress_are_stable() {
    let mut state = sample_state();
    state.candidates = vec![candidate_with_edits()];
    state.active_job = Some(AssetJobProgress {
        job_id: AssetJobId(8),
        kind: AssetJobKind::CandidateSearch,
        phase: "Generating directions".to_owned(),
        completed: 3,
        total: 6,
    });

    let slots = candidate_gallery::candidate_slots(&state.candidates);
    assert_eq!(slots.len(), 6);
    assert!(slots[0].is_some());
    assert!(slots[1].is_none());

    let summary = candidate_gallery::candidate_summary(&state.candidates[0]);
    assert_eq!(summary.structural_summary, "1 structural change");
    assert_eq!(summary.numeric_summary, "2 value changes");
    assert_eq!(summary.validation, "Warning");
    assert_eq!(
        summary.edit_lines,
        vec![
            "Shade width increases: 1.00 -> 1.25",
            "Switch on",
            "Socket move decreases: 3.00 -> 2.00"
        ]
    );
    assert_eq!(
        candidate_gallery::generation_progress_label(&state).as_deref(),
        Some("Generating directions: 3/6 (50%)")
    );
}

#[test]
fn empty_candidate_explanation_is_explicit() {
    let candidate = AssetCandidate {
        id: AssetCandidateId(10),
        title: String::new(),
        structural_changes: 0,
        numeric_changes: 0,
        edits: Vec::new(),
        validation: AssetValidationState::Valid,
    };

    let summary = candidate_gallery::candidate_summary(&candidate);

    assert_eq!(summary.title, "Candidate 10");
    assert_eq!(
        summary.edit_lines,
        vec![candidate_gallery::empty_explanatory_edit_list()]
    );
}

#[test]
fn history_rows_include_branch_labels_and_undo_state() {
    let revisions = vec![
        AssetHistoryRevision {
            id: AssetRevisionId(0),
            parent: None,
            label: "Start".to_owned(),
            operation_summary: "Base recipe".to_owned(),
            child_count: 2,
            selected: false,
        },
        AssetHistoryRevision {
            id: AssetRevisionId(1),
            parent: Some(AssetRevisionId(0)),
            label: "Wider shade".to_owned(),
            operation_summary: "Set shade width to 1.25".to_owned(),
            child_count: 0,
            selected: true,
        },
        AssetHistoryRevision {
            id: AssetRevisionId(2),
            parent: Some(AssetRevisionId(0)),
            label: "Taller stem".to_owned(),
            operation_summary: String::new(),
            child_count: 0,
            selected: false,
        },
    ];

    let rows = history::build_history_rows(&revisions);

    assert_eq!(rows[0].label, "Revision 0: Start");
    assert_eq!(rows[0].branch_label, "2 branches");
    assert_eq!(rows[1].depth, 1);
    assert_eq!(rows[2].operation_summary, "No operation summary");
    assert!(history::can_undo(&revisions));
    assert_eq!(history::undo_command(true), Some(AssetAppCommand::Undo));
}

#[test]
fn lock_behavior_blocks_edits_and_deduplicates_lock_commands() {
    let unlocked = parameter(ParameterId(1), AssetParameterGroup::Size, false);
    let locked = parameter(ParameterId(2), AssetParameterGroup::Size, true);

    assert_eq!(
        inspector::parameter_command(&unlocked, 3.5),
        Some(AssetAppCommand::SetParameter(ParameterId(1), 2.0))
    );
    assert_eq!(inspector::parameter_command(&locked, 1.5), None);
    assert_eq!(
        inspector::parameter_lock_command(&unlocked, true),
        Some(AssetAppCommand::SetParameterLock(ParameterId(1), true))
    );
    assert_eq!(inspector::parameter_lock_command(&locked, true), None);

    let part = optional_part(PartInstanceId(4), true, GeneratedPartKind::Authored, 0);
    assert_eq!(
        inspector::part_presence_command(&part, false, false),
        Some(AssetAppCommand::ToggleOptionalPart(
            PartInstanceId(4),
            false
        ))
    );
    assert_eq!(inspector::part_presence_command(&part, false, true), None);
    assert_eq!(
        inspector::part_lock_command(PartInstanceId(4), false, true),
        Some(AssetAppCommand::SetPartLock(PartInstanceId(4), true))
    );
    assert_eq!(
        inspector::subtree_lock_command(PartInstanceId(4), false, true),
        Some(AssetAppCommand::SetSubtreeLock(PartInstanceId(4), true))
    );
    assert_eq!(
        inspector::topology_lock_command(PartDefinitionId(7), false, true),
        Some(AssetAppCommand::SetTopologyLock(PartDefinitionId(7), true))
    );
}

#[test]
fn viewport_overlay_labels_include_modeling_overlays_only() {
    let overlay = asset_viewport::AssetViewportOverlay {
        selected_part_name: Some("Shade".to_owned()),
        selected_part_bounds: Some(asset_viewport::NormalizedRect {
            min: [0.2, 0.2],
            max: [0.8, 0.8],
        }),
        socket_markers: vec![asset_viewport::SocketMarker {
            name: "stem".to_owned(),
            position: [0.5, 0.7],
        }],
        validation_marker: Some(AssetValidationState::Error("socket mismatch".to_owned())),
        wireframe: true,
        ..Default::default()
    };

    assert_eq!(
        asset_viewport::overlay_labels(&overlay),
        vec![
            "Selected part: Shade",
            "Part bounds",
            "1 socket marker(s)",
            "Validation: Blocked",
            "Wireframe on"
        ]
    );
}

fn sample_state() -> AssetUiState {
    let mut state = AssetUiState::empty("Desk lamp");
    state.selected_part = Some(PartInstanceId(2));
    state.parts = vec![
        AssetPart {
            id: PartInstanceId(1),
            parent: None,
            definition: PartDefinitionId(1),
            name: "Lamp".to_owned(),
            definition_name: "Lamp assembly".to_owned(),
            enabled: true,
            optional: false,
            generated: GeneratedPartKind::Authored,
            socket_count: 1,
            region_count: 0,
            warning_count: 0,
        },
        AssetPart {
            id: PartInstanceId(2),
            parent: Some(PartInstanceId(1)),
            definition: PartDefinitionId(2),
            name: "Left shade panel".to_owned(),
            definition_name: "Shade panel".to_owned(),
            enabled: true,
            optional: false,
            generated: GeneratedPartKind::Mirrored,
            socket_count: 0,
            region_count: 1,
            warning_count: 0,
        },
        optional_part(
            PartInstanceId(3),
            false,
            GeneratedPartKind::LinearArray { index: 2, count: 4 },
            1,
        ),
    ];
    state.parameters = vec![
        parameter(ParameterId(1), AssetParameterGroup::Size, false),
        parameter(ParameterId(2), AssetParameterGroup::Placement, false),
    ];
    state.part_locks = BTreeSet::new();
    state
}

fn optional_part(
    id: PartInstanceId,
    enabled: bool,
    generated: GeneratedPartKind,
    warning_count: usize,
) -> AssetPart {
    AssetPart {
        id,
        parent: Some(PartInstanceId(1)),
        definition: PartDefinitionId(2),
        name: "Right shade panel".to_owned(),
        definition_name: "Shade panel".to_owned(),
        enabled,
        optional: true,
        generated,
        socket_count: 0,
        region_count: 1,
        warning_count,
    }
}

fn parameter(id: ParameterId, group: AssetParameterGroup, locked: bool) -> AssetParameter {
    AssetParameter {
        id,
        part: Some(PartInstanceId(2)),
        definition: Some(PartDefinitionId(2)),
        label: format!("{} control", group.label()),
        technical_name: format!("definition.2.parameter.{}", id.0),
        group,
        value: 1.0,
        minimum: 0.0,
        maximum: 2.0,
        step: 0.01,
        locked,
        topology_changing: group == AssetParameterGroup::DetailDensity,
        beginner_description: "Beginner description".to_owned(),
    }
}

fn candidate_with_edits() -> AssetCandidate {
    AssetCandidate {
        id: AssetCandidateId(7),
        title: "Wider shade".to_owned(),
        structural_changes: 1,
        numeric_changes: 2,
        validation: AssetValidationState::Warning("thin panel".to_owned()),
        edits: vec![
            AssetCandidateEdit {
                subject: "Shade".to_owned(),
                label: "width".to_owned(),
                before: Some(1.0),
                after: Some(1.25),
                structural: false,
            },
            AssetCandidateEdit {
                subject: "Switch".to_owned(),
                label: "on".to_owned(),
                before: None,
                after: None,
                structural: true,
            },
            AssetCandidateEdit {
                subject: "Socket".to_owned(),
                label: "move".to_owned(),
                before: Some(3.0),
                after: Some(2.0),
                structural: false,
            },
        ],
    }
}
