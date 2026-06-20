//! Deterministic uniform-curvature bend candidate generation.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::{
    explained_fraction, sum_squared_distance, vertex_area_weights_from_parts,
    weighted_sum_squared_distance,
};

use super::bend::{BendParameters, canonicalize_bend_frame, evaluate_bend};
use super::diagnostics::ProgramOperatorDiagnostics;
use super::inference::{
    BendFitSettings, FittedOperatorCandidate, FittingDiagnostics, InferenceError,
    validate_bend_fit_settings,
};
use super::program::ProgramOperator;

const AXIS_DEDUP_DOT: f64 = 0.999;
const DIRECTION_DEDUP_DOT: f64 = 0.999;
const MIN_VECTOR_LENGTH: f64 = 1.0e-12;
const PARAMETER_QUANTIZATION: f64 = 1.0e-7;
const GEOMETRY_DUPLICATE_RELATIVE_EPSILON: f64 = 1.0e-7;
const MAX_COARSE_CANDIDATES: usize = 32;
const REFINED_CANDIDATES: usize = 8;
const RETURNED_CANDIDATES: usize = 16;

/// Generates deterministic bend candidates for an ordered source/target pair.
///
/// `topology` is used only to derive vertex area weights when non-empty
/// explicit `surface_weights` are not supplied.
pub fn generate_bend_candidates(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    topology: &[u32],
    surface_weights: &[f64],
    settings: BendFitSettings,
) -> Result<Vec<FittedOperatorCandidate>, InferenceError> {
    validate_bend_fit_settings(&settings)?;
    validate_position_pair(source_positions, target_positions)?;

    let weights = candidate_weights(source_positions, topology, surface_weights)?;
    let weighted_error_before =
        weighted_sum_squared_distance(source_positions, target_positions, &weights);
    let raw_error_before = sum_squared_distance(source_positions, target_positions);
    if weighted_error_before <= f64::EPSILON || settings.maximum_absolute_angle_radians <= 0.0 {
        return Ok(Vec::new());
    }
    let context = EvaluationContext {
        source_positions,
        target_positions,
        weights: &weights,
        weighted_error_before,
        raw_error_before,
        settings: &settings,
    };

    let residuals = residual_vectors(source_positions, target_positions);
    let centroid = weighted_centroid(source_positions, &weights);
    let pca = weighted_pca_axes(source_positions, &weights, centroid);
    let axes = axis_candidates(&pca.axes);
    let mut candidates = Vec::new();
    let mut seen_parameter_keys = BTreeSet::new();
    let mut duplicate_parameter_rejections = 0usize;

    for axis in axes {
        let directions = direction_candidates(axis, &pca.axes, &residuals, &weights);
        for direction in directions {
            let Some(binormal) = normalize(cross(axis, direction)) else {
                continue;
            };
            for origin in
                origin_candidates(source_positions, &weights, centroid, direction, binormal)
            {
                let intervals = interval_candidates(
                    source_positions,
                    &weights,
                    origin,
                    axis,
                    f64::from(settings.minimum_interval_length),
                );
                for (interval_start, interval_end) in intervals {
                    for angle in initial_angle_candidates(settings.maximum_absolute_angle_radians) {
                        let Some(candidate) = evaluate_candidate(
                            &context,
                            BendCandidateParameters {
                                origin,
                                axis,
                                direction,
                                angle,
                                interval_start,
                                interval_end,
                            },
                        ) else {
                            continue;
                        };
                        if !seen_parameter_keys.insert(candidate.parameter_key.clone()) {
                            duplicate_parameter_rejections += 1;
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
            }
        }
    }

    candidates.sort_by(compare_scored_candidates);
    let geometry_epsilon = geometry_duplicate_epsilon(source_positions, target_positions);
    let (coarse, coarse_geometry_rejections) =
        deduplicate_geometry(candidates, MAX_COARSE_CANDIDATES, geometry_epsilon);

    let refinement_rounds = settings.maximum_refinement_iterations.max(3);
    let mut refined = Vec::new();
    let mut total_coordinate_evaluations = 0usize;
    for (coarse_rank, candidate) in coarse.iter().take(REFINED_CANDIDATES).enumerate() {
        let (mut candidate, evaluations) =
            refine_candidate(candidate.clone(), &context, refinement_rounds);
        candidate.coarse_rank = Some(coarse_rank);
        candidate.refinement_rounds = refinement_rounds;
        candidate.coordinate_descent_evaluations = evaluations;
        total_coordinate_evaluations += evaluations;
        refined.push(candidate);
    }

    let mut final_pool = refined;
    final_pool.extend(coarse.into_iter().skip(REFINED_CANDIDATES));
    final_pool.sort_by(compare_scored_candidates);
    let (final_candidates, final_geometry_rejections) =
        deduplicate_geometry(final_pool, RETURNED_CANDIDATES, geometry_epsilon);
    let duplicate_geometry_rejections = coarse_geometry_rejections + final_geometry_rejections;

    Ok(final_candidates
        .into_iter()
        .enumerate()
        .map(|(rank, candidate)| {
            fitted_candidate(
                candidate,
                rank,
                duplicate_parameter_rejections,
                duplicate_geometry_rejections,
                total_coordinate_evaluations,
            )
        })
        .collect())
}

#[derive(Debug, Copy, Clone)]
struct PcaFrame {
    axes: [[f64; 3]; 3],
}

#[derive(Debug, Copy, Clone)]
struct BendCandidateParameters {
    origin: [f64; 3],
    axis: [f64; 3],
    direction: [f64; 3],
    angle: f64,
    interval_start: f64,
    interval_end: f64,
}

#[derive(Debug, Copy, Clone)]
struct EvaluationContext<'a> {
    source_positions: &'a [[f32; 3]],
    target_positions: &'a [[f32; 3]],
    weights: &'a [f64],
    weighted_error_before: f64,
    raw_error_before: f64,
    settings: &'a BendFitSettings,
}

#[derive(Debug, Clone)]
struct ScoredBendCandidate {
    parameters: BendParameters,
    cumulative_positions: Vec<[f32; 3]>,
    weighted_error_before: f64,
    weighted_error_after: f64,
    raw_error_before: f64,
    raw_error_after: f64,
    weighted_explained_fraction: f64,
    raw_explained_fraction: f64,
    parameter_key: String,
    stable_candidate_id: String,
    coarse_rank: Option<usize>,
    refinement_rounds: usize,
    coordinate_descent_evaluations: usize,
}

fn validate_position_pair(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
) -> Result<(), InferenceError> {
    if source_positions.is_empty() || source_positions.len() != target_positions.len() {
        return Err(InferenceError::InvalidSettings(
            "source and target positions must be non-empty and have equal counts",
        ));
    }
    if !source_positions
        .iter()
        .chain(target_positions)
        .flatten()
        .all(|value| value.is_finite())
    {
        return Err(InferenceError::InvalidSettings(
            "source and target positions must be finite",
        ));
    }
    Ok(())
}

fn candidate_weights(
    source_positions: &[[f32; 3]],
    topology: &[u32],
    surface_weights: &[f64],
) -> Result<Vec<f64>, InferenceError> {
    let weights = if !surface_weights.is_empty() {
        if surface_weights.len() != source_positions.len() {
            return Err(InferenceError::InvalidSettings(
                "surface weights must match source vertex count",
            ));
        }
        if surface_weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        {
            return Err(InferenceError::InvalidSettings(
                "surface weights must be finite and non-negative",
            ));
        }
        surface_weights.to_vec()
    } else if !topology.is_empty() {
        vertex_area_weights_from_parts(source_positions, topology)
    } else {
        vec![1.0; source_positions.len()]
    };
    Ok(normalize_weights(weights))
}

fn normalize_weights(mut weights: Vec<f64>) -> Vec<f64> {
    let total = weights
        .iter()
        .copied()
        .filter(|weight| weight.is_finite() && *weight > 0.0)
        .sum::<f64>();
    if !total.is_finite() || total <= f64::EPSILON {
        return vec![1.0; weights.len()];
    }
    let average = total / weights.len().max(1) as f64;
    for weight in &mut weights {
        if !weight.is_finite() || *weight <= 0.0 {
            *weight = average;
        }
        *weight /= average;
    }
    weights
}

fn residual_vectors(source_positions: &[[f32; 3]], target_positions: &[[f32; 3]]) -> Vec<[f64; 3]> {
    source_positions
        .iter()
        .zip(target_positions)
        .map(|(source, target)| {
            [
                f64::from(target[0]) - f64::from(source[0]),
                f64::from(target[1]) - f64::from(source[1]),
                f64::from(target[2]) - f64::from(source[2]),
            ]
        })
        .collect()
}

fn weighted_centroid(positions: &[[f32; 3]], weights: &[f64]) -> [f64; 3] {
    let mut total = [0.0; 3];
    let mut total_weight = 0.0;
    for (index, position) in positions.iter().enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        total = add(total, scale(to_f64(*position), weight));
        total_weight += weight;
    }
    if total_weight <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    scale(total, total_weight.recip())
}

fn weighted_pca_axes(positions: &[[f32; 3]], weights: &[f64], centroid: [f64; 3]) -> PcaFrame {
    let mut covariance = [[0.0; 3]; 3];
    for (index, position) in positions.iter().enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let delta = sub(to_f64(*position), centroid);
        for row in 0..3 {
            for col in row..3 {
                covariance[row][col] += weight * delta[row] * delta[col];
            }
        }
    }
    covariance[1][0] = covariance[0][1];
    covariance[2][0] = covariance[0][2];
    covariance[2][1] = covariance[1][2];
    let eigens = symmetric_eigen(covariance);
    PcaFrame {
        axes: [eigens[0].1, eigens[1].1, eigens[2].1],
    }
}

