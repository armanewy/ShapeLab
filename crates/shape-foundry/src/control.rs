//! Whole-model customizer control contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_family::ParameterExecutionPolicy;
use shape_family_compile::FamilyValue;

use crate::CUSTOMIZER_PROFILE_SCHEMA_VERSION;

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
