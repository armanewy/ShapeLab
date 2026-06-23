//! Character proportion and joint grammar.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CHARACTER_GRAMMAR_SCHEMA_VERSION, CharacterControlId, CharacterGrammarId, ScalarRange,
    UnitQuaternion,
};

const PROPORTION_GRAMMAR_ID: &str = "shape.character.proportion.v1";
const POSE_NORMALIZATION_ID: &str = "proportion.pose.normalization";
const POSE_HASH_DOMAIN: &[u8] = b"shape-lab.character.normalized-pose.v1\0";
const EPSILON: f32 = 0.0001;

/// Stable local joint identifier for proportion and pose contracts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CharacterJointId(pub String);

impl CharacterJointId {
    /// Borrow the stable string identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable normalized-pose identifier derived from canonical pose contents.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CharacterPoseId(pub String);

impl CharacterPoseId {
    /// Borrow the stable string identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Semantic category for a scalar character proportion control.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProportionControlGroup {
    /// Overall height and head-count controls.
    Height,
    /// Arm and leg length controls.
    LimbProportion,
    /// Torso length, width, taper, and depth controls.
    TorsoShape,
    /// Shoulder width and slope controls.
    Shoulder,
    /// Pelvis width and tilt controls.
    Pelvis,
    /// Hand scale and palm/finger controls.
    Hand,
    /// Foot length, width, and arch controls.
    Foot,
    /// Controls used by normalized-pose application.
    PoseNormalization,
}

/// Runtime-neutral semantic meaning for a proportion control.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProportionControlSemantic {
    /// Character stature in meters.
    Stature,
    /// Number of head lengths contained in total stature.
    HeadCount,
    /// Arm span multiplier relative to neutral shoulder-to-wrist distance.
    ArmSpan,
    /// Forearm share of shoulder-to-wrist distance.
    ForearmRatio,
    /// Leg length multiplier relative to neutral hip-to-ankle distance.
    LegLength,
    /// Shin share of hip-to-ankle distance.
    ShinRatio,
    /// Torso height multiplier from pelvis to neck.
    TorsoLength,
    /// Ribcage width multiplier.
    RibcageWidth,
    /// Waist width multiplier.
    WaistWidth,
    /// Torso depth multiplier.
    TorsoDepth,
    /// Shoulder width multiplier.
    ShoulderWidth,
    /// Shoulder slope in degrees.
    ShoulderSlope,
    /// Pelvis width multiplier.
    PelvisWidth,
    /// Pelvis tilt in degrees.
    PelvisTilt,
    /// Whole-hand scale multiplier.
    HandScale,
    /// Palm length share of hand length.
    PalmLength,
    /// Finger length share of hand length.
    FingerLength,
    /// Foot length multiplier.
    FootLength,
    /// Foot width multiplier.
    FootWidth,
    /// Foot arch height multiplier.
    ArchHeight,
    /// Blend weight for applying normalized rest-pose correction.
    RestPoseBlend,
}

/// Unit contract for a proportion control's scalar range.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProportionUnit {
    /// Meters in normalized character authoring space.
    Meters,
    /// Number of head lengths contained in total stature.
    HeadCount,
    /// Unitless multiplier around a neutral authored value of 1.0.
    Multiplier,
    /// Unitless ratio where 0.0 to 1.0 spans a parent segment.
    Ratio,
    /// Degrees.
    Degrees,
    /// Unitless normalized blend weight.
    NormalizedWeight,
}

/// One scalar semantic control in the proportion grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProportionControl {
    /// Stable deterministic control identifier.
    pub id: CharacterControlId,
    /// High-level category used by editors and validation.
    pub group: ProportionControlGroup,
    /// Runtime-neutral semantic meaning.
    pub semantic: ProportionControlSemantic,
    /// Inclusive finite authored scalar range.
    pub range: ScalarRange,
    /// Scalar unit contract.
    pub unit: ProportionUnit,
}

impl ProportionControl {
    /// Returns true when the control has a stable ID and valid finite range.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        stable_identifier_is_valid(&self.id.0) && self.range.is_valid()
    }
}

/// Body side for symmetric joint contracts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JointSide {
    /// Midline joint.
    Center,
    /// Character left side.
    Left,
    /// Character right side.
    Right,
}

/// Semantic role for a joint frame.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JointFrameRole {
    /// Model root.
    Root,
    /// Pelvis center.
    Pelvis,
    /// Spine segment.
    Spine,
    /// Neck base.
    Neck,
    /// Head center.
    Head,
    /// Clavicle segment.
    Clavicle,
    /// Shoulder joint.
    Shoulder,
    /// Elbow joint.
    Elbow,
    /// Wrist joint.
    Wrist,
    /// Hand center.
    Hand,
    /// Hip joint.
    Hip,
    /// Knee joint.
    Knee,
    /// Ankle joint.
    Ankle,
    /// Foot center.
    Foot,
    /// Toe pivot.
    Toe,
}

/// Orthonormal basis contract for a joint frame.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointFrameAxes {
    /// Local +X/right axis in normalized character space.
    pub right: [f32; 3],
    /// Local +Y/up axis in normalized character space.
    pub up: [f32; 3],
    /// Local +Z/forward axis in normalized character space.
    pub forward: [f32; 3],
}

impl JointFrameAxes {
    /// Canonical right-handed basis used by the default proportion grammar.
    pub const IDENTITY: Self = Self {
        right: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        forward: [0.0, 0.0, 1.0],
    };

    /// Returns true when the basis is finite, unit length, orthogonal, and right-handed.
    #[must_use]
    pub fn is_orthonormal(self) -> bool {
        vec3_is_unit(self.right)
            && vec3_is_unit(self.up)
            && vec3_is_unit(self.forward)
            && dot(self.right, self.up).abs() <= EPSILON
            && dot(self.right, self.forward).abs() <= EPSILON
            && dot(self.up, self.forward).abs() <= EPSILON
            && vec3_approx_eq(cross(self.right, self.up), self.forward)
    }
}

/// Finite authored angular and stretch limits for a joint frame.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointLimitContract {
    /// Primary bend range in degrees.
    pub bend_degrees: ScalarRange,
    /// Twist range in degrees.
    pub twist_degrees: ScalarRange,
    /// Stretch multiplier range for proportion solvers.
    pub stretch: ScalarRange,
}

impl JointLimitContract {
    /// Returns true when all ranges are finite and contain their defaults.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.bend_degrees.is_valid()
            && self.twist_degrees.is_valid()
            && positive_multiplier_range_is_valid(self.stretch)
    }
}

/// One normalized character-space joint frame and hierarchy contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointFrameContract {
    /// Stable deterministic joint identifier.
    pub id: CharacterJointId,
    /// Parent joint identifier. `None` is reserved for the single root.
    pub parent: Option<CharacterJointId>,
    /// Symmetric counterpart when the joint is lateral.
    pub mirror: Option<CharacterJointId>,
    /// Semantic role.
    pub role: JointFrameRole,
    /// Body side.
    pub side: JointSide,
    /// Rest origin in normalized character space where height is 1.0.
    pub rest_origin: [f32; 3],
    /// Rest orientation in normalized character space.
    pub rest_rotation: UnitQuaternion,
    /// Orthonormal basis contract.
    pub axes: JointFrameAxes,
    /// Finite joint limits.
    pub limits: JointLimitContract,
}

impl JointFrameContract {
    /// Returns true when this frame is locally valid without checking graph links.
    #[must_use]
    pub fn is_locally_valid(&self) -> bool {
        stable_identifier_is_valid(&self.id.0)
            && self
                .parent
                .as_ref()
                .is_none_or(|parent| stable_identifier_is_valid(&parent.0))
            && self
                .mirror
                .as_ref()
                .is_none_or(|mirror| stable_identifier_is_valid(&mirror.0))
            && vec3_is_finite(self.rest_origin)
            && self.rest_rotation.is_canonical()
            && self.axes.is_orthonormal()
            && self.limits.is_valid()
    }
}

/// Coordinate-space contract for normalized pose samples.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PoseNormalizationSpace {
    /// Model-space samples are shifted so the root joint is at the origin.
    RootRelativeModel,
}

/// Deterministic duplicate policy for multiple samples with the same joint ID.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DuplicatePosePolicy {
    /// Keep the lexicographically first canonical sample for a joint.
    StableFirstByCanonicalValue,
}

