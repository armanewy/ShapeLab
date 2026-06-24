//! Whole-model direction board panel boundary.

use shape_foundry::{FoundryCandidateId, FoundryCommand};
use shape_render::OrbitCamera;
use shape_search::foundry::{
    FoundryCandidateControlChange, FoundryCandidateMode, FoundryCandidateRejectionReason,
    FoundryCandidateRequest,
};

use crate::foundry::{FoundryAppCommand, FoundryCandidateCard};

/// The direction board always reserves six candidate direction slots.
pub(crate) const VISIBLE_DIRECTION_CANDIDATE_CARDS: usize = 6;
/// Candidate proposal count used by the lightweight mode request helpers.
pub(crate) const DEFAULT_DIRECTION_PROPOSALS: usize = 24;
/// Button copy for accepting a direction.
pub(crate) const CHOOSE_THIS_DIRECTION_LABEL: &str = "Choose This Direction";
/// Button copy for rejecting a direction.
pub(crate) const REJECT_DIRECTION_LABEL: &str = "Reject";

/// Whole-model search modes exposed by the direction board.
pub(crate) const DIRECTION_BOARD_MODES: [FoundryCandidateMode; 5] = [
    FoundryCandidateMode::Refine,
    FoundryCandidateMode::Explore,
    FoundryCandidateMode::Silhouette,
    FoundryCandidateMode::Structure,
    FoundryCandidateMode::Detail,
];

/// Transient UI state needed to derive pure direction-board view data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DirectionBoardState {
    /// Selected candidate card, when one is active.
    pub selected_candidate: Option<FoundryCandidateId>,
    /// Hovered candidate card, when one is active.
    pub hovered_candidate: Option<FoundryCandidateId>,
    /// Active generation mode tab.
    pub active_mode: Option<FoundryCandidateMode>,
    /// Whether the A/B comparison is flipped.
    pub comparison_flipped: bool,
    /// Deterministic seed for mode request helpers.
    pub generation_seed: u64,
    /// Optional customizer strategy ID for mode request helpers.
    pub strategy_id: Option<String>,
}

impl DirectionBoardState {
    fn with_normalized_selection(mut self, candidates: &[FoundryCandidateCard]) -> Self {
        let explicit_selection_is_present = self
            .selected_candidate
            .as_ref()
            .is_some_and(|selected| candidates.iter().any(|candidate| candidate.id == *selected));
        if !explicit_selection_is_present {
            self.selected_candidate = candidates
                .iter()
                .find(|candidate| candidate.selected)
                .map(|candidate| candidate.id.clone());
        }
        self
    }
}

/// Complete data-only direction board snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionBoardView {
    /// Unchanged parent card shown before candidates.
    pub parent: DirectionCardView,
    /// Exactly six visible candidate card slots.
    pub candidate_slots: Vec<DirectionCardSlot>,
    /// Rows the eventual panel can render directly.
    pub rows: Vec<DirectionBoardRow>,
    /// Mode actions for Refine, Explore, Silhouette, Structure, and Detail.
    pub mode_actions: Vec<DirectionModeAction>,
    /// A/B comparison for the active candidate.
    pub comparison: Option<DirectionComparisonView>,
    /// Board-level validation and guard state.
    pub validation: DirectionBoardValidation,
    /// Number of isolated-part option cards emitted by this board.
    pub isolated_part_option_card_count: usize,
}

/// One render row in the direction board.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionBoardRow {
    /// Row kind.
    pub kind: DirectionBoardRowKind,
    /// Cards or empty slots in this row.
    pub cards: Vec<DirectionCardSlot>,
}

/// Stable row categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionBoardRowKind {
    /// Current unchanged parent.
    Parent,
    /// Candidate directions.
    Candidates,
}

/// A filled candidate/parent card or an empty reserved candidate slot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DirectionCardSlot {
    /// Filled card.
    Filled(Box<DirectionCardView>),
    /// Empty reserved candidate slot.
    Empty(DirectionEmptyCard),
}

