//! Non-AI asset candidate scoring and representative selection.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_core::{Aabb, Scalar};

/// Number of fixed-camera projected silhouette samples used by asset scoring.
pub const FIXED_CAMERA_COUNT: usize = 3;

const DEFAULT_REPRESENTATIVE_COUNT: usize = 6;
const DEFAULT_DUPLICATE_DISTANCE: Scalar = 0.035;
const EPSILON: Scalar = 1.0e-6;

/// Raw asset candidate facts consumed by scoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCandidateInput {
    /// Stable candidate identifier.
    pub id: String,
    /// Stable recipe identity used to collapse retessellated versions.
    pub recipe_fingerprint: String,
    /// Whether the source recipe validated.
    pub recipe_valid: bool,
    /// Whether compilation completed.
    pub compile_succeeded: bool,
    /// Whether this candidate requires closed manifold output.
    pub requires_closed_part: bool,
    /// Whether required closed output is manifold.
    pub closed_manifold: bool,
    /// Measured accidental intersection depth or ratio.
    pub accidental_intersection: Scalar,
    /// Maximum accepted accidental intersection.
    pub intersection_tolerance: Scalar,
    /// Number of attachments required by the recipe.
    pub required_attachment_count: usize,
    /// Number of required attachments actually present.
    pub attached_attachment_count: usize,
    /// Produced triangle count.
    pub triangle_count: usize,
    /// Maximum allowed triangle count.
    pub triangle_budget: usize,
    /// Whether generated geometry values are finite.
    pub geometry_finite: bool,
    /// Whether all exported geometry keeps semantic provenance.
    pub provenance_complete: bool,
    /// World-space bounds.
    pub world_bounds: Aabb,
    /// Approximate enclosed or occupied volume.
    pub volume_approximation: Scalar,
    /// Projected silhouette occupancy from the fixed camera set.
    pub silhouette_occupancy: [Scalar; FIXED_CAMERA_COUNT],
    /// Per-part approximate volumes.
    pub part_volumes: Vec<Scalar>,
    /// Authored or detected region count.
    pub region_count: usize,
    /// Authored or detected fine-detail count.
    pub detail_count: usize,
    /// Approximate bilateral/radial symmetry score in `[0, 1]`.
    pub symmetry_score: Scalar,
    /// Count of repeated visual elements.
    pub repeated_element_count: usize,
    /// Bevel radii measured in world units.
    pub bevel_radii: Vec<Scalar>,
    /// Topology complexity cost independent of tessellation density.
    pub topology_cost: Scalar,
    /// Near-coincident surface ratio in `[0, 1]`.
    pub near_coincident_surface_ratio: Scalar,
    /// Count of detached visual components beyond the primary component.
    pub detached_visual_components: usize,
}

impl AssetCandidateInput {
    /// Create a candidate input with conservative valid defaults.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        recipe_fingerprint: impl Into<String>,
        world_bounds: Aabb,
    ) -> Self {
        let volume = bounds_volume(world_bounds).max(EPSILON);
        let scale = world_bounds.extent().max_element().max(1.0);
        Self {
            id: id.into(),
            recipe_fingerprint: recipe_fingerprint.into(),
            recipe_valid: true,
            compile_succeeded: true,
            requires_closed_part: true,
            closed_manifold: true,
            accidental_intersection: 0.0,
            intersection_tolerance: 0.001,
            required_attachment_count: 0,
            attached_attachment_count: 0,
            triangle_count: 1_000,
            triangle_budget: 50_000,
            geometry_finite: true,
            provenance_complete: true,
            world_bounds,
            volume_approximation: volume * 0.55,
            silhouette_occupancy: [0.45, 0.45, 0.45],
            part_volumes: vec![volume * 0.55],
            region_count: 1,
            detail_count: 0,
            symmetry_score: 0.5,
            repeated_element_count: 0,
            bevel_radii: vec![scale * 0.02],
            topology_cost: 0.25,
            near_coincident_surface_ratio: 0.0,
            detached_visual_components: 0,
        }
    }
}

