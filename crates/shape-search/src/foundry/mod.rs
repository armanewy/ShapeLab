//! Control-space candidate generation for Foundry documents.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use shape_asset::{GeometrySource, ModelingOperationSpec};
use shape_core::{Aabb, Scalar, Transform3};
use shape_foundry::{
    ControlDeltaExplanation, ControlEvaluationContext, ControlKind, ControlTopologyBehavior,
    ControlValue, CustomizerControl, CustomizerProfile, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCandidateId, FoundryCatalogResolver, FoundryCommand,
    FoundryCompilationError, FoundryCompilationOutput, FoundryConformanceSummary, FoundryEdit,
    FoundryLockMode, FoundryLockTarget, apply_foundry_command, canonicalize_control_value,
    compile_foundry_document, default_control_value, effective_control_domain,
    explain_control_delta,
};
use thiserror::Error;

use crate::asset::scoring::{
    AssetCandidateInput, AssetDescriptor, AssetScoringReport, AssetSelectionPolicy,
    FIXED_CAMERA_COUNT, score_and_select_asset_candidates_with_policy,
};

const MIN_PROPOSALS: usize = 24;
const MAX_PROPOSALS: usize = 72;
const MAX_SURVIVORS: usize = 6;
const SILHOUETTE_MASK_SIDE: usize = 32;
const SILHOUETTE_MASK_WORDS: usize = (SILHOUETTE_MASK_SIDE * SILHOUETTE_MASK_SIDE) / 64;
const DEPTH_HISTOGRAM_BINS: usize = 8;
const EPSILON: Scalar = 1.0e-6;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCandidateRequest {
    /// Deterministic generation seed.
    pub seed: u64,
    /// Number of proposal programs to attempt. Must be between 24 and 72.
    pub proposal_count: usize,
    /// Maximum number of survivors to return. The generator caps this at six.
    pub result_count: usize,
    /// Search policy.
    pub mode: FoundryCandidateMode,
    /// Optional customizer strategy ID from the resolved profile.
    pub strategy_id: Option<String>,
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
}

/// Candidate generation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCandidateOutput {
    /// Surviving candidate plans in deterministic max-min diversity order.
    pub candidates: Vec<FoundryCandidatePlan>,
    /// Generation-level diagnostics.
    pub diagnostics: FoundryCandidateGenerationDiagnostics,
    /// Scoring, hard-rejection, duplicate-collapse, and diversity report.
    pub scoring_report: AssetScoringReport,
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
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
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
        strategy_controls.as_ref(),
        &mut locked_targets_skipped,
    );
    if opportunities.is_empty() {
        return Err(FoundryCandidateError::NoEditableControls);
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
    };
    let mut seen_programs = BTreeSet::new();
    let mut scoring_inputs = Vec::new();
    let mut accepted_plans = BTreeMap::<String, FoundryCandidatePlan>::new();

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
            proposal_seed,
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

        let plan = FoundryCandidatePlan {
            id: candidate_id,
            label: proposal.edit.label.clone(),
            edit: proposal.edit,
            document: candidate_document,
            changed_controls: proposal.changed_controls,
            diagnostics: FoundryCandidateDiagnostics {
                changes: proposal.changes,
            },
            descriptor: crate::asset::scoring::asset_descriptor(&scoring_input),
            recipe_fingerprint: scoring_input.recipe_fingerprint.clone(),
            conformance: compiled.conformance_summary,
        };
        scoring_inputs.push(scoring_input);
        accepted_plans.insert(plan.id.0.clone(), plan);
    }

    diagnostics.scored_candidates = scoring_inputs.len();
    diagnostics.accepted_candidates = accepted_plans.len();

    let scoring_policy = AssetSelectionPolicy {
        representative_count: request.result_count.min(MAX_SURVIVORS),
        duplicate_descriptor_distance: 0.005,
        ..AssetSelectionPolicy::default()
    };
    let scoring_report =
        score_and_select_asset_candidates_with_policy(&scoring_inputs, &scoring_policy);
    let candidates = scoring_report
        .representatives
        .iter()
        .filter_map(|candidate| accepted_plans.remove(&candidate.id))
        .collect::<Vec<_>>();
    diagnostics.returned_candidates = candidates.len();

    Ok(FoundryCandidateOutput {
        candidates,
        diagnostics,
        scoring_report,
    })
}

fn validate_request(request: &FoundryCandidateRequest) -> Result<(), FoundryCandidateError> {
    if request.proposal_count < MIN_PROPOSALS || request.proposal_count > MAX_PROPOSALS {
        return Err(FoundryCandidateError::InvalidRequest(
            "proposal_count must be between 24 and 72",
        ));
    }
    if request.result_count == 0 {
        return Err(FoundryCandidateError::InvalidRequest(
            "result_count must be greater than zero",
        ));
    }
    Ok(())
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
        if !mode_allows_control(mode, class, control.topology_behavior, &kind) {
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
            "profile",
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
) -> bool {
    match mode {
        FoundryCandidateMode::Refine => {
            topology_behavior == ControlTopologyBehavior::TopologyPreserving
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
    seed: u64,
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
    let labels = changes
        .iter()
        .map(|change| change.control_label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let label = format!(
        "{} candidate: {}",
        mode_label(mode),
        if labels.is_empty() {
            "Foundry controls"
        } else {
            labels.as_str()
        }
    );
    Some(CandidateEditProposal {
        edit: FoundryEdit {
            label: format!("{label} #{proposal_index:04}"),
            commands,
        },
        changed_controls,
        changes,
    })
    .map(|mut proposal| {
        proposal.edit.label.push_str(&format!(" ({seed:016x})"));
        proposal
    })
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

fn mode_label(mode: FoundryCandidateMode) -> &'static str {
    match mode {
        FoundryCandidateMode::Refine => "Refine",
        FoundryCandidateMode::Explore => "Explore",
        FoundryCandidateMode::Silhouette => "Silhouette",
        FoundryCandidateMode::Structure => "Structure",
        FoundryCandidateMode::Detail => "Detail",
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
