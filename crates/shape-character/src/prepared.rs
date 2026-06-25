//! Prepared known-base character customization contracts.
//!
//! Prepared templates are not arbitrary mesh imports. They describe a mesh that
//! has already been authored against the versioned character base library with
//! deformation cages, weight sets, and landmark constraints.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CharacterBaseId, CharacterControlId, CharacterLandmarkId, CharacterRegionId, ScalarRange,
    base::{
        BASE_TOPOLOGY_LIBRARY_VERSION, BaseFingerprint, BaseTopologyVersion, HUMANOID_BODY_BASE_ID,
        HUMANOID_HEAD_BASE_ID, base_topology_library, fingerprinted_character_bases,
    },
    proportion::CharacterPoseId,
};

/// Current schema for prepared character templates.
pub const PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION: u32 = 1;

/// Stable prepared deformation cage identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCageId(pub String);

/// Stable prepared weight-set identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedWeightSetId(pub String);

/// Current schema for prepared hero templates.
pub const PREPARED_HERO_TEMPLATE_SCHEMA_VERSION: u32 = 1;

const REQUIRED_HERO_PROVIDER_SLOTS: [&str; 9] = [
    "headgear",
    "shoulders",
    "torso_armor",
    "belt_skirt",
    "gauntlets",
    "boots",
    "weapon",
    "back_accessory",
    "hair_head_mass",
];

/// Prepared high-quality hero template contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedHeroTemplate {
    /// Hero template schema version.
    pub schema_version: u32,
    /// Stable template identifier.
    pub template_id: String,
    /// Product-facing name.
    pub display_name: String,
    /// Versioned base topology reference.
    pub base_topology: HeroBaseTopologyRef,
    /// Production-looking semantic topology descriptor.
    pub topology_descriptor: HeroTopologyDescriptor,
    /// Landmark inventory used by the prepared template.
    pub landmarks: Vec<HeroLandmarkBinding>,
    /// Semantic regions used by controls, cages, and providers.
    pub semantic_regions: Vec<HeroSemanticRegionBinding>,
    /// Hero deformation cages.
    pub cages: Vec<HeroDeformationCage>,
    /// Hero weight-set references.
    pub weight_sets: Vec<HeroWeightSet>,
    /// Future provider slots for gear and hair/head mass.
    pub provider_slots: Vec<HeroProviderSlot>,
    /// Novice-facing controls.
    pub control_profile: HeroControlProfile,
    /// Quality gate profile.
    pub quality_gate_profile: HeroQualityGateProfile,
    /// Review state.
    pub review_manifest: HeroTemplateReviewManifest,
    /// Unsupported operations reported honestly.
    pub unsupported_operations: Vec<HeroUnsupportedOperation>,
    /// Explicit external mesh claim, if a bad author tries to make one.
    pub external_mesh_claim: Option<String>,
    /// Embedded prepared character contract used by customization.
    pub prepared_character: PreparedCharacterTemplate,
}

impl PreparedHeroTemplate {
    /// Validate the prepared hero template against current semantic contracts.
    pub fn validate(&self) -> PreparedHeroResult<()> {
        if self.schema_version != PREPARED_HERO_TEMPLATE_SCHEMA_VERSION {
            return Err(PreparedHeroError::InvalidSchemaVersion {
                found: self.schema_version,
            });
        }
        validate_identifier("hero template id", &self.template_id)?;
        validate_label("hero display name", &self.display_name)?;
        self.prepared_character.validate()?;
        if self.prepared_character.template_id != self.base_topology.prepared_template_id {
            return Err(PreparedHeroError::PreparedTemplateMismatch);
        }
        self.base_topology.validate(&self.prepared_character)?;
        let prepared_regions = prepared_region_ids(&self.prepared_character);
        let prepared_landmarks = prepared_landmark_ids(&self.prepared_character);
        let prepared_cages = prepared_cage_ids(&self.prepared_character);
        let prepared_weights = prepared_weight_ids(&self.prepared_character);
        let prepared_weight_cages = prepared_weight_cage_ids(&self.prepared_character);
        let prepared_controls = prepared_control_ids(&self.prepared_character);

        self.topology_descriptor
            .validate(&prepared_landmarks, &prepared_regions)?;
        if self.external_mesh_claim.is_some() {
            return Err(PreparedHeroError::ExternalMeshClaim);
        }
        if self.prepared_character.novice_controls.len() > 7 {
            return Err(PreparedHeroError::TooManyPrimaryControls {
                count: self.prepared_character.novice_controls.len(),
                max: 7,
            });
        }

        validate_hero_landmarks(&self.landmarks, &prepared_landmarks)?;
        let semantic_regions = validate_hero_regions(&self.semantic_regions, &prepared_regions)?;
        let hero_cages = validate_hero_cages(&self.cages, &prepared_cages, &semantic_regions)?;
        validate_hero_weights(
            &self.weight_sets,
            &prepared_weight_cages,
            &hero_cages,
            &semantic_regions,
        )?;
        validate_hero_provider_slots(&self.provider_slots, &semantic_regions)?;
        self.control_profile.validate(
            &prepared_controls,
            &prepared_cages,
            &prepared_weights,
            &semantic_regions,
        )?;
        self.quality_gate_profile.validate()?;
        self.review_manifest.validate(&self.quality_gate_profile)?;
        ensure_unsupported_operations(&self.unsupported_operations)?;

        Ok(())
    }
}

/// Versioned hero base topology reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroBaseTopologyRef {
    /// Prepared character template ID.
    pub prepared_template_id: String,
    /// Character base library fingerprint.
    pub base_library_fingerprint: BaseFingerprint,
    /// Topology version.
    pub topology_version: BaseTopologyVersion,
    /// Required base references.
    pub required_bases: Vec<PreparedBaseReference>,
}

impl HeroBaseTopologyRef {
    fn validate(&self, prepared: &PreparedCharacterTemplate) -> PreparedHeroResult<()> {
        validate_identifier("prepared template id", &self.prepared_template_id)?;
        if self.base_library_fingerprint.0.is_empty() {
            return Err(PreparedHeroError::MissingBaseFingerprint);
        }
        if self.base_library_fingerprint != prepared.base_library_fingerprint {
            return Err(PreparedHeroError::BaseFingerprintMismatch);
        }
        if self.topology_version != BASE_TOPOLOGY_LIBRARY_VERSION
            || !self.topology_version.is_valid()
        {
            return Err(PreparedHeroError::UnrecognizedTopologyVersion {
                found: self.topology_version.to_string(),
            });
        }
        ensure_non_empty("hero required_bases", self.required_bases.len())?;
        let prepared_bases = prepared
            .bases
            .iter()
            .map(|base| (base.base_id.0.as_str(), base.fingerprint.as_str()))
            .collect::<BTreeMap<_, _>>();
        if self.required_bases.len() != prepared_bases.len() {
            let provided = self
                .required_bases
                .iter()
                .map(|base| base.base_id.0.clone())
                .collect::<BTreeSet<_>>();
            let missing = prepared
                .bases
                .iter()
                .find(|base| !provided.contains(&base.base_id.0))
                .map(|base| base.base_id.0.clone())
                .unwrap_or_else(|| "duplicate required base".to_owned());
            return Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::MissingRequiredBase { id: missing },
            ));
        }
        let mut required_base_ids = BTreeSet::new();
        for base in &self.required_bases {
            if !required_base_ids.insert(base.base_id.0.clone()) {
                return Err(PreparedHeroError::DuplicateId {
                    collection: "hero required bases",
                    id: base.base_id.0.clone(),
                });
            }
            let Some(fingerprint) = prepared_bases.get(base.base_id.0.as_str()) else {
                return Err(PreparedHeroError::UnknownReference {
                    field: "hero required base",
                    id: base.base_id.0.clone(),
                });
            };
            if *fingerprint != base.fingerprint.as_str() {
                return Err(PreparedHeroError::BaseFingerprintMismatch);
            }
        }
        Ok(())
    }
}

/// Production-looking topology descriptor for a prepared hero base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroTopologyDescriptor {
    /// Body loop descriptions.
    pub body_loops: Vec<String>,
    /// Shoulder, torso, and hip regions.
    pub shoulder_torso_hip_regions: Vec<CharacterRegionId>,
    /// Head and facial plane descriptions.
    pub head_and_facial_planes: Vec<String>,
    /// Hand and foot landmarks currently supported.
    pub hand_foot_landmarks: Vec<CharacterLandmarkId>,
    /// Stylized proportion notes.
    pub stylized_proportions: Vec<String>,
    /// Joint deformation loop descriptions.
    pub joint_deformation_loops: Vec<String>,
    /// Silhouette landmarks.
    pub silhouette_landmarks: Vec<CharacterLandmarkId>,
    /// Symmetry/asymmetry policy.
    pub symmetry_policy: String,
    /// Regions intended for armor and gear attachment.
    pub gear_attachment_regions: Vec<CharacterRegionId>,
}

impl HeroTopologyDescriptor {
    fn validate(
        &self,
        prepared_landmarks: &BTreeSet<String>,
        prepared_regions: &BTreeSet<String>,
    ) -> PreparedHeroResult<()> {
        ensure_non_empty("hero body loops", self.body_loops.len())?;
        ensure_non_empty(
            "hero shoulder torso hip regions",
            self.shoulder_torso_hip_regions.len(),
        )?;
        ensure_non_empty(
            "hero head and facial planes",
            self.head_and_facial_planes.len(),
        )?;
        ensure_non_empty("hero hand foot landmarks", self.hand_foot_landmarks.len())?;
        ensure_non_empty("hero stylized proportions", self.stylized_proportions.len())?;
        ensure_non_empty(
            "hero joint deformation loops",
            self.joint_deformation_loops.len(),
        )?;
        ensure_non_empty("hero silhouette landmarks", self.silhouette_landmarks.len())?;
        ensure_non_empty(
            "hero gear attachment regions",
            self.gear_attachment_regions.len(),
        )?;
        validate_label("hero symmetry policy", &self.symmetry_policy)?;
        ensure_known_regions(
            "hero shoulder torso hip region",
            &self.shoulder_torso_hip_regions,
            prepared_regions,
        )?;
        ensure_known_regions(
            "hero gear attachment region",
            &self.gear_attachment_regions,
            prepared_regions,
        )?;
        ensure_known_landmarks(
            "hero hand foot landmark",
            &self.hand_foot_landmarks,
            prepared_landmarks,
        )?;
        ensure_known_landmarks(
            "hero silhouette landmark",
            &self.silhouette_landmarks,
            prepared_landmarks,
        )?;
        Ok(())
    }
}

/// Hero landmark binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroLandmarkBinding {
    /// Landmark ID from the prepared contract.
    pub landmark_id: CharacterLandmarkId,
    /// Product-facing label.
    pub label: String,
    /// Whether the landmark is required for validation.
    pub required: bool,
}

/// Hero semantic region binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroSemanticRegionBinding {
    /// Region ID from the prepared contract.
    pub region_id: CharacterRegionId,
    /// Product-facing label.
    pub label: String,
    /// Whether the region is required.
    pub required: bool,
}

/// Hero deformation cage binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroDeformationCage {
    /// Prepared cage ID.
    pub cage_id: PreparedCageId,
    /// Product-facing label.
    pub label: String,
    /// Semantic regions influenced by the cage.
    pub semantic_regions: Vec<CharacterRegionId>,
}

/// Hero weight-set binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroWeightSet {
    /// Prepared weight-set ID.
    pub weight_set_id: PreparedWeightSetId,
    /// Cage driven by this weight set.
    pub cage_id: PreparedCageId,
    /// Semantic region influenced by this weight set.
    pub semantic_region: CharacterRegionId,
}

/// Future gear or hair/head-mass provider slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroProviderSlot {
    /// Stable slot ID.
    pub slot_id: String,
    /// Product-facing label.
    pub label: String,
    /// Semantic regions the slot may affect.
    pub semantic_regions: Vec<CharacterRegionId>,
    /// Whether the slot is required for v1 validation.
    pub required: bool,
    /// Whether this wave has populated providers for the slot.
    pub populated_in_v1: bool,
}

/// Hero control profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroControlProfile {
    /// Maximum primary novice controls.
    pub maximum_primary_controls: usize,
    /// Primary controls.
    pub controls: Vec<HeroControlBinding>,
}