/// Hard rejection reasons applied before scoring or diversity selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AssetHardRejectionReason {
    /// The source recipe failed validation.
    InvalidRecipe,
    /// The candidate failed compilation.
    CompileFailure,
    /// A required closed part was not manifold.
    NonManifoldRequiredClosedPart,
    /// Accidental intersections exceeded tolerance.
    AccidentalIntersectionAboveTolerance,
    /// A required attachment was missing.
    MissingAttachment,
    /// The triangle budget was exceeded.
    TriangleBudgetExceeded,
    /// Geometry or descriptor values were non-finite.
    NonFiniteGeometry,
    /// Geometry provenance was incomplete.
    IncompleteProvenance,
}

/// Candidate rejected by a hard gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRejectedCandidate {
    /// Rejected candidate identifier.
    pub id: String,
    /// Reason the candidate cannot be considered.
    pub reason: AssetHardRejectionReason,
}

/// Descriptor fields kept separate for explainable candidate comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetDescriptor {
    /// World-space bounds.
    pub world_bounds: Aabb,
    /// Approximate enclosed or occupied volume.
    pub volume_approximation: Scalar,
    /// Projected silhouette occupancy from the fixed camera set.
    pub silhouette_occupancy: [Scalar; FIXED_CAMERA_COUNT],
    /// Number of visual parts.
    pub part_count: usize,
    /// Descending normalized proportions for the major parts.
    pub major_part_proportions: Vec<Scalar>,
    /// Combined region and fine-detail count.
    pub region_detail_count: usize,
    /// Approximate bilateral/radial symmetry score in `[0, 1]`.
    pub symmetry_score: Scalar,
    /// Count of repeated visual elements.
    pub repeated_element_count: usize,
    /// Bevel radii normalized by model size.
    pub bevel_to_size_ratios: Vec<Scalar>,
    /// Topology complexity cost independent of tessellation density.
    pub topology_cost: Scalar,
}

/// Quality penalty channels used by the selection policy.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetQualityPenalties {
    /// Penalty for tiny visual parts.
    pub tiny_parts: Scalar,
    /// Penalty for extreme thinness.
    pub extreme_thinness: Scalar,
    /// Penalty for near-coincident surfaces.
    pub near_coincident_surfaces: Scalar,
    /// Penalty for inconsistent bevel scale.
    pub inconsistent_bevel_scale: Scalar,
    /// Penalty for detached visual components.
    pub detached_visual_components: Scalar,
    /// Penalty for too much detail relative to primary forms.
    pub excessive_detail_relative_to_primary_forms: Scalar,
}

impl AssetQualityPenalties {
    /// Return the weighted penalty under `weights`.
    #[must_use]
    pub fn weighted_total(&self, weights: &AssetQualityWeights) -> Scalar {
        let terms = [
            (self.tiny_parts, weights.tiny_parts),
            (self.extreme_thinness, weights.extreme_thinness),
            (
                self.near_coincident_surfaces,
                weights.near_coincident_surfaces,
            ),
            (
                self.inconsistent_bevel_scale,
                weights.inconsistent_bevel_scale,
            ),
            (
                self.detached_visual_components,
                weights.detached_visual_components,
            ),
            (
                self.excessive_detail_relative_to_primary_forms,
                weights.excessive_detail_relative_to_primary_forms,
            ),
        ];
        weighted_average(&terms)
    }
}

/// Per-descriptor weights for diversity distance.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetDescriptorWeights {
    /// World bounds weight.
    pub world_bounds: Scalar,
    /// Volume approximation weight.
    pub volume_approximation: Scalar,
    /// Fixed-camera silhouette occupancy weight.
    pub silhouette_occupancy: Scalar,
    /// Part-count weight.
    pub part_count: Scalar,
    /// Major-part proportion vector weight.
    pub major_part_proportions: Scalar,
    /// Region/detail count weight.
    pub region_detail_count: Scalar,
    /// Symmetry-score weight.
    pub symmetry_score: Scalar,
    /// Repeated-element count weight.
    pub repeated_element_count: Scalar,
    /// Bevel-to-size ratio weight.
    pub bevel_to_size_ratios: Scalar,
    /// Topology-cost weight.
    pub topology_cost: Scalar,
}

impl Default for AssetDescriptorWeights {
    fn default() -> Self {
        Self {
            world_bounds: 0.85,
            volume_approximation: 0.7,
            silhouette_occupancy: 1.4,
            part_count: 0.65,
            major_part_proportions: 0.8,
            region_detail_count: 0.45,
            symmetry_score: 0.45,
            repeated_element_count: 0.35,
            bevel_to_size_ratios: 0.35,
            topology_cost: 0.45,
        }
    }
}