/// Contract for deterministic pose normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseNormalizationContract {
    /// Stable deterministic contract identifier.
    pub id: CharacterControlId,
    /// Root joint used for root-relative normalization.
    pub root_joint: CharacterJointId,
    /// Joints expected by a full-body normalized pose.
    pub required_joints: Vec<CharacterJointId>,
    /// Pose coordinate-space rule.
    pub space: PoseNormalizationSpace,
    /// Duplicate-joint sample rule.
    pub duplicate_policy: DuplicatePosePolicy,
    /// Inclusive scale range applied while normalizing samples.
    pub scale_range: ScalarRange,
}

impl PoseNormalizationContract {
    /// Returns true when the standalone pose-normalization contract is finite and stable.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        stable_identifier_is_valid(&self.id.0)
            && stable_identifier_is_valid(&self.root_joint.0)
            && self
                .required_joints
                .iter()
                .all(|joint| stable_identifier_is_valid(&joint.0))
            && ids_are_strictly_sorted(self.required_joints.iter().map(CharacterJointId::as_str))
            && self.required_joints.contains(&self.root_joint)
            && positive_multiplier_range_is_valid(self.scale_range)
    }
}

/// Complete default character proportion grammar contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterProportionGrammar {
    /// Stable grammar namespace.
    pub id: CharacterGrammarId,
    /// Shared character grammar schema version.
    pub schema_version: u32,
    /// Scalar semantic controls in deterministic ID order.
    pub controls: Vec<ProportionControl>,
    /// Joint frame hierarchy in parent-before-child order.
    pub joint_frames: Vec<JointFrameContract>,
    /// Deterministic pose normalization contract.
    pub pose_normalization: PoseNormalizationContract,
}

impl CharacterProportionGrammar {
    /// Validate this grammar and return a deterministic report.
    #[must_use]
    pub fn validation_report(&self) -> ProportionValidationReport {
        validate_proportion_grammar(self)
    }

    /// Returns true when the grammar validation report has no issues.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validation_report().is_valid()
    }
}

/// Deterministic validation code for proportion grammar issues.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProportionValidationCode {
    /// Grammar ID is malformed.
    InvalidGrammarId,
    /// Grammar schema version does not match the crate contract.
    SchemaVersionMismatch,
    /// Control ID is malformed.
    InvalidControlId,
    /// Control IDs are not strictly sorted.
    ControlOrder,
    /// Control ID appears more than once.
    DuplicateControlId,
    /// Control range is not finite or its default is outside the range.
    InvalidControlRange,
    /// Control unit and range semantics are incompatible.
    InvalidControlUnit,
    /// Control semantic appears more than once.
    DuplicateControlSemantic,
    /// Required control semantic is missing.
    MissingControlSemantic,
    /// Control contract does not match the required semantic declaration.
    InvalidControlSemantic,
    /// A required control group has no semantic control.
    MissingControlGroup,
    /// Joint ID is malformed.
    InvalidJointId,
    /// Joint IDs are duplicated.
    DuplicateJointId,
    /// The joint hierarchy does not have exactly one root.
    RootJointCount,
    /// A joint parent is missing from the hierarchy.
    MissingJointParent,
    /// A joint parent appears after its child.
    JointParentOrder,
    /// Joint frame basis, orientation, origin, or limits are invalid.
    InvalidJointFrame,
    /// Mirror joint link is missing or not reciprocal.
    InvalidMirrorJoint,
    /// Mirrored lateral joint origins are not symmetric.
    InvalidMirrorOrigin,
    /// Pose normalization contract is malformed.
    InvalidPoseNormalization,
    /// Pose normalization references a joint that is not in the hierarchy.
    MissingPoseJoint,
}

/// One deterministic validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProportionValidationIssue {
    /// Machine-readable validation code.
    pub code: ProportionValidationCode,
    /// Stable subject identifier.
    pub subject: String,
    /// Human-readable issue summary.
    pub message: String,
}

/// Deterministic validation report for a proportion grammar.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProportionValidationReport {
    /// Validation issues in deterministic traversal order.
    pub issues: Vec<ProportionValidationIssue>,
}

impl ProportionValidationReport {
    /// Returns true when no issues were reported.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        code: ProportionValidationCode,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ProportionValidationIssue {
            code,
            subject: subject.into(),
            message: message.into(),
        });
    }
}

/// Raw pose sample before deterministic normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointPoseSample {
    /// Stable joint identifier.
    pub joint: CharacterJointId,
    /// Authored local or model-space rotation.
    pub rotation: UnitQuaternion,
    /// Authored model-space translation.
    pub translation: [f32; 3],
    /// Authored joint scale.
    pub scale: f32,
}

/// Canonical pose sample after normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedJointPose {
    /// Stable joint identifier.
    pub joint: CharacterJointId,
    /// Unit quaternion with deterministic sign.
    pub rotation: UnitQuaternion,
    /// Root-relative model-space translation.
    pub translation: [f32; 3],
    /// Clamped finite joint scale.
    pub scale: f32,
}

impl NormalizedJointPose {
    /// Returns true when all pose values are finite and canonical.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        stable_identifier_is_valid(&self.joint.0)
            && self.rotation.is_canonical()
            && vec3_is_finite(self.translation)
            && self.scale.is_finite()
            && self.scale > 0.0
    }
}

/// Normalized pose with a deterministic content-derived identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedPose {
    /// Stable BLAKE3-derived ID for canonical pose contents.
    pub id: CharacterPoseId,
    /// Canonical joint poses in deterministic joint ID order.
    pub joints: Vec<NormalizedJointPose>,
}

impl NormalizedPose {
    /// Returns true when the pose is sorted, finite, and its ID matches its contents.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        stable_identifier_is_valid(&self.id.0)
            && self.joints.iter().all(NormalizedJointPose::is_valid)
            && ids_are_strictly_sorted(self.joints.iter().map(|pose| pose.joint.as_str()))
            && stable_pose_id(&self.joints) == self.id
    }
}

/// Errors emitted by deterministic pose normalization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PoseNormalizationError {
    /// The pose normalization contract itself is invalid.
    #[error("invalid pose normalization contract: {0}")]
    InvalidContract(String),
    /// A joint ID in a pose sample is malformed.
    #[error("invalid joint ID in pose sample: {0}")]
    InvalidJointId(String),
    /// A joint ID is validly formed but outside the normalization contract.
    #[error("pose sample references unknown joint: {0}")]
    UnknownJoint(String),
    /// A full-body pose sample set is missing a required joint.
    #[error("pose sample is missing required joint: {0}")]
    MissingRequiredJoint(String),
    /// Translation contains a non-finite component.
    #[error("pose sample for {0} contains a non-finite translation")]
    NonFiniteTranslation(String),
    /// Scale is non-finite.
    #[error("pose sample for {0} contains a non-finite scale")]
    NonFiniteScale(String),
    /// Rotation contains a non-finite component.
    #[error("pose sample for {0} contains a non-finite rotation")]
    NonFiniteRotation(String),
    /// Rotation has zero length and cannot be normalized.
    #[error("pose sample for {0} contains a zero-length rotation")]
    ZeroLengthRotation(String),
}

/// Build the default versioned character proportion grammar.
#[must_use]
pub fn proportion_grammar() -> CharacterProportionGrammar {
    CharacterProportionGrammar {
        id: CharacterGrammarId(PROPORTION_GRAMMAR_ID.to_owned()),
        schema_version: CHARACTER_GRAMMAR_SCHEMA_VERSION,
        controls: proportion_controls(),
        joint_frames: joint_frame_contracts(),
        pose_normalization: pose_normalization_contract(),
    }
}