impl HeroControlProfile {
    fn validate(
        &self,
        prepared_controls: &BTreeSet<String>,
        prepared_cages: &BTreeSet<String>,
        prepared_weights: &BTreeSet<String>,
        semantic_regions: &BTreeSet<String>,
    ) -> PreparedHeroResult<()> {
        if self.maximum_primary_controls > 7 || self.controls.len() > 7 {
            return Err(PreparedHeroError::TooManyPrimaryControls {
                count: self.controls.len(),
                max: 7,
            });
        }
        ensure_non_empty("hero controls", self.controls.len())?;
        let mut ids = BTreeSet::new();
        for control in &self.controls {
            validate_identifier("hero control id", &control.control_id.0)?;
            validate_label("hero control label", &control.label)?;
            if exposes_internal_id(&control.label) {
                return Err(PreparedHeroError::InternalMetadataExposed {
                    field: "hero control label",
                    value: control.label.clone(),
                });
            }
            if !ids.insert(control.control_id.0.clone()) {
                return Err(PreparedHeroError::DuplicateId {
                    collection: "hero controls",
                    id: control.control_id.0.clone(),
                });
            }
            if !prepared_controls.contains(&control.control_id.0) {
                return Err(PreparedHeroError::UnknownReference {
                    field: "hero control",
                    id: control.control_id.0.clone(),
                });
            }
            ensure_non_empty("hero control cages", control.cages.len())?;
            for cage in &control.cages {
                if !prepared_cages.contains(&cage.0) {
                    return Err(PreparedHeroError::UnknownReference {
                        field: "hero control cage",
                        id: cage.0.clone(),
                    });
                }
            }
            ensure_non_empty("hero control weight sets", control.weight_sets.len())?;
            for weights in &control.weight_sets {
                if !prepared_weights.contains(&weights.0) {
                    return Err(PreparedHeroError::UnknownReference {
                        field: "hero control weight set",
                        id: weights.0.clone(),
                    });
                }
            }
            ensure_non_empty(
                "hero control semantic regions",
                control.semantic_regions.len(),
            )?;
            for region in &control.semantic_regions {
                if !semantic_regions.contains(&region.0) {
                    return Err(PreparedHeroError::UnknownReference {
                        field: "hero control semantic region",
                        id: region.0.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Hero control binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroControlBinding {
    /// Prepared control ID.
    pub control_id: CharacterControlId,
    /// Product-facing label.
    pub label: String,
    /// Cages affected by this control.
    pub cages: Vec<PreparedCageId>,
    /// Weight sets affected by this control.
    pub weight_sets: Vec<PreparedWeightSetId>,
    /// Semantic regions affected by this control.
    pub semantic_regions: Vec<CharacterRegionId>,
}

/// Hero quality tier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroQualityTier {
    /// Internal only.
    Draft,
    /// Validated prototype; not production-ready.
    Prototype,
    /// Usable after complete evidence and manual review.
    Usable,
    /// Showcase after human and adversarial approval.
    Showcase,
}

/// Hero validation status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroValidationStatus {
    /// Contract validation passes, but full visual/mesh evidence is incomplete.
    PrototypeValidated,
    /// Contract is invalid.
    Invalid,
}

/// Hero quality gate profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroQualityGateProfile {
    /// Minimum tier claimed by the template.
    pub target_tier: HeroQualityTier,
    /// Validation status.
    pub validation_status: HeroValidationStatus,
    /// Evidence requirements.
    pub required_evidence: Vec<String>,
}

impl HeroQualityGateProfile {
    fn validate(&self) -> PreparedHeroResult<()> {
        ensure_non_empty("hero required evidence", self.required_evidence.len())?;
        if self.target_tier > HeroQualityTier::Prototype {
            return Err(PreparedHeroError::UnsupportedQualityClaim);
        }
        if self.validation_status != HeroValidationStatus::PrototypeValidated {
            return Err(PreparedHeroError::InvalidValidationStatus);
        }
        Ok(())
    }
}

/// Human review status for a hero template.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroHumanReviewStatus {
    /// Review is pending.
    Pending,
    /// Human approval was recorded.
    Approved,
}

/// Hero template review manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroTemplateReviewManifest {
    /// Tier achieved by current evidence.
    pub achieved_tier: HeroQualityTier,
    /// Human review status.
    pub human_review_status: HeroHumanReviewStatus,
    /// Whether contact-sheet evidence is present.
    pub contact_sheet_evidence: bool,
    /// Product-safe notes.
    pub notes: Vec<String>,
}

impl HeroTemplateReviewManifest {
    fn validate(&self, quality: &HeroQualityGateProfile) -> PreparedHeroResult<()> {
        if self.achieved_tier > quality.target_tier {
            return Err(PreparedHeroError::UnsupportedQualityClaim);
        }
        if self.achieved_tier == HeroQualityTier::Showcase
            && (self.human_review_status != HeroHumanReviewStatus::Approved
                || !self.contact_sheet_evidence)
        {
            return Err(PreparedHeroError::ShowcaseMissingReviewEvidence);
        }
        ensure_non_empty("hero review notes", self.notes.len())?;
        Ok(())
    }
}

/// Unsupported prepared hero operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroUnsupportedOperation {
    /// Arbitrary mesh import is not supported.
    ArbitraryMeshImport,
    /// Hidden raw mesh payloads are not supported.
    HiddenRawMeshPayload,
    /// Direct vertex editing is not supported.
    DirectVertexEdit,
    /// Textures and UVs are not supported.
    TexturesMaterialsUvs,
    /// Rigging and animation are not supported.
    RiggingAnimation,
    /// Dota/IP reconstruction is not supported.
    DotaIpReconstruction,
}

/// Prepared known-base character template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterTemplate {
    /// Template schema version.
    pub schema_version: u32,
    /// Stable template identifier.
    pub template_id: String,
    /// Human-readable template label.
    pub label: String,
    /// Character base library version this template was prepared against.
    pub base_library_version: BaseTopologyVersion,
    /// Character base library fingerprint this template was prepared against.
    pub base_library_fingerprint: BaseFingerprint,
    /// Exact base references required by the prepared mesh.
    pub bases: Vec<PreparedBaseReference>,
    /// Semantic landmark bindings required before customization.
    pub landmark_bindings: Vec<PreparedLandmarkBinding>,
    /// Deformation cages authored for the prepared mesh.
    pub cages: Vec<PreparedDeformationCage>,
    /// Weight sets binding mesh regions to cages.
    pub weight_sets: Vec<PreparedWeightSet>,
    /// Novice-facing whole-character controls.
    pub novice_controls: Vec<PreparedCharacterControl>,
}

impl PreparedCharacterTemplate {
    /// Validate the prepared template against the current known-base library.
    pub fn validate(&self) -> PreparedCharacterResult<()> {
        if self.schema_version != PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION {
            return Err(PreparedCharacterError::InvalidSchemaVersion {
                found: self.schema_version,
            });
        }
        validate_identifier("template_id", &self.template_id)?;
        validate_label("label", &self.label)?;
        if self.base_library_version != BASE_TOPOLOGY_LIBRARY_VERSION {
            return Err(PreparedCharacterError::StaleBaseLibraryVersion {
                found: self.base_library_version.to_string(),
                expected: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
            });
        }
        let library = base_topology_library();
        let expected_library_fingerprint = library.fingerprint();
        if self.base_library_fingerprint != expected_library_fingerprint {
            return Err(PreparedCharacterError::StaleBaseLibraryFingerprint {
                found: self.base_library_fingerprint.0.clone(),
                expected: expected_library_fingerprint.0,
            });
        }

        let known_bases = fingerprinted_character_bases()
            .into_iter()
            .map(|base| (base.base.id.0.clone(), base))
            .collect::<BTreeMap<_, _>>();
        let mut base_ids = BTreeSet::new();
        for base in &self.bases {
            validate_identifier("base id", &base.base_id.0)?;
            if !base_ids.insert(base.base_id.0.clone()) {
                return Err(PreparedCharacterError::DuplicateId {
                    collection: "bases",
                    id: base.base_id.0.clone(),
                });
            }
            let Some(known) = known_bases.get(&base.base_id.0) else {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "base",
                    id: base.base_id.0.clone(),
                });
            };
            if base.fingerprint != known.fingerprint {
                return Err(PreparedCharacterError::StaleBaseFingerprint {
                    base: base.base_id.0.clone(),
                    found: base.fingerprint.0.clone(),
                    expected: known.fingerprint.0.clone(),
                });
            }
        }
        if !base_ids.contains(HUMANOID_BODY_BASE_ID) {
            return Err(PreparedCharacterError::MissingRequiredBase {
                id: HUMANOID_BODY_BASE_ID.to_owned(),
            });
        }

        let known_region_ids_by_base = known_bases
            .values()
            .map(|base| {
                (
                    base.base.id.0.clone(),
                    base.base
                        .regions
                        .iter()
                        .map(|region| region.id.0.clone())
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let known_landmark_ids_by_base = known_bases
            .values()
            .map(|base| {
                (
                    base.base.id.0.clone(),
                    base.base
                        .landmarks
                        .iter()
                        .map(|landmark| landmark.id.0.clone())
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        ensure_non_empty("landmark_bindings", self.landmark_bindings.len())?;
        let mut landmark_ids = BTreeSet::new();
        let mut landmark_base_ids = BTreeMap::new();
        for binding in &self.landmark_bindings {
            binding.validate(&base_ids, &known_landmark_ids_by_base)?;
            if !landmark_ids.insert(binding.landmark_id.0.clone()) {
                return Err(PreparedCharacterError::DuplicateId {
                    collection: "landmark_bindings",
                    id: binding.landmark_id.0.clone(),
                });
            }
            landmark_base_ids.insert(binding.landmark_id.0.clone(), binding.base_id.0.clone());
        }

        ensure_non_empty("cages", self.cages.len())?;
        let mut cage_ids = BTreeSet::new();
        let mut cage_base_ids = BTreeMap::new();
        let mut cage_target_region_ids = BTreeMap::new();
        for cage in &self.cages {
            cage.validate(&base_ids, &known_region_ids_by_base, &landmark_base_ids)?;
            if !cage_ids.insert(cage.id.0.clone()) {
                return Err(PreparedCharacterError::DuplicateId {
                    collection: "cages",
                    id: cage.id.0.clone(),
                });
            }
            cage_base_ids.insert(cage.id.0.clone(), cage.base_id.0.clone());
            cage_target_region_ids.insert(
                cage.id.0.clone(),
                cage.target_regions
                    .iter()
                    .map(|region| region.0.clone())
                    .collect::<BTreeSet<_>>(),
            );
        }

        ensure_non_empty("weight_sets", self.weight_sets.len())?;
        let mut weight_ids = BTreeSet::new();
        let mut weight_cage_ids = BTreeMap::new();
        for weights in &self.weight_sets {
            weights.validate(
                &cage_base_ids,
                &cage_target_region_ids,
                &known_region_ids_by_base,
            )?;
            if !weight_ids.insert(weights.id.0.clone()) {
                return Err(PreparedCharacterError::DuplicateId {
                    collection: "weight_sets",
                    id: weights.id.0.clone(),
                });
            }
            weight_cage_ids.insert(weights.id.0.clone(), weights.cage_id.0.clone());
        }

        ensure_non_empty("novice_controls", self.novice_controls.len())?;
        if self.novice_controls.len() > 6 {
            return Err(PreparedCharacterError::TooManyPrimaryControls {
                count: self.novice_controls.len(),
                max: 6,
            });
        }
        let mut control_ids = BTreeSet::new();
        let mut control_kinds = BTreeSet::new();
        for control in &self.novice_controls {
            control.validate(&cage_ids, &weight_cage_ids)?;
            if !control_ids.insert(control.id.0.clone()) {
                return Err(PreparedCharacterError::DuplicateId {
                    collection: "novice_controls",
                    id: control.id.0.clone(),
                });
            }
            if !control_kinds.insert(control.kind) {
                return Err(PreparedCharacterError::DuplicateControlKind { kind: control.kind });
            }
        }
        for required in PreparedCharacterControlKind::ALL {
            if !control_kinds.contains(&required) {
                return Err(PreparedCharacterError::MissingControlKind { kind: required });
            }
        }

        Ok(())
    }
}

/// Exact base reference for a prepared template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedBaseReference {
    /// Versioned base ID.
    pub base_id: CharacterBaseId,
    /// Content fingerprint for that base.
    pub fingerprint: BaseFingerprint,
}

/// Landmark constraint binding for a prepared mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedLandmarkBinding {
    /// Landmark used by the prepared template.
    pub landmark_id: CharacterLandmarkId,
    /// Owning base ID.
    pub base_id: CharacterBaseId,
    /// Whether customization requires this landmark.
    pub required: bool,
    /// Maximum allowed landmark fit error in meters.
    pub max_error_meters: f32,
}

impl PreparedLandmarkBinding {
    fn validate(
        &self,
        base_ids: &BTreeSet<String>,
        known_landmark_ids_by_base: &BTreeMap<String, BTreeSet<String>>,
    ) -> PreparedCharacterResult<()> {
        validate_identifier("landmark id", &self.landmark_id.0)?;
        if !base_ids.contains(&self.base_id.0) {
            return Err(PreparedCharacterError::UnknownReference {
                field: "landmark base",
                id: self.base_id.0.clone(),
            });
        }
        let Some(known_landmark_ids) = known_landmark_ids_by_base.get(&self.base_id.0) else {
            return Err(PreparedCharacterError::UnknownReference {
                field: "landmark base",
                id: self.base_id.0.clone(),
            });
        };
        if !known_landmark_ids.contains(&self.landmark_id.0) {
            return Err(PreparedCharacterError::UnknownReference {
                field: "landmark",
                id: self.landmark_id.0.clone(),
            });
        }
        if !self.max_error_meters.is_finite() || self.max_error_meters < 0.0 {
            return Err(PreparedCharacterError::InvalidMetric {
                field: "landmark max error",
            });
        }
        Ok(())
    }
}

/// Deformation cage metadata for a prepared mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedDeformationCage {
    /// Stable cage ID.
    pub id: PreparedCageId,
    /// Base this cage is authored against.
    pub base_id: CharacterBaseId,
    /// Semantic regions affected by the cage.
    pub target_regions: Vec<CharacterRegionId>,
    /// Number of cage control points.
    pub control_point_count: usize,
    /// Source-space cage bounds in canonical meters.
    pub bounds_min: [f32; 3],
    /// Source-space cage bounds in canonical meters.
    pub bounds_max: [f32; 3],
    /// Landmark bindings used to constrain this cage.
    pub landmark_bindings: Vec<CharacterLandmarkId>,
}

impl PreparedDeformationCage {
    fn validate(
        &self,
        base_ids: &BTreeSet<String>,
        known_region_ids_by_base: &BTreeMap<String, BTreeSet<String>>,
        landmark_base_ids: &BTreeMap<String, String>,
    ) -> PreparedCharacterResult<()> {
        validate_identifier("cage id", &self.id.0)?;
        if !base_ids.contains(&self.base_id.0) {
            return Err(PreparedCharacterError::UnknownReference {
                field: "cage base",
                id: self.base_id.0.clone(),
            });
        }
        let Some(known_region_ids) = known_region_ids_by_base.get(&self.base_id.0) else {
            return Err(PreparedCharacterError::UnknownReference {
                field: "cage base",
                id: self.base_id.0.clone(),
            });
        };
        ensure_non_empty("cage target_regions", self.target_regions.len())?;
        ensure_unique_strings(
            "cage target_regions",
            self.target_regions.iter().map(|region| region.0.clone()),
        )?;
        for region in &self.target_regions {
            validate_identifier("region id", &region.0)?;
            if !known_region_ids.contains(&region.0) {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "region",
                    id: region.0.clone(),
                });
            }
        }
        if self.control_point_count < 4 {
            return Err(PreparedCharacterError::InvalidCount {
                field: "cage control points",
            });
        }
        if !bounds_are_valid(self.bounds_min, self.bounds_max) {
            return Err(PreparedCharacterError::InvalidBounds {
                field: "cage bounds",
            });
        }
        ensure_non_empty("cage landmark_bindings", self.landmark_bindings.len())?;
        ensure_unique_strings(
            "cage landmark_bindings",
            self.landmark_bindings
                .iter()
                .map(|landmark| landmark.0.clone()),
        )?;
        for landmark in &self.landmark_bindings {
            let Some(landmark_base_id) = landmark_base_ids.get(&landmark.0) else {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "cage landmark binding",
                    id: landmark.0.clone(),
                });
            };
            if landmark_base_id != &self.base_id.0 {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "cage landmark base",
                    id: landmark.0.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Weight-set metadata for a prepared mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedWeightSet {
    /// Stable weight-set ID.
    pub id: PreparedWeightSetId,
    /// Cage that drives this weight set.
    pub cage_id: PreparedCageId,
    /// Region influenced by this weight set.
    pub target_region: CharacterRegionId,
    /// Maximum cage influences on any prepared vertex.
    pub max_influences_per_vertex: u8,
    /// Maximum absolute error from a normalized weight sum of 1.0.
    pub normalized_weight_sum_epsilon: f32,
}

impl PreparedWeightSet {
    fn validate(
        &self,
        cage_base_ids: &BTreeMap<String, String>,
        cage_target_region_ids: &BTreeMap<String, BTreeSet<String>>,
        known_region_ids_by_base: &BTreeMap<String, BTreeSet<String>>,
    ) -> PreparedCharacterResult<()> {
        validate_identifier("weight set id", &self.id.0)?;
        let Some(cage_base_id) = cage_base_ids.get(&self.cage_id.0) else {
            return Err(PreparedCharacterError::UnknownReference {
                field: "weight set cage",
                id: self.cage_id.0.clone(),
            });
        };
        let Some(known_region_ids) = known_region_ids_by_base.get(cage_base_id) else {
            return Err(PreparedCharacterError::UnknownReference {
                field: "weight set cage base",
                id: cage_base_id.clone(),
            });
        };
        if !known_region_ids.contains(&self.target_region.0) {
            validate_identifier("weight set region", &self.target_region.0)?;
            return Err(PreparedCharacterError::UnknownReference {
                field: "weight set region",
                id: self.target_region.0.clone(),
            });
        }
        let Some(cage_regions) = cage_target_region_ids.get(&self.cage_id.0) else {
            return Err(PreparedCharacterError::UnknownReference {
                field: "weight set cage",
                id: self.cage_id.0.clone(),
            });
        };
        if !cage_regions.contains(&self.target_region.0) {
            return Err(PreparedCharacterError::UnknownReference {
                field: "weight set cage region",
                id: self.target_region.0.clone(),
            });
        }
        if self.max_influences_per_vertex == 0 {
            return Err(PreparedCharacterError::InvalidCount {
                field: "weight max influences",
            });
        }
        if !self.normalized_weight_sum_epsilon.is_finite()
            || !(0.0..=0.01).contains(&self.normalized_weight_sum_epsilon)
        {
            return Err(PreparedCharacterError::InvalidMetric {
                field: "weight normalized sum epsilon",
            });
        }
        Ok(())
    }
}

/// Novice-facing prepared character control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterControl {
    /// Stable control ID.
    pub id: CharacterControlId,
    /// Human-readable label.
    pub label: String,
    /// Product-level control kind.
    pub kind: PreparedCharacterControlKind,
    /// Control domain.
    pub domain: PreparedCharacterControlDomain,
    /// Cages affected by this control.
    pub affected_cages: Vec<PreparedCageId>,
    /// Weight sets affected by this control.
    pub affected_weight_sets: Vec<PreparedWeightSetId>,
}

