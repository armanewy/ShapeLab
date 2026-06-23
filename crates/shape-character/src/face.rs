//! Face and head deformation grammar.

use crate::{
    CharacterControlId, CharacterLoopId, CharacterRegionId, CharacterSymmetryId, ScalarRange,
    base::{HUMANOID_EYES_BASE_ID, HUMANOID_HEAD_BASE_ID, base_topology_library},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_COMPACT_TARGETS: usize = 4;
const MAX_COMPACT_CONTROLS: usize = 5;
const MAX_COMPACT_CAGE_HANDLES: u8 = 12;
const MAX_COMPACT_SUPPORT_REGIONS: u8 = 4;

/// Required high-level coverage categories for face/head deformation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FaceRegionClass {
    Skull,
    Jaw,
    Cheek,
    Brow,
    Nose,
    MouthLoop,
    Eye,
    Ear,
}

impl FaceRegionClass {
    /// Stable semantic class id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skull => "face.class.skull",
            Self::Jaw => "face.class.jaw",
            Self::Cheek => "face.class.cheek",
            Self::Brow => "face.class.brow",
            Self::Nose => "face.class.nose",
            Self::MouthLoop => "face.class.mouth_loop",
            Self::Eye => "face.class.eye",
            Self::Ear => "face.class.ear",
        }
    }
}

/// Semantic side used by symmetry validation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FaceSide {
    Midline,
    Left,
    Right,
}

/// Stable semantic face/head region targets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FaceRegionTarget {
    SkullCranium,
    JawMandible,
    CheekLeft,
    CheekRight,
    BrowLeft,
    BrowRight,
    NoseBridge,
    NoseTip,
    NoseAlaLeft,
    NoseAlaRight,
    MouthOuterLoop,
    MouthInnerLoop,
    EyeLeftUpperLid,
    EyeLeftLowerLid,
    EyeRightUpperLid,
    EyeRightLowerLid,
    EarLeft,
    EarRight,
}

impl FaceRegionTarget {
    /// Stable semantic region id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SkullCranium => "face.skull.cranium",
            Self::JawMandible => "face.jaw.mandible",
            Self::CheekLeft => "face.cheek.left",
            Self::CheekRight => "face.cheek.right",
            Self::BrowLeft => "face.brow.left",
            Self::BrowRight => "face.brow.right",
            Self::NoseBridge => "face.nose.bridge",
            Self::NoseTip => "face.nose.tip",
            Self::NoseAlaLeft => "face.nose.ala.left",
            Self::NoseAlaRight => "face.nose.ala.right",
            Self::MouthOuterLoop => "face.mouth.outer_loop",
            Self::MouthInnerLoop => "face.mouth.inner_loop",
            Self::EyeLeftUpperLid => "face.eye.left_upper_lid",
            Self::EyeLeftLowerLid => "face.eye.left_lower_lid",
            Self::EyeRightUpperLid => "face.eye.right_upper_lid",
            Self::EyeRightLowerLid => "face.eye.right_lower_lid",
            Self::EarLeft => "face.ear.left",
            Self::EarRight => "face.ear.right",
        }
    }

    /// Shared stable region id wrapper.
    #[must_use]
    pub fn region_id(self) -> CharacterRegionId {
        CharacterRegionId(self.base_region_id().to_owned())
    }

    /// Versioned base that owns this target.
    #[must_use]
    pub const fn base_id(self) -> &'static str {
        match self {
            Self::EyeLeftUpperLid
            | Self::EyeLeftLowerLid
            | Self::EyeRightUpperLid
            | Self::EyeRightLowerLid => HUMANOID_EYES_BASE_ID,
            Self::SkullCranium
            | Self::JawMandible
            | Self::CheekLeft
            | Self::CheekRight
            | Self::BrowLeft
            | Self::BrowRight
            | Self::NoseBridge
            | Self::NoseTip
            | Self::NoseAlaLeft
            | Self::NoseAlaRight
            | Self::MouthOuterLoop
            | Self::MouthInnerLoop
            | Self::EarLeft
            | Self::EarRight => HUMANOID_HEAD_BASE_ID,
        }
    }

    /// Resolvable region ID in the versioned base topology library.
    #[must_use]
    pub const fn base_region_id(self) -> &'static str {
        match self {
            Self::SkullCranium => "head.cranium",
            Self::JawMandible => "head.jaw",
            Self::CheekLeft => "head.left_cheek",
            Self::CheekRight => "head.right_cheek",
            Self::BrowLeft => "head.left_brow",
            Self::BrowRight => "head.right_brow",
            Self::NoseBridge => "head.nose_bridge_region",
            Self::NoseTip => "head.nose_tip_region",
            Self::NoseAlaLeft => "head.left_nose_ala",
            Self::NoseAlaRight => "head.right_nose_ala",
            Self::MouthOuterLoop => "head.mouth_outer",
            Self::MouthInnerLoop => "head.mouth_inner",
            Self::EyeLeftUpperLid => "eyes.left_upper_lid",
            Self::EyeLeftLowerLid => "eyes.left_lower_lid",
            Self::EyeRightUpperLid => "eyes.right_upper_lid",
            Self::EyeRightLowerLid => "eyes.right_lower_lid",
            Self::EarLeft => "head.left_ear",
            Self::EarRight => "head.right_ear",
        }
    }

    /// High-level target category.
    #[must_use]
    pub const fn class(self) -> FaceRegionClass {
        match self {
            Self::SkullCranium => FaceRegionClass::Skull,
            Self::JawMandible => FaceRegionClass::Jaw,
            Self::CheekLeft | Self::CheekRight => FaceRegionClass::Cheek,
            Self::BrowLeft | Self::BrowRight => FaceRegionClass::Brow,
            Self::NoseBridge | Self::NoseTip | Self::NoseAlaLeft | Self::NoseAlaRight => {
                FaceRegionClass::Nose
            }
            Self::MouthOuterLoop | Self::MouthInnerLoop => FaceRegionClass::MouthLoop,
            Self::EyeLeftUpperLid
            | Self::EyeLeftLowerLid
            | Self::EyeRightUpperLid
            | Self::EyeRightLowerLid => FaceRegionClass::Eye,
            Self::EarLeft | Self::EarRight => FaceRegionClass::Ear,
        }
    }

    /// Semantic side for mirror/asymmetry policy checks.
    #[must_use]
    pub const fn side(self) -> FaceSide {
        match self {
            Self::CheekLeft
            | Self::BrowLeft
            | Self::NoseAlaLeft
            | Self::EyeLeftUpperLid
            | Self::EyeLeftLowerLid
            | Self::EarLeft => FaceSide::Left,
            Self::CheekRight
            | Self::BrowRight
            | Self::NoseAlaRight
            | Self::EyeRightUpperLid
            | Self::EyeRightLowerLid
            | Self::EarRight => FaceSide::Right,
            Self::SkullCranium
            | Self::JawMandible
            | Self::NoseBridge
            | Self::NoseTip
            | Self::MouthOuterLoop
            | Self::MouthInnerLoop => FaceSide::Midline,
        }
    }

    /// Bilateral counterpart used by symmetric and independent bilateral operators.
    #[must_use]
    pub const fn counterpart(self) -> Option<Self> {
        match self {
            Self::CheekLeft => Some(Self::CheekRight),
            Self::CheekRight => Some(Self::CheekLeft),
            Self::BrowLeft => Some(Self::BrowRight),
            Self::BrowRight => Some(Self::BrowLeft),
            Self::NoseAlaLeft => Some(Self::NoseAlaRight),
            Self::NoseAlaRight => Some(Self::NoseAlaLeft),
            Self::EyeLeftUpperLid => Some(Self::EyeRightUpperLid),
            Self::EyeLeftLowerLid => Some(Self::EyeRightLowerLid),
            Self::EyeRightUpperLid => Some(Self::EyeLeftUpperLid),
            Self::EyeRightLowerLid => Some(Self::EyeLeftLowerLid),
            Self::EarLeft => Some(Self::EarRight),
            Self::EarRight => Some(Self::EarLeft),
            Self::SkullCranium
            | Self::JawMandible
            | Self::NoseBridge
            | Self::NoseTip
            | Self::MouthOuterLoop
            | Self::MouthInnerLoop => None,
        }
    }

    /// Required loop preservation when this target is itself a topological loop.
    #[must_use]
    pub const fn required_loop(self) -> Option<FaceLoopTarget> {
        match self {
            Self::MouthOuterLoop => Some(FaceLoopTarget::MouthOuter),
            Self::MouthInnerLoop => Some(FaceLoopTarget::MouthInner),
            Self::EyeLeftUpperLid | Self::EyeLeftLowerLid => Some(FaceLoopTarget::EyeLeftLid),
            Self::EyeRightUpperLid | Self::EyeRightLowerLid => Some(FaceLoopTarget::EyeRightLid),
            Self::SkullCranium
            | Self::JawMandible
            | Self::CheekLeft
            | Self::CheekRight
            | Self::BrowLeft
            | Self::BrowRight
            | Self::NoseBridge
            | Self::NoseTip
            | Self::NoseAlaLeft
            | Self::NoseAlaRight
            | Self::EarLeft
            | Self::EarRight => None,
        }
    }
}

