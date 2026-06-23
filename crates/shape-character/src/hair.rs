//! Hair grammar.
//!
//! The hair grammar is intentionally semantic and bounded. It describes masses,
//! guide curves, renderable cards, clumps, parting regions, and procedural
//! placement controls. It does not attempt arbitrary strand-perfect
//! reconstruction.

use crate::{
    CharacterControlId, CharacterGrammarId, CharacterLandmarkId, CharacterRegionId, ScalarRange,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Stable namespace used for deterministic hair identifiers.
pub const HAIR_GRAMMAR_NAMESPACE: &str = "shape-character.hair.v1";

/// Hair grammar schema version.
pub const HAIR_GRAMMAR_SCHEMA_VERSION: u32 = 1;

/// Hard cap for generated hair card or clump placements.
pub const MAX_PROCEDURAL_HAIR_INSTANCES: u32 = 20_000;

/// Maximum number of Bezier/control points carried by one semantic guide curve.
pub const MAX_CURVE_CONTROL_POINTS: usize = 8;

/// Minimum number of points needed for a semantic guide curve.
pub const MIN_CURVE_CONTROL_POINTS: usize = 2;

/// Maximum subdivisions for one authored hair card.
pub const MAX_CARD_SEGMENTS: u32 = 32;

/// Stable hair element identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HairElementId(pub String);

impl HairElementId {
    /// Build a deterministic identifier from an element kind and semantic key.
    #[must_use]
    pub fn deterministic(kind: HairElementKind, key: impl AsRef<str>) -> Self {
        Self(format!(
            "hair:{}:{}",
            kind.as_str(),
            deterministic_suffix(kind.as_str(), key.as_ref())
        ))
    }

    /// Borrow the identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true when the identifier is non-empty and in the hair namespace.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut parts = self.0.split(':');
        let Some(prefix) = parts.next() else {
            return false;
        };
        let Some(kind) = parts.next() else {
            return false;
        };
        let Some(digest) = parts.next() else {
            return false;
        };
        parts.next().is_none()
            && prefix == "hair"
            && HairElementKind::from_str(kind).is_some()
            && digest.len() == 16
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }

    /// Parsed deterministic ID kind.
    #[must_use]
    pub fn kind(&self) -> Option<HairElementKind> {
        let mut parts = self.0.split(':');
        (parts.next() == Some("hair"))
            .then(|| parts.next())
            .flatten()
            .and_then(HairElementKind::from_str)
    }
}

/// Hair element categories that participate in deterministic IDs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairElementKind {
    /// Volumetric or silhouette mass.
    Mass,
    /// Semantic guide curve.
    Curve,
    /// Renderable card strip.
    Card,
    /// Procedural clump group.
    Clump,
    /// Parting or separation region.
    PartingRegion,
    /// Procedural placement contract.
    Placement,
    /// Operation contract.
    Operation,
}

impl HairElementKind {
    /// Stable string used in deterministic IDs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mass => "mass",
            Self::Curve => "curve",
            Self::Card => "card",
            Self::Clump => "clump",
            Self::PartingRegion => "parting-region",
            Self::Placement => "placement",
            Self::Operation => "operation",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "mass" => Some(Self::Mass),
            "curve" => Some(Self::Curve),
            "card" => Some(Self::Card),
            "clump" => Some(Self::Clump),
            "parting-region" => Some(Self::PartingRegion),
            "placement" => Some(Self::Placement),
            "operation" => Some(Self::Operation),
            _ => None,
        }
    }
}

/// Required primitive kinds for the initial hair grammar.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairPrimitiveKind {
    /// Bounded silhouette or volume mass.
    Mass,
    /// Semantic guide curve for flow and silhouette.
    Curve,
    /// Renderable hair card strip.
    Card,
    /// Procedural clump grouping.
    Clump,
    /// Parting or separation region.
    PartingRegion,
}

impl From<HairPrimitiveKind> for HairElementKind {
    fn from(value: HairPrimitiveKind) -> Self {
        match value {
            HairPrimitiveKind::Mass => Self::Mass,
            HairPrimitiveKind::Curve => Self::Curve,
            HairPrimitiveKind::Card => Self::Card,
            HairPrimitiveKind::Clump => Self::Clump,
            HairPrimitiveKind::PartingRegion => Self::PartingRegion,
        }
    }
}

/// Required primitive set for interoperable hair grammar implementations.
pub const REQUIRED_HAIR_PRIMITIVES: [HairPrimitiveKind; 5] = [
    HairPrimitiveKind::Mass,
    HairPrimitiveKind::Curve,
    HairPrimitiveKind::Card,
    HairPrimitiveKind::Clump,
    HairPrimitiveKind::PartingRegion,
];

/// Small integer range with a contained default.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountRange {
    /// Inclusive minimum.
    pub min: u32,
    /// Inclusive maximum.
    pub max: u32,
    /// Default authored value.
    pub default: u32,
}

impl CountRange {
    /// Construct a count range.
    #[must_use]
    pub const fn new(min: u32, max: u32, default: u32) -> Self {
        Self { min, max, default }
    }

    /// Validate ordered bounds and default containment.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.min <= self.default && self.default <= self.max
    }

    /// Validate the range and enforce an explicit maximum.
    #[must_use]
    pub const fn is_bounded_by(self, cap: u32) -> bool {
        self.is_valid() && self.max <= cap
    }
}

/// Diagnostics emitted by hair grammar validation.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairDiagnostic {
    /// Schema version mismatch.
    #[error("invalid hair grammar schema version {schema_version}")]
    InvalidSchemaVersion {
        /// Actual schema version.
        schema_version: u32,
    },
    /// Identifier is empty or outside the local hair namespace.
    #[error("invalid hair id for {field}")]
    InvalidHairId {
        /// Field or primitive being validated.
        field: String,
    },
    /// Shared character identifier is empty.
    #[error("invalid character id for {field}")]
    InvalidCharacterId {
        /// Field or primitive being validated.
        field: String,
    },
    /// Floating-point range has non-finite values or an out-of-bounds default.
    #[error("invalid scalar range for {field}")]
    InvalidScalarRange {
        /// Field or control being validated.
        field: String,
    },
    /// Floating-point range is not contained in `[0, 1]`.
    #[error("invalid normalized range for {field}")]
    InvalidNormalizedRange {
        /// Field or control being validated.
        field: String,
    },
    /// Count range has unordered bounds or an out-of-bounds default.
    #[error("invalid count range for {field}")]
    InvalidCountRange {
        /// Field or control being validated.
        field: String,
    },
    /// Procedural placement is not bounded by the grammar cap.
    #[error("unbounded procedural placement for {field}")]
    UnboundedPlacement {
        /// Field or placement being validated.
        field: String,
        /// Requested maximum.
        requested_max: u32,
        /// Grammar maximum.
        allowed_max: u32,
    },
    /// A required collection is empty or outside its allowed size.
    #[error("invalid collection size for {field}")]
    InvalidCollection {
        /// Field or primitive being validated.
        field: String,
    },
    /// Duplicate stable ID.
    #[error("duplicate id for {field}: {id}")]
    DuplicateId {
        /// Collection or field being validated.
        field: String,
        /// Duplicate ID.
        id: String,
    },
    /// Reference points to a missing record.
    #[error("missing reference for {field}: {id}")]
    MissingReference {
        /// Field being validated.
        field: String,
        /// Missing ID.
        id: String,
    },
    /// Reference points to an existing record that is not semantically compatible.
    #[error("inconsistent reference for {field}: {id}")]
    InconsistentReference {
        /// Field being validated.
        field: String,
        /// Inconsistent ID.
        id: String,
    },
    /// Operation contract does not match the semantic operation kind.
    #[error("operation contract mismatch for {field}")]
    OperationContractMismatch {
        /// Field being validated.
        field: String,
    },
    /// Point, vector, or direction contains non-finite coordinates.
    #[error("non-finite coordinate for {field}")]
    NonFiniteCoordinate {
        /// Field or primitive being validated.
        field: String,
    },
    /// Direction vector is zero length.
    #[error("zero-length direction for {field}")]
    ZeroDirection {
        /// Field or primitive being validated.
        field: String,
    },
    /// Strand-perfect reconstruction is explicitly outside this grammar.
    #[error("strand-perfect hair reconstruction is unsupported")]
    UnsupportedStrandPerfectReconstruction {
        /// Optional strand count from the rejected request.
        requested_strands: Option<u32>,
    },
}

