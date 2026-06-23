//! Synthetic known-base character benchmark corpus.
//!
//! Public corpus cases expose deterministic mesh artifacts only. The authored
//! semantic programs and answer-key metadata stay in test-only fixtures so the
//! inverse path cannot recover by reading the target program.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Current synthetic character corpus schema.
pub const GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION: u32 = 2;

/// Public deterministic corpus of mesh-only inverse inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedCharacterCorpus {
    /// Corpus schema version.
    pub schema_version: u32,
    /// Seed used for deterministic case generation.
    pub seed: u64,
    /// Exposed benchmark cases. These do not include target programs.
    pub cases: Vec<ExposedCharacterBenchmarkCase>,
}

impl GeneratedCharacterCorpus {
    /// Generate a deterministic mesh-only benchmark corpus for a seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            schema_version: GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION,
            seed,
            cases: exposed_character_cases(seed),
        }
    }

    /// Find a case by stable ID.
    #[must_use]
    pub fn case_by_id(&self, id: &str) -> Option<&ExposedCharacterBenchmarkCase> {
        self.cases.iter().find(|case| case.id == id)
    }

    /// Validate public artifacts and case IDs without consulting answer keys.
    pub fn validate_public_inputs(&self) -> Result<(), CharacterCorpusError> {
        if self.schema_version != GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION {
            return Err(CharacterCorpusError::InvalidSchemaVersion {
                found: self.schema_version,
            });
        }
        if self.cases.is_empty() {
            return Err(CharacterCorpusError::EmptyCorpus);
        }
        let expected_cases = exposed_character_cases(self.seed);
        let expected_ids = expected_cases
            .iter()
            .map(|case| case.id.clone())
            .collect::<BTreeSet<_>>();
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            require_nonempty("case id", &case.id)?;
            if !ids.insert(case.id.clone()) {
                return Err(CharacterCorpusError::DuplicateCaseId {
                    id: case.id.clone(),
                });
            }
            case.mesh.validate()?;
            let Some(expected) = expected_cases
                .iter()
                .find(|expected| expected.id == case.id)
            else {
                return Err(CharacterCorpusError::InvalidCaseSet {
                    reason: format!("unexpected case id {}", case.id),
                });
            };
            if case.mesh != expected.mesh {
                return Err(CharacterCorpusError::InvalidMeshArtifact {
                    id: case.mesh.id.clone(),
                    reason: "mesh descriptor does not match deterministic seed and known-base fingerprints"
                        .to_owned(),
                });
            }
        }
        if ids != expected_ids {
            return Err(CharacterCorpusError::InvalidCaseSet {
                reason: "case set must match deterministic corpus case IDs".to_owned(),
            });
        }
        Ok(())
    }
}

/// Generate a deterministic mesh-only benchmark corpus for a seed.
#[must_use]
pub fn generated_character_corpus(seed: u64) -> GeneratedCharacterCorpus {
    GeneratedCharacterCorpus::from_seed(seed)
}

/// One public benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposedCharacterBenchmarkCase {
    /// Stable opaque benchmark case ID.
    pub id: String,
    /// Mesh artifact exposed to inverse systems.
    pub mesh: CharacterMeshArtifact,
}

/// Public mesh artifact descriptor exposed to inverse systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterMeshArtifact {
    /// Stable artifact ID.
    pub id: String,
    /// Canonical units.
    pub canonical_units: String,
    /// Canonical coordinate convention.
    pub coordinate_system: String,
    /// ID-independent known-base semantic descriptor fingerprint.
    pub semantic_descriptor_fingerprint: String,
    /// Raw geometry size for strict compression accounting.
    pub raw_geometry_size: CharacterRawGeometrySize,
    /// Connected mesh/object component count.
    pub connected_component_count: usize,
    /// Axis-aligned bounds in canonical space.
    pub bounds_min: [f32; 3],
    /// Axis-aligned bounds in canonical space.
    pub bounds_max: [f32; 3],
    /// Stable topology fingerprint for the exposed mesh artifact.
    pub topology_fingerprint: String,
    /// Stable canonical-position fingerprint for the exposed mesh artifact.
    pub canonical_position_fingerprint: String,
    /// Fingerprint of the complete public artifact descriptor.
    pub artifact_fingerprint: String,
}

/// Raw geometry size for public mesh artifact accounting.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterRawGeometrySize {
    /// Number of canonical vertices.
    pub vertex_count: usize,
    /// Number of canonical triangle faces.
    pub face_count: usize,
    /// Number of bytes required by f32 xyz vertex positions.
    pub position_bytes: usize,
    /// Number of bytes required by u32 triangle indices.
    pub topology_bytes: usize,
}

/// Descriptor-level known-base character feature flags inferred from public
/// mesh observations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownBaseCharacterMeshFeatures {
    /// Symmetric controls are present.
    pub symmetric: bool,
    /// Asymmetric correction evidence is present.
    pub asymmetric: bool,
    /// Pose deformation evidence is present.
    pub posed: bool,
    /// Garment shell/opening evidence is present.
    pub clothed: bool,
    /// Hair mass/card evidence is present.
    pub hair: bool,
    /// Topology-changing edit evidence is present.
    pub topology_edited: bool,
}

/// ID-independent public mesh signature for known-base character descriptors.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownBaseCharacterMeshSignature {
    /// Number of canonical vertices.
    pub vertex_count: usize,
    /// Number of canonical triangle faces.
    pub face_count: usize,
    /// Connected mesh/object component count.
    pub connected_component_count: usize,
    /// Axis-aligned minimum bounds as exact f32 bit patterns.
    pub bounds_min_bits: [u32; 3],
    /// Axis-aligned maximum bounds as exact f32 bit patterns.
    pub bounds_max_bits: [u32; 3],
}

impl KnownBaseCharacterMeshSignature {
    /// Return true when the public mesh descriptor has this signature.
    #[must_use]
    pub fn matches_mesh(self, mesh: &CharacterMeshArtifact) -> bool {
        self.vertex_count == mesh.raw_geometry_size.vertex_count
            && self.face_count == mesh.raw_geometry_size.face_count
            && self.connected_component_count == mesh.connected_component_count
            && self.bounds_min_bits == bounds_bits(mesh.bounds_min)
            && self.bounds_max_bits == bounds_bits(mesh.bounds_max)
    }
}

/// Expected ID-independent exact-output fingerprints for a known-base
/// character descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownBaseCharacterMeshDescriptor {
    /// Public semantic descriptor fingerprint.
    pub semantic_descriptor_fingerprint: String,
    /// Expected topology fingerprint.
    pub topology_fingerprint: String,
    /// Expected canonical-position fingerprint.
    pub canonical_position_fingerprint: String,
}

/// Candidate feature sets recognized by the known-base character public
/// descriptor contract.
#[must_use]
pub fn known_base_character_feature_candidates() -> Vec<KnownBaseCharacterMeshFeatures> {
    let mut candidates = Vec::new();
    for asymmetric in [false, true] {
        for posed in [false, true] {
            for clothed in [false, true] {
                for hair in [false, true] {
                    for topology_edited in [false, true] {
                        if topology_edited && !(asymmetric || clothed || hair) {
                            continue;
                        }
                        candidates.push(KnownBaseCharacterMeshFeatures {
                            symmetric: !asymmetric,
                            asymmetric,
                            posed,
                            clothed,
                            hair,
                            topology_edited,
                        });
                    }
                }
            }
        }
    }
    candidates
}

/// Expected ID-independent public signature for descriptor-level known-base
/// character features.
#[must_use]
pub fn known_base_character_signature_for_features(
    features: KnownBaseCharacterMeshFeatures,
) -> KnownBaseCharacterMeshSignature {
    let mut vertex_count = 2_400 + usize::from(features.asymmetric) * 64;
    let mut face_count = 4_800 + usize::from(features.asymmetric) * 128;
    if features.posed {
        vertex_count += 128;
        face_count += 192;
    }
    if features.clothed {
        vertex_count += 640;
        face_count += 1_160;
    }
    if features.hair {
        vertex_count += 520;
        face_count += 780;
    }
    if features.topology_edited {
        vertex_count += 240;
        face_count += 420;
    }

    KnownBaseCharacterMeshSignature {
        vertex_count,
        face_count,
        connected_component_count: 1 + usize::from(features.clothed) + usize::from(features.hair),
        bounds_min_bits: bounds_bits([
            -0.55 - if features.hair { 0.08 } else { 0.0 },
            0.0,
            -0.28 - if features.clothed { 0.04 } else { 0.0 },
        ]),
        bounds_max_bits: bounds_bits([
            0.55 + if features.asymmetric { 0.05 } else { 0.0 },
            1.92 + if features.hair { 0.10 } else { 0.0 },
            0.34 + if features.posed { 0.08 } else { 0.0 },
        ]),
    }
}