/// Stable topological loops preserved by face operators.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FaceLoopTarget {
    MouthOuter,
    MouthInner,
    EyeLeftLid,
    EyeRightLid,
}

const MOUTH_OUTER_LOOP_TARGETS: &[FaceRegionTarget] = &[FaceRegionTarget::MouthOuterLoop];
const MOUTH_INNER_LOOP_TARGETS: &[FaceRegionTarget] = &[FaceRegionTarget::MouthInnerLoop];
const EYE_LEFT_LID_LOOP_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::EyeLeftUpperLid,
    FaceRegionTarget::EyeLeftLowerLid,
];
const EYE_RIGHT_LID_LOOP_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::EyeRightUpperLid,
    FaceRegionTarget::EyeRightLowerLid,
];

impl FaceLoopTarget {
    /// Stable topological loop id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MouthOuter => "face.loop.mouth.outer",
            Self::MouthInner => "face.loop.mouth.inner",
            Self::EyeLeftLid => "face.loop.eye.left_lid",
            Self::EyeRightLid => "face.loop.eye.right_lid",
        }
    }

    /// Shared stable loop id wrapper.
    #[must_use]
    pub fn loop_id(self) -> CharacterLoopId {
        CharacterLoopId(self.base_loop_id().to_owned())
    }

    /// Versioned base that owns this loop.
    #[must_use]
    pub const fn base_id(self) -> &'static str {
        match self {
            Self::MouthOuter | Self::MouthInner => HUMANOID_HEAD_BASE_ID,
            Self::EyeLeftLid | Self::EyeRightLid => HUMANOID_EYES_BASE_ID,
        }
    }

    /// Resolvable loop ID in the versioned base topology library.
    #[must_use]
    pub const fn base_loop_id(self) -> &'static str {
        match self {
            Self::MouthOuter => "head.loop.mouth_outer",
            Self::MouthInner => "head.loop.mouth_inner",
            Self::EyeLeftLid => "eyes.loop.left_orbital_rim",
            Self::EyeRightLid => "eyes.loop.right_orbital_rim",
        }
    }

    /// Required preservation policy for this loop.
    #[must_use]
    pub const fn required_policy(self) -> FaceLoopPreservationPolicy {
        match self {
            Self::MouthOuter | Self::MouthInner => {
                FaceLoopPreservationPolicy::LipSealAndCornerOrder
            }
            Self::EyeLeftLid | Self::EyeRightLid => {
                FaceLoopPreservationPolicy::LidClosureAndCanthusOrder
            }
        }
    }

    /// Region target that owns this topological loop.
    #[must_use]
    pub const fn region_target(self) -> FaceRegionTarget {
        match self {
            Self::MouthOuter => FaceRegionTarget::MouthOuterLoop,
            Self::MouthInner => FaceRegionTarget::MouthInnerLoop,
            Self::EyeLeftLid => FaceRegionTarget::EyeLeftUpperLid,
            Self::EyeRightLid => FaceRegionTarget::EyeRightUpperLid,
        }
    }

    /// All region targets that participate in this topological loop.
    #[must_use]
    pub const fn owning_targets(self) -> &'static [FaceRegionTarget] {
        match self {
            Self::MouthOuter => MOUTH_OUTER_LOOP_TARGETS,
            Self::MouthInner => MOUTH_INNER_LOOP_TARGETS,
            Self::EyeLeftLid => EYE_LEFT_LID_LOOP_TARGETS,
            Self::EyeRightLid => EYE_RIGHT_LID_LOOP_TARGETS,
        }
    }
}

/// Loop invariants an operator promises to keep intact.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceLoopPreservationPolicy {
    BoundaryOrderAndClosure,
    LipSealAndCornerOrder,
    LidClosureAndCanthusOrder,
}

/// Loop preservation declaration for a compact operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaceLoopPreservation {
    pub target: FaceLoopTarget,
    pub policy: FaceLoopPreservationPolicy,
}

/// Symmetry/asymmetry policy for an operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceSymmetryPolicy {
    MidlineLocked,
    MirroredAcrossMidline,
    IndependentBilateral,
    AsymmetricAllowed,
}

impl FaceSymmetryPolicy {
    /// Shared symmetry id wrapper, when a plane is enforced.
    #[must_use]
    pub fn symmetry_id(self) -> Option<CharacterSymmetryId> {
        match self {
            Self::MidlineLocked | Self::MirroredAcrossMidline => {
                Some(CharacterSymmetryId("head.symmetry.sagittal".to_owned()))
            }
            Self::IndependentBilateral | Self::AsymmetricAllowed => None,
        }
    }
}

