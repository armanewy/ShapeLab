#![forbid(unsafe_code)]

//! Procedural non-humanoid preset contracts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shape_core::{
    NodeId, NodeKind, PrimitiveKind, ShapeDocument, ShapeNode, Transform3, ValidationReport,
    validate_document,
};
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
    #[error("I do not know the preset named {0}. Choose one from New From Preset.")]
    UnknownPreset(String),
    /// A built-in preset produced an invalid document.
    #[error("The {id} preset could not be opened because it has {issue_count} setup issue(s).")]
    InvalidPreset {
        /// Preset identifier.
        id: String,
        /// Number of validation issues.
        issue_count: usize,
        /// Full validation report from shape-core.
        report: ValidationReport,
    },
}

/// List built-in presets in stable display order.
#[must_use]
pub fn list_presets() -> Vec<PresetMetadata> {
    vec![
        metadata(
            "desk-lamp",
            "Desk Lamp",
            "Everyday desk lamp with a heavy foot, two supports, and a wide shade.",
        ),
        metadata(
            "toy-submarine",
            "Toy Submarine",
            "Playful submarine with round window hollows, a top tower, and tail fins.",
        ),
        metadata(
            "alien-plant",
            "Alien Plant",
            "Stylized plant with a ground bulb, bending stalks, pods, and leaf blades.",
        ),
        metadata(
            "sky-shrine",
            "Sky Shrine",
            "Small architectural shrine with steps, pillars, a roof, and a floating ring.",
        ),
    ]
}

/// Build a preset document.
pub fn build_preset(id: &PresetId) -> Result<ShapeDocument, PresetError> {
    let document = match id.0.as_str() {
        "desk-lamp" => desk_lamp(),
        "toy-submarine" => toy_submarine(),
        "alien-plant" => alien_plant(),
        "sky-shrine" => sky_shrine(),
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

fn desk_lamp() -> ShapeDocument {
    let mut builder = DocumentBuilder::new("Desk Lamp");
    let base = builder.add_node(
        "Heavy round foot",
        &["base", "bottom", "stable"],
        transform!((0.0, 0.08, 0.0), (0.0, 0.0, 0.0), (1.15, 0.28, 1.15)),
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.22,
            radius: 0.58,
            roundness: 0.04,
        }),
    );
    let base_ring = builder.add_node(
        "Raised foot rim",
        &["base", "detail", "rim"],
        transform!((0.0, 0.23, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Torus {
            major_radius: 0.5,
            minor_radius: 0.045,
        }),
    );
    let lower_stem = builder.add_node(
        "Lower support bar",
        &["bar", "lower", "support"],
        transform!((-0.24, 0.78, 0.0), (0.0, 0.0, -18.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 0.62,
            radius: 0.07,
        }),
    );
    let hinge = builder.add_node(
        "Round elbow joint",
        &["joint", "pivot", "support"],
        transform!((-0.42, 1.34, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.15 }),
    );
    let upper_stem = builder.add_node(
        "Upper support bar",
        &["bar", "support", "upper"],
        transform!((0.08, 1.63, 0.0), (0.0, 0.0, 36.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 0.56,
            radius: 0.06,
        }),
    );
    let stem_blend = builder.add_node(
        "Adjustable support",
        &["focus", "support", "tune"],
        Transform3::default(),
        NodeKind::SmoothUnion {
            children: vec![lower_stem, hinge, upper_stem],
            smoothness: 0.12,
        },
    );
    let shade = builder.add_node(
        "Wide lamp shade",
        &["focus", "shade", "top"],
        transform!((0.54, 2.1, 0.0), (0.0, 0.0, -8.0), (1.15, 0.78, 0.92)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.48, 0.24, 0.42),
            roundness: 0.16,
        }),
    );
    builder.set_root(
        "Whole desk lamp",
        &["everyday", "lamp", "preset"],
        NodeKind::Union {
            children: vec![base, base_ring, stem_blend, shade],
        },
    );
    builder.finish()
}

