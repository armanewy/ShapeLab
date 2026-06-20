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
    PrimitiveKind, Scalar, SetScalarEdit, ShapeDocument, ShapeNode, Transform3,
};
use thiserror::Error;

const MAX_DESCRIPTOR_RESOLUTION: usize = 24;
const MAX_DESCRIPTOR_SAMPLES: usize = 262_144;
const DOMAIN_PADDING_FRACTION: Scalar = 0.35;
const DOMAIN_ESCAPE_FRACTION: Scalar = 0.15;
const MIN_OCCUPANCY_FRACTION: Scalar = 0.001;
const MAX_OCCUPANCY_FRACTION: Scalar = 0.999;
const MIN_EFFECTIVE_NORMALIZED_CHANGE: Scalar = 0.002;
const SCALE_EPSILON: Scalar = 1.0e-4;
const VALUE_EPSILON: Scalar = 1.0e-5;

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

    let proposal_seeds: Vec<u64> = (0..request.proposal_count)
        .map(|index| proposal_seed(request.seed, index as u64))
        .collect();
    let parent_vector = normalized_parameter_vector(document, &mutable_params)?;

    let mut proposals: Vec<Proposal> = proposal_seeds
        .par_iter()
        .enumerate()
        .filter_map(|(proposal_index, seed)| {
            build_proposal(
                document,
                request,
                &mutable_params,
                &parent_vector,
                &parent_descriptor,
                comparison_domain,
                proposal_index,
                *seed,
            )
        })
        .collect();

    proposals.sort_by(compare_proposals_for_mode(request.mode));
    proposals = remove_duplicate_parameter_vectors(proposals);

    Ok(
        select_diverse(proposals, request.result_count, request.mode)
            .into_iter()
            .map(Proposal::into_candidate)
            .collect(),
    )
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

#[allow(clippy::too_many_arguments)]
fn build_proposal(
    document: &ShapeDocument,
    request: &SearchRequest,
    mutable_params: &[ParamDescriptor],
    parent_vector: &[Scalar],
    parent_descriptor: &DescriptorMetrics,
    comparison_domain: Aabb,
    proposal_index: usize,
    proposal_seed: u64,
) -> Option<Proposal> {
    let mut rng = ChaCha8Rng::seed_from_u64(proposal_seed);
    let edit = build_edit(
        document,
        mutable_params,
        request.mode,
        proposal_index,
        proposal_seed,
        &mut rng,
    )
    .ok()?;
    if edit.operations.is_empty() {
        return None;
    }

    let candidate_document = shape_core::apply_edit(document, &edit).ok()?;
    validate_document_for_search(&candidate_document).ok()?;

    let candidate_field = SearchField::compile(&candidate_document).ok()?;
    let escape_domain = comparison_domain.expanded(
        comparison_domain.extent().max_element() * DOMAIN_ESCAPE_FRACTION + VALUE_EPSILON,
    );
    if !escape_domain.contains_aabb(&candidate_field.bounds()) {
        return None;
    }

    let descriptor = describe_field(
        &candidate_field,
        comparison_domain,
        request.descriptor_resolution,
    )
    .ok()?;
    let candidate_vector = normalized_parameter_vector(&candidate_document, mutable_params).ok()?;
    let parameter_distance = vector_distance(parent_vector, &candidate_vector);
    if parameter_distance < MIN_EFFECTIVE_NORMALIZED_CHANGE {
        return None;
    }
    let distance_from_parent =
        descriptor_distance(parent_descriptor, &descriptor, parameter_distance);
    if !distance_from_parent.is_finite() || distance_from_parent <= 0.0 {
        return None;
    }

    Some(Proposal {
        proposal_index,
        candidate: Candidate {
            id: CandidateId(stable_candidate_id(proposal_seed, proposal_index as u64)),
            document: candidate_document.clone(),
            edit,
            descriptor: descriptor.to_public(),
            distance_from_parent,
        },
        metrics: descriptor,
        param_vector_key: exact_parameter_vector(&candidate_document, mutable_params).ok()?,
    })
}