/// Per-quality-channel weights for selection penalties.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetQualityWeights {
    /// Tiny-parts weight.
    pub tiny_parts: Scalar,
    /// Extreme-thinness weight.
    pub extreme_thinness: Scalar,
    /// Near-coincident-surface weight.
    pub near_coincident_surfaces: Scalar,
    /// Inconsistent-bevel-scale weight.
    pub inconsistent_bevel_scale: Scalar,
    /// Detached-visual-component weight.
    pub detached_visual_components: Scalar,
    /// Excessive-detail weight.
    pub excessive_detail_relative_to_primary_forms: Scalar,
}

impl Default for AssetQualityWeights {
    fn default() -> Self {
        Self {
            tiny_parts: 0.9,
            extreme_thinness: 1.0,
            near_coincident_surfaces: 1.25,
            inconsistent_bevel_scale: 0.7,
            detached_visual_components: 1.35,
            excessive_detail_relative_to_primary_forms: 0.95,
        }
    }
}

/// Documented weighted policy for duplicate collapse and representative selection.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetSelectionPolicy {
    /// Maximum number of representatives to return.
    pub representative_count: usize,
    /// Descriptor distance below which candidates are duplicate visual options.
    pub duplicate_descriptor_distance: Scalar,
    /// Descriptor weights used by max-min diversity.
    pub descriptor_weights: AssetDescriptorWeights,
    /// Quality weights used for duplicate retention and selection penalties.
    pub quality_weights: AssetQualityWeights,
    /// Weight applied to the minimum descriptor distance during selection.
    pub diversity_weight: Scalar,
    /// Weight applied to quality penalty during selection.
    pub quality_penalty_weight: Scalar,
}

impl Default for AssetSelectionPolicy {
    fn default() -> Self {
        Self {
            representative_count: DEFAULT_REPRESENTATIVE_COUNT,
            duplicate_descriptor_distance: DEFAULT_DUPLICATE_DISTANCE,
            descriptor_weights: AssetDescriptorWeights::default(),
            quality_weights: AssetQualityWeights::default(),
            diversity_weight: 1.0,
            quality_penalty_weight: 0.14,
        }
    }
}

/// Candidate after hard rejections, descriptor extraction, and quality penalties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetScoredCandidate {
    /// Stable candidate identifier.
    pub id: String,
    /// Stable recipe identity used to collapse retessellated versions.
    pub recipe_fingerprint: String,
    /// Produced triangle count retained only for diagnostics and duplicate ties.
    pub triangle_count: usize,
    /// Descriptor metrics.
    pub descriptor: AssetDescriptor,
    /// Quality penalty channels.
    pub quality_penalties: AssetQualityPenalties,
    /// Weighted penalty under the policy.
    pub weighted_quality_penalty: Scalar,
}

/// Duplicate group collapsed before representative selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDuplicateGroup {
    /// Candidate retained for scoring.
    pub kept_id: String,
    /// All candidate IDs in the duplicate group, including `kept_id`.
    pub member_ids: Vec<String>,
}

/// Full scoring report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetScoringReport {
    /// Hard-rejected candidates.
    pub rejected_candidates: Vec<AssetRejectedCandidate>,
    /// All candidates that passed hard rejection.
    pub scored_candidates: Vec<AssetScoredCandidate>,
    /// Candidates remaining after duplicate collapse.
    pub unique_candidates: Vec<AssetScoredCandidate>,
    /// Duplicate groups collapsed before selection.
    pub duplicate_groups: Vec<AssetDuplicateGroup>,
    /// Deterministic representative candidates in selection order.
    pub representatives: Vec<AssetScoredCandidate>,
}

impl AssetScoringReport {
    /// Count hard rejections by reason.
    #[must_use]
    pub fn rejection_counts(&self) -> BTreeMap<AssetHardRejectionReason, usize> {
        let mut counts = BTreeMap::new();
        for rejected in &self.rejected_candidates {
            *counts.entry(rejected.reason).or_insert(0) += 1;
        }
        counts
    }
}

/// Score and select candidates with the default policy.
#[must_use]
pub fn score_and_select_asset_candidates(candidates: &[AssetCandidateInput]) -> AssetScoringReport {
    score_and_select_asset_candidates_with_policy(candidates, &AssetSelectionPolicy::default())
}