/// Compact cage family used to parameterize each operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceCageKind {
    SkullEllipsoid,
    MandibleHinge,
    CheekPatchPair,
    BrowArcPair,
    NoseWedge,
    MouthLoopRing,
    OrbitalLoopPair,
    EarShellPair,
}

/// Compact region support kernel.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceRegionKernel {
    SmoothCompact,
    HingeLimited,
    SurfaceSlide,
    LoopConstrained,
}

/// Compact cage/region parameterization with normalized finite ranges.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceCageParameterization {
    pub cage: FaceCageKind,
    pub kernel: FaceRegionKernel,
    pub handle_count: u8,
    pub max_influence_regions: u8,
    pub support_radius: ScalarRange,
    pub falloff: ScalarRange,
}

impl FaceCageParameterization {
    /// Returns true when the operator has finite, bounded local support.
    #[must_use]
    pub fn is_compact(self) -> bool {
        self.handle_count > 0
            && self.handle_count <= MAX_COMPACT_CAGE_HANDLES
            && self.max_influence_regions > 0
            && self.max_influence_regions <= MAX_COMPACT_SUPPORT_REGIONS
            && normalized_range_is_valid(self.support_radius)
            && normalized_range_is_valid(self.falloff)
    }
}

/// Semantic scalar control with a finite authored range and default.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceControlSpec {
    pub id: &'static str,
    pub range: ScalarRange,
}

impl FaceControlSpec {
    /// Shared stable control id wrapper.
    #[must_use]
    pub fn control_id(self) -> CharacterControlId {
        CharacterControlId(self.id.to_owned())
    }

    /// Returns true when `value` is finite and inside this control range.
    #[must_use]
    pub fn contains(self, value: f32) -> bool {
        value.is_finite() && self.range.min <= value && value <= self.range.max
    }
}

/// Runtime value for validating authored face controls.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceControlValue<'a> {
    pub control: &'a str,
    pub value: f32,
}

/// Compact semantic face/head deformation operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct FaceDeformationOperator<'a> {
    pub id: &'static str,
    pub class: FaceRegionClass,
    pub targets: &'a [FaceRegionTarget],
    pub parameterization: FaceCageParameterization,
    pub controls: &'a [FaceControlSpec],
    pub symmetry: FaceSymmetryPolicy,
    pub preserves_loops: &'a [FaceLoopPreservation],
}

impl FaceDeformationOperator<'_> {
    /// Returns true when this declaration is locally bounded and small enough for cage editing.
    #[must_use]
    pub fn is_compact(self) -> bool {
        !self.targets.is_empty()
            && self.targets.len() <= MAX_COMPACT_TARGETS
            && !self.controls.is_empty()
            && self.controls.len() <= MAX_COMPACT_CONTROLS
            && self.parameterization.is_compact()
    }

    /// Returns true when all compact grammar validation rules pass.
    #[must_use]
    pub fn is_admissible(self) -> bool {
        validate_face_operator(&self).is_ok()
    }

    /// Finds a control declaration by stable id.
    #[must_use]
    pub fn control(self, id: &str) -> Option<FaceControlSpec> {
        self.controls
            .iter()
            .copied()
            .find(|control| control.id == id)
    }

    /// Returns true when a loop preservation declaration exists for `target`.
    #[must_use]
    pub fn preserves_loop(self, target: FaceLoopTarget) -> bool {
        self.preserves_loops
            .iter()
            .any(|preservation| preservation.target == target)
    }

    /// Base-topology symmetry ID required by this operator, when applicable.
    #[must_use]
    pub fn symmetry_id(self) -> Option<CharacterSymmetryId> {
        match self.symmetry {
            FaceSymmetryPolicy::MidlineLocked | FaceSymmetryPolicy::MirroredAcrossMidline => {
                Some(CharacterSymmetryId(match self.class {
                    FaceRegionClass::Eye => "eyes.symmetry.ocular_midline".to_owned(),
                    _ => "head.symmetry.sagittal".to_owned(),
                }))
            }
            FaceSymmetryPolicy::IndependentBilateral | FaceSymmetryPolicy::AsymmetricAllowed => {
                None
            }
        }
    }

    /// Versioned base that owns the operator symmetry contract.
    #[must_use]
    pub fn symmetry_base_id(self) -> Option<&'static str> {
        match self.symmetry {
            FaceSymmetryPolicy::MidlineLocked | FaceSymmetryPolicy::MirroredAcrossMidline => {
                Some(match self.class {
                    FaceRegionClass::Eye => HUMANOID_EYES_BASE_ID,
                    _ => HUMANOID_HEAD_BASE_ID,
                })
            }
            FaceSymmetryPolicy::IndependentBilateral | FaceSymmetryPolicy::AsymmetricAllowed => {
                None
            }
        }
    }
}

/// Validation failures for face/head deformation grammar declarations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FaceGrammarError {
    #[error("face operator id cannot be empty")]
    EmptyOperatorId,
    #[error("face operator id `{operator}` appears more than once")]
    DuplicateOperatorId { operator: String },
    #[error("face operator `{operator}` must target at least one region")]
    MissingTargets { operator: String },
    #[error("face operator `{operator}` must expose at least one control")]
    MissingControls { operator: String },
    #[error("face operator `{operator}` is not compact")]
    OperatorNotCompact { operator: String },
    #[error("face operator `{operator}` has invalid control range `{control}`")]
    InvalidControlRange { operator: String, control: String },
    #[error("face operator `{operator}` declares duplicate target `{target}`")]
    DuplicateTarget { operator: String, target: String },
    #[error("face operator `{operator}` declares duplicate control `{control}`")]
    DuplicateControl { operator: String, control: String },
    #[error("face operator `{operator}` declares duplicate loop preservation `{loop_id}`")]
    DuplicateLoopPreservation { operator: String, loop_id: String },
    #[error("face operator `{operator}` has target `{target}` outside class `{class}`")]
    TargetClassMismatch {
        operator: String,
        target: String,
        class: &'static str,
    },
    #[error("face operator `{operator}` has unpaired bilateral target `{target}`")]
    UnpairedBilateralTarget { operator: String, target: String },
    #[error("face operator `{operator}` uses midline symmetry with lateral target `{target}`")]
    LateralTargetInMidlineOperator { operator: String, target: String },
    #[error(
        "face operator `{operator}` declares loop preservation `{loop_id}` without owning target"
    )]
    LoopPreservationWithoutTarget { operator: String, loop_id: String },
    #[error("face operator `{operator}` is missing loop preservation `{loop_id}`")]
    MissingLoopPreservation { operator: String, loop_id: String },
    #[error("face operator `{operator}` has invalid loop preservation policy for `{loop_id}`")]
    InvalidLoopPreservationPolicy { operator: String, loop_id: String },
    #[error("face target `{target}` references missing base region `{region}`")]
    UnresolvedBaseRegion { target: String, region: String },
    #[error("face loop `{loop_id}` references missing base loop `{base_loop}`")]
    UnresolvedBaseLoop { loop_id: String, base_loop: String },
    #[error("face operator `{operator}` references missing base symmetry `{symmetry}`")]
    UnresolvedBaseSymmetry { operator: String, symmetry: String },
    #[error("face operator `{operator}` has incompatible cage/kernel for class `{class}`")]
    IncompatibleParameterization {
        operator: String,
        class: &'static str,
    },
    #[error("face grammar is missing required target `{target}`")]
    MissingRequiredTarget { target: String },
    #[error("face grammar is missing required class `{class}`")]
    MissingRequiredClass { class: &'static str },
    #[error("face operator `{operator}` received unknown control `{control}`")]
    UnknownControlValue { operator: String, control: String },
    #[error("face operator `{operator}` received duplicate control value `{control}`")]
    DuplicateControlValue { operator: String, control: String },
    #[error("face operator `{operator}` received non-finite control `{control}`")]
    NonFiniteControlValue { operator: String, control: String },
    #[error("face operator `{operator}` received out-of-range control `{control}`")]
    ControlValueOutOfRange { operator: String, control: String },
}

