#![forbid(unsafe_code)]

//! Runtime-neutral asset layer lineage contracts.
//!
//! These helpers describe how mesh, surface, rig, motion, collision, and export
//! artifacts bind to one another. They intentionally do not perform mesh edits
//! or dependency graph execution.

use serde::{Deserialize, Serialize};

/// Current schema version for game asset layer references.
pub const GAME_ASSET_LAYER_REF_SCHEMA_VERSION: u32 = 1;

/// Runtime-neutral asset layers whose stale state can be reasoned about.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameAssetLayerKind {
    /// Frozen mesh/topology layer.
    Mesh,
    /// Surface UV/material/texture layer.
    Surface,
    /// Rig, skeleton, pivots, sockets, or skin binding layer.
    Rig,
    /// Motion clip layer.
    Motion,
    /// Collision proxy layer.
    Collision,
    /// Portable/export package layer.
    Export,
}

/// Stable reference from one game-asset layer to its source artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameAssetLayerRef {
    /// Reference schema version.
    pub schema_version: u32,
    /// Layer described by this reference.
    pub layer: GameAssetLayerKind,
    /// Package-relative artifact reference.
    pub artifact_ref: String,
    /// Frozen mesh fingerprint this layer was authored against.
    pub frozen_mesh_fingerprint: String,
    /// Frozen topology fingerprint this layer was authored against.
    pub topology_fingerprint: String,
    /// Parent layer artifact reference, when this layer depends on another.
    #[serde(default)]
    pub parent_artifact_ref: Option<String>,
}

impl GameAssetLayerRef {
    /// Construct a layer reference with the current schema version.
    #[must_use]
    pub fn new(
        layer: GameAssetLayerKind,
        artifact_ref: impl Into<String>,
        frozen_mesh_fingerprint: impl Into<String>,
        topology_fingerprint: impl Into<String>,
        parent_artifact_ref: Option<String>,
    ) -> Self {
        Self {
            schema_version: GAME_ASSET_LAYER_REF_SCHEMA_VERSION,
            layer,
            artifact_ref: artifact_ref.into(),
            frozen_mesh_fingerprint: frozen_mesh_fingerprint.into(),
            topology_fingerprint: topology_fingerprint.into(),
            parent_artifact_ref,
        }
    }
}

/// Result of checking whether a layer reference is structurally usable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameAssetLayerValidationReport {
    /// True when all required fields are present and the schema is supported.
    pub valid: bool,
    /// Stable issue codes.
    pub issue_codes: Vec<String>,
}

/// Validate a single layer reference.
#[must_use]
pub fn validate_game_asset_layer_ref(
    layer_ref: &GameAssetLayerRef,
) -> GameAssetLayerValidationReport {
    let mut issue_codes = Vec::new();
    if layer_ref.schema_version != GAME_ASSET_LAYER_REF_SCHEMA_VERSION {
        issue_codes.push("unsupported_game_asset_layer_ref_schema".to_owned());
    }
    if layer_ref.artifact_ref.trim().is_empty() {
        issue_codes.push("missing_layer_artifact_ref".to_owned());
    }
    if layer_ref.frozen_mesh_fingerprint.trim().is_empty() {
        issue_codes.push("missing_layer_frozen_mesh_fingerprint".to_owned());
    }
    if layer_ref.topology_fingerprint.trim().is_empty() {
        issue_codes.push("missing_layer_topology_fingerprint".to_owned());
    }
    GameAssetLayerValidationReport {
        valid: issue_codes.is_empty(),
        issue_codes,
    }
}

/// Return true when a surface artifact still binds to the provided frozen mesh.
#[must_use]
pub fn surface_binds_to_frozen_mesh(
    surface: &GameAssetLayerRef,
    frozen_mesh_fingerprint: &str,
) -> bool {
    surface.layer == GameAssetLayerKind::Surface
        && !frozen_mesh_fingerprint.trim().is_empty()
        && surface.frozen_mesh_fingerprint == frozen_mesh_fingerprint
}

/// Return the layers invalidated by a frozen mesh or topology change.
#[must_use]
pub fn layers_invalidated_by_frozen_topology_change(
    old_mesh_fingerprint: &str,
    old_topology_fingerprint: &str,
    new_mesh_fingerprint: &str,
    new_topology_fingerprint: &str,
) -> Vec<GameAssetLayerKind> {
    if old_mesh_fingerprint == new_mesh_fingerprint
        && old_topology_fingerprint == new_topology_fingerprint
    {
        return Vec::new();
    }
    vec![
        GameAssetLayerKind::Surface,
        GameAssetLayerKind::Rig,
        GameAssetLayerKind::Motion,
        GameAssetLayerKind::Export,
    ]
}

/// Return true when a material-only surface variant leaves rig and motion
/// lineage valid.
#[must_use]
pub fn material_only_variant_preserves_rig_motion(
    base_surface: &GameAssetLayerRef,
    variant_surface: &GameAssetLayerRef,
) -> bool {
    base_surface.layer == GameAssetLayerKind::Surface
        && variant_surface.layer == GameAssetLayerKind::Surface
        && base_surface.frozen_mesh_fingerprint == variant_surface.frozen_mesh_fingerprint
        && base_surface.topology_fingerprint == variant_surface.topology_fingerprint
}

/// Return true when a motion layer is bound to the referenced rig artifact.
#[must_use]
pub fn motion_binds_to_rig(motion: &GameAssetLayerRef, rig_artifact_ref: &str) -> bool {
    motion.layer == GameAssetLayerKind::Motion
        && motion
            .parent_artifact_ref
            .as_deref()
            .is_some_and(|parent| parent == rig_artifact_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_only_variant_preserves_rig_motion_lineage() {
        let base = GameAssetLayerRef::new(
            GameAssetLayerKind::Surface,
            "surface/base.json",
            "mesh:abc",
            "topology:abc",
            Some("mesh/frozen.obj".to_owned()),
        );
        let variant = GameAssetLayerRef::new(
            GameAssetLayerKind::Surface,
            "surface/variants/clean/surface-artifact.json",
            "mesh:abc",
            "topology:abc",
            Some("mesh/frozen.obj".to_owned()),
        );

        assert!(material_only_variant_preserves_rig_motion(&base, &variant));
        assert!(surface_binds_to_frozen_mesh(&variant, "mesh:abc"));
    }

    #[test]
    fn frozen_topology_change_invalidates_dependent_layers() {
        assert_eq!(
            layers_invalidated_by_frozen_topology_change("mesh:a", "topo:a", "mesh:b", "topo:a"),
            vec![
                GameAssetLayerKind::Surface,
                GameAssetLayerKind::Rig,
                GameAssetLayerKind::Motion,
                GameAssetLayerKind::Export
            ]
        );
        assert!(
            layers_invalidated_by_frozen_topology_change("mesh:a", "topo:a", "mesh:a", "topo:a")
                .is_empty()
        );
    }

    #[test]
    fn motion_layer_binds_to_rig_artifact_ref() {
        let motion = GameAssetLayerRef::new(
            GameAssetLayerKind::Motion,
            "motion/walk.json",
            "mesh:abc",
            "topology:abc",
            Some("rig/rig-artifact.json".to_owned()),
        );

        assert!(motion_binds_to_rig(&motion, "rig/rig-artifact.json"));
        assert!(!motion_binds_to_rig(&motion, "rig/other.json"));
    }
}
