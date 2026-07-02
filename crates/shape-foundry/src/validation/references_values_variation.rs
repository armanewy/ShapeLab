
fn validate_ref(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    content_ref: &CatalogContentRef,
) {
    let subject = subject.into();
    validate_identifier(
        report,
        format!("{subject}.stable_id"),
        &content_ref.stable_id,
    );
    if content_ref.schema_version == 0 {
        report.push(
            format!("{subject}.schema_version"),
            "invalid_catalog_ref_schema",
            "Catalog content reference schema version must be greater than zero.",
        );
    }
}

fn validate_catalog_lock_ref(
    report: &mut FoundryValidationReport,
    actual: Option<&CatalogContentRef>,
    subject: &str,
    expected: &CatalogContentRef,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(_) => report.push(
            subject,
            "catalog_lock_ref_mismatch",
            "Catalog lock reference must match the document's exact content reference.",
        ),
        None => report.push(
            subject,
            "missing_catalog_lock_ref",
            "Catalog lock is missing a required exact content reference.",
        ),
    }
}

fn validate_pack_catalog_lock_ref(
    report: &mut FoundryValidationReport,
    actual: Option<&CatalogContentRef>,
    subject: &str,
    expected: Option<&CatalogContentRef>,
    required: bool,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual == expected => {}
        (Some(_), _) => report.push(
            subject,
            "catalog_lock_ref_mismatch",
            "Catalog lock reference must match a shared pack content reference.",
        ),
        (None, _) if required => report.push(
            subject,
            "missing_catalog_lock_ref",
            "Catalog lock is missing a required exact content reference.",
        ),
        (None, _) => {}
    }
}

fn shared_member_catalog_ref<'a>(
    pack: &'a FoundryPackDocument,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    let mut documents = pack.members.values();
    let first_ref = documents
        .next()
        .and_then(|document| member_catalog_ref(document, key))?;
    documents
        .all(|document| member_catalog_ref(document, key) == Some(first_ref))
        .then_some(first_ref)
}

fn member_catalog_ref<'a>(
    document: &'a FoundryAssetDocument,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    match key {
        CATALOG_LOCK_KEY_FAMILY => Some(&document.family_content_ref),
        CATALOG_LOCK_KEY_STYLE => Some(&document.style_content_ref),
        _ => None,
    }
}

fn validate_set_control_value(
    report: &mut FoundryValidationReport,
    control: &CustomizerControl,
    value: &ControlValue,
) {
    if !control_value_matches_kind(&control.kind, value) {
        report.push(
            "set_control.value",
            "control_value_kind_mismatch",
            "SetControl value type does not match the referenced control kind.",
        );
        return;
    }
    match (&control.kind, value) {
        (ControlKind::ChoiceGallery { options }, ControlValue::Choice(value))
            if !options.iter().any(|option| option.value == *value) =>
        {
            report.push(
                "set_control.value",
                "unknown_choice_option",
                "SetControl references an unknown choice option.",
            );
        }
        (ControlKind::ProviderGallery { options, .. }, ControlValue::Provider(value))
            if !options.iter().any(|option| option.provider_id == *value) =>
        {
            report.push(
                "set_control.value",
                "unknown_provider_option",
                "SetControl references an unknown provider option.",
            );
        }
        _ => {}
    }
    let option_key = control_value_option_key(value);
    if let Some(reason) = control.domain.unavailable_options.get(&option_key) {
        report.push(
            "set_control.value",
            "unavailable_control_option",
            format!("SetControl selected unavailable option `{option_key}`: {reason}"),
        );
    }
    if !control_value_in_domain(&control.domain, value) {
        report.push(
            "set_control.value",
            "control_value_outside_domain",
            "SetControl value is outside the control's feasible domain.",
        );
    }
}

fn validate_control_value(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &ControlValue,
) {
    if let ControlValue::Scalar(value) = value
        && !value.is_finite()
    {
        report.push(
            subject,
            "non_finite_control_value",
            "Control scalar values must be finite.",
        );
    }
}

fn validate_variation_intent(
    report: &mut FoundryValidationReport,
    subject: &str,
    intent: &VariationIntent,
) {
    validate_variation_scope(report, &format!("{subject}.scope"), &intent.scope);
    if intent.channels.is_empty() {
        report.push(
            format!("{subject}.channels"),
            "empty_variation_channels",
            "Variation intent must include at least one channel.",
        );
    }
    for (index, channel) in intent.channels.iter().enumerate() {
        validate_variation_channel(report, &format!("{subject}.channels.{index}"), channel);
    }
    if intent.human_label.trim().is_empty() {
        report.push(
            format!("{subject}.human_label"),
            "empty_variation_label",
            "Variation intent label must not be empty.",
        );
    }
    if intent.human_summary.trim().is_empty() {
        report.push(
            format!("{subject}.human_summary"),
            "empty_variation_summary",
            "Variation intent summary must not be empty.",
        );
    }
}

fn validate_variation_scope(
    report: &mut FoundryValidationReport,
    subject: &str,
    scope: &VariationScope,
) {
    match scope {
        VariationScope::WholeAsset => {}
        VariationScope::SemanticPartGroup {
            group_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "group_id", group_id, display_name),
        VariationScope::MaterialSlot {
            slot_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "slot_id", slot_id, display_name),
        VariationScope::DetailZone {
            zone_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "zone_id", zone_id, display_name),
        VariationScope::RigRegion {
            region_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "region_id", region_id, display_name),
        VariationScope::MotionSet {
            motion_set_id,
            display_name,
        } => validate_variation_scoped_label(
            report,
            subject,
            "motion_set_id",
            motion_set_id,
            display_name,
        ),
        VariationScope::Custom {
            scope_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "scope_id", scope_id, display_name),
    }
}

fn validate_variation_channel(
    report: &mut FoundryValidationReport,
    subject: &str,
    channel: &VariationChannel,
) {
    if let VariationChannel::Custom {
        channel_id,
        display_name,
    } = channel
    {
        validate_identifier(report, format!("{subject}.channel_id"), channel_id);
        if display_name.trim().is_empty() {
            report.push(
                format!("{subject}.display_name"),
                "empty_variation_display_name",
                "Variation channel display name must not be empty.",
            );
        }
    }
}

fn validate_variation_scoped_label(
    report: &mut FoundryValidationReport,
    subject: &str,
    id_field: &str,
    id: &str,
    display_name: &str,
) {
    validate_identifier(report, format!("{subject}.{id_field}"), id);
    if display_name.trim().is_empty() {
        report.push(
            format!("{subject}.display_name"),
            "empty_variation_display_name",
            "Variation scope display name must not be empty.",
        );
    }
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

fn control_value_in_domain(domain: &crate::FeasibleControlDomain, value: &ControlValue) -> bool {
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

fn control_value_option_key(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => value.to_string(),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn validate_identifier(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    let subject = subject.into();
    if value.is_empty() {
        report.push(subject, "empty_identifier", "Identifier must not be empty.");
        return;
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        report.push(
            subject,
            "invalid_identifier",
            "Identifier must contain only ASCII letters, digits, dashes, underscores, or dots.",
        );
    }
}

fn validate_non_empty(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.is_empty() {
        report.push(subject, "empty_text", "Text field must not be empty.");
    }
}
