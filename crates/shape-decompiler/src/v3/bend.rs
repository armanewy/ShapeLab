//! Uniform-curvature bend operator contracts and evaluation.
//!
//! The bend convention is:
//!
//! - the validated frame is right handed
//! - `binormal = longitudinal_axis x bend_direction`
//! - a positive angle bends the neutral axis toward `bend_direction`
//! - points before the interval are unchanged
//! - points inside the interval follow uniform curvature
//! - points after the interval follow a terminal rigid tail
//! - all positions are continuous at both interval boundaries
//! - the neutral-axis tangent is continuous
//! - there is no claim of complete Jacobian continuity for off-axis points
//! - near-zero angles have an exact identity path

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MIN_AXIS_LENGTH: f64 = 1.0e-6;
const MIN_DIRECTION_ORTHOGONAL_RATIO: f64 = 1.0e-6;
const IDENTITY_ANGLE_EPSILON: f32 = 1.0e-7;
const SERIES_ANGLE_EPSILON: f64 = 1.0e-4;
const MAX_ABS_BEND_ANGLE: f32 = std::f32::consts::PI;
const DEFAULT_BEND_ABSOLUTE_EPSILON: f64 = 1.0e-6;
const DEFAULT_BEND_SHAPE_RELATIVE_EPSILON: f64 = 1.0e-5;
const DEFAULT_BEND_ULP_MULTIPLIER: f64 = 2.0;

/// Parameters for a uniform-curvature bend over a longitudinal interval.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendParameters {
    /// Point on the neutral axis used as the bend frame origin.
    pub origin: [f32; 3],
    /// Longitudinal neutral-axis direction before bending.
    pub longitudinal_axis: [f32; 3],
    /// Direction that a positive bend angle initially bends toward.
    pub bend_direction: [f32; 3],
    /// Total signed bend angle across the interval, in radians.
    pub angle_radians: f32,
    /// Start coordinate along the longitudinal axis.
    pub interval_start: f32,
    /// End coordinate along the longitudinal axis.
    pub interval_end: f32,
}

/// Canonical bend parameters with a normalized right-handed frame.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedBendParameters {
    /// Point on the neutral axis used as the bend frame origin.
    pub origin: [f32; 3],
    /// Normalized longitudinal neutral-axis direction.
    pub longitudinal_axis: [f32; 3],
    /// Normalized direction orthogonal to the longitudinal axis.
    pub bend_direction: [f32; 3],
    /// Normalized right-handed binormal, equal to longitudinal x direction.
    pub binormal: [f32; 3],
    /// Total signed bend angle across the interval, in radians.
    pub angle_radians: f32,
    /// Start coordinate along the longitudinal axis.
    pub interval_start: f32,
    /// End coordinate along the longitudinal axis.
    pub interval_end: f32,
}

impl ValidatedBendParameters {
    /// Returns true when this bend must evaluate as exact identity.
    pub fn is_identity_angle(self) -> bool {
        self.angle_radians.abs() <= IDENTITY_ANGLE_EPSILON
    }
}

/// Policy for comparing evaluated bend positions to a baked stage.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendStageVerificationPolicy {
    /// Absolute Euclidean tolerance floor.
    pub absolute_epsilon: f64,
    /// Tolerance multiplier for the intrinsic scale of the compared shapes.
    pub shape_relative_epsilon: f64,
    /// Tolerance multiplier for local `f32` coordinate spacing.
    pub ulp_multiplier: f64,
}

impl Default for BendStageVerificationPolicy {
    fn default() -> Self {
        Self {
            absolute_epsilon: DEFAULT_BEND_ABSOLUTE_EPSILON,
            shape_relative_epsilon: DEFAULT_BEND_SHAPE_RELATIVE_EPSILON,
            ulp_multiplier: DEFAULT_BEND_ULP_MULTIPLIER,
        }
    }
}

