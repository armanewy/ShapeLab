
fn validate_primitive_property(
    report: &mut PrimitivePropertyValidationReport,
    index: usize,
    property: &PrimitiveProperty,
) {
    let subject = format!("properties.{index}");
    validate_identifier(
        report,
        format!("{subject}.property_id"),
        &property.property_id,
        "invalid_primitive_property_id",
    );
    validate_product_label(
        report,
        format!("{subject}.display_name"),
        &property.display_name,
        "invalid_primitive_property_label",
    );
    validate_product_label(
        report,
        format!("{subject}.user_facing_description"),
        &property.user_facing_description,
        "invalid_primitive_property_description",
    );
    validate_property_domain(report, &subject, property);

    if !domain_matches_kind(&property.domain, property.value_kind) {
        report.push(
            format!("{subject}.domain"),
            "property_domain_kind_mismatch",
            "Property domain must match its declared value kind.",
        );
    }
    if !property_value_matches_kind(&property.default_value, property.value_kind) {
        report.push(
            format!("{subject}.default_value"),
            "property_default_kind_mismatch",
            "Property default value must match its declared value kind.",
        );
    } else if !domain_contains_value(&property.domain, &property.default_value) {
        report.push(
            format!("{subject}.default_value"),
            "property_default_outside_domain",
            "Property default value must be inside the domain.",
        );
    }
    if property.topology_behavior == PrimitiveTopologyBehavior::DiscreteTopology
        && property.value_kind != PrimitivePropertyValueKind::Choice
    {
        report.push(
            format!("{subject}.topology_behavior"),
            "topology_change_must_be_choice",
            "Topology-changing primitive properties must be discrete choices.",
        );
    }
}

fn validate_property_domain(
    report: &mut PrimitivePropertyValidationReport,
    subject: &str,
    property: &PrimitiveProperty,
) {
    match &property.domain {
        PrimitivePropertyDomain::Length {
            minimum,
            maximum,
            step,
        }
        | PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step,
        } => validate_numeric_domain(
            report,
            format!("{subject}.domain"),
            *minimum,
            *maximum,
            *step,
        ),
        PrimitivePropertyDomain::Angle {
            minimum_degrees,
            maximum_degrees,
            step_degrees,
        } => validate_numeric_domain(
            report,
            format!("{subject}.domain"),
            *minimum_degrees,
            *maximum_degrees,
            *step_degrees,
        ),
        PrimitivePropertyDomain::Boolean => {}
        PrimitivePropertyDomain::Choice { options } => {
            if options.is_empty() {
                report.push(
                    format!("{subject}.domain.options"),
                    "empty_choice_domain",
                    "Choice domains must include at least one option.",
                );
            }
            let mut option_ids = BTreeSet::new();
            for (index, option) in options.iter().enumerate() {
                validate_identifier(
                    report,
                    format!("{subject}.domain.options.{index}.choice_id"),
                    &option.choice_id,
                    "invalid_choice_id",
                );
                validate_product_label(
                    report,
                    format!("{subject}.domain.options.{index}.display_name"),
                    &option.display_name,
                    "invalid_choice_label",
                );
                if !option_ids.insert(option.choice_id.as_str()) {
                    report.push(
                        format!("{subject}.domain.options.{index}.choice_id"),
                        "duplicate_choice_id",
                        "Choice IDs must be unique within one property.",
                    );
                }
            }
        }
    }
}

fn validate_numeric_domain(
    report: &mut PrimitivePropertyValidationReport,
    subject: String,
    minimum: f32,
    maximum: f32,
    step: f32,
) {
    if !minimum.is_finite() || !maximum.is_finite() || !step.is_finite() {
        report.push(
            subject,
            "non_finite_property_domain",
            "Numeric property domains must use finite bounds and step sizes.",
        );
    } else if minimum > maximum || step <= 0.0 {
        report.push(
            subject,
            "invalid_property_domain_range",
            "Numeric property domains must have ordered bounds and a positive step size.",
        );
    }
}

fn validate_identifier(
    report: &mut PrimitivePropertyValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
        || contains_internal_term(value)
    {
        report.push(
            subject,
            code,
            "Stable primitive IDs must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_label(
    report: &mut PrimitivePropertyValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || trimmed.contains("::")
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        report.push(
            subject,
            code,
            "Primitive property labels and descriptions must be product-safe.",
        );
    }
}

fn domain_matches_kind(
    domain: &PrimitivePropertyDomain,
    value_kind: PrimitivePropertyValueKind,
) -> bool {
    matches!(
        (domain, value_kind),
        (
            PrimitivePropertyDomain::Length { .. },
            PrimitivePropertyValueKind::Length
        ) | (
            PrimitivePropertyDomain::Ratio { .. },
            PrimitivePropertyValueKind::Ratio
        ) | (
            PrimitivePropertyDomain::Boolean,
            PrimitivePropertyValueKind::Boolean
        ) | (
            PrimitivePropertyDomain::Choice { .. },
            PrimitivePropertyValueKind::Choice
        ) | (
            PrimitivePropertyDomain::Angle { .. },
            PrimitivePropertyValueKind::Angle
        )
    )
}

fn property_value_matches_kind(
    value: &PrimitivePropertyValue,
    value_kind: PrimitivePropertyValueKind,
) -> bool {
    matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Length(value), PrimitivePropertyValueKind::Length)
            if value.is_finite()
    ) || matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Ratio(value), PrimitivePropertyValueKind::Ratio)
            if value.is_finite()
    ) || matches!(
        (value, value_kind),
        (
            PrimitivePropertyValue::Boolean(_),
            PrimitivePropertyValueKind::Boolean
        )
    ) || matches!(
        (value, value_kind),
        (
            PrimitivePropertyValue::Choice(_),
            PrimitivePropertyValueKind::Choice
        )
    ) || matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Angle(value), PrimitivePropertyValueKind::Angle)
            if value.is_finite()
    )
}

fn domain_contains_value(domain: &PrimitivePropertyDomain, value: &PrimitivePropertyValue) -> bool {
    match (domain, value) {
        (
            PrimitivePropertyDomain::Length {
                minimum, maximum, ..
            },
            PrimitivePropertyValue::Length(value),
        )
        | (
            PrimitivePropertyDomain::Ratio {
                minimum, maximum, ..
            },
            PrimitivePropertyValue::Ratio(value),
        ) => value.is_finite() && value >= minimum && value <= maximum,
        (
            PrimitivePropertyDomain::Angle {
                minimum_degrees,
                maximum_degrees,
                ..
            },
            PrimitivePropertyValue::Angle(value),
        ) => value.is_finite() && value >= minimum_degrees && value <= maximum_degrees,
        (PrimitivePropertyDomain::Boolean, PrimitivePropertyValue::Boolean(_)) => true,
        (PrimitivePropertyDomain::Choice { options }, PrimitivePropertyValue::Choice(choice)) => {
            options.iter().any(|option| option.choice_id == *choice)
        }
        _ => false,
    }
}
