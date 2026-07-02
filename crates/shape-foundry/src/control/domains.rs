
fn family_control_domain(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> Result<FeasibleControlDomain, ControlEvaluationError> {
    let mut domain = kind_domain(control);
    if matches!(control.kind, ControlKind::ProviderGallery { .. }) {
        return Ok(domain);
    }
    for binding in &control.bindings {
        let slot =
            find_family_slot(context.family_parameter_slots, &binding.slot).ok_or_else(|| {
                ControlEvaluationError::UnknownFamilySlot {
                    control_id: control.id.clone(),
                    slot: binding.slot.clone(),
                }
            })?;
        let binding_domain = binding_family_control_domain(control, binding, slot)?;
        domain = intersect_feasible_domains(&domain, &binding_domain);
    }
    Ok(domain)
}

fn kind_domain(control: &CustomizerControl) -> FeasibleControlDomain {
    match &control.kind {
        ControlKind::ContinuousAxis { .. } => control.domain.clone(),
        ControlKind::IntegerStepper { .. } => control.domain.clone(),
        ControlKind::Toggle { .. } => FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![ControlValue::Toggle(false), ControlValue::Toggle(true)],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        ControlKind::ChoiceGallery { options } => FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: options
                .iter()
                .map(|option| ControlValue::Choice(option.value.clone()))
                .collect(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        ControlKind::ProviderGallery { options, .. } => FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: options
                .iter()
                .map(|option| ControlValue::Provider(option.provider_id.clone()))
                .collect(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
    }
}

fn binding_family_control_domain(
    control: &CustomizerControl,
    binding: &ControlSlotBinding,
    slot: &FamilyParameterSlot,
) -> Result<FeasibleControlDomain, ControlEvaluationError> {
    match (&control.kind, &slot.kind) {
        (
            ControlKind::ContinuousAxis { .. },
            FamilyParameterKind::Length { .. }
            | FamilyParameterKind::Ratio
            | FamilyParameterKind::Angle { .. }
            | FamilyParameterKind::Custom(_),
        ) => {
            let Some(range) = slot.range else {
                return Ok(kind_domain(control));
            };
            Ok(inverse_numeric_domain(
                control,
                binding,
                range.minimum,
                range.maximum,
            )?)
        }
        (ControlKind::IntegerStepper { .. }, FamilyParameterKind::Count)
        | (ControlKind::IntegerStepper { .. }, FamilyParameterKind::Custom(_)) => {
            let Some(range) = slot.range else {
                return Ok(kind_domain(control));
            };
            integer_binding_domain(control, binding, range.minimum, range.maximum)
        }
        (ControlKind::Toggle { .. }, FamilyParameterKind::Toggle) => Ok(kind_domain(control)),
        (ControlKind::ChoiceGallery { .. }, FamilyParameterKind::Choice(choices)) => {
            Ok(FeasibleControlDomain {
                continuous_intervals: Vec::new(),
                discrete_values: choices
                    .iter()
                    .map(|choice| ControlValue::Choice(choice.clone()))
                    .collect(),
                unavailable_options: BTreeMap::new(),
                certification: DomainCertification::DiscreteSamples,
            })
        }
        _ => Err(ControlEvaluationError::IncompatibleFamilySlot {
            control_id: control.id.clone(),
            slot: binding.slot.clone(),
        }),
    }
}

fn integer_binding_domain(
    control: &CustomizerControl,
    binding: &ControlSlotBinding,
    minimum: f32,
    maximum: f32,
) -> Result<FeasibleControlDomain, ControlEvaluationError> {
    let mut values = Vec::new();
    for value in &control.domain.discrete_values {
        let ControlValue::Integer(integer) = value else {
            continue;
        };
        let output = binding.response.evaluate(*integer as f32).ok_or_else(|| {
            ControlEvaluationError::NonFiniteControlOutput {
                control_id: control.id.clone(),
                slot: binding.slot.clone(),
            }
        })?;
        let snapped = output.round();
        if snapped.is_finite() && minimum <= snapped && snapped <= maximum {
            values.push(ControlValue::Integer(*integer));
        }
    }
    Ok(FeasibleControlDomain {
        continuous_intervals: Vec::new(),
        discrete_values: values,
        unavailable_options: BTreeMap::new(),
        certification: DomainCertification::DiscreteSamples,
    })
}

fn inverse_numeric_domain(
    control: &CustomizerControl,
    binding: &ControlSlotBinding,
    minimum: f32,
    maximum: f32,
) -> Result<FeasibleControlDomain, ControlEvaluationError> {
    let intervals = match &binding.response {
        ResponseCurve::Linear => vec![ClosedInterval { minimum, maximum }],
        ResponseCurve::Piecewise { points, .. } => {
            let mut intervals = Vec::new();
            for window in points.windows(2) {
                let [left_input, left_output] = window[0];
                let [right_input, right_output] = window[1];
                if !left_input.is_finite()
                    || !right_input.is_finite()
                    || !left_output.is_finite()
                    || !right_output.is_finite()
                {
                    return Err(ControlEvaluationError::NonFiniteControlOutput {
                        control_id: control.id.clone(),
                        slot: binding.slot.clone(),
                    });
                }
                let input_minimum = left_input.min(right_input);
                let input_maximum = left_input.max(right_input);
                let output_delta = right_output - left_output;
                if output_delta == 0.0 {
                    if minimum <= left_output && left_output <= maximum {
                        intervals.push(ClosedInterval {
                            minimum: input_minimum,
                            maximum: input_maximum,
                        });
                    }
                    continue;
                }
                if !output_delta.is_finite() {
                    return Err(ControlEvaluationError::NonFiniteControlOutput {
                        control_id: control.id.clone(),
                        slot: binding.slot.clone(),
                    });
                }
                let t0 = ((minimum - left_output) / output_delta).clamp(0.0, 1.0);
                let t1 = ((maximum - left_output) / output_delta).clamp(0.0, 1.0);
                let t_minimum = t0.min(t1);
                let t_maximum = t0.max(t1);
                let output_at_minimum = left_output + output_delta * t_minimum;
                let output_at_maximum = left_output + output_delta * t_maximum;
                if output_at_maximum < minimum || output_at_minimum > maximum {
                    continue;
                }
                intervals.push(ClosedInterval {
                    minimum: left_input + (right_input - left_input) * t_minimum,
                    maximum: left_input + (right_input - left_input) * t_maximum,
                });
            }
            intervals
        }
    };
    let mut domain = FeasibleControlDomain {
        continuous_intervals: intervals,
        discrete_values: Vec::new(),
        unavailable_options: BTreeMap::new(),
        certification: DomainCertification::CertifiedContinuous,
    };
    normalize_domain(&mut domain);
    Ok(domain)
}

fn intersect_feasible_domains(
    left: &FeasibleControlDomain,
    right: &FeasibleControlDomain,
) -> FeasibleControlDomain {
    let mut continuous_intervals = Vec::new();
    for left_interval in &left.continuous_intervals {
        for right_interval in &right.continuous_intervals {
            let minimum = left_interval.minimum.max(right_interval.minimum);
            let maximum = left_interval.maximum.min(right_interval.maximum);
            if minimum <= maximum {
                continuous_intervals.push(ClosedInterval { minimum, maximum });
            }
        }
    }

    let mut discrete_values = Vec::new();
    for value in &left.discrete_values {
        if right.contains_available_value(value) {
            discrete_values.push(value.clone());
        }
    }
    for value in &right.discrete_values {
        if left.contains_available_value(value) && !discrete_values.contains(value) {
            discrete_values.push(value.clone());
        }
    }

    let mut unavailable_options = left.unavailable_options.clone();
    for (option, reason) in &right.unavailable_options {
        unavailable_options
            .entry(option.clone())
            .and_modify(|existing| {
                if existing != reason {
                    *existing = format!("{existing}; {reason}");
                }
            })
            .or_insert_with(|| reason.clone());
    }

    let certification = combine_certification(
        &left.certification,
        &right.certification,
        !continuous_intervals.is_empty(),
    );
    let mut domain = FeasibleControlDomain {
        continuous_intervals,
        discrete_values,
        unavailable_options,
        certification,
    };
    normalize_domain(&mut domain);
    domain
}

fn combine_certification(
    left: &DomainCertification,
    right: &DomainCertification,
    has_continuous_values: bool,
) -> DomainCertification {
    if has_continuous_values
        && *left == DomainCertification::CertifiedContinuous
        && *right == DomainCertification::CertifiedContinuous
    {
        DomainCertification::CertifiedContinuous
    } else if matches!(left, DomainCertification::Uncertified { .. })
        || matches!(right, DomainCertification::Uncertified { .. })
    {
        DomainCertification::Uncertified {
            reason: "intersected with uncertified domain".to_owned(),
        }
    } else {
        DomainCertification::DiscreteSamples
    }
}

fn normalize_domain(domain: &mut FeasibleControlDomain) {
    domain
        .continuous_intervals
        .retain(|interval| interval.minimum.is_finite() && interval.maximum.is_finite());
    domain.continuous_intervals.sort_by(|left, right| {
        left.minimum
            .total_cmp(&right.minimum)
            .then(left.maximum.total_cmp(&right.maximum))
    });
    let mut merged = Vec::<ClosedInterval>::new();
    for interval in domain.continuous_intervals.drain(..) {
        if let Some(last) = merged.last_mut()
            && interval.minimum <= last.maximum
        {
            last.maximum = last.maximum.max(interval.maximum);
            continue;
        }
        if interval.minimum <= interval.maximum {
            merged.push(interval);
        }
    }
    domain.continuous_intervals = merged;
    sort_control_values(&mut domain.discrete_values);
    domain.discrete_values.dedup();
}

fn remove_unavailable_discrete_values(domain: &mut FeasibleControlDomain) {
    domain
        .discrete_values
        .retain(|value| !domain.unavailable_options.contains_key(&value.option_key()));
}