/// Metrics from comparing an evaluated bend stage to baked positions.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendStageVerificationReport {
    /// Maximum absolute per-component error.
    pub max_component_error: f64,
    /// Maximum Euclidean vertex error.
    pub max_euclidean_error: f64,
    /// Mean Euclidean vertex error.
    pub mean_euclidean_error: f64,
    /// Root-mean-square Euclidean vertex error.
    pub rms_euclidean_error: f64,
    /// Number of vertices outside the declared tolerance.
    pub outside_tolerance: usize,
    /// Whether the semantic bend stage satisfied the policy.
    pub passed: bool,
}

impl Default for BendStageVerificationReport {
    fn default() -> Self {
        Self {
            max_component_error: 0.0,
            max_euclidean_error: 0.0,
            mean_euclidean_error: 0.0,
            rms_euclidean_error: 0.0,
            outside_tolerance: 0,
            passed: true,
        }
    }
}

/// Validation failures for bend parameters.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BendValidationError {
    /// One or more scalar parameters were not finite.
    #[error("bend parameters must be finite")]
    NonFiniteParameter,
    /// The longitudinal axis was too short to normalize.
    #[error("bend longitudinal axis must be non-zero")]
    DegenerateLongitudinalAxis,
    /// The bend direction had no usable component orthogonal to the axis.
    #[error("bend direction must have a non-zero component orthogonal to the longitudinal axis")]
    DegenerateBendDirection,
    /// The interval end must be strictly greater than the interval start.
    #[error("bend interval_end must be greater than interval_start")]
    InvalidInterval,
    /// The bend angle is outside the initially supported range.
    #[error("bend angle magnitude must be at most pi radians")]
    AngleOutOfRange,
}

/// Evaluation failures for bend operators.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BendEvaluationError {
    /// Parameter validation failed before evaluation.
    #[error(transparent)]
    Validation(#[from] BendValidationError),
    /// A source position was not finite.
    #[error("bend source positions must be finite")]
    NonFinitePosition,
    /// Evaluation produced a value that cannot be represented as finite `f32`.
    #[error("bend evaluation produced a non-finite coordinate")]
    NonFiniteOutput,
    /// Legacy variant retained for callers compiled against the contract stub.
    #[error("uniform-curvature bend evaluation is not implemented yet")]
    NotImplemented,
}

/// Verification failures for semantic bend-to-baked comparison.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BendStageVerificationError {
    /// Bend evaluation failed before comparison.
    #[error(transparent)]
    Evaluation(#[from] BendEvaluationError),
    /// The evaluated source and baked stage do not have matching vertex counts.
    #[error(
        "bend stage source vertex count {source_count} did not match baked vertex count {baked_count}"
    )]
    VertexCountMismatch {
        /// Number of source vertices.
        source_count: usize,
        /// Number of baked vertices.
        baked_count: usize,
    },
    /// The verification policy contained a non-finite or negative tolerance.
    #[error("bend stage verification policy must contain finite non-negative tolerances")]
    InvalidPolicy,
    /// A baked position was not finite.
    #[error("bend baked stage positions must be finite")]
    NonFiniteBakedPosition,
}

/// Validates and canonicalizes bend parameters.
///
/// The bend direction is projected away from the longitudinal axis before
/// normalization, so accepted output always carries an orthogonal frame.
pub fn validate_bend_parameters(
    parameters: &BendParameters,
) -> Result<ValidatedBendParameters, BendValidationError> {
    canonicalize_bend_frame(parameters)
}

/// Evaluates a bend over a slice of positions.
pub fn evaluate_bend(
    parameters: &BendParameters,
    positions: &[[f32; 3]],
) -> Result<Vec<[f32; 3]>, BendEvaluationError> {
    let validated = validate_bend_parameters(parameters)?;
    if positions
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(BendEvaluationError::NonFinitePosition);
    }
    if validated.is_identity_angle() {
        return Ok(positions.to_vec());
    }
    positions
        .iter()
        .copied()
        .map(|position| evaluate_validated_bend_point(validated, position))
        .collect()
}