pub const REQUIRED_FACE_REGION_CLASSES: &[FaceRegionClass] = &[
    FaceRegionClass::Skull,
    FaceRegionClass::Jaw,
    FaceRegionClass::Cheek,
    FaceRegionClass::Brow,
    FaceRegionClass::Nose,
    FaceRegionClass::MouthLoop,
    FaceRegionClass::Eye,
    FaceRegionClass::Ear,
];

pub const ALL_FACE_REGION_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::SkullCranium,
    FaceRegionTarget::JawMandible,
    FaceRegionTarget::CheekLeft,
    FaceRegionTarget::CheekRight,
    FaceRegionTarget::BrowLeft,
    FaceRegionTarget::BrowRight,
    FaceRegionTarget::NoseBridge,
    FaceRegionTarget::NoseTip,
    FaceRegionTarget::NoseAlaLeft,
    FaceRegionTarget::NoseAlaRight,
    FaceRegionTarget::MouthOuterLoop,
    FaceRegionTarget::MouthInnerLoop,
    FaceRegionTarget::EyeLeftUpperLid,
    FaceRegionTarget::EyeLeftLowerLid,
    FaceRegionTarget::EyeRightUpperLid,
    FaceRegionTarget::EyeRightLowerLid,
    FaceRegionTarget::EarLeft,
    FaceRegionTarget::EarRight,
];

const SKULL_TARGETS: &[FaceRegionTarget] = &[FaceRegionTarget::SkullCranium];
const JAW_TARGETS: &[FaceRegionTarget] = &[FaceRegionTarget::JawMandible];
const CHEEK_TARGETS: &[FaceRegionTarget] =
    &[FaceRegionTarget::CheekLeft, FaceRegionTarget::CheekRight];
const BROW_TARGETS: &[FaceRegionTarget] =
    &[FaceRegionTarget::BrowLeft, FaceRegionTarget::BrowRight];
const NOSE_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::NoseBridge,
    FaceRegionTarget::NoseTip,
    FaceRegionTarget::NoseAlaLeft,
    FaceRegionTarget::NoseAlaRight,
];
const MOUTH_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::MouthOuterLoop,
    FaceRegionTarget::MouthInnerLoop,
];
const EYE_TARGETS: &[FaceRegionTarget] = &[
    FaceRegionTarget::EyeLeftUpperLid,
    FaceRegionTarget::EyeLeftLowerLid,
    FaceRegionTarget::EyeRightUpperLid,
    FaceRegionTarget::EyeRightLowerLid,
];
const EAR_TARGETS: &[FaceRegionTarget] = &[FaceRegionTarget::EarLeft, FaceRegionTarget::EarRight];

const SKULL_CONTROLS: &[FaceControlSpec] = &[
    control("face.skull.width", 0.75, 1.25, 1.0),
    control("face.skull.height", 0.75, 1.25, 1.0),
    control("face.skull.depth", 0.75, 1.25, 1.0),
];

const JAW_CONTROLS: &[FaceControlSpec] = &[
    control("face.jaw.width", 0.70, 1.30, 1.0),
    control("face.jaw.length", 0.70, 1.30, 1.0),
    control("face.jaw.chin_projection", -1.0, 1.0, 0.0),
    control("face.jaw.angle", -1.0, 1.0, 0.0),
];

const CHEEK_CONTROLS: &[FaceControlSpec] = &[
    control("face.cheek.fullness", -1.0, 1.0, 0.0),
    control("face.cheekbone.width", 0.70, 1.30, 1.0),
    control("face.cheekbone.height", -1.0, 1.0, 0.0),
];

const BROW_CONTROLS: &[FaceControlSpec] = &[
    control("face.brow.left_height", -1.0, 1.0, 0.0),
    control("face.brow.right_height", -1.0, 1.0, 0.0),
    control("face.brow.arc", -1.0, 1.0, 0.0),
];

const NOSE_CONTROLS: &[FaceControlSpec] = &[
    control("face.nose.bridge_width", 0.60, 1.40, 1.0),
    control("face.nose.bridge_height", -1.0, 1.0, 0.0),
    control("face.nose.tip_projection", -1.0, 1.0, 0.0),
    control("face.nose.alar_width", 0.60, 1.40, 1.0),
];

const MOUTH_CONTROLS: &[FaceControlSpec] = &[
    control("face.mouth.width", 0.60, 1.40, 1.0),
    control("face.mouth.upper_lip_volume", -1.0, 1.0, 0.0),
    control("face.mouth.lower_lip_volume", -1.0, 1.0, 0.0),
    control("face.mouth.corner_pull", -1.0, 1.0, 0.0),
];

const EYE_CONTROLS: &[FaceControlSpec] = &[
    control("face.eye.left_aperture", 0.50, 1.50, 1.0),
    control("face.eye.right_aperture", 0.50, 1.50, 1.0),
    control("face.eye.cantus_tilt", -1.0, 1.0, 0.0),
    control("face.eye.orbit_scale", 0.75, 1.25, 1.0),
];

const EAR_CONTROLS: &[FaceControlSpec] = &[
    control("face.ear.scale", 0.60, 1.40, 1.0),
    control("face.ear.flare", -1.0, 1.0, 0.0),
    control("face.ear.lobe", -1.0, 1.0, 0.0),
];

const MOUTH_LOOP_PRESERVATION: &[FaceLoopPreservation] = &[
    FaceLoopPreservation {
        target: FaceLoopTarget::MouthOuter,
        policy: FaceLoopPreservationPolicy::LipSealAndCornerOrder,
    },
    FaceLoopPreservation {
        target: FaceLoopTarget::MouthInner,
        policy: FaceLoopPreservationPolicy::LipSealAndCornerOrder,
    },
];