fn toy_submarine() -> ShapeDocument {
    let mut builder = DocumentBuilder::new("Toy Submarine");
    let hull = builder.add_node(
        "Rounded main body",
        &["body", "focus", "main"],
        transform!((0.0, 0.0, 0.0), (0.0, 0.0, 90.0), (1.0, 0.92, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 1.05,
            radius: 0.36,
        }),
    );
    let port_a = builder.add_node(
        "Front window hollow",
        &["front", "opening", "window"],
        transform!((0.48, 0.04, -0.31), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.13 }),
    );
    let port_b = builder.add_node(
        "Middle window hollow",
        &["middle", "opening", "window"],
        transform!((0.0, 0.04, -0.33), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.13 }),
    );
    let port_c = builder.add_node(
        "Back window hollow",
        &["back", "opening", "window"],
        transform!((-0.48, 0.04, -0.31), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.13 }),
    );
    let hull_with_ports = builder.add_node(
        "Body with window hollows",
        &["body", "main", "windows"],
        Transform3::default(),
        NodeKind::Difference {
            base: hull,
            subtractors: vec![port_a, port_b, port_c],
        },
    );
    let tower = builder.add_node(
        "Top lookout tower",
        &["focus", "top", "tower"],
        transform!((0.02, 0.48, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.34, 0.28, 0.23),
            roundness: 0.08,
        }),
    );
    let periscope = builder.add_node(
        "Small lookout pipe",
        &["detail", "pipe", "top"],
        transform!((0.02, 0.82, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.22,
            radius: 0.045,
            roundness: 0.015,
        }),
    );
    let top_fin = builder.add_node(
        "Top tail fin",
        &["fin", "tail", "top"],
        transform!((-1.0, 0.36, 0.0), (0.0, 0.0, -12.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.28, 0.09, 0.055),
            roundness: 0.025,
        }),
    );
    let left_fin = builder.add_node(
        "Left side fin",
        &["fin", "left", "side"],
        transform!((-0.98, -0.04, -0.38), (0.0, 18.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.24, 0.055, 0.035),
            roundness: 0.018,
        }),
    );
    let right_fin = builder.add_node(
        "Right side fin",
        &["fin", "right", "side"],
        transform!((-0.98, -0.04, 0.38), (0.0, -18.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.24, 0.055, 0.035),
            roundness: 0.018,
        }),
    );
    builder.set_root(
        "Whole toy submarine",
        &["preset", "submarine", "toy"],
        NodeKind::SmoothUnion {
            children: vec![
                hull_with_ports,
                tower,
                periscope,
                top_fin,
                left_fin,
                right_fin,
            ],
            smoothness: 0.08,
        },
    );
    builder.finish()
}

fn alien_plant() -> ShapeDocument {
    let mut builder = DocumentBuilder::new("Alien Plant");
    let base_bulb = builder.add_node(
        "Ground bulb",
        &["base", "bulb", "bottom"],
        transform!((0.0, 0.22, 0.0), (0.0, 0.0, 0.0), (1.25, 0.72, 1.25)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.38 }),
    );
    let central_stem = builder.add_node(
        "Tall center stalk",
        &["focus", "stalk", "support"],
        transform!((0.0, 0.9, 0.0), (0.0, 0.0, 4.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 0.74,
            radius: 0.1,
        }),
    );
    let left_branch = builder.add_node(
        "Left bending branch",
        &["branch", "left", "support"],
        transform!((-0.32, 1.32, 0.0), (0.0, 0.0, 46.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 0.42,
            radius: 0.065,
        }),
    );
    let right_branch = builder.add_node(
        "Right bending branch",
        &["branch", "right", "support"],
        transform!((0.34, 1.43, 0.0), (0.0, 0.0, -43.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Capsule {
            half_length: 0.45,
            radius: 0.065,
        }),
    );
    let top_pod = builder.add_node(
        "Large top pod",
        &["focus", "pod", "top"],
        transform!((0.03, 1.76, 0.0), (0.0, 0.0, 0.0), (1.0, 1.18, 0.86)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.32 }),
    );
    let left_pod = builder.add_node(
        "Left small pod",
        &["branch", "left", "pod"],
        transform!((-0.67, 1.62, 0.0), (0.0, 0.0, 0.0), (1.0, 1.08, 0.9)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.17 }),
    );
    let right_pod = builder.add_node(
        "Right small pod",
        &["branch", "pod", "right"],
        transform!((0.7, 1.74, 0.0), (0.0, 0.0, 0.0), (1.0, 1.08, 0.9)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.18 }),
    );
    let left_leaf = builder.add_node(
        "Lower left leaf blade",
        &["blade", "bottom", "left"],
        transform!((-0.34, 0.48, 0.24), (0.0, 18.0, 28.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.3, 0.045, 0.12),
            roundness: 0.04,
        }),
    );
    let right_leaf = builder.add_node(
        "Lower right leaf blade",
        &["blade", "bottom", "right"],
        transform!((0.36, 0.54, -0.22), (0.0, -22.0, -26.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.32, 0.045, 0.12),
            roundness: 0.04,
        }),
    );
    builder.set_root(
        "Whole alien plant",
        &["organic", "plant", "preset"],
        NodeKind::SmoothUnion {
            children: vec![
                base_bulb,
                central_stem,
                left_branch,
                right_branch,
                top_pod,
                left_pod,
                right_pod,
                left_leaf,
                right_leaf,
            ],
            smoothness: 0.11,
        },
    );
    builder.finish()
}

