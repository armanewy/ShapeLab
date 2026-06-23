//! Whole-model customizer control contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_family::{
    FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot, ParameterExecutionPolicy,
};
use shape_family_compile::FamilyValue;

use crate::CUSTOMIZER_PROFILE_SCHEMA_VERSION;
use crate::document::FoundryAssetDocument;
use crate::edit::TouchedSemanticTarget;

/// Default number of whole-model samples for topology-preserving sliders.
pub const DEFAULT_PREVIEW_SAMPLE_COUNT: usize = 5;

/// Inclusive interval for continuous control domains.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedInterval {
    /// Minimum accepted value.
    pub minimum: f32,
    /// Maximum accepted value.
    pub maximum: f32,
}

/// Certification level for a feasible control domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainCertification {
    /// Every value inside each interval is certified valid.
    CertifiedContinuous,
    /// Only the listed discrete values are certified valid.
    DiscreteSamples,
    /// The domain is known but not certified for live slider interpolation.
    Uncertified {
        /// Human-readable reason.
        reason: String,
    },
}

/// Nonconvex domain of values that can be safely offered for a control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeasibleControlDomain {
    /// Certified continuous intervals.
    pub continuous_intervals: Vec<ClosedInterval>,
    /// Certified discrete values.
    pub discrete_values: Vec<ControlValue>,
    /// Unavailable option IDs and their deterministic reasons.
    pub unavailable_options: BTreeMap<String, String>,
    /// Certification level.
    pub certification: DomainCertification,
}

impl Default for FeasibleControlDomain {
    fn default() -> Self {
        Self {
            continuous_intervals: Vec::new(),
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::Uncertified {
                reason: "not evaluated".to_owned(),
            },
        }
    }
}

impl FeasibleControlDomain {
    /// Return true when the domain contains at least one currently available value.
    #[must_use]
    pub fn has_available_values(&self) -> bool {
        !self.continuous_intervals.is_empty()
            || self
                .discrete_values
                .iter()
                .any(|value| !self.unavailable_options.contains_key(&value.option_key()))
    }

    /// Return true when the supplied value is available in this domain.
    #[must_use]
    pub fn contains_available_value(&self, value: &ControlValue) -> bool {
        if self.unavailable_options.contains_key(&value.option_key()) {
            return false;
        }
        control_value_in_domain(self, value)
    }

    /// Return the deterministic unavailable reason for a value, if present.
    #[must_use]
    pub fn unavailable_reason(&self, value: &ControlValue) -> Option<&str> {
        self.unavailable_options
            .get(&value.option_key())
            .map(String::as_str)
    }
}

/// Canonical control value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlValue {
    /// Floating-point scalar.
    Scalar(f32),
    /// Integer value.
    Integer(i64),
    /// Boolean value.
    Toggle(bool),
    /// Symbolic choice.
    Choice(String),
    /// Provider ID.
    Provider(String),
}

impl ControlValue {
    /// Deterministic key used for unavailable options and explanations.
    #[must_use]
    pub fn option_key(&self) -> String {
        match self {
            Self::Scalar(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Toggle(value) => value.to_string(),
            Self::Choice(value) | Self::Provider(value) => value.clone(),
        }
    }
}

impl From<FamilyValue> for ControlValue {
    fn from(value: FamilyValue) -> Self {
        match value {
            FamilyValue::Scalar(value) => Self::Scalar(value),
            FamilyValue::Integer(value) => Self::Integer(i64::from(value)),
            FamilyValue::Toggle(value) => Self::Toggle(value),
            FamilyValue::Choice(value) => Self::Choice(value),
        }
    }
}

/// How a control maps its normalized value into family slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseCurve {
    /// Direct normalized value.
    Linear,
    /// Piecewise points in authored control space.
    Piecewise {
        /// Ordered `[input, output]` points.
        points: Vec<[f32; 2]>,
        /// Whether outputs must be monotonic.
        monotonic: bool,
    },
}

