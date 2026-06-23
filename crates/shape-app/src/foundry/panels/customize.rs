//! Character-customizer-style control deck panel boundary.

use shape_foundry::{
    ControlDivergence, ControlEvaluationContext, ControlEvaluationError, ControlKind, ControlValue,
    CustomizerControl, CustomizerProfile, DEFAULT_PREVIEW_SAMPLE_COUNT, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCommand, FoundryLock, FoundryLockMode, FoundryLockTarget,
    canonicalize_control_value, control_divergence, default_control_value,
    effective_control_domain, evaluate_control, whole_model_preview_sample_requests_with_count,
};

use crate::foundry::{
    FoundryAppCommand,
    view_model::{FoundryControlPresentation, FoundryControlView, FoundryOptionCard},
};

/// Options used when building the customizer deck.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct CustomizeDeckOptions {
    /// Whether Advanced Recipe controls should be included in displayed rows.
    pub advanced_open: bool,
    /// Number of generated filmstrip samples for sampled controls.
    pub preview_sample_count: usize,
}

impl Default for CustomizeDeckOptions {
    fn default() -> Self {
        Self {
            advanced_open: false,
            preview_sample_count: DEFAULT_PREVIEW_SAMPLE_COUNT,
        }
    }
}

/// UI-ready control deck split into default and Advanced Recipe rows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CustomizeDeckView {
    /// Every visible control row, including controls hidden behind Advanced Recipe.
    pub controls: Vec<FoundryControlView>,
    /// Primary rows visible by default.
    pub primary_controls: Vec<FoundryControlView>,
    /// Visible non-primary rows shown by Advanced Recipe.
    pub advanced_controls: Vec<FoundryControlView>,
    /// Rows the current collapsed/open state should display.
    pub displayed_controls: Vec<FoundryControlView>,
    /// Whether Advanced Recipe is open.
    pub advanced_open: bool,
}

/// One row for the Advanced Recipe drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdvancedRecipeRow {
    /// Stable control ID.
    pub control_id: String,
    /// Human-facing label.
    pub label: String,
    /// Technical path; keep this out of the primary deck.
    pub technical_path: String,
    /// Current value label.
    pub value: Option<String>,
    /// Divergence badge label.
    pub divergence: String,
    /// Whether the row is locked for editing.
    pub locked: bool,
}

/// Build the customizer deck from a profile and semantic document.
pub(crate) fn customize_deck(
    profile: &CustomizerProfile,
    document: &FoundryAssetDocument,
    context: ControlEvaluationContext<'_>,
    options: CustomizeDeckOptions,
) -> Result<CustomizeDeckView, ControlEvaluationError> {
    let mut controls = Vec::new();
    let mut primary_controls = Vec::new();
    let mut advanced_controls = Vec::new();

    for control in &profile.controls {
        if !control.visible {
            continue;
        }

        let view = control_view(control, document, context, options.preview_sample_count)?;
        if view.visible {
            primary_controls.push(view.clone());
        } else {
            advanced_controls.push(view.clone());
        }
        controls.push(view);
    }

    let mut displayed_controls = primary_controls.clone();
    if options.advanced_open {
        displayed_controls.extend(advanced_controls.clone());
    }

    Ok(CustomizeDeckView {
        controls,
        primary_controls,
        advanced_controls,
        displayed_controls,
        advanced_open: options.advanced_open,
    })
}

/// Build one control view row.
pub(crate) fn control_view(
    control: &CustomizerControl,
    document: &FoundryAssetDocument,
    context: ControlEvaluationContext<'_>,
    preview_sample_count: usize,
) -> Result<FoundryControlView, ControlEvaluationError> {
    let effective_domain = effective_control_domain(control, context)?;
    let default_value = Some(default_control_value(control, context)?);
    let current_value = current_control_value(control, document, context)?;
    let locked = control_is_locked(&document.foundry_locks, control);
    let divergence = row_control_divergence(control, document, context, &current_value)?;
    let options = control_options(
        control,
        Some(&current_value),
        &effective_domain,
        context,
        preview_sample_count,
    )?;

    Ok(FoundryControlView {
        id: control.id.clone(),
        label: control.label.clone(),
        section: control.section.clone(),
        kind: control_kind_label(&control.kind).to_owned(),
        presentation: control_presentation(&control.kind),
        value: Some(current_value),
        default_value,
        primary: control.primary,
        visible: control.primary,
        locked,
        topology_behavior: control.topology_behavior,
        divergence,
        options,
        advanced_path: Some(control_technical_path(control)),
        help: None,
    })
}

/// Return Advanced Recipe rows, including technical paths.
#[must_use]
pub(crate) fn advanced_recipe_rows(deck: &CustomizeDeckView) -> Vec<AdvancedRecipeRow> {
    deck.controls
        .iter()
        .map(|control| AdvancedRecipeRow {
            control_id: control.id.clone(),
            label: control.label.clone(),
            technical_path: control
                .advanced_path
                .clone()
                .unwrap_or_else(|| format!("controls.{}", control.id)),
            value: control.value.as_ref().map(control_value_label),
            divergence: divergence_label(control.divergence).to_owned(),
            locked: control.locked,
        })
        .collect()
}