/// Score and select candidates with an explicit policy.
#[must_use]
pub fn score_and_select_asset_candidates_with_policy(
    candidates: &[AssetCandidateInput],
    policy: &AssetSelectionPolicy,
) -> AssetScoringReport {
    let policy = policy.sanitized();
    let mut rejected_candidates = Vec::new();
    let mut scored_candidates = Vec::new();

    for candidate in candidates {
        if let Some(reason) = hard_rejection_reason(candidate) {
            rejected_candidates.push(AssetRejectedCandidate {
                id: candidate.id.clone(),
                reason,
            });
            continue;
        }

        let descriptor = describe_candidate(candidate);
        let quality_penalties = quality_penalties(candidate, &descriptor);
        let weighted_quality_penalty = quality_penalties.weighted_total(&policy.quality_weights);
        scored_candidates.push(AssetScoredCandidate {
            id: candidate.id.clone(),
            recipe_fingerprint: candidate.recipe_fingerprint.clone(),
            triangle_count: candidate.triangle_count,
            descriptor,
            quality_penalties,
            weighted_quality_penalty,
        });
    }

    rejected_candidates.sort_by(|left, right| left.id.cmp(&right.id));
    scored_candidates.sort_by(compare_scored_candidates);
    let (unique_candidates, duplicate_groups) = collapse_duplicates(&scored_candidates, &policy);
    let representatives = select_representatives(&unique_candidates, &policy);

    AssetScoringReport {
        rejected_candidates,
        scored_candidates,
        unique_candidates,
        duplicate_groups,
        representatives,
    }
}

/// Return the first hard rejection reason for `candidate`, if any.
#[must_use]
pub fn hard_rejection_reason(candidate: &AssetCandidateInput) -> Option<AssetHardRejectionReason> {
    if !candidate.recipe_valid {
        return Some(AssetHardRejectionReason::InvalidRecipe);
    }
    if !candidate.compile_succeeded {
        return Some(AssetHardRejectionReason::CompileFailure);
    }
    if candidate.requires_closed_part && !candidate.closed_manifold {
        return Some(AssetHardRejectionReason::NonManifoldRequiredClosedPart);
    }
    if candidate.accidental_intersection > candidate.intersection_tolerance.max(0.0) {
        return Some(AssetHardRejectionReason::AccidentalIntersectionAboveTolerance);
    }
    if candidate.attached_attachment_count < candidate.required_attachment_count {
        return Some(AssetHardRejectionReason::MissingAttachment);
    }
    if candidate.triangle_count > candidate.triangle_budget {
        return Some(AssetHardRejectionReason::TriangleBudgetExceeded);
    }
    if !candidate_geometry_is_finite(candidate) {
        return Some(AssetHardRejectionReason::NonFiniteGeometry);
    }
    if !candidate.provenance_complete {
        return Some(AssetHardRejectionReason::IncompleteProvenance);
    }
    None
}

/// Return a descriptor for a hard-rejection-free candidate.
#[must_use]
pub fn asset_descriptor(candidate: &AssetCandidateInput) -> AssetDescriptor {
    describe_candidate(candidate)
}

/// Compute weighted descriptor distance under the supplied weights.
#[must_use]
pub fn asset_descriptor_distance(
    left: &AssetDescriptor,
    right: &AssetDescriptor,
    weights: &AssetDescriptorWeights,
) -> Scalar {
    let terms = [
        (
            bounds_distance(left.world_bounds, right.world_bounds),
            weights.world_bounds,
        ),
        (
            log_scalar_distance(left.volume_approximation, right.volume_approximation),
            weights.volume_approximation,
        ),
        (
            silhouette_distance(&left.silhouette_occupancy, &right.silhouette_occupancy),
            weights.silhouette_occupancy,
        ),
        (
            count_distance(left.part_count, right.part_count),
            weights.part_count,
        ),
        (
            vector_distance(&left.major_part_proportions, &right.major_part_proportions),
            weights.major_part_proportions,
        ),
        (
            count_distance(left.region_detail_count, right.region_detail_count),
            weights.region_detail_count,
        ),
        (
            (left.symmetry_score - right.symmetry_score).abs(),
            weights.symmetry_score,
        ),
        (
            count_distance(left.repeated_element_count, right.repeated_element_count),
            weights.repeated_element_count,
        ),
        (
            vector_distance(&left.bevel_to_size_ratios, &right.bevel_to_size_ratios),
            weights.bevel_to_size_ratios,
        ),
        (
            log_scalar_distance(left.topology_cost, right.topology_cost),
            weights.topology_cost,
        ),
    ];
    weighted_average(&terms)
}