/// Result type for hair grammar validation.
pub type HairResult<T> = Result<T, HairDiagnostic>;

/// Returns true when a scalar range is finite, ordered, and contains the default.
#[must_use]
pub fn is_valid_scalar_range(range: ScalarRange) -> bool {
    range.is_valid()
}

/// Returns true when a scalar range is valid and fully contained in `[0, 1]`.
#[must_use]
pub fn is_valid_normalized_range(range: ScalarRange) -> bool {
    range.is_valid() && range.min >= 0.0 && range.max <= 1.0
}

/// Returns true when a scalar range is valid and non-negative.
#[must_use]
pub fn is_valid_non_negative_range(range: ScalarRange) -> bool {
    range.is_valid() && range.min >= 0.0
}

/// Deterministic grammar id helper for hair contracts.
#[must_use]
pub fn deterministic_hair_grammar_id(key: impl AsRef<str>) -> CharacterGrammarId {
    CharacterGrammarId(format!(
        "hair:grammar:{}",
        deterministic_suffix("grammar", key.as_ref())
    ))
}

/// Hair control roles with finite default ranges.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairControlRole {
    /// Length multiplier for masses, cards, and clumps.
    Length,
    /// Density multiplier for masses and generated children.
    Density,
    /// Width multiplier for cards.
    Width,
    /// Curl intensity for guides and clumps.
    Curl,
    /// Frizz or flyaway amount.
    Frizz,
    /// Lift away from the scalp.
    Lift,
    /// Influence of a parting region.
    PartingInfluence,
    /// Placement jitter within authored bounds.
    PlacementJitter,
}

/// Required control roles for the initial hair grammar.
pub const REQUIRED_HAIR_CONTROLS: [HairControlRole; 8] = [
    HairControlRole::Length,
    HairControlRole::Density,
    HairControlRole::Width,
    HairControlRole::Curl,
    HairControlRole::Frizz,
    HairControlRole::Lift,
    HairControlRole::PartingInfluence,
    HairControlRole::PlacementJitter,
];

impl HairControlRole {
    /// Stable string used in control identifiers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Density => "density",
            Self::Width => "width",
            Self::Curl => "curl",
            Self::Frizz => "frizz",
            Self::Lift => "lift",
            Self::PartingInfluence => "parting-influence",
            Self::PlacementJitter => "placement-jitter",
        }
    }

    /// Finite, bounded default range for this control role.
    #[must_use]
    pub const fn default_range(self) -> ScalarRange {
        match self {
            Self::Length => ScalarRange {
                min: 0.0,
                max: 2.0,
                default: 1.0,
            },
            Self::Density => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.5,
            },
            Self::Width => ScalarRange {
                min: 0.05,
                max: 2.0,
                default: 1.0,
            },
            Self::Curl => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.25,
            },
            Self::Frizz => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.1,
            },
            Self::Lift => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.25,
            },
            Self::PartingInfluence => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.75,
            },
            Self::PlacementJitter => ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.2,
            },
        }
    }
}

/// Deterministic control id helper for hair control roles.
#[must_use]
pub fn deterministic_hair_control_id(role: HairControlRole) -> CharacterControlId {
    CharacterControlId(format!("hair:control:{}", role.as_str()))
}

/// Compact control contract for hair operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairControlContract {
    /// Stable semantic control id.
    pub id: CharacterControlId,
    /// Role of this control.
    pub role: HairControlRole,
    /// Finite authored range.
    pub range: ScalarRange,
}

impl HairControlContract {
    /// Build the default contract for a control role.
    #[must_use]
    pub fn default_for(role: HairControlRole) -> Self {
        Self {
            id: deterministic_hair_control_id(role),
            role,
            range: role.default_range(),
        }
    }

    /// Validate the control contract.
    pub fn validate(&self) -> HairResult<()> {
        require_character_control_id("control.id", &self.id)?;
        if self.id != deterministic_hair_control_id(self.role) {
            return Err(HairDiagnostic::InconsistentReference {
                field: "control.id".to_owned(),
                id: self.id.0.clone(),
            });
        }
        require_scalar_range("control.range", self.range)
    }

    /// Returns true when the control id and range are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Default hair controls required by operation contracts.
#[must_use]
pub fn default_hair_controls() -> Vec<HairControlContract> {
    [
        HairControlRole::Length,
        HairControlRole::Density,
        HairControlRole::Width,
        HairControlRole::Curl,
        HairControlRole::Frizz,
        HairControlRole::Lift,
        HairControlRole::PartingInfluence,
        HairControlRole::PlacementJitter,
    ]
    .into_iter()
    .map(HairControlContract::default_for)
    .collect()
}

/// Supported compact hair operation kinds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairOperationKind {
    /// Define or edit a semantic hair mass.
    DefineMass,
    /// Define or edit a guide curve.
    DefineCurve,
    /// Define or edit a card strip.
    DefineCard,
    /// Define or edit a clump group.
    DefineClump,
    /// Define or edit a parting region.
    DefinePartingRegion,
    /// Place bounded procedural cards or clumps.
    PlaceProceduralHair,
}

impl HairOperationKind {
    /// Stable operation key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DefineMass => "define-mass",
            Self::DefineCurve => "define-curve",
            Self::DefineCard => "define-card",
            Self::DefineClump => "define-clump",
            Self::DefinePartingRegion => "define-parting-region",
            Self::PlaceProceduralHair => "place-procedural-hair",
        }
    }
}

/// Required operation set for interoperable hair grammar implementations.
pub const REQUIRED_HAIR_OPERATIONS: [HairOperationKind; 6] = [
    HairOperationKind::DefineMass,
    HairOperationKind::DefineCurve,
    HairOperationKind::DefineCard,
    HairOperationKind::DefineClump,
    HairOperationKind::DefinePartingRegion,
    HairOperationKind::PlaceProceduralHair,
];

/// Compact operation contract. It names the operation, primitive inputs and
/// output, required controls, and any output bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HairOperationContract {
    /// Stable operation id.
    pub id: HairElementId,
    /// Supported operation kind.
    pub kind: HairOperationKind,
    /// Primitive inputs required by the operation.
    pub inputs: Vec<HairPrimitiveKind>,
    /// Primitive output produced or edited by the operation.
    pub output: HairPrimitiveKind,
    /// Required semantic control ids.
    pub controls: Vec<CharacterControlId>,
    /// Optional maximum number of generated output instances.
    pub max_output_instances: Option<u32>,
}