fn symmetric_eigen(mut matrix: [[f64; 3]; 3]) -> [(f64, [f64; 3]); 3] {
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..24 {
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            let apq = matrix[p][q];
            if apq.abs() <= 1.0e-18 {
                continue;
            }
            let app = matrix[p][p];
            let aqq = matrix[q][q];
            let tau = (aqq - app) / (2.0 * apq);
            let t = if tau >= 0.0 {
                1.0 / (tau + (1.0 + tau * tau).sqrt())
            } else {
                -1.0 / (-tau + (1.0 + tau * tau).sqrt())
            };
            let c = 1.0 / (1.0 + t * t).sqrt();
            let s = t * c;

            for k in [0usize, 1, 2] {
                if k != p && k != q {
                    let akp = matrix[k][p];
                    let akq = matrix[k][q];
                    matrix[k][p] = c * akp - s * akq;
                    matrix[p][k] = matrix[k][p];
                    matrix[k][q] = s * akp + c * akq;
                    matrix[q][k] = matrix[k][q];
                }
            }
            matrix[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
            matrix[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
            matrix[p][q] = 0.0;
            matrix[q][p] = 0.0;

            for row in &mut vectors {
                let vip = row[p];
                let viq = row[q];
                row[p] = c * vip - s * viq;
                row[q] = s * vip + c * viq;
            }
        }
    }

    let mut eigens = [
        (
            matrix[0][0],
            canonicalize_vector([vectors[0][0], vectors[1][0], vectors[2][0]]),
        ),
        (
            matrix[1][1],
            canonicalize_vector([vectors[0][1], vectors[1][1], vectors[2][1]]),
        ),
        (
            matrix[2][2],
            canonicalize_vector([vectors[0][2], vectors[1][2], vectors[2][2]]),
        ),
    ];
    eigens.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| compare_vectors(left.1, right.1))
    });
    eigens
}

