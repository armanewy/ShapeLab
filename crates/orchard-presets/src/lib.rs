#![forbid(unsafe_code)]

//! Procedural preset contracts for the Box Primitive reset.

use std::collections::BTreeSet;

use orchard_core_legacy::{
    NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3, ValidationReport,
    validate_document,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! vec3 {
    ($x:expr, $y:expr, $z:expr) => {{
        let mut value = Transform3::default().translation;
        value.x = $x;
        value.y = $y;
        value.z = $z;
        value
    }};
}

macro_rules! transform {
    (($tx:expr, $ty:expr, $tz:expr), ($rx:expr, $ry:expr, $rz:expr), ($sx:expr, $sy:expr, $sz:expr)) => {
        Transform3 {
            translation: vec3!($tx, $ty, $tz),
            rotation_degrees: vec3!($rx, $ry, $rz),
            scale: vec3!($sx, $sy, $sz),
        }
    };
}

/// Built-in preset identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PresetId(pub String);

/// Human-facing preset metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetMetadata {
    /// Stable preset ID.
    pub id: PresetId,
    /// Display name.
    pub name: String,
    /// Short description.
    pub description: String,
}

/// Preset errors.
#[derive(Debug, Error)]
pub enum PresetError {
    /// Unknown preset.
    #[error("I do not know the preset named {0}. Choose Box Primitive.")]
    UnknownPreset(String),
    /// A built-in preset produced an invalid document.
    #[error("The {id} preset could not be opened because it has {issue_count} setup issue(s).")]
    InvalidPreset {
        /// Preset identifier.
        id: String,
        /// Number of validation issues.
        issue_count: usize,
        /// Full validation report from orchard-core-legacy.
        report: ValidationReport,
    },
}

/// List built-in presets in stable display order.
#[must_use]
pub fn list_presets() -> Vec<PresetMetadata> {
    vec![metadata(
        "box-primitive",
        "Box Primitive",
        "Closed box-like volume with editable proportions and edge softness.",
    )]
}

/// Build a preset document.
pub fn build_preset(id: &PresetId) -> Result<ShapeDocument, PresetError> {
    let document = match id.0.as_str() {
        "box-primitive" => box_primitive(),
        _ => return Err(PresetError::UnknownPreset(id.0.clone())),
    };
    let report = validate_document(&document);
    if report.is_valid() {
        Ok(document)
    } else {
        Err(PresetError::InvalidPreset {
            id: id.0.clone(),
            issue_count: report.issues.len(),
            report,
        })
    }
}

fn metadata(id: &str, name: &str, description: &str) -> PresetMetadata {
    PresetMetadata {
        id: PresetId(id.to_owned()),
        name: name.to_owned(),
        description: description.to_owned(),
    }
}

fn box_primitive() -> ShapeDocument {
    let mut builder = DocumentBuilder::new("Box Primitive");
    let body = builder.add_node(
        "Closed box body",
        &["box", "body", "focus"],
        transform!((0.0, 0.62, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.78, 0.5, 0.58),
            roundness: 0.08,
        }),
    );
    builder.set_root(
        "Box Primitive",
        &["box", "primitive", "preset"],
        NodeKind::Union {
            children: vec![body],
        },
    );
    builder.finish()
}

struct DocumentBuilder {
    document: ShapeDocument,
}

impl DocumentBuilder {
    fn new(title: &str) -> Self {
        let root = ShapeNode {
            id: NodeId(1),
            name: title.to_owned(),
            tags: tags(&["preset"]),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Union {
                children: Vec::new(),
            },
        };
        Self {
            document: ShapeDocument::new(title.to_owned(), root),
        }
    }

    fn add_node(
        &mut self,
        name: &str,
        tag_values: &[&str],
        transform: Transform3,
        kind: NodeKind,
    ) -> NodeId {
        let id = NodeId(self.document.next_node_id);
        self.document.next_node_id = self.document.next_node_id.saturating_add(1);
        self.document.nodes.insert(
            id,
            ShapeNode {
                id,
                name: name.to_owned(),
                tags: tags(tag_values),
                enabled: true,
                transform,
                kind,
            },
        );
        id
    }

    fn set_root(&mut self, name: &str, tag_values: &[&str], kind: NodeKind) {
        if let Some(root) = self.document.nodes.get_mut(&self.document.root) {
            root.name = name.to_owned();
            root.tags = tags(tag_values);
            root.kind = kind;
        }
    }

    fn finish(self) -> ShapeDocument {
        self.document
    }
}

fn tags(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use orchard_core_legacy::{NodeKind, PrimitiveKind, descendants_of, validate_document};

    use super::{PresetError, PresetId, build_preset, list_presets};

    #[test]
    fn stable_preset_listing_only_exposes_box_primitive() {
        let ids: Vec<String> = list_presets()
            .into_iter()
            .map(|preset| preset.id.0)
            .collect();

        assert_eq!(ids, vec!["box-primitive"]);
    }

    #[test]
    fn unknown_preset_is_rejected() {
        assert!(matches!(
            build_preset(&PresetId("missing".to_owned())),
            Err(PresetError::UnknownPreset(id)) if id == "missing"
        ));
    }

    #[test]
    fn box_primitive_preset_validates() {
        let document = build_preset(&PresetId("box-primitive".to_owned())).unwrap();
        assert!(validate_document(&document).is_valid());
        assert!(document.nodes.contains_key(&document.root));
    }

    #[test]
    fn box_primitive_root_reaches_every_node() {
        let document = build_preset(&PresetId("box-primitive".to_owned())).unwrap();
        let mut reachable: BTreeSet<_> = descendants_of(&document, document.root)
            .unwrap()
            .into_iter()
            .collect();
        reachable.insert(document.root);
        let all_nodes: BTreeSet<_> = document.nodes.keys().copied().collect();
        assert_eq!(reachable, all_nodes);
    }

    #[test]
    fn box_primitive_uses_one_rounded_box_body() {
        let document = build_preset(&PresetId("box-primitive".to_owned())).unwrap();
        let rounded_boxes = document
            .nodes
            .values()
            .filter(|node| {
                matches!(
                    node.kind,
                    NodeKind::Primitive(PrimitiveKind::RoundedBox { .. })
                )
            })
            .count();
        assert_eq!(rounded_boxes, 1);
        assert_eq!(document.nodes.len(), 2);
    }

    #[test]
    fn box_primitive_has_beginner_facing_names_and_tags() {
        let document = build_preset(&PresetId("box-primitive".to_owned())).unwrap();
        for node in document.nodes.values() {
            let name = node.name.to_lowercase();
            for forbidden in ["mesh", "topology", "kernel", "provider", "slot"] {
                assert!(
                    !name.contains(forbidden),
                    "{forbidden} leaked in {}",
                    node.name
                );
            }
            assert!(!node.tags.is_empty(), "{} should expose tags", node.name);
        }
    }
}