impl HairOperationContract {
    /// Build the default compact contract for an operation kind.
    #[must_use]
    pub fn default_for(kind: HairOperationKind) -> Self {
        let controls = match kind {
            HairOperationKind::DefineMass => vec![
                deterministic_hair_control_id(HairControlRole::Length),
                deterministic_hair_control_id(HairControlRole::Density),
            ],
            HairOperationKind::DefineCurve => vec![
                deterministic_hair_control_id(HairControlRole::Curl),
                deterministic_hair_control_id(HairControlRole::Lift),
            ],
            HairOperationKind::DefineCard => vec![
                deterministic_hair_control_id(HairControlRole::Width),
                deterministic_hair_control_id(HairControlRole::Lift),
            ],
            HairOperationKind::DefineClump => vec![
                deterministic_hair_control_id(HairControlRole::Density),
                deterministic_hair_control_id(HairControlRole::Frizz),
            ],
            HairOperationKind::DefinePartingRegion => {
                vec![deterministic_hair_control_id(
                    HairControlRole::PartingInfluence,
                )]
            }
            HairOperationKind::PlaceProceduralHair => vec![
                deterministic_hair_control_id(HairControlRole::Length),
                deterministic_hair_control_id(HairControlRole::Density),
                deterministic_hair_control_id(HairControlRole::Width),
                deterministic_hair_control_id(HairControlRole::PlacementJitter),
            ],
        };
        let (inputs, output, max_output_instances) = match kind {
            HairOperationKind::DefineMass => (vec![], HairPrimitiveKind::Mass, None),
            HairOperationKind::DefineCurve => (
                vec![HairPrimitiveKind::Mass],
                HairPrimitiveKind::Curve,
                None,
            ),
            HairOperationKind::DefineCard => (
                vec![HairPrimitiveKind::Mass, HairPrimitiveKind::Curve],
                HairPrimitiveKind::Card,
                None,
            ),
            HairOperationKind::DefineClump => (
                vec![HairPrimitiveKind::Mass, HairPrimitiveKind::Curve],
                HairPrimitiveKind::Clump,
                None,
            ),
            HairOperationKind::DefinePartingRegion => (
                vec![HairPrimitiveKind::Mass, HairPrimitiveKind::Curve],
                HairPrimitiveKind::PartingRegion,
                None,
            ),
            HairOperationKind::PlaceProceduralHair => (
                vec![
                    HairPrimitiveKind::Mass,
                    HairPrimitiveKind::Curve,
                    HairPrimitiveKind::Card,
                    HairPrimitiveKind::Clump,
                    HairPrimitiveKind::PartingRegion,
                ],
                HairPrimitiveKind::Card,
                Some(MAX_PROCEDURAL_HAIR_INSTANCES),
            ),
        };

        Self {
            id: HairElementId::deterministic(HairElementKind::Operation, kind.as_str()),
            kind,
            inputs,
            output,
            controls,
            max_output_instances,
        }
    }

    /// Validate this operation contract.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("operation.id", &self.id, HairElementKind::Operation)?;
        let expected = HairOperationContract::default_for(self.kind);
        if self.id != expected.id
            || self.inputs != expected.inputs
            || self.output != expected.output
            || self.controls != expected.controls
            || self.max_output_instances != expected.max_output_instances
        {
            return Err(HairDiagnostic::OperationContractMismatch {
                field: format!("operation.{:?}", self.kind),
            });
        }
        if self.controls.is_empty() {
            return Err(HairDiagnostic::InvalidCollection {
                field: "operation.controls".to_owned(),
            });
        }
        for control in &self.controls {
            require_character_control_id("operation.controls", control)?;
        }
        if let Some(max_output_instances) = self.max_output_instances
            && max_output_instances > MAX_PROCEDURAL_HAIR_INSTANCES
        {
            return Err(HairDiagnostic::UnboundedPlacement {
                field: "operation.max_output_instances".to_owned(),
                requested_max: max_output_instances,
                allowed_max: MAX_PROCEDURAL_HAIR_INSTANCES,
            });
        }
        Ok(())
    }

    /// Returns true when this operation contract is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Default hair operation contracts.
#[must_use]
pub fn default_hair_operation_contracts() -> Vec<HairOperationContract> {
    REQUIRED_HAIR_OPERATIONS
        .into_iter()
        .map(HairOperationContract::default_for)
        .collect()
}

/// Semantic mass categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairMassKind {
    /// Crown/top hair volume.
    Crown,
    /// Forehead/fringe mass.
    Fringe,
    /// Side mass.
    Side,
    /// Nape/back mass.
    Nape,
    /// Locally defined mass.
    Custom,
}

/// Bounded hair mass contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairMass {
    /// Stable mass id.
    pub id: HairElementId,
    /// Semantic mass kind.
    pub kind: HairMassKind,
    /// Scalp region this mass grows from.
    pub scalp_region: CharacterRegionId,
    /// Character landmarks that anchor the mass.
    pub anchors: Vec<CharacterLandmarkId>,
    /// Normalized silhouette contribution.
    pub silhouette: ScalarRange,
    /// Length multiplier.
    pub length: ScalarRange,
    /// Density multiplier.
    pub density: ScalarRange,
    /// Lift away from the scalp.
    pub lift: ScalarRange,
}

impl HairMass {
    /// Validate this hair mass.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("mass.id", &self.id, HairElementKind::Mass)?;
        require_character_region_id("mass.scalp_region", &self.scalp_region)?;
        if self.anchors.is_empty() {
            return Err(HairDiagnostic::InvalidCollection {
                field: "mass.anchors".to_owned(),
            });
        }
        for anchor in &self.anchors {
            require_character_landmark_id("mass.anchors", anchor)?;
        }
        require_normalized_range("mass.silhouette", self.silhouette)?;
        require_non_negative_range("mass.length", self.length)?;
        require_normalized_range("mass.density", self.density)?;
        require_normalized_range("mass.lift", self.lift)
    }

    /// Returns true when this hair mass is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Role of a semantic guide curve.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairCurveRole {
    /// Main flow direction.
    Flow,
    /// Outer silhouette control.
    Silhouette,
    /// Parting or split control.
    Parting,
}

/// Semantic hair guide curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairGuideCurve {
    /// Stable curve id.
    pub id: HairElementId,
    /// Owning mass id.
    pub mass_id: HairElementId,
    /// Curve role.
    pub role: HairCurveRole,
    /// Root point in normalized local scalp space.
    pub root: [f32; 3],
    /// Bounded semantic control points in local character space.
    pub control_points: Vec<[f32; 3]>,
    /// Curve tension range.
    pub tension: ScalarRange,
    /// Curl influence range.
    pub curl: ScalarRange,
}

impl HairGuideCurve {
    /// Validate this guide curve.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("curve.id", &self.id, HairElementKind::Curve)?;
        require_hair_id_kind("curve.mass_id", &self.mass_id, HairElementKind::Mass)?;
        require_normalized_point3("curve.root", self.root)?;
        if self.control_points.len() < MIN_CURVE_CONTROL_POINTS
            || self.control_points.len() > MAX_CURVE_CONTROL_POINTS
        {
            return Err(HairDiagnostic::InvalidCollection {
                field: "curve.control_points".to_owned(),
            });
        }
        for point in &self.control_points {
            require_finite_point3("curve.control_points", *point)?;
        }
        require_normalized_range("curve.tension", self.tension)?;
        require_normalized_range("curve.curl", self.curl)
    }

    /// Returns true when this guide curve is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Side binding for a hair card.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairCardSide {
    /// Centered around the guide.
    Center,
    /// Left side of the guide direction.
    Left,
    /// Right side of the guide direction.
    Right,
}

/// Renderable hair card strip controlled by a semantic guide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairCard {
    /// Stable card id.
    pub id: HairElementId,
    /// Owning mass id.
    pub mass_id: HairElementId,
    /// Guide curve id.
    pub curve_id: HairElementId,
    /// Card side relative to the guide.
    pub side: HairCardSide,
    /// Width multiplier.
    pub width: ScalarRange,
    /// Length multiplier.
    pub length_scale: ScalarRange,
    /// Tip taper.
    pub taper: ScalarRange,
    /// Twist in radians.
    pub twist_radians: ScalarRange,
    /// Bounded card subdivisions.
    pub segments: CountRange,
}

impl HairCard {
    /// Validate this hair card.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("card.id", &self.id, HairElementKind::Card)?;
        require_hair_id_kind("card.mass_id", &self.mass_id, HairElementKind::Mass)?;
        require_hair_id_kind("card.curve_id", &self.curve_id, HairElementKind::Curve)?;
        require_non_negative_range("card.width", self.width)?;
        require_non_negative_range("card.length_scale", self.length_scale)?;
        require_normalized_range("card.taper", self.taper)?;
        require_scalar_range("card.twist_radians", self.twist_radians)?;
        if !self.segments.is_bounded_by(MAX_CARD_SEGMENTS) || self.segments.min == 0 {
            return Err(HairDiagnostic::InvalidCountRange {
                field: "card.segments".to_owned(),
            });
        }
        Ok(())
    }

    /// Returns true when this hair card is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Procedural clump grouping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairClump {
    /// Stable clump id.
    pub id: HairElementId,
    /// Owning mass id.
    pub mass_id: HairElementId,
    /// Guide curves controlling clump flow.
    pub curve_ids: Vec<HairElementId>,
    /// Child-card count per clump.
    pub child_count: CountRange,
    /// Root radius in local character units.
    pub radius: ScalarRange,
    /// Density multiplier.
    pub density: ScalarRange,
    /// Tip taper.
    pub taper: ScalarRange,
    /// Flyaway/frizz amount.
    pub flyaway: ScalarRange,
}