/// Command emitted by an Advanced Recipe toggle.
#[must_use]
pub(crate) fn advanced_recipe_open_command(open: bool) -> FoundryAppCommand {
    FoundryAppCommand::SetAdvancedRecipeOpen(open)
}

/// Command emitted when the active row changes.
#[must_use]
pub(crate) fn select_control_command(control_id: Option<String>) -> FoundryAppCommand {
    FoundryAppCommand::SelectControl(control_id)
}

/// Emit a preview update for a transient control value.
#[must_use]
pub(crate) fn preview_control_value_intents(
    control: &FoundryControlView,
    value: ControlValue,
) -> Vec<FoundryAppCommand> {
    if control.locked {
        return Vec::new();
    }

    vec![
        set_control_command(&control.id, value),
        FoundryAppCommand::RequestPreview,
    ]
}

/// Commit a value and request an exact rebuild on release.
#[must_use]
pub(crate) fn release_control_value_intents(
    control: &FoundryControlView,
    value: ControlValue,
) -> Vec<FoundryAppCommand> {
    if control.locked {
        return Vec::new();
    }

    vec![
        set_control_command(&control.id, value),
        FoundryAppCommand::RequestBuild,
    ]
}

/// Commit a filmstrip/gallery option using release semantics.
#[must_use]
pub(crate) fn choose_option_intents(
    control: &FoundryControlView,
    option: &FoundryOptionCard,
) -> Vec<FoundryAppCommand> {
    if option.control_id != control.id || option.unavailable_reason.is_some() {
        return Vec::new();
    }

    release_control_value_intents(control, option.value.clone())
}

/// Reset a control to its authored default and request an exact rebuild.
#[must_use]
pub(crate) fn reset_control_intents(control: &FoundryControlView) -> Vec<FoundryAppCommand> {
    if !control_can_reset(control) {
        return Vec::new();
    }

    vec![
        FoundryAppCommand::run(FoundryCommand::ResetControl {
            control_id: control.id.clone(),
        }),
        FoundryAppCommand::RequestBuild,
    ]
}

/// Return whether a reset button should be enabled.
#[must_use]
pub(crate) fn control_can_reset(control: &FoundryControlView) -> bool {
    !control.locked && control.value.is_some() && control.value != control.default_value
}

/// Emit a lock-mode change for one control, avoiding duplicate lock commands.
#[must_use]
pub(crate) fn control_lock_command(
    control: &FoundryControlView,
    locked: bool,
) -> Option<FoundryAppCommand> {
    if control.locked == locked {
        return None;
    }

    let mode = if locked {
        FoundryLockMode::Locked
    } else {
        FoundryLockMode::SearchProtected
    };
    Some(FoundryAppCommand::run(FoundryCommand::SetLock {
        lock: FoundryLock {
            target: FoundryLockTarget::Control(control.id.clone()),
            mode,
            reason: locked.then(|| "Locked from customizer deck".to_owned()),
        },
    }))
}

/// Human-facing label for a divergence state.
#[must_use]
pub(crate) fn divergence_label(divergence: ControlDivergence) -> &'static str {
    match divergence {
        ControlDivergence::Synced => "Synced",
        ControlDivergence::DivergedByOverride => "Diverged",
        ControlDivergence::Unavailable => "Unavailable",
        ControlDivergence::ConstraintLimited => "Constraint limited",
    }
}

/// Human-facing label for a control value.
#[must_use]
pub(crate) fn control_value_label(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => trimmed_scalar_label(*value),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(true) => "On".to_owned(),
        ControlValue::Toggle(false) => "Off".to_owned(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn set_control_command(control_id: &str, value: ControlValue) -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::SetControl {
        control_id: control_id.to_owned(),
        value,
    })
}

fn current_control_value(
    control: &CustomizerControl,
    document: &FoundryAssetDocument,
    context: ControlEvaluationContext<'_>,
) -> Result<ControlValue, ControlEvaluationError> {
    match document.control_state.get(&control.id) {
        Some(value) => canonicalize_control_value(control, context, value.clone()),
        None => default_control_value(control, context),
    }
}

fn row_control_divergence(
    control: &CustomizerControl,
    document: &FoundryAssetDocument,
    context: ControlEvaluationContext<'_>,
    value: &ControlValue,
) -> Result<ControlDivergence, ControlEvaluationError> {
    let document_divergence = control_divergence(control, document);
    if matches!(
        document_divergence,
        ControlDivergence::DivergedByOverride | ControlDivergence::Unavailable
    ) {
        return Ok(document_divergence);
    }

    evaluate_control(control, context, value.clone()).map(|evaluated| evaluated.divergence)
}

