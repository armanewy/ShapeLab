//! Control-space candidate generation for Foundry documents.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use shape_asset::{GeometrySource, ModelingOperationSpec};
use shape_core::{Aabb, Scalar, Transform3};
use shape_foundry::{
    CandidateExplanationQuality, CandidateLegibilityClass, CandidateVariationMetadata,
    CandidateVisibleDeltaReport, ControlDeltaExplanation, ControlEvaluationContext, ControlKind,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION, FeasibleControlDomain, FoundryAssetDocument,
    FoundryCandidateId, FoundryCatalogResolver, FoundryCommand, FoundryCompilationError,
    FoundryCompilationOutput, FoundryConformanceSummary, FoundryEdit, FoundryLockMode,
    FoundryLockTarget, FoundryPartGroupDescriptor, FoundryPreferenceProfile,
    FoundryPreferenceScope, MaterialSlotChange, PerceptualCandidateReport,
    SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON, SemanticPartGroupChange, VariationChannel,
    VariationIntent, VariationScope, apply_foundry_command,
    built_in_part_group_descriptors_for_profile, canonicalize_control_value,
    compile_foundry_document, default_control_value, effective_control_domain,
    explain_control_delta,
};
use thiserror::Error;

use crate::asset::scoring::{
    AssetCandidateInput, AssetDescriptor, AssetScoredCandidate, AssetScoringReport,
    AssetSelectionPolicy, FIXED_CAMERA_COUNT, asset_descriptor_distance,
    score_and_select_asset_candidates_with_policy,
};

/// Minimum proposal programs a Foundry candidate request may ask the generator
/// to attempt.
pub const FOUNDRY_MIN_PROPOSAL_COUNT: usize = 8;
/// Maximum proposal programs a Foundry candidate request may ask the generator
/// to attempt.
pub const FOUNDRY_MAX_PROPOSAL_COUNT: usize = 72;
/// Maximum representative candidates returned to the native direction board.
pub const FOUNDRY_MAX_RESULT_COUNT: usize = 6;
const SILHOUETTE_MASK_SIDE: usize = 32;
const SILHOUETTE_MASK_WORDS: usize = (SILHOUETTE_MASK_SIDE * SILHOUETTE_MASK_SIDE) / 64;
const DEPTH_HISTOGRAM_BINS: usize = 8;
const EPSILON: Scalar = 1.0e-6;
const LEGIBILITY_DUPLICATE_AVERAGE_DELTA: f32 = 0.018;
const LEGIBILITY_DUPLICATE_MAX_DELTA: f32 = 0.035;
const LEGIBILITY_CLEAR_AVERAGE_DELTA: f32 = 0.075;
const LEGIBILITY_CLEAR_MAX_DELTA: f32 = 0.115;
const LEGIBILITY_STRONG_AVERAGE_DELTA: f32 = 0.16;
const LEGIBILITY_SUBTLE_DETAIL_DELTA: f32 = 0.035;
const LEGIBILITY_FOCUS_SELECTED_DELTA: f32 = 0.065;
const LEGIBILITY_ENDPOINT_CLEAR_DELTA: f32 = 0.06;
const INV_SQRT_2: f32 = 0.707_106_77;
const INV_SQRT_3: f32 = 0.577_350_26;
const INV_SQRT_6: f32 = 0.408_248_3;
const TWO_INV_SQRT_6: f32 = 0.816_496_6;

const SILHOUETTE_CAMERAS: [SilhouetteCamera; FIXED_CAMERA_COUNT] = [
    SilhouetteCamera {
        u: [1.0, 0.0, 0.0],
        v: [0.0, 1.0, 0.0],
        depth: [0.0, 0.0, 1.0],
    },
    SilhouetteCamera {
        u: [0.0, 0.0, 1.0],
        v: [0.0, 1.0, 0.0],
        depth: [1.0, 0.0, 0.0],
    },
    SilhouetteCamera {
        u: [1.0, 0.0, 0.0],
        v: [0.0, 0.0, 1.0],
        depth: [0.0, 1.0, 0.0],
    },
    SilhouetteCamera {
        u: [INV_SQRT_2, -INV_SQRT_2, 0.0],
        v: [INV_SQRT_6, INV_SQRT_6, -TWO_INV_SQRT_6],
        depth: [INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],
    },
];

#[derive(Debug, Copy, Clone)]
struct SilhouetteCamera {
    u: [f32; 3],
    v: [f32; 3],
    depth: [f32; 3],
}

/// Candidate search mode in Foundry control space.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCandidateMode {
    /// One or two local, topology-preserving control edits.
    Refine,
    /// Two to four broader control edits.
    Explore,
    /// Silhouette and major proportion controls only.
    Silhouette,
    /// Provider, role-presence, and repetition controls.
    Structure,
    /// Detail density and edge-treatment controls.
    Detail,
}

/// Request for Foundry control-space candidate plans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidateRequest {
    /// Deterministic generation seed.
    pub seed: u64,
    /// Maximum number of proposal programs to attempt. Must be between 8 and 72.
    pub proposal_count: usize,
    /// Maximum number of survivors to return. The generator caps this at six.
    pub result_count: usize,
    /// Search policy.
    pub mode: FoundryCandidateMode,
    /// Optional customizer strategy ID from the resolved profile.
    pub strategy_id: Option<String>,
    /// Optional local preference profile used to bias candidate selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preference_profile: Option<FoundryPreferenceProfile>,
    /// Product-safe variation intent.
    #[serde(default)]
    pub variation_intent: VariationIntent,
}

/// One accepted candidate plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidatePlan {
    /// Stable candidate ID within this generation request.
    pub id: FoundryCandidateId,
    /// Human-facing label.
    pub label: String,
    /// Replayable Foundry edit program used when the candidate is accepted.
    pub edit: FoundryEdit,
    /// Candidate document after applying `edit` to the unchanged parent.
    pub document: FoundryAssetDocument,
    /// Changed controls in command order.
    pub changed_controls: Vec<String>,
    /// Candidate diagnostics and explanations.
    pub diagnostics: FoundryCandidateDiagnostics,
    /// Mesh-derived descriptor used for duplicate collapse and diversity.
    pub descriptor: AssetDescriptor,
    /// Fingerprint of the compiled recipe.
    pub recipe_fingerprint: String,
    /// Final conformance summary from Foundry compilation.
    pub conformance: FoundryConformanceSummary,
    /// Product-safe variation metadata.
    #[serde(default)]
    pub variation_metadata: CandidateVariationMetadata,
}

/// Candidate generation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidateOutput {
    /// Surviving candidate plans in deterministic max-min diversity order.
    pub candidates: Vec<FoundryCandidatePlan>,
    /// Generation-level diagnostics.
    pub diagnostics: FoundryCandidateGenerationDiagnostics,
    /// Product-facing reliability report for empty or weak candidate results.
    #[serde(default)]
    pub reliability_report: FoundryCandidateReliabilityReport,
    /// Scoring, hard-rejection, duplicate-collapse, and diversity report.
    pub scoring_report: AssetScoringReport,
    /// Local preference-bias report.
    pub preference_report: FoundryCandidatePreferenceReport,
}

/// Product-facing minimum-usefulness status for a candidate request.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum FoundryCandidateMinimumResult {
    /// Enough candidates survived for the requested scope.
    #[default]
    Useful,
    /// Whole-asset generation returned fewer than two useful candidates.
    NoUsefulCandidates,
    /// Focused generation returned zero useful focused candidates.
    NoFocusedCandidates,
}

/// Structured product-facing candidate failure reasons.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCandidateFailureReason {
    /// Candidates collapsed into duplicate-looking ideas.
    TooSimilar,
    /// Candidate changes did not affect visible authored controls or geometry.
    HiddenChange,
    /// Candidate changes landed outside the requested focus scope.
    WrongScope,
    /// Locks or search protection removed all useful controls.
    LockedOut,
    /// The requested focused part has no bound controls.
    NoBoundControls,
    /// The available controls are visible only as subtle detail changes.
    ControlTooSubtle,
    /// Provider or channel support was unavailable.
    ProviderUnavailable,
    /// Render or descriptor delta evidence was unavailable.
    RenderDeltaUnavailable,
    /// Validation or conformance rejected the candidate.
    ValidationFailed,
}

impl FoundryCandidateFailureReason {
    /// Return a stable product label for this failure reason.
    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::TooSimilar => "Too similar",
            Self::HiddenChange => "Hidden change",
            Self::WrongScope => "Wrong scope",
            Self::LockedOut => "Locked out",
            Self::NoBoundControls => "No bound controls",
            Self::ControlTooSubtle => "Control too subtle",
            Self::ProviderUnavailable => "Provider unavailable",
            Self::RenderDeltaUnavailable => "Render delta unavailable",
            Self::ValidationFailed => "Validation failed",
        }
    }
}

/// Counted failure reason row, sorted by impact in reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateFailureReasonCount {
    /// Structured reason.
    pub reason: FoundryCandidateFailureReason,
    /// Number of observations behind this reason.
    pub count: usize,
}

/// Human-visible fallback action suggested after a weak candidate result.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCandidateFallbackAction {
    /// Leave the selected part and try whole-asset generation.
    TryWholeAssetIdeas,
    /// Clear the current focus selection.
    ClearFocus,
    /// Unlock controls that block candidate generation.
    UnlockControls,
    /// Select another authored part group.
    TryAnotherPart,
    /// Switch to Detail mode for subtle detail controls.
    UseDetailMode,
    /// The focused part is authored but has no focused variants.
    NoFocusedVariants,
}

impl FoundryCandidateFallbackAction {
    /// Return the exact user-facing action text.
    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::TryWholeAssetIdeas => "Try whole-asset ideas",
            Self::ClearFocus => "Clear focus",
            Self::UnlockControls => "Unlock controls",
            Self::TryAnotherPart => "Try another part",
            Self::UseDetailMode => "Use Detail mode",
            Self::NoFocusedVariants => "This part has no focused variants yet",
        }
    }
}

/// Focused-part candidate capability for one product-facing part group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryFocusedPartCapabilityReport {
    /// Stable part group ID.
    pub group_id: String,
    /// Product-facing display name.
    pub display_name: String,
    /// Whether Shape ideas can be generated for this group right now.
    pub can_generate_shape_ideas: bool,
    /// Deterministic estimate of useful focused candidate count.
    pub likely_candidate_count: usize,
    /// Structured blockers for this group.
    #[serde(default)]
    pub blocked_reasons: Vec<FoundryCandidateFailureReason>,
    /// Suggested user-visible action for this group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<FoundryCandidateFallbackAction>,
}

/// Product-facing reliability report attached to every candidate response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateReliabilityReport {
    /// Minimum useful-result status.
    pub minimum_result: FoundryCandidateMinimumResult,
    /// Most important reasons for a weak or empty result.
    #[serde(default)]
    pub top_reasons: Vec<FoundryCandidateFailureReasonCount>,
    /// Suggested user-visible fallback action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<FoundryCandidateFallbackAction>,
    /// Focused-part capability rows for known part groups.
    #[serde(default)]
    pub focused_part_capabilities: Vec<FoundryFocusedPartCapabilityReport>,
    /// Plain-language deterministic summary.
    pub human_summary: String,
}

/// Diagnostics for one accepted Foundry candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateDiagnostics {
    /// Changed controls in edit command order.
    pub changes: Vec<FoundryCandidateControlChange>,
}

/// One changed Foundry control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateControlChange {
    /// Stable category.
    pub kind: FoundryCandidateChangeKind,
    /// Control ID.
    pub control_id: String,
    /// Human-facing control label.
    pub control_label: String,
    /// Previous value summary.
    pub before: String,
    /// Candidate value summary.
    pub after: String,
    /// Human-facing explanation using the control label.
    pub message: String,
    /// Lower-level deterministic delta explanations from control evaluation.
    pub details: Vec<ControlDeltaExplanation>,
    /// Whether the control is topology-changing.
    pub topology_changing: bool,
}

/// Stable candidate change categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCandidateChangeKind {
    /// Continuous scalar control.
    Numeric,
    /// Integer or count-like control.
    Repetition,
    /// Boolean role-presence control.
    RolePresence,
    /// Choice-gallery control.
    Choice,
    /// Provider-gallery control.
    Provider,
    /// Detail-density or edge-treatment control.
    Detail,
}

/// Generation-level diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateGenerationDiagnostics {
    /// Requested proposals.
    pub requested_proposals: usize,
    /// Requested survivor count before the six-candidate cap.
    pub requested_candidates: usize,
    /// Proposal programs attempted.
    pub attempted_proposals: usize,
    /// Proposal rows sent to scoring, including hard-rejection rows.
    pub scored_candidates: usize,
    /// Candidate plans accepted before scoring representative selection.
    pub accepted_candidates: usize,
    /// Returned survivor count.
    pub returned_candidates: usize,
    /// Editable controls after strategy, visibility, mode, and lock filters.
    pub available_control_count: usize,
    /// Controls skipped because of Foundry locks or search protection.
    pub locked_targets_skipped: usize,
    /// Rejection counters.
    pub rejections: BTreeMap<FoundryCandidateRejectionReason, usize>,
    /// Duplicate-looking visual rejections, including duplicate collapse.
    #[serde(default)]
    pub duplicate_looking_rejections: usize,
    /// Hidden/internal or too-subtle visual rejections.
    #[serde(default)]
    pub hidden_internal_rejections: usize,
    /// Focus or scoped requests rejected because the visible change was outside scope.
    #[serde(default)]
    pub wrong_scope_rejections: usize,
    /// Human-readable generation summary.
    pub human_summary: String,
}

/// Visibility report for one profile's control endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryControlEndpointVisibilityReport {
    /// Profile ID that owns the controls.
    pub profile_id: String,
    /// Per-control endpoint rows.
    pub controls: Vec<FoundryControlEndpointVisibilityRow>,
    /// Product-safe warnings for controls that do not visibly read.
    pub warnings: Vec<String>,
}

/// Visibility report for one control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryControlEndpointVisibilityRow {
    /// Stable control ID.
    pub control_id: String,
    /// Human-facing label.
    pub control_label: String,
    /// Product-visible class for the strongest endpoint sample.
    pub legibility_class: CandidateLegibilityClass,
    /// Strongest visible delta report across sampled endpoints.
    pub visible_delta: CandidateVisibleDeltaReport,
    /// Strict perceptual evidence for the strongest endpoint sample.
    pub perceptual_report: PerceptualCandidateReport,
    /// Endpoint sample count attempted.
    pub endpoint_sample_count: usize,
    /// Plain-language warning when the endpoint is not clearly visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Report for optional local preference biasing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidatePreferenceReport {
    /// Whether a preference profile was supplied.
    pub requested: bool,
    /// Whether the supplied profile affected candidate selection.
    pub applied: bool,
    /// Whether the supplied profile matched the current family/profile scope.
    pub scope_matched: bool,
    /// Current family/profile scope used for matching.
    pub scope: FoundryPreferenceScope,
    /// Deterministic reason preference biasing was ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored_reason: Option<String>,
    /// Scores for selected candidates in final order.
    pub selected_scores: Vec<FoundryCandidatePreferenceScore>,
}

/// Preference score for one selected candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidatePreferenceScore {
    /// Candidate ID.
    pub candidate_id: FoundryCandidateId,
    /// Bounded profile score in `[-1, 1]`.
    pub score: f32,
    /// Bounded contribution added during selection.
    pub selection_bonus: f32,
    /// Visible controls changed by the candidate.
    pub changed_controls: Vec<String>,
}

/// Rejection reasons emitted during Foundry candidate generation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCandidateRejectionReason {
    /// The selected controls produced no command.
    EmptyProgram,
    /// A proposal repeated an already attempted command program.
    DuplicateProgram,
    /// Foundry command replay rejected the proposal.
    EditRejected,
    /// Foundry compilation rejected the proposal before conformance could pass.
    CompileRejected,
    /// The proposal compiled but final conformance did not accept it.
    ConformanceRejected,
    /// A descriptor could not be produced for scoring.
    DescriptorRejected,
    /// Requested variation channel is not supported by the current kit.
    UnsupportedChannel,
    /// Proposal changed no user-facing visible surface.
    HiddenOnlyChange,
    /// Candidate was too subtle for a normal direction card.
    TooSubtle,
    /// Candidate looked like a duplicate after preview comparison.
    DuplicateLooking,
    /// Product explanation did not match visible evidence.
    ExplanationMismatch,
}

/// Foundry candidate generation errors.
#[derive(Debug, Error)]
pub enum FoundryCandidateError {
    /// Request fields are inconsistent.
    #[error("invalid foundry candidate request: {0}")]
    InvalidRequest(&'static str),
    /// The input document or catalog failed parent compilation.
    #[error("parent foundry document failed compilation: {0}")]
    ParentCompilationFailed(String),
    /// The requested strategy does not exist in the resolved profile.
    #[error("unknown foundry candidate strategy `{0}`")]
    UnknownCandidateStrategy(String),
    /// No unlocked editable controls match the request.
    #[error("no editable foundry controls match the request")]
    NoEditableControls,
}

/// Generate Foundry candidate plans as replayable [`FoundryEdit`] programs.
///
/// The parent document is never mutated. Each proposal is applied to a clone,
/// compiled through Foundry, conformance-checked, converted to mesh-derived
/// descriptors, collapsed for duplicates, and selected with the existing
/// max-min diversity policy.
pub fn generate_foundry_candidate_plans(
    document: &FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
    request: &FoundryCandidateRequest,
) -> Result<FoundryCandidateOutput, FoundryCandidateError> {
    validate_request(request)?;

    let parent_output = compile_foundry_document(document, resolver)
        .map_err(|error| FoundryCandidateError::ParentCompilationFailed(format!("{error:?}")))?;
    let preference_scope = FoundryPreferenceScope::new(
        parent_output.catalog.family.id.clone(),
        document.customizer_profile_ref.stable_id.clone(),
    );
    let preference_profile =
        request_preference_profile(request.preference_profile.as_ref(), &preference_scope);
    let variation_intent = request.variation_intent.clone().normalized();
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    if let Some(reason) = unsupported_variation_reason(document, &variation_intent) {
        return Ok(empty_candidate_output(EmptyCandidateOutputContext {
            document,
            profile: &parent_output.catalog.customizer_profile,
            context,
            request,
            preference_scope,
            preference_profile,
            reason: FoundryCandidateRejectionReason::UnsupportedChannel,
            detail: reason,
            locked_targets_skipped: 0,
            forced_reason: Some(FoundryCandidateFailureReason::ProviderUnavailable),
        }));
    }
    let strategy_controls = strategy_control_ids(
        &parent_output.catalog.customizer_profile,
        &request.strategy_id,
    )?;

    let mut locked_targets_skipped = 0;
    let opportunities = collect_control_opportunities(
        document,
        &parent_output.catalog.customizer_profile,
        context,
        request.mode,
        &variation_intent,
        strategy_controls.as_ref(),
        &mut locked_targets_skipped,
    );
    if opportunities.is_empty() {
        let failure_reason = no_editable_failure_reason(
            document,
            &parent_output.catalog.customizer_profile,
            context,
            &variation_intent,
            locked_targets_skipped,
        );
        return Ok(empty_candidate_output(EmptyCandidateOutputContext {
            document,
            profile: &parent_output.catalog.customizer_profile,
            context,
            request,
            preference_scope,
            preference_profile,
            reason: FoundryCandidateRejectionReason::EmptyProgram,
            detail: "No editable controls match this candidate request.".to_owned(),
            locked_targets_skipped,
            forced_reason: Some(failure_reason),
        }));
    }

    let mut diagnostics = FoundryCandidateGenerationDiagnostics {
        requested_proposals: request.proposal_count,
        requested_candidates: request.result_count,
        attempted_proposals: 0,
        scored_candidates: 0,
        accepted_candidates: 0,
        returned_candidates: 0,
        available_control_count: opportunities.len(),
        locked_targets_skipped,
        rejections: BTreeMap::new(),
        duplicate_looking_rejections: 0,
        hidden_internal_rejections: 0,
        wrong_scope_rejections: 0,
        human_summary: String::new(),
    };
    let mut seen_programs = BTreeSet::new();
    let mut scoring_inputs = Vec::new();
    let mut accepted_plans = BTreeMap::<String, FoundryCandidatePlan>::new();
    let parent_scoring_input = descriptor_input_from_output("parent", &parent_output);
    let target_accepted_plans = request.result_count.clamp(1, FOUNDRY_MAX_RESULT_COUNT);

    for proposal_index in 0..request.proposal_count {
        diagnostics.attempted_proposals += 1;
        let proposal_seed = proposal_seed(request.seed, proposal_index as u64);
        let candidate_id = candidate_id(proposal_seed, proposal_index);
        let mut rng = ChaCha8Rng::seed_from_u64(proposal_seed);
        let selected = select_opportunities(&opportunities, request.mode, proposal_index, &mut rng);
        let proposal = build_candidate_edit(
            &parent_output.catalog.customizer_profile,
            context,
            selected,
            request.mode,
            &variation_intent,
            proposal_index,
            &mut rng,
        );
        let Some(proposal) = proposal else {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::EmptyProgram,
            );
            continue;
        };
        let program_key = format!("{:?}", proposal.edit.commands);
        if !seen_programs.insert(program_key) {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::DuplicateProgram,
            );
            continue;
        }