fn axis_candidates(pca_axes: &[[f64; 3]; 3]) -> Vec<[f64; 3]> {
    let mut axes = Vec::new();
    for axis in pca_axes
        .iter()
        .copied()
        .chain([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    {
        push_dedup_axis(&mut axes, axis);
    }
    axes
}

fn push_dedup_axis(axes: &mut Vec<[f64; 3]>, axis: [f64; 3]) {
    let Some(axis) = normalize(axis).map(canonicalize_vector) else {
        return;
    };
    if axes
        .iter()
        .all(|existing| dot(*existing, axis).abs() <= AXIS_DEDUP_DOT)
    {
        axes.push(axis);
    }
}

fn direction_candidates(
    axis: [f64; 3],
    pca_axes: &[[f64; 3]; 3],
    residuals: &[[f64; 3]],
    weights: &[f64],
) -> Vec<[f64; 3]> {
    let mut directions = Vec::new();
    for candidate in
        pca_axes
            .iter()
            .copied()
            .chain([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    {
        push_dedup_direction(&mut directions, project_orthogonal(candidate, axis));
    }

    let mean_residual = weighted_mean_vectors(residuals, weights);
    push_dedup_direction(&mut directions, project_orthogonal(mean_residual, axis));

    if let Some(dominant_residual) = dominant_projected_residual_direction(axis, residuals, weights)
    {
        push_dedup_direction(&mut directions, dominant_residual);
    }
    directions
}

fn push_dedup_direction(directions: &mut Vec<[f64; 3]>, direction: [f64; 3]) {
    let Some(direction) = normalize(direction).map(canonicalize_vector) else {
        return;
    };
    if directions
        .iter()
        .all(|existing| dot(*existing, direction).abs() <= DIRECTION_DEDUP_DOT)
    {
        directions.push(direction);
    }
}

fn weighted_mean_vectors(vectors: &[[f64; 3]], weights: &[f64]) -> [f64; 3] {
    let mut total = [0.0; 3];
    let mut total_weight = 0.0;
    for (index, vector) in vectors.iter().copied().enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        total = add(total, scale(vector, weight));
        total_weight += weight;
    }
    if total_weight <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    scale(total, total_weight.recip())
}

fn dominant_projected_residual_direction(
    axis: [f64; 3],
    residuals: &[[f64; 3]],
    weights: &[f64],
) -> Option<[f64; 3]> {
    let projected: Vec<[f64; 3]> = residuals
        .iter()
        .copied()
        .map(|residual| project_orthogonal(residual, axis))
        .collect();
    let mean = weighted_mean_vectors(&projected, weights);
    let mut covariance = [[0.0; 3]; 3];
    for (index, residual) in projected.iter().copied().enumerate() {
        let weight = weights.get(index).copied().unwrap_or(1.0);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let centered = sub(residual, mean);
        for row in 0..3 {
            for col in row..3 {
                covariance[row][col] += weight * centered[row] * centered[col];
            }
        }
    }
    covariance[1][0] = covariance[0][1];
    covariance[2][0] = covariance[0][2];
    covariance[2][1] = covariance[1][2];
    let eigens = symmetric_eigen(covariance);
    let stable_floor = residual_scale_squared(residuals).max(1.0) * 1.0e-12;
    if eigens[0].0 <= stable_floor || eigens[0].0 <= eigens[1].0 * 1.05 {
        return None;
    }
    normalize(project_orthogonal(eigens[0].1, axis)).map(canonicalize_vector)
}

fn residual_scale_squared(residuals: &[[f64; 3]]) -> f64 {
    residuals
        .iter()
        .copied()
        .map(|residual| dot(residual, residual))
        .fold(0.0, f64::max)
}

fn origin_candidates(
    source_positions: &[[f32; 3]],
    weights: &[f64],
    centroid: [f64; 3],
    direction: [f64; 3],
    binormal: [f64; 3],
) -> Vec<[f64; 3]> {
    let bbox_center = bounding_box_center(source_positions);
    let direction_extent = projection_extent(source_positions, direction);
    let binormal_extent = projection_extent(source_positions, binormal);
    let mut origins = Vec::new();
    push_dedup_origin(&mut origins, centroid);
    push_dedup_origin(&mut origins, bbox_center);
    for sign in [-1.0, 1.0] {
        push_dedup_origin(
            &mut origins,
            add(centroid, scale(direction, sign * 0.25 * direction_extent)),
        );
        push_dedup_origin(
            &mut origins,
            add(centroid, scale(binormal, sign * 0.25 * binormal_extent)),
        );
    }

    if origins.is_empty() {
        push_dedup_origin(&mut origins, weighted_centroid(source_positions, weights));
    }
    origins
}

fn push_dedup_origin(origins: &mut Vec<[f64; 3]>, origin: [f64; 3]) {
    if origin.iter().any(|coordinate| !coordinate.is_finite()) {
        return;
    }
    let scale = origins
        .iter()
        .copied()
        .map(|existing| squared_length(sub(existing, origin)).sqrt())
        .fold(1.0_f64, f64::max);
    let epsilon_squared = (PARAMETER_QUANTIZATION * scale).powi(2);
    if origins
        .iter()
        .all(|existing| squared_length(sub(*existing, origin)) > epsilon_squared)
    {
        origins.push(origin);
    }
}

fn bounding_box_center(positions: &[[f32; 3]]) -> [f64; 3] {
    let mut min = to_f64(positions[0]);
    let mut max = min;
    for position in positions.iter().skip(1) {
        let position = to_f64(*position);
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    scale(add(min, max), 0.5)
}

fn interval_candidates(
    source_positions: &[[f32; 3]],
    weights: &[f64],
    origin: [f64; 3],
    axis: [f64; 3],
    minimum_interval_length: f64,
) -> Vec<(f64, f64)> {
    let projections: Vec<f64> = source_positions
        .iter()
        .map(|position| dot(sub(to_f64(*position), origin), axis))
        .collect();
    let pairs = [
        (0.0, 1.0),
        (0.05, 0.95),
        (0.10, 0.90),
        (0.20, 0.80),
        (0.0, 0.75),
        (0.25, 1.0),
    ];
    let mut intervals = Vec::new();
    for (start_quantile, end_quantile) in pairs {
        let start = weighted_quantile(&projections, weights, start_quantile);
        let end = weighted_quantile(&projections, weights, end_quantile);
        if end - start >= minimum_interval_length && start.is_finite() && end.is_finite() {
            let key = interval_key(start, end);
            if intervals.iter().all(|(existing_start, existing_end)| {
                interval_key(*existing_start, *existing_end) != key
            }) {
                intervals.push((start, end));
            }
        }
    }
    intervals
}

fn weighted_quantile(values: &[f64], weights: &[f64], quantile: f64) -> f64 {
    let mut weighted_values = values
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, value)| {
            let weight = weights.get(index).copied().unwrap_or(1.0);
            (value.is_finite() && weight.is_finite() && weight > 0.0).then_some((value, weight))
        })
        .collect::<Vec<_>>();
    weighted_values.sort_by(|left, right| left.0.total_cmp(&right.0));
    if weighted_values.is_empty() {
        return 0.0;
    }
    if quantile <= 0.0 {
        return weighted_values[0].0;
    }
    if quantile >= 1.0 {
        return weighted_values[weighted_values.len() - 1].0;
    }

    let mut grouped = Vec::<(f64, f64)>::new();
    for (value, weight) in weighted_values {
        if let Some((last_value, last_weight)) = grouped.last_mut()
            && value.to_bits() == last_value.to_bits()
        {
            *last_weight += weight;
            continue;
        }
        grouped.push((value, weight));
    }
    if grouped.len() == 1 {
        return grouped[0].0;
    }

    let total_weight = grouped.iter().map(|(_, weight)| *weight).sum::<f64>();
    let mut cumulative_before = 0.0;
    let mut control_points = Vec::with_capacity(grouped.len());
    for (index, (value, weight)) in grouped.iter().copied().enumerate() {
        let position = if index == 0 {
            0.0
        } else if index == grouped.len() - 1 {
            1.0
        } else {
            cumulative_before / total_weight
        };
        control_points.push((position, value));
        cumulative_before += weight;
    }
    for window in control_points.windows(2) {
        let (left_q, left_value) = window[0];
        let (right_q, right_value) = window[1];
        if quantile <= right_q {
            let span = right_q - left_q;
            if span <= f64::EPSILON {
                return right_value;
            }
            let t = ((quantile - left_q) / span).clamp(0.0, 1.0);
            return left_value + (right_value - left_value) * t;
        }
    }
    control_points[control_points.len() - 1].1
}

fn initial_angle_candidates(maximum_absolute_angle: f32) -> Vec<f64> {
    let mut angles = Vec::new();
    for degrees in [5.0_f64, 10.0, 15.0, 22.5, 30.0, 45.0, 60.0, 90.0, 120.0] {
        let radians = degrees.to_radians();
        if radians <= f64::from(maximum_absolute_angle) {
            angles.push(radians);
            angles.push(-radians);
        }
    }
    angles
}

fn evaluate_candidate(
    context: &EvaluationContext<'_>,
    parameters: BendCandidateParameters,
) -> Option<ScoredBendCandidate> {
    if parameters.interval_end - parameters.interval_start
        < f64::from(context.settings.minimum_interval_length)
        || parameters.angle.abs() > f64::from(context.settings.maximum_absolute_angle_radians)
    {
        return None;
    }
    let parameters = BendParameters {
        origin: to_f32_checked(parameters.origin)?,
        longitudinal_axis: to_f32_checked(parameters.axis)?,
        bend_direction: to_f32_checked(parameters.direction)?,
        angle_radians: to_f32_scalar(parameters.angle)?,
        interval_start: to_f32_scalar(parameters.interval_start)?,
        interval_end: to_f32_scalar(parameters.interval_end)?,
    };
    let validated = canonicalize_bend_frame(&parameters).ok()?;
    let parameters = BendParameters {
        origin: validated.origin,
        longitudinal_axis: validated.longitudinal_axis,
        bend_direction: validated.bend_direction,
        angle_radians: validated.angle_radians,
        interval_start: validated.interval_start,
        interval_end: validated.interval_end,
    };
    let cumulative_positions = evaluate_bend(&parameters, context.source_positions).ok()?;
    if cumulative_positions
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return None;
    }
    let weighted_error_after = weighted_sum_squared_distance(
        &cumulative_positions,
        context.target_positions,
        context.weights,
    );
    let raw_error_after = sum_squared_distance(&cumulative_positions, context.target_positions);
    if !weighted_error_after.is_finite()
        || !raw_error_after.is_finite()
        || weighted_error_after
            >= context.weighted_error_before - improvement_epsilon(context.weighted_error_before)
    {
        return None;
    }
    Some(ScoredBendCandidate {
        parameters,
        cumulative_positions,
        weighted_error_before: context.weighted_error_before,
        weighted_error_after,
        raw_error_before: context.raw_error_before,
        raw_error_after,
        weighted_explained_fraction: explained_fraction(
            context.weighted_error_before,
            weighted_error_after,
        ),
        raw_explained_fraction: explained_fraction(context.raw_error_before, raw_error_after),
        parameter_key: parameter_key(&parameters),
        stable_candidate_id: stable_bend_candidate_id(&parameters),
        coarse_rank: None,
        refinement_rounds: 0,
        coordinate_descent_evaluations: 0,
    })
}

fn improvement_epsilon(error: f64) -> f64 {
    (error.abs() * 1.0e-12).max(1.0e-18)
}

fn compare_scored_candidates(left: &ScoredBendCandidate, right: &ScoredBendCandidate) -> Ordering {
    left.weighted_error_after
        .total_cmp(&right.weighted_error_after)
        .then_with(|| left.raw_error_after.total_cmp(&right.raw_error_after))
        .then_with(|| right.refinement_rounds.cmp(&left.refinement_rounds))
        .then_with(|| left.parameter_key.cmp(&right.parameter_key))
        .then_with(|| left.stable_candidate_id.cmp(&right.stable_candidate_id))
}

fn deduplicate_geometry(
    candidates: Vec<ScoredBendCandidate>,
    limit: usize,
    epsilon: f64,
) -> (Vec<ScoredBendCandidate>, usize) {
    let mut kept = Vec::<ScoredBendCandidate>::new();
    let mut rejected = 0usize;
    let epsilon_squared = epsilon * epsilon;
    for candidate in candidates {
        if kept.iter().any(|existing| {
            geometry_distance_squared(
                &existing.cumulative_positions,
                &candidate.cumulative_positions,
            ) <= epsilon_squared
        }) {
            rejected += 1;
            continue;
        }
        kept.push(candidate);
        if kept.len() >= limit {
            break;
        }
    }
    (kept, rejected)
}

fn geometry_distance_squared(left: &[[f32; 3]], right: &[[f32; 3]]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = sub(to_f64(*left), to_f64(*right));
            squared_length(delta)
        })
        .fold(0.0, f64::max)
}

