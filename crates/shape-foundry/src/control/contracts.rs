
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