impl AssetSelectionPolicy {
    fn sanitized(self) -> Self {
        Self {
            representative_count: self.representative_count,
            duplicate_descriptor_distance: finite_non_negative_or_default(
                self.duplicate_descriptor_distance,
                DEFAULT_DUPLICATE_DISTANCE,
            ),
            descriptor_weights: self.descriptor_weights.sanitized(),
            quality_weights: self.quality_weights.sanitized(),
            diversity_weight: finite_non_negative_or_default(self.diversity_weight, 1.0),
            quality_penalty_weight: finite_non_negative_or_default(
                self.quality_penalty_weight,
                0.14,
            ),
        }
    }
}

impl AssetDescriptorWeights {
    fn sanitized(self) -> Self {
        let defaults = Self::default();
        Self {
            world_bounds: finite_non_negative_or_default(self.world_bounds, defaults.world_bounds),
            volume_approximation: finite_non_negative_or_default(
                self.volume_approximation,
                defaults.volume_approximation,
            ),
            silhouette_occupancy: finite_non_negative_or_default(
                self.silhouette_occupancy,
                defaults.silhouette_occupancy,
            ),
            part_count: finite_non_negative_or_default(self.part_count, defaults.part_count),
            major_part_proportions: finite_non_negative_or_default(
                self.major_part_proportions,
                defaults.major_part_proportions,
            ),
            region_detail_count: finite_non_negative_or_default(
                self.region_detail_count,
                defaults.region_detail_count,
            ),
            symmetry_score: finite_non_negative_or_default(
                self.symmetry_score,
                defaults.symmetry_score,
            ),
            repeated_element_count: finite_non_negative_or_default(
                self.repeated_element_count,
                defaults.repeated_element_count,
            ),
            bevel_to_size_ratios: finite_non_negative_or_default(
                self.bevel_to_size_ratios,
                defaults.bevel_to_size_ratios,
            ),
            topology_cost: finite_non_negative_or_default(
                self.topology_cost,
                defaults.topology_cost,
            ),
        }
    }
}

impl AssetQualityWeights {
    fn sanitized(self) -> Self {
        let defaults = Self::default();
        Self {
            tiny_parts: finite_non_negative_or_default(self.tiny_parts, defaults.tiny_parts),
            extreme_thinness: finite_non_negative_or_default(
                self.extreme_thinness,
                defaults.extreme_thinness,
            ),
            near_coincident_surfaces: finite_non_negative_or_default(
                self.near_coincident_surfaces,
                defaults.near_coincident_surfaces,
            ),
            inconsistent_bevel_scale: finite_non_negative_or_default(
                self.inconsistent_bevel_scale,
                defaults.inconsistent_bevel_scale,
            ),
            detached_visual_components: finite_non_negative_or_default(
                self.detached_visual_components,
                defaults.detached_visual_components,
            ),
            excessive_detail_relative_to_primary_forms: finite_non_negative_or_default(
                self.excessive_detail_relative_to_primary_forms,
                defaults.excessive_detail_relative_to_primary_forms,
            ),
        }
    }
}

fn describe_candidate(candidate: &AssetCandidateInput) -> AssetDescriptor {
    let scale = candidate.world_bounds.extent().max_element().max(EPSILON);
    AssetDescriptor {
        world_bounds: candidate.world_bounds,
        volume_approximation: candidate.volume_approximation.max(0.0),
        silhouette_occupancy: candidate.silhouette_occupancy.map(clamp_unit),
        part_count: candidate.part_volumes.len(),
        major_part_proportions: major_part_proportions(&candidate.part_volumes),
        region_detail_count: candidate.region_count + candidate.detail_count,
        symmetry_score: clamp_unit(candidate.symmetry_score),
        repeated_element_count: candidate.repeated_element_count,
        bevel_to_size_ratios: bevel_to_size_ratios(&candidate.bevel_radii, scale),
        topology_cost: candidate.topology_cost.max(0.0),
    }
}

