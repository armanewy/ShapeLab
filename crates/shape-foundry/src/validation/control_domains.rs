
fn validate_control(
    report: &mut FoundryValidationReport,
    index: usize,
    control: &CustomizerControl,
    section_ids: &BTreeSet<&str>,
    slot_owners: &mut BTreeMap<String, String>,
    provider_role_owners: &mut BTreeMap<String, String>,
) {
    validate_identifier(report, format!("controls.{index}.id"), &control.id);
    validate_non_empty(report, format!("controls.{index}.label"), &control.label);
    if let Some(section) = &control.section {
        validate_identifier(report, format!("controls.{index}.section"), section);
        if !section_ids.contains(section.as_str()) {
            report.push(
                format!("controls.{index}.section"),
                "unknown_control_section",
                "Control references an unknown section.",
            );
        }
    }
    if control.bindings.is_empty() && !matches!(control.kind, ControlKind::ProviderGallery { .. }) {
        report.push(
            format!("controls.{index}.bindings"),
            "control_without_bindings",
            "Controls must own at least one family slot binding.",
        );
    }
    for (binding_index, binding) in control.bindings.iter().enumerate() {
        validate_control_binding(report, index, binding_index, control.visible, binding);
        if let Some(previous) = slot_owners.insert(binding.slot.clone(), control.id.clone()) {
            report.push(
                format!("controls.{index}.bindings.{binding_index}.slot"),
                "conflicting_control_ownership",
                format!(
                    "Family slot `{}` is already owned by control `{previous}`.",
                    binding.slot
                ),
            );
        }
    }
    match &control.kind {
        ControlKind::ContinuousAxis { default } => {
            if !default.is_finite() {
                report.push(
                    format!("controls.{index}.kind.default"),
                    "non_finite_control_default",
                    "Continuous control defaults must be finite.",
                );
            }
            if control.topology_behavior == ControlTopologyBehavior::TopologyChanging {
                report.push(
                    format!("controls.{index}.topology_behavior"),
                    "topology_changing_live_control",
                    "Topology-changing controls must be represented as discrete values.",
                );
            }
            if control.domain.certification != DomainCertification::CertifiedContinuous {
                report.push(
                    format!("controls.{index}.domain.certification"),
                    "uncertified_continuous_control",
                    "ContinuousAxis controls require a certified continuous domain.",
                );
            }
            if control.domain.continuous_intervals.is_empty() {
                report.push(
                    format!("controls.{index}.domain.continuous_intervals"),
                    "missing_continuous_control_interval",
                    "ContinuousAxis controls require at least one continuous interval.",
                );
            }
        }
        ControlKind::IntegerStepper { .. } | ControlKind::Toggle { .. } => {
            if !control.domain.continuous_intervals.is_empty() {
                report.push(
                    format!("controls.{index}.domain.continuous_intervals"),
                    "continuous_domain_for_discrete_control",
                    "Discrete controls must expose discrete values rather than continuous intervals.",
                );
            }
        }
        ControlKind::ChoiceGallery { options } => {
            validate_choice_options(report, index, options);
            validate_choice_domain(report, index, control, options);
        }
        ControlKind::ProviderGallery { role, options } => {
            validate_identifier(report, format!("controls.{index}.kind.role"), role);
            if let Some(first_control_id) =
                provider_role_owners.insert(role.clone(), control.id.clone())
            {
                report.push(
                    format!("controls.{index}.kind.role"),
                    "conflicting_provider_role_owner",
                    format!(
                        "Provider role `{role}` is already controlled by `{first_control_id}`."
                    ),
                );
            }
            validate_provider_options(report, index, options);
            validate_provider_domain(report, index, control, options);
        }
    }
    if control.topology_behavior == ControlTopologyBehavior::TopologyChanging
        && !control.domain.continuous_intervals.is_empty()
    {
        report.push(
            format!("controls.{index}.domain.continuous_intervals"),
            "topology_changing_continuous_domain",
            "Topology-changing controls must be represented by discrete values only.",
        );
    }
    validate_domain(report, index, control);
}

