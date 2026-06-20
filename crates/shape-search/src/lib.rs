#![forbid(unsafe_code)]

//! Deterministic candidate search.

use std::collections::{BTreeMap, BTreeSet};

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shape_core::{
    Aabb, CandidateId, CoreError, EditProgram, NodeId, NodeKind, ParamDescriptor, ParamGroup,
    ParamPath, PrimitiveKind, Scalar, SetScalarEdit, ShapeDocument, ShapeNode, Transform3,
};
use thiserror::Error;

const MAX_DESCRIPTOR_RESOLUTION: usize = 24;
const MAX_DESCRIPTOR_SAMPLES: usize = 262_144;
const DOMAIN_PADDING_FRACTION: Scalar = 0.35;
const DOMAIN_ESCAPE_FRACTION: Scalar = 0.15;
const MIN_OCCUPANCY_FRACTION: Scalar = 0.001;
const MAX_OCCUPANCY_FRACTION: Scalar = 0.999;
const FALLBACK_PASS_COUNT: usize = 2;
const FALLBACK_PROPOSALS_PER_RESULT: usize = 16;
const MAX_FALLBACK_PROPOSALS: usize = 512;
const SCALE_EPSILON: Scalar = 1.0e-4;
const VALUE_EPSILON: Scalar = 1.0e-5;
const LOCK_NEIGHBOR_PENALTY: Scalar = 0.18;
const BOUNDARY_PENALTY_ZONE: Scalar = 0.045;

type Point3 = [Scalar; 3];

macro_rules! vec3_value {
    ($x:expr, $y:expr, $z:expr) => {{
        let mut value = Transform3::default().translation;
        value.x = $x;
        value.y = $y;
        value.z = $z;
        value
    }};
}

macro_rules! shape_vec3_is_finite {
    ($value:expr) => {
        $value.x.is_finite() && $value.y.is_finite() && $value.z.is_finite()
    };
}

macro_rules! point_from_shape_vec3 {
    ($value:expr) => {
        [$value.x, $value.y, $value.z]
    };
}

/// Exploration distance.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorationMode {
    /// Local changes around the current model.
    Refine,
    /// Broader changes for directional discovery.
    Explore,
}

/// Target affected by mutation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetScope {
    /// Only the selected node.
    Selected,
    /// Selected node and descendants.
    Subtree,
    /// Entire document.
    WholeModel,
}

/// Candidate generation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Deterministic seed.
    pub seed: u64,
    /// Number of raw proposals.
    pub proposal_count: usize,
    /// Number of final candidates.
    pub result_count: usize,
    /// Descriptor grid resolution.
    pub descriptor_resolution: usize,
    /// Selected node, if any.
    pub selected_node: Option<NodeId>,
    /// Target scope.
    pub target_scope: TargetScope,
    /// Enabled parameter groups.
    pub enabled_groups: BTreeSet<ParamGroup>,
    /// Exploration mode.
    pub mode: ExplorationMode,
}

/// Coarse geometric descriptor for diversity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeDescriptor {
    /// Packed occupancy words followed by normalized metrics.
    pub values: Vec<f32>,
}

/// Generated candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Stable ID within a generation.
    pub id: CandidateId,
    /// Candidate document.
    pub document: ShapeDocument,
    /// Edit that produced the candidate.
    pub edit: EditProgram,
    /// Coarse descriptor.
    pub descriptor: ShapeDescriptor,
    /// Distance from parent descriptor.
    pub distance_from_parent: f32,
}

/// Candidate generation result with diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchOutput {
    /// Final candidate documents in stable slot order.
    pub candidates: Vec<Candidate>,
    /// Quality gates, rejection counts, and accepted edit summaries.
    pub diagnostics: SearchDiagnostics,
}

/// Search quality diagnostics for one generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchDiagnostics {
    /// Requested raw proposals from the user request.
    pub requested_proposals: usize,
    /// Requested final candidates from the user request.
    pub requested_candidates: usize,
    /// Total proposal attempts, including fallback passes.
    pub attempted_proposals: usize,
    /// Proposals that passed edit, validation, descriptor, and distance gates.
    pub valid_proposals: usize,
    /// Final candidates returned to the caller.
    pub candidates_returned: usize,
    /// Number of fallback passes that were actually run.
    pub fallback_passes: usize,
    /// Mutable parameters available after scope, group, and lock filters.
    pub mutable_parameter_count: usize,
    /// Combined rejection counters across all passes.
    pub rejections: BTreeMap<SearchRejectionReason, usize>,
    /// Per-pass quality thresholds and rejection counters.
    pub passes: Vec<SearchPassDiagnostics>,
    /// Aggregate accepted-candidate parameter changes.
    pub parameter_changes: Vec<ParameterChangeSummary>,
    /// Smallest returned parent distance.
    pub minimum_parent_distance: Scalar,
    /// Mean returned parent distance.
    pub mean_parent_distance: Scalar,
    /// Smallest returned changed-parameter distance.
    pub minimum_parameter_distance: Scalar,
    /// Smallest returned visual descriptor distance.
    pub minimum_visual_distance: Scalar,
    /// Smallest returned occupancy-bit distance.
    pub minimum_occupancy_distance: Scalar,
    /// Mean preservation penalty applied to returned candidates.
    pub mean_preservation_penalty: Scalar,
}

/// Diagnostics for one deterministic proposal pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchPassDiagnostics {
    /// Zero-based pass index. Pass zero is the requested proposal batch.
    pub pass_index: usize,
    /// Deterministic global proposal index offset for this pass.
    pub proposal_offset: usize,
    /// Proposals requested in this pass.
    pub proposal_count: usize,
    /// Proposals attempted in this pass.
    pub attempted_proposals: usize,
    /// Proposals accepted into the candidate pool by this pass.
    pub accepted_proposals: usize,
    /// Quality thresholds used by this pass.
    pub thresholds: SearchThresholds,
    /// Rejection counters for this pass.
    pub rejections: BTreeMap<SearchRejectionReason, usize>,
}

/// Distance thresholds used by proposal gates and duplicate suppression.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchThresholds {
    /// Minimum normalized changed-parameter distance from the parent.
    pub minimum_parameter_distance: Scalar,
    /// Minimum visual descriptor distance from the parent.
    pub minimum_visual_distance: Scalar,
    /// Minimum occupancy-bit distance from the parent.
    pub minimum_occupancy_distance: Scalar,
    /// Minimum full parameter-vector distance between kept proposals.
    pub duplicate_parameter_distance: Scalar,
    /// Minimum occupancy-bit distance between kept proposals.
    pub duplicate_occupancy_distance: Scalar,
}

/// Why a proposal was rejected before final selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SearchRejectionReason {
    /// Mutating selected parameters produced no scalar operation after snapping.
    EmptyEdit,
    /// The edit could not be built from current scalar values.
    EditBuildFailed,
    /// Core rejected the edit application.
    CoreEditRejected,
    /// Core or search validation rejected the edited document.
    ValidationRejected,
    /// Candidate bounds escaped the comparison domain.
    BoundsEscaped,
    /// Candidate descriptor could not be produced.
    DescriptorRejected,
    /// Parameter-vector extraction failed.
    ParameterVectorUnavailable,
    /// Changed-parameter distance was too small.
    ParameterDistanceTooSmall,
    /// Visual descriptor distance was too small.
    VisualDistanceTooSmall,
    /// Occupancy-bit distance was too small.
    OccupancyDistanceTooSmall,
    /// Combined parent distance was non-finite or zero.
    NonFiniteDistance,
    /// Exact parameter vector was already present.
    DuplicateParameterVector,
    /// Parameter vector was too close to an already kept proposal.
    DuplicateParameterDistance,
    /// Occupancy bits were too close to an already kept proposal.
    DuplicateOccupancyDistance,
}

/// Aggregate change summary for one parameter across returned candidates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterChangeSummary {
    /// Edited parameter path.
    pub path: ParamPath,
    /// Parameter group.
    pub group: ParamGroup,
    /// Returned candidates that changed this parameter.
    pub changed_candidates: usize,
    /// Mean signed scalar delta.
    pub mean_delta: Scalar,
    /// Largest absolute scalar delta.
    pub max_abs_delta: Scalar,
    /// Mean absolute normalized delta.
    pub mean_abs_normalized_delta: Scalar,
    /// Largest absolute normalized delta.
    pub max_abs_normalized_delta: Scalar,
}