fn quality_penalties(
    candidate: &AssetCandidateInput,
    descriptor: &AssetDescriptor,
) -> AssetQualityPenalties {
    AssetQualityPenalties {
        tiny_parts: tiny_parts_penalty(&candidate.part_volumes),
        extreme_thinness: extreme_thinness_penalty(candidate.world_bounds),
        near_coincident_surfaces: clamp_unit(candidate.near_coincident_surface_ratio),
        inconsistent_bevel_scale: inconsistent_bevel_penalty(&descriptor.bevel_to_size_ratios),
        detached_visual_components: detached_components_penalty(
            candidate.detached_visual_components,
            descriptor.part_count,
        ),
        excessive_detail_relative_to_primary_forms: excessive_detail_penalty(descriptor),
    }
}

fn collapse_duplicates(
    candidates: &[AssetScoredCandidate],
    policy: &AssetSelectionPolicy,
) -> (Vec<AssetScoredCandidate>, Vec<AssetDuplicateGroup>) {
    let mut groups: Vec<WorkingDuplicateGroup> = Vec::new();
    for candidate in candidates {
        if let Some(group_index) = groups
            .iter()
            .position(|group| duplicate_match(candidate, &group.kept, policy))
        {
            let group = &mut groups[group_index];
            group.member_ids.push(candidate.id.clone());
            if duplicate_candidate_order(candidate, &group.kept) == Ordering::Less {
                group.kept = candidate.clone();
            }
        } else {
            groups.push(WorkingDuplicateGroup {
                kept: candidate.clone(),
                member_ids: vec![candidate.id.clone()],
            });
        }
    }

    let mut unique_candidates: Vec<_> = groups.iter().map(|group| group.kept.clone()).collect();
    unique_candidates.sort_by(compare_scored_candidates);
    let mut duplicate_groups: Vec<_> = groups
        .into_iter()
        .filter(|group| group.member_ids.len() > 1)
        .map(|mut group| {
            group.member_ids.sort();
            AssetDuplicateGroup {
                kept_id: group.kept.id,
                member_ids: group.member_ids,
            }
        })
        .collect();
    duplicate_groups.sort_by(|left, right| left.kept_id.cmp(&right.kept_id));
    (unique_candidates, duplicate_groups)
}

fn duplicate_match(
    candidate: &AssetScoredCandidate,
    kept: &AssetScoredCandidate,
    policy: &AssetSelectionPolicy,
) -> bool {
    (!candidate.recipe_fingerprint.is_empty()
        && candidate.recipe_fingerprint == kept.recipe_fingerprint)
        || asset_descriptor_distance(
            &candidate.descriptor,
            &kept.descriptor,
            &policy.descriptor_weights,
        ) <= policy.duplicate_descriptor_distance
}

fn select_representatives(
    candidates: &[AssetScoredCandidate],
    policy: &AssetSelectionPolicy,
) -> Vec<AssetScoredCandidate> {
    if candidates.is_empty() || policy.representative_count == 0 {
        return Vec::new();
    }

    let mut remaining = candidates.to_vec();
    remaining.sort_by(compare_scored_candidates);
    let seed_index = remaining
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| duplicate_candidate_order(left, right))
        .map(|(index, _)| index)
        .unwrap_or(0);
    let mut selected = vec![remaining.remove(seed_index)];

    while selected.len() < policy.representative_count && !remaining.is_empty() {
        let next_index = remaining
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                compare_selection_candidate(left, right, &selected, policy)
            })
            .map(|(index, _)| index)
            .unwrap_or(0);
        selected.push(remaining.remove(next_index));
    }

    selected
}

fn compare_selection_candidate(
    left: &AssetScoredCandidate,
    right: &AssetScoredCandidate,
    selected: &[AssetScoredCandidate],
    policy: &AssetSelectionPolicy,
) -> Ordering {
    let left_minimum = minimum_distance_to_selected(left, selected, policy);
    let right_minimum = minimum_distance_to_selected(right, selected, policy);
    let left_value = left_minimum * policy.diversity_weight
        - left.weighted_quality_penalty * policy.quality_penalty_weight;
    let right_value = right_minimum * policy.diversity_weight
        - right.weighted_quality_penalty * policy.quality_penalty_weight;

    compare_scalar(left_value, right_value)
        .then_with(|| compare_scalar(left_minimum, right_minimum))
        .then_with(|| duplicate_candidate_order(right, left))
}

