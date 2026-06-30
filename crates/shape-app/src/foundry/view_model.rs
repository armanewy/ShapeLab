//! UI-ready view-model contracts for the native Foundry surface.

use std::collections::BTreeMap;

use shape_foundry::{
    CandidateLegibilityClass, ControlDivergence, ControlTopologyBehavior, ControlValue,
    FoundryCandidateId, FoundryDocumentId, FoundryLock, FoundryPackDocument,
};
use shape_render::OrbitCamera;
use shape_search::foundry::{
    FoundryCandidateControlChange, FoundryCandidateMode, FoundryCandidateRejectionReason,
};

/// Whole-model candidate direction card.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryCandidateCard {
    /// Stable candidate ID.
    pub id: FoundryCandidateId,
    /// Candidate slot in the direction board.
    pub slot: usize,
    /// Search mode that produced this card.
    pub mode: Option<FoundryCandidateMode>,
    /// True for the unchanged parent card.
    pub parent: bool,
    /// Human-facing title.
    pub title: String,
    /// Human-facing subtitle.
    pub subtitle: String,
    /// Whole-model preview ID.
    pub preview_id: Option<String>,
    /// RGBA8 preview bytes.
    pub rgba8: Vec<u8>,
    /// Preview width.
    pub width: u32,
    /// Preview height.
    pub height: u32,
    /// Camera shared by cards in the same comparison.
    pub camera: Option<OrbitCamera>,
    /// Preview-specific failure, without invalidating the candidate itself.
    pub preview_failure: Option<String>,
    /// Changed customizer controls.
    pub changed_controls: Vec<String>,
    /// Changed provider roles.
    pub changed_roles: Vec<String>,
    /// Structured candidate explanations from the generic candidate engine.
    pub explanations: Vec<FoundryCandidateControlChange>,
    /// Rejection reasons for invalid or unavailable cards.
    pub rejections: BTreeMap<FoundryCandidateRejectionReason, usize>,
    /// Validation label for badges.
    pub validation_label: String,
    /// Validation detail for tooltips.
    pub validation_detail: Option<String>,
    /// Whether this card can be accepted.
    pub selectable: bool,
    /// Whether this card is currently selected.
    pub selected: bool,
    /// Product-facing variation intent label.
    pub variation_intent_label: String,
    /// Product-facing scope label.
    pub variation_scope_label: String,
    /// Product-facing channel labels.
    pub variation_channel_labels: Vec<String>,
    /// Product-facing visible delta label.
    pub visible_delta_label: String,
    /// Product-facing change summary.
    pub what_changed_summary: String,
    /// Product legibility class.
    pub legibility_class: CandidateLegibilityClass,
    /// Focus part label, when the card targets a semantic part group.
    pub focus_part_label: Option<String>,
    /// Plain-language reason surface mode is unavailable.
    pub surface_unavailable_reason: Option<String>,
}

/// One customizer control row/card.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryControlView {
    /// Stable control ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Optional section label.
    pub section: Option<String>,
    /// Human-facing control kind label.
    pub kind: String,
    /// Deterministic control presentation.
    pub presentation: FoundryControlPresentation,
    /// Current control value.
    pub value: Option<ControlValue>,
    /// Authored default value, when available.
    pub default_value: Option<ControlValue>,
    /// Whether this is a primary novice-facing control.
    pub primary: bool,
    /// Whether this row is visible outside Advanced Recipe.
    pub visible: bool,
    /// Whether edits are currently locked.
    pub locked: bool,
    /// Human-facing reason edits are locked, when available.
    pub locked_reason: Option<String>,
    /// Topology behavior for preview/release semantics.
    pub topology_behavior: ControlTopologyBehavior,
    /// Divergence between source controls and generated recipe.
    pub divergence: ControlDivergence,
    /// Feasible options or filmstrip samples.
    pub options: Vec<FoundryOptionCard>,
    /// Numeric range for direct bounded controls.
    pub numeric_range: Option<FoundryNumericRange>,
    /// Technical path shown only in tooltips or Advanced Recipe.
    pub advanced_path: Option<String>,
    /// Human-facing helper text.
    pub help: Option<String>,
}

/// Bounded numeric range for a direct property control.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct FoundryNumericRange {
    /// Inclusive minimum value.
    pub minimum: f32,
    /// Inclusive maximum value.
    pub maximum: f32,
    /// Stepper increment.
    pub step: f32,
}

/// Deterministic presentation kind for one customizer control.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FoundryControlPresentation {
    /// Continuous macro axis, usually backed by several family slots.
    ContinuousMacroAxis,
    /// Integer stepper.
    Stepper,
    /// Binary toggle.
    Toggle,
    /// Whole-model choice gallery.
    ChoiceGallery,
    /// Whole-model provider gallery.
    ProviderGallery,
}

/// Whole-model option card for choices, providers, and sampled controls.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryOptionCard {
    /// Owning control ID.
    pub control_id: String,
    /// Option value.
    pub value: ControlValue,
    /// Human-facing option label.
    pub label: String,
    /// Provider role when this option selects a provider.
    pub provider_role: Option<String>,
    /// Whole-model preview ID.
    pub preview_id: Option<String>,
    /// RGBA8 preview bytes.
    pub rgba8: Vec<u8>,
    /// Preview width.
    pub width: u32,
    /// Preview height.
    pub height: u32,
    /// Camera used for this option preview.
    pub camera: Option<OrbitCamera>,
    /// Whether this option is currently selected.
    pub selected: bool,
    /// Why this option is unavailable.
    pub unavailable_reason: Option<String>,
}

/// Family-pack workspace view.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct FoundryPackView {
    /// Current pack ID.
    pub pack_id: Option<String>,
    /// Source pack document, when the workspace has one.
    pub pack: Option<FoundryPackDocument>,
    /// Pack members keyed by member ID.
    pub members: BTreeMap<String, FoundryDocumentId>,
    /// Selected member ID.
    pub selected_member: Option<String>,
    /// Locks shared across pack members.
    pub shared_locks: Vec<FoundryLock>,
    /// Provider choices shared across pack members.
    pub shared_provider_choices: BTreeMap<String, String>,
    /// Member-specific override counts.
    pub member_override_counts: BTreeMap<String, usize>,
    /// Coherence warnings shown before export.
    pub coherence_warnings: Vec<String>,
    /// Whether every member currently satisfies the pack policy.
    pub coherent: bool,
    /// Whether the pack can be exported now.
    pub can_export: bool,
}
