//! Versioned character base topology library.

use crate::{
    CHARACTER_GRAMMAR_SCHEMA_VERSION, CharacterBaseId, CharacterLandmarkId, CharacterLoopId,
    CharacterRegionId, CharacterSymmetryId,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Current semantic topology contract version.
pub const CHARACTER_BASE_TOPOLOGY_VERSION: BaseTopologyVersion = BaseTopologyVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Current semantic topology library version.
pub const BASE_TOPOLOGY_LIBRARY_VERSION: BaseTopologyVersion = CHARACTER_BASE_TOPOLOGY_VERSION;

/// Versioned humanoid body base identifier.
pub const HUMANOID_BODY_BASE_ID: &str = "base.humanoid.body.v1";
/// Versioned humanoid head base identifier.
pub const HUMANOID_HEAD_BASE_ID: &str = "base.humanoid.head.v1";
/// Versioned humanoid hands base identifier.
pub const HUMANOID_HANDS_BASE_ID: &str = "base.humanoid.hands.v1";
/// Versioned humanoid feet base identifier.
pub const HUMANOID_FEET_BASE_ID: &str = "base.humanoid.feet.v1";
/// Versioned humanoid eyes base identifier.
pub const HUMANOID_EYES_BASE_ID: &str = "base.humanoid.eyes.v1";
/// Versioned humanoid teeth base identifier.
pub const HUMANOID_TEETH_BASE_ID: &str = "base.humanoid.teeth.v1";

/// The required bases that make up the first humanoid topology library.
pub const REQUIRED_BASE_IDS: [&str; 6] = [
    HUMANOID_BODY_BASE_ID,
    HUMANOID_HEAD_BASE_ID,
    HUMANOID_HANDS_BASE_ID,
    HUMANOID_FEET_BASE_ID,
    HUMANOID_EYES_BASE_ID,
    HUMANOID_TEETH_BASE_ID,
];

/// Semantic version for authored topology contracts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BaseTopologyVersion {
    /// Major version for incompatible contract changes.
    pub major: u16,
    /// Minor version for compatible contract additions.
    pub minor: u16,
    /// Patch version for non-contractual corrections.
    pub patch: u16,
}

impl BaseTopologyVersion {
    /// Returns true when the version can identify a published topology contract.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.major > 0
    }

    fn update_fingerprint(self, hasher: &mut blake3::Hasher) {
        feed_u16(hasher, self.major);
        feed_u16(hasher, self.minor);
        feed_u16(hasher, self.patch);
    }
}

impl std::fmt::Display for BaseTopologyVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Stable BLAKE3 fingerprint for a base or base library.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BaseFingerprint(pub String);

impl BaseFingerprint {
    /// Hex digest text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Serializable collection of topology bases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseTopologyLibrary {
    /// Grammar schema version used by every base.
    pub schema_version: u32,
    /// Library contract version.
    pub library_version: BaseTopologyVersion,
    /// Authored topology bases.
    pub bases: Vec<CharacterBaseTopology>,
}

impl BaseTopologyLibrary {
    /// Validate the library and all of its bases.
    pub fn validate(&self) -> Result<(), BaseTopologyError> {
        if self.schema_version != CHARACTER_GRAMMAR_SCHEMA_VERSION {
            return Err(BaseTopologyError::InvalidSchemaVersion {
                base: "base topology library".to_owned(),
                schema_version: self.schema_version,
            });
        }

        if self.library_version != BASE_TOPOLOGY_LIBRARY_VERSION {
            return Err(BaseTopologyError::InvalidTopologyVersion {
                base: "base topology library".to_owned(),
                version: self.library_version.to_string(),
            });
        }

        ensure_non_empty(&self.bases, "base topology library", "bases")?;

        let mut base_ids = BTreeSet::new();
        let mut region_ids = BTreeSet::new();
        let mut landmark_ids = BTreeSet::new();
        let mut symmetry_ids = BTreeSet::new();
        let mut loop_ids = BTreeSet::new();
        let mut contract_ids = BTreeSet::new();
        for base in &self.bases {
            ensure_unique(&mut base_ids, &base.id.0, "base topology library", "bases")?;
            base.validate()?;
            for region in &base.regions {
                ensure_unique(
                    &mut region_ids,
                    &region.id.0,
                    "base topology library",
                    "regions",
                )?;
            }
            for landmark in &base.landmarks {
                ensure_unique(
                    &mut landmark_ids,
                    &landmark.id.0,
                    "base topology library",
                    "landmarks",
                )?;
            }
            for symmetry in &base.symmetries {
                ensure_unique(
                    &mut symmetry_ids,
                    &symmetry.id.0,
                    "base topology library",
                    "symmetries",
                )?;
            }
            for topology_loop in &base.loops {
                ensure_unique(
                    &mut loop_ids,
                    &topology_loop.id.0,
                    "base topology library",
                    "loops",
                )?;
            }
            for contract in &base.contracts {
                ensure_unique(
                    &mut contract_ids,
                    &contract.id.0,
                    "base topology library",
                    "contracts",
                )?;
            }
        }
        for required in REQUIRED_BASE_IDS {
            if !base_ids.contains(required) {
                return Err(BaseTopologyError::MissingRequiredBase {
                    base: "base topology library".to_owned(),
                    id: required.to_owned(),
                });
            }
        }

        Ok(())
    }

    /// Deterministic fingerprint of the complete library contents.
    #[must_use]
    pub fn fingerprint(&self) -> BaseFingerprint {
        let mut hasher = blake3::Hasher::new();
        feed_str(&mut hasher, "shape-character.base-library");
        feed_u32(&mut hasher, self.schema_version);
        self.library_version.update_fingerprint(&mut hasher);
        feed_len(&mut hasher, self.bases.len());
        for base in &self.bases {
            base.update_fingerprint(&mut hasher);
        }
        BaseFingerprint(hasher.finalize().to_hex().to_string())
    }
}

/// Base topology plus the fingerprint for its current content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintedCharacterBase {
    /// Authored base contract.
    pub base: CharacterBaseTopology,
    /// BLAKE3 fingerprint of `base`.
    pub fingerprint: BaseFingerprint,
}

impl FingerprintedCharacterBase {
    /// Create a fingerprinted base from authored content.
    #[must_use]
    pub fn new(base: CharacterBaseTopology) -> Self {
        let fingerprint = base.fingerprint();
        Self { base, fingerprint }
    }

    /// Returns true when the stored fingerprint matches the current base content.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.fingerprint == self.base.fingerprint()
    }
}

/// Serializable versioned character base topology contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterBaseTopology {
    /// Grammar schema version.
    pub schema_version: u32,
    /// Contract version for this base.
    pub base_version: BaseTopologyVersion,
    /// Stable versioned base identifier.
    pub id: CharacterBaseId,
    /// Human-readable base name.
    pub name: String,
    /// High-level semantic domain for this base.
    pub domain: BaseSemanticDomain,
    /// Semantic surface and volume regions.
    pub regions: Vec<BaseRegion>,
    /// Stable semantic anchors.
    pub landmarks: Vec<BaseLandmark>,
    /// Symmetry contracts.
    pub symmetries: Vec<BaseSymmetry>,
    /// Topological loop contracts.
    pub loops: Vec<BaseTopologyLoop>,
    /// Required topology contracts.
    pub contracts: Vec<TopologyContract>,
}

impl CharacterBaseTopology {
    /// Validate local IDs, references, symmetry, loops, and contract coverage.
    pub fn validate(&self) -> Result<(), BaseTopologyError> {
        let base = self.id.0.as_str();
        if self.schema_version != CHARACTER_GRAMMAR_SCHEMA_VERSION {
            return Err(BaseTopologyError::InvalidSchemaVersion {
                base: base.to_owned(),
                schema_version: self.schema_version,
            });
        }

        if self.base_version != CHARACTER_BASE_TOPOLOGY_VERSION {
            return Err(BaseTopologyError::InvalidTopologyVersion {
                base: base.to_owned(),
                version: self.base_version.to_string(),
            });
        }

        ensure_non_empty(&self.regions, base, "regions")?;
        ensure_non_empty(&self.landmarks, base, "landmarks")?;
        ensure_non_empty(&self.symmetries, base, "symmetries")?;
        ensure_non_empty(&self.loops, base, "loops")?;
        ensure_non_empty(&self.contracts, base, "contracts")?;

        let mut region_ids = BTreeSet::new();
        let mut region_by_id = BTreeMap::new();
        for region in &self.regions {
            ensure_unique(&mut region_ids, &region.id.0, base, "regions")?;
            ensure_non_empty(&region.roles, base, "region roles")?;
            ensure_unique_references(
                base,
                &region.id.0,
                "region roles",
                region.roles.iter().map(|role| role.as_str().to_owned()),
            )?;
            region_by_id.insert(region.id.0.as_str(), region);
        }

        let mut landmark_ids = BTreeSet::new();
        let mut landmark_by_id = BTreeMap::new();
        for landmark in &self.landmarks {
            ensure_unique(&mut landmark_ids, &landmark.id.0, base, "landmarks")?;
            landmark_by_id.insert(landmark.id.0.as_str(), landmark);
        }

        let mut symmetry_ids = BTreeSet::new();
        for symmetry in &self.symmetries {
            ensure_unique(&mut symmetry_ids, &symmetry.id.0, base, "symmetries")?;
        }

        let mut loop_ids = BTreeSet::new();
        for topology_loop in &self.loops {
            ensure_unique(&mut loop_ids, &topology_loop.id.0, base, "loops")?;
        }

        let mut contract_ids = BTreeSet::new();
        for contract in &self.contracts {
            ensure_unique(&mut contract_ids, &contract.id.0, base, "contracts")?;
        }

        for region in &self.regions {
            if let Some(parent) = &region.parent {
                ensure_reference(&region_ids, &parent.0, base, &region.id.0, "regions")?;
            }
        }
        validate_region_hierarchy(base, &self.regions)?;

        for landmark in &self.landmarks {
            ensure_reference(
                &region_ids,
                &landmark.region.0,
                base,
                &landmark.id.0,
                "regions",
            )?;
            if let Some(mirror) = &landmark.mirror {
                if mirror == &landmark.id {
                    return Err(BaseTopologyError::SelfMirror {
                        base: base.to_owned(),
                        id: landmark.id.0.clone(),
                    });
                }
                ensure_reference(&landmark_ids, &mirror.0, base, &landmark.id.0, "landmarks")?;
                let mirrored = landmark_by_id
                    .get(mirror.0.as_str())
                    .expect("checked above");
                if mirrored.mirror.as_ref() != Some(&landmark.id) {
                    return Err(BaseTopologyError::AsymmetricMirror {
                        base: base.to_owned(),
                        id: landmark.id.0.clone(),
                        mirror: mirror.0.clone(),
                    });
                }
            }
        }

        for symmetry in &self.symmetries {
            if symmetry.region_pairs.is_empty()
                && symmetry.landmark_pairs.is_empty()
                && symmetry.fixed_regions.is_empty()
                && symmetry.fixed_landmarks.is_empty()
            {
                return Err(BaseTopologyError::EmptyReferenceList {
                    base: base.to_owned(),
                    owner: symmetry.id.0.clone(),
                    collection: "symmetry references",
                });
            }

            for pair in &symmetry.region_pairs {
                ensure_reference(&region_ids, &pair.left.0, base, &symmetry.id.0, "regions")?;
                ensure_reference(&region_ids, &pair.right.0, base, &symmetry.id.0, "regions")?;
            }

            for pair in &symmetry.landmark_pairs {
                ensure_reference(
                    &landmark_ids,
                    &pair.left.0,
                    base,
                    &symmetry.id.0,
                    "landmarks",
                )?;
                ensure_reference(
                    &landmark_ids,
                    &pair.right.0,
                    base,
                    &symmetry.id.0,
                    "landmarks",
                )?;
            }

            for region in &symmetry.fixed_regions {
                ensure_reference(&region_ids, &region.0, base, &symmetry.id.0, "regions")?;
            }

            for landmark in &symmetry.fixed_landmarks {
                ensure_reference(
                    &landmark_ids,
                    &landmark.0,
                    base,
                    &symmetry.id.0,
                    "landmarks",
                )?;
            }
            validate_symmetry_reference_uniqueness(base, symmetry)?;
            validate_symmetry_semantics(base, symmetry, &region_by_id, &landmark_by_id)?;
        }

        for topology_loop in &self.loops {
            if !topology_loop.closed {
                return Err(BaseTopologyError::OpenTopologyLoop {
                    base: base.to_owned(),
                    id: topology_loop.id.0.clone(),
                });
            }
            ensure_non_empty(&topology_loop.regions, base, "loop regions")?;
            ensure_non_empty(&topology_loop.landmarks, base, "loop landmarks")?;
            ensure_unique_references(
                base,
                &topology_loop.id.0,
                "loop regions",
                topology_loop.regions.iter().map(|region| region.0.clone()),
            )?;
            ensure_unique_references(
                base,
                &topology_loop.id.0,
                "loop landmarks",
                topology_loop
                    .landmarks
                    .iter()
                    .map(|landmark| landmark.0.clone()),
            )?;
            for region in &topology_loop.regions {
                ensure_reference(&region_ids, &region.0, base, &topology_loop.id.0, "regions")?;
            }
            for landmark in &topology_loop.landmarks {
                ensure_reference(
                    &landmark_ids,
                    &landmark.0,
                    base,
                    &topology_loop.id.0,
                    "landmarks",
                )?;
            }
        }

        let mut covered_regions = BTreeSet::new();
        let mut covered_landmarks = BTreeSet::new();
        let mut covered_symmetries = BTreeSet::new();
        let mut covered_loops = BTreeSet::new();

        for contract in &self.contracts {
            if contract.required_regions.is_empty()
                && contract.required_landmarks.is_empty()
                && contract.required_symmetries.is_empty()
                && contract.required_loops.is_empty()
            {
                return Err(BaseTopologyError::EmptyReferenceList {
                    base: base.to_owned(),
                    owner: contract.id.0.clone(),
                    collection: "contract requirements",
                });
            }
            ensure_unique_references(
                base,
                &contract.id.0,
                "contract regions",
                contract
                    .required_regions
                    .iter()
                    .map(|region| region.0.clone()),
            )?;
            ensure_unique_references(
                base,
                &contract.id.0,
                "contract landmarks",
                contract
                    .required_landmarks
                    .iter()
                    .map(|landmark| landmark.0.clone()),
            )?;
            ensure_unique_references(
                base,
                &contract.id.0,
                "contract symmetries",
                contract
                    .required_symmetries
                    .iter()
                    .map(|symmetry| symmetry.0.clone()),
            )?;
            ensure_unique_references(
                base,
                &contract.id.0,
                "contract loops",
                contract
                    .required_loops
                    .iter()
                    .map(|topology_loop| topology_loop.0.clone()),
            )?;

            for region in &contract.required_regions {
                ensure_reference(&region_ids, &region.0, base, &contract.id.0, "regions")?;
                covered_regions.insert(region.0.clone());
            }

            for landmark in &contract.required_landmarks {
                ensure_reference(
                    &landmark_ids,
                    &landmark.0,
                    base,
                    &contract.id.0,
                    "landmarks",
                )?;
                covered_landmarks.insert(landmark.0.clone());
            }

            for symmetry in &contract.required_symmetries {
                ensure_reference(
                    &symmetry_ids,
                    &symmetry.0,
                    base,
                    &contract.id.0,
                    "symmetries",
                )?;
                covered_symmetries.insert(symmetry.0.clone());
            }

            for topology_loop in &contract.required_loops {
                ensure_reference(&loop_ids, &topology_loop.0, base, &contract.id.0, "loops")?;
                covered_loops.insert(topology_loop.0.clone());
            }
        }

        ensure_covered(&region_ids, &covered_regions, base, "regions")?;
        ensure_covered(&landmark_ids, &covered_landmarks, base, "landmarks")?;
        ensure_covered(&symmetry_ids, &covered_symmetries, base, "symmetries")?;
        ensure_covered(&loop_ids, &covered_loops, base, "loops")?;

        Ok(())
    }