impl PreparedCharacterControl {
    fn validate(
        &self,
        cage_ids: &BTreeSet<String>,
        weight_cage_ids: &BTreeMap<String, String>,
    ) -> PreparedCharacterResult<()> {
        validate_identifier("control id", &self.id.0)?;
        validate_label("control label", &self.label)?;
        if self.label != self.kind.required_label() {
            return Err(PreparedCharacterError::InvalidControlLabel {
                kind: self.kind,
                found: self.label.clone(),
                expected: self.kind.required_label().to_owned(),
            });
        }
        self.domain.validate(self.kind)?;
        ensure_non_empty("control affected_cages", self.affected_cages.len())?;
        ensure_unique_strings(
            "control affected_cages",
            self.affected_cages.iter().map(|cage| cage.0.clone()),
        )?;
        let affected_cage_ids = self
            .affected_cages
            .iter()
            .map(|cage| cage.0.clone())
            .collect::<BTreeSet<_>>();
        for cage in &self.affected_cages {
            if !cage_ids.contains(&cage.0) {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "control cage",
                    id: cage.0.clone(),
                });
            }
        }
        ensure_non_empty(
            "control affected_weight_sets",
            self.affected_weight_sets.len(),
        )?;
        ensure_unique_strings(
            "control affected_weight_sets",
            self.affected_weight_sets
                .iter()
                .map(|weights| weights.0.clone()),
        )?;
        let mut weighted_cage_ids = BTreeSet::new();
        for weights in &self.affected_weight_sets {
            let Some(weight_cage_id) = weight_cage_ids.get(&weights.0) else {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "control weight set",
                    id: weights.0.clone(),
                });
            };
            if !affected_cage_ids.contains(weight_cage_id) {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "control weight set cage",
                    id: weights.0.clone(),
                });
            }
            weighted_cage_ids.insert(weight_cage_id.clone());
        }
        for cage in &affected_cage_ids {
            if !weighted_cage_ids.contains(cage) {
                return Err(PreparedCharacterError::UnknownReference {
                    field: "control cage weight set",
                    id: cage.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Required novice control kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedCharacterControlKind {
    /// Whole body proportion controls.
    BodyProportions,
    /// Head and face shape controls.
    HeadShape,
    /// Garment fit controls.
    GarmentFit,
    /// Pose preset selector.
    PosePreset,
    /// Whole-character silhouette controls.
    Silhouette,
    /// Detail density control.
    DetailLevel,
}

impl PreparedCharacterControlKind {
    /// Required primary controls for the first prepared-character surface.
    pub const ALL: [Self; 6] = [
        Self::BodyProportions,
        Self::HeadShape,
        Self::GarmentFit,
        Self::PosePreset,
        Self::Silhouette,
        Self::DetailLevel,
    ];

    fn required_label(self) -> &'static str {
        match self {
            Self::BodyProportions => "Body Proportions",
            Self::HeadShape => "Head Shape",
            Self::GarmentFit => "Garment Fit",
            Self::PosePreset => "Pose Preset",
            Self::Silhouette => "Silhouette",
            Self::DetailLevel => "Detail Level",
        }
    }
}

/// Domain for a prepared character control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedCharacterControlDomain {
    /// Scalar whole-model control.
    Scalar {
        /// Inclusive finite scalar range.
        range: ScalarRange,
    },
    /// Choice among normalized pose presets.
    PosePreset {
        /// Stable default preset.
        default_preset: String,
        /// Available presets.
        presets: Vec<PreparedPosePreset>,
    },
}

impl PreparedCharacterControlDomain {
    fn validate(&self, kind: PreparedCharacterControlKind) -> PreparedCharacterResult<()> {
        match self {
            Self::Scalar { range } => {
                if kind == PreparedCharacterControlKind::PosePreset {
                    return Err(PreparedCharacterError::InvalidControlDomain {
                        control: "pose preset".to_owned(),
                    });
                }
                if !range.is_valid() {
                    return Err(PreparedCharacterError::InvalidScalarRange {
                        field: "control range",
                    });
                }
            }
            Self::PosePreset {
                default_preset,
                presets,
            } => {
                if kind != PreparedCharacterControlKind::PosePreset {
                    return Err(PreparedCharacterError::InvalidControlDomain {
                        control: "non-pose control".to_owned(),
                    });
                }
                validate_identifier("default pose preset", default_preset)?;
                ensure_non_empty("pose presets", presets.len())?;
                let mut preset_ids = BTreeSet::new();
                for preset in presets {
                    preset.validate()?;
                    if !preset_ids.insert(preset.id.clone()) {
                        return Err(PreparedCharacterError::DuplicateId {
                            collection: "pose presets",
                            id: preset.id.clone(),
                        });
                    }
                }
                if !preset_ids.contains(default_preset) {
                    return Err(PreparedCharacterError::UnknownReference {
                        field: "default pose preset",
                        id: default_preset.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Prepared pose preset entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedPosePreset {
    /// Stable preset ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Normalized pose program ID.
    pub normalized_pose: CharacterPoseId,
}

impl PreparedPosePreset {
    fn validate(&self) -> PreparedCharacterResult<()> {
        validate_identifier("pose preset id", &self.id)?;
        validate_label("pose preset label", &self.label)?;
        validate_identifier("normalized pose", &self.normalized_pose.0)
    }
}

/// Request to apply prepared whole-character controls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterCustomizationRequest {
    /// Target prepared template ID.
    pub template_id: String,
    /// Control values to apply.
    pub values: Vec<PreparedCharacterControlValue>,
}

/// One control value in a customization request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterControlValue {
    /// Control ID.
    pub control: CharacterControlId,
    /// Requested value.
    pub value: PreparedControlValue,
}

/// Supported prepared control value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedControlValue {
    /// Scalar control value.
    Scalar(f32),
    /// Choice value.
    Choice(String),
}

/// Deterministic prepared customization report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterCustomizationReport {
    /// True when the template and every requested control value were accepted.
    pub accepted: bool,
    /// Template ID.
    pub template_id: String,
    /// Base library fingerprint used for the customization.
    pub base_library_fingerprint: BaseFingerprint,
    /// Applied controls in deterministic control-ID order.
    pub applied_controls: Vec<PreparedAppliedControl>,
    /// Rejected controls or template errors.
    pub rejected_controls: Vec<PreparedControlRejection>,
    /// Normalized prepared customization program.
    pub program: PreparedCharacterCustomizationProgram,
}

/// One applied prepared control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAppliedControl {
    /// Control ID.
    pub control: CharacterControlId,
    /// Control kind.
    pub kind: PreparedCharacterControlKind,
    /// Canonical value.
    pub value: PreparedControlValue,
}

/// One rejected prepared control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedControlRejection {
    /// Control ID or template field.
    pub control: String,
    /// Deterministic reason.
    pub reason: String,
}

/// Normalized prepared customization program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCharacterCustomizationProgram {
    /// Program schema.
    pub schema_version: u32,
    /// Explicit policy: no raw mesh payload is carried by this program.
    pub mesh_payload_policy: PreparedMeshPayloadPolicy,
    /// Cage deltas generated by accepted controls.
    pub cage_deltas: Vec<PreparedCageDelta>,
    /// Landmark constraints preserved during customization.
    pub preserved_landmarks: Vec<CharacterLandmarkId>,
}

/// Mesh payload policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedMeshPayloadPolicy {
    /// Prepared customization references authored cages and weights only.
    NoRawMeshPayload,
}

/// Deterministic cage delta summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCageDelta {
    /// Cage affected by the control.
    pub cage: PreparedCageId,
    /// Weight sets affected on this cage.
    pub weight_sets: Vec<PreparedWeightSetId>,
    /// Deterministic normalized deformation magnitude.
    pub normalized_magnitude: f32,
}