fn validate_control_binding(
    report: &mut FoundryValidationReport,
    control_index: usize,
    binding_index: usize,
    visible: bool,
    binding: &ControlSlotBinding,
) {
    validate_identifier(
        report,
        format!("controls.{control_index}.bindings.{binding_index}.slot"),
        &binding.slot,
    );
    if visible && binding.slot_policy != ParameterExecutionPolicy::RequiredBinding {
        report.push(
            format!("controls.{control_index}.bindings.{binding_index}.slot_policy"),
            "visible_control_non_required_slot",
            "Visible customizer controls should own RequiredBinding slots; advisory/runtime slots belong in metadata until a runtime adapter consumes them.",
        );
    }
    validate_response_curve(
        report,
        format!("controls.{control_index}.bindings.{binding_index}.response"),
        &binding.response,
    );
}

fn validate_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
) {
    let available_discrete_count = control
        .domain
        .discrete_values
        .iter()
        .filter(|value| {
            !control
                .domain
                .unavailable_options
                .contains_key(&control_value_option_key(value))
        })
        .count();
    if control.domain.continuous_intervals.is_empty() && available_discrete_count == 0 {
        report.push(
            format!("controls.{control_index}.domain"),
            "empty_feasible_control_domain",
            "A control domain must expose at least one available value.",
        );
    }
    for (index, interval) in control.domain.continuous_intervals.iter().enumerate() {
        validate_interval(
            report,
            format!("controls.{control_index}.domain.continuous_intervals.{index}"),
            *interval,
        );
    }
    for (index, value) in control.domain.discrete_values.iter().enumerate() {
        validate_control_value(
            report,
            format!("controls.{control_index}.domain.discrete_values.{index}"),
            value,
        );
        if !control_value_matches_kind(&control.kind, value) {
            report.push(
                format!("controls.{control_index}.domain.discrete_values.{index}"),
                "control_domain_value_kind_mismatch",
                "Control domain value type must match the control kind.",
            );
        }
    }
    for (option, reason) in &control.domain.unavailable_options {
        validate_identifier(
            report,
            format!("controls.{control_index}.domain.unavailable_options.{option}"),
            option,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.domain.unavailable_options.{option}.reason"),
            reason,
        );
    }
}

fn validate_interval(
    report: &mut FoundryValidationReport,
    subject: String,
    interval: ClosedInterval,
) {
    if !interval.minimum.is_finite() || !interval.maximum.is_finite() {
        report.push(
            subject,
            "non_finite_control_interval",
            "Continuous control intervals must be finite.",
        );
    } else if interval.minimum > interval.maximum {
        report.push(
            subject,
            "inverted_control_interval",
            "Continuous control interval minimum must not exceed maximum.",
        );
    }
}

fn validate_choice_options(
    report: &mut FoundryValidationReport,
    control_index: usize,
    options: &[ChoiceOption],
) {
    if options.is_empty() {
        report.push(
            format!("controls.{control_index}.kind.options"),
            "empty_choice_gallery",
            "ChoiceGallery controls must contain at least one option.",
        );
    }
    let mut seen = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        validate_identifier(
            report,
            format!("controls.{control_index}.kind.options.{index}.value"),
            &option.value,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.label"),
            &option.label,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.preview.preview_id"),
            &option.preview.preview_id,
        );
        if !seen.insert(option.value.as_str()) {
            report.push(
                format!("controls.{control_index}.kind.options.{index}.value"),
                "duplicate_choice_option",
                "Choice option values must be unique.",
            );
        }
    }
}

