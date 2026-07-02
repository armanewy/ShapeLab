
/// Compute the effective authored ∩ family ∩ conformance domain for a control.
pub fn effective_control_domain(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> Result<FeasibleControlDomain, ControlEvaluationError> {
    let mut domain = control.domain.clone();
    domain = intersect_feasible_domains(&domain, &family_control_domain(control, context)?);
    if let Some(conformance_domain) = context
        .constraint_range_provider
        .feasible_control_domain(&control.id)
    {
        domain = intersect_feasible_domains(&domain, &conformance_domain);
    }
    normalize_domain(&mut domain);
    remove_unavailable_discrete_values(&mut domain);
    if !domain.has_available_values() {
        return Err(ControlEvaluationError::EmptyFeasibleDomain {
            control_id: control.id.clone(),
        });
    }
    Ok(domain)
}

/// Return the authored default value for one control, canonicalized into the effective domain.
pub fn default_control_value(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> Result<ControlValue, ControlEvaluationError> {
    let domain = effective_control_domain(control, context)?;
    let raw = match &control.kind {
        ControlKind::ContinuousAxis { default } => ControlValue::Scalar(*default),
        ControlKind::IntegerStepper { default } => ControlValue::Integer(*default),
        ControlKind::Toggle { default } => ControlValue::Toggle(*default),
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| ControlValue::Choice(option.value.clone()))
            .find(|value| domain.contains_available_value(value))
            .ok_or_else(|| ControlEvaluationError::MissingDefaultOption {
                control_id: control.id.clone(),
            })?,
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| ControlValue::Provider(option.provider_id.clone()))
            .find(|value| domain.contains_available_value(value))
            .ok_or_else(|| ControlEvaluationError::MissingDefaultOption {
                control_id: control.id.clone(),
            })?,
    };
    canonicalize_control_value_with_domain(control, &domain, raw)
}

/// Return the default state for every control in a profile.
pub fn default_control_state(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
) -> Result<BTreeMap<String, ControlValue>, ControlEvaluationError> {
    profile
        .controls
        .iter()
        .map(|control| Ok((control.id.clone(), default_control_value(control, context)?)))
        .collect()
}

/// Canonicalize a control value into the effective domain.
pub fn canonicalize_control_value(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    value: ControlValue,
) -> Result<ControlValue, ControlEvaluationError> {
    let domain = effective_control_domain(control, context)?;
    canonicalize_control_value_with_domain(control, &domain, value)
}

/// Evaluate one control into family parameters or provider selections.
pub fn evaluate_control(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    value: ControlValue,
) -> Result<EvaluatedControl, ControlEvaluationError> {
    let domain = effective_control_domain(control, context)?;
    let value = canonicalize_control_value_with_domain(control, &domain, value)?;
    let mut slot_values = BTreeMap::new();
    let mut provider_selections = BTreeMap::new();

    if let ControlKind::ProviderGallery { role, .. } = &control.kind {
        let ControlValue::Provider(provider_id) = &value else {
            return Err(ControlEvaluationError::WrongValueKind {
                control_id: control.id.clone(),
            });
        };
        provider_selections.insert(role.clone(), provider_id.clone());
    } else {
        for binding in &control.bindings {
            let slot = find_family_slot(context.family_parameter_slots, &binding.slot).ok_or_else(
                || ControlEvaluationError::UnknownFamilySlot {
                    control_id: control.id.clone(),
                    slot: binding.slot.clone(),
                },
            )?;
            let output = evaluate_binding_value(control, binding, slot, &value)?;
            slot_values.insert(binding.slot.clone(), output);
        }
    }

    let divergence = evaluated_control_divergence(control, &domain);
    Ok(EvaluatedControl {
        control_id: control.id.clone(),
        value,
        slot_values,
        provider_selections,
        domain,
        divergence,
    })
}