/// Customize a prepared known-base character template.
#[must_use]
pub fn customize_prepared_character_template(
    template: &PreparedCharacterTemplate,
    request: &PreparedCharacterCustomizationRequest,
) -> PreparedCharacterCustomizationReport {
    let mut rejected_controls = Vec::new();
    if let Err(error) = template.validate() {
        rejected_controls.push(PreparedControlRejection {
            control: "template".to_owned(),
            reason: error.to_string(),
        });
    }
    if request.template_id != template.template_id {
        rejected_controls.push(PreparedControlRejection {
            control: "template_id".to_owned(),
            reason: "request targets a different prepared template".to_owned(),
        });
    }
    if !rejected_controls.is_empty() {
        return rejected_customization_report(template, rejected_controls);
    }

    let controls = template
        .novice_controls
        .iter()
        .map(|control| (control.id.0.clone(), control))
        .collect::<BTreeMap<_, _>>();
    let mut applied_by_id = BTreeMap::<String, PreparedAppliedControl>::new();
    let mut seen_values = BTreeSet::new();
    for value in &request.values {
        let control_id = value.control.0.clone();
        if !seen_values.insert(control_id.clone()) {
            rejected_controls.push(PreparedControlRejection {
                control: control_id,
                reason: "duplicate control value".to_owned(),
            });
            continue;
        }
        let Some(control) = controls.get(&control_id) else {
            rejected_controls.push(PreparedControlRejection {
                control: control_id,
                reason: "unknown prepared control".to_owned(),
            });
            continue;
        };
        if let Err(reason) = validate_control_value(control, &value.value) {
            rejected_controls.push(PreparedControlRejection {
                control: control_id,
                reason,
            });
            continue;
        }
        applied_by_id.insert(
            control_id,
            PreparedAppliedControl {
                control: value.control.clone(),
                kind: control.kind,
                value: value.value.clone(),
            },
        );
    }
    if !rejected_controls.is_empty() {
        return rejected_customization_report(template, rejected_controls);
    }
    for control in &template.novice_controls {
        if !applied_by_id.contains_key(&control.id.0) {
            applied_by_id.insert(
                control.id.0.clone(),
                PreparedAppliedControl {
                    control: control.id.clone(),
                    kind: control.kind,
                    value: default_control_value(control),
                },
            );
        }
    }

    let applied_controls = applied_by_id.into_values().collect::<Vec<_>>();
    let program = customization_program(template, &applied_controls);
    PreparedCharacterCustomizationReport {
        accepted: rejected_controls.is_empty(),
        template_id: template.template_id.clone(),
        base_library_fingerprint: template.base_library_fingerprint.clone(),
        applied_controls,
        rejected_controls,
        program,
    }
}

fn rejected_customization_report(
    template: &PreparedCharacterTemplate,
    rejected_controls: Vec<PreparedControlRejection>,
) -> PreparedCharacterCustomizationReport {
    PreparedCharacterCustomizationReport {
        accepted: false,
        template_id: template.template_id.clone(),
        base_library_fingerprint: template.base_library_fingerprint.clone(),
        applied_controls: Vec::new(),
        rejected_controls,
        program: empty_customization_program(),
    }
}

fn empty_customization_program() -> PreparedCharacterCustomizationProgram {
    PreparedCharacterCustomizationProgram {
        schema_version: PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION,
        mesh_payload_policy: PreparedMeshPayloadPolicy::NoRawMeshPayload,
        cage_deltas: Vec::new(),
        preserved_landmarks: Vec::new(),
    }
}

fn validate_control_value(
    control: &PreparedCharacterControl,
    value: &PreparedControlValue,
) -> Result<(), String> {
    match (&control.domain, value) {
        (PreparedCharacterControlDomain::Scalar { range }, PreparedControlValue::Scalar(value)) => {
            if value.is_finite() && *value >= range.min && *value <= range.max {
                Ok(())
            } else {
                Err("scalar value is outside the prepared control range".to_owned())
            }
        }
        (
            PreparedCharacterControlDomain::PosePreset { presets, .. },
            PreparedControlValue::Choice(value),
        ) => {
            if presets.iter().any(|preset| preset.id == *value) {
                Ok(())
            } else {
                Err("pose preset is not available for this prepared template".to_owned())
            }
        }
        _ => Err("control value kind does not match the prepared domain".to_owned()),
    }
}

fn default_control_value(control: &PreparedCharacterControl) -> PreparedControlValue {
    match &control.domain {
        PreparedCharacterControlDomain::Scalar { range } => {
            PreparedControlValue::Scalar(range.default)
        }
        PreparedCharacterControlDomain::PosePreset { default_preset, .. } => {
            PreparedControlValue::Choice(default_preset.clone())
        }
    }
}