fn sky_shrine() -> ShapeDocument {
    let mut builder = DocumentBuilder::new("Sky Shrine");
    let lower_step = builder.add_node(
        "Wide lower step",
        &["base", "bottom", "step"],
        transform!((0.0, 0.08, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.78, 0.08, 0.52),
            roundness: 0.035,
        }),
    );
    let upper_step = builder.add_node(
        "Raised upper step",
        &["base", "middle", "step"],
        transform!((0.0, 0.23, 0.0), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.58, 0.08, 0.38),
            roundness: 0.035,
        }),
    );
    let left_pillar = builder.add_node(
        "Left front pillar",
        &["front", "left", "pillar"],
        transform!((-0.34, 0.72, -0.2), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.45,
            radius: 0.075,
            roundness: 0.02,
        }),
    );
    let right_pillar = builder.add_node(
        "Right front pillar",
        &["front", "pillar", "right"],
        transform!((0.34, 0.72, -0.2), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.45,
            radius: 0.075,
            roundness: 0.02,
        }),
    );
    let back_pillar = builder.add_node(
        "Back center pillar",
        &["back", "pillar", "support"],
        transform!((0.0, 0.72, 0.24), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.42,
            radius: 0.065,
            roundness: 0.018,
        }),
    );
    let top_beam = builder.add_node(
        "Flat top beam",
        &["beam", "support", "top"],
        transform!((0.0, 1.18, -0.06), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.54, 0.075, 0.18),
            roundness: 0.03,
        }),
    );
    let roof = builder.add_node(
        "Lifted roof slab",
        &["focus", "roof", "top"],
        transform!((0.0, 1.34, -0.02), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::RoundedBox {
            half_extents: vec3!(0.66, 0.09, 0.34),
            roundness: 0.045,
        }),
    );
    let floating_ring = builder.add_node(
        "Floating halo ring",
        &["focus", "halo", "symbol"],
        transform!((0.0, 1.0, -0.36), (90.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Torus {
            major_radius: 0.22,
            minor_radius: 0.035,
        }),
    );
    let center_gem = builder.add_node(
        "Center light bead",
        &["focus", "light", "symbol"],
        transform!((0.0, 1.0, -0.36), (0.0, 0.0, 0.0), (1.0, 1.0, 1.0)),
        NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.09 }),
    );
    builder.set_root(
        "Whole sky shrine",
        &["architecture", "preset", "shrine"],
        NodeKind::Union {
            children: vec![
                lower_step,
                upper_step,
                left_pillar,
                right_pillar,
                back_pillar,
                top_beam,
                roof,
                floating_ring,
                center_gem,
            ],
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
    use std::collections::{BTreeMap, BTreeSet};

    use shape_core::{NodeKind, descendants_of, validate_document};

    use super::{PresetError, PresetId, build_preset, list_presets};

    #[test]
    fn stable_preset_listing() {
        let ids: Vec<String> = list_presets()
            .into_iter()
            .map(|preset| preset.id.0)
            .collect();

        assert_eq!(
            ids,
            vec!["desk-lamp", "toy-submarine", "alien-plant", "sky-shrine"]
        );
    }

    #[test]
    fn unknown_preset_is_rejected() {
        assert!(matches!(
            build_preset(&PresetId("missing".to_owned())),
            Err(PresetError::UnknownPreset(id)) if id == "missing"
        ));
    }

    #[test]
    fn all_presets_validate() {
        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            assert!(
                validate_document(&document).is_valid(),
                "{} should validate",
                preset.id.0
            );
            assert!(document.nodes.contains_key(&document.root));
        }
    }

    #[test]
    fn all_preset_roots_reach_every_node() {
        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            let mut reachable: BTreeSet<_> = descendants_of(&document, document.root)
                .unwrap()
                .into_iter()
                .collect();
            reachable.insert(document.root);
            let all_nodes: BTreeSet<_> = document.nodes.keys().copied().collect();
            assert_eq!(
                reachable, all_nodes,
                "{} has unreachable nodes",
                preset.id.0
            );
        }
    }

    #[test]
    fn presets_are_geometrically_and_structurally_distinct() {
        let mut signatures = BTreeSet::new();
        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            signatures.insert((
                document.nodes.len(),
                csg_counts(&document),
                primitive_counts(&document),
            ));
        }

        assert_eq!(signatures.len(), 4);
    }

    #[test]
    fn presets_demonstrate_required_graph_operations() {
        let mut has_union = false;
        let mut has_smooth_union = false;
        let mut has_difference_or_intersection = false;

        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            for node in document.nodes.values() {
                match node.kind {
                    NodeKind::Union { .. } => has_union = true,
                    NodeKind::SmoothUnion { .. } => has_smooth_union = true,
                    NodeKind::Difference { .. } | NodeKind::Intersection { .. } => {
                        has_difference_or_intersection = true;
                    }
                    NodeKind::Primitive(_) => {}
                }
            }
        }

        assert!(has_union);
        assert!(has_smooth_union);
        assert!(has_difference_or_intersection);
    }

    #[test]
    fn presets_contain_no_humanoid_terms() {
        let forbidden = ["humanoid", "head", "torso", "arm", "leg"];
        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            for node in document.nodes.values() {
                let name = node.name.to_lowercase();
                assert!(
                    forbidden.iter().all(|term| !name.contains(term)),
                    "{} has humanoid term in node name {}",
                    preset.id.0,
                    node.name
                );
                for tag in &node.tags {
                    let tag = tag.to_lowercase();
                    assert!(
                        forbidden.iter().all(|term| !tag.contains(term)),
                        "{} has humanoid term in tag {tag}",
                        preset.id.0
                    );
                }
            }
        }
    }

    #[test]
    fn presets_have_beginner_facing_part_names_and_useful_tags() {
        let geometry_terms = ["capsule", "cylinder", "torus", "cutter", "primitive"];
        for preset in list_presets() {
            let document = build_preset(&preset.id).unwrap();
            let mut has_focus_tag = false;
            for node in document.nodes.values() {
                let name = node.name.to_lowercase();
                assert!(
                    geometry_terms.iter().all(|term| !name.contains(term)),
                    "{} has geometry term in node name {}",
                    preset.id.0,
                    node.name
                );
                assert!(
                    !node.tags.is_empty(),
                    "{} node {} should expose outliner tags",
                    preset.id.0,
                    node.name
                );
                has_focus_tag |= node.tags.contains("focus");
            }
            assert!(
                has_focus_tag,
                "{} should mark at least one focus part",
                preset.id.0
            );
        }
    }

    fn csg_counts(document: &shape_core::ShapeDocument) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for node in document.nodes.values() {
            let key = match node.kind {
                NodeKind::Union { .. } => "union",
                NodeKind::SmoothUnion { .. } => "smooth_union",
                NodeKind::Difference { .. } => "difference",
                NodeKind::Intersection { .. } => "intersection",
                NodeKind::Primitive(_) => "primitive",
            };
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    fn primitive_counts(document: &shape_core::ShapeDocument) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for node in document.nodes.values() {
            if let NodeKind::Primitive(kind) = &node.kind {
                let key = match kind {
                    shape_core::PrimitiveKind::Sphere { .. } => "sphere",
                    shape_core::PrimitiveKind::RoundedBox { .. } => "rounded_box",
                    shape_core::PrimitiveKind::Capsule { .. } => "capsule",
                    shape_core::PrimitiveKind::Cylinder { .. } => "cylinder",
                    shape_core::PrimitiveKind::Torus { .. } => "torus",
                };
                *counts.entry(key).or_insert(0) += 1;
            }
        }
        counts
    }
}