fn validate_provider_options(
    report: &mut FoundryValidationReport,
    control_index: usize,
    options: &[ProviderOption],
) {
    if options.is_empty() {
        report.push(
            format!("controls.{control_index}.kind.options"),
            "empty_provider_gallery",
            "ProviderGallery controls must contain at least one option.",
        );
    }
    let mut seen = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        validate_identifier(
            report,
            format!("controls.{control_index}.kind.options.{index}.provider_id"),
            &option.provider_id,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.label"),
            &option.label,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.preview.preview_id"),
            &option.preview.preview_id,
        );
        if !seen.insert(option.provider_id.as_str()) {
            report.push(
                format!("controls.{control_index}.kind.options.{index}.provider_id"),
                "duplicate_provider_option",
                "Provider option IDs must be unique.",
            );
        }
    }
}

fn validate_choice_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    options: &[ChoiceOption],
) {
    let option_ids = options
        .iter()
        .map(|option| option.value.as_str())
        .collect::<BTreeSet<_>>();
    validate_symbolic_domain(
        report,
        control_index,
        control,
        &option_ids,
        "choice",
        |value| match value {
            ControlValue::Choice(value) => Some(value.as_str()),
            _ => None,
        },
    );
}

fn validate_provider_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    options: &[ProviderOption],
) {
    let option_ids = options
        .iter()
        .map(|option| option.provider_id.as_str())
        .collect::<BTreeSet<_>>();
    validate_symbolic_domain(
        report,
        control_index,
        control,
        &option_ids,
        "provider",
        |value| match value {
            ControlValue::Provider(value) => Some(value.as_str()),
            _ => None,
        },
    );
}

fn validate_symbolic_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    option_ids: &BTreeSet<&str>,
    option_kind: &str,
    value_key: impl Fn(&ControlValue) -> Option<&str>,
) {
    let mut covered_options = BTreeSet::new();
    for (value_index, value) in control.domain.discrete_values.iter().enumerate() {
        let Some(key) = value_key(value) else {
            continue;
        };
        if !option_ids.contains(key) {
            report.push(
                format!("controls.{control_index}.domain.discrete_values.{value_index}"),
                format!("unknown_{option_kind}_domain_value"),
                format!("Control domain references an unknown {option_kind} option."),
            );
        }
        covered_options.insert(key);
    }
    for option in control.domain.unavailable_options.keys() {
        if !option_ids.contains(option.as_str()) {
            report.push(
                format!("controls.{control_index}.domain.unavailable_options.{option}"),
                format!("unknown_unavailable_{option_kind}_option"),
                format!("Unavailable option references an unknown {option_kind} option."),
            );
        }
        covered_options.insert(option.as_str());
    }
    for option in option_ids {
        if !covered_options.contains(option) {
            report.push(
                format!("controls.{control_index}.domain"),
                format!("missing_{option_kind}_option_domain"),
                format!("Every {option_kind} option must be marked available or unavailable."),
            );
        }
    }
}

fn validate_response_curve(
    report: &mut FoundryValidationReport,
    subject: String,
    curve: &ResponseCurve,
) {
    match curve {
        ResponseCurve::Linear => {}
        ResponseCurve::Piecewise { points, monotonic } => {
            if points.len() < 2 {
                report.push(
                    subject,
                    "too_few_response_curve_points",
                    "Piecewise response curves must contain at least two points.",
                );
                return;
            }
            let mut previous_input = f32::NEG_INFINITY;
            let mut previous_output = f32::NEG_INFINITY;
            for (index, [input, output]) in points.iter().copied().enumerate() {
                if !input.is_finite() || !output.is_finite() {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_finite_response_curve_point",
                        "Response curve points must be finite.",
                    );
                }
                if input <= previous_input {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_increasing_response_curve_input",
                        "Response curve input points must be strictly increasing.",
                    );
                }
                if index > 0 && !(output - previous_output).is_finite() {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_finite_response_curve_output",
                        "Response curve interpolation must not produce non-finite output.",
                    );
                }
                if *monotonic && output < previous_output {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_monotonic_response_curve",
                        "Response curve output must be monotonic when required.",
                    );
                }
                previous_input = input;
                previous_output = output;
            }
        }
    }
}