        let mut candidate_document = document.clone();
        let mut edit_failed = false;
        for command in &proposal.edit.commands {
            if apply_foundry_command(&mut candidate_document, command).is_err() {
                edit_failed = true;
                break;
            }
        }
        if edit_failed {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::EditRejected,
            );
            continue;
        }

        let compiled = match compile_foundry_document(&candidate_document, resolver) {
            Ok(output) => output,
            Err(error) => {
                let reason = compile_error_rejection_reason(&error);
                increment_rejection(&mut diagnostics, reason);
                let mut rejected_input =
                    descriptor_input_from_output(&candidate_id.0, &parent_output);
                rejected_input.compile_succeeded = false;
                rejected_input.recipe_fingerprint = format!("rejected-{}", candidate_id.0);
                scoring_inputs.push(rejected_input);
                continue;
            }
        };

        if !compiled.conformance_summary.accepted {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::ConformanceRejected,
            );
            let mut rejected_input = descriptor_input_from_output(&candidate_id.0, &parent_output);
            rejected_input.compile_succeeded = false;
            rejected_input.recipe_fingerprint = format!("rejected-{}", candidate_id.0);
            scoring_inputs.push(rejected_input);
            continue;
        }

        let mut scoring_input = descriptor_input_from_output(&candidate_id.0, &compiled);
        if !candidate_descriptor_is_usable(&scoring_input) {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::DescriptorRejected,
            );
            scoring_input.geometry_finite = false;
            scoring_inputs.push(scoring_input);
            continue;
        }

        let legibility_evidence = CandidateLegibilityEvidence {
            candidate_id: candidate_id.0.clone(),
            parent: &parent_scoring_input,
            candidate: &scoring_input,
        };
        let variation_metadata = variation_metadata_for_proposal(
            document,
            &variation_intent,
            &proposal,
            Some(&legibility_evidence),
        );
        if let Some(reason) = variation_rejection_reason(&variation_metadata) {
            if metadata_is_wrong_scope_rejection(&variation_metadata) {
                diagnostics.wrong_scope_rejections += 1;
            }
            increment_rejection(&mut diagnostics, reason);
            if matches!(
                reason,
                FoundryCandidateRejectionReason::DuplicateLooking
                    | FoundryCandidateRejectionReason::TooSubtle
            ) {
                scoring_inputs.push(scoring_input);
            }
            continue;
        }
        let plan = FoundryCandidatePlan {
            id: candidate_id,
            label: proposal.edit.label.clone(),
            edit: proposal.edit,
            document: candidate_document,
            changed_controls: proposal.changed_controls.clone(),
            diagnostics: FoundryCandidateDiagnostics {
                changes: proposal.changes.clone(),
            },
            descriptor: crate::asset::scoring::asset_descriptor(&scoring_input),
            recipe_fingerprint: scoring_input.recipe_fingerprint.clone(),
            conformance: compiled.conformance_summary,
            variation_metadata,
        };
        scoring_inputs.push(scoring_input);
        accepted_plans.insert(plan.id.0.clone(), plan);
        if accepted_plans.len() >= target_accepted_plans {
            break;
        }
    }

    diagnostics.scored_candidates = scoring_inputs.len();
    diagnostics.accepted_candidates = accepted_plans.len();

    let scoring_policy = AssetSelectionPolicy {
        representative_count: request.result_count.min(FOUNDRY_MAX_RESULT_COUNT),
        duplicate_descriptor_distance: 0.005,
        ..AssetSelectionPolicy::default()
    };
    let scoring_report =
        score_and_select_asset_candidates_with_policy(&scoring_inputs, &scoring_policy);
    let duplicate_collapses = accepted_duplicate_member_count(&scoring_report, &accepted_plans);
    for _ in 0..duplicate_collapses {
        increment_rejection(
            &mut diagnostics,
            FoundryCandidateRejectionReason::DuplicateLooking,
        );
    }
    let selected_ids = selected_candidate_ids(
        &scoring_report,
        &accepted_plans,
        preference_profile,
        &scoring_policy,
    );
    let preference_report = preference_report(
        request.preference_profile.as_ref(),
        preference_profile,
        preference_scope,
        &selected_ids,
        &accepted_plans,
    );
    let candidates = selected_ids
        .iter()
        .filter_map(|candidate_id| accepted_plans.remove(candidate_id))
        .collect::<Vec<_>>();
    diagnostics.returned_candidates = candidates.len();
    update_legibility_rejection_totals(&mut diagnostics);
    diagnostics.human_summary = candidate_generation_human_summary(&diagnostics);
    let reliability_report = candidate_reliability_report(
        document,
        &parent_output.catalog.customizer_profile,
        context,
        &variation_intent,
        &diagnostics,
        candidates.len(),
        None,
    );

    Ok(FoundryCandidateOutput {
        candidates,
        diagnostics,
        reliability_report,
        scoring_report,
        preference_report,
    })
}

/// Generate replayable candidate draft plans without compiling every candidate.
///
/// This is the interactive path for direction boards: it validates the request,
/// resolves the parent profile once, builds deterministic edits, and defers
/// candidate compilation/conformance to the preview legibility job.
pub fn generate_foundry_candidate_draft_plans(
    document: &FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
    request: &FoundryCandidateRequest,
) -> Result<FoundryCandidateOutput, FoundryCandidateError> {
    validate_request(request)?;

    let parent_output = compile_foundry_document(document, resolver)
        .map_err(|error| FoundryCandidateError::ParentCompilationFailed(format!("{error:?}")))?;
    let preference_scope = FoundryPreferenceScope::new(
        parent_output.catalog.family.id.clone(),
        document.customizer_profile_ref.stable_id.clone(),
    );
    let preference_profile =
        request_preference_profile(request.preference_profile.as_ref(), &preference_scope);
    let variation_intent = request.variation_intent.clone().normalized();
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    if let Some(reason) = unsupported_variation_reason(document, &variation_intent) {
        return Ok(empty_candidate_output(EmptyCandidateOutputContext {
            document,
            profile: &parent_output.catalog.customizer_profile,
            context,
            request,
            preference_scope,
            preference_profile,
            reason: FoundryCandidateRejectionReason::UnsupportedChannel,
            detail: reason,
            locked_targets_skipped: 0,
            forced_reason: Some(FoundryCandidateFailureReason::ProviderUnavailable),
        }));
    }
    let strategy_controls = strategy_control_ids(
        &parent_output.catalog.customizer_profile,
        &request.strategy_id,
    )?;

    let mut locked_targets_skipped = 0;
    let opportunities = collect_control_opportunities(
        document,
        &parent_output.catalog.customizer_profile,
        context,
        request.mode,
        &variation_intent,
        strategy_controls.as_ref(),
        &mut locked_targets_skipped,
    );
    if opportunities.is_empty() {
        let failure_reason = no_editable_failure_reason(
            document,
            &parent_output.catalog.customizer_profile,
            context,
            &variation_intent,
            locked_targets_skipped,
        );
        return Ok(empty_candidate_output(EmptyCandidateOutputContext {
            document,
            profile: &parent_output.catalog.customizer_profile,
            context,
            request,
            preference_scope,
            preference_profile,
            reason: FoundryCandidateRejectionReason::EmptyProgram,
            detail: "No editable controls match this candidate request.".to_owned(),
            locked_targets_skipped,
            forced_reason: Some(failure_reason),
        }));
    }

    let mut diagnostics = FoundryCandidateGenerationDiagnostics {
        requested_proposals: request.proposal_count,
        requested_candidates: request.result_count,
        attempted_proposals: 0,
        scored_candidates: 0,
        accepted_candidates: 0,
        returned_candidates: 0,
        available_control_count: opportunities.len(),
        locked_targets_skipped,
        rejections: BTreeMap::new(),
        duplicate_looking_rejections: 0,
        hidden_internal_rejections: 0,
        wrong_scope_rejections: 0,
        human_summary: String::new(),
    };
    let mut seen_programs = BTreeSet::new();
    let mut accepted_plans = BTreeMap::<String, FoundryCandidatePlan>::new();
    let mut selected_ids = Vec::new();
    let parent_descriptor = crate::asset::scoring::asset_descriptor(&descriptor_input_from_output(
        "parent",
        &parent_output,
    ));
    let target_accepted_plans = request.result_count.clamp(1, FOUNDRY_MAX_RESULT_COUNT);

    for proposal_index in 0..request.proposal_count {
        diagnostics.attempted_proposals += 1;
        let proposal_seed = proposal_seed(request.seed, proposal_index as u64);
        let candidate_id = candidate_id(proposal_seed, proposal_index);
        let mut rng = ChaCha8Rng::seed_from_u64(proposal_seed);
        let selected = select_opportunities(&opportunities, request.mode, proposal_index, &mut rng);
        let proposal = build_candidate_edit(
            &parent_output.catalog.customizer_profile,
            context,
            selected,
            request.mode,
            &variation_intent,
            proposal_index,
            &mut rng,
        );
        let Some(proposal) = proposal else {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::EmptyProgram,
            );
            continue;
        };
        let program_key = format!("{:?}", proposal.edit.commands);
        if !seen_programs.insert(program_key) {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::DuplicateProgram,
            );
            continue;
        }

        let mut candidate_document = document.clone();
        let mut edit_failed = false;
        for command in &proposal.edit.commands {
            if apply_foundry_command(&mut candidate_document, command).is_err() {
                edit_failed = true;
                break;
            }
        }
        if edit_failed {
            increment_rejection(
                &mut diagnostics,
                FoundryCandidateRejectionReason::EditRejected,
            );
            continue;
        }

        let candidate_key = candidate_id.0.clone();
        let variation_metadata =
            variation_metadata_for_proposal(document, &variation_intent, &proposal, None);
        // Draft generation runs before candidate preview rendering. Visual
        // duplicate/subtle rejection needs rendered evidence and is applied by
        // the preview legibility pass instead of this pending-card path.
        let plan = FoundryCandidatePlan {
            id: candidate_id,
            label: proposal.edit.label.clone(),
            edit: proposal.edit,
            document: candidate_document,
            changed_controls: proposal.changed_controls.clone(),
            diagnostics: FoundryCandidateDiagnostics {
                changes: proposal.changes.clone(),
            },
            descriptor: parent_descriptor.clone(),
            recipe_fingerprint: format!("draft-{candidate_key}"),
            conformance: FoundryConformanceSummary {
                accepted: true,
                required_failure_count: 0,
                advisory_issue_count: 0,
                runtime_deferred_count: 0,
            },
            variation_metadata,
        };
        selected_ids.push(candidate_key.clone());
        accepted_plans.insert(candidate_key, plan);
        if accepted_plans.len() >= target_accepted_plans {
            break;
        }
    }

    diagnostics.scored_candidates = accepted_plans.len();
    diagnostics.accepted_candidates = accepted_plans.len();
    selected_ids.truncate(request.result_count.min(FOUNDRY_MAX_RESULT_COUNT));
    let preference_report = preference_report(
        request.preference_profile.as_ref(),
        preference_profile,
        preference_scope,
        &selected_ids,
        &accepted_plans,
    );
    let candidates = selected_ids
        .iter()
        .filter_map(|candidate_id| accepted_plans.remove(candidate_id))
        .collect::<Vec<_>>();
    diagnostics.returned_candidates = candidates.len();
    update_legibility_rejection_totals(&mut diagnostics);
    diagnostics.human_summary = candidate_generation_human_summary(&diagnostics);
    let reliability_report = candidate_reliability_report(
        document,
        &parent_output.catalog.customizer_profile,
        context,
        &variation_intent,
        &diagnostics,
        candidates.len(),
        None,
    );

    Ok(FoundryCandidateOutput {
        candidates,
        diagnostics,
        reliability_report,
        scoring_report: AssetScoringReport {
            rejected_candidates: Vec::new(),
            scored_candidates: Vec::new(),
            unique_candidates: Vec::new(),
            duplicate_groups: Vec::new(),
            representatives: Vec::new(),
        },
        preference_report,
    })
}

