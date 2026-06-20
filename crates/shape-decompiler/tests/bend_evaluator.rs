#![forbid(unsafe_code)]

use shape_decompiler::v3::bend::{
    BendParameters, BendStageVerificationPolicy, BendValidationError, compare_bend_to_baked_stage,
    evaluate_bend, evaluate_bend_point, validate_bend_parameters,
};

const CLOSE_EPSILON: f32 = 2.0e-6;

fn bend_x_to_y(angle_radians: f32) -> BendParameters {
    BendParameters {
        origin: [0.0, 0.0, 0.0],
        longitudinal_axis: [1.0, 0.0, 0.0],
        bend_direction: [0.0, 1.0, 0.0],
        angle_radians,
        interval_start: 0.0,
        interval_end: 1.0,
    }
}

fn assert_bits_eq(actual: [f32; 3], expected: [f32; 3]) {
    assert_eq!(actual[0].to_bits(), expected[0].to_bits());
    assert_eq!(actual[1].to_bits(), expected[1].to_bits());
    assert_eq!(actual[2].to_bits(), expected[2].to_bits());
}

fn assert_close(actual: [f32; 3], expected: [f32; 3], epsilon: f32) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= epsilon,
            "axis {axis}: actual {:?}, expected {:?}, epsilon {epsilon}",
            actual,
            expected
        );
    }
}

fn distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
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

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    scale(vector, length.recip())
}

fn next_f32(value: f32) -> f32 {
    let bits = value.to_bits();
    if value.is_sign_negative() {
        f32::from_bits(bits.wrapping_sub(1))
    } else {
        f32::from_bits(bits.wrapping_add(1))
    }
}

fn random_unit(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let mantissa = ((*state >> 40) as u32) & 0x00ff_ffff;
    (mantissa as f32 / 0x00ff_ffff as f32) * 2.0 - 1.0
}

#[test]
fn zero_angle_returns_identical_f32_bits() {
    let parameters = bend_x_to_y(5.0e-8);
    let positions = [
        [1.25, -0.0, f32::from_bits(1)],
        [
            f32::from_bits(0xbf7f_ffff),
            2048.5,
            f32::from_bits(0x8000_0000),
        ],
    ];

    let evaluated = evaluate_bend(&parameters, &positions).unwrap();

    assert_bits_eq(evaluated[0], positions[0]);
    assert_bits_eq(evaluated[1], positions[1]);
}

#[test]
fn positive_ninety_degree_bend_matches_neutral_axis_points() {
    let parameters = bend_x_to_y(std::f32::consts::FRAC_PI_2);
    let points = [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [1.0, 0.0, 0.0]];

    let evaluated = evaluate_bend(&parameters, &points).unwrap();

    let half_phi_x = (2.0_f32.sqrt()) / std::f32::consts::PI;
    let half_phi_y = (1.0 - 0.5_f32.sqrt()) * 2.0 / std::f32::consts::PI;
    let end = 2.0 / std::f32::consts::PI;
    assert_close(evaluated[0], [0.0, 0.0, 0.0], CLOSE_EPSILON);
    assert_close(evaluated[1], [half_phi_x, half_phi_y, 0.0], CLOSE_EPSILON);
    assert_close(evaluated[2], [end, end, 0.0], CLOSE_EPSILON);
}

#[test]
fn negative_bend_turns_toward_negative_direction() {
    let parameters = bend_x_to_y(-std::f32::consts::FRAC_PI_2);

    let evaluated = evaluate_bend_point(&parameters, [1.0, 0.0, 0.0]).unwrap();

    let radius = 2.0 / std::f32::consts::PI;
    assert_close(evaluated, [radius, -radius, 0.0], CLOSE_EPSILON);
}

#[test]
fn points_before_interval_remain_bit_identical() {
    let parameters = bend_x_to_y(std::f32::consts::FRAC_PI_2);
    let position = [-0.25, f32::from_bits(0x8000_0000), 3.5];

    let evaluated = evaluate_bend_point(&parameters, position).unwrap();

    assert_bits_eq(evaluated, position);
}