fn customization_program(
    template: &PreparedCharacterTemplate,
    applied_controls: &[PreparedAppliedControl],
) -> PreparedCharacterCustomizationProgram {
    let controls = template
        .novice_controls
        .iter()
        .map(|control| (control.id.0.clone(), control))
        .collect::<BTreeMap<_, _>>();
    let weight_cage_ids = template
        .weight_sets
        .iter()
        .map(|weights| (weights.id.0.clone(), weights.cage_id.0.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut cage_delta_map = BTreeMap::<String, PreparedCageDelta>::new();
    for applied in applied_controls {
        let Some(control) = controls.get(&applied.control.0) else {
            continue;
        };
        let magnitude = normalized_magnitude(control, &applied.value);
        for cage in &control.affected_cages {
            let entry = cage_delta_map
                .entry(cage.0.clone())
                .or_insert_with(|| PreparedCageDelta {
                    cage: cage.clone(),
                    weight_sets: Vec::new(),
                    normalized_magnitude: 0.0,
                });
            entry.normalized_magnitude += magnitude;
            for weights in &control.affected_weight_sets {
                if weight_cage_ids.get(&weights.0) != Some(&cage.0) {
                    continue;
                }
                if !entry.weight_sets.iter().any(|id| id == weights) {
                    entry.weight_sets.push(weights.clone());
                }
            }
            entry.weight_sets.sort();
        }
    }
    let mut preserved_landmarks = template
        .landmark_bindings
        .iter()
        .filter(|binding| binding.required)
        .map(|binding| binding.landmark_id.clone())
        .collect::<Vec<_>>();
    preserved_landmarks.sort();
    PreparedCharacterCustomizationProgram {
        schema_version: PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION,
        mesh_payload_policy: PreparedMeshPayloadPolicy::NoRawMeshPayload,
        cage_deltas: cage_delta_map
            .into_values()
            .map(|mut delta| {
                delta.normalized_magnitude = round_metric(delta.normalized_magnitude.min(1.0));
                delta
            })
            .collect(),
        preserved_landmarks,
    }
}

fn normalized_magnitude(control: &PreparedCharacterControl, value: &PreparedControlValue) -> f32 {
    match (&control.domain, value) {
        (PreparedCharacterControlDomain::Scalar { range }, PreparedControlValue::Scalar(value)) => {
            let span = (range.max - range.min).max(f32::EPSILON);
            ((value - range.default).abs() / span).clamp(0.0, 1.0)
        }
        (
            PreparedCharacterControlDomain::PosePreset { default_preset, .. },
            PreparedControlValue::Choice(value),
        ) => f32::from(value != default_preset) * 0.25,
        _ => 0.0,
    }
}

/// Built-in prepared humanoid template for known-base customization tests.
#[must_use]
pub fn prepared_humanoid_template() -> PreparedCharacterTemplate {
    let library = base_topology_library();
    let bases = fingerprinted_character_bases()
        .into_iter()
        .map(|base| PreparedBaseReference {
            base_id: base.base.id,
            fingerprint: base.fingerprint,
        })
        .collect::<Vec<_>>();
    let body = CharacterBaseId(HUMANOID_BODY_BASE_ID.to_owned());
    let head = CharacterBaseId(HUMANOID_HEAD_BASE_ID.to_owned());
    let body_cage = PreparedCageId("prepared.cage.body".to_owned());
    let head_cage = PreparedCageId("prepared.cage.head".to_owned());
    let garment_cage = PreparedCageId("prepared.cage.garment_fit".to_owned());
    let detail_cage = PreparedCageId("prepared.cage.detail".to_owned());
    let body_weights = PreparedWeightSetId("prepared.weights.body".to_owned());
    let head_weights = PreparedWeightSetId("prepared.weights.head".to_owned());
    let garment_weights = PreparedWeightSetId("prepared.weights.garment_fit".to_owned());
    let detail_weights = PreparedWeightSetId("prepared.weights.detail".to_owned());

    PreparedCharacterTemplate {
        schema_version: PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION,
        template_id: "prepared.humanoid.known_base.v1".to_owned(),
        label: "Prepared Humanoid Known Base".to_owned(),
        base_library_version: BASE_TOPOLOGY_LIBRARY_VERSION,
        base_library_fingerprint: library.fingerprint(),
        bases,
        landmark_bindings: vec![
            landmark_binding("body.pelvis_center", &body, true),
            landmark_binding("body.neck_base", &body, true),
            landmark_binding("body.left_shoulder", &body, true),
            landmark_binding("body.right_shoulder", &body, true),
            landmark_binding("body.left_hip", &body, true),
            landmark_binding("body.right_hip", &body, true),
            landmark_binding("body.left_wrist", &body, false),
            landmark_binding("body.right_wrist", &body, false),
            landmark_binding("body.left_knee", &body, false),
            landmark_binding("body.right_knee", &body, false),
            landmark_binding("head.crown", &head, true),
            landmark_binding("head.brow_center", &head, true),
            landmark_binding("head.chin", &head, true),
        ],
        cages: vec![
            cage(
                &body_cage.0,
                &body,
                &[
                    "body.torso",
                    "body.pelvis",
                    "body.left_arm",
                    "body.right_arm",
                    "body.left_leg",
                    "body.right_leg",
                ],
                &[
                    "body.pelvis_center",
                    "body.neck_base",
                    "body.left_shoulder",
                    "body.right_shoulder",
                    "body.left_knee",
                    "body.right_knee",
                ],
                16,
            ),
            cage(
                &head_cage.0,
                &head,
                &["head.cranium", "head.face", "head.jaw"],
                &["head.crown", "head.brow_center", "head.chin"],
                8,
            ),
            cage(
                &garment_cage.0,
                &body,
                &["body.torso", "body.pelvis"],
                &["body.pelvis_center", "body.neck_base"],
                12,
            ),
            cage(
                &detail_cage.0,
                &body,
                &["body.torso", "body.left_arm", "body.right_arm"],
                &["body.left_shoulder", "body.right_shoulder"],
                10,
            ),
        ],
        weight_sets: vec![
            weights(&body_weights.0, &body_cage.0, "body.torso"),
            weights(&head_weights.0, &head_cage.0, "head.face"),
            weights(&garment_weights.0, &garment_cage.0, "body.torso"),
            weights(&detail_weights.0, &detail_cage.0, "body.torso"),
        ],
        novice_controls: vec![
            scalar_control(
                "prepared.control.body_proportions",
                "Body Proportions",
                PreparedCharacterControlKind::BodyProportions,
                ScalarRange {
                    min: -1.0,
                    max: 1.0,
                    default: 0.0,
                },
                &[&body_cage],
                &[&body_weights],
            ),
            scalar_control(
                "prepared.control.head_shape",
                "Head Shape",
                PreparedCharacterControlKind::HeadShape,
                ScalarRange {
                    min: -1.0,
                    max: 1.0,
                    default: 0.0,
                },
                &[&head_cage],
                &[&head_weights],
            ),
            scalar_control(
                "prepared.control.garment_fit",
                "Garment Fit",
                PreparedCharacterControlKind::GarmentFit,
                ScalarRange {
                    min: -0.5,
                    max: 0.5,
                    default: 0.0,
                },
                &[&garment_cage],
                &[&garment_weights],
            ),
            pose_control(
                "prepared.control.pose_preset",
                "Pose Preset",
                &[&body_cage],
                &[&body_weights],
            ),
            scalar_control(
                "prepared.control.silhouette",
                "Silhouette",
                PreparedCharacterControlKind::Silhouette,
                ScalarRange {
                    min: -1.0,
                    max: 1.0,
                    default: 0.0,
                },
                &[&body_cage, &head_cage],
                &[&body_weights, &head_weights],
            ),
            scalar_control(
                "prepared.control.detail_level",
                "Detail Level",
                PreparedCharacterControlKind::DetailLevel,
                ScalarRange {
                    min: 0.0,
                    max: 1.0,
                    default: 0.5,
                },
                &[&detail_cage],
                &[&detail_weights],
            ),
        ],
    }
}

/// Built-in prepared hero template v1.
#[must_use]
pub fn prepared_hero_template_v1() -> PreparedHeroTemplate {
    let prepared = prepared_humanoid_template();
    let body_regions = [
        "body.torso",
        "body.pelvis",
        "body.left_arm",
        "body.right_arm",
        "body.left_leg",
        "body.right_leg",
        "head.cranium",
        "head.face",
    ];
    let hero_regions = body_regions
        .iter()
        .map(|region| hero_region(region, &region.replace('.', " ")))
        .collect::<Vec<_>>();
    let hero_landmarks = [
        ("body.neck_base", "Neck base"),
        ("body.left_shoulder", "Left shoulder"),
        ("body.right_shoulder", "Right shoulder"),
        ("body.left_hip", "Left hip"),
        ("body.right_hip", "Right hip"),
        ("head.crown", "Crown"),
        ("head.brow_center", "Brow"),
        ("head.chin", "Chin"),
    ]
    .into_iter()
    .map(|(id, label)| hero_landmark(id, label, true))
    .collect::<Vec<_>>();
    let body_cage = PreparedCageId("prepared.cage.body".to_owned());
    let head_cage = PreparedCageId("prepared.cage.head".to_owned());
    let garment_cage = PreparedCageId("prepared.cage.garment_fit".to_owned());
    let detail_cage = PreparedCageId("prepared.cage.detail".to_owned());
    let body_weights = PreparedWeightSetId("prepared.weights.body".to_owned());
    let head_weights = PreparedWeightSetId("prepared.weights.head".to_owned());
    let garment_weights = PreparedWeightSetId("prepared.weights.garment_fit".to_owned());
    let detail_weights = PreparedWeightSetId("prepared.weights.detail".to_owned());

    PreparedHeroTemplate {
        schema_version: PREPARED_HERO_TEMPLATE_SCHEMA_VERSION,
        template_id: "prepared-hero-template-v1".to_owned(),
        display_name: "Prepared Hero Template v1".to_owned(),
        base_topology: HeroBaseTopologyRef {
            prepared_template_id: prepared.template_id.clone(),
            base_library_fingerprint: prepared.base_library_fingerprint.clone(),
            topology_version: prepared.base_library_version,
            required_bases: prepared.bases.clone(),
        },
        topology_descriptor: HeroTopologyDescriptor {
            body_loops: vec![
                "ribcage contour loop".to_owned(),
                "pelvis support loop".to_owned(),
                "shoulder cap loop pair".to_owned(),
                "hip-to-leg transition loops".to_owned(),
            ],
            shoulder_torso_hip_regions: vec![
                CharacterRegionId("body.left_arm".to_owned()),
                CharacterRegionId("body.right_arm".to_owned()),
                CharacterRegionId("body.torso".to_owned()),
                CharacterRegionId("body.pelvis".to_owned()),
            ],
            head_and_facial_planes: vec![
                "cranium mass plane".to_owned(),
                "brow-to-nose facial plane".to_owned(),
                "jaw and chin plane".to_owned(),
            ],
            hand_foot_landmarks: vec![
                CharacterLandmarkId("body.left_wrist".to_owned()),
                CharacterLandmarkId("body.right_wrist".to_owned()),
                CharacterLandmarkId("body.left_knee".to_owned()),
                CharacterLandmarkId("body.right_knee".to_owned()),
            ],
            stylized_proportions: vec![
                "heroic shoulder width".to_owned(),
                "compact waist and readable torso mass".to_owned(),
                "slightly oversized head planes for clay readability".to_owned(),
            ],
            joint_deformation_loops: vec![
                "neck base deformation loop".to_owned(),
                "shoulder bend support loops".to_owned(),
                "hip and pelvis transition loops".to_owned(),
            ],
            silhouette_landmarks: vec![
                CharacterLandmarkId("body.left_shoulder".to_owned()),
                CharacterLandmarkId("body.right_shoulder".to_owned()),
                CharacterLandmarkId("head.crown".to_owned()),
                CharacterLandmarkId("head.chin".to_owned()),
            ],
            symmetry_policy:
                "Default symmetric hero base with authored asymmetry allowed only through reviewed controls."
                    .to_owned(),
            gear_attachment_regions: body_regions
                .iter()
                .map(|region| CharacterRegionId((*region).to_owned()))
                .collect(),
        },
        landmarks: hero_landmarks,
        semantic_regions: hero_regions,
        cages: vec![
            hero_cage(
                &body_cage,
                "Body Proportions",
                &["body.torso", "body.pelvis", "body.left_arm", "body.right_arm"],
            ),
            hero_cage(
                &head_cage,
                "Head Shape",
                &["head.cranium", "head.face"],
            ),
            hero_cage(
                &garment_cage,
                "Garment / Armor Fit",
                &["body.torso", "body.pelvis"],
            ),
            hero_cage(
                &detail_cage,
                "Detail Level",
                &["body.torso", "body.left_arm", "body.right_arm"],
            ),
        ],
        weight_sets: vec![
            hero_weights(&body_weights, &body_cage, "body.torso"),
            hero_weights(&head_weights, &head_cage, "head.face"),
            hero_weights(&garment_weights, &garment_cage, "body.torso"),
            hero_weights(&detail_weights, &detail_cage, "body.torso"),
        ],
        provider_slots: vec![
            hero_slot("headgear", "Headgear", &["head.cranium", "head.face"], false),
            hero_slot(
                "shoulders",
                "Shoulders",
                &["body.left_arm", "body.right_arm"],
                false,
            ),
            hero_slot("torso_armor", "Torso Armor", &["body.torso"], false),
            hero_slot("belt_skirt", "Belt / Skirt", &["body.pelvis"], false),
            hero_slot(
                "gauntlets",
                "Gauntlets",
                &["body.left_arm", "body.right_arm"],
                false,
            ),
            hero_slot(
                "boots",
                "Boots",
                &["body.left_leg", "body.right_leg"],
                false,
            ),
            hero_slot("weapon", "Weapon", &["body.right_arm"], false),
            hero_slot("back_accessory", "Back Accessory", &["body.torso"], false),
            hero_slot(
                "hair_head_mass",
                "Hair / Head Mass",
                &["head.cranium"],
                false,
            ),
        ],
        control_profile: HeroControlProfile {
            maximum_primary_controls: 6,
            controls: prepared
                .novice_controls
                .iter()
                .map(hero_control)
                .collect(),
        },
        quality_gate_profile: HeroQualityGateProfile {
            target_tier: HeroQualityTier::Prototype,
            validation_status: HeroValidationStatus::PrototypeValidated,
            required_evidence: vec![
                "Prepared template contract validates".to_owned(),
                "Full clay mesh evidence is not available in v1".to_owned(),
                "Export/reopen remains unsupported until authored mesh output exists".to_owned(),
            ],
        },
        review_manifest: HeroTemplateReviewManifest {
            achieved_tier: HeroQualityTier::Prototype,
            human_review_status: HeroHumanReviewStatus::Pending,
            contact_sheet_evidence: false,
            notes: vec![
                "Prototype prepared hero contract; no Showcase claim.".to_owned(),
                "No arbitrary import, Dota/IP reconstruction, materials, UVs, rigging, or animation."
                    .to_owned(),
            ],
        },
        unsupported_operations: vec![
            HeroUnsupportedOperation::ArbitraryMeshImport,
            HeroUnsupportedOperation::HiddenRawMeshPayload,
            HeroUnsupportedOperation::DirectVertexEdit,
            HeroUnsupportedOperation::TexturesMaterialsUvs,
            HeroUnsupportedOperation::RiggingAnimation,
            HeroUnsupportedOperation::DotaIpReconstruction,
        ],
        external_mesh_claim: None,
        prepared_character: prepared,
    }
}

fn hero_landmark(id: &str, label: &str, required: bool) -> HeroLandmarkBinding {
    HeroLandmarkBinding {
        landmark_id: CharacterLandmarkId(id.to_owned()),
        label: label.to_owned(),
        required,
    }
}

fn hero_region(id: &str, label: &str) -> HeroSemanticRegionBinding {
    HeroSemanticRegionBinding {
        region_id: CharacterRegionId(id.to_owned()),
        label: label.to_owned(),
        required: true,
    }
}

fn hero_cage(
    cage_id: &PreparedCageId,
    label: &str,
    semantic_regions: &[&str],
) -> HeroDeformationCage {
    HeroDeformationCage {
        cage_id: cage_id.clone(),
        label: label.to_owned(),
        semantic_regions: semantic_regions
            .iter()
            .map(|region| CharacterRegionId((*region).to_owned()))
            .collect(),
    }
}

fn hero_weights(
    weight_set_id: &PreparedWeightSetId,
    cage_id: &PreparedCageId,
    semantic_region: &str,
) -> HeroWeightSet {
    HeroWeightSet {
        weight_set_id: weight_set_id.clone(),
        cage_id: cage_id.clone(),
        semantic_region: CharacterRegionId(semantic_region.to_owned()),
    }
}

fn hero_slot(
    slot_id: &str,
    label: &str,
    semantic_regions: &[&str],
    populated_in_v1: bool,
) -> HeroProviderSlot {
    HeroProviderSlot {
        slot_id: slot_id.to_owned(),
        label: label.to_owned(),
        semantic_regions: semantic_regions
            .iter()
            .map(|region| CharacterRegionId((*region).to_owned()))
            .collect(),
        required: false,
        populated_in_v1,
    }
}

fn hero_control(control: &PreparedCharacterControl) -> HeroControlBinding {
    let semantic_regions = match control.kind {
        PreparedCharacterControlKind::BodyProportions
        | PreparedCharacterControlKind::Silhouette
        | PreparedCharacterControlKind::PosePreset => {
            vec![
                "body.torso",
                "body.pelvis",
                "body.left_arm",
                "body.right_arm",
            ]
        }
        PreparedCharacterControlKind::HeadShape => vec!["head.cranium", "head.face"],
        PreparedCharacterControlKind::GarmentFit => vec!["body.torso", "body.pelvis"],
        PreparedCharacterControlKind::DetailLevel => {
            vec!["body.torso", "body.left_arm", "body.right_arm"]
        }
    };
    let label = match control.kind {
        PreparedCharacterControlKind::GarmentFit => "Garment / Armor Fit",
        _ => &control.label,
    };
    HeroControlBinding {
        control_id: control.id.clone(),
        label: label.to_owned(),
        cages: control.affected_cages.clone(),
        weight_sets: control.affected_weight_sets.clone(),
        semantic_regions: semantic_regions
            .into_iter()
            .map(|region| CharacterRegionId(region.to_owned()))
            .collect(),
    }
}

fn landmark_binding(
    landmark_id: &str,
    base_id: &CharacterBaseId,
    required: bool,
) -> PreparedLandmarkBinding {
    PreparedLandmarkBinding {
        landmark_id: CharacterLandmarkId(landmark_id.to_owned()),
        base_id: base_id.clone(),
        required,
        max_error_meters: 0.002,
    }
}

fn cage(
    id: &str,
    base_id: &CharacterBaseId,
    target_regions: &[&str],
    landmark_bindings: &[&str],
    control_point_count: usize,
) -> PreparedDeformationCage {
    PreparedDeformationCage {
        id: PreparedCageId(id.to_owned()),
        base_id: base_id.clone(),
        target_regions: target_regions
            .iter()
            .map(|region| CharacterRegionId((*region).to_owned()))
            .collect(),
        control_point_count,
        bounds_min: [-0.6, 0.0, -0.35],
        bounds_max: [0.6, 1.95, 0.4],
        landmark_bindings: landmark_bindings
            .iter()
            .map(|landmark| CharacterLandmarkId((*landmark).to_owned()))
            .collect(),
    }
}

fn weights(id: &str, cage_id: &str, target_region: &str) -> PreparedWeightSet {
    PreparedWeightSet {
        id: PreparedWeightSetId(id.to_owned()),
        cage_id: PreparedCageId(cage_id.to_owned()),
        target_region: CharacterRegionId(target_region.to_owned()),
        max_influences_per_vertex: 4,
        normalized_weight_sum_epsilon: 0.001,
    }
}

fn scalar_control(
    id: &str,
    label: &str,
    kind: PreparedCharacterControlKind,
    range: ScalarRange,
    cages: &[&PreparedCageId],
    weights: &[&PreparedWeightSetId],
) -> PreparedCharacterControl {
    PreparedCharacterControl {
        id: CharacterControlId(id.to_owned()),
        label: label.to_owned(),
        kind,
        domain: PreparedCharacterControlDomain::Scalar { range },
        affected_cages: cages.iter().map(|cage| (*cage).clone()).collect(),
        affected_weight_sets: weights.iter().map(|weight| (*weight).clone()).collect(),
    }
}

fn pose_control(
    id: &str,
    label: &str,
    cages: &[&PreparedCageId],
    weights: &[&PreparedWeightSetId],
) -> PreparedCharacterControl {
    PreparedCharacterControl {
        id: CharacterControlId(id.to_owned()),
        label: label.to_owned(),
        kind: PreparedCharacterControlKind::PosePreset,
        domain: PreparedCharacterControlDomain::PosePreset {
            default_preset: "neutral".to_owned(),
            presets: vec![
                PreparedPosePreset {
                    id: "neutral".to_owned(),
                    label: "Neutral".to_owned(),
                    normalized_pose: CharacterPoseId("pose.neutral".to_owned()),
                },
                PreparedPosePreset {
                    id: "contrapposto".to_owned(),
                    label: "Contrapposto".to_owned(),
                    normalized_pose: CharacterPoseId("pose.contrapposto".to_owned()),
                },
            ],
        },
        affected_cages: cages.iter().map(|cage| (*cage).clone()).collect(),
        affected_weight_sets: weights.iter().map(|weight| (*weight).clone()).collect(),
    }
}

fn prepared_region_ids(template: &PreparedCharacterTemplate) -> BTreeSet<String> {
    template
        .cages
        .iter()
        .flat_map(|cage| cage.target_regions.iter().map(|region| region.0.clone()))
        .chain(
            template
                .weight_sets
                .iter()
                .map(|weights| weights.target_region.0.clone()),
        )
        .collect()
}

fn prepared_landmark_ids(template: &PreparedCharacterTemplate) -> BTreeSet<String> {
    template
        .landmark_bindings
        .iter()
        .map(|landmark| landmark.landmark_id.0.clone())
        .collect()
}

fn prepared_cage_ids(template: &PreparedCharacterTemplate) -> BTreeSet<String> {
    template
        .cages
        .iter()
        .map(|cage| cage.id.0.clone())
        .collect()
}

fn prepared_weight_ids(template: &PreparedCharacterTemplate) -> BTreeSet<String> {
    template
        .weight_sets
        .iter()
        .map(|weights| weights.id.0.clone())
        .collect()
}

fn prepared_weight_cage_ids(template: &PreparedCharacterTemplate) -> BTreeMap<String, String> {
    template
        .weight_sets
        .iter()
        .map(|weights| (weights.id.0.clone(), weights.cage_id.0.clone()))
        .collect()
}

fn prepared_control_ids(template: &PreparedCharacterTemplate) -> BTreeSet<String> {
    template
        .novice_controls
        .iter()
        .map(|control| control.id.0.clone())
        .collect()
}

fn validate_hero_product_label(field: &'static str, value: &str) -> PreparedHeroResult<()> {
    validate_label(field, value)?;
    if exposes_internal_id(value) {
        return Err(PreparedHeroError::InternalMetadataExposed {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn ensure_known_regions(
    field: &'static str,
    regions: &[CharacterRegionId],
    prepared_regions: &BTreeSet<String>,
) -> PreparedHeroResult<()> {
    for region in regions {
        if !prepared_regions.contains(&region.0) {
            return Err(PreparedHeroError::UnknownReference {
                field,
                id: region.0.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_known_landmarks(
    field: &'static str,
    landmarks: &[CharacterLandmarkId],
    prepared_landmarks: &BTreeSet<String>,
) -> PreparedHeroResult<()> {
    for landmark in landmarks {
        if !prepared_landmarks.contains(&landmark.0) {
            return Err(PreparedHeroError::UnknownReference {
                field,
                id: landmark.0.clone(),
            });
        }
    }
    Ok(())
}

fn validate_hero_landmarks(
    landmarks: &[HeroLandmarkBinding],
    prepared_landmarks: &BTreeSet<String>,
) -> PreparedHeroResult<()> {
    ensure_non_empty("hero landmarks", landmarks.len())?;
    let mut ids = BTreeSet::new();
    let mut required_count = 0_usize;
    for landmark in landmarks {
        validate_identifier("hero landmark", &landmark.landmark_id.0)?;
        validate_hero_product_label("hero landmark label", &landmark.label)?;
        if !ids.insert(landmark.landmark_id.0.clone()) {
            return Err(PreparedHeroError::DuplicateId {
                collection: "hero landmarks",
                id: landmark.landmark_id.0.clone(),
            });
        }
        if !prepared_landmarks.contains(&landmark.landmark_id.0) {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero landmark",
                id: landmark.landmark_id.0.clone(),
            });
        }
        required_count += usize::from(landmark.required);
    }
    if required_count == 0 {
        return Err(PreparedHeroError::MissingRequiredLandmarks);
    }
    Ok(())
}

fn validate_hero_regions(
    regions: &[HeroSemanticRegionBinding],
    prepared_regions: &BTreeSet<String>,
) -> PreparedHeroResult<BTreeSet<String>> {
    ensure_non_empty("hero semantic regions", regions.len())?;
    let mut ids = BTreeSet::new();
    let mut required_count = 0_usize;
    for region in regions {
        validate_identifier("hero semantic region", &region.region_id.0)?;
        validate_hero_product_label("hero semantic region label", &region.label)?;
        if !ids.insert(region.region_id.0.clone()) {
            return Err(PreparedHeroError::DuplicateId {
                collection: "hero semantic regions",
                id: region.region_id.0.clone(),
            });
        }
        if !prepared_regions.contains(&region.region_id.0) {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero semantic region",
                id: region.region_id.0.clone(),
            });
        }
        required_count += usize::from(region.required);
    }
    if required_count == 0 {
        return Err(PreparedHeroError::MissingRequiredSemanticRegions);
    }
    Ok(ids)
}

fn validate_hero_cages(
    cages: &[HeroDeformationCage],
    prepared_cages: &BTreeSet<String>,
    semantic_regions: &BTreeSet<String>,
) -> PreparedHeroResult<BTreeSet<String>> {
    ensure_non_empty("hero cages", cages.len())?;
    let mut ids = BTreeSet::new();
    for cage in cages {
        validate_identifier("hero cage", &cage.cage_id.0)?;
        validate_hero_product_label("hero cage label", &cage.label)?;
        if !ids.insert(cage.cage_id.0.clone()) {
            return Err(PreparedHeroError::DuplicateId {
                collection: "hero cages",
                id: cage.cage_id.0.clone(),
            });
        }
        if !prepared_cages.contains(&cage.cage_id.0) {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero cage",
                id: cage.cage_id.0.clone(),
            });
        }
        ensure_non_empty("hero cage semantic regions", cage.semantic_regions.len())?;
        for region in &cage.semantic_regions {
            if !semantic_regions.contains(&region.0) {
                return Err(PreparedHeroError::UnknownReference {
                    field: "hero cage semantic region",
                    id: region.0.clone(),
                });
            }
        }
    }
    Ok(ids)
}

fn validate_hero_weights(
    weights: &[HeroWeightSet],
    prepared_weight_cages: &BTreeMap<String, String>,
    hero_cages: &BTreeSet<String>,
    semantic_regions: &BTreeSet<String>,
) -> PreparedHeroResult<()> {
    ensure_non_empty("hero weight sets", weights.len())?;
    let mut ids = BTreeSet::new();
    for weight in weights {
        validate_identifier("hero weight set", &weight.weight_set_id.0)?;
        if !ids.insert(weight.weight_set_id.0.clone()) {
            return Err(PreparedHeroError::DuplicateId {
                collection: "hero weight sets",
                id: weight.weight_set_id.0.clone(),
            });
        }
        let Some(expected_cage) = prepared_weight_cages.get(&weight.weight_set_id.0) else {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero weight set",
                id: weight.weight_set_id.0.clone(),
            });
        };
        if !hero_cages.contains(&weight.cage_id.0) {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero weight set cage",
                id: weight.cage_id.0.clone(),
            });
        }
        if expected_cage != &weight.cage_id.0 {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero weight set prepared cage",
                id: weight.cage_id.0.clone(),
            });
        }
        if !semantic_regions.contains(&weight.semantic_region.0) {
            return Err(PreparedHeroError::UnknownReference {
                field: "hero weight semantic region",
                id: weight.semantic_region.0.clone(),
            });
        }
    }
    Ok(())
}

