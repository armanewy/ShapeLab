//! Product-safe variation scope, channel, and legibility contracts.

use serde::{Deserialize, Serialize};

/// Product-visible area a variation is allowed to affect.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum VariationScope {
    /// The whole authored asset.
    #[default]
    WholeAsset,
    /// A human-facing semantic part group.
    SemanticPartGroup {
        /// Stable group ID.
        group_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// A future material slot.
    MaterialSlot {
        /// Stable slot ID.
        slot_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// A human-facing detail zone.
    DetailZone {
        /// Stable zone ID.
        zone_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Reserved rig region.
    RigRegion {
        /// Stable region ID.
        region_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Reserved motion set.
    MotionSet {
        /// Stable motion-set ID.
        motion_set_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Custom/internal scope.
    Custom {
        /// Stable scope ID.
        scope_id: String,
        /// Product-facing display name.
        display_name: String,
    },
}

impl VariationScope {
    /// Return the product label for this scope.
    #[must_use]
    pub fn display_label(&self) -> &str {
        match self {
            Self::WholeAsset => "Whole Asset",
            Self::SemanticPartGroup { display_name, .. }
            | Self::MaterialSlot { display_name, .. }
            | Self::DetailZone { display_name, .. }
            | Self::RigRegion { display_name, .. }
            | Self::MotionSet { display_name, .. }
            | Self::Custom { display_name, .. } => display_name,
        }
    }

    /// Return true when this is a part-group focus scope.
    #[must_use]
    pub fn is_focus_part(&self) -> bool {
        matches!(self, Self::SemanticPartGroup { .. })
    }

    /// Return the stable part-group ID if this scope focuses a part group.
    #[must_use]
    pub fn semantic_part_group_id(&self) -> Option<&str> {
        match self {
            Self::SemanticPartGroup { group_id, .. } => Some(group_id),
            _ => None,
        }
    }
}

/// Product channel being varied.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "channel", rename_all = "snake_case")]
pub enum VariationChannel {
    /// Coherent whole-asset look. May combine supported shape and surface changes.
    #[default]
    CompleteLook,
    /// Geometry, proportions, silhouette, or structure.
    Shape,
    /// Material/surface slot changes. Reserved until surface payloads exist.
    Surface,
    /// Wear/weathering. Reserved until surface payloads exist.
    Wear,
    /// Detail-only changes.
    Detail,
    /// Reserved rig channel.
    Rig,
    /// Reserved motion channel.
    Motion,
    /// Reserved gameplay channel.
    Gameplay,
    /// Custom/internal channel.
    Custom {
        /// Stable channel ID.
        channel_id: String,
        /// Product-facing display name.
        display_name: String,
    },
}

impl VariationChannel {
    /// Return the product label for this channel.
    #[must_use]
    pub fn display_label(&self) -> &str {
        match self {
            Self::CompleteLook => "Complete Look",
            Self::Shape => "Shape",
            Self::Surface => "Surface",
            Self::Wear => "Wear",
            Self::Detail => "Detail",
            Self::Rig => "Rig",
            Self::Motion => "Motion",
            Self::Gameplay => "Gameplay",
            Self::Custom { display_name, .. } => display_name,
        }
    }

    /// Return true when this channel is visible to the default novice UI today.
    #[must_use]
    pub fn is_default_product_channel(&self, surface_supported: bool) -> bool {
        match self {
            Self::CompleteLook | Self::Shape => true,
            Self::Surface => surface_supported,
            Self::Wear
            | Self::Detail
            | Self::Rig
            | Self::Motion
            | Self::Gameplay
            | Self::Custom { .. } => false,
        }
    }
}

/// Product intent for one generation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariationIntent {
    /// Scope allowed to vary.
    #[serde(default)]
    pub scope: VariationScope,
    /// Channels requested for this variation.
    #[serde(default = "default_variation_channels")]
    pub channels: Vec<VariationChannel>,
    /// Product-facing label.
    #[serde(default = "default_variation_label")]
    pub human_label: String,
    /// Product-facing summary.
    #[serde(default = "default_variation_summary")]
    pub human_summary: String,
}

impl Default for VariationIntent {
    fn default() -> Self {
        Self::complete_look()
    }
}

impl VariationIntent {
    /// Default whole-asset complete-look intent.
    #[must_use]
    pub fn complete_look() -> Self {
        Self {
            scope: VariationScope::WholeAsset,
            channels: default_variation_channels(),
            human_label: default_variation_label(),
            human_summary: default_variation_summary(),
        }
    }

    /// Whole-asset shape-only intent.
    #[must_use]
    pub fn whole_asset_shape() -> Self {
        Self {
            scope: VariationScope::WholeAsset,
            channels: vec![VariationChannel::Shape],
            human_label: "Shape".to_owned(),
            human_summary: "Vary the asset shape while preserving surface assumptions.".to_owned(),
        }
    }

    /// Whole-asset surface intent.
    #[must_use]
    pub fn whole_asset_surface() -> Self {
        Self {
            scope: VariationScope::WholeAsset,
            channels: vec![VariationChannel::Surface],
            human_label: "Surface".to_owned(),
            human_summary: "Vary surface treatment when a surface pack is available.".to_owned(),
        }
    }

    /// Focus one semantic part group through a shape channel.
    #[must_use]
    pub fn focus_part_shape(group_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        Self {
            scope: VariationScope::SemanticPartGroup {
                group_id: group_id.into(),
                display_name: display_name.clone(),
            },
            channels: vec![VariationChannel::Shape],
            human_label: "Focus Part".to_owned(),
            human_summary: format!("Vary the {display_name} part group."),
        }
    }

    /// Return true if any requested channel matches `channel`.
    #[must_use]
    pub fn includes_channel(&self, channel: &VariationChannel) -> bool {
        self.channels.iter().any(|existing| existing == channel)
    }

    /// Normalize an intent so it always has at least one channel and finite labels.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.channels.is_empty() {
            self.channels = default_variation_channels();
        }
        if self.human_label.trim().is_empty() {
            self.human_label = self
                .channels
                .first()
                .map_or_else(default_variation_label, |channel| {
                    channel.display_label().to_owned()
                });
        }
        if self.human_summary.trim().is_empty() {
            self.human_summary = default_variation_summary();
        }
        self
    }
}

fn default_variation_channels() -> Vec<VariationChannel> {
    vec![VariationChannel::CompleteLook]
}

fn default_variation_label() -> String {
    "Complete Looks".to_owned()
}

fn default_variation_summary() -> String {
    "Vary the whole asset as a coherent look.".to_owned()
}

/// Current variation focus stored with a document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryVariationState {
    /// Active variation intent.
    #[serde(default)]
    pub intent: VariationIntent,
}