impl DirectionCardSlot {
    /// Return the card when this slot is filled.
    #[must_use]
    pub(crate) fn as_card(&self) -> Option<&DirectionCardView> {
        match self {
            Self::Filled(card) => Some(card.as_ref()),
            Self::Empty(_) => None,
        }
    }

    /// Return true when this is an empty reserved candidate slot.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty(_))
    }
}

/// Empty reserved candidate card slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionEmptyCard {
    /// Visual slot index.
    pub slot: usize,
    /// Placeholder title.
    pub title: String,
    /// Placeholder subtitle.
    pub subtitle: String,
    /// Empty slots are still reserved for whole-model direction cards.
    pub preview_scope: DirectionImageScope,
}

/// UI-ready whole-model card summary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionCardView {
    /// Source Foundry candidate card.
    pub card: FoundryCandidateCard,
    /// Parent/candidate role.
    pub kind: DirectionCardKind,
    /// Visual slot index in the board.
    pub slot: usize,
    /// Human-facing mode label.
    pub mode_label: &'static str,
    /// Whole-model preview data.
    pub preview: DirectionPreviewView,
    /// Human-facing explanation lines.
    pub explanations: Vec<String>,
    /// Badges for changed provider/family roles.
    pub changed_role_badges: Vec<DirectionBadge>,
    /// Validation badge and state.
    pub validation_badge: DirectionValidationBadge,
    /// Whether the pointer is over this card.
    pub hovered: bool,
    /// Whether this card is selected.
    pub selected: bool,
    /// Whether the card should render highlighted.
    pub highlighted: bool,
    /// Card-level intents and labels.
    pub actions: DirectionCardActions,
}

/// Direction card role.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionCardKind {
    /// The unchanged parent/current model.
    UnchangedParent,
    /// A generated whole-model candidate.
    Candidate,
}

/// Image data used by direction cards and comparison panes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionPreviewView {
    /// Whole-model preview ID.
    pub preview_id: Option<String>,
    /// RGBA8 preview bytes.
    pub rgba8: Vec<u8>,
    /// Preview width.
    pub width: u32,
    /// Preview height.
    pub height: u32,
    /// Fixed camera for this whole-model preview.
    pub camera: Option<OrbitCamera>,
    /// Preview scope. Direction cards only use whole-model imagery.
    pub scope: DirectionImageScope,
}

impl DirectionPreviewView {
    /// Return true when this preview carries pixels and dimensions.
    #[must_use]
    pub(crate) fn has_image(&self) -> bool {
        let expected_len = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|pixels| pixels.checked_mul(4));
        self.width > 0 && self.height > 0 && expected_len == Some(self.rgba8.len())
    }
}

/// Direction preview scope.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionImageScope {
    /// Full asset/model preview.
    WholeModel,
}

/// Generic badge surfaced by a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionBadge {
    /// Badge label.
    pub label: String,
    /// Badge tone.
    pub tone: DirectionBadgeTone,
}

/// Badge tone.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionBadgeTone {
    /// Changed role badge.
    RoleChanged,
    /// Informational badge.
    Info,
}

/// Validation badge for one card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionValidationBadge {
    /// Human-facing validation label.
    pub label: String,
    /// Optional detail shown in tooltip-like surfaces.
    pub detail: Option<String>,
    /// Stable validation state.
    pub state: DirectionValidationState,
    /// Number of generation rejection diagnostics on this card.
    pub rejection_count: usize,
}

/// Stable validation states for cards.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionValidationState {
    /// The unchanged parent/current model.
    Unchanged,
    /// Candidate can be chosen.
    Valid,
    /// Candidate has non-blocking validation details.
    Warning,
    /// Candidate cannot currently be chosen.
    Invalid,
}