/// Search errors.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Request fields are inconsistent or unsafe.
    #[error("invalid search request: {0}")]
    InvalidRequest(&'static str),
    /// The source document cannot be searched.
    #[error("invalid shape document: {0}")]
    InvalidDocument(String),
    /// The request has no mutable parameters after filters and locks.
    #[error("no mutable parameters match the request")]
    NoMutableParameters,
    /// The current shape cannot produce a usable descriptor.
    #[error("invalid descriptor: {0}")]
    InvalidDescriptor(&'static str),
    /// Core edit application failed.
    #[error("core edit failed: {0}")]
    Core(#[from] CoreError),
}

/// Generate diverse candidate documents.
pub fn generate_candidates(
    document: &ShapeDocument,
    request: &SearchRequest,
) -> Result<Vec<Candidate>, SearchError> {
    Ok(generate_candidates_with_diagnostics(document, request)?.candidates)
}

/// Generate diverse candidate documents and return search diagnostics.
pub fn generate_candidates_with_diagnostics(
    document: &ShapeDocument,
    request: &SearchRequest,
) -> Result<SearchOutput, SearchError> {
    validate_request(request)?;
    validate_document_for_search(document)?;

    let parent_field = SearchField::compile(document)?;
    let parent_bounds = parent_field.bounds();
    if parent_bounds.is_empty() {
        return Err(SearchError::InvalidDescriptor("parent bounds are empty"));
    }
    let comparison_domain = comparison_domain(parent_bounds);
    let parent_descriptor = describe_field(
        &parent_field,
        comparison_domain,
        request.descriptor_resolution,
    )?;
    let mutable_params = mutable_parameters(document, request)?;
    if mutable_params.is_empty() {
        return Err(SearchError::NoMutableParameters);
    }

    let mut diagnostics = SearchDiagnostics::new(
        request.proposal_count,
        request.result_count,
        mutable_params.len(),
    );
    let mut proposal_pool = Vec::new();
    let passes = search_passes(request);

    for pass in passes {
        let attempts: Vec<ProposalAttempt> = (0..pass.proposal_count)
            .into_par_iter()
            .map(|local_index| {
                let proposal_index = pass.proposal_offset + local_index;
                build_proposal(
                    document,
                    request,
                    &mutable_params,
                    &parent_descriptor,
                    comparison_domain,
                    proposal_index,
                    proposal_seed(request.seed, proposal_index as u64),
                    &pass,
                )
            })
            .collect();

        let mut pass_diagnostics = SearchPassDiagnostics {
            pass_index: pass.pass_index,
            proposal_offset: pass.proposal_offset,
            proposal_count: pass.proposal_count,
            attempted_proposals: attempts.len(),
            accepted_proposals: 0,
            thresholds: pass.thresholds,
            rejections: BTreeMap::new(),
        };

        let mut proposals = Vec::new();
        for attempt in attempts {
            match attempt {
                Ok(proposal) => proposals.push(proposal),
                Err(reason) => increment_rejection(&mut pass_diagnostics.rejections, reason),
            }
        }

        proposals.sort_by(compare_proposals_for_mode(request.mode));
        let accepted = merge_unique_proposals(&mut proposal_pool, proposals, &pass);
        pass_diagnostics.accepted_proposals = accepted.accepted_count;
        merge_rejections(&mut pass_diagnostics.rejections, accepted.rejections);
        diagnostics.add_pass(pass_diagnostics);

        let selected_count =
            select_diverse(proposal_pool.clone(), request.result_count, request.mode).len();
        if selected_count >= request.result_count {
            break;
        }
    }

    proposal_pool.sort_by(compare_proposals_for_mode(request.mode));
    let selected = select_diverse(proposal_pool, request.result_count, request.mode);
    diagnostics.finalize(&selected, &mutable_params);

    Ok(SearchOutput {
        candidates: selected.into_iter().map(Proposal::into_candidate).collect(),
        diagnostics,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_proposal(
    document: &ShapeDocument,
    request: &SearchRequest,
    mutable_params: &[ParamDescriptor],
    parent_descriptor: &DescriptorMetrics,
    comparison_domain: Aabb,
    proposal_index: usize,
    proposal_seed: u64,
    pass: &SearchPass,
) -> ProposalAttempt {
    let mut rng = ChaCha8Rng::seed_from_u64(proposal_seed);
    let edit = build_edit(
        document,
        mutable_params,
        request.mode,
        proposal_index,
        proposal_seed,
        pass.mutation_scale,
        &mut rng,
    )
    .map_err(|_| SearchRejectionReason::EditBuildFailed)?;
    if edit.operations.is_empty() {
        return Err(SearchRejectionReason::EmptyEdit);
    }

    let candidate_document = shape_core::apply_edit(document, &edit)
        .map_err(|_| SearchRejectionReason::CoreEditRejected)?;
    validate_document_for_search(&candidate_document)
        .map_err(|_| SearchRejectionReason::ValidationRejected)?;

    let candidate_field = SearchField::compile(&candidate_document)
        .map_err(|_| SearchRejectionReason::ValidationRejected)?;
    let escape_domain = comparison_domain.expanded(
        comparison_domain.extent().max_element() * DOMAIN_ESCAPE_FRACTION + VALUE_EPSILON,
    );
    if !escape_domain.contains_aabb(&candidate_field.bounds()) {
        return Err(SearchRejectionReason::BoundsEscaped);
    }

    let descriptor = describe_field(
        &candidate_field,
        comparison_domain,
        request.descriptor_resolution,
    )
    .map_err(|_| SearchRejectionReason::DescriptorRejected)?;
    let candidate_vector = normalized_parameter_vector(&candidate_document, mutable_params)
        .map_err(|_| SearchRejectionReason::ParameterVectorUnavailable)?;
    let parameter_distance = edit_parameter_distance(&edit, mutable_params);
    if parameter_distance < pass.thresholds.minimum_parameter_distance {
        return Err(SearchRejectionReason::ParameterDistanceTooSmall);
    }
    let visual_distance = descriptor_distance_without_params(parent_descriptor, &descriptor);
    if visual_distance < pass.thresholds.minimum_visual_distance {
        return Err(SearchRejectionReason::VisualDistanceTooSmall);
    }
    let occupancy_distance = occupancy_hamming_distance(parent_descriptor, &descriptor);
    if occupancy_distance < pass.thresholds.minimum_occupancy_distance {
        return Err(SearchRejectionReason::OccupancyDistanceTooSmall);
    }
    let distance_from_parent =
        descriptor_distance(parent_descriptor, &descriptor, parameter_distance);
    if !distance_from_parent.is_finite() || distance_from_parent <= 0.0 {
        return Err(SearchRejectionReason::NonFiniteDistance);
    }

    let param_vector_key = exact_parameter_vector(&candidate_document, mutable_params)
        .map_err(|_| SearchRejectionReason::ParameterVectorUnavailable)?;
    let preservation_penalty = preservation_penalty(document, &edit, mutable_params);

    Ok(Proposal {
        proposal_index,
        candidate: Candidate {
            id: CandidateId(stable_candidate_id(proposal_seed, proposal_index as u64)),
            document: candidate_document,
            edit,
            descriptor: descriptor.to_public(),
            distance_from_parent,
        },
        metrics: descriptor,
        param_vector_key,
        param_vector: candidate_vector,
        parameter_distance,
        visual_distance,
        occupancy_distance,
        preservation_penalty,
    })
}

type ProposalAttempt = Result<Proposal, SearchRejectionReason>;

impl SearchDiagnostics {
    fn new(
        requested_proposals: usize,
        requested_candidates: usize,
        mutable_parameter_count: usize,
    ) -> Self {
        Self {
            requested_proposals,
            requested_candidates,
            attempted_proposals: 0,
            valid_proposals: 0,
            candidates_returned: 0,
            fallback_passes: 0,
            mutable_parameter_count,
            rejections: BTreeMap::new(),
            passes: Vec::new(),
            parameter_changes: Vec::new(),
            minimum_parent_distance: 0.0,
            mean_parent_distance: 0.0,
            minimum_parameter_distance: 0.0,
            minimum_visual_distance: 0.0,
            minimum_occupancy_distance: 0.0,
            mean_preservation_penalty: 0.0,
        }
    }

    fn add_pass(&mut self, pass: SearchPassDiagnostics) {
        self.attempted_proposals += pass.attempted_proposals;
        self.valid_proposals += pass.accepted_proposals;
        if pass.pass_index > 0 {
            self.fallback_passes += 1;
        }
        merge_rejections(&mut self.rejections, pass.rejections.clone());
        self.passes.push(pass);
    }

    fn finalize(&mut self, selected: &[Proposal], mutable_params: &[ParamDescriptor]) {
        self.candidates_returned = selected.len();
        self.parameter_changes = parameter_change_summaries(selected, mutable_params);
        if selected.is_empty() {
            return;
        }

        self.minimum_parent_distance = selected
            .iter()
            .map(|proposal| proposal.candidate.distance_from_parent)
            .fold(Scalar::INFINITY, Scalar::min);
        self.mean_parent_distance = selected
            .iter()
            .map(|proposal| proposal.candidate.distance_from_parent)
            .sum::<Scalar>()
            / selected.len() as Scalar;
        self.minimum_parameter_distance = selected
            .iter()
            .map(|proposal| proposal.parameter_distance)
            .fold(Scalar::INFINITY, Scalar::min);
        self.minimum_visual_distance = selected
            .iter()
            .map(|proposal| proposal.visual_distance)
            .fold(Scalar::INFINITY, Scalar::min);
        self.minimum_occupancy_distance = selected
            .iter()
            .map(|proposal| proposal.occupancy_distance)
            .fold(Scalar::INFINITY, Scalar::min);
        self.mean_preservation_penalty = selected
            .iter()
            .map(|proposal| proposal.preservation_penalty)
            .sum::<Scalar>()
            / selected.len() as Scalar;
    }
}

fn validate_request(request: &SearchRequest) -> Result<(), SearchError> {
    if request.proposal_count == 0 {
        return Err(SearchError::InvalidRequest(
            "proposal_count must be greater than zero",
        ));
    }
    if request.result_count == 0 {
        return Err(SearchError::InvalidRequest(
            "result_count must be greater than zero",
        ));
    }
    if request.descriptor_resolution < 2 {
        return Err(SearchError::InvalidRequest(
            "descriptor_resolution must be at least two",
        ));
    }
    if request.descriptor_resolution > MAX_DESCRIPTOR_RESOLUTION {
        return Err(SearchError::InvalidRequest(
            "descriptor_resolution exceeds the MVP safety limit",
        ));
    }
    let sample_count = request
        .descriptor_resolution
        .checked_mul(request.descriptor_resolution)
        .and_then(|value| value.checked_mul(request.descriptor_resolution))
        .ok_or(SearchError::InvalidRequest(
            "descriptor_resolution overflows sample count",
        ))?;
    if sample_count > MAX_DESCRIPTOR_SAMPLES {
        return Err(SearchError::InvalidRequest(
            "descriptor_resolution requests too many samples",
        ));
    }
    if request.enabled_groups.is_empty() {
        return Err(SearchError::NoMutableParameters);
    }
    Ok(())
}

fn validate_document_for_search(document: &ShapeDocument) -> Result<(), SearchError> {
    let report = shape_core::validate_document(document);
    if !report.is_valid() {
        return Err(SearchError::InvalidDocument(format!(
            "{} validation issue(s)",
            report.issues.len()
        )));
    }
    SearchField::compile(document).map(|_| ())
}

fn mutable_parameters(
    document: &ShapeDocument,
    request: &SearchRequest,
) -> Result<Vec<ParamDescriptor>, SearchError> {
    let target_nodes = target_nodes(document, request)?;
    let mut descriptors: Vec<ParamDescriptor> = shape_core::enumerate_parameters(document)
        .into_iter()
        .filter(|descriptor| target_nodes.contains(&descriptor.path.node))
        .filter(|descriptor| request.enabled_groups.contains(&descriptor.group))
        .filter(|descriptor| !document.locks.contains(&descriptor.path))
        .filter(descriptor_has_valid_range)
        .filter(|descriptor| shape_core::get_scalar(document, &descriptor.path).is_ok())
        .collect();
    descriptors.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(descriptors)
}

fn target_nodes(
    document: &ShapeDocument,
    request: &SearchRequest,
) -> Result<BTreeSet<NodeId>, SearchError> {
    match request.target_scope {
        TargetScope::WholeModel => Ok(document.nodes.keys().copied().collect()),
        TargetScope::Selected => {
            let selected = request.selected_node.unwrap_or(document.root);
            if !document.nodes.contains_key(&selected) {
                return Err(CoreError::UnknownNode(selected).into());
            }
            Ok(BTreeSet::from([selected]))
        }
        TargetScope::Subtree => {
            let selected = request.selected_node.unwrap_or(document.root);
            let mut nodes = BTreeSet::from([selected]);
            for descendant in shape_core::descendants_of(document, selected)? {
                nodes.insert(descendant);
            }
            Ok(nodes)
        }
    }
}

fn descriptor_has_valid_range(descriptor: &ParamDescriptor) -> bool {
    descriptor.minimum.is_finite()
        && descriptor.maximum.is_finite()
        && descriptor.mutation_sigma.is_finite()
        && descriptor.step.is_finite()
        && descriptor.minimum < descriptor.maximum
        && descriptor.mutation_sigma > 0.0
}

fn build_edit(
    document: &ShapeDocument,
    mutable_params: &[ParamDescriptor],
    mode: ExplorationMode,
    proposal_index: usize,
    proposal_seed: u64,
    mutation_scale: Scalar,
    rng: &mut ChaCha8Rng,
) -> Result<EditProgram, SearchError> {
    let selected_indices = choose_parameter_indices(mutable_params, mode, proposal_index, rng);
    let mut operations = Vec::new();
    for param_index in selected_indices {
        let descriptor = &mutable_params[param_index];
        let before = shape_core::get_scalar(document, &descriptor.path)?;
        let after = mutate_value(before, descriptor, mode, mutation_scale, rng);
        if (after - before).abs() > VALUE_EPSILON {
            operations.push(SetScalarEdit {
                path: descriptor.path.clone(),
                before,
                after,
            });
        }
    }
    operations.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(EditProgram {
        label: format!(
            "{} direction {}",
            match mode {
                ExplorationMode::Refine => "Refine",
                ExplorationMode::Explore => "Explore",
            },
            proposal_index + 1
        ),
        seed: proposal_seed,
        operations,
    })
}

fn choose_parameter_indices(
    mutable_params: &[ParamDescriptor],
    mode: ExplorationMode,
    proposal_index: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<usize> {
    let parameter_count = mutable_params.len();
    let target_count = match mode {
        ExplorationMode::Refine => {
            if parameter_count == 1 || rng.random_bool(0.70) {
                1
            } else {
                2
            }
        }
        ExplorationMode::Explore => {
            if parameter_count == 1 {
                1
            } else {
                let upper = parameter_count.min(5);
                rng.random_range(2..=upper)
            }
        }
    };
    let target_count = target_count.min(parameter_count);

    let grouped = grouped_parameter_indices(mutable_params);
    let groups: Vec<Vec<usize>> = grouped.into_values().collect();
    if groups.is_empty() {
        return Vec::new();
    }
    let start_group = proposal_index % groups.len();
    let mut indices = Vec::new();
    for offset in 0..groups.len() {
        if indices.len() == target_count {
            break;
        }
        let group_index = (start_group + offset) % groups.len();
        let group = &groups[group_index];
        let selected = group[rng.random_range(0..group.len())];
        if !indices.contains(&selected) {
            indices.push(selected);
        }
    }

    while indices.len() < target_count {
        let selected = rng.random_range(0..parameter_count);
        if !indices.contains(&selected) {
            indices.push(selected);
        }
    }

    indices.sort_unstable();
    indices
}

fn grouped_parameter_indices(
    mutable_params: &[ParamDescriptor],
) -> BTreeMap<ParamGroup, Vec<usize>> {
    let mut grouped: BTreeMap<ParamGroup, Vec<usize>> = BTreeMap::new();
    for (index, descriptor) in mutable_params.iter().enumerate() {
        grouped.entry(descriptor.group).or_default().push(index);
    }
    grouped
}

fn mutate_value(
    before: Scalar,
    descriptor: &ParamDescriptor,
    mode: ExplorationMode,
    mutation_scale: Scalar,
    rng: &mut ChaCha8Rng,
) -> Scalar {
    let normalized_before = normalize(before, descriptor);
    let range = descriptor.maximum - descriptor.minimum;
    let base_sigma = (descriptor.mutation_sigma / range).clamp(0.006, 0.09);
    let mode_scale = match mode {
        ExplorationMode::Refine => 0.70,
        ExplorationMode::Explore => 1.65,
    };
    let normal_delta: Scalar = StandardNormal.sample(rng);
    let mut delta = normal_delta * base_sigma * mode_scale * mutation_scale;
    let limits = mutation_limits(descriptor.group, mode, mutation_scale);
    let min_delta = limits.minimum;
    if delta.abs() < min_delta {
        let sign = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        delta = sign * min_delta;
    }
    delta = delta.clamp(-limits.maximum, limits.maximum);
    let normalized_after = (normalized_before + delta).clamp(0.0, 1.0);
    snap_to_step(
        descriptor.minimum + normalized_after * range,
        descriptor.minimum,
        descriptor.maximum,
        descriptor.step,
    )
}

#[derive(Debug, Copy, Clone)]
struct MutationLimits {
    minimum: Scalar,
    maximum: Scalar,
}

fn mutation_limits(
    group: ParamGroup,
    mode: ExplorationMode,
    mutation_scale: Scalar,
) -> MutationLimits {
    let (minimum, maximum) = match (group, mode) {
        (ParamGroup::Form, ExplorationMode::Refine) => (0.008, 0.060),
        (ParamGroup::Form, ExplorationMode::Explore) => (0.024, 0.180),
        (ParamGroup::Placement, ExplorationMode::Refine) => (0.006, 0.050),
        (ParamGroup::Placement, ExplorationMode::Explore) => (0.018, 0.140),
        (ParamGroup::Rotation, ExplorationMode::Refine) => (0.004, 0.025),
        (ParamGroup::Rotation, ExplorationMode::Explore) => (0.010, 0.075),
        (ParamGroup::Scale, ExplorationMode::Refine) => (0.005, 0.040),
        (ParamGroup::Scale, ExplorationMode::Explore) => (0.012, 0.100),
        (ParamGroup::Blend, ExplorationMode::Refine) => (0.006, 0.040),
        (ParamGroup::Blend, ExplorationMode::Explore) => (0.015, 0.120),
    };
    MutationLimits {
        minimum: minimum * mutation_scale,
        maximum: maximum * (0.75 + mutation_scale * 0.25),
    }
}

fn snap_to_step(value: Scalar, minimum: Scalar, maximum: Scalar, step: Scalar) -> Scalar {
    let clamped = value.clamp(minimum, maximum);
    if step <= 0.0 || !step.is_finite() {
        return clamped;
    }
    let steps = ((clamped - minimum) / step).round();
    (minimum + steps * step).clamp(minimum, maximum)
}

fn normalize(value: Scalar, descriptor: &ParamDescriptor) -> Scalar {
    ((value - descriptor.minimum) / (descriptor.maximum - descriptor.minimum)).clamp(0.0, 1.0)
}

fn normalized_parameter_vector(
    document: &ShapeDocument,
    descriptors: &[ParamDescriptor],
) -> Result<Vec<Scalar>, SearchError> {
    descriptors
        .iter()
        .map(|descriptor| {
            shape_core::get_scalar(document, &descriptor.path)
                .map(|value| normalize(value, descriptor))
                .map_err(SearchError::from)
        })
        .collect()
}

fn exact_parameter_vector(
    document: &ShapeDocument,
    descriptors: &[ParamDescriptor],
) -> Result<Vec<u32>, SearchError> {
    descriptors
        .iter()
        .map(|descriptor| {
            shape_core::get_scalar(document, &descriptor.path)
                .map(|value| canonical_bits(value).to_bits())
                .map_err(SearchError::from)
        })
        .collect()
}

fn canonical_bits(value: Scalar) -> Scalar {
    if value == 0.0 { 0.0 } else { value }
}

fn vector_distance(left: &[Scalar], right: &[Scalar]) -> Scalar {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let sum = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<Scalar>();
    (sum / left.len() as Scalar).sqrt()
}

fn vector_euclidean_distance(left: &[Scalar], right: &[Scalar]) -> Scalar {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<Scalar>()
        .sqrt()
}

fn edit_parameter_distance(edit: &EditProgram, descriptors: &[ParamDescriptor]) -> Scalar {
    let mut sum = 0.0;
    let mut count = 0_usize;
    for operation in &edit.operations {
        if let Some(descriptor) = descriptor_for_path(descriptors, &operation.path) {
            let delta = normalized_delta(operation.before, operation.after, descriptor).abs();
            sum += delta * delta;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        (sum / count as Scalar).sqrt()
    }
}

fn normalized_delta(before: Scalar, after: Scalar, descriptor: &ParamDescriptor) -> Scalar {
    (after - before) / (descriptor.maximum - descriptor.minimum)
}

fn descriptor_for_path<'a>(
    descriptors: &'a [ParamDescriptor],
    path: &ParamPath,
) -> Option<&'a ParamDescriptor> {
    descriptors
        .iter()
        .find(|descriptor| descriptor.path.eq(path))
}

fn preservation_penalty(
    document: &ShapeDocument,
    edit: &EditProgram,
    descriptors: &[ParamDescriptor],
) -> Scalar {
    if edit.operations.is_empty() {
        return 0.0;
    }

    let mut penalty = 0.0;
    for operation in &edit.operations {
        if document
            .locks
            .iter()
            .any(|locked| locked.node == operation.path.node)
        {
            penalty += LOCK_NEIGHBOR_PENALTY;
        }
        if let Some(descriptor) = descriptor_for_path(descriptors, &operation.path) {
            let normalized_after = normalize(operation.after, descriptor);
            let boundary_distance = normalized_after.min(1.0 - normalized_after);
            if boundary_distance < BOUNDARY_PENALTY_ZONE {
                penalty += (BOUNDARY_PENALTY_ZONE - boundary_distance) / BOUNDARY_PENALTY_ZONE;
            }
        }
    }
    (penalty / edit.operations.len() as Scalar).clamp(0.0, 1.0)
}

#[derive(Debug, Clone)]
struct ParameterChangeAccumulator {
    group: ParamGroup,
    count: usize,
    delta_sum: Scalar,
    max_abs_delta: Scalar,
    abs_normalized_sum: Scalar,
    max_abs_normalized_delta: Scalar,
}

fn parameter_change_summaries(
    selected: &[Proposal],
    descriptors: &[ParamDescriptor],
) -> Vec<ParameterChangeSummary> {
    let mut accumulators: BTreeMap<ParamPath, ParameterChangeAccumulator> = BTreeMap::new();
    for proposal in selected {
        for operation in &proposal.candidate.edit.operations {
            if let Some(descriptor) = descriptor_for_path(descriptors, &operation.path) {
                let delta = operation.after - operation.before;
                let abs_normalized_delta =
                    normalized_delta(operation.before, operation.after, descriptor).abs();
                let accumulator = accumulators.entry(operation.path.clone()).or_insert(
                    ParameterChangeAccumulator {
                        group: descriptor.group,
                        count: 0,
                        delta_sum: 0.0,
                        max_abs_delta: 0.0,
                        abs_normalized_sum: 0.0,
                        max_abs_normalized_delta: 0.0,
                    },
                );
                accumulator.count += 1;
                accumulator.delta_sum += delta;
                accumulator.max_abs_delta = accumulator.max_abs_delta.max(delta.abs());
                accumulator.abs_normalized_sum += abs_normalized_delta;
                accumulator.max_abs_normalized_delta = accumulator
                    .max_abs_normalized_delta
                    .max(abs_normalized_delta);
            }
        }
    }

    accumulators
        .into_iter()
        .map(|(path, accumulator)| ParameterChangeSummary {
            path,
            group: accumulator.group,
            changed_candidates: accumulator.count,
            mean_delta: accumulator.delta_sum / accumulator.count as Scalar,
            max_abs_delta: accumulator.max_abs_delta,
            mean_abs_normalized_delta: accumulator.abs_normalized_sum / accumulator.count as Scalar,
            max_abs_normalized_delta: accumulator.max_abs_normalized_delta,
        })
        .collect()
}

#[derive(Debug, Copy, Clone)]
struct SearchPass {
    pass_index: usize,
    proposal_offset: usize,
    proposal_count: usize,
    thresholds: SearchThresholds,
    mutation_scale: Scalar,
}

fn search_passes(request: &SearchRequest) -> Vec<SearchPass> {
    let base_thresholds = thresholds_for_mode(request.mode, request.descriptor_resolution);
    let fallback_proposal_count = request
        .proposal_count
        .max(
            request
                .result_count
                .saturating_mul(FALLBACK_PROPOSALS_PER_RESULT),
        )
        .min(MAX_FALLBACK_PROPOSALS);
    let mut passes = Vec::with_capacity(FALLBACK_PASS_COUNT + 1);
    let mut proposal_offset = 0;
    passes.push(SearchPass {
        pass_index: 0,
        proposal_offset,
        proposal_count: request.proposal_count,
        thresholds: base_thresholds,
        mutation_scale: 1.0,
    });
    proposal_offset += request.proposal_count;

    let fallback_profiles = [
        (relaxed_thresholds(base_thresholds, 0.55), 0.78),
        (final_fallback_thresholds(base_thresholds), 0.58),
    ];
    for (fallback_index, (thresholds, mutation_scale)) in fallback_profiles
        .into_iter()
        .take(FALLBACK_PASS_COUNT)
        .enumerate()
    {
        passes.push(SearchPass {
            pass_index: fallback_index + 1,
            proposal_offset,
            proposal_count: fallback_proposal_count,
            thresholds,
            mutation_scale,
        });
        proposal_offset += fallback_proposal_count;
    }
    passes
}

fn thresholds_for_mode(mode: ExplorationMode, descriptor_resolution: usize) -> SearchThresholds {
    let occupancy_unit = occupancy_unit_distance(descriptor_resolution);
    match mode {
        ExplorationMode::Refine => SearchThresholds {
            minimum_parameter_distance: 0.006,
            minimum_visual_distance: occupancy_unit * 0.35,
            minimum_occupancy_distance: occupancy_unit * 0.75,
            duplicate_parameter_distance: 0.010,
            duplicate_occupancy_distance: occupancy_unit * 0.75,
        },
        ExplorationMode::Explore => SearchThresholds {
            minimum_parameter_distance: 0.018,
            minimum_visual_distance: occupancy_unit * 0.75,
            minimum_occupancy_distance: occupancy_unit * 1.50,
            duplicate_parameter_distance: 0.024,
            duplicate_occupancy_distance: occupancy_unit * 1.25,
        },
    }
}

fn relaxed_thresholds(thresholds: SearchThresholds, scale: Scalar) -> SearchThresholds {
    SearchThresholds {
        minimum_parameter_distance: thresholds.minimum_parameter_distance * scale,
        minimum_visual_distance: thresholds.minimum_visual_distance * scale,
        minimum_occupancy_distance: thresholds.minimum_occupancy_distance * scale,
        duplicate_parameter_distance: thresholds.duplicate_parameter_distance * scale,
        duplicate_occupancy_distance: thresholds.duplicate_occupancy_distance * scale,
    }
}

fn final_fallback_thresholds(thresholds: SearchThresholds) -> SearchThresholds {
    SearchThresholds {
        minimum_parameter_distance: thresholds.minimum_parameter_distance * 0.25,
        minimum_visual_distance: 0.0,
        minimum_occupancy_distance: 0.0,
        duplicate_parameter_distance: thresholds.duplicate_parameter_distance * 0.25,
        duplicate_occupancy_distance: 0.0,
    }
}

fn occupancy_unit_distance(descriptor_resolution: usize) -> Scalar {
    let sample_count = descriptor_resolution
        .saturating_mul(descriptor_resolution)
        .saturating_mul(descriptor_resolution);
    let word_count = sample_count.div_ceil(16).max(1);
    1.0 / (word_count * 16) as Scalar
}

#[derive(Debug, Default)]
struct MergeResult {
    accepted_count: usize,
    rejections: BTreeMap<SearchRejectionReason, usize>,
}

fn merge_unique_proposals(
    proposal_pool: &mut Vec<Proposal>,
    proposals: Vec<Proposal>,
    pass: &SearchPass,
) -> MergeResult {
    let mut result = MergeResult::default();
    for proposal in proposals {
        if let Some(reason) = duplicate_reason(&proposal, proposal_pool, pass.thresholds) {
            increment_rejection(&mut result.rejections, reason);
        } else {
            result.accepted_count += 1;
            proposal_pool.push(proposal);
        }
    }
    result
}

fn duplicate_reason(
    proposal: &Proposal,
    existing: &[Proposal],
    thresholds: SearchThresholds,
) -> Option<SearchRejectionReason> {
    for other in existing {
        if proposal.param_vector_key == other.param_vector_key {
            return Some(SearchRejectionReason::DuplicateParameterVector);
        }
        if vector_euclidean_distance(&proposal.param_vector, &other.param_vector)
            < thresholds.duplicate_parameter_distance
        {
            return Some(SearchRejectionReason::DuplicateParameterDistance);
        }
        if occupancy_hamming_distance(&proposal.metrics, &other.metrics)
            < thresholds.duplicate_occupancy_distance
        {
            return Some(SearchRejectionReason::DuplicateOccupancyDistance);
        }
    }
    None
}

fn increment_rejection(
    rejections: &mut BTreeMap<SearchRejectionReason, usize>,
    reason: SearchRejectionReason,
) {
    *rejections.entry(reason).or_insert(0) += 1;
}

fn merge_rejections(
    target: &mut BTreeMap<SearchRejectionReason, usize>,
    source: BTreeMap<SearchRejectionReason, usize>,
) {
    for (reason, count) in source {
        *target.entry(reason).or_insert(0) += count;
    }
}

fn compare_proposals_for_mode(
    mode: ExplorationMode,
) -> impl Fn(&Proposal, &Proposal) -> std::cmp::Ordering {
    move |left, right| {
        let left_score = proposal_rank_score(left, mode);
        let right_score = proposal_rank_score(right, mode);
        let distance_order = match mode {
            ExplorationMode::Refine => left_score.total_cmp(&right_score),
            ExplorationMode::Explore => right_score.total_cmp(&left_score),
        };
        distance_order.then_with(|| left.proposal_index.cmp(&right.proposal_index))
    }
}

fn proposal_rank_score(proposal: &Proposal, mode: ExplorationMode) -> Scalar {
    let penalty = proposal.preservation_penalty;
    match mode {
        ExplorationMode::Refine => proposal.candidate.distance_from_parent + penalty * 0.25,
        ExplorationMode::Explore => proposal.candidate.distance_from_parent - penalty * 0.25,
    }
}

fn select_diverse(
    mut proposals: Vec<Proposal>,
    result_count: usize,
    mode: ExplorationMode,
) -> Vec<Proposal> {
    if proposals.len() <= result_count {
        return proposals;
    }

    let mut selected = vec![proposals.remove(0)];
    while selected.len() < result_count && !proposals.is_empty() {
        let mut best_index = 0;
        let mut best_score = Scalar::NEG_INFINITY;
        for (index, proposal) in proposals.iter().enumerate() {
            let nearest_selected = selected
                .iter()
                .map(|selected| proposal_diversity_distance(proposal, selected))
                .fold(Scalar::INFINITY, Scalar::min);
            let score = match mode {
                ExplorationMode::Refine => {
                    nearest_selected
                        - proposal.candidate.distance_from_parent * 0.20
                        - proposal.preservation_penalty * 0.25
                }
                ExplorationMode::Explore => {
                    nearest_selected + proposal.candidate.distance_from_parent * 0.30
                        - proposal.preservation_penalty * 0.25
                }
            };
            if score.total_cmp(&best_score).is_gt()
                || (score.total_cmp(&best_score).is_eq()
                    && proposal.proposal_index < proposals[best_index].proposal_index)
            {
                best_score = score;
                best_index = index;
            }
        }
        selected.push(proposals.remove(best_index));
    }
    selected
}

fn proposal_diversity_distance(left: &Proposal, right: &Proposal) -> Scalar {
    descriptor_distance_without_params(&left.metrics, &right.metrics) * 0.62
        + vector_distance(&left.param_vector, &right.param_vector) * 0.18
        + occupancy_hamming_distance(&left.metrics, &right.metrics) * 0.10
        + edit_path_distance(&left.candidate.edit, &right.candidate.edit) * 0.10
}

fn edit_path_distance(left: &EditProgram, right: &EditProgram) -> Scalar {
    let mut union_count = 0_usize;
    let mut shared_count = 0_usize;
    let mut right_paths: BTreeSet<ParamPath> = right
        .operations
        .iter()
        .map(|operation| operation.path.clone())
        .collect();
    for operation in &left.operations {
        union_count += 1;
        if right_paths.remove(&operation.path) {
            shared_count += 1;
        }
    }
    union_count += right_paths.len();
    if union_count == 0 {
        0.0
    } else {
        1.0 - shared_count as Scalar / union_count as Scalar
    }
}

#[derive(Debug, Clone)]
struct Proposal {
    proposal_index: usize,
    candidate: Candidate,
    metrics: DescriptorMetrics,
    param_vector_key: Vec<u32>,
    param_vector: Vec<Scalar>,
    parameter_distance: Scalar,
    visual_distance: Scalar,
    occupancy_distance: Scalar,
    preservation_penalty: Scalar,
}

impl Proposal {
    fn into_candidate(self) -> Candidate {
        self.candidate
    }
}

#[derive(Debug, Clone)]
struct DescriptorMetrics {
    packed_occupancy: Vec<u16>,
    occupied_fraction: Scalar,
    centroid: Point3,
    extent: Point3,
    center: Point3,
}

impl DescriptorMetrics {
    fn to_public(&self) -> ShapeDescriptor {
        let mut values = Vec::with_capacity(self.packed_occupancy.len() + 10);
        values.extend(self.packed_occupancy.iter().map(|word| f32::from(*word)));
        values.push(self.occupied_fraction);
        values.extend(self.centroid);
        values.extend(self.extent);
        values.extend(self.center);
        ShapeDescriptor { values }
    }
}

fn describe_field(
    field: &SearchField,
    domain: Aabb,
    resolution: usize,
) -> Result<DescriptorMetrics, SearchError> {
    let sample_count = resolution
        .checked_mul(resolution)
        .and_then(|value| value.checked_mul(resolution))
        .ok_or(SearchError::InvalidDescriptor(
            "descriptor sample count overflows",
        ))?;
    let word_count = sample_count.div_ceil(16);
    let mut packed_occupancy = vec![0_u16; word_count];
    let extent = domain.extent();
    if !shape_vec3_is_finite!(extent) || extent.min_element() <= 0.0 {
        return Err(SearchError::InvalidDescriptor(
            "comparison domain is invalid",
        ));
    }

    let denom = (resolution - 1) as Scalar;
    let mut occupied_count = 0_usize;
    let mut centroid_sum = [0.0; 3];
    let mut occupied_bounds = Aabb::empty();
    let mut sample_index = 0_usize;

    for z in 0..resolution {
        for y in 0..resolution {
            for x in 0..resolution {
                let point = [
                    domain.min.x + extent.x * x as Scalar / denom,
                    domain.min.y + extent.y * y as Scalar / denom,
                    domain.min.z + extent.z * z as Scalar / denom,
                ];
                let sample = field.sample(point);
                if !sample.is_finite() {
                    return Err(SearchError::InvalidDescriptor("field sample is non-finite"));
                }
                if sample <= 0.0 {
                    packed_occupancy[sample_index / 16] |= 1_u16 << (sample_index % 16);
                    occupied_count += 1;
                    centroid_sum[0] += point[0];
                    centroid_sum[1] += point[1];
                    centroid_sum[2] += point[2];
                    let point_bounds = aabb_from_points(point, point);
                    occupied_bounds = occupied_bounds.union(&point_bounds);
                }
                sample_index += 1;
            }
        }
    }

    let occupied_fraction = occupied_count as Scalar / sample_count as Scalar;
    if occupied_fraction <= MIN_OCCUPANCY_FRACTION {
        return Err(SearchError::InvalidDescriptor("occupancy is empty"));
    }
    if occupied_fraction >= MAX_OCCUPANCY_FRACTION {
        return Err(SearchError::InvalidDescriptor("occupancy is full"));
    }

    let occupied_count = occupied_count as Scalar;
    let centroid = normalize_point(
        [
            centroid_sum[0] / occupied_count,
            centroid_sum[1] / occupied_count,
            centroid_sum[2] / occupied_count,
        ],
        domain,
    );
    let occupied_extent = [
        occupied_bounds.extent().x / extent.x,
        occupied_bounds.extent().y / extent.y,
        occupied_bounds.extent().z / extent.z,
    ];
    let occupied_center = normalize_point(point_from_shape_vec3!(occupied_bounds.center()), domain);
    if !point_is_finite(centroid)
        || !point_is_finite(occupied_extent)
        || !point_is_finite(occupied_center)
    {
        return Err(SearchError::InvalidDescriptor(
            "descriptor metrics are non-finite",
        ));
    }

    Ok(DescriptorMetrics {
        packed_occupancy,
        occupied_fraction,
        centroid,
        extent: occupied_extent,
        center: occupied_center,
    })
}

fn descriptor_distance(
    parent: &DescriptorMetrics,
    candidate: &DescriptorMetrics,
    parameter_distance: Scalar,
) -> Scalar {
    descriptor_distance_without_params(parent, candidate) * 0.90 + parameter_distance * 0.10
}

fn descriptor_distance_without_params(
    left: &DescriptorMetrics,
    right: &DescriptorMetrics,
) -> Scalar {
    let hamming = occupancy_hamming_distance(left, right);
    let volume = (left.occupied_fraction - right.occupied_fraction).abs();
    let centroid = normalized_vec3_distance(left.centroid, right.centroid);
    let extent = normalized_vec3_distance(left.extent, right.extent);
    let center = normalized_vec3_distance(left.center, right.center);
    hamming * 0.45 + volume * 0.15 + centroid * 0.15 + extent * 0.15 + center * 0.10
}

fn occupancy_hamming_distance(left: &DescriptorMetrics, right: &DescriptorMetrics) -> Scalar {
    let max_words = left
        .packed_occupancy
        .len()
        .max(right.packed_occupancy.len());
    if max_words == 0 {
        return 0.0;
    }
    let mut changed_bits = 0_u32;
    for index in 0..max_words {
        let left_word = left
            .packed_occupancy
            .get(index)
            .copied()
            .unwrap_or_default();
        let right_word = right
            .packed_occupancy
            .get(index)
            .copied()
            .unwrap_or_default();
        changed_bits += (left_word ^ right_word).count_ones();
    }
    changed_bits as Scalar / (max_words * 16) as Scalar
}

fn normalized_vec3_distance(left: Point3, right: Point3) -> Scalar {
    distance3(left, right) / 3.0_f32.sqrt()
}

fn normalize_point(point: Point3, domain: Aabb) -> Point3 {
    let extent = domain.extent();
    [
        (point[0] - domain.min.x) / extent.x,
        (point[1] - domain.min.y) / extent.y,
        (point[2] - domain.min.z) / extent.z,
    ]
}

fn comparison_domain(bounds: Aabb) -> Aabb {
    let padding = bounds.extent().max_element() * DOMAIN_PADDING_FRACTION + VALUE_EPSILON;
    bounds.expanded(padding)
}

fn proposal_seed(seed: u64, proposal_index: u64) -> u64 {
    splitmix64(seed ^ proposal_index.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn stable_candidate_id(proposal_seed: u64, proposal_index: u64) -> u64 {
    splitmix64(proposal_seed ^ proposal_index.rotate_left(17))
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[derive(Debug, Clone)]
struct SearchField {
    document: ShapeDocument,
}

impl SearchField {
    fn compile(document: &ShapeDocument) -> Result<Self, SearchError> {
        validate_search_graph(document)?;
        let field = Self {
            document: document.clone(),
        };
        let bounds = field.node_bounds(document.root)?;
        if bounds.is_empty() {
            return Err(SearchError::InvalidDocument(
                "root bounds are empty".to_owned(),
            ));
        }
        Ok(field)
    }

    fn sample(&self, point: Point3) -> Scalar {
        self.sample_node(self.document.root, point)
    }

    fn bounds(&self) -> Aabb {
        self.node_bounds(self.document.root)
            .unwrap_or_else(|_| Aabb::empty())
    }

    fn sample_node(&self, node_id: NodeId, point: Point3) -> Scalar {
        let Some(node) = self.document.nodes.get(&node_id) else {
            return Scalar::INFINITY;
        };
        if !node.enabled {
            return Scalar::INFINITY;
        }
        let inverse = node.transform.matrix().inverse();
        let local_point = inverse.transform_point3(vec3_value!(point[0], point[1], point[2]));
        let local_point = point_from_shape_vec3!(local_point);
        let local_distance = match &node.kind {
            NodeKind::Primitive(kind) => sample_primitive(kind, local_point),
            NodeKind::Union { children } => self.sample_union(children, local_point),
            NodeKind::SmoothUnion {
                children,
                smoothness,
            } => self.sample_smooth_union(children, *smoothness, local_point),
            NodeKind::Difference { base, subtractors } => {
                let base_distance = self.sample_node(*base, local_point);
                let subtract_distance = self.sample_union(subtractors, local_point);
                base_distance.max(-subtract_distance)
            }
            NodeKind::Intersection { children } => self.sample_intersection(children, local_point),
        };
        local_distance * min_abs_scale(&node.transform)
    }

    fn sample_union(&self, children: &[NodeId], point: Point3) -> Scalar {
        children
            .iter()
            .map(|child| self.sample_node(*child, point))
            .fold(Scalar::INFINITY, Scalar::min)
    }

    fn sample_intersection(&self, children: &[NodeId], point: Point3) -> Scalar {
        children
            .iter()
            .map(|child| self.sample_node(*child, point))
            .fold(Scalar::NEG_INFINITY, Scalar::max)
    }

    fn sample_smooth_union(
        &self,
        children: &[NodeId],
        smoothness: Scalar,
        point: Point3,
    ) -> Scalar {
        let mut distances = children
            .iter()
            .map(|child| self.sample_node(*child, point))
            .filter(|distance| distance.is_finite());
        let Some(first) = distances.next() else {
            return Scalar::INFINITY;
        };
        distances.fold(first, |left, right| smooth_min(left, right, smoothness))
    }

    fn node_bounds(&self, node_id: NodeId) -> Result<Aabb, SearchError> {
        let mut visiting = BTreeSet::new();
        self.node_bounds_inner(node_id, &mut visiting)
    }

    fn node_bounds_inner(
        &self,
        node_id: NodeId,
        visiting: &mut BTreeSet<NodeId>,
    ) -> Result<Aabb, SearchError> {
        if !visiting.insert(node_id) {
            return Err(SearchError::InvalidDocument(
                "cycle in shape graph".to_owned(),
            ));
        }
        let node = self
            .document
            .nodes
            .get(&node_id)
            .ok_or(CoreError::UnknownNode(node_id))?;
        let bounds = if node.enabled {
            let local_bounds = match &node.kind {
                NodeKind::Primitive(kind) => primitive_bounds(kind),
                NodeKind::Union { children } => self.children_union_bounds(children, visiting)?,
                NodeKind::SmoothUnion {
                    children,
                    smoothness,
                } => self
                    .children_union_bounds(children, visiting)?
                    .expanded(*smoothness),
                NodeKind::Difference { base, .. } => self.node_bounds_inner(*base, visiting)?,
                NodeKind::Intersection { children } => {
                    self.children_intersection_bounds(children, visiting)?
                }
            };
            local_bounds.transformed(&node.transform)
        } else {
            Aabb::empty()
        };
        visiting.remove(&node_id);
        Ok(bounds)
    }

    fn children_union_bounds(
        &self,
        children: &[NodeId],
        visiting: &mut BTreeSet<NodeId>,
    ) -> Result<Aabb, SearchError> {
        let mut bounds = Aabb::empty();
        for child in children {
            bounds = bounds.union(&self.node_bounds_inner(*child, visiting)?);
        }
        Ok(bounds)
    }

    fn children_intersection_bounds(
        &self,
        children: &[NodeId],
        visiting: &mut BTreeSet<NodeId>,
    ) -> Result<Aabb, SearchError> {
        let mut iter = children.iter();
        let Some(first) = iter.next() else {
            return Ok(Aabb::empty());
        };
        let mut bounds = self.node_bounds_inner(*first, visiting)?;
        for child in iter {
            bounds = bounds.intersection(&self.node_bounds_inner(*child, visiting)?);
        }
        Ok(bounds)
    }
}

fn validate_search_graph(document: &ShapeDocument) -> Result<(), SearchError> {
    if !document.nodes.contains_key(&document.root) {
        return Err(SearchError::InvalidDocument(
            "root node is missing".to_owned(),
        ));
    }
    let mut references = BTreeMap::new();
    for (id, node) in &document.nodes {
        if node.id != *id {
            return Err(SearchError::InvalidDocument(
                "node ID does not match map key".to_owned(),
            ));
        }
        validate_node(node)?;
        references.insert(*id, referenced_nodes(&node.kind));
    }
    for (id, children) in &references {
        for child in children {
            if !document.nodes.contains_key(child) {
                return Err(SearchError::InvalidDocument(format!(
                    "node {} references missing node {}",
                    id.0, child.0
                )));
            }
        }
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    detect_cycles(document.root, &references, &mut visiting, &mut visited)
}

fn validate_node(node: &ShapeNode) -> Result<(), SearchError> {
    validate_transform(&node.transform)?;
    match &node.kind {
        NodeKind::Primitive(kind) => validate_primitive(kind),
        NodeKind::Union { children } | NodeKind::Intersection { children } => {
            validate_children(children)
        }
        NodeKind::SmoothUnion {
            children,
            smoothness,
        } => {
            validate_children(children)?;
            if !smoothness.is_finite() || *smoothness < 0.0 {
                return Err(SearchError::InvalidDocument(
                    "smooth union smoothness must be finite and non-negative".to_owned(),
                ));
            }
            Ok(())
        }
        NodeKind::Difference { base, subtractors } => {
            if subtractors.is_empty() {
                return Err(SearchError::InvalidDocument(
                    "difference must have at least one subtractor".to_owned(),
                ));
            }
            if subtractors.iter().any(|child| child == base) {
                return Err(SearchError::InvalidDocument(
                    "difference cannot subtract its base from itself".to_owned(),
                ));
            }
            Ok(())
        }
    }
}

fn validate_transform(transform: &Transform3) -> Result<(), SearchError> {
    if !shape_vec3_is_finite!(transform.translation)
        || !shape_vec3_is_finite!(transform.rotation_degrees)
        || !shape_vec3_is_finite!(transform.scale)
    {
        return Err(SearchError::InvalidDocument(
            "transform contains non-finite values".to_owned(),
        ));
    }
    if min_abs_scale(transform) <= SCALE_EPSILON {
        return Err(SearchError::InvalidDocument(
            "transform scale is too close to zero".to_owned(),
        ));
    }
    Ok(())
}

fn validate_primitive(kind: &PrimitiveKind) -> Result<(), SearchError> {
    match kind {
        PrimitiveKind::Sphere { radius } => validate_positive(*radius, "sphere radius"),
        PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        } => {
            if !shape_vec3_is_finite!(*half_extents) || half_extents.min_element() <= 0.0 {
                return Err(SearchError::InvalidDocument(
                    "rounded box extents must be positive".to_owned(),
                ));
            }
            if !roundness.is_finite() || *roundness < 0.0 || *roundness > half_extents.min_element()
            {
                return Err(SearchError::InvalidDocument(
                    "rounded box roundness is outside valid bounds".to_owned(),
                ));
            }
            Ok(())
        }
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => {
            validate_positive(*half_length, "capsule half length")?;
            validate_positive(*radius, "capsule radius")
        }
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            roundness,
        } => {
            validate_positive(*half_height, "cylinder half height")?;
            validate_positive(*radius, "cylinder radius")?;
            if !roundness.is_finite() || *roundness < 0.0 || *roundness > radius.min(*half_height) {
                return Err(SearchError::InvalidDocument(
                    "cylinder roundness is outside valid bounds".to_owned(),
                ));
            }
            Ok(())
        }
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => {
            validate_positive(*major_radius, "torus major radius")?;
            validate_positive(*minor_radius, "torus minor radius")?;
            if minor_radius >= major_radius {
                return Err(SearchError::InvalidDocument(
                    "torus minor radius must be smaller than major radius".to_owned(),
                ));
            }
            Ok(())
        }
    }
}

fn validate_positive(value: Scalar, name: &'static str) -> Result<(), SearchError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(SearchError::InvalidDocument(format!(
            "{name} must be finite and positive"
        )))
    }
}

fn validate_children(children: &[NodeId]) -> Result<(), SearchError> {
    if children.is_empty() {
        Err(SearchError::InvalidDocument(
            "combiner must have at least one child".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn detect_cycles(
    node: NodeId,
    references: &BTreeMap<NodeId, Vec<NodeId>>,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<(), SearchError> {
    if visited.contains(&node) {
        return Ok(());
    }
    if !visiting.insert(node) {
        return Err(SearchError::InvalidDocument(
            "cycle in shape graph".to_owned(),
        ));
    }
    if let Some(children) = references.get(&node) {
        for child in children {
            detect_cycles(*child, references, visiting, visited)?;
        }
    }
    visiting.remove(&node);
    visited.insert(node);
    Ok(())
}

fn referenced_nodes(kind: &NodeKind) -> Vec<NodeId> {
    match kind {
        NodeKind::Primitive(_) => Vec::new(),
        NodeKind::Union { children }
        | NodeKind::SmoothUnion { children, .. }
        | NodeKind::Intersection { children } => children.clone(),
        NodeKind::Difference { base, subtractors } => {
            let mut nodes = vec![*base];
            nodes.extend(subtractors.iter().copied());
            nodes
        }
    }
}

fn sample_primitive(kind: &PrimitiveKind, point: Point3) -> Scalar {
    match kind {
        PrimitiveKind::Sphere { radius } => length3(point) - *radius,
        PrimitiveKind::RoundedBox {
            half_extents,
            roundness,
        } => {
            let inner = [
                (half_extents.x - *roundness).max(0.0),
                (half_extents.y - *roundness).max(0.0),
                (half_extents.z - *roundness).max(0.0),
            ];
            let q = [
                point[0].abs() - inner[0],
                point[1].abs() - inner[1],
                point[2].abs() - inner[2],
            ];
            length3([q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)])
                + q[0].max(q[1]).max(q[2]).min(0.0)
                - *roundness
        }
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => {
            let closest_y = point[1].clamp(-*half_length, *half_length);
            length3([point[0], point[1] - closest_y, point[2]]) - *radius
        }
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            roundness,
        } => {
            let inner_radius = (*radius - *roundness).max(0.0);
            let inner_height = (*half_height - *roundness).max(0.0);
            let q = [
                length2(point[0], point[2]) - inner_radius,
                point[1].abs() - inner_height,
            ];
            length2(q[0].max(0.0), q[1].max(0.0)) + q[0].max(q[1]).min(0.0) - *roundness
        }
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => {
            let q = [length2(point[0], point[2]) - *major_radius, point[1]];
            length2(q[0], q[1]) - *minor_radius
        }
    }
}

fn primitive_bounds(kind: &PrimitiveKind) -> Aabb {
    match kind {
        PrimitiveKind::Sphere { radius } => {
            aabb_from_points([-*radius, -*radius, -*radius], [*radius, *radius, *radius])
        }
        PrimitiveKind::RoundedBox { half_extents, .. } => Aabb {
            min: -*half_extents,
            max: *half_extents,
        },
        PrimitiveKind::Capsule {
            half_length,
            radius,
        } => aabb_from_points(
            [-*radius, -*half_length - *radius, -*radius],
            [*radius, *half_length + *radius, *radius],
        ),
        PrimitiveKind::Cylinder {
            half_height,
            radius,
            ..
        } => aabb_from_points(
            [-*radius, -*half_height, -*radius],
            [*radius, *half_height, *radius],
        ),
        PrimitiveKind::Torus {
            major_radius,
            minor_radius,
        } => {
            let outer = *major_radius + *minor_radius;
            aabb_from_points(
                [-outer, -*minor_radius, -outer],
                [outer, *minor_radius, outer],
            )
        }
    }
}

fn smooth_min(left: Scalar, right: Scalar, smoothness: Scalar) -> Scalar {
    if smoothness <= 0.0 {
        return left.min(right);
    }
    let h = (0.5 + 0.5 * (right - left) / smoothness).clamp(0.0, 1.0);
    left * h + right * (1.0 - h) - smoothness * h * (1.0 - h)
}

fn min_abs_scale(transform: &Transform3) -> Scalar {
    transform.scale.abs().min_element()
}

fn aabb_from_points(min: Point3, max: Point3) -> Aabb {
    Aabb {
        min: vec3_value!(min[0], min[1], min[2]),
        max: vec3_value!(max[0], max[1], max[2]),
    }
}

fn point_is_finite(value: Point3) -> bool {
    value[0].is_finite() && value[1].is_finite() && value[2].is_finite()
}

fn distance3(left: Point3, right: Point3) -> Scalar {
    length3([left[0] - right[0], left[1] - right[1], left[2] - right[2]])
}

fn length3(value: Point3) -> Scalar {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn length2(x: Scalar, y: Scalar) -> Scalar {
    (x * x + y * y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_core::{ParamPath, ShapeNode, Transform3, validate_document};

    fn all_groups() -> BTreeSet<ParamGroup> {
        [
            ParamGroup::Form,
            ParamGroup::Placement,
            ParamGroup::Rotation,
            ParamGroup::Scale,
            ParamGroup::Blend,
        ]
        .into_iter()
        .collect()
    }

    fn only_groups(groups: &[ParamGroup]) -> BTreeSet<ParamGroup> {
        groups.iter().copied().collect()
    }

    fn primitive_node(id: u64, name: &str, kind: PrimitiveKind) -> ShapeNode {
        ShapeNode {
            id: NodeId(id),
            name: name.to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Primitive(kind),
        }
    }

    fn representative_document() -> ShapeDocument {
        let root = ShapeNode {
            id: NodeId(1),
            name: "Model".to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::SmoothUnion {
                children: vec![NodeId(2), NodeId(3), NodeId(4)],
                smoothness: 0.12,
            },
        };
        let mut document = ShapeDocument::new("representative", root);
        let mut left = primitive_node(2, "Left sphere", PrimitiveKind::Sphere { radius: 0.75 });
        left.transform.translation = vec3_value!(-0.55, 0.0, 0.0);
        let mut box_node = primitive_node(
            3,
            "Center box",
            PrimitiveKind::RoundedBox {
                half_extents: vec3_value!(0.45, 0.9, 0.35),
                roundness: 0.08,
            },
        );
        box_node.transform.translation = vec3_value!(0.35, 0.05, 0.0);
        let mut cap = primitive_node(
            4,
            "Top capsule",
            PrimitiveKind::Capsule {
                half_length: 0.55,
                radius: 0.22,
            },
        );
        cap.transform.translation = vec3_value!(0.0, 1.05, 0.0);
        document.nodes.insert(NodeId(2), left);
        document.nodes.insert(NodeId(3), box_node);
        document.nodes.insert(NodeId(4), cap);
        document.next_node_id = 5;
        document
    }

    fn scope_document() -> ShapeDocument {
        let root = ShapeNode {
            id: NodeId(1),
            name: "Root".to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Union {
                children: vec![NodeId(2), NodeId(3)],
            },
        };
        let mut document = ShapeDocument::new("scope", root);
        let mut sphere = primitive_node(2, "Sphere", PrimitiveKind::Sphere { radius: 0.6 });
        sphere.transform.translation = vec3_value!(-0.5, 0.0, 0.0);
        let mut cylinder = primitive_node(
            3,
            "Cylinder",
            PrimitiveKind::Cylinder {
                half_height: 0.6,
                radius: 0.3,
                roundness: 0.03,
            },
        );
        cylinder.transform.translation = vec3_value!(0.5, 0.0, 0.0);
        document.nodes.insert(NodeId(2), sphere);
        document.nodes.insert(NodeId(3), cylinder);
        document.next_node_id = 4;
        document
    }

    fn difference_fixture_document() -> ShapeDocument {
        let root = ShapeNode {
            id: NodeId(1),
            name: "Mixed preset".to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::SmoothUnion {
                children: vec![NodeId(2), NodeId(5)],
                smoothness: 0.1,
            },
        };
        let difference = ShapeNode {
            id: NodeId(2),
            name: "Cut block".to_owned(),
            tags: BTreeSet::new(),
            enabled: true,
            transform: Transform3::default(),
            kind: NodeKind::Difference {
                base: NodeId(3),
                subtractors: vec![NodeId(4)],
            },
        };
        let mut block = primitive_node(
            3,
            "Block",
            PrimitiveKind::RoundedBox {
                half_extents: vec3_value!(0.8, 0.45, 0.55),
                roundness: 0.08,
            },
        );
        block.transform.translation = vec3_value!(0.0, 0.1, 0.0);
        let mut cutter = primitive_node(
            4,
            "Cutter",
            PrimitiveKind::Cylinder {
                half_height: 0.8,
                radius: 0.2,
                roundness: 0.03,
            },
        );
        cutter.transform.rotation_degrees = vec3_value!(90.0, 0.0, 0.0);
        let mut torus = primitive_node(
            5,
            "Offset torus",
            PrimitiveKind::Torus {
                major_radius: 0.42,
                minor_radius: 0.07,
            },
        );
        torus.transform.translation = vec3_value!(0.95, 0.25, 0.0);
        let mut document = ShapeDocument::new("mixed preset", root);
        document.nodes.insert(NodeId(2), difference);
        document.nodes.insert(NodeId(3), block);
        document.nodes.insert(NodeId(4), cutter);
        document.nodes.insert(NodeId(5), torus);
        document.next_node_id = 6;
        document
    }

    fn preset_fixture_documents() -> Vec<(String, ShapeDocument)> {
        vec![
            ("representative".to_owned(), representative_document()),
            ("scope".to_owned(), scope_document()),
            ("difference".to_owned(), difference_fixture_document()),
        ]
    }

    fn request(seed: u64, mode: ExplorationMode) -> SearchRequest {
        SearchRequest {
            seed,
            proposal_count: 96,
            result_count: 6,
            descriptor_resolution: 7,
            selected_node: Some(NodeId(2)),
            target_scope: TargetScope::WholeModel,
            enabled_groups: all_groups(),
            mode,
        }
    }

    fn candidates(seed: u64, mode: ExplorationMode) -> Vec<Candidate> {
        generate_candidates(&representative_document(), &request(seed, mode))
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"))
    }

    fn output(seed: u64, mode: ExplorationMode) -> SearchOutput {
        generate_candidates_with_diagnostics(&representative_document(), &request(seed, mode))
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"))
    }

    #[test]
    fn same_request_and_seed_produce_identical_candidates_and_edits() {
        let first = candidates(100, ExplorationMode::Explore);
        let second = candidates(100, ExplorationMode::Explore);
        assert_eq!(first, second);
    }

    #[test]
    fn different_seeds_generally_differ() {
        let first = candidates(100, ExplorationMode::Explore);
        let second = candidates(101, ExplorationMode::Explore);
        assert_ne!(
            first
                .iter()
                .map(|candidate| &candidate.edit)
                .collect::<Vec<_>>(),
            second
                .iter()
                .map(|candidate| &candidate.edit)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn locked_values_never_change() {
        let mut document = representative_document();
        let locked = ParamPath {
            node: NodeId(2),
            key: "primitive.radius".to_owned(),
        };
        document.locks.insert(locked.clone());
        let mut search_request = request(20, ExplorationMode::Explore);
        search_request.target_scope = TargetScope::WholeModel;
        let locked_before = shape_core::get_scalar(&document, &locked)
            .unwrap_or_else(|error| panic!("locked parameter missing: {error}"));
        let results = generate_candidates(&document, &search_request)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(!results.is_empty());
        for candidate in results {
            assert!(
                candidate
                    .edit
                    .operations
                    .iter()
                    .all(|operation| operation.path != locked)
            );
            let locked_after = shape_core::get_scalar(&candidate.document, &locked)
                .unwrap_or_else(|error| panic!("locked parameter missing: {error}"));
            assert_eq!(locked_before, locked_after);
        }
    }

    #[test]
    fn group_filtering_limits_edited_parameters() {
        let document = representative_document();
        let mut search_request = request(30, ExplorationMode::Explore);
        search_request.enabled_groups = only_groups(&[ParamGroup::Placement]);
        let results = generate_candidates(&document, &search_request)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(!results.is_empty());
        for candidate in results {
            assert!(
                candidate
                    .edit
                    .operations
                    .iter()
                    .all(|operation| { operation.path.key.starts_with("transform.translation.") })
            );
        }
    }

    #[test]
    fn selected_subtree_and_whole_model_scopes_work() {
        let document = scope_document();

        let mut selected = request(40, ExplorationMode::Refine);
        selected.selected_node = Some(NodeId(2));
        selected.target_scope = TargetScope::Selected;
        selected.enabled_groups = only_groups(&[ParamGroup::Form]);
        let selected_results = generate_candidates(&document, &selected)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(selected_results.iter().all(|candidate| {
            candidate
                .edit
                .operations
                .iter()
                .all(|operation| operation.path.node == NodeId(2))
        }));

        let mut subtree = selected.clone();
        subtree.selected_node = Some(NodeId(1));
        subtree.target_scope = TargetScope::Subtree;
        let subtree_results = generate_candidates(&document, &subtree)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(subtree_results.iter().any(|candidate| {
            candidate
                .edit
                .operations
                .iter()
                .any(|operation| operation.path.node == NodeId(3))
        }));

        let mut whole = selected;
        whole.target_scope = TargetScope::WholeModel;
        let whole_results = generate_candidates(&document, &whole)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(whole_results.iter().any(|candidate| {
            candidate
                .edit
                .operations
                .iter()
                .any(|operation| operation.path.node == NodeId(3))
        }));
    }

    #[test]
    fn invalid_proposals_are_rejected_without_aborting_the_run() {
        let mut document = scope_document();
        let cylinder = document
            .nodes
            .get_mut(&NodeId(3))
            .unwrap_or_else(|| panic!("test document missing cylinder"));
        cylinder.kind = NodeKind::Primitive(PrimitiveKind::Cylinder {
            half_height: 0.12,
            radius: 0.14,
            roundness: 0.11,
        });
        let mut search_request = request(50, ExplorationMode::Explore);
        search_request.proposal_count = 160;
        search_request.target_scope = TargetScope::WholeModel;
        search_request.enabled_groups = only_groups(&[ParamGroup::Form]);
        let results = generate_candidates(&document, &search_request)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(!results.is_empty());
        assert!(results.len() <= search_request.result_count);
        for candidate in results {
            SearchField::compile(&candidate.document)
                .unwrap_or_else(|error| panic!("invalid candidate escaped: {error}"));
        }
    }

    #[test]
    fn candidates_are_not_duplicate_parameter_vectors() {
        let document = representative_document();
        let search_request = request(60, ExplorationMode::Explore);
        let params = mutable_parameters(&document, &search_request)
            .unwrap_or_else(|error| panic!("mutable parameter discovery failed: {error}"));
        let results = generate_candidates(&document, &search_request)
            .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        let mut seen = BTreeSet::new();
        for candidate in results {
            let vector = exact_parameter_vector(&candidate.document, &params)
                .unwrap_or_else(|error| panic!("parameter vector failed: {error}"));
            assert!(seen.insert(vector));
        }
    }

    #[test]
    fn diagnostics_report_rejections_and_parameter_changes() {
        let mut search_request = request(61, ExplorationMode::Explore);
        search_request.proposal_count = 160;
        let result =
            generate_candidates_with_diagnostics(&representative_document(), &search_request)
                .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert_eq!(
            result.candidates.len(),
            result.diagnostics.candidates_returned
        );
        assert!(result.diagnostics.attempted_proposals >= search_request.proposal_count);
        assert!(result.diagnostics.valid_proposals >= result.candidates.len());
        assert!(!result.diagnostics.passes.is_empty());
        assert!(!result.diagnostics.parameter_changes.is_empty());
        assert!(result.diagnostics.rejections.values().sum::<usize>() > 0);
        assert!(result.diagnostics.minimum_parent_distance > 0.0);
        assert!(result.diagnostics.minimum_visual_distance > 0.0);
        assert!(result.diagnostics.minimum_occupancy_distance > 0.0);
    }

    #[test]
    fn fallback_runs_when_initial_pass_has_too_few_survivors() {
        let mut search_request = request(62, ExplorationMode::Explore);
        search_request.proposal_count = 1;
        search_request.result_count = 4;
        let result =
            generate_candidates_with_diagnostics(&representative_document(), &search_request)
                .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        assert!(result.diagnostics.fallback_passes > 0);
        assert!(result.diagnostics.attempted_proposals > search_request.proposal_count);
        assert!(!result.candidates.is_empty());
    }

    #[test]
    fn duplicate_suppression_reports_parameter_or_occupancy_matches() {
        let mut search_request = request(63, ExplorationMode::Refine);
        search_request.proposal_count = 256;
        search_request.result_count = 5;
        search_request.descriptor_resolution = 7;
        let result =
            generate_candidates_with_diagnostics(&representative_document(), &search_request)
                .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
        let duplicate_rejections = [
            SearchRejectionReason::DuplicateParameterVector,
            SearchRejectionReason::DuplicateParameterDistance,
            SearchRejectionReason::DuplicateOccupancyDistance,
        ]
        .into_iter()
        .map(|reason| {
            result
                .diagnostics
                .rejections
                .get(&reason)
                .copied()
                .unwrap_or(0)
        })
        .sum::<usize>();
        assert!(duplicate_rejections > 0);
    }

    #[test]
    fn preset_results_are_stable_diverse_and_valid() {
        for (name, document) in preset_fixture_documents() {
            for mode in [ExplorationMode::Refine, ExplorationMode::Explore] {
                let search_request = SearchRequest {
                    seed: 200,
                    proposal_count: 144,
                    result_count: 5,
                    descriptor_resolution: 9,
                    selected_node: None,
                    target_scope: TargetScope::WholeModel,
                    enabled_groups: all_groups(),
                    mode,
                };
                let first = generate_candidates_with_diagnostics(&document, &search_request)
                    .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
                let second = generate_candidates_with_diagnostics(&document, &search_request)
                    .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
                assert_eq!(first.candidates, second.candidates, "{name}");
                assert_eq!(
                    first.candidates.len(),
                    search_request.result_count,
                    "{name}"
                );
                assert!(
                    first.diagnostics.parameter_changes.len() > 1,
                    "{name} should change more than one parameter across results"
                );
                for candidate in first.candidates {
                    assert!(
                        validate_document(&candidate.document).is_valid(),
                        "{name} produced an invalid result"
                    );
                }
            }
        }
    }

    #[test]
    fn explore_stays_broader_than_refine_across_presets() {
        for (name, document) in preset_fixture_documents() {
            let base_request = SearchRequest {
                seed: 210,
                proposal_count: 128,
                result_count: 5,
                descriptor_resolution: 9,
                selected_node: None,
                target_scope: TargetScope::WholeModel,
                enabled_groups: all_groups(),
                mode: ExplorationMode::Refine,
            };
            let refine = generate_candidates(&document, &base_request)
                .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
            let mut explore_request = base_request;
            explore_request.mode = ExplorationMode::Explore;
            let explore = generate_candidates(&document, &explore_request)
                .unwrap_or_else(|error| panic!("candidate generation failed: {error}"));
            assert!(
                mean_distance(&explore) > mean_distance(&refine),
                "{name} explore should stay broader than refine"
            );
        }
    }

    #[test]
    fn transform_and_blend_changes_stay_in_safe_ranges() {
        let result = output(64, ExplorationMode::Explore);
        for candidate in result.candidates {
            for operation in candidate.edit.operations {
                let delta = (operation.after - operation.before).abs();
                if operation.path.key.starts_with("transform.translation.") {
                    assert!(delta <= 1.42, "translation delta {delta}");
                } else if operation
                    .path
                    .key
                    .starts_with("transform.rotation_degrees.")
                {
                    assert!(delta <= 55.0, "rotation delta {delta}");
                } else if operation.path.key.starts_with("transform.scale.") {
                    assert!(delta <= 1.02, "scale delta {delta}");
                } else if operation.path.key == "csg.smoothness" {
                    assert!(delta <= 0.25, "blend delta {delta}");
                }
            }
        }
    }

    #[test]
    fn explore_has_greater_mean_parent_distance_than_refine() {
        let refine = candidates(70, ExplorationMode::Refine);
        let explore = candidates(70, ExplorationMode::Explore);
        let refine_mean = mean_distance(&refine);
        let explore_mean = mean_distance(&explore);
        assert!(
            explore_mean > refine_mean,
            "explore_mean={explore_mean} refine_mean={refine_mean}"
        );
    }

    #[test]
    fn result_ordering_is_independent_of_rayon_thread_count() {
        let document = representative_document();
        let search_request = request(80, ExplorationMode::Explore);
        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap_or_else(|error| panic!("failed to build one-thread pool: {error}"))
            .install(|| {
                generate_candidates(&document, &search_request)
                    .unwrap_or_else(|error| panic!("candidate generation failed: {error}"))
            });
        let four_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap_or_else(|error| panic!("failed to build four-thread pool: {error}"))
            .install(|| {
                generate_candidates(&document, &search_request)
                    .unwrap_or_else(|error| panic!("candidate generation failed: {error}"))
            });
        assert_eq!(one_thread, four_threads);
    }

    #[test]
    fn search_source_avoids_category_specific_terms() {
        let source = include_str!("lib.rs").to_ascii_lowercase();
        let banned_terms = [
            ["hu", "manoid"].concat(),
            ["he", "ad"].concat(),
            ["tor", "so"].concat(),
            ["ar", "m"].concat(),
            ["le", "g"].concat(),
        ];
        for term in banned_terms {
            assert!(
                !source.contains(&term),
                "source contains category-specific term {term}"
            );
        }
    }

    fn mean_distance(candidates: &[Candidate]) -> Scalar {
        candidates
            .iter()
            .map(|candidate| candidate.distance_from_parent)
            .sum::<Scalar>()
            / candidates.len().max(1) as Scalar
    }
}
