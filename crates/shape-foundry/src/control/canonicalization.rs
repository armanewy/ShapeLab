
fn canonicalize_control_value_with_domain(
    control: &CustomizerControl,
    domain: &FeasibleControlDomain,
    value: ControlValue,
) -> Result<ControlValue, ControlEvaluationError> {
    if !control_value_matches_kind(&control.kind, &value) {
        return Err(ControlEvaluationError::WrongValueKind {
            control_id: control.id.clone(),
        });
    }
    if let ControlValue::Scalar(value) = value
        && !value.is_finite()
    {
        return Err(ControlEvaluationError::NonFiniteControlValue {
            control_id: control.id.clone(),
        });
    }
    if let Some(reason) = domain.unavailable_reason(&value) {
        return Err(ControlEvaluationError::UnavailableOption {
            control_id: control.id.clone(),
            option: value.option_key(),
            reason: reason.to_owned(),
        });
    }
    match (&control.kind, value) {
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(value)) => Ok(
            ControlValue::Scalar(canonical_scalar(&control.id, domain, value)?),
        ),
        (ControlKind::IntegerStepper { .. }, ControlValue::Integer(value)) => Ok(
            ControlValue::Integer(canonical_integer(&control.id, domain, value)?),
        ),
        (ControlKind::Toggle { .. }, ControlValue::Toggle(value)) => {
            let candidate = ControlValue::Toggle(value);
            if domain.contains_available_value(&candidate) {
                Ok(candidate)
            } else {
                first_available_value(domain).ok_or_else(|| {
                    ControlEvaluationError::EmptyFeasibleDomain {
                        control_id: control.id.clone(),
                    }
                })
            }
        }
        (ControlKind::ChoiceGallery { options }, ControlValue::Choice(value)) => {
            if !options.iter().any(|option| option.value == value) {
                return Err(ControlEvaluationError::UnknownOption {
                    control_id: control.id.clone(),
                    option: value,
                });
            }
            let candidate = ControlValue::Choice(value);
            if domain.contains_available_value(&candidate) {
                Ok(candidate)
            } else {
                Err(ControlEvaluationError::EmptyFeasibleDomain {
                    control_id: control.id.clone(),
                })
            }
        }
        (ControlKind::ProviderGallery { options, .. }, ControlValue::Provider(value)) => {
            if !options.iter().any(|option| option.provider_id == value) {
                return Err(ControlEvaluationError::UnknownOption {
                    control_id: control.id.clone(),
                    option: value,
                });
            }
            let candidate = ControlValue::Provider(value);
            if domain.contains_available_value(&candidate) {
                Ok(candidate)
            } else {
                Err(ControlEvaluationError::EmptyFeasibleDomain {
                    control_id: control.id.clone(),
                })
            }
        }
        _ => Err(ControlEvaluationError::WrongValueKind {
            control_id: control.id.clone(),
        }),
    }
}

fn canonical_scalar(
    control_id: &str,
    domain: &FeasibleControlDomain,
    value: f32,
) -> Result<f32, ControlEvaluationError> {
    if !value.is_finite() {
        return Err(ControlEvaluationError::NonFiniteControlValue {
            control_id: control_id.to_owned(),
        });
    }
    if domain
        .continuous_intervals
        .iter()
        .any(|interval| interval.minimum <= value && value <= interval.maximum)
    {
        return Ok(value);
    }
    let mut best = None::<f32>;
    for interval in &domain.continuous_intervals {
        for endpoint in [interval.minimum, interval.maximum] {
            best = Some(match best {
                Some(current) if (current - value).abs() <= (endpoint - value).abs() => current,
                _ => endpoint,
            });
        }
    }
    for candidate in &domain.discrete_values {
        if let ControlValue::Scalar(candidate) = candidate {
            best = Some(match best {
                Some(current) if (current - value).abs() <= (*candidate - value).abs() => current,
                _ => *candidate,
            });
        }
    }
    best.ok_or_else(|| ControlEvaluationError::EmptyFeasibleDomain {
        control_id: control_id.to_owned(),
    })
}