/// Action/intents exposed by one card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionCardActions {
    /// Select this card.
    pub select: DirectionBoardIntent,
    /// Hover this card.
    pub hover: DirectionBoardIntent,
    /// Clear hover state.
    pub clear_hover: DirectionBoardIntent,
    /// Accept this candidate when available.
    pub choose: Option<DirectionBoardIntent>,
    /// Reject this candidate when available.
    pub reject: Option<DirectionBoardIntent>,
    /// Accept button label.
    pub choose_label: &'static str,
    /// Reject button label.
    pub reject_label: &'static str,
}

/// Pure direction-board intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectionBoardIntent {
    /// Select a candidate, or clear selection with `None`.
    Select(Option<FoundryCandidateId>),
    /// Set or clear hover state.
    Hover(Option<FoundryCandidateId>),
    /// Choose a candidate direction.
    Accept(FoundryCandidateId),
    /// Reject a candidate direction.
    Reject(FoundryCandidateId),
}

impl DirectionBoardIntent {
    /// Convert intents backed by existing app commands to [`FoundryAppCommand`].
    #[must_use]
    pub(crate) fn app_command(&self) -> Option<FoundryAppCommand> {
        match self {
            Self::Select(candidate_id) => {
                Some(FoundryAppCommand::SelectCandidate(candidate_id.clone()))
            }
            Self::Hover(_) => None,
            Self::Accept(candidate_id) => Some(accept_candidate_command(candidate_id.clone())),
            Self::Reject(candidate_id) => Some(reject_candidate_command(candidate_id.clone())),
        }
    }
}

/// Search mode action and request data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionModeAction {
    /// Search mode.
    pub mode: FoundryCandidateMode,
    /// Human-facing label.
    pub label: &'static str,
    /// Whether this mode is active.
    pub selected: bool,
    /// Request data for the later panel/job wiring.
    pub request: FoundryCandidateRequest,
}

impl DirectionModeAction {
    /// Convert the mode action into the reducer command that preserves the requested mode.
    #[must_use]
    pub(crate) fn app_command(&self) -> FoundryAppCommand {
        FoundryAppCommand::RequestCandidates(self.request.clone())
    }
}

/// A/B comparison state for the selected direction.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionComparisonView {
    /// A side of the comparison.
    pub a: DirectionComparisonSide,
    /// B side of the comparison.
    pub b: DirectionComparisonSide,
    /// Whether the comparison sides are flipped.
    pub flipped: bool,
    /// Whether both sides share the same fixed camera.
    pub fixed_camera: bool,
    /// Comparison preview scope.
    pub scope: DirectionImageScope,
}

/// One side of an A/B comparison.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectionComparisonSide {
    /// A or B label.
    pub label: &'static str,
    /// Parent or candidate role.
    pub role: DirectionComparisonRole,
    /// Card ID.
    pub card_id: FoundryCandidateId,
    /// Card title.
    pub title: String,
    /// Whole-model preview data.
    pub preview: DirectionPreviewView,
}

/// Semantic role for one comparison side.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectionComparisonRole {
    /// Unchanged parent/current model.
    Parent,
    /// Selected candidate direction.
    Candidate,
}

/// Board-level validation and guards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionBoardValidation {
    /// Number of visible candidate slots.
    pub candidate_slot_count: usize,
    /// Number of filled candidate slots.
    pub filled_candidate_count: usize,
    /// Whether the parent and all filled candidate slots carry image pixels.
    pub preview_images_present: bool,
    /// Number of input candidates beyond the six visible slots.
    pub overflow_candidate_count: usize,
    /// Whether all filled previews share the parent's fixed camera.
    pub fixed_camera: bool,
    /// Whether every preview in this board is whole-model scoped.
    pub whole_model_only: bool,
    /// Whether this board emitted zero isolated-part option cards.
    pub no_isolated_part_option_cards: bool,
}