impl HairClump {
    /// Validate this hair clump.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("clump.id", &self.id, HairElementKind::Clump)?;
        require_hair_id_kind("clump.mass_id", &self.mass_id, HairElementKind::Mass)?;
        if self.curve_ids.is_empty() {
            return Err(HairDiagnostic::InvalidCollection {
                field: "clump.curve_ids".to_owned(),
            });
        }
        for curve_id in &self.curve_ids {
            require_hair_id_kind("clump.curve_ids", curve_id, HairElementKind::Curve)?;
        }
        if !self
            .child_count
            .is_bounded_by(MAX_PROCEDURAL_HAIR_INSTANCES)
            || self.child_count.min == 0
        {
            return Err(HairDiagnostic::UnboundedPlacement {
                field: "clump.child_count".to_owned(),
                requested_max: self.child_count.max,
                allowed_max: MAX_PROCEDURAL_HAIR_INSTANCES,
            });
        }
        require_non_negative_range("clump.radius", self.radius)?;
        require_normalized_range("clump.density", self.density)?;
        require_normalized_range("clump.taper", self.taper)?;
        require_normalized_range("clump.flyaway", self.flyaway)
    }

    /// Returns true when this hair clump is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Parting side semantics.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairPartSide {
    /// Center part.
    Center,
    /// Left part.
    Left,
    /// Right part.
    Right,
    /// Radial/crown part.
    Radial,
    /// Locally defined part.
    Custom,
}

/// Region that separates flow fields or masks placement across a part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairPartingRegion {
    /// Stable parting region id.
    pub id: HairElementId,
    /// Character scalp region containing the part.
    pub scalp_region: CharacterRegionId,
    /// Parting side.
    pub side: HairPartSide,
    /// Optional centerline guide curve.
    pub center_curve_id: Option<HairElementId>,
    /// Unit-ish 2D local direction.
    pub direction: [f32; 2],
    /// Influence strength.
    pub influence: ScalarRange,
    /// Feather width around the part.
    pub feather_width: ScalarRange,
}

impl HairPartingRegion {
    /// Validate this parting region.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("parting.id", &self.id, HairElementKind::PartingRegion)?;
        require_character_region_id("parting.scalp_region", &self.scalp_region)?;
        if let Some(center_curve_id) = &self.center_curve_id {
            require_hair_id_kind(
                "parting.center_curve_id",
                center_curve_id,
                HairElementKind::Curve,
            )?;
        }
        require_finite_direction2("parting.direction", self.direction)?;
        require_normalized_range("parting.influence", self.influence)?;
        require_non_negative_range("parting.feather_width", self.feather_width)
    }

    /// Returns true when this parting region is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Deterministic seed for procedural hair placement.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HairPlacementSeed(pub u64);

impl HairPlacementSeed {
    /// Build a deterministic seed from a semantic key.
    #[must_use]
    pub fn deterministic(key: impl AsRef<str>) -> Self {
        let hash = deterministic_hash("placement-seed", key.as_ref());
        let bytes = hash.as_bytes();
        Self(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

/// Bounded procedural placement strategy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairPlacementStrategy {
    /// Place roots in a bounded scalp UV rectangle.
    ScalpUv,
    /// Place along one or more guide curves.
    AlongCurve,
    /// Place while masking or splitting around a parting region.
    PartingAware,
}

/// Bounded procedural placement controls for cards and clumps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairPlacementBounds {
    /// Stable placement id.
    pub id: HairElementId,
    /// Placement strategy.
    pub strategy: HairPlacementStrategy,
    /// Owning mass id.
    pub mass_id: HairElementId,
    /// Character scalp region containing roots.
    pub scalp_region: CharacterRegionId,
    /// Guide curves required by curve-following placement strategies.
    pub curve_ids: Vec<HairElementId>,
    /// Parting region required by part-aware placement strategies.
    pub parting_region_id: Option<HairElementId>,
    /// Deterministic placement seed.
    pub seed: HairPlacementSeed,
    /// Normalized root U range.
    pub root_u: ScalarRange,
    /// Normalized root V range.
    pub root_v: ScalarRange,
    /// Bounded generated instance count.
    pub instance_count: CountRange,
    /// Minimum spacing in normalized scalp UV units.
    pub min_spacing: ScalarRange,
    /// Per-instance length scale.
    pub length_scale: ScalarRange,
    /// Per-instance width scale.
    pub width_scale: ScalarRange,
    /// Jitter applied within the bounded region.
    pub jitter: ScalarRange,
    /// Bias toward clump centers.
    pub clump_bias: ScalarRange,
}

impl HairPlacementBounds {
    /// Build a bounded scalp-UV placement contract with conservative defaults.
    #[must_use]
    pub fn scalp_uv(
        key: impl AsRef<str>,
        mass_id: HairElementId,
        scalp_region: CharacterRegionId,
    ) -> Self {
        let key = key.as_ref();
        Self {
            id: HairElementId::deterministic(HairElementKind::Placement, key),
            strategy: HairPlacementStrategy::ScalpUv,
            mass_id,
            scalp_region,
            curve_ids: Vec::new(),
            parting_region_id: None,
            seed: HairPlacementSeed::deterministic(key),
            root_u: ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.5,
            },
            root_v: ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.5,
            },
            instance_count: CountRange::new(1, 512, 128),
            min_spacing: ScalarRange {
                min: 0.0,
                max: 0.25,
                default: 0.02,
            },
            length_scale: ScalarRange {
                min: 0.0,
                max: 2.0,
                default: 1.0,
            },
            width_scale: ScalarRange {
                min: 0.05,
                max: 2.0,
                default: 1.0,
            },
            jitter: ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.2,
            },
            clump_bias: ScalarRange {
                min: 0.0,
                max: 1.0,
                default: 0.5,
            },
        }
    }

    /// Validate finite controls and enforce explicit procedural bounds.
    pub fn validate(&self) -> HairResult<()> {
        require_hair_id_kind("placement.id", &self.id, HairElementKind::Placement)?;
        require_hair_id_kind("placement.mass_id", &self.mass_id, HairElementKind::Mass)?;
        require_character_region_id("placement.scalp_region", &self.scalp_region)?;
        for curve_id in &self.curve_ids {
            require_hair_id_kind("placement.curve_ids", curve_id, HairElementKind::Curve)?;
        }
        if let Some(parting_region_id) = &self.parting_region_id {
            require_hair_id_kind(
                "placement.parting_region_id",
                parting_region_id,
                HairElementKind::PartingRegion,
            )?;
        }
        match self.strategy {
            HairPlacementStrategy::ScalpUv => {
                if !self.curve_ids.is_empty() || self.parting_region_id.is_some() {
                    return Err(HairDiagnostic::InconsistentReference {
                        field: "placement.strategy".to_owned(),
                        id: self.id.0.clone(),
                    });
                }
            }
            HairPlacementStrategy::AlongCurve => {
                if self.curve_ids.is_empty() {
                    return Err(HairDiagnostic::InvalidCollection {
                        field: "placement.curve_ids".to_owned(),
                    });
                }
                if self.parting_region_id.is_some() {
                    return Err(HairDiagnostic::InconsistentReference {
                        field: "placement.parting_region_id".to_owned(),
                        id: self.id.0.clone(),
                    });
                }
            }
            HairPlacementStrategy::PartingAware => {
                if self.parting_region_id.is_none() {
                    return Err(HairDiagnostic::MissingReference {
                        field: "placement.parting_region_id".to_owned(),
                        id: self.id.0.clone(),
                    });
                }
                if !self.curve_ids.is_empty() {
                    return Err(HairDiagnostic::InconsistentReference {
                        field: "placement.curve_ids".to_owned(),
                        id: self.id.0.clone(),
                    });
                }
            }
        }
        require_normalized_range("placement.root_u", self.root_u)?;
        require_normalized_range("placement.root_v", self.root_v)?;
        if !self.instance_count.is_valid() || self.instance_count.min == 0 {
            return Err(HairDiagnostic::InvalidCountRange {
                field: "placement.instance_count".to_owned(),
            });
        }
        if !self
            .instance_count
            .is_bounded_by(MAX_PROCEDURAL_HAIR_INSTANCES)
        {
            return Err(HairDiagnostic::UnboundedPlacement {
                field: "placement.instance_count".to_owned(),
                requested_max: self.instance_count.max,
                allowed_max: MAX_PROCEDURAL_HAIR_INSTANCES,
            });
        }
        require_non_negative_range("placement.min_spacing", self.min_spacing)?;
        require_non_negative_range("placement.length_scale", self.length_scale)?;
        require_non_negative_range("placement.width_scale", self.width_scale)?;
        require_normalized_range("placement.jitter", self.jitter)?;
        require_normalized_range("placement.clump_bias", self.clump_bias)
    }

    /// Returns true when this placement is finite and bounded.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Versioned semantic hair grammar document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HairGrammar {
    /// Hair grammar schema version.
    pub schema_version: u32,
    /// Stable hair grammar id.
    pub id: CharacterGrammarId,
    /// Control contracts used by operations.
    pub controls: Vec<HairControlContract>,
    /// Hair masses.
    pub masses: Vec<HairMass>,
    /// Semantic guide curves.
    pub curves: Vec<HairGuideCurve>,
    /// Renderable card strips.
    pub cards: Vec<HairCard>,
    /// Procedural clump groups.
    pub clumps: Vec<HairClump>,
    /// Parting regions.
    pub parting_regions: Vec<HairPartingRegion>,
    /// Bounded placement contracts.
    pub placements: Vec<HairPlacementBounds>,
    /// Compact operation contracts.
    pub operations: Vec<HairOperationContract>,
}