    /// Deterministic BLAKE3 fingerprint of authored base content.
    #[must_use]
    pub fn fingerprint(&self) -> BaseFingerprint {
        let mut hasher = blake3::Hasher::new();
        self.update_fingerprint(&mut hasher);
        BaseFingerprint(hasher.finalize().to_hex().to_string())
    }

    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "shape-character.base");
        feed_u32(hasher, self.schema_version);
        self.base_version.update_fingerprint(hasher);
        feed_str(hasher, &self.id.0);
        feed_str(hasher, &self.name);
        feed_str(hasher, self.domain.as_str());

        feed_len(hasher, self.regions.len());
        for region in &self.regions {
            region.update_fingerprint(hasher);
        }

        feed_len(hasher, self.landmarks.len());
        for landmark in &self.landmarks {
            landmark.update_fingerprint(hasher);
        }

        feed_len(hasher, self.symmetries.len());
        for symmetry in &self.symmetries {
            symmetry.update_fingerprint(hasher);
        }

        feed_len(hasher, self.loops.len());
        for topology_loop in &self.loops {
            topology_loop.update_fingerprint(hasher);
        }

        feed_len(hasher, self.contracts.len());
        for contract in &self.contracts {
            contract.update_fingerprint(hasher);
        }
    }
}

/// High-level base domain.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseSemanticDomain {
    /// Whole humanoid body.
    HumanoidBody,
    /// Head shell and primary facial anchors.
    Head,
    /// Left and right hands.
    Hands,
    /// Left and right feet.
    Feet,
    /// Eye forms and eyelid apertures.
    Eyes,
    /// Teeth and dental arches.
    Teeth,
}

impl BaseSemanticDomain {
    fn as_str(self) -> &'static str {
        match self {
            Self::HumanoidBody => "humanoid_body",
            Self::Head => "head",
            Self::Hands => "hands",
            Self::Feet => "feet",
            Self::Eyes => "eyes",
            Self::Teeth => "teeth",
        }
    }
}

/// Side semantics for mirrored and centered topology.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticSide {
    /// Centerline or unpaired element.
    Center,
    /// Left side from the character's perspective.
    Left,
    /// Right side from the character's perspective.
    Right,
    /// Region or loop that is intentionally bilateral.
    Bilateral,
}

impl SemanticSide {
    fn as_str(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Left => "left",
            Self::Right => "right",
            Self::Bilateral => "bilateral",
        }
    }
}

/// Coarse semantic role for a region.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRegionRole {
    /// Primary mass or root region.
    Core,
    /// Deformable outer surface.
    DeformationSurface,
    /// Attachment point for adjacent regions.
    Attachment,
    /// Joint or hinge area.
    Articulation,
    /// Limb segment.
    Limb,
    /// Finger, toe, or tooth group.
    Digit,
    /// Sensory feature such as eyes.
    SensoryFeature,
    /// Mouth or dental feature.
    OralFeature,
    /// Ordered dental arch.
    DentalArc,
    /// Boundary or rim.
    Boundary,
}

impl SemanticRegionRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::DeformationSurface => "deformation_surface",
            Self::Attachment => "attachment",
            Self::Articulation => "articulation",
            Self::Limb => "limb",
            Self::Digit => "digit",
            Self::SensoryFeature => "sensory_feature",
            Self::OralFeature => "oral_feature",
            Self::DentalArc => "dental_arc",
            Self::Boundary => "boundary",
        }
    }
}

/// Structural kind for semantic regions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    /// Root or central mass.
    Core,
    /// Deformable surface area.
    Surface,
    /// Joint or articulation band.
    Joint,
    /// Limb or appendage.
    Appendage,
    /// Finger, toe, or tooth group.
    Digit,
    /// Sensory or facial feature.
    Feature,
    /// Socket or aperture support.
    Socket,
    /// Dental arch.
    Arch,
}

impl RegionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Surface => "surface",
            Self::Joint => "joint",
            Self::Appendage => "appendage",
            Self::Digit => "digit",
            Self::Feature => "feature",
            Self::Socket => "socket",
            Self::Arch => "arch",
        }
    }
}

/// Stable semantic region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRegion {
    /// Stable region ID.
    pub id: CharacterRegionId,
    /// Human-readable name.
    pub name: String,
    /// Side semantics.
    pub side: SemanticSide,
    /// Structural region kind.
    pub kind: RegionKind,
    /// Optional parent region.
    pub parent: Option<CharacterRegionId>,
    /// Semantic roles fulfilled by the region.
    pub roles: Vec<SemanticRegionRole>,
}

impl BaseRegion {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "region");
        feed_str(hasher, &self.id.0);
        feed_str(hasher, &self.name);
        feed_str(hasher, self.side.as_str());
        feed_str(hasher, self.kind.as_str());
        feed_optional_str(hasher, self.parent.as_ref().map(|id| id.0.as_str()));
        feed_len(hasher, self.roles.len());
        for role in &self.roles {
            feed_str(hasher, role.as_str());
        }
    }
}

/// Semantic landmark kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandmarkKind {
    /// Center anchor for a region.
    Center,
    /// Attachment or seam point.
    Attachment,
    /// Joint anchor.
    Articulation,
    /// Distal tip.
    Tip,
    /// Boundary or rim marker.
    Boundary,
    /// Top or most projecting marker.
    Apex,
    /// Root marker.
    Root,
    /// Midline marker.
    Midline,
    /// Rim or contour marker.
    Rim,
}

impl LandmarkKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Attachment => "attachment",
            Self::Articulation => "articulation",
            Self::Tip => "tip",
            Self::Boundary => "boundary",
            Self::Apex => "apex",
            Self::Root => "root",
            Self::Midline => "midline",
            Self::Rim => "rim",
        }
    }
}

/// Stable semantic landmark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseLandmark {
    /// Stable landmark ID.
    pub id: CharacterLandmarkId,
    /// Human-readable name.
    pub name: String,
    /// Owning region.
    pub region: CharacterRegionId,
    /// Side semantics.
    pub side: SemanticSide,
    /// Semantic landmark kind.
    pub kind: LandmarkKind,
    /// Optional mirrored landmark.
    pub mirror: Option<CharacterLandmarkId>,
}

impl BaseLandmark {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "landmark");
        feed_str(hasher, &self.id.0);
        feed_str(hasher, &self.name);
        feed_str(hasher, &self.region.0);
        feed_str(hasher, self.side.as_str());
        feed_str(hasher, self.kind.as_str());
        feed_optional_str(hasher, self.mirror.as_ref().map(|id| id.0.as_str()));
    }
}

/// Symmetry contract kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymmetryKind {
    /// Bilateral mirror relationship.
    BilateralMirror,
    /// Fixed midline relationship.
    Midline,
}

impl SymmetryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::BilateralMirror => "bilateral_mirror",
            Self::Midline => "midline",
        }
    }
}

/// Named symmetry plane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymmetryPlane {
    /// Character sagittal plane.
    Sagittal,
    /// Eye pair midline.
    OcularMidline,
    /// Dental midline.
    DentalMidline,
}

impl SymmetryPlane {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sagittal => "sagittal",
            Self::OcularMidline => "ocular_midline",
            Self::DentalMidline => "dental_midline",
        }
    }
}

/// Region pair that should mirror across a symmetry plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionSymmetryPair {
    /// Left-side region.
    pub left: CharacterRegionId,
    /// Right-side region.
    pub right: CharacterRegionId,
}

impl RegionSymmetryPair {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, &self.left.0);
        feed_str(hasher, &self.right.0);
    }
}

/// Landmark pair that should mirror across a symmetry plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandmarkSymmetryPair {
    /// Left-side landmark.
    pub left: CharacterLandmarkId,
    /// Right-side landmark.
    pub right: CharacterLandmarkId,
}

impl LandmarkSymmetryPair {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, &self.left.0);
        feed_str(hasher, &self.right.0);
    }
}

/// Symmetry contract for regions and landmarks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseSymmetry {
    /// Stable symmetry ID.
    pub id: CharacterSymmetryId,
    /// Symmetry kind.
    pub kind: SymmetryKind,
    /// Plane used by the symmetry.
    pub plane: SymmetryPlane,
    /// Mirrored region pairs.
    pub region_pairs: Vec<RegionSymmetryPair>,
    /// Mirrored landmark pairs.
    pub landmark_pairs: Vec<LandmarkSymmetryPair>,
    /// Centerline regions fixed on the plane.
    pub fixed_regions: Vec<CharacterRegionId>,
    /// Centerline landmarks fixed on the plane.
    pub fixed_landmarks: Vec<CharacterLandmarkId>,
}

impl BaseSymmetry {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "symmetry");
        feed_str(hasher, &self.id.0);
        feed_str(hasher, self.kind.as_str());
        feed_str(hasher, self.plane.as_str());
        feed_len(hasher, self.region_pairs.len());
        for pair in &self.region_pairs {
            pair.update_fingerprint(hasher);
        }
        feed_id_list(
            hasher,
            self.landmark_pairs.len(),
            &self.landmark_pairs,
            |hasher, pair| {
                pair.update_fingerprint(hasher);
            },
        );
        feed_len(hasher, self.fixed_regions.len());
        for region in &self.fixed_regions {
            feed_str(hasher, &region.0);
        }
        feed_len(hasher, self.fixed_landmarks.len());
        for landmark in &self.fixed_landmarks {
            feed_str(hasher, &landmark.0);
        }
    }
}

/// Topology loop kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyLoopKind {
    /// Joint or articulation loop.
    Articulation,
    /// External boundary loop.
    Boundary,
    /// Contour loop.
    Contour,
    /// Cross-section loop.
    CrossSection,
    /// Aperture loop.
    Aperture,
    /// Dental or anatomical arch loop.
    Arch,
    /// Surface flow loop.
    Flow,
}

impl TopologyLoopKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Articulation => "articulation",
            Self::Boundary => "boundary",
            Self::Contour => "contour",
            Self::CrossSection => "cross_section",
            Self::Aperture => "aperture",
            Self::Arch => "arch",
            Self::Flow => "flow",
        }
    }
}

/// Stable topology loop contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseTopologyLoop {
    /// Stable loop ID.
    pub id: CharacterLoopId,
    /// Human-readable name.
    pub name: String,
    /// Loop kind.
    pub kind: TopologyLoopKind,
    /// Whether the loop is topologically closed.
    pub closed: bool,
    /// Regions participating in the loop.
    pub regions: Vec<CharacterRegionId>,
    /// Landmarks anchoring the loop.
    pub landmarks: Vec<CharacterLandmarkId>,
}

impl BaseTopologyLoop {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "loop");
        feed_str(hasher, &self.id.0);
        feed_str(hasher, &self.name);
        feed_str(hasher, self.kind.as_str());
        feed_bool(hasher, self.closed);
        feed_len(hasher, self.regions.len());
        for region in &self.regions {
            feed_str(hasher, &region.0);
        }
        feed_len(hasher, self.landmarks.len());
        for landmark in &self.landmarks {
            feed_str(hasher, &landmark.0);
        }
    }
}

/// Stable topology contract identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TopologyContractId(pub String);

/// Topology contract kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyContractKind {
    /// Regions must retain the declared hierarchy.
    RegionHierarchy,
    /// Landmarks must remain present and attached to their regions.
    LandmarkAnchors,
    /// Required loops must remain closed.
    LoopClosure,
    /// Required symmetry must remain satisfiable.
    SymmetryCoverage,
    /// Ordered semantic parts must remain ordered.
    Ordering,
    /// Aperture loops must remain nested around the feature.
    Aperture,
    /// Dental or contour arches must remain continuous.
    ArchContinuity,
}

impl TopologyContractKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::RegionHierarchy => "region_hierarchy",
            Self::LandmarkAnchors => "landmark_anchors",
            Self::LoopClosure => "loop_closure",
            Self::SymmetryCoverage => "symmetry_coverage",
            Self::Ordering => "ordering",
            Self::Aperture => "aperture",
            Self::ArchContinuity => "arch_continuity",
        }
    }
}

/// Declarative contract tying required topology records together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyContract {
    /// Stable contract ID.
    pub id: TopologyContractId,
    /// Contract kind.
    pub kind: TopologyContractKind,
    /// Human-readable contract summary.
    pub summary: String,
    /// Regions required by this contract.
    pub required_regions: Vec<CharacterRegionId>,
    /// Landmarks required by this contract.
    pub required_landmarks: Vec<CharacterLandmarkId>,
    /// Symmetries required by this contract.
    pub required_symmetries: Vec<CharacterSymmetryId>,
    /// Loops required by this contract.
    pub required_loops: Vec<CharacterLoopId>,
}