const EYE_LOOP_PRESERVATION: &[FaceLoopPreservation] = &[
    FaceLoopPreservation {
        target: FaceLoopTarget::EyeLeftLid,
        policy: FaceLoopPreservationPolicy::LidClosureAndCanthusOrder,
    },
    FaceLoopPreservation {
        target: FaceLoopTarget::EyeRightLid,
        policy: FaceLoopPreservationPolicy::LidClosureAndCanthusOrder,
    },
];

/// Built-in compact face/head deformation operators.
pub const FACE_DEFORMATION_OPERATORS: &[FaceDeformationOperator<'static>] = &[
    FaceDeformationOperator {
        id: "face.operator.skull.compact_cranium",
        class: FaceRegionClass::Skull,
        targets: SKULL_TARGETS,
        parameterization: cage(
            FaceCageKind::SkullEllipsoid,
            FaceRegionKernel::SmoothCompact,
            8,
            1,
            ScalarRange {
                min: 0.65,
                max: 1.0,
                default: 0.90,
            },
            ScalarRange {
                min: 0.15,
                max: 0.85,
                default: 0.45,
            },
        ),
        controls: SKULL_CONTROLS,
        symmetry: FaceSymmetryPolicy::MidlineLocked,
        preserves_loops: &[],
    },
    FaceDeformationOperator {
        id: "face.operator.jaw.compact_mandible",
        class: FaceRegionClass::Jaw,
        targets: JAW_TARGETS,
        parameterization: cage(
            FaceCageKind::MandibleHinge,
            FaceRegionKernel::HingeLimited,
            6,
            1,
            ScalarRange {
                min: 0.35,
                max: 0.75,
                default: 0.55,
            },
            ScalarRange {
                min: 0.20,
                max: 0.80,
                default: 0.50,
            },
        ),
        controls: JAW_CONTROLS,
        symmetry: FaceSymmetryPolicy::MidlineLocked,
        preserves_loops: &[],
    },
    FaceDeformationOperator {
        id: "face.operator.cheek.compact_patch_pair",
        class: FaceRegionClass::Cheek,
        targets: CHEEK_TARGETS,
        parameterization: cage(
            FaceCageKind::CheekPatchPair,
            FaceRegionKernel::SurfaceSlide,
            8,
            2,
            ScalarRange {
                min: 0.25,
                max: 0.55,
                default: 0.40,
            },
            ScalarRange {
                min: 0.10,
                max: 0.70,
                default: 0.35,
            },
        ),
        controls: CHEEK_CONTROLS,
        symmetry: FaceSymmetryPolicy::MirroredAcrossMidline,
        preserves_loops: &[],
    },
    FaceDeformationOperator {
        id: "face.operator.brow.compact_arc_pair",
        class: FaceRegionClass::Brow,
        targets: BROW_TARGETS,
        parameterization: cage(
            FaceCageKind::BrowArcPair,
            FaceRegionKernel::SurfaceSlide,
            8,
            2,
            ScalarRange {
                min: 0.20,
                max: 0.45,
                default: 0.30,
            },
            ScalarRange {
                min: 0.10,
                max: 0.65,
                default: 0.35,
            },
        ),
        controls: BROW_CONTROLS,
        symmetry: FaceSymmetryPolicy::AsymmetricAllowed,
        preserves_loops: &[],
    },
    FaceDeformationOperator {
        id: "face.operator.nose.compact_wedge",
        class: FaceRegionClass::Nose,
        targets: NOSE_TARGETS,
        parameterization: cage(
            FaceCageKind::NoseWedge,
            FaceRegionKernel::SmoothCompact,
            10,
            4,
            ScalarRange {
                min: 0.18,
                max: 0.50,
                default: 0.32,
            },
            ScalarRange {
                min: 0.10,
                max: 0.75,
                default: 0.40,
            },
        ),
        controls: NOSE_CONTROLS,
        symmetry: FaceSymmetryPolicy::AsymmetricAllowed,
        preserves_loops: &[],
    },
    FaceDeformationOperator {
        id: "face.operator.mouth.compact_loop_ring",
        class: FaceRegionClass::MouthLoop,
        targets: MOUTH_TARGETS,
        parameterization: cage(
            FaceCageKind::MouthLoopRing,
            FaceRegionKernel::LoopConstrained,
            12,
            2,
            ScalarRange {
                min: 0.12,
                max: 0.40,
                default: 0.24,
            },
            ScalarRange {
                min: 0.10,
                max: 0.60,
                default: 0.30,
            },
        ),
        controls: MOUTH_CONTROLS,
        symmetry: FaceSymmetryPolicy::MidlineLocked,
        preserves_loops: MOUTH_LOOP_PRESERVATION,
    },
    FaceDeformationOperator {
        id: "face.operator.eye.compact_orbital_loops",
        class: FaceRegionClass::Eye,
        targets: EYE_TARGETS,
        parameterization: cage(
            FaceCageKind::OrbitalLoopPair,
            FaceRegionKernel::LoopConstrained,
            12,
            2,
            ScalarRange {
                min: 0.12,
                max: 0.36,
                default: 0.22,
            },
            ScalarRange {
                min: 0.10,
                max: 0.55,
                default: 0.30,
            },
        ),
        controls: EYE_CONTROLS,
        symmetry: FaceSymmetryPolicy::IndependentBilateral,
        preserves_loops: EYE_LOOP_PRESERVATION,
    },
    FaceDeformationOperator {
        id: "face.operator.ear.compact_shell_pair",
        class: FaceRegionClass::Ear,
        targets: EAR_TARGETS,
        parameterization: cage(
            FaceCageKind::EarShellPair,
            FaceRegionKernel::SurfaceSlide,
            8,
            2,
            ScalarRange {
                min: 0.18,
                max: 0.48,
                default: 0.30,
            },
            ScalarRange {
                min: 0.10,
                max: 0.70,
                default: 0.35,
            },
        ),
        controls: EAR_CONTROLS,
        symmetry: FaceSymmetryPolicy::MirroredAcrossMidline,
        preserves_loops: &[],
    },
];

/// Validate all built-in face/head operators and required target coverage.
pub fn validate_face_deformation_grammar() -> Result<(), FaceGrammarError> {
    let mut operator_ids = BTreeSet::new();
    for operator in FACE_DEFORMATION_OPERATORS {
        if !operator_ids.insert(operator.id) {
            return Err(FaceGrammarError::DuplicateOperatorId {
                operator: operator.id.to_owned(),
            });
        }
        validate_face_operator(operator)?;
    }

    for target in ALL_FACE_REGION_TARGETS {
        if !FACE_DEFORMATION_OPERATORS
            .iter()
            .any(|operator| operator.targets.contains(target))
        {
            return Err(FaceGrammarError::MissingRequiredTarget {
                target: target.as_str().to_owned(),
            });
        }
    }

    for class in REQUIRED_FACE_REGION_CLASSES {
        if !FACE_DEFORMATION_OPERATORS
            .iter()
            .any(|operator| operator.class == *class)
        {
            return Err(FaceGrammarError::MissingRequiredClass {
                class: class.as_str(),
            });
        }
    }

    Ok(())
}

