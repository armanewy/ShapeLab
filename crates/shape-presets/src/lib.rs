#![forbid(unsafe_code)]

//! Procedural non-humanoid preset contracts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shape_core::{NodeId, PrimitiveKind, ShapeDocument, ShapeNode, Transform3};
use thiserror::Error;

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
    #[error("unknown preset {0}")]
    UnknownPreset(String),
}

/// List built-in presets.
#[must_use]
pub fn list_presets() -> Vec<PresetMetadata> {
    vec![
        PresetMetadata {
            id: PresetId("desk-lamp".to_owned()),
            name: "Desk Lamp".to_owned(),
            description: "A simple adjustable desk lamp.".to_owned(),
        },
        PresetMetadata {
            id: PresetId("toy-submarine".to_owned()),
            name: "Toy Submarine".to_owned(),
            description: "A rounded toy submarine.".to_owned(),
        },
        PresetMetadata {
            id: PresetId("alien-plant".to_owned()),
            name: "Alien Plant".to_owned(),
            description: "A stylized plant-like object.".to_owned(),
        },
    ]
}

/// Build a preset document. Wave 1 replaces this bootstrap placeholder.
pub fn build_preset(id: &PresetId) -> Result<ShapeDocument, PresetError> {
    if !list_presets().iter().any(|preset| &preset.id == id) {
        return Err(PresetError::UnknownPreset(id.0.clone()));
    }
    let root = ShapeNode {
        id: NodeId(1),
        name: id.0.clone(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Transform3::default(),
        kind: shape_core::NodeKind::Primitive(PrimitiveKind::Sphere { radius: 1.0 }),
    };
    Ok(ShapeDocument::new(id.0.clone(), root))
}