#[test]
fn position_is_continuous_at_interval_start() {
    let parameters = bend_x_to_y(1.25);
    let h = 1.0e-4;
    let before = evaluate_bend_point(&parameters, [-h, 0.0, 0.0]).unwrap();
    let at_start = evaluate_bend_point(&parameters, [0.0, 0.0, 0.0]).unwrap();
    let after = evaluate_bend_point(&parameters, [h, 0.0, 0.0]).unwrap();

    assert!(distance(before, at_start) <= 2.0 * h);
    assert!(distance(after, at_start) <= 2.0 * h);
}

#[test]
fn position_is_continuous_at_interval_end() {
    let parameters = bend_x_to_y(1.25);
    let h = 1.0e-4;
    let before = evaluate_bend_point(&parameters, [1.0 - h, 0.0, 0.0]).unwrap();
    let at_end = evaluate_bend_point(&parameters, [1.0, 0.0, 0.0]).unwrap();
    let after = evaluate_bend_point(&parameters, [1.0 + h, 0.0, 0.0]).unwrap();

    assert!(distance(before, at_end) <= 2.0 * h);
    assert!(distance(after, at_end) <= 2.0 * h);
}

#[test]
fn neutral_axis_tangent_is_continuous_by_finite_difference() {
    let angle = 0.9;
    let parameters = bend_x_to_y(angle);
    let h = 1.0e-3;
    let start_left = scale(
        sub(
            evaluate_bend_point(&parameters, [0.0, 0.0, 0.0]).unwrap(),
            evaluate_bend_point(&parameters, [-h, 0.0, 0.0]).unwrap(),
        ),
        h.recip(),
    );
    let start_right = scale(
        sub(
            evaluate_bend_point(&parameters, [h, 0.0, 0.0]).unwrap(),
            evaluate_bend_point(&parameters, [0.0, 0.0, 0.0]).unwrap(),
        ),
        h.recip(),
    );
    let end_left = scale(
        sub(
            evaluate_bend_point(&parameters, [1.0, 0.0, 0.0]).unwrap(),
            evaluate_bend_point(&parameters, [1.0 - h, 0.0, 0.0]).unwrap(),
        ),
        h.recip(),
    );
    let end_right = scale(
        sub(
            evaluate_bend_point(&parameters, [1.0 + h, 0.0, 0.0]).unwrap(),
            evaluate_bend_point(&parameters, [1.0, 0.0, 0.0]).unwrap(),
        ),
        h.recip(),
    );

    assert_close(start_left, [1.0, 0.0, 0.0], 1.0e-3);
    assert_close(start_right, [1.0, 0.0, 0.0], 1.0e-3);
    assert_close(end_left, [angle.cos(), angle.sin(), 0.0], 2.0e-3);
    assert_close(end_right, [angle.cos(), angle.sin(), 0.0], 2.0e-3);
}

#[test]
fn arbitrary_translated_origin_is_supported() {
    let parameters = BendParameters {
        origin: [10.0, -5.0, 2.0],
        longitudinal_axis: [1.0, 0.0, 0.0],
        bend_direction: [0.0, 1.0, 0.0],
        angle_radians: std::f32::consts::FRAC_PI_2,
        interval_start: 2.0,
        interval_end: 4.0,
    };

    let evaluated = evaluate_bend_point(&parameters, [13.0, -5.0, 2.0]).unwrap();

    let expected_x = 12.0 + 2.0 * 2.0_f32.sqrt() / std::f32::consts::PI;
    let expected_y = -5.0 + 4.0 * (1.0 - 0.5_f32.sqrt()) / std::f32::consts::PI;
    assert_close(evaluated, [expected_x, expected_y, 2.0], 3.0e-6);
}