/// Validate a compact face/head deformation operator declaration.
pub fn validate_face_operator(
    operator: &FaceDeformationOperator<'_>,
) -> Result<(), FaceGrammarError> {
    if operator.id.is_empty() {
        return Err(FaceGrammarError::EmptyOperatorId);
    }

    if operator.targets.is_empty() {
        return Err(FaceGrammarError::MissingTargets {
            operator: operator.id.to_owned(),
        });
    }

    if operator.controls.is_empty() {
        return Err(FaceGrammarError::MissingControls {
            operator: operator.id.to_owned(),
        });
    }

    if !operator.is_compact() {
        return Err(FaceGrammarError::OperatorNotCompact {
            operator: operator.id.to_owned(),
        });
    }
    if !parameterization_matches_class(operator.class, operator.parameterization) {
        return Err(FaceGrammarError::IncompatibleParameterization {
            operator: operator.id.to_owned(),
            class: operator.class.as_str(),
        });
    }

    let base_refs = base_reference_sets();
    let mut seen_targets = BTreeSet::new();
    for target in operator.targets {
        if !seen_targets.insert(target.as_str()) {
            return Err(FaceGrammarError::DuplicateTarget {
                operator: operator.id.to_owned(),
                target: target.as_str().to_owned(),
            });
        }
        if target.class() != operator.class {
            return Err(FaceGrammarError::TargetClassMismatch {
                operator: operator.id.to_owned(),
                target: target.as_str().to_owned(),
                class: operator.class.as_str(),
            });
        }
        if !base_refs.has_region(target.base_id(), target.base_region_id()) {
            return Err(FaceGrammarError::UnresolvedBaseRegion {
                target: target.as_str().to_owned(),
                region: target.base_region_id().to_owned(),
            });
        }
    }

    let mut seen_controls = BTreeSet::new();
    for control in operator.controls {
        if !seen_controls.insert(control.id) {
            return Err(FaceGrammarError::DuplicateControl {
                operator: operator.id.to_owned(),
                control: control.id.to_owned(),
            });
        }
        if control.id.is_empty() || !control.range.is_valid() {
            return Err(FaceGrammarError::InvalidControlRange {
                operator: operator.id.to_owned(),
                control: control.id.to_owned(),
            });
        }
    }

    validate_symmetry_policy(operator)?;
    validate_loop_preservation(operator, &base_refs)?;
    if let (Some(symmetry), Some(base_id)) = (operator.symmetry_id(), operator.symmetry_base_id())
        && !base_refs.has_symmetry(base_id, symmetry.0.as_str())
    {
        return Err(FaceGrammarError::UnresolvedBaseSymmetry {
            operator: operator.id.to_owned(),
            symmetry: symmetry.0,
        });
    }

    Ok(())
}