/// Binding from a visible control to one family parameter slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlSlotBinding {
    /// Family parameter slot ID.
    pub slot: String,
    /// Required/advisory/runtime nature of the family slot.
    pub slot_policy: ParameterExecutionPolicy,
    /// Response curve from control value to slot value.
    pub response: ResponseCurve,
}

/// Whole-model preview reference used by gallery options and candidate samples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WholeModelPreviewRef {
    /// Stable preview request ID.
    pub preview_id: String,
    /// Optional recipe/artifact fingerprint available after a build.
    pub artifact_fingerprint: Option<String>,
}

/// One symbolic option for a choice gallery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceOption {
    /// Option value.
    pub value: String,
    /// Human-facing label.
    pub label: String,
    /// Whole-model preview for this option.
    pub preview: WholeModelPreviewRef,
}

/// One provider gallery option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderOption {
    /// Provider ID.
    pub provider_id: String,
    /// Human-facing label.
    pub label: String,
    /// Whole-model preview for this provider.
    pub preview: WholeModelPreviewRef,
}

/// Control kind and authored option set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlKind {
    /// Continuous normalized axis that may fan out to multiple required slots.
    ContinuousAxis {
        /// Authored default normalized value.
        default: f32,
    },
    /// Integer stepper.
    IntegerStepper {
        /// Authored default integer.
        default: i64,
    },
    /// Binary toggle.
    Toggle {
        /// Authored default value.
        default: bool,
    },
    /// Choice gallery with whole-model option previews.
    ChoiceGallery {
        /// Gallery options.
        options: Vec<ChoiceOption>,
    },
    /// Provider gallery with whole-model option previews.
    ProviderGallery {
        /// Family role whose provider is selected.
        role: String,
        /// Provider options.
        options: Vec<ProviderOption>,
    },
}

/// Whether a control can be previewed continuously or only at discrete states.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlTopologyBehavior {
    /// Preserves topology and can be previewed continuously if certified.
    TopologyPreserving,
    /// Changes topology and must be represented by discrete values/options.
    TopologyChanging,
    /// Consumed by runtime/export adapters only.
    RuntimeOnly,
}

/// Advanced divergence state shown when semantic source and generated asset disagree.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDivergence {
    /// Control state is synced with generated geometry.
    Synced,
    /// A local override touched one of this control's semantic targets.
    DivergedByOverride,
    /// The control cannot currently be applied.
    Unavailable,
    /// The control is narrowed by constraints.
    ConstraintLimited,
}

/// One visible customizer control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomizerControl {
    /// Stable control ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Optional section ID.
    pub section: Option<String>,
    /// Whether this control appears in the novice primary surface.
    pub primary: bool,
    /// Whether this control is visible at all.
    pub visible: bool,
    /// Control kind.
    pub kind: ControlKind,
    /// Family slot bindings owned by this control.
    pub bindings: Vec<ControlSlotBinding>,
    /// Authored and conformance-narrowed feasible domain.
    pub domain: FeasibleControlDomain,
    /// Topology behavior.
    pub topology_behavior: ControlTopologyBehavior,
    /// Current divergence.
    pub divergence: ControlDivergence,
}

/// Section in a customizer profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomizerSection {
    /// Stable section ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
}

/// Candidate generation strategy surfaced by a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateStrategy {
    /// Stable strategy ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Control IDs this strategy may edit.
    pub control_ids: Vec<String>,
}

/// Whole-model customizer profile for one family/style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomizerProfile {
    /// Customizer profile schema version.
    pub schema_version: u32,
    /// Family ID.
    pub family_id: String,
    /// Optional style ID.
    pub style_id: Option<String>,
    /// Sections.
    pub sections: Vec<CustomizerSection>,
    /// Controls.
    pub controls: Vec<CustomizerControl>,
    /// Candidate strategies.
    pub candidate_strategies: Vec<CandidateStrategy>,
    /// Maximum controls allowed in the primary novice surface.
    pub maximum_primary_controls: u32,
}

