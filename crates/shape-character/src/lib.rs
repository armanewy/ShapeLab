#![forbid(unsafe_code)]

//! Versioned semantic character grammar contracts.

pub mod base;
pub mod corpus;
pub mod face;
pub mod garment;
pub mod hair;
pub mod prepared;
pub mod proportion;

use serde::{Deserialize, Serialize};

/// Stable character grammar schema version.
pub const CHARACTER_GRAMMAR_SCHEMA_VERSION: u32 = 1;

/// Versioned grammar namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterGrammarId(pub String);

/// Stable semantic control identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterControlId(pub String);

/// Stable semantic region identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterRegionId(pub String);

/// Stable landmark identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterLandmarkId(pub String);

/// Stable topology-loop identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterLoopId(pub String);

/// Stable symmetry-plane identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterSymmetryId(pub String);

/// Stable versioned-base identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CharacterBaseId(pub String);

/// Shared scalar control range.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalarRange {
    /// Inclusive minimum.
    pub min: f32,
    /// Inclusive maximum.
    pub max: f32,
    /// Default authored value.
    pub default: f32,
}

impl ScalarRange {
    /// Validate finite ordered bounds and default containment.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.default.is_finite()
            && self.min <= self.default
            && self.default <= self.max
    }
}

/// Compact normalized quaternion used by character contracts.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitQuaternion {
    /// Canonical `[x, y, z, w]` components.
    pub value: [f32; 4],
}

impl UnitQuaternion {
    /// Identity rotation.
    pub const IDENTITY: Self = Self {
        value: [0.0, 0.0, 0.0, 1.0],
    };

    /// Returns true when the quaternion is finite and normalized.
    #[must_use]
    pub fn is_canonical(self) -> bool {
        let norm = self
            .value
            .iter()
            .map(|component| component * component)
            .sum::<f32>();
        self.value.iter().all(|component| component.is_finite())
            && (norm - 1.0).abs() <= 0.0001
            && quaternion_sign_is_canonical(self.value)
    }
}

fn quaternion_sign_is_canonical(value: [f32; 4]) -> bool {
    if value[3].abs() > 0.0001 {
        return value[3] > 0.0;
    }
    value[..3]
        .iter()
        .find(|component| component.abs() > 0.0001)
        .is_none_or(|component| *component > 0.0)
}