/// Evaluates a single point under a bend.
pub fn evaluate_bend_point(
    parameters: &BendParameters,
    position: [f32; 3],
) -> Result<[f32; 3], BendEvaluationError> {
    let validated = validate_bend_parameters(parameters)?;
    if position.iter().any(|coordinate| !coordinate.is_finite()) {
        return Err(BendEvaluationError::NonFinitePosition);
    }
    if validated.is_identity_angle() {
        return Ok(position);
    }
    evaluate_validated_bend_point(validated, position)
}

/// Compares an evaluated bend stage to authoritative baked stage positions.
///
/// The tolerance follows the same shape as schema-2 approximate residual
/// scoring: an absolute floor, an intrinsic shape-relative floor, and a local
/// `f32` ULP floor computed around each compared shape's centroid.
pub fn compare_bend_to_baked_stage(
    parameters: &BendParameters,
    source_positions: &[[f32; 3]],
    baked_positions: &[[f32; 3]],
    policy: BendStageVerificationPolicy,
) -> Result<BendStageVerificationReport, BendStageVerificationError> {
    if !policy.absolute_epsilon.is_finite()
        || !policy.shape_relative_epsilon.is_finite()
        || !policy.ulp_multiplier.is_finite()
        || policy.absolute_epsilon < 0.0
        || policy.shape_relative_epsilon < 0.0
        || policy.ulp_multiplier < 0.0
    {
        return Err(BendStageVerificationError::InvalidPolicy);
    }
    if source_positions.len() != baked_positions.len() {
        return Err(BendStageVerificationError::VertexCountMismatch {
            source_count: source_positions.len(),
            baked_count: baked_positions.len(),
        });
    }
    if baked_positions
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(BendStageVerificationError::NonFiniteBakedPosition);
    }

    let evaluated = evaluate_bend(parameters, source_positions)?;
    let intrinsic_scale = intrinsic_shape_scale(&evaluated, baked_positions);
    let base_epsilon = policy
        .absolute_epsilon
        .max(intrinsic_scale * policy.shape_relative_epsilon);
    let evaluated_centroid = centroid_f64(&evaluated);
    let baked_centroid = centroid_f64(baked_positions);
    let mut report = BendStageVerificationReport::default();
    let mut sum_distance = 0.0_f64;
    let mut sum_distance_squared = 0.0_f64;

    for (evaluated_position, baked_position) in evaluated.iter().zip(baked_positions) {
        let delta = [
            f64::from(evaluated_position[0]) - f64::from(baked_position[0]),
            f64::from(evaluated_position[1]) - f64::from(baked_position[1]),
            f64::from(evaluated_position[2]) - f64::from(baked_position[2]),
        ];
        let distance_squared = dot_f64(delta, delta);
        let distance = distance_squared.sqrt();
        let local_ulp = position_local_coordinate_ulp(
            *evaluated_position,
            *baked_position,
            evaluated_centroid,
            baked_centroid,
        );
        let epsilon = base_epsilon.max(policy.ulp_multiplier * local_ulp);

        report.max_component_error = report
            .max_component_error
            .max(delta[0].abs())
            .max(delta[1].abs())
            .max(delta[2].abs());
        report.max_euclidean_error = report.max_euclidean_error.max(distance);
        sum_distance += distance;
        sum_distance_squared += distance_squared;
        if distance > epsilon {
            report.outside_tolerance += 1;
        }
    }

    if !evaluated.is_empty() {
        let count = evaluated.len() as f64;
        report.mean_euclidean_error = sum_distance / count;
        report.rms_euclidean_error = (sum_distance_squared / count).sqrt();
    }
    report.passed = report.outside_tolerance == 0;
    Ok(report)
}