#[test]
fn arbitrary_normalized_axis_is_supported() {
    let parameters = BendParameters {
        origin: [0.0, 0.0, 0.0],
        longitudinal_axis: [0.0, 1.0, 0.0],
        bend_direction: [0.0, 0.0, 1.0],
        angle_radians: std::f32::consts::FRAC_PI_2,
        interval_start: 0.0,
        interval_end: 2.0,
    };

    let evaluated = evaluate_bend_point(&parameters, [0.0, 2.0, 0.0]).unwrap();

    let end = 4.0 / std::f32::consts::PI;
    assert_close(evaluated, [0.0, end, end], CLOSE_EPSILON);
}

#[test]
fn nonorthogonal_input_direction_is_orthogonalized() {
    let mut nonorthogonal = bend_x_to_y(0.8);
    nonorthogonal.bend_direction = [5.0, 1.0, 0.0];
    let orthogonal = bend_x_to_y(0.8);

    let validated = validate_bend_parameters(&nonorthogonal).unwrap();
    let evaluated_nonorthogonal = evaluate_bend_point(&nonorthogonal, [0.75, 0.25, 0.0]).unwrap();
    let evaluated_orthogonal = evaluate_bend_point(&orthogonal, [0.75, 0.25, 0.0]).unwrap();

    assert_close(validated.bend_direction, [0.0, 1.0, 0.0], f32::EPSILON);
    assert_bits_eq(evaluated_nonorthogonal, evaluated_orthogonal);
}

#[test]
fn large_coordinate_offsets_remain_finite() {
    let parameters = BendParameters {
        origin: [100_000_000.0, -100_000_000.0, 50_000_000.0],
        longitudinal_axis: [1.0, 0.0, 0.0],
        bend_direction: [0.0, 1.0, 0.0],
        angle_radians: 0.75,
        interval_start: 0.0,
        interval_end: 1024.0,
    };
    let source = [[100_000_512.0, -99_999_872.0, 50_000_000.0]];

    let baked = evaluate_bend(&parameters, &source).unwrap();
    let report = compare_bend_to_baked_stage(
        &parameters,
        &source,
        &baked,
        BendStageVerificationPolicy::default(),
    )
    .unwrap();

    assert!(
        baked
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
    );
    assert!(report.passed);
}

#[test]
fn object_scales_are_supported() {
    for scale_value in [1.0e-3_f32, 1.0, 1.0e3] {
        let parameters = BendParameters {
            origin: [0.0, 0.0, 0.0],
            longitudinal_axis: [1.0, 0.0, 0.0],
            bend_direction: [0.0, 1.0, 0.0],
            angle_radians: std::f32::consts::FRAC_PI_2,
            interval_start: 0.0,
            interval_end: scale_value,
        };

        let evaluated = evaluate_bend_point(&parameters, [scale_value, 0.0, 0.0]).unwrap();

        let end = 2.0 * scale_value / std::f32::consts::PI;
        assert_close(
            evaluated,
            [end, end, 0.0],
            scale_value.abs() * 2.0e-6 + 1.0e-8,
        );
    }
}

#[test]
fn near_zero_angle_is_stable() {
    let exact_identity = bend_x_to_y(5.0e-8);
    let position = [0.75, 0.125, -0.25];

    let identity_evaluated = evaluate_bend_point(&exact_identity, position).unwrap();
    assert_bits_eq(identity_evaluated, position);

    let near_zero = bend_x_to_y(2.0e-7);
    let evaluated = evaluate_bend_point(&near_zero, [1.0, 0.0, 0.0]).unwrap();
    assert!(evaluated.iter().all(|coordinate| coordinate.is_finite()));
    assert!((evaluated[0] - 1.0).abs() <= 1.0e-6);
    assert!(evaluated[1].abs() <= 1.0e-5);
}