fn refine_candidate(
    initial: ScoredBendCandidate,
    context: &EvaluationContext<'_>,
    rounds: usize,
) -> (ScoredBendCandidate, usize) {
    let mut best = initial;
    let mut evaluations = 0usize;
    let axis = to_f64(best.parameters.longitudinal_axis);
    let direction = to_f64(best.parameters.bend_direction);
    let Some(binormal) = normalize(cross(axis, direction)) else {
        return (best, evaluations);
    };
    let axis_extent = projection_extent(context.source_positions, axis).max(MIN_VECTOR_LENGTH);
    let direction_extent =
        projection_extent(context.source_positions, direction).max(axis_extent * 0.05);
    let binormal_extent =
        projection_extent(context.source_positions, binormal).max(axis_extent * 0.05);

    for round in 0..rounds {
        let shrink = 0.4_f64.powi(round as i32);
        let angle_step = 10.0_f64.to_radians() * shrink;
        let interval_step = axis_extent * 0.05 * shrink;
        let direction_origin_step = direction_extent * 0.10 * shrink;
        let binormal_origin_step = binormal_extent * 0.10 * shrink;

        for coordinate in [
            RefinementCoordinate::Angle(angle_step),
            RefinementCoordinate::IntervalStart(interval_step),
            RefinementCoordinate::IntervalEnd(interval_step),
            RefinementCoordinate::OriginDirection(direction_origin_step),
            RefinementCoordinate::OriginBinormal(binormal_origin_step),
        ] {
            for sign in [-1.0, 1.0] {
                evaluations += 1;
                let trial_parameters = refined_parameters(
                    &best.parameters,
                    axis,
                    direction,
                    binormal,
                    coordinate,
                    sign,
                );
                let Some(trial) = evaluate_candidate(context, trial_parameters) else {
                    continue;
                };
                if compare_scored_candidates(&trial, &best).is_lt() {
                    best = trial;
                }
            }
        }
    }
    (best, evaluations)
}