fn validate_hero_provider_slots(
    slots: &[HeroProviderSlot],
    semantic_regions: &BTreeSet<String>,
) -> PreparedHeroResult<()> {
    ensure_non_empty("hero provider slots", slots.len())?;
    let mut ids = BTreeSet::new();
    for slot in slots {
        validate_identifier("hero provider slot", &slot.slot_id)?;
        validate_hero_product_label("hero provider slot label", &slot.label)?;
        if !ids.insert(slot.slot_id.clone()) {
            return Err(PreparedHeroError::DuplicateId {
                collection: "hero provider slots",
                id: slot.slot_id.clone(),
            });
        }
        ensure_non_empty(
            "hero provider slot semantic regions",
            slot.semantic_regions.len(),
        )?;
        for region in &slot.semantic_regions {
            if !semantic_regions.contains(&region.0) {
                return Err(PreparedHeroError::UnknownReference {
                    field: "hero provider slot semantic region",
                    id: region.0.clone(),
                });
            }
        }
    }
    for required in REQUIRED_HERO_PROVIDER_SLOTS {
        if !ids.contains(required) {
            return Err(PreparedHeroError::MissingRequiredProviderSlot {
                slot: required.to_owned(),
            });
        }
    }
    Ok(())
}

fn ensure_unsupported_operations(
    unsupported: &[HeroUnsupportedOperation],
) -> PreparedHeroResult<()> {
    ensure_non_empty("hero unsupported operations", unsupported.len())?;
    let set = unsupported.iter().copied().collect::<BTreeSet<_>>();
    for required in [
        HeroUnsupportedOperation::ArbitraryMeshImport,
        HeroUnsupportedOperation::HiddenRawMeshPayload,
        HeroUnsupportedOperation::DirectVertexEdit,
        HeroUnsupportedOperation::TexturesMaterialsUvs,
        HeroUnsupportedOperation::RiggingAnimation,
        HeroUnsupportedOperation::DotaIpReconstruction,
    ] {
        if !set.contains(&required) {
            return Err(PreparedHeroError::MissingUnsupportedOperation);
        }
    }
    Ok(())
}