impl CustomizerProfile {
    /// Create an empty profile using the default primary-control limit.
    #[must_use]
    pub fn empty(family_id: impl Into<String>, style_id: Option<String>) -> Self {
        Self {
            schema_version: CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            family_id: family_id.into(),
            style_id,
            sections: Vec::new(),
            controls: Vec::new(),
            candidate_strategies: Vec::new(),
            maximum_primary_controls: 7,
        }
    }
}

/// Conformance-provided feasible domains for controls.
///
/// This intentionally returns a full [`FeasibleControlDomain`] rather than a
/// single numeric range so discrete and nonconvex feasibility survives
/// constraint narrowing.
pub trait ConstraintRangeProvider {
    /// Return a conformance-provided feasible domain for a control, if known.
    fn feasible_control_domain(&self, control_id: &str) -> Option<FeasibleControlDomain>;
}

/// Constraint provider that leaves authored/family domains unchanged.
#[derive(Debug, Copy, Clone, Default)]
pub struct NoConstraintRangeProvider;

impl ConstraintRangeProvider for NoConstraintRangeProvider {
    fn feasible_control_domain(&self, _control_id: &str) -> Option<FeasibleControlDomain> {
        None
    }
}

impl ConstraintRangeProvider for BTreeMap<String, FeasibleControlDomain> {
    fn feasible_control_domain(&self, control_id: &str) -> Option<FeasibleControlDomain> {
        self.get(control_id).cloned()
    }
}

/// Shared no-op provider for callers that only need authored and family domains.
pub static NO_CONSTRAINT_RANGE_PROVIDER: NoConstraintRangeProvider = NoConstraintRangeProvider;

/// Context used by safe control evaluation.
#[derive(Copy, Clone)]
pub struct ControlEvaluationContext<'a> {
    /// Family parameter slots that constrain and receive control output.
    pub family_parameter_slots: &'a [FamilyParameterSlot],
    /// Conformance-provided feasible domains.
    pub constraint_range_provider: &'a dyn ConstraintRangeProvider,
}

impl<'a> ControlEvaluationContext<'a> {
    /// Build a context without conformance narrowing.
    #[must_use]
    pub fn new(family_parameter_slots: &'a [FamilyParameterSlot]) -> Self {
        Self {
            family_parameter_slots,
            constraint_range_provider: &NO_CONSTRAINT_RANGE_PROVIDER,
        }
    }

    /// Build a context with conformance-provided feasible domains.
    #[must_use]
    pub fn with_constraint_range_provider(
        family_parameter_slots: &'a [FamilyParameterSlot],
        constraint_range_provider: &'a dyn ConstraintRangeProvider,
    ) -> Self {
        Self {
            family_parameter_slots,
            constraint_range_provider,
        }
    }
}

/// Safe-control evaluation failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlEvaluationError {
    /// The requested control ID does not exist in the profile.
    UnknownControl {
        /// Control ID.
        control_id: String,
    },
    /// The supplied value variant does not match the control kind.
    WrongValueKind {
        /// Control ID.
        control_id: String,
    },
    /// A scalar control value was not finite.
    NonFiniteControlValue {
        /// Control ID.
        control_id: String,
    },
    /// A response curve produced a non-finite value.
    NonFiniteControlOutput {
        /// Control ID.
        control_id: String,
        /// Family slot ID.
        slot: String,
    },
    /// No value remains after authored, family, and conformance domains intersect.
    EmptyFeasibleDomain {
        /// Control ID.
        control_id: String,
    },
    /// A control references a family slot that is absent from the family schema.
    UnknownFamilySlot {
        /// Control ID.
        control_id: String,
        /// Family slot ID.
        slot: String,
    },
    /// A binding cannot map this control value into the target family slot kind.
    IncompatibleFamilySlot {
        /// Control ID.
        control_id: String,
        /// Family slot ID.
        slot: String,
    },
    /// Two controls tried to own the same family slot at runtime.
    ConflictingSlotOwnership {
        /// Family slot ID.
        slot: String,
        /// First control ID.
        first_control_id: String,
        /// Second control ID.
        second_control_id: String,
    },
    /// Two controls tried to own the same provider role at runtime.
    ConflictingProviderOwnership {
        /// Provider role.
        role: String,
        /// First control ID.
        first_control_id: String,
        /// Second control ID.
        second_control_id: String,
    },
    /// A symbolic option was not authored by the control.
    UnknownOption {
        /// Control ID.
        control_id: String,
        /// Option key.
        option: String,
    },
    /// A symbolic option exists but is currently unavailable.
    UnavailableOption {
        /// Control ID.
        control_id: String,
        /// Option key.
        option: String,
        /// Deterministic reason.
        reason: String,
    },
    /// A symbolic control has no available default option.
    MissingDefaultOption {
        /// Control ID.
        control_id: String,
    },
}