/// Human-facing semantic part-group change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPartGroupChange {
    /// Stable group ID.
    pub group_id: String,
    /// Product-facing display name.
    pub display_name: String,
    /// Product-facing change label.
    pub change_label: String,
    /// Whether the change is visible in product preview evidence.
    pub visible: bool,
}

/// Human-facing material-slot change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSlotChange {
    /// Stable slot ID.
    pub slot_id: String,
    /// Product-facing display name.
    pub display_name: String,
    /// Product-facing change label.
    pub change_label: String,
    /// Whether a real surface payload exists for this slot change.
    pub surface_payload_ready: bool,
}

/// Product-visible legibility class for a candidate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CandidateLegibilityClass {
    /// Strongly readable difference.
    Strong,
    /// Clear product-visible difference.
    Clear,
    /// Subtle, but the card can explain the visible change.
    SubtleButExplainable,
    /// Detail-only change.
    DetailOnly,
    /// Too subtle for a normal direction card.
    TooSubtle,
    /// Looks like a duplicate of an already returned direction.
    DuplicateLooking,
    /// Requested channel or scope is unsupported.
    Unsupported,
}

impl CandidateLegibilityClass {
    /// Return true when the class may be shown as a normal selectable direction.
    #[must_use]
    pub fn selectable(self) -> bool {
        matches!(
            self,
            Self::Strong | Self::Clear | Self::SubtleButExplainable | Self::DetailOnly
        )
    }

    /// Return the product label for this class.
    #[must_use]
    pub fn display_label(self) -> &'static str {
        match self {
            Self::Strong => "Strong change",
            Self::Clear => "Clear change",
            Self::SubtleButExplainable => "Subtle but visible",
            Self::DetailOnly => "Detail change",
            Self::TooSubtle => "Too subtle",
            Self::DuplicateLooking => "Looks too similar",
            Self::Unsupported => "Unavailable",
        }
    }
}