/// Expected ID-independent exact-output descriptor fingerprints for
/// known-base character features.
#[must_use]
pub fn known_base_character_descriptor_for_features(
    features: KnownBaseCharacterMeshFeatures,
) -> KnownBaseCharacterMeshDescriptor {
    let signature = known_base_character_signature_for_features(features);
    let base_key = base_topology_fingerprint_key();
    let topology_fingerprint = fingerprint(&[
        "character-topology".to_owned(),
        base_key.clone(),
        signature.vertex_count.to_string(),
        signature.face_count.to_string(),
        signature.connected_component_count.to_string(),
        bounds_key_from_bits(signature.bounds_min_bits),
        bounds_key_from_bits(signature.bounds_max_bits),
        coverage_key(features),
    ]);
    let canonical_position_fingerprint = fingerprint(&[
        "character-position".to_owned(),
        base_key.clone(),
        signature.vertex_count.to_string(),
        signature.face_count.to_string(),
        bounds_key_from_bits(signature.bounds_min_bits),
        bounds_key_from_bits(signature.bounds_max_bits),
        coverage_key(features),
    ]);
    let semantic_descriptor_fingerprint = fingerprint(&[
        "character-known-base-semantic-descriptor".to_owned(),
        base_key,
        signature.vertex_count.to_string(),
        signature.face_count.to_string(),
        signature.connected_component_count.to_string(),
        bounds_key_from_bits(signature.bounds_min_bits),
        bounds_key_from_bits(signature.bounds_max_bits),
        coverage_key(features),
        topology_fingerprint.clone(),
        canonical_position_fingerprint.clone(),
    ]);

    KnownBaseCharacterMeshDescriptor {
        semantic_descriptor_fingerprint,
        topology_fingerprint,
        canonical_position_fingerprint,
    }
}

/// Compute the complete public artifact fingerprint for a mesh artifact.
#[must_use]
pub fn character_mesh_artifact_fingerprint(artifact: &CharacterMeshArtifact) -> String {
    artifact_fingerprint(artifact)
}

impl CharacterMeshArtifact {
    /// Validate finite mesh artifact metadata.
    pub fn validate(&self) -> Result<(), CharacterCorpusError> {
        require_nonempty("mesh id", &self.id)?;
        require_nonempty("canonical units", &self.canonical_units)?;
        require_nonempty("coordinate system", &self.coordinate_system)?;
        require_nonempty(
            "semantic descriptor fingerprint",
            &self.semantic_descriptor_fingerprint,
        )?;
        require_nonempty("topology fingerprint", &self.topology_fingerprint)?;
        require_nonempty(
            "canonical position fingerprint",
            &self.canonical_position_fingerprint,
        )?;
        require_nonempty("artifact fingerprint", &self.artifact_fingerprint)?;
        if self.raw_geometry_size.vertex_count == 0 || self.raw_geometry_size.face_count == 0 {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "mesh must contain vertices and faces".to_owned(),
            });
        }
        if self.connected_component_count == 0 {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "mesh must contain at least one connected component".to_owned(),
            });
        }
        let bounds_are_valid = self.bounds_min.iter().all(|value| value.is_finite())
            && self.bounds_max.iter().all(|value| value.is_finite())
            && self.bounds_min[0] < self.bounds_max[0]
            && self.bounds_min[1] < self.bounds_max[1]
            && self.bounds_min[2] < self.bounds_max[2];
        if !bounds_are_valid {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "mesh bounds must be finite and ordered".to_owned(),
            });
        }
        let expected_position_bytes =
            expected_geometry_bytes(self.raw_geometry_size.vertex_count, 3, 4);
        if expected_position_bytes != Some(self.raw_geometry_size.position_bytes) {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "position byte count must match f32 xyz vertex storage".to_owned(),
            });
        }
        let expected_topology_bytes =
            expected_geometry_bytes(self.raw_geometry_size.face_count, 3, 4);
        if expected_topology_bytes != Some(self.raw_geometry_size.topology_bytes) {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "topology byte count must match triangle index storage".to_owned(),
            });
        }
        let expected = artifact_fingerprint(self);
        if self.artifact_fingerprint != expected {
            return Err(CharacterCorpusError::InvalidMeshArtifact {
                id: self.id.clone(),
                reason: "artifact fingerprint does not match public descriptor".to_owned(),
            });
        }
        Ok(())
    }
}

/// Private case coverage flags used for benchmark fixture generation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CharacterBenchmarkCoverage {
    symmetric: bool,
    asymmetric: bool,
    posed: bool,
    clothed: bool,
    hair: bool,
    topology_edited: bool,
}

impl From<CharacterBenchmarkCoverage> for KnownBaseCharacterMeshFeatures {
    fn from(coverage: CharacterBenchmarkCoverage) -> Self {
        Self {
            symmetric: coverage.symmetric,
            asymmetric: coverage.asymmetric,
            posed: coverage.posed,
            clothed: coverage.clothed,
            hair: coverage.hair,
            topology_edited: coverage.topology_edited,
        }
    }
}