impl TopologyContract {
    fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        feed_str(hasher, "contract");
        feed_str(hasher, &self.id.0);
        feed_str(hasher, self.kind.as_str());
        feed_str(hasher, &self.summary);
        feed_len(hasher, self.required_regions.len());
        for region in &self.required_regions {
            feed_str(hasher, &region.0);
        }
        feed_len(hasher, self.required_landmarks.len());
        for landmark in &self.required_landmarks {
            feed_str(hasher, &landmark.0);
        }
        feed_len(hasher, self.required_symmetries.len());
        for symmetry in &self.required_symmetries {
            feed_str(hasher, &symmetry.0);
        }
        feed_len(hasher, self.required_loops.len());
        for topology_loop in &self.required_loops {
            feed_str(hasher, &topology_loop.0);
        }
    }
}

/// Validation failures for authored base topology contracts.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BaseTopologyError {
    /// Schema version mismatch.
    #[error("{base} has invalid schema version {schema_version}")]
    InvalidSchemaVersion {
        /// Base or library being validated.
        base: String,
        /// Actual schema version.
        schema_version: u32,
    },
    /// Topology version is not publishable.
    #[error("{base} has invalid topology version {version}")]
    InvalidTopologyVersion {
        /// Base or library being validated.
        base: String,
        /// Actual topology version.
        version: String,
    },
    /// Required collection is empty.
    #[error("{base} has no {collection}")]
    EmptyCollection {
        /// Base being validated.
        base: String,
        /// Empty collection name.
        collection: &'static str,
    },
    /// Published base library is missing a required built-in base.
    #[error("{base} is missing required base {id}")]
    MissingRequiredBase {
        /// Library being validated.
        base: String,
        /// Missing base ID.
        id: String,
    },
    /// Duplicate ID in a collection.
    #[error("{base} has duplicate {collection} id {id}")]
    DuplicateId {
        /// Base being validated.
        base: String,
        /// Collection name.
        collection: &'static str,
        /// Duplicate ID.
        id: String,
    },
    /// Duplicate reference within a single authored record.
    #[error("{base} {owner} has duplicate {collection} reference {id}")]
    DuplicateReference {
        /// Base being validated.
        base: String,
        /// Owner record ID.
        owner: String,
        /// Reference collection name.
        collection: &'static str,
        /// Duplicate referenced ID or value.
        id: String,
    },
    /// Reference to an ID not present in the target collection.
    #[error("{base} {owner} references missing {collection} id {id}")]
    MissingReference {
        /// Base being validated.
        base: String,
        /// Owner record ID.
        owner: String,
        /// Referenced collection name.
        collection: &'static str,
        /// Missing ID.
        id: String,
    },
    /// A record has no references where references are required.
    #[error("{base} {owner} has an empty {collection} reference list")]
    EmptyReferenceList {
        /// Base being validated.
        base: String,
        /// Owner record ID.
        owner: String,
        /// Empty reference list name.
        collection: &'static str,
    },
    /// Landmark mirror relation is not reciprocal.
    #[error("{base} landmark {id} mirror relation does not point back from {mirror}")]
    AsymmetricMirror {
        /// Base being validated.
        base: String,
        /// Landmark ID.
        id: String,
        /// Mirror landmark ID.
        mirror: String,
    },
    /// Landmark mirror relation points to itself.
    #[error("{base} landmark {id} cannot mirror itself")]
    SelfMirror {
        /// Base being validated.
        base: String,
        /// Landmark ID.
        id: String,
    },
    /// Region hierarchy contains a cycle.
    #[error("{base} region hierarchy contains a cycle at {id}")]
    RegionHierarchyCycle {
        /// Base being validated.
        base: String,
        /// Region ID where the cycle was detected.
        id: String,
    },
    /// Symmetry pairs or fixed records do not match declared side semantics.
    #[error("{base} symmetry {symmetry} has invalid side semantics for {id}")]
    InvalidSymmetrySide {
        /// Base being validated.
        base: String,
        /// Symmetry ID.
        symmetry: String,
        /// Invalid region or landmark ID.
        id: String,
    },
    /// Symmetry kind is inconsistent with the references it declares.
    #[error("{base} symmetry {symmetry} has invalid {kind} reference shape")]
    InvalidSymmetryShape {
        /// Base being validated.
        base: String,
        /// Symmetry ID.
        symmetry: String,
        /// Declared symmetry kind.
        kind: &'static str,
    },
    /// Required loop is not closed.
    #[error("{base} topology loop {id} is not closed")]
    OpenTopologyLoop {
        /// Base being validated.
        base: String,
        /// Open loop ID.
        id: String,
    },
    /// ID is valid but not covered by any topology contract.
    #[error("{base} {collection} id {id} is not covered by any topology contract")]
    UncoveredContractId {
        /// Base being validated.
        base: String,
        /// Collection name.
        collection: &'static str,
        /// Uncovered ID.
        id: String,
    },
}

/// Required built-in base IDs.
#[must_use]
pub fn required_base_ids() -> &'static [&'static str] {
    &REQUIRED_BASE_IDS
}

/// Complete built-in base topology library.
#[must_use]
pub fn base_topology_library() -> BaseTopologyLibrary {
    BaseTopologyLibrary {
        schema_version: CHARACTER_GRAMMAR_SCHEMA_VERSION,
        library_version: BASE_TOPOLOGY_LIBRARY_VERSION,
        bases: builtin_character_bases(),
    }
}

/// Built-in versioned character bases.
#[must_use]
pub fn builtin_character_bases() -> Vec<CharacterBaseTopology> {
    vec![
        humanoid_body_base(),
        humanoid_head_base(),
        humanoid_hands_base(),
        humanoid_feet_base(),
        humanoid_eyes_base(),
        humanoid_teeth_base(),
    ]
}

/// Built-in bases with their content fingerprints.
#[must_use]
pub fn fingerprinted_character_bases() -> Vec<FingerprintedCharacterBase> {
    builtin_character_bases()
        .into_iter()
        .map(FingerprintedCharacterBase::new)
        .collect()
}

/// Find a built-in character base by ID.
#[must_use]
pub fn builtin_character_base(id: &str) -> Option<CharacterBaseTopology> {
    match id {
        HUMANOID_BODY_BASE_ID => Some(humanoid_body_base()),
        HUMANOID_HEAD_BASE_ID => Some(humanoid_head_base()),
        HUMANOID_HANDS_BASE_ID => Some(humanoid_hands_base()),
        HUMANOID_FEET_BASE_ID => Some(humanoid_feet_base()),
        HUMANOID_EYES_BASE_ID => Some(humanoid_eyes_base()),
        HUMANOID_TEETH_BASE_ID => Some(humanoid_teeth_base()),
        _ => None,
    }
}

