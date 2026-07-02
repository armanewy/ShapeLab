
fn evaluate_effective_family_request(
    document: &crate::FoundryAssetDocument,
    catalog: &FoundryResolvedCatalog,
) -> Result<(FamilyInstantiationRequest, Vec<ProviderOverrideRequest>), FoundryCompilationError> {
    let controls = catalog
        .customizer_profile
        .controls
        .iter()
        .map(|control| (control.id.as_str(), control))
        .collect::<BTreeMap<_, _>>();
    let mut parameters = BTreeMap::new();
    let mut provider_requests = document
        .provider_overrides
        .values()
        .map(|override_row| ProviderOverrideRequest {
            role: override_row.role.clone(),
            provider_id: override_row.provider_ref.stable_id.clone(),
        })
        .collect::<Vec<_>>();

    for (control_id, value) in &document.control_state {
        let Some(control) = controls.get(control_id.as_str()) else {
            return Err(FoundryCompilationError::UnknownControl {
                control_id: control_id.clone(),
            });
        };
        ensure_control_value_available(control_id, control, value)?;
        match (&control.kind, value) {
            (
                ControlKind::ProviderGallery { role, options },
                ControlValue::Provider(provider_id),
            ) => {
                if !options
                    .iter()
                    .any(|option| option.provider_id == *provider_id)
                {
                    return Err(FoundryCompilationError::UnknownProviderOption {
                        role: role.clone(),
                        provider_id: provider_id.clone(),
                    });
                }
                push_provider_request(
                    &mut provider_requests,
                    ProviderOverrideRequest {
                        role: role.clone(),
                        provider_id: provider_id.clone(),
                    },
                )?;
            }
            _ => {
                for binding in &control.bindings {
                    let family_value =
                        family_value_from_control(control_id, &control.kind, value, binding)?;
                    parameters.insert(binding.slot.clone(), family_value);
                }
            }
        }
    }

    let mut request = FamilyInstantiationRequest {
        family_id: catalog.family.id.clone(),
        style_kit_id: catalog.style_kit.id.clone(),
        parameters,
        seed: document.seed,
    };
    apply_family_parameter_defaults(&catalog.family, &mut request);

    Ok((request, provider_requests))
}

fn apply_family_parameter_defaults(
    family: &AssetFamilySchema,
    request: &mut FamilyInstantiationRequest,
) {
    for slot in &family.parameter_slots {
        if !request.parameters.contains_key(&slot.id)
            && let Some(default_value) = &slot.default_value
        {
            request
                .parameters
                .insert(slot.id.clone(), family_value_from_default(default_value));
        }
    }
}

fn family_value_from_default(default_value: &FamilyDefaultValue) -> FamilyValue {
    match default_value {
        FamilyDefaultValue::Scalar(value) => FamilyValue::Scalar(*value),
        FamilyDefaultValue::Integer(value) => FamilyValue::Integer(*value),
        FamilyDefaultValue::Toggle(value) => FamilyValue::Toggle(*value),
        FamilyDefaultValue::Choice(value) => FamilyValue::Choice(value.clone()),
    }
}

fn push_provider_request(
    requests: &mut Vec<ProviderOverrideRequest>,
    request: ProviderOverrideRequest,
) -> Result<(), FoundryCompilationError> {
    if let Some(existing) = requests
        .iter()
        .find(|existing| existing.role == request.role)
    {
        if existing.provider_id != request.provider_id {
            return Err(FoundryCompilationError::ProviderOverrideConflict {
                role: request.role,
                first: existing.provider_id.clone(),
                second: request.provider_id,
            });
        }
        return Ok(());
    }
    requests.push(request);
    Ok(())
}

fn ensure_control_value_available(
    control_id: &str,
    control: &crate::CustomizerControl,
    value: &ControlValue,
) -> Result<(), FoundryCompilationError> {
    if !control_value_matches_kind(&control.kind, value) {
        return Err(FoundryCompilationError::ControlValueKindMismatch {
            control_id: control_id.to_owned(),
        });
    }
    if control
        .domain
        .discrete_values
        .iter()
        .any(|allowed| allowed == value)
    {
        return Ok(());
    }
    if let ControlValue::Scalar(value) = value
        && control
            .domain
            .continuous_intervals
            .iter()
            .any(|interval| *value >= interval.minimum && *value <= interval.maximum)
    {
        return Ok(());
    }
    if control.domain.discrete_values.is_empty() && control.domain.continuous_intervals.is_empty() {
        return Ok(());
    }
    Err(FoundryCompilationError::ControlValueUnavailable {
        control_id: control_id.to_owned(),
    })
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

fn family_value_from_control(
    control_id: &str,
    kind: &ControlKind,
    value: &ControlValue,
    binding: &crate::ControlSlotBinding,
) -> Result<FamilyValue, FoundryCompilationError> {
    match (kind, value) {
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(value)) => {
            Ok(FamilyValue::Scalar(apply_response_curve(
                control_id,
                &binding.slot,
                *value,
                &binding.response,
            )?))
        }
        (ControlKind::IntegerStepper { .. }, ControlValue::Integer(value)) => {
            let value = u32::try_from(*value).map_err(|_| {
                FoundryCompilationError::NegativeIntegerControl {
                    control_id: control_id.to_owned(),
                    value: *value,
                }
            })?;
            Ok(FamilyValue::Integer(value))
        }
        (ControlKind::Toggle { .. }, ControlValue::Toggle(value)) => {
            Ok(FamilyValue::Toggle(*value))
        }
        (ControlKind::ChoiceGallery { .. }, ControlValue::Choice(value)) => {
            Ok(FamilyValue::Choice(value.clone()))
        }
        _ => Err(FoundryCompilationError::ControlValueKindMismatch {
            control_id: control_id.to_owned(),
        }),
    }
}

fn apply_response_curve(
    control_id: &str,
    slot: &str,
    value: f32,
    response: &ResponseCurve,
) -> Result<f32, FoundryCompilationError> {
    match response {
        ResponseCurve::Linear => Ok(value),
        ResponseCurve::Piecewise { points, .. } => {
            if points.is_empty() {
                return Err(FoundryCompilationError::InvalidResponseCurve {
                    control_id: control_id.to_owned(),
                    slot: slot.to_owned(),
                });
            }
            if points.len() == 1 {
                return (points[0][0] == value)
                    .then_some(points[0][1])
                    .ok_or_else(|| FoundryCompilationError::InvalidResponseCurve {
                        control_id: control_id.to_owned(),
                        slot: slot.to_owned(),
                    });
            }
            for pair in points.windows(2) {
                let [left, right] = pair else {
                    continue;
                };
                let [x0, y0] = *left;
                let [x1, y1] = *right;
                if value >= x0 && value <= x1 && x1 != x0 {
                    let t = (value - x0) / (x1 - x0);
                    return Ok(y0 + (y1 - y0) * t);
                }
            }
            Err(FoundryCompilationError::InvalidResponseCurve {
                control_id: control_id.to_owned(),
                slot: slot.to_owned(),
            })
        }
    }
}