impl HairGrammar {
    /// Build an empty, valid grammar shell with required controls and operations.
    #[must_use]
    pub fn empty(key: impl AsRef<str>) -> Self {
        Self {
            schema_version: HAIR_GRAMMAR_SCHEMA_VERSION,
            id: deterministic_hair_grammar_id(key),
            controls: default_hair_controls(),
            masses: Vec::new(),
            curves: Vec::new(),
            cards: Vec::new(),
            clumps: Vec::new(),
            parting_regions: Vec::new(),
            placements: Vec::new(),
            operations: default_hair_operation_contracts(),
        }
    }

    /// Validate all contained contracts.
    pub fn validate(&self) -> HairResult<()> {
        if self.schema_version != HAIR_GRAMMAR_SCHEMA_VERSION {
            return Err(HairDiagnostic::InvalidSchemaVersion {
                schema_version: self.schema_version,
            });
        }
        require_character_grammar_id("grammar.id", &self.id)?;
        ensure_required_controls(&self.controls)?;
        ensure_required_operations(&self.operations)?;
        let control_ids = unique_character_control_ids("grammar.controls", &self.controls)?;
        let mass_ids = unique_hair_ids("grammar.masses", self.masses.iter().map(|mass| &mass.id))?;
        let curve_ids =
            unique_hair_ids("grammar.curves", self.curves.iter().map(|curve| &curve.id))?;
        let curve_mass_by_id = self
            .curves
            .iter()
            .map(|curve| (curve.id.0.clone(), curve.mass_id.0.clone()))
            .collect::<BTreeMap<_, _>>();
        let card_ids = unique_hair_ids("grammar.cards", self.cards.iter().map(|card| &card.id))?;
        let clump_ids =
            unique_hair_ids("grammar.clumps", self.clumps.iter().map(|clump| &clump.id))?;
        let parting_ids = unique_hair_ids(
            "grammar.parting_regions",
            self.parting_regions.iter().map(|parting| &parting.id),
        )?;
        let placement_ids = unique_hair_ids(
            "grammar.placements",
            self.placements.iter().map(|placement| &placement.id),
        )?;
        let operation_ids = unique_hair_ids(
            "grammar.operations",
            self.operations.iter().map(|operation| &operation.id),
        )?;
        drop((card_ids, clump_ids, placement_ids, operation_ids));

        for control in &self.controls {
            control.validate()?;
        }
        for mass in &self.masses {
            mass.validate()?;
        }
        for curve in &self.curves {
            curve.validate()?;
            require_hair_reference("curve.mass_id", &curve.mass_id, &mass_ids)?;
        }
        for card in &self.cards {
            card.validate()?;
            require_hair_reference("card.mass_id", &card.mass_id, &mass_ids)?;
            require_hair_reference("card.curve_id", &card.curve_id, &curve_ids)?;
            require_curve_mass_reference(
                "card.curve_id",
                &card.curve_id,
                &card.mass_id,
                &curve_mass_by_id,
            )?;
        }
        for clump in &self.clumps {
            clump.validate()?;
            require_hair_reference("clump.mass_id", &clump.mass_id, &mass_ids)?;
            for curve_id in &clump.curve_ids {
                require_hair_reference("clump.curve_ids", curve_id, &curve_ids)?;
                require_curve_mass_reference(
                    "clump.curve_ids",
                    curve_id,
                    &clump.mass_id,
                    &curve_mass_by_id,
                )?;
            }
        }
        for parting_region in &self.parting_regions {
            parting_region.validate()?;
            if let Some(center_curve_id) = &parting_region.center_curve_id {
                require_hair_reference("parting.center_curve_id", center_curve_id, &curve_ids)?;
            }
        }
        for placement in &self.placements {
            placement.validate()?;
            require_hair_reference("placement.mass_id", &placement.mass_id, &mass_ids)?;
            for curve_id in &placement.curve_ids {
                require_hair_reference("placement.curve_ids", curve_id, &curve_ids)?;
                require_curve_mass_reference(
                    "placement.curve_ids",
                    curve_id,
                    &placement.mass_id,
                    &curve_mass_by_id,
                )?;
            }
            if let Some(parting_region_id) = &placement.parting_region_id {
                require_hair_reference(
                    "placement.parting_region_id",
                    parting_region_id,
                    &parting_ids,
                )?;
            }
        }
        for operation in &self.operations {
            operation.validate()?;
            for control in &operation.controls {
                if !control_ids.contains(&control.0) {
                    return Err(HairDiagnostic::MissingReference {
                        field: "operation.controls".to_owned(),
                        id: control.0.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Returns true when every contained contract is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Requested reconstruction target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HairReconstructionRequest {
    /// Semantic masses, guides, cards, clumps, parting, and bounded placement.
    SemanticGrammar,
    /// Unsupported arbitrary strand-perfect reconstruction.
    StrandPerfect {
        /// Optional strand count from the request.
        requested_strands: Option<u32>,
    },
}

impl HairReconstructionRequest {
    /// Validate whether the requested reconstruction target is supported.
    pub fn validate_supported(&self) -> HairResult<()> {
        match self {
            Self::SemanticGrammar => Ok(()),
            Self::StrandPerfect { requested_strands } => {
                Err(HairDiagnostic::UnsupportedStrandPerfectReconstruction {
                    requested_strands: *requested_strands,
                })
            }
        }
    }
}

/// Validate whether a reconstruction request is supported by the hair grammar.
pub fn validate_reconstruction_request(request: &HairReconstructionRequest) -> HairResult<()> {
    request.validate_supported()
}

fn deterministic_suffix(kind: &str, key: &str) -> String {
    let hash = deterministic_hash(kind, key);
    hash.to_hex().to_string()[..16].to_owned()
}

fn deterministic_hash(kind: &str, key: &str) -> blake3::Hash {
    let canonical_key = key
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let mut hasher = blake3::Hasher::new();
    hasher.update(HAIR_GRAMMAR_NAMESPACE.as_bytes());
    hasher.update(b"\0");
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical_key.as_bytes());
    hasher.finalize()
}

fn ensure_required_controls(controls: &[HairControlContract]) -> HairResult<()> {
    if controls.is_empty() {
        return Err(HairDiagnostic::InvalidCollection {
            field: "grammar.controls".to_owned(),
        });
    }
    let ids = controls
        .iter()
        .map(|control| control.id.0.as_str())
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_HAIR_CONTROLS {
        let required_id = deterministic_hair_control_id(required);
        if !ids.contains(required_id.0.as_str()) {
            return Err(HairDiagnostic::MissingReference {
                field: "grammar.controls".to_owned(),
                id: required_id.0,
            });
        }
    }
    Ok(())
}

fn ensure_required_operations(operations: &[HairOperationContract]) -> HairResult<()> {
    if operations.is_empty() {
        return Err(HairDiagnostic::InvalidCollection {
            field: "grammar.operations".to_owned(),
        });
    }
    let kinds = operations
        .iter()
        .map(|operation| operation.kind)
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_HAIR_OPERATIONS {
        if !kinds.contains(&required) {
            return Err(HairDiagnostic::MissingReference {
                field: "grammar.operations".to_owned(),
                id: required.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn unique_character_control_ids(
    field: &'static str,
    controls: &[HairControlContract],
) -> HairResult<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for control in controls {
        if !ids.insert(control.id.0.clone()) {
            return Err(HairDiagnostic::DuplicateId {
                field: field.to_owned(),
                id: control.id.0.clone(),
            });
        }
    }
    Ok(ids)
}

fn unique_hair_ids<'a>(
    field: &'static str,
    ids: impl Iterator<Item = &'a HairElementId>,
) -> HairResult<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.0.clone()) {
            return Err(HairDiagnostic::DuplicateId {
                field: field.to_owned(),
                id: id.0.clone(),
            });
        }
    }
    Ok(seen)
}

fn require_hair_reference(
    field: &'static str,
    id: &HairElementId,
    ids: &BTreeSet<String>,
) -> HairResult<()> {
    if ids.contains(&id.0) {
        Ok(())
    } else {
        Err(HairDiagnostic::MissingReference {
            field: field.to_owned(),
            id: id.0.clone(),
        })
    }
}

fn require_curve_mass_reference(
    field: &'static str,
    curve_id: &HairElementId,
    mass_id: &HairElementId,
    curve_mass_by_id: &BTreeMap<String, String>,
) -> HairResult<()> {
    if curve_mass_by_id.get(&curve_id.0) == Some(&mass_id.0) {
        Ok(())
    } else {
        Err(HairDiagnostic::InconsistentReference {
            field: field.to_owned(),
            id: curve_id.0.clone(),
        })
    }
}

fn require_hair_id(field: &'static str, id: &HairElementId) -> HairResult<()> {
    if id.is_valid() {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidHairId {
            field: field.to_owned(),
        })
    }
}

fn require_hair_id_kind(
    field: &'static str,
    id: &HairElementId,
    kind: HairElementKind,
) -> HairResult<()> {
    require_hair_id(field, id)?;
    if id.kind() == Some(kind) {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidHairId {
            field: field.to_owned(),
        })
    }
}

fn require_character_grammar_id(field: &'static str, id: &CharacterGrammarId) -> HairResult<()> {
    if !id.0.trim().is_empty() {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidCharacterId {
            field: field.to_owned(),
        })
    }
}

fn require_character_control_id(field: &'static str, id: &CharacterControlId) -> HairResult<()> {
    if !id.0.trim().is_empty() {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidCharacterId {
            field: field.to_owned(),
        })
    }
}

fn require_character_region_id(field: &'static str, id: &CharacterRegionId) -> HairResult<()> {
    if !id.0.trim().is_empty() {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidCharacterId {
            field: field.to_owned(),
        })
    }
}

fn require_character_landmark_id(field: &'static str, id: &CharacterLandmarkId) -> HairResult<()> {
    if !id.0.trim().is_empty() {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidCharacterId {
            field: field.to_owned(),
        })
    }
}