fn exposes_internal_id(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "cage", "landmark", "weight", "semantic", "region", "provider", "scalar", "mesh", "vertex",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

/// Prepared hero validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PreparedHeroError {
    /// Unsupported schema version.
    #[error("unsupported prepared hero schema version {found}")]
    InvalidSchemaVersion {
        /// Found schema version.
        found: u32,
    },
    /// Wrapped prepared character error.
    #[error(transparent)]
    PreparedCharacter(#[from] PreparedCharacterError),
    /// Prepared template ID mismatch.
    #[error("hero base topology reference targets a different prepared template")]
    PreparedTemplateMismatch,
    /// Required base fingerprint is missing.
    #[error("prepared hero base fingerprint is required")]
    MissingBaseFingerprint,
    /// Base fingerprint does not match the prepared contract.
    #[error("prepared hero base fingerprint does not match the prepared character contract")]
    BaseFingerprintMismatch,
    /// Topology version is not recognized.
    #[error("prepared hero topology version {found} is not recognized")]
    UnrecognizedTopologyVersion {
        /// Found version.
        found: String,
    },
    /// Unknown reference.
    #[error("{field} references unknown id {id}")]
    UnknownReference {
        /// Field name.
        field: &'static str,
        /// Unknown ID.
        id: String,
    },
    /// Duplicate id.
    #[error("duplicate id {id} in {collection}")]
    DuplicateId {
        /// Collection.
        collection: &'static str,
        /// Duplicate ID.
        id: String,
    },
    /// Missing required landmarks.
    #[error("prepared hero template must declare required landmarks")]
    MissingRequiredLandmarks,
    /// Missing required semantic regions.
    #[error("prepared hero template must declare required semantic regions")]
    MissingRequiredSemanticRegions,
    /// Missing required provider slot.
    #[error("prepared hero template is missing required provider slot {slot}")]
    MissingRequiredProviderSlot {
        /// Missing slot ID.
        slot: String,
    },
    /// Too many primary controls.
    #[error("prepared hero template has {count} primary controls, maximum is {max}")]
    TooManyPrimaryControls {
        /// Found count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Product-facing metadata exposes internal implementation terms.
    #[error("{field} exposes internal metadata term in {value}")]
    InternalMetadataExposed {
        /// Field name.
        field: &'static str,
        /// Bad value.
        value: String,
    },
    /// Quality claim is unsupported.
    #[error("prepared hero template cannot claim this quality tier from v1 evidence")]
    UnsupportedQualityClaim,
    /// Invalid validation status.
    #[error("prepared hero validation status must be prototype_validated")]
    InvalidValidationStatus,
    /// Showcase lacks review evidence.
    #[error("prepared hero Showcase requires human approval and contact-sheet evidence")]
    ShowcaseMissingReviewEvidence,
    /// Unsupported operation inventory is incomplete.
    #[error("prepared hero unsupported operation inventory is incomplete")]
    MissingUnsupportedOperation,
    /// External mesh claim is not allowed.
    #[error("prepared hero template must not claim arbitrary external mesh reconstruction")]
    ExternalMeshClaim,
}

/// Prepared hero validation result.
pub type PreparedHeroResult<T> = Result<T, PreparedHeroError>;

/// Prepared character validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PreparedCharacterError {
    /// Unsupported template schema version.
    #[error("unsupported prepared character schema version {found}")]
    InvalidSchemaVersion {
        /// Found schema version.
        found: u32,
    },
    /// Required text field was empty or unstable.
    #[error("{field} must be a stable nonempty identifier")]
    InvalidIdentifier {
        /// Field name.
        field: &'static str,
    },
    /// Human-facing label was empty.
    #[error("{field} must not be empty")]
    InvalidLabel {
        /// Field name.
        field: &'static str,
    },
    /// Base library version is stale.
    #[error("prepared base library version {found} does not match current {expected}")]
    StaleBaseLibraryVersion {
        /// Found version.
        found: String,
        /// Expected version.
        expected: String,
    },
    /// Base library fingerprint is stale.
    #[error("prepared base library fingerprint {found} does not match current {expected}")]
    StaleBaseLibraryFingerprint {
        /// Found fingerprint.
        found: String,
        /// Expected fingerprint.
        expected: String,
    },
    /// Base fingerprint is stale.
    #[error("prepared base {base} fingerprint {found} does not match current {expected}")]
    StaleBaseFingerprint {
        /// Base ID.
        base: String,
        /// Found fingerprint.
        found: String,
        /// Expected fingerprint.
        expected: String,
    },
    /// Required base is missing.
    #[error("prepared template is missing required base {id}")]
    MissingRequiredBase {
        /// Required base ID.
        id: String,
    },
    /// Collection is empty.
    #[error("{collection} must not be empty")]
    EmptyCollection {
        /// Collection name.
        collection: &'static str,
    },
    /// Duplicate ID in a collection.
    #[error("duplicate id {id} in {collection}")]
    DuplicateId {
        /// Collection name.
        collection: &'static str,
        /// Duplicate ID.
        id: String,
    },
    /// Duplicate primary control kind.
    #[error("duplicate prepared control kind {kind:?}")]
    DuplicateControlKind {
        /// Duplicated kind.
        kind: PreparedCharacterControlKind,
    },
    /// Required primary control kind is missing.
    #[error("missing prepared control kind {kind:?}")]
    MissingControlKind {
        /// Missing kind.
        kind: PreparedCharacterControlKind,
    },
    /// Too many primary controls.
    #[error("prepared template has {count} primary controls, maximum is {max}")]
    TooManyPrimaryControls {
        /// Found count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Unknown reference.
    #[error("{field} references unknown id {id}")]
    UnknownReference {
        /// Field name.
        field: &'static str,
        /// Unknown ID.
        id: String,
    },
    /// Invalid metric.
    #[error("{field} metric is invalid")]
    InvalidMetric {
        /// Field name.
        field: &'static str,
    },
    /// Invalid count.
    #[error("{field} count is invalid")]
    InvalidCount {
        /// Field name.
        field: &'static str,
    },
    /// Invalid bounds.
    #[error("{field} are invalid")]
    InvalidBounds {
        /// Field name.
        field: &'static str,
    },
    /// Scalar range is invalid.
    #[error("{field} is not a finite ordered scalar range")]
    InvalidScalarRange {
        /// Field name.
        field: &'static str,
    },
    /// Control label does not match the approved product surface.
    #[error("invalid label {found} for prepared control kind {kind:?}; expected {expected}")]
    InvalidControlLabel {
        /// Control kind.
        kind: PreparedCharacterControlKind,
        /// Found label.
        found: String,
        /// Expected label.
        expected: String,
    },
    /// Control kind and domain disagree.
    #[error("invalid domain for prepared control {control}")]
    InvalidControlDomain {
        /// Control description.
        control: String,
    },
}

/// Prepared character validation result.
pub type PreparedCharacterResult<T> = Result<T, PreparedCharacterError>;

fn validate_identifier(field: &'static str, value: &str) -> PreparedCharacterResult<()> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(PreparedCharacterError::InvalidIdentifier { field })
    }
}

fn validate_label(field: &'static str, value: &str) -> PreparedCharacterResult<()> {
    if value.trim().is_empty() {
        Err(PreparedCharacterError::InvalidLabel { field })
    } else {
        Ok(())
    }
}

fn ensure_non_empty(collection: &'static str, count: usize) -> PreparedCharacterResult<()> {
    if count == 0 {
        Err(PreparedCharacterError::EmptyCollection { collection })
    } else {
        Ok(())
    }
}

fn ensure_unique_strings(
    collection: &'static str,
    values: impl IntoIterator<Item = String>,
) -> PreparedCharacterResult<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(PreparedCharacterError::DuplicateId {
                collection,
                id: value,
            });
        }
    }
    Ok(())
}

fn bounds_are_valid(min: [f32; 3], max: [f32; 3]) -> bool {
    min.iter().all(|value| value.is_finite())
        && max.iter().all(|value| value.is_finite())
        && min[0] < max[0]
        && min[1] < max[1]
        && min[2] < max[2]
}