fn minimum_distance_to_selected(
    candidate: &AssetScoredCandidate,
    selected: &[AssetScoredCandidate],
    policy: &AssetSelectionPolicy,
) -> Scalar {
    selected
        .iter()
        .map(|representative| {
            asset_descriptor_distance(
                &candidate.descriptor,
                &representative.descriptor,
                &policy.descriptor_weights,
            )
        })
        .fold(Scalar::INFINITY, Scalar::min)
}

fn candidate_geometry_is_finite(candidate: &AssetCandidateInput) -> bool {
    candidate.geometry_finite
        && aabb_is_finite(candidate.world_bounds)
        && !candidate.world_bounds.is_empty()
        && candidate.volume_approximation.is_finite()
        && candidate.volume_approximation >= 0.0
        && candidate
            .silhouette_occupancy
            .iter()
            .all(|value| value.is_finite())
        && candidate.part_volumes.iter().all(|value| value.is_finite())
        && candidate.bevel_radii.iter().all(|value| value.is_finite())
        && candidate.accidental_intersection.is_finite()
        && candidate.intersection_tolerance.is_finite()
        && candidate.symmetry_score.is_finite()
        && candidate.topology_cost.is_finite()
        && candidate.near_coincident_surface_ratio.is_finite()
}

fn aabb_is_finite(bounds: Aabb) -> bool {
    bounds.min.is_finite() && bounds.max.is_finite()
}

fn major_part_proportions(part_volumes: &[Scalar]) -> Vec<Scalar> {
    let mut positive_volumes: Vec<_> = part_volumes
        .iter()
        .copied()
        .map(|volume| volume.max(0.0))
        .collect();
    let total = positive_volumes.iter().sum::<Scalar>();
    if total <= EPSILON {
        return Vec::new();
    }
    positive_volumes.sort_by(|left, right| compare_scalar(*right, *left));
    positive_volumes
        .into_iter()
        .map(|volume| volume / total)
        .collect()
}

fn bevel_to_size_ratios(bevel_radii: &[Scalar], scale: Scalar) -> Vec<Scalar> {
    let mut ratios: Vec<_> = bevel_radii
        .iter()
        .copied()
        .filter(|radius| radius.is_finite())
        .map(|radius| (radius.max(0.0) / scale).min(1.0))
        .collect();
    ratios.sort_by(|left, right| compare_scalar(*left, *right));
    ratios
}

fn tiny_parts_penalty(part_volumes: &[Scalar]) -> Scalar {
    if part_volumes.len() < 2 {
        return 0.0;
    }
    let positive: Vec<_> = part_volumes
        .iter()
        .copied()
        .map(|volume| volume.max(0.0))
        .collect();
    let largest = positive.iter().copied().fold(0.0, Scalar::max);
    if largest <= EPSILON {
        return 0.0;
    }
    let threshold = largest * 0.025;
    let tiny_count = positive
        .iter()
        .filter(|volume| **volume > 0.0 && **volume < threshold)
        .count();
    (tiny_count as Scalar / positive.len() as Scalar).clamp(0.0, 1.0)
}

fn extreme_thinness_penalty(bounds: Aabb) -> Scalar {
    let extent = bounds.extent();
    let maximum = extent.max_element();
    if maximum <= EPSILON {
        return 1.0;
    }
    let minimum = extent.min_element().max(0.0);
    let ratio = minimum / maximum;
    ((0.08 - ratio) / 0.08).clamp(0.0, 1.0)
}

fn inconsistent_bevel_penalty(ratios: &[Scalar]) -> Scalar {
    if ratios.len() < 2 {
        return 0.0;
    }
    let mean = ratios.iter().sum::<Scalar>() / ratios.len() as Scalar;
    if mean <= EPSILON {
        return 0.0;
    }
    let relative_deviation = ratios
        .iter()
        .map(|ratio| (ratio - mean).abs())
        .sum::<Scalar>()
        / ratios.len() as Scalar
        / mean;
    ((relative_deviation - 0.45) / 1.2).clamp(0.0, 1.0)
}

fn detached_components_penalty(detached_visual_components: usize, part_count: usize) -> Scalar {
    if detached_visual_components == 0 {
        return 0.0;
    }
    (detached_visual_components as Scalar / part_count.max(1) as Scalar).clamp(0.0, 1.0)
}