#[derive(Debug, Copy, Clone)]
enum RefinementCoordinate {
    Angle(f64),
    IntervalStart(f64),
    IntervalEnd(f64),
    OriginDirection(f64),
    OriginBinormal(f64),
}

fn refined_parameters(
    parameters: &BendParameters,
    axis: [f64; 3],
    direction: [f64; 3],
    binormal: [f64; 3],
    coordinate: RefinementCoordinate,
    sign: f64,
) -> BendCandidateParameters {
    let mut origin = to_f64(parameters.origin);
    let mut angle = f64::from(parameters.angle_radians);
    let mut interval_start = f64::from(parameters.interval_start);
    let mut interval_end = f64::from(parameters.interval_end);
    match coordinate {
        RefinementCoordinate::Angle(step) => angle += sign * step,
        RefinementCoordinate::IntervalStart(step) => interval_start += sign * step,
        RefinementCoordinate::IntervalEnd(step) => interval_end += sign * step,
        RefinementCoordinate::OriginDirection(step) => {
            origin = add(origin, scale(direction, sign * step));
        }
        RefinementCoordinate::OriginBinormal(step) => {
            origin = add(origin, scale(binormal, sign * step));
        }
    }
    BendCandidateParameters {
        origin,
        axis,
        direction,
        angle,
        interval_start,
        interval_end,
    }
}

