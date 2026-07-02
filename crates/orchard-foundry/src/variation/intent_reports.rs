
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
            human_summary:
                "Vary surface treatment when textured previews and material candidates are available."
                    .to_owned(),
        }
    }

    /// Whole-asset detail-only intent.
    #[must_use]
    pub fn whole_asset_detail() -> Self {
        Self {
            scope: VariationScope::WholeAsset,
            channels: vec![VariationChannel::Detail],
            human_label: "Detail".to_owned(),
            human_summary: "Vary small visual details without claiming shape or surface changes."
                .to_owned(),
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

/// Multi-camera perceptual report used to reject unreadable candidates before
/// selection or UI presentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptualCandidateReport {
    /// Stable candidate ID.
    pub candidate_id: String,
    /// Per fixed-camera visible delta in `0..1`.
    pub render_delta_by_camera: Vec<f32>,
    /// Maximum fixed-camera delta in `0..1`.
    pub max_delta: f32,
    /// Average fixed-camera delta in `0..1`.
    pub average_delta: f32,
    /// Projected silhouette delta in `0..1`.
    pub silhouette_delta: f32,
    /// Projected/world bounding-box delta in `0..1`.
    pub bbox_delta: f32,
    /// Product-visible part groups changed by this candidate.
    pub changed_part_groups: Vec<String>,
    /// Product-visible controls changed by this candidate.
    pub changed_controls: Vec<String>,
    /// Product-visible classification.
    pub legibility_class: CandidateLegibilityClass,
    /// Product-safe rejection reason, when the candidate was rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
    /// Plain-language summary for diagnostics.
    pub human_summary: String,
}

impl Default for PerceptualCandidateReport {
    fn default() -> Self {
        Self {
            candidate_id: String::new(),
            render_delta_by_camera: Vec::new(),
            max_delta: 0.0,
            average_delta: 0.0,
            silhouette_delta: 0.0,
            bbox_delta: 0.0,
            changed_part_groups: Vec::new(),
            changed_controls: Vec::new(),
            legibility_class: CandidateLegibilityClass::Unsupported,
            reject_reason: None,
            human_summary: "No perceptual evidence was generated.".to_owned(),
        }
    }
}

impl PerceptualCandidateReport {
    /// Build a report while clamping every score to `0..1`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate_id: impl Into<String>,
        render_delta_by_camera: Vec<f32>,
        max_delta: f32,
        average_delta: f32,
        silhouette_delta: f32,
        bbox_delta: f32,
        changed_part_groups: Vec<String>,
        changed_controls: Vec<String>,
        legibility_class: CandidateLegibilityClass,
        reject_reason: Option<String>,
        human_summary: impl Into<String>,
    ) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            render_delta_by_camera: render_delta_by_camera
                .into_iter()
                .map(clamp_score)
                .collect(),
            max_delta: clamp_score(max_delta),
            average_delta: clamp_score(average_delta),
            silhouette_delta: clamp_score(silhouette_delta),
            bbox_delta: clamp_score(bbox_delta),
            changed_part_groups,
            changed_controls,
            legibility_class,
            reject_reason,
            human_summary: human_summary.into(),
        }
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
    /// Strict multi-camera legibility evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perceptual_report: Option<PerceptualCandidateReport>,
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
            perceptual_report: None,
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
