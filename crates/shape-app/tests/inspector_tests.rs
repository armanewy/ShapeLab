#![allow(dead_code)]

#[path = "../src/commands.rs"]
mod commands;
#[path = "../src/panels/inspector.rs"]
mod inspector;
#[path = "../src/jobs.rs"]
mod jobs;
#[path = "../src/panels/outliner.rs"]
mod outliner;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/viewport.rs"]
mod viewport;

use std::collections::{BTreeMap, BTreeSet};

use commands::AppCommand;
use inspector::{
    InspectorPanelState, grouped_parameter_sections, lock_command, parameter_group_command,
    sanitize_search_counts, scalar_command, target_scope_command,
};
use outliner::build_outliner_rows;
use shape_core::{
    NodeId, NodeKind, ParamDescriptor, ParamGroup, ParamPath, PrimitiveKind, ShapeDocument,
    ShapeNode, Transform3,
};
use shape_search::TargetScope;

#[test]
fn scalar_command_emits_only_for_unlocked_changes() {
    let descriptor = descriptor(
        NodeId(7),
        "primitive.radius",
        ParamGroup::Form,
        0.01,
        5.0,
        0.01,
    );

    assert_eq!(scalar_command(&descriptor, 1.0, 1.0, false), None);
    assert_eq!(scalar_command(&descriptor, 1.0, 1.5, true), None);
    assert_eq!(
        scalar_command(&descriptor, 1.0, 1.5, false),
        Some(AppCommand::SetScalar {
            path: descriptor.path.clone(),
            value: 1.5
        })
    );
}

#[test]
fn lock_and_scope_helpers_avoid_duplicate_commands() {
    let path = ParamPath {
        node: NodeId(3),
        key: "transform.scale.x".to_owned(),
    };

    assert_eq!(lock_command(&path, false, false), None);
    assert_eq!(
        lock_command(&path, false, true),
        Some(AppCommand::ToggleLock {
            path: path.clone(),
            locked: true
        })
    );

    assert_eq!(
        target_scope_command(TargetScope::Selected, TargetScope::Selected),
        None
    );
    assert_eq!(
        target_scope_command(TargetScope::Selected, TargetScope::WholeModel),
        Some(AppCommand::SetTargetScope(TargetScope::WholeModel))
    );
}

#[test]
fn parameter_group_command_tracks_enabled_set() {
    let mut enabled = BTreeSet::new();
    enabled.insert(ParamGroup::Form);

    assert_eq!(
        parameter_group_command(ParamGroup::Form, &enabled, true),
        None
    );
    assert_eq!(
        parameter_group_command(ParamGroup::Rotation, &enabled, true),
        Some(AppCommand::SetParameterGroup {
            group: ParamGroup::Rotation,
            enabled: true
        })
    );
    assert_eq!(
        parameter_group_command(ParamGroup::Form, &enabled, false),
        Some(AppCommand::SetParameterGroup {
            group: ParamGroup::Form,
            enabled: false
        })
    );
}

#[test]
fn grouped_parameters_follow_display_order() {
    let document = sample_document_with_smooth_group();
    let sections = grouped_parameter_sections(&document, NodeId(0));
    let groups = sections
        .iter()
        .map(|section| section.group)
        .collect::<Vec<_>>();

    assert_eq!(
        groups,
        vec![
            ParamGroup::Placement,
            ParamGroup::Rotation,
            ParamGroup::Scale,
            ParamGroup::Blend
        ]
    );

    let sphere_sections = grouped_parameter_sections(&document, NodeId(1));
    assert_eq!(sphere_sections[0].group, ParamGroup::Form);
    assert!(
        sphere_sections[0]
            .descriptors
            .iter()
            .any(|descriptor| descriptor.path.key == "primitive.radius")
    );
}

#[test]
fn outliner_marks_shared_dag_reference_once() {
    let mut document = ShapeDocument::new(
        "Shared",
        node(
            0,
            "Root",
            NodeKind::Union {
                children: vec![NodeId(1), NodeId(2)],
            },
        ),
    );
    document.nodes.insert(
        NodeId(1),
        node(
            1,
            "Shared sphere",
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.5 }),
        ),
    );
    document.nodes.insert(
        NodeId(2),
        node(
            2,
            "Nested group",
            NodeKind::Union {
                children: vec![NodeId(1)],
            },
        ),
    );
    document.next_node_id = 3;

    let rows = build_outliner_rows(&document);
    let shared_rows = rows
        .iter()
        .filter(|row| row.node == Some(NodeId(1)) && row.shared_reference)
        .collect::<Vec<_>>();

    assert_eq!(shared_rows.len(), 1);
    assert_eq!(shared_rows[0].depth, 2);
    assert_eq!(shared_rows[0].kind, "Sphere");
}

#[test]
fn search_counts_stay_within_safe_bounds() {
    let mut state = InspectorPanelState {
        proposal_count: 0,
        result_count: 99,
    };

    sanitize_search_counts(&mut state);

    assert_eq!(state.proposal_count, 1);
    assert_eq!(state.result_count, 1);
}

fn descriptor(
    node: NodeId,
    key: &str,
    group: ParamGroup,
    minimum: f32,
    maximum: f32,
    step: f32,
) -> ParamDescriptor {
    ParamDescriptor {
        path: ParamPath {
            node,
            key: key.to_owned(),
        },
        label: key.to_owned(),
        group,
        minimum,
        maximum,
        step,
        mutation_sigma: step,
    }
}

fn sample_document_with_smooth_group() -> ShapeDocument {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        NodeId(0),
        node(
            0,
            "Soft assembly",
            NodeKind::SmoothUnion {
                children: vec![NodeId(1)],
                smoothness: 0.1,
            },
        ),
    );
    nodes.insert(
        NodeId(1),
        node(
            1,
            "Sphere",
            NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.5 }),
        ),
    );
    ShapeDocument {
        schema_version: 1,
        title: "Soft".to_owned(),
        root: NodeId(0),
        nodes,
        next_node_id: 2,
        locks: BTreeSet::new(),
    }
}

fn node(id: u64, name: &str, kind: NodeKind) -> ShapeNode {
    ShapeNode {
        id: NodeId(id),
        name: name.to_owned(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Transform3::default(),
        kind,
    }
}