fn fitted_candidate(
    candidate: ScoredBendCandidate,
    rank: usize,
    duplicate_parameter_rejections: usize,
    duplicate_geometry_rejections: usize,
    total_coordinate_evaluations: usize,
) -> FittedOperatorCandidate {
    let diagnostics = ProgramOperatorDiagnostics::Bend {
        parameters: candidate.parameters,
    };
    let semantic_parameter_count = diagnostics.semantic_parameter_count();
    let semantic_metadata_bytes = diagnostics.semantic_metadata_bytes();
    FittedOperatorCandidate {
        operator: ProgramOperator::Bend(candidate.parameters),
        diagnostics,
        cumulative_positions: candidate.cumulative_positions,
        weighted_error_before: candidate.weighted_error_before,
        weighted_error_after: candidate.weighted_error_after,
        raw_error_before: candidate.raw_error_before,
        raw_error_after: candidate.raw_error_after,
        weighted_explained_fraction: candidate.weighted_explained_fraction,
        raw_explained_fraction: candidate.raw_explained_fraction,
        semantic_parameter_count,
        semantic_metadata_bytes,
        stable_candidate_id: candidate.stable_candidate_id,
        fitting_diagnostics: FittingDiagnostics {
            generator: "deterministic_bend_fit".to_owned(),
            coarse_rank: candidate.coarse_rank.or(Some(rank)),
            refinement_rounds: candidate.refinement_rounds,
            coordinate_descent_evaluations: candidate
                .coordinate_descent_evaluations
                .max(total_coordinate_evaluations),
            duplicate_parameter_rejections,
            duplicate_geometry_rejections,
        },
    }
}