/// Builds the canonical right-handed bend frame.
pub fn canonicalize_bend_frame(
    parameters: &BendParameters,
) -> Result<ValidatedBendParameters, BendValidationError> {
    if !parameters.origin.iter().all(|value| value.is_finite())
        || !parameters
            .longitudinal_axis
            .iter()
            .all(|value| value.is_finite())
        || !parameters
            .bend_direction
            .iter()
            .all(|value| value.is_finite())
        || !parameters.angle_radians.is_finite()
        || !parameters.interval_start.is_finite()
        || !parameters.interval_end.is_finite()
    {
        return Err(BendValidationError::NonFiniteParameter);
    }
    if parameters.interval_end <= parameters.interval_start {
        return Err(BendValidationError::InvalidInterval);
    }
    if parameters.angle_radians.abs() > MAX_ABS_BEND_ANGLE {
        return Err(BendValidationError::AngleOutOfRange);
    }

    let axis = normalize_f64(to_f64(parameters.longitudinal_axis))
        .ok_or(BendValidationError::DegenerateLongitudinalAxis)?;
    let direction = to_f64(parameters.bend_direction);
    let direction_length =
        length_f64(direction).ok_or(BendValidationError::DegenerateBendDirection)?;
    if direction_length <= MIN_AXIS_LENGTH {
        return Err(BendValidationError::DegenerateBendDirection);
    }

    let direction_dot_axis = dot_f64(direction, axis);
    let projected_direction = sub_f64(direction, scale_f64(axis, direction_dot_axis));
    let projected_length =
        length_f64(projected_direction).ok_or(BendValidationError::DegenerateBendDirection)?;
    if projected_length <= MIN_AXIS_LENGTH
        || projected_length / direction_length <= MIN_DIRECTION_ORTHOGONAL_RATIO
    {
        return Err(BendValidationError::DegenerateBendDirection);
    }

    let longitudinal_axis = canonicalize_signed_zeroes(to_f32(axis));
    let mut bend_direction = canonicalize_signed_zeroes(to_f32(scale_f64(
        projected_direction,
        projected_length.recip(),
    )));
    let mut angle_radians = canonicalize_zero(parameters.angle_radians);
    if should_flip_direction_sign(bend_direction) {
        bend_direction = negate_f32(bend_direction);
        angle_radians = canonicalize_zero(-angle_radians);
    }
    let binormal = normalize_f64(cross_f64(to_f64(longitudinal_axis), to_f64(bend_direction)))
        .ok_or(BendValidationError::DegenerateBendDirection)?;

    Ok(ValidatedBendParameters {
        origin: parameters.origin,
        longitudinal_axis,
        bend_direction,
        binormal: canonicalize_signed_zeroes(to_f32(binormal)),
        angle_radians,
        interval_start: parameters.interval_start,
        interval_end: parameters.interval_end,
    })
}

fn evaluate_validated_bend_point(
    parameters: ValidatedBendParameters,
    position: [f32; 3],
) -> Result<[f32; 3], BendEvaluationError> {
    let origin = to_f64(parameters.origin);
    let axis = to_f64(parameters.longitudinal_axis);
    let direction = to_f64(parameters.bend_direction);
    let binormal = to_f64(parameters.binormal);
    let point = to_f64(position);
    let relative = sub_f64(point, origin);
    let s = dot_f64(relative, axis);
    let u = dot_f64(relative, direction);
    let v = dot_f64(relative, binormal);
    let s0 = f64::from(parameters.interval_start);
    let s1 = f64::from(parameters.interval_end);

    if s < s0 {
        return Ok(position);
    }

    let angle = f64::from(parameters.angle_radians);
    let length = s1 - s0;
    let curvature = angle / length;
    let bent = if s <= s1 {
        let t = s - s0;
        let phi = curvature * t;
        let center = add_f64(
            add_f64(origin, scale_f64(axis, s0)),
            add_f64(
                scale_f64(axis, sin_over_curvature(phi, curvature, t)),
                scale_f64(direction, one_minus_cos_over_curvature(phi, curvature, t)),
            ),
        );
        let normal = add_f64(scale_f64(axis, -phi.sin()), scale_f64(direction, phi.cos()));
        add_f64(
            add_f64(center, scale_f64(normal, u)),
            scale_f64(binormal, v),
        )
    } else {
        let center_end = add_f64(
            add_f64(origin, scale_f64(axis, s0)),
            add_f64(
                scale_f64(axis, sin_over_curvature(angle, curvature, length)),
                scale_f64(
                    direction,
                    one_minus_cos_over_curvature(angle, curvature, length),
                ),
            ),
        );
        let tangent_end = add_f64(
            scale_f64(axis, angle.cos()),
            scale_f64(direction, angle.sin()),
        );
        let normal_end = add_f64(
            scale_f64(axis, -angle.sin()),
            scale_f64(direction, angle.cos()),
        );
        add_f64(
            add_f64(center_end, scale_f64(tangent_end, s - s1)),
            add_f64(scale_f64(normal_end, u), scale_f64(binormal, v)),
        )
    };

    to_f32_checked(bent)
}