fn round_metric(value: f32) -> f32 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_humanoid_template_validates_and_records_base_fingerprints() {
        let template = prepared_humanoid_template();

        template.validate().expect("prepared template validates");

        assert_eq!(
            template.schema_version,
            PREPARED_CHARACTER_TEMPLATE_SCHEMA_VERSION
        );
        assert_eq!(template.base_library_version, BASE_TOPOLOGY_LIBRARY_VERSION);
        assert_eq!(
            template.base_library_fingerprint,
            base_topology_library().fingerprint()
        );
        assert_eq!(template.novice_controls.len(), 6);
        for kind in PreparedCharacterControlKind::ALL {
            assert!(
                template
                    .novice_controls
                    .iter()
                    .any(|control| control.kind == kind)
            );
        }
    }

    #[test]
    fn prepared_hero_template_v1_validates_and_roundtrips() {
        let template = prepared_hero_template_v1();

        template
            .validate()
            .expect("prepared hero template validates");
        assert_eq!(
            template.schema_version,
            PREPARED_HERO_TEMPLATE_SCHEMA_VERSION
        );
        assert_eq!(template.template_id, "prepared-hero-template-v1");
        assert_eq!(
            template.base_topology.base_library_fingerprint,
            base_topology_library().fingerprint()
        );
        assert_eq!(
            template.base_topology.topology_version,
            BASE_TOPOLOGY_LIBRARY_VERSION
        );
        assert_eq!(template.control_profile.controls.len(), 6);
        assert!(template.control_profile.controls.len() <= 7);
        let labels = template
            .control_profile
            .controls
            .iter()
            .map(|control| control.label.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            labels,
            BTreeSet::from([
                "Body Proportions",
                "Head Shape",
                "Garment / Armor Fit",
                "Pose Preset",
                "Silhouette",
                "Detail Level"
            ])
        );

        let json = serde_json::to_string(&template).expect("serialize prepared hero template");
        let roundtrip: PreparedHeroTemplate =
            serde_json::from_str(&json).expect("deserialize prepared hero template");
        assert_eq!(roundtrip, template);
        roundtrip.validate().expect("roundtrip validates");
    }

    #[test]
    fn prepared_hero_requires_base_fingerprint_and_topology_version() {
        let mut missing_fingerprint = prepared_hero_template_v1();
        missing_fingerprint.base_topology.base_library_fingerprint = BaseFingerprint(String::new());
        assert_eq!(
            missing_fingerprint.validate(),
            Err(PreparedHeroError::MissingBaseFingerprint)
        );

        let mut stale_fingerprint = prepared_hero_template_v1();
        stale_fingerprint.base_topology.base_library_fingerprint =
            BaseFingerprint("stale".to_owned());
        assert_eq!(
            stale_fingerprint.validate(),
            Err(PreparedHeroError::BaseFingerprintMismatch)
        );

        let mut missing_version = prepared_hero_template_v1();
        missing_version.base_topology.topology_version = BaseTopologyVersion {
            major: 0,
            minor: 0,
            patch: 0,
        };
        assert_eq!(
            missing_version.validate(),
            Err(PreparedHeroError::UnrecognizedTopologyVersion {
                found: "0.0.0".to_owned()
            })
        );
    }

    #[test]
    fn prepared_hero_requires_exact_base_inventory() {
        let mut missing_base = prepared_hero_template_v1();
        let removed = missing_base
            .base_topology
            .required_bases
            .pop()
            .expect("base reference");
        assert_eq!(
            missing_base.validate(),
            Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::MissingRequiredBase {
                    id: removed.base_id.0
                }
            ))
        );

        let mut duplicate_base = prepared_hero_template_v1();
        duplicate_base.base_topology.required_bases[1] =
            duplicate_base.base_topology.required_bases[0].clone();
        assert_eq!(
            duplicate_base.validate(),
            Err(PreparedHeroError::DuplicateId {
                collection: "hero required bases",
                id: duplicate_base.base_topology.required_bases[0]
                    .base_id
                    .0
                    .clone()
            })
        );
    }

    #[test]
    fn prepared_hero_requires_landmark_region_and_cage_bindings() {
        let mut missing_landmarks = prepared_hero_template_v1();
        missing_landmarks.landmarks.clear();
        assert_eq!(
            missing_landmarks.validate(),
            Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::EmptyCollection {
                    collection: "hero landmarks"
                }
            ))
        );

        let mut missing_regions = prepared_hero_template_v1();
        missing_regions.semantic_regions.clear();
        assert_eq!(
            missing_regions.validate(),
            Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::EmptyCollection {
                    collection: "hero semantic regions"
                }
            ))
        );

        let mut missing_cages = prepared_hero_template_v1();
        missing_cages.cages.clear();
        assert_eq!(
            missing_cages.validate(),
            Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::EmptyCollection {
                    collection: "hero cages"
                }
            ))
        );
    }

    #[test]
    fn prepared_hero_validates_topology_descriptor_references() {
        let mut bad_region = prepared_hero_template_v1();
        bad_region.topology_descriptor.gear_attachment_regions[0] =
            CharacterRegionId("body.unprepared".to_owned());
        assert_eq!(
            bad_region.validate(),
            Err(PreparedHeroError::UnknownReference {
                field: "hero gear attachment region",
                id: "body.unprepared".to_owned()
            })
        );

        let mut bad_landmark = prepared_hero_template_v1();
        bad_landmark.topology_descriptor.hand_foot_landmarks[0] =
            CharacterLandmarkId("body.unprepared_landmark".to_owned());
        assert_eq!(
            bad_landmark.validate(),
            Err(PreparedHeroError::UnknownReference {
                field: "hero hand foot landmark",
                id: "body.unprepared_landmark".to_owned()
            })
        );

        let mut missing_hand_foot = prepared_hero_template_v1();
        missing_hand_foot
            .topology_descriptor
            .hand_foot_landmarks
            .clear();
        assert_eq!(
            missing_hand_foot.validate(),
            Err(PreparedHeroError::PreparedCharacter(
                PreparedCharacterError::EmptyCollection {
                    collection: "hero hand foot landmarks"
                }
            ))
        );
    }

    #[test]
    fn prepared_hero_validates_weight_set_cage_bindings() {
        let mut unknown_cage = prepared_hero_template_v1();
        unknown_cage.weight_sets[0].cage_id = PreparedCageId("prepared.cage.missing".to_owned());
        assert_eq!(
            unknown_cage.validate(),
            Err(PreparedHeroError::UnknownReference {
                field: "hero weight set cage",
                id: "prepared.cage.missing".to_owned()
            })
        );

        let mut mismatched_cage = prepared_hero_template_v1();
        mismatched_cage.weight_sets[0].cage_id = PreparedCageId("prepared.cage.head".to_owned());
        assert_eq!(
            mismatched_cage.validate(),
            Err(PreparedHeroError::UnknownReference {
                field: "hero weight set prepared cage",
                id: "prepared.cage.head".to_owned()
            })
        );
    }

    #[test]
    fn prepared_hero_rejects_hidden_raw_mesh_payloads_and_external_mesh_claims() {
        let template = prepared_hero_template_v1();
        assert_eq!(
            template.prepared_character.template_id,
            "prepared.humanoid.known_base.v1"
        );
        assert_eq!(
            customize_prepared_character_template(
                &template.prepared_character,
                &PreparedCharacterCustomizationRequest {
                    template_id: template.prepared_character.template_id.clone(),
                    values: Vec::new()
                }
            )
            .program
            .mesh_payload_policy,
            PreparedMeshPayloadPolicy::NoRawMeshPayload
        );

        let mut value = serde_json::to_value(&template).expect("serialize prepared hero template");
        value.as_object_mut().expect("template object").insert(
            "mesh_payload".to_owned(),
            serde_json::json!({"vertices": [0, 1, 2]}),
        );
        assert!(
            serde_json::from_value::<PreparedHeroTemplate>(value).is_err(),
            "prepared hero schema must reject hidden raw mesh payload fields"
        );

        let mut external_claim = prepared_hero_template_v1();
        external_claim.external_mesh_claim = Some("imported external hero mesh".to_owned());
        assert_eq!(
            external_claim.validate(),
            Err(PreparedHeroError::ExternalMeshClaim)
        );
    }

    #[test]
    fn prepared_hero_controls_are_limited_and_hide_internal_metadata() {
        let template = prepared_hero_template_v1();
        for control in &template.control_profile.controls {
            let lower = control.label.to_ascii_lowercase();
            for forbidden in [
                "cage", "landmark", "weight", "semantic", "region", "provider", "vertex",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "control label {} exposed {forbidden}",
                    control.label
                );
            }
        }

        let mut too_many = prepared_hero_template_v1();
        too_many
            .control_profile
            .controls
            .push(too_many.control_profile.controls[0].clone());
        too_many
            .control_profile
            .controls
            .push(too_many.control_profile.controls[1].clone());
        assert_eq!(
            too_many.validate(),
            Err(PreparedHeroError::TooManyPrimaryControls { count: 8, max: 7 })
        );

        let mut internal_label = prepared_hero_template_v1();
        internal_label.control_profile.controls[0].label = "Cage Weight".to_owned();
        assert_eq!(
            internal_label.validate(),
            Err(PreparedHeroError::InternalMetadataExposed {
                field: "hero control label",
                value: "Cage Weight".to_owned()
            })
        );
    }

    #[test]
    fn prepared_hero_declares_required_gear_provider_slots() {
        let template = prepared_hero_template_v1();
        let slots = template
            .provider_slots
            .iter()
            .map(|slot| slot.slot_id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            slots,
            BTreeSet::from([
                "headgear",
                "shoulders",
                "torso_armor",
                "belt_skirt",
                "gauntlets",
                "boots",
                "weapon",
                "back_accessory",
                "hair_head_mass"
            ])
        );
        assert!(
            template
                .provider_slots
                .iter()
                .all(|slot| !slot.populated_in_v1)
        );

        let mut missing_weapon = prepared_hero_template_v1();
        missing_weapon
            .provider_slots
            .retain(|slot| slot.slot_id != "weapon");
        assert_eq!(
            missing_weapon.validate(),
            Err(PreparedHeroError::MissingRequiredProviderSlot {
                slot: "weapon".to_owned()
            })
        );
    }

    #[test]
    fn prepared_hero_product_labels_hide_internal_metadata_terms() {
        let template = prepared_hero_template_v1();
        for label in template
            .landmarks
            .iter()
            .map(|landmark| landmark.label.as_str())
            .chain(
                template
                    .semantic_regions
                    .iter()
                    .map(|region| region.label.as_str()),
            )
            .chain(template.cages.iter().map(|cage| cage.label.as_str()))
            .chain(
                template
                    .provider_slots
                    .iter()
                    .map(|slot| slot.label.as_str()),
            )
            .chain(
                template
                    .control_profile
                    .controls
                    .iter()
                    .map(|control| control.label.as_str()),
            )
        {
            assert!(
                !exposes_internal_id(label),
                "product-facing hero label exposed internal term: {label}"
            );
        }

        let mut internal_cage_label = prepared_hero_template_v1();
        internal_cage_label.cages[0].label = "Body Cage".to_owned();
        assert_eq!(
            internal_cage_label.validate(),
            Err(PreparedHeroError::InternalMetadataExposed {
                field: "hero cage label",
                value: "Body Cage".to_owned()
            })
        );
    }

    #[test]
    fn prepared_hero_cannot_be_showcase_without_human_review_and_contact_sheet() {
        let quality = HeroQualityGateProfile {
            target_tier: HeroQualityTier::Showcase,
            validation_status: HeroValidationStatus::PrototypeValidated,
            required_evidence: vec!["contract evidence".to_owned()],
        };
        let review = HeroTemplateReviewManifest {
            achieved_tier: HeroQualityTier::Showcase,
            human_review_status: HeroHumanReviewStatus::Pending,
            contact_sheet_evidence: false,
            notes: vec!["not reviewed".to_owned()],
        };

        assert_eq!(
            review.validate(&quality),
            Err(PreparedHeroError::ShowcaseMissingReviewEvidence)
        );

        let mut invalid_claim = prepared_hero_template_v1();
        invalid_claim.review_manifest.achieved_tier = HeroQualityTier::Showcase;
        assert_eq!(
            invalid_claim.validate(),
            Err(PreparedHeroError::UnsupportedQualityClaim)
        );
    }

    #[test]
    fn customization_is_deterministic_and_carries_no_raw_mesh_payload() {
        let template = prepared_humanoid_template();
        let request = PreparedCharacterCustomizationRequest {
            template_id: template.template_id.clone(),
            values: vec![
                scalar_value("prepared.control.silhouette", 0.75),
                choice_value("prepared.control.pose_preset", "contrapposto"),
                scalar_value("prepared.control.garment_fit", -0.2),
            ],
        };

        let first = customize_prepared_character_template(&template, &request);
        let second = customize_prepared_character_template(&template, &request);

        assert_eq!(first, second);
        assert!(first.accepted, "{:?}", first.rejected_controls);
        assert_delta_weight_sets_match_cages(&template, &first.program);
        assert_eq!(
            first.program.mesh_payload_policy,
            PreparedMeshPayloadPolicy::NoRawMeshPayload
        );
        assert!(!first.program.cage_deltas.is_empty());
        assert!(
            first
                .program
                .preserved_landmarks
                .iter()
                .any(|landmark| landmark.0 == "body.neck_base")
        );
        let json = serde_json::to_value(&first).expect("serialize customization report");
        assert_no_raw_mesh_payload_keys(&json);
    }

    #[test]
    fn customization_rejects_unknown_duplicate_and_out_of_range_controls() {
        let template = prepared_humanoid_template();
        let request = PreparedCharacterCustomizationRequest {
            template_id: template.template_id.clone(),
            values: vec![
                scalar_value("prepared.control.body_proportions", 2.0),
                scalar_value("prepared.control.body_proportions", 0.2),
                scalar_value("prepared.control.unknown", 0.0),
            ],
        };

        let report = customize_prepared_character_template(&template, &request);

        assert!(!report.accepted);
        assert!(report.rejected_controls.iter().any(|rejection| {
            rejection.control == "prepared.control.body_proportions"
                && rejection.reason.contains("outside")
        }));
        assert!(
            report
                .rejected_controls
                .iter()
                .any(|rejection| rejection.reason.contains("duplicate"))
        );
        assert!(report.rejected_controls.iter().any(|rejection| {
            rejection.control == "prepared.control.unknown" && rejection.reason.contains("unknown")
        }));
        assert!(report.applied_controls.is_empty());
        assert!(report.program.cage_deltas.is_empty());
    }

    #[test]
    fn validation_rejects_stale_base_fingerprint_and_missing_preparation() {
        let mut stale = prepared_humanoid_template();
        stale.bases[0].fingerprint = BaseFingerprint("stale".to_owned());
        assert!(matches!(
            stale.validate(),
            Err(PreparedCharacterError::StaleBaseFingerprint { .. })
        ));

        let mut unprepared = prepared_humanoid_template();
        unprepared.cages.clear();
        assert_eq!(
            unprepared.validate(),
            Err(PreparedCharacterError::EmptyCollection {
                collection: "cages"
            })
        );
    }

    #[test]
    fn customization_rejects_invalid_template_without_cage_deltas() {
        let mut template = prepared_humanoid_template();
        template.cages.clear();
        let request = PreparedCharacterCustomizationRequest {
            template_id: template.template_id.clone(),
            values: vec![scalar_value("prepared.control.body_proportions", 0.3)],
        };

        let report = customize_prepared_character_template(&template, &request);

        assert!(!report.accepted);
        assert!(report.applied_controls.is_empty());
        assert!(report.program.cage_deltas.is_empty());
        assert!(report.program.preserved_landmarks.is_empty());
        assert!(
            report
                .rejected_controls
                .iter()
                .any(|rejection| rejection.control == "template")
        );
    }

    #[test]
    fn validation_rejects_cross_base_cage_and_weight_references() {
        let mut cross_region = prepared_humanoid_template();
        cross_region.cages[0].target_regions[0] = CharacterRegionId("head.face".to_owned());
        assert!(matches!(
            cross_region.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "region",
                ..
            })
        ));

        let mut cross_landmark = prepared_humanoid_template();
        cross_landmark.cages[0].landmark_bindings[0] = CharacterLandmarkId("head.crown".to_owned());
        assert!(matches!(
            cross_landmark.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "cage landmark base",
                ..
            })
        ));

        let mut cross_weight = prepared_humanoid_template();
        cross_weight.weight_sets[0].target_region = CharacterRegionId("head.face".to_owned());
        assert!(matches!(
            cross_weight.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "weight set region",
                ..
            })
        ));

        let mut same_base_wrong_region = prepared_humanoid_template();
        same_base_wrong_region.weight_sets[0].target_region =
            CharacterRegionId("body.neck".to_owned());
        assert!(matches!(
            same_base_wrong_region.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "weight set cage region",
                ..
            })
        ));
    }

    #[test]
    fn validation_rejects_stale_library_and_unavailable_template_actions() {
        let mut stale_library = prepared_humanoid_template();
        stale_library.base_library_fingerprint = BaseFingerprint("stale".to_owned());
        assert!(matches!(
            stale_library.validate(),
            Err(PreparedCharacterError::StaleBaseLibraryFingerprint { .. })
        ));

        let mut bad_control = prepared_humanoid_template();
        bad_control.novice_controls[0].affected_cages[0] = PreparedCageId("missing".to_owned());
        assert!(matches!(
            bad_control.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "control cage",
                ..
            })
        ));

        let mut bad_label = prepared_humanoid_template();
        bad_label.novice_controls[0].label = "Edit Any Mesh".to_owned();
        assert!(matches!(
            bad_label.validate(),
            Err(PreparedCharacterError::InvalidControlLabel { .. })
        ));
    }

    #[test]
    fn validation_rejects_control_weight_sets_from_unaffected_cages() {
        let mut bad_control = prepared_humanoid_template();
        bad_control.novice_controls[0].affected_weight_sets[0] =
            PreparedWeightSetId("prepared.weights.head".to_owned());
        assert!(matches!(
            bad_control.validate(),
            Err(PreparedCharacterError::UnknownReference {
                field: "control weight set cage",
                ..
            })
        ));
    }

    fn scalar_value(control: &str, value: f32) -> PreparedCharacterControlValue {
        PreparedCharacterControlValue {
            control: CharacterControlId(control.to_owned()),
            value: PreparedControlValue::Scalar(value),
        }
    }

    fn choice_value(control: &str, value: &str) -> PreparedCharacterControlValue {
        PreparedCharacterControlValue {
            control: CharacterControlId(control.to_owned()),
            value: PreparedControlValue::Choice(value.to_owned()),
        }
    }

    fn assert_delta_weight_sets_match_cages(
        template: &PreparedCharacterTemplate,
        program: &PreparedCharacterCustomizationProgram,
    ) {
        let weight_cage_ids = template
            .weight_sets
            .iter()
            .map(|weights| (weights.id.0.as_str(), weights.cage_id.0.as_str()))
            .collect::<BTreeMap<_, _>>();
        for delta in &program.cage_deltas {
            for weights in &delta.weight_sets {
                assert_eq!(
                    weight_cage_ids.get(weights.0.as_str()).copied(),
                    Some(delta.cage.0.as_str()),
                    "weight set {} was attached to cage {}",
                    weights.0,
                    delta.cage.0
                );
            }
        }
    }

    fn assert_no_raw_mesh_payload_keys(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                for forbidden in ["raw_mesh", "vertices", "vertex_positions"] {
                    assert!(
                        !object.contains_key(forbidden),
                        "serialized report contains forbidden geometry payload key {forbidden}"
                    );
                }
                for nested in object.values() {
                    assert_no_raw_mesh_payload_keys(nested);
                }
            }
            serde_json::Value::Array(values) => {
                for nested in values {
                    assert_no_raw_mesh_payload_keys(nested);
                }
            }
            _ => {}
        }
    }
}