impl DirectionBoardValidation {
    /// Return true when the board satisfies the direction-card invariants.
    #[must_use]
    pub(crate) fn is_valid(&self) -> bool {
        self.candidate_slot_count == VISIBLE_DIRECTION_CANDIDATE_CARDS
            && self.filled_candidate_count == VISIBLE_DIRECTION_CANDIDATE_CARDS
            && self.preview_images_present
            && self.fixed_camera
            && self.whole_model_only
            && self.no_isolated_part_option_cards
    }
}

/// Build the pure direction-board view from an unchanged parent and candidates.
#[must_use]
pub(crate) fn direction_board_view(
    parent: &FoundryCandidateCard,
    candidates: &[FoundryCandidateCard],
    state: DirectionBoardState,
) -> DirectionBoardView {
    let state = state.with_normalized_selection(candidates);
    let parent_view = direction_card_view(parent, DirectionCardKind::UnchangedParent, 0, &state);
    let candidate_slots = candidate_slots(candidates)
        .into_iter()
        .enumerate()
        .map(|(slot, candidate)| match candidate {
            Some(candidate) => DirectionCardSlot::Filled(Box::new(direction_card_view(
                candidate,
                DirectionCardKind::Candidate,
                slot,
                &state,
            ))),
            None => DirectionCardSlot::Empty(empty_candidate_card(slot)),
        })
        .collect::<Vec<_>>();

    let comparison = selected_candidate_card(&candidate_slots, state.selected_candidate.as_ref())
        .map(|candidate| ab_flip_comparison(&parent_view, candidate, state.comparison_flipped));
    let validation = board_validation(&parent_view, &candidate_slots, candidates.len());
    let mode_actions = direction_mode_actions(
        state.active_mode,
        state.generation_seed,
        state.strategy_id.clone(),
    );
    let rows = vec![
        DirectionBoardRow {
            kind: DirectionBoardRowKind::Parent,
            cards: vec![DirectionCardSlot::Filled(Box::new(parent_view.clone()))],
        },
        DirectionBoardRow {
            kind: DirectionBoardRowKind::Candidates,
            cards: candidate_slots.clone(),
        },
    ];

    DirectionBoardView {
        parent: parent_view,
        candidate_slots,
        rows,
        mode_actions,
        comparison,
        validation,
        isolated_part_option_card_count: 0,
    }
}

/// Return exactly six visible candidate slots.
#[must_use]
pub(crate) fn candidate_slots(
    candidates: &[FoundryCandidateCard],
) -> Vec<Option<&FoundryCandidateCard>> {
    (0..VISIBLE_DIRECTION_CANDIDATE_CARDS)
        .map(|index| candidates.get(index))
        .collect()
}

/// Build mode actions for the direction board.
#[must_use]
pub(crate) fn direction_mode_actions(
    active_mode: Option<FoundryCandidateMode>,
    seed: u64,
    strategy_id: Option<String>,
) -> Vec<DirectionModeAction> {
    DIRECTION_BOARD_MODES
        .into_iter()
        .map(|mode| DirectionModeAction {
            mode,
            label: mode_label(mode),
            selected: active_mode == Some(mode),
            request: candidate_request_for_mode(mode, seed, strategy_id.clone()),
        })
        .collect()
}

/// Build a Foundry candidate request for one board mode.
#[must_use]
pub(crate) fn candidate_request_for_mode(
    mode: FoundryCandidateMode,
    seed: u64,
    strategy_id: Option<String>,
) -> FoundryCandidateRequest {
    FoundryCandidateRequest {
        seed,
        proposal_count: DEFAULT_DIRECTION_PROPOSALS,
        result_count: VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode,
        strategy_id,
        preference_profile: None,
    }
}

/// Human-facing label for a Foundry candidate mode.
#[must_use]
pub(crate) fn mode_label(mode: FoundryCandidateMode) -> &'static str {
    match mode {
        FoundryCandidateMode::Refine => "Refine",
        FoundryCandidateMode::Explore => "Explore",
        FoundryCandidateMode::Silhouette => "Silhouette",
        FoundryCandidateMode::Structure => "Structure",
        FoundryCandidateMode::Detail => "Detail",
    }
}

