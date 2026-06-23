//! Build stamp and compiled-output contract types.

use serde::{Deserialize, Serialize};
use shape_family_compile::identity::{
    ArtifactFingerprint, BuildFingerprint, GeometryInputFingerprint, RecipeFingerprint,
};

/// Deterministic build stamp emitted after foundry compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryBuildStamp {
    /// Geometry input fingerprint used for the build.
    pub geometry_input_fingerprint: GeometryInputFingerprint,
    /// Build fingerprint including conformance contracts and compiler versions.
    pub build_fingerprint: BuildFingerprint,
    /// Exact generated recipe fingerprint.
    pub recipe_fingerprint: RecipeFingerprint,
    /// Compiled artifact fingerprint.
    pub artifact_fingerprint: ArtifactFingerprint,
    /// Shape Foundry crate version.
    pub foundry_version: String,
    /// Shape Family Compile crate version.
    pub family_compile_version: String,
}

/// Exact generated recipe snapshot persisted beside semantic foundry sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedRecipeSnapshot {
    /// Recipe schema version.
    pub schema_version: u32,
    /// Canonical JSON recipe payload.
    pub canonical_json: String,
    /// Fingerprint of the recipe payload.
    pub recipe_fingerprint: RecipeFingerprint,
}
