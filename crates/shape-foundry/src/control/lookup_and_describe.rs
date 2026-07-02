
fn continuous_preview_values(
    domain: &FeasibleControlDomain,
    sample_count: usize,
) -> Vec<ControlValue> {
    let Some(first) = domain.continuous_intervals.first() else {
        return domain.discrete_values.clone();
    };
    let last = domain.continuous_intervals.last().unwrap_or(first);
    let minimum = first.minimum;
    let maximum = last.maximum;
    let sample_count = sample_count.max(1);
    if sample_count == 1 || minimum == maximum {
        return vec![ControlValue::Scalar((minimum + maximum) * 0.5)];
    }
    (0..sample_count)
        .map(|index| {
            let t = index as f32 / (sample_count - 1) as f32;
            ControlValue::Scalar(minimum + (maximum - minimum) * t)
        })
        .collect()
}

fn find_control<'a>(
    profile: &'a CustomizerProfile,
    control_id: &str,
) -> Option<&'a CustomizerControl> {
    profile
        .controls
        .iter()
        .find(|control| control.id == control_id)
}

fn find_family_slot<'a>(
    slots: &'a [FamilyParameterSlot],
    slot_id: &str,
) -> Option<&'a FamilyParameterSlot> {
    slots.iter().find(|slot| slot.id == slot_id)
}

fn first_available_value(domain: &FeasibleControlDomain) -> Option<ControlValue> {
    domain
        .discrete_values
        .iter()
        .find(|value| domain.contains_available_value(value))
        .cloned()
}

fn control_value_matches_kind(kind: &ControlKind, value: &ControlValue) -> bool {
    matches!(
        (kind, value),
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(_))
            | (ControlKind::IntegerStepper { .. }, ControlValue::Integer(_))
            | (ControlKind::Toggle { .. }, ControlValue::Toggle(_))
            | (ControlKind::ChoiceGallery { .. }, ControlValue::Choice(_))
            | (
                ControlKind::ProviderGallery { .. },
                ControlValue::Provider(_)
            )
    )
}

fn control_value_in_domain(domain: &FeasibleControlDomain, value: &ControlValue) -> bool {
    match value {
        ControlValue::Scalar(value) => {
            domain
                .continuous_intervals
                .iter()
                .any(|interval| interval.minimum <= *value && *value <= interval.maximum)
                || domain.discrete_values.iter().any(|candidate| {
                    matches!(candidate, ControlValue::Scalar(candidate) if candidate == value)
                })
        }
        ControlValue::Integer(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Integer(candidate) if candidate == value)),
        ControlValue::Toggle(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Toggle(candidate) if candidate == value)),
        ControlValue::Choice(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Choice(candidate) if candidate == value)),
        ControlValue::Provider(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Provider(candidate) if candidate == value)),
    }
}

fn evaluated_control_divergence(
    control: &CustomizerControl,
    effective_domain: &FeasibleControlDomain,
) -> ControlDivergence {
    if control.divergence != ControlDivergence::Synced {
        return control.divergence;
    }
    let mut authored_domain = control.domain.clone();
    normalize_domain(&mut authored_domain);
    remove_unavailable_discrete_values(&mut authored_domain);
    if authored_domain != *effective_domain {
        ControlDivergence::ConstraintLimited
    } else {
        ControlDivergence::Synced
    }
}

fn sort_control_values(values: &mut [ControlValue]) {
    values.sort_by(|left, right| {
        control_value_rank(left)
            .cmp(&control_value_rank(right))
            .then_with(|| match (left, right) {
                (ControlValue::Scalar(left), ControlValue::Scalar(right)) => left.total_cmp(right),
                (ControlValue::Integer(left), ControlValue::Integer(right)) => left.cmp(right),
                (ControlValue::Toggle(left), ControlValue::Toggle(right)) => left.cmp(right),
                (ControlValue::Choice(left), ControlValue::Choice(right))
                | (ControlValue::Provider(left), ControlValue::Provider(right)) => left.cmp(right),
                _ => std::cmp::Ordering::Equal,
            })
    });
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

fn describe_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => value.to_string(),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn describe_optional_family_value(value: Option<&FamilyValue>) -> String {
    value
        .map(describe_family_value)
        .unwrap_or_else(|| "<unset>".to_owned())
}

fn describe_family_value(value: &FamilyValue) -> String {
    match value {
        FamilyValue::Scalar(value) => value.to_string(),
        FamilyValue::Integer(value) => value.to_string(),
        FamilyValue::Toggle(value) => value.to_string(),
        FamilyValue::Choice(value) => value.clone(),
    }
}

impl From<FamilyDefaultValue> for ControlValue {
    fn from(value: FamilyDefaultValue) -> Self {
        match value {
            FamilyDefaultValue::Scalar(value) => Self::Scalar(value),
            FamilyDefaultValue::Integer(value) => Self::Integer(i64::from(value)),
            FamilyDefaultValue::Toggle(value) => Self::Toggle(value),
            FamilyDefaultValue::Choice(value) => Self::Choice(value),
        }
    }
}