/// Generate a strict visible endpoint report for every visible profile control.
pub fn generate_foundry_control_endpoint_visibility_report(
    document: &FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryControlEndpointVisibilityReport, FoundryCandidateError> {
    let parent_output = compile_foundry_document(document, resolver)
        .map_err(|error| FoundryCandidateError::ParentCompilationFailed(format!("{error:?}")))?;
    let parent_input = descriptor_input_from_output("parent", &parent_output);
    let profile = &parent_output.catalog.customizer_profile;
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    let mut rows = Vec::new();
    let mut warnings = Vec::new();

    for control in profile.controls.iter().filter(|control| {
        control.visible && control.topology_behavior != ControlTopologyBehavior::RuntimeOnly
    }) {
        let domain = effective_control_domain(control, context).unwrap_or_else(|_| {
            let mut domain = control.domain.clone();
            let unavailable = domain
                .unavailable_options
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            domain
                .discrete_values
                .retain(|value| !unavailable.contains(&value.option_key()));
            domain
        });
        if !domain.has_available_values() {
            let row =
                unsupported_endpoint_row(control, 0, "Control endpoint domain is unavailable.");
            if let Some(warning) = &row.warning {
                warnings.push(warning.clone());
            }
            rows.push(row);
            continue;
        }
        let samples = endpoint_samples_for_control(control, &domain, context);
        if samples.is_empty() {
            let row =
                unsupported_endpoint_row(control, 0, "Control endpoint has no sample values.");
            if let Some(warning) = &row.warning {
                warnings.push(warning.clone());
            }
            rows.push(row);
            continue;
        }

        let mut best: Option<(CandidateVisibleDeltaReport, PerceptualCandidateReport)> = None;
        let mut attempted = 0_usize;
        for (sample_index, value) in samples.into_iter().enumerate() {
            let mut candidate_document = document.clone();
            if apply_foundry_command(
                &mut candidate_document,
                &FoundryCommand::SetControl {
                    control_id: control.id.clone(),
                    value,
                },
            )
            .is_err()
            {
                continue;
            }
            let Ok(compiled) = compile_foundry_document(&candidate_document, resolver) else {
                continue;
            };
            if !compiled.conformance_summary.accepted {
                continue;
            }
            attempted += 1;
            let endpoint_id = format!("endpoint-{}-{sample_index}", control.id);
            let candidate_input = descriptor_input_from_output(&endpoint_id, &compiled);
            let proposal = endpoint_candidate_proposal(control, context);
            let changed_groups = changed_part_groups_for_proposal(document, &proposal);
            let evidence = CandidateLegibilityEvidence {
                candidate_id: endpoint_id,
                parent: &parent_input,
                candidate: &candidate_input,
            };
            let visible_delta =
                endpoint_visible_delta_report(&changed_groups, &proposal, &evidence);
            let perceptual_report = perceptual_candidate_report(
                &VariationIntent::whole_asset_shape(),
                &proposal,
                &changed_groups,
                &visible_delta,
                &evidence,
            );
            let replace_best = best.as_ref().is_none_or(|(existing, _)| {
                visible_delta.screen_space_delta_score > existing.screen_space_delta_score
            });
            if replace_best {
                best = Some((visible_delta, perceptual_report));
            }
        }

        let Some((visible_delta, perceptual_report)) = best else {
            let row = unsupported_endpoint_row(
                control,
                attempted,
                "Control endpoint samples did not compile into valid preview evidence.",
            );
            if let Some(warning) = &row.warning {
                warnings.push(warning.clone());
            }
            rows.push(row);
            continue;
        };
        let warning = endpoint_warning(control, visible_delta.legibility_class);
        if let Some(warning) = &warning {
            warnings.push(warning.clone());
        }
        rows.push(FoundryControlEndpointVisibilityRow {
            control_id: control.id.clone(),
            control_label: control.label.clone(),
            legibility_class: visible_delta.legibility_class,
            visible_delta,
            perceptual_report,
            endpoint_sample_count: attempted,
            warning,
        });
    }

    rows.sort_by(|left, right| left.control_id.cmp(&right.control_id));
    warnings.sort();
    Ok(FoundryControlEndpointVisibilityReport {
        profile_id: profile.family_id.clone(),
        controls: rows,
        warnings,
    })
}

fn validate_request(request: &FoundryCandidateRequest) -> Result<(), FoundryCandidateError> {
    if request.proposal_count < FOUNDRY_MIN_PROPOSAL_COUNT
        || request.proposal_count > FOUNDRY_MAX_PROPOSAL_COUNT
    {
        return Err(FoundryCandidateError::InvalidRequest(
            "proposal_count must be between 8 and 72",
        ));
    }
    if request.result_count == 0 {
        return Err(FoundryCandidateError::InvalidRequest(
            "result_count must be greater than zero",
        ));
    }
    Ok(())
}

fn candidate_generation_human_summary(
    diagnostics: &FoundryCandidateGenerationDiagnostics,
) -> String {
    format!(
        "Generated {} clear ideas. Rejected {} that looked too similar. Rejected {} because changes were hidden/internal. Rejected {} because changes were wrong scope.",
        diagnostics.returned_candidates,
        diagnostics.duplicate_looking_rejections,
        diagnostics.hidden_internal_rejections,
        diagnostics.wrong_scope_rejections
    )
}

fn update_legibility_rejection_totals(diagnostics: &mut FoundryCandidateGenerationDiagnostics) {
    diagnostics.duplicate_looking_rejections = rejection_count(
        diagnostics,
        FoundryCandidateRejectionReason::DuplicateLooking,
    );
    let subtle_or_hidden = rejection_count(diagnostics, FoundryCandidateRejectionReason::TooSubtle)
        + rejection_count(
            diagnostics,
            FoundryCandidateRejectionReason::HiddenOnlyChange,
        );
    diagnostics.hidden_internal_rejections =
        subtle_or_hidden.saturating_sub(diagnostics.wrong_scope_rejections);
}

fn rejection_count(
    diagnostics: &FoundryCandidateGenerationDiagnostics,
    reason: FoundryCandidateRejectionReason,
) -> usize {
    diagnostics.rejections.get(&reason).copied().unwrap_or(0)
}

fn candidate_reliability_report(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    variation_intent: &VariationIntent,
    diagnostics: &FoundryCandidateGenerationDiagnostics,
    returned_candidate_count: usize,
    forced_reason: Option<FoundryCandidateFailureReason>,
) -> FoundryCandidateReliabilityReport {
    let minimum_result = minimum_result_for_request(variation_intent, returned_candidate_count);
    let focused_part_capabilities = focused_part_capability_reports(document, profile, context);
    let top_reasons = top_failure_reasons(
        diagnostics,
        forced_reason,
        minimum_result,
        &focused_part_capabilities,
        variation_intent,
    );
    let suggested_action = fallback_action_for_report(
        minimum_result,
        variation_intent,
        top_reasons.first().map(|row| row.reason),
    );
    let human_summary = reliability_human_summary(
        minimum_result,
        returned_candidate_count,
        &top_reasons,
        suggested_action,
    );
    FoundryCandidateReliabilityReport {
        minimum_result,
        top_reasons,
        suggested_action,
        focused_part_capabilities,
        human_summary,
    }
}

fn minimum_result_for_request(
    variation_intent: &VariationIntent,
    returned_candidate_count: usize,
) -> FoundryCandidateMinimumResult {
    if variation_intent.scope.is_focus_part() && returned_candidate_count == 0 {
        FoundryCandidateMinimumResult::NoFocusedCandidates
    } else if !variation_intent.scope.is_focus_part() && returned_candidate_count < 2 {
        FoundryCandidateMinimumResult::NoUsefulCandidates
    } else {
        FoundryCandidateMinimumResult::Useful
    }
}

fn top_failure_reasons(
    diagnostics: &FoundryCandidateGenerationDiagnostics,
    forced_reason: Option<FoundryCandidateFailureReason>,
    minimum_result: FoundryCandidateMinimumResult,
    focused_part_capabilities: &[FoundryFocusedPartCapabilityReport],
    variation_intent: &VariationIntent,
) -> Vec<FoundryCandidateFailureReasonCount> {
    if minimum_result == FoundryCandidateMinimumResult::Useful && forced_reason.is_none() {
        return Vec::new();
    }

    let mut counts = BTreeMap::<FoundryCandidateFailureReason, usize>::new();
    for (reason, count) in &diagnostics.rejections {
        let failure_reason = failure_reason_for_rejection(*reason);
        *counts.entry(failure_reason).or_insert(0) += *count;
    }
    if diagnostics.wrong_scope_rejections > 0 {
        *counts
            .entry(FoundryCandidateFailureReason::WrongScope)
            .or_insert(0) += diagnostics.wrong_scope_rejections;
    }
    if let Some(reason) = forced_reason {
        let forced_count = diagnostics
            .rejections
            .values()
            .copied()
            .sum::<usize>()
            .saturating_add(1)
            .max(1);
        *counts.entry(reason).or_insert(0) += forced_count;
    }
    if counts.is_empty() {
        if let Some(reason) =
            focused_request_blocked_reason(focused_part_capabilities, variation_intent)
        {
            counts.insert(reason, 1);
        } else {
            counts.insert(FoundryCandidateFailureReason::HiddenChange, 1);
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|(reason, count)| FoundryCandidateFailureReasonCount { reason, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.reason.cmp(&right.reason))
    });
    rows.truncate(3);
    rows
}

fn failure_reason_for_rejection(
    reason: FoundryCandidateRejectionReason,
) -> FoundryCandidateFailureReason {
    match reason {
        FoundryCandidateRejectionReason::DuplicateProgram
        | FoundryCandidateRejectionReason::DuplicateLooking => {
            FoundryCandidateFailureReason::TooSimilar
        }
        FoundryCandidateRejectionReason::EmptyProgram
        | FoundryCandidateRejectionReason::HiddenOnlyChange
        | FoundryCandidateRejectionReason::ExplanationMismatch => {
            FoundryCandidateFailureReason::HiddenChange
        }
        FoundryCandidateRejectionReason::TooSubtle => {
            FoundryCandidateFailureReason::ControlTooSubtle
        }
        FoundryCandidateRejectionReason::UnsupportedChannel
        | FoundryCandidateRejectionReason::EditRejected => {
            FoundryCandidateFailureReason::ProviderUnavailable
        }
        FoundryCandidateRejectionReason::DescriptorRejected => {
            FoundryCandidateFailureReason::RenderDeltaUnavailable
        }
        FoundryCandidateRejectionReason::CompileRejected
        | FoundryCandidateRejectionReason::ConformanceRejected => {
            FoundryCandidateFailureReason::ValidationFailed
        }
    }
}

fn focused_request_blocked_reason(
    focused_part_capabilities: &[FoundryFocusedPartCapabilityReport],
    variation_intent: &VariationIntent,
) -> Option<FoundryCandidateFailureReason> {
    let group_id = variation_intent.scope.semantic_part_group_id()?;
    focused_part_capabilities
        .iter()
        .find(|row| row.group_id == group_id)
        .and_then(|row| row.blocked_reasons.first().copied())
}

fn fallback_action_for_report(
    minimum_result: FoundryCandidateMinimumResult,
    variation_intent: &VariationIntent,
    top_reason: Option<FoundryCandidateFailureReason>,
) -> Option<FoundryCandidateFallbackAction> {
    if minimum_result == FoundryCandidateMinimumResult::Useful {
        return None;
    }
    match top_reason {
        Some(FoundryCandidateFailureReason::LockedOut) => {
            Some(FoundryCandidateFallbackAction::UnlockControls)
        }
        Some(FoundryCandidateFailureReason::NoBoundControls) => {
            Some(FoundryCandidateFallbackAction::NoFocusedVariants)
        }
        Some(FoundryCandidateFailureReason::ControlTooSubtle) => {
            Some(FoundryCandidateFallbackAction::UseDetailMode)
        }
        Some(FoundryCandidateFailureReason::WrongScope) => {
            Some(FoundryCandidateFallbackAction::ClearFocus)
        }
        Some(
            FoundryCandidateFailureReason::ProviderUnavailable
            | FoundryCandidateFailureReason::RenderDeltaUnavailable
            | FoundryCandidateFailureReason::ValidationFailed,
        ) if variation_intent.scope.is_focus_part() => {
            Some(FoundryCandidateFallbackAction::TryAnotherPart)
        }
        Some(
            FoundryCandidateFailureReason::HiddenChange | FoundryCandidateFailureReason::TooSimilar,
        ) if variation_intent.scope.is_focus_part() => {
            Some(FoundryCandidateFallbackAction::ClearFocus)
        }
        _ => Some(FoundryCandidateFallbackAction::TryWholeAssetIdeas),
    }
}

fn reliability_human_summary(
    minimum_result: FoundryCandidateMinimumResult,
    returned_candidate_count: usize,
    top_reasons: &[FoundryCandidateFailureReasonCount],
    suggested_action: Option<FoundryCandidateFallbackAction>,
) -> String {
    if minimum_result == FoundryCandidateMinimumResult::Useful {
        return format!("Generated {returned_candidate_count} useful candidate(s).");
    }
    let status = match minimum_result {
        FoundryCandidateMinimumResult::Useful => "Useful",
        FoundryCandidateMinimumResult::NoUsefulCandidates => "NoUsefulCandidates",
        FoundryCandidateMinimumResult::NoFocusedCandidates => "NoFocusedCandidates",
    };
    let reasons = if top_reasons.is_empty() {
        "no structured reason".to_owned()
    } else {
        top_reasons
            .iter()
            .map(|row| format!("{} ({})", row.reason.display_label(), row.count))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let action = suggested_action
        .map(FoundryCandidateFallbackAction::display_label)
        .unwrap_or("Try whole-asset ideas");
    format!("{status}: {reasons}. Suggested fallback: {action}.")
}

fn focused_part_capability_reports(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
) -> Vec<FoundryFocusedPartCapabilityReport> {
    let slot_roles = slot_roles_by_id(context);
    let mut descriptors = built_in_part_group_descriptors(document);
    descriptors.sort_by(|left, right| left.group_id.cmp(&right.group_id));
    descriptors
        .iter()
        .map(|descriptor| {
            focused_part_capability_report_for_descriptor(
                document,
                profile,
                context,
                &slot_roles,
                descriptor,
            )
        })
        .collect()
}

fn focused_part_capability_report_for_descriptor(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    slot_roles: &BTreeMap<&str, &str>,
    descriptor: &FoundryPartGroupDescriptor,
) -> FoundryFocusedPartCapabilityReport {
    let mut matched_controls = 0_usize;
    let mut locked_controls = 0_usize;
    let mut subtle_controls = 0_usize;
    let mut provider_unavailable_controls = 0_usize;
    let mut useful_capacity = 0_usize;
    let focus_intent = VariationIntent::focus_part_shape(
        descriptor.group_id.clone(),
        descriptor.display_name.clone(),
    );

    let mut controls = profile.controls.iter().collect::<Vec<_>>();
    controls.sort_by(|left, right| left.id.cmp(&right.id));
    for control in controls {
        if !control_matches_part_group_descriptor(control, descriptor) {
            continue;
        }
        matched_controls += 1;
        if !control.visible || control.topology_behavior == ControlTopologyBehavior::RuntimeOnly {
            continue;
        }
        let kind = classify_control_kind(control, slot_roles);
        if protected_by_lock(document, control, &kind) {
            locked_controls += 1;
            continue;
        }
        let class = classify_control(control, &kind);
        if class == ControlClass::Detail {
            subtle_controls += 1;
        }
        if !mode_allows_control(
            FoundryCandidateMode::Refine,
            class,
            control.topology_behavior,
            &kind,
            &focus_intent,
        ) || !variation_allows_control(document, &focus_intent, control, class)
        {
            continue;
        }
        let Ok(domain) = effective_control_domain(control, context) else {
            provider_unavailable_controls += usize::from(matches!(
                kind,
                ControlOpportunityKind::Provider { .. }
                    | ControlOpportunityKind::RolePresence { .. }
            ));
            continue;
        };
        let Some(current) = current_control_value(document, control, context, &domain) else {
            continue;
        };
        if !domain.contains_available_value(&current) {
            continue;
        }
        let capacity = control_candidate_capacity(&kind, &domain, &current);
        if capacity == 0
            && matches!(
                kind,
                ControlOpportunityKind::Provider { .. }
                    | ControlOpportunityKind::Choice
                    | ControlOpportunityKind::RolePresence { .. }
            )
        {
            provider_unavailable_controls += 1;
        }
        useful_capacity = useful_capacity.saturating_add(capacity);
    }

    let likely_candidate_count = useful_capacity.min(FOUNDRY_MAX_RESULT_COUNT);
    let mut blocked_reasons = Vec::new();
    if !descriptor.focusable
        || !descriptor.supports_channel(&VariationChannel::Shape)
        || matched_controls == 0
        || descriptor.bound_control_ids.is_empty() && descriptor.bound_provider_roles.is_empty()
    {
        blocked_reasons.push(FoundryCandidateFailureReason::NoBoundControls);
    }
    if likely_candidate_count == 0 && locked_controls > 0 {
        push_unique_failure_reason(
            &mut blocked_reasons,
            FoundryCandidateFailureReason::LockedOut,
        );
    }
    if likely_candidate_count == 0 && subtle_controls > 0 {
        push_unique_failure_reason(
            &mut blocked_reasons,
            FoundryCandidateFailureReason::ControlTooSubtle,
        );
    }
    if likely_candidate_count == 0 && provider_unavailable_controls > 0 {
        push_unique_failure_reason(
            &mut blocked_reasons,
            FoundryCandidateFailureReason::ProviderUnavailable,
        );
    }
    if likely_candidate_count == 0 && blocked_reasons.is_empty() {
        blocked_reasons.push(FoundryCandidateFailureReason::HiddenChange);
    }
    let can_generate_shape_ideas = likely_candidate_count > 0;
    let suggested_action = (!can_generate_shape_ideas)
        .then(|| fallback_action_for_capability(&blocked_reasons))
        .flatten();

    FoundryFocusedPartCapabilityReport {
        group_id: descriptor.group_id.clone(),
        display_name: descriptor.display_name.clone(),
        can_generate_shape_ideas,
        likely_candidate_count,
        blocked_reasons,
        suggested_action,
    }
}

fn slot_roles_by_id(context: ControlEvaluationContext<'_>) -> BTreeMap<&str, &str> {
    context
        .family_parameter_slots
        .iter()
        .filter_map(|slot| {
            slot.target_role
                .as_ref()
                .map(|role| (slot.id.as_str(), role.as_str()))
        })
        .collect()
}

fn control_matches_part_group_descriptor(
    control: &CustomizerControl,
    descriptor: &FoundryPartGroupDescriptor,
) -> bool {
    if descriptor
        .bound_control_ids
        .iter()
        .any(|control_id| control_id == &control.id)
        || control.bindings.iter().any(|binding| {
            descriptor
                .bound_control_ids
                .iter()
                .any(|control_id| control_id == &binding.slot)
        })
    {
        return true;
    }
    matches!(
        &control.kind,
        ControlKind::ProviderGallery { role, .. }
            if descriptor
                .bound_provider_roles
                .iter()
                .any(|provider_role| provider_role == role)
    )
}

fn current_control_value(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    domain: &FeasibleControlDomain,
) -> Option<ControlValue> {
    let raw_current = document
        .control_state
        .get(&control.id)
        .cloned()
        .unwrap_or_else(|| {
            default_control_value(control, context).unwrap_or_else(|_| first_domain_value(domain))
        });
    canonicalize_control_value(control, context, raw_current).ok()
}

fn control_candidate_capacity(
    kind: &ControlOpportunityKind,
    domain: &FeasibleControlDomain,
    current: &ControlValue,
) -> usize {
    match (kind, current) {
        (ControlOpportunityKind::Scalar, ControlValue::Scalar(_)) => {
            usize::from(domain.continuous_intervals.iter().any(|interval| {
                domain.contains_available_value(&ControlValue::Scalar(interval.minimum))
                    || domain.contains_available_value(&ControlValue::Scalar(interval.maximum))
            }))
        }
        (ControlOpportunityKind::Integer, ControlValue::Integer(current)) => {
            available_integers(domain)
                .into_iter()
                .filter(|value| value != current)
                .count()
        }
        (ControlOpportunityKind::Toggle, ControlValue::Toggle(current))
        | (ControlOpportunityKind::RolePresence { .. }, ControlValue::Toggle(current)) => {
            usize::from(domain.contains_available_value(&ControlValue::Toggle(!current)))
        }
        (ControlOpportunityKind::Choice, ControlValue::Choice(_))
        | (ControlOpportunityKind::Provider { .. }, ControlValue::Provider(_)) => domain
            .discrete_values
            .iter()
            .filter(|value| *value != current && domain.contains_available_value(value))
            .count(),
        _ => 0,
    }
}

fn fallback_action_for_capability(
    blocked_reasons: &[FoundryCandidateFailureReason],
) -> Option<FoundryCandidateFallbackAction> {
    if blocked_reasons.contains(&FoundryCandidateFailureReason::LockedOut) {
        Some(FoundryCandidateFallbackAction::UnlockControls)
    } else if blocked_reasons.contains(&FoundryCandidateFailureReason::ControlTooSubtle) {
        Some(FoundryCandidateFallbackAction::UseDetailMode)
    } else if blocked_reasons.contains(&FoundryCandidateFailureReason::NoBoundControls) {
        Some(FoundryCandidateFallbackAction::NoFocusedVariants)
    } else if blocked_reasons.contains(&FoundryCandidateFailureReason::ProviderUnavailable) {
        Some(FoundryCandidateFallbackAction::TryAnotherPart)
    } else if blocked_reasons.is_empty() {
        None
    } else {
        Some(FoundryCandidateFallbackAction::TryWholeAssetIdeas)
    }
}

fn push_unique_failure_reason(
    reasons: &mut Vec<FoundryCandidateFailureReason>,
    reason: FoundryCandidateFailureReason,
) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn no_editable_failure_reason(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    variation_intent: &VariationIntent,
    locked_targets_skipped: usize,
) -> FoundryCandidateFailureReason {
    let capabilities = focused_part_capability_reports(document, profile, context);
    if let Some(reason) = focused_request_blocked_reason(&capabilities, variation_intent) {
        return reason;
    }
    if locked_targets_skipped > 0 {
        FoundryCandidateFailureReason::LockedOut
    } else if variation_intent.scope.is_focus_part() {
        FoundryCandidateFailureReason::NoBoundControls
    } else {
        FoundryCandidateFailureReason::HiddenChange
    }
}

struct EmptyCandidateOutputContext<'a> {
    document: &'a FoundryAssetDocument,
    profile: &'a CustomizerProfile,
    context: ControlEvaluationContext<'a>,
    request: &'a FoundryCandidateRequest,
    preference_scope: FoundryPreferenceScope,
    preference_profile: Option<&'a FoundryPreferenceProfile>,
    reason: FoundryCandidateRejectionReason,
    detail: String,
    locked_targets_skipped: usize,
    forced_reason: Option<FoundryCandidateFailureReason>,
}

fn empty_candidate_output(input: EmptyCandidateOutputContext<'_>) -> FoundryCandidateOutput {
    let mut rejections = BTreeMap::new();
    rejections.insert(
        input.reason,
        input
            .request
            .result_count
            .clamp(1, FOUNDRY_MAX_RESULT_COUNT),
    );
    let diagnostics = FoundryCandidateGenerationDiagnostics {
        requested_proposals: input.request.proposal_count,
        requested_candidates: input.request.result_count,
        attempted_proposals: 0,
        scored_candidates: 0,
        accepted_candidates: 0,
        returned_candidates: 0,
        available_control_count: 0,
        locked_targets_skipped: input.locked_targets_skipped,
        rejections,
        duplicate_looking_rejections: 0,
        hidden_internal_rejections: 0,
        wrong_scope_rejections: 0,
        human_summary: String::new(),
    };
    let mut preference_report = preference_report(
        input.request.preference_profile.as_ref(),
        input.preference_profile,
        input.preference_scope,
        &[],
        &BTreeMap::new(),
    );
    preference_report.ignored_reason = Some(input.detail);
    let mut diagnostics = diagnostics;
    update_legibility_rejection_totals(&mut diagnostics);
    diagnostics.human_summary = candidate_generation_human_summary(&diagnostics);
    let reliability_report = candidate_reliability_report(
        input.document,
        input.profile,
        input.context,
        &input.request.variation_intent.clone().normalized(),
        &diagnostics,
        0,
        input.forced_reason,
    );
    FoundryCandidateOutput {
        candidates: Vec::new(),
        diagnostics,
        reliability_report,
        scoring_report: AssetScoringReport {
            rejected_candidates: Vec::new(),
            scored_candidates: Vec::new(),
            unique_candidates: Vec::new(),
            duplicate_groups: Vec::new(),
            representatives: Vec::new(),
        },
        preference_report,
    }
}

fn unsupported_variation_reason(
    document: &FoundryAssetDocument,
    intent: &VariationIntent,
) -> Option<String> {
    if intent.channels.iter().any(|channel| {
        matches!(
            channel,
            VariationChannel::Surface
                | VariationChannel::Wear
                | VariationChannel::Rig
                | VariationChannel::Motion
                | VariationChannel::Gameplay
                | VariationChannel::Custom { .. }
        )
    }) {
        return Some(SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON.to_owned());
    }
    if let VariationScope::SemanticPartGroup { group_id, .. } = &intent.scope
        && !known_part_groups(document)
            .iter()
            .any(|group| group.group_id == *group_id)
    {
        return Some(
            "Focus Part is available when this asset exposes editable part groups.".to_owned(),
        );
    }
    if matches!(
        intent.scope,
        VariationScope::MaterialSlot { .. }
            | VariationScope::RigRegion { .. }
            | VariationScope::MotionSet { .. }
            | VariationScope::Custom { .. }
    ) {
        return Some("This variation scope is reserved for future authored payloads.".to_owned());
    }
    None
}

fn endpoint_samples_for_control(
    control: &CustomizerControl,
    domain: &FeasibleControlDomain,
    context: ControlEvaluationContext<'_>,
) -> Vec<ControlValue> {
    let current = default_control_value(control, context)
        .ok()
        .and_then(|value| canonicalize_control_value(control, context, value).ok());
    let mut values = Vec::new();
    for value in &domain.discrete_values {
        if current.as_ref() != Some(value) && domain.contains_available_value(value) {
            push_unique_value(&mut values, value.clone());
        }
    }
    match control.kind {
        ControlKind::ContinuousAxis { .. } => {
            for interval in &domain.continuous_intervals {
                for value in [interval.minimum, interval.maximum] {
                    let value = ControlValue::Scalar(value);
                    if current.as_ref() != Some(&value) && domain.contains_available_value(&value) {
                        push_unique_value(&mut values, value);
                    }
                }
            }
        }
        ControlKind::IntegerStepper { .. } => {
            for interval in &domain.continuous_intervals {
                for value in [
                    interval.minimum.ceil() as i64,
                    interval.maximum.floor() as i64,
                ] {
                    let value = ControlValue::Integer(value);
                    if current.as_ref() != Some(&value) && domain.contains_available_value(&value) {
                        push_unique_value(&mut values, value);
                    }
                }
            }
        }
        ControlKind::Toggle { .. }
        | ControlKind::ChoiceGallery { .. }
        | ControlKind::ProviderGallery { .. } => {}
    }
    values
}

fn push_unique_value(values: &mut Vec<ControlValue>, value: ControlValue) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn endpoint_candidate_proposal(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> CandidateEditProposal {
    let class = classify_control(control, &classify_control_kind(control, &BTreeMap::new()));
    let kind = match class {
        ControlClass::Detail => FoundryCandidateChangeKind::Detail,
        ControlClass::Structure => match control.kind {
            ControlKind::IntegerStepper { .. } => FoundryCandidateChangeKind::Repetition,
            ControlKind::ProviderGallery { .. } => FoundryCandidateChangeKind::Provider,
            ControlKind::Toggle { .. } => FoundryCandidateChangeKind::RolePresence,
            ControlKind::ChoiceGallery { .. } => FoundryCandidateChangeKind::Choice,
            ControlKind::ContinuousAxis { .. } => FoundryCandidateChangeKind::Numeric,
        },
        ControlClass::Silhouette => FoundryCandidateChangeKind::Numeric,
    };
    let before = default_control_value(control, context)
        .map(|value| value_label(control, &value))
        .unwrap_or_else(|_| "default".to_owned());
    let change = FoundryCandidateControlChange {
        kind,
        control_id: control.id.clone(),
        control_label: control.label.clone(),
        before,
        after: "endpoint".to_owned(),
        message: format!("{} endpoint visibility checked.", control.label),
        details: Vec::new(),
        topology_changing: control.topology_behavior == ControlTopologyBehavior::TopologyChanging,
    };
    CandidateEditProposal {
        edit: FoundryEdit {
            label: format!("{} Endpoint", control.label),
            commands: Vec::new(),
        },
        changed_controls: vec![control.id.clone()],
        changes: vec![change],
    }
}

fn endpoint_visible_delta_report(
    changed_part_groups: &[SemanticPartGroupChange],
    proposal: &CandidateEditProposal,
    evidence: &CandidateLegibilityEvidence<'_>,
) -> CandidateVisibleDeltaReport {
    let measured = measured_delta_from_evidence(evidence);
    let shape_delta = measured
        .bbox_delta
        .max(measured.silhouette_delta)
        .max(measured.average_delta);
    let structure_delta = measured.structure_delta.max(
        if proposal.changes.iter().any(|change| {
            matches!(
                change.kind,
                FoundryCandidateChangeKind::Provider
                    | FoundryCandidateChangeKind::RolePresence
                    | FoundryCandidateChangeKind::Repetition
            )
        }) {
            0.08
        } else {
            0.0
        },
    );
    let detail_delta = if proposal
        .changes
        .iter()
        .any(|change| change.kind == FoundryCandidateChangeKind::Detail)
    {
        measured.detail_delta.max(LEGIBILITY_SUBTLE_DETAIL_DELTA)
    } else {
        measured.detail_delta
    };
    let semantic_endpoint_delta = if changed_part_groups.is_empty() {
        0.0
    } else {
        LEGIBILITY_ENDPOINT_CLEAR_DELTA
    };
    let selected_part_delta = if changed_part_groups.is_empty() {
        0.0
    } else {
        shape_delta
            .max(structure_delta)
            .max(detail_delta)
            .max(semantic_endpoint_delta)
    };
    let legibility_class = endpoint_legibility_class(
        shape_delta
            .max(structure_delta)
            .max(detail_delta)
            .max(semantic_endpoint_delta)
            .max(measured.average_delta),
        measured.max_delta,
        proposal,
    );
    let mut reasons = Vec::new();
    if !matches!(
        legibility_class,
        CandidateLegibilityClass::Strong
            | CandidateLegibilityClass::Clear
            | CandidateLegibilityClass::SubtleButExplainable
    ) {
        reasons.push("Control endpoint does not visibly change the preview.".to_owned());
    }
    CandidateVisibleDeltaReport::new(
        shape_delta,
        measured.silhouette_delta,
        measured.average_delta,
        structure_delta,
        0.0,
        0.0,
        selected_part_delta,
        legibility_class,
        reasons,
        false,
    )
}

fn endpoint_legibility_class(
    average_delta: f32,
    max_delta: f32,
    proposal: &CandidateEditProposal,
) -> CandidateLegibilityClass {
    if average_delta >= LEGIBILITY_STRONG_AVERAGE_DELTA || max_delta >= LEGIBILITY_CLEAR_MAX_DELTA {
        CandidateLegibilityClass::Strong
    } else if average_delta >= LEGIBILITY_ENDPOINT_CLEAR_DELTA
        || max_delta >= LEGIBILITY_CLEAR_AVERAGE_DELTA
    {
        CandidateLegibilityClass::Clear
    } else if proposal
        .changes
        .iter()
        .any(|change| change.kind == FoundryCandidateChangeKind::Detail)
        && average_delta >= LEGIBILITY_SUBTLE_DETAIL_DELTA
    {
        CandidateLegibilityClass::SubtleButExplainable
    } else if average_delta < LEGIBILITY_DUPLICATE_AVERAGE_DELTA
        && max_delta < LEGIBILITY_DUPLICATE_MAX_DELTA
    {
        CandidateLegibilityClass::DuplicateLooking
    } else {
        CandidateLegibilityClass::TooSubtle
    }
}

fn endpoint_warning(
    control: &CustomizerControl,
    class: CandidateLegibilityClass,
) -> Option<String> {
    let clear_enough = matches!(
        class,
        CandidateLegibilityClass::Strong | CandidateLegibilityClass::Clear
    ) || (control.id == "edge_softness"
        && class == CandidateLegibilityClass::SubtleButExplainable);
    (!clear_enough).then(|| {
        format!(
            "{} endpoint is {} and should not drive candidate generation.",
            control.label,
            class.display_label()
        )
    })
}

fn unsupported_endpoint_row(
    control: &CustomizerControl,
    endpoint_sample_count: usize,
    reason: &str,
) -> FoundryControlEndpointVisibilityRow {
    let visible_delta = CandidateVisibleDeltaReport::new(
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        CandidateLegibilityClass::Unsupported,
        vec![reason.to_owned()],
        false,
    );
    FoundryControlEndpointVisibilityRow {
        control_id: control.id.clone(),
        control_label: control.label.clone(),
        legibility_class: CandidateLegibilityClass::Unsupported,
        visible_delta,
        perceptual_report: PerceptualCandidateReport::new(
            format!("endpoint-{}", control.id),
            Vec::new(),
            0.0,
            0.0,
            0.0,
            0.0,
            Vec::new(),
            vec![control.id.clone()],
            CandidateLegibilityClass::Unsupported,
            Some(reason.to_owned()),
            format!("{} endpoint unsupported: {reason}", control.label),
        ),
        endpoint_sample_count,
        warning: Some(format!(
            "{} endpoint is Unsupported and should not drive candidate generation.",
            control.label
        )),
    }
}

fn variation_allows_control(
    document: &FoundryAssetDocument,
    intent: &VariationIntent,
    control: &CustomizerControl,
    class: ControlClass,
) -> bool {
    if intent.channels.iter().any(|channel| {
        matches!(
            channel,
            VariationChannel::Surface
                | VariationChannel::Wear
                | VariationChannel::Rig
                | VariationChannel::Motion
                | VariationChannel::Gameplay
                | VariationChannel::Custom { .. }
        )
    }) {
        return false;
    }
    let channel_allows = intent.channels.iter().any(|channel| match channel {
        VariationChannel::CompleteLook => true,
        VariationChannel::Shape => class != ControlClass::Detail,
        VariationChannel::Detail => class == ControlClass::Detail,
        VariationChannel::Surface
        | VariationChannel::Wear
        | VariationChannel::Rig
        | VariationChannel::Motion
        | VariationChannel::Gameplay
        | VariationChannel::Custom { .. } => false,
    });
    if !channel_allows {
        return false;
    }
    if let VariationScope::SemanticPartGroup { group_id, .. } = &intent.scope {
        return control_matches_part_group(document, control, group_id);
    }
    true
}

struct CandidateLegibilityEvidence<'a> {
    candidate_id: String,
    parent: &'a AssetCandidateInput,
    candidate: &'a AssetCandidateInput,
}

fn variation_metadata_for_proposal(
    document: &FoundryAssetDocument,
    intent: &VariationIntent,
    proposal: &CandidateEditProposal,
    evidence: Option<&CandidateLegibilityEvidence<'_>>,
) -> CandidateVariationMetadata {
    let mut changed_part_groups = changed_part_groups_for_proposal(document, proposal);
    if let VariationScope::SemanticPartGroup {
        group_id,
        display_name,
    } = &intent.scope
    {
        changed_part_groups.retain(|group| group.group_id == *group_id);
        if changed_part_groups.is_empty() && !proposal.changed_controls.is_empty() {
            changed_part_groups.push(SemanticPartGroupChange {
                group_id: group_id.clone(),
                display_name: display_name.clone(),
                change_label: "Shape adjusted".to_owned(),
                visible: true,
            });
        }
    }
    let changed_roles = changed_roles_from_commands(&proposal.edit.commands);
    let changed_material_slots = changed_material_slots_for_intent(intent);
    let visible_delta =
        candidate_visible_delta_report(intent, proposal, &changed_part_groups, evidence);
    let perceptual_report = evidence.map(|evidence| {
        perceptual_candidate_report(
            intent,
            proposal,
            &changed_part_groups,
            &visible_delta,
            evidence,
        )
    });
    let explanation_quality = CandidateExplanationQuality {
        explanation_matches_changed_controls: !proposal.changed_controls.is_empty()
            && proposal
                .changes
                .iter()
                .all(|change| proposal.changed_controls.contains(&change.control_id)),
        explanation_matches_visible_delta: visible_delta.legibility_class.selectable(),
        human_summary_available: !intent.human_summary.trim().is_empty()
            && !proposal.edit.label.trim().is_empty(),
    };
    CandidateVariationMetadata {
        intent: intent.clone(),
        changed_part_groups,
        changed_material_slots,
        changed_controls: proposal.changed_controls.clone(),
        changed_roles,
        respects_locks: true,
        visible_delta,
        perceptual_report,
        explanation_quality,
    }
}

fn variation_rejection_reason(
    metadata: &CandidateVariationMetadata,
) -> Option<FoundryCandidateRejectionReason> {
    match metadata.visible_delta.legibility_class {
        CandidateLegibilityClass::Strong
        | CandidateLegibilityClass::Clear
        | CandidateLegibilityClass::SubtleButExplainable
        | CandidateLegibilityClass::DetailOnly => {
            if !metadata
                .explanation_quality
                .explanation_matches_changed_controls
                || !metadata
                    .explanation_quality
                    .explanation_matches_visible_delta
            {
                Some(FoundryCandidateRejectionReason::ExplanationMismatch)
            } else {
                None
            }
        }
        CandidateLegibilityClass::TooSubtle => Some(FoundryCandidateRejectionReason::TooSubtle),
        CandidateLegibilityClass::DuplicateLooking => {
            Some(FoundryCandidateRejectionReason::DuplicateLooking)
        }
        CandidateLegibilityClass::Unsupported => {
            Some(FoundryCandidateRejectionReason::UnsupportedChannel)
        }
    }
}

fn metadata_is_wrong_scope_rejection(metadata: &CandidateVariationMetadata) -> bool {
    !metadata.visible_delta.legibility_class.selectable()
        && metadata
            .visible_delta
            .blocking_reasons
            .iter()
            .any(|reason| {
                reason.contains("Focus Part") || reason.contains("outside the requested scope")
            })
}

fn candidate_visible_delta_report(
    intent: &VariationIntent,
    proposal: &CandidateEditProposal,
    changed_part_groups: &[SemanticPartGroupChange],
    evidence: Option<&CandidateLegibilityEvidence<'_>>,
) -> CandidateVisibleDeltaReport {
    let has_detail = proposal
        .changes
        .iter()
        .any(|change| change.kind == FoundryCandidateChangeKind::Detail);
    let has_structure = proposal.changes.iter().any(|change| {
        matches!(
            change.kind,
            FoundryCandidateChangeKind::Provider
                | FoundryCandidateChangeKind::RolePresence
                | FoundryCandidateChangeKind::Repetition
        )
    });
    let has_shape = proposal.changes.iter().any(|change| {
        change.kind != FoundryCandidateChangeKind::Detail || change.topology_changing
    });
    let measured = evidence.map(measured_delta_from_evidence);
    let silhouette_delta = measured
        .as_ref()
        .map_or(0.0, |delta| delta.silhouette_delta);
    let screen_space_delta = measured.as_ref().map_or(0.0, |delta| delta.average_delta);
    let bbox_delta = measured.as_ref().map_or(0.0, |delta| delta.bbox_delta);
    let structure_signal = measured
        .as_ref()
        .map_or(0.0, |delta| delta.structure_delta)
        .max(if has_structure { 0.08 } else { 0.0 });
    let detail_delta = if has_detail {
        measured
            .as_ref()
            .map_or(LEGIBILITY_SUBTLE_DETAIL_DELTA, |delta| {
                delta.detail_delta.max(LEGIBILITY_SUBTLE_DETAIL_DELTA)
            })
    } else {
        0.0
    };
    let shape_delta = if has_shape {
        bbox_delta.max(silhouette_delta).max(screen_space_delta)
    } else {
        0.0
    };
    let selected_part_delta = selected_part_delta_for_intent(
        intent,
        changed_part_groups,
        shape_delta.max(structure_signal).max(detail_delta),
    );
    let surface_delta = 0.0;
    let (class, reasons) = classify_candidate_delta(
        intent,
        shape_delta,
        silhouette_delta,
        screen_space_delta,
        structure_signal,
        surface_delta,
        detail_delta,
        selected_part_delta,
        changed_part_groups,
    );
    CandidateVisibleDeltaReport::new(
        shape_delta,
        silhouette_delta,
        screen_space_delta,
        structure_signal,
        surface_delta,
        0.0,
        selected_part_delta,
        class,
        reasons,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn classify_candidate_delta(
    intent: &VariationIntent,
    shape_delta: f32,
    silhouette_delta: f32,
    screen_space_delta: f32,
    structure_delta: f32,
    surface_delta: f32,
    detail_delta: f32,
    selected_part_delta: f32,
    changed_part_groups: &[SemanticPartGroupChange],
) -> (CandidateLegibilityClass, Vec<String>) {
    let mut reasons = Vec::new();
    let wants_shape = intent.includes_channel(&VariationChannel::Shape);
    let wants_complete = intent.includes_channel(&VariationChannel::CompleteLook);
    let wants_detail = intent.includes_channel(&VariationChannel::Detail);
    let wants_surface = intent.includes_channel(&VariationChannel::Surface);

    if wants_surface {
        reasons.push(SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON.to_owned());
        return (CandidateLegibilityClass::Unsupported, reasons);
    }
    let detail_only_evidence = detail_delta > 0.0
        && shape_delta <= 0.0
        && silhouette_delta <= 0.0
        && structure_delta <= 0.0;
    if !wants_detail && detail_only_evidence {
        reasons.push("Detail-only changes are shown only in Detail mode.".to_owned());
        return (CandidateLegibilityClass::TooSubtle, reasons);
    }
    if let VariationScope::SemanticPartGroup { group_id, .. } = &intent.scope {
        let selected_changed = changed_part_groups
            .iter()
            .any(|group| group.group_id == *group_id && group.visible);
        let unrelated_changed = changed_part_groups
            .iter()
            .any(|group| group.group_id != *group_id && group.visible);
        if !selected_changed || selected_part_delta < LEGIBILITY_FOCUS_SELECTED_DELTA {
            reasons
                .push("Focus Part needs a visible change on the selected part group.".to_owned());
            return (CandidateLegibilityClass::TooSubtle, reasons);
        }
        if unrelated_changed {
            reasons.push(
                "Focus Part rejected because another part group changed outside the requested scope."
                    .to_owned(),
            );
            return (CandidateLegibilityClass::TooSubtle, reasons);
        }
    }

    let geometry_evidence = shape_delta.max(silhouette_delta).max(structure_delta);
    let focus_evidence = if intent.scope.is_focus_part() {
        selected_part_delta
    } else {
        0.0
    };
    let perceptual_evidence = geometry_evidence
        .max(screen_space_delta)
        .max(focus_evidence);
    let clear_threshold = if intent.scope.is_focus_part() {
        LEGIBILITY_FOCUS_SELECTED_DELTA
    } else {
        LEGIBILITY_CLEAR_AVERAGE_DELTA
    };
    if wants_detail {
        if detail_delta >= LEGIBILITY_SUBTLE_DETAIL_DELTA
            || perceptual_evidence >= LEGIBILITY_SUBTLE_DETAIL_DELTA
        {
            return (CandidateLegibilityClass::DetailOnly, reasons);
        }
        reasons
            .push("Detail directions need visible detail evidence or a detail label.".to_owned());
        return (CandidateLegibilityClass::TooSubtle, reasons);
    }
    if perceptual_evidence < LEGIBILITY_DUPLICATE_AVERAGE_DELTA
        && silhouette_delta < LEGIBILITY_DUPLICATE_MAX_DELTA
    {
        reasons.push("Candidate looks identical to the parent at preview size.".to_owned());
        return (CandidateLegibilityClass::DuplicateLooking, reasons);
    }
    if wants_shape && perceptual_evidence < clear_threshold {
        reasons.push(
            "Shape directions need visible geometry, structure, or silhouette change.".to_owned(),
        );
        return (CandidateLegibilityClass::TooSubtle, reasons);
    }
    if wants_complete
        && perceptual_evidence < LEGIBILITY_CLEAR_AVERAGE_DELTA
        && surface_delta <= 0.0
    {
        reasons.push("Complete Looks need a visible shape or surface difference.".to_owned());
        return (CandidateLegibilityClass::TooSubtle, reasons);
    }
    if perceptual_evidence >= LEGIBILITY_STRONG_AVERAGE_DELTA
        || silhouette_delta >= LEGIBILITY_CLEAR_MAX_DELTA
    {
        (CandidateLegibilityClass::Strong, reasons)
    } else if perceptual_evidence >= clear_threshold
        || silhouette_delta >= LEGIBILITY_CLEAR_MAX_DELTA
    {
        (CandidateLegibilityClass::Clear, reasons)
    } else {
        reasons.push("Visible change is too small for a normal direction card.".to_owned());
        (CandidateLegibilityClass::TooSubtle, reasons)
    }
}

fn changed_part_groups_for_proposal(
    document: &FoundryAssetDocument,
    proposal: &CandidateEditProposal,
) -> Vec<SemanticPartGroupChange> {
    let mut groups = Vec::new();
    for change in &proposal.changes {
        for group in known_part_groups(document) {
            if control_change_matches_group(change, &group)
                && !groups
                    .iter()
                    .any(|existing: &SemanticPartGroupChange| existing.group_id == group.group_id)
            {
                groups.push(SemanticPartGroupChange {
                    group_id: group.group_id,
                    display_name: group.display_name,
                    change_label: change_label_for_group(change.kind),
                    visible: true,
                });
            }
        }
    }
    groups
}

fn changed_material_slots_for_intent(intent: &VariationIntent) -> Vec<MaterialSlotChange> {
    if intent
        .channels
        .iter()
        .any(|channel| matches!(channel, VariationChannel::Surface | VariationChannel::Wear))
    {
        vec![MaterialSlotChange {
            slot_id: "surface-pack".to_owned(),
            display_name: "Surface Pack".to_owned(),
            change_label: "Surface payload unavailable".to_owned(),
            surface_payload_ready: false,
        }]
    } else {
        Vec::new()
    }
}

fn changed_roles_from_commands(commands: &[FoundryCommand]) -> Vec<String> {
    let mut roles = Vec::new();
    for command in commands {
        match command {
            FoundryCommand::SelectProvider { role, .. }
            | FoundryCommand::SetRolePresence { role, .. }
                if !roles.contains(role) =>
            {
                roles.push(role.clone());
            }
            _ => {}
        }
    }
    roles
}

#[derive(Debug, Clone)]
struct KnownPartGroup {
    group_id: String,
    display_name: String,
    bound_control_ids: BTreeSet<String>,
    bound_provider_roles: BTreeSet<String>,
}

fn known_part_groups(document: &FoundryAssetDocument) -> Vec<KnownPartGroup> {
    built_in_part_group_descriptors(document)
        .into_iter()
        .map(part_group_from_descriptor)
        .collect()
}

fn built_in_part_group_descriptors(
    document: &FoundryAssetDocument,
) -> Vec<FoundryPartGroupDescriptor> {
    let profile_hint = format!(
        "{} {}",
        document.family_content_ref.stable_id, document.customizer_profile_ref.stable_id
    );
    built_in_part_group_descriptors_for_profile(&profile_hint)
}

fn part_group_from_descriptor(descriptor: FoundryPartGroupDescriptor) -> KnownPartGroup {
    KnownPartGroup {
        group_id: descriptor.group_id,
        display_name: descriptor.display_name,
        bound_control_ids: descriptor.bound_control_ids.into_iter().collect(),
        bound_provider_roles: descriptor.bound_provider_roles.into_iter().collect(),
    }
}

fn control_matches_part_group(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
    group_id: &str,
) -> bool {
    known_part_groups(document)
        .into_iter()
        .find(|group| group.group_id == group_id)
        .is_some_and(|group| {
            if group.bound_control_ids.contains(&control.id)
                || control
                    .bindings
                    .iter()
                    .any(|binding| group.bound_control_ids.contains(&binding.slot))
            {
                return true;
            }
            matches!(
                &control.kind,
                ControlKind::ProviderGallery { role, .. }
                    if group.bound_provider_roles.contains(role)
            )
        })
}

fn control_change_matches_group(
    change: &FoundryCandidateControlChange,
    group: &KnownPartGroup,
) -> bool {
    if group.bound_control_ids.contains(&change.control_id) {
        return true;
    }
    let text = format!("{} {}", change.control_id, change.control_label).to_ascii_lowercase();
    if group
        .bound_provider_roles
        .iter()
        .any(|role| text.contains(&role.to_ascii_lowercase()))
    {
        return true;
    }
    match group.group_id.as_str() {
        "body" => contains_any(&text, &["body", "proportion", "height", "width", "size"]),
        "panels" => contains_any(&text, &["panel", "depth"]),
        "vents" => text.contains("vent"),
        "handles" => text.contains("handle"),
        "edge-trim" => contains_any(&text, &["edge", "trim", "bevel"]),
        "fasteners" => contains_any(&text, &["bolt", "fastener", "connector"]),
        "deck" => contains_any(&text, &["deck", "span", "length"]),
        "supports" => contains_any(&text, &["support", "pier"]),
        "bracing" => contains_any(&text, &["brace", "bracing"]),
        "railing" => text.contains("rail"),
        "ramps" => text.contains("ramp"),
        "base" => text.contains("base"),
        "stem" => contains_any(&text, &["stem", "height"]),
        "joints" => text.contains("joint"),
        "shade" => text.contains("shade"),
        "trim" => contains_any(&text, &["trim", "edge"]),
        _ => false,
    }
}

fn change_label_for_group(kind: FoundryCandidateChangeKind) -> String {
    match kind {
        FoundryCandidateChangeKind::Numeric => "Proportion adjusted",
        FoundryCandidateChangeKind::Repetition => "Count adjusted",
        FoundryCandidateChangeKind::RolePresence => "Part presence adjusted",
        FoundryCandidateChangeKind::Choice => "Option changed",
        FoundryCandidateChangeKind::Provider => "Structure changed",
        FoundryCandidateChangeKind::Detail => "Detail adjusted",
    }
    .to_owned()
}

#[derive(Debug, Clone, PartialEq)]
struct MeasuredCandidateDelta {
    render_delta_by_camera: Vec<f32>,
    max_delta: f32,
    average_delta: f32,
    silhouette_delta: f32,
    bbox_delta: f32,
    structure_delta: f32,
    detail_delta: f32,
}

fn measured_delta_from_evidence(
    evidence: &CandidateLegibilityEvidence<'_>,
) -> MeasuredCandidateDelta {
    let mut render_delta_by_camera = Vec::with_capacity(FIXED_CAMERA_COUNT);
    for camera in 0..FIXED_CAMERA_COUNT {
        let mask_delta = mask_delta_for_camera(
            evidence.parent.silhouette_masks.get(camera),
            evidence.candidate.silhouette_masks.get(camera),
        );
        let occupancy_delta = (evidence.parent.silhouette_occupancy[camera]
            - evidence.candidate.silhouette_occupancy[camera])
            .abs();
        let perimeter_delta = vector_index_delta(
            &evidence.parent.silhouette_perimeter,
            &evidence.candidate.silhouette_perimeter,
            camera,
        );
        let depth_delta = depth_histogram_delta_for_camera(
            &evidence.parent.depth_histogram,
            &evidence.candidate.depth_histogram,
            camera,
        );
        render_delta_by_camera.push(
            (mask_delta * 0.58
                + occupancy_delta * 0.18
                + perimeter_delta * 0.12
                + depth_delta * 0.12)
                .clamp(0.0, 1.0),
        );
    }
    let average_delta = average(&render_delta_by_camera);
    let max_delta = render_delta_by_camera.iter().copied().fold(0.0, f32::max);
    let silhouette_delta = render_delta_by_camera
        .iter()
        .copied()
        .zip(
            evidence
                .parent
                .silhouette_occupancy
                .iter()
                .zip(evidence.candidate.silhouette_occupancy.iter()),
        )
        .map(|(camera_delta, (left, right))| camera_delta.max((left - right).abs()))
        .fold(0.0, f32::max);
    let bbox_delta = bounds_delta(
        evidence.parent.world_bounds,
        evidence.candidate.world_bounds,
    );
    let structure_delta = count_delta(
        evidence.parent.detached_visual_components + evidence.parent.part_volumes.len(),
        evidence.candidate.detached_visual_components + evidence.candidate.part_volumes.len(),
    )
    .max(count_delta(
        evidence.parent.repeated_element_count,
        evidence.candidate.repeated_element_count,
    ))
    .max(count_delta(
        evidence.parent.region_count,
        evidence.candidate.region_count,
    ));
    let detail_delta = count_delta(
        evidence.parent.detail_count,
        evidence.candidate.detail_count,
    )
    .max(bevel_delta(
        &evidence.parent.bevel_radii,
        &evidence.candidate.bevel_radii,
        evidence
            .parent
            .world_bounds
            .extent()
            .max_element()
            .max(evidence.candidate.world_bounds.extent().max_element())
            .max(EPSILON),
    ));
    MeasuredCandidateDelta {
        render_delta_by_camera,
        max_delta,
        average_delta,
        silhouette_delta,
        bbox_delta,
        structure_delta,
        detail_delta,
    }
}

fn perceptual_candidate_report(
    intent: &VariationIntent,
    proposal: &CandidateEditProposal,
    changed_part_groups: &[SemanticPartGroupChange],
    visible_delta: &CandidateVisibleDeltaReport,
    evidence: &CandidateLegibilityEvidence<'_>,
) -> PerceptualCandidateReport {
    let measured = measured_delta_from_evidence(evidence);
    let reject_reason = visible_delta.blocking_reasons.first().cloned().or_else(|| {
        (!visible_delta.legibility_class.selectable())
            .then(|| visible_delta.legibility_class.display_label().to_owned())
    });
    let human_summary = if visible_delta.legibility_class.selectable() {
        format!(
            "{} reads as {} with {:.0}% average preview delta.",
            intent.human_label,
            visible_delta.legibility_class.display_label(),
            measured.average_delta * 100.0
        )
    } else {
        format!(
            "{} rejected: {}",
            proposal.edit.label,
            reject_reason
                .as_deref()
                .unwrap_or("preview difference is not visible enough")
        )
    };
    PerceptualCandidateReport::new(
        evidence.candidate_id.clone(),
        measured.render_delta_by_camera,
        measured.max_delta,
        measured.average_delta,
        measured.silhouette_delta,
        measured.bbox_delta,
        changed_part_groups
            .iter()
            .map(|group| group.group_id.clone())
            .collect(),
        proposal.changed_controls.clone(),
        visible_delta.legibility_class,
        reject_reason,
        human_summary,
    )
}

fn selected_part_delta_for_intent(
    intent: &VariationIntent,
    changed_part_groups: &[SemanticPartGroupChange],
    visible_delta: f32,
) -> f32 {
    let VariationScope::SemanticPartGroup { group_id, .. } = &intent.scope else {
        return 0.0;
    };
    if changed_part_groups
        .iter()
        .any(|group| group.group_id == *group_id && group.visible)
    {
        visible_delta.max(LEGIBILITY_FOCUS_SELECTED_DELTA)
    } else {
        0.0
    }
}

fn accepted_duplicate_member_count(
    scoring_report: &AssetScoringReport,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> usize {
    let mut duplicate_members = BTreeSet::new();
    for group in &scoring_report.duplicate_groups {
        for member_id in &group.member_ids {
            if accepted_plans.contains_key(member_id) {
                duplicate_members.insert(member_id.clone());
            }
        }
    }
    duplicate_members.len()
}

fn mask_delta_for_camera(left: Option<&Vec<u64>>, right: Option<&Vec<u64>>) -> f32 {
    let left = left.map_or(&[][..], Vec::as_slice);
    let right = right.map_or(&[][..], Vec::as_slice);
    let word_count = left.len().max(right.len());
    if word_count == 0 {
        return 0.0;
    }
    let differing_bits = (0..word_count)
        .map(|index| {
            let left_word = left.get(index).copied().unwrap_or(0);
            let right_word = right.get(index).copied().unwrap_or(0);
            (left_word ^ right_word).count_ones() as usize
        })
        .sum::<usize>();
    (differing_bits as f32 / (word_count as f32 * 64.0)).clamp(0.0, 1.0)
}

fn vector_index_delta(left: &[f32], right: &[f32], index: usize) -> f32 {
    (left.get(index).copied().unwrap_or(0.0) - right.get(index).copied().unwrap_or(0.0))
        .abs()
        .clamp(0.0, 1.0)
}

fn depth_histogram_delta_for_camera(left: &[f32], right: &[f32], camera: usize) -> f32 {
    let start = camera * DEPTH_HISTOGRAM_BINS;
    let end = start + DEPTH_HISTOGRAM_BINS;
    vector_delta(
        left.get(start..end).unwrap_or(&[]),
        right.get(start..end).unwrap_or(&[]),
    )
}

fn vector_delta(left: &[f32], right: &[f32]) -> f32 {
    let length = left.len().max(right.len());
    if length == 0 {
        return 0.0;
    }
    (0..length)
        .map(|index| {
            (left.get(index).copied().unwrap_or(0.0) - right.get(index).copied().unwrap_or(0.0))
                .abs()
        })
        .sum::<f32>()
        / length as f32
}

fn bounds_delta(left: Aabb, right: Aabb) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let left_extent = left.extent();
    let right_extent = right.extent();
    let left_scale = left_extent.max_element().max(EPSILON);
    let right_scale = right_extent.max_element().max(EPSILON);
    let center_scale = ((left_scale + right_scale) * 0.5).max(EPSILON);
    let center_delta = ((left.center() - right.center()).length() / center_scale).min(1.0);
    let shape_delta = ((left_extent / left_scale - right_extent / right_scale).length()
        / 3.0_f32.sqrt())
    .min(1.0);
    let scale_delta = ((left_scale / right_scale).ln().abs() / 4.0).min(1.0);
    ((center_delta + shape_delta + scale_delta) / 3.0).clamp(0.0, 1.0)
}

fn count_delta(left: usize, right: usize) -> f32 {
    if left == right {
        return 0.0;
    }
    left.abs_diff(right) as f32 / left.max(right).max(1) as f32
}

fn bevel_delta(left: &[f32], right: &[f32], scale: f32) -> f32 {
    let length = left.len().max(right.len());
    if length == 0 {
        return 0.0;
    }
    (0..length)
        .map(|index| {
            let left = left.get(index).copied().unwrap_or(0.0) / scale;
            let right = right.get(index).copied().unwrap_or(0.0) / scale;
            (left - right).abs()
        })
        .sum::<f32>()
        .min(1.0)
}

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn request_preference_profile<'a>(
    profile: Option<&'a FoundryPreferenceProfile>,
    scope: &FoundryPreferenceScope,
) -> Option<&'a FoundryPreferenceProfile> {
    let profile = profile?;
    if profile.schema_version != FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION
        || !profile.local_only
        || !profile.matches_scope(scope)
        || profile.is_empty()
    {
        return None;
    }
    Some(profile)
}

fn selected_candidate_ids(
    scoring_report: &AssetScoringReport,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
    preference_profile: Option<&FoundryPreferenceProfile>,
    policy: &AssetSelectionPolicy,
) -> Vec<String> {
    let accepted_candidates = accepted_scored_candidates(scoring_report, accepted_plans, policy);
    let Some(profile) = preference_profile else {
        return accepted_candidates
            .into_iter()
            .map(|candidate| candidate.id.clone())
            .collect();
    };

    let target_count = accepted_candidates.len();
    let mut remaining = accepted_candidates;
    remaining.sort_by(|left, right| left.id.cmp(&right.id));
    let mut selected = Vec::<AssetScoredCandidate>::new();
    while selected.len() < target_count && !remaining.is_empty() {
        let Some(index) =
            best_preference_candidate_index(&remaining, &selected, profile, policy, accepted_plans)
        else {
            break;
        };
        selected.push(remaining.remove(index));
    }

    if selected.is_empty() {
        accepted_scored_candidates(scoring_report, accepted_plans, policy)
            .into_iter()
            .map(|candidate| candidate.id.clone())
            .collect()
    } else {
        selected.into_iter().map(|candidate| candidate.id).collect()
    }
}

fn accepted_scored_candidates(
    scoring_report: &AssetScoringReport,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
    policy: &AssetSelectionPolicy,
) -> Vec<AssetScoredCandidate> {
    let mut accepted = scoring_report
        .representatives
        .iter()
        .filter(|candidate| accepted_plans.contains_key(&candidate.id))
        .cloned()
        .collect::<Vec<_>>();

    let target_count = policy.representative_count.min(accepted_plans.len());
    let mut seen = accepted
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<BTreeSet<_>>();
    let mut covered_duplicate_members = BTreeSet::new();
    for candidate in &accepted {
        mark_duplicate_group_members(
            scoring_report,
            &candidate.id,
            &mut covered_duplicate_members,
        );
    }
    if accepted.len() >= target_count {
        return accepted;
    }

    for candidate in &scoring_report.scored_candidates {
        if !accepted_plans.contains_key(&candidate.id)
            || seen.contains(&candidate.id)
            || covered_duplicate_members.contains(&candidate.id)
        {
            continue;
        }
        seen.insert(candidate.id.clone());
        mark_duplicate_group_members(
            scoring_report,
            &candidate.id,
            &mut covered_duplicate_members,
        );
        accepted.push(candidate.clone());
        if accepted.len() >= target_count {
            break;
        }
    }
    accepted
}

fn mark_duplicate_group_members(
    scoring_report: &AssetScoringReport,
    candidate_id: &str,
    covered_duplicate_members: &mut BTreeSet<String>,
) {
    for group in &scoring_report.duplicate_groups {
        if group.kept_id == candidate_id || group.member_ids.iter().any(|id| id == candidate_id) {
            covered_duplicate_members.extend(group.member_ids.iter().cloned());
        }
    }
}

fn best_preference_candidate_index(
    candidates: &[AssetScoredCandidate],
    selected: &[AssetScoredCandidate],
    profile: &FoundryPreferenceProfile,
    policy: &AssetSelectionPolicy,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> Option<usize> {
    let novelty_floor = profile.bounded_novelty_floor();
    let has_novel_candidates = !selected.is_empty()
        && candidates.iter().any(|candidate| {
            minimum_distance_to_selected_candidate(candidate, selected, policy) >= novelty_floor
        });
    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            !has_novel_candidates
                || minimum_distance_to_selected_candidate(candidate, selected, policy)
                    >= novelty_floor
        })
        .max_by(|(_, left), (_, right)| {
            compare_preference_selection_candidate(
                left,
                right,
                selected,
                profile,
                policy,
                accepted_plans,
            )
        })
        .map(|(index, _)| index)
}