#[test]
fn invalid_intervals_and_frames_are_rejected() {
    let mut parameters = bend_x_to_y(0.5);
    parameters.interval_end = parameters.interval_start;
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::InvalidInterval
    );

    let mut parameters = bend_x_to_y(0.5);
    parameters.interval_end = -1.0;
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::InvalidInterval
    );

    let mut parameters = bend_x_to_y(0.5);
    parameters.longitudinal_axis = [0.0, 0.0, 0.0];
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::DegenerateLongitudinalAxis
    );

    let mut parameters = bend_x_to_y(0.5);
    parameters.bend_direction = [1.0, 1.0e-8, 0.0];
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::DegenerateBendDirection
    );

    let mut parameters = bend_x_to_y(std::f32::consts::PI + 1.0e-4);
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::AngleOutOfRange
    );

    parameters = bend_x_to_y(f32::NAN);
    assert_eq!(
        validate_bend_parameters(&parameters).unwrap_err(),
        BendValidationError::NonFiniteParameter
    );
}

#[test]
fn deterministic_results_and_direction_sign_canonicalization() {
    let parameters = bend_x_to_y(0.9);
    let equivalent = BendParameters {
        bend_direction: [0.0, -1.0, 0.0],
        angle_radians: -0.9,
        ..parameters
    };
    let point = [1.4, 0.25, -0.5];

    let first = evaluate_bend_point(&parameters, point).unwrap();
    let second = evaluate_bend_point(&parameters, point).unwrap();
    let equivalent_result = evaluate_bend_point(&equivalent, point).unwrap();

    assert_bits_eq(first, second);
    assert_bits_eq(first, equivalent_result);
}

#[test]
fn semantic_stage_verification_uses_local_ulp_floor() {
    let parameters = bend_x_to_y(0.0);
    let source = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let mut baked = source;
    baked[1][0] = next_f32(baked[1][0]);

    let default_report = compare_bend_to_baked_stage(
        &parameters,
        &source,
        &baked,
        BendStageVerificationPolicy::default(),
    )
    .unwrap();
    let strict_report = compare_bend_to_baked_stage(
        &parameters,
        &source,
        &baked,
        BendStageVerificationPolicy {
            absolute_epsilon: 0.0,
            shape_relative_epsilon: 0.0,
            ulp_multiplier: 0.0,
        },
    )
    .unwrap();

    assert!(default_report.passed);
    assert_eq!(strict_report.outside_tolerance, 1);
    assert!(!strict_report.passed);
}

#[test]
fn seeded_parameter_corpus_never_emits_non_finite_output() {
    let mut state = 0x5eed_1234_5678_9abc_u64;
    for _ in 0..256 {
        let mut axis = [
            random_unit(&mut state),
            random_unit(&mut state),
            random_unit(&mut state),
        ];
        if dot(axis, axis) < 1.0e-4 {
            axis[0] += 0.25;
        }
        let axis = normalize(axis);
        let helper = if axis[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let direction = normalize(cross(helper, axis));
        let origin = [
            random_unit(&mut state) * 1000.0,
            random_unit(&mut state) * 1000.0,
            random_unit(&mut state) * 1000.0,
        ];
        let interval_start = random_unit(&mut state) * 5.0;
        let interval_length = 1.0e-3 + (random_unit(&mut state).abs() * 10.0);
        let mut angle = random_unit(&mut state) * std::f32::consts::PI;
        if angle.abs() <= 1.0e-5 {
            angle = 0.25;
        }
        let parameters = BendParameters {
            origin,
            longitudinal_axis: axis,
            bend_direction: direction,
            angle_radians: angle,
            interval_start,
            interval_end: interval_start + interval_length,
        };
        let validated = validate_bend_parameters(&parameters).unwrap();
        let samples = [
            interval_start - interval_length * 0.5,
            interval_start,
            interval_start + interval_length * 0.5,
            interval_start + interval_length,
            interval_start + interval_length * 1.5,
        ];
        let positions = samples.map(|s| {
            let u = random_unit(&mut state) * interval_length * 0.25;
            let v = random_unit(&mut state) * interval_length * 0.25;
            add(
                add(
                    add(origin, scale(validated.longitudinal_axis, s)),
                    scale(validated.bend_direction, u),
                ),
                scale(validated.binormal, v),
            )
        });

        let evaluated = evaluate_bend(&parameters, &positions).unwrap();

        assert!(
            evaluated
                .iter()
                .flatten()
                .all(|coordinate| coordinate.is_finite())
        );
    }
}