/// Validate finite authored control values for a specific operator.
pub fn validate_face_control_values(
    operator: &FaceDeformationOperator<'_>,
    values: &[FaceControlValue<'_>],
) -> Result<(), FaceGrammarError> {
    let mut seen_values = BTreeSet::new();
    for value in values {
        if !seen_values.insert(value.control) {
            return Err(FaceGrammarError::DuplicateControlValue {
                operator: operator.id.to_owned(),
                control: value.control.to_owned(),
            });
        }
        let Some(control) = operator.control(value.control) else {
            return Err(FaceGrammarError::UnknownControlValue {
                operator: operator.id.to_owned(),
                control: value.control.to_owned(),
            });
        };

        if !value.value.is_finite() {
            return Err(FaceGrammarError::NonFiniteControlValue {
                operator: operator.id.to_owned(),
                control: value.control.to_owned(),
            });
        }

        if !control.contains(value.value) {
            return Err(FaceGrammarError::ControlValueOutOfRange {
                operator: operator.id.to_owned(),
                control: value.control.to_owned(),
            });
        }
    }

    Ok(())
}

fn parameterization_matches_class(
    class: FaceRegionClass,
    parameterization: FaceCageParameterization,
) -> bool {
    matches!(
        (class, parameterization.cage, parameterization.kernel),
        (
            FaceRegionClass::Skull,
            FaceCageKind::SkullEllipsoid,
            FaceRegionKernel::SmoothCompact
        ) | (
            FaceRegionClass::Jaw,
            FaceCageKind::MandibleHinge,
            FaceRegionKernel::HingeLimited
        ) | (
            FaceRegionClass::Cheek,
            FaceCageKind::CheekPatchPair,
            FaceRegionKernel::SurfaceSlide
        ) | (
            FaceRegionClass::Brow,
            FaceCageKind::BrowArcPair,
            FaceRegionKernel::SurfaceSlide
        ) | (
            FaceRegionClass::Nose,
            FaceCageKind::NoseWedge,
            FaceRegionKernel::SmoothCompact
        ) | (
            FaceRegionClass::MouthLoop,
            FaceCageKind::MouthLoopRing,
            FaceRegionKernel::LoopConstrained
        ) | (
            FaceRegionClass::Eye,
            FaceCageKind::OrbitalLoopPair,
            FaceRegionKernel::LoopConstrained
        ) | (
            FaceRegionClass::Ear,
            FaceCageKind::EarShellPair,
            FaceRegionKernel::SurfaceSlide
        )
    )
}

const fn control(id: &'static str, min: f32, max: f32, default: f32) -> FaceControlSpec {
    FaceControlSpec {
        id,
        range: ScalarRange { min, max, default },
    }
}

const fn cage(
    cage: FaceCageKind,
    kernel: FaceRegionKernel,
    handle_count: u8,
    max_influence_regions: u8,
    support_radius: ScalarRange,
    falloff: ScalarRange,
) -> FaceCageParameterization {
    FaceCageParameterization {
        cage,
        kernel,
        handle_count,
        max_influence_regions,
        support_radius,
        falloff,
    }
}

fn normalized_range_is_valid(range: ScalarRange) -> bool {
    range.is_valid() && range.min >= 0.0 && range.max <= 1.0
}

fn validate_symmetry_policy(
    operator: &FaceDeformationOperator<'_>,
) -> Result<(), FaceGrammarError> {
    match operator.symmetry {
        FaceSymmetryPolicy::MidlineLocked => {
            for target in operator.targets {
                if target.side() != FaceSide::Midline {
                    return Err(FaceGrammarError::LateralTargetInMidlineOperator {
                        operator: operator.id.to_owned(),
                        target: target.as_str().to_owned(),
                    });
                }
            }
        }
        FaceSymmetryPolicy::MirroredAcrossMidline | FaceSymmetryPolicy::IndependentBilateral => {
            for target in operator.targets {
                if target.side() == FaceSide::Midline {
                    return Err(FaceGrammarError::LateralTargetInMidlineOperator {
                        operator: operator.id.to_owned(),
                        target: target.as_str().to_owned(),
                    });
                }
            }
            if let Some(target) = first_unpaired_bilateral_target(operator.targets) {
                return Err(FaceGrammarError::UnpairedBilateralTarget {
                    operator: operator.id.to_owned(),
                    target: target.as_str().to_owned(),
                });
            }
        }
        FaceSymmetryPolicy::AsymmetricAllowed => {}
    }

    Ok(())
}

fn first_unpaired_bilateral_target(targets: &[FaceRegionTarget]) -> Option<FaceRegionTarget> {
    targets.iter().copied().find(|target| {
        target
            .counterpart()
            .is_some_and(|counterpart| !targets.contains(&counterpart))
    })
}

fn validate_loop_preservation(
    operator: &FaceDeformationOperator<'_>,
    base_refs: &BaseReferenceSets,
) -> Result<(), FaceGrammarError> {
    let mut seen = BTreeSet::new();
    for preservation in operator.preserves_loops {
        if !seen.insert(preservation.target.as_str()) {
            return Err(FaceGrammarError::DuplicateLoopPreservation {
                operator: operator.id.to_owned(),
                loop_id: preservation.target.as_str().to_owned(),
            });
        }
        if !preservation
            .target
            .owning_targets()
            .iter()
            .all(|target| operator.targets.contains(target))
        {
            return Err(FaceGrammarError::LoopPreservationWithoutTarget {
                operator: operator.id.to_owned(),
                loop_id: preservation.target.as_str().to_owned(),
            });
        }
        if preservation.policy != preservation.target.required_policy() {
            return Err(FaceGrammarError::InvalidLoopPreservationPolicy {
                operator: operator.id.to_owned(),
                loop_id: preservation.target.as_str().to_owned(),
            });
        }
        if !base_refs.has_loop(
            preservation.target.base_id(),
            preservation.target.base_loop_id(),
        ) {
            return Err(FaceGrammarError::UnresolvedBaseLoop {
                loop_id: preservation.target.as_str().to_owned(),
                base_loop: preservation.target.base_loop_id().to_owned(),
            });
        }
    }

    for target in operator.targets {
        if let Some(loop_target) = target.required_loop()
            && !operator.preserves_loop(loop_target)
        {
            return Err(FaceGrammarError::MissingLoopPreservation {
                operator: operator.id.to_owned(),
                loop_id: loop_target.as_str().to_owned(),
            });
        }
    }

    Ok(())
}

struct BaseReferenceSets {
    regions: BTreeMap<String, BTreeSet<String>>,
    loops: BTreeMap<String, BTreeSet<String>>,
    symmetries: BTreeMap<String, BTreeSet<String>>,
}

impl BaseReferenceSets {
    fn has_region(&self, base_id: &str, region_id: &str) -> bool {
        self.regions
            .get(base_id)
            .is_some_and(|regions| regions.contains(region_id))
    }

    fn has_loop(&self, base_id: &str, loop_id: &str) -> bool {
        self.loops
            .get(base_id)
            .is_some_and(|loops| loops.contains(loop_id))
    }

    fn has_symmetry(&self, base_id: &str, symmetry_id: &str) -> bool {
        self.symmetries
            .get(base_id)
            .is_some_and(|symmetries| symmetries.contains(symmetry_id))
    }
}

fn base_reference_sets() -> BaseReferenceSets {
    let library = base_topology_library();
    BaseReferenceSets {
        regions: library
            .bases
            .iter()
            .map(|base| {
                (
                    base.id.0.clone(),
                    base.regions
                        .iter()
                        .map(|region| region.id.0.clone())
                        .collect(),
                )
            })
            .collect(),
        loops: library
            .bases
            .iter()
            .map(|base| {
                (
                    base.id.0.clone(),
                    base.loops
                        .iter()
                        .map(|topology_loop| topology_loop.id.0.clone())
                        .collect(),
                )
            })
            .collect(),
        symmetries: library
            .bases
            .iter()
            .map(|base| {
                (
                    base.id.0.clone(),
                    base.symmetries
                        .iter()
                        .map(|symmetry| symmetry.id.0.clone())
                        .collect(),
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn required_facial_targets_are_covered() {
        let covered_targets = FACE_DEFORMATION_OPERATORS
            .iter()
            .flat_map(|operator| operator.targets.iter().copied())
            .collect::<BTreeSet<_>>();
        let covered_classes = FACE_DEFORMATION_OPERATORS
            .iter()
            .map(|operator| operator.class)
            .collect::<BTreeSet<_>>();
        let base_refs = base_reference_sets();

        for target in ALL_FACE_REGION_TARGETS {
            assert!(
                covered_targets.contains(target),
                "missing semantic target {}",
                target.as_str()
            );
            assert!(base_refs.has_region(target.base_id(), target.region_id().0.as_str()));
        }

        for class in REQUIRED_FACE_REGION_CLASSES {
            assert!(
                covered_classes.contains(class),
                "missing face class {}",
                class.as_str()
            );
        }
    }

    #[test]
    fn operators_are_compact_and_admissible() {
        validate_face_deformation_grammar().expect("built-in face grammar must validate");

        for operator in FACE_DEFORMATION_OPERATORS {
            assert!(operator.is_compact(), "{} is not compact", operator.id);
            assert!(
                operator.is_admissible(),
                "{} is not admissible",
                operator.id
            );
            assert!(operator.controls.iter().all(|control| {
                control.control_id().0.starts_with("face.") && control.range.is_valid()
            }));
            assert!(
                operator.symmetry == FaceSymmetryPolicy::AsymmetricAllowed
                    || operator.symmetry_id().is_some()
                    || operator.symmetry == FaceSymmetryPolicy::IndependentBilateral
            );
            if let Some(symmetry) = operator.symmetry_id() {
                let symmetry_base = operator
                    .symmetry_base_id()
                    .expect("symmetry id must have a base id");
                assert!(base_reference_sets().has_symmetry(symmetry_base, symmetry.0.as_str()));
            }
        }
    }

    #[test]
    fn mouth_and_eye_loop_preservation_is_declared() {
        let mouth = FACE_DEFORMATION_OPERATORS
            .iter()
            .find(|operator| operator.class == FaceRegionClass::MouthLoop)
            .expect("mouth loop operator");
        let eye = FACE_DEFORMATION_OPERATORS
            .iter()
            .find(|operator| operator.class == FaceRegionClass::Eye)
            .expect("eye loop operator");

        assert!(mouth.preserves_loop(FaceLoopTarget::MouthOuter));
        assert!(mouth.preserves_loop(FaceLoopTarget::MouthInner));
        assert!(eye.preserves_loop(FaceLoopTarget::EyeLeftLid));
        assert!(eye.preserves_loop(FaceLoopTarget::EyeRightLid));

        for loop_target in [
            FaceLoopTarget::MouthOuter,
            FaceLoopTarget::MouthInner,
            FaceLoopTarget::EyeLeftLid,
            FaceLoopTarget::EyeRightLid,
        ] {
            assert!(
                base_reference_sets()
                    .has_loop(loop_target.base_id(), loop_target.loop_id().0.as_str())
            );
        }
    }

    #[test]
    fn validation_rejects_invalid_ranges_and_non_finite_controls() {
        let invalid_controls = [FaceControlSpec {
            id: "face.test.invalid_range",
            range: ScalarRange {
                min: 1.0,
                max: -1.0,
                default: 0.0,
            },
        }];
        let invalid_range_operator = FaceDeformationOperator {
            controls: &invalid_controls,
            ..FACE_DEFORMATION_OPERATORS[0]
        };

        assert!(matches!(
            validate_face_operator(&invalid_range_operator),
            Err(FaceGrammarError::InvalidControlRange { .. })
        ));

        let duplicate_targets = [FaceRegionTarget::CheekLeft, FaceRegionTarget::CheekLeft];
        let duplicate_target_operator = FaceDeformationOperator {
            targets: &duplicate_targets,
            ..FACE_DEFORMATION_OPERATORS[2]
        };
        assert!(matches!(
            validate_face_operator(&duplicate_target_operator),
            Err(FaceGrammarError::DuplicateTarget { .. })
        ));

        let duplicate_controls = [
            FACE_DEFORMATION_OPERATORS[0].controls[0],
            FACE_DEFORMATION_OPERATORS[0].controls[0],
        ];
        let duplicate_control_operator = FaceDeformationOperator {
            controls: &duplicate_controls,
            ..FACE_DEFORMATION_OPERATORS[0]
        };
        assert!(matches!(
            validate_face_operator(&duplicate_control_operator),
            Err(FaceGrammarError::DuplicateControl { .. })
        ));

        let bad_loop_preservation = [FaceLoopPreservation {
            target: FaceLoopTarget::EyeLeftLid,
            policy: FaceLoopPreservationPolicy::LipSealAndCornerOrder,
        }];
        let bad_loop_operator = FaceDeformationOperator {
            preserves_loops: &bad_loop_preservation,
            ..FACE_DEFORMATION_OPERATORS
                .iter()
                .copied()
                .find(|operator| operator.class == FaceRegionClass::Eye)
                .expect("eye operator")
        };
        assert!(matches!(
            validate_face_operator(&bad_loop_operator),
            Err(FaceGrammarError::InvalidLoopPreservationPolicy { .. })
        ));

        let duplicate_loop_preservation = [
            FaceLoopPreservation {
                target: FaceLoopTarget::MouthOuter,
                policy: FaceLoopPreservationPolicy::LipSealAndCornerOrder,
            },
            FaceLoopPreservation {
                target: FaceLoopTarget::MouthOuter,
                policy: FaceLoopPreservationPolicy::LipSealAndCornerOrder,
            },
        ];
        let duplicate_loop_operator = FaceDeformationOperator {
            preserves_loops: &duplicate_loop_preservation,
            ..FACE_DEFORMATION_OPERATORS
                .iter()
                .copied()
                .find(|operator| operator.class == FaceRegionClass::MouthLoop)
                .expect("mouth operator")
        };
        assert!(matches!(
            validate_face_operator(&duplicate_loop_operator),
            Err(FaceGrammarError::DuplicateLoopPreservation { .. })
        ));

        let invalid_symmetry_operator = FaceDeformationOperator {
            symmetry: FaceSymmetryPolicy::MirroredAcrossMidline,
            ..FACE_DEFORMATION_OPERATORS
                .iter()
                .copied()
                .find(|operator| operator.class == FaceRegionClass::Skull)
                .expect("skull operator")
        };
        assert!(matches!(
            validate_face_operator(&invalid_symmetry_operator),
            Err(FaceGrammarError::LateralTargetInMidlineOperator { .. })
        ));

        let bad_parameterization = FaceDeformationOperator {
            parameterization: cage(
                FaceCageKind::OrbitalLoopPair,
                FaceRegionKernel::LoopConstrained,
                4,
                1,
                ScalarRange {
                    min: 0.10,
                    max: 0.50,
                    default: 0.25,
                },
                ScalarRange {
                    min: 0.10,
                    max: 0.50,
                    default: 0.25,
                },
            ),
            ..FACE_DEFORMATION_OPERATORS
                .iter()
                .copied()
                .find(|operator| operator.class == FaceRegionClass::Jaw)
                .expect("jaw operator")
        };
        assert!(matches!(
            validate_face_operator(&bad_parameterization),
            Err(FaceGrammarError::IncompatibleParameterization { .. })
        ));

        let missing_loop_preservation = FaceDeformationOperator {
            preserves_loops: &[],
            ..FACE_DEFORMATION_OPERATORS
                .iter()
                .copied()
                .find(|operator| operator.class == FaceRegionClass::MouthLoop)
                .expect("mouth operator")
        };
        assert!(matches!(
            validate_face_operator(&missing_loop_preservation),
            Err(FaceGrammarError::MissingLoopPreservation { .. })
        ));

        let skull = FACE_DEFORMATION_OPERATORS
            .iter()
            .find(|operator| operator.class == FaceRegionClass::Skull)
            .expect("skull operator");
        let non_finite_values = [FaceControlValue {
            control: "face.skull.width",
            value: f32::NAN,
        }];

        assert!(matches!(
            validate_face_control_values(skull, &non_finite_values),
            Err(FaceGrammarError::NonFiniteControlValue { .. })
        ));

        let out_of_range_values = [FaceControlValue {
            control: "face.skull.width",
            value: 10.0,
        }];

        assert!(matches!(
            validate_face_control_values(skull, &out_of_range_values),
            Err(FaceGrammarError::ControlValueOutOfRange { .. })
        ));

        let duplicate_values = [
            FaceControlValue {
                control: "face.skull.width",
                value: 1.0,
            },
            FaceControlValue {
                control: "face.skull.width",
                value: 1.1,
            },
        ];
        assert!(matches!(
            validate_face_control_values(skull, &duplicate_values),
            Err(FaceGrammarError::DuplicateControlValue { .. })
        ));

        let unknown_values = [FaceControlValue {
            control: "face.skull.unknown",
            value: 1.0,
        }];
        assert!(matches!(
            validate_face_control_values(skull, &unknown_values),
            Err(FaceGrammarError::UnknownControlValue { .. })
        ));
    }
}