fn build_edit(
    document: &ShapeDocument,
    mutable_params: &[ParamDescriptor],
    mode: ExplorationMode,
    proposal_index: usize,
    proposal_seed: u64,
    rng: &mut ChaCha8Rng,
) -> Result<EditProgram, SearchError> {
    let selected_indices = choose_parameter_indices(mutable_params.len(), mode, rng);
    let mut operations = Vec::new();
    for param_index in selected_indices {
        let descriptor = &mutable_params[param_index];
        let before = shape_core::get_scalar(document, &descriptor.path)?;
        let after = mutate_value(before, descriptor, mode, rng);
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
    parameter_count: usize,
    mode: ExplorationMode,
    rng: &mut ChaCha8Rng,
) -> Vec<usize> {
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
    let mut indices: Vec<usize> = (0..parameter_count).collect();
    for index in 0..target_count {
        let swap_with = rng.random_range(index..parameter_count);
        indices.swap(index, swap_with);
    }
    indices.truncate(target_count);
    indices.sort_unstable();
    indices
}

fn mutate_value(
    before: Scalar,
    descriptor: &ParamDescriptor,
    mode: ExplorationMode,
    rng: &mut ChaCha8Rng,
) -> Scalar {
    let normalized_before = normalize(before, descriptor);
    let range = descriptor.maximum - descriptor.minimum;
    let base_sigma = (descriptor.mutation_sigma / range).clamp(0.01, 0.35);
    let mode_scale = match mode {
        ExplorationMode::Refine => 0.75,
        ExplorationMode::Explore => 2.35,
    };
    let normal_delta: Scalar = StandardNormal.sample(rng);
    let mut delta = normal_delta * base_sigma * mode_scale;
    let min_delta = match mode {
        ExplorationMode::Refine => 0.01,
        ExplorationMode::Explore => 0.035,
    };
    if delta.abs() < min_delta {
        let sign = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        delta = sign * min_delta;
    }
    let normalized_after = (normalized_before + delta).clamp(0.0, 1.0);
    snap_to_step(
        descriptor.minimum + normalized_after * range,
        descriptor.minimum,
        descriptor.maximum,
        descriptor.step,
    )
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

fn remove_duplicate_parameter_vectors(proposals: Vec<Proposal>) -> Vec<Proposal> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for proposal in proposals {
        if seen.insert(proposal.param_vector_key.clone()) {
            unique.push(proposal);
        }
    }
    unique
}

fn compare_proposals_for_mode(
    mode: ExplorationMode,
) -> impl Fn(&Proposal, &Proposal) -> std::cmp::Ordering {
    move |left, right| {
        let distance_order = match mode {
            ExplorationMode::Refine => left
                .candidate
                .distance_from_parent
                .total_cmp(&right.candidate.distance_from_parent),
            ExplorationMode::Explore => right
                .candidate
                .distance_from_parent
                .total_cmp(&left.candidate.distance_from_parent),
        };
        distance_order.then_with(|| left.proposal_index.cmp(&right.proposal_index))
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
                .map(|selected| {
                    descriptor_distance_without_params(&proposal.metrics, &selected.metrics)
                })
                .fold(Scalar::INFINITY, Scalar::min);
            let score = match mode {
                ExplorationMode::Refine => {
                    nearest_selected - proposal.candidate.distance_from_parent * 0.20
                }
                ExplorationMode::Explore => {
                    nearest_selected + proposal.candidate.distance_from_parent * 0.30
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

#[derive(Debug, Clone)]
struct Proposal {
    proposal_index: usize,
    candidate: Candidate,
    metrics: DescriptorMetrics,
    param_vector_key: Vec<u32>,
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
    use shape_core::{ParamPath, ShapeNode, Transform3};

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