/// Render/descriptor-backed visible delta report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateVisibleDeltaReport {
    /// Geometry/shape delta in 0..1.
    pub shape_delta_score: f32,
    /// Silhouette delta in 0..1.
    pub silhouette_delta_score: f32,
    /// Card-size screen-space delta in 0..1.
    pub screen_space_delta_score: f32,
    /// Structural/provider/role delta in 0..1.
    pub structure_delta_score: f32,
    /// Surface/material delta in 0..1.
    pub surface_delta_score: f32,
    /// Wear delta in 0..1.
    pub wear_delta_score: f32,
    /// Selected part-group delta in 0..1.
    pub selected_part_delta_score: f32,
    /// Product-visible classification.
    pub legibility_class: CandidateLegibilityClass,
    /// Product-safe blocking reasons.
    #[serde(default)]
    pub blocking_reasons: Vec<String>,
    /// Whether manual review is required before stronger claims.
    pub manual_review_required: bool,
}

impl Default for CandidateVisibleDeltaReport {
    fn default() -> Self {
        Self {
            shape_delta_score: 0.0,
            silhouette_delta_score: 0.0,
            screen_space_delta_score: 0.0,
            structure_delta_score: 0.0,
            surface_delta_score: 0.0,
            wear_delta_score: 0.0,
            selected_part_delta_score: 0.0,
            legibility_class: CandidateLegibilityClass::Unsupported,
            blocking_reasons: Vec::new(),
            manual_review_required: false,
        }
    }
}

impl CandidateVisibleDeltaReport {
    /// Build a report while clamping every score to 0..1 and replacing non-finite
    /// inputs with zero.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        shape_delta_score: f32,
        silhouette_delta_score: f32,
        screen_space_delta_score: f32,
        structure_delta_score: f32,
        surface_delta_score: f32,
        wear_delta_score: f32,
        selected_part_delta_score: f32,
        legibility_class: CandidateLegibilityClass,
        blocking_reasons: Vec<String>,
        manual_review_required: bool,
    ) -> Self {
        Self {
            shape_delta_score: clamp_score(shape_delta_score),
            silhouette_delta_score: clamp_score(silhouette_delta_score),
            screen_space_delta_score: clamp_score(screen_space_delta_score),
            structure_delta_score: clamp_score(structure_delta_score),
            surface_delta_score: clamp_score(surface_delta_score),
            wear_delta_score: clamp_score(wear_delta_score),
            selected_part_delta_score: clamp_score(selected_part_delta_score),
            legibility_class,
            blocking_reasons,
            manual_review_required,
        }
    }

    /// Product label for the visible delta class.
    #[must_use]
    pub fn label(&self) -> &'static str {
        self.legibility_class.display_label()
    }
}

/// Candidate explanation consistency checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateExplanationQuality {
    /// Explanation mentions changed controls.
    pub explanation_matches_changed_controls: bool,
    /// Explanation matches visible delta evidence.
    pub explanation_matches_visible_delta: bool,
    /// A product summary is available.
    pub human_summary_available: bool,
}

impl Default for CandidateExplanationQuality {
    fn default() -> Self {
        Self {
            explanation_matches_changed_controls: true,
            explanation_matches_visible_delta: true,
            human_summary_available: true,
        }
    }
}

/// Product metadata attached to every candidate plan/card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateVariationMetadata {
    /// Requested intent.
    #[serde(default)]
    pub intent: VariationIntent,
    /// Human-facing part-group changes.
    #[serde(default)]
    pub changed_part_groups: Vec<SemanticPartGroupChange>,
    /// Human-facing material-slot changes.
    #[serde(default)]
    pub changed_material_slots: Vec<MaterialSlotChange>,
    /// Changed visible controls.
    #[serde(default)]
    pub changed_controls: Vec<String>,
    /// Changed visible roles.
    #[serde(default)]
    pub changed_roles: Vec<String>,
    /// Whether the candidate respected active locks.
    pub respects_locks: bool,
    /// Visible delta evidence.
    #[serde(default)]
    pub visible_delta: CandidateVisibleDeltaReport,
    /// Explanation quality.
    #[serde(default)]
    pub explanation_quality: CandidateExplanationQuality,
}

impl Default for CandidateVariationMetadata {
    fn default() -> Self {
        Self {
            intent: VariationIntent::default(),
            changed_part_groups: Vec::new(),
            changed_material_slots: Vec::new(),
            changed_controls: Vec::new(),
            changed_roles: Vec::new(),
            respects_locks: true,
            visible_delta: CandidateVisibleDeltaReport::default(),
            explanation_quality: CandidateExplanationQuality::default(),
        }
    }
}

fn clamp_score(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