/// Humanoid body topology contract.
#[must_use]
pub fn humanoid_body_base() -> CharacterBaseTopology {
    let regions = vec![
        region(
            "body.root",
            "Body root",
            SemanticSide::Center,
            RegionKind::Core,
            None,
            &[SemanticRegionRole::Core],
        ),
        region(
            "body.torso",
            "Torso",
            SemanticSide::Center,
            RegionKind::Surface,
            Some("body.root"),
            &[
                SemanticRegionRole::Core,
                SemanticRegionRole::DeformationSurface,
            ],
        ),
        region(
            "body.pelvis",
            "Pelvis",
            SemanticSide::Center,
            RegionKind::Joint,
            Some("body.root"),
            &[SemanticRegionRole::Core, SemanticRegionRole::Articulation],
        ),
        region(
            "body.neck",
            "Neck",
            SemanticSide::Center,
            RegionKind::Joint,
            Some("body.torso"),
            &[
                SemanticRegionRole::Attachment,
                SemanticRegionRole::Articulation,
            ],
        ),
        region(
            "body.left_arm",
            "Left arm",
            SemanticSide::Left,
            RegionKind::Appendage,
            Some("body.torso"),
            &[SemanticRegionRole::Limb, SemanticRegionRole::Attachment],
        ),
        region(
            "body.right_arm",
            "Right arm",
            SemanticSide::Right,
            RegionKind::Appendage,
            Some("body.torso"),
            &[SemanticRegionRole::Limb, SemanticRegionRole::Attachment],
        ),
        region(
            "body.left_leg",
            "Left leg",
            SemanticSide::Left,
            RegionKind::Appendage,
            Some("body.pelvis"),
            &[SemanticRegionRole::Limb, SemanticRegionRole::Attachment],
        ),
        region(
            "body.right_leg",
            "Right leg",
            SemanticSide::Right,
            RegionKind::Appendage,
            Some("body.pelvis"),
            &[SemanticRegionRole::Limb, SemanticRegionRole::Attachment],
        ),
    ];

    let landmarks = vec![
        landmark(
            "body.pelvis_center",
            "Pelvis center",
            "body.pelvis",
            SemanticSide::Center,
            LandmarkKind::Center,
            None,
        ),
        landmark(
            "body.navel",
            "Navel",
            "body.torso",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "body.sternum",
            "Sternum",
            "body.torso",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "body.neck_base",
            "Neck base",
            "body.neck",
            SemanticSide::Center,
            LandmarkKind::Attachment,
            None,
        ),
        landmark(
            "body.left_shoulder",
            "Left shoulder",
            "body.left_arm",
            SemanticSide::Left,
            LandmarkKind::Attachment,
            Some("body.right_shoulder"),
        ),
        landmark(
            "body.right_shoulder",
            "Right shoulder",
            "body.right_arm",
            SemanticSide::Right,
            LandmarkKind::Attachment,
            Some("body.left_shoulder"),
        ),
        landmark(
            "body.left_elbow",
            "Left elbow",
            "body.left_arm",
            SemanticSide::Left,
            LandmarkKind::Articulation,
            Some("body.right_elbow"),
        ),
        landmark(
            "body.right_elbow",
            "Right elbow",
            "body.right_arm",
            SemanticSide::Right,
            LandmarkKind::Articulation,
            Some("body.left_elbow"),
        ),
        landmark(
            "body.left_wrist",
            "Left wrist",
            "body.left_arm",
            SemanticSide::Left,
            LandmarkKind::Articulation,
            Some("body.right_wrist"),
        ),
        landmark(
            "body.right_wrist",
            "Right wrist",
            "body.right_arm",
            SemanticSide::Right,
            LandmarkKind::Articulation,
            Some("body.left_wrist"),
        ),
        landmark(
            "body.left_hip",
            "Left hip",
            "body.left_leg",
            SemanticSide::Left,
            LandmarkKind::Attachment,
            Some("body.right_hip"),
        ),
        landmark(
            "body.right_hip",
            "Right hip",
            "body.right_leg",
            SemanticSide::Right,
            LandmarkKind::Attachment,
            Some("body.left_hip"),
        ),
        landmark(
            "body.left_knee",
            "Left knee",
            "body.left_leg",
            SemanticSide::Left,
            LandmarkKind::Articulation,
            Some("body.right_knee"),
        ),
        landmark(
            "body.right_knee",
            "Right knee",
            "body.right_leg",
            SemanticSide::Right,
            LandmarkKind::Articulation,
            Some("body.left_knee"),
        ),
        landmark(
            "body.left_ankle",
            "Left ankle",
            "body.left_leg",
            SemanticSide::Left,
            LandmarkKind::Articulation,
            Some("body.right_ankle"),
        ),
        landmark(
            "body.right_ankle",
            "Right ankle",
            "body.right_leg",
            SemanticSide::Right,
            LandmarkKind::Articulation,
            Some("body.left_ankle"),
        ),
    ];

    let symmetries = vec![symmetry(
        "body.symmetry.sagittal",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::Sagittal,
        &[
            ("body.left_arm", "body.right_arm"),
            ("body.left_leg", "body.right_leg"),
        ],
        &[
            ("body.left_shoulder", "body.right_shoulder"),
            ("body.left_elbow", "body.right_elbow"),
            ("body.left_wrist", "body.right_wrist"),
            ("body.left_hip", "body.right_hip"),
            ("body.left_knee", "body.right_knee"),
            ("body.left_ankle", "body.right_ankle"),
        ],
        &["body.root", "body.torso", "body.pelvis", "body.neck"],
        &[
            "body.pelvis_center",
            "body.navel",
            "body.sternum",
            "body.neck_base",
        ],
    )];

    let loops = vec![
        topology_loop(
            "body.loop.torso_girth",
            "Torso girth",
            TopologyLoopKind::CrossSection,
            &["body.torso"],
            &["body.navel", "body.sternum"],
        ),
        topology_loop(
            "body.loop.shoulder_girdle",
            "Shoulder girdle",
            TopologyLoopKind::Articulation,
            &["body.torso", "body.left_arm", "body.right_arm", "body.neck"],
            &[
                "body.left_shoulder",
                "body.right_shoulder",
                "body.neck_base",
                "body.sternum",
            ],
        ),
        topology_loop(
            "body.loop.pelvic_girdle",
            "Pelvic girdle",
            TopologyLoopKind::Articulation,
            &["body.pelvis", "body.left_leg", "body.right_leg"],
            &["body.left_hip", "body.right_hip", "body.pelvis_center"],
        ),
        topology_loop(
            "body.loop.left_arm_flow",
            "Left arm flow",
            TopologyLoopKind::Flow,
            &["body.left_arm"],
            &["body.left_shoulder", "body.left_elbow", "body.left_wrist"],
        ),
        topology_loop(
            "body.loop.right_arm_flow",
            "Right arm flow",
            TopologyLoopKind::Flow,
            &["body.right_arm"],
            &[
                "body.right_shoulder",
                "body.right_elbow",
                "body.right_wrist",
            ],
        ),
        topology_loop(
            "body.loop.left_leg_flow",
            "Left leg flow",
            TopologyLoopKind::Flow,
            &["body.left_leg"],
            &["body.left_hip", "body.left_knee", "body.left_ankle"],
        ),
        topology_loop(
            "body.loop.right_leg_flow",
            "Right leg flow",
            TopologyLoopKind::Flow,
            &["body.right_leg"],
            &["body.right_hip", "body.right_knee", "body.right_ankle"],
        ),
    ];

    let contracts = coverage_contracts("body", &regions, &landmarks, &symmetries, &loops);
    base(
        HUMANOID_BODY_BASE_ID,
        "Humanoid body",
        BaseSemanticDomain::HumanoidBody,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

/// Humanoid head topology contract.
#[must_use]
pub fn humanoid_head_base() -> CharacterBaseTopology {
    let regions = vec![
        region(
            "head.cranium",
            "Cranium",
            SemanticSide::Center,
            RegionKind::Core,
            None,
            &[
                SemanticRegionRole::Core,
                SemanticRegionRole::DeformationSurface,
            ],
        ),
        region(
            "head.face",
            "Face",
            SemanticSide::Center,
            RegionKind::Surface,
            Some("head.cranium"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.jaw",
            "Jaw",
            SemanticSide::Center,
            RegionKind::Joint,
            Some("head.face"),
            &[SemanticRegionRole::Articulation],
        ),
        region(
            "head.nose",
            "Nose",
            SemanticSide::Center,
            RegionKind::Feature,
            Some("head.face"),
            &[SemanticRegionRole::Attachment, SemanticRegionRole::Boundary],
        ),
        region(
            "head.left_brow",
            "Left brow",
            SemanticSide::Left,
            RegionKind::Surface,
            Some("head.face"),
            &[
                SemanticRegionRole::DeformationSurface,
                SemanticRegionRole::Boundary,
            ],
        ),
        region(
            "head.right_brow",
            "Right brow",
            SemanticSide::Right,
            RegionKind::Surface,
            Some("head.face"),
            &[
                SemanticRegionRole::DeformationSurface,
                SemanticRegionRole::Boundary,
            ],
        ),
        region(
            "head.nose_bridge_region",
            "Nose bridge region",
            SemanticSide::Center,
            RegionKind::Feature,
            Some("head.nose"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.nose_tip_region",
            "Nose tip region",
            SemanticSide::Center,
            RegionKind::Feature,
            Some("head.nose"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.left_nose_ala",
            "Left nose ala",
            SemanticSide::Left,
            RegionKind::Feature,
            Some("head.nose"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.right_nose_ala",
            "Right nose ala",
            SemanticSide::Right,
            RegionKind::Feature,
            Some("head.nose"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.mouth_outer",
            "Mouth outer loop region",
            SemanticSide::Center,
            RegionKind::Feature,
            Some("head.jaw"),
            &[
                SemanticRegionRole::OralFeature,
                SemanticRegionRole::Boundary,
            ],
        ),
        region(
            "head.mouth_inner",
            "Mouth inner loop region",
            SemanticSide::Center,
            RegionKind::Feature,
            Some("head.jaw"),
            &[
                SemanticRegionRole::OralFeature,
                SemanticRegionRole::Boundary,
            ],
        ),
        region(
            "head.left_ear",
            "Left ear",
            SemanticSide::Left,
            RegionKind::Feature,
            Some("head.cranium"),
            &[SemanticRegionRole::Attachment, SemanticRegionRole::Boundary],
        ),
        region(
            "head.right_ear",
            "Right ear",
            SemanticSide::Right,
            RegionKind::Feature,
            Some("head.cranium"),
            &[SemanticRegionRole::Attachment, SemanticRegionRole::Boundary],
        ),
        region(
            "head.left_cheek",
            "Left cheek",
            SemanticSide::Left,
            RegionKind::Surface,
            Some("head.face"),
            &[SemanticRegionRole::DeformationSurface],
        ),
        region(
            "head.right_cheek",
            "Right cheek",
            SemanticSide::Right,
            RegionKind::Surface,
            Some("head.face"),
            &[SemanticRegionRole::DeformationSurface],
        ),
    ];

    let landmarks = vec![
        landmark(
            "head.crown",
            "Crown",
            "head.cranium",
            SemanticSide::Center,
            LandmarkKind::Apex,
            None,
        ),
        landmark(
            "head.brow_center",
            "Brow center",
            "head.face",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "head.left_brow_peak",
            "Left brow peak",
            "head.left_brow",
            SemanticSide::Left,
            LandmarkKind::Apex,
            Some("head.right_brow_peak"),
        ),
        landmark(
            "head.right_brow_peak",
            "Right brow peak",
            "head.right_brow",
            SemanticSide::Right,
            LandmarkKind::Apex,
            Some("head.left_brow_peak"),
        ),
        landmark(
            "head.nose_bridge",
            "Nose bridge",
            "head.nose_bridge_region",
            SemanticSide::Center,
            LandmarkKind::Root,
            None,
        ),
        landmark(
            "head.nose_tip",
            "Nose tip",
            "head.nose_tip_region",
            SemanticSide::Center,
            LandmarkKind::Tip,
            None,
        ),
        landmark(
            "head.left_nose_ala_peak",
            "Left nose ala peak",
            "head.left_nose_ala",
            SemanticSide::Left,
            LandmarkKind::Apex,
            Some("head.right_nose_ala_peak"),
        ),
        landmark(
            "head.right_nose_ala_peak",
            "Right nose ala peak",
            "head.right_nose_ala",
            SemanticSide::Right,
            LandmarkKind::Apex,
            Some("head.left_nose_ala_peak"),
        ),
        landmark(
            "head.chin",
            "Chin",
            "head.jaw",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "head.left_mouth_corner",
            "Left mouth corner",
            "head.mouth_outer",
            SemanticSide::Left,
            LandmarkKind::Boundary,
            Some("head.right_mouth_corner"),
        ),
        landmark(
            "head.right_mouth_corner",
            "Right mouth corner",
            "head.mouth_outer",
            SemanticSide::Right,
            LandmarkKind::Boundary,
            Some("head.left_mouth_corner"),
        ),
        landmark(
            "head.upper_lip_mid",
            "Upper lip midpoint",
            "head.mouth_inner",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "head.lower_lip_mid",
            "Lower lip midpoint",
            "head.mouth_inner",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "head.left_cheek_peak",
            "Left cheek peak",
            "head.left_cheek",
            SemanticSide::Left,
            LandmarkKind::Apex,
            Some("head.right_cheek_peak"),
        ),
        landmark(
            "head.right_cheek_peak",
            "Right cheek peak",
            "head.right_cheek",
            SemanticSide::Right,
            LandmarkKind::Apex,
            Some("head.left_cheek_peak"),
        ),
        landmark(
            "head.left_jaw_angle",
            "Left jaw angle",
            "head.jaw",
            SemanticSide::Left,
            LandmarkKind::Boundary,
            Some("head.right_jaw_angle"),
        ),
        landmark(
            "head.right_jaw_angle",
            "Right jaw angle",
            "head.jaw",
            SemanticSide::Right,
            LandmarkKind::Boundary,
            Some("head.left_jaw_angle"),
        ),
        landmark(
            "head.left_ear_root",
            "Left ear root",
            "head.left_ear",
            SemanticSide::Left,
            LandmarkKind::Attachment,
            Some("head.right_ear_root"),
        ),
        landmark(
            "head.right_ear_root",
            "Right ear root",
            "head.right_ear",
            SemanticSide::Right,
            LandmarkKind::Attachment,
            Some("head.left_ear_root"),
        ),
        landmark(
            "head.left_ear_lobe",
            "Left ear lobe",
            "head.left_ear",
            SemanticSide::Left,
            LandmarkKind::Tip,
            Some("head.right_ear_lobe"),
        ),
        landmark(
            "head.right_ear_lobe",
            "Right ear lobe",
            "head.right_ear",
            SemanticSide::Right,
            LandmarkKind::Tip,
            Some("head.left_ear_lobe"),
        ),
    ];

    let symmetries = vec![symmetry(
        "head.symmetry.sagittal",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::Sagittal,
        &[
            ("head.left_ear", "head.right_ear"),
            ("head.left_cheek", "head.right_cheek"),
            ("head.left_brow", "head.right_brow"),
            ("head.left_nose_ala", "head.right_nose_ala"),
        ],
        &[
            ("head.left_cheek_peak", "head.right_cheek_peak"),
            ("head.left_jaw_angle", "head.right_jaw_angle"),
            ("head.left_ear_root", "head.right_ear_root"),
            ("head.left_ear_lobe", "head.right_ear_lobe"),
            ("head.left_brow_peak", "head.right_brow_peak"),
            ("head.left_nose_ala_peak", "head.right_nose_ala_peak"),
            ("head.left_mouth_corner", "head.right_mouth_corner"),
        ],
        &[
            "head.cranium",
            "head.face",
            "head.jaw",
            "head.nose",
            "head.nose_bridge_region",
            "head.nose_tip_region",
            "head.mouth_outer",
            "head.mouth_inner",
        ],
        &[
            "head.crown",
            "head.brow_center",
            "head.nose_bridge",
            "head.nose_tip",
            "head.chin",
            "head.upper_lip_mid",
            "head.lower_lip_mid",
        ],
    )];

    let loops = vec![
        topology_loop(
            "head.loop.cranial_equator",
            "Cranial equator",
            TopologyLoopKind::Contour,
            &["head.cranium"],
            &["head.crown", "head.brow_center"],
        ),
        topology_loop(
            "head.loop.facial_mask",
            "Facial mask",
            TopologyLoopKind::Boundary,
            &[
                "head.face",
                "head.left_cheek",
                "head.right_cheek",
                "head.nose",
                "head.left_brow",
                "head.right_brow",
                "head.nose_bridge_region",
                "head.nose_tip_region",
                "head.left_nose_ala",
                "head.right_nose_ala",
            ],
            &[
                "head.brow_center",
                "head.left_brow_peak",
                "head.right_brow_peak",
                "head.nose_bridge",
                "head.left_cheek_peak",
                "head.right_cheek_peak",
                "head.chin",
            ],
        ),
        topology_loop(
            "head.loop.mouth_outer",
            "Outer mouth loop",
            TopologyLoopKind::Boundary,
            &["head.mouth_outer"],
            &[
                "head.left_mouth_corner",
                "head.upper_lip_mid",
                "head.right_mouth_corner",
                "head.lower_lip_mid",
            ],
        ),
        topology_loop(
            "head.loop.mouth_inner",
            "Inner mouth loop",
            TopologyLoopKind::Aperture,
            &["head.mouth_inner"],
            &[
                "head.left_mouth_corner",
                "head.upper_lip_mid",
                "head.right_mouth_corner",
                "head.lower_lip_mid",
            ],
        ),
        topology_loop(
            "head.loop.jawline",
            "Jawline",
            TopologyLoopKind::Contour,
            &["head.jaw"],
            &["head.left_jaw_angle", "head.chin", "head.right_jaw_angle"],
        ),
        topology_loop(
            "head.loop.left_ear_attachment",
            "Left ear attachment",
            TopologyLoopKind::Articulation,
            &["head.left_ear", "head.cranium"],
            &["head.left_ear_root", "head.left_ear_lobe"],
        ),
        topology_loop(
            "head.loop.right_ear_attachment",
            "Right ear attachment",
            TopologyLoopKind::Articulation,
            &["head.right_ear", "head.cranium"],
            &["head.right_ear_root", "head.right_ear_lobe"],
        ),
    ];

    let contracts = coverage_contracts("head", &regions, &landmarks, &symmetries, &loops);
    base(
        HUMANOID_HEAD_BASE_ID,
        "Humanoid head",
        BaseSemanticDomain::Head,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

/// Humanoid hands topology contract.
#[must_use]
pub fn humanoid_hands_base() -> CharacterBaseTopology {
    const DIGITS: [(&str, &str); 5] = [
        ("thumb", "thumb"),
        ("index", "index finger"),
        ("middle", "middle finger"),
        ("ring", "ring finger"),
        ("little", "little finger"),
    ];

    let mut regions = Vec::new();
    let mut landmarks = Vec::new();
    let mut loops = Vec::new();

    for (side_name, side_label, side, other_side_name) in [
        ("left", "Left", SemanticSide::Left, "right"),
        ("right", "Right", SemanticSide::Right, "left"),
    ] {
        let palm_id = format!("hands.{side_name}_palm");
        let wrist_id = format!("hands.{side_name}_wrist");
        regions.push(region_owned(
            palm_id.clone(),
            format!("{side_label} palm"),
            side,
            RegionKind::Surface,
            None,
            &[
                SemanticRegionRole::Core,
                SemanticRegionRole::DeformationSurface,
            ],
        ));
        regions.push(region_owned(
            wrist_id.clone(),
            format!("{side_label} wrist"),
            side,
            RegionKind::Joint,
            Some(palm_id.clone()),
            &[
                SemanticRegionRole::Attachment,
                SemanticRegionRole::Articulation,
            ],
        ));

        landmarks.push(landmark_owned(
            format!("hands.{side_name}_wrist_center"),
            format!("{side_label} wrist center"),
            wrist_id.clone(),
            side,
            LandmarkKind::Articulation,
            Some(format!("hands.{other_side_name}_wrist_center")),
        ));
        landmarks.push(landmark_owned(
            format!("hands.{side_name}_palm_center"),
            format!("{side_label} palm center"),
            palm_id.clone(),
            side,
            LandmarkKind::Center,
            Some(format!("hands.{other_side_name}_palm_center")),
        ));

        loops.push(topology_loop_owned(
            format!("hands.loop.{side_name}_palm_border"),
            format!("{side_label} palm border"),
            TopologyLoopKind::Boundary,
            vec![rid(palm_id.clone()), rid(wrist_id.clone())],
            vec![
                lid(format!("hands.{side_name}_wrist_center")),
                lid(format!("hands.{side_name}_palm_center")),
            ],
        ));
        loops.push(topology_loop_owned(
            format!("hands.loop.{side_name}_wrist_cuff"),
            format!("{side_label} wrist cuff"),
            TopologyLoopKind::Articulation,
            vec![rid(wrist_id.clone()), rid(palm_id.clone())],
            vec![lid(format!("hands.{side_name}_wrist_center"))],
        ));

        for (digit_name, digit_label) in DIGITS {
            let region_id = format!("hands.{side_name}_{digit_name}");
            let mirror_digit = format!("hands.{other_side_name}_{digit_name}");
            regions.push(region_owned(
                region_id.clone(),
                format!("{side_label} {digit_label}"),
                side,
                RegionKind::Digit,
                Some(palm_id.clone()),
                &[SemanticRegionRole::Digit, SemanticRegionRole::Articulation],
            ));

            landmarks.push(landmark_owned(
                format!("hands.{side_name}_{digit_name}_base"),
                format!("{side_label} {digit_label} base"),
                region_id.clone(),
                side,
                LandmarkKind::Root,
                Some(format!("{mirror_digit}_base")),
            ));
            landmarks.push(landmark_owned(
                format!("hands.{side_name}_{digit_name}_tip"),
                format!("{side_label} {digit_label} tip"),
                region_id.clone(),
                side,
                LandmarkKind::Tip,
                Some(format!("{mirror_digit}_tip")),
            ));

            loops.push(topology_loop_owned(
                format!("hands.loop.{side_name}_{digit_name}_flow"),
                format!("{side_label} {digit_label} flow"),
                TopologyLoopKind::Flow,
                vec![rid(region_id.clone()), rid(palm_id.clone())],
                vec![
                    lid(format!("hands.{side_name}_{digit_name}_base")),
                    lid(format!("hands.{side_name}_{digit_name}_tip")),
                    lid(format!("hands.{side_name}_palm_center")),
                ],
            ));
        }
    }

    loops.push(topology_loop(
        "hands.loop.left_digit_fan",
        "Left digit fan",
        TopologyLoopKind::Contour,
        &[
            "hands.left_thumb",
            "hands.left_index",
            "hands.left_middle",
            "hands.left_ring",
            "hands.left_little",
            "hands.left_palm",
        ],
        &[
            "hands.left_thumb_tip",
            "hands.left_index_tip",
            "hands.left_middle_tip",
            "hands.left_ring_tip",
            "hands.left_little_tip",
            "hands.left_palm_center",
        ],
    ));
    loops.push(topology_loop(
        "hands.loop.right_digit_fan",
        "Right digit fan",
        TopologyLoopKind::Contour,
        &[
            "hands.right_thumb",
            "hands.right_index",
            "hands.right_middle",
            "hands.right_ring",
            "hands.right_little",
            "hands.right_palm",
        ],
        &[
            "hands.right_thumb_tip",
            "hands.right_index_tip",
            "hands.right_middle_tip",
            "hands.right_ring_tip",
            "hands.right_little_tip",
            "hands.right_palm_center",
        ],
    ));

    let mut region_pairs = vec![
        ("hands.left_palm".to_owned(), "hands.right_palm".to_owned()),
        (
            "hands.left_wrist".to_owned(),
            "hands.right_wrist".to_owned(),
        ),
    ];
    let mut landmark_pairs = vec![
        (
            "hands.left_wrist_center".to_owned(),
            "hands.right_wrist_center".to_owned(),
        ),
        (
            "hands.left_palm_center".to_owned(),
            "hands.right_palm_center".to_owned(),
        ),
    ];
    for (digit_name, _) in DIGITS {
        region_pairs.push((
            format!("hands.left_{digit_name}"),
            format!("hands.right_{digit_name}"),
        ));
        landmark_pairs.push((
            format!("hands.left_{digit_name}_base"),
            format!("hands.right_{digit_name}_base"),
        ));
        landmark_pairs.push((
            format!("hands.left_{digit_name}_tip"),
            format!("hands.right_{digit_name}_tip"),
        ));
    }
    let symmetries = vec![symmetry_owned(
        "hands.symmetry.sagittal",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::Sagittal,
        region_pairs
            .into_iter()
            .map(|(left, right)| RegionSymmetryPair {
                left: rid(left),
                right: rid(right),
            })
            .collect(),
        landmark_pairs
            .into_iter()
            .map(|(left, right)| LandmarkSymmetryPair {
                left: lid(left),
                right: lid(right),
            })
            .collect(),
        Vec::new(),
        Vec::new(),
    )];

    let mut contracts = coverage_contracts("hands", &regions, &landmarks, &symmetries, &loops);
    contracts.push(contract(
        "hands.contract.digit_order",
        TopologyContractKind::Ordering,
        "Digit order must remain thumb, index, middle, ring, little on each hand.",
        region_ids(&[
            "hands.left_thumb",
            "hands.left_index",
            "hands.left_middle",
            "hands.left_ring",
            "hands.left_little",
            "hands.right_thumb",
            "hands.right_index",
            "hands.right_middle",
            "hands.right_ring",
            "hands.right_little",
        ]),
        landmark_ids(&[
            "hands.left_thumb_tip",
            "hands.left_index_tip",
            "hands.left_middle_tip",
            "hands.left_ring_tip",
            "hands.left_little_tip",
            "hands.right_thumb_tip",
            "hands.right_index_tip",
            "hands.right_middle_tip",
            "hands.right_ring_tip",
            "hands.right_little_tip",
        ]),
        Vec::new(),
        loop_ids(&["hands.loop.left_digit_fan", "hands.loop.right_digit_fan"]),
    ));

    base(
        HUMANOID_HANDS_BASE_ID,
        "Humanoid hands",
        BaseSemanticDomain::Hands,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

/// Humanoid feet topology contract.
#[must_use]
pub fn humanoid_feet_base() -> CharacterBaseTopology {
    const TOES: [(&str, &str); 5] = [
        ("big_toe", "big toe"),
        ("second_toe", "second toe"),
        ("third_toe", "third toe"),
        ("fourth_toe", "fourth toe"),
        ("little_toe", "little toe"),
    ];

    let mut regions = Vec::new();
    let mut landmarks = Vec::new();
    let mut loops = Vec::new();

    for (side_name, side_label, side, other_side_name) in [
        ("left", "Left", SemanticSide::Left, "right"),
        ("right", "Right", SemanticSide::Right, "left"),
    ] {
        let sole_id = format!("feet.{side_name}_sole");
        let ankle_id = format!("feet.{side_name}_ankle");
        let heel_id = format!("feet.{side_name}_heel");
        regions.push(region_owned(
            sole_id.clone(),
            format!("{side_label} sole"),
            side,
            RegionKind::Surface,
            None,
            &[
                SemanticRegionRole::Core,
                SemanticRegionRole::DeformationSurface,
            ],
        ));
        regions.push(region_owned(
            ankle_id.clone(),
            format!("{side_label} ankle"),
            side,
            RegionKind::Joint,
            Some(sole_id.clone()),
            &[
                SemanticRegionRole::Attachment,
                SemanticRegionRole::Articulation,
            ],
        ));
        regions.push(region_owned(
            heel_id.clone(),
            format!("{side_label} heel"),
            side,
            RegionKind::Surface,
            Some(sole_id.clone()),
            &[SemanticRegionRole::DeformationSurface],
        ));

        landmarks.push(landmark_owned(
            format!("feet.{side_name}_ankle_center"),
            format!("{side_label} ankle center"),
            ankle_id.clone(),
            side,
            LandmarkKind::Articulation,
            Some(format!("feet.{other_side_name}_ankle_center")),
        ));
        landmarks.push(landmark_owned(
            format!("feet.{side_name}_heel_back"),
            format!("{side_label} heel back"),
            heel_id.clone(),
            side,
            LandmarkKind::Boundary,
            Some(format!("feet.{other_side_name}_heel_back")),
        ));
        landmarks.push(landmark_owned(
            format!("feet.{side_name}_ball"),
            format!("{side_label} ball"),
            sole_id.clone(),
            side,
            LandmarkKind::Center,
            Some(format!("feet.{other_side_name}_ball")),
        ));

        loops.push(topology_loop_owned(
            format!("feet.loop.{side_name}_outline"),
            format!("{side_label} foot outline"),
            TopologyLoopKind::Boundary,
            vec![
                rid(sole_id.clone()),
                rid(heel_id.clone()),
                rid(ankle_id.clone()),
            ],
            vec![
                lid(format!("feet.{side_name}_heel_back")),
                lid(format!("feet.{side_name}_ball")),
                lid(format!("feet.{side_name}_ankle_center")),
            ],
        ));
        loops.push(topology_loop_owned(
            format!("feet.loop.{side_name}_ankle_cuff"),
            format!("{side_label} ankle cuff"),
            TopologyLoopKind::Articulation,
            vec![rid(ankle_id.clone()), rid(sole_id.clone())],
            vec![lid(format!("feet.{side_name}_ankle_center"))],
        ));

        for (toe_name, toe_label) in TOES {
            let region_id = format!("feet.{side_name}_{toe_name}");
            let mirror_toe = format!("feet.{other_side_name}_{toe_name}");
            regions.push(region_owned(
                region_id.clone(),
                format!("{side_label} {toe_label}"),
                side,
                RegionKind::Digit,
                Some(sole_id.clone()),
                &[SemanticRegionRole::Digit, SemanticRegionRole::Articulation],
            ));
            landmarks.push(landmark_owned(
                format!("feet.{side_name}_{toe_name}_base"),
                format!("{side_label} {toe_label} base"),
                region_id.clone(),
                side,
                LandmarkKind::Root,
                Some(format!("{mirror_toe}_base")),
            ));
            landmarks.push(landmark_owned(
                format!("feet.{side_name}_{toe_name}_tip"),
                format!("{side_label} {toe_label} tip"),
                region_id.clone(),
                side,
                LandmarkKind::Tip,
                Some(format!("{mirror_toe}_tip")),
            ));
        }

        loops.push(topology_loop_owned(
            format!("feet.loop.{side_name}_toe_fan"),
            format!("{side_label} toe fan"),
            TopologyLoopKind::Contour,
            TOES.iter()
                .map(|(toe_name, _)| rid(format!("feet.{side_name}_{toe_name}")))
                .chain(std::iter::once(rid(sole_id.clone())))
                .collect(),
            TOES.iter()
                .map(|(toe_name, _)| lid(format!("feet.{side_name}_{toe_name}_tip")))
                .chain(std::iter::once(lid(format!("feet.{side_name}_ball"))))
                .collect(),
        ));
    }

    let mut region_pairs = vec![
        ("feet.left_sole".to_owned(), "feet.right_sole".to_owned()),
        ("feet.left_ankle".to_owned(), "feet.right_ankle".to_owned()),
        ("feet.left_heel".to_owned(), "feet.right_heel".to_owned()),
    ];
    let mut landmark_pairs = vec![
        (
            "feet.left_ankle_center".to_owned(),
            "feet.right_ankle_center".to_owned(),
        ),
        (
            "feet.left_heel_back".to_owned(),
            "feet.right_heel_back".to_owned(),
        ),
        ("feet.left_ball".to_owned(), "feet.right_ball".to_owned()),
    ];
    for (toe_name, _) in TOES {
        region_pairs.push((
            format!("feet.left_{toe_name}"),
            format!("feet.right_{toe_name}"),
        ));
        landmark_pairs.push((
            format!("feet.left_{toe_name}_base"),
            format!("feet.right_{toe_name}_base"),
        ));
        landmark_pairs.push((
            format!("feet.left_{toe_name}_tip"),
            format!("feet.right_{toe_name}_tip"),
        ));
    }
    let symmetries = vec![symmetry_owned(
        "feet.symmetry.sagittal",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::Sagittal,
        region_pairs
            .into_iter()
            .map(|(left, right)| RegionSymmetryPair {
                left: rid(left),
                right: rid(right),
            })
            .collect(),
        landmark_pairs
            .into_iter()
            .map(|(left, right)| LandmarkSymmetryPair {
                left: lid(left),
                right: lid(right),
            })
            .collect(),
        Vec::new(),
        Vec::new(),
    )];

    let mut contracts = coverage_contracts("feet", &regions, &landmarks, &symmetries, &loops);
    contracts.push(contract(
        "feet.contract.toe_order",
        TopologyContractKind::Ordering,
        "Toe order must remain big, second, third, fourth, little on each foot.",
        region_ids(&[
            "feet.left_big_toe",
            "feet.left_second_toe",
            "feet.left_third_toe",
            "feet.left_fourth_toe",
            "feet.left_little_toe",
            "feet.right_big_toe",
            "feet.right_second_toe",
            "feet.right_third_toe",
            "feet.right_fourth_toe",
            "feet.right_little_toe",
        ]),
        landmark_ids(&[
            "feet.left_big_toe_tip",
            "feet.left_second_toe_tip",
            "feet.left_third_toe_tip",
            "feet.left_fourth_toe_tip",
            "feet.left_little_toe_tip",
            "feet.right_big_toe_tip",
            "feet.right_second_toe_tip",
            "feet.right_third_toe_tip",
            "feet.right_fourth_toe_tip",
            "feet.right_little_toe_tip",
        ]),
        Vec::new(),
        loop_ids(&["feet.loop.left_toe_fan", "feet.loop.right_toe_fan"]),
    ));

    base(
        HUMANOID_FEET_BASE_ID,
        "Humanoid feet",
        BaseSemanticDomain::Feet,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

/// Humanoid eyes topology contract.
#[must_use]
pub fn humanoid_eyes_base() -> CharacterBaseTopology {
    let mut regions = Vec::new();
    let mut landmarks = Vec::new();
    let mut loops = Vec::new();

    for (side_name, side_label, side, other_side_name) in [
        ("left", "Left", SemanticSide::Left, "right"),
        ("right", "Right", SemanticSide::Right, "left"),
    ] {
        for (part, label, kind, roles) in [
            (
                "sclera",
                "sclera",
                RegionKind::Socket,
                vec![SemanticRegionRole::SensoryFeature],
            ),
            (
                "iris",
                "iris",
                RegionKind::Feature,
                vec![
                    SemanticRegionRole::SensoryFeature,
                    SemanticRegionRole::Boundary,
                ],
            ),
            (
                "pupil",
                "pupil",
                RegionKind::Feature,
                vec![
                    SemanticRegionRole::SensoryFeature,
                    SemanticRegionRole::Boundary,
                ],
            ),
            (
                "upper_lid",
                "upper lid",
                RegionKind::Surface,
                vec![
                    SemanticRegionRole::Boundary,
                    SemanticRegionRole::Articulation,
                ],
            ),
            (
                "lower_lid",
                "lower lid",
                RegionKind::Surface,
                vec![
                    SemanticRegionRole::Boundary,
                    SemanticRegionRole::Articulation,
                ],
            ),
        ] {
            let parent = match part {
                "sclera" => None,
                "iris" | "pupil" | "upper_lid" | "lower_lid" => {
                    Some(format!("eyes.{side_name}_sclera"))
                }
                _ => None,
            };
            regions.push(region_owned(
                format!("eyes.{side_name}_{part}"),
                format!("{side_label} {label}"),
                side,
                kind,
                parent,
                &roles,
            ));
        }

        for (mark, label, region_part, kind) in [
            ("center", "center", "sclera", LandmarkKind::Center),
            (
                "pupil_center",
                "pupil center",
                "pupil",
                LandmarkKind::Center,
            ),
            (
                "inner_canthus",
                "inner canthus",
                "sclera",
                LandmarkKind::Boundary,
            ),
            (
                "outer_canthus",
                "outer canthus",
                "sclera",
                LandmarkKind::Boundary,
            ),
            (
                "upper_lid_peak",
                "upper lid peak",
                "upper_lid",
                LandmarkKind::Apex,
            ),
            (
                "lower_lid_peak",
                "lower lid peak",
                "lower_lid",
                LandmarkKind::Apex,
            ),
        ] {
            landmarks.push(landmark_owned(
                format!("eyes.{side_name}_{mark}"),
                format!("{side_label} eye {label}"),
                format!("eyes.{side_name}_{region_part}"),
                side,
                kind,
                Some(format!("eyes.{other_side_name}_{mark}")),
            ));
        }

        loops.push(topology_loop_owned(
            format!("eyes.loop.{side_name}_orbital_rim"),
            format!("{side_label} orbital rim"),
            TopologyLoopKind::Aperture,
            vec![
                rid(format!("eyes.{side_name}_sclera")),
                rid(format!("eyes.{side_name}_upper_lid")),
                rid(format!("eyes.{side_name}_lower_lid")),
            ],
            vec![
                lid(format!("eyes.{side_name}_inner_canthus")),
                lid(format!("eyes.{side_name}_outer_canthus")),
                lid(format!("eyes.{side_name}_upper_lid_peak")),
                lid(format!("eyes.{side_name}_lower_lid_peak")),
            ],
        ));
        loops.push(topology_loop_owned(
            format!("eyes.loop.{side_name}_iris_ring"),
            format!("{side_label} iris ring"),
            TopologyLoopKind::Aperture,
            vec![rid(format!("eyes.{side_name}_iris"))],
            vec![lid(format!("eyes.{side_name}_center"))],
        ));
        loops.push(topology_loop_owned(
            format!("eyes.loop.{side_name}_pupil_ring"),
            format!("{side_label} pupil ring"),
            TopologyLoopKind::Aperture,
            vec![rid(format!("eyes.{side_name}_pupil"))],
            vec![lid(format!("eyes.{side_name}_pupil_center"))],
        ));
    }

    let region_pairs = [
        ("eyes.left_sclera", "eyes.right_sclera"),
        ("eyes.left_iris", "eyes.right_iris"),
        ("eyes.left_pupil", "eyes.right_pupil"),
        ("eyes.left_upper_lid", "eyes.right_upper_lid"),
        ("eyes.left_lower_lid", "eyes.right_lower_lid"),
    ];
    let landmark_pairs = [
        ("eyes.left_center", "eyes.right_center"),
        ("eyes.left_pupil_center", "eyes.right_pupil_center"),
        ("eyes.left_inner_canthus", "eyes.right_inner_canthus"),
        ("eyes.left_outer_canthus", "eyes.right_outer_canthus"),
        ("eyes.left_upper_lid_peak", "eyes.right_upper_lid_peak"),
        ("eyes.left_lower_lid_peak", "eyes.right_lower_lid_peak"),
    ];
    let symmetries = vec![symmetry(
        "eyes.symmetry.ocular_midline",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::OcularMidline,
        &region_pairs,
        &landmark_pairs,
        &[],
        &[],
    )];

    let mut contracts = coverage_contracts("eyes", &regions, &landmarks, &symmetries, &loops);
    contracts.push(contract(
        "eyes.contract.aperture_nesting",
        TopologyContractKind::Aperture,
        "Orbital rim, iris ring, and pupil ring must remain nested per eye.",
        region_ids(&[
            "eyes.left_sclera",
            "eyes.left_iris",
            "eyes.left_pupil",
            "eyes.right_sclera",
            "eyes.right_iris",
            "eyes.right_pupil",
        ]),
        landmark_ids(&[
            "eyes.left_center",
            "eyes.left_pupil_center",
            "eyes.right_center",
            "eyes.right_pupil_center",
        ]),
        Vec::new(),
        loop_ids(&[
            "eyes.loop.left_orbital_rim",
            "eyes.loop.left_iris_ring",
            "eyes.loop.left_pupil_ring",
            "eyes.loop.right_orbital_rim",
            "eyes.loop.right_iris_ring",
            "eyes.loop.right_pupil_ring",
        ]),
    ));

    base(
        HUMANOID_EYES_BASE_ID,
        "Humanoid eyes",
        BaseSemanticDomain::Eyes,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

/// Humanoid teeth topology contract.
#[must_use]
pub fn humanoid_teeth_base() -> CharacterBaseTopology {
    let regions = vec![
        region(
            "teeth.upper_gum",
            "Upper gum",
            SemanticSide::Center,
            RegionKind::Arch,
            None,
            &[
                SemanticRegionRole::OralFeature,
                SemanticRegionRole::DentalArc,
            ],
        ),
        region(
            "teeth.lower_gum",
            "Lower gum",
            SemanticSide::Center,
            RegionKind::Arch,
            None,
            &[
                SemanticRegionRole::OralFeature,
                SemanticRegionRole::DentalArc,
            ],
        ),
        region(
            "teeth.upper_central_incisors",
            "Upper central incisors",
            SemanticSide::Center,
            RegionKind::Digit,
            Some("teeth.upper_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.lower_central_incisors",
            "Lower central incisors",
            SemanticSide::Center,
            RegionKind::Digit,
            Some("teeth.lower_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.upper_left_canine",
            "Upper left canine",
            SemanticSide::Left,
            RegionKind::Digit,
            Some("teeth.upper_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.upper_right_canine",
            "Upper right canine",
            SemanticSide::Right,
            RegionKind::Digit,
            Some("teeth.upper_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.lower_left_canine",
            "Lower left canine",
            SemanticSide::Left,
            RegionKind::Digit,
            Some("teeth.lower_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.lower_right_canine",
            "Lower right canine",
            SemanticSide::Right,
            RegionKind::Digit,
            Some("teeth.lower_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.upper_left_molars",
            "Upper left molars",
            SemanticSide::Left,
            RegionKind::Digit,
            Some("teeth.upper_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.upper_right_molars",
            "Upper right molars",
            SemanticSide::Right,
            RegionKind::Digit,
            Some("teeth.upper_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.lower_left_molars",
            "Lower left molars",
            SemanticSide::Left,
            RegionKind::Digit,
            Some("teeth.lower_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
        region(
            "teeth.lower_right_molars",
            "Lower right molars",
            SemanticSide::Right,
            RegionKind::Digit,
            Some("teeth.lower_gum"),
            &[SemanticRegionRole::Digit, SemanticRegionRole::DentalArc],
        ),
    ];

    let landmarks = vec![
        landmark(
            "teeth.upper_midline",
            "Upper dental midline",
            "teeth.upper_central_incisors",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "teeth.lower_midline",
            "Lower dental midline",
            "teeth.lower_central_incisors",
            SemanticSide::Center,
            LandmarkKind::Midline,
            None,
        ),
        landmark(
            "teeth.upper_left_canine_tip",
            "Upper left canine tip",
            "teeth.upper_left_canine",
            SemanticSide::Left,
            LandmarkKind::Tip,
            Some("teeth.upper_right_canine_tip"),
        ),
        landmark(
            "teeth.upper_right_canine_tip",
            "Upper right canine tip",
            "teeth.upper_right_canine",
            SemanticSide::Right,
            LandmarkKind::Tip,
            Some("teeth.upper_left_canine_tip"),
        ),
        landmark(
            "teeth.lower_left_canine_tip",
            "Lower left canine tip",
            "teeth.lower_left_canine",
            SemanticSide::Left,
            LandmarkKind::Tip,
            Some("teeth.lower_right_canine_tip"),
        ),
        landmark(
            "teeth.lower_right_canine_tip",
            "Lower right canine tip",
            "teeth.lower_right_canine",
            SemanticSide::Right,
            LandmarkKind::Tip,
            Some("teeth.lower_left_canine_tip"),
        ),
        landmark(
            "teeth.upper_left_molar_back",
            "Upper left molar back",
            "teeth.upper_left_molars",
            SemanticSide::Left,
            LandmarkKind::Boundary,
            Some("teeth.upper_right_molar_back"),
        ),
        landmark(
            "teeth.upper_right_molar_back",
            "Upper right molar back",
            "teeth.upper_right_molars",
            SemanticSide::Right,
            LandmarkKind::Boundary,
            Some("teeth.upper_left_molar_back"),
        ),
        landmark(
            "teeth.lower_left_molar_back",
            "Lower left molar back",
            "teeth.lower_left_molars",
            SemanticSide::Left,
            LandmarkKind::Boundary,
            Some("teeth.lower_right_molar_back"),
        ),
        landmark(
            "teeth.lower_right_molar_back",
            "Lower right molar back",
            "teeth.lower_right_molars",
            SemanticSide::Right,
            LandmarkKind::Boundary,
            Some("teeth.lower_left_molar_back"),
        ),
    ];

    let symmetries = vec![symmetry(
        "teeth.symmetry.dental_midline",
        SymmetryKind::BilateralMirror,
        SymmetryPlane::DentalMidline,
        &[
            ("teeth.upper_left_canine", "teeth.upper_right_canine"),
            ("teeth.lower_left_canine", "teeth.lower_right_canine"),
            ("teeth.upper_left_molars", "teeth.upper_right_molars"),
            ("teeth.lower_left_molars", "teeth.lower_right_molars"),
        ],
        &[
            (
                "teeth.upper_left_canine_tip",
                "teeth.upper_right_canine_tip",
            ),
            (
                "teeth.lower_left_canine_tip",
                "teeth.lower_right_canine_tip",
            ),
            (
                "teeth.upper_left_molar_back",
                "teeth.upper_right_molar_back",
            ),
            (
                "teeth.lower_left_molar_back",
                "teeth.lower_right_molar_back",
            ),
        ],
        &[
            "teeth.upper_gum",
            "teeth.lower_gum",
            "teeth.upper_central_incisors",
            "teeth.lower_central_incisors",
        ],
        &["teeth.upper_midline", "teeth.lower_midline"],
    )];

    let loops = vec![
        topology_loop(
            "teeth.loop.upper_arch",
            "Upper dental arch",
            TopologyLoopKind::Arch,
            &[
                "teeth.upper_gum",
                "teeth.upper_left_molars",
                "teeth.upper_left_canine",
                "teeth.upper_central_incisors",
                "teeth.upper_right_canine",
                "teeth.upper_right_molars",
            ],
            &[
                "teeth.upper_left_molar_back",
                "teeth.upper_left_canine_tip",
                "teeth.upper_midline",
                "teeth.upper_right_canine_tip",
                "teeth.upper_right_molar_back",
            ],
        ),
        topology_loop(
            "teeth.loop.lower_arch",
            "Lower dental arch",
            TopologyLoopKind::Arch,
            &[
                "teeth.lower_gum",
                "teeth.lower_left_molars",
                "teeth.lower_left_canine",
                "teeth.lower_central_incisors",
                "teeth.lower_right_canine",
                "teeth.lower_right_molars",
            ],
            &[
                "teeth.lower_left_molar_back",
                "teeth.lower_left_canine_tip",
                "teeth.lower_midline",
                "teeth.lower_right_canine_tip",
                "teeth.lower_right_molar_back",
            ],
        ),
        topology_loop(
            "teeth.loop.bite_line",
            "Bite line",
            TopologyLoopKind::Contour,
            &[
                "teeth.upper_central_incisors",
                "teeth.lower_central_incisors",
            ],
            &["teeth.upper_midline", "teeth.lower_midline"],
        ),
    ];

    let mut contracts = coverage_contracts("teeth", &regions, &landmarks, &symmetries, &loops);
    contracts.push(contract(
        "teeth.contract.arch_continuity",
        TopologyContractKind::ArchContinuity,
        "Upper and lower dental arches must remain continuous around the dental midline.",
        all_region_ids(&regions),
        landmark_ids(&["teeth.upper_midline", "teeth.lower_midline"]),
        Vec::new(),
        loop_ids(&["teeth.loop.upper_arch", "teeth.loop.lower_arch"]),
    ));

    base(
        HUMANOID_TEETH_BASE_ID,
        "Humanoid teeth",
        BaseSemanticDomain::Teeth,
        BaseTopologyCollections::new(regions, landmarks, symmetries, loops, contracts),
    )
}

struct BaseTopologyCollections {
    regions: Vec<BaseRegion>,
    landmarks: Vec<BaseLandmark>,
    symmetries: Vec<BaseSymmetry>,
    loops: Vec<BaseTopologyLoop>,
    contracts: Vec<TopologyContract>,
}

impl BaseTopologyCollections {
    fn new(
        regions: Vec<BaseRegion>,
        landmarks: Vec<BaseLandmark>,
        symmetries: Vec<BaseSymmetry>,
        loops: Vec<BaseTopologyLoop>,
        contracts: Vec<TopologyContract>,
    ) -> Self {
        Self {
            regions,
            landmarks,
            symmetries,
            loops,
            contracts,
        }
    }
}

fn base(
    id: &str,
    name: &str,
    domain: BaseSemanticDomain,
    collections: BaseTopologyCollections,
) -> CharacterBaseTopology {
    CharacterBaseTopology {
        schema_version: CHARACTER_GRAMMAR_SCHEMA_VERSION,
        base_version: CHARACTER_BASE_TOPOLOGY_VERSION,
        id: bid(id),
        name: name.to_owned(),
        domain,
        regions: collections.regions,
        landmarks: collections.landmarks,
        symmetries: collections.symmetries,
        loops: collections.loops,
        contracts: collections.contracts,
    }
}

fn region(
    id: &str,
    name: &str,
    side: SemanticSide,
    kind: RegionKind,
    parent: Option<&str>,
    roles: &[SemanticRegionRole],
) -> BaseRegion {
    region_owned(
        id.to_owned(),
        name.to_owned(),
        side,
        kind,
        parent.map(str::to_owned),
        roles,
    )
}

fn region_owned(
    id: String,
    name: String,
    side: SemanticSide,
    kind: RegionKind,
    parent: Option<String>,
    roles: &[SemanticRegionRole],
) -> BaseRegion {
    BaseRegion {
        id: rid(id),
        name,
        side,
        kind,
        parent: parent.map(rid),
        roles: roles.to_vec(),
    }
}

fn landmark(
    id: &str,
    name: &str,
    region: &str,
    side: SemanticSide,
    kind: LandmarkKind,
    mirror: Option<&str>,
) -> BaseLandmark {
    landmark_owned(
        id.to_owned(),
        name.to_owned(),
        region.to_owned(),
        side,
        kind,
        mirror.map(str::to_owned),
    )
}

fn landmark_owned(
    id: String,
    name: String,
    region: String,
    side: SemanticSide,
    kind: LandmarkKind,
    mirror: Option<String>,
) -> BaseLandmark {
    BaseLandmark {
        id: lid(id),
        name,
        region: rid(region),
        side,
        kind,
        mirror: mirror.map(lid),
    }
}

fn symmetry(
    id: &str,
    kind: SymmetryKind,
    plane: SymmetryPlane,
    region_pairs: &[(&str, &str)],
    landmark_pairs: &[(&str, &str)],
    fixed_regions: &[&str],
    fixed_landmarks: &[&str],
) -> BaseSymmetry {
    symmetry_owned(
        id,
        kind,
        plane,
        region_pairs
            .iter()
            .map(|(left, right)| RegionSymmetryPair {
                left: rid(*left),
                right: rid(*right),
            })
            .collect(),
        landmark_pairs
            .iter()
            .map(|(left, right)| LandmarkSymmetryPair {
                left: lid(*left),
                right: lid(*right),
            })
            .collect(),
        region_ids(fixed_regions),
        landmark_ids(fixed_landmarks),
    )
}

fn symmetry_owned(
    id: &str,
    kind: SymmetryKind,
    plane: SymmetryPlane,
    region_pairs: Vec<RegionSymmetryPair>,
    landmark_pairs: Vec<LandmarkSymmetryPair>,
    fixed_regions: Vec<CharacterRegionId>,
    fixed_landmarks: Vec<CharacterLandmarkId>,
) -> BaseSymmetry {
    BaseSymmetry {
        id: sid(id),
        kind,
        plane,
        region_pairs,
        landmark_pairs,
        fixed_regions,
        fixed_landmarks,
    }
}

fn topology_loop(
    id: &str,
    name: &str,
    kind: TopologyLoopKind,
    regions: &[&str],
    landmarks: &[&str],
) -> BaseTopologyLoop {
    topology_loop_owned(
        id.to_owned(),
        name.to_owned(),
        kind,
        region_ids(regions),
        landmark_ids(landmarks),
    )
}

fn topology_loop_owned(
    id: String,
    name: String,
    kind: TopologyLoopKind,
    regions: Vec<CharacterRegionId>,
    landmarks: Vec<CharacterLandmarkId>,
) -> BaseTopologyLoop {
    BaseTopologyLoop {
        id: loop_id(id),
        name,
        kind,
        closed: true,
        regions,
        landmarks,
    }
}

fn contract(
    id: &str,
    kind: TopologyContractKind,
    summary: &str,
    required_regions: Vec<CharacterRegionId>,
    required_landmarks: Vec<CharacterLandmarkId>,
    required_symmetries: Vec<CharacterSymmetryId>,
    required_loops: Vec<CharacterLoopId>,
) -> TopologyContract {
    TopologyContract {
        id: TopologyContractId(id.to_owned()),
        kind,
        summary: summary.to_owned(),
        required_regions,
        required_landmarks,
        required_symmetries,
        required_loops,
    }
}

fn coverage_contracts(
    prefix: &str,
    regions: &[BaseRegion],
    landmarks: &[BaseLandmark],
    symmetries: &[BaseSymmetry],
    loops: &[BaseTopologyLoop],
) -> Vec<TopologyContract> {
    vec![
        contract(
            &format!("{prefix}.contract.region_hierarchy"),
            TopologyContractKind::RegionHierarchy,
            "All semantic regions must remain present and parented as declared.",
            all_region_ids(regions),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        contract(
            &format!("{prefix}.contract.landmark_anchors"),
            TopologyContractKind::LandmarkAnchors,
            "All semantic landmarks must remain present and attached to their declared regions.",
            Vec::new(),
            all_landmark_ids(landmarks),
            Vec::new(),
            Vec::new(),
        ),
        contract(
            &format!("{prefix}.contract.symmetry"),
            TopologyContractKind::SymmetryCoverage,
            "Declared symmetry relationships must remain satisfiable.",
            Vec::new(),
            Vec::new(),
            all_symmetry_ids(symmetries),
            Vec::new(),
        ),
        contract(
            &format!("{prefix}.contract.loop_closure"),
            TopologyContractKind::LoopClosure,
            "Declared topology loops must remain closed and anchored.",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            all_loop_ids(loops),
        ),
    ]
}

fn bid(value: impl Into<String>) -> CharacterBaseId {
    CharacterBaseId(value.into())
}

fn rid(value: impl Into<String>) -> CharacterRegionId {
    CharacterRegionId(value.into())
}

fn lid(value: impl Into<String>) -> CharacterLandmarkId {
    CharacterLandmarkId(value.into())
}

fn sid(value: impl Into<String>) -> CharacterSymmetryId {
    CharacterSymmetryId(value.into())
}

fn loop_id(value: impl Into<String>) -> CharacterLoopId {
    CharacterLoopId(value.into())
}

fn region_ids(values: &[&str]) -> Vec<CharacterRegionId> {
    values.iter().copied().map(rid).collect()
}

fn landmark_ids(values: &[&str]) -> Vec<CharacterLandmarkId> {
    values.iter().copied().map(lid).collect()
}

fn loop_ids(values: &[&str]) -> Vec<CharacterLoopId> {
    values.iter().copied().map(loop_id).collect()
}

fn all_region_ids(regions: &[BaseRegion]) -> Vec<CharacterRegionId> {
    regions.iter().map(|region| region.id.clone()).collect()
}

fn all_landmark_ids(landmarks: &[BaseLandmark]) -> Vec<CharacterLandmarkId> {
    landmarks
        .iter()
        .map(|landmark| landmark.id.clone())
        .collect()
}

fn all_symmetry_ids(symmetries: &[BaseSymmetry]) -> Vec<CharacterSymmetryId> {
    symmetries
        .iter()
        .map(|symmetry| symmetry.id.clone())
        .collect()
}

fn all_loop_ids(loops: &[BaseTopologyLoop]) -> Vec<CharacterLoopId> {
    loops
        .iter()
        .map(|topology_loop| topology_loop.id.clone())
        .collect()
}

fn ensure_non_empty<T>(
    values: &[T],
    base: &str,
    collection: &'static str,
) -> Result<(), BaseTopologyError> {
    if values.is_empty() {
        Err(BaseTopologyError::EmptyCollection {
            base: base.to_owned(),
            collection,
        })
    } else {
        Ok(())
    }
}

fn ensure_unique(
    ids: &mut BTreeSet<String>,
    id: &str,
    base: &str,
    collection: &'static str,
) -> Result<(), BaseTopologyError> {
    if ids.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(BaseTopologyError::DuplicateId {
            base: base.to_owned(),
            collection,
            id: id.to_owned(),
        })
    }
}

fn ensure_reference(
    ids: &BTreeSet<String>,
    id: &str,
    base: &str,
    owner: &str,
    collection: &'static str,
) -> Result<(), BaseTopologyError> {
    if ids.contains(id) {
        Ok(())
    } else {
        Err(BaseTopologyError::MissingReference {
            base: base.to_owned(),
            owner: owner.to_owned(),
            collection,
            id: id.to_owned(),
        })
    }
}

fn ensure_unique_references(
    base: &str,
    owner: &str,
    collection: &'static str,
    values: impl IntoIterator<Item = String>,
) -> Result<(), BaseTopologyError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(BaseTopologyError::DuplicateReference {
                base: base.to_owned(),
                owner: owner.to_owned(),
                collection,
                id: value,
            });
        }
    }
    Ok(())
}

fn validate_region_hierarchy(base: &str, regions: &[BaseRegion]) -> Result<(), BaseTopologyError> {
    let parents = regions
        .iter()
        .map(|region| {
            (
                region.id.0.as_str(),
                region.parent.as_ref().map(|id| id.0.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for region in regions {
        let mut seen = BTreeSet::new();
        let mut current = Some(region.id.0.as_str());
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(BaseTopologyError::RegionHierarchyCycle {
                    base: base.to_owned(),
                    id: region.id.0.clone(),
                });
            }
            current = parents.get(id).copied().flatten();
        }
    }

    Ok(())
}

fn validate_symmetry_reference_uniqueness(
    base: &str,
    symmetry: &BaseSymmetry,
) -> Result<(), BaseTopologyError> {
    ensure_unique_references(
        base,
        &symmetry.id.0,
        "symmetry region pairs",
        symmetry
            .region_pairs
            .iter()
            .flat_map(|pair| [pair.left.0.clone(), pair.right.0.clone()]),
    )?;
    ensure_unique_references(
        base,
        &symmetry.id.0,
        "symmetry landmark pairs",
        symmetry
            .landmark_pairs
            .iter()
            .flat_map(|pair| [pair.left.0.clone(), pair.right.0.clone()]),
    )?;
    ensure_unique_references(
        base,
        &symmetry.id.0,
        "symmetry fixed regions",
        symmetry.fixed_regions.iter().map(|region| region.0.clone()),
    )?;
    ensure_unique_references(
        base,
        &symmetry.id.0,
        "symmetry fixed landmarks",
        symmetry
            .fixed_landmarks
            .iter()
            .map(|landmark| landmark.0.clone()),
    )
}

fn validate_symmetry_semantics(
    base: &str,
    symmetry: &BaseSymmetry,
    regions: &BTreeMap<&str, &BaseRegion>,
    landmarks: &BTreeMap<&str, &BaseLandmark>,
) -> Result<(), BaseTopologyError> {
    match symmetry.kind {
        SymmetryKind::BilateralMirror => {
            if symmetry.region_pairs.is_empty() && symmetry.landmark_pairs.is_empty() {
                return Err(BaseTopologyError::InvalidSymmetryShape {
                    base: base.to_owned(),
                    symmetry: symmetry.id.0.clone(),
                    kind: symmetry.kind.as_str(),
                });
            }
        }
        SymmetryKind::Midline => {
            if !symmetry.region_pairs.is_empty() || !symmetry.landmark_pairs.is_empty() {
                return Err(BaseTopologyError::InvalidSymmetryShape {
                    base: base.to_owned(),
                    symmetry: symmetry.id.0.clone(),
                    kind: symmetry.kind.as_str(),
                });
            }
            if symmetry.fixed_regions.is_empty() && symmetry.fixed_landmarks.is_empty() {
                return Err(BaseTopologyError::InvalidSymmetryShape {
                    base: base.to_owned(),
                    symmetry: symmetry.id.0.clone(),
                    kind: symmetry.kind.as_str(),
                });
            }
        }
    }

    for pair in &symmetry.region_pairs {
        let left = regions[pair.left.0.as_str()];
        let right = regions[pair.right.0.as_str()];
        if left.side != SemanticSide::Left || right.side != SemanticSide::Right {
            return Err(BaseTopologyError::InvalidSymmetrySide {
                base: base.to_owned(),
                symmetry: symmetry.id.0.clone(),
                id: pair.left.0.clone(),
            });
        }
    }

    for pair in &symmetry.landmark_pairs {
        let left = landmarks[pair.left.0.as_str()];
        let right = landmarks[pair.right.0.as_str()];
        if left.side != SemanticSide::Left
            || right.side != SemanticSide::Right
            || left.mirror.as_ref() != Some(&right.id)
            || right.mirror.as_ref() != Some(&left.id)
        {
            return Err(BaseTopologyError::InvalidSymmetrySide {
                base: base.to_owned(),
                symmetry: symmetry.id.0.clone(),
                id: pair.left.0.clone(),
            });
        }
    }

    for region in &symmetry.fixed_regions {
        let side = regions[region.0.as_str()].side;
        if !matches!(side, SemanticSide::Center | SemanticSide::Bilateral) {
            return Err(BaseTopologyError::InvalidSymmetrySide {
                base: base.to_owned(),
                symmetry: symmetry.id.0.clone(),
                id: region.0.clone(),
            });
        }
    }

    for landmark in &symmetry.fixed_landmarks {
        if landmarks[landmark.0.as_str()].side != SemanticSide::Center {
            return Err(BaseTopologyError::InvalidSymmetrySide {
                base: base.to_owned(),
                symmetry: symmetry.id.0.clone(),
                id: landmark.0.clone(),
            });
        }
    }

    Ok(())
}

fn ensure_covered(
    ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
    base: &str,
    collection: &'static str,
) -> Result<(), BaseTopologyError> {
    if let Some(id) = ids.difference(covered).next() {
        return Err(BaseTopologyError::UncoveredContractId {
            base: base.to_owned(),
            collection,
            id: id.clone(),
        });
    }
    Ok(())
}

fn feed_bool(hasher: &mut blake3::Hasher, value: bool) {
    hasher.update(&[u8::from(value)]);
}

fn feed_u16(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&value.to_le_bytes());
}

fn feed_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

fn feed_len(hasher: &mut blake3::Hasher, value: usize) {
    hasher.update(&(value as u64).to_le_bytes());
}

fn feed_str(hasher: &mut blake3::Hasher, value: &str) {
    feed_len(hasher, value.len());
    hasher.update(value.as_bytes());
}

fn feed_optional_str(hasher: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            feed_bool(hasher, true);
            feed_str(hasher, value);
        }
        None => feed_bool(hasher, false),
    }
}

fn feed_id_list<T>(
    hasher: &mut blake3::Hasher,
    len: usize,
    values: &[T],
    mut feed: impl FnMut(&mut blake3::Hasher, &T),
) {
    feed_len(hasher, len);
    for value in values {
        feed(hasher, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_bases_exist() {
        let library = base_topology_library();
        assert_eq!(library.bases.len(), REQUIRED_BASE_IDS.len());
        library
            .validate()
            .expect("built-in library should validate");

        for required_id in required_base_ids() {
            let base = builtin_character_base(required_id)
                .unwrap_or_else(|| panic!("missing required base {required_id}"));
            assert_eq!(base.id.0, *required_id);
        }

        let domains: BTreeSet<_> = library.bases.iter().map(|base| base.domain).collect();
        assert_eq!(
            domains,
            BTreeSet::from([
                BaseSemanticDomain::HumanoidBody,
                BaseSemanticDomain::Head,
                BaseSemanticDomain::Hands,
                BaseSemanticDomain::Feet,
                BaseSemanticDomain::Eyes,
                BaseSemanticDomain::Teeth,
            ])
        );
    }

    #[test]
    fn validation_rejects_missing_required_base() {
        let mut library = base_topology_library();
        library
            .bases
            .retain(|base| base.id.0 != HUMANOID_TEETH_BASE_ID);

        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::MissingRequiredBase { id, .. })
                if id == HUMANOID_TEETH_BASE_ID
        ));
    }

    #[test]
    fn validation_rejects_cross_base_duplicate_ids() {
        let mut library = base_topology_library();
        let duplicate_region = library.bases[0].regions[0].id.clone();
        let mut duplicate_base = library.bases[0].clone();
        duplicate_base.id = bid("base.test.cross_duplicate.v1");
        library.bases.push(duplicate_base);
        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::DuplicateId {
                base,
                collection: "regions",
                id,
            }) if base == "base topology library" && id == duplicate_region.0
        ));

        let mut library = base_topology_library();
        let duplicate_landmark = library.bases[0].landmarks[0].id.clone();
        let mut duplicate_base = library.bases[0].clone();
        duplicate_base.id = bid("base.test.cross_duplicate_landmark.v1");
        uniquify_region_ids(&mut duplicate_base, "cross_landmark");
        duplicate_base.validate().expect("test base stays valid");
        library.bases.push(duplicate_base);
        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::DuplicateId {
                base,
                collection: "landmarks",
                id,
            }) if base == "base topology library" && id == duplicate_landmark.0
        ));

        let mut library = base_topology_library();
        let duplicate_symmetry = library.bases[0].symmetries[0].id.clone();
        let mut duplicate_base = library.bases[0].clone();
        duplicate_base.id = bid("base.test.cross_duplicate_symmetry.v1");
        uniquify_region_ids(&mut duplicate_base, "cross_symmetry");
        uniquify_landmark_ids(&mut duplicate_base, "cross_symmetry");
        duplicate_base.validate().expect("test base stays valid");
        library.bases.push(duplicate_base);
        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::DuplicateId {
                base,
                collection: "symmetries",
                id,
            }) if base == "base topology library" && id == duplicate_symmetry.0
        ));

        let mut library = base_topology_library();
        let duplicate_loop = library.bases[0].loops[0].id.clone();
        let mut duplicate_base = library.bases[0].clone();
        duplicate_base.id = bid("base.test.cross_duplicate_loop.v1");
        uniquify_region_ids(&mut duplicate_base, "cross_loop");
        uniquify_landmark_ids(&mut duplicate_base, "cross_loop");
        uniquify_symmetry_ids(&mut duplicate_base, "cross_loop");
        duplicate_base.validate().expect("test base stays valid");
        library.bases.push(duplicate_base);
        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::DuplicateId {
                base,
                collection: "loops",
                id,
            }) if base == "base topology library" && id == duplicate_loop.0
        ));

        let mut library = base_topology_library();
        let duplicate_contract = library.bases[0].contracts[0].id.clone();
        let mut duplicate_base = library.bases[0].clone();
        duplicate_base.id = bid("base.test.cross_duplicate_contract.v1");
        uniquify_region_ids(&mut duplicate_base, "cross_contract");
        uniquify_landmark_ids(&mut duplicate_base, "cross_contract");
        uniquify_symmetry_ids(&mut duplicate_base, "cross_contract");
        uniquify_loop_ids(&mut duplicate_base, "cross_contract");
        duplicate_base.validate().expect("test base stays valid");
        library.bases.push(duplicate_base);
        assert!(matches!(
            library.validate(),
            Err(BaseTopologyError::DuplicateId {
                base,
                collection: "contracts",
                id,
            }) if base == "base topology library" && id == duplicate_contract.0
        ));
    }

    #[test]
    fn fingerprints_are_deterministic_and_content_sensitive() {
        let body = humanoid_body_base();
        assert_eq!(body.fingerprint(), body.fingerprint());
        assert_eq!(
            base_topology_library().fingerprint(),
            base_topology_library().fingerprint()
        );

        let mut changed = body.clone();
        changed.landmarks[0].name.push_str(" changed");
        assert_ne!(body.fingerprint(), changed.fingerprint());

        let fingerprinted = FingerprintedCharacterBase::new(body);
        assert!(fingerprinted.is_current());
    }

    #[test]
    fn topology_contracts_are_complete() {
        for base in builtin_character_bases() {
            base.validate()
                .unwrap_or_else(|error| panic!("{} failed validation: {error}", base.id.0));

            let covered_regions: BTreeSet<_> = base
                .contracts
                .iter()
                .flat_map(|contract| contract.required_regions.iter().map(|id| id.0.clone()))
                .collect();
            let covered_landmarks: BTreeSet<_> = base
                .contracts
                .iter()
                .flat_map(|contract| contract.required_landmarks.iter().map(|id| id.0.clone()))
                .collect();
            let covered_symmetries: BTreeSet<_> = base
                .contracts
                .iter()
                .flat_map(|contract| contract.required_symmetries.iter().map(|id| id.0.clone()))
                .collect();
            let covered_loops: BTreeSet<_> = base
                .contracts
                .iter()
                .flat_map(|contract| contract.required_loops.iter().map(|id| id.0.clone()))
                .collect();

            assert_eq!(
                covered_regions,
                id_set(base.regions.iter().map(|region| &region.id.0))
            );
            assert_eq!(
                covered_landmarks,
                id_set(base.landmarks.iter().map(|landmark| &landmark.id.0))
            );
            assert_eq!(
                covered_symmetries,
                id_set(base.symmetries.iter().map(|symmetry| &symmetry.id.0))
            );
            assert_eq!(
                covered_loops,
                id_set(base.loops.iter().map(|topology_loop| &topology_loop.id.0))
            );
        }
    }

    #[test]
    fn validation_rejects_open_loops_hierarchy_cycles_and_bad_symmetry_sides() {
        let mut open_loop = humanoid_body_base();
        let open_loop_id = open_loop.loops[0].id.0.clone();
        open_loop.loops[0].closed = false;
        assert!(matches!(
            open_loop.validate(),
            Err(BaseTopologyError::OpenTopologyLoop { id, .. }) if id == open_loop_id
        ));

        let mut cyclic = humanoid_body_base();
        let cyclic_region_id = cyclic.regions[0].id.clone();
        cyclic.regions[0].parent = Some(cyclic_region_id.clone());
        assert!(matches!(
            cyclic.validate(),
            Err(BaseTopologyError::RegionHierarchyCycle { id, .. })
                if id == cyclic_region_id.0
        ));

        let mut bad_symmetry = humanoid_body_base();
        let pair = bad_symmetry.symmetries[0].region_pairs[0].clone();
        bad_symmetry.symmetries[0].region_pairs[0].right =
            bad_symmetry.symmetries[0].fixed_regions[0].clone();
        assert!(matches!(
            bad_symmetry.validate(),
            Err(BaseTopologyError::InvalidSymmetrySide { id, .. }) if id == pair.left.0
        ));
    }

    #[test]
    fn validation_rejects_version_drift_self_mirrors_and_duplicate_references() {
        let mut future_library = base_topology_library();
        future_library.library_version.major += 1;
        assert!(matches!(
            future_library.validate(),
            Err(BaseTopologyError::InvalidTopologyVersion { .. })
        ));

        let mut future_base = humanoid_body_base();
        future_base.base_version.major += 1;
        assert!(matches!(
            future_base.validate(),
            Err(BaseTopologyError::InvalidTopologyVersion { .. })
        ));

        let mut self_mirror = humanoid_body_base();
        let landmark_id = self_mirror.landmarks[0].id.clone();
        self_mirror.landmarks[0].mirror = Some(landmark_id.clone());
        assert!(matches!(
            self_mirror.validate(),
            Err(BaseTopologyError::SelfMirror { id, .. }) if id == landmark_id.0
        ));

        let mut duplicate_roles = humanoid_body_base();
        let duplicated_role = duplicate_roles.regions[0].roles[0];
        duplicate_roles.regions[0].roles.push(duplicated_role);
        assert!(matches!(
            duplicate_roles.validate(),
            Err(BaseTopologyError::DuplicateReference {
                collection: "region roles",
                ..
            })
        ));

        let mut duplicate_loop_refs = humanoid_body_base();
        let duplicated_region = duplicate_loop_refs.loops[0].regions[0].clone();
        duplicate_loop_refs.loops[0].regions.push(duplicated_region);
        assert!(matches!(
            duplicate_loop_refs.validate(),
            Err(BaseTopologyError::DuplicateReference {
                collection: "loop regions",
                ..
            })
        ));

        let mut duplicate_contract_refs = humanoid_body_base();
        let duplicated_region = duplicate_contract_refs.contracts[0].required_regions[0].clone();
        duplicate_contract_refs.contracts[0]
            .required_regions
            .push(duplicated_region);
        assert!(matches!(
            duplicate_contract_refs.validate(),
            Err(BaseTopologyError::DuplicateReference {
                collection: "contract regions",
                ..
            })
        ));

        let mut duplicate_symmetry_refs = humanoid_body_base();
        let duplicated_pair = duplicate_symmetry_refs.symmetries[0].region_pairs[0].clone();
        duplicate_symmetry_refs.symmetries[0]
            .region_pairs
            .push(duplicated_pair);
        assert!(matches!(
            duplicate_symmetry_refs.validate(),
            Err(BaseTopologyError::DuplicateReference {
                collection: "symmetry region pairs",
                ..
            })
        ));

        let mut invalid_midline = humanoid_body_base();
        invalid_midline.symmetries[0].kind = SymmetryKind::Midline;
        assert!(matches!(
            invalid_midline.validate(),
            Err(BaseTopologyError::InvalidSymmetryShape { .. })
        ));
    }

    #[test]
    fn ids_are_unique() {
        let mut all_ids = BTreeSet::new();

        for base in builtin_character_bases() {
            assert_unique_test_id(&mut all_ids, &base.id.0);

            for region in &base.regions {
                assert_unique_test_id(&mut all_ids, &region.id.0);
            }
            for landmark in &base.landmarks {
                assert_unique_test_id(&mut all_ids, &landmark.id.0);
            }
            for symmetry in &base.symmetries {
                assert_unique_test_id(&mut all_ids, &symmetry.id.0);
            }
            for topology_loop in &base.loops {
                assert_unique_test_id(&mut all_ids, &topology_loop.id.0);
            }
            for contract in &base.contracts {
                assert_unique_test_id(&mut all_ids, &contract.id.0);
            }
        }
    }

    fn id_set<'a>(ids: impl Iterator<Item = &'a String>) -> BTreeSet<String> {
        ids.cloned().collect()
    }

    fn assert_unique_test_id(ids: &mut BTreeSet<String>, id: &str) {
        assert!(ids.insert(id.to_owned()), "duplicate id {id}");
    }

    fn uniquify_region_ids(base: &mut CharacterBaseTopology, suffix: &str) {
        let replacements = base
            .regions
            .iter()
            .map(|region| {
                (
                    region.id.clone(),
                    rid(format!("{}.{}", region.id.0, suffix)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for region in &mut base.regions {
            region.id = replacements[&region.id].clone();
            if let Some(parent) = &mut region.parent {
                *parent = replacements[parent].clone();
            }
        }
        for landmark in &mut base.landmarks {
            landmark.region = replacements[&landmark.region].clone();
        }
        for symmetry in &mut base.symmetries {
            for pair in &mut symmetry.region_pairs {
                pair.left = replacements[&pair.left].clone();
                pair.right = replacements[&pair.right].clone();
            }
            for region in &mut symmetry.fixed_regions {
                *region = replacements[region].clone();
            }
        }
        for topology_loop in &mut base.loops {
            for region in &mut topology_loop.regions {
                *region = replacements[region].clone();
            }
        }
        for contract in &mut base.contracts {
            for region in &mut contract.required_regions {
                *region = replacements[region].clone();
            }
        }
    }

    fn uniquify_landmark_ids(base: &mut CharacterBaseTopology, suffix: &str) {
        let replacements = base
            .landmarks
            .iter()
            .map(|landmark| {
                (
                    landmark.id.clone(),
                    lid(format!("{}.{}", landmark.id.0, suffix)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for landmark in &mut base.landmarks {
            landmark.id = replacements[&landmark.id].clone();
            if let Some(mirror) = &mut landmark.mirror {
                *mirror = replacements[mirror].clone();
            }
        }
        for symmetry in &mut base.symmetries {
            for pair in &mut symmetry.landmark_pairs {
                pair.left = replacements[&pair.left].clone();
                pair.right = replacements[&pair.right].clone();
            }
            for landmark in &mut symmetry.fixed_landmarks {
                *landmark = replacements[landmark].clone();
            }
        }
        for topology_loop in &mut base.loops {
            for landmark in &mut topology_loop.landmarks {
                *landmark = replacements[landmark].clone();
            }
        }
        for contract in &mut base.contracts {
            for landmark in &mut contract.required_landmarks {
                *landmark = replacements[landmark].clone();
            }
        }
    }

    fn uniquify_symmetry_ids(base: &mut CharacterBaseTopology, suffix: &str) {
        let replacements = base
            .symmetries
            .iter()
            .map(|symmetry| {
                (
                    symmetry.id.clone(),
                    sid(format!("{}.{}", symmetry.id.0, suffix)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for symmetry in &mut base.symmetries {
            symmetry.id = replacements[&symmetry.id].clone();
        }
        for contract in &mut base.contracts {
            for symmetry in &mut contract.required_symmetries {
                *symmetry = replacements[symmetry].clone();
            }
        }
    }

    fn uniquify_loop_ids(base: &mut CharacterBaseTopology, suffix: &str) {
        let replacements = base
            .loops
            .iter()
            .map(|topology_loop| {
                (
                    topology_loop.id.clone(),
                    loop_id(format!("{}.{}", topology_loop.id.0, suffix)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for topology_loop in &mut base.loops {
            topology_loop.id = replacements[&topology_loop.id].clone();
        }
        for contract in &mut base.contracts {
            for topology_loop in &mut contract.required_loops {
                *topology_loop = replacements[topology_loop].clone();
            }
        }
    }
}