/// Evaluate a whole control state, filling omitted controls with canonical defaults.
pub fn evaluate_control_state(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    state: &BTreeMap<String, ControlValue>,
) -> Result<EvaluatedControlState, ControlEvaluationError> {
    let mut control_values = BTreeMap::new();
    let mut family_parameters = BTreeMap::new();
    let mut provider_selections = BTreeMap::new();
    let mut controls = BTreeMap::new();
    let mut slot_owners = BTreeMap::<String, String>::new();
    let mut provider_owners = BTreeMap::<String, String>::new();

    for control in &profile.controls {
        let raw_value = match state.get(&control.id) {
            Some(value) => value.clone(),
            None => default_control_value(control, context)?,
        };
        let evaluated = evaluate_control(control, context, raw_value)?;
        for slot in evaluated.slot_values.keys() {
            if let Some(first_control_id) = slot_owners.insert(slot.clone(), control.id.clone()) {
                return Err(ControlEvaluationError::ConflictingSlotOwnership {
                    slot: slot.clone(),
                    first_control_id,
                    second_control_id: control.id.clone(),
                });
            }
        }
        for role in evaluated.provider_selections.keys() {
            if let Some(first_control_id) = provider_owners.insert(role.clone(), control.id.clone())
            {
                return Err(ControlEvaluationError::ConflictingProviderOwnership {
                    role: role.clone(),
                    first_control_id,
                    second_control_id: control.id.clone(),
                });
            }
        }
        control_values.insert(control.id.clone(), evaluated.value.clone());
        family_parameters.extend(evaluated.slot_values.clone());
        provider_selections.extend(evaluated.provider_selections.clone());
        controls.insert(control.id.clone(), evaluated);
    }

    Ok(EvaluatedControlState {
        control_values,
        family_parameters,
        provider_selections,
        controls,
    })
}

/// Reset one control state row to the canonical authored default and return a deterministic delta.
pub fn reset_control_state(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    state: &mut BTreeMap<String, ControlValue>,
    control_id: &str,
) -> Result<ControlDelta, ControlEvaluationError> {
    let control = find_control(profile, control_id).ok_or_else(|| {
        ControlEvaluationError::UnknownControl {
            control_id: control_id.to_owned(),
        }
    })?;
    let previous = state.get(control_id).cloned();
    let current = default_control_value(control, context)?;
    let delta = explain_control_delta(profile, context, control_id, previous, current)?;
    state.insert(control_id.to_owned(), delta.current.clone());
    Ok(delta)
}

/// Explain the delta between a previous value and a new value for one control.
pub fn explain_control_delta(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    control_id: &str,
    previous: Option<ControlValue>,
    current: ControlValue,
) -> Result<ControlDelta, ControlEvaluationError> {
    let control = find_control(profile, control_id).ok_or_else(|| {
        ControlEvaluationError::UnknownControl {
            control_id: control_id.to_owned(),
        }
    })?;
    let previous_evaluated = match &previous {
        Some(value) => Some(evaluate_control(control, context, value.clone())?),
        None => None,
    };
    let current_evaluated = evaluate_control(control, context, current)?;
    let mut slot_deltas = Vec::new();
    let mut provider_deltas = Vec::new();
    let mut explanations = Vec::new();

    if previous.is_none() {
        explanations.push(ControlDeltaExplanation {
            subject: format!("controls.{}", control.id),
            code: "control_default_applied".to_owned(),
            message: format!(
                "Control `{}` used default value `{}`.",
                control.id,
                describe_control_value(&current_evaluated.value)
            ),
        });
    } else if current_evaluated.value == default_control_value(control, context)? {
        explanations.push(ControlDeltaExplanation {
            subject: format!("controls.{}", control.id),
            code: "control_reset_to_default".to_owned(),
            message: format!(
                "Control `{}` reset to default value `{}`.",
                control.id,
                describe_control_value(&current_evaluated.value)
            ),
        });
    } else if previous.as_ref() != Some(&current_evaluated.value) {
        explanations.push(ControlDeltaExplanation {
            subject: format!("controls.{}", control.id),
            code: "control_value_changed".to_owned(),
            message: format!(
                "Control `{}` changed to `{}`.",
                control.id,
                describe_control_value(&current_evaluated.value)
            ),
        });
    }

    let previous_slots = previous_evaluated
        .as_ref()
        .map(|evaluated| &evaluated.slot_values);
    let mut slots = BTreeSet::new();
    if let Some(previous_slots) = previous_slots {
        slots.extend(previous_slots.keys().cloned());
    }
    slots.extend(current_evaluated.slot_values.keys().cloned());
    for slot in slots {
        let previous_value = previous_slots.and_then(|values| values.get(&slot)).cloned();
        let current_value = current_evaluated.slot_values.get(&slot).cloned();
        if previous_value != current_value {
            explanations.push(ControlDeltaExplanation {
                subject: format!("controls.{}.bindings.{}", control.id, slot),
                code: "slot_value_changed".to_owned(),
                message: format!(
                    "Family slot `{slot}` changed from `{}` to `{}`.",
                    describe_optional_family_value(previous_value.as_ref()),
                    describe_optional_family_value(current_value.as_ref())
                ),
            });
            slot_deltas.push(ControlSlotDelta {
                slot,
                previous: previous_value,
                current: current_value,
            });
        }
    }

    let previous_providers = previous_evaluated
        .as_ref()
        .map(|evaluated| &evaluated.provider_selections);
    let mut roles = BTreeSet::new();
    if let Some(previous_providers) = previous_providers {
        roles.extend(previous_providers.keys().cloned());
    }
    roles.extend(current_evaluated.provider_selections.keys().cloned());
    for role in roles {
        let previous_provider = previous_providers
            .and_then(|values| values.get(&role))
            .cloned();
        let current_provider = current_evaluated.provider_selections.get(&role).cloned();
        if previous_provider != current_provider {
            explanations.push(ControlDeltaExplanation {
                subject: format!("controls.{}.providers.{}", control.id, role),
                code: "provider_selection_changed".to_owned(),
                message: format!(
                    "Provider role `{role}` changed from `{}` to `{}`.",
                    previous_provider.as_deref().unwrap_or("<unset>"),
                    current_provider.as_deref().unwrap_or("<unset>")
                ),
            });
            provider_deltas.push(ControlProviderDelta {
                role,
                previous: previous_provider,
                current: current_provider,
            });
        }
    }

    Ok(ControlDelta {
        control_id: control.id.clone(),
        previous,
        current: current_evaluated.value,
        slot_deltas,
        provider_deltas,
        explanations,
    })
}