fn interval_key(start: f64, end: f64) -> (i64, i64) {
    (quantize(start), quantize(end))
}

fn parameter_key(parameters: &BendParameters) -> String {
    let mut parts = Vec::with_capacity(12);
    for value in parameters.origin {
        parts.push(quantize(f64::from(value)));
    }
    for value in parameters.longitudinal_axis {
        parts.push(quantize(f64::from(value)));
    }
    for value in parameters.bend_direction {
        parts.push(quantize(f64::from(value)));
    }
    parts.push(quantize(f64::from(parameters.angle_radians)));
    parts.push(quantize(f64::from(parameters.interval_start)));
    parts.push(quantize(f64::from(parameters.interval_end)));
    parts
        .into_iter()
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join(":")
}

fn quantize(value: f64) -> i64 {
    (value / PARAMETER_QUANTIZATION).round() as i64
}

fn stable_bend_candidate_id(parameters: &BendParameters) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn update(hash: &mut u64, value: f32) {
        for byte in value.to_bits().to_le_bytes() {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(FNV_PRIME);
        }
    }

    let mut hash = FNV_OFFSET;
    for value in parameters.origin {
        update(&mut hash, value);
    }
    for value in parameters.longitudinal_axis {
        update(&mut hash, value);
    }
    for value in parameters.bend_direction {
        update(&mut hash, value);
    }
    update(&mut hash, parameters.angle_radians);
    update(&mut hash, parameters.interval_start);
    update(&mut hash, parameters.interval_end);
    format!("bend-{hash:016x}")
}