/// Public validation error for generated corpus artifacts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CharacterCorpusError {
    /// Unsupported corpus schema.
    #[error("unsupported character corpus schema version {found}")]
    InvalidSchemaVersion {
        /// Found schema version.
        found: u32,
    },
    /// Corpus has no cases.
    #[error("character corpus contains no cases")]
    EmptyCorpus,
    /// Duplicate case ID.
    #[error("duplicate character corpus case id: {id}")]
    DuplicateCaseId {
        /// Duplicated ID.
        id: String,
    },
    /// Case set does not match the deterministic corpus for the seed.
    #[error("invalid character corpus case set: {reason}")]
    InvalidCaseSet {
        /// Reason.
        reason: String,
    },
    /// Required field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },
    /// Invalid mesh artifact.
    #[error("invalid character mesh artifact {id}: {reason}")]
    InvalidMeshArtifact {
        /// Artifact ID.
        id: String,
        /// Reason.
        reason: String,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct CharacterCaseSpec {
    public_id: &'static str,
    private_id: &'static str,
    label: &'static str,
    coverage: CharacterBenchmarkCoverage,
}

fn exposed_character_cases(seed: u64) -> Vec<ExposedCharacterBenchmarkCase> {
    character_case_specs()
        .into_iter()
        .map(|spec| ExposedCharacterBenchmarkCase {
            id: spec.public_id.to_owned(),
            mesh: mesh_artifact_for_case(seed, spec.public_id, spec.coverage),
        })
        .collect()
}

fn character_case_specs() -> Vec<CharacterCaseSpec> {
    vec![
        CharacterCaseSpec {
            public_id: "character.case.0001",
            private_id: "character.neutral_known_base",
            label: "Neutral known-base humanoid",
            coverage: CharacterBenchmarkCoverage {
                symmetric: true,
                asymmetric: false,
                posed: false,
                clothed: false,
                hair: false,
                topology_edited: false,
            },
        },
        CharacterCaseSpec {
            public_id: "character.case.0002",
            private_id: "character.posed_asymmetric_face",
            label: "Posed humanoid with asymmetric facial correction",
            coverage: CharacterBenchmarkCoverage {
                symmetric: false,
                asymmetric: true,
                posed: true,
                clothed: false,
                hair: false,
                topology_edited: false,
            },
        },
        CharacterCaseSpec {
            public_id: "character.case.0003",
            private_id: "character.clothed_tunic",
            label: "Known-base humanoid with tunic shell and opening",
            coverage: CharacterBenchmarkCoverage {
                symmetric: true,
                asymmetric: false,
                posed: true,
                clothed: true,
                hair: false,
                topology_edited: true,
            },
        },
        CharacterCaseSpec {
            public_id: "character.case.0004",
            private_id: "character.hair_card_mass",
            label: "Known-base humanoid with bounded hair mass and cards",
            coverage: CharacterBenchmarkCoverage {
                symmetric: true,
                asymmetric: false,
                posed: false,
                clothed: false,
                hair: true,
                topology_edited: true,
            },
        },
        CharacterCaseSpec {
            public_id: "character.case.0005",
            private_id: "character.full_mixed_edit",
            label: "Known-base clothed and styled character with asymmetric edits",
            coverage: CharacterBenchmarkCoverage {
                symmetric: false,
                asymmetric: true,
                posed: true,
                clothed: true,
                hair: true,
                topology_edited: true,
            },
        },
    ]
}

fn mesh_artifact_for_case(
    _seed: u64,
    case_id: &str,
    coverage: CharacterBenchmarkCoverage,
) -> CharacterMeshArtifact {
    let features = KnownBaseCharacterMeshFeatures::from(coverage);
    let signature = known_base_character_signature_for_features(features);
    let descriptor = known_base_character_descriptor_for_features(features);
    let mut artifact = CharacterMeshArtifact {
        id: format!("mesh.{case_id}"),
        canonical_units: "meters".to_owned(),
        coordinate_system: "y_up_z_forward_right_handed".to_owned(),
        semantic_descriptor_fingerprint: descriptor.semantic_descriptor_fingerprint,
        raw_geometry_size: CharacterRawGeometrySize {
            vertex_count: signature.vertex_count,
            face_count: signature.face_count,
            position_bytes: signature.vertex_count * 3 * 4,
            topology_bytes: signature.face_count * 3 * 4,
        },
        connected_component_count: signature.connected_component_count,
        bounds_min: [
            f32::from_bits(signature.bounds_min_bits[0]),
            f32::from_bits(signature.bounds_min_bits[1]),
            f32::from_bits(signature.bounds_min_bits[2]),
        ],
        bounds_max: [
            f32::from_bits(signature.bounds_max_bits[0]),
            f32::from_bits(signature.bounds_max_bits[1]),
            f32::from_bits(signature.bounds_max_bits[2]),
        ],
        topology_fingerprint: descriptor.topology_fingerprint,
        canonical_position_fingerprint: descriptor.canonical_position_fingerprint,
        artifact_fingerprint: String::new(),
    };
    artifact.artifact_fingerprint = artifact_fingerprint(&artifact);
    artifact
}

fn artifact_fingerprint(artifact: &CharacterMeshArtifact) -> String {
    fingerprint(&[
        "character-mesh-artifact".to_owned(),
        artifact.id.clone(),
        artifact.canonical_units.clone(),
        artifact.coordinate_system.clone(),
        artifact.semantic_descriptor_fingerprint.clone(),
        artifact.raw_geometry_size.vertex_count.to_string(),
        artifact.raw_geometry_size.face_count.to_string(),
        artifact.raw_geometry_size.position_bytes.to_string(),
        artifact.raw_geometry_size.topology_bytes.to_string(),
        artifact.connected_component_count.to_string(),
        bounds_key(artifact.bounds_min),
        bounds_key(artifact.bounds_max),
        artifact.topology_fingerprint.clone(),
        artifact.canonical_position_fingerprint.clone(),
    ])
}

fn bounds_key(bounds: [f32; 3]) -> String {
    format!(
        "{:08x},{:08x},{:08x}",
        bounds[0].to_bits(),
        bounds[1].to_bits(),
        bounds[2].to_bits()
    )
}

fn bounds_bits(bounds: [f32; 3]) -> [u32; 3] {
    [
        bounds[0].to_bits(),
        bounds[1].to_bits(),
        bounds[2].to_bits(),
    ]
}

fn bounds_key_from_bits(bounds: [u32; 3]) -> String {
    format!("{:08x},{:08x},{:08x}", bounds[0], bounds[1], bounds[2])
}

fn coverage_key(coverage: KnownBaseCharacterMeshFeatures) -> String {
    format!(
        "sym={} asym={} pose={} cloth={} hair={} topo={}",
        coverage.symmetric,
        coverage.asymmetric,
        coverage.posed,
        coverage.clothed,
        coverage.hair,
        coverage.topology_edited
    )
}

#[cfg(test)]
fn mesh_provenance_key(seed: u64, case_id: &str, coverage: CharacterBenchmarkCoverage) -> String {
    fingerprint(&[
        "mesh-provenance".to_owned(),
        base_topology_fingerprint_key(),
        seed.to_string(),
        case_id.to_owned(),
        coverage_key(coverage.into()),
    ])
}

fn base_topology_fingerprint_key() -> String {
    let library = crate::base::base_topology_library();
    let mut base_fingerprints = crate::base::fingerprinted_character_bases()
        .into_iter()
        .map(|fingerprinted| {
            format!(
                "{}@{}={}",
                fingerprinted.base.id.0,
                fingerprinted.base.base_version,
                fingerprinted.fingerprint.0
            )
        })
        .collect::<Vec<_>>();
    base_fingerprints.sort();
    format!(
        "library@{}={};bases={}",
        crate::base::BASE_TOPOLOGY_LIBRARY_VERSION,
        library.fingerprint().0,
        base_fingerprints.join("|")
    )
}

fn require_nonempty(field: &'static str, value: &str) -> Result<(), CharacterCorpusError> {
    if value.is_empty() {
        Err(CharacterCorpusError::EmptyField { field })
    } else {
        Ok(())
    }
}

fn expected_geometry_bytes(count: usize, lanes: usize, bytes_per_lane: usize) -> Option<usize> {
    count
        .checked_mul(lanes)
        .and_then(|value| value.checked_mul(bytes_per_lane))
}

fn fingerprint(parts: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"shape-character.synthetic-corpus.v1\0");
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod private_fixtures {
    use shape_program::{
        BaseTopologyReference, GrammarProfile, ModelingOperation, ModelingOperationKind,
        ModelingProgram, OperationPayloadDescriptor, OperationPayloadKind, ProgramDependencyGraph,
        ProgramOperationId, SemanticParameter, SemanticPartId, SemanticRegionId, SemanticSelection,
        SemanticSelectionId, SemanticSelectionPayload,
        deformation::deformation_operator_contract,
        evaluator::{EvaluatorConfig, semantic_output_fingerprint},
        topology::topology_contract_for,
    };

    use crate::{
        CHARACTER_GRAMMAR_SCHEMA_VERSION, CharacterControlId, CharacterLandmarkId,
        CharacterRegionId, ScalarRange, UnitQuaternion,
        base::{
            BASE_TOPOLOGY_LIBRARY_VERSION, base_topology_library, fingerprinted_character_bases,
        },
        face::{FACE_DEFORMATION_OPERATORS, FaceDeformationOperator},
        garment::{
            BodyRegionSide, BodyRegionTarget, BodySurfaceAnchor, BoundedFoldField, FoldDirection,
            GARMENT_GRAMMAR_SCHEMA_VERSION, GarmentBase, GarmentCurve, GarmentGrammarDocument,
            GarmentOperation, fold_amplitude_range, fold_falloff_range, fold_radius_range,
            fold_wavelength_range, opening_clearance_range, seam_allowance_range,
            shell_offset_range, shell_thickness_range,
        },
        hair::{
            CountRange, HAIR_GRAMMAR_SCHEMA_VERSION, HairCard, HairCardSide, HairClump,
            HairCurveRole, HairElementId, HairElementKind, HairGrammar, HairGuideCurve, HairMass,
            HairMassKind, HairPartSide, HairPartingRegion, HairPlacementBounds,
            HairPlacementStrategy,
        },
        proportion::{
            JointPoseSample, NormalizedPose, joint_frame_contracts, normalize_pose_samples,
            proportion_grammar,
        },
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct AuthoredCharacterCase {
        pub(super) id: String,
        pub(super) public_id: String,
        pub(super) label: String,
        pub(super) coverage: CharacterBenchmarkCoverage,
        pub(super) mesh: CharacterMeshArtifact,
        pub(super) source_program: ModelingProgram,
        pub(super) answer_key: CharacterRecoveryAnswerKey,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct CharacterRecoveryAnswerKey {
        pub(super) schema_version: u32,
        pub(super) base_library: CharacterBaseLibraryReference,
        pub(super) base_topologies: Vec<CharacterBaseReference>,
        pub(super) proportion_grammar: CharacterGrammarReference,
        pub(super) face_grammar: CharacterGrammarReference,
        pub(super) garment_grammar: Option<CharacterGrammarReference>,
        pub(super) hair_grammar_reference: Option<CharacterGrammarReference>,
        pub(super) proportion_controls: Vec<CharacterScalarControlValue>,
        pub(super) normalized_pose: Option<NormalizedPose>,
        pub(super) face_operations: Vec<CharacterFaceOperationApplication>,
        pub(super) asymmetric_corrections: Vec<AsymmetricCorrectionAnnotation>,
        pub(super) garment_document: Option<GarmentGrammarDocument>,
        pub(super) hair_grammar: Option<HairGrammar>,
        pub(super) topology_edits: Vec<CharacterTopologyEditAnnotation>,
        pub(super) mesh_provenance_key: String,
        pub(super) source_output_fingerprint: String,
        pub(super) expected_mesh: CharacterMeshArtifact,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub(super) struct CharacterBaseLibraryReference {
        pub(super) catalog_id: String,
        pub(super) version: String,
        pub(super) fingerprint: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub(super) struct CharacterBaseReference {
        pub(super) base_id: String,
        pub(super) version: String,
        pub(super) fingerprint: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub(super) struct CharacterGrammarReference {
        pub(super) grammar_id: String,
        pub(super) schema_version: u32,
        pub(super) version: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct CharacterScalarControlValue {
        control_id: CharacterControlId,
        value: f32,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct CharacterFaceOperationApplication {
        operator_id: String,
        control_values: Vec<CharacterScalarControlValue>,
        target_regions: Vec<String>,
        preserves_loops: Vec<String>,
        asymmetric: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct AsymmetricCorrectionAnnotation {
        id: String,
        region: String,
        side: String,
        magnitude: f32,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub(super) struct CharacterTopologyEditAnnotation {
        pub(super) id: String,
        pub(super) edit_kind: CharacterTopologyEditKind,
        pub(super) target: String,
        pub(super) expected_part_delta: i32,
        pub(super) expected_boundary_loop_delta: i32,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub(super) enum CharacterTopologyEditKind {
        GarmentShell,
        GarmentOpening,
        HairCardStrip,
        AccessorySplit,
    }

    pub(super) fn authored_character_cases(seed: u64) -> Vec<AuthoredCharacterCase> {
        let mut rng = CharacterCorpusRng::new(seed);
        character_case_specs()
            .into_iter()
            .map(|spec| authored_case(seed, spec, &mut rng))
            .collect()
    }

    fn authored_case(
        seed: u64,
        spec: CharacterCaseSpec,
        rng: &mut CharacterCorpusRng,
    ) -> AuthoredCharacterCase {
        let id = spec.private_id;
        let public_id = spec.public_id;
        let coverage = spec.coverage;
        let proportion_controls = authored_proportion_controls(coverage, rng);
        let normalized_pose = coverage.posed.then(|| authored_pose(id, coverage, rng));
        let face_operations = authored_face_operations(coverage, rng);
        let asymmetric_corrections = authored_asymmetric_corrections(coverage, rng);
        let garment_document = coverage
            .clothed
            .then(|| authored_garment_document(coverage));
        let hair_grammar = coverage.hair.then(|| authored_hair_grammar(id, coverage));
        let topology_edits = authored_topology_edits(coverage);
        let base_library = base_library_reference();
        let base_topologies = base_references();
        let proportion_grammar_ref = proportion_grammar_reference();
        let face_grammar_ref = face_grammar_reference();
        let garment_grammar_ref = garment_document.as_ref().map(garment_grammar_reference);
        let hair_grammar_ref = hair_grammar.as_ref().map(hair_grammar_reference);
        let provenance_key = mesh_provenance_key(seed, public_id, coverage);
        let mut answer_key = CharacterRecoveryAnswerKey {
            schema_version: GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION,
            base_library,
            base_topologies,
            proportion_grammar: proportion_grammar_ref,
            face_grammar: face_grammar_ref,
            garment_grammar: garment_grammar_ref,
            hair_grammar_reference: hair_grammar_ref,
            proportion_controls,
            normalized_pose,
            face_operations,
            asymmetric_corrections,
            garment_document,
            hair_grammar,
            topology_edits,
            mesh_provenance_key: provenance_key,
            source_output_fingerprint: String::new(),
            expected_mesh: placeholder_mesh_artifact(id),
        };
        let source_program = source_program_for_answer_key(id, &answer_key);
        answer_key.source_output_fingerprint =
            source_program_fingerprint(&source_program).expect("authored source program hashes");
        let mesh = mesh_artifact_for_case(seed, public_id, coverage);
        answer_key.expected_mesh = mesh.clone();

        AuthoredCharacterCase {
            id: id.to_owned(),
            public_id: public_id.to_owned(),
            label: spec.label.to_owned(),
            coverage,
            mesh,
            source_program,
            answer_key,
        }
    }

    pub(super) fn source_program_fingerprint(
        program: &ModelingProgram,
    ) -> Result<String, shape_program::evaluator::EvaluatorContractError> {
        semantic_output_fingerprint(program, &EvaluatorConfig::canonical())
    }

    fn base_library_reference() -> CharacterBaseLibraryReference {
        let library = base_topology_library();
        CharacterBaseLibraryReference {
            catalog_id: "shape-character.humanoid.base-library".to_owned(),
            version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
            fingerprint: library.fingerprint().0,
        }
    }

    fn base_topology_reference() -> BaseTopologyReference {
        let library = base_library_reference();
        BaseTopologyReference {
            catalog_id: library.catalog_id,
            version: library.version,
            fingerprint: library.fingerprint,
        }
    }

    fn base_references() -> Vec<CharacterBaseReference> {
        fingerprinted_character_bases()
            .into_iter()
            .map(|fingerprinted| CharacterBaseReference {
                base_id: fingerprinted.base.id.0,
                version: fingerprinted.base.base_version.to_string(),
                fingerprint: fingerprinted.fingerprint.0,
            })
            .collect()
    }

    fn proportion_grammar_reference() -> CharacterGrammarReference {
        let grammar = proportion_grammar();
        CharacterGrammarReference {
            grammar_id: grammar.id.0,
            schema_version: grammar.schema_version,
            version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
        }
    }

    fn face_grammar_reference() -> CharacterGrammarReference {
        CharacterGrammarReference {
            grammar_id: "shape.character.face.v1".to_owned(),
            schema_version: CHARACTER_GRAMMAR_SCHEMA_VERSION,
            version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
        }
    }

    fn garment_grammar_reference(document: &GarmentGrammarDocument) -> CharacterGrammarReference {
        CharacterGrammarReference {
            grammar_id: document.grammar.0.clone(),
            schema_version: GARMENT_GRAMMAR_SCHEMA_VERSION,
            version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
        }
    }

    fn hair_grammar_reference(grammar: &HairGrammar) -> CharacterGrammarReference {
        CharacterGrammarReference {
            grammar_id: grammar.id.0.clone(),
            schema_version: HAIR_GRAMMAR_SCHEMA_VERSION,
            version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
        }
    }

    fn authored_proportion_controls(
        coverage: CharacterBenchmarkCoverage,
        rng: &mut CharacterCorpusRng,
    ) -> Vec<CharacterScalarControlValue> {
        let mut values = Vec::new();
        let grammar = proportion_grammar();
        for control in grammar.controls {
            let selected = matches!(
                control.id.0.as_str(),
                "proportion.height.stature"
                    | "proportion.height.head_count"
                    | "proportion.limb.arm_span"
                    | "proportion.limb.leg_length"
                    | "proportion.torso.ribcage_width"
                    | "proportion.torso.waist_width"
                    | "proportion.shoulder.width"
                    | "proportion.pelvis.width"
                    | "proportion.hand.scale"
                    | "proportion.foot.length"
            );
            if selected {
                let value = if coverage.asymmetric {
                    control.range.default + (control.range.max - control.range.default) * 0.35
                } else if coverage.symmetric {
                    control.range.default
                        + (rng.unit_f32() - 0.5) * (control.range.max - control.range.min) * 0.20
                } else {
                    control.range.default
                };
                values.push(CharacterScalarControlValue {
                    control_id: control.id,
                    value: clamp_range(value, control.range),
                });
            }
        }
        values.sort_by(|left, right| left.control_id.cmp(&right.control_id));
        values
    }

    fn authored_pose(
        case_id: &str,
        coverage: CharacterBenchmarkCoverage,
        rng: &mut CharacterCorpusRng,
    ) -> NormalizedPose {
        let samples = joint_frame_contracts()
            .into_iter()
            .map(|joint| {
                let mut translation = joint.rest_origin;
                if coverage.posed {
                    let sway = rng.scalar_f32(-0.015, 0.015);
                    if joint.id.0.contains("wrist.left") || joint.id.0.contains("hand.left") {
                        translation[2] += 0.08 + sway;
                        translation[1] += 0.04;
                    }
                    if joint.id.0.contains("wrist.right") || joint.id.0.contains("hand.right") {
                        translation[2] -= 0.06 + sway;
                        translation[1] -= 0.02;
                    }
                    if joint.id.0.contains("knee.left") || joint.id.0.contains("ankle.left") {
                        translation[2] += 0.04;
                    }
                    if case_id.contains("full_mixed") && joint.id.0 == "joint.head" {
                        translation[0] += 0.015;
                    }
                }
                JointPoseSample {
                    joint: joint.id,
                    rotation: UnitQuaternion::IDENTITY,
                    translation,
                    scale: if case_id.contains("full_mixed") && coverage.asymmetric {
                        1.01
                    } else {
                        1.0
                    },
                }
            })
            .collect::<Vec<_>>();
        normalize_pose_samples(&samples).expect("authored full pose should normalize")
    }

    fn authored_face_operations(
        coverage: CharacterBenchmarkCoverage,
        rng: &mut CharacterCorpusRng,
    ) -> Vec<CharacterFaceOperationApplication> {
        let wanted = if coverage.asymmetric {
            [
                "face.operator.brow.compact_arc_pair",
                "face.operator.nose.compact_wedge",
            ]
        } else {
            [
                "face.operator.skull.compact_cranium",
                "face.operator.cheek.compact_patch_pair",
            ]
        };
        let mut operations = FACE_DEFORMATION_OPERATORS
            .iter()
            .filter(|operator| wanted.contains(&operator.id))
            .map(|operator| face_application(operator, coverage.asymmetric, rng))
            .collect::<Vec<_>>();
        operations.sort_by(|left, right| left.operator_id.cmp(&right.operator_id));
        operations
    }

    fn face_application(
        operator: &FaceDeformationOperator<'_>,
        asymmetric: bool,
        rng: &mut CharacterCorpusRng,
    ) -> CharacterFaceOperationApplication {
        CharacterFaceOperationApplication {
            operator_id: operator.id.to_owned(),
            control_values: operator
                .controls
                .iter()
                .map(|control| CharacterScalarControlValue {
                    control_id: control.control_id(),
                    value: clamp_range(
                        control.range.default + rng.scalar_f32(-0.08, 0.08),
                        control.range,
                    ),
                })
                .collect(),
            target_regions: operator
                .targets
                .iter()
                .map(|target| target.base_region_id().to_owned())
                .collect(),
            preserves_loops: operator
                .preserves_loops
                .iter()
                .map(|preservation| preservation.target.base_loop_id().to_owned())
                .collect(),
            asymmetric,
        }
    }

    fn authored_asymmetric_corrections(
        coverage: CharacterBenchmarkCoverage,
        rng: &mut CharacterCorpusRng,
    ) -> Vec<AsymmetricCorrectionAnnotation> {
        if !coverage.asymmetric {
            return Vec::new();
        }
        vec![
            AsymmetricCorrectionAnnotation {
                id: "asym.face.brow.left_lift".to_owned(),
                region: "head.left_brow".to_owned(),
                side: "left".to_owned(),
                magnitude: rng.scalar_f32(0.05, 0.12),
            },
            AsymmetricCorrectionAnnotation {
                id: "asym.face.nose_ala.right_width".to_owned(),
                region: "head.right_nose_ala".to_owned(),
                side: "right".to_owned(),
                magnitude: rng.scalar_f32(0.02, 0.08),
            },
        ]
    }

    fn authored_garment_document(coverage: CharacterBenchmarkCoverage) -> GarmentGrammarDocument {
        let torso = BodyRegionTarget::whole("body.torso");
        let neck_loop =
            GarmentCurve::body_loop("body.loop.neck", BodyRegionTarget::whole("body.neck"));
        let hem = GarmentCurve::anchored_path(
            vec![
                BodySurfaceAnchor::new(
                    BodyRegionTarget::sided("body.torso", BodyRegionSide::Left),
                    0.1,
                    0.88,
                ),
                BodySurfaceAnchor::new(BodyRegionTarget::whole("body.torso"), 0.5, 0.95),
                BodySurfaceAnchor::new(
                    BodyRegionTarget::sided("body.torso", BodyRegionSide::Right),
                    0.9,
                    0.88,
                ),
            ],
            false,
        );
        let base = GarmentBase::shell_from_body_region(
            torso.clone(),
            shell_offset_range(),
            shell_thickness_range(),
        );
        let fold = BoundedFoldField::body_region(
            torso.clone(),
            if coverage.asymmetric {
                FoldDirection::AlongV
            } else {
                FoldDirection::AlongU
            },
            fold_amplitude_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        );
        let mut document = GarmentGrammarDocument::empty();
        document.bases.push(base);
        document.operations = vec![
            GarmentOperation::shell_from_body_region(
                torso.clone(),
                shell_offset_range(),
                shell_thickness_range(),
            ),
            GarmentOperation::opening(torso.clone(), neck_loop, opening_clearance_range()),
            GarmentOperation::seam(torso.clone(), hem.clone(), seam_allowance_range()),
            GarmentOperation::fold_curve(
                torso.clone(),
                hem,
                fold_amplitude_range(),
                fold_radius_range(),
            ),
            GarmentOperation::bounded_fold_field(torso, fold),
        ];
        document
    }

    fn authored_hair_grammar(case_id: &str, coverage: CharacterBenchmarkCoverage) -> HairGrammar {
        let mut grammar = HairGrammar::empty(case_id);
        let mass_id =
            HairElementId::deterministic(HairElementKind::Mass, format!("{case_id}.main"));
        let curve_id =
            HairElementId::deterministic(HairElementKind::Curve, format!("{case_id}.flow"));
        let card_id =
            HairElementId::deterministic(HairElementKind::Card, format!("{case_id}.card"));
        let clump_id =
            HairElementId::deterministic(HairElementKind::Clump, format!("{case_id}.clump"));
        let parting_id =
            HairElementId::deterministic(HairElementKind::PartingRegion, format!("{case_id}.part"));
        grammar.masses.push(HairMass {
            id: mass_id.clone(),
            kind: if coverage.asymmetric {
                HairMassKind::Side
            } else {
                HairMassKind::Crown
            },
            scalp_region: CharacterRegionId("head.scalp".to_owned()),
            anchors: vec![
                CharacterLandmarkId("head.landmark.crown".to_owned()),
                CharacterLandmarkId("head.landmark.nape".to_owned()),
            ],
            silhouette: normalized_scalar(0.72),
            length: nonnegative_scalar(if coverage.asymmetric { 1.25 } else { 0.85 }),
            density: normalized_scalar(0.66),
            lift: normalized_scalar(0.32),
        });
        grammar.curves.push(HairGuideCurve {
            id: curve_id.clone(),
            mass_id: mass_id.clone(),
            role: HairCurveRole::Flow,
            root: [0.5, 0.85, 0.5],
            control_points: vec![[0.45, 0.95, 0.05], [0.48, 1.05, -0.08], [0.52, 0.92, -0.16]],
            tension: normalized_scalar(0.45),
            curl: normalized_scalar(if coverage.asymmetric { 0.35 } else { 0.15 }),
        });
        grammar.cards.push(HairCard {
            id: card_id,
            mass_id: mass_id.clone(),
            curve_id: curve_id.clone(),
            side: HairCardSide::Center,
            width: nonnegative_scalar(0.55),
            length_scale: nonnegative_scalar(1.05),
            taper: normalized_scalar(0.72),
            twist_radians: signed_scalar(0.12),
            segments: CountRange::new(2, 8, 4),
        });
        grammar.clumps.push(HairClump {
            id: clump_id,
            mass_id: mass_id.clone(),
            curve_ids: vec![curve_id.clone()],
            child_count: CountRange::new(4, 64, 16),
            radius: nonnegative_scalar(0.08),
            density: normalized_scalar(0.48),
            taper: normalized_scalar(0.65),
            flyaway: normalized_scalar(0.18),
        });
        grammar.parting_regions.push(HairPartingRegion {
            id: parting_id.clone(),
            scalp_region: CharacterRegionId("head.scalp".to_owned()),
            side: if coverage.asymmetric {
                HairPartSide::Left
            } else {
                HairPartSide::Center
            },
            center_curve_id: Some(curve_id),
            direction: [1.0, 0.0],
            influence: normalized_scalar(0.7),
            feather_width: nonnegative_scalar(0.05),
        });
        let mut placement = HairPlacementBounds::scalp_uv(
            format!("{case_id}.placement"),
            mass_id,
            CharacterRegionId("head.scalp".to_owned()),
        );
        placement.strategy = HairPlacementStrategy::PartingAware;
        placement.parting_region_id = Some(parting_id);
        placement.instance_count = CountRange::new(8, 256, 96);
        grammar.placements.push(placement);
        grammar
    }

    fn authored_topology_edits(
        coverage: CharacterBenchmarkCoverage,
    ) -> Vec<CharacterTopologyEditAnnotation> {
        let mut edits = Vec::new();
        if coverage.clothed {
            edits.push(CharacterTopologyEditAnnotation {
                id: "topology.garment.shell".to_owned(),
                edit_kind: CharacterTopologyEditKind::GarmentShell,
                target: "body.torso".to_owned(),
                expected_part_delta: 1,
                expected_boundary_loop_delta: 1,
            });
            edits.push(CharacterTopologyEditAnnotation {
                id: "topology.garment.neck_opening".to_owned(),
                edit_kind: CharacterTopologyEditKind::GarmentOpening,
                target: "body.loop.neck".to_owned(),
                expected_part_delta: 0,
                expected_boundary_loop_delta: 1,
            });
        }
        if coverage.hair {
            edits.push(CharacterTopologyEditAnnotation {
                id: "topology.hair.card_strip".to_owned(),
                edit_kind: CharacterTopologyEditKind::HairCardStrip,
                target: "head.scalp".to_owned(),
                expected_part_delta: 1,
                expected_boundary_loop_delta: 0,
            });
        }
        if coverage.topology_edited && coverage.asymmetric {
            edits.push(CharacterTopologyEditAnnotation {
                id: "topology.accessory.side_split".to_owned(),
                edit_kind: CharacterTopologyEditKind::AccessorySplit,
                target: "body.shoulder.left".to_owned(),
                expected_part_delta: 1,
                expected_boundary_loop_delta: 2,
            });
        }
        edits
    }

    fn source_program_for_answer_key(
        case_id: &str,
        answer_key: &CharacterRecoveryAnswerKey,
    ) -> ModelingProgram {
        let selections = vec![
            selection_part("sel.character.body", "character.body"),
            selection_region("sel.character.face", "head.cranium"),
            selection_region("sel.character.torso", "body.torso"),
            selection_region("sel.character.scalp", "head.scalp"),
            selection_region("sel.character.shoulder_left", "body.shoulder.left"),
            selection_edge_class("sel.character.neck_edge", "body.loop.neck"),
        ];
        let mut operations = Vec::new();
        operations.push(operation(
            format!("op.{case_id}.base"),
            ModelingOperationKind::PrimitiveCreate,
            vec![],
            std::iter::once(SemanticParameter::Choice {
                name: "mesh_provenance_key".to_owned(),
                value: answer_key.mesh_provenance_key.clone(),
            })
            .chain(
                answer_key
                    .base_topologies
                    .iter()
                    .map(|base| SemanticParameter::Choice {
                        name: base.base_id.clone(),
                        value: base.fingerprint.clone(),
                    }),
            )
            .collect(),
            512,
        ));
        operations.push(operation(
            format!("op.{case_id}.proportions"),
            ModelingOperationKind::Lattice,
            vec![SemanticSelectionId("sel.character.torso".to_owned())],
            answer_key
                .proportion_controls
                .iter()
                .map(|control| SemanticParameter::Scalar {
                    name: control.control_id.0.clone(),
                    value: f64::from(control.value),
                })
                .collect(),
            384,
        ));
        if let Some(pose) = &answer_key.normalized_pose {
            operations.push(operation(
                format!("op.{case_id}.pose"),
                ModelingOperationKind::JointChainDeformation,
                vec![SemanticSelectionId("sel.character.body".to_owned())],
                vec![
                    SemanticParameter::Choice {
                        name: "pose_id".to_owned(),
                        value: pose.id.0.clone(),
                    },
                    SemanticParameter::Integer {
                        name: "joint_count".to_owned(),
                        value: i64::try_from(pose.joints.len()).expect("joint count fits i64"),
                    },
                ],
                320,
            ));
        }
        for face in &answer_key.face_operations {
            operations.push(operation(
                format!("op.{case_id}.face.{}", suffix(&face.operator_id)),
                if face.asymmetric {
                    ModelingOperationKind::BoundedCorrectiveBasis
                } else {
                    ModelingOperationKind::CageDeformation
                },
                vec![SemanticSelectionId("sel.character.face".to_owned())],
                face.control_values
                    .iter()
                    .map(|control| SemanticParameter::Scalar {
                        name: control.control_id.0.clone(),
                        value: f64::from(control.value),
                    })
                    .collect(),
                192,
            ));
        }
        if answer_key.garment_document.is_some() {
            operations.push(operation(
                format!("op.{case_id}.garment.shell"),
                ModelingOperationKind::ShellSolidify,
                vec![SemanticSelectionId("sel.character.torso".to_owned())],
                vec![SemanticParameter::Integer {
                    name: "operation_count".to_owned(),
                    value: answer_key
                        .garment_document
                        .as_ref()
                        .map(|document| i64::try_from(document.operations.len()).unwrap_or(0))
                        .unwrap_or(0),
                }],
                256,
            ));
        }
        if answer_key.hair_grammar.is_some() {
            operations.push(operation(
                format!("op.{case_id}.hair.cards"),
                ModelingOperationKind::Array,
                vec![SemanticSelectionId("sel.character.scalp".to_owned())],
                vec![
                    SemanticParameter::Integer {
                        name: "mass_count".to_owned(),
                        value: answer_key
                            .hair_grammar
                            .as_ref()
                            .map(|grammar| i64::try_from(grammar.masses.len()).unwrap_or(0))
                            .unwrap_or(0),
                    },
                    SemanticParameter::Integer {
                        name: "card_count".to_owned(),
                        value: answer_key
                            .hair_grammar
                            .as_ref()
                            .map(|grammar| i64::try_from(grammar.cards.len()).unwrap_or(0))
                            .unwrap_or(0),
                    },
                ],
                256,
            ));
        }
        for topology in &answer_key.topology_edits {
            operations.push(operation(
                format!("op.{case_id}.{}", topology.id),
                match topology.edit_kind {
                    CharacterTopologyEditKind::GarmentShell
                    | CharacterTopologyEditKind::HairCardStrip => ModelingOperationKind::Separate,
                    CharacterTopologyEditKind::GarmentOpening => ModelingOperationKind::Split,
                    CharacterTopologyEditKind::AccessorySplit => ModelingOperationKind::Split,
                },
                vec![topology_edit_selection_id(topology)],
                vec![
                    SemanticParameter::Integer {
                        name: "part_delta".to_owned(),
                        value: i64::from(topology.expected_part_delta),
                    },
                    SemanticParameter::Integer {
                        name: "boundary_loop_delta".to_owned(),
                        value: i64::from(topology.expected_boundary_loop_delta),
                    },
                ],
                128,
            ));
        }

        let mut program = ModelingProgram::strict_from_primitives();
        program.grammar_profile = GrammarProfile::StrictFromVersionedLibrary;
        program.base_topology = Some(base_topology_reference());
        program.selections = selections;
        program.operations = operations;
        program.dependency_graph = dependency_graph(&program.operations);
        program
    }

    pub(super) fn topology_edit_selection_id(
        topology: &CharacterTopologyEditAnnotation,
    ) -> SemanticSelectionId {
        let id = match topology.edit_kind {
            CharacterTopologyEditKind::GarmentShell => "sel.character.torso",
            CharacterTopologyEditKind::GarmentOpening => "sel.character.neck_edge",
            CharacterTopologyEditKind::HairCardStrip => "sel.character.scalp",
            CharacterTopologyEditKind::AccessorySplit => "sel.character.shoulder_left",
        };
        SemanticSelectionId(id.to_owned())
    }

    fn operation(
        id: impl Into<String>,
        kind: ModelingOperationKind,
        selections: Vec<SemanticSelectionId>,
        mut parameters: Vec<SemanticParameter>,
        affected_element_count: usize,
    ) -> ModelingOperation {
        pad_parameters_to_contract(kind, &mut parameters);
        let semantic_parameter_count = parameters.iter().map(parameter_width).sum::<usize>();
        let affected_element_count = affected_element_count
            .max(semantic_parameter_count * 8)
            .max(1);
        ModelingOperation {
            id: ProgramOperationId(id.into()),
            kind,
            selections,
            parameters,
            affected_element_count,
            payloads: vec![OperationPayloadDescriptor {
                kind: OperationPayloadKind::SemanticParameters,
                encoded_bytes: semantic_parameter_count * 16,
                semantic_parameter_count,
                affected_element_count,
                perturbation_valid: true,
            }],
        }
    }

    fn pad_parameters_to_contract(
        kind: ModelingOperationKind,
        parameters: &mut Vec<SemanticParameter>,
    ) {
        let required_count = topology_contract_for(kind)
            .map(|contract| contract.semantic_parameter_count)
            .or_else(|| {
                deformation_operator_contract(kind)
                    .map(|contract| usize::from(contract.semantic_parameter_count.minimum))
            });
        let Some(required_count) = required_count else {
            return;
        };
        while parameters.iter().map(parameter_width).sum::<usize>() < required_count {
            let index = parameters.len();
            parameters.push(SemanticParameter::Scalar {
                name: format!("contract_control_{index}"),
                value: 0.0,
            });
        }
    }

    fn parameter_width(parameter: &SemanticParameter) -> usize {
        match parameter {
            SemanticParameter::Scalar { .. }
            | SemanticParameter::Integer { .. }
            | SemanticParameter::Boolean { .. }
            | SemanticParameter::Choice { .. } => 1,
            SemanticParameter::Vector3 { .. } => 3,
            SemanticParameter::Quaternion { .. } => 4,
        }
    }

    fn dependency_graph(operations: &[ModelingOperation]) -> ProgramDependencyGraph {
        let mut operation_edges = operations
            .windows(2)
            .map(|pair| (pair[0].id.clone(), pair[1].id.clone()))
            .collect::<Vec<_>>();
        operation_edges.sort();
        operation_edges.dedup();

        let mut selection_edges = operations
            .iter()
            .flat_map(|operation| {
                operation
                    .selections
                    .iter()
                    .cloned()
                    .map(|selection| (selection, operation.id.clone()))
            })
            .collect::<Vec<_>>();
        selection_edges.sort();
        selection_edges.dedup();

        ProgramDependencyGraph {
            operation_edges,
            selection_edges,
        }
    }

    fn placeholder_mesh_artifact(case_id: &str) -> CharacterMeshArtifact {
        CharacterMeshArtifact {
            id: format!("mesh.{case_id}.placeholder"),
            canonical_units: "meters".to_owned(),
            coordinate_system: "y_up_z_forward_right_handed".to_owned(),
            semantic_descriptor_fingerprint: "placeholder".to_owned(),
            raw_geometry_size: CharacterRawGeometrySize {
                vertex_count: 1,
                face_count: 1,
                position_bytes: 12,
                topology_bytes: 12,
            },
            connected_component_count: 1,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [1.0, 1.0, 1.0],
            topology_fingerprint: "placeholder".to_owned(),
            canonical_position_fingerprint: "placeholder".to_owned(),
            artifact_fingerprint: "placeholder".to_owned(),
        }
    }

    fn selection_part(id: &str, part: &str) -> SemanticSelection {
        SemanticSelection {
            id: SemanticSelectionId(id.to_owned()),
            payload: SemanticSelectionPayload::Part {
                part: SemanticPartId(part.to_owned()),
            },
        }
    }

    fn selection_region(id: &str, region: &str) -> SemanticSelection {
        SemanticSelection {
            id: SemanticSelectionId(id.to_owned()),
            payload: SemanticSelectionPayload::Region {
                region: SemanticRegionId(region.to_owned()),
            },
        }
    }

    fn selection_edge_class(id: &str, class: &str) -> SemanticSelection {
        SemanticSelection {
            id: SemanticSelectionId(id.to_owned()),
            payload: SemanticSelectionPayload::EdgeClass {
                class: class.to_owned(),
            },
        }
    }

    fn clamp_range(value: f32, range: ScalarRange) -> f32 {
        value.clamp(range.min, range.max)
    }

    fn normalized_scalar(default: f32) -> ScalarRange {
        ScalarRange {
            min: 0.0,
            max: 1.0,
            default,
        }
    }

    fn nonnegative_scalar(default: f32) -> ScalarRange {
        ScalarRange {
            min: 0.0,
            max: (default * 2.0).max(1.0),
            default,
        }
    }

    fn signed_scalar(default: f32) -> ScalarRange {
        ScalarRange {
            min: -1.0,
            max: 1.0,
            default,
        }
    }

    fn suffix(value: &str) -> String {
        value.rsplit('.').next().unwrap_or(value).replace('_', "-")
    }

    #[derive(Debug, Copy, Clone)]
    struct CharacterCorpusRng {
        state: u64,
    }

    impl CharacterCorpusRng {
        fn new(seed: u64) -> Self {
            Self {
                state: seed ^ 0x517c_c1b7_2722_0a95,
            }
        }

        fn next_u64(&mut self) -> u64 {
            self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut value = self.state;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        }

        fn unit_f32(&mut self) -> f32 {
            let bits = self.next_u64() >> 40;
            (bits as f32) / ((1_u64 << 24) as f32)
        }

        fn scalar_f32(&mut self, min: f32, max: f32) -> f32 {
            ((min + (max - min) * self.unit_f32()) * 1000.0).round() / 1000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CHARACTER_GRAMMAR_SCHEMA_VERSION,
        base::{
            BASE_TOPOLOGY_LIBRARY_VERSION, base_topology_library, fingerprinted_character_bases,
        },
        garment::GARMENT_GRAMMAR_SCHEMA_VERSION,
        hair::HAIR_GRAMMAR_SCHEMA_VERSION,
    };
    use shape_program::{
        GrammarProfile, RawGeometrySize, SemanticAdmissibilityPolicy, SemanticTopologyExact,
        SerializationOrderExact,
        runtime::{ForwardRuntimeConfig, validate_forward_program_runtime},
    };
    use shape_program_verify::{StrictVerificationEvidence, verify_strict_semantic_program};

    use super::private_fixtures::*;
    use super::*;

    #[test]
    fn generated_character_corpus_is_deterministic() {
        let first = generated_character_corpus(17);
        let second = generated_character_corpus(17);
        let third = generated_character_corpus(18);

        assert_eq!(
            first.schema_version,
            GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION
        );
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).expect("public corpus should serialize"),
            serde_json::to_string(&second).expect("public corpus should serialize")
        );
        assert_ne!(first, third);
        first
            .validate_public_inputs()
            .expect("public corpus should validate");
    }

    #[test]
    fn public_known_base_descriptor_helpers_match_generated_meshes() {
        for case in generated_character_corpus(19).cases {
            let features = known_base_character_feature_candidates()
                .into_iter()
                .find(|features| {
                    known_base_character_signature_for_features(*features).matches_mesh(&case.mesh)
                })
                .expect("generated mesh maps to a known-base descriptor feature set");
            let descriptor = known_base_character_descriptor_for_features(features);

            assert_eq!(
                case.mesh.semantic_descriptor_fingerprint,
                descriptor.semantic_descriptor_fingerprint
            );
            assert_eq!(
                case.mesh.topology_fingerprint,
                descriptor.topology_fingerprint
            );
            assert_eq!(
                case.mesh.canonical_position_fingerprint,
                descriptor.canonical_position_fingerprint
            );
        }
    }

    #[test]
    fn public_known_base_candidates_do_not_advertise_unbacked_topology_edits() {
        for features in known_base_character_feature_candidates() {
            assert!(
                !features.topology_edited
                    || features.asymmetric
                    || features.clothed
                    || features.hair,
                "topology-edited descriptor candidates must have an operation source"
            );
        }
    }

    #[test]
    fn hidden_programs_exist_only_in_private_fixtures() {
        let private_cases = authored_character_cases(22);
        let public = generated_character_corpus(22);

        assert_eq!(private_cases.len(), public.cases.len());
        assert!(private_cases.iter().all(|case| {
            case.source_program.grammar_profile == GrammarProfile::StrictFromVersionedLibrary
                && case.source_program.base_topology.is_some()
                && !case.source_program.operations.is_empty()
        }));

        let public_value = serde_json::to_value(&public).expect("public corpus should serialize");
        assert_public_corpus_shape(&public_value);
        assert_no_keys_recursive(
            &public_value,
            &[
                "source_program",
                "answer_key",
                "grammar_profile",
                "operations",
                "dependency_graph",
                "coverage",
                "label",
            ],
        );
        assert_no_string_fragment_recursive(&public_value, "op.character.");
    }

    #[test]
    fn exposed_benchmark_inputs_do_not_include_literal_target_programs() {
        let corpus = generated_character_corpus(33);
        for case in &corpus.cases {
            let value = serde_json::to_value(case).expect("case should serialize");
            assert_case_input_shape(&value);
            assert_no_keys_recursive(
                &value,
                &[
                    "answer_key",
                    "source_program",
                    "proportion_controls",
                    "face_operations",
                    "garment_document",
                    "hair_grammar",
                    "topology_edits",
                    "coverage",
                    "label",
                ],
            );
        }
    }

    #[test]
    fn public_mesh_artifact_validation_rejects_stale_descriptors() {
        let corpus = generated_character_corpus(34);
        let mut stale_bytes = corpus.cases[0].mesh.clone();
        stale_bytes.raw_geometry_size.position_bytes += 4;
        assert!(matches!(
            stale_bytes.validate(),
            Err(CharacterCorpusError::InvalidMeshArtifact { .. })
        ));

        let mut stale_fingerprint = corpus.cases[0].mesh.clone();
        stale_fingerprint.artifact_fingerprint = "stale".to_owned();
        assert!(matches!(
            stale_fingerprint.validate(),
            Err(CharacterCorpusError::InvalidMeshArtifact { .. })
        ));

        let mut huge_size = corpus.cases[0].mesh.clone();
        huge_size.raw_geometry_size.vertex_count = usize::MAX;
        huge_size.raw_geometry_size.position_bytes = usize::MAX;
        assert!(matches!(
            huge_size.validate(),
            Err(CharacterCorpusError::InvalidMeshArtifact { .. })
        ));
    }

    #[test]
    fn public_corpus_rejects_extra_fields_and_wrong_case_meshes() {
        let corpus = generated_character_corpus(35);
        let mut top_level_extra = serde_json::to_value(&corpus).expect("corpus serializes");
        top_level_extra["answer_key"] = serde_json::json!({});
        assert!(serde_json::from_value::<GeneratedCharacterCorpus>(top_level_extra).is_err());

        let mut case_extra = serde_json::to_value(&corpus).expect("corpus serializes");
        case_extra["cases"][0]["coverage"] = serde_json::json!({"hair": true});
        assert!(serde_json::from_value::<GeneratedCharacterCorpus>(case_extra).is_err());

        let mut mesh_extra = serde_json::to_value(&corpus).expect("corpus serializes");
        mesh_extra["cases"][0]["mesh"]["source_program"] = serde_json::json!({});
        assert!(serde_json::from_value::<GeneratedCharacterCorpus>(mesh_extra).is_err());

        let mut raw_size_extra = serde_json::to_value(&corpus).expect("corpus serializes");
        raw_size_extra["cases"][0]["mesh"]["raw_geometry_size"]["coverage"] =
            serde_json::json!("posed");
        assert!(serde_json::from_value::<GeneratedCharacterCorpus>(raw_size_extra).is_err());

        let mut mixed_case = generated_character_corpus(35);
        mixed_case.cases[0].mesh = generated_character_corpus(35).cases[1].mesh.clone();
        assert!(matches!(
            mixed_case.validate_public_inputs(),
            Err(CharacterCorpusError::InvalidMeshArtifact { .. })
        ));

        let mut missing_case = generated_character_corpus(35);
        missing_case.cases.pop();
        assert!(matches!(
            missing_case.validate_public_inputs(),
            Err(CharacterCorpusError::InvalidCaseSet { .. })
        ));
    }

    #[test]
    fn generated_meshes_and_private_artifacts_validate() {
        let public = generated_character_corpus(44);
        for case in authored_character_cases(44) {
            case.mesh.validate().expect("mesh artifact should validate");
            let public_case = public
                .case_by_id(&case.public_id)
                .expect("private fixture should project to a public case");
            assert_eq!(public_case.mesh, case.mesh);
            assert_source_mesh_provenance(&case);
            if let Some(pose) = &case.answer_key.normalized_pose {
                assert!(pose.is_valid());
            }
            if let Some(document) = &case.answer_key.garment_document {
                document
                    .validate()
                    .expect("garment answer-key document should validate");
            }
            if let Some(grammar) = &case.answer_key.hair_grammar {
                grammar
                    .validate()
                    .expect("hair answer-key grammar should validate");
            }
            verify_private_source_program(&case);
        }
    }

    #[test]
    fn topology_edit_program_selections_match_answer_key_targets() {
        for case in authored_character_cases(45) {
            for edit in &case.answer_key.topology_edits {
                let operation_id = format!("op.{}.{}", case.id, edit.id);
                let operation = case
                    .source_program
                    .operations
                    .iter()
                    .find(|operation| operation.id.0 == operation_id)
                    .unwrap_or_else(|| panic!("missing topology edit operation {operation_id}"));
                assert_eq!(
                    operation.selections,
                    vec![topology_edit_selection_id(edit)],
                    "topology edit {} with target {} selected the wrong program target",
                    edit.id,
                    edit.target
                );
            }
        }
    }

    #[test]
    fn base_fingerprints_and_grammar_versions_are_recorded() {
        let cases = authored_character_cases(55);
        let required_base_ids = fingerprinted_character_bases()
            .into_iter()
            .map(|base| base.base.id.0)
            .collect::<BTreeSet<_>>();

        for case in cases {
            assert_eq!(
                case.answer_key.schema_version,
                GENERATED_CHARACTER_CORPUS_SCHEMA_VERSION
            );
            assert_eq!(
                case.answer_key.base_library.version,
                BASE_TOPOLOGY_LIBRARY_VERSION.to_string()
            );
            assert_eq!(
                case.answer_key.base_library.fingerprint,
                base_topology_library().fingerprint().0
            );
            assert_eq!(
                case.answer_key.proportion_grammar.schema_version,
                CHARACTER_GRAMMAR_SCHEMA_VERSION
            );
            assert_eq!(
                case.answer_key.proportion_grammar.version,
                BASE_TOPOLOGY_LIBRARY_VERSION.to_string()
            );
            assert!(!case.answer_key.proportion_grammar.grammar_id.is_empty());
            assert_eq!(
                case.answer_key.face_grammar.schema_version,
                CHARACTER_GRAMMAR_SCHEMA_VERSION
            );
            assert!(!case.answer_key.face_grammar.grammar_id.is_empty());
            if case.coverage.clothed {
                let garment = case
                    .answer_key
                    .garment_grammar
                    .as_ref()
                    .expect("clothed cases record garment grammar");
                assert_eq!(garment.schema_version, GARMENT_GRAMMAR_SCHEMA_VERSION);
                assert!(!garment.grammar_id.is_empty());
            }
            if case.coverage.hair {
                let hair = case
                    .answer_key
                    .hair_grammar_reference
                    .as_ref()
                    .expect("hair cases record hair grammar");
                assert_eq!(hair.schema_version, HAIR_GRAMMAR_SCHEMA_VERSION);
                assert!(!hair.grammar_id.is_empty());
            }
            let case_base_ids = case
                .answer_key
                .base_topologies
                .iter()
                .map(|base| base.base_id.clone())
                .collect::<BTreeSet<_>>();
            assert_eq!(case_base_ids, required_base_ids);
            for base in &case.answer_key.base_topologies {
                assert_eq!(base.version, BASE_TOPOLOGY_LIBRARY_VERSION.to_string());
                assert!(!base.fingerprint.is_empty());
            }
        }
    }

    #[test]
    fn corpus_cases_cover_required_character_variants() {
        let cases = authored_character_cases(66);
        assert!(cases.iter().any(|case| case.coverage.symmetric));
        assert!(cases.iter().any(|case| case.coverage.asymmetric));
        assert!(cases.iter().any(|case| case.coverage.clothed));
        assert!(cases.iter().any(|case| case.coverage.hair));
        assert!(cases.iter().any(|case| case.coverage.posed));
        assert!(cases.iter().any(|case| case.coverage.topology_edited));
        assert!(cases.iter().any(|case| {
            case.coverage.asymmetric
                && case.coverage.clothed
                && case.coverage.hair
                && case.coverage.posed
                && case.coverage.topology_edited
        }));
    }

    fn verify_private_source_program(case: &AuthoredCharacterCase) {
        let evidence = StrictVerificationEvidence {
            canonical_positions_exact: true,
            semantic_topology_exact: SemanticTopologyExact {
                graph: true,
                polygon_boundaries: true,
                winding: true,
                part_object_membership: true,
                geometry: true,
            },
            serialization_order_exact: SerializationOrderExact {
                vertex_order: true,
                face_order: true,
            },
            residual_bytes: 0,
            literal_target_mesh_bytes: 0,
            per_vertex_independent_position_parameters: 0,
            perturbation_valid: true,
            target_index_permutation_adapter_bytes: 0,
        };
        let report = verify_strict_semantic_program(
            &case.source_program,
            &SemanticAdmissibilityPolicy::strict(),
            raw_geometry_size(case.mesh.raw_geometry_size),
            &evidence,
        )
        .expect("strict verification should run");
        assert!(
            report.accepted,
            "private source program for {} should verify: {:?}",
            case.id, report.issues
        );

        let runtime = validate_forward_program_runtime(
            case.id.clone(),
            &case.source_program,
            &ForwardRuntimeConfig::canonical(),
        );
        assert!(
            runtime.accepted,
            "private source program for {} should pass runtime gate: {:?}",
            case.id, runtime.issues
        );
    }

    fn assert_source_mesh_provenance(case: &AuthoredCharacterCase) {
        assert_eq!(
            case.answer_key.source_output_fingerprint,
            source_program_fingerprint(&case.source_program).expect("source program hashes")
        );
        assert!(
            case.source_program
                .operations
                .iter()
                .flat_map(|operation| operation.parameters.iter())
                .any(|parameter| matches!(
                    parameter,
                    shape_program::SemanticParameter::Choice { name, value }
                        if name == "mesh_provenance_key"
                            && value == &case.answer_key.mesh_provenance_key
                ))
        );
        assert_eq!(case.answer_key.expected_mesh, case.mesh);
        assert!(
            case.source_program
                .base_topology
                .as_ref()
                .is_some_and(|base| base.fingerprint == case.answer_key.base_library.fingerprint)
        );
    }

    fn raw_geometry_size(size: CharacterRawGeometrySize) -> RawGeometrySize {
        RawGeometrySize {
            vertex_count: size.vertex_count,
            face_count: size.face_count,
            position_bytes: size.position_bytes,
            topology_bytes: size.topology_bytes,
        }
    }

    fn assert_public_corpus_shape(value: &serde_json::Value) {
        let object = value
            .as_object()
            .expect("corpus should serialize as object");
        assert_eq!(
            object.keys().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "schema_version".to_owned(),
                "seed".to_owned(),
                "cases".to_owned()
            ])
        );
        for case in object
            .get("cases")
            .and_then(serde_json::Value::as_array)
            .expect("cases should serialize as array")
        {
            assert_case_input_shape(case);
        }
    }

    fn assert_case_input_shape(value: &serde_json::Value) {
        let object = value.as_object().expect("case should serialize as object");
        assert_eq!(
            object.keys().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from(["id".to_owned(), "mesh".to_owned()])
        );
        let mesh = object
            .get("mesh")
            .and_then(serde_json::Value::as_object)
            .expect("mesh should serialize as object");
        assert!(mesh.contains_key("semantic_descriptor_fingerprint"));
        assert!(mesh.contains_key("topology_fingerprint"));
        assert!(mesh.contains_key("canonical_position_fingerprint"));
        assert!(mesh.contains_key("artifact_fingerprint"));
    }

    fn assert_no_keys_recursive(value: &serde_json::Value, forbidden: &[&str]) {
        match value {
            serde_json::Value::Object(object) => {
                for key in forbidden {
                    assert!(
                        !object.contains_key(*key),
                        "public input leaked forbidden key {key}"
                    );
                }
                for child in object.values() {
                    assert_no_keys_recursive(child, forbidden);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    assert_no_keys_recursive(child, forbidden);
                }
            }
            _ => {}
        }
    }

    fn assert_no_string_fragment_recursive(value: &serde_json::Value, forbidden: &str) {
        match value {
            serde_json::Value::String(value) => {
                assert!(
                    !value.contains(forbidden),
                    "public input leaked forbidden string fragment {forbidden}"
                );
            }
            serde_json::Value::Object(object) => {
                for child in object.values() {
                    assert_no_string_fragment_recursive(child, forbidden);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    assert_no_string_fragment_recursive(child, forbidden);
                }
            }
            _ => {}
        }
    }
}
