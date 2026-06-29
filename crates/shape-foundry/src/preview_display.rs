//! Foundry preview display-mode contracts.
//!
//! These contracts are viewport/display metadata only. They do not add UVs,
//! texture files, material maps, decals, labels, or a material editor.

use serde::{Deserialize, Serialize};

/// Display mode for untextured Foundry clay previews.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FoundryPreviewDisplayMode {
    /// One neutral gray display class.
    #[default]
    PureClay,
    /// Neutral gray values by semantic role/part group.
    SemanticClay,
    /// Bright developer-only part colors for diagnostics.
    DiagnosticPartColor,
}

impl FoundryPreviewDisplayMode {
    /// Default novice mode: Semantic Clay when assignments exist, Pure Clay otherwise.
    #[must_use]
    pub fn novice_default(assignments: &[SemanticClayRoleAssignment]) -> Self {
        if assignments.is_empty() {
            Self::PureClay
        } else {
            Self::SemanticClay
        }
    }

    /// Return true when this mode is safe as a default novice display mode.
    #[must_use]
    pub const fn default_novice_safe(self) -> bool {
        matches!(self, Self::PureClay | Self::SemanticClay)
    }
}

/// Preview-only neutral gray assignment for Semantic Clay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticClayRoleAssignment {
    /// Family role or semantic part-group ID.
    pub role_or_part_group: String,
    /// Product-safe display label.
    pub display_label: String,
    /// Neutral gray value in `0..=1`.
    pub neutral_gray_value: f32,
    /// Higher priority wins when assignments overlap.
    pub priority: u8,
    /// Whether the assignment applies to generated candidate previews.
    pub applies_to_candidates: bool,
}

impl SemanticClayRoleAssignment {
    /// Construct one assignment with clamped gray value.
    #[must_use]
    pub fn new(
        role_or_part_group: impl Into<String>,
        display_label: impl Into<String>,
        neutral_gray_value: f32,
        priority: u8,
        applies_to_candidates: bool,
    ) -> Self {
        Self {
            role_or_part_group: role_or_part_group.into(),
            display_label: display_label.into(),
            neutral_gray_value: neutral_gray_value.clamp(0.0, 1.0),
            priority,
            applies_to_candidates,
        }
    }
}

/// Separate quality result for Pure Clay and Semantic Clay gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryClayQualityGateRecord {
    /// Strict Pure Clay mesh-readability result.
    pub pure_clay_pass: bool,
    /// Semantic Clay readability result.
    pub semantic_clay_readability_pass: bool,
    /// Display mode used to collect this evidence.
    pub display_mode_used: FoundryPreviewDisplayMode,
}

impl FoundryClayQualityGateRecord {
    /// Return true only when both separate gate results passed.
    #[must_use]
    pub const fn both_pass(&self) -> bool {
        self.pure_clay_pass && self.semantic_clay_readability_pass
    }
}

/// Validate Semantic Clay assignments.
#[must_use]
pub fn validate_semantic_clay_assignments(
    assignments: &[SemanticClayRoleAssignment],
) -> Vec<String> {
    let mut issues = Vec::new();
    for (index, assignment) in assignments.iter().enumerate() {
        if assignment.role_or_part_group.trim().is_empty() {
            issues.push(format!("assignments.{index}.role_or_part_group is empty"));
        }
        if assignment.display_label.trim().is_empty() {
            issues.push(format!("assignments.{index}.display_label is empty"));
        }
        if !assignment.neutral_gray_value.is_finite()
            || !(0.0..=1.0).contains(&assignment.neutral_gray_value)
        {
            issues.push(format!(
                "assignments.{index}.neutral_gray_value must be finite 0..=1"
            ));
        }
    }
    issues
}