/// Compute divergence for one control against local semantic overrides.
#[must_use]
pub fn control_divergence(
    control: &CustomizerControl,
    document: &FoundryAssetDocument,
) -> ControlDivergence {
    if document.local_recipe_overrides.iter().any(|override_row| {
        override_row.touched_targets.iter().any(|target| {
            matches!(
                target,
                TouchedSemanticTarget::FamilySlot(slot)
                    if control.bindings.iter().any(|binding| binding.slot == *slot)
            )
        })
    }) {
        ControlDivergence::DivergedByOverride
    } else if !control.domain.has_available_values() {
        ControlDivergence::Unavailable
    } else {
        control.divergence
    }
}

/// Compute divergence for every control in a profile.
#[must_use]
pub fn control_divergence_state(
    profile: &CustomizerProfile,
    document: &FoundryAssetDocument,
) -> BTreeMap<String, ControlDivergence> {
    profile
        .controls
        .iter()
        .map(|control| (control.id.clone(), control_divergence(control, document)))
        .collect()
}

/// Generate default whole-model preview sample requests for one control.
pub fn whole_model_preview_sample_requests(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> Result<Vec<WholeModelPreviewSampleRequest>, ControlEvaluationError> {
    whole_model_preview_sample_requests_with_count(control, context, DEFAULT_PREVIEW_SAMPLE_COUNT)
}

/// Generate whole-model preview sample requests for one control.
pub fn whole_model_preview_sample_requests_with_count(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    sample_count: usize,
) -> Result<Vec<WholeModelPreviewSampleRequest>, ControlEvaluationError> {
    let domain = effective_control_domain(control, context)?;
    let mut values = match (
        &control.kind,
        control.topology_behavior,
        &domain.certification,
    ) {
        (
            ControlKind::ContinuousAxis { .. },
            ControlTopologyBehavior::TopologyPreserving,
            DomainCertification::CertifiedContinuous,
        ) => continuous_preview_values(&domain, sample_count),
        _ => domain.discrete_values.clone(),
    };
    sort_control_values(&mut values);
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let value = canonicalize_control_value_with_domain(control, &domain, value)?;
            Ok(WholeModelPreviewSampleRequest {
                preview_id: format!("{}-preview-{index}", control.id),
                control_id: control.id.clone(),
                sample_index: index as u32,
                value,
                build_kind: ControlBuildRequestKind::PreviewSample,
            })
        })
        .collect()
}

/// Build an exact whole-model request for the value selected on control release.
pub fn whole_model_exact_build_request(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    value: ControlValue,
) -> Result<WholeModelPreviewSampleRequest, ControlEvaluationError> {
    let value = canonicalize_control_value(control, context, value)?;
    Ok(WholeModelPreviewSampleRequest {
        preview_id: format!("{}-exact-release", control.id),
        control_id: control.id.clone(),
        sample_index: 0,
        value,
        build_kind: ControlBuildRequestKind::ExactOnRelease,
    })
}