fn control_options(
    control: &CustomizerControl,
    current_value: Option<&ControlValue>,
    effective_domain: &FeasibleControlDomain,
    context: ControlEvaluationContext<'_>,
    preview_sample_count: usize,
) -> Result<Vec<FoundryOptionCard>, ControlEvaluationError> {
    match &control.kind {
        ControlKind::ChoiceGallery { options } => Ok(options
            .iter()
            .map(|option| {
                let value = ControlValue::Choice(option.value.clone());
                option_card(
                    control,
                    value,
                    option.label.clone(),
                    None,
                    Some(option.preview.preview_id.clone()),
                    current_value,
                    effective_domain,
                )
            })
            .collect()),
        ControlKind::ProviderGallery { role, options } => Ok(options
            .iter()
            .map(|option| {
                let value = ControlValue::Provider(option.provider_id.clone());
                option_card(
                    control,
                    value,
                    option.label.clone(),
                    Some(role.clone()),
                    Some(option.preview.preview_id.clone()),
                    current_value,
                    effective_domain,
                )
            })
            .collect()),
        ControlKind::ContinuousAxis { .. }
        | ControlKind::IntegerStepper { .. }
        | ControlKind::Toggle { .. } => {
            whole_model_preview_sample_requests_with_count(control, context, preview_sample_count)
                .map(|requests| {
                    requests
                        .into_iter()
                        .map(|request| {
                            option_card(
                                control,
                                request.value.clone(),
                                control_value_label(&request.value),
                                None,
                                Some(request.preview_id),
                                current_value,
                                effective_domain,
                            )
                        })
                        .collect()
                })
        }
    }
}

fn option_card(
    control: &CustomizerControl,
    value: ControlValue,
    label: String,
    provider_role: Option<String>,
    preview_id: Option<String>,
    current_value: Option<&ControlValue>,
    effective_domain: &FeasibleControlDomain,
) -> FoundryOptionCard {
    FoundryOptionCard {
        control_id: control.id.clone(),
        unavailable_reason: option_unavailable_reason(effective_domain, &value),
        selected: current_value == Some(&value),
        value,
        label,
        provider_role,
        preview_id,
        rgba8: Vec::new(),
        width: 0,
        height: 0,
        camera: None,
    }
}

fn option_unavailable_reason(
    effective_domain: &FeasibleControlDomain,
    value: &ControlValue,
) -> Option<String> {
    if let Some(reason) = effective_domain.unavailable_reason(value) {
        Some(reason.to_owned())
    } else if effective_domain.contains_available_value(value) {
        None
    } else {
        Some("outside current constraints".to_owned())
    }
}

fn control_is_locked(locks: &[FoundryLock], control: &CustomizerControl) -> bool {
    lock_target_is_locked(locks, &FoundryLockTarget::Control(control.id.clone()))
        || match &control.kind {
            ControlKind::ProviderGallery { role, .. } => {
                lock_target_is_locked(locks, &FoundryLockTarget::Provider(role.clone()))
                    || lock_target_is_locked(locks, &FoundryLockTarget::Role(role.clone()))
            }
            ControlKind::ContinuousAxis { .. }
            | ControlKind::IntegerStepper { .. }
            | ControlKind::Toggle { .. }
            | ControlKind::ChoiceGallery { .. } => false,
        }
}

fn lock_target_is_locked(locks: &[FoundryLock], target: &FoundryLockTarget) -> bool {
    locks
        .iter()
        .any(|lock| lock.target == *target && lock.mode == FoundryLockMode::Locked)
}

fn control_kind_label(kind: &ControlKind) -> &'static str {
    match kind {
        ControlKind::ContinuousAxis { .. } => "Macro Axis",
        ControlKind::IntegerStepper { .. } => "Stepper",
        ControlKind::Toggle { .. } => "Toggle",
        ControlKind::ChoiceGallery { .. } => "Choice Gallery",
        ControlKind::ProviderGallery { .. } => "Provider Gallery",
    }
}

fn control_presentation(kind: &ControlKind) -> FoundryControlPresentation {
    match kind {
        ControlKind::ContinuousAxis { .. } => FoundryControlPresentation::ContinuousMacroAxis,
        ControlKind::IntegerStepper { .. } => FoundryControlPresentation::Stepper,
        ControlKind::Toggle { .. } => FoundryControlPresentation::Toggle,
        ControlKind::ChoiceGallery { .. } => FoundryControlPresentation::ChoiceGallery,
        ControlKind::ProviderGallery { .. } => FoundryControlPresentation::ProviderGallery,
    }
}

fn control_technical_path(control: &CustomizerControl) -> String {
    match &control.kind {
        ControlKind::ProviderGallery { role, .. } => {
            format!("controls.{}.providers.{role}", control.id)
        }
        _ if !control.bindings.is_empty() => {
            let slots = control
                .bindings
                .iter()
                .map(|binding| binding.slot.as_str())
                .collect::<Vec<_>>()
                .join(",");
            format!("controls.{}.bindings.{slots}", control.id)
        }
        _ => format!("controls.{}", control.id),
    }
}

fn trimmed_scalar_label(value: f32) -> String {
    let mut label = format!("{value:.3}");
    while label.contains('.') && label.ends_with('0') {
        label.pop();
    }
    if label.ends_with('.') {
        label.pop();
    }
    if label == "-0" { "0".to_owned() } else { label }
}