fn require_scalar_range(field: &'static str, range: ScalarRange) -> HairResult<()> {
    if is_valid_scalar_range(range) {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidScalarRange {
            field: field.to_owned(),
        })
    }
}

fn require_normalized_range(field: &'static str, range: ScalarRange) -> HairResult<()> {
    if !is_valid_scalar_range(range) {
        return Err(HairDiagnostic::InvalidScalarRange {
            field: field.to_owned(),
        });
    }
    if is_valid_normalized_range(range) {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidNormalizedRange {
            field: field.to_owned(),
        })
    }
}

fn require_non_negative_range(field: &'static str, range: ScalarRange) -> HairResult<()> {
    if !is_valid_scalar_range(range) {
        return Err(HairDiagnostic::InvalidScalarRange {
            field: field.to_owned(),
        });
    }
    if is_valid_non_negative_range(range) {
        Ok(())
    } else {
        Err(HairDiagnostic::InvalidScalarRange {
            field: field.to_owned(),
        })
    }
}

fn require_finite_point3(field: &'static str, point: [f32; 3]) -> HairResult<()> {
    if point.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(HairDiagnostic::NonFiniteCoordinate {
            field: field.to_owned(),
        })
    }
}

fn require_normalized_point3(field: &'static str, point: [f32; 3]) -> HairResult<()> {
    if point
        .iter()
        .all(|component| component.is_finite() && (0.0..=1.0).contains(component))
    {
        Ok(())
    } else {
        Err(HairDiagnostic::NonFiniteCoordinate {
            field: field.to_owned(),
        })
    }
}

