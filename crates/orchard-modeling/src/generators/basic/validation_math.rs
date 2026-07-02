
fn bevel_profile(
    definition: &PartDefinition,
    default_radius: f32,
    default_segments: u32,
) -> (f32, u32) {
    definition.geometry.operations.iter().fold(
        (default_radius, default_segments),
        |profile, operation| match operation {
            ModelingOperationSpec::SetBevelProfile {
                radius, segments, ..
            } => (*radius, *segments),
            _ => profile,
        },
    )
}

fn positive_triplet(values: [f32; 3], label: &'static str) -> Result<[f32; 3], ModelingError> {
    for value in values {
        finite_positive(value, label)?;
    }
    Ok(values)
}

fn finite_positive(value: f32, label: &'static str) -> Result<f32, ModelingError> {
    if value.is_finite() && value > EPSILON {
        Ok(value)
    } else {
        Err(ModelingError::InvalidInput(format!(
            "{label} must be finite and positive"
        )))
    }
}

fn finite_non_negative(value: f32, label: &'static str) -> Result<f32, ModelingError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(ModelingError::InvalidInput(format!(
            "{label} must be finite and non-negative"
        )))
    }
}

fn dedup_sorted_f32(mut values: Vec<f32>) -> Vec<f32> {
    values.sort_by(f32::total_cmp);
    values.dedup_by(|left, right| (*left - *right).abs() <= EPSILON);
    values
}

fn region_transition(
    first: Option<RegionId>,
    second: Option<RegionId>,
) -> Option<(RegionId, RegionId)> {
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(if first <= second {
            (first, second)
        } else {
            (second, first)
        }),
        _ => None,
    }
}

fn has_duplicate_indices(vertices: &[u32]) -> bool {
    let mut seen = BTreeSet::new();
    vertices.iter().any(|vertex| !seen.insert(*vertex))
}

fn quantize(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn negate(value: [f32; 3]) -> [f32; 3] {
    [-value[0], -value[1], -value[2]]
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