/// Build an explicit select-card intent.
#[must_use]
pub(crate) fn select_candidate_intent(
    candidate_id: Option<FoundryCandidateId>,
) -> DirectionBoardIntent {
    DirectionBoardIntent::Select(candidate_id)
}

/// Build an explicit hover-card intent.
#[must_use]
pub(crate) fn hover_candidate_intent(
    candidate_id: Option<FoundryCandidateId>,
) -> DirectionBoardIntent {
    DirectionBoardIntent::Hover(candidate_id)
}

/// Build an explicit choose-card intent.
#[must_use]
pub(crate) fn choose_direction_intent(candidate_id: FoundryCandidateId) -> DirectionBoardIntent {
    DirectionBoardIntent::Accept(candidate_id)
}

/// Build an explicit reject-card intent.
#[must_use]
pub(crate) fn reject_direction_intent(candidate_id: FoundryCandidateId) -> DirectionBoardIntent {
    DirectionBoardIntent::Reject(candidate_id)
}

/// Build the app command for accepting a candidate direction.
#[must_use]
pub(crate) fn accept_candidate_command(candidate_id: FoundryCandidateId) -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::AcceptCandidate { candidate_id })
}

/// Build the app command for rejecting a candidate direction.
#[must_use]
pub(crate) fn reject_candidate_command(candidate_id: FoundryCandidateId) -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::RejectCandidate { candidate_id })
}

/// Build A/B comparison data. When flipped, the selected candidate is A.
#[must_use]
pub(crate) fn ab_flip_comparison(
    parent: &DirectionCardView,
    candidate: &DirectionCardView,
    flipped: bool,
) -> DirectionComparisonView {
    let parent_side = DirectionComparisonSide {
        label: if flipped { "B" } else { "A" },
        role: DirectionComparisonRole::Parent,
        card_id: parent.card.id.clone(),
        title: parent.card.title.clone(),
        preview: parent.preview.clone(),
    };
    let candidate_side = DirectionComparisonSide {
        label: if flipped { "A" } else { "B" },
        role: DirectionComparisonRole::Candidate,
        card_id: candidate.card.id.clone(),
        title: candidate.card.title.clone(),
        preview: candidate.preview.clone(),
    };
    let (a, b) = if flipped {
        (candidate_side, parent_side)
    } else {
        (parent_side, candidate_side)
    };

    DirectionComparisonView {
        fixed_camera: previews_share_fixed_camera(&a.preview, &b.preview),
        a,
        b,
        flipped,
        scope: DirectionImageScope::WholeModel,
    }
}

/// Return true when the board emitted no isolated-part option cards.
#[must_use]
pub(crate) fn no_isolated_part_option_cards(board: &DirectionBoardView) -> bool {
    board.isolated_part_option_card_count == 0
        && board.validation.no_isolated_part_option_cards
        && board.validation.whole_model_only
}

fn direction_card_view(
    card: &FoundryCandidateCard,
    kind: DirectionCardKind,
    slot: usize,
    state: &DirectionBoardState,
) -> DirectionCardView {
    let selected = match kind {
        DirectionCardKind::UnchangedParent => false,
        DirectionCardKind::Candidate => state
            .selected_candidate
            .as_ref()
            .map_or(card.selected, |selected| selected == &card.id),
    };
    let hovered = matches!(kind, DirectionCardKind::Candidate)
        && state.hovered_candidate.as_ref() == Some(&card.id);
    let highlighted = selected || hovered;
    let preview = DirectionPreviewView {
        preview_id: card.preview_id.clone(),
        rgba8: card.rgba8.clone(),
        width: card.width,
        height: card.height,
        camera: card.camera.clone(),
        scope: DirectionImageScope::WholeModel,
    };

    DirectionCardView {
        card: card.clone(),
        kind,
        slot,
        mode_label: card.mode.map_or("Current", mode_label),
        preview,
        explanations: explanation_lines(card),
        changed_role_badges: changed_role_badges(card),
        validation_badge: validation_badge(card, kind),
        hovered,
        selected,
        highlighted,
        actions: card_actions(card, kind),
    }
}