fn require_finite_direction2(field: &'static str, direction: [f32; 2]) -> HairResult<()> {
    if !direction.iter().all(|component| component.is_finite()) {
        return Err(HairDiagnostic::NonFiniteCoordinate {
            field: field.to_owned(),
        });
    }
    let length_squared = direction[0] * direction[0] + direction[1] * direction[1];
    if length_squared <= f32::EPSILON {
        return Err(HairDiagnostic::ZeroDirection {
            field: field.to_owned(),
        });
    }
    if (length_squared.sqrt() - 1.0).abs() <= 0.0001 {
        Ok(())
    } else {
        Err(HairDiagnostic::NonFiniteCoordinate {
            field: field.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(min: f32, max: f32, default: f32) -> ScalarRange {
        ScalarRange { min, max, default }
    }

    fn mass_id() -> HairElementId {
        HairElementId::deterministic(HairElementKind::Mass, "crown")
    }

    fn curve_id() -> HairElementId {
        HairElementId::deterministic(HairElementKind::Curve, "crown-flow")
    }

    fn valid_mass() -> HairMass {
        HairMass {
            id: mass_id(),
            kind: HairMassKind::Crown,
            scalp_region: CharacterRegionId("scalp.crown".to_owned()),
            anchors: vec![CharacterLandmarkId("head.crown".to_owned())],
            silhouette: scalar(0.0, 1.0, 0.7),
            length: scalar(0.0, 2.0, 1.0),
            density: scalar(0.0, 1.0, 0.8),
            lift: scalar(0.0, 1.0, 0.2),
        }
    }

    fn valid_curve() -> HairGuideCurve {
        HairGuideCurve {
            id: curve_id(),
            mass_id: mass_id(),
            role: HairCurveRole::Flow,
            root: [0.5, 0.4, 0.0],
            control_points: vec![[0.5, 0.6, 0.1], [0.45, 0.9, 0.2]],
            tension: scalar(0.0, 1.0, 0.5),
            curl: scalar(0.0, 1.0, 0.25),
        }
    }

    fn valid_card() -> HairCard {
        HairCard {
            id: HairElementId::deterministic(HairElementKind::Card, "crown-card"),
            mass_id: mass_id(),
            curve_id: curve_id(),
            side: HairCardSide::Center,
            width: scalar(0.05, 2.0, 1.0),
            length_scale: scalar(0.0, 2.0, 1.0),
            taper: scalar(0.0, 1.0, 0.8),
            twist_radians: scalar(-1.0, 1.0, 0.0),
            segments: CountRange::new(1, 8, 4),
        }
    }

    fn valid_clump() -> HairClump {
        HairClump {
            id: HairElementId::deterministic(HairElementKind::Clump, "crown-clump"),
            mass_id: mass_id(),
            curve_ids: vec![curve_id()],
            child_count: CountRange::new(1, 64, 12),
            radius: scalar(0.0, 0.5, 0.1),
            density: scalar(0.0, 1.0, 0.6),
            taper: scalar(0.0, 1.0, 0.7),
            flyaway: scalar(0.0, 1.0, 0.1),
        }
    }

    fn valid_parting() -> HairPartingRegion {
        HairPartingRegion {
            id: HairElementId::deterministic(HairElementKind::PartingRegion, "center-part"),
            scalp_region: CharacterRegionId("scalp.crown".to_owned()),
            side: HairPartSide::Center,
            center_curve_id: Some(curve_id()),
            direction: [1.0, 0.0],
            influence: scalar(0.0, 1.0, 0.9),
            feather_width: scalar(0.0, 0.25, 0.05),
        }
    }

    #[test]
    fn required_primitives_and_operations_are_declared() {
        assert_eq!(
            REQUIRED_HAIR_PRIMITIVES,
            [
                HairPrimitiveKind::Mass,
                HairPrimitiveKind::Curve,
                HairPrimitiveKind::Card,
                HairPrimitiveKind::Clump,
                HairPrimitiveKind::PartingRegion,
            ]
        );
        assert_eq!(
            REQUIRED_HAIR_OPERATIONS,
            [
                HairOperationKind::DefineMass,
                HairOperationKind::DefineCurve,
                HairOperationKind::DefineCard,
                HairOperationKind::DefineClump,
                HairOperationKind::DefinePartingRegion,
                HairOperationKind::PlaceProceduralHair,
            ]
        );

        let first = HairElementId::deterministic(HairElementKind::Mass, "Crown Mass");
        let second = HairElementId::deterministic(HairElementKind::Mass, " crown   mass ");
        let curve = HairElementId::deterministic(HairElementKind::Curve, "Crown Mass");
        assert_eq!(first, second);
        assert_ne!(first, curve);
        assert!(first.is_valid());
        assert!(!HairElementId("hair:mass:ABCDEF0123456789".to_owned()).is_valid());
        assert!(!HairElementId("hair:mass:abc".to_owned()).is_valid());
        assert!(!HairElementId("hair:unknown:abcdef0123456789".to_owned()).is_valid());

        let operations = default_hair_operation_contracts();
        assert_eq!(operations.len(), REQUIRED_HAIR_OPERATIONS.len());
        assert!(operations.iter().all(HairOperationContract::is_valid));
    }

    #[test]
    fn primitive_contracts_validate() {
        let mass = valid_mass();
        let curve = valid_curve();
        let card = valid_card();
        let clump = valid_clump();
        let parting = valid_parting();

        assert!(mass.is_valid());
        assert!(curve.is_valid());
        assert!(card.is_valid());
        assert!(clump.is_valid());
        assert!(parting.is_valid());
    }

    #[test]
    fn primitive_validation_rejects_wrong_kinds_and_unbounded_geometry() {
        let mut mass = valid_mass();
        mass.id = curve_id();
        assert!(matches!(
            mass.validate(),
            Err(HairDiagnostic::InvalidHairId { field }) if field == "mass.id"
        ));

        let mut curve = valid_curve();
        curve.mass_id = curve.id.clone();
        assert!(matches!(
            curve.validate(),
            Err(HairDiagnostic::InvalidHairId { field }) if field == "curve.mass_id"
        ));

        let mut curve = valid_curve();
        curve.root = [1.2, 0.5, 0.0];
        assert!(matches!(
            curve.validate(),
            Err(HairDiagnostic::NonFiniteCoordinate { field }) if field == "curve.root"
        ));

        let mut card = valid_card();
        card.segments = CountRange::new(0, 0, 0);
        assert!(matches!(
            card.validate(),
            Err(HairDiagnostic::InvalidCountRange { field }) if field == "card.segments"
        ));

        let mut clump = valid_clump();
        clump.child_count = CountRange::new(0, 0, 0);
        assert!(matches!(
            clump.validate(),
            Err(HairDiagnostic::UnboundedPlacement { field, .. }) if field == "clump.child_count"
        ));

        let mut parting = valid_parting();
        parting.direction = [2.0, 0.0];
        assert!(matches!(
            parting.validate(),
            Err(HairDiagnostic::NonFiniteCoordinate { field }) if field == "parting.direction"
        ));
    }

    #[test]
    fn grammar_validation_rejects_schema_required_sets_and_duplicate_ids() {
        let mut invalid_schema = HairGrammar::empty("crown grammar");
        invalid_schema.schema_version += 1;
        assert_eq!(
            invalid_schema.validate(),
            Err(HairDiagnostic::InvalidSchemaVersion {
                schema_version: HAIR_GRAMMAR_SCHEMA_VERSION + 1
            })
        );

        let mut missing_controls = HairGrammar::empty("crown grammar");
        missing_controls.controls.clear();
        assert_eq!(
            missing_controls.validate(),
            Err(HairDiagnostic::InvalidCollection {
                field: "grammar.controls".to_owned()
            })
        );

        let mut missing_operations = HairGrammar::empty("crown grammar");
        missing_operations.operations.clear();
        assert_eq!(
            missing_operations.validate(),
            Err(HairDiagnostic::InvalidCollection {
                field: "grammar.operations".to_owned()
            })
        );

        let mut duplicate_controls = HairGrammar::empty("crown grammar");
        duplicate_controls
            .controls
            .push(duplicate_controls.controls[0].clone());
        assert!(matches!(
            duplicate_controls.validate(),
            Err(HairDiagnostic::DuplicateId {
                field,
                ..
            }) if field == "grammar.controls"
        ));

        let mut duplicate_masses = HairGrammar::empty("crown grammar");
        duplicate_masses.masses = vec![valid_mass(), valid_mass()];
        assert!(matches!(
            duplicate_masses.validate(),
            Err(HairDiagnostic::DuplicateId {
                field,
                ..
            }) if field == "grammar.masses"
        ));
    }

    #[test]
    fn grammar_validation_rejects_missing_references_and_contract_mismatch() {
        let mut missing_curve = HairGrammar::empty("crown grammar");
        missing_curve.masses = vec![valid_mass()];
        missing_curve.cards = vec![valid_card()];
        assert!(matches!(
            missing_curve.validate(),
            Err(HairDiagnostic::MissingReference { field, id })
                if field == "card.curve_id" && id == curve_id().0
        ));

        let mut missing_mass = HairGrammar::empty("crown grammar");
        let mut curve = valid_curve();
        curve.mass_id = HairElementId::deterministic(HairElementKind::Mass, "missing mass");
        missing_mass.curves = vec![curve.clone()];
        assert!(matches!(
            missing_mass.validate(),
            Err(HairDiagnostic::MissingReference { field, id })
                if field == "curve.mass_id" && id == curve.mass_id.0
        ));

        let mut bad_operation = HairGrammar::empty("crown grammar");
        let procedural = bad_operation
            .operations
            .iter_mut()
            .find(|operation| operation.kind == HairOperationKind::PlaceProceduralHair)
            .expect("procedural placement operation");
        procedural.max_output_instances = None;
        assert!(matches!(
            bad_operation.validate(),
            Err(HairDiagnostic::OperationContractMismatch { field })
                if field == "operation.PlaceProceduralHair"
        ));

        let mut cross_mass = HairGrammar::empty("crown grammar");
        let other_mass_id = HairElementId::deterministic(HairElementKind::Mass, "other mass");
        let other_mass = HairMass {
            id: other_mass_id.clone(),
            ..valid_mass()
        };
        let other_curve = HairGuideCurve {
            id: HairElementId::deterministic(HairElementKind::Curve, "other curve"),
            mass_id: other_mass_id,
            ..valid_curve()
        };
        cross_mass.masses = vec![valid_mass(), other_mass];
        cross_mass.curves = vec![valid_curve(), other_curve.clone()];
        cross_mass.cards = vec![HairCard {
            curve_id: other_curve.id.clone(),
            ..valid_card()
        }];
        assert!(matches!(
            cross_mass.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. }) if field == "card.curve_id"
        ));

        let mut cross_mass_clump = HairGrammar::empty("crown grammar");
        let other_mass_id = HairElementId::deterministic(HairElementKind::Mass, "other mass");
        let other_mass = HairMass {
            id: other_mass_id.clone(),
            ..valid_mass()
        };
        let other_curve = HairGuideCurve {
            id: HairElementId::deterministic(HairElementKind::Curve, "other curve"),
            mass_id: other_mass_id,
            ..valid_curve()
        };
        cross_mass_clump.masses = vec![valid_mass(), other_mass];
        cross_mass_clump.curves = vec![valid_curve(), other_curve.clone()];
        cross_mass_clump.clumps = vec![HairClump {
            curve_ids: vec![other_curve.id],
            ..valid_clump()
        }];
        assert!(matches!(
            cross_mass_clump.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. }) if field == "clump.curve_ids"
        ));

        let mut missing_placement_curve = HairGrammar::empty("crown grammar");
        let missing_curve_id =
            HairElementId::deterministic(HairElementKind::Curve, "missing placement curve");
        let mut placement = HairPlacementBounds::scalp_uv(
            "missing placement curve",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        placement.strategy = HairPlacementStrategy::AlongCurve;
        placement.curve_ids = vec![missing_curve_id.clone()];
        missing_placement_curve.masses = vec![valid_mass()];
        missing_placement_curve.curves = vec![valid_curve()];
        missing_placement_curve.placements = vec![placement];
        assert!(matches!(
            missing_placement_curve.validate(),
            Err(HairDiagnostic::MissingReference { field, id })
                if field == "placement.curve_ids" && id == missing_curve_id.0
        ));

        let mut missing_placement_parting = HairGrammar::empty("crown grammar");
        let missing_parting_id =
            HairElementId::deterministic(HairElementKind::PartingRegion, "missing parting");
        let mut placement = HairPlacementBounds::scalp_uv(
            "missing placement parting",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        placement.strategy = HairPlacementStrategy::PartingAware;
        placement.parting_region_id = Some(missing_parting_id.clone());
        missing_placement_parting.masses = vec![valid_mass()];
        missing_placement_parting.placements = vec![placement];
        assert!(matches!(
            missing_placement_parting.validate(),
            Err(HairDiagnostic::MissingReference { field, id })
                if field == "placement.parting_region_id" && id == missing_parting_id.0
        ));

        let mut cross_mass_placement = HairGrammar::empty("crown grammar");
        let other_mass_id = HairElementId::deterministic(HairElementKind::Mass, "placement mass");
        let other_mass = HairMass {
            id: other_mass_id.clone(),
            ..valid_mass()
        };
        let other_curve = HairGuideCurve {
            id: HairElementId::deterministic(HairElementKind::Curve, "placement curve"),
            mass_id: other_mass_id,
            ..valid_curve()
        };
        let mut placement = HairPlacementBounds::scalp_uv(
            "cross mass placement",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        placement.strategy = HairPlacementStrategy::AlongCurve;
        placement.curve_ids = vec![other_curve.id.clone()];
        cross_mass_placement.masses = vec![valid_mass(), other_mass];
        cross_mass_placement.curves = vec![valid_curve(), other_curve];
        cross_mass_placement.placements = vec![placement];
        assert!(matches!(
            cross_mass_placement.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. })
                if field == "placement.curve_ids"
        ));
    }

    #[test]
    fn placement_is_finite_and_bounded() {
        let placement = HairPlacementBounds::scalp_uv(
            "crown placement",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        assert!(placement.is_bounded());

        let mut unbounded = placement.clone();
        unbounded.instance_count = CountRange::new(1, MAX_PROCEDURAL_HAIR_INSTANCES + 1, 128);
        assert_eq!(
            unbounded.validate(),
            Err(HairDiagnostic::UnboundedPlacement {
                field: "placement.instance_count".to_owned(),
                requested_max: MAX_PROCEDURAL_HAIR_INSTANCES + 1,
                allowed_max: MAX_PROCEDURAL_HAIR_INSTANCES,
            })
        );

        let mut zero_instances = placement.clone();
        zero_instances.instance_count = CountRange::new(0, 512, 128);
        assert_eq!(
            zero_instances.validate(),
            Err(HairDiagnostic::InvalidCountRange {
                field: "placement.instance_count".to_owned(),
            })
        );

        let mut invalid_uv = placement;
        invalid_uv.root_u = scalar(0.0, 1.2, 0.5);
        assert_eq!(
            invalid_uv.validate(),
            Err(HairDiagnostic::InvalidNormalizedRange {
                field: "placement.root_u".to_owned(),
            })
        );
    }

    #[test]
    fn placement_strategy_payloads_are_validated() {
        let mut scalp_uv = HairPlacementBounds::scalp_uv(
            "scalp",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        scalp_uv.curve_ids = vec![curve_id()];
        assert!(matches!(
            scalp_uv.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. })
                if field == "placement.strategy"
        ));
        scalp_uv.curve_ids.clear();
        scalp_uv.parting_region_id = Some(valid_parting().id);
        assert!(matches!(
            scalp_uv.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. })
                if field == "placement.strategy"
        ));

        let mut along_curve = HairPlacementBounds::scalp_uv(
            "along",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        along_curve.strategy = HairPlacementStrategy::AlongCurve;
        assert!(matches!(
            along_curve.validate(),
            Err(HairDiagnostic::InvalidCollection { field }) if field == "placement.curve_ids"
        ));

        along_curve.curve_ids = vec![curve_id()];
        along_curve.parting_region_id = Some(valid_parting().id);
        assert!(matches!(
            along_curve.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. })
                if field == "placement.parting_region_id"
        ));
        along_curve.parting_region_id = None;
        assert!(along_curve.validate().is_ok());

        let mut parting_aware = HairPlacementBounds::scalp_uv(
            "parting",
            mass_id(),
            CharacterRegionId("scalp.crown".to_owned()),
        );
        parting_aware.strategy = HairPlacementStrategy::PartingAware;
        assert!(matches!(
            parting_aware.validate(),
            Err(HairDiagnostic::MissingReference { field, .. })
                if field == "placement.parting_region_id"
        ));
        parting_aware.parting_region_id = Some(valid_parting().id);
        parting_aware.curve_ids = vec![curve_id()];
        assert!(matches!(
            parting_aware.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. })
                if field == "placement.curve_ids"
        ));
        parting_aware.curve_ids.clear();
        assert!(parting_aware.validate().is_ok());

        let mut grammar = HairGrammar::empty("placement grammar");
        grammar.masses = vec![valid_mass()];
        grammar.curves = vec![valid_curve()];
        grammar.parting_regions = vec![valid_parting()];
        grammar.placements = vec![along_curve, parting_aware];
        grammar.validate().expect("placement references resolve");
    }

    #[test]
    fn controls_have_valid_finite_ranges() {
        let controls = default_hair_controls();
        assert_eq!(controls.len(), 8);
        assert!(controls.iter().all(HairControlContract::is_valid));

        let mut invalid = HairControlContract::default_for(HairControlRole::Frizz);
        invalid.range.default = f32::NAN;
        assert_eq!(
            invalid.validate(),
            Err(HairDiagnostic::InvalidScalarRange {
                field: "control.range".to_owned(),
            })
        );

        let mut mismatched = HairControlContract::default_for(HairControlRole::Frizz);
        mismatched.id = deterministic_hair_control_id(HairControlRole::Curl);
        assert!(matches!(
            mismatched.validate(),
            Err(HairDiagnostic::InconsistentReference { field, .. }) if field == "control.id"
        ));
    }

    #[test]
    fn strand_perfect_reconstruction_is_rejected() {
        assert_eq!(
            validate_reconstruction_request(&HairReconstructionRequest::SemanticGrammar),
            Ok(())
        );

        let rejected = HairReconstructionRequest::StrandPerfect {
            requested_strands: Some(120_000),
        };
        assert_eq!(
            validate_reconstruction_request(&rejected),
            Err(HairDiagnostic::UnsupportedStrandPerfectReconstruction {
                requested_strands: Some(120_000),
            })
        );
    }
}
