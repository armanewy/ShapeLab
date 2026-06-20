//! Uniform-curvature bend operator contracts.
//!
//! The intended bend convention is:
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
//! - near-zero angles must have an exact identity path
//!
//! Formula evaluation is deliberately not implemented in this wave. The
//! near-zero identity path is implemented so callers can validate plumbing
//! without introducing approximate bend math yet.

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MIN_AXIS_LENGTH: f32 = 1.0e-6;
const IDENTITY_ANGLE_EPSILON: f32 = 1.0e-7;

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
    /// The interval end must not be before the interval start.
    #[error("bend interval_end must be greater than or equal to interval_start")]
    InvalidInterval,
}

/// Evaluation failures for bend operators.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BendEvaluationError {
    /// Parameter validation failed before evaluation.
    #[error(transparent)]
    Validation(#[from] BendValidationError),
    /// Non-zero uniform-curvature bend formulas are not implemented yet.
    #[error("uniform-curvature bend evaluation is not implemented yet")]
    NotImplemented,
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
///
/// Only the near-zero exact identity path is implemented. Non-zero angles
/// return [`BendEvaluationError::NotImplemented`] until the bend formulas land.
pub fn evaluate_bend(
    parameters: &BendParameters,
    positions: &[[f32; 3]],
) -> Result<Vec<[f32; 3]>, BendEvaluationError> {
    let validated = validate_bend_parameters(parameters)?;
    if validated.is_identity_angle() {
        return Ok(positions.to_vec());
    }
    Err(BendEvaluationError::NotImplemented)
}

/// Evaluates a single point under a bend.
///
/// Only the near-zero exact identity path is implemented. Non-zero angles
/// return [`BendEvaluationError::NotImplemented`] until the bend formulas land.
pub fn evaluate_bend_point(
    parameters: &BendParameters,
    position: [f32; 3],
) -> Result<[f32; 3], BendEvaluationError> {
    let validated = validate_bend_parameters(parameters)?;
    if validated.is_identity_angle() {
        return Ok(position);
    }
    Err(BendEvaluationError::NotImplemented)
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
    if parameters.interval_end < parameters.interval_start {
        return Err(BendValidationError::InvalidInterval);
    }

    let longitudinal_axis = normalize(parameters.longitudinal_axis)
        .ok_or(BendValidationError::DegenerateLongitudinalAxis)?;
    let direction_dot_axis = dot(parameters.bend_direction, longitudinal_axis);
    let projected_direction = sub(
        parameters.bend_direction,
        scale(longitudinal_axis, direction_dot_axis),
    );
    let bend_direction =
        normalize(projected_direction).ok_or(BendValidationError::DegenerateBendDirection)?;
    let binormal = normalize(cross(longitudinal_axis, bend_direction))
        .ok_or(BendValidationError::DegenerateBendDirection)?;

    Ok(ValidatedBendParameters {
        origin: parameters.origin,
        longitudinal_axis,
        bend_direction,
        binormal,
        angle_radians: parameters.angle_radians,
        interval_start: parameters.interval_start,
        interval_end: parameters.interval_end,
    })
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(vector, vector).sqrt();
    if length <= MIN_AXIS_LENGTH || !length.is_finite() {
        return None;
    }
    Some(scale(vector, length.recip()))
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn scale(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
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
    fn non_zero_bend_is_explicitly_not_implemented() {
        let error = evaluate_bend_point(
            &BendParameters {
                origin: [0.0, 0.0, 0.0],
                longitudinal_axis: [0.0, 1.0, 0.0],
                bend_direction: [1.0, 0.0, 0.0],
                angle_radians: 0.25,
                interval_start: 0.0,
                interval_end: 1.0,
            },
            [1.0, 2.0, 3.0],
        )
        .unwrap_err();

        assert_eq!(error, BendEvaluationError::NotImplemented);
    }
}