fn empty_candidate_card(slot: usize) -> DirectionEmptyCard {
    DirectionEmptyCard {
        slot,
        title: format!("Candidate {}", slot + 1),
        subtitle: "No direction yet".to_owned(),
        preview_scope: DirectionImageScope::WholeModel,
    }
}

fn card_actions(card: &FoundryCandidateCard, kind: DirectionCardKind) -> DirectionCardActions {
    match kind {
        DirectionCardKind::UnchangedParent => DirectionCardActions {
            select: select_candidate_intent(None),
            hover: hover_candidate_intent(None),
            clear_hover: hover_candidate_intent(None),
            choose: None,
            reject: None,
            choose_label: CHOOSE_THIS_DIRECTION_LABEL,
            reject_label: REJECT_DIRECTION_LABEL,
        },
        DirectionCardKind::Candidate => DirectionCardActions {
            select: select_candidate_intent(Some(card.id.clone())),
            hover: hover_candidate_intent(Some(card.id.clone())),
            clear_hover: hover_candidate_intent(None),
            choose: candidate_can_be_chosen(card).then(|| choose_direction_intent(card.id.clone())),
            reject: Some(reject_direction_intent(card.id.clone())),
            choose_label: CHOOSE_THIS_DIRECTION_LABEL,
            reject_label: REJECT_DIRECTION_LABEL,
        },
    }
}

fn candidate_can_be_chosen(card: &FoundryCandidateCard) -> bool {
    card.selectable && candidate_rejection_count(card) == 0
}

fn explanation_lines(card: &FoundryCandidateCard) -> Vec<String> {
    if card.parent {
        return vec!["Unchanged parent".to_owned()];
    }

    let mut lines = card
        .explanations
        .iter()
        .map(explanation_line)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("No explanatory control changes".to_owned());
    }
    lines
}

fn explanation_line(change: &FoundryCandidateControlChange) -> String {
    if change.message.trim().is_empty() {
        format!(
            "{} changed from `{}` to `{}`.",
            change.control_label, change.before, change.after
        )
    } else {
        change.message.clone()
    }
}

fn changed_role_badges(card: &FoundryCandidateCard) -> Vec<DirectionBadge> {
    let mut badges = Vec::new();
    for role in &card.changed_roles {
        if !role.trim().is_empty()
            && !badges
                .iter()
                .any(|badge: &DirectionBadge| badge.label == *role)
        {
            badges.push(DirectionBadge {
                label: role.clone(),
                tone: DirectionBadgeTone::RoleChanged,
            });
        }
    }
    badges
}

fn validation_badge(
    card: &FoundryCandidateCard,
    kind: DirectionCardKind,
) -> DirectionValidationBadge {
    let rejection_count = candidate_rejection_count(card);
    let detail = card
        .validation_detail
        .clone()
        .or_else(|| rejection_detail(&card.rejections));
    let label = if card.validation_label.trim().is_empty() {
        match kind {
            DirectionCardKind::UnchangedParent => "Unchanged",
            DirectionCardKind::Candidate => "Valid",
        }
        .to_owned()
    } else {
        card.validation_label.clone()
    };
    let state = match kind {
        DirectionCardKind::UnchangedParent if rejection_count == 0 => {
            DirectionValidationState::Unchanged
        }
        DirectionCardKind::UnchangedParent => DirectionValidationState::Warning,
        DirectionCardKind::Candidate if !card.selectable || rejection_count > 0 => {
            DirectionValidationState::Invalid
        }
        DirectionCardKind::Candidate if detail.is_some() => DirectionValidationState::Warning,
        DirectionCardKind::Candidate => DirectionValidationState::Valid,
    };

    DirectionValidationBadge {
        label,
        detail,
        state,
        rejection_count,
    }
}