fn canonical_integer(
    control_id: &str,
    domain: &FeasibleControlDomain,
    value: i64,
) -> Result<i64, ControlEvaluationError> {
    let mut best = None::<i64>;
    for candidate in &domain.discrete_values {
        if let ControlValue::Integer(candidate) = candidate {
            best = Some(match best {
                Some(current)
                    if integer_distance(current, value) <= integer_distance(*candidate, value) =>
                {
                    current
                }
                _ => *candidate,
            });
        }
    }
    best.ok_or_else(|| ControlEvaluationError::EmptyFeasibleDomain {
        control_id: control_id.to_owned(),
    })
}

fn integer_distance(left: i64, right: i64) -> u64 {
    left.abs_diff(right)
}

fn evaluate_binding_value(
    control: &CustomizerControl,
    binding: &ControlSlotBinding,
    slot: &FamilyParameterSlot,
    value: &ControlValue,
) -> Result<FamilyValue, ControlEvaluationError> {
    match (&control.kind, value, &slot.kind) {
        (
            ControlKind::ContinuousAxis { .. },
            ControlValue::Scalar(value),
            FamilyParameterKind::Length { .. }
            | FamilyParameterKind::Ratio
            | FamilyParameterKind::Angle { .. }
            | FamilyParameterKind::Custom(_),
        ) => {
            let output = binding.response.evaluate(*value).ok_or_else(|| {
                ControlEvaluationError::NonFiniteControlOutput {
                    control_id: control.id.clone(),
                    slot: binding.slot.clone(),
                }
            })?;
            if let Some(range) = slot.range
                && (output < range.minimum || output > range.maximum)
            {
                return Err(ControlEvaluationError::EmptyFeasibleDomain {
                    control_id: control.id.clone(),
                });
            }
            Ok(FamilyValue::Scalar(output))
        }
        (
            ControlKind::IntegerStepper { .. },
            ControlValue::Integer(value),
            FamilyParameterKind::Count,
        )
        | (
            ControlKind::IntegerStepper { .. },
            ControlValue::Integer(value),
            FamilyParameterKind::Custom(_),
        ) => {
            let output = binding.response.evaluate(*value as f32).ok_or_else(|| {
                ControlEvaluationError::NonFiniteControlOutput {
                    control_id: control.id.clone(),
                    slot: binding.slot.clone(),
                }
            })?;
            let snapped = output.round();
            if !snapped.is_finite() || snapped < 0.0 || snapped > u32::MAX as f32 {
                return Err(ControlEvaluationError::NonFiniteControlOutput {
                    control_id: control.id.clone(),
                    slot: binding.slot.clone(),
                });
            }
            if let Some(range) = slot.range
                && (snapped < range.minimum || snapped > range.maximum)
            {
                return Err(ControlEvaluationError::EmptyFeasibleDomain {
                    control_id: control.id.clone(),
                });
            }
            Ok(FamilyValue::Integer(snapped as u32))
        }
        (ControlKind::Toggle { .. }, ControlValue::Toggle(value), FamilyParameterKind::Toggle) => {
            Ok(FamilyValue::Toggle(*value))
        }
        (
            ControlKind::ChoiceGallery { .. },
            ControlValue::Choice(value),
            FamilyParameterKind::Choice(choices),
        ) if choices.iter().any(|choice| choice == value) => Ok(FamilyValue::Choice(value.clone())),
        _ => Err(ControlEvaluationError::IncompatibleFamilySlot {
            control_id: control.id.clone(),
            slot: binding.slot.clone(),
        }),
    }
}

impl ResponseCurve {
    /// Evaluate a response curve and reject non-finite output.
    #[must_use]
    pub fn evaluate(&self, input: f32) -> Option<f32> {
        if !input.is_finite() {
            return None;
        }
        let output = match self {
            Self::Linear => input,
            Self::Piecewise { points, .. } => evaluate_piecewise(points, input)?,
        };
        output.is_finite().then_some(output)
    }
}

fn evaluate_piecewise(points: &[[f32; 2]], input: f32) -> Option<f32> {
    let first = points.first()?;
    let last = points.last()?;
    if input <= first[0] {
        return Some(first[1]);
    }
    if input >= last[0] {
        return Some(last[1]);
    }
    for window in points.windows(2) {
        let [left_input, left_output] = window[0];
        let [right_input, right_output] = window[1];
        if left_input <= input && input <= right_input {
            let span = right_input - left_input;
            if span <= 0.0 || !span.is_finite() {
                return None;
            }
            let t = (input - left_input) / span;
            return Some(left_output + (right_output - left_output) * t);
        }
    }
    None
}