fn sin_over_curvature(phi: f64, curvature: f64, t: f64) -> f64 {
    if phi.abs() <= SERIES_ANGLE_EPSILON {
        let phi_squared = phi * phi;
        t * (1.0 - phi_squared / 6.0 + phi_squared * phi_squared / 120.0
            - phi_squared * phi_squared * phi_squared / 5040.0)
    } else {
        phi.sin() / curvature
    }
}

fn one_minus_cos_over_curvature(phi: f64, curvature: f64, t: f64) -> f64 {
    if phi.abs() <= SERIES_ANGLE_EPSILON {
        let phi_squared = phi * phi;
        curvature
            * t
            * t
            * (0.5 - phi_squared / 24.0 + phi_squared * phi_squared / 720.0
                - phi_squared * phi_squared * phi_squared / 40320.0)
    } else {
        (1.0 - phi.cos()) / curvature
    }
}

fn intrinsic_shape_scale(left: &[[f32; 3]], right: &[[f32; 3]]) -> f64 {
    rms_radius(left)
        .max(rms_radius(right))
        .max(bounding_box_diagonal(left))
        .max(bounding_box_diagonal(right))
}

fn rms_radius(positions: &[[f32; 3]]) -> f64 {
    if positions.is_empty() {
        return 0.0;
    }
    let centroid = centroid_f64(positions);
    let sum_squared = positions
        .iter()
        .map(|position| {
            let delta = [
                f64::from(position[0]) - centroid[0],
                f64::from(position[1]) - centroid[1],
                f64::from(position[2]) - centroid[2],
            ];
            dot_f64(delta, delta)
        })
        .sum::<f64>();
    (sum_squared / positions.len() as f64).sqrt()
}