fn candidate_rejection_count(card: &FoundryCandidateCard) -> usize {
    card.rejections.values().copied().sum()
}

fn rejection_detail(
    rejections: &std::collections::BTreeMap<FoundryCandidateRejectionReason, usize>,
) -> Option<String> {
    if rejections.is_empty() {
        return None;
    }

    Some(
        rejections
            .iter()
            .map(|(reason, count)| format!("{count} {}", rejection_label(*reason)))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn rejection_label(reason: FoundryCandidateRejectionReason) -> &'static str {
    match reason {
        FoundryCandidateRejectionReason::EmptyProgram => "empty program",
        FoundryCandidateRejectionReason::DuplicateProgram => "duplicate program",
        FoundryCandidateRejectionReason::EditRejected => "edit rejected",
        FoundryCandidateRejectionReason::CompileRejected => "compile rejected",
        FoundryCandidateRejectionReason::ConformanceRejected => "conformance rejected",
        FoundryCandidateRejectionReason::DescriptorRejected => "descriptor rejected",
    }
}

fn selected_candidate_card<'a>(
    candidate_slots: &'a [DirectionCardSlot],
    selected_candidate: Option<&FoundryCandidateId>,
) -> Option<&'a DirectionCardView> {
    let selected_candidate = selected_candidate?;
    candidate_slots
        .iter()
        .filter_map(DirectionCardSlot::as_card)
        .find(|candidate| candidate.card.id == *selected_candidate)
}

fn board_validation(
    parent: &DirectionCardView,
    candidate_slots: &[DirectionCardSlot],
    input_candidate_count: usize,
) -> DirectionBoardValidation {
    let filled_candidate_count = candidate_slots
        .iter()
        .filter(|slot| slot.as_card().is_some())
        .count();
    DirectionBoardValidation {
        candidate_slot_count: candidate_slots.len(),
        filled_candidate_count,
        preview_images_present: board_preview_images_are_present(parent, candidate_slots),
        overflow_candidate_count: input_candidate_count
            .saturating_sub(VISIBLE_DIRECTION_CANDIDATE_CARDS),
        fixed_camera: board_uses_fixed_camera(parent, candidate_slots),
        whole_model_only: board_is_whole_model_only(parent, candidate_slots),
        no_isolated_part_option_cards: true,
    }
}

fn board_preview_images_are_present(
    parent: &DirectionCardView,
    candidate_slots: &[DirectionCardSlot],
) -> bool {
    parent.preview.has_image()
        && candidate_slots
            .iter()
            .filter_map(DirectionCardSlot::as_card)
            .all(|candidate| candidate.preview.has_image())
}

fn board_uses_fixed_camera(
    parent: &DirectionCardView,
    candidate_slots: &[DirectionCardSlot],
) -> bool {
    let Some(parent_camera) = parent.preview.camera.as_ref() else {
        return false;
    };

    candidate_slots
        .iter()
        .filter_map(DirectionCardSlot::as_card)
        .all(|candidate| candidate.preview.camera.as_ref() == Some(parent_camera))
}

fn board_is_whole_model_only(
    parent: &DirectionCardView,
    candidate_slots: &[DirectionCardSlot],
) -> bool {
    parent.preview.scope == DirectionImageScope::WholeModel
        && candidate_slots.iter().all(|slot| match slot {
            DirectionCardSlot::Filled(card) => {
                card.preview.scope == DirectionImageScope::WholeModel
            }
            DirectionCardSlot::Empty(card) => card.preview_scope == DirectionImageScope::WholeModel,
        })
}

fn previews_share_fixed_camera(a: &DirectionPreviewView, b: &DirectionPreviewView) -> bool {
    a.scope == DirectionImageScope::WholeModel
        && b.scope == DirectionImageScope::WholeModel
        && a.camera.is_some()
        && a.camera == b.camera
}