fn geometry_duplicate_epsilon(left: &[[f32; 3]], right: &[[f32; 3]]) -> f64 {
    GEOMETRY_DUPLICATE_RELATIVE_EPSILON
        * bounding_box_diagonal(left)
            .max(bounding_box_diagonal(right))
            .max(1.0)
}

fn bounding_box_diagonal(positions: &[[f32; 3]]) -> f64 {
    let mut min = to_f64(positions[0]);
    let mut max = min;
    for position in positions.iter().skip(1) {
        let position = to_f64(*position);
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    squared_length(sub(max, min)).sqrt()
}

fn projection_extent(positions: &[[f32; 3]], axis: [f64; 3]) -> f64 {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for position in positions {
        let projection = dot(to_f64(*position), axis);
        min = min.min(projection);
        max = max.max(projection);
    }
    (max - min).max(0.0)
}

fn compare_vectors(left: [f64; 3], right: [f64; 3]) -> Ordering {
    left[0]
        .total_cmp(&right[0])
        .then_with(|| left[1].total_cmp(&right[1]))
        .then_with(|| left[2].total_cmp(&right[2]))
}

fn canonicalize_vector(vector: [f64; 3]) -> [f64; 3] {
    let mut largest_axis = 0usize;
    let mut largest_abs = vector[0].abs();
    for (axis, component) in vector.iter().enumerate().skip(1) {
        let component_abs = component.abs();
        if component_abs > largest_abs {
            largest_axis = axis;
            largest_abs = component_abs;
        }
    }
    let mut vector = vector;
    if vector[largest_axis].is_sign_negative() {
        vector = scale(vector, -1.0);
    }
    canonicalize_zeroes(vector)
}

fn canonicalize_zeroes(vector: [f64; 3]) -> [f64; 3] {
    [
        canonicalize_zero(vector[0]),
        canonicalize_zero(vector[1]),
        canonicalize_zero(vector[2]),
    ]
}

fn canonicalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn project_orthogonal(vector: [f64; 3], axis: [f64; 3]) -> [f64; 3] {
    sub(vector, scale(axis, dot(vector, axis)))
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length_squared = squared_length(vector);
    if !length_squared.is_finite() || length_squared <= MIN_VECTOR_LENGTH * MIN_VECTOR_LENGTH {
        return None;
    }
    Some(scale(vector, length_squared.sqrt().recip()))
}

fn squared_length(vector: [f64; 3]) -> f64 {
    dot(vector, vector)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn to_f64(vector: [f32; 3]) -> [f64; 3] {
    [
        f64::from(vector[0]),
        f64::from(vector[1]),
        f64::from(vector[2]),
    ]
}

fn to_f32_checked(vector: [f64; 3]) -> Option<[f32; 3]> {
    Some([
        to_f32_scalar(vector[0])?,
        to_f32_scalar(vector[1])?,
        to_f32_scalar(vector[2])?,
    ])
}

fn to_f32_scalar(value: f64) -> Option<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return None;
    }
    let value = value as f32;
    value
        .is_finite()
        .then_some(if value == 0.0 { 0.0 } else { value })
}
