#![allow(dead_code)]

#[path = "../src/commands.rs"]
mod commands;
#[path = "../src/panels/gallery.rs"]
mod gallery;
#[path = "../src/panels/history.rs"]
mod history;
#[path = "../src/jobs.rs"]
mod jobs;
#[path = "../src/panels/menus.rs"]
mod menus;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/viewport.rs"]
mod viewport;

use std::path::PathBuf;

use commands::AppCommand;
use shape_core::{
    CandidateId, EditProgram, NodeId, NodeKind, ParamPath, PrimitiveKind, RevisionId,
    SetScalarEdit, ShapeDocument, ShapeNode, Transform3,
};
use shape_presets::PresetId;
use shape_project::Project;
use shape_search::{Candidate, ShapeDescriptor};

fn test_document(title: &str, radius: f32) -> ShapeDocument {
    ShapeDocument::new(
        title,
        ShapeNode {
            id: NodeId(1),
            name: "Root sphere".to_owned(),
            tags: Default::default(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Primitive(PrimitiveKind::Sphere { radius }),
        },
    )
}

fn radius_path() -> ParamPath {
    ParamPath {
        node: NodeId(1),
        key: "primitive.radius".to_owned(),
    }
}

fn radius_candidate(label: &str, radius: f32, id: u64) -> Candidate {
    Candidate {
        id: CandidateId(id),
        document: test_document(label, radius),
        edit: EditProgram {
            label: label.to_owned(),
            seed: id,
            operations: vec![SetScalarEdit {
                path: radius_path(),
                before: 1.0,
                after: radius,
            }],
        },
        descriptor: ShapeDescriptor {
            values: vec![radius],
        },
        distance_from_parent: (radius - 1.0).abs(),
    }
}

#[test]
fn edit_summary_formats_revision_changes() {
    assert_eq!(history::edit_summary(None), "Starting shape");

    let candidate = radius_candidate("Wider", 1.25, 10);

    assert_eq!(
        history::edit_summary(Some(&candidate.edit)),
        "Radius 1.00 -> 1.25"
    );
}

#[test]
fn candidate_difference_formatting_uses_node_and_parameter_labels() {
    let parent = test_document("Parent", 1.0);
    let candidate = radius_candidate("Wider", 1.35, 11);

    let lines = gallery::candidate_difference_lines(&parent, &candidate, 3);

    assert_eq!(lines, vec!["Root sphere Radius: 1.00 -> 1.35"]);
    assert_eq!(gallery::distance_label(0.04), "Subtle change");
    assert_eq!(gallery::distance_label(0.40), "Large change");
}

#[test]
fn branch_label_identifies_branch_points() {
    let mut project = Project::new("Branch test", test_document("Initial", 1.0));
    let first = project
        .accept_candidate(radius_candidate("First direction", 1.15, 1))
        .unwrap();
    project.undo().unwrap();
    let second = project
        .accept_candidate(radius_candidate("Second direction", 1.35, 2))
        .unwrap();

    assert_eq!(
        project.children_of(RevisionId(0)),
        vec![RevisionId(1), RevisionId(2)]
    );
    assert_eq!(
        history::branch_label(&project, RevisionId(0)),
        "Branch point: 2 directions"
    );
    assert_eq!(
        history::branch_label(&project, first),
        "No child directions"
    );
    assert_eq!(
        history::branch_label(&project, second),
        "No child directions"
    );
}

#[test]
fn command_emission_helpers_return_app_commands() {
    let project_path = PathBuf::from("demo.shapelab.json");
    let export_path = PathBuf::from("demo.obj");
    let preset = PresetId("desk-lamp".to_owned());

    assert_eq!(
        menus::command_for_preset(preset.clone()),
        AppCommand::LoadPreset(preset)
    );
    assert_eq!(
        menus::command_for_direct_action(menus::DirectMenuAction::Save),
        AppCommand::Save
    );
    assert_eq!(
        menus::command_for_direct_action(menus::DirectMenuAction::FitView),
        AppCommand::FitView
    );
    assert_eq!(
        menus::command_for_path_action(menus::PathMenuAction::OpenProject, project_path.clone()),
        AppCommand::OpenProject(project_path)
    );
    assert_eq!(
        menus::command_for_path_action(
            menus::PathMenuAction::ExportCurrentObj,
            export_path.clone()
        ),
        AppCommand::ExportCurrentObj(export_path)
    );
    assert_eq!(
        gallery::accept_candidate_command(CandidateId(7)),
        AppCommand::AcceptCandidate(CandidateId(7))
    );
    assert_eq!(
        history::switch_revision_command(RevisionId(3)),
        AppCommand::SwitchRevision(RevisionId(3))
    );
}

#[test]
fn menu_file_suggestions_are_safe_and_title_based() {
    assert_eq!(
        menus::suggested_project_file_name("Desk Lamp / Final"),
        "desk-lamp-final.shapelab.json"
    );
    assert_eq!(
        menus::suggested_obj_file_name("Desk Lamp / Final"),
        "desk-lamp-final.obj"
    );
    assert_eq!(
        menus::suggested_project_file_name("CON"),
        "shape-con.shapelab.json"
    );
    assert_eq!(menus::suggested_obj_file_name("   "), "untitled.obj");
}