/// Evaluated output for one control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatedControl {
    /// Control ID.
    pub control_id: String,
    /// Canonical control value used for evaluation.
    pub value: ControlValue,
    /// Family parameter values emitted by bindings.
    pub slot_values: BTreeMap<String, FamilyValue>,
    /// Provider selections emitted by provider gallery controls.
    pub provider_selections: BTreeMap<String, String>,
    /// Effective authored ∩ family ∩ conformance domain.
    pub domain: FeasibleControlDomain,
    /// Divergence state for this evaluation.
    pub divergence: ControlDivergence,
}

/// Evaluated output for a whole control state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatedControlState {
    /// Canonical control values keyed by control ID.
    pub control_values: BTreeMap<String, ControlValue>,
    /// Merged family parameter values keyed by slot ID.
    pub family_parameters: BTreeMap<String, FamilyValue>,
    /// Merged provider selections keyed by role ID.
    pub provider_selections: BTreeMap<String, String>,
    /// Per-control evaluation rows.
    pub controls: BTreeMap<String, EvaluatedControl>,
}

/// One changed family slot in a control delta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlSlotDelta {
    /// Family slot ID.
    pub slot: String,
    /// Previous evaluated value.
    pub previous: Option<FamilyValue>,
    /// Current evaluated value.
    pub current: Option<FamilyValue>,
}

/// One changed provider selection in a control delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlProviderDelta {
    /// Provider role.
    pub role: String,
    /// Previous provider ID.
    pub previous: Option<String>,
    /// Current provider ID.
    pub current: Option<String>,
}

/// Deterministic human-facing explanation for a control delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDeltaExplanation {
    /// Stable subject path.
    pub subject: String,
    /// Stable explanation code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Deterministic delta between two control values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlDelta {
    /// Control ID.
    pub control_id: String,
    /// Previous semantic control value, if explicitly present.
    pub previous: Option<ControlValue>,
    /// Current canonical control value.
    pub current: ControlValue,
    /// Changed family slots.
    pub slot_deltas: Vec<ControlSlotDelta>,
    /// Changed provider roles.
    pub provider_deltas: Vec<ControlProviderDelta>,
    /// Deterministic explanations.
    pub explanations: Vec<ControlDeltaExplanation>,
}

/// Build mode requested for a whole-model preview sample.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlBuildRequestKind {
    /// Approximate/interactive sample suitable for hover or drag preview.
    PreviewSample,
    /// Exact build requested when the user releases a control.
    ExactOnRelease,
}

/// Whole-model preview request for a control value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WholeModelPreviewSampleRequest {
    /// Stable request ID.
    pub preview_id: String,
    /// Control ID being sampled.
    pub control_id: String,
    /// Sample index.
    pub sample_index: u32,
    /// Control value to build.
    pub value: ControlValue,
    /// Build request kind.
    pub build_kind: ControlBuildRequestKind,
}

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
        ControlKind::ContinuousAxis { .. } => FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: -1.0,
                maximum: 1.0,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
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