/// Build scalar controls in deterministic ID order.
#[must_use]
pub fn proportion_controls() -> Vec<ProportionControl> {
    let mut controls = vec![
        control(
            "proportion.height.stature",
            ProportionControlGroup::Height,
            ProportionControlSemantic::Stature,
            range(1.20, 2.40, 1.75),
            ProportionUnit::Meters,
        ),
        control(
            "proportion.height.head_count",
            ProportionControlGroup::Height,
            ProportionControlSemantic::HeadCount,
            range(6.0, 9.0, 7.5),
            ProportionUnit::HeadCount,
        ),
        control(
            "proportion.limb.arm_span",
            ProportionControlGroup::LimbProportion,
            ProportionControlSemantic::ArmSpan,
            range(0.85, 1.15, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.limb.forearm_ratio",
            ProportionControlGroup::LimbProportion,
            ProportionControlSemantic::ForearmRatio,
            range(0.42, 0.58, 0.50),
            ProportionUnit::Ratio,
        ),
        control(
            "proportion.limb.leg_length",
            ProportionControlGroup::LimbProportion,
            ProportionControlSemantic::LegLength,
            range(0.85, 1.15, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.limb.shin_ratio",
            ProportionControlGroup::LimbProportion,
            ProportionControlSemantic::ShinRatio,
            range(0.42, 0.58, 0.50),
            ProportionUnit::Ratio,
        ),
        control(
            "proportion.torso.length",
            ProportionControlGroup::TorsoShape,
            ProportionControlSemantic::TorsoLength,
            range(0.85, 1.15, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.torso.ribcage_width",
            ProportionControlGroup::TorsoShape,
            ProportionControlSemantic::RibcageWidth,
            range(0.80, 1.25, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.torso.waist_width",
            ProportionControlGroup::TorsoShape,
            ProportionControlSemantic::WaistWidth,
            range(0.70, 1.20, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.torso.depth",
            ProportionControlGroup::TorsoShape,
            ProportionControlSemantic::TorsoDepth,
            range(0.75, 1.25, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.shoulder.width",
            ProportionControlGroup::Shoulder,
            ProportionControlSemantic::ShoulderWidth,
            range(0.80, 1.25, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.shoulder.slope",
            ProportionControlGroup::Shoulder,
            ProportionControlSemantic::ShoulderSlope,
            range(-20.0, 20.0, 0.0),
            ProportionUnit::Degrees,
        ),
        control(
            "proportion.pelvis.width",
            ProportionControlGroup::Pelvis,
            ProportionControlSemantic::PelvisWidth,
            range(0.80, 1.25, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.pelvis.tilt",
            ProportionControlGroup::Pelvis,
            ProportionControlSemantic::PelvisTilt,
            range(-18.0, 18.0, 0.0),
            ProportionUnit::Degrees,
        ),
        control(
            "proportion.hand.scale",
            ProportionControlGroup::Hand,
            ProportionControlSemantic::HandScale,
            range(0.75, 1.30, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.hand.palm_length",
            ProportionControlGroup::Hand,
            ProportionControlSemantic::PalmLength,
            range(0.40, 0.62, 0.52),
            ProportionUnit::Ratio,
        ),
        control(
            "proportion.hand.finger_length",
            ProportionControlGroup::Hand,
            ProportionControlSemantic::FingerLength,
            range(0.38, 0.60, 0.48),
            ProportionUnit::Ratio,
        ),
        control(
            "proportion.foot.length",
            ProportionControlGroup::Foot,
            ProportionControlSemantic::FootLength,
            range(0.75, 1.30, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.foot.width",
            ProportionControlGroup::Foot,
            ProportionControlSemantic::FootWidth,
            range(0.70, 1.30, 1.0),
            ProportionUnit::Multiplier,
        ),
        control(
            "proportion.foot.arch_height",
            ProportionControlGroup::Foot,
            ProportionControlSemantic::ArchHeight,
            range(0.0, 1.0, 0.5),
            ProportionUnit::NormalizedWeight,
        ),
        control(
            "proportion.pose.rest_blend",
            ProportionControlGroup::PoseNormalization,
            ProportionControlSemantic::RestPoseBlend,
            range(0.0, 1.0, 1.0),
            ProportionUnit::NormalizedWeight,
        ),
    ];
    controls.sort_by(|left, right| left.id.cmp(&right.id));
    controls
}

/// Build joint frame contracts in parent-before-child hierarchy order.
#[must_use]
pub fn joint_frame_contracts() -> Vec<JointFrameContract> {
    vec![
        joint(
            "joint.root",
            None,
            None,
            JointFrameRole::Root,
            JointSide::Center,
            [0.0, 0.0, 0.0],
        ),
        joint(
            "joint.pelvis",
            Some("joint.root"),
            None,
            JointFrameRole::Pelvis,
            JointSide::Center,
            [0.0, 0.53, 0.0],
        ),
        joint(
            "joint.spine.lower",
            Some("joint.pelvis"),
            None,
            JointFrameRole::Spine,
            JointSide::Center,
            [0.0, 0.63, 0.0],
        ),
        joint(
            "joint.spine.upper",
            Some("joint.spine.lower"),
            None,
            JointFrameRole::Spine,
            JointSide::Center,
            [0.0, 0.78, 0.0],
        ),
        joint(
            "joint.neck",
            Some("joint.spine.upper"),
            None,
            JointFrameRole::Neck,
            JointSide::Center,
            [0.0, 0.88, 0.0],
        ),
        joint(
            "joint.head",
            Some("joint.neck"),
            None,
            JointFrameRole::Head,
            JointSide::Center,
            [0.0, 0.95, 0.0],
        ),
        joint(
            "joint.clavicle.left",
            Some("joint.spine.upper"),
            Some("joint.clavicle.right"),
            JointFrameRole::Clavicle,
            JointSide::Left,
            [-0.08, 0.83, 0.0],
        ),
        joint(
            "joint.shoulder.left",
            Some("joint.clavicle.left"),
            Some("joint.shoulder.right"),
            JointFrameRole::Shoulder,
            JointSide::Left,
            [-0.18, 0.82, 0.0],
        ),
        joint(
            "joint.elbow.left",
            Some("joint.shoulder.left"),
            Some("joint.elbow.right"),
            JointFrameRole::Elbow,
            JointSide::Left,
            [-0.34, 0.62, 0.0],
        ),
        joint(
            "joint.wrist.left",
            Some("joint.elbow.left"),
            Some("joint.wrist.right"),
            JointFrameRole::Wrist,
            JointSide::Left,
            [-0.46, 0.43, 0.0],
        ),
        joint(
            "joint.hand.left",
            Some("joint.wrist.left"),
            Some("joint.hand.right"),
            JointFrameRole::Hand,
            JointSide::Left,
            [-0.49, 0.38, 0.0],
        ),
        joint(
            "joint.clavicle.right",
            Some("joint.spine.upper"),
            Some("joint.clavicle.left"),
            JointFrameRole::Clavicle,
            JointSide::Right,
            [0.08, 0.83, 0.0],
        ),
        joint(
            "joint.shoulder.right",
            Some("joint.clavicle.right"),
            Some("joint.shoulder.left"),
            JointFrameRole::Shoulder,
            JointSide::Right,
            [0.18, 0.82, 0.0],
        ),
        joint(
            "joint.elbow.right",
            Some("joint.shoulder.right"),
            Some("joint.elbow.left"),
            JointFrameRole::Elbow,
            JointSide::Right,
            [0.34, 0.62, 0.0],
        ),
        joint(
            "joint.wrist.right",
            Some("joint.elbow.right"),
            Some("joint.wrist.left"),
            JointFrameRole::Wrist,
            JointSide::Right,
            [0.46, 0.43, 0.0],
        ),
        joint(
            "joint.hand.right",
            Some("joint.wrist.right"),
            Some("joint.hand.left"),
            JointFrameRole::Hand,
            JointSide::Right,
            [0.49, 0.38, 0.0],
        ),
        joint(
            "joint.hip.left",
            Some("joint.pelvis"),
            Some("joint.hip.right"),
            JointFrameRole::Hip,
            JointSide::Left,
            [-0.09, 0.52, 0.0],
        ),
        joint(
            "joint.knee.left",
            Some("joint.hip.left"),
            Some("joint.knee.right"),
            JointFrameRole::Knee,
            JointSide::Left,
            [-0.10, 0.29, 0.01],
        ),
        joint(
            "joint.ankle.left",
            Some("joint.knee.left"),
            Some("joint.ankle.right"),
            JointFrameRole::Ankle,
            JointSide::Left,
            [-0.09, 0.06, 0.0],
        ),
        joint(
            "joint.foot.left",
            Some("joint.ankle.left"),
            Some("joint.foot.right"),
            JointFrameRole::Foot,
            JointSide::Left,
            [-0.09, 0.02, 0.08],
        ),
        joint(
            "joint.toe.left",
            Some("joint.foot.left"),
            Some("joint.toe.right"),
            JointFrameRole::Toe,
            JointSide::Left,
            [-0.09, 0.01, 0.16],
        ),
        joint(
            "joint.hip.right",
            Some("joint.pelvis"),
            Some("joint.hip.left"),
            JointFrameRole::Hip,
            JointSide::Right,
            [0.09, 0.52, 0.0],
        ),
        joint(
            "joint.knee.right",
            Some("joint.hip.right"),
            Some("joint.knee.left"),
            JointFrameRole::Knee,
            JointSide::Right,
            [0.10, 0.29, 0.01],
        ),
        joint(
            "joint.ankle.right",
            Some("joint.knee.right"),
            Some("joint.ankle.left"),
            JointFrameRole::Ankle,
            JointSide::Right,
            [0.09, 0.06, 0.0],
        ),
        joint(
            "joint.foot.right",
            Some("joint.ankle.right"),
            Some("joint.foot.left"),
            JointFrameRole::Foot,
            JointSide::Right,
            [0.09, 0.02, 0.08],
        ),
        joint(
            "joint.toe.right",
            Some("joint.foot.right"),
            Some("joint.toe.left"),
            JointFrameRole::Toe,
            JointSide::Right,
            [0.09, 0.01, 0.16],
        ),
    ]
}

/// Build the default deterministic pose normalization contract.
#[must_use]
pub fn pose_normalization_contract() -> PoseNormalizationContract {
    let mut required_joints = joint_frame_contracts()
        .into_iter()
        .map(|frame| frame.id)
        .collect::<Vec<_>>();
    required_joints.sort();
    PoseNormalizationContract {
        id: CharacterControlId(POSE_NORMALIZATION_ID.to_owned()),
        root_joint: joint_id("joint.root"),
        required_joints,
        space: PoseNormalizationSpace::RootRelativeModel,
        duplicate_policy: DuplicatePosePolicy::StableFirstByCanonicalValue,
        scale_range: range(0.50, 1.50, 1.0),
    }
}

/// Validate a proportion grammar and return deterministic issues.
#[must_use]
pub fn validate_proportion_grammar(
    grammar: &CharacterProportionGrammar,
) -> ProportionValidationReport {
    let mut report = ProportionValidationReport::default();

    if !stable_identifier_is_valid(&grammar.id.0) {
        report.push(
            ProportionValidationCode::InvalidGrammarId,
            &grammar.id.0,
            "grammar ID must be a stable lowercase identifier",
        );
    }

    if grammar.schema_version != CHARACTER_GRAMMAR_SCHEMA_VERSION {
        report.push(
            ProportionValidationCode::SchemaVersionMismatch,
            grammar.id.0.clone(),
            "grammar schema version does not match the character grammar contract",
        );
    }

    validate_controls(&grammar.controls, &mut report);
    validate_joint_frames(&grammar.joint_frames, &mut report);
    validate_pose_contract(
        &grammar.pose_normalization,
        &grammar.joint_frames,
        &mut report,
    );

    report
}

/// Normalize raw pose samples into deterministic root-relative joint order.
///
/// Quaternions are normalized and sign-canonicalized, translations are shifted
/// relative to the root joint when present, scales are clamped to the contract,
/// and duplicate joint samples keep the first canonical value after sorting.
pub fn normalize_pose_samples(
    samples: &[JointPoseSample],
) -> Result<NormalizedPose, PoseNormalizationError> {
    normalize_pose_samples_with_contract(samples, &pose_normalization_contract())
}

/// Normalize raw pose samples with an explicit contract.
pub fn normalize_pose_samples_with_contract(
    samples: &[JointPoseSample],
    contract: &PoseNormalizationContract,
) -> Result<NormalizedPose, PoseNormalizationError> {
    if !contract.is_valid() {
        return Err(PoseNormalizationError::InvalidContract(
            contract.id.0.clone(),
        ));
    }

    let mut joints = samples
        .iter()
        .map(|sample| normalize_joint_pose_sample(sample, contract))
        .collect::<Result<Vec<_>, _>>()?;

    joints.sort_by(|left, right| {
        left.joint
            .cmp(&right.joint)
            .then_with(|| pose_sort_key(left).cmp(&pose_sort_key(right)))
    });
    joints.dedup_by(|left, right| left.joint == right.joint);

    let present_joints = joints
        .iter()
        .map(|pose| pose.joint.clone())
        .collect::<BTreeSet<_>>();
    for required in &contract.required_joints {
        if !present_joints.contains(required) {
            return Err(PoseNormalizationError::MissingRequiredJoint(
                required.0.clone(),
            ));
        }
    }

    let root_translation = joints
        .iter()
        .find(|pose| pose.joint == contract.root_joint)
        .map(|pose| pose.translation)
        .ok_or_else(|| {
            PoseNormalizationError::MissingRequiredJoint(contract.root_joint.0.clone())
        })?;
    for pose in &mut joints {
        pose.translation = [
            canonical_f32(pose.translation[0] - root_translation[0]),
            canonical_f32(pose.translation[1] - root_translation[1]),
            canonical_f32(pose.translation[2] - root_translation[2]),
        ];
        if !vec3_is_finite(pose.translation) {
            return Err(PoseNormalizationError::NonFiniteTranslation(
                pose.joint.0.clone(),
            ));
        }
    }

    let pose = NormalizedPose {
        id: stable_pose_id(&joints),
        joints,
    };
    if !pose.is_valid() {
        return Err(PoseNormalizationError::InvalidContract(
            contract.id.0.clone(),
        ));
    }
    Ok(pose)
}

fn validate_controls(controls: &[ProportionControl], report: &mut ProportionValidationReport) {
    let mut seen_ids = BTreeSet::new();
    let mut seen_groups = BTreeSet::new();
    let mut seen_semantics = BTreeSet::new();
    let mut previous_id: Option<&CharacterControlId> = None;

    for control in controls {
        if !stable_identifier_is_valid(&control.id.0) {
            report.push(
                ProportionValidationCode::InvalidControlId,
                &control.id.0,
                "control ID must be stable lowercase ASCII",
            );
        }
        if previous_id.is_some_and(|previous| previous >= &control.id) {
            report.push(
                ProportionValidationCode::ControlOrder,
                &control.id.0,
                "controls must be ordered by deterministic ID",
            );
        }
        previous_id = Some(&control.id);

        if !seen_ids.insert(control.id.clone()) {
            report.push(
                ProportionValidationCode::DuplicateControlId,
                &control.id.0,
                "control ID appears more than once",
            );
        }
        if !seen_semantics.insert(control.semantic) {
            report.push(
                ProportionValidationCode::DuplicateControlSemantic,
                format!("{:?}", control.semantic),
                "control semantic appears more than once",
            );
        }
        if !control.range.is_valid() {
            report.push(
                ProportionValidationCode::InvalidControlRange,
                &control.id.0,
                "control range must be finite and contain its default",
            );
        }
        if !control_unit_accepts_range(control.unit, control.range) {
            report.push(
                ProportionValidationCode::InvalidControlUnit,
                &control.id.0,
                "control unit and range semantics are incompatible",
            );
        }
        if !control_matches_required_contract(control, required_control_contract(control.semantic))
        {
            report.push(
                ProportionValidationCode::InvalidControlSemantic,
                &control.id.0,
                "control does not match the required semantic id, group, unit, and range",
            );
        }
        seen_groups.insert(control.group);
    }

    for required in REQUIRED_CONTROL_SEMANTICS {
        if !seen_semantics.contains(required) {
            report.push(
                ProportionValidationCode::MissingControlSemantic,
                format!("{required:?}"),
                "required proportion control semantic is missing",
            );
        }
    }

    validate_hand_share_pair(controls, report);

    for group in [
        ProportionControlGroup::Height,
        ProportionControlGroup::LimbProportion,
        ProportionControlGroup::TorsoShape,
        ProportionControlGroup::Shoulder,
        ProportionControlGroup::Pelvis,
        ProportionControlGroup::Hand,
        ProportionControlGroup::Foot,
        ProportionControlGroup::PoseNormalization,
    ] {
        if !seen_groups.contains(&group) {
            report.push(
                ProportionValidationCode::MissingControlGroup,
                format!("{group:?}"),
                "required proportion control group has no controls",
            );
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct RequiredControlContract {
    id: &'static str,
    group: ProportionControlGroup,
    unit: ProportionUnit,
    range: ScalarRange,
}

const REQUIRED_CONTROL_SEMANTICS: &[ProportionControlSemantic] = &[
    ProportionControlSemantic::Stature,
    ProportionControlSemantic::HeadCount,
    ProportionControlSemantic::ArmSpan,
    ProportionControlSemantic::ForearmRatio,
    ProportionControlSemantic::LegLength,
    ProportionControlSemantic::ShinRatio,
    ProportionControlSemantic::TorsoLength,
    ProportionControlSemantic::RibcageWidth,
    ProportionControlSemantic::WaistWidth,
    ProportionControlSemantic::TorsoDepth,
    ProportionControlSemantic::ShoulderWidth,
    ProportionControlSemantic::ShoulderSlope,
    ProportionControlSemantic::PelvisWidth,
    ProportionControlSemantic::PelvisTilt,
    ProportionControlSemantic::HandScale,
    ProportionControlSemantic::PalmLength,
    ProportionControlSemantic::FingerLength,
    ProportionControlSemantic::FootLength,
    ProportionControlSemantic::FootWidth,
    ProportionControlSemantic::ArchHeight,
    ProportionControlSemantic::RestPoseBlend,
];

fn required_control_contract(semantic: ProportionControlSemantic) -> RequiredControlContract {
    match semantic {
        ProportionControlSemantic::Stature => RequiredControlContract {
            id: "proportion.height.stature",
            group: ProportionControlGroup::Height,
            unit: ProportionUnit::Meters,
            range: range(1.20, 2.40, 1.75),
        },
        ProportionControlSemantic::HeadCount => RequiredControlContract {
            id: "proportion.height.head_count",
            group: ProportionControlGroup::Height,
            unit: ProportionUnit::HeadCount,
            range: range(6.0, 9.0, 7.5),
        },
        ProportionControlSemantic::ArmSpan => RequiredControlContract {
            id: "proportion.limb.arm_span",
            group: ProportionControlGroup::LimbProportion,
            unit: ProportionUnit::Multiplier,
            range: range(0.85, 1.15, 1.0),
        },
        ProportionControlSemantic::ForearmRatio => RequiredControlContract {
            id: "proportion.limb.forearm_ratio",
            group: ProportionControlGroup::LimbProportion,
            unit: ProportionUnit::Ratio,
            range: range(0.42, 0.58, 0.50),
        },
        ProportionControlSemantic::LegLength => RequiredControlContract {
            id: "proportion.limb.leg_length",
            group: ProportionControlGroup::LimbProportion,
            unit: ProportionUnit::Multiplier,
            range: range(0.85, 1.15, 1.0),
        },
        ProportionControlSemantic::ShinRatio => RequiredControlContract {
            id: "proportion.limb.shin_ratio",
            group: ProportionControlGroup::LimbProportion,
            unit: ProportionUnit::Ratio,
            range: range(0.42, 0.58, 0.50),
        },
        ProportionControlSemantic::TorsoLength => RequiredControlContract {
            id: "proportion.torso.length",
            group: ProportionControlGroup::TorsoShape,
            unit: ProportionUnit::Multiplier,
            range: range(0.85, 1.15, 1.0),
        },
        ProportionControlSemantic::RibcageWidth => RequiredControlContract {
            id: "proportion.torso.ribcage_width",
            group: ProportionControlGroup::TorsoShape,
            unit: ProportionUnit::Multiplier,
            range: range(0.80, 1.25, 1.0),
        },
        ProportionControlSemantic::WaistWidth => RequiredControlContract {
            id: "proportion.torso.waist_width",
            group: ProportionControlGroup::TorsoShape,
            unit: ProportionUnit::Multiplier,
            range: range(0.70, 1.20, 1.0),
        },
        ProportionControlSemantic::TorsoDepth => RequiredControlContract {
            id: "proportion.torso.depth",
            group: ProportionControlGroup::TorsoShape,
            unit: ProportionUnit::Multiplier,
            range: range(0.75, 1.25, 1.0),
        },
        ProportionControlSemantic::ShoulderWidth => RequiredControlContract {
            id: "proportion.shoulder.width",
            group: ProportionControlGroup::Shoulder,
            unit: ProportionUnit::Multiplier,
            range: range(0.80, 1.25, 1.0),
        },
        ProportionControlSemantic::ShoulderSlope => RequiredControlContract {
            id: "proportion.shoulder.slope",
            group: ProportionControlGroup::Shoulder,
            unit: ProportionUnit::Degrees,
            range: range(-20.0, 20.0, 0.0),
        },
        ProportionControlSemantic::PelvisWidth => RequiredControlContract {
            id: "proportion.pelvis.width",
            group: ProportionControlGroup::Pelvis,
            unit: ProportionUnit::Multiplier,
            range: range(0.80, 1.25, 1.0),
        },
        ProportionControlSemantic::PelvisTilt => RequiredControlContract {
            id: "proportion.pelvis.tilt",
            group: ProportionControlGroup::Pelvis,
            unit: ProportionUnit::Degrees,
            range: range(-18.0, 18.0, 0.0),
        },
        ProportionControlSemantic::HandScale => RequiredControlContract {
            id: "proportion.hand.scale",
            group: ProportionControlGroup::Hand,
            unit: ProportionUnit::Multiplier,
            range: range(0.75, 1.30, 1.0),
        },
        ProportionControlSemantic::PalmLength => RequiredControlContract {
            id: "proportion.hand.palm_length",
            group: ProportionControlGroup::Hand,
            unit: ProportionUnit::Ratio,
            range: range(0.40, 0.62, 0.52),
        },
        ProportionControlSemantic::FingerLength => RequiredControlContract {
            id: "proportion.hand.finger_length",
            group: ProportionControlGroup::Hand,
            unit: ProportionUnit::Ratio,
            range: range(0.38, 0.60, 0.48),
        },
        ProportionControlSemantic::FootLength => RequiredControlContract {
            id: "proportion.foot.length",
            group: ProportionControlGroup::Foot,
            unit: ProportionUnit::Multiplier,
            range: range(0.75, 1.30, 1.0),
        },
        ProportionControlSemantic::FootWidth => RequiredControlContract {
            id: "proportion.foot.width",
            group: ProportionControlGroup::Foot,
            unit: ProportionUnit::Multiplier,
            range: range(0.70, 1.30, 1.0),
        },
        ProportionControlSemantic::ArchHeight => RequiredControlContract {
            id: "proportion.foot.arch_height",
            group: ProportionControlGroup::Foot,
            unit: ProportionUnit::NormalizedWeight,
            range: range(0.0, 1.0, 0.5),
        },
        ProportionControlSemantic::RestPoseBlend => RequiredControlContract {
            id: "proportion.pose.rest_blend",
            group: ProportionControlGroup::PoseNormalization,
            unit: ProportionUnit::NormalizedWeight,
            range: range(0.0, 1.0, 1.0),
        },
    }
}

fn control_matches_required_contract(
    control: &ProportionControl,
    expected: RequiredControlContract,
) -> bool {
    control.id.0 == expected.id
        && control.group == expected.group
        && control.unit == expected.unit
        && range_approx_eq(control.range, expected.range)
}

fn range_approx_eq(left: ScalarRange, right: ScalarRange) -> bool {
    approx_eq(left.min, right.min)
        && approx_eq(left.max, right.max)
        && approx_eq(left.default, right.default)
}

fn control_unit_accepts_range(unit: ProportionUnit, range: ScalarRange) -> bool {
    if !range.is_valid() {
        return false;
    }
    match unit {
        ProportionUnit::Meters => range.min >= 0.0,
        ProportionUnit::HeadCount => range.min >= 1.0,
        ProportionUnit::Multiplier => positive_multiplier_range_is_valid(range),
        ProportionUnit::Ratio | ProportionUnit::NormalizedWeight => {
            range.min >= 0.0 && range.max <= 1.0
        }
        ProportionUnit::Degrees => true,
    }
}

fn positive_multiplier_range_is_valid(range: ScalarRange) -> bool {
    range.is_valid() && range.min > 0.0 && range.max <= 10.0
}

fn validate_hand_share_pair(
    controls: &[ProportionControl],
    report: &mut ProportionValidationReport,
) {
    let palm = controls
        .iter()
        .find(|control| control.semantic == ProportionControlSemantic::PalmLength);
    let finger = controls
        .iter()
        .find(|control| control.semantic == ProportionControlSemantic::FingerLength);
    let (Some(palm), Some(finger)) = (palm, finger) else {
        return;
    };

    let complementary = approx_eq(palm.range.default + finger.range.default, 1.0)
        && approx_eq(palm.range.min + finger.range.max, 1.0)
        && approx_eq(palm.range.max + finger.range.min, 1.0);
    if !complementary {
        report.push(
            ProportionValidationCode::InvalidControlUnit,
            &finger.id.0,
            "palm and finger hand-length shares must be complementary",
        );
    }
}

fn validate_joint_frames(frames: &[JointFrameContract], report: &mut ProportionValidationReport) {
    let mut ids = BTreeMap::<CharacterJointId, usize>::new();
    let mut root_count = 0usize;

    for (index, frame) in frames.iter().enumerate() {
        if !stable_identifier_is_valid(&frame.id.0) {
            report.push(
                ProportionValidationCode::InvalidJointId,
                &frame.id.0,
                "joint ID must be stable lowercase ASCII",
            );
        }
        if !frame.is_locally_valid() {
            report.push(
                ProportionValidationCode::InvalidJointFrame,
                &frame.id.0,
                "joint frame origin, rotation, axes, parent IDs, mirror IDs, and limits must be valid",
            );
        }
        if ids.insert(frame.id.clone(), index).is_some() {
            report.push(
                ProportionValidationCode::DuplicateJointId,
                &frame.id.0,
                "joint ID appears more than once",
            );
        }
        if frame.parent.is_none() {
            root_count += 1;
            if frame.role != JointFrameRole::Root
                || frame.side != JointSide::Center
                || frame.mirror.is_some()
            {
                report.push(
                    ProportionValidationCode::RootJointCount,
                    &frame.id.0,
                    "root joint must have root role, center side, and no mirror",
                );
            }
        } else if frame.role == JointFrameRole::Root {
            report.push(
                ProportionValidationCode::RootJointCount,
                &frame.id.0,
                "only the hierarchy root may use the root role",
            );
        }
    }

    if root_count != 1 {
        report.push(
            ProportionValidationCode::RootJointCount,
            "joint.root",
            "joint hierarchy must contain exactly one root",
        );
    }

    for (index, frame) in frames.iter().enumerate() {
        if let Some(parent) = &frame.parent {
            match ids.get(parent) {
                Some(parent_index) if *parent_index < index => {}
                Some(_) => report.push(
                    ProportionValidationCode::JointParentOrder,
                    &frame.id.0,
                    "joint parent must appear before child in the frame list",
                ),
                None => report.push(
                    ProportionValidationCode::MissingJointParent,
                    &frame.id.0,
                    format!("missing parent joint {}", parent.0),
                ),
            }
        }

        match frame.side {
            JointSide::Center if frame.mirror.is_some() => {
                report.push(
                    ProportionValidationCode::InvalidMirrorJoint,
                    &frame.id.0,
                    "center joints must not declare mirror joints",
                );
            }
            JointSide::Left | JointSide::Right if frame.mirror.is_none() => {
                report.push(
                    ProportionValidationCode::InvalidMirrorJoint,
                    &frame.id.0,
                    "lateral joints must declare a reciprocal mirror joint",
                );
            }
            _ => {}
        }

        let Some(mirror) = &frame.mirror else {
            continue;
        };
        let Some(mirror_index) = ids.get(mirror) else {
            report.push(
                ProportionValidationCode::InvalidMirrorJoint,
                &frame.id.0,
                format!("missing mirror joint {}", mirror.0),
            );
            continue;
        };
        let mirror_frame = &frames[*mirror_index];
        if mirror_frame.mirror.as_ref() != Some(&frame.id)
            || mirror_frame.role != frame.role
            || !matches!(
                (frame.side, mirror_frame.side),
                (JointSide::Left, JointSide::Right) | (JointSide::Right, JointSide::Left)
            )
        {
            report.push(
                ProportionValidationCode::InvalidMirrorJoint,
                &frame.id.0,
                "mirror joint link must be reciprocal, same-role, and opposite-side",
            );
        }
        if !mirrored_origins_are_valid(frame, mirror_frame) {
            report.push(
                ProportionValidationCode::InvalidMirrorOrigin,
                &frame.id.0,
                "lateral mirror origins must reflect across the character midline",
            );
        }
    }
}

fn validate_pose_contract(
    contract: &PoseNormalizationContract,
    frames: &[JointFrameContract],
    report: &mut ProportionValidationReport,
) {
    if !contract.is_valid() {
        report.push(
            ProportionValidationCode::InvalidPoseNormalization,
            &contract.id.0,
            "pose normalization contract must have stable sorted IDs and finite scale range",
        );
    }

    let joint_ids = frames
        .iter()
        .map(|frame| frame.id.clone())
        .collect::<BTreeSet<_>>();

    if !joint_ids.contains(&contract.root_joint) {
        report.push(
            ProportionValidationCode::MissingPoseJoint,
            &contract.root_joint.0,
            "pose normalization root joint is missing from joint frames",
        );
    }
    if !contract.required_joints.contains(&contract.root_joint) {
        report.push(
            ProportionValidationCode::InvalidPoseNormalization,
            &contract.root_joint.0,
            "pose normalization root joint must be listed as required",
        );
    }

    if let Some(root_frame) = frames.iter().find(|frame| frame.id == contract.root_joint)
        && (root_frame.parent.is_some()
            || root_frame.role != JointFrameRole::Root
            || root_frame.side != JointSide::Center
            || root_frame.mirror.is_some())
    {
        report.push(
            ProportionValidationCode::InvalidPoseNormalization,
            &contract.root_joint.0,
            "pose normalization root must reference the hierarchy root frame",
        );
    }

    for joint in &contract.required_joints {
        if !joint_ids.contains(joint) {
            report.push(
                ProportionValidationCode::MissingPoseJoint,
                &joint.0,
                "pose normalization required joint is missing from joint frames",
            );
        }
    }

    let required_joint_ids = contract
        .required_joints
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for joint in joint_ids.difference(&required_joint_ids) {
        report.push(
            ProportionValidationCode::MissingPoseJoint,
            &joint.0,
            "pose normalization required joints must cover every joint frame",
        );
    }
}

fn normalize_joint_pose_sample(
    sample: &JointPoseSample,
    contract: &PoseNormalizationContract,
) -> Result<NormalizedJointPose, PoseNormalizationError> {
    if !stable_identifier_is_valid(&sample.joint.0) {
        return Err(PoseNormalizationError::InvalidJointId(
            sample.joint.0.clone(),
        ));
    }
    if !contract.required_joints.contains(&sample.joint) {
        return Err(PoseNormalizationError::UnknownJoint(sample.joint.0.clone()));
    }
    if !vec3_is_finite(sample.translation) {
        return Err(PoseNormalizationError::NonFiniteTranslation(
            sample.joint.0.clone(),
        ));
    }
    if !sample.scale.is_finite() {
        return Err(PoseNormalizationError::NonFiniteScale(
            sample.joint.0.clone(),
        ));
    }

    Ok(NormalizedJointPose {
        joint: sample.joint.clone(),
        rotation: canonicalize_quaternion(sample.rotation, &sample.joint)?,
        translation: [
            canonical_f32(sample.translation[0]),
            canonical_f32(sample.translation[1]),
            canonical_f32(sample.translation[2]),
        ],
        scale: canonical_f32(
            sample
                .scale
                .clamp(contract.scale_range.min, contract.scale_range.max),
        ),
    })
}

fn canonicalize_quaternion(
    rotation: UnitQuaternion,
    joint: &CharacterJointId,
) -> Result<UnitQuaternion, PoseNormalizationError> {
    if !rotation.value.iter().all(|component| component.is_finite()) {
        return Err(PoseNormalizationError::NonFiniteRotation(joint.0.clone()));
    }

    let norm = rotation
        .value
        .iter()
        .map(|component| f64::from(*component) * f64::from(*component))
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::from(EPSILON) {
        return Err(PoseNormalizationError::ZeroLengthRotation(joint.0.clone()));
    }

    let mut value = [
        canonical_f32((f64::from(rotation.value[0]) / norm) as f32),
        canonical_f32((f64::from(rotation.value[1]) / norm) as f32),
        canonical_f32((f64::from(rotation.value[2]) / norm) as f32),
        canonical_f32((f64::from(rotation.value[3]) / norm) as f32),
    ];

    if quaternion_should_flip(value) {
        value = [
            canonical_f32(-value[0]),
            canonical_f32(-value[1]),
            canonical_f32(-value[2]),
            canonical_f32(-value[3]),
        ];
    }

    let canonical = UnitQuaternion { value };
    if canonical.is_canonical() {
        Ok(canonical)
    } else {
        Err(PoseNormalizationError::ZeroLengthRotation(joint.0.clone()))
    }
}

fn quaternion_should_flip(value: [f32; 4]) -> bool {
    if value[3].abs() > EPSILON {
        return value[3] < 0.0;
    }

    value[..3]
        .iter()
        .find(|component| component.abs() > EPSILON)
        .is_some_and(|component| *component < 0.0)
}

fn stable_pose_id(joints: &[NormalizedJointPose]) -> CharacterPoseId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(POSE_HASH_DOMAIN);
    for pose in joints {
        hasher.update(pose.joint.0.as_bytes());
        hasher.update(&[0]);
        for component in pose.rotation.value {
            hasher.update(&canonical_f32_bits(component).to_le_bytes());
        }
        for component in pose.translation {
            hasher.update(&canonical_f32_bits(component).to_le_bytes());
        }
        hasher.update(&canonical_f32_bits(pose.scale).to_le_bytes());
        hasher.update(&[0xff]);
    }
    CharacterPoseId(format!(
        "pose.normalized.blake3.{}",
        hasher.finalize().to_hex()
    ))
}

fn pose_sort_key(pose: &NormalizedJointPose) -> ([u32; 4], [u32; 3], u32) {
    (
        [
            canonical_f32_bits(pose.rotation.value[0]),
            canonical_f32_bits(pose.rotation.value[1]),
            canonical_f32_bits(pose.rotation.value[2]),
            canonical_f32_bits(pose.rotation.value[3]),
        ],
        [
            canonical_f32_bits(pose.translation[0]),
            canonical_f32_bits(pose.translation[1]),
            canonical_f32_bits(pose.translation[2]),
        ],
        canonical_f32_bits(pose.scale),
    )
}

fn control(
    id: &str,
    group: ProportionControlGroup,
    semantic: ProportionControlSemantic,
    range: ScalarRange,
    unit: ProportionUnit,
) -> ProportionControl {
    ProportionControl {
        id: CharacterControlId(id.to_owned()),
        group,
        semantic,
        range,
        unit,
    }
}

fn joint(
    id: &str,
    parent: Option<&str>,
    mirror: Option<&str>,
    role: JointFrameRole,
    side: JointSide,
    rest_origin: [f32; 3],
) -> JointFrameContract {
    JointFrameContract {
        id: joint_id(id),
        parent: parent.map(joint_id),
        mirror: mirror.map(joint_id),
        role,
        side,
        rest_origin,
        rest_rotation: UnitQuaternion::IDENTITY,
        axes: JointFrameAxes::IDENTITY,
        limits: limits_for_role(role),
    }
}

fn joint_id(id: &str) -> CharacterJointId {
    CharacterJointId(id.to_owned())
}

fn limits_for_role(role: JointFrameRole) -> JointLimitContract {
    match role {
        JointFrameRole::Root => joint_limits(0.0, 0.0, 0.0, 0.0),
        JointFrameRole::Pelvis => joint_limits(-25.0, 25.0, -20.0, 20.0),
        JointFrameRole::Spine => joint_limits(-35.0, 35.0, -30.0, 30.0),
        JointFrameRole::Neck => joint_limits(-45.0, 45.0, -55.0, 55.0),
        JointFrameRole::Head => joint_limits(-35.0, 35.0, -45.0, 45.0),
        JointFrameRole::Clavicle => joint_limits(-35.0, 35.0, -25.0, 25.0),
        JointFrameRole::Shoulder => joint_limits(-120.0, 120.0, -90.0, 90.0),
        JointFrameRole::Elbow => joint_limits(0.0, 150.0, -70.0, 70.0),
        JointFrameRole::Wrist => joint_limits(-80.0, 80.0, -90.0, 90.0),
        JointFrameRole::Hand => joint_limits(-20.0, 20.0, -30.0, 30.0),
        JointFrameRole::Hip => joint_limits(-100.0, 100.0, -70.0, 70.0),
        JointFrameRole::Knee => joint_limits(0.0, 160.0, -20.0, 20.0),
        JointFrameRole::Ankle => joint_limits(-55.0, 55.0, -35.0, 35.0),
        JointFrameRole::Foot => joint_limits(-20.0, 20.0, -25.0, 25.0),
        JointFrameRole::Toe => joint_limits(-20.0, 45.0, -10.0, 10.0),
    }
}

fn joint_limits(
    bend_min: f32,
    bend_max: f32,
    twist_min: f32,
    twist_max: f32,
) -> JointLimitContract {
    JointLimitContract {
        bend_degrees: range(bend_min, bend_max, 0.0),
        twist_degrees: range(twist_min, twist_max, 0.0),
        stretch: range(1.0, 1.0, 1.0),
    }
}

fn range(min: f32, max: f32, default: f32) -> ScalarRange {
    ScalarRange { min, max, default }
}

fn stable_identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= EPSILON
}

fn ids_are_strictly_sorted<'a>(ids: impl Iterator<Item = &'a str>) -> bool {
    let mut previous = None;
    for id in ids {
        if previous.is_some_and(|previous_id| previous_id >= id) {
            return false;
        }
        previous = Some(id);
    }
    true
}

fn mirrored_origins_are_valid(left: &JointFrameContract, right: &JointFrameContract) -> bool {
    matches!(
        (left.side, right.side),
        (JointSide::Left, JointSide::Right) | (JointSide::Right, JointSide::Left)
    ) && (left.rest_origin[0] + right.rest_origin[0]).abs() <= EPSILON
        && (left.rest_origin[1] - right.rest_origin[1]).abs() <= EPSILON
        && (left.rest_origin[2] - right.rest_origin[2]).abs() <= EPSILON
}

fn vec3_is_finite(value: [f32; 3]) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn vec3_is_unit(value: [f32; 3]) -> bool {
    vec3_is_finite(value) && (dot(value, value) - 1.0).abs() <= EPSILON
}

fn vec3_approx_eq(left: [f32; 3], right: [f32; 3]) -> bool {
    (left[0] - right[0]).abs() <= EPSILON
        && (left[1] - right[1]).abs() <= EPSILON
        && (left[2] - right[2]).abs() <= EPSILON
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn canonical_f32(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}

fn canonical_f32_bits(value: f32) -> u32 {
    canonical_f32(value).to_bits()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn required_semantic_controls_exist() {
        let grammar = proportion_grammar();
        let ids = grammar
            .controls
            .iter()
            .map(|control| control.id.0.as_str())
            .collect::<BTreeSet<_>>();
        let groups = grammar
            .controls
            .iter()
            .map(|control| control.group)
            .collect::<BTreeSet<_>>();

        for id in [
            "proportion.height.stature",
            "proportion.height.head_count",
            "proportion.limb.arm_span",
            "proportion.limb.forearm_ratio",
            "proportion.limb.leg_length",
            "proportion.limb.shin_ratio",
            "proportion.torso.length",
            "proportion.torso.ribcage_width",
            "proportion.torso.waist_width",
            "proportion.torso.depth",
            "proportion.shoulder.width",
            "proportion.shoulder.slope",
            "proportion.pelvis.width",
            "proportion.pelvis.tilt",
            "proportion.hand.scale",
            "proportion.hand.palm_length",
            "proportion.hand.finger_length",
            "proportion.foot.length",
            "proportion.foot.width",
            "proportion.foot.arch_height",
            "proportion.pose.rest_blend",
        ] {
            assert!(ids.contains(id), "missing control {id}");
        }

        for group in [
            ProportionControlGroup::Height,
            ProportionControlGroup::LimbProportion,
            ProportionControlGroup::TorsoShape,
            ProportionControlGroup::Shoulder,
            ProportionControlGroup::Pelvis,
            ProportionControlGroup::Hand,
            ProportionControlGroup::Foot,
            ProportionControlGroup::PoseNormalization,
        ] {
            assert!(groups.contains(&group), "missing control group {group:?}");
        }

        assert!(ids_are_strictly_sorted(
            grammar.controls.iter().map(|control| control.id.0.as_str())
        ));
    }

    #[test]
    fn control_ranges_are_finite_and_defaults_are_contained() {
        let grammar = proportion_grammar();

        for control in &grammar.controls {
            assert!(control.is_valid(), "invalid control {}", control.id.0);
            assert!(control.range.min.is_finite());
            assert!(control.range.max.is_finite());
            assert!(control.range.default.is_finite());
            assert!(control.range.min <= control.range.default);
            assert!(control.range.default <= control.range.max);
        }

        assert!(
            grammar.validation_report().is_valid(),
            "{:?}",
            grammar.validation_report().issues
        );
    }

    #[test]
    fn validation_rejects_unit_and_hand_share_contract_mismatches() {
        let mut grammar = proportion_grammar();
        let head_count = grammar
            .controls
            .iter_mut()
            .find(|control| control.semantic == ProportionControlSemantic::HeadCount)
            .expect("head-count control exists");
        head_count.unit = ProportionUnit::Ratio;

        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidControlUnit
                && issue.subject == "proportion.height.head_count"
        }));

        let mut grammar = proportion_grammar();
        let finger = grammar
            .controls
            .iter_mut()
            .find(|control| control.semantic == ProportionControlSemantic::FingerLength)
            .expect("finger-length control exists");
        finger.range.max = 0.70;

        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidControlUnit
                && issue.subject == "proportion.hand.finger_length"
        }));

        let mut grammar = proportion_grammar();
        let duplicate = grammar
            .controls
            .iter()
            .find(|control| control.semantic == ProportionControlSemantic::HeadCount)
            .expect("head-count control exists")
            .clone();
        grammar.controls.push(duplicate);
        grammar
            .controls
            .sort_by(|left, right| left.id.cmp(&right.id));
        let report = grammar.validation_report();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == ProportionValidationCode::DuplicateControlSemantic)
        );

        let mut grammar = proportion_grammar();
        grammar
            .controls
            .retain(|control| control.semantic != ProportionControlSemantic::HeadCount);
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::MissingControlSemantic
                && issue.subject == "HeadCount"
        }));

        let mut grammar = proportion_grammar();
        let stature = grammar
            .controls
            .iter_mut()
            .find(|control| control.semantic == ProportionControlSemantic::Stature)
            .expect("stature control exists");
        stature.range = range(0.10, 100.0, 1.75);
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidControlSemantic
                && issue.subject == "proportion.height.stature"
        }));
    }

    #[test]
    fn joint_frames_are_normalized_and_parented_correctly() {
        let grammar = proportion_grammar();
        let report = grammar.validation_report();
        assert!(report.is_valid(), "{:?}", report.issues);

        let frame_indices = grammar
            .joint_frames
            .iter()
            .enumerate()
            .map(|(index, frame)| (frame.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let roots = grammar
            .joint_frames
            .iter()
            .filter(|frame| frame.parent.is_none())
            .collect::<Vec<_>>();

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id.0, "joint.root");

        for (index, frame) in grammar.joint_frames.iter().enumerate() {
            assert!(frame.rest_rotation.is_canonical(), "{}", frame.id.0);
            assert!(frame.axes.is_orthonormal(), "{}", frame.id.0);
            assert!(vec3_is_finite(frame.rest_origin), "{}", frame.id.0);
            assert!(frame.limits.is_valid(), "{}", frame.id.0);

            if let Some(parent) = &frame.parent {
                let parent_index = frame_indices[parent];
                assert!(
                    parent_index < index,
                    "parent {} must come before child {}",
                    parent.0,
                    frame.id.0
                );
            }

            if let Some(mirror) = &frame.mirror {
                let mirror_frame = &grammar.joint_frames[frame_indices[mirror]];
                assert_eq!(mirror_frame.mirror.as_ref(), Some(&frame.id));
                assert!(mirrored_origins_are_valid(frame, mirror_frame));
            }
        }
    }

    #[test]
    fn pose_normalization_is_deterministic() {
        let mut samples = identity_pose_samples();
        set_pose_sample(
            &mut samples,
            "joint.root",
            [0.0, 0.0, 0.0, -1.0],
            [10.0, 2.0, -3.0],
            1.0,
        );
        set_pose_sample(
            &mut samples,
            "joint.head",
            [0.0, 1.0, 0.0, 1.0],
            [10.0, 3.0, -3.0],
            3.0,
        );
        set_pose_sample(
            &mut samples,
            "joint.wrist.left",
            [0.0, 0.0, 0.0, -2.0],
            [-0.4, 1.2, 0.0],
            0.25,
        );
        samples.push(pose_sample(
            "joint.wrist.left",
            [0.0, 0.0, 1.0, 1.0],
            [-0.2, 1.0, 0.0],
            1.0,
        ));
        let mut reversed = samples.clone();
        reversed.reverse();

        let first = normalize_pose_samples(&samples).expect("pose should normalize");
        let second = normalize_pose_samples(&reversed).expect("pose should normalize");

        assert_eq!(first, second);
        assert!(first.is_valid());
        assert!(first.id.0.starts_with("pose.normalized.blake3."));
        assert_eq!(
            first.joints.len(),
            pose_normalization_contract().required_joints.len()
        );

        let root = first
            .joints
            .iter()
            .find(|pose| pose.joint.0 == "joint.root")
            .expect("root pose exists");
        assert_eq!(root.translation, [0.0, 0.0, 0.0]);
        assert_eq!(root.rotation, UnitQuaternion::IDENTITY);

        let head = first
            .joints
            .iter()
            .find(|pose| pose.joint.0 == "joint.head")
            .expect("head pose exists");
        assert_eq!(head.scale, 1.5);
    }

    #[test]
    fn pose_normalization_rejects_unknown_joints_and_noncanonical_external_quaternions() {
        let mut samples = identity_pose_samples();
        samples.push(pose_sample(
            "joint.extra",
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
            1.0,
        ));
        let unknown = normalize_pose_samples(&samples);
        assert!(matches!(
            unknown,
            Err(PoseNormalizationError::UnknownJoint(joint)) if joint == "joint.extra"
        ));

        let mut missing = identity_pose_samples();
        missing.retain(|sample| sample.joint.0 != "joint.head");
        assert!(matches!(
            normalize_pose_samples(&missing),
            Err(PoseNormalizationError::MissingRequiredJoint(joint)) if joint == "joint.head"
        ));

        let mut overflowing = identity_pose_samples();
        set_pose_sample(
            &mut overflowing,
            "joint.root",
            [0.0, 0.0, 0.0, 1.0],
            [-f32::MAX, 0.0, 0.0],
            1.0,
        );
        set_pose_sample(
            &mut overflowing,
            "joint.head",
            [f32::MAX, f32::MAX, f32::MAX, f32::MAX],
            [f32::MAX, 0.0, 0.0],
            1.0,
        );
        assert!(matches!(
            normalize_pose_samples(&overflowing),
            Err(PoseNormalizationError::NonFiniteTranslation(joint)) if joint == "joint.head"
        ));

        let pose = NormalizedJointPose {
            joint: CharacterJointId("joint.root".to_owned()),
            rotation: UnitQuaternion {
                value: [0.0, 0.0, 0.0, -1.0],
            },
            translation: [0.0, 0.0, 0.0],
            scale: 1.0,
        };
        assert!(!pose.is_valid());

        let invalid_scale = NormalizedJointPose {
            rotation: UnitQuaternion::IDENTITY,
            scale: -1.0,
            ..pose
        };
        assert!(!invalid_scale.is_valid());
    }

    #[test]
    fn validation_rejects_joint_and_pose_contract_invariants() {
        let mut grammar = proportion_grammar();
        let shoulder_left = grammar
            .joint_frames
            .iter_mut()
            .find(|frame| frame.id.0 == "joint.shoulder.left")
            .expect("left shoulder exists");
        shoulder_left.mirror = None;
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidMirrorJoint
                && issue.subject == "joint.shoulder.left"
        }));

        let mut grammar = proportion_grammar();
        let root = grammar
            .joint_frames
            .iter_mut()
            .find(|frame| frame.id.0 == "joint.root")
            .expect("root exists");
        root.role = JointFrameRole::Pelvis;
        let report = grammar.validation_report();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == ProportionValidationCode::RootJointCount)
        );

        let mut grammar = proportion_grammar();
        let pelvis = grammar
            .joint_frames
            .iter_mut()
            .find(|frame| frame.id.0 == "joint.pelvis")
            .expect("pelvis exists");
        pelvis.mirror = Some(joint_id("joint.root"));
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidMirrorJoint
                && issue.subject == "joint.pelvis"
        }));

        let mut grammar = proportion_grammar();
        grammar.pose_normalization.root_joint = joint_id("joint.pelvis");
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::InvalidPoseNormalization
                && issue.subject == "joint.pelvis"
        }));

        let mut grammar = proportion_grammar();
        let pelvis = grammar
            .joint_frames
            .iter_mut()
            .find(|frame| frame.id.0 == "joint.pelvis")
            .expect("pelvis exists");
        pelvis.role = JointFrameRole::Root;
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::RootJointCount
                && issue.subject == "joint.pelvis"
        }));

        let mut grammar = proportion_grammar();
        grammar
            .pose_normalization
            .required_joints
            .retain(|joint| joint.0 != "joint.root");
        let report = grammar.validation_report();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == ProportionValidationCode::InvalidPoseNormalization)
        );

        let mut grammar = proportion_grammar();
        grammar
            .pose_normalization
            .required_joints
            .retain(|joint| joint.0 != "joint.head");
        let report = grammar.validation_report();
        assert!(report.issues.iter().any(|issue| {
            issue.code == ProportionValidationCode::MissingPoseJoint
                && issue.subject == "joint.head"
        }));

        let mut grammar = proportion_grammar();
        grammar.joint_frames[0].limits.stretch = range(-1.0, 1.0, 1.0);
        let report = grammar.validation_report();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == ProportionValidationCode::InvalidJointFrame)
        );
    }

    fn pose_sample(
        joint: &str,
        rotation: [f32; 4],
        translation: [f32; 3],
        scale: f32,
    ) -> JointPoseSample {
        JointPoseSample {
            joint: CharacterJointId(joint.to_owned()),
            rotation: UnitQuaternion { value: rotation },
            translation,
            scale,
        }
    }

    fn identity_pose_samples() -> Vec<JointPoseSample> {
        pose_normalization_contract()
            .required_joints
            .into_iter()
            .map(|joint| pose_sample(&joint.0, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0], 1.0))
            .collect()
    }

    fn set_pose_sample(
        samples: &mut [JointPoseSample],
        joint: &str,
        rotation: [f32; 4],
        translation: [f32; 3],
        scale: f32,
    ) {
        let sample = samples
            .iter_mut()
            .find(|sample| sample.joint.0 == joint)
            .unwrap_or_else(|| panic!("missing sample {joint}"));
        *sample = pose_sample(joint, rotation, translation, scale);
    }
}