fn bounding_box_diagonal(positions: &[[f32; 3]]) -> f64 {
    let Some(first) = positions.first() else {
        return 0.0;
    };
    let mut min = to_f64(*first);
    let mut max = min;
    for position in positions.iter().skip(1) {
        let position = to_f64(*position);
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    let delta = sub_f64(max, min);
    dot_f64(delta, delta).sqrt()
}

fn centroid_f64(positions: &[[f32; 3]]) -> [f64; 3] {
    if positions.is_empty() {
        return [0.0, 0.0, 0.0];
    }
    let sum = positions.iter().fold([0.0, 0.0, 0.0], |sum, position| {
        add_f64(sum, to_f64(*position))
    });
    scale_f64(sum, (positions.len() as f64).recip())
}

fn position_local_coordinate_ulp(
    left: [f32; 3],
    right: [f32; 3],
    left_centroid: [f64; 3],
    right_centroid: [f64; 3],
) -> f64 {
    (0..3)
        .flat_map(|axis| {
            [
                (f64::from(left[axis]) - left_centroid[axis]) as f32,
                (f64::from(right[axis]) - right_centroid[axis]) as f32,
            ]
        })
        .map(f32_ulp)
        .fold(0.0_f64, f64::max)
}

fn f32_ulp(value: f32) -> f64 {
    if !value.is_finite() {
        return f64::INFINITY;
    }
    if value == 0.0 {
        return f64::from(f32::from_bits(1));
    }
    let bits = value.to_bits();
    let next_bits = if value.is_sign_negative() {
        bits.wrapping_sub(1)
    } else {
        bits.wrapping_add(1)
    };
    let next = f32::from_bits(next_bits);
    if next.is_finite() {
        (f64::from(next) - f64::from(value)).abs()
    } else {
        let previous_bits = if value.is_sign_negative() {
            bits.wrapping_add(1)
        } else {
            bits.wrapping_sub(1)
        };
        (f64::from(value) - f64::from(f32::from_bits(previous_bits))).abs()
    }
}

fn normalize_f64(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = length_f64(vector)?;
    if length <= MIN_AXIS_LENGTH {
        return None;
    }
    Some(scale_f64(vector, length.recip()))
}

fn length_f64(vector: [f64; 3]) -> Option<f64> {
    let length_squared = dot_f64(vector, vector);
    if !length_squared.is_finite() {
        return None;
    }
    let length = length_squared.sqrt();
    if !length.is_finite() {
        return None;
    }
    Some(length)
}

fn dot_f64(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross_f64(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn add_f64(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub_f64(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale_f64(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn to_f64(vector: [f32; 3]) -> [f64; 3] {
    [
        f64::from(vector[0]),
        f64::from(vector[1]),
        f64::from(vector[2]),
    ]
}

fn to_f32(vector: [f64; 3]) -> [f32; 3] {
    [vector[0] as f32, vector[1] as f32, vector[2] as f32]
}

fn to_f32_checked(vector: [f64; 3]) -> Result<[f32; 3], BendEvaluationError> {
    if vector.iter().any(|coordinate| !coordinate.is_finite()) {
        return Err(BendEvaluationError::NonFiniteOutput);
    }
    let converted = to_f32(vector);
    if converted.iter().any(|coordinate| !coordinate.is_finite()) {
        return Err(BendEvaluationError::NonFiniteOutput);
    }
    Ok(converted)
}

fn canonicalize_signed_zeroes(vector: [f32; 3]) -> [f32; 3] {
    [
        canonicalize_zero(vector[0]),
        canonicalize_zero(vector[1]),
        canonicalize_zero(vector[2]),
    ]
}

fn canonicalize_zero(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}

fn should_flip_direction_sign(direction: [f32; 3]) -> bool {
    direction
        .iter()
        .copied()
        .find(|component| *component != 0.0)
        .is_some_and(|component| component.is_sign_negative())
}

fn negate_f32(vector: [f32; 3]) -> [f32; 3] {
    [
        canonicalize_zero(-vector[0]),
        canonicalize_zero(-vector[1]),
        canonicalize_zero(-vector[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_frame_is_normalized_and_right_handed() {
        let validated = validate_bend_parameters(&BendParameters {
            origin: [1.0, 2.0, 3.0],
            longitudinal_axis: [0.0, 2.0, 0.0],
            bend_direction: [1.0, 1.0, 0.0],
            angle_radians: 0.5,
            interval_start: -1.0,
            interval_end: 1.0,
        })
        .unwrap();

        assert_eq!(validated.longitudinal_axis, [0.0, 1.0, 0.0]);
        assert_eq!(validated.bend_direction, [1.0, 0.0, 0.0]);
        assert_eq!(validated.binormal, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn near_zero_angle_has_exact_identity_path() {
        let parameters = BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [0.0, 1.0, 0.0],
            bend_direction: [1.0, 0.0, 0.0],
            angle_radians: 0.0,
            interval_start: 0.0,
            interval_end: 1.0,
        };
        let positions = [[1.0, 2.0, 3.0]];

        let evaluated = evaluate_bend(&parameters, &positions).unwrap();

        assert_eq!(evaluated, positions);
    }

    #[test]
    fn non_zero_bend_evaluates() {
        let evaluated = evaluate_bend_point(
            &BendParameters {
                origin: [0.0, 0.0, 0.0],
                longitudinal_axis: [1.0, 0.0, 0.0],
                bend_direction: [0.0, 1.0, 0.0],
                angle_radians: std::f32::consts::FRAC_PI_2,
                interval_start: 0.0,
                interval_end: 1.0,
            },
            [1.0, 0.0, 0.0],
        )
        .unwrap();

        let expected = 2.0 / std::f32::consts::PI;
        assert!((evaluated[0] - expected).abs() <= 1.0e-6);
        assert!((evaluated[1] - expected).abs() <= 1.0e-6);
        assert_eq!(evaluated[2], 0.0);
    }
}