fn compare_preference_selection_candidate(
    left: &AssetScoredCandidate,
    right: &AssetScoredCandidate,
    selected: &[AssetScoredCandidate],
    profile: &FoundryPreferenceProfile,
    policy: &AssetSelectionPolicy,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> Ordering {
    let left_value = preference_selection_value(left, selected, profile, policy, accepted_plans);
    let right_value = preference_selection_value(right, selected, profile, policy, accepted_plans);
    compare_scalar(left_value, right_value)
        .then_with(|| {
            compare_scalar(
                preference_bonus(left, profile, accepted_plans),
                preference_bonus(right, profile, accepted_plans),
            )
        })
        .then_with(|| right.id.cmp(&left.id))
}

fn preference_selection_value(
    candidate: &AssetScoredCandidate,
    selected: &[AssetScoredCandidate],
    profile: &FoundryPreferenceProfile,
    policy: &AssetSelectionPolicy,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> f32 {
    let diversity = if selected.is_empty() {
        0.0
    } else {
        minimum_distance_to_selected_candidate(candidate, selected, policy)
    };
    diversity * policy.diversity_weight + preference_bonus(candidate, profile, accepted_plans)
        - candidate.weighted_quality_penalty * policy.quality_penalty_weight
}

fn preference_bonus(
    candidate: &AssetScoredCandidate,
    profile: &FoundryPreferenceProfile,
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> f32 {
    accepted_plans.get(&candidate.id).map_or(0.0, |plan| {
        profile.score_changed_controls(&plan.changed_controls)
            * profile.bounded_selection_strength()
    })
}

fn minimum_distance_to_selected_candidate(
    candidate: &AssetScoredCandidate,
    selected: &[AssetScoredCandidate],
    policy: &AssetSelectionPolicy,
) -> f32 {
    selected
        .iter()
        .map(|representative| {
            asset_descriptor_distance(
                &candidate.descriptor,
                &representative.descriptor,
                &policy.descriptor_weights,
            )
        })
        .fold(f32::INFINITY, f32::min)
}

fn compare_scalar(left: f32, right: f32) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn preference_report(
    requested_profile: Option<&FoundryPreferenceProfile>,
    applied_profile: Option<&FoundryPreferenceProfile>,
    scope: FoundryPreferenceScope,
    selected_ids: &[String],
    accepted_plans: &BTreeMap<String, FoundryCandidatePlan>,
) -> FoundryCandidatePreferenceReport {
    let requested = requested_profile.is_some();
    let scope_matched = requested_profile.is_some_and(|profile| profile.matches_scope(&scope));
    let ignored_reason = if requested && applied_profile.is_none() {
        Some(preference_ignored_reason(
            requested_profile.expect("requested profile"),
            &scope,
        ))
    } else {
        None
    };
    let selected_scores = selected_ids
        .iter()
        .filter_map(|candidate_id| {
            accepted_plans
                .get(candidate_id)
                .map(|plan| preference_score_row(applied_profile, plan))
        })
        .collect();
    FoundryCandidatePreferenceReport {
        requested,
        applied: applied_profile.is_some(),
        scope_matched,
        scope,
        ignored_reason,
        selected_scores,
    }
}

fn preference_ignored_reason(
    profile: &FoundryPreferenceProfile,
    scope: &FoundryPreferenceScope,
) -> String {
    if profile.schema_version != FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION {
        return "unsupported_preference_schema".to_owned();
    }
    if !profile.local_only {
        return "preference_profile_not_local".to_owned();
    }
    if !profile.matches_scope(scope) {
        return "preference_scope_mismatch".to_owned();
    }
    if profile.is_empty() {
        return "empty_preference_profile".to_owned();
    }
    "preference_profile_ignored".to_owned()
}

fn preference_score_row(
    profile: Option<&FoundryPreferenceProfile>,
    plan: &FoundryCandidatePlan,
) -> FoundryCandidatePreferenceScore {
    let score = profile.map_or(0.0, |profile| {
        profile.score_changed_controls(&plan.changed_controls)
    });
    let selection_bonus =
        profile.map_or(0.0, |profile| score * profile.bounded_selection_strength());
    FoundryCandidatePreferenceScore {
        candidate_id: plan.id.clone(),
        score,
        selection_bonus,
        changed_controls: plan.changed_controls.clone(),
    }
}

fn compile_error_rejection_reason(
    error: &FoundryCompilationError,
) -> FoundryCandidateRejectionReason {
    if matches!(error, FoundryCompilationError::FinalConformanceRejected(_)) {
        FoundryCandidateRejectionReason::ConformanceRejected
    } else {
        FoundryCandidateRejectionReason::CompileRejected
    }
}

fn strategy_control_ids(
    profile: &CustomizerProfile,
    strategy_id: &Option<String>,
) -> Result<Option<BTreeSet<String>>, FoundryCandidateError> {
    let Some(strategy_id) = strategy_id else {
        return Ok(None);
    };
    let Some(strategy) = profile
        .candidate_strategies
        .iter()
        .find(|strategy| strategy.id == *strategy_id)
    else {
        return Err(FoundryCandidateError::UnknownCandidateStrategy(
            strategy_id.clone(),
        ));
    };
    Ok(Some(strategy.control_ids.iter().cloned().collect()))
}

#[derive(Debug, Clone)]
struct CandidateEditProposal {
    edit: FoundryEdit,
    changed_controls: Vec<String>,
    changes: Vec<FoundryCandidateControlChange>,
}

#[derive(Debug, Clone)]
struct ControlOpportunity {
    control_id: String,
    label: String,
    current: ControlValue,
    domain: FeasibleControlDomain,
    kind: ControlOpportunityKind,
    class: ControlClass,
    topology_behavior: ControlTopologyBehavior,
}

impl ControlOpportunity {
    fn topology_changing(&self) -> bool {
        self.topology_behavior == ControlTopologyBehavior::TopologyChanging
    }

    fn is_provider_or_role_presence(&self) -> bool {
        matches!(
            self.kind,
            ControlOpportunityKind::Provider { .. } | ControlOpportunityKind::RolePresence { .. }
        )
    }
}

#[derive(Debug, Clone)]
enum ControlOpportunityKind {
    Scalar,
    Integer,
    Toggle,
    Choice,
    Provider { role: String },
    RolePresence { role: String },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ControlClass {
    Silhouette,
    Structure,
    Detail,
}

fn collect_control_opportunities(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    mode: FoundryCandidateMode,
    variation_intent: &VariationIntent,
    strategy_controls: Option<&BTreeSet<String>>,
    locked_targets_skipped: &mut usize,
) -> Vec<ControlOpportunity> {
    let slot_roles = context
        .family_parameter_slots
        .iter()
        .filter_map(|slot| {
            slot.target_role
                .as_ref()
                .map(|role| (slot.id.as_str(), role.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut controls = profile.controls.iter().collect::<Vec<_>>();
    controls.sort_by(|left, right| left.id.cmp(&right.id));

    let mut opportunities = Vec::new();
    for control in controls {
        if !control.visible || control.topology_behavior == ControlTopologyBehavior::RuntimeOnly {
            continue;
        }
        if strategy_controls.is_some_and(|ids| !ids.contains(&control.id)) {
            continue;
        }

        let kind = classify_control_kind(control, &slot_roles);
        if protected_by_lock(document, control, &kind) {
            *locked_targets_skipped += 1;
            continue;
        }
        let class = classify_control(control, &kind);
        if !mode_allows_control(
            mode,
            class,
            control.topology_behavior,
            &kind,
            variation_intent,
        ) {
            continue;
        }
        if !variation_allows_control(document, variation_intent, control, class) {
            continue;
        }

        let Ok(domain) = effective_control_domain(control, context) else {
            continue;
        };
        let raw_current = document
            .control_state
            .get(&control.id)
            .cloned()
            .unwrap_or_else(|| {
                default_control_value(control, context)
                    .unwrap_or_else(|_| first_domain_value(&domain))
            });
        let Ok(current) = canonicalize_control_value(control, context, raw_current) else {
            continue;
        };
        if !domain.contains_available_value(&current) {
            continue;
        }
        opportunities.push(ControlOpportunity {
            control_id: control.id.clone(),
            label: control.label.clone(),
            current,
            domain,
            kind,
            class,
            topology_behavior: control.topology_behavior,
        });
    }
    opportunities.sort_by(|left, right| left.control_id.cmp(&right.control_id));
    opportunities
}

fn classify_control_kind(
    control: &CustomizerControl,
    slot_roles: &BTreeMap<&str, &str>,
) -> ControlOpportunityKind {
    match &control.kind {
        ControlKind::ContinuousAxis { .. } => ControlOpportunityKind::Scalar,
        ControlKind::IntegerStepper { .. } => ControlOpportunityKind::Integer,
        ControlKind::Toggle { .. } => role_for_toggle(control, slot_roles)
            .map(|role| ControlOpportunityKind::RolePresence { role })
            .unwrap_or(ControlOpportunityKind::Toggle),
        ControlKind::ChoiceGallery { .. } => ControlOpportunityKind::Choice,
        ControlKind::ProviderGallery { role, .. } => {
            ControlOpportunityKind::Provider { role: role.clone() }
        }
    }
}

fn role_for_toggle(
    control: &CustomizerControl,
    slot_roles: &BTreeMap<&str, &str>,
) -> Option<String> {
    for binding in &control.bindings {
        if let Some(role) = slot_roles.get(binding.slot.as_str()) {
            return Some((*role).to_owned());
        }
    }
    control.id.strip_prefix("has_").map(str::to_owned)
}

fn protected_by_lock(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
    kind: &ControlOpportunityKind,
) -> bool {
    if lock_matches(document, &FoundryLockTarget::Control(control.id.clone())) {
        return true;
    }
    match kind {
        ControlOpportunityKind::Provider { role } => {
            lock_matches(document, &FoundryLockTarget::Provider(role.clone()))
                || lock_matches(document, &FoundryLockTarget::Role(role.clone()))
        }
        ControlOpportunityKind::RolePresence { role } => {
            lock_matches(document, &FoundryLockTarget::Role(role.clone()))
        }
        ControlOpportunityKind::Scalar
        | ControlOpportunityKind::Integer
        | ControlOpportunityKind::Toggle
        | ControlOpportunityKind::Choice => false,
    }
}

fn lock_matches(document: &FoundryAssetDocument, target: &FoundryLockTarget) -> bool {
    document.foundry_locks.iter().any(|lock| {
        lock.target == *target
            && matches!(
                lock.mode,
                FoundryLockMode::Locked | FoundryLockMode::SearchProtected
            )
    })
}

fn classify_control(control: &CustomizerControl, kind: &ControlOpportunityKind) -> ControlClass {
    if matches!(
        kind,
        ControlOpportunityKind::Provider { .. } | ControlOpportunityKind::RolePresence { .. }
    ) {
        return ControlClass::Structure;
    }

    let mut text = format!("{} {}", control.id, control.label).to_ascii_lowercase();
    for binding in &control.bindings {
        text.push(' ');
        text.push_str(&binding.slot.to_ascii_lowercase());
    }

    if contains_any(
        &text,
        &[
            "detail", "edge", "bevel", "segment", "weather", "trim", "bolt", "rim", "corner",
            "profile", "density",
        ],
    ) {
        return ControlClass::Detail;
    }
    if matches!(kind, ControlOpportunityKind::Integer)
        || contains_any(
            &text,
            &["count", "repeat", "array", "provider", "presence", "role"],
        )
    {
        return ControlClass::Structure;
    }
    if control.topology_behavior == ControlTopologyBehavior::TopologyChanging {
        return ControlClass::Structure;
    }
    ControlClass::Silhouette
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn mode_allows_control(
    mode: FoundryCandidateMode,
    class: ControlClass,
    topology_behavior: ControlTopologyBehavior,
    kind: &ControlOpportunityKind,
    variation_intent: &VariationIntent,
) -> bool {
    match mode {
        FoundryCandidateMode::Refine => {
            (topology_behavior == ControlTopologyBehavior::TopologyPreserving
                || variation_intent.scope.is_focus_part())
                && !matches!(
                    kind,
                    ControlOpportunityKind::Provider { .. }
                        | ControlOpportunityKind::RolePresence { .. }
                )
        }
        FoundryCandidateMode::Explore => true,
        FoundryCandidateMode::Silhouette => class == ControlClass::Silhouette,
        FoundryCandidateMode::Structure => class == ControlClass::Structure,
        FoundryCandidateMode::Detail => class == ControlClass::Detail,
    }
}

fn select_opportunities<'a>(
    opportunities: &'a [ControlOpportunity],
    mode: FoundryCandidateMode,
    proposal_index: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<&'a ControlOpportunity> {
    if opportunities.is_empty() {
        return Vec::new();
    }

    let (minimum, maximum) = selection_bounds(mode);
    let capacity = selection_capacity(opportunities, mode);
    if capacity < minimum {
        return Vec::new();
    }

    let maximum = maximum.min(capacity);
    let minimum = minimum.min(maximum);
    let target_count = if minimum == maximum {
        minimum
    } else {
        rng.random_range(minimum..=maximum)
    };

    let mut order = (0..opportunities.len()).collect::<Vec<_>>();
    let rotation = proposal_index % opportunities.len().max(1);
    order.rotate_left(rotation);
    let mut ranked = order
        .into_iter()
        .map(|index| (proposal_seed(rng.random::<u64>(), index as u64), index))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut selected = Vec::with_capacity(target_count);
    let mut provider_or_presence_count = 0;
    for (_, index) in ranked {
        let opportunity = &opportunities[index];
        if mode == FoundryCandidateMode::Explore
            && opportunity.is_provider_or_role_presence()
            && provider_or_presence_count >= 1
        {
            continue;
        }
        if opportunity.is_provider_or_role_presence() {
            provider_or_presence_count += 1;
        }
        selected.push(opportunity);
        if selected.len() == target_count {
            break;
        }
    }
    selected
}

fn selection_capacity(opportunities: &[ControlOpportunity], mode: FoundryCandidateMode) -> usize {
    if mode != FoundryCandidateMode::Explore {
        return opportunities.len();
    }

    let provider_or_presence = opportunities
        .iter()
        .filter(|opportunity| opportunity.is_provider_or_role_presence())
        .count();
    let other_controls = opportunities.len().saturating_sub(provider_or_presence);
    other_controls + provider_or_presence.min(1)
}

fn selection_bounds(mode: FoundryCandidateMode) -> (usize, usize) {
    match mode {
        FoundryCandidateMode::Refine => (1, 2),
        FoundryCandidateMode::Explore => (2, 4),
        FoundryCandidateMode::Silhouette
        | FoundryCandidateMode::Structure
        | FoundryCandidateMode::Detail => (1, 3),
    }
}

fn build_candidate_edit(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    selected: Vec<&ControlOpportunity>,
    mode: FoundryCandidateMode,
    variation_intent: &VariationIntent,
    proposal_index: usize,
    rng: &mut ChaCha8Rng,
) -> Option<CandidateEditProposal> {
    let required_command_count = selection_bounds(mode).0.min(selected.len());
    let mut commands = Vec::new();
    let mut changed_controls = Vec::new();
    let mut changes = Vec::new();
    let mut seen_controls = BTreeSet::new();
    for opportunity in selected {
        if !seen_controls.insert(opportunity.control_id.clone()) {
            continue;
        }
        let control = profile
            .controls
            .iter()
            .find(|control| control.id == opportunity.control_id)?;
        let Some(value) = mutate_control_value(opportunity, mode, rng) else {
            continue;
        };
        if value == opportunity.current {
            continue;
        }
        let command = FoundryCommand::SetControl {
            control_id: opportunity.control_id.clone(),
            value: value.clone(),
        };
        let change = describe_change(
            profile,
            context,
            control,
            opportunity,
            opportunity.current.clone(),
            value,
        )?;
        commands.push(command);
        changed_controls.push(opportunity.control_id.clone());
        changes.push(change);
    }

    if commands.is_empty() || commands.len() < required_command_count {
        return None;
    }
    let label = candidate_intent_label(variation_intent, mode, proposal_index);
    Some(CandidateEditProposal {
        edit: FoundryEdit { label, commands },
        changed_controls,
        changes,
    })
}

fn candidate_intent_label(
    intent: &VariationIntent,
    mode: FoundryCandidateMode,
    proposal_index: usize,
) -> String {
    const COMPLETE_LOOK_LABELS: [&str; 6] = [
        "Compact Vented",
        "Reinforced Cargo",
        "Minimal Industrial",
        "Worn Field Crate",
        "Heavy Utility",
        "Clean Lab Crate",
    ];
    const SHAPE_LABELS: [&str; 6] = [
        "Broader Silhouette",
        "Compact Structure",
        "Raised Profile",
        "Balanced Frame",
        "Heavy Utility",
        "Clean Proportions",
    ];
    const DETAIL_LABELS: [&str; 6] = [
        "Sharper Details",
        "Cleaner Trim",
        "Added Fasteners",
        "Refined Edges",
        "Panel Detail",
        "Service Detail",
    ];
    const FOCUS_LABELS: [&str; 6] = [
        "Focused Variant",
        "Focused Structure",
        "Focused Detail",
        "Focused Profile",
        "Focused Trim",
        "Focused Utility",
    ];

    let labels = if intent.scope.is_focus_part() {
        &FOCUS_LABELS
    } else if intent.includes_channel(&VariationChannel::Shape)
        || matches!(
            mode,
            FoundryCandidateMode::Silhouette | FoundryCandidateMode::Structure
        )
    {
        &SHAPE_LABELS
    } else if intent.includes_channel(&VariationChannel::Detail)
        || mode == FoundryCandidateMode::Detail
    {
        &DETAIL_LABELS
    } else {
        &COMPLETE_LOOK_LABELS
    };
    labels[proposal_index % labels.len()].to_owned()
}

fn mutate_control_value(
    opportunity: &ControlOpportunity,
    mode: FoundryCandidateMode,
    rng: &mut ChaCha8Rng,
) -> Option<ControlValue> {
    let value = match (&opportunity.kind, &opportunity.current) {
        (ControlOpportunityKind::Scalar, ControlValue::Scalar(current)) => {
            ControlValue::Scalar(mutate_scalar(*current, &opportunity.domain, mode, rng)?)
        }
        (ControlOpportunityKind::Integer, ControlValue::Integer(current)) => {
            ControlValue::Integer(mutate_integer(*current, &opportunity.domain, mode, rng)?)
        }
        (ControlOpportunityKind::Toggle, ControlValue::Toggle(current))
        | (ControlOpportunityKind::RolePresence { .. }, ControlValue::Toggle(current)) => {
            ControlValue::Toggle(!current)
        }
        (ControlOpportunityKind::Choice, ControlValue::Choice(current)) => choose_discrete_value(
            &opportunity.domain,
            &ControlValue::Choice(current.clone()),
            rng,
        )?,
        (ControlOpportunityKind::Provider { .. }, ControlValue::Provider(current)) => {
            choose_discrete_value(
                &opportunity.domain,
                &ControlValue::Provider(current.clone()),
                rng,
            )?
        }
        _ => return None,
    };
    Some(value).filter(|candidate| opportunity.domain.contains_available_value(candidate))
}

fn mutate_scalar(
    current: f32,
    domain: &FeasibleControlDomain,
    mode: FoundryCandidateMode,
    rng: &mut ChaCha8Rng,
) -> Option<f32> {
    let (minimum, maximum) = continuous_domain_bounds(domain)?;
    let span = (maximum - minimum).abs().max(EPSILON);
    let movement = numeric_movement_fraction(mode);
    let direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    let amount = span * movement * rng.random_range(0.35..=1.0);
    let mut candidate = current + direction * amount;
    candidate = clamp_to_continuous_domain(candidate, domain)?;
    if (candidate - current).abs() <= EPSILON {
        candidate = clamp_to_continuous_domain(current - direction * amount, domain)?;
    }
    ((candidate - current).abs() > EPSILON).then_some(candidate)
}

fn mutate_integer(
    current: i64,
    domain: &FeasibleControlDomain,
    mode: FoundryCandidateMode,
    rng: &mut ChaCha8Rng,
) -> Option<i64> {
    let values = available_integers(domain);
    if values.len() < 2 {
        return None;
    }
    let minimum = *values.first()?;
    let maximum = *values.last()?;
    let span = (maximum - minimum).abs().max(1) as f32;
    let max_delta = (span * numeric_movement_fraction(mode)).ceil().max(1.0) as i64;
    let mut local = values
        .iter()
        .copied()
        .filter(|value| *value != current && (*value - current).abs() <= max_delta)
        .collect::<Vec<_>>();
    if local.is_empty() {
        local = values
            .iter()
            .copied()
            .filter(|value| *value != current)
            .collect();
    }
    local.sort();
    (!local.is_empty()).then(|| local[rng.random_range(0..local.len())])
}

fn numeric_movement_fraction(mode: FoundryCandidateMode) -> f32 {
    match mode {
        FoundryCandidateMode::Refine => 0.15,
        FoundryCandidateMode::Explore => 0.45,
        FoundryCandidateMode::Silhouette => 0.35,
        FoundryCandidateMode::Structure => 0.45,
        FoundryCandidateMode::Detail => 0.35,
    }
}

fn continuous_domain_bounds(domain: &FeasibleControlDomain) -> Option<(f32, f32)> {
    let first = domain.continuous_intervals.first()?;
    let last = domain.continuous_intervals.last().unwrap_or(first);
    Some((first.minimum, last.maximum))
}

fn clamp_to_continuous_domain(value: f32, domain: &FeasibleControlDomain) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    domain
        .continuous_intervals
        .iter()
        .map(|interval| value.clamp(interval.minimum, interval.maximum))
        .min_by(|left, right| {
            (left - value)
                .abs()
                .partial_cmp(&(right - value).abs())
                .unwrap_or(Ordering::Equal)
        })
}

fn available_integers(domain: &FeasibleControlDomain) -> Vec<i64> {
    let mut values = domain
        .discrete_values
        .iter()
        .filter(|value| domain.contains_available_value(value))
        .filter_map(|value| match value {
            ControlValue::Integer(value) => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn choose_discrete_value(
    domain: &FeasibleControlDomain,
    current: &ControlValue,
    rng: &mut ChaCha8Rng,
) -> Option<ControlValue> {
    let mut values = domain
        .discrete_values
        .iter()
        .filter(|value| *value != current && domain.contains_available_value(value))
        .cloned()
        .collect::<Vec<_>>();
    values.sort_by(control_value_order);
    (!values.is_empty()).then(|| values[rng.random_range(0..values.len())].clone())
}

fn first_domain_value(domain: &FeasibleControlDomain) -> ControlValue {
    domain
        .discrete_values
        .iter()
        .find(|value| domain.contains_available_value(value))
        .cloned()
        .or_else(|| {
            domain
                .continuous_intervals
                .first()
                .map(|interval| ControlValue::Scalar((interval.minimum + interval.maximum) * 0.5))
        })
        .unwrap_or(ControlValue::Scalar(0.0))
}

fn control_value_order(left: &ControlValue, right: &ControlValue) -> Ordering {
    control_value_rank(left)
        .cmp(&control_value_rank(right))
        .then_with(|| match (left, right) {
            (ControlValue::Scalar(left), ControlValue::Scalar(right)) => left.total_cmp(right),
            (ControlValue::Integer(left), ControlValue::Integer(right)) => left.cmp(right),
            (ControlValue::Toggle(left), ControlValue::Toggle(right)) => left.cmp(right),
            (ControlValue::Choice(left), ControlValue::Choice(right))
            | (ControlValue::Provider(left), ControlValue::Provider(right)) => left.cmp(right),
            _ => Ordering::Equal,
        })
}

fn control_value_rank(value: &ControlValue) -> u8 {
    match value {
        ControlValue::Scalar(_) => 0,
        ControlValue::Integer(_) => 1,
        ControlValue::Toggle(_) => 2,
        ControlValue::Choice(_) => 3,
        ControlValue::Provider(_) => 4,
    }
}

fn describe_change(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    control: &CustomizerControl,
    opportunity: &ControlOpportunity,
    before: ControlValue,
    after: ControlValue,
) -> Option<FoundryCandidateControlChange> {
    let delta = explain_control_delta(
        profile,
        context,
        &control.id,
        Some(before.clone()),
        after.clone(),
    )
    .ok()?;
    let before = value_label(control, &before);
    let after = value_label(control, &after);
    Some(FoundryCandidateControlChange {
        kind: change_kind(opportunity),
        control_id: control.id.clone(),
        control_label: opportunity.label.clone(),
        before: before.clone(),
        after: after.clone(),
        message: format!("{} changed from `{before}` to `{after}`.", control.label),
        details: delta.explanations,
        topology_changing: opportunity.topology_changing(),
    })
}

fn change_kind(opportunity: &ControlOpportunity) -> FoundryCandidateChangeKind {
    if opportunity.class == ControlClass::Detail {
        return FoundryCandidateChangeKind::Detail;
    }
    match opportunity.kind {
        ControlOpportunityKind::Scalar => FoundryCandidateChangeKind::Numeric,
        ControlOpportunityKind::Integer => FoundryCandidateChangeKind::Repetition,
        ControlOpportunityKind::Toggle => FoundryCandidateChangeKind::Choice,
        ControlOpportunityKind::Choice => FoundryCandidateChangeKind::Choice,
        ControlOpportunityKind::Provider { .. } => FoundryCandidateChangeKind::Provider,
        ControlOpportunityKind::RolePresence { .. } => FoundryCandidateChangeKind::RolePresence,
    }
}

fn value_label(control: &CustomizerControl, value: &ControlValue) -> String {
    match (&control.kind, value) {
        (ControlKind::ChoiceGallery { options }, ControlValue::Choice(value)) => options
            .iter()
            .find(|option| option.value == *value)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| value.clone()),
        (ControlKind::ProviderGallery { options, .. }, ControlValue::Provider(value)) => options
            .iter()
            .find(|option| option.provider_id == *value)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| value.clone()),
        (_, ControlValue::Scalar(value)) => format!("{value:.3}"),
        (_, ControlValue::Integer(value)) => value.to_string(),
        (_, ControlValue::Toggle(value)) => value.to_string(),
        (_, ControlValue::Choice(value) | ControlValue::Provider(value)) => value.clone(),
    }
}

fn descriptor_input_from_output(
    id: &str,
    output: &FoundryCompilationOutput,
) -> AssetCandidateInput {
    let bounds = aabb_from_positions(&output.artifact.combined_preview.mesh.positions);
    let mut input =
        AssetCandidateInput::new(id, output.build_stamp.recipe_fingerprint.0.to_hex(), bounds);
    input.recipe_valid = shape_asset::validate_asset_recipe(&output.recipe).is_valid();
    input.compile_succeeded = output.artifact.validation_report.is_valid();
    input.closed_manifold = output.artifact.validation_report.is_valid();
    input.triangle_count = output.artifact.statistics.triangle_count as usize;
    input.triangle_budget = 250_000;
    input.geometry_finite = positions_are_finite(&output.artifact.combined_preview.mesh.positions);
    input.provenance_complete = !output
        .artifact
        .provenance_report
        .part_region_operation_mappings
        .is_empty();
    input.world_bounds = bounds;
    input.part_volumes = output
        .artifact
        .compiled_parts
        .iter()
        .map(|part| {
            mesh_bounds_volume(
                part.world_mesh.bounds.min,
                part.world_mesh.bounds.max,
                part.world_mesh.bounds.is_empty(),
            )
        })
        .filter(|volume| volume.is_finite() && *volume > 0.0)
        .collect();
    if input.part_volumes.is_empty() {
        input.part_volumes = vec![bounds_volume(bounds).max(EPSILON)];
    }
    input.volume_approximation = input.part_volumes.iter().sum::<f32>().max(EPSILON);
    let (masks, occupancy, perimeter, depth_histogram) = silhouette_descriptors(
        &output.artifact.combined_preview.mesh.positions,
        &output.artifact.combined_preview.mesh.indices,
    );
    input.silhouette_masks = masks;
    input.silhouette_occupancy = occupancy;
    input.silhouette_perimeter = perimeter;
    input.depth_histogram = depth_histogram;
    input.region_count = output
        .recipe
        .definitions
        .values()
        .map(|definition| definition.regions.len())
        .sum();
    input.detail_count = output
        .recipe
        .definitions
        .values()
        .map(|definition| definition.geometry.operations.len())
        .sum();
    input.repeated_element_count = repeated_element_count(&output.recipe);
    input.bevel_radii = bevel_radii(&output.recipe);
    input.symmetry_score =
        approximate_symmetry_score(&output.artifact.combined_preview.mesh.positions);
    input.topology_cost = topology_cost(&output.recipe);
    input.detached_visual_components = output.artifact.compiled_parts.len().saturating_sub(1);
    input
}

fn candidate_descriptor_is_usable(input: &AssetCandidateInput) -> bool {
    input.geometry_finite
        && !input.world_bounds.is_empty()
        && input.triangle_count > 0
        && input
            .silhouette_occupancy
            .iter()
            .all(|value| value.is_finite())
}

fn aabb_from_positions(positions: &[[f32; 3]]) -> Aabb {
    let mut minimum = [Scalar::INFINITY; 3];
    let mut maximum = [Scalar::NEG_INFINITY; 3];
    for point in positions {
        if !point.iter().all(|value| value.is_finite()) {
            continue;
        }
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(point[axis]);
            maximum[axis] = maximum[axis].max(point[axis]);
        }
    }
    if minimum[0] > maximum[0] {
        return Aabb::empty();
    }
    let mut min = Transform3::default().translation;
    min.x = minimum[0];
    min.y = minimum[1];
    min.z = minimum[2];
    let mut max = Transform3::default().translation;
    max.x = maximum[0];
    max.y = maximum[1];
    max.z = maximum[2];
    Aabb { min, max }
}

fn positions_are_finite(positions: &[[f32; 3]]) -> bool {
    !positions.is_empty()
        && positions
            .iter()
            .all(|point| point.iter().all(|value| value.is_finite()))
}

fn mesh_bounds_volume(min: [f32; 3], max: [f32; 3], empty: bool) -> f32 {
    if empty {
        return 0.0;
    }
    ((max[0] - min[0]).max(0.0) * (max[1] - min[1]).max(0.0) * (max[2] - min[2]).max(0.0)).max(0.0)
}

fn bounds_volume(bounds: Aabb) -> f32 {
    if bounds.is_empty() {
        return 0.0;
    }
    let extent = bounds.extent();
    (extent.x * extent.y * extent.z).max(0.0)
}

fn silhouette_descriptors(
    positions: &[[f32; 3]],
    indices: &[u32],
) -> (Vec<Vec<u64>>, [f32; FIXED_CAMERA_COUNT], Vec<f32>, Vec<f32>) {
    let mut masks = Vec::with_capacity(FIXED_CAMERA_COUNT);
    let mut occupancy = [0.0; FIXED_CAMERA_COUNT];
    let mut perimeter = Vec::with_capacity(FIXED_CAMERA_COUNT);
    let mut depth_histogram = Vec::with_capacity(FIXED_CAMERA_COUNT * DEPTH_HISTOGRAM_BINS);

    for (camera_index, camera) in SILHOUETTE_CAMERAS.into_iter().enumerate() {
        let bounds = projected_bounds(positions, camera);
        let mut mask = vec![0_u64; SILHOUETTE_MASK_WORDS];
        let mut depths = vec![0.0_f32; DEPTH_HISTOGRAM_BINS];
        for triangle in indices.chunks(3) {
            if triangle.len() != 3 {
                continue;
            }
            let Some(points) = triangle_points(positions, triangle) else {
                continue;
            };
            rasterize_projected_triangle(&mut mask, &points, camera, &bounds);
            let depth = points
                .iter()
                .map(|point| normalize_axis(project(point, camera.depth), bounds[2]))
                .sum::<f32>()
                / 3.0;
            let bin = ((depth.clamp(0.0, 0.999_999) * DEPTH_HISTOGRAM_BINS as f32) as usize)
                .min(DEPTH_HISTOGRAM_BINS - 1);
            depths[bin] += 1.0;
        }
        let set_bits = mask
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum::<usize>();
        occupancy[camera_index] =
            set_bits as f32 / (SILHOUETTE_MASK_SIDE * SILHOUETTE_MASK_SIDE) as f32;
        perimeter.push(mask_perimeter(&mask));
        let depth_total = depths.iter().sum::<f32>().max(EPSILON);
        depth_histogram.extend(depths.into_iter().map(|value| value / depth_total));
        masks.push(mask);
    }

    (masks, occupancy, perimeter, depth_histogram)
}

fn numeric_bounds(positions: &[[f32; 3]]) -> [[f32; 2]; 3] {
    let mut bounds = [[f32::INFINITY, f32::NEG_INFINITY]; 3];
    for point in positions {
        for axis in 0..3 {
            bounds[axis][0] = bounds[axis][0].min(point[axis]);
            bounds[axis][1] = bounds[axis][1].max(point[axis]);
        }
    }
    for axis_bounds in &mut bounds {
        if !axis_bounds[0].is_finite()
            || !axis_bounds[1].is_finite()
            || axis_bounds[0] >= axis_bounds[1]
        {
            *axis_bounds = [0.0, 1.0];
        }
    }
    bounds
}

fn projected_bounds(positions: &[[f32; 3]], camera: SilhouetteCamera) -> [[f32; 2]; 3] {
    let mut bounds = [[f32::INFINITY, f32::NEG_INFINITY]; 3];
    for point in positions {
        let projected = [
            project(point, camera.u),
            project(point, camera.v),
            project(point, camera.depth),
        ];
        for axis in 0..3 {
            bounds[axis][0] = bounds[axis][0].min(projected[axis]);
            bounds[axis][1] = bounds[axis][1].max(projected[axis]);
        }
    }
    for axis_bounds in &mut bounds {
        if !axis_bounds[0].is_finite()
            || !axis_bounds[1].is_finite()
            || axis_bounds[0] >= axis_bounds[1]
        {
            *axis_bounds = [0.0, 1.0];
        }
    }
    bounds
}

fn triangle_points<'a>(positions: &'a [[f32; 3]], triangle: &[u32]) -> Option<[&'a [f32; 3]; 3]> {
    Some([
        positions.get(triangle[0] as usize)?,
        positions.get(triangle[1] as usize)?,
        positions.get(triangle[2] as usize)?,
    ])
}

fn rasterize_projected_triangle(
    mask: &mut [u64],
    points: &[&[f32; 3]; 3],
    camera: SilhouetteCamera,
    bounds: &[[f32; 2]; 3],
) {
    let mut minimum = [f32::INFINITY; 2];
    let mut maximum = [f32::NEG_INFINITY; 2];
    for point in points {
        let projected = [
            normalize_axis(project(point, camera.u), bounds[0]),
            normalize_axis(project(point, camera.v), bounds[1]),
        ];
        minimum[0] = minimum[0].min(projected[0]);
        minimum[1] = minimum[1].min(projected[1]);
        maximum[0] = maximum[0].max(projected[0]);
        maximum[1] = maximum[1].max(projected[1]);
    }
    let min_x = grid_index(minimum[0]);
    let max_x = grid_index(maximum[0]);
    let min_y = grid_index(minimum[1]);
    let max_y = grid_index(maximum[1]);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            set_mask_bit(mask, y * SILHOUETTE_MASK_SIDE + x);
        }
    }
}

fn project(point: &[f32; 3], axis: [f32; 3]) -> f32 {
    point[0] * axis[0] + point[1] * axis[1] + point[2] * axis[2]
}

fn normalize_axis(value: f32, bounds: [f32; 2]) -> f32 {
    let span = (bounds[1] - bounds[0]).max(EPSILON);
    ((value - bounds[0]) / span).clamp(0.0, 1.0)
}

fn grid_index(value: f32) -> usize {
    (value.clamp(0.0, 0.999_999) * SILHOUETTE_MASK_SIDE as f32) as usize
}

fn set_mask_bit(mask: &mut [u64], bit: usize) {
    let word = bit / 64;
    let offset = bit % 64;
    if let Some(value) = mask.get_mut(word) {
        *value |= 1_u64 << offset;
    }
}

fn mask_bit(mask: &[u64], x: isize, y: isize) -> bool {
    if x < 0 || y < 0 || x >= SILHOUETTE_MASK_SIDE as isize || y >= SILHOUETTE_MASK_SIDE as isize {
        return false;
    }
    let bit = y as usize * SILHOUETTE_MASK_SIDE + x as usize;
    mask.get(bit / 64)
        .is_some_and(|word| (*word & (1_u64 << (bit % 64))) != 0)
}

fn mask_perimeter(mask: &[u64]) -> f32 {
    let mut boundary = 0_usize;
    let mut filled = 0_usize;
    for y in 0..SILHOUETTE_MASK_SIDE as isize {
        for x in 0..SILHOUETTE_MASK_SIDE as isize {
            if !mask_bit(mask, x, y) {
                continue;
            }
            filled += 1;
            if !mask_bit(mask, x - 1, y)
                || !mask_bit(mask, x + 1, y)
                || !mask_bit(mask, x, y - 1)
                || !mask_bit(mask, x, y + 1)
            {
                boundary += 1;
            }
        }
    }
    if filled == 0 {
        0.0
    } else {
        boundary as f32 / filled as f32
    }
}

fn repeated_element_count(recipe: &shape_asset::AssetRecipe) -> usize {
    recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .map(|operation| match operation {
            ModelingOperationSpec::LinearArray { count, .. }
            | ModelingOperationSpec::RadialArray { count, .. } => count.saturating_sub(1) as usize,
            _ => 0,
        })
        .sum()
}

fn bevel_radii(recipe: &shape_asset::AssetRecipe) -> Vec<f32> {
    let mut radii = Vec::new();
    for definition in recipe.definitions.values() {
        match &definition.geometry.source {
            GeometrySource::RoundedBox { radius, .. } => radii.push(*radius),
            GeometrySource::Cylinder { radius, .. } => radii.push(*radius * 0.08),
            _ => {}
        }
        for operation in &definition.geometry.operations {
            match operation {
                ModelingOperationSpec::SetBevelProfile { radius, .. } => radii.push(*radius),
                ModelingOperationSpec::BevelBoundaryLoop { width, .. } => radii.push(*width),
                _ => {}
            }
        }
    }
    if radii.is_empty() {
        radii.push(0.01);
    }
    radii
}

fn topology_cost(recipe: &shape_asset::AssetRecipe) -> f32 {
    let definitions = recipe.definitions.len() as f32;
    let instances = recipe.instances.len() as f32;
    let operations = recipe
        .definitions
        .values()
        .map(|definition| definition.geometry.operations.len())
        .sum::<usize>() as f32;
    (definitions * 0.12 + instances * 0.08 + operations * 0.16).max(0.0)
}

fn approximate_symmetry_score(positions: &[[f32; 3]]) -> f32 {
    if positions.is_empty() {
        return 0.0;
    }
    let bounds = numeric_bounds(positions);
    let center_x = (bounds[0][0] + bounds[0][1]) * 0.5;
    let mean_abs_x = positions
        .iter()
        .map(|point| (point[0] - center_x).abs())
        .sum::<f32>()
        / positions.len() as f32;
    let mean_x = positions
        .iter()
        .map(|point| point[0] - center_x)
        .sum::<f32>()
        / positions.len() as f32;
    if mean_abs_x <= EPSILON {
        1.0
    } else {
        (1.0 - (mean_x.abs() / mean_abs_x).min(1.0)).clamp(0.0, 1.0)
    }
}

fn proposal_seed(seed: u64, proposal_index: u64) -> u64 {
    let mut value = seed ^ proposal_index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn candidate_id(seed: u64, proposal_index: usize) -> FoundryCandidateId {
    FoundryCandidateId(format!(
        "foundry-{:016x}-{:04}",
        proposal_seed(seed ^ 0xa5a5_5a5a_1234_5678, proposal_index as u64),
        proposal_index
    ))
}

fn increment_rejection(
    diagnostics: &mut FoundryCandidateGenerationDiagnostics,
    reason: FoundryCandidateRejectionReason,
) {
    *diagnostics.rejections.entry(reason).or_insert(0) += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    use shape_family_compile::conformance::FamilyConformanceReport;
    use shape_foundry::{ClosedInterval, DomainCertification};
    use shape_foundry_catalog::{roman_bridge, scifi_crate, stylized_lamp};

    #[test]
    fn silhouette_descriptor_views_are_distinct() {
        let mut depth_axes = BTreeSet::new();
        let mut projection_planes = BTreeSet::new();
        for camera in SILHOUETTE_CAMERAS {
            assert_orthonormal_camera(camera);
            depth_axes.insert(canonical_axis(camera.depth));
            projection_planes.insert((
                canonical_axis(camera.u),
                canonical_axis(camera.v),
                canonical_axis(camera.depth),
            ));
        }
        assert_eq!(depth_axes.len(), FIXED_CAMERA_COUNT);
        assert_eq!(projection_planes.len(), FIXED_CAMERA_COUNT);
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.1, 0.2, 0.05],
            [0.7, 0.4, 0.3],
            [0.4, 0.9, 0.6],
        ];
        let indices = vec![2, 3, 4];

        let (masks, occupancy, perimeter, depth_histogram) =
            silhouette_descriptors(&positions, &indices);

        assert_eq!(masks.len(), FIXED_CAMERA_COUNT);
        assert_eq!(
            masks.iter().collect::<BTreeSet<_>>().len(),
            FIXED_CAMERA_COUNT
        );
        assert_eq!(occupancy.len(), FIXED_CAMERA_COUNT);
        assert_eq!(perimeter.len(), FIXED_CAMERA_COUNT);
        assert_eq!(
            depth_histogram.len(),
            FIXED_CAMERA_COUNT * DEPTH_HISTOGRAM_BINS
        );
    }

    fn assert_orthonormal_camera(camera: SilhouetteCamera) {
        assert!((dot(camera.u, camera.u) - 1.0).abs() < 1.0e-5);
        assert!((dot(camera.v, camera.v) - 1.0).abs() < 1.0e-5);
        assert!((dot(camera.depth, camera.depth) - 1.0).abs() < 1.0e-5);
        assert!(dot(camera.u, camera.v).abs() < 1.0e-5);
        assert!(dot(camera.u, camera.depth).abs() < 1.0e-5);
        assert!(dot(camera.v, camera.depth).abs() < 1.0e-5);
    }

    fn canonical_axis(axis: [f32; 3]) -> [i32; 3] {
        [
            (axis[0] * 1_000_000.0).round() as i32,
            (axis[1] * 1_000_000.0).round() as i32,
            (axis[2] * 1_000_000.0).round() as i32,
        ]
    }

    fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
        left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
    }

    fn test_candidate_input(id: &str, extent: [f32; 3], mask_word: u64) -> AssetCandidateInput {
        let bounds = test_bounds(extent);
        let mut input = AssetCandidateInput::new(id, format!("recipe-{id}"), bounds);
        input.silhouette_masks = vec![vec![mask_word; SILHOUETTE_MASK_WORDS]; FIXED_CAMERA_COUNT];
        input.silhouette_occupancy = [if mask_word == 0 { 0.0 } else { 0.72 }; FIXED_CAMERA_COUNT];
        input.silhouette_perimeter =
            vec![if mask_word == 0 { 0.0 } else { 0.25 }; FIXED_CAMERA_COUNT];
        input.depth_histogram = vec![0.0; FIXED_CAMERA_COUNT * DEPTH_HISTOGRAM_BINS];
        for camera in 0..FIXED_CAMERA_COUNT {
            input.depth_histogram[camera * DEPTH_HISTOGRAM_BINS] = 1.0;
        }
        input.part_volumes = vec![extent[0] * extent[1] * extent[2]];
        input.volume_approximation = input.part_volumes[0];
        input
    }

    fn test_bounds(extent: [f32; 3]) -> Aabb {
        let mut min = Transform3::default().translation;
        min.x = -extent[0] * 0.5;
        min.y = -extent[1] * 0.5;
        min.z = -extent[2] * 0.5;
        let mut max = Transform3::default().translation;
        max.x = extent[0] * 0.5;
        max.y = extent[1] * 0.5;
        max.z = extent[2] * 0.5;
        Aabb { min, max }
    }

    fn numeric_proposal(
        control_id: &str,
        kind: FoundryCandidateChangeKind,
    ) -> CandidateEditProposal {
        CandidateEditProposal {
            edit: FoundryEdit {
                label: format!("{control_id} variant"),
                commands: Vec::new(),
            },
            changed_controls: vec![control_id.to_owned()],
            changes: vec![FoundryCandidateControlChange {
                kind,
                control_id: control_id.to_owned(),
                control_label: control_id.replace('_', " "),
                before: "before".to_owned(),
                after: "after".to_owned(),
                message: format!("{control_id} changed."),
                details: Vec::new(),
                topology_changing: kind != FoundryCandidateChangeKind::Detail,
            }],
        }
    }

    #[test]
    fn identical_mesh_rejected_as_duplicate_looking() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let candidate = parent.clone();
        let proposal = numeric_proposal("body_proportions", FoundryCandidateChangeKind::Numeric);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "same".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::complete_look(),
            &proposal,
            Some(&evidence),
        );

        assert_eq!(
            metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::DuplicateLooking
        );
        assert_eq!(
            variation_rejection_reason(&metadata),
            Some(FoundryCandidateRejectionReason::DuplicateLooking)
        );
    }

    #[test]
    fn tiny_scalar_only_change_rejected_for_whole_asset() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let candidate = test_candidate_input("tiny", [1.01, 1.0, 1.0], 0);
        let proposal = numeric_proposal("body_proportions", FoundryCandidateChangeKind::Numeric);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "tiny".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::complete_look(),
            &proposal,
            Some(&evidence),
        );

        assert_eq!(
            metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::DuplicateLooking
        );
    }

    #[test]
    fn obvious_handle_provider_change_is_accepted() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let mut candidate = test_candidate_input("handle", [1.0, 1.0, 1.0], u64::MAX);
        candidate.detached_visual_components = 1;
        let proposal = numeric_proposal("handle_style", FoundryCandidateChangeKind::Provider);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "handle".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::complete_look(),
            &proposal,
            Some(&evidence),
        );

        assert!(matches!(
            metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
        ));
    }

    #[test]
    fn focused_refine_handles_generate_scifi_crate_candidates() {
        let fixture = scifi_crate::fixture_catalog();
        let request = FoundryCandidateRequest {
            seed: fixture.document.seed,
            proposal_count: 12,
            result_count: 3,
            mode: FoundryCandidateMode::Refine,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::focus_part_shape("handles", "Handles"),
        };

        let output = generate_foundry_candidate_draft_plans(&fixture.document, &fixture, &request)
            .expect("focused handles request should generate candidates");

        assert_eq!(output.diagnostics.available_control_count, 1);
        assert!(
            !output.candidates.is_empty(),
            "focused handles request should return visible candidate cards"
        );
        assert!(output.candidates.iter().all(|candidate| {
            candidate
                .changed_controls
                .iter()
                .any(|control| control == "handle_style")
        }));
    }

    #[test]
    fn obvious_body_proportion_change_is_accepted() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let candidate = test_candidate_input("body", [1.8, 1.0, 0.8], u64::MAX);
        let proposal = numeric_proposal("body_proportions", FoundryCandidateChangeKind::Numeric);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "body".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::whole_asset_shape(),
            &proposal,
            Some(&evidence),
        );

        assert!(matches!(
            metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
        ));
    }

    #[test]
    fn detail_only_candidate_not_accepted_in_whole_asset_mode() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let mut candidate = parent.clone();
        candidate.id = "detail".to_owned();
        candidate.detail_count += 1;
        let proposal = numeric_proposal("detail_density", FoundryCandidateChangeKind::Detail);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "detail".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::complete_look(),
            &proposal,
            Some(&evidence),
        );

        assert!(!metadata.visible_delta.legibility_class.selectable());
    }

    #[test]
    fn fewer_than_six_survivors_reported_honestly() {
        let diagnostics = FoundryCandidateGenerationDiagnostics {
            requested_proposals: 8,
            requested_candidates: 6,
            attempted_proposals: 8,
            scored_candidates: 3,
            accepted_candidates: 3,
            returned_candidates: 3,
            available_control_count: 7,
            locked_targets_skipped: 0,
            rejections: BTreeMap::from([(FoundryCandidateRejectionReason::TooSubtle, 5)]),
            duplicate_looking_rejections: 0,
            hidden_internal_rejections: 5,
            wrong_scope_rejections: 0,
            human_summary: String::new(),
        };

        assert_eq!(
            candidate_generation_human_summary(&diagnostics),
            "Generated 3 clear ideas. Rejected 0 that looked too similar. Rejected 5 because changes were hidden/internal. Rejected 0 because changes were wrong scope."
        );
    }

    #[test]
    fn visible_bridge_support_and_bracing_changes_are_accepted() {
        let parent = test_candidate_input("parent", [4.0, 1.0, 1.0], 0);
        let candidate = test_candidate_input("bridge", [4.4, 1.2, 1.4], u64::MAX);
        let document = roman_bridge::fixture_catalog().document;

        for control_id in ["support_rhythm", "bracing_style"] {
            let proposal = numeric_proposal(control_id, FoundryCandidateChangeKind::Provider);
            let evidence = CandidateLegibilityEvidence {
                candidate_id: control_id.to_owned(),
                parent: &parent,
                candidate: &candidate,
            };

            let metadata = variation_metadata_for_proposal(
                &document,
                &VariationIntent::whole_asset_shape(),
                &proposal,
                Some(&evidence),
            );

            assert!(
                matches!(
                    metadata.visible_delta.legibility_class,
                    CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
                ),
                "{control_id} should be accepted: {:?}",
                metadata.visible_delta
            );
        }
    }

    #[test]
    fn visible_lamp_shade_and_base_changes_are_accepted() {
        let parent = test_candidate_input("parent", [1.0, 2.0, 1.0], 0);
        let document = stylized_lamp::fixture_catalog().document;
        let cases = [
            (
                "shade_style",
                FoundryCandidateChangeKind::Choice,
                test_candidate_input("shade", [1.4, 2.2, 1.4], u64::MAX),
            ),
            (
                "base_weight",
                FoundryCandidateChangeKind::Numeric,
                test_candidate_input("base", [1.8, 1.8, 1.3], u64::MAX),
            ),
        ];

        for (control_id, kind, candidate) in cases {
            let proposal = numeric_proposal(control_id, kind);
            let evidence = CandidateLegibilityEvidence {
                candidate_id: control_id.to_owned(),
                parent: &parent,
                candidate: &candidate,
            };

            let metadata = variation_metadata_for_proposal(
                &document,
                &VariationIntent::whole_asset_shape(),
                &proposal,
                Some(&evidence),
            );

            assert!(
                matches!(
                    metadata.visible_delta.legibility_class,
                    CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
                ),
                "{control_id} should be accepted: {:?}",
                metadata.visible_delta
            );
        }
    }

    #[test]
    fn surface_cannot_pass_with_shape_only_evidence() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let candidate = test_candidate_input("surface", [1.8, 1.0, 0.8], u64::MAX);
        let proposal = numeric_proposal("body_proportions", FoundryCandidateChangeKind::Numeric);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "surface".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };

        let metadata = variation_metadata_for_proposal(
            &scifi_crate::fixture_catalog().document,
            &VariationIntent::whole_asset_surface(),
            &proposal,
            Some(&evidence),
        );

        assert_eq!(
            metadata.visible_delta.legibility_class,
            CandidateLegibilityClass::Unsupported
        );
    }

    #[test]
    fn shape_cannot_pass_with_surface_only_evidence() {
        let (class, reasons) = classify_candidate_delta(
            &VariationIntent::whole_asset_shape(),
            0.0,
            0.0,
            0.0,
            0.0,
            0.9,
            0.0,
            0.0,
            &[],
        );

        assert_eq!(class, CandidateLegibilityClass::DuplicateLooking);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn endpoint_visibility_report_generated_for_scifi_crate() {
        let fixture = scifi_crate::fixture_catalog();
        let report =
            generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
                .expect("endpoint visibility report should generate");
        let rows = report
            .controls
            .iter()
            .map(|row| (row.control_id.as_str(), row.legibility_class))
            .collect::<BTreeMap<_, _>>();

        for control_id in [
            "body_proportions",
            "structural_heft",
            "panel_depth",
            "vent_density",
            "handle_style",
            "detail_density",
        ] {
            assert!(
                matches!(
                    rows.get(control_id),
                    Some(CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong)
                ),
                "{control_id} should be at least Clear: {:?}",
                report
                    .controls
                    .iter()
                    .find(|row| row.control_id == control_id)
            );
        }
        assert!(matches!(
            rows.get("edge_softness"),
            Some(
                CandidateLegibilityClass::SubtleButExplainable
                    | CandidateLegibilityClass::Clear
                    | CandidateLegibilityClass::Strong
            )
        ));
    }

    #[test]
    fn endpoint_visibility_reports_generated_for_bridge_and_lamp() {
        let bridge = roman_bridge::fixture_catalog();
        let bridge_report =
            generate_foundry_control_endpoint_visibility_report(&bridge.document, &bridge)
                .expect("bridge endpoint visibility report should generate");
        assert_endpoint_controls_clear(
            &bridge_report,
            &[
                "span_length",
                "deck_width",
                "structural_heft",
                "support_rhythm",
                "bracing_style",
                "railing",
                "edge_finish",
            ],
            &[],
        );

        let lamp = stylized_lamp::fixture_catalog();
        let lamp_report =
            generate_foundry_control_endpoint_visibility_report(&lamp.document, &lamp)
                .expect("lamp endpoint visibility report should generate");
        assert_endpoint_controls_clear(
            &lamp_report,
            &[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "joint_size",
                "shade_style",
                "shade_scale",
            ],
            &["edge_softness"],
        );
    }

    fn assert_endpoint_controls_clear(
        report: &FoundryControlEndpointVisibilityReport,
        major_controls: &[&str],
        subtle_allowed_controls: &[&str],
    ) {
        let rows = report
            .controls
            .iter()
            .map(|row| (row.control_id.as_str(), row.legibility_class))
            .collect::<BTreeMap<_, _>>();
        for control_id in major_controls {
            assert!(
                matches!(
                    rows.get(control_id),
                    Some(CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong)
                ),
                "{} {control_id} should be at least Clear: {:?}",
                report.profile_id,
                report
                    .controls
                    .iter()
                    .find(|row| row.control_id == *control_id)
            );
        }
        for control_id in subtle_allowed_controls {
            assert!(
                matches!(
                    rows.get(control_id),
                    Some(
                        CandidateLegibilityClass::SubtleButExplainable
                            | CandidateLegibilityClass::Clear
                            | CandidateLegibilityClass::Strong
                    )
                ),
                "{} {control_id} should be visible or explicitly subtle: {:?}",
                report.profile_id,
                report
                    .controls
                    .iter()
                    .find(|row| row.control_id == *control_id)
            );
        }
    }

    #[test]
    fn legibility_report_deterministic_across_repeated_runs() {
        let parent = test_candidate_input("parent", [1.0, 1.0, 1.0], 0);
        let candidate = test_candidate_input("body", [1.8, 1.0, 0.8], u64::MAX);
        let proposal = numeric_proposal("body_proportions", FoundryCandidateChangeKind::Numeric);
        let evidence = CandidateLegibilityEvidence {
            candidate_id: "body".to_owned(),
            parent: &parent,
            candidate: &candidate,
        };
        let document = scifi_crate::fixture_catalog().document;

        let first = variation_metadata_for_proposal(
            &document,
            &VariationIntent::whole_asset_shape(),
            &proposal,
            Some(&evidence),
        );
        let second = variation_metadata_for_proposal(
            &document,
            &VariationIntent::whole_asset_shape(),
            &proposal,
            Some(&evidence),
        );

        assert_eq!(first.perceptual_report, second.perceptual_report);
    }

    #[test]
    fn endpoint_reports_are_deterministic_across_repeated_runs() {
        let fixture = stylized_lamp::fixture_catalog();

        let first =
            generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
                .expect("endpoint visibility report should generate");
        let second =
            generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
                .expect("endpoint visibility report should generate");

        assert_eq!(first, second);
    }

    #[test]
    fn explore_selection_refuses_one_control_provider_role_fallback() {
        let opportunities = vec![
            provider_opportunity("handle_provider", "handle"),
            role_presence_opportunity("has_handle", "handle"),
        ];
        let mut rng = ChaCha8Rng::seed_from_u64(7);

        let selected =
            select_opportunities(&opportunities, FoundryCandidateMode::Explore, 0, &mut rng);

        assert!(selected.is_empty());
    }

    #[test]
    fn explore_selection_counts_only_one_provider_or_presence_control() {
        let opportunities = vec![
            provider_opportunity("handle_provider", "handle"),
            role_presence_opportunity("has_handle", "handle"),
            scalar_opportunity("body_width"),
        ];
        let mut rng = ChaCha8Rng::seed_from_u64(11);

        let selected =
            select_opportunities(&opportunities, FoundryCandidateMode::Explore, 0, &mut rng);

        assert_eq!(selected.len(), 2);
        assert_eq!(
            selected
                .iter()
                .filter(|opportunity| opportunity.is_provider_or_role_presence())
                .count(),
            1
        );
    }

    #[test]
    fn conformance_compilation_error_maps_to_conformance_rejection() {
        let conformance_error =
            FoundryCompilationError::FinalConformanceRejected(FamilyConformanceReport::default());
        assert_eq!(
            compile_error_rejection_reason(&conformance_error),
            FoundryCandidateRejectionReason::ConformanceRejected
        );

        let compile_error = FoundryCompilationError::UnknownControl {
            control_id: "missing".to_owned(),
        };
        assert_eq!(
            compile_error_rejection_reason(&compile_error),
            FoundryCandidateRejectionReason::CompileRejected
        );
    }

    fn scalar_opportunity(control_id: &str) -> ControlOpportunity {
        ControlOpportunity {
            control_id: control_id.to_owned(),
            label: control_id.to_owned(),
            current: ControlValue::Scalar(0.5),
            domain: FeasibleControlDomain {
                continuous_intervals: vec![ClosedInterval {
                    minimum: 0.0,
                    maximum: 1.0,
                }],
                discrete_values: Vec::new(),
                unavailable_options: BTreeMap::new(),
                certification: DomainCertification::CertifiedContinuous,
            },
            kind: ControlOpportunityKind::Scalar,
            class: ControlClass::Silhouette,
            topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        }
    }

    fn provider_opportunity(control_id: &str, role: &str) -> ControlOpportunity {
        ControlOpportunity {
            control_id: control_id.to_owned(),
            label: control_id.to_owned(),
            current: ControlValue::Provider("standard".to_owned()),
            domain: FeasibleControlDomain {
                continuous_intervals: Vec::new(),
                discrete_values: vec![
                    ControlValue::Provider("standard".to_owned()),
                    ControlValue::Provider("wide".to_owned()),
                ],
                unavailable_options: BTreeMap::new(),
                certification: DomainCertification::DiscreteSamples,
            },
            kind: ControlOpportunityKind::Provider {
                role: role.to_owned(),
            },
            class: ControlClass::Structure,
            topology_behavior: ControlTopologyBehavior::TopologyChanging,
        }
    }

    fn role_presence_opportunity(control_id: &str, role: &str) -> ControlOpportunity {
        ControlOpportunity {
            control_id: control_id.to_owned(),
            label: control_id.to_owned(),
            current: ControlValue::Toggle(true),
            domain: FeasibleControlDomain {
                continuous_intervals: Vec::new(),
                discrete_values: vec![ControlValue::Toggle(false), ControlValue::Toggle(true)],
                unavailable_options: BTreeMap::new(),
                certification: DomainCertification::DiscreteSamples,
            },
            kind: ControlOpportunityKind::RolePresence {
                role: role.to_owned(),
            },
            class: ControlClass::Structure,
            topology_behavior: ControlTopologyBehavior::TopologyChanging,
        }
    }
}
