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
                ],
                &[
                    "body.pelvis_center",
                    "body.neck_base",
                    "body.left_shoulder",
                    "body.right_shoulder",
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
            CharacterRegionId("body.left_leg".to_owned());
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