fn excessive_detail_penalty(descriptor: &AssetDescriptor) -> Scalar {
    let primary_forms = descriptor
        .major_part_proportions
        .iter()
        .filter(|proportion| **proportion >= 0.12)
        .count()
        .max(1);
    let detail_budget = primary_forms * 6 + descriptor.repeated_element_count.min(6);
    let detail_pressure = descriptor.region_detail_count as Scalar / detail_budget.max(1) as Scalar;
    ((detail_pressure - 1.0) / 2.0).clamp(0.0, 1.0)
}

fn bounds_distance(left: Aabb, right: Aabb) -> Scalar {
    let left_extent = left.extent();
    let right_extent = right.extent();
    let left_scale = left_extent.max_element().max(EPSILON);
    let right_scale = right_extent.max_element().max(EPSILON);
    let center_scale = ((left_scale + right_scale) * 0.5).max(EPSILON);
    let center_distance = ((left.center() - right.center()).length() / center_scale).min(4.0) / 4.0;
    let left_shape = left_extent / left_scale;
    let right_shape = right_extent / right_scale;
    let shape_distance = ((left_shape - right_shape).length() / 3.0_f32.sqrt()).min(1.0);
    let scale_distance = ((left_scale / right_scale).ln().abs() / 4.0).min(1.0);
    (center_distance + shape_distance + scale_distance) / 3.0
}

fn log_scalar_distance(left: Scalar, right: Scalar) -> Scalar {
    let left = left.max(EPSILON);
    let right = right.max(EPSILON);
    ((left / right).ln().abs() / 4.0).min(1.0)
}

fn silhouette_distance(
    left: &[Scalar; FIXED_CAMERA_COUNT],
    right: &[Scalar; FIXED_CAMERA_COUNT],
) -> Scalar {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (left - right).abs())
        .sum::<Scalar>()
        / FIXED_CAMERA_COUNT as Scalar
}

fn count_distance(left: usize, right: usize) -> Scalar {
    if left == right {
        return 0.0;
    }
    let maximum = left.max(right).max(1) as Scalar;
    (left.abs_diff(right) as Scalar / maximum).min(1.0)
}

fn vector_distance(left: &[Scalar], right: &[Scalar]) -> Scalar {
    let length = left.len().max(right.len());
    if length == 0 {
        return 0.0;
    }
    (0..length)
        .map(|index| {
            let left_value = left.get(index).copied().unwrap_or(0.0);
            let right_value = right.get(index).copied().unwrap_or(0.0);
            (left_value - right_value).abs()
        })
        .sum::<Scalar>()
        / length as Scalar
}

fn bounds_volume(bounds: Aabb) -> Scalar {
    if bounds.is_empty() || !aabb_is_finite(bounds) {
        return 0.0;
    }
    let extent = bounds.extent();
    (extent.x * extent.y * extent.z).max(0.0)
}

fn weighted_average(terms: &[(Scalar, Scalar)]) -> Scalar {
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    for (value, weight) in terms {
        if value.is_finite() && weight.is_finite() && *weight > 0.0 {
            weighted_sum += value.clamp(0.0, 1.0) * weight;
            weight_sum += weight;
        }
    }
    if weight_sum <= EPSILON {
        0.0
    } else {
        weighted_sum / weight_sum
    }
}

fn duplicate_candidate_order(
    left: &AssetScoredCandidate,
    right: &AssetScoredCandidate,
) -> Ordering {
    compare_scalar(
        left.weighted_quality_penalty,
        right.weighted_quality_penalty,
    )
    .then_with(|| left.triangle_count.cmp(&right.triangle_count))
    .then_with(|| left.id.cmp(&right.id))
}

fn compare_scored_candidates(
    left: &AssetScoredCandidate,
    right: &AssetScoredCandidate,
) -> Ordering {
    left.id.cmp(&right.id)
}

fn compare_scalar(left: Scalar, right: Scalar) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn finite_non_negative_or_default(value: Scalar, default: Scalar) -> Scalar {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        default
    }
}

fn clamp_unit(value: Scalar) -> Scalar {
    value.clamp(0.0, 1.0)
}

struct WorkingDuplicateGroup {
    kept: AssetScoredCandidate,
    member_ids: Vec<String>,
}
