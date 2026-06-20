//! Shape graph outliner panel.

#![allow(dead_code)]

use std::collections::{BTreeSet, VecDeque};

use egui::RichText;
use shape_core::{NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode};

use crate::commands::AppCommand;
use crate::state::AppState;

/// A flattened, testable row in the shape graph outliner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutlinerRow {
    pub node: Option<NodeId>,
    pub depth: usize,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub shared_reference: bool,
}

/// Render the shape graph outliner and return commands requested by the user.
pub(crate) fn show(ui: &mut egui::Ui, state: &AppState) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    ui.heading("Parts");
    ui.label("Pick the whole model or a named part before generating options.");

    let Ok(document) = state.project.current_document() else {
        ui.weak("The current history step is unavailable.");
        return commands;
    };

    let rows = build_outliner_rows(document);
    for row in rows {
        render_row(ui, state.selected_node, &row, &mut commands);
    }
    commands
}

/// Build stable outliner rows without touching egui.
#[must_use]
pub(crate) fn build_outliner_rows(document: &ShapeDocument) -> Vec<OutlinerRow> {
    let mut rows = vec![OutlinerRow {
        node: None,
        depth: 0,
        name: "Whole Model".to_owned(),
        kind: format!("{} starting point", document.title),
        tags: Vec::new(),
        enabled: true,
        shared_reference: false,
    }];

    let mut expanded = BTreeSet::new();
    append_node_rows(document, document.root, 0, &mut expanded, &mut rows);
    rows
}

/// Return child references in their document order.
#[must_use]
pub(crate) fn child_ids(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Primitive(_) => Vec::new(),
        NodeKind::Union { children }
        | NodeKind::SmoothUnion { children, .. }
        | NodeKind::Intersection { children } => children.clone(),
        NodeKind::Difference { base, subtractors } => {
            let mut ids = Vec::with_capacity(subtractors.len() + 1);
            ids.push(*base);
            ids.extend(subtractors.iter().copied());
            ids
        }
    }
}

/// Human-facing node kind label for the outliner.
#[must_use]
pub(crate) fn kind_label(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Primitive(primitive) => primitive_kind_label(primitive),
        NodeKind::Union { .. } => "Group",
        NodeKind::SmoothUnion { .. } => "Soft group",
        NodeKind::Difference { .. } => "Cut group",
        NodeKind::Intersection { .. } => "Overlap group",
    }
}

fn append_node_rows(
    document: &ShapeDocument,
    node_id: NodeId,
    depth: usize,
    expanded: &mut BTreeSet<NodeId>,
    rows: &mut Vec<OutlinerRow>,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        rows.push(OutlinerRow {
            node: Some(node_id),
            depth,
            name: format!("Missing node {}", node_id.0),
            kind: "Missing".to_owned(),
            tags: Vec::new(),
            enabled: false,
            shared_reference: false,
        });
        return;
    };

    let shared_reference = !expanded.insert(node_id);
    rows.push(row_from_node(node, depth, shared_reference));
    if shared_reference {
        return;
    }

    let mut pending = VecDeque::from(child_ids(&node.kind));
    while let Some(child) = pending.pop_front() {
        append_node_rows(document, child, depth + 1, expanded, rows);
    }
}

fn row_from_node(node: &ShapeNode, depth: usize, shared_reference: bool) -> OutlinerRow {
    OutlinerRow {
        node: Some(node.id),
        depth,
        name: node.name.clone(),
        kind: kind_label(&node.kind).to_owned(),
        tags: node.tags.iter().cloned().collect(),
        enabled: node.enabled,
        shared_reference,
    }
}

fn render_row(
    ui: &mut egui::Ui,
    selected_node: Option<NodeId>,
    row: &OutlinerRow,
    commands: &mut Vec<AppCommand>,
) {
    ui.horizontal(|ui| {
        ui.add_space((row.depth as f32) * 14.0);

        let selected = selected_node == row.node;
        let mut label = RichText::new(row.name.clone());
        if !row.enabled {
            label = label.italics().color(ui.visuals().weak_text_color());
        }
        if selected {
            label = label.strong();
        }

        let response = ui
            .selectable_label(selected, label)
            .on_hover_text(row_hover_text(row));
        if response.clicked() && selected_node != row.node {
            commands.push(AppCommand::SelectNode(row.node));
        }

        let mut kind = RichText::new(display_kind_label(&row.kind)).small();
        if !row.enabled {
            kind = kind.color(ui.visuals().weak_text_color());
        }
        ui.label(kind);

        if row.shared_reference {
            ui.label(
                RichText::new("shared")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .on_hover_text("This part is reused elsewhere, so it is not expanded again.");
        }

        if !row.enabled {
            ui.label(
                RichText::new("off")
                    .small()
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
        }

        for tag in &row.tags {
            ui.label(RichText::new(format!("#{tag}")).small());
        }
    });
}

fn row_hover_text(row: &OutlinerRow) -> String {
    if row.node.is_none() {
        return "Select this when the next options should be allowed to change the whole model."
            .to_owned();
    }

    let mut parts = vec![display_kind_label(&row.kind).to_owned()];
    if row.shared_reference {
        parts.push("shared reference".to_owned());
    }
    if !row.enabled {
        parts.push("currently disabled".to_owned());
    }
    if !row.tags.is_empty() {
        parts.push(format!("tags: {}", row.tags.join(", ")));
    }
    parts.join(" | ")
}

fn primitive_kind_label(kind: &PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Sphere { .. } => "Sphere",
        PrimitiveKind::RoundedBox { .. } => "Rounded box",
        PrimitiveKind::Capsule { .. } => "Capsule",
        PrimitiveKind::Cylinder { .. } => "Cylinder",
        PrimitiveKind::Torus { .. } => "Ring",
    }
}

fn display_kind_label(kind: &str) -> &str {
    match kind {
        "Sphere" => "Round part",
        "Rounded box" => "Rounded block",
        "Capsule" => "Long rounded part",
        "Cylinder" => "Round column",
        "Group" => "Part group",
        "Soft group" => "Blended group",
        "Cut group" => "Hollowed group",
        other => other,
    }
}
